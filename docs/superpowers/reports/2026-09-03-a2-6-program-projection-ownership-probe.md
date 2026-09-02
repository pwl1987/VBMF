# A2-6-00 — Runtime Projection SoT / Ownership Probe

> Status: `PROBE ONLY / NO CODE CHANGE`
> Authority: A2-5 终裁（A2-6 六刀链冻结 + 禁止捷径红线 + 八问必答）
> Date: 2026-09-03 · Change: a2-6-program-projection · Base: master `2166d25`
> 核心：先钉死"ProgramMaster 由谁拥有/从哪里产生"，再谈 projection。

---

## 1. 八问逐项证据

### Q1 — ProgramMaster Owner（谁拥有"当前 ProgramMaster"？）

**现状：零 owner。** `SessionManager` 私有字段全清点（session.rs L254-268）：
resources/leases/sup/backend/devices/bindings/registry/mode/tuning/sessions/
events/observation_revision/observation_lineage——**零 Program/MasterJoin 引用**
（`grep ProgramMaster|MasterJoin|program::` session.rs = 0）。`MediaSession`
字段（graphs/source_ports/output_ports/claims/lease/pipeline/outputs/inputs/
health）同样零。

候选状态（探针事实，非推荐）：
- A. SessionManager——现聚合根，但 A2-5 已锁 Program≠Runtime 事实（塞入=
  边界反向）；
- B. 独立 Program Runtime Owner——新类型，等价于给 Program Domain 一个
  Runtime 侧 custody（谁调用 join() 谁写结果）；A2-5-05 已证 `join()` 零
  生产调用者——**ProgramMaster 当前根本无处产生**；
- C. Channel/Control Plane——控制面未建（A4 线）；
- D. Production Orchestrator——V0.2 未定义，远期。
**交 OQ-1 裁决（本 probe 不预裁）**；探针关键事实：**任何 owner 选择都
必须先回答"join() 由谁在什么时机调用"——这是所有权问题的真正源头**。

### Q2 — 生命周期（何时生成/刷新？）

`join()` 生产调用者 = **零**（仅 master_join.rs 测试内 5 处）。三 Master
当前零生产 writer。即：**不存在"ProgramMaster 生命周期"，只存在未来
"谁推进三 Master stage/declaration 并调用 join()"的问题**。V0.2 线索：
三 Master stage 推进对应 Graph 处理节点事实（Normalize/Switcher/Composition
完成等）——**执行事实产生于 Execution/Watchdog 观测**，但 Program stage
是 Canonical 声明，写入者归属未定。**交 OQ-2 裁决**。

### Q3 — Snapshot 关系（与 CanonicalRuntimeState 同一 observation boundary？）

**不是，也不应绑**。`CanonicalRuntimeState` 装配点唯一 =
`SessionManager::runtime_state()` → `assemble()`（runtime_state.rs L160，
纯函数）；D14 `SnapshotObservation{revision,lineage,observed_at_ms}` 是
Runtime 观察域。Program Domain 语义快照若并入：
(a) 违 A2-4-04 红线"Event Projection 不成 Join"同构禁令（Runtime 不反向
重建 Program）；
(b) D14 swept/non-transactional 语义与 C′ 矛盾快照语义冲突（A2-4-05 终裁：
无 timestamp/revision 不足以判源——绑进 D14 等于给 C′ 矛盾强加时序解释）。
**倾向（待裁）：独立 snapshot 边界**；若未来需要关联，用投影层并列展示
（API 层并列两 snapshot），非存储层合并。

### Q4 — API 资源命名

`api_boundary.rs` 先例：ApiDevice/ApiPort/ApiResource/ApiSession/
ApiInputSummary/ApiCapability——命名从**消费语义**出发非内部类型名直译。
消费者问题（"节目语义"还是"运行状态"）未答——**A2-6-01 裁，命名清单
(ApiProgram/ApiProgramMaster/ApiProgramSnapshot/ApiChannelProgram) 均不
预设**。

### Q5 — join_result: None 投影

