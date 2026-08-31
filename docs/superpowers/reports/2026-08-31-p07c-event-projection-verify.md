# Verify Report — p07c-event-projection（Phase 0.7C-6: Event Projection Foundation + D8 EventSink Decoupling）

- 日期：2026-08-31 · 验证人：开发 AI（ZCode）· 模式：full workflow
- 分支：`comet/p07c-event-projection`（base `a6c5925`）· 提交：`f1f753e`（实现）+ `0f73c2e`（盒上迭代收敛）
- 前置：Architecture Probe（只读）`docs/superpowers/reports/2026-08-31-p07c6-architecture-probe.md`
- 结论：**PASS**（0 CRIT / 0 IMP / 2 NOTE）

## 1. 范围对表（终审裁定逐项）

| 终审/probe 项 | 落点（f1f753e + 0f73c2e） | 证据 |
|---|---|---|
| **D8：Runtime Event → Sink → Log → Projection 生产边** | `RuntimeEventSink` trait + `RuntimeEventLog` impl；组合根唯一 `Arc<RuntimeEventLog>` 注入 SessionManager 与 Supervisor | `evt_proj_rt_01_decoupled_single_table` |
| SessionManager 不再穿 Supervisor | `emit()` 直连 sink；`sup.lock().record()` 路径删除；`sup` 只剩决策职责（watchdog 调用面零变更） | 源码 diff + decoupled 测试 |
| Supervisor 回归纯决策引擎 | 删 `events` 字段与 `record`/`drain_events`/`pending_events`（probe Q3：生产代码零调用者）；决策/ingest 事件经注入 sink；决策逻辑/状态机/退避零改动 | supervisor 既有 10 项测试全绿（夹具更新） |
| 单表单锁，事件不分流 | 两生产者（session emit + supervisor 决策）经同一 log 的同一 Mutex 串行 | decoupled 测试：发射序 `created < fault < 后续迁移` 跨生产者保持 |
| **project() 纯函数 + EventProjection** | `{total, kind_counts, session_states, session_failures, has_critical}`——组合非展开（BTreeMap 确定性）；零副作用；**Observation≠Configuration**（只读快照绝不写回） | `evt_proj_rt_01_project_is_pure_and_fifo` |
| **四语义零偷改（probe Q4 基线）** | 顺序（FIFO 投影序=发射序）/ 丢失（两级丢弃行为不变+计数器可见，投影=drain 所见不伪造）/ 重复（双发容忍计数×3）/ failure 隔离（drain 后投影零影响、纯函数无 panic） | 四项独立测试 |
| **Event Projection ≠ External API** | event_projection.rs 零 transport/RPC 面；不做 Health Reducer 完整实现/词表变更/持久化 | proposal 不做清单 + 源码 |
| 事件词表 14 变体封闭零改动 | `evt_proj_rt_01_vocabulary_snapshot` 回归 | 盒上 PASS |

## 2. 三层证据

### Unit/Simulation（盒上，~/p07_results.txt 第五轮 + ~/p07_run_console.log）
- 命令：`bash ~/p07_verify.sh`（cd ~/media-agent-build）
- 结果：14 项全 0（fmt×2 / test×4 / clippy×4 / build×3 / PROOF）
- 测试计数：**138 / 138 / 188 / 138**（mock 182→188，+6：`evt_proj_rt_01_{vocabulary_snapshot, project_is_pure_and_fifo, loss_semantics_visible, duplicate_tolerant, projection_failure_isolation, decoupled_single_table}`）

### Hardware（真机 lytv@10.30.15.10，bmd,gstreamer 构建）
- 命令：`VBMF_SESSION_LIFECYCLE=1 MEDIA_AGENT_DEVICE_BINDING=/home/lytv/loopback-manifest-v2.json timeout 240 ./target/debug/media-agent`
- 结果：**GATE_EXIT=0**（工件 `~/p07_gate_hw.log`）：
  - `EVENT-PROJECTION-RT-01 total=46 kinds={lease_granted:4, resource_allocated:3, session_created:4, session_failed:1, session_state_changed:31, source_materialized:3}`
  - `session_states={…3×released, 1×provisioningfailed, 1×leased} session_failures={…:1} has_critical=true`——投影如实捕获全 gate 生命周期（含 RESOURCE-RT-01 故意制造的被拒会话 provisioningfailed）
  - `dropped_obs=0 dropped_crit=0`（零丢失）
  - 回归：`SESSION-RT-01/RESOURCE-RT-01 ALL PASS`（IDEMPOTENCY/ERROR-MODEL/COMMAND-CONTRACT 段同轮全过）

### CI
- PR required checks 以 GitHub 实跑为准（§6）。

## 3. 红线核验

- **Observation≠Configuration**：投影是纯只读函数（`&[RuntimeEvent]` → 快照），无任何写回路径。
- **词表零改动**：14 kind() 快照回归；probe 发现的 4 个零生产词表项（IdentityResolved/SignalVerified/LoopbackVerified/ResourceReservationExpired）保持原状未点亮（登记演进，不顺手做）。
- **零触碰**：resource/lease/pipeline/preflight/runtime_state/runtime_query 零 diff；command/idempotency/error_model 仅 world() 夹具构造参数机械更新（语义零变更）。
- **Decision-level 变更披露**：SessionManager::new 与 Supervisor::new 签名变化（新增 sink 参数）——这是 D8 解耦的主题内变更（终审指定核心工作），非范围蔓延。

## 4. 迭代披露（四轮，全部如实）

1. R1：`RuntimeEventLog` 无 Clone——测试改克隆 `Arc`（`log.clone()` 为 Arc 克隆）。
2. R2：测试摸 `SessionManager` 私有 `sup` 字段（E0616）——保留构造时的 `sup` Arc 句柄。
3. R3：两处断言与既有事实不符——①两级丢弃策略中 Observation 被挤出不计数（计数器只记全 Critical 拒收；**既有语义已注释锁定**，若演进为"挤出不静默"须同步改 events.rs 契约）；②create 内部先发相位迁移再发 session_created。
4. R4：state_changed 的 `to` 是 canonical 小写相位（`running`）非 Rust 变体名。

## 5. 文档对账

- Phase Map：0.7C-6 行 ✅ COMPLETE（tag `phase-0.7C6-event-projection`）；0.7C 行下一项 = **External API**（§3 顺序全部完成）。
- 债表：**D8 → CLOSED @ p07c-event-projection**（解耦证据引用）；D14/D15/Preflight 粒度 P1 保持 OPEN。

## 6. 分级

- **CRIT：0** · **IMP：0**
- NOTE 1：两级丢弃策略中 Observation 被挤出时 `dropped_observations` 不递增（既有 0.7A P1-3 实现语义——计数器只记"全 Critical 拒收新观测"）。测试已按既有实现锁定；若终审认为"挤出不静默"应包含挤出场景，属 events.rs 契约演进（需新决策，本 change 零偷改）。
- NOTE 2：Health Reducer 完整实现未做（watchdog tick 的 report_failure 调用面零变更）——Supervisor 作为事件消费者的轮询式消费属 watchdog 演进，当前决策仍由调用驱动（与解耦前一致）。
