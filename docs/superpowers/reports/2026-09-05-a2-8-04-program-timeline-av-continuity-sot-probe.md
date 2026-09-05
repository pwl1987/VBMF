# A2-8-04 Program Timeline / AV Continuity — SoT 探针（零代码）

状态: **OQ-T1..T6 已裁（R50, 2026-09-05, 修订后冻结——全文见 §7）** ——
下一动作=六路实际证据采集（observation only; 其最小观测面实现按 §7
冻结边界单独落地, 先取证后判据）; observation only 无 Engine
（tasks item 6 原文约束）。
授权来源: R49 用户终裁（OQ-R1..R5 关闭后"直接开 A2-8-04"）+
R50 用户二层代码真相审计终裁（T3/T4/T5 修订后冻结）;
开发线=comet/a2-8-dual-input-switch@639b0f3..（R50 两单元）,
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
- **【R50 校准】生产 stalled 恒 false**: InputPts.stalled 结构字段在
  （contracts/switch.rs:52）, 但生产 GStreamer observe() 硬编码
  `stalled: false`（switch_graph.rs:936; Mock=真实 stalled 状态
  switch_mock.rs:84——Mock≠GStreamer 契约面又一例）——**结构字段在≠
  生产证据在**。
- **【R50 校准】progress_since=聚合 A/V"或"语义**:
  program_progress_since/input_progress_since 均
  `video 增量 OR audio 增量`（program_execution.rs:160-174）——只证
  "该列整体在推进", 不证逐平面推进。
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
| discontinuity | DiscontinuityDeclared 合法边界区分 + program 双平面 PlaneContinuity | input/bridge 平面无 PlaneContinuity 级证据面（仅 PtsMonotonicity——是否足够=裁面）; **【R50 校准】adapter 证据行在 first_mapped 时无条件 DiscontinuityDeclared+video_continuity 硬编码 Continuous（switch_graph.rs:969-984; switch_mock.rs:434 同构+测试 :861-869 锁定）——与 Freeze"声明边界≠transition 本身; Preserve=连续"不完全一致=correctness 缺口（主账 §65.4, 不本轮修）** |
| divergence | av_paired（**pad 分离**语义: video/audio 活动 pad 不一致——已检出+已测） | **PTS 时序漂移（A/V PTS delta）=零观测面**; 两语义未消歧; 容差/界限数据不存在（真机同源 8-10ms 仅日志级观察）; **【R50 前置】V/A PTS 全链 ns 单位（ClockTime::nseconds）=可比性成立≠阈值授权** |
| starvation | stalled 旗（**R50 校准: 生产恒 false, switch_graph.rs:936——结构在≠证据在**）+ program/input progress_since（**R50 校准: 聚合 A/V"或"语义, program_execution.rs:160-174**）+ bridge alive_in_window（观察时钟, G/H-1） | "before/after switch 窗口内六路各自无饥饿"的验收判据形状未定义; **逐平面 progress 证据面=无（六路 starvation 现不可证）** |

## §4 OQ 汇总（R50 终裁——原文问题与提案默认保留, 终裁全文见 §7）

| OQ | 问题 | 提案默认 | 终裁（R50） |
|---|---|---|---|
| OQ-T1 | 六路判据落点: 扩展 L4（触碰"L4 判据零变化"冻结, 需解冻授权）vs 新增独立验收节（gate 内新层/新函数, L4 十项不动; 参照 L4-SWITCH/L4-TIMELINE 记账拆分先例） | **新增独立验收节** | 🟢 **接受（独立验收面; L4 冻结表面零改动）** |
| OQ-T2 | divergence 语义: pad 分离（av_paired 已覆盖）vs PTS 时序漂移（需新纯观测字段）vs 两者分开记账; 漂移界限=首版只测量+真机分布取证后再裁（禁拍脑袋容差） | 两者分开; 漂移首版只测量 | 🟢 **接受（附前置: ns 可比性成立≠阈值授权）** |
| OQ-T3 | starvation 判据: 复用 progress_since 帧增量窗（R45 已裁采样增量语义）vs 新观察时钟窗 | 复用 progress_since | 🔴 **原案拒绝·修订后接受（逐平面证据+观测先行, 见 §7）** |
| OQ-T4 | input/bridge 平面是否需要 PlaneContinuity 级证据面 | 首版 PtsMonotonicity 四态足够 | 🔴 **原案拒绝·修订后接受（四态已在; 缺口=生产语义过宽→correctness change, 见 §7）** |
| OQ-T5 | 合成谓词形状: 六路×四模式通过态如何组合为验收结论（全路 ValidMonotonic\|DiscontinuityDeclared+无饥饿+漂移在界?） | 待裁（探针不预设） | 🟡 **修订后冻结（六路×四模式证据矩阵, 禁合成大布尔）** |
| OQ-T6 | 与 C-TIMELINE-01 冻结关系确认: 纯验证不新增映射语义（B 已实现） | 确认不重开 | 🟢 **接受（+实现↔Freeze 一致性验证职责）** |

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

