# A2-8-03-01 Phase-1 Design / Implementation Probe — G-1 事件→Custody 生产链 + G-2 Runtime 监督消费面（2026-09-05, 零代码）

- **授权**: 第四十三轮终裁（主账 §58.2 ③）——「授权 A2-8-03-01 的
  第一阶段：G-1 Identity/Custody + G-2 Runtime Consumption 设计/实现
  探针；**暂不授权 Program-domain recovery implementation**」。
  本探针=设计裁面+OQ 清单, **零生产代码**; OQ 裁决后方有实现授权。
- **依赖序（R43 冻结, 禁并列同时开工）**: G-1 → G-2 → Failure
  Attribution → Recovery Contract → G-3 → G-4。
- **继承红线**: Supervisor 禁 `switch()`/`begin_switch()`（四层证据,
  03-00 §2; 本轮 grep 复核 supervisor.rs 零命中维持）; A2-7 custody
  七不（custody.rs:11-13）; C-TIMELINE Freeze「Recover 语义冻结不实现」
  ——G-1/G-2 接线不得偷渡 Program 域恢复语义。
- **行号锚**: 全部锚于 `d981728`（R42 提交后代码零变更, 与 03-00 同基）。

## §1 现状实锚（03-00 之后的展开新事实）

### 1.1 事件分发拓扑（events.rs 全读）

- 词表 14 变体（:50-107, serde tagged enum——**RuntimeEvent 是
  wire-facing 序列化类型**, 任何字段/变体加法=词表裁面）; kind/is_fault/
  severity（:109-146; 故障四类 Critical 不可被观测挤出）。
- `RuntimeEventLog`（:199-270）: 有界（默认 1024, :209-210）+
  Mutex<VecDeque> + **P1-3 两级丢弃**（:194-198 文档; Observation 满时
  可被挤出/全 Critical 时新观测丢弃/Critical 强推挤最旧——
  dropped_observations/dropped_criticals 计数暴露 :250-257）。**drain
  =破坏性 FIFO 单次消费**（:259-261）。
- `RuntimeEventSink` trait（:283-285, D8 解耦: emit 永不阻塞永不失败）;
  `FanoutSink`（:303-333, **P0-7D D3 定稿=双日志分流**）: emit 同序
  双写 `projection`（外送: transport 投影端点+gate 证据路径 drain）+
  `internal`（内消费: watchdog tick drain→health::reduce）。D3 的
  存在理由=**破坏性 drain 的消费者竞争**——单日志多消费者互相掏空
  （:293-302 文档原文）。
- 设计句（:4-5）: 「`supervisor.rs` 是唯一事件出口（见 supervisor
  归一化）」——本义=**vendor 类型经 Supervisor.ingest 归一化**;
  canonical 事件（SignalVerified/HealthChanged）由 watchdogs 直接
  emit 已有先例（watchdog.rs:186/:523）。该句与 G-1 生产者选项的
  张力见 OQ-G1-3。

### 1.2 关键新发现: internal 平面**多消费者竞争 drain**——「单一
drain 点」假设不成立

生产 drain 普查（grep 全仓 `.drain()`, 排除 #[cfg(test)]）:

| 平面 | drain 点 | 消费者 |
|---|---|---|
| internal | watchdog.rs:192 | ingest watchdog tick（**每输入管线一个**, 双输入=2 个）|
| internal | watchdog.rs:537 | execution group watchdog tick（每组一个）|
| projection | transport.rs:231-235 | `GET /api/v1/events/projection` 投影端点 |
| projection | gates/loopback.rs·gates/session_lifecycle.rs | gate 证据路径（gate 期）|

- 组合根（bin/media-agent.rs:39）: `internal_log = &world.internal_log`
  ——**ingest watchdog（:529, 每输入）与 group watchdog（:479/:492-493）
  接收同一实例**; 自测面（:106）同。双输入会话下 ≥3 个线程以 500ms
  tick 竞争 drain 同一 internal 日志: 每条事件被**恰一个**消费者取走
  （谁先 tick 谁得）。
- 后果（现状即如此, 非本探针引入）: ① 每个 watchdog 的 health_fold
  只见自己 drain 到的部分流, `*agent_state` 写入互相覆盖（近似态）;
  ② `fault_trigger_from_events` 只在 ingest 面（watchdog.rs:196-197）
  调用——**被 group watchdog drain 走的 PipelineFault 不触发任何
  supervisor 决策**（ingest 面自有 bus Error 直报路径兜底, 故行为
  可接受但语义=事件驱动谓词只覆盖部分流）。