唯一合法语义 = "尚未形成 Join Result"（02 终裁）。**禁投影成
unknown/not_ready/failed/degraded**——四者各自有独立语义（R-A 语义不可
坍缩）。API 表达候选：null / 独立 wire 值 / 字段缺席——**A2-6-01 裁**
（wire 契约属 API Boundary 层）。

### Q6 — AVSync 出现位置

`AVSyncClassification` 唯一家 = master_join.rs（Join-side classification
input）。A2-6 若暴露：只能是 **Join classification input 的 API projection
透传**，绝不能投影为 Runtime Health（`AVSYNC_FAILED ≠ program.status=
failed`——§8.10 red 后须 classify_failure_domain，PLAYER 绝不切源）。
词消歧（Clock offset/drift）维持 A2-5-01 终裁。

### Q7 — video_failed/audio_failed 外露

**维持不暴露**（A2-5-05 终裁）。现状复核：`media_semantics` 在
api_boundary 测试中为空 vec（L517）——现有 API 零 media 语义暴露先例；
Runtime 本就持有 failed 事实，API 复制 = 三处表示。证据不足不加。

### Q8 — inconsistency 进 API

维持 A2-5-05 结论：**默认不直接暴露**；除非 API 明确出现"为什么
FAILED/DEGRADED"真实需求，再在 **API 层**形成用户语义（不是 Program
Domain 长 reason 字段）。

## 2. 禁止捷径红线（终裁 §九，A2-6 全程）

**禁**从 `SessionRuntimeState`/`GraphRuntimeIntent`/`CanonicalRuntimeState`
临时重建 VideoMaster/AudioMaster/MetadataMaster 再 compose——Runtime facts
反推 Program semantics = 边界反向（Program=是什么，Runtime=现在发生什么，
不可互推）。

## 3. 补充红线（本 probe 增补，待并入 01 裁定）

- **零生产触发点事实**：`join()`/三 Master writer 零——**任何 projection
  设计在"三 Master 谁写"裁决前都是空中楼阁**（OQ-2 是全链第一前置）；
- `assemble()` 唯一装配点不动；RuntimeQuery 纯读 allowlist（7 个查询方法
  + new 构造方法 = 8 项公开 surface）在
  A2-6-02 前零扩展；
- transport `/api/v1/runtime` 响应体（ApiQuerySnapshot）冻结不动——A2-6-03
  才接新端点（若有）。

## 4. Open Questions（交用户裁决）

| # | 问题 | 候选 | 倾向（非裁决） |
|---|---|---|---|
| OQ-1 | ProgramMaster owner | A SessionManager / B 独立 Program Runtime Owner / C Control Plane / D Orchestrator | 探针倾向 B（custody 型 owner，A4 线成型前）——但必须与 OQ-2 联合裁 |
| OQ-2 | join()/三 Master 写入时机 | session start 时 / 运行中事件驱动 / config apply / watchdog 观测回写 | **真前置**：V0.2 stage 事实源=处理节点完成观测；倾向"运行中事件驱动 + Owner 调用"——但执行事实→Canonical 声明的写入通道须裁（或显式 deferred 到 A2-7 执行面存在时） |
| OQ-3 | Snapshot 边界 | 独立 / 并入 D14 | 独立（证据 §1-Q3） |
| OQ-4 | API 资源命名与语义 | 节目语义 vs 运行状态；四个候选名 | A2-6-01 消费者裁 |
| OQ-5 | None 投影形态 | null / 独立 wire 值 / 缺席 | A2-6-01 裁（wire 层） |

Q6/Q7/Q8 终裁已给（透传禁 Health 化 / 不暴露 / 默认不暴露），无新问。

## 5. No-Build Gate

零 .rs diff；不改 RuntimeState/RuntimeQuery/transport/api_boundary/三
Master/Join/ProgramMaster；禁止捷径红线全程生效。

## 6. 证据文件清单

session.rs L187/L254-268（MediaSession.health/SessionManager 字段全清点）·
runtime_state.rs L122-160（SnapshotObservation/assemble）·
runtime_query.rs L36-78（7 个 Pure-Read 查询方法 + new 构造方法, 公开
surface 共 8 项）· api_boundary.rs L6-15/L37-160/
L517（禁令原文/资源族/零 media 投影）· transport.rs L195-246（端点冻结）·
master_join.rs（join() 零生产调用者）· program/mod.rs L7-9（Canonical/
Adapter 边界纪律）。

