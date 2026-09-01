---
comet_change: p07d-event-integration
role: technical-design
canonical_spec: openspec
---

# 0.7D 事件内消费集成 — Design Doc（深度技术设计）

> 上游：`docs/openspec/changes/p07d-event-integration/{proposal,design,tasks}.md` + handoff（hash 88223a11…）。
> 本文是 open 阶段高层 design.md 的深度细化：D3 定稿、映射表逐项、接线与测试策略。

## 1. 现状事实（probe 实证，全部实码核对）

- 事件链：生产（session.rs 七种 kind + supervisor.rs 决策事件）→ `RuntimeEventLog`（组合根单表，1024 cap 两级丢弃）→ 外送（transport.rs:229 `drain→project`；main.rs:1091 gate 证据路径）。**内部无任何消费者**。
- `supervisor.rs` 决策事件实况：`report_failure`→Restart 路径发 `PipelineFault{retryable:true}`、Escalate 路径发 `HealthChanged{to:"manual_required"}`（from:"unhealthy"）；`report_recovered`→`HealthChanged{to:"recovered"}`（from:"restarting"）；`escalate`→`HealthChanged{to:"manual_required"}`（from:"running"）；`ingest`→默认映射器归一化上游观测。**`begin_restart`/`backoff` 不发事件**。
- `main.rs` agent_state 散写全集（七处）：Ready:499（构造初值）/ Capturing:537,1233 / Degraded:1253,1258 / Ready:1274 / ManualRequired:1467,1483。**从不写 Restarting/Backoff**；全部位于 watch loop / 诊断自启动路径，生产路径（无 watch loop）维持 Ready 不变。
- `health.rs` 为 Gate 2.1 冻结 skeleton（`#![allow(dead_code)]`，未接线）。
- 四项零生产事件（实测 0 个非测试 emit）：IdentityResolved / SignalVerified / LoopbackVerified / ResourceReservationExpired。

## 2. D3 定稿：单日志多消费者 drain 语义

### 2.1 三候选对勘

| 候选 | 机制 | 致命缺陷 |
|------|------|----------|
| A. 非破坏快照读 | reducer 每 tick 读日志快照 | 重叠窗口重复折叠（同事件被多个 tick 反复计入派生态），需窗口去重或游标，语义脆弱 |
| B. 游标增量读 | reducer 持单调游标只读新增 | **外送 `drain()` 清空日志后游标失效**——游标>新长度时丢失窗口且不可检测，两消费者仍然竞争 |
| **C. 双日志分流（选定）** | 组合根 `FanoutSink` 同序双写 | 内存 2×1024 事件（可忽略）；emit 路径多一次锁获取 |

### 2.2 选定设计：`FanoutSink`

```rust
/// 组合根事件分流: 同一 emit 顺序写入外送投影日志与内部消费日志。
/// 契约继承 RuntimeEventSink: 永不阻塞、永不失败。
pub struct FanoutSink {
    projection: Arc<RuntimeEventLog>,   // 外送侧 (transport 投影端点 + gate 证据路径照旧 drain)
    internal: Arc<RuntimeEventLog>,     // 内消费侧 (watchdog tick drain -> reduce)
}
impl RuntimeEventSink for FanoutSink {
    fn emit(&self, ev: RuntimeEvent) {
        self.projection.push(ev.clone());
        self.internal.push(ev);
    }
}
```

- **接线**：组合根构造 `projection`/`internal` 两日志 + `FanoutSink`；生产者（SessionManager/Supervisor）持有的 `Arc<dyn RuntimeEventSink>` 指向 FanoutSink；`TransportContext.events` 与 gate 路径接 `projection` 实例。**transport.rs 零改动**（`Arc<RuntimeEventLog>` 类型不变，只换背后实例——main.rs 接线层变更）。
- **四语义保持**：每条日志独立维持 FIFO/两级丢弃/容量上界（同一 `RuntimeEventLog` 类）；顺序一致（emit 内顺序双 push，锁逐条获取，全序保持）；重复容忍（同一事件在两日志各一份，消费语义各自独立）；failure 隔离（push 永不失败；锁中毒即 panic 传播，与现状单日志行为一致）。
- **`EventProjection ≠ CanonicalRuntimeState`**：internal 侧只被 reduce 读取派生观测态，不写回任何 Command/Graph/Backend 路径。

## 3. Health Reducer（health.rs：skeleton → 完整实现）

### 3.1 折叠状态与签名（纯函数）

```rust
/// reducer 持久折叠上下文（跨 tick 保留; 最小派生面, 禁万能 struct）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthFold {
    pub agent: AgentState,
    pub active_sessions: usize,
}
/// 纯函数: 同输入同输出; 消费侧只读派生, 零写回。
pub fn reduce(state: &HealthFold, events: &[RuntimeEvent]) -> HealthFold;
```

### 3.2 映射表（14 kinds 逐项）

| 事件 | 折叠效果 |
|------|----------|
| `HealthChanged{to:"manual_required"}` | agent=ManualRequired（决策权威，最高优先） |
| `HealthChanged{to:"recovered"}` | agent=Capturing（active_sessions>0）else Ready |
| `PipelineFault{retryable:true}` | agent=Degraded（重启排程中，待 recovered） |
| `PipelineFault{retryable:false}` / `HardwareFault` | agent=Degraded（等待 escalate 决策事件） |
| `SessionFailed` | agent=Degraded |
| `AmbiguousIdentity` | agent=Degraded（拒识待 Policy） |
| `SessionCreated` | active_sessions+=1（态不变） |
| `SessionStateChanged{to:"Released"}` | active_sessions-=1；归零且无故障 pending → Ready |
| `SessionStateChanged{to:"Running"}` | agent=Capturing 候选（无更高级 pending 时生效） |
| `LeaseGranted` / `ResourceAllocated` / `SourceMaterialized` / `IdentityResolved` / `SignalVerified` / `LoopbackVerified` | 观测记账，不改主态（点亮后入流即可投影） |
| `ResourceReservationExpired` | agent=Degraded（资源面运维可见降级） |
| 其余 `SessionStateChanged`（微相位） | 态不变（折叠上下文不追踪微相位） |