- ⇒ **G-1 消费拓扑设计的硬约束**: custody 若挂任一既有 drain 点,
  只见事件子集; 「恰一次全量」需要新平面或 emit 时捕获（OQ-G1-2）。

### 1.3 身份链现状: bus Error 路径身份在 watchdog 已知、mapper 边界归零

- ingest watchdog bus Error 路径（watchdog.rs:174-181）:
  `sup.ingest(EventSource::Upstream, "pipeline error: {detail}")` →
  `DefaultRuntimeEventMapper`（events.rs:164-189）关键字归类 →
  `PipelineFault{pipeline: Uuid::nil(), …}`——**watchdog 持有
  device_uuid（spawn 参数）却在 mapper 边界丢弃**（ingest 签名
  `(source, observation: &str)` 无身份通道, supervisor.rs ingest 面）。
- Supervisor echo（report_failure → `PipelineFault{summary:
  RESTART_ECHO_SUMMARY}`, supervisor.rs:210 + :34）——桥与
  fault_trigger 均排除（custody.rs:143/supervisor.rs:51）。
- ⇒ 生产面 PipelineFault 仅两源: nil（桥拒收）+ echo（桥拒收）——
  **真实故障零入链**（03-00 Q3 结论的机制根因: 身份丢失在
  watchdog→ingest→mapper 这一跳）。

### 1.4 custody 面双零生产调用（grep 复核）

- `observations_from_events`: 定义 custody.rs:136, 其余命中全部在
  #[cfg(test)]（:399-:730）——零生产调用者。
- `custody_snapshot`/`attribute_failures`: 生产调用**零**（仅
  custody_snapshot 内部 :190 + 测试）。
- 桥提取规则（:125-135 文档+代码一致, A2-7 已裁）: 只提取
  PipelineFault; echo 排除; **nil 不吸收**（无身份证据不归因
  fail-closed）; HardwareFault/SessionFailed/HealthChanged/ClockLost
  不提取（等 attribution contract）; avsync 恒 Unknown。
- 身份语义（:60-68 文档）: `FailureObservation.pipeline_id` 沿用
  `PipelineFault.pipeline` = **设备 canonical 身份（DeviceId）legacy
  misnamed**; 同 enum 内 `SourceMaterialized.pipeline` 是 Pipeline
  identity——同名双语义=V0.3 Event Contract 债。`attribute_failures`
  （:98-113）按该身份做设备级相关（跨实例污染防线在设备粒度）。
- A2-7 冻结语义维持: `custody_snapshot` advance **零触发**、三 Master
  诚实停留初始态（:171-174）; 当前唯一合法 ProgramMaster=
  join_result:None。

### 1.5 G-2 挂点依赖面

- 组 watchdog loop（watchdog.rs:494-545）已有链: `switcher.observe
  (&graph).program` → `execution_group_observe_fold` → folded.actions →
  Supervisor → `ctrl.recover`（仅故障输入自身 handle）;
  `complete_switch` 仅 Observed 确认落定 Desired（:530-535, T10）。
- **组 watchdog 无 MediaTapPort 依赖**——`bridge_liveness`/
  `assemble_bridge_health`（G/H-1, program_execution.rs:134-154）所需
  tap 面不在其参数表（ctrl/switcher/group/sup/lm/agent_state/sink/
  internal_log, bin:479-493）: G-2 桥证据接线=组合根+签名变更面。
- 分类器 `classify_failure_domain`（program_execution.rs:179-200,
  单故障假设+首因遮蔽 documented :176-177）生产调用唯一=gates/
  dual_input.rs:827-832（L5.4）; `assemble_timeline_sample` gate-only
  （:596）; `assemble_bridge_health` gate-only（:758）。

## §2 R43 目标链逐边现状映射（裁决 §六链 × 实锚）

