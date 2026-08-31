# PHASE_0_7A_POST_MERGE_DEBT — Phase 0.7A 合并后债务清单（冻结）

> 状态：DEBT REGISTER（P1/后续阶段债务登记；逐项不得在 0.7A 之外顺手实现）。
> 来源：`comet/p07-session-runtime` 四轮 Merge Gate 复核记录（verify 报告 §7/§8/§9 附录）+ 终审裁定。
> 纪律：下一阶段（0.7B Media Semantics / 0.7C External Integration）开工前，本清单为**强制阅读项**；
> 任何一项的关闭都必须走独立 change + 三层测试，不允许"顺手修"。

## 生命周期编排（P1-1 类）

| # | 债务 | 现状 | 关闭条件 | 目标阶段 |
|---|------|------|----------|----------|
| D1 | **LifecycleJournal / LifecycleTransaction**：`start()` 各失败点（instantiate/allocate/start/materialize）各自手写 rollback；新增步骤需复制回滚逻辑 | 手写分段 rollback（已覆盖已知失败点，单测锁定） | `CompletedStep[]` 记录 + `rollback(reverse)` 统一引擎；0.7B 新增步骤（Normalize/Clock/Audio/Encoder/Output）接入 | 0.7B 开工前 |
| D2 | **derive_claims FAIL 化（RESOURCE-RESOLUTION-01）**：intent 设备找不到 Resource 时 claims 为空 → Preflight 仅 WARN | WARN（legacy/无 manifest 路径兼容） | 资源解析三态 `Resolved/Missing/Ambiguous`：Missing/Ambiguous → FAIL | 0.7B |
| D3 | **per-claim Reservation TTL**：预留过期目前以 Reserved 相位停留窗口近似（tick 驱动） | 近似实现（crash-cleanup 语义成立） | 每 claim 独立 TTL + 显式 Renew/Expire/Abort 生命周期 | 0.7B |

## Preflight 精确化（P1-2 类）

| # | 债务 | 现状 | 关闭条件 | 目标阶段 |
|---|------|------|----------|----------|
| D4 | **PortAvailability 精确到端口**：当前仅"设备有任意端口" | 设备级判定 | `source.port_id → 具体端口 → direction==Input → capability 兼容 → runtime binding 可用`（SDI/HDMI/IP 混合卡前置条件） | 0.7B（Port Availability Contract） |
| D5 | **IdentityBinding 实查强度（IDENTITY-BINDING-01）**：当前仅 key-existence | contains_key | 实查 `Confidence==High / match_kind==ManifestVerified` + identity strength 分别判定 | 0.7B |
| D6 | **BACKEND-CAPABILITY-01**：BackendCapability stage 恒 WARN 占位 | 占位报告 | 真实能力探针接线 + 硬性判定规则 | 0.7B/0.7C |

## 结构清理（P1-3 类）

| # | 债务 | 现状 | 关闭条件 | 目标阶段 |
|---|------|------|----------|----------|
| D7 | **`backend: OnceLock` → 直接字段**：构造注入已是真实语义，OnceLock 制造"可后换"假象 | OnceLock（行为正确） | 改 `Arc<dyn MediaBackend>` 直字段 | 0.7A 余项/0.7B |
| D8 | **RuntimeEventSink 与 Supervisor 解耦**：SessionManager 经 `Supervisor::record` 出口（Supervisor 既是决策者又是事件表持有者） | 能工作；架构方向倒置 | `RuntimeEventSink → RuntimeEventLog → {Health Reducer, Supervisor, External Projection}` | 0.7C（Event Architecture） |
| D9 | **create 幂等键（Idempotency-Key）**：随机 SessionId 不解决业务幂等 | 随机 UUID | External API 引入 Idempotency-Key 语义 | 0.7C |
| D10 | **Session 内多 Pipeline**：start() 取 `plans.first()`，多设备会话仅物化首计划 | 单管线（租约/资源已全量持有） | 多管线实例编排（含每管线 Health/句柄表） | 0.7B+ |

## 词汇卫生（已裁定不阻塞，随 Identity/Provider 清理）

