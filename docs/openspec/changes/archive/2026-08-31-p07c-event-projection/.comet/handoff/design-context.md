# Comet Design Handoff

- Change: p07c-event-projection
- Phase: design
- Mode: compact
- Context hash: e87caecc8917d7c101373fb552282b98659ff54ce58fe06ba65ba347a316df32

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07c-event-projection/proposal.md

- Source: docs/openspec/changes/p07c-event-projection/proposal.md
- Lines: 1-28
- SHA256: 56cb67265c2a17176cf543c112c4f4a3473647f0a416c396cc9909c03e2e026a

```md
# Change: Phase 0.7C-6 — p07c-event-projection（Event Projection Foundation + D8 EventSink Decoupling：Runtime→Event→Projection 第一条生产边）

## Why

0.7C-5 Merge Gate PASS（master=`a6c5925`）。终审裁定：**0.7C-6 不是"增加几个事件类型"，而是第一次真正建立 `Runtime Event → RuntimeEventSink → RuntimeEventLog → Projection` 这条生产边**——与 0.7C-1 的 `Canonical → Runtime State` 边合起来才构成完整 Runtime Architecture。D8（Supervisor 既是决策者又是事件表持有者的职责倒置）是本阶段核心工作。**Event Projection（内部架构边界）与 External API（外部契约边界）严格分离，绝不合并**。开工前置：只读 Architecture Probe 已完成（`docs/superpowers/reports/2026-08-31-p07c6-architecture-probe.md`，四问：14 变体词表/生产点全景、Supervisor 三重职责病灶、**零生产消费者**、四语义基线）。

## What Changes

- **D8 解耦（supervisor.rs / session.rs / main.rs 组合根）**：
  - **`RuntimeEventSink` trait（events.rs）**：`fn emit(&self, ev: RuntimeEvent)`——事件出口的抽象；`RuntimeEventLog` 实现之（现有 push 语义原样）。
  - **组合根唯一 log 实例**：main.rs 构造 `Arc<RuntimeEventLog>`，同时注入 SessionManager（新参数）与 Supervisor（构造注入）——**单表单锁**，事件不分流。
  - **SessionManager**：`emit()` 改走注入的 sink（**不再穿 Supervisor**——`sup.lock().record()` 路径删除）；`sup` 字段只剩决策职责。
  - **Supervisor 收窄**：删自有 `events` 字段与 `record`/`drain_events`/`pending_events` API（**probe 证实零生产调用者，收窄安全**）；决策事件（report_failure/report_recovered/escalate）与 `ingest` 归一化改经注入 sink 发射；测试同步更新。Supervisor 回归纯决策引擎（0.6 HARD RULE 更纯）。
- **Event Projection Foundation（`src/event_projection.rs` 新）**：
  - **`project(events: &[RuntimeEvent]) -> EventProjection` 纯函数**——从事件流构建只读快照：per-session 最新状态与 failed 计数、fault/critical 存在性、事件总数（含 kind 分布）。**零字段万能 struct 禁令延续**：投影字段仅由四语义测试需要驱动，组合非展开。
  - **四语义测试锁定（probe Q4 为基线，零偷改）**：顺序（FIFO 投影序=发射序）；丢失（cap 溢出两级丢弃语义不变 + drop 计数可见；重复发射不破坏投影）；重复（同事件双发投影容忍）；projection failure（纯函数，不改事件流、无副作用——drain 后投影是消费侧行为）。
- **门禁 EVENT-PROJECTION-RT-01（三层）**：Unit（词表快照 14 变体零改动回归/投影纯函数/四语义）；Simulation（解耦后 SessionManager+Supervisor 双生产者单表汇聚：事件按发射序全量落表、Supervisor 无自有残留）；Hardware（真机 SESSION_LIFECYCLE gate 段追加投影输出：生命周期事件 drain→project→打印快照——消费接线实证）。
- **CI**：测试并入现有矩阵。

## Capabilities

（`skip_specs: true`——SoT 为终审 0.7C-5 Gate（Event Projection=Runtime→Event→Projection 生产边；与 External API 分离）+ 债表 D8 + probe 报告 Q4 四语义基线。）

## Impact

- 编译：五套 feature 不回退；event_projection.rs 零 vendor 依赖。
- 受影响：`events.rs`（+sink trait+impl）、`supervisor.rs`（删表+构造注入+测试）、`session.rs`（emit 接线+构造签名）、`main.rs`（组合根+gate 段）、新 `event_projection.rs`；Phase Map（0.7C-6 行）；债表（**D8 → CLOSED**）。既有事件发射点（session 14 处）**零语义变更**——只换出口。
- **明确不做**：External API/REST/RPC transport；Health Reducer 完整实现（watchdog tick 语义不动）；Supervisor 改事件驱动决策（watchdog 演进）；事件词表变更/零生产 4 项点亮（登记演进）；事件持久化/跨进程总线（Kafka/NATS 禁令延续）；PreflightFailure 粒度 P1（终审裁定 deferred，不顺手改）；adapter 专属 RuntimeEventMapper 接线。

```