| 链边 | 现状实锚 | 判定 |
|---|---|---|
| Runtime observation | watchdog 三件套 observe+fold+闩锁在册 | 在位（Input 域）|
| → RuntimeEvent | fold 事实**不成为事件**; 仅 SignalVerified/HealthChanged/nil-PipelineFault | 部分——G-1b 身份缺口 |
| → Event custody/identity | 桥+快照双零生产调用; nil/echo 双拒收 | **G-1（拓扑+身份）** |
| → Failure-domain attribution | classify gate-only | G-2+后续边 |
| → Supervisor decision | supervisor 只消费事件+设备句柄; **不消费 custody 快照/FailureDomain**（03-00 Q5） | Recovery Contract 裁面（后置） |
| → Recovery/Escalate | Input 级闭环在册（watchdog recover+tap 重放）; Program 域零路径 | **G-3（本轮未授权）** |
| → Evidence | gate 证据行+projection 端点在册 | 随 G-1/G-2 扩面待裁 |

## §3 G-1 设计裁面（Identity/Custody 生产链闭合）

两子缺口: **G-1a 消费拓扑**（custody 如何看到全部事件恰一次——§1.2
约束）; **G-1b 生产身份**（真实故障带身份入链——§1.3 根因）。

### OQ-G1-1（身份语义——先行裁决, R43「身份语义先行」）

`PipelineFault.pipeline` 承载 device 身份（legacy misnamed, A2-7-03
终裁标记 V0.3 cleanup）。闭合生产链前裁:

- **(a) 形式化现状=设备身份**: custody 归因恒设备粒度; V0.3 债维持
  登记不提前偿付。最小变更面; 字段名语义债进入生产链（文档防线
  custody.rs:61-68 已在）。跨实例污染防线=设备级（同设备新旧管线
  实例故障同键——recover 重建后旧故障事实仍匹配新周期, 需周期边界
  裁决配合 OQ-G1-5）。
- **(b) 类型级修正提前**: PipelineFault 改承载真 PipelineId——V0.3
  Event Contract cleanup 拉入 03; 变更面大（mapper/echo/桥/
  fault_trigger/watchdog 发射面+serde wire）; 且 PipelineHandle(u64)
  ↔Uuid 两级身份映射 SoT 未裁（A2-7-02 已禁强行统一）。
- **(c) 加法双字段**: PipelineFault 增 `device_id`（serde additive）,
  custody 显式消费 device_id、`pipeline` 字段语义留给 V0.3——语义
  显式化; 词面加法仍须裁（A2-4 词表纪律: 加法演进允许但过裁面）。

### OQ-G1-2（消费拓扑——「恰一次全量」的机制）

- **(a) FanoutSink 增第三平面（custody log）**: emit 同序三写;
  custody 消费者自持节奏 drain。与 P0-7D D3 先例同构（D3 本就为
  drain 竞争而生）; 代价=**D3「双日志」定稿的契约修订**（组合根+
  FanoutSink 构造面）+每事件一份拷贝。
- **(b) 挂全部既有 drain 点**: 各 watchdog tick drain 后就地调桥、
  各自累积, 快照时聚合。零契约变更; 但多累积器聚合=**第二 SoT
  风险**（A2-7「零第二 SoT」红线张力）+恰一次语义靠约定非结构。
- **(c) emit 时同步折叠**: 包装 sink 内联 accumulate 进 custody
  累积器。反转 A2-7「Custody 不订阅不持 Runtime 引用」设计原则
  （custody.rs:115-117）——架构原则翻转须显式裁, 非实现细节。
- **(d) drain projection 平面**: 否决候选——与 transport/gate 竞争+
  平面语义错置（外送投影≠事实源）; 列出以示排除理由。

### OQ-G1-3（生产者身份修复——真实故障入链的发射面）

- **(a) watchdog 直接 emit `PipelineFault{pipeline: device_uuid,…}`**
  （替换 bus Error 的 ingest 路径或并列）: 与 watchdogs 已直接 emit
  canonical 事件同构; 与 events.rs:4-5「supervisor 唯一事件出口」
  设计句的张力须裁（该句本义=vendor 归一化单出口, canonical 直发
  有先例——释法裁决）。**双计防线回归要求**: 现路径 bus Error 在
  当 tick 直报 report_failure（watchdog.rs:201-212, 「同一 if 内至多
  一次无跨 tick 双计」:169-173）; 若事件成为唯一触发源则语义迁移
  须整链回归（emit→drain→fault_trigger→report_failure→echo 排除）。
- **(b) `Supervisor.ingest` 签名扩展携带身份**（+device: Uuid）:
  保持 vendor 归一化单出口原则; supervisor API 契约签名变更;
  mapper 产事件带真身份。
