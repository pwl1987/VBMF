# A2-7-00 — Execution Materialization SoT / Ownership Probe

> Status: `PROBE ONLY / NO CODE CHANGE`
> Authority: A2-6 终裁（A2-7 六刀链冻结 + 分账特殊要求 + 十项禁止清单）
> Date: 2026-09-03 · Change: a2-7-execution-materialization · Base: master `caab630`
> 核心问题：**"什么运行时事实、什么时刻、以什么 ownership/SoT/语义等级，
> 允许推动 Program Domain 状态前进？"**

---

## 1. 分账（用户裁定特殊要求：已有 vs 缺失）

### 1.1 已有（**禁止在 A2-7 重造**——P0/P1 execution infrastructure 实锚）

| 能力 | 实锚 | 职责边界 |
|---|---|---|
| Materialization | `pipeline.rs`：`from_intent` L145 / `materialize` L529 / `materialize_with_output` L549 | GraphRuntimeIntent → PipelinePlan（Registry/Resolver/身份强度/device-number 实选卡）→ 实例化 |
| Execution SPI | `contracts/backend.rs` L22-30：`MediaBackend{instantiate,start,stop,recover,observe}` | 消费已授权 PipelinePlan，**不自行找设备** |
| Session 生命周期 | `session.rs` L11 冻结链：Intent→Preflight→Reserve→**Lease→Binding verify→instantiate→Allocate→start**→Running；失败精确逆序回滚 L530-531/L724-725；creator=destroyer 零孤儿 | SessionManager = Session lifecycle owner |
| Watchdog 观测链 | `watchdog.rs`：Bus drain → acceptance（**b1_first_video/b2_first_audio/b3_valid_pts** L65-67）→ a4 闩锁 SignalVerified（L46/175-179）→ AgentState → Supervisor → recovery | Runtime Health/Recovery（**非 Program writer**） |
| Bus 事件词表 | `pipeline_events.rs` L44-50：`PipelineBusEventKind{Error,Eos,StateChanged,Warning,ClockLost}` | pipeline 级观测 |
| 观测结构 | `PipelineHandle(u64)` L460；`PipelineHealth`+`MediaRt01Acceptance` L245/259；appsink 回调写 `video_first_pts/audio_first_pts/pts_state` | gate 观测域 |
| 物化事实投影 | `MediaSession.outputs: Vec<String>` L183（A2-5-04"物化事实类"先例） | wire 投影 |
| Production 等待 Intent | manifest 校验后不自动启管线，等 Control Plane StartPipeline | 未来 Custody 触发链的天然挂点 |

### 1.2 缺失（A2-7 真正的空白）

- **Execution Fact boundary**：零类型/零定义——运行时事实无 Program 可消费的
  规范形态；
- **Program Runtime Custody**：零（角色已批 A2-6-00，实现 deferred 至本阶段）；
- **三 Master writer**：`advance()/advance_to()` 域外调用 = 零（唯一命中 =
  api_boundary PMAPI 测试底座）；
- **Metadata declaration producer**：零（`default()=Unknown` 是唯一现状，
  无任何事实来源）；
- **`MasterJoinInput` 生产装配**：零（failed facts/avsync 无真实供给方）。

**分账结论**：Materialization 不是 A2-7 的空白；**Program Semantic Lifecycle
（Fact boundary + Custody + writer 链）才是**。

## 2. Execution Fact 候选七维裁表（9 候选逐一；初判供 A2-7-01 裁，非终裁）

| # | 候选 | 来源（实锚） | durable? | 纯 observation? | 可推 stage? | 仅 Join/failure input? | 初判 |
|---|---|---|---|---|---|---|---|
| 1 | PipelineHandle(u64) | instantiate 产物 L460 | 否（进程内） | 是 | **否**（只证明管线对象存在，零节点完成语义） | 否 | 生命周期关联用，非 stage 事实 |
| 2 | PipelineBusEvent::{StateChanged,Eos,Error,Warning} | Bus L44-50 | 否 | 是 | **粒度不足**：pipeline 级非节点级（"playing"≠任何 Master stage 完成） | Error 候选 failure input | 待裁（A/B 问） |
| 3 | **acceptance b1/b2/b3**（first video/audio pts + valid） | appsink 回调 L65-67 | 否 | 是 | **SOURCE_RAW→NORMALIZED 最强候选**（首有效帧经过=处理链前端完成）；中间步候选见 §3-A | b3 失败候选 failed | **核心候选** |
| 4 | SignalVerified（闩锁事件） | watchdog L175-179 | 是（event log） | 是 | 否（信号存在≠stage） | **failed 反相候选**（!signal） | Join input 候选 |
| 5 | SessionState/SessionPhase | session.rs | 是 | 是 | **禁**（Running≠MASTER_JOINED——A2-5 红线机械化禁令） | 否 | 生命周期关联，非 stage 事实 |
| 6 | outputs 物化投影 | L183 | 否 | 是 | PROGRAM_COMPOSED **候选但存疑**（output 物化≠composition 完成——当前无 composition 执行节点） | 否 | 待裁 |
| 7 | lease/resource state | resource/lease | 是 | 是 | 否 | 资源面→Runtime 域，非 Program | 不入 |
| 8 | ApiSession.outputs | wire 投影 | — | — | — | — | 投影产物非事实源 |
| 9 | ClockLost bus event | L50 | 否 | 是 | 否 | **Clock≠AVSync**（A2-5-01 消歧维持）；Clock 域观测 | 不入 AVSync 直推 |

