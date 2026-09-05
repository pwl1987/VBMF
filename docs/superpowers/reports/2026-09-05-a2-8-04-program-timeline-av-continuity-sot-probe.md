# A2-8-04 Program Timeline / AV Continuity — SoT 探针（零代码）

状态: **PROBE DELIVERED / OQ-T1..T6 待用户裁决** —— 裁决前零实现;
observation only 无 Engine（tasks item 6 原文约束）。
授权来源: R49 用户终裁（OQ-R1..R5 关闭后"直接开 A2-8-04"）;
开发线=comet/a2-8-dual-input-switch@44a32bb..（本探针为其后独立单元）,
master=7745968 勿混。

## §1 任务定义锚（tasks item 6 原文, 不重释）

- "六路 PTS before/after switch 无 rollback/discontinuity/divergence/
  starvation; Program Timeline Continuity / Timestamp Normalization 方案
  裁决与验证（observation only，无 Engine——方案设计裁决属 02/04）"。
- 方案裁决已由 C-TIMELINE-01 完成并实现（B=Source Segment Mapping 主+
  A=identity single-segment 执行机制; 真机 L4 Timeline 三连续 PASS）——
  本探针**不重开四方案**, 只做验证形态探查。

## §2 As-Is 实锚（"六路"的代码形态——实读发现, 非类推）

- **六路=TimelineSample 六 PTS 流**: input/bridge/program 三列 ×
  video/audio 双平面（program_execution.rs:66-77）。每路独立
  PtsMonotonicity 四态: Unknown / ValidMonotonic / DiscontinuityDeclared /
  NonMonotonic（pipeline.rs:263+; DiscontinuityDeclared 仅经
  observe_*_pts_declared 产生——C-TIMELINE-01 Freeze §14 语义）。
  三列各自独立测量点 join（assemble_timeline_sample, :83-118: 输入=
  PipelineHealth 弧; 桥=BridgeObservation 行; 程序=ProgramObservation）。
- **六路已在 gate L4 同采（只测量）**: dual_input.rs:586-618 pre/post
  switch 双输入各一行（pre_a/pre_b/post…）; print_row 全六路 PTS+六态+
  program_alive 成行打印; program 列整图共享。
- **现判据面（L4, 冻结——Batch 1 "L4 判据零变化"）**: L4-SWITCH
  （Desired=Observed 落定, :635）+ L4-TIMELINE 九项合取（:650-671,
  program video 主导: program_video_pts_state≠NonMonotonic·mapped>=pre·
  post>=mapped·discontinuity_state·v/a PlaneContinuity·epoch/seg/
  src_pts/mapped/offset 证据链）——**其余五路=只测量不判**。
- **组级折叠既有检出面**（watchdog.rs:398 execution_group_observe_fold,
  纯函数）: av_paired=video_active==audio_active（**pad 级 A/V 分离可检
  出**, 测试 group_fold_rt_01_av_divergence_detected）; pts_monotonic=
  输入双平面全 ValidMonotonic; program_alive（fold 版）=双平面 PTS 在场
  且均未回退; input_pts[].stalled（contracts/switch.rs:44-52——帧计数
  冻结=Observation 事实非故障结论）。
- **timeline 证据面**: TimelineTransitionEvidence video/audio
  PlaneContinuity（Unproven/Continuous/DeclaredDiscontinuity/Violated,
  program_timeline.rs:158-166/:191-193）+ discontinuity_state。
- **真机基线**: L4 Timeline 三连续 PASS（Preserve·epoch 0·offset
  118799ns·mapped==pre_v 精确相等·v/a=Continuous）; sim F5 双平面
  121/121+162/162。

## §3 四失败模式 × 现有面映射（缺口如实, 不自行补模型）

| 模式 | 现有面 | 缺口 |
|---|---|---|
| rollback | 六路各 PtsMonotonicity::NonMonotonic 独立态 | 仅 program_video 入判据; 其余五路未判（判据可从现有态直接派生, 零新观测面） |
| discontinuity | DiscontinuityDeclared 合法边界区分 + program 双平面 PlaneContinuity | input/bridge 平面无 PlaneContinuity 级证据面（仅 PtsMonotonicity——是否足够=裁面） |
| divergence | av_paired（**pad 分离**语义: video/audio 活动 pad 不一致——已检出+已测） | **PTS 时序漂移（A/V PTS delta）=零观测面**; 两语义未消歧; 容差/界限数据不存在（真机同源 8-10ms 仅日志级观察） |
| starvation | stalled 旗 + program/input progress_since（帧增量, R45 已裁语义）+ bridge alive_in_window（观察时钟, G/H-1） | "before/after switch 窗口内六路各自无饥饿"的验收判据形状未定义（窗口语义/阈值归属未裁） |

## §4 OQ 待裁（OQ-T1..T6）

| OQ | 问题 | 提案默认 |
|---|---|---|
| OQ-T1 | 六路判据落点: 扩展 L4（触碰"L4 判据零变化"冻结, 需解冻授权）vs 新增独立验收节（gate 内新层/新函数, L4 十项不动; 参照 L4-SWITCH/L4-TIMELINE 记账拆分先例） | **新增独立验收节** |
| OQ-T2 | divergence 语义: pad 分离（av_paired 已覆盖）vs PTS 时序漂移（需新纯观测字段）vs 两者分开记账; 漂移界限=首版只测量+真机分布取证后再裁（禁拍脑袋容差） | 两者分开; 漂移首版只测量 |
| OQ-T3 | starvation 判据: 复用 progress_since 帧增量窗（R45 已裁采样增量语义）vs 新观察时钟窗 | 复用 progress_since |
| OQ-T4 | input/bridge 平面是否需要 PlaneContinuity 级证据面 | 首版 PtsMonotonicity 四态足够 |
| OQ-T5 | 合成谓词形状: 六路×四模式通过态如何组合为验收结论（全路 ValidMonotonic\|DiscontinuityDeclared+无饥饿+漂移在界?） | 待裁（探针不预设） |
| OQ-T6 | 与 C-TIMELINE-01 冻结关系确认: 纯验证不新增映射语义（B 已实现） | 确认不重开 |

## §5 红线继承

observation only 无 Engine; **L4 冻结判据零触碰**（解冻需用户明示授权）;
PtsMonotonicity 四态禁洗; sampled_at_ms 禁修 PTS; Timeline liveness 观察
时钟分层不混（与 PTS 严格分离）; 首跑 FAIL 留证禁为跑绿改判据; 已 PASS
各层+九组件不动; 盒矩阵纪律（fmt→default→mock→bmd+gst→clippy×2→bins;
gates 真机复跑前必须 cargo build --bin media-agent-gates; cargo 全经盒
SSH 10.30.15.10）。

## §6 最小变更面预估（未授权, 待 OQ 裁后细化）

gate dual_input.rs 新增独立六路验收节（纯判据函数+证据行打印, 不动 L4）+
TimelineSample 可能新增 A/V delta 纯观测字段（仅 OQ-T2 裁"漂移"才需要）+
tests; runtime 判决面/watchdog/supervisor/timeline 执行面零触碰。