- **(c) SessionManager 面发射**: 否决候选——不观 bus、不知故障时刻
  （bus Error 只在 watchdog 面）; 列出以示排除理由。

### OQ-G1-4（桥规则冻结面确认）

提取规则五条（只 PipelineFault/echo 排除/nil 不吸收/其余 kind 不
提取/avsync Unknown）=A2-7 已裁语义。03-01 接线默认**零改动**; 任何
扩面（如 SessionFailed 入桥）=回 A2-7 裁决面, 非本阶段自决。确认或
显式另开裁面。

### OQ-G1-5（custody_snapshot 生产调用点 + Observation SoT 归属）

- 谁在何周期调 custody_snapshot（现为零调用）: 选项=watchdog tick /
  API 读出面（请求时装配, 与「消费时装配零第二 SoT」最合）/独立
  custody 周期线程（新常驻面, 须裁）。
- advance 零触发+三 Master 初始态是 A2-7 冻结——闭合后: (i) 快照
  只作观测证据（Master 推进仍等 transition evidence, 维持冻结）;
  (ii) 解冻 advance=回 A2-7 裁决。**默认 (i)**。
- R43 问「Observation 的 source of truth 是谁」: supervisor 双通道
  （自身 fault_trigger 谓词 + custody 快照事实）并存是否合法、谁是
  权威面——**G-1 闭合后两通道语义边界须裁**（建议: supervisor=决策
  触发面（实时, 部分流）; custody=归因事实面（全量, 周期）——职能
  分离叙事, 待裁）。

### OQ-G1-6（丢弃语义 × custody 事实性）

RuntimeEventLog 有界+两级丢弃（P1-3）: custody 平面若继承, 极端容量
下 dropped_criticals>0 即**故障事实丢失**（Masters 可能在故障曾发生
时报健康）。选项: (a) 接受 best-effort+丢弃计数入证据面（与
absence≠evidence 同构——丢弃计数器暴露即不静默）; (b) custody 平面
独立容量/不丢策略（容量语义须裁）。默认 (a), 待确认。

### OQ-G1-7（词面纪律总则）

RuntimeEvent 为 serde wire 类型: OQ-G1-1(c)/OQ-G1-3 的选项即词面
影响清单; 任何新变体/字段=词表裁面过审, 本探针零词面假设。

## §4 G-2 设计裁面（Runtime 监督消费面）

### OQ-G2-1（进入驻留的能力子集与顺序）

classify_failure_domain / assemble_timeline_sample /
assemble_bridge_health 三者（现 gate-only）哪些进 runtime 驻留、顺序
如何。R43 DAG 中 Failure Attribution 在 G-2 之后——建议 TimelineSample/
bridge_health（纯观测装配）先行、classify（归因语义）后接, 但**待裁**。

### OQ-G2-2（挂点与依赖注入）

组 watchdog loop（watchdog.rs:494-545）为天然挂点（复用既有
observe→fold 链, 不新建引擎——R42 §八不重造）。硬事实: 组 watchdog
**无 MediaTapPort 依赖**（§1.5）——bridge 证据接线=bin 组合根+watchdog
签名变更面; 备选=独立消费点（新常驻面, 须裁）。

### OQ-G2-3（消费输出走向）

runtime 驻留观测/分类结果的去向: (a) 只进 custody/证据面（观测,
无决策语义）; (b) 驱动 `GroupAction` 扩词（如 ReportProgramFailure）——
**GroupAction 封闭词表 {ReportInputFailure} 是 T10/T12 类型级红线**,
扩词=watchdog 契约裁面; (c) 新 RuntimeEvent 变体（词面裁面）。
注意: Supervisor 现不消费 FailureDomain/ProgramObservation（03-00
Q5）——G-2 **不得自动等于 Supervisor 消费 Program 域**（那是 DAG 后段
Recovery Contract 裁面, 且 G-3 未授权）。

### OQ-G2-4（单故障语义维持）

classify 单故障假设+首因遮蔽（documented 设计语义）runtime 驻留后
维持, 禁扩 multi-fault（十五轮终裁）; 漏报 Program 方向≠误归因
（03-00 Q12 结论维持）。

### OQ-G2-5（A2-7 冻结面兼容）