## §7 OQ-T1..T6 终裁（R50, 2026-09-05 用户二层代码真相审计; 修订后冻结）

### 7.1 裁决总表（用户 §20 终裁表 + 本轮代码核验实锚）

| OQ | 终裁 | 依据/实锚 |
|---|---|---|
| T1 | 🟢 接受（冻结） | 独立 A2-8-04 验收面; **L4-SWITCH/L4-TIMELINE 冻结表面零改动**——不偷偷扩大旧 L4 判据（C-TIMELINE 已定 L4-SWITCH 不动、L4-TIMELINE 按 TimelineObservation 演进） |
| T2 | 🟢 接受（冻结, 附前置） | **D1 平面/pad 结构性分离**（av_paired, watchdog.rs:456-461, 已检出+已测）与 **D2 PTS 时序漂移**分账——二者非同一证据; PTS 全链 ns 单位（ClockTime::nseconds: controller.rs:560/588/686+switch_graph.rs:180/328/354/503）=V/A 可比性成立**≠阈值授权**; 漂移首版只测量+真机分布取证后再裁界 |
| T3 | 🔴 原案拒绝·修订后接受（冻结） | progress_since=聚合 A/V"或"（program_execution.rs:160-174）只证列整体推进, 不能证六路逐平面 starvation; 生产 InputPts.stalled 硬编码 false（switch_graph.rs:936; Mock 真实 stalled switch_mock.rs:84——结构在≠证据在）。**修订边界=六路 PTS continuity+六路各自 progress evidence+absence≠evidence; 执行序=先观测探针取证六路推进行为（V/A×input/bridge/program）→据实定 starvation window; 禁发明阈值、禁立即实现六路 starvation 判据** |
| T4 | 🔴 原案拒绝·修订后接受（冻结） | **事实核验修正**: PtsMonotonicity 实为四态（pipeline.rs:263-276, Batch 1 落地+Freeze 语义注释在位）——用户审计"当前三态"与此不符, 如实记录（主账 §65.4）。**真缺口=第四态生产使用语义过宽**: adapter 证据行在 first_mapped 时无条件 DiscontinuityDeclared+video_continuity 硬编码 Continuous（switch_graph.rs:969-984; switch_mock.rs:434 同构+测试 :861-869 锁定）——"发生 Segment transition/存在映射"≠"声明了不连续"（Preserve=同 epoch 连续时间线）; 裁决级 Authority snapshot 路径语义正确（program_timeline.rs:616-618 仅声明边界置 Declared）。**处置=登记→A2-8-04 取证→C-TIMELINE implementation correctness change（独立队列, 不本轮修）**; A2-8-04 discontinuity 证据=逐路四态**如实读出**+declared vs unexpected 区分+与 Freeze 语义偏差如实记录（**一致性验证, 非代码修正**）; input/bridge 平面首版不新增 PlaneContinuity 级面（取证发现不足再回裁） |
| T5 | 🟡 修订后冻结 | 验收模型=**六路×四模式证据矩阵**: PathEvidence[6]（V/A-input/V/A-bridge/V/A-program）×FailureMode 证据面（rollback/discontinuity/D1/D2/starvation）, 每格=**证据（E）非布尔**; **absence 与 false 分离**（absence≠evidence 契约全表适用）; **禁预设单一合成大布尔**——各模式证据类型本不同（态读出/漂移测量/推进取证）, 最终验收谓词由验收层在证据矩阵填充后定义 |
| T6 | 🟢 接受（冻结, 新增职责） | 不重开 C-TIMELINE-01 十二 OQ/四方案; A2-8-04=验证/取证/发现实现缺口+**实现↔Design Freeze 一致性验证**（一致性验证≠重新裁决设计） |

### 7.2 §2/§3 As-Is 校准说明（R50）

§2 新增两条校准（生产 stalled 恒 false; progress_since 聚合语义）; §3
discontinuity/divergence/starvation 三行已就地校准并标注【R50 校准】——
原文错误表述（"stalled 面件已在"暗含生产证据在）不保留为结论, 但 §4
原提案默认列保留为裁决前历史（被终裁列取代）。