## docs/openspec/changes/p07c-event-projection/design.md

- Source: docs/openspec/changes/p07c-event-projection/design.md
- Lines: 1-89
- SHA256: 04af426106ea28cc771834ef01c1f3d00b11dcb25d0d248fbd486e12dc863b68

[TRUNCATED]

```md
# Design: Phase 0.7C-6 — p07c-event-projection

## 0. 终审裁定落点 + probe 结论引用

| 终审/probe 裁定 | 设计落点 |
|---|---|
| 0.7C-6 = 建立 `Runtime Event → Sink → Log → Projection` 生产边（非加事件类型） | §1 sink trait + §3 投影 Foundation |
| D8 是核心工作（Supervisor 决策者兼事件表持有者=职责倒置） | §2 Supervisor 收窄（probe Q2 三重职责→纯决策） |
| **Event Projection（内部边界）≠ External API（外部边界），不合并** | §4 明确不做清单首位 |
| 四语义零偷改（probe Q4 基线） | §3.2 测试矩阵逐项 |
| probe：drain_events/record 零生产消费者/调用者（除 supervisor 测试） | §2 API 收窄安全性论证 |

## 1. `RuntimeEventSink`（events.rs 增量）

```rust
/// 事件出口抽象 (D8) — 生产者只依赖本 trait, 不依赖 Supervisor。
/// 契约 (probe Q4): emit 永不阻塞、永不失败 (满则按两级丢弃策略+计数)。
pub trait RuntimeEventSink: Send + Sync {
    fn emit(&self, ev: RuntimeEvent);
}