G-2 接线不得改: FailurePath 调用方预归因语义 / FailureScope
SharedPipeline 证据制（无 path 证据不凭空单路归因）/ Degraded 首版
不可达边界 / custody 七不红线。

### OQ-G2-6（sim/自测模式消费面）

internal 平面在无 watchdog 的运行形态下无人 drain（自测面 bin:106
除外）——custody 生产链在 sim 模式行为（空转=诚实无事实 vs 须常驻
消费者）待裁; 关联 G-4（Mock recover 验证面, DAG 末位后置）。

## §5 红线与不重建清单（继承+本轮新增）

- **Supervisor 禁 switch/begin_switch**: 四层证据维持（03-00 §2;
  本轮 supervisor.rs grep 零命中复核）。G-1/G-2 任何选项不得给
  Supervisor 增切换面。
- **不重建**: watchdog 三件套 / Bridge liveness / progress 判定 /
  FailureDomain 分类器 / Supervisor 决策引擎 / recover+tap 重放——
  全部在册, 03 只做收敛/接线/验证（R42 §八+R43 §六维持）。
- **A2-7 七不**（custody.rs:11-13）+ 三 Master 初始态 + 零第二 SoT。
- **C-TIMELINE Freeze**: Recover 语义不实现; G-1/G-2 不得偷渡。
- **D8/FanoutSink D3「双日志」定稿**: 任何平面加法/构造面变更=
  显式裁面（OQ-G1-2a）, 非实现细节。
- **禁顺手修隔离队列**: PORT-IDENTITY-AND-RESOURCE-ADDRESSING /
  canonical UUID namespace / Event Contract V0.3 其余项 / audio
  capability 独立性 / serial production binding / C1-P1 / converter
  interlace / CONTRACT-ANCHOR-DOC-SYNC+Mock A/B（独立同步轮, R43 已裁
  合并处理且倾向 B——本轮零触碰）。

## §6 探针边界披露

- 零代码、零矩阵（无生产变更; R40 runtime 证据继续为 baseline——
  R43 §十三零代码轮不重跑）。
- §1.2 internal 平面多消费者竞争 drain 为**现状事实陈述**（非本
  探针引入的缺陷判定）: 其 health_fold 部分流近似语义是否单独立项
  =待裁（默认: 随 G-1 拓扑裁决一并处置, 不单独开 change）。
- 未授权面: G-3 Program 域恢复实现 / G-4 Mock 面 / 词面任何变更 /
  FanoutSink 契约变更——全部等 OQ 裁决。

## §7 OQ 汇总（待用户裁决）

| # | 裁面 | 选项 | 默认倾向（供裁, 不代选） |
|---|---|---|---|
| OQ-G1-1 | PipelineFault.pipeline 身份语义 | (a)形式化设备身份 (b)类型级修正提前 (c)加法双字段 | —（先行裁决） |
| OQ-G1-2 | custody 消费拓扑 | (a)第三平面 (b)全 drain 点挂载 (c)emit 时折叠 (d)projection 否决 | (a) |
| OQ-G1-3 | 真实故障发射面 | (a)watchdog 直发带身份 (b)ingest 签名扩展 (c)SessionManager 否决 | (a)或(b) |
| OQ-G1-4 | 桥规则冻结确认 | 原样冻结 / 扩面另裁 | 原样冻结 |
| OQ-G1-5 | 快照调用点+SoT 边界 | 调用点三选+advance(i)(ii)+双通道职能 | (i)+请求时装配+职能分离 |
| OQ-G1-6 | 丢弃×custody | (a)best-effort+计数入证据 (b)独立容量策略 | (a) |
| OQ-G1-7 | 词面纪律 | 随 G1-1/G1-3 选项过审 | — |
| OQ-G2-1 | 驻留能力子集与顺序 | 三能力×顺序 | 观测先行, classify 后接 |
| OQ-G2-2 | 挂点+tap 依赖注入 | 组 watchdog 复用 / 独立消费点 | 组 watchdog |
| OQ-G2-3 | 输出走向 | (a)只观测面 (b)GroupAction 扩词 (c)新事件变体 | (a) |
| OQ-G2-4 | 单故障语义维持 | 维持（禁扩） | 维持 |
| OQ-G2-5 | A2-7 冻结面兼容 | 不触碰 | 不触碰 |
| OQ-G2-6 | sim 模式消费面 | 空转诚实 / 常驻消费者 | 待裁 |