---

## 7. 用户终裁记录（A2-6-00 → A2-6-01 Gate，2026-09-03）

> 复核基准：master=2166d25 真实代码交叉核验。**A2-6-00 = APPROVED / CLOSED**
> （不按报告倾向"B + 立即落地"直接进实现）。

### 核心裁决：OQ-1/OQ-2 联合裁 = 逻辑定角色、工程暂不创建

- **OQ-1 = B（角色批准，实现 deferred）**：批准方向 =
  **Program Runtime Custody**（Runtime/Orchestration 侧独立角色，对当前
  Program Domain snapshot 负责持有与刷新：receives execution facts →
  advances domain declarations → invokes join() → publishes snapshot）。
  **A2-6 不创建**——`join()` 零生产调用者时建 Owner 只能是"空壳容器"
  （为了未来而设计）；等 A2-7 执行事实链建立后落地。
- **🔴 双禁令**：ProgramMaster 塞入 CanonicalRuntimeState 禁；SessionManager
  直接成为 owner 禁（两者都会把 Runtime aggregate 变成 Program Domain
  aggregate）。
- **OQ-2 = Deferred to A2-7（非无限期）**：生命周期终态 = **执行事实驱动**，
  但链路必须是 `Execution/Materialization → Execution Fact → Custody/
  Orchestration → advance/join() → snapshot`——**Watchdog 不是 ProgramMaster
  writer**（Watchdog=观察/恢复，禁止升级为 Program Domain writer）。
  正式表述："A2-5/A2-6 阶段不建立 ProgramMaster 生命周期写入实现；生产
  触发点延后至 A2-7 Materialization/Execution Fact 链建立后，由独立
  Program Runtime Custody 或等价 Orchestration boundary 负责；Watchdog/
  Event Projection 不直接成为 Program Domain writer。"
- **OQ-3 = 独立 snapshot（批准）+ 细则**："独立 snapshot" ≠ API 不能同时
  展示——API 响应可并列 `runtime_snapshot + program_snapshot`（**并列
  projection**），非存储/所有权层合并。
- **OQ-4 = Deferred to A2-6-01**（消费语义出来再裁命名）。
- **OQ-5 = 内部 CLOSED + wire Deferred**：内部 `None=尚未形成 Join Result`
  绝不变（≠UNKNOWN/NOT_READY/DEGRADED/FAILED）；null vs 字段缺席属 wire
  contract → A2-6-01。
- **Q6/Q7/Q8 原裁决全部批准**（透传禁 Health 化 / failed 不暴露防三处
  表示 / inconsistency 默认不暴露、需求出现在 API 层定义）。
- **事实修正**："allowlist 7 fn" → **7 个 Pure-Read 查询方法 + new 构造
  方法 = 8 项公开 surface**（本报告已改；不重开 00）。

### A2-6 全局边界图（终裁 §十）

```
Program Domain（六类型） ◄── owned/refreshed by ── Program Runtime Custody
        ▲                                             （角色已批, A2-7 落地）
        │ semantic types                        ▲ execution facts
        │                                              │
        └────────────────────────────── Execution/Materialization（A2-7）

Runtime State / Health·Watchdog / Event Projection ──┐
ProgramMaster ───────────────────────────────────────┴──→ Projection / API
```

**最重要一条**：Runtime State 不反推 ProgramMaster；Watchdog 不直接写
ProgramMaster；API 不负责制造 ProgramMaster。

### 下一步

**A2-6-01 Consumer + Projection Shape Probe**（立即开始，Probe Only）——
七项：真实消费者 / API Resource 语义 / None wire / AVSync projection /
snapshot 并列 / 是否需要 ProgramQuery / 到 API Boundary 的唯一转换点。
**硬 Gate：不能因为没有 owner，就在 A2-6 里临时创建"假的当前
ProgramMaster"用于投影。** 01 任务严格限定"真实消费者→Projection Shape"，
不偷偷开始实现 Custody。