- `DeviceIdentitySource::RealBmd` 等 vendor-named 枚举值仍在 Domain vocabulary（0.6 终审遗留）。
- `ResolverEvidence.bmd_device_handle` / `DeviceBindingManifest.bmd_device_handle` JSON 键名保留（宿主证据文件，非 Domain schema）。

## 优先级分组（2026-08-30 重分类, p07b-consolidation 终审裁定）

### 🔴 0.7C 前必须（External API 的设备/资源/能力问答依赖）

- ~~**D2** derive_claims FAIL 化（RESOURCE-RESOLUTION-01）~~ ✅ **CLOSED @ p07c-runtime-state (2026-08-31)**: preflight Stage3 三态 Resolution, 设备无派生 input 资源 ⇒ FAIL
- ~~**D4** PortAvailability 精确化（端口级 direction/capability/availability）~~ ✅ **CLOSED @ p07c-runtime-state**: 端口级 (port_id 精确匹配+方向 / None ⇒ ≥1 Input 端口), 镜像 materialize 冻结语义
- ~~**D5** IdentityBinding 实查（IDENTITY-BINDING-01）~~ ✅ **CLOSED @ p07c-runtime-state**: `is_production_grade()` (HIGH+精确匹配/ManifestVerified), preflight+create 双侧实查
- ~~**D6** BACKEND-CAPABILITY-01（真实能力探针 + 硬性判定）~~ ✅ **CLOSED @ p07c-runtime-query (2026-08-31)**: capability projection (DeviceCapabilities→DeviceCapabilitiesSummary) + preflight 硬判定 (Unsupported ⇒ FAIL / Unknown ⇒ WARN 不臆造 / Supported ⇒ Pass); 真机 DeviceCapabilities 未探测 → Unknown 合法
- **D6 演进项**（非阻塞）: 真实 BMD SDK 深度能力探针（当前用 DeviceCapabilities 投影；SDK 深探针属 Provider 后续演进）——按终审"closure ≠ forever"原则登记

### 🟡 可延后（不阻塞 Canonical semantic / External API）

- D1（LifecycleJournal）、D3（per-claim TTL）、D7（OnceLock 简化）、D10（Session 内多 Pipeline）
- D9（create 幂等键）→ 移交 0.7C Command Contract 内解决（Idempotency 环节）

### 优先级调整

- **D11 Clock Observation Timeline：优先级上调**（广播时钟天然是时间序列——Locked→Lost→Recovered 是事件流；Clock 策略阶段的前置）
- **D8 EventSink 解耦**：与 0.7D Event Projection 同期考虑

### 新登记 D13（0.7B-2C 终审）

| # | 债务 | 说明 | 目标阶段 |
|---|------|------|----------|
| D13 | **`observe_transitional` debug_assert 弱约束**：`CanonicalTimecode::observe_transitional` 用 debug_assert 校验 presence——release build 不强制；应 Result 化或强类型拆分 API | P1 semantic hardening；**不单独开 change**，随 timecode 下一触碰点处理 | timecode 后续 |

## 0.7C 系列终审追加登记 (2026-08-31)

| # | 债务 | 说明 | 目标阶段 |
|---|------|------|----------|
| D14 | **Runtime Snapshot Consistency**：`runtime_state()` 是各源独立观测的拼合 snapshot，非事务一致 | 需定义 source observation time / state version / 一致性语义；已作为契约注释标注在 CanonicalRuntimeState | Runtime Query 后续 |
| D15 | **Media Flow Cardinality**：`PortId ≠ Media Stream`——一 Port 可对应 0/1/N flows | audio 多轨/timecode/metadata 进入后必须显式建模；已作为契约注释标注在 PortMediaSemantics（Vec 结构已避免过度限制） | 0.7B Audio 扩展/后续 |

## 失败矩阵覆盖现状（SESSION-RT-01 / RESOURCE-RT-01）

Preflight✓ Reserve✓ Lease(partial)✓ Binding/Materialize✓ Allocate(partial×multi)✓ Start✓ Stop(fail×single/multi)✓ Close✓ Double-start/stop✓ Tick/Expiry✓ Resource-conflict✓ Multi-device✓。
**未覆盖（登记不阻塞）**：Backend unavailable at create、provider disappeared mid-run（依赖真实硬件故障注入，随 0.7B 硬件验收扩展）。
