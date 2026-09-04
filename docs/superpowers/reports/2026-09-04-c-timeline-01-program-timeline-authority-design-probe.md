# C-TIMELINE-01 Design SoT Probe: Program Timeline Authority & PTS Continuity（零代码）

- 日期：2026-09-04；分支 `comet/a2-8-dual-input-switch`（基线 d123b45 之上）。
- 性质：**只读探针，零代码**——按"探针先行"纪律：形态未知先取证据，
  缺口原样上报，不自行补模型，不预裁方案。
- 裁决链：A2-8-01 第三轮终裁③（术语=Program Timeline Continuity /
  Timestamp Normalization；四方案未裁；'出口再生成'措辞废止）→ A2-8
  第二十六轮终裁（C-TIMELINE-01 CONFIRMED + 设计十问 v2 + 反假修复
  红线）→ 第二十六轮终裁补正（执行令：d123b45 保持，下一轮直接进入
  本设计，**十项冻结前禁写 normalization 实现**）→ 本探针。
- 探针使命：为十问 v2 逐问提供**代码 / 真机 / V0.2 spec 三面证据**与
  选项空间，汇总 OQ 待裁清单。**方案裁决权全在用户**。

## 1. 术语基线与消歧

- **规范术语**（A2-8-01 三轮终裁③冻结）：Program Timeline Continuity /
  Timestamp Normalization；"出口再生成"措辞已废止。
- **四方案**（未裁）：A 切后 Regenerator / B Clock-Segment Timeline /
  C 出口 normalization / D 切换新 timebase。
- **消歧一**：`switch_graph.rs:496` 注释中的"方案 A"= A2-8-01 的
  **切换执行方案**（成对单 epoch：video+audio 双 selector 同目标同
  epoch 切换），与本处 Timestamp Normalization 四方案 A-D **无关**。
  后续设计文档必须显式消歧，禁混用。
- **消歧二**：三个"时钟/时间"概念分层——
  1. `TimelineSample.sampled_at_ms` / Bridge liveness = **wall-clock
     观察时钟**（二十轮 G/H-1 冻结，与 PTS 严格分离）；
  2. 三列 PTS（input/bridge/program）= **media-clock** 证据；
  3. V0.2 Channel reference clock（spec §时钟章）= **Channel 级观察
     校准时钟**（Latency Probe/AVSync/Recording 时间戳按此校准）。
  三者禁互推、禁互修（C-TIMELINE-01 禁拿 sampled_at_ms 修 PTS）。
- **消歧三**：V0.2 AVSync Manager 的 Offset/Drift Correction = **A/V
  相对校正**（同源内 video vs audio），非 **A/B 切换时间线映射**
  （跨源 Program 轴衔接）——两职责边界是本设计待裁点（OQ-12）。

## 2. 现状代码事实（SoT 锚点）

### 2.1 时间从哪来（三段链）

| 段 | 事实 | 锚点 |
| --- | --- | --- |
| 输入管线 | 每输入一条独立 GStreamer pipeline（launch 串 `{video_src} ! video/x-raw ! tee name=v …`），各自时钟域/base-time；decklinkvideosrc 产生源侧 PTS | pipeline.rs:206-210 |
| 桥 | 输入侧 tee 分支注入 intervideosink/interaudiosink；program graph 用 intervideosrc/interaudiosrc 消费——跨管线 buffer 携带输入侧时间戳；bridge address 非新 identity | controller.rs:624-625；switch_graph.rs:253-258/:297-300；program_execution.rs:30 |
| Program graph | 双源 → input-selector(video) + input-selector(audio) → queue → appsink；**零 clock/base_time/latency 设置**（全文件 grep 零命中）；**零 timeline 层**（identity/videorate/set_offset/timestamp mutation/segment manipulation 全零命中） | switch_graph.rs:8-12/:218/:276/:313 |

- capsfilter 仅 Simulation 形态；**Bridged => None**（switch_graph.rs:219-231，
  :231 精确锚点）——真实桥接透传输入管线实际 caps/媒体时间属性。

### 2.2 切换语义（已冻结，非本设计对象）

- 成对切换：双 selector 同目标同 epoch（切换执行"方案 A"），P1-1
  双平面补偿（video 成·audio 败→回滚；再败→degraded+active=None）；
  input-selector 于下一缓冲生效=帧边界对齐。switch_graph.rs:490-505。
- ExecutionGroup 纯 Desired 状态机：恰 {session_id, inputs, desired,
  switch_epoch} 零时间戳；complete_switch 须真实 Observed 才推进。
  switch_execution.rs:93-100。**终裁确认：ExecutionGroup 不是 Timeline
  Authority，此禁令永久有效。**

### 2.3 观测面（只测量，行为零触碰）

- appsink 回调 `buf.pts()` → `observe_video_pts/observe_audio_pts`：
  首有效 PTS→ValidMonotonic；之后 `pts < last`→NonMonotonic（sticky）；
  无 PTS 帧不参与。帧计数=真实 sample 递增。pipeline.rs:236-246/:291-311。
- ProgramObservation（observed_active/video_active/audio_active/
  switch_epoch/input_pts/program pts×2/state×2/frames×2）=
  contracts/switch.rs:56-77（既有 Execution Adapter Contract，非为 L4
  事后拼凑——终裁确认）。
- TimelineSample 三列独立测量 program_execution.rs:59-77；program_alive
  =复合字段（:111-112）。

### 2.4 声明面（Adapter Gap）

- `PipelinePlan.normalize: bool` 存在、**零消费**（pipeline.rs:136-141
  doc 自认"未被 Execution Adapter 消费——normalize=true/false 生成
  管线相同"）。normalize=true **不是 Execution Fact**（Intent≠Fact）。
- 影响矩阵终裁标注：normalize=**核心入口之一**（未来设计的 Plan 声明
  候选载体——去留待裁 OQ-10）。

### 2.5 Gate 面（不动）

- L4 五合取 dual_input.rs:644-648；H1 跳过 :774。L4 证据原则与 H1
  均冻结（二十六轮终裁），本设计不改 Gate 表面。

## 3. 真机证据（§33.5 摘要）

- 确定性签名（复跑 2 逐项复现）：Input/Bridge **全列 ValidMonotonic**，
  仅 Program 列切换后 NonMonotonic——真实 appsink buffer PTS 回退。
- A/B in 列互差 8-10ms（B 落后）；pre→post program PTS +4.0s≈settle
  窗推进；alive=false=NonMonotonic 的复合字段推论。
- **异构输入**：A=1080i25（电视）/ B=1080p25（ball）——video format
  continuity 与 PTS continuity 是两条独立未定义轴（终裁十三节）。

## 4. V0.2 spec 证据面（docs/architecture/ARCHITECTURE_V0.2.md）

| spec 锚点 | 内容 | 对本设计的意义 |
| --- | --- | --- |
| :830 | "AV Sync 测量在 Master Join 处；AV Sync 不再是普通 Process Node——是 Master Join 的属性" | AVSync 测量归属已冻结；切换时间线映射是另一职责 |
| :874-880 | AVSync Manager §3.8：measure_offset_ms=当前 Video PTS − Audio PTS | A/V 相对校正语义 |
| :1235-1237 | Errata-9：AVSync Manager=Measurement+Offset/Drift Correction+Failure Classification，**不做 Recovery** | 角色边界参照 |
| :1211-1220 | **Channel reference clock**：同一 Channel 内所有 Latency Probe/AVSync/Recording 时间戳按此 reference 校准；clock_quality 等级（SYSTEM=BEST_EFFORT ms 级） | spec 已有 Channel 级**观察校准**时钟概念——非 Program 媒体时间权威，边界待裁（OQ-12） |
| :99/:503 | PACKET_SWITCH"时间戳连续性"=**压缩域**切换语境，前提"主备 codec+profile 完全一致"；主备时间戳/GOP 不齐→画面跳变 | RAW/SDI live 双输入**无此对位条款**；异构策略（OQ-4）无 spec 先例 |
| :537-540 | timestamp_continuity / audio_continuity = QC checklist 字段 | 观测性要求存在，执行语义不存在 |
| :1955-1966 | 视频 PTS 异常 discontinuity🟠P1 归 Source QC；AV Sync offset 40/100/250ms | 异常分类先例 |

**Spec 缺口（关键发现）**：V0.2 全文 "timeline" 命中皆为 Incident
Timeline（X4）与 Playout 时间线——**无 Program 媒体时间线 Authority
声明**。RAW live 双输入切换的 Program PTS 所有权/映射/epoch 在 spec
层为空白，须由本设计裁决补齐（这是比代码 Gap 更上游的空白）。

## 5. 十问 v2 逐问：证据与选项空间（不预裁）

### Q1 Program Timeline Authority

- 事实：program graph 零时间权威（§2.1）；spec 空白（§4）；落点禁令
  已排除 ExecutionGroup/Supervisor/MediaBackend；三轮终裁④冻结
  "Pipeline 不知 A/B/Program——Program Execution 层组合"。
- 选项空间：(a) SwitchGraph GStreamer 执行层内建（adapter 实现细节）；
  (b) Program Execution 层新 **Timeline Authority 组件**（编排 inter 桥
  与 selector 的组合件）；(c) 出口下游（muxer/encoder 前）；(d)
  `PipelinePlan.normalize` 激活为 Plan 声明 + Adapter 执行（Intent→
  Plan→Fact 三段现成挂点）。
- 待裁：owner 层级；其与 SwitchExecutionAdapter 的关系（内嵌 vs 并列
  组合）；Authority 的最小状态集（epoch/origin/offset？）。

### Q2 A→B 切换 PTS mapping

- 事实：两时钟域差实测 8-10ms 量级；selector 帧边界对齐已有；GStreamer
  segment/offset 机制未用；反假修复红线禁 max(last+dur, incoming)。