## 3. 六问证据（A-F）

**A/B（Video/Audio 逐阶段驱动事实）——探针关键披露**：当前管线真实拓扑 =
`src → caps → (normalize 可选) → tee → appsink(+outputs)`——**不存在独立的
Switcher/Composition/Mixer 执行节点**（Switcher 仍是占位字符串未消费，A2-4
审计已锚）。因此：
- `SOURCE_RAW→NORMALIZED`：候选 = normalize 后首有效帧（b1/b3，粒度待裁）；
- `NORMALIZED→SWITCHED` / `→PROGRAM_COMPOSED`：**当前执行事实不存在**
  （无对应节点）——如实披露：这两步推进在 A2-7 内可能只能 **deferred**
  （等对应 Engine 落地）或**由 Custody 依据显式声明推进**（声明性推进，
  需裁）；
- `PROGRAM_COMPOSER→MASTER_JOINED`：语义时刻候选 = **join() 调用本身**
  （Master Join = 联合判定点）——即终态由 Custody 调 join 后经
  `advance_to(MasterJoined)` 表达，待裁。
- Audio 独立裁（b2 候选），禁从 Video 推导。

**C（Metadata declaration producer）**：完全缺失。候选 = 控制面/manifest
config（`loopback-manifest`/`PrototypeOutputConfig` 形态）或 A4 Channel
定义；当前诚实值恒 Unknown。**交 OQ-2。**

**D（failed 来源）**：Runtime 域已有事实源（`PipelineFault/HardwareFault/
SessionFailed` + acceptance 失败 + bus Error）——Custody **转换**为
`video_failed/audio_failed` bool 注入 JoinInput；映射规则（哪些事件×哪个
管线 → 哪路 failed）**交 OQ-3**。

**E（AVSync 上游）**：候选 = `video_first_pts/audio_first_pts` 时值对比
测量（执行侧产生 `AVSyncClassification`）——**Join 零阈值零测量维持**；
ClockLost≠AVSync 直推（§2-#9）。测量挂点（gate appsink 已有双 PTS vs 新
观测）与分级计算归属 **交 OQ-4**；不新建 AVSync Engine（禁止清单）。

**F（snapshot ownership）**：Custody（角色已批）。挂载层先验 = 独立
Runtime/Orchestration 侧模块，与 SessionManager **协作不取代**
（Session=Session lifecycle owner / Custody=Program semantic lifecycle
owner）；触发挂点候选 = Production StartPipeline Intent 路径（§1.1 末行）。
**交 OQ-5。**

## 4. 禁止清单（终裁 §冻结，A2-7 全程）

❌ 重写 materialize() / SessionManager / MediaBackend / GStreamer
abstraction · ❌ Watchdog/Supervisor 改造为 Program owner · ❌ 新建 AVSync
Engine · ❌ 新建 Query/Transport API · ❌ 双输入真机切换（A2-8）· ❌
万能 ExecutionFact 巨型 struct（按事实域拆：VideoExecutionFacts/
AudioExecutionFacts/MetadataExecutionFacts/FailureFacts/AVSyncObservation
——A2-7-01 按裁分）。

## 5. Open Questions（交用户裁决）

| # | 问题 | 倾向（非裁决） |
|---|---|---|
| OQ-1 | stage 推进事实映射逐阶段裁（含 §3-A 披露：SWITCHED/PROGRAM_COMPOSED 执行事实当前不存在——deferred vs 声明性推进） | A2-7-01 主裁题 |
| OQ-2 | Metadata declaration producer（config/manifest/A4/deferred） | 待裁；当前恒 Unknown 是唯一诚实值 |
| OQ-3 | failed fact 转换边界（哪些 Runtime 事件 → video/audio_failed） | 待裁 |
| OQ-4 | AVSync 上游（PTS 对比测量挂点与分级归属 vs deferred） | 待裁；禁新 Engine |
| OQ-5 | Custody 挂载层与 SessionManager 协作形态（独立模块 vs 经 Session 扩展点） | 独立模块倾向；协作接口 01 裁 |

