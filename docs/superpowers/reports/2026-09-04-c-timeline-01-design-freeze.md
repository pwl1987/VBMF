# A2-8-C-TIMELINE-01 Design Freeze — Program Timeline Authority & Source Segment Mapping

- 日期：2026-09-04；分支 `comet/a2-8-dual-input-switch`；基线 1db28e2 之上（零代码）。
- 权威来源：用户十问终裁（2026-09-04），全文落账于设计探针 §11
  （`2026-09-04-c-timeline-01-program-timeline-authority-design-probe.md`）。
  **本文件为整理稿；与 §11 终裁原文如有出入以终裁原文为准。**
- **修订记录**：2026-09-04 第三十二轮终裁（设计探针 §16.2.1）——§3
  program_epoch 语义统一为 **Preserve=同世代不变 / NewEpoch（及 Hard
  Recover 重建）=+1**，消除与 Batch 1 实现（f82e625）的双 SoT；修订仅
  涉及 §3 与其衔接表述，其余 14 项零变化。
- 冻结效力：本文 15 项 + 八红线 R1-R8 + OQ-1..12 裁定即 C-TIMELINE-01
  冻结设计面。implementation change 以本文件为设计 SoT 开工；实现期
  任何与本文冲突的选择**必须停下回裁，不得由开发助手自行裁定**。
- 状态：Design Freeze 已形成；**不进入实现**；下一动作=开
  `A2-8-C-TIMELINE-01` implementation change（正常 change 流程）。

## 0. 架构总图与四方案裁定

冻结模型（终裁照录）：

```text
                   ProgramExecutionRuntime
                            │
             ┌──────────────┴──────────────┐
             │                             │
      SwitchExecution               TimelineAuthority
             │                             │
       switch_epoch                  program_epoch
             │                             │
             └──────────────┬──────────────┘
                            │
                     Timeline Plan
                            │
             ┌──────────────┴──────────────┐
             │                             │
         Source A                      Source B
         Segment A                    Segment B
             │                             │
             └──────────────┬──────────────┘
                            │
                      Program Timeline
                            │
                  ┌─────────┴─────────┐
                  │                   │
                Video               Audio
                  │                   │
                  └─────────┬─────────┘
                            │
                     GStreamer Adapter
                            │
                Segment/Event/PTS mapping
                            │
                     Program Graph
                            │
                          appsink
                            │
                    Timeline Evidence
```

四方案裁定（OQ-11，终裁照录）：

| 方案 | 终裁 | 结论 |
| --- | --- | --- |
| A 切后 Regenerator | 🟡 部分采用 | 采用"切换后重新建立 Source Segment Mapping"的机制，但不是独立 Regenerator |
| B Clock-Segment Timeline | 🟢 核心采用 | C-TIMELINE-01 主方案 |
| C 出口 normalization | 🔴 不采用 | "出口再生成"概念正式废止 |
| D 切换新 timebase | 🔴 不采用 | 与 Program Timeline Continuity 目标冲突 |

总体结论：**B 为主 + A 的执行机制 + 明确排除 C/D**。核心不是"把
PTS 改大"、不是出口重新生成、更不是每次切换创建全新 timebase。

现状锚（演进起点，非改动授权）：`ProgramExecutionRuntime` 已存在
（program_execution.rs:209，Inner{group, switcher, graph, taps,
tap_port, watchdog_stop}）——终裁结构=在其上增设 TimelineAuthority
组件；Bridged program graph 现状=inter[video/audio]src →
input-selector → queue → appsink（switch_graph.rs:217-241/:275-306），
**零 clock/base_time/timeline 层**（§2.1 探针已证）。

## 1. TimelineAuthority Domain Contract（冻结项①）

- **Owner 层级**：Program Execution 层的 Timeline Authority——不放
  ExecutionGroup、Supervisor、MediaBackend、单独 Pipeline、纯出口
  muxer（R3/R4/R5）。
- **形态约束**：不做"大型独立 Engine"（否则制造新的架构中心）；
  目标结构 = `ProgramExecutionRuntime{SwitchExecution,
  TimelineAuthority, ProgramGraph, Observation}` 四组件并列。
- **分层链**：`TimelineAuthority → ProgramTimelinePlan → GStreamer
  Execution Adapter → GStreamer segment/timestamp/pad execution`。
  **Domain 层拥有"时间线语义"，Adapter 层拥有"怎么让 GStreamer
  做到"**（R6）。
