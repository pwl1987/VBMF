---
comet_change: p07c-event-projection
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-31-p07c-event-projection
status: final
---

# Design Doc — p07c-event-projection（Phase 0.7C-6: Event Projection Foundation + D8 EventSink Decoupling）

> open design.md §1-§5 实现级细化。锚点：终审 0.7C-5 Gate（Runtime→Event→Projection 生产边；与 External API 分离）+ probe 报告 `docs/superpowers/reports/2026-08-31-p07c6-architecture-probe.md`（四问/Q4 四语义基线）。

## 1. events.rs 增量（sink trait）

```rust
/// 事件出口抽象 (D8) — 生产者只依赖本 trait; 契约: emit 永不阻塞、永不失败
/// (满则两级丢弃+计数 — probe Q4 丢失语义零偷改)。
pub trait RuntimeEventSink: Send + Sync {
    fn emit(&self, ev: RuntimeEvent);
}
impl RuntimeEventSink for RuntimeEventLog {
    fn emit(&self, ev: RuntimeEvent) { self.push(ev); }
}
```

## 2. Supervisor 收窄（supervisor.rs）

```rust
pub struct Supervisor {
    policy: RestartPolicy,
    states: HashMap<Uuid, Status>,
    sink: Arc<dyn RuntimeEventSink>,   // 替换原 events: RuntimeEventLog
}
impl Supervisor {
    pub fn new(policy: RestartPolicy, sink: Arc<dyn RuntimeEventSink>) -> Self;
    // ingest(): mapper 后 → self.sink.emit(ev)
    // report_failure(): PipelineFault/HealthChanged → self.sink.emit(...)
    // report_recovered()/escalate(): HealthChanged → self.sink.emit(...)
    // 删除: events 字段、record()、drain_events()、pending_events()
    //   (probe Q3: 三 API 生产代码零调用者; 唯一调用在本文件测试 — 同步改)
    // 决策逻辑/状态机/circuit breaker/退避: 零改动
}
```

Supervisor 测试更新模式：`let log = Arc::new(RuntimeEventLog::new()); let mut s = Supervisor::new(policy, log.clone()); ... log.drain()` 替代 `s.drain_events()`。

## 3. SessionManager 接线（session.rs）

- 字段：`events: Arc<dyn RuntimeEventSink>`（构造参数，位置：supervisor 参数之后）。
- `emit()`：`self.sup.lock().unwrap().record(ev)` → `self.events.emit(ev)`（14 个发射点零语义变更——只换出口）。
- 夹具波及（机械）：`world()` ×3（command.rs / idempotency.rs / error_model.rs 测试）+ session.rs 测试 + main.rs 组合点 ×2——统一模式：

```rust
let log = Arc::new(RuntimeEventLog::new());
// supervisor 参数: Arc::new(Mutex::new(Supervisor::new(policy, log.clone())))
// session 参数:    log.clone() as Arc<dyn RuntimeEventSink>
// 断言/gate: 用 log.clone() 的 drain()
```

session.rs 内部测试若经 supervisor 查事件 → 改经 log。watchdog tick 的 report_failure 调用方零变更。

## 4. event_projection.rs（新）

```rust
/// 事件流只读投影 — 字段仅由消费语义需要驱动 (组合非展开, 禁万能 struct)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventProjection {
    pub total: usize,                                   // 输入事件数
    pub kind_counts: BTreeMap<String, usize>,           // kind() → 次数 (稳定序)
    pub session_states: BTreeMap<String, String>,       // session → 最后 SessionStateChanged.to
    pub session_failures: BTreeMap<String, usize>,      // session → SessionFailed 次数
    pub has_critical: bool,                             // is_fault() 存在
}
pub fn project(events: &[RuntimeEvent]) -> EventProjection;   // 纯函数: 零副作用/确定性
```

Session 变体字段：SessionCreated/SessionStateChanged/SessionFailed/SessionFailed 携带 session_id（String）——project 内 match 提取；其余事件只进 kind_counts/total/has_critical。

## 5. 测试矩阵（evt_proj_rt_01_*，feature=mock）

1. `vocabulary_snapshot` — 14 kind() 快照零改动回归。
2. `project_is_pure_and_fifo` — 确定性（两次相等）+ 投影演进序=输入序 + total/kind_counts 精确。
3. `loss_semantics_visible` — 既有两级丢弃测试回归 + 满表丢最旧后投影反映的是 drain 所见（drop 计数在 log，不伪造进投影）。
4. `duplicate_tolerant` — 双发同一事件 kind_counts×2、投影不崩（重复容忍）。
5. `decoupled_single_table` — Simulation：mgr.create/start + sup.report_failure 交替 → log.drain() 含 SessionCreated/SessionStateChanged/LeaseGranted/PipelineFault 且**发射序**保持；Supervisor 无自有残留（编译级 API 缺失）。
6. `projection_failure_isolation` — drain 后投影（事件流空不受影响）；project 无 expect/panic 路径（纯函数）。
7. 真机 EVENT-PROJECTION-RT-01 — SESSION_LIFECYCLE 生命周期完成后：组合根 log drain → project → `EVENT-PROJECTION-RT-01 total=? session_states=? session_failures=? has_critical=?`；回归 SESSION/RESOURCE/IDEMPOTENCY/ERROR-MODEL-RT-01 + COMMAND-CONTRACT-RT-01。

## 6. 触碰面清单

events.rs（+trait/impl）/ supervisor.rs（收窄+构造）/ session.rs（emit 接线+构造+夹具）/ event_projection.rs（新）/ main.rs（组合根+gate）/ 三处测试 world()。**零触碰**：resource.rs / lease.rs / pipeline.rs / preflight.rs / runtime_state.rs / runtime_query.rs / command.rs 语义 / idempotency.rs 语义 / error_model.rs 语义（后三者仅 world() 夹具构造参数机械更新）。