## 6. No-Build Gate

零 .rs diff；十项禁止清单全程；01 前不定义 ExecutionFact 任何形态。

## 7. 证据文件清单

pipeline.rs L145/L245-259/L460/L529-549 · contracts/backend.rs L22-30 ·
session.rs L11/L183/L530-531/L724-725/L874 · watchdog.rs L46/L65-67/L175-
179/L237/255 · pipeline_events.rs L35-50 · A2-6-00 报告（writer 零清点）·
A2-5 系列（R-A..R-J/五步优先序/Custody 角色）。

---

## 8. 用户终裁记录（A2-7-00 → A2-7-01 Gate，2026-09-03）

> **A2-7-00 = CLOSED / APPROVED**。全局核心原则（终裁原文）：
> **"A2-7 不是'把 Runtime 状态映射成 ProgramMaster'，而是建立一条有证据、
> 有 ownership、有 attribution 的 Execution Fact → Program Semantic
> Lifecycle 链。"** Runtime 世界（PipelineHealth/MediaRt01Acceptance/
> PipelineBusEvent/SessionState/Phase/Lease/Resource/Supervisor/outputs）
> 丰富但**无一可直接等价于 Program truth**。

### 五问终裁

| OQ | 终裁 | 关键收紧 |
|---|---|---|
| OQ-1 | **事实驱动 + 缺失则 Deferred**；无真实执行节点 ≠ Program stage | **否掉"声明性推进"**（Intent≠Execution Fact——"config 要求 SWITCH"不证明"SWITCH 已完成"）； SOURCE_RAW→NORMALIZED **收紧**：b1/b3 只证明"RAW ingest 首有效帧已形成"，**禁自动命名 NormalizeComplete**——是否足以证明 NORMALIZED 取决于 normalize=true 实际元素链与可观测完成点（01 必须查死，高风险点） |
| OQ-2 | **无真实 producer → join_declaration=UNKNOWN → Join.ready=false**（fail-closed 正确行为） | **否掉 config/manifest 自动生成** Participating/NotPresent；禁 facts.is_empty()→NotPresent；A4 Channel 未来可为 producer 但禁提前借用不存在的语义 |
| OQ-3 | Runtime 产生 failure fact → **Custody 按"来源+关联 execution identity+media path"attribution** → JoinInput 注入；Join 不读 Runtime | **禁止机械等价**：Session terminated/Lease lost/ClockLost/Health degraded/Supervisor restarting ≠ video_failed；**"Failure classification/path attribution first, Join bool injection second"；不建 FailureDomain/FailureReason enum**（A2-5 不污染 Join 维持） |
| OQ-4 | **允许复用已有双路 PTS**（video/audio first/last pts + pts_state 独立维护）作为第一版 measurement source → AVSyncClassification | 三锁死：ClockLost≠AVSyncFailed / Health≠AVSyncClassification / Join 不计算阈值；**不建 AVSync Engine**；禁把 PipelineHealth 直接暴露成 AVSyncClassification |
| OQ-5 | **独立 Program Runtime Custody**（Runtime/Orchestration 边界）；SessionManager=Session lifecycle owner（协作不拥有 Program）；Supervisor=Recovery decision owner（另一条横向线） | CLOSED；三分图落盘（session facts → Custody → 三 Master → join → snapshot） |

### Stage 推进终裁表（OQ-1 附表）

| Transition | 当前合法驱动 | 结论 |
|---|---|---|
| SOURCE_RAW→NORMALIZED | 真实 normalize 执行完成后的有效输出事实 | ✅ 可实现（**01 查死完成语义**） |
| NORMALIZED→SWITCHED | 无 Switcher node/fact | ⏸️ Deferred |
| SWITCHED→PROGRAM_COMPOSED | 无 Composition node/fact | ⏸️ Deferred |
| MIXED（Audio） | 无 Mixer fact | ⏸️ Deferred |
| LOUDNESS_NORMALIZED | 无 Loudness fact | ⏸️ Deferred |
| DELAY_COMPENSATED | 无 Delay fact | ⏸️ Deferred |
| →MASTER_JOINED | Join 判定形成之后 | ✅ 可定义 |

### A2-7-01 放行（APPROVED TO IMPLEMENT，Probe+设计先行）

四空白：① Execution Fact Shape（**禁万能巨型 struct**，按域拆候选）②
Video/Audio attribution ③ Metadata declaration source ④ Custody lifecycle。
**不写 Custody implementation/Query/Transport，不碰 A2-8。**
**核心任务：查死 SOURCE_RAW→NORMALIZED 真实 execution completion 语义**
（唯一"看似有事实、实际可能只是 ingest acceptance"的高风险点——否则
NORMALIZED 成为无证据假状态）。