impl RuntimeEventSink for RuntimeEventLog {
    fn emit(&self, ev: RuntimeEvent) { self.push(ev) }   // push 语义零改动
}
```

## 2. D8 解耦（单表双生产者）

**组合根（main.rs）**：

```rust
let event_log = Arc::new(RuntimeEventLog::new());          // 唯一实例
let supervisor = Supervisor::new(RestartPolicy::default(), Arc::clone(&event_log) as Arc<dyn RuntimeEventSink>);
let mgr = SessionManager::new(..., Arc::new(Mutex::new(supervisor)), event_log, ...);
```

- **SessionManager**：新字段 `events: Arc<dyn RuntimeEventSink>`（构造参数）；`emit()` 改 `self.events.emit(ev)`——**删除 `sup.lock().record()` 穿越路径**（session.rs:279）。`sup` 字段只剩 `report_failure` 等决策调用（watchdog tick 既有路径零变更）。
- **Supervisor**：删 `events: RuntimeEventLog` 字段 + `record`/`drain_events`/`pending_events` 三 API（probe Q3 证实零生产调用者——唯一调用在 supervisor 自己的测试）；`new(policy, sink: Arc<dyn RuntimeEventSink>)`；`report_failure`/`report_recovered`/`escalate`/`ingest` 内部 `self.events.push(...)` → `self.sink.emit(...)`。**决策逻辑/状态机/退避零改动**。
- **事件不丢失核对**：解耦前 session emit 与 supervisor 决策事件都进同一张 supervisor 内表；解耦后两者进同一张组合根表——**事件集合不变，仅出口归属变化**。顺序保持：两生产者经 `RuntimeEventLog` 同一把 Mutex 串行（全局 FIFO 维持）。
- **gate/测试夹具波及**（机械性）：`world()` 构造点 ×3（command/idempotency/error_model 测试）+ session 测试 + main.rs ×2——补传 log 参数；session 测试中经 `mgr.events()` 查询断言的（若有）改从组合根 log drain。SessionManager 是否暴露 `event_log()` 访问器：**提供**（`pub fn event_log(&self) -> Arc<RuntimeEventLog>`？——不，SessionManager 持的是 `Arc<dyn RuntimeEventSink>`（trait 对象，无 drain）。测试/gate 需要 drain——**另存一份 Arc<RuntimeEventLog> 于调用侧**（构造时同源克隆），SessionManager 不提供 drain 面eria（保持生产者角色纯净）。夹具模式：`let log = Arc::new(RuntimeEventLog::new()); let mgr = ...((log.clone() as Arc<dyn _>)); let supervisor = ...(log.clone())`。

## 3. Event Projection Foundation（`src/event_projection.rs` 新）

### 3.1 投影类型与纯函数

```rust
/// 事件流只读投影 — 仅由消费语义需要的字段组成 (组合非展开, 禁万能 struct)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventProjection {
    /// 事件总数 (投影输入长度)。
    pub total: usize,
    /// 各 kind 计数 (canonical kind 字符串 → 次数; BTreeMap 稳定序)。
    pub kind_counts: BTreeMap<String, usize>,
    /// 每 session 最新状态 (session_id → 最后一条 SessionStateChanged.to)。
    pub session_states: BTreeMap<String, String>,
    /// SessionFailed 计数 (每 session)。
    pub session_failures: BTreeMap<String, usize>,
    /// 存在过 Critical 事件 (is_fault())。
    pub has_critical: bool,
}

/// 纯函数: 只读切片 → 投影。不改事件流、无副作用、确定性 (同输入同输出)。
pub fn project(events: &[RuntimeEvent]) -> EventProjection;
```

字段依据：四语义测试需要（total/kind_counts 验证顺序与完整性；session_states 验证最新态投影；session_failures 验证失败汇聚；has_critical 验证故障可见）——无多余字段。`RuntimeEvent` 需提取 session id 的事件仅 Session* 变体（其余无 session 维度）。

### 3.2 四语义测试矩阵（evt-proj-rt-01_*）

| 测试 | 语义 | 断言 |
|---|---|---|
| `vocabulary_snapshot` | 词表 | 14 变体 kind() 快照零改动回归（解耦不碰词表） |
| `project_is_pure_and_fifo` | 顺序+纯度 | 同输入两次投影相等；投影序=输入序（kind_counts 与 session_states 演进序）；入参切片不被修改（借用语义） |
| `loss_semantics_visible` | 丢失 | log 满时两级丢弃行为不变（Observation 挤出/Critical 保护/drop 计数）——**events.rs 既有测试回归 + 投影内 drop 计数不进入**（drop 计数在 log 上，投影输入是 drained 事件——丢失由计数器暴露，非投影伪造） |
| `duplicate_tolerant` | 重复 | 同事件双发投影两次计数——投影状态一致不崩（重复容忍锁定） |
| `decoupled_single_table` | D8 | Simulation：SessionManager 生命周期（create→start→stop 失败注入→close）+ supervisor report_failure 交替调用 → 组合根 log drain 全量含两类事件、**发射序**保持；Supervisor 自身无事件残留 API（编译级：record/drain_events 不存在） |
| `projection_failure_isolation` | failure | 投影函数消费 &[RuntimeEvent]——零副作用（drain 后投影，事件流已空不受影响；投影失败不存在路径——纯函数无 panic 点，expect 零使用） |
| 真机 EVENT-PROJECTION-RT-01 | Hardware | SESSION_LIFECYCLE 生命周期完成后：`event_log.drain()` → `project()` → 打印 `EVENT-PROJECTION-RT-01 total=? session_states=? failures=? critical=?`（消费接线实证）+ 回归全部门禁 |

## 4. 不做（终审分离裁定 + probe 演进项）

```