- 现有组件职责不变：ExecutionGroup 保持纯 Desired 状态机（恰
  {session_id, inputs, desired, switch_epoch} 零时间戳，
  switch_execution.rs:93-100）；SwitchExecutionAdapter 保持 Program
  graph 物化/切换/观测 owner（contracts/switch.rs:84 起）。

## 2. ProgramTimeline（冻结项②）

- Program Timeline 是 **Program Execution 层的独立媒体时间线权威**：
  一条连续轴；所有进入 Program 的 buffer 的 Program PTS 由该时间线
  语义定义（经 Source Segment mapping 得出）。
- 它**不是** wall-clock（R1）、**不是**任一输入源自身的时钟、
  **不是** Channel reference clock（OQ-12 三时钟职权切开：Timeline
  Authority=Program PTS 应该是什么；AVSync Manager=V↔A 相对关系；
  Channel Reference Clock=observation calibration，后两者均不能成为
  Program PTS generator）。
- 每个 Source 进入 Program 时被映射到 Program Timeline；切换通过
  新 Source Segment 建立连续映射（见 §4/§5）。

## 3. ProgramEpoch（冻结项③）

- 语义：**媒体语义时间线世代**（区别于执行事件计数）。
- 三计数器职权分离（第三十二轮终裁修正, 2026-09-04；原文"一次成功
  program switch → program_epoch 推进（起始对应 SwitchEpoch 1 →
  TimelineEpoch 1）"废止——该口径使 ProgramEpoch 退化成另一种 switch
  counter，破坏 ProgramEpoch≠switch_epoch 的初衷）：

```text
switch_epoch  = execution event count（执行事件计数, ExecutionGroup 所有）
segment_id    = source segment generation（每次切换段世代 +1）
program_epoch = discontinuous program timeline generation
                Preserve（连续性成立）   → 同世代不变
                NewEpoch（连续性不可证） → program_epoch + 1
                Hard Recover（重建）     → program_epoch + 1（§13）
```

- A→B 切换且 Program Timeline 连续成立（Preserve）时：`ProgramEpoch=N
  保持, SegmentId=N+1` 完全合理——段世代推进表达"发生了段切换"，epoch
  不变表达"时间线世代未断"。
- 与 `switch_epoch` 的关系：**不复用、不同物**；两状态机经
  `SwitchExecutionPlan`/`SwitchExecuted` 做关联但各自拥有自己的状态。
- recover 可以只变 program_epoch 不发生业务 source switch（见 §13）。
- Video/Audio **共享同一 Program Epoch**（同一次切换同一 epoch），
  但不共享 PTS 数值序列（见 §9）。

## 4. SourceSegment（冻结项④）

- 结构（终裁照录）：

```text
SourceSegment {
    source_id
    program_epoch
    source_start_pts
    program_start_pts
    offset
}
```

- 新段映射语义（终裁照录）：`mapping_B = Program continuity anchor
- Source B continuity anchor`——保存的是 Segment 结构，不是
  `last_program_pts` 单值。
- 每个进入 Program 的源恰由其 Segment 描述；系统必须能回答：
  **"这个 Program PTS 是由哪个 Source、哪个 Segment、经过什么
  mapping 得来的？"**——否则 recover / discontinuity / A→B→A /
  clock drift / encoder restart 都会再次失去语义基础。

## 5. TimelineMapping（冻结项⑤）

- 段内映射：`program_pts = f(source_pts, segment mapping)`（offset
  mapping；具体函数形态=implementation change 按本语义细化）。
- **永久禁止**（R2）：`max(last_pts + duration, incoming_pts)` 类
  "PTS 不回退"假闭合——PTS 连续性是映射问题，不是数字大小修复
  问题。
- **永久禁止**（R1）：用 wall-clock（含 sampled_at_ms/observed_at）
  修 media PTS。
- Video 与 Audio 各自独立 mapping（见 §9），是否同函数=实现细节，
  不得引入跨平面耦合语义。

## 6. Discontinuity（冻结项⑥）

- **双层表达**：Domain Discontinuity 声明在前，GStreamer
  Segment/Event 为执行载体在后（`TimelineAuthority → {Program
  Segment, Discontinuity declaration} → GStreamer Adapter → Gst
  Segment/Event`）。**Gst Segment Event 不是 Authority**（R6）。
- PtsState 四态（终裁照录）：

```text
PtsState
 ├── Unknown
 ├── ValidMonotonic
 ├── DiscontinuityDeclared
 └── NonMonotonic
```

