# 0.7C-6 Preflight / Architecture Probe — Event Projection + D8 EventSink Decoupling（只读调查）

- 日期：2026-08-31 · 基线：master=`a6c5925`（0.7C-5 baseline）
- 性质：只读探针（零代码改动）——回答终审 0.7C-5 Gate 指定的四件事，作为 p07c-event-projection design 输入。

## Q1 当前 RuntimeEvent 的真实生产事件

**词表**：14 变体（0.6D 11 个 + 0.7A 增 SessionCreated/SessionStateChanged/SessionFailed；`events.rs` kind()/severity()/is_fault() 完整）。

**真实生产点（grep `RuntimeEvent::` 构造 + `emit`/`record`/`ingest` 调用）**：

| 生产者 | 事件 | 触发路径 |
|---|---|---|
| `session.rs` `emit()`（279 行 → `sup.lock().record(ev)`） | SessionCreated ×1（create 成功）、SessionStateChanged ×4（start/stop/phase 迁移）、SessionFailed ×7（各失败回滚点：create 失败/materialize/allocate/start/stop 失败/close 非法等）、LeaseGranted ×1、SourceMaterialized ×1、ResourceAllocated ×1 | SessionManager 生命周期 |
| `supervisor.rs` `report_failure` | PipelineFault{retryable=true}（Restart 裁决）/ HealthChanged{→manual_required}（Escalate 裁决） | 健康探针报告失败 |
| `supervisor.rs` `report_recovered` / `escalate` | HealthChanged{recovered} / HealthChanged{manual_required} | 恢复/强制升级 |
| `supervisor.rs` `ingest`（DefaultRuntimeEventMapper 硬编码） | AmbiguousIdentity / HardwareFault / PipelineFault（按观测关键字归类；无故障语义观测不伪造） | vendor 观测归一化入口 |

**零生产词表项（4 个，登记不点亮——点亮属接线演进非解耦必需）**：`IdentityResolved`、`SignalVerified`、`LoopbackVerified`、`ResourceReservationExpired`。

## Q2 `Supervisor::record` 当前承担的职责（D8 病灶确认）

Supervisor 三重职责混合（`supervisor.rs:108-262`）：

1. **重启决策引擎**：`policy`/`states`（circuit breaker + 指数退避）+ `report_failure`/`report_recovered`/`begin_restart`/`backoff`/`escalate`——这是它的本职工件（0.6 冻结，HARD RULE 只决策不执行）。
2. **唯一事件出口持有者**：自有字段 `events: RuntimeEventLog` + `record()`/`ingest()`/`drain_events()`/`pending_events()`——**决策者兼事件表持有者 = D8 职责倒置病灶**。
3. **vendor 归一化入口**：`ingest()` 硬编码 `DefaultRuntimeEventMapper`（Adapter 专属 mapper 机制未接）。

**调用关系**：`SessionManager.emit()`（session.rs:279）→ `sup.lock().unwrap().record(ev)`——SessionManager 的事件**必须穿过 Supervisor**（经其 Mutex）才能落表。

## Q3 真实依赖现有事件的消费者

**零生产消费者**。`drain_events`/`pending_events` 在 supervisor.rs 之外无任何调用（main.rs / health.rs / runtime_query.rs / runtime_state.rs 均无）。runtime_state 是**拉式聚合**（直接读 SessionManager 状态，非事件驱动）；health 靠 watchdog tick。事件流现状 = **生产已接线、消费未接线**（RPC/下游属 External API 阶段）。当前唯一"消费者"是 supervisor.rs 自己的单元测试。

（旁注：main.rs:1212 的 `dropped_bus_events()`/`clock_lost_events()` 是 pipeline 侧独立计数器，与 RuntimeEvent 流无关。）

## Q4 D8 解耦必须保持不偷改的四语义（现状 = 0.7C-6 测试锁定基线）

| 语义 | 现状（代码级事实） | 解耦红线 |
|---|---|---|
| **顺序** | 单 `Mutex<VecDeque>`：所有生产者（session emit / supervisor 决策 / ingest）经同一把锁串行 push_back，`drain(..)` 全排空 → **全局 FIFO**，生产顺序=投递顺序 | 解耦后仍单表单锁（组合根唯一 `Arc<RuntimeEventLog>`），不得引入多表分流或乱序消费 |
| **丢失** | **有界 + 两级丢弃是既有特性（P1-3）**：cap 1024；Observation 满时被挤出（找最旧 Observation 移除）、全 Critical 时新 Observation 直接丢弃；**Critical 永不被 Observation 挤出**（强推才挤最旧）；`dropped_observations`/`dropped_criticals` 计数器暴露、丢弃不静默 | 不得偷改成无界（内存风险）或静默丢（违反"丢弃不静默"）；drop 计数必须继续可见并可进入投影 |
| **重复** | 无去重：同事实双 emit = 双事件（事件≠命令，无幂等需求） | 投影端必须容忍重复（重复事件不得破坏投影状态）；不得偷偷加去重（改变语义） |
| **projection failure** | 生产者 push 永不阻塞、永不失败（满则丢弃+计数）；消费者 drain 与生产者互不阻塞 | 解耦后 sink 实现保持"发射永不阻塞生产者"；投影函数必须是纯函数（panic/失败不影响事件流） |

## D8 目标形态（终审 0.7C-5 Gate 指定）

```
Runtime Event（session/supervisor 决策/ingest）
      ↓ RuntimeEventSink（trait, 组合根注入）
RuntimeEventLog（唯一实例, 有界+两级丢弃+drop 计数）
      ├── Health Reducer（watchdog 语义不变, 后续演进）
      ├── Supervisor（纯决策引擎——失去事件表字段, 决策事件经注入 sink 发射）
      └── External Projection（External API 阶段）
```

## 0.7C-6 范围裁定输入（probe 结论）

- **做**：`RuntimeEventSink` trait + `RuntimeEventLog` 实现；组合根唯一 log 实例注入 SessionManager（emit 直连 sink，不再穿 Supervisor）与 Supervisor（决策事件经 sink，删自有 events 字段与 record/drain_events/pending_events API——零生产调用者，收窄安全）；`project(events)` 纯函数投影 Foundation + 四语义测试锁定；gate 段真机投影输出（消费接线实证）。
- **不做**：Health Reducer 完整实现（watchdog 语义不动）；External Projection/REST；事件词表变更/零生产项点亮；Supervisor 改事件驱动决策（watchdog 演进）；PreflightFailure 粒度 P1（终审裁定 deferred）。
- **红线**：事件顺序/丢失/重复/projection-failure 四语义零偷改（测试锁定基线=本报告 Q4）。