### 7.3 执行边界与序（用户 §十二/§十五/§二十三-§二十四）

- **现在不写 A2-8-04 判据实现代码**; 六路 starvation 证据面与阈值
  待取证后定——观测先行。
- DiscontinuityDeclared 过宽=C-TIMELINE implementation correctness
  change 队列——A2-8-04 只取证不修; 禁趁收尾轮顺手修。
- C-TIMELINE 文档状态 vs 代码状态=状态校准已做（design-freeze 文档
  §19 附录; 不回滚不重设计）。
- **执行序: 本探针冻结（R50）→ 六路实际证据采集（observation only,
  最小观测面实现按本 §7 边界单独落地）→ 发现 implementation gaps →
  C-TIMELINE correctness change → A2-8-04 Gate**。
- **禁回头重复 R46-R49 旧活体验证**。
- 红线继承（§5）全保持: L4 冻结判据零触碰; PtsMonotonicity 四态禁洗;
  sampled_at_ms 禁修 PTS; 首跑 FAIL 留证禁为跑绿改判据。

## §8 Unit 1 实现 + 六路真机首采（R51, 2026-09-05; observation only 落地）

### 8.1 变更面（4d95ec6; §7.3 边界全守）

- program_execution.rs 新增: `EvidencePhase` / `PathEvidence{pts,pts_state,
  frames,advanced}` / `SixPathEvidence{六路+sampled_at_ms+switch_epoch+
  program_av_delta_ns}` / `SixPathInputs` / `assemble_six_path_evidence`
  ——纯 Domain 只测量; advanced=Option<bool> 三值（None=无可比帧计数 /
  Some(false)=有证据未推进）实现 absence≠false; av_delta=|v−a| 只测量
  （T2: ns 可比≠阈值授权）。**帧计数原料全已在**（健康弧帧计数/
  BridgeObservation.video_frames,audio_frames/ProgramObservation 帧计数）
  ——缺口纯在逐路 join 面, 契约零改。+3 纯函数测试（T3 聚合误报场景锁死/
  absence≠false/epoch 透传）。
- gates/dual_input.rs: L4 判据输入捕获后追加观测节——PRE 对 / SPAN（跨
  切换推进=starvation 证据基础）/ POST 对, 每设备一行; 切换失败=如实缺席。
  零判据零阈值; L4/Supervisor/contracts/MediaTap/watchdog/switch_graph/
  program_timeline 零触碰。
- 盒矩阵全绿（fmt / default 227(+3) / mock 393 / bmd+gst 251 / clippy×2）;
  rt_01 flaky 一次中间跑留证（过滤重跑+全量复跑绿——既有隔离债非本轮
  diff 面）。gates bin 重建须带 hardware features（分发 cfg 门控）。

### 8.2 六路真机首采（2026-09-05 13:15 CST; gate 全链 10/10 按账本首次全绿）

- 证据: ~/a2-8-02i-evidence/2026-09-05-r51-a204-sixpath-observation/
  （header 五件套 + sha==HEAD 4d95ec6 + manifest v5 当日核验[L1a 2/2
  production_grade · L1c dn0/dn1 signal=true]）。
- 数据（PRE/SPAN/POST × {A,B} 每设备行六路）: 全部六路 advanced=Some(true)
  × 全 ValidMonotonic; **SPAN 含被切离 A 路持续推进——跨切换 starvation
  未观测（单次样本）**; **av_delta: 7,149,169ns(pre) → 15,482,503ns(post)
  ——切换后 V/A 差翻倍, 首个 D2 PTS 漂移实测点（只测量, 不设阈值）**;
  program 列整图共享（行间重复=TimelineSample 惯例）。
- L5 全 PASS 但观测窗 B 类候选**不因单次 PASS 复案**（概率性重叠未再现≠
  消除）。工件全为既有隔离债零新增。
- T6 一致性: adapter observe() 行 Declared 过宽原样（Gap B 队列不变）;
  L4 裁决消费 Authority snapshot（语义正确）——分层与 §7.1 T4 一致。

### 8.3 下一步（§7.3 执行序不变）

- 多场景继续采集（多次切换 / 长窗 / 异构 1080i25↔1080p25 format 边界）→
  填 T5 证据矩阵（每格=证据 E, absence≠false）→ 验收谓词待矩阵填充后由
  验收层定义 → C-TIMELINE correctness change（Gap B: adapter 行语义）→
  A2-8-04 Gate。禁发明阈值; L4 冻结维持; 禁回头重复旧活体验证。