- **必须冻结的区分**：`declared discontinuity + expected PTS
  transition ≠ unexpected backward PTS`——前者=系统知道发生了合法
  时间线边界；后者=观测到不符合当前连续性规则的事实。禁把
  NonMonotonic 简单改成 ValidMonotonic，也禁把两者混同。
- 现状锚：PtsMonotonicity 三态（Unknown/ValidMonotonic/NonMonotonic，
  pipeline.rs:236-246）只测量不声明；四态=实现期演化（与 §14 L4
  判据联动）。

## 7. TimelineMapped Execution Fact（冻结项⑦）

- 必须有 Execution Fact；**不得是裸 bool** `normalized = true`。
- 结构（终裁照录）：

```text
TimelineMapped {
    program_epoch
    source_id
    segment_id
    mapping
    evidence
}
```

- **TimelineMapped ≠ TimelineHealthy**（R8）：完成映射是 Fact，
  时间线健康是另一判定，两者必须分离。
- 衔接既有 fact 纪律：Intent≠Fact（`PipelinePlan.normalize=true`
  ≠已 normalize）；fact absent ≠ fact=false；否声明性推进。

## 8. TimelineEvidence——Observation 证明面（冻结项⑧）

- **不修改现有 ProgramObservation 来承担一切**（contracts/switch.rs
  :56-77 保持既有职责）；建专门 Timeline 证据对象（终裁照录）：

```text
TimelineObservation {
    program_epoch
    source_id
    segment_id
    input_pts
    mapped_program_pts
    mapping_offset
    discontinuity_state
    video_continuity
    audio_continuity
    observed_at
}
```

- **observed_at = wall clock（观察层），绝对不能用于计算
  program_pts**（R1）。
- "真的 normalize 了"的可观测定义（≥以下 7 条，终裁照录）：
  1. Program PTS 连续；
  2. Source→Program mapping 与 declared segment 一致；
  3. video continuity；
  4. audio continuity；
  5. epoch 一致；
  6. segment transition 符合声明；
  7. 没有未声明 backward jump。
- **单纯 `pts > previous_pts` 永远不足以证明 normalize 成功**。

## 9. Video/Audio 双平面规则（冻结项⑨）

- 同一次切换、同一个 Program Epoch；**不共享一个数值序列**：
  Video Segment(epoch=N)+video mapping，Audio Segment(epoch=N)+
  audio mapping——各自独立 PTS、各自独立 media clock。
- A/V 相对关系由 AVSync 负责验证（OQ-12 职权边界），Timeline
  Authority 不做 A/V 相对校正、AVSync 不决定 Program PTS。

## 10. Switch→Timeline 状态转移（冻结项⑩）

- 状态机（终裁照录）：

```text
Stable(A)
   ↓
SwitchRequested(B)
   ↓
SwitchExecuted(B)
   ↓
TimelineTransition(B)
   ↓
Stable(B)
```

- settle 语义 = **TimelineTransition 段**：timeline mapping 已生效，
  但稳定性尚未完成确认。settle 期间 Program PTS **必须已经属于新
  Program Timeline**——禁继续假装属于 A、禁等 settle 结束才开始 B、
  禁暂停 PTS、禁用 wall-clock 补时间。
- `settle ≠ timeline gap`、`settle ≠ timestamp freeze`。此定义对
  L5（故障注入/recover 验证）非常重要。

## 11. GStreamer Adapter Execution Contract（冻结项⑪）

- 输入=ProgramTimelinePlan（Domain 产出）；输出=GStreamer
  segment event / timestamp / pad 层执行。**Segment/Event/PTS
  mapping 是 Execution Adapter 机制，不是 Domain Authority**（R6）。
- 承载面=Bridged program graph（switch_graph.rs；现状零
  clock/base_time/timeline 层，capsfilter Bridged=>None :231）。
- **SPI 具体签名**（build_program_graph 演进 vs 新方法 vs
  ProgramTimelinePlan 经何路径入 Adapter）=implementation change
  首刀设计问题，本冻结不预裁；只冻结上述分层与职权。

## 12. 1080i/1080p 当前边界（冻结项⑫）

- **Timeline 层不承担格式归一化**（R7）——PTS continuity 与 format
  continuity 解耦（OQ-4）。
- 当前 A2-8 阶段=**Switch Boundary Adaptation**：异构输入允许进入
  Timeline 设计，但 **Program Format Contract 必须显式声明"当前
  不保证无缝 format continuity"**。
- deinterlace / frame-rate conversion / pixel format / resolution
  conversion=**独立 Program Media Format Policy**（未来独立裁决），
  不得因 C-TIMELINE-01 顺手塞入。