- 选项空间：切换时刻 offset 补偿（新源 PTS−接续点）/ 重映射到 Program
  连续轴 / segment event 显式声明 discontinuity / 组合。
- 待裁：映射执行层（与 Q1 owner 绑定）；是否需要缓存/lookahead；
  audio 与 video 映射是否同函数。

### Q3 Video/Audio epoch 共享

- 事实：切换执行=成对单 epoch（video+audio 同 switch_epoch）；Program
  video/audio PTS 两列独立观测。
- 待裁：Program timeline epoch 复用 switch_epoch / 独立 media epoch /
  per-plane epoch——与切换执行 epoch 的关系须显式冻结。

### Q4 异构 1080i25↔1080p25 输入策略（新增问）

- 事实：真机 A=1080i25/B=1080p25；Bridged 透传 caps（无统一格式
  normalize）；V0.2 PACKET_SWITCH 的"主备全等"前提在 RAW 域无对位。
- 选项空间（终裁五选项）：pass-through / Deinterlace / Caps normalize /
  Format conversion / Switch boundary adaptation。
- 待裁：允许异构并存 or 强制归一；field/frame duration 语义（i 与 p
  的 frame 边界不同）；与 PTS 映射（Q2）的耦合序； converter
  interlace 断言工件（每跑恰 9 条，未定性）与此问的关联待裁。

### Q5 switch settle 时间语义

- 事实：pre→post +4.0s≈settle 窗实测；settle 期间 program 列仍出帧。
- 待裁：settle 期间 PTS 归属（旧源尾/新源头/显式 gap 声明）；settle
  窗是否为 Authority 状态机的一段。

### Q6 discontinuity / segment event 语义

- 事实：PtsMonotonicity sticky 只测不声明；GStreamer segment event
  未使用；V0.2 将"PTS 异常 discontinuity"归 Source QC 观测。
- 待裁：discontinuity 声明面（域内事件 vs Gst segment event vs 两者）；
  observation 状态机是否扩展（如 DiscontinuityDeclared 与 sticky
  NonMonotonic 的关系）。

### Q7 recover 后 timeline 处理

- 事实：L5=recover 复流路径（未执行，H1 skip）；Supervisor=recovery
  decision only。
- 待裁：recover 后 epoch/timeline 重建语义；与 A2-8-03 supervision 的
  接缝（不提前侵入）。

### Q8 normalization 的 Execution Fact

- 事实：A2-7 冻结 fact absent≠fact=false、否声明性推进；
  MASTER_JOINED 候选=join() 调用本身；normalize=true 现≠Fact。
- 待裁：normalization fact 形态（归 Execution Adapter）；fact 名/键/
  证据；与 SourceMaterialized / SwitchExecuted 既有 fact 的关系。

### Q9 Observation 如何证明"真的 normalize 了"

- 事实：影响矩阵标注 ProgramObservation/TimelineSample ⚠️可能增列
  （execution/normalization evidence）。
- 待裁：证明面=ProgramObservation 增列 / TimelineSample 增列 / 新
  evidence 对象；"真的 normalize"的**可观测定义**（PTS 连续 + segment
  一致 + 帧率稳定 + A/V epoch 一致？——定义本身待裁）。

### Q10 落点禁令（已冻结，实现期红线）

- 不塞 ExecutionGroup / Supervisor / MediaBackend；switch_execution.rs
  纯模型（零 GStreamer）；Pipeline 不知 A/B/Program。
- 附带待裁：`PipelinePlan.normalize` 字段去留——bool 现状 vs 词表化
  （裸 bool 禁的词表纪律）vs 删除重建。

## 6. 四方案 A-D 对照（证据排列，非预裁）

| 方案 | 语义 | 十问覆盖初判 | 已知约束/风险 |
| --- | --- | --- | --- |
| A 切后 Regenerator | 切换点后重建 Program 时间 | Q1/Q2/Q5 | 引入再生延迟；需新执行组件；"重建"的 Fact 语义须定义 |
| B Clock-Segment Timeline | Program 时钟按源分段（每源一段连续） | Q1/Q2/Q6 | segment 语义=声明"段内连续、段间切换"；下游须理解段边界 |
| C 出口 normalization | 出口处统一归一 | Q1/Q8 | '出口再生成'措辞已废——C 的精确语义须重定义；出口层 owner 待裁 |
| D 切换新 timebase | 每次切换起新 timebase | Q1/Q2/Q3/Q7 | epoch 语义重；下游 encoder 适配面大；与"连续性"目标张力最大 |

（本表仅排列证据覆盖面与风险，**方案裁决与组合权在用户**；四方案
非互斥，可组合。）

## 7. 反假修复红线（照录 + 锚点）

1. 禁 `max(last_program_pts + duration, incoming_pts)` 类"PTS 不回退"
   假闭合——NonMonotonic→ValidMonotonic 不代表 AV sync / frame
   duration / segment semantics / latency / switch boundary 正确。
2. 禁拿 `sampled_at_ms`（wall-clock）修 PTS（media-clock）；禁把
   PLAYING / 帧基 alive 冒充 timeline healthy。
3. 禁 Intent 冒充 Fact（normalize=true ≠ 已 normalize；fact absent≠
   fact=false）。
4. 禁塞 ExecutionGroup / Supervisor / MediaBackend / SwitchExecution
   域模型（SwitchGraph=GStreamer 执行边界⚠️其影响待裁，但 Domain
   纯净不变）。
5. H1 / L4 证据原则 / Gate 表面不动（除非另行显式授权）。

## 8. OQ 待裁清单（汇总，供用户终裁）

- **OQ-1** Program Timeline Authority owner 层级（§5-Q1 四候选 a-d）。
- **OQ-2** 切换 PTS mapping 语义与执行层（含 audio/video 是否同函数）。
- **OQ-3** Program timeline epoch 与 switch_epoch 的关系。
- **OQ-4** 异构输入策略（五选项）+ field/frame duration 语义 +
  converter interlace 工件关联。
- **OQ-5** settle 期间 PTS 归属。
- **OQ-6** discontinuity 声明面与 PtsMonotonicity 状态机的关系。
- **OQ-7** recover 后 timeline 重建边界（A2-8-03 接缝）。
- **OQ-8** normalization Execution Fact 形态。
- **OQ-9** Observation 证明面与"真的 normalize"可观测定义。
- **OQ-10** `PipelinePlan.normalize` 字段去留。
- **OQ-11** 四方案 A-D 裁决（含组合；含对 Q4 异构维度的覆盖）。
- **OQ-12** C-TIMELINE-01 与 V0.2 AVSync Manager（A/V 相对校正）/
  Channel reference clock（观察校准时钟）的职责边界三消歧。

## 9. 不变量（冻结面，实现期红线）

1. Intent → Plan → Fact 三段；fact absent ≠ fact=false。
2. ExecutionGroup / Supervisor / MediaBackend 零 timeline 职责。
3. 三列观测只测量；observation clock 与 media PTS 严格分离。
4. Pipeline 不知 A/B/Program；switch_execution.rs 零 GStreamer。
5. 切换执行语义（成对单 epoch + P1-1 补偿 + TargetAlreadyActive
   纵深）不动。
6. H1 / L4 证据原则 / Gate 表面不动。
7. 词表纪律：裸 bool 禁、enum+Option 复合、键集恰 N 锁蔓延。

## 10. 下一步

1. 本探针=设计裁决输入；**OQ-1..12 待用户终裁**（十项冻结前禁写
   normalization 实现——执行令原文）。
2. 裁决后：冻结设计 → 独立 change（A2-8-C-TIMELINE-01）开工 → 实现 →
   L4/L5 按 §29.2 纪律复跑。
3. 独立队列不阻塞：C1-P1（poll 内可选 bus check）· converter
   interlace 断言定性 · 现场项（BNC#4 对端/dn2 线缆/照片）·
   PORT-IDENTITY / canonical UUID namespace / A2-8-03~05。

## 11. 十问终裁（用户裁决接收与落账，2026-09-04）

### 11.1 裁决接收与前置复核

- 来源：用户对本报告 §8 OQ-1..12 的完整终裁（"C-TIMELINE-01 十问
  终裁"）——**不需要再补一轮代码探针**。
- 用户前置独立复核声明：已复核本探针报告与当前分支实际代码，尤其
  确认 Bridged 路径 = intervideosrc/interaudiosrc → input-selector →
  queue → appsink 且 **Bridged 不做 capsfilter**（= 当前 Program Graph
  无隐藏时间线归一化层）；`PipelinePlan.normalize` 只是声明字段，
  **未形成 Execution Fact**。
- 本侧锚点复核（HEAD 1db28e2，树干净；全部吻合）：

| 裁决引用事实 | 仓库锚点 | 复核 |
| --- | --- | --- |
| Bridged program graph = inter[video/audio]src → input-selector → queue → appsink | switch_graph.rs:8-12；video 平面 selector:218 + queue:233 + appsink:234；audio 平面 :276/:277/:278；Bridged 源 :256/:258/:298/:300；链结 :313 | 吻合 |
| Bridged 无 capsfilter（透传输入实际 caps/媒体时间属性） | switch_graph.rs:219-231（:231 `Bridged => None`） | 吻合 |
| Program graph 零隐藏时间线层（零 clock/base_time/latency 设置） | §2.1 全文件 grep 零命中（本轮无代码变更，证据沿用） | 吻合 |
| PipelinePlan.normalize = 声明未消费、非 Fact | pipeline.rs:136-141 doc 自认；:227 等多处 `normalize: true` 字面量存在 | 吻合 |
| ExecutionGroup 恰 {session_id, inputs, desired, switch_epoch} 零时间戳 | switch_execution.rs:93-100 | 吻合 |
| ProgramExecutionRuntime 已存在（终裁结构图的容器名 = 现有类型名） | program_execution.rs:209（Inner{group, switcher, graph, taps, tap_port, watchdog_stop}） | 吻合——终裁结构 = 在其上增设 TimelineAuthority 组件，非新造引擎 |

