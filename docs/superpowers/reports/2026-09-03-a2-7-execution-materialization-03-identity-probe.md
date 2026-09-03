# A2-7-03 — Runtime Failure Fact → Custody 生产接线（身份 SoT 反推 + 实现）

> Status: **ACCEPTED WITH REQUIRED FOLLOW-UP / NOT CLOSED**（终裁三段式:
> Identity Probe **CLOSED** / Custody Bridge Implementation **CLOSED** /
> Production Runtime Connection **DEFERRED TO 04**）
> Authority: A2-7-02 三轮终裁 §8（CLOSED；03 主任务 = 真实生产接线 + 反推
> PipelineHandle↔Uuid 身份 SoT，不凭空设计 mapping table）
> Date: 2026-09-03 · Base: `22e5e6c`

---

## 0. 终裁修正记录（2026-09-03 复核，§1-§7 要点）

1. **mapping 表表述收紧**：§1.2 "不需要也不应建立映射表" 的正确含义 =
   **"No new mapping table / No second identity registry"**——Session 已存
   `SessionInput{device_id, handle}` 关联（属 Session/Execution 生命周期），
   并非系统无 Handle↔Device 关联；未来 execution attribution 优先从
   Session→SessionInput 建 correlation，禁建第二张全局身份表（此点终裁
   明确批准）。
2. **`PipelineFault.pipeline` = Legacy/misnamed field（债务登记）**：当前
   实现里承载 device identity 是**事实**，但**不得升格为"本来就应该
   DeviceId"**——同 enum 内 `SourceMaterialized.pipeline` = Pipeline
   identity（`Uuid::new_v5(nil, plan)` 物化身份），**同字段名同类型双语义
   = Canonical Event Contract ambiguity**（CANONICAL_IDENTITY.md 已分
   DeviceId/PipelineId/SessionId 三 scope）。类型级修正留 **Event Contract
   cleanup / V0.3**（本 change 不蔓延）。
3. **`FailureObservation.pipeline_id` 标记 legacy event-field correlated
   identity（= DeviceId in current RuntimeEvent implementation）**——防未来
   开发者误读为 PipelineHandle/PipelineId；类型级重命名同留 V0.3。
4. **三身份语义分层记档**（终裁 §5）：DeviceId=Supervisor fault
   attribution/hardware ownership · PipelineHandle/PipelineId=Execution
   instance/backend lifecycle · SessionId=Orchestration lifecycle——禁一个
   Uuid 字段兼任三者；未来多 Pipeline/多输入下 `PipelineFault.pipeline=
   DeviceId` 只能表达"哪个设备维度需 Supervisor 决策"，不能表达"哪个
   Pipeline 实例失败"（当前 Supervisor 最小模型下可行，精确 attribution
   属后续）。
5. **Bridge 尚无生产调用者（03 未 CLOSED 的核心原因）**：真实生产故障链
   现状 = GStreamer ERROR → watchdog → `Supervisor::ingest` → mapper 产
   `PipelineFault(nil)` → 桥**拒收** → Supervisor recovery 产 RESTART_ECHO
   → 桥**再拒收**——即真实故障**尚未经桥进入 Custody**。桥已写好（中段），
   首尾未闭合；闭合属 04。

## 0'. A2-7-04 进入条件（终裁冻结）

```text
SessionManager.create/start → Backend.instantiate/start →
PipelineHandle + SessionInput{device_id, handle} →
Watchdog real bus observation → RuntimeEvent →
event consumer/drain → observations_from_events() →
attribute_failures(device_id, ...) → MasterJoinInput →
join() → ProgramMaster
```

**04 第一验收重点 ≠ "ProgramMaster 必须 ACCEPTABLE"**，而是：
真实故障 → 正确 Device correlation → 他设备零污染 → Supervisor echo 不
重复计故障 → Custody 收到**恰一次**真实 failure fact → Join 正确得到
FAILED。此链跑通 = Runtime→Event→Custody→Join 真闭环。

---

## 1. 身份 SoT 反推（现有真实代码与生命周期，非新设计）

### 1.1 `PipelineFault.pipeline: Uuid` 的值来源全量清点