优先级格：`ManualRequired > Degraded > Capturing > Ready`；`recovered`/会话归零是重置边。`Starting` 为构造 bootstrap 态（`HealthFold` 初值），非事件派生。

**诚实登记（不派生）**：`Restarting` / `Backoff`——词表在册但无事件生产者（`begin_restart`/`backoff` 不发事件），且现行散写也从不写这两态；本期不为其新增 supervisor 发射点（无消费者的发射=事件噪声，违背点亮纪律），登记为后续 watchdog 演进项（若未来 /health 需要 restart 粒度再补发射+派生）。

### 3.3 main.rs 七处散写收敛

watch loop tick 内统一：`drain internal → reduce → 写 agent_state`，七处命令式赋值删除。等价性目标=现行实际写集 {Ready, Capturing, Degraded, ManualRequired} 逐场景同终态（Simulation 等价性测试逐场景断言，见 §6）。生产路径（无 watch loop）：无 tick 即无派生，维持构造态 Ready——与现状**行为等价**（现状生产路径同样不改态），不回归；生产 tick 属 Node 控制面集成（非目标）。

## 4. Supervisor 事件驱动输入

watch loop tick 内：internal drain 出的故障类切片（`is_fault()`）与既有轮询条件（acceptance/bus 事件）**OR** 后触发 `report_failure`；**每 tick 每设备至多一次**（防双计导致 attempts/backoff 翻倍——等价性测试锁定决策序列一致）。`report_failure/begin_restart/report_recovered` 签名与语义零变更（纯决策引擎形态保持）。

## 5. 四项事件点亮锚点（实名，只加 emit）

| 事件 | 锚点 | 语义时机 |
|------|------|----------|
| `IdentityResolved` | session.rs `create()` binding-verify 成功点（device_id 解析 + 绑定 confidence 收敛处） | 身份解析收敛（confidence canonical 字符串，如 "high/exact"） |
| `SignalVerified` | main.rs watchdog a4 翻真点（`first_frame_ok()`: 双路首帧 + PTS 单调; 闩锁去重） | 输入信号检出且首缓冲验收（生产点亮, 4.3 真机实证 kind_counts=1） |
| `LoopbackVerified` | main.rs VBMF_LOOPBACK gate 段 `all_pass` 验收点（signal.rs `verify_fixtures` 双门之后; fixture 级 → device_id=nil 未归属） | 输出→SDI→输入收到预期信号（4.3 真机实证 loopback_verified=1, 方案 A 独立入口闭环） |
| `ResourceReservationExpired` | session.rs tick 过期滞留调用点（`expire_reservations_of`; resource.rs 仅持状态过渡） | 预留窗口到期自动回收（4.3 真机实证: 生产 5s tick 驱动 30s 窗口, 精确计数 1） |

emit 经生产者既有注入 sink（FanoutSink）；词表/serde tag/平面零改动；Simulation 断言各 kind `kind_counts` 增量精确（不产生噪声事件）。

## 6. 测试策略

- **Unit**（health.rs tests）：reduce 纯函数——同输入同输出；映射表逐行；优先级格（Degraded 压 Capturing、ManualRequired 压一切）；归零回 Ready；故障 pending 保持（Degraded 不被 Observation 偷翻转）。
- **Simulation**（mock feature）：
  - FanoutSink：双日志顺序一致（同 emit 序列两日志 drain 逐条相等）；drop 计数独立；重复事件容忍。
  - 新旧等价：Mock 全链场景（启动/运行/故障/恢复/释放/多会话）断言 reducer 终态 = 现行散写终态。
  - Supervisor 消费等价：事件驱动 vs 纯轮询同场景同决策序列（attempts/backoff/action 逐 tick 相等）。
  - 4 事件点亮：`kind_counts` 各 +N 精确断言。
  - 回归：`evt_proj_rt_01_*` ×6 全绿（0.7C-6 四语义零偷改）。
- **Hardware**（盒上 `VBMF_SESSION_LIFECYCLE=1` gate 段新增 EVENT-INTEGRATION-RT-01）：真机生命周期事件流 → reducer 派生终态打印并断言 = 现行路径终态；外送投影独立实证（EVENT-PROJECTION-RT-01 回归：projection 日志事件计数不因内消费减少）；TRANSPORT-RT-01 回归；/health 字段逐字段不变。

## 7. 风险与缓解

- [FanoutSink 双写放大锁竞争] → emit 频率为生命周期级（非逐帧），实测无感；Simulation 并发 emit 顺序一致性测试。
- [reducer 与散写终态不一致] → §6 等价性测试逐场景；不一致即修映射表，不改测试迁就实现。
- [事件驱动+轮询双触发使 backoff 翻倍] → 每 tick 每设备单次 report_failure 门闸 + 决策序列等价测试。
- [点亮锚点选错制造噪声] → §5 锚点均为语义真值点（非日志/重试点）；kind_counts 精确断言。
- [删陈旧目录误删] → 删前 diff 归档件零差异复核（已核一次，删时再核）。

## 8. Migration

全部 additive（FanoutSink/HealthFold/reduce/emit 点）+ 行为收敛（散写删除，等价性测试兜底）；单 change revert 即回滚。CI 沿用七 required checks；盒上全矩阵 + 真机 gate 不降。