Full source: docs/openspec/changes/p07c-event-projection/design.md

## docs/openspec/changes/p07c-event-projection/tasks.md

- Source: docs/openspec/changes/p07c-event-projection/tasks.md
- Lines: 1-50
- SHA256: c6f2647d6af3c79a4b66c3ba171ebb488b2d1976fc19012d7d34d5b07256c3c6

```md
# Tasks: Phase 0.7C-6 — p07c-event-projection

四栏纪律：`Contract: 已有(引用冻结文档节号) / Implementation / Verification / Gate`。

## 1. D8 解耦（design.md §1/§2）
- [ ] RuntimeEventSink trait + RuntimeEventLog impl（push 语义零改动）
      Contract: 终审 0.7C-5 Gate（D8 核心工作；目标形态图）+ 债表 D8
      Implementation: events.rs 增量
      Verification: `evt_proj_rt_01_vocabulary_snapshot`（词表零改动回归）+ 既有 log 语义测试回归
      Gate: EVENT-PROJECTION-RT-01 Unit 层
- [ ] 组合根单表：main.rs 唯一 Arc<RuntimeEventLog> 注入 SessionManager 与 Supervisor
      Contract: probe Q4 顺序语义（单表单锁全局 FIFO）
      Implementation: main.rs 组合根
      Verification: `evt_proj_rt_01_decoupled_single_table`
      Gate: EVENT-PROJECTION-RT-01 Simulation 层
- [ ] SessionManager emit 直连 sink（删除 sup.lock().record 穿越）+ Supervisor 收窄（删 events 字段与 record/drain_events/pending_events；决策事件经注入 sink）
      Contract: D8 职责倒置病灶（probe Q2）；收窄安全性=probe Q3 零生产调用者
      Implementation: session.rs + supervisor.rs（决策逻辑零改动）
      Verification: 同上 Simulation 测试 + supervisor 既有决策测试全绿（更新构造）
      Gate: EVENT-PROJECTION-RT-01 Simulation 层

## 2. Event Projection Foundation（design.md §3）
- [ ] project(events) 纯函数 + EventProjection 组合式字段（禁万能 struct）
      Contract: 0.7 红线（Observation≠Configuration——投影只读快照绝不写回）
      Implementation: event_projection.rs
      Verification: `evt_proj_rt_01_project_is_pure_and_fifo` + `evt_proj_rt_01_projection_failure_isolation`
      Gate: EVENT-PROJECTION-RT-01 Unit 层
- [ ] 四语义锁定：顺序/丢失（既有两级丢弃回归）/重复容忍/failure 隔离
      Contract: probe Q4 基线（终审裁定"零偷改"）
      Implementation: 测试矩阵
      Verification: `evt_proj_rt_01_{loss_semantics_visible, duplicate_tolerant}`
      Gate: EVENT-PROJECTION-RT-01 Unit/Simulation 层

## 3. 真机与回归
- [ ] gate 段投影输出（生命周期后 drain→project→打印）+ 全门禁回归
      Contract: PHASE_IMPLEMENTATION_MAP §3（Event Projection 项）
      Implementation: main.rs gate 段
      Verification: 盒上 VBMF_SESSION_LIFECYCLE=1 真机跑
      Gate: EVENT-PROJECTION-RT-01 Hardware 层 + SESSION/RESOURCE/IDEMPOTENCY/ERROR-MODEL-RT-01 回归
- [ ] 五套 feature 编译不回退 + 盒上全矩阵
      Contract: CI 七 checks 口径
      Verification: p07_verify.sh 全绿
      Gate: PR required checks

## 4. 文档与收尾
- [ ] 债表 D8 → CLOSED（引解耦证据）；Phase Map 0.7C-6 行 COMPLETE；0.7C 下一项 = External API
      Contract: PHASE_IMPLEMENTATION_MAP=唯一 SoT；债务 closure≠forever
      Verification: 文档对账
      Gate: verify
- [ ] verify（0 CRIT/0 IMP 目标）→ archive → PR → merge → tag phase-0.7C6-event-projection → 删分支

```