### 11.2 总体架构裁定（照录要旨）

> **采用 "Program Timeline Authority + Source Segment Mapping" 组合。**
> Program Timeline 是 Program Execution 层的独立权威；每个输入 Source
> 在进入 Program 时被映射到 Program Timeline；切换时通过新的 Source
> Segment 建立连续映射；Video/Audio 共享 Program Epoch 但保留各自
> media PTS；GStreamer Segment/Event 是执行层承载机制而非架构权威。

四方案**不是四选一**：**A 部分采用（执行机制）+ B 作为核心方案 +
明确排除 C/D**——"B 为主 + A 的执行机制 + 排除 C/D"。

### 11.3 OQ-1..12 逐问裁定表

| OQ | 终裁 | 关键语义（关键裁定近逐字保留） |
| --- | --- | --- |
| OQ-1 Authority 落点 | **Program Execution 层 TimelineAuthority** | 不放 ExecutionGroup/Supervisor/MediaBackend/单独 Pipeline/纯出口 muxer；**不做大型独立 Engine**（否则制造新架构中心）；目标结构 `ProgramExecutionRuntime{SwitchExecution, TimelineAuthority, ProgramGraph, Observation}`；链路 `TimelineAuthority → ProgramTimelinePlan → GStreamer Execution Adapter → Gst segment/timestamp/pad`；**Domain 层拥有时间线语义，Adapter 层拥有"怎么让 GStreamer 做到"** |
| OQ-2 PTS mapping | **Program Timeline 连续轴 + Source Segment Offset Mapping** | `max(last_pts+duration, incoming_pts)` **明确永久禁止**；`SourceSegment{source_id, program_epoch, source_start_pts, program_start_pts, offset}`；`mapping_B = Program continuity anchor − Source B continuity anchor`；PTS 连续性是**映射问题非数字大小修复问题**；必须能回答"这个 Program PTS 由哪个 Source、哪个 Segment、经什么 mapping 得来"（否则 recover/discontinuity/A→B→A/clock drift/encoder restart 再次失去语义基础） |
| OQ-3 epoch | **V/A 共享 Program Epoch、不共享数值序列** | 同一次切换同一 Program Epoch，各自独立 PTS/mapping/media clock，相对关系由 AVSync 验证；**switch_epoch ≠ program_epoch**，关系=一次成功 program switch → TimelineEpoch 推进（SwitchEpoch 1 → TimelineEpoch 1 起始对应，不复用）；理由：Switch 是执行事件，Timeline Epoch 是媒体语义；recover 可能变 timeline epoch 未必发生业务 source switch |
| OQ-4 异构策略 | **Timeline 层不承担格式归一化——Q4 与 Q2 解耦** | PTS continuity 归 Timeline Authority，格式 continuity 归 Media Format/Program Graph，二者不是同一问题；当前阶段=**Switch Boundary Adaptation**（非偷偷转换）；deinterlace/帧率/像素/分辨率转换由**独立 Program Media Format Policy** 决定，不得因 C-TIMELINE-01 顺手塞入；先允许异构输入进入 Timeline 设计，但 **Program Format Contract 显式声明"当前不保证无缝 format continuity"**；不阻止修 PTS |
| OQ-5 settle | **状态语义（非等待后修 PTS）** | `Stable(A)→SwitchRequested(B)→SwitchExecuted(B)→TimelineTransition(B)→Stable(B)`；settle 期间 Program PTS **必须已属于新 Program Timeline**（禁假装属 A/等 settle 结束才开始 B/暂停 PTS/用 wall-clock 补时间）；`settle ≠ timeline gap ≠ timestamp freeze`——mapping 已生效、稳定性尚未确认；此定义对 L5 非常重要 |
| OQ-6 discontinuity | **Domain Discontinuity + GStreamer Segment/Event 双层表达** | `TimelineAuthority → {Program Segment, Discontinuity declaration} → GStreamer Adapter → Gst Segment/Event`；**Gst Segment Event 不是 Authority，只是执行载体**；PtsState 扩展四态 `Unknown/ValidMonotonic/DiscontinuityDeclared/NonMonotonic`；**declared discontinuity + expected PTS transition ≠ unexpected backward PTS**（必须冻结） |
| OQ-7 recover | **本轮不实现，语义冻结** | Recover 后不得简单继承旧 Timeline 状态：`Recover → 新 execution instance → Timeline reconstruction → 新 source segment → Program timeline continues`；两类：**Soft Recover**（execution 重建、timeline 可连续）/**Hard Recover**（continuity 无法证明→新 ProgramTimeline epoch）；给 A2-8-03 留正确接口；**Supervisor 只决定 recover，不拥有 Timeline** |
| OQ-8 Execution Fact | **结构化 TimelineMapped** | 必须有 Fact；不得是裸 `normalized=true`；`TimelineMapped{program_epoch, source_id, segment_id, mapping, evidence}`；**TimelineMapped ≠ TimelineHealthy**（关键） |
| OQ-9 证明面 | **专门 TimelineObservation** | 不修改现有 ProgramObservation 承担一切；`TimelineObservation{program_epoch, source_id, segment_id, input_pts, mapped_program_pts, mapping_offset, discontinuity_state, video_continuity, audio_continuity, observed_at}`；**observed_at=wall clock，绝对不能用于计算 program_pts**；"真的完成"≥7 条：①Program PTS 连续 ②Source→Program mapping 与 declared segment 一致 ③video continuity ④audio continuity ⑤epoch 一致 ⑥segment transition 符合声明 ⑦无未声明 backward jump；**单纯 pts>previous_pts 永远不足** |
| OQ-10 normalize 字段 | **删除裸 bool** | `normalize: bool` 违反已冻结词表纪律（裸 bool 禁）；未来方向 `PipelinePlan → TimelinePolicy`；**本轮设计冻结前不改代码** |
| OQ-11 四方案 | **A 部分采用 / B 核心采用 / C 不采用 / D 不采用** | A=采用"切换后重新建立 Source Segment Mapping"机制但非独立 Regenerator；B=Clock-Segment Timeline **主方案**；C=出口 normalization 不采用（**"出口再生成"概念正式废止**）；D=切换新 timebase 不采用（与 Program Timeline Continuity 目标冲突） |
| OQ-12 三时钟职权 | **彻底切开、不得互相越权** | ①**Program Timeline Authority**（Program PTS 应该是什么——Program Execution）②**AVSync Manager**（V↔A 相对关系——不能决定切换后 Program PTS 应为多少）③**Channel Reference Clock**（观察系统的 latency/AVSync/Recording 时间戳参考——observation calibration，**不能成为 Program PTS generator**） |

### 11.4 八条红线 R1-R8（照录）

1. **R1** 不得用 wall-clock 修 PTS。
2. **R2** 不得用 `max(last + duration, incoming)` 伪造连续性。
3. **R3** Timeline Authority 不进入 ExecutionGroup。
4. **R4** Timeline Authority 不进入 Supervisor。
5. **R5** Timeline Authority 不进入 MediaBackend。
6. **R6** GStreamer Segment/Event 是 Execution Adapter 机制，不是 Domain
   Authority。
7. **R7** 1080i/1080p 格式转换不由 Timeline Authority 偷做。
8. **R8** Normalization Fact 与 Timeline Healthy 必须分离。

### 11.5 影响与不触碰清单（照录）

- **不碰**（已 PASS/已冻结面）：L0、L1a、L1b、L1c、L1d、L2、L3、
  L4-SWITCH、Teardown、SwitchExecution、SessionManager、Resolver、
  PortRegistry、ResourceRegistry、Supervisor。**只解决 L4-TIMELINE**。
- 状态严格保持：`A2-8-02-I = FAIL-PENDING-CORRECTION`（L4-SWITCH
  PASS / L4-TIMELINE FAIL-PENDING-CORRECTION / L5 SKIPPED BY H1）——
  **不能因为现在有了设计就把 Gate 改成 PASS**。
- 隔离禁顺手修：C1-P1（保持独立 P1，不重开 C1，不阻塞本设计；探针
  事实锁定被用户确认准确——300ms 宽限后 bus error 只查一次、后续
  轮询只采样 signal 属性，晚到异步 Error 确实可能表现为 Some(false)）、
  converter interlace assertion、PORT-IDENTITY、canonical UUID namespace。

### 11.6 执行令与状态

- "设计 SoT 探针"阶段**正式结束**；**下一阶段不是直接写代码**。
- 下一动作=**Design Freeze**（15 项清单：TimelineAuthority Domain
  Contract / ProgramTimeline / ProgramEpoch / SourceSegment /
  TimelineMapping / Discontinuity / TimelineMapped Execution Fact /
  TimelineEvidence / Video-Audio 双平面规则 / Switch→Timeline 状态转移 /
  GStreamer Adapter execution contract / 1080i-1080p 当前边界 / Recover
  接口语义 / L4 如何重新证明 / 不变量与失败条件）。
- Design Freeze 已形成：
  `docs/superpowers/reports/2026-09-04-c-timeline-01-design-freeze.md`
  （本报告 §11 落账 + 冻结文档；如有出入以本 §11 终裁原文为准）。
- 冻结之后才能开 `A2-8-C-TIMELINE-01` implementation change。
- 结论照录："**C-TIMELINE-01 十问可以冻结；架构方向正式确定为
  'Program Timeline Authority + Clock-Segment Timeline + Source Segment
  Mapping'，不进入实现，下一动作只做 Design Freeze。**"

## 12. Design Freeze 复核通过 + Implementation Change 正式开启（2026-09-04，零代码）

### 12.1 裁决接收

- 用户实际复核 f3158a0 推送后的 Design Freeze：**核心冻结内容与十问
  终裁一致**——复核通过，本轮闭合，不需回裁、不需补探针。
  - `f3158a0 = 有效 Design Freeze`。
  - 用户确认要点：ProgramExecutionRuntime=现有组合根增设
    TimelineAuthority 未凭空造 Engine 层；Program Timeline 与
    wall-clock/Channel Reference Clock/AVSync 职责分离；ProgramEpoch
    与 switch_epoch 拆分+V/A 共 epoch 各自 mapping；SourceSegment/
    TimelineMapping 冻结"映射问题非数字调大"+双禁（wall-clock/
    max 假闭合）；TimelineMapped≠TimelineHealthy 防 Intent 冒 Fact；
    TimelineObservation 独立+observed_at 限观察；四方案=B 核心+
    A 的 Segment 重建机制、C/D 淘汰，未被偷换成"简单选 A/B"。
- **正式进入 A2-8-C-TIMELINE-01 Implementation Change**。
- 工程纪律（照录）："下一轮不是'看到 Freeze 就直接大改代码'"——
  第一步=**实现前代码拓扑探针 / Impact Map → 最小变更面冻结 → 再写
  代码**；重点钉死十项落点（①PipelinePlan 替换 normalize ②Runtime
  构造/生命周期挂入 ③SourceSegment 注入边界 ④intersrc 后/selector
  前后承担 timestamp/segment ⑤GStreamer SEGMENT/EVENT 真实发送路径
  ⑥V/A 分别处理 segment ⑦appsink 观察与 TimelineEvidence 对齐
  ⑧switch→mapping→observation 事务顺序 ⑨failure/rollback timeline
  处置 ⑩L4 判据升级为 Timeline Mapping Evidence）；**第 4/5/6 项
  不准凭架构图猜，以真实 Rust/GStreamer API 与现有 graph 生命周期
  为准**。

### 12.2 工程状态表（照录冻结）

| 项目 | 状态 |
| --- | --- |
| C1 Resolver Timing | PASS / CLOSED |
| C1-P1 async Bus Error | 独立 P1，隔离 |
| L0–L3 | PASS |
| L4-SWITCH | PASS |
| L4-TIMELINE | FAIL-PENDING-CORRECTION |
| L5 | SKIPPED BY H1 |
| A2-8-02-I | FAIL-PENDING-CORRECTION |
| C-TIMELINE-01 Design | FROZEN |
| C-TIMELINE-01 Implementation | 下一阶段 |
| converter interlace assertion | 独立队列 |
| PortIdentity / UUID namespace | 独立队列 |

### 12.3 本轮入口动作

- **Implementation Impact Map 已交付（零代码）**：
  `2026-09-04-c-timeline-01-implementation-impact-map.md`——As-Is
  拓扑实锚（组合根两生产构造点/Plan 面 8 构造点 2 生产/切换全序/
  观测链/GStreamer 高层 API 零存量）+ 盒上 gst-inspect 实证
  （GStreamer 1.28.2：input-selector 无时间戳改写·drop-backwards=
  丢帧藏证禁入方案·intersrc do-timestamp=false 原始终戳透传·
  **identity single-segment 真实存在=方案 B 现成 primitive 候选**）
  + gstreamer-0.23.7 crate 实证（event::Segment::new/Pad::send_event/
  PadProbeInfo::buffer_mut/EVENT_DOWNSTREAM 全真实可用）+ 十项逐项
  现状锚+候选+**OQ-IMP-1..7 待裁清单**+最小变更面候选 9 行+执行序
  提案（sim 实验刀前置）。
- 节奏指示（照录）："先一轮完整 Impact Map + implementation probe，
  确认实际代码落点后一次性进入最小实现批次；不要再把已经冻结的架构
  重新讨论。"

## 13. OQ-IMP-1..7 裁决 + SIM-01 实验刀执行（2026-09-04）

### 13.1 裁决接收（照录决策表）

| OQ | 裁决 | 结论 |
| --- | --- | --- |
| IMP-1 | `normalize: bool` → `TimelinePolicy` | **ADOPT**：删除 normalize 语义；TimelinePolicy 为明确域语义（首版表达 SourceNative / ProgramTimelineMapped），禁任何含糊 bool 开关（normalize=true/fix_pts/force_continuity 类禁） |
| IMP-2 | Timeline 数据入 Adapter 方式 | **ADOPT**：扩展现有 Plan/materialization 链，**不新增** Timeline trait/Port/Controller SPI，不改 MediaBackend::recover；链=Runtime→TimelineAuthority→Timeline Plan/Mapping→PipelinePlan/materialization→GStreamer Adapter；禁 SessionManager/Supervisor/MediaBackend/GStreamerController 横向侵入；ProgramEpoch authority 永在 ProgramExecutionRuntime/TimelineAuthority |
| IMP-3 | Segment 执行点组合 | **SIM EXPERIMENT**（授权 sim-only 实验刀） |
| IMP-4 | TimelineEvidence 读出面 | **ADOPT**：Adapter 装配、Runtime 独立读取；**不塞进 PipelineHealth**；结构=GStreamer Adapter{PipelineHealth, BridgeObservation, TimelineEvidence}→Runtime→TimelineAuthority；**Evidence 不是 Authority**——禁"GStreamer 说 PTS=xxx→Authority 自动接受" |
| IMP-5 | Adapter 微观序 | **SIM EXPERIMENT**；必须验证 SwitchExecuted 后 TimelineTransition 已成立（settle=等稳定证据非等时间线重建） |
| IMP-6 | 失败三结局 | **ADOPT**：①Preserve（同 epoch+mapping valid+continuity valid→保持）②NewEpoch（执行成功但 continuity 不可证→ProgramEpoch++，禁硬接 PTS/max 假闭合=R2 绝对禁区）③FailClosed（mapping/segment transition/epoch invalid·undeclared backward jump·evidence 不足→transition=failed）；**不增第四种"猜测成功"** |
| IMP-7 | L4-TIMELINE 谓词 | **ADOPT**：升级为 Timeline Mapping Evidence 判定（TimelineMapped∧Program PTS continuity∧Video∧Audio∧ProgramEpoch consistent∧Segment transition declared∧No undeclared backward jump）；参考结构 TimelineTransitionEvidence{declared_segment, observed_segment, program_epoch, source_id, source_pts, mapped_program_pts, mapping_offset, video_continuity, audio_continuity, discontinuity_state, undeclared_backward_jump}；L4 之问从"PTS 有没有倒退"改为"B 是否按 TimelineAuthority 声明的 SourceSegment 映射合法进入同一 Program Timeline 且 V/A 双连续性证据成立" |

- 实验范围授权（照录）：A2-8-C-TIMELINE-01-SIM-01 十项（最小双源模拟 graph/
  A/B 独立 PTS 源/input-selector/Segment 注入位置/identity.single-segment/
  Pad::send_event(Segment)/buffer PTS probe/V-A 分开/记录 active-pad/
  记录 Segment→buffer→appsink 事件 PTS 序列）；**只回答 IMP-3+IMP-5**。
- 实验禁改清单（照录遵守）：normalize/PipelineHealth/L4/SwitchGraph 正式
  逻辑/Production graph 全未触碰；实验工程=盒上 scratch 不入库。

### 13.2 SIM-01 已执行（2026-09-04T12:16Z 收官，9 变体 2583 行日志）

报告=`2026-09-04-c-timeline-01-sim-01-experiment.md`（F1-F7 全证据+候选
结论+诚实边界）；工程/日志在盒 `~/ct-sim-01/`（sha256 归档）。关键事实：

- **F1** inter 桥隐式按接收墙钟重定基——200ms 生产者基差跨桥后仅剩
  **0.108-0.267ms 相位差**；真机 8-10ms 同源现象；Program NonMonotonic=
  切换点相位回退（问题规模=帧内相位级，与 Freeze 映射模型吻合）。
- **F2** 翻 active-pad 后 selector **自然转发** stream-start(B)→caps(B)
  →segment(B) 到 appsink——切换边界在事件流上天然可见（免费边界标记）。
- **F3** identity single-segment **只吃段不修 PTS**：vb 下游只见 1 个
  segment 但 PTS 仍回退 −0.155ms——**"吞段假阳性"实证**（观察点 7）；
  不得作为机制或证明面。
- **F4** 控制线程 `Pad::send_event(Segment)` **两序均被拒**（sent=false）
  ——外部段注入路径不可行。
- **F5** selector src BUFFER probe + Domain 声明映射（anchor−B_anchor）
  **完整可行**：vd-pre backward=0、B 首帧精确落 anchor（A 末帧+40ms）、
  121/121 映射节拍规整；aud-map 同（162/162）——**V/A 双平面独立成立**。
- **F6** 微观序：pre-flip 安装结构性无竞态（规范序候选）；post-flip 以
  ~1ms 赢得竞态（窗口真实但窄）；**附带发现：set_property 后立即 readback
  =旧值 sink_0（9/9）而 buffer 流已切**——"已执行"证明禁立即 readback，
  生效边界=下一缓冲。
- **F7** v0/aud 基线唯一翻转点=切换后相位回退——与生产 L4 签名同构，
  实验有效性锚。

### 13.3 状态

- IMP-3/IMP-5 候选结论已出（报告 §4/§5：执行点=selector 后 per-plane
  BUFFER probe 声明映射+F2 自然段边界；微观序=pre-flip 安装→翻 pad→
  生效边界=下一缓冲→Observed 走帧/事件序列），**待用户终裁**。
- 裁后即冻结最小变更面（Impact Map §4 九行候选）→ 正式最小实现批次。
- 顺带发现（登记不阻塞）：gstreamer-rs 0.23 无公开 parse_launch
  （auto/functions crate-private）——生产同构程序化构链不受影响。

## 14. IMP-3/IMP-5 终裁 + IMP-2 实现层纠偏 + 最小实现批次开工令（第三十一轮，2026-09-04）

用户完成以 a5c20b5 为基线的实际仓库闭环复核后给出终裁。结论：**IMP-3/5
可以终裁；IMP-2 需按真实代码拓扑做一次"实现层纠偏"（非架构重开）**。

### 14.1 账前代码断言复核（本助手对真实仓库逐项核锚，全部成立）

| 终裁引用的代码事实 | 实锚 | 复核 |
| --- | --- | --- |
| ProgramExecutionRuntime 持 group+switcher+graph+taps；创建序 Tap→graph→Start，停止序 Program Stop→Tap Detach | program_execution.rs:198-206/:241-291/:317-339 | ✅ |
| ExecutionGroup 只有 {session_id, inputs, desired, switch_epoch}，无 Timeline 状态 | switch_execution.rs:92-101 | ✅ |
| 生产链 SessionManager→SessionInput→ExecutionGroup→ProgramExecutionRuntime→SwitchExecutionAdapter→GStreamerSwitchAdapter→build_program_pipeline→inter→selector→queue→appsink | switch_graph.rs:399-435/:245-359 | ✅ |
| `build_program_pipeline()` 依 ExecutionGroup 双 SessionInput 构图，**不消费 PipelinePlan**；PipelinePlan=上游 ingest 采集计划 | switch_graph.rs:400-415（签名只收 handle/mode/devices/initial_active）+ pipeline.rs:133-147 | ✅（IMP-2 纠偏的事实基础成立） |
| switch() 真实时序=video active-pad→audio active-pad→g.active/g.av_epoch→SwitchExecuted；complete_switch 等 observe 见 target 才推进 Desired | switch_graph.rs:459-524 + switch_execution.rs:184-192 | ✅ |
| ProgramObservation 无 TimelineEvidence；SwitchGraph 无 timeline 字段 | contracts/switch.rs:55-73 + switch_graph.rs:38-60 | ✅ |
| PtsMonotonicity 三态 backward=sticky | pipeline.rs:239-247/:294-318 | ✅ |
| 现行 L4 判据=completed∧observed==B∧epoch==1∧双 plane state≠NonMonotonic∧pts.is_some | gates/dual_input.rs:644-648 | ✅ |
| recover()=保存 taps→remove→stop→rebuild→Playing→replay taps（本 change 不碰） | controller.rs recover 链（02-C 已核锚） | ✅ |

### 14.2 终裁照录（IMP 最终裁决表）

| 项目 | 最终裁决 |
| --- | --- |
| IMP-1 | ✅ ADOPT：`normalize` 删除，`TimelinePolicy` 取代裸 bool |
| IMP-2 | ✅ **ADOPT WITH CORRECTION**：PipelinePlan 只承载声明；ProgramTimeline 必须走 ProgramExecution→ProgramTimelinePlan→Adapter |
| IMP-3 | ✅ ADOPT：selector 后 per-plane EVENT+BUFFER probe |
| IMP-4 | ✅ ADOPT：TimelineEvidence 独立；observation wrapper，不污染 ProgramObservation |
| IMP-5 | ✅ ADOPT：anchor→declaration/install→active-pad→Segment event→next buffer→mapping→evidence→settle |
| IMP-6 | ✅ ADOPT：Preserve/NewEpoch/FailClosed |
| IMP-7 | ✅ ADOPT：L4=TimelineTransition Evidence proof |

IMP-3 细则（照录）：

- 执行点=**selector 后、每个 media plane 独立的 BUFFER execution probe**；
  配套 selector src pad 的 EVENT_DOWNSTREAM probe 捕获自然 stream-start/
  caps/segment 边界（F2）。与真实 Bridged graph（inter→selector→queue→
  appsink，无其它时间线层）完全对齐。
- 为什么不是 selector 前：不能证明"最终进入 Program 的 buffer 已按
  Program Timeline 映射"——C-TIMELINE 要证 **Program 侧事实**。
- 为什么不是 identity：F3 已证"segment 表面连续 ≠ program timeline 已
  建立"（吞段假阳性）——identity 只能是 GStreamer 行为工具，不能承担
  Authority/proof。
- F4 精确表述（照录）："不是说 GStreamer 永远不能发送 Segment"，而是
  **当前实验中控制线程直接向该 Program pad 外部注入 Segment 的路径实际
  sent=false，不能作为本实现的主注入机制**——不过度扩大解释为
  "GStreamer Segment API 不可用"。

IMP-5 微观序冻结（照录 ①-⑩）：

```text
① 取得旧 Program Timeline anchor
② TimelineAuthority 声明新的 SourceSegment / Mapping
③ 将 timeline transition state 安装到 Program execution graph
④ 执行 active-pad 翻转
⑤ selector downstream 自然产生 B 的 Segment/Event
⑥ 第一枚属于 B 的实际 BUFFER 到达 selector-output probe
⑦ 对该 BUFFER 执行 Source→Program mapping
⑧ 产生 TimelineMapped / TimelineEvidence
⑨ settle：等待稳定证据
⑩ Stable(B)
```

- ⑤⑥ 为两个关键状态界；**SwitchExecuted ≠ TimelineTransition complete**
  （active-pad set_property ≠ 时间线已切完）。
- **active-pad readback 只能做辅助 execution observation，不能作为
  Timeline 生效边界**（F6：set_property 后立即 readback 仍可能旧值而
  数据流已切）。
- F6 正式实现约束（照录冻结）：必须"**事件确认 + 下一 Buffer**"——
  `TimelineTransition(B)` 生效点=Segment(B) 被 downstream event 流证实
  **且**下一枚属于 B 的实际 media buffer 在 selector 输出侧被捕获；
  禁 set active-pad→立即 read→认为 B 已开始。

IMP-2 纠偏（照录）："Timeline 数据进入现有 PipelinePlan/materialization
链"按字面实施会落错地方——PipelinePlan 属 **ingest**（DeviceInfo→
Resolver→device-number→port→SourcePlan→ingest 管线）；Program Timeline
真实路径=ExecutionGroup→ProgramExecutionRuntime→SwitchExecutionAdapter→
SwitchGraph，**没有 PipelinePlan**。最终：`PipelinePlan.normalize→
TimelinePolicy` 只解决历史声明问题（契约清理）；真正的 Program Timeline
=ProgramExecutionRuntime→TimelineAuthority→ProgramTimelinePlan/
TimelineTransition→Program graph execution adapter——**不把 Program
Timeline 语义硬塞进 ingest PipelinePlan**（与冻结 §1 分层链一致）。

normalize 删除与 Timeline 实现=**两件事**（照录）：A. PipelinePlan 契约
清理（Intent≠Execution Fact+裸 bool 词表）；B. Program Timeline 映射实现
（ProgramExecutionRuntime+GStreamerSwitchAdapter/SwitchGraph）=L4 真正
修复面。**禁宣称"改了 PipelinePlan 就完成 Program Timeline"**。

IMP-4 契约演进（照录）：`ProgramExecutionObservation{program:
ProgramObservation, timeline: TimelineObservation/Evidence}`——observe()
返回该组合：不污染 ProgramObservation 语义/不新增第二个 Timeline SPI/
仍只有一个 Program observation surface/L4 直接消费 timeline evidence/
Mock 与 GStreamer 实现同一返回结构=**现有观察契约的最小演进**（用户
明示"这是目前仓库里我认为必须补上的一个遗漏"）。

SwitchGraph 侧（照录 §七）：增加 adapter-side `TimelineExecutionState`，
Video/Audio 每 plane 独立 {current_source, current_segment, mapping,
transition_state, last_source_pts, last_program_pts, continuity}+共享
program_epoch——**不是把 TimelineAuthority 塞进 SwitchGraph**；Authority
在 ProgramExecutionRuntime，SwitchGraph 只保存"当前执行中的映射状态"。

Source identity 闭合（照录 §八）：**必须由"声明+Event+Buffer"三件事实
共同闭合，禁瞬时 property readback**（F6 滞后）——与 TimelineMapped≠
TimelineHealthy 冻结一致。

V/A 双平面（照录 §九）：ProgramEpoch 共享，VideoSegment+VideoMapping 与
AudioSegment+AudioMapping **各自独立**（SIM-01 121/121+162/162 证明
可实现）；禁一个 offset 同时改 video/audio。

PtsMonotonicity 升级（照录 §十）：加 `DiscontinuityDeclared` 成四态；
纪律=**Segment transition declared + expected transition observed →
DiscontinuityDeclared/合法 transition**，**禁发现 backward 就把状态改回
ValidMonotonic**（不洗状态，Freeze §6）。

IMP-6 状态机映射（照录 §十一）：**Preserve**=epoch=N+mapping valid+
continuity valid+segment transition valid→继续 epoch=N；**NewEpoch**=执行
成功但 continuity 不可证→epoch N→N+1（不是去改旧 PTS）；**FailClosed**
=mapping missing/segment mismatch/wrong epoch/undeclared backward jump/
evidence insufficient→TimelineTransition failed，**不能继续把 Program
当成正常 Stable**。

recover()（照录 §十二）：本 change **不实现 recover timeline**（留
A2-8-03）——只让 Timeline 状态**能够表示** recover 后重建；改 recover
会同时越界 Timeline+MediaBackend recover+Supervisor。

L4 最终替换（照录 §十三）：

```text
switch execution correct
 AND timeline transition declared
 AND observed Segment(B)
 AND B first mapped buffer observed
 AND mapped PTS continuity
 AND video continuity AND audio continuity
 AND ProgramEpoch consistent
 AND no undeclared backward jump
```

**L4-TIMELINE 不再是 PTS monotonicity test，而是 TimelineTransition
proof**（与冻结 §14 一致）。

### 14.3 最终最小代码面（照录 §十四）+ 排除清单

```text
1. pipeline.rs        PipelinePlan.normalize → TimelinePolicy（声明清理）
2. program_execution.rs  新增 TimelineAuthority/ProgramTimeline/ProgramEpoch/
                         SourceSegment/TimelineTransition/TimelineMapped/
                         TimelineEvidence; transition orchestration
3. contracts/switch.rs   ProgramExecutionObservation{program, timeline}
4. switch_execution.rs   不动（ExecutionGroup/switch_epoch/Desired 不动,
                         不放 TimelineAuthority）
5. switch_graph.rs       video/audio selector src 各 EVENT+BUFFER probe;
                         adapter-side TimelineExecutionState; 无 Authority
6. dual_input.rs         仅升级 L4-TIMELINE proof; 其余不动
```

排除（照录）：SessionManager/Resolver/PortRegistry/ResourceRegistry/
Supervisor/MediaBackend::recover/MediaTap/C1 Signal Probe/switch
correctness/1080i-1080p format 全 NO。

### 14.4 批次授权与实现纪律（照录 §十六）

- **SIM-01 已足够支撑 IMP-3/5 终裁，不需要第二轮实验。**
- 正式进入 A2-8-C-TIMELINE-01 Implementation：**第一批=Domain+contract+
  Mock**（一次完成）→ **第二批=GStreamer Adapter+L4**（一次完成）→
  最后真机复跑。
- 固定实现纪律（照录）："**TimelineAuthority 产生'应该怎样映射'的声明；
  selector downstream Event/Buffer 产生'实际上发生了什么'的证据；两者
  在 Runtime 中闭合成 TimelineMapped。**"

### 14.5 本助手披露项（实现期裁定边界，disclosed）

1. **observe() 契约演进的机械波及**：终裁 §六 observe()→
   ProgramExecutionObservation 使既有消费面需机械适配——watchdog.rs:491
   （fold 输入取 `.program`）、registry.rs:393/:404、dual_input.rs L2b/L3/
   L4/L5 观测行（`.program.` 路径前缀；**L4 判据表达式零变化**）。
   watchdog/registry 不在 §十四 六文件清单内=契约演变的不可避免机械
   连带，零语义变化，逐处可审。
2. **epoch 计数口径衔接**：Freeze §3"一次成功 program switch →
   program_epoch 推进（SwitchEpoch 1→TimelineEpoch 1）"与终裁 §十一
   "Preserve=epoch N 不变/NewEpoch=N→N+1"并存——按**第三十一轮 §十一
   （更晚、更具体）实现**：初始 epoch=0；Preserve 保持；NewEpoch +1；
   recover 重建 +1（§13 冻结语义）。差异已披露，如需 §3 字面口径由
   用户下轮纠正（单方法计数策略，零结构性返工）。
3. **install 路径 trait 化**：ProgramTimelinePlan 进入 Adapter 的签名
   （Freeze §11 明示"不预裁，implementation change 首刀"）按 IMP-2
   纠偏链落为 SwitchExecutionAdapter 既有 trait 上的最小方法
   （默认=未实装 fail-closed 错误；GStreamer 实装=第二批）——非第二
   SPI、非新 trait。

## 15. Batch 1 实现账（Domain + contract + Mock，2026-09-04 落地）

按 §14.4 批次授权执行；第二批（GStreamer Adapter+L4）与真机复跑未动。

### 15.1 交付面

| 文件 | 变更 | 形状 |
| --- | --- | --- |
| `src/program_timeline.rs`（**新**，纯 Domain 零 GStreamer） | 全部时间线 Domain | `ProgramEpoch`/`SegmentId`/`MediaPlane`/`AnchorPair`/`SourceSegment{source_id, program_epoch, segment_id, source_start_pts, program_start_pts, offset}`（`declare`=program_anchor−source_anchor 单点生产 + `map_pts` 段内映射）/`ProgramTimelinePlan{target, switch_epoch, video, audio}`（Freeze §4 结构+§7 segment_id 补全）/`PlaneContinuity{Unproven,Continuous,DeclaredDiscontinuity,Violated}`（禁裸 bool）/`BackwardJumpFact`/`TimelineTransitionEvidence`（IMP-7 十一字段照录）/`TimelineMapped`（Freeze §7 五字段）/`TimelineObservation`（Freeze §8 恰十键 + `no_evidence` 诚实缺席行）/`TransitionFailure`（九词封闭 + thiserror）/`TransitionOutcome{Preserved,NewEpoch,Failed}`/`PlaneTransitionState{AwaitSegmentEvent→AwaitFirstMappedBuffer→Mapped}`（⑤⑥）/`TimelinePhase{Stable→SwitchRequested→SwitchExecuted→TimelineTransition→Stable; TransitionFailed=IMP-6 终态}`/`PlaneTimeline`（§七 per-plane 形状）/`TimelineAuthority`（`declare_transition`/`abort_transition`/`on_switch_executed`(epoch 联动校验)/`on_segment_event`(⑤ 身份闭合)/`on_mapped_buffer`(⑥⑦ mapped==f(source) 证据校验+四态+连续性+双平面齐备闭合 ⑧)/`close_transition`(Preserve=epoch 不变/NewEpoch=epoch+1 按观测实况 re-base offset 不改 PTS)/`confirm_settled`(⑨⑩)/`on_program_pts`(稳态四态机·未声明回退=NonMonotonic sticky+FailClosed)/`fail_closed`(外部检出入口)/`snapshot` |
| `src/pipeline.rs` | IMP-1 契约清理 + 四态 | `TimelinePolicy{SourceNative, ProgramTimelineMapped}` 取代 `normalize: bool`（8 处构造位 true→ProgramTimelineMapped；serde 无 default；Gap 登记移交文档化）；`PtsMonotonicity` + `DiscontinuityDeclared` 四态 + `observe_video/audio_pts_declared`（声明边界非回退→DiscontinuityDeclared；**声明不豁免回退**——declared 路径回退仍 NonMonotonic sticky）；既有 `observe_*_pts` 语义零变化 |
| `src/contracts/switch.rs` | IMP-4 契约演进 | `ProgramExecutionObservation{program, timeline}`；trait `observe()` 返回组合面（单一 observation surface）；`install_timeline_transition(graph, &ProgramTimelinePlan)` 默认=未实装 fail-closed Err（§14.5.3） |
| `src/adapters/switch_mock.rs` | Mock 闭环 | 双模式出口（未安装=legacy 独立再生成流**逐字节保持**；已安装+已执行=映射后源流 `program=f(source)`，F5 同构；翻转后首个 observe tick=Segment(B) 等价事件、无缓冲交付，再下一 tick=首枚映射缓冲——F6 生效边界=下一缓冲同构）；`install_timeline_transition` 实装（pre-flip epoch 联动/target 非 active fail-closed）；`switch` 增声明↔计划联动纵深；timeline 证据行仅在首枚映射缓冲后成事实（此前 no_evidence） |
| `src/adapters/gstreamer/switch_graph.rs` | 诚实边界 | 仅 `observe()` 包装（timeline 行=`no_evidence`——**第二批实装 probe 前 absence≠evidence 不伪造**）；probe/TimelineExecutionState/安装=第二批；tests 机械 `.program` |
| 机械适配（§14.5.1 披露） | `.program` 路径 | `watchdog.rs`（fold 输入 1 处+注释）、`registry.rs`（2 处）、`gates/dual_input.rs`（**恰 7 处绑定行**，L4 判据表达式零变化）、`program_execution.rs`（FailingSwitcher 签名+2 测试位）、`lib.rs`（模块注册） |

### 15.2 实现期裁定（disclosed，第二批复核点）

1. **§8/IMP-7 单行字段规范载体=video 平面**（source_pts/mapped_program_pts/
   mapping_offset/input_pts/discontinuity_state 单值取 video 边界帧；audio
   独立性由 audio_continuity+PlaneTimeline 承载）——冻结单行形状内的实现
   约定，代码文档注明。
2. **顺序违反 ≠ 终态**：⑤ 重复/⑥ 先于 ⑤=拒收当前证据（`EvidenceOutOfOrder`
   Err，transition 仍在途）；身份/映射不匹配（SegmentMismatch/MappingMismatch）
   =`fail_closed` 终态——前者可重报证据，后者为矛盾不可自愈。
3. **epoch 口径**按 §14.5.2（Preserve 保持/NewEpoch+1/初始 0）。
4. **Mock 双模式**：legacy 行为逐字节保持由既有测试锁死
   （switch_rt_01_program_pts_monotonic_across_switch 等全绿）。

### 15.3 验证（盒 10.30.15.10，2026-09-04）

- `cargo fmt --check` OK；`cargo test`（default）**217** pass；
  `cargo test --features mock` **377** pass 0 fail（新增 18：timeline_rt_01
  ×12 + switch_rt_02 ×3 + pipeline 四态/wire ×3）；
  `cargo test --features bmd-provider,gstreamer-backend` **236** pass 0 fail
  （含真实双切换 GStreamer 双测——observe 契约演进零回退实证）；
  `cargo clippy --features mock --all-targets -- -D warnings` PASS；
  `cargo clippy --features bmd-provider,gstreamer-backend --all-targets -- -D
  warnings` PASS。
- 关键闭环测试 `switch_rt_02_canonical_order_and_mapped_outlet_close_loop`：
  真实 `TimelineAuthority` 声明→pre-flip 安装→翻转→边界 tick 无缓冲交付→
  首枚映射缓冲出口=f(source)→Authority 校验证据→**Preserve(epoch 不变)→
  settle→Stable**——§14.4 实现纪律（Authority 声明+Adapter 证据+Runtime
  闭合）在 Mock 层全链成立。
- `timeline_rt_01_observation_keyset_locked_to_freeze_shape`：§8 键集恰十键
  wire 锁。

### 15.4 未做（第二批范围，未越界）

switch_graph selector 后 per-plane EVENT+BUFFER probe（IMP-3 执行点）·
adapter 侧 `TimelineExecutionState`（终裁 §七）·GStreamer
`install_timeline_transition` 实装·`ProgramExecutionRuntime` 挂
TimelineAuthority+transition orchestration（①-⑩ 驱动）·`dual_input.rs`
L4-TIMELINE 谓词升级（IMP-7 九项合取）·真机复跑（§29.2 纪律）。以上全未
触碰；`recover`/Supervisor/SessionManager/Resolver/PortRegistry/
ResourceRegistry/MediaTap/C1 零触碰。

## 16. Batch 1 复核终裁：APPROVED + 两前置 + Batch 2 开工令（第三十二轮，2026-09-04）

用户按 f82e625 实际代码+diff+冻结文档全盘复核。

### 16.1 总体裁决（照录要点）

**Batch 1 = ✅ APPROVED（无需回滚/大范围返工）**。四项成立确认：
①Domain/GStreamer 真分层（program_timeline.rs 纯 Domain，链=
TimelineAuthority→ProgramTimelinePlan→证据校验，与冻结分层一致）；
②ExecutionGroup 零污染（仍恰 {session_id, inputs, desired, switch_epoch}，
边界必须继续保持）；③observe 契约机械波及处理正确（watchdog/registry/
dual_input 经 `.program` 用原有语义，**未发现隐藏语义扩散**）；④GStreamer
侧诚实缺席正确（no_evidence 边界对）。

关闭项（照录）：**PipelinePlan 与 Program Timeline 边界正式关闭不再回头**
（build_program_pipeline 实吃 devices/initial_active，构图
inter→selector→queue→appsink；ProgramTimelinePlan 在 program_timeline.rs
正确归属）。**SwitchExecution 调用链零 Timeline 污染确认**
（Intent→plan→begin→switch→set_active×2→g.active/av_epoch→SwitchExecuted；
complete_switch 仍 Observed 驱动；Batch 2 orchestration **不得把
on_switch_executed 变成第二个 switch state machine**——两状态机经
SwitchExecutionPlan/SwitchExecuted 关联但各自拥有自己的状态）。

批准+限制（照录）：install_timeline_transition 放置正确，但 Batch 2 硬
条件=**install 只做 Domain Plan→Adapter TimelineExecutionState 安装**，
真正执行由 EVENT/BUFFER probe 完成——禁"install 完=TimelineMapped"的
Intent/Fact 混淆。SourceSegment::declare 正确（offset=anchor−anchor 符合
F5/R2），但 **offset 只能来源于 TimelineAuthority 声明的 AnchorPair——
GStreamer probe 禁重算 offset 覆盖 Domain 声明**（否则 Authority 被架空；
正确方向=Authority 声明 offset→Adapter 应用→Evidence 验证==该 offset）。
V/A 结构无冲突，**Batch 2 必须两 selector 各挂一套 execution state/probes
（禁 audio=video 附属）**。PtsMonotonicity（PTS 流观测回退/合法边界）与
PlaneContinuity（平面相对当前 Segment 连续证明）**二者不能合并**——已分开
=正确。video 单行规范载体**批准但仅限 wire/evidence serialization**——
内部 state 必须 Video State/Audio State/Shared ProgramEpoch 三立，禁把
audio 降格为附属条件。Mock 通过≠GStreamer 被证明（SIM-01 已证
active-pad/Segment/BUFFER 存在真实异步时序——**Batch 2 才是风险高峰**）。

### 16.2 两项 Batch 2 前置（照录，"不应再进入讨论，直接处理"）

1. **BLOCKER-DOC：Design Freeze epoch 文本统一**。Freeze §3"一次成功
   program switch → program_epoch 推进"与代码"Preserve 不变"=语义定义
   冲突（双 SoT 不可接受）。终裁：**采用 Preserve 不增 epoch**——
   ```text
   switch_epoch  = execution event count
   segment_id    = source segment generation（每次切换段世代+1）
   program_epoch = discontinuous program timeline generation
                   （Preserve=同世代不变；NewEpoch/Hard Recover=+1）
   ```
   否则 ProgramEpoch 退化成另一种 switch counter，破坏"ProgramEpoch≠
   switch_epoch"初衷。**docs-only consistency patch 先行**（§3+相关
   §10/§13/§15 文本统一），非代码 blocker、不回滚 Batch 1。
2. **BLOCKER-IMPLEMENTATION：no_evidence 消除虚假 epoch=0**。实际
   ProgramEpoch(7) 而行报 0 → "0"变成看起来真实的值，违反 absence≠
   evidence。修法：`no_evidence(program_epoch, observed_at)` 或 adapter
   构造时注入当前已知 epoch——**不改 Option<ProgramEpoch>（十键形状已锁）**；
   正确缺席表达=program_epoch=当前已知 epoch + 其余字段缺席。

### 16.3 三个非阻塞风险（照录）

- **P2**：SourceSegment::declare 用 i64 承载 u64 差值（当前 ns 范围无实际
  问题；未来宜改显式有符号差值算法，不阻断）。
- **P1**：no_evidence epoch（=前置②）。
- **P1**：TimelineMapped.mapping 用完整 SourceSegment 合理，但**不得当
  immutable history 唯一仓库**——Segment=历史事实、TimelineExecutionState=
  当前运行映射；未来 A→B→A/recover/new epoch 须允许段历史**累积不覆盖**
  （与冻结"段只增不改"一致）——Batch 2 顺便锁测试。

### 16.4 Batch 2 冻结结构 + 禁做清单 + 十四步顺序（照录）

结构（照录 §十二）：

```text
ProgramExecutionRuntime
│ ├── ExecutionGroup(switch_epoch)
│ ├── TimelineAuthority(program_epoch)
│ ├── ProgramGraph
│ └── Observation{ProgramObservation, TimelineObservation}
SwitchGraph
│ ├── video_selector{EVENT_DOWNSTREAM probe, BUFFER probe}
│ ├── audio_selector{EVENT_DOWNSTREAM probe, BUFFER probe}
│ └── TimelineExecutionState
```

生效链：TimelineAuthority→Plan→install→SwitchExecuted→Segment(B)
observed→first B buffer→mapping→TimelineMapped→both V/A mapped→settle→
Stable(B)。

禁做（照录 §十三）：TimelineAuthority 不入 SwitchGraph（只持 Adapter
execution state）·set_active 不产生 ProgramEpoch·active-pad readback 不判
Timeline 生效·identity.single-segment 不用·外部控制线程 send_event 主路径
不用·MediaBackend::recover 不改·Supervisor 不碰。

十四步顺序（照录 §十七，1-2 直接处理；3-12 主实现；14 只在矩阵绿后）：

```text
1 docs-only epoch consistency fix
2 no_evidence epoch 修正
3 SwitchGraph TimelineExecutionState
4/5 Video/Audio EVENT_DOWNSTREAM probe
6/7 Video/Audio BUFFER mapping probe
8 GStreamer install_timeline_transition
9 ProgramExecutionRuntime 挂 TimelineAuthority
10 Runtime orchestration ①-⑩
11 ProgramExecutionObservation.timeline 真证据装配
12 dual_input L4-TIMELINE 九项合取
13 Mock/GStreamer 双轨回归
14 真机 A2-8-02-I 复跑
```

A2-8-02-I 保持 FAIL-PENDING-CORRECTION 直至真机 Timeline Evidence PASS；
Batch 1 不改变门禁状态。

## 17. Batch 2 实现账（十四步执行，2026-09-04 落地）

按 §16.4 顺序执行；步骤 1-2 直接处理，3-12 主实现，13 双轨回归全绿，
14 真机复跑见 §18。

### 17.1 交付面（按十四步）

| 步 | 交付 | 落点 |
| --- | --- | --- |
| 1 | Freeze §3 epoch 文本统一（docs-only commit 59aec43: 修订记录头 + 三计数器职权分离表；§10/§13/§15 核对无需改——§13 Hard Recover=+1 本自洽） | design-freeze.md |
| 2 | `no_evidence(program_epoch, observed_at)` 携带当前已知 epoch（十键形状不改 Option）+ `no_evidence_carries_current_epoch` 测试；段历史**只增不改**（`video/audio_history` + `segment_history(plane)` + `segment_history_accumulates_never_overwrites` 测试——风险 3 锁；abort 声明不入历史） | program_timeline.rs |
| 3⑧ | `TimelineExecutionState{plan, executed, video, audio}`（每 plane `{segment(冻结), segment_observed, first_mapped, last_observed}`）+ `install_timeline_transition` 实装（pre-flip epoch 联动 + **V/A 声明一致性** fail-closed；**只安装**——§六硬条件文档钉死）；switch() 增声明↔执行计划联动（翻转前拒收）+ 成功仅置 `executed`（**不产生 ProgramEpoch**） | switch_graph.rs |
| 4-7 | `attach_plane_probes`：每 selector src pad 双探针——EVENT_DOWNSTREAM（Segment 事件=声明驱动身份：翻转后该平面首个 Segment 即 target 段——F2；禁 readback）+ BUFFER（**施加声明段冻结 offset**——禁 probe 重算[§七硬条件]；`make_mut().set_pts`；无声明/未执行/⑤未观测→透传零改写=legacy 逐字节保持）+ selector sink pad 分支观察探针（①锚证据: per-plane per-branch last PTS+节拍） | switch_graph.rs |
| ① 契约 | `sample_switch_anchors(graph, target)`（纯观测: program 连续性锚=出口实测 PTS+active 分支节拍; target 源连续性锚=target 分支 PTS+节拍; 缺席=fail-closed）+ `timeline_execution_facts(graph)`（⑤⑥⑦ per-plane 证据输入）——同一 trait 两个证据方法（非第二 SPI），默认 fail-closed/None | contracts/switch.rs |
| ⑨⑩ | `Inner.timeline: TimelineAuthority`（create 以 initial_active 锚定 epoch 0）+ `switch_program(intent)`：①基准 on_program_pts+锚采样→②declare（唯一 offset 生产点）→③install→④begin/switch（失败→abort+传播）→⑤-⑧轮询 observe+facts→Authority 校验闭合（超时=EvidenceInsufficient FailClosed）→⑨settle 稳定窗 3 轮（回退→FailClosed; 停滞超时不 FailClosed——归故障面）→⑩confirm_settled+complete_switch（Observed 驱动）；**不是第二个 switch state machine**（两状态机经 plan/executed 关联各自拥有） | program_execution.rs |
| ⑪ | `observe_execution()`：program=adapter 既有平面 + timeline=**Authority snapshot**（Domain SoT——epoch/段/连续性恒当前）；adapter 行=执行侧原始证据（epoch=声明 epoch）双行分工文档钉死 | program_execution.rs + switch_graph.rs observe |
| ⑫ | L4 改 `rt.switch_program` + 九项合取：L4-SWITCH 语义保持（completed∧observed==B∧epoch==1∧pts 在场∧≠NonMonotonic）∧ L4-TIMELINE（Preserved∧declared==observed segment∧V/A Continuous∧无未声明回退∧disc≠NonMonotonic∧epoch 一致∧mapped>pre∧出口≥边界帧）+ timeline 证据全打印 | gates/dual_input.rs |

### 17.2 实现期裁定（disclosed）

1. **三 trait 证据方法**（install/sample/facts）均为既有 SwitchExecutionAdapter
   上的最小证据/执行面（Freeze §11 "签名=implementation 首刀"落地）——
   非新 trait/第二 SPI。
2. **双 timeline 行分工**：裁决级=Runtime Authority snapshot（observe_
   execution）；adapter 行=执行侧原始证据（epoch=声明 epoch）。理由：
   NewEpoch 后 adapter 行 epoch 会滞后于 Domain 真值——裁决面恒以 Domain
   为准（单一 SoT）。
3. **锚公式**（SIM-01 F5 同构）: `program_anchor=出口实测PTS+active分支
   节拍`, `source_anchor=target分支PTS+target节拍`——offset 仍只在
   Authority declare 单点产生。
4. settle 停滞超时**不** FailClosed（停滞=Observation 事实归 watchdog/
   Gate 故障面；时间线证据已闭合）——与 ⑤-⑧ 证据超时 FailClosed 区分。

### 17.3 验证（盒 10.30.15.10，2026-09-04；⑬ 双轨回归）

- fmt --check OK；default **217** / mock **381**（+4: no_evidence epoch、
  段历史锁、runtime 全链 ×2）/ bmd+gst **237**（+1）全过 0 fail；
  clippy ×2 `-D warnings` PASS。
- **Mock 轨**：`timeline_rt_02_runtime_switch_program_full_chain_preserved`
  （Runtime ①-⑩→Preserve(epoch 不变)+Desired 落定+双行一致）+
  `timeline_rt_02_runtime_switch_aborts_timeline_on_backend_failure`
  （switch 失败→abort 回 Stable+恒等段诚实投影）。
- **GStreamer 轨**：`switch_graph_rt_02_timeline_full_chain_real_gstreamer`
  ——**真实元素/真实流线程/真实 input-selector/真实探针**（Simulation
  形态）Runtime 全链 2.18s **Preserve**：V/A 双平面 Continuous、无未声明
  回退、出口沿映射轴推进、Desired=Observed——**SIM-01 结论（F2/F5/F6）
  在生产 switch_graph 实证成立**（mock≠GStreamer 风险收口）。
- gstreamer-rs 0.23 实锚：`Buffer::make_mut()` 返回 `&mut BufferRef`
  （非 Result——SIM-01 时代记忆修正）；EVENT probe `info.event()`/
  `ev.type_()==EventType::Segment` 可用。

### 17.4 未做/边界

真机复跑（步骤 14）见 §18；`recover`/Supervisor/SessionManager/Resolver/
PortRegistry/ResourceRegistry/MediaTap/C1/switch_execution.rs 全零触碰；
identity/send_event/readback/`install=TimelineMapped` 全禁做清单遵守。

## 18. 步骤 14：真机 A2-8-02-I 复跑（2026-09-04 22:15 CST，Batch 2 后首跑）

### 18.1 执行纪律（§29.2/§28.3）

- 证据头：date 2026-09-04 22:14 CST / date -u 14:14 UTC / timedatectl
  Asia/Shanghai（**同日无 host clock mismatch**）；repo HEAD=**3ff66ad**
  （已 push；盒非 git checkout——逐文件 sha256 **68/68 .rs 全等** 于 HEAD）；
  bin `media-agent-gates` sha256 `31e294f4…`（build 后复核）；manifest=v5
  （`~/a2-8-02i-v5.manifest.json`，当日 L1a 2/2 production_grade 复核绑定
  仍成立）。
- 硬件前置核验：ball 测试源常驻（PID 992634, 7h17m, dn=2 sink **勿杀**）；
  `gst decklinkvideosrc dn=0/dn=1` 双卡可开且出帧（信号在）。观察事实：
  ffmpeg decklink 输入打不开（"No such device or address"）——ffmpeg 仅曾
  作 correlation 工具，非 gate 依赖（gate 走 gst/resolver）。
- 证据归档：盒 `~/a2-8-02i-evidence/2026-09-04-2230-batch2-ctimeline/`
  （header.txt + run.log 9688B, sha256 前缀 `5758c42d`）。

### 18.2 结果（EXIT=2, 8/10 verdicts, 全链 L1→L5+Teardown 完成）

| 层 | 判定 | 关键证据 |
| --- | --- | --- |
| L1a | PASS | 2/2 production_grade（v5 绑定当日复核成立） |
| L1b | PASS | 双卡 Sdi/input Supported（audio=video 推导注记） |
| **L1c** | **PASS** | **双卡 signal=true（dn0/dn1——C1 后再次真机成立）** |
| L1d | PASS | 双设备唯一 InputResource+ID 对应（H2 闭环） |
| L2a | PASS | 双输入 session；H3 精确 port 定位 |
| L2b | PASS | 双 tap 各 80 帧 |
| L3 | PASS | video 120→210 / audio 160→280, prog ValidMonotonic |
| **L4** | **FAIL（单失败项=判据转写）** | **switch_ok=true**（completed/observed=B/epoch=1/
ValidMonotonic/pts 在场）∧ **timeline_ok=false——但 outcome=Preserved**：offset=118799ns（相位级, F1 预测同源）·Segment(B) 观测·首枚映射缓冲 source 6971476040→mapped 6971594839（==f(source) 过 Authority 校验）·V/A **双平面 Continuous**·无未声明回退·DiscontinuityDeclared·**epoch 保持 0（Preserve）**·settle 后 authority 位置 7138261506 持续推进 |
| L5 | SKIPPED BY H1 | L4 FAIL → 注入序列跳过（设计性） |
| Teardown | PASS | Program Stop→Tap Detach→Input Stop→Release |

### 18.3 决定性事实

1. **A2-8-01 架构硬事实的真机表达（prog NonMonotonic 确定性签名）已消失**
   ——post-switch prog state=**ValidMonotonic**（02-I 历次 L4 FAIL 根因
   不复现）。Timeline 层在**真实 DeckLink 双输入**上全链成立：
   声明→pre-flip 安装→翻转→Segment(B) 自然事件→首枚映射缓冲（声明
   offset 施加+证据校验）→V/A 双平面连续→Preserve（epoch 不变）。
2. **L4 唯一失败项=九项合取中 `mapped_program_pts > pre_v` 的严格大于**：
   真机首枚 B 帧落位与 A 末帧时间线位置**精确相等**（6971594839 ==
   6971594839, 零隙拼接; 两帧共享同一 PTS 值——非回退）。机制: 翻转
   时刻 selector 已在途的 B 帧≈branch 锚帧位; 锚公式结构性保证
   `mapped ∈ [pv, pv+delta]` **恒不回退**（等节拍下 dA−dB 相消）。冻结
   语义（PtsMonotonicity "非回退 = pts≥last"）下相等=合法连续——
   健康弧 equal→保持 ValidMonotonic ✓, Authority `>=`→Continuous ✓,
   均与 Gate 转写的严格 `>` 不一致。

### 18.4 分类（首跑 FAIL 留证纪律——**未改任何代码**）

- **B 类 Gate 判据转写**：`dual_input.rs` L4 项 `ev.mapped_program_pts >
  pre_v` 应为 `>=`（对齐冻结"非回退"语义）。非 A 类（硬件全好: 双信号/
  双绑定/桥/出口全 PASS）；非 C 类（实现无缺陷——Preserved+双平面
  Continuous 真机达成; 失败在验收转写层）。
- **修正待用户裁决后执行**（单字符 `>`→`>=`）→ 复跑预期 L4 PASS→L5
  注入序列首次真机执行。
- 已知工件（隔离队列不变）：gst_video_converter interlace 断言每跑出现
  （1080i25 电视源）；teardown `gst_pad_unlink` CRITICAL ×4 同历跑。

### 18.5 02-I 状态

**仍 FAIL-PENDING-CORRECTION（8/10）**——但性质迁移：由"架构缺口
（L4-TIMELINE 不可证明）"变为"验收判据单点转写（B 类）"。A2-8 Switch
Execution + Program Timeline Continuity 基础能力在真实双输入硬件上
**执行级达成**（Preserve 证据链完整）；正式 PASS 待 B 类修正+复跑
（含 L5 首次真机注入序列）。