| emit 路径 | `pipeline` 值 | 实锚 |
|---|---|---|
| `Supervisor::report_failure(&handle)` | `*handle` = **device_id**（supervisor.rs L38 注释原文："Supervisor 决策句柄 = 设备维度，`register`/`report_failure` 均以 device_id 注册"；bootstrap L118 `register(d.device_id)`） | supervisor.rs L195-200 |
| watchdog `report_failure(&device_uuid)` | 同上（watchdog 的 `device_uuid` 参数 = gate/main 装配点传入的设备 canonical 身份；gates 注释 L164-165："与 IdentityResolved 的 device_id 同源"） | watchdog.rs L205 + gates/session_lifecycle.rs L160-165 |
| `Supervisor::ingest`（上游 mapper） | **`Uuid::nil()`**（未归属——DefaultRuntimeEventMapper 产的上游故障无身份，fault_trigger 谓词显式匹配 nil 容忍） | events.rs L181-186 + supervisor.rs L39-40 |
| echo 排除 | `report_failure` Restart 路径自发的 PipelineFault = 决策回声（`RESTART_ECHO_SUMMARY`），非新故障事实 | supervisor.rs L41-42 |

### 1.2 结论：`PipelineFault.pipeline` 的真实语义 = **device_id（设备 canonical 身份）**

**不是** PipelineHandle(u64) 的映射——两者是不同层的身份，且**当前代码已有
清晰分工**：
- `PipelineHandle(u64)` = Backend 执行实例句柄（instantiate 产物，instantiate/
  start/stop/recover 生命周期单位）；
- `PipelineFault.pipeline: Uuid` = **设备维度决策身份**（Supervisor 注册/
  决策/回声排除全部按 device_id）。

**因此 Custody 的 `pipeline_id` 语义修正为 `device_id`（设备身份）**——这
不是新 mapping，是**读出现有代码的真实语义**：22e5e6c 的 `pipeline_id`
命名承自事件字段名，但其语义锚 = 设备身份。两身份**不需要也不应建立**
Handle↔Uuid 映射表（supervisor 从未需要；watchdog 持两者但按各层使用）。

### 1.3 多输入事实（Alpha-1）

`MediaSession.inputs: Vec<SessionInput{device_id, handle}>`（每管线句柄表）；
`MediaSession.pipeline` = 首输入兼容字段。诊断 watchdog 当前只 spawn 一个
（首设备）。Custody 快照天然**按设备身份**归因——多输入时每设备各自
custody 周期（本刀不实现多 Custody 管理，语义已对齐）。

## 2. 生产接线实现（本刀交付）

### 2.1 `custody.rs` 增生产桥（纯函数，零运行时接线面）

```rust
/// 从 RuntimeEvent 流提取 FailureObservation（生产桥——Runtime failure
/// fact → Custody 归因输入的唯一转换点）。
pub fn observations_from_events(events: &[RuntimeEvent]) -> CustodyObservations
```

- 只提取 `PipelineFault{pipeline, summary, retryable}`；
- **回声排除**（summary == RESTART_ECHO_SUMMARY 不入——Supervisor 决策
  回声非新故障事实，与 fault_trigger_from_events 同律）；
- `Uuid::nil()`（mapper 未归属上游故障）**不吸收**——无身份证据不归因
  （fail-closed；与 supervisor nil 容忍不同场景：Supervisor 需容忍以驱动
  决策，Custody 需拒绝以避免误归因到真实设备——nil == 任何 device_id
  匹配失败即零污染，但显式跳过表达意图）；
- HardwareFault/SessionFailed/HealthChanged/ClockLost 不提取（终裁维持）；
- avsync 恒 Unknown（OQ-4 deferred 维持）。

### 2.2 命名修正（doc 级，零 API 破坏）

`FailureObservation.pipeline_id` doc 更正为"设备 canonical 身份（=
`RuntimeEvent::PipelineFault.pipeline` 的真实语义——Supervisor 决策句柄）"；
字段名保留 `pipeline_id`（承事件字段名，避免无关 churn；语义注释对齐）。

## 3. No-Build Gate

不动：events.rs/supervisor.rs/watchdog.rs/Join/Runtime 契约/session.rs；
不建 Handle↔Uuid mapping；不做多 Custody 管理/Query/Transport/A2-8。

## 4. 证据清单

supervisor.rs L36-42（归属注释）/L161-178（ingest/register）/L190-215
（report_failure emit）· events.rs L166-187（DefaultRuntimeEventMapper nil）·
watchdog.rs L30/L163-171/L190/L205（device_uuid 链）· bootstrap.rs L118 ·
gates/session_lifecycle.rs L160-166 · bin/media-agent.rs L106-115/L404-417 ·
session.rs L186-197（多输入句柄表）。