- converter interlace 断言工件（真机每跑恰 9 条，未定性）=隔离
  队列，不趁修、不并入本设计。

## 13. Recover 接口语义（冻结项⑬——本轮不实现）

- 语义冻结：Recover 后**不得简单继承旧 Timeline 状态**——

```text
Recover → 新 execution instance → Timeline reconstruction
        → 新 source segment → Program timeline continues
```

- 两类（终裁照录）：**Soft Recover**（execution 重建、timeline 可
  连续）/ **Hard Recover**（timeline continuity 无法证明 → 新
  ProgramTimeline epoch）。
- **Supervisor 只决定 recover，不拥有 Timeline**（R4）。接口留给
  A2-8-03（failure/supervision），本设计不提前侵入。

## 14. L4 如何重新证明（冻结项⑭）

- 前提：implementation change 完成 + 按 §29.2 纪律真机复跑（证据
  头五件套 / 冻结 bin 复核 / 当日 Discovery 核验）。
- L4-TIMELINE 判据从现行"prog pts state≠NonMonotonic ∧ pts.is_some"
  （dual_input.rs:644-648）演化到基于 TimelineObservation 的证明
  （§8 七条）；**DiscontinuityDeclared + expected transition = 合法
  时间线边界**，不计为未声明回退。
- Gate 表面改动属 implementation change 范围（须按既有纪律走授权/
  五段门；本设计不预先改 Gate）。
- 状态纪律：**设计存在 ≠ Gate PASS**；02-I 保持
  FAIL-PENDING-CORRECTION 直至复跑证据支持。

## 15. 不变量与失败条件（冻结项⑮）

八条红线 R1-R8（终裁照录）：

1. **R1** 不得用 wall-clock 修 PTS。
2. **R2** 不得用 `max(last + duration, incoming)` 伪造连续性。
3. **R3** Timeline Authority 不进入 ExecutionGroup。
4. **R4** Timeline Authority 不进入 Supervisor。
5. **R5** Timeline Authority 不进入 MediaBackend。
6. **R6** GStreamer Segment/Event 是 Execution Adapter 机制，不是
   Domain Authority。
7. **R7** 1080i/1080p 格式转换不由 Timeline Authority 偷做。
8. **R8** Normalization Fact 与 Timeline Healthy 必须分离。

继承冻结面（不因本设计失效）：设计探针 §9 七不变量（Intent→Plan→
Fact 三段；三列观测只测量；observation clock 与 media PTS 严格分离；
Pipeline 不知 A/B/Program；切换执行语义不动；H1/L4 证据原则/Gate
表面不动；词表纪律）全部继续有效。

不触碰清单（终裁照录）：L0、L1a、L1b、L1c、L1d、L2、L3、L4-SWITCH、
Teardown、SwitchExecution、SessionManager、Resolver、PortRegistry、
ResourceRegistry、Supervisor——**只解决 L4-TIMELINE**。

失败条件（实现期回裁触发）：任何与 R1-R8 / OQ-1..12 裁定 / 本 15 项
冲突的实现选择=停下回裁。

## 16. PipelinePlan.normalize 处置（OQ-10 落地路径）

- 冻结：**删除裸 `normalize: bool`**（违反词表纪律"裸 bool 禁"）；
  方向 `PipelinePlan → TimelinePolicy`（明确语义声明），而非
  `normalize = true`。
- **本轮设计冻结前/冻结时零代码**；字段删除与 TimelinePolicy 建立
  =implementation change 的一部分。

## 17. 隔离清单（禁趁本设计顺手修）

C1-P1（signal polling 窗口 bus error 未二次 drain——独立 P1，不重开
C1，修复面=poll iteration 内可选 bus check，禁重新设计 Resolver）·
converter interlace 断言定性 · 现场项（BNC#4 对端/dn2→Mini 线缆/照片，
用户侧）· PORT-IDENTITY-AND-RESOURCE-ADDRESSING · canonical UUID
namespace · A2-8-03/04/05。

## 18. 开工条件（implementation change 入口）

- 本 Design Freeze 为设计 SoT；OQ-1..12 已全裁（探针 §11）。
- 开 `A2-8-C-TIMELINE-01` implementation change 走正常 change 流程；
  实现期五段门 / CI 全矩阵 / 真机 Gate 复跑纪律照旧（§29.2）。
- 在该 change 授权落地前，**任何 normalization 实现代码不动**
  （含 C1-P1——保持隔离）。
