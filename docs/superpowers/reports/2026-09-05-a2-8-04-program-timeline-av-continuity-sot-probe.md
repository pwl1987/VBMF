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

## §9 R52 执行：多场景采集落地 + 真机 20 切换三场景 + pr_v 闩锁首证（主账 §67）

### 9.1 变更面（§8.3 执行序兑现; §7 边界全保持）

- 新 gate `gates/a204_obs.rs`（VBMF_A2_8_04_OBS·第七 env）: N/DWELL 参数化
  交替 A↔B（dwell=0=连续切换; N 调大=长窗）, 每切换 PRE 对/SPAN/POST 对
  复用 §8 R51 投影——**零 Domain API 扩张（R52 §十）**, 无判据无阈值,
  exit=采集完整性; S5 format 行 caps=None 缺席如实（异构事实仍以二十五轮
  canonical closure 为据）。盒矩阵 227/393/**254(+3)**/clippy×2 全绿;
  sha 盒==HEAD e843eba。
- dual_input/program_execution/switch_graph/Supervisor/契约零触碰; Gap B
  未修（R53 队列）。

### 9.2 真机数据（四跑全 EXIT=0; 证据盒 2026-09-05-r52-a204-multi-scenario）

- **20 切换（6+10+4 三 dwell 场景）全 Preserved; av_epoch 1..N 严格递增;
  segment 1..N; ProgramEpoch(0) 全程保持**——Preserve 多切换连续性真机
  首证。dwell=5s/1s/0 三场景 + run4 dual_input 回归 10/10（判据面零扰动）。
- **T5 矩阵填充（每格=证据 E）**: [六路×ValidMonotonic]大面积正证据
  （run1 36×6, run3 24×6, run2 44-60/60）; **[pr_v×NonMonotonic]=16 行
  首证**（run2 switch #8 B→A 边界 per-buffer 单次回退→状态闩锁无复位;
  采样 pts 严格递增+帧推进+Preserved+Authority Continuous 并存——**adapter
  原始行闩锁语义首次真机显形, =R53 correctness change 直接输入证据**;
  pr_a 0/60; dwell 5s/0 场景 0 次: 20 样本 1 次, 概率性如实）; [adv=
  Some(false)]=三跑全 0。
- **D2 av_delta 方向振荡**: B 活跃 ≈15.0-40.1ms / A 活跃 ≈1.7-26.7ms,
  无单调漂移; SPAN≈POST（1/20 例外后回落）——候选解释=两源内在 A/V skew
  不同（i25 电视 vs p25 ball）, 登记非裁决; 阈值仍禁（T2）。
- 工件全为既有隔离债（pad_unlink ×4/PortId ×2/interlace ×3）零新增;
  MainContext WARN 未复现。

### 9.3 下一步（执行序不变）

- **R53 C-TIMELINE correctness（Gap B）**: switch_graph.rs mapped→Declared
  过宽修正 + 测试四锁（mapped+monotonic≠Declared/显式 declaration→Declared/
  真回退→NonMonotonic/Unknown=证据不足）+ **闩锁无复位语义纳入设计输入
  （§9.2 首证）**; 然后全矩阵回归 → A2-8-04 Gate（验收谓词矩阵填充后由
  验收层定义）→ A2-8-05。禁发明阈值; L4 冻结维持; 禁回头重复旧活体验证。

## §10 第五十三轮（R53）: C-TIMELINE correctness 落地——两 face 修正 + 五锁 + 真机分层签名（d1a4fc6）

### 10.1 变更面（§7 边界全保持）

- **仅 switch_graph.rs**（+387/−40; Domain/契约/mock/gates/L4/Supervisor/
  watchdog 零触碰; Observation 结果未入任何控制路径）。
- Fix 1 行装配（Gap B）: `MappedContinuation` 段作用域状态机 +
  `plane_row_state` 四态派生——**Mapped ≠ DiscontinuityDeclared**; V/A 对称。
- Fix 2 arc 生命周期（§9.2 闩锁首证的作用面=pr_v/pr_a ←
  TimelineSample.program_*_state ← 程序图健康弧）:
  `note_declared_boundary` 于段首枚映射缓冲通知健康弧——
  `observe_*_pts_declared`（pipeline 预留 API 首个生产调用者）+ 干净边界
  段基准重开（上一段 NonMonotonic 解除, 闩锁不跨声明边界）+ 违例边界
  NonMonotonic 传播（声明不洗回退）; 段内普通单调帧永不自动恢复。
- 五锁测试入硬件矩阵（254→259）; default 227/mock 393 不变。

### 10.2 真机数据（T5 矩阵更新; 证据盒 2026-09-05-r53-ctimeline-correctness）

- **干净跑新基线格 [pr_v/pr_a×DiscontinuityDeclared]**: run1 34+2·run2
  58+2·run3（N=4）同签名——首切前 PRE=VM, 此后边界事实保持; 全 Preserved·
  ProgramEpoch(0) 保持·adv=Some(false)=0·NonMonotonic=0（20 切换样本）。
- **闩锁解除真机样本未获得**: 概率性边界回退本轮未复现（历史 30 切换 1
  次）; 解除路径由 rt_05 单测确定性锁定——T5 该格="单测证明+真机待样本"
  （如实, 不制造事件）。
- **L4 分层实证（run4 dual_input 10/10）**: 同帧"程序面
  state=DiscontinuityDeclared + Authority outcome=Preserved v/a=Continuous
  timeline_ok=true"——两 face 语义分层正确, 九项合取零影响; L3 切前 VM=
  legacy 路径不变实证。
- dual_input 全链第二轮 10/10（判据面零扰动两轮实证）。

### 10.3 下一步（执行序不变）

- T5 矩阵续填（DiscontinuityDeclared 新基线格+闩锁解除真机样本待采）→
  验收谓词由验收层定义 → A2-8-04 Gate → A2-8-05; 登记: switch_mock
  行为分歧（mock 行 Declared-forever+VM 硬编码——mock-sync 轮留后续）。
  禁发明阈值; L4 冻结维持; 禁回头重复旧活体验证。

## §11 第五十四轮（R54）: T5 证据矩阵续填+正式化——零代码真机取证轮（2f30d16 基线）

### 11.1 轮次性质与执行（零代码; 复用 R52 OBS gate + R53 四态语义）

- 裁决来源: R53 = Unit B PASS（用户终裁+按仓库数据独立复核 13 项全过）;
  R54 范围 = T5 矩阵续填（§10.3 执行序兑现）。**预期零代码 = 实际零代码**:
  本地 HEAD 2f30d16 与盒 ~/media-agent-build 72/72 源 sha256 全等
  （locale/分隔符归一后 diff 清零）; gates bin 重建（bmd-provider,
  gstreamer-backend）md5=7a0ed95c… 与 R53 逐字节一致 = 零改动的确定性
  复现佐证。
- 真机四跑（2026-09-05 15:15-15:26 CST; 证据盒 ~/a2-8-02i-evidence/
  2026-09-05-r54-a204-t5-matrix/: header 五件套[REV=2f30d16·status 0 行]
  + 72 文件 box.sha256 + gates-bin.md5 + manifest-v5.md5[7521d17e… 同 R53]
  + 四 run 日志各自 md5）:
  - **run1**（N=30, dwell=1000ms, 15:19:46 完, EXIT=0）: 30/30 采集完整;
    全 Preserved; pr_v/pr_a = DiscontinuityDeclared 178 + ValidMonotonic 2
    （首切前 PRE=VM 签名 = R53 基线的 30 切换扩展）; in/br 四路 VM=180;
    NonMonotonic=0; adv=Some(false)=0。
  - **run2**（burst N=30, dwell=0, 15:24:22 完, EXIT=0）: 同签名全净
    （DD 178+2 / NM=0 / adv=0）。
  - **run3**（dual_input 首跑, EXIT=2）: **L1c FAIL——B 类瞬态前置条件**
    （6ede00d0/ball 源 signal=Some(false) → H1 fail-stop, 3/4 verdicts,
    L2-L5 未执行）; 首跑 FAIL 留证禁改判据, 日志原样保留; 时序佐证=run2
    结束前数秒该源仍在供帧（in_a VM=180）→ 判瞬态信号非代码回归。
  - **run4**（dual_input 重试, 15:26:21 完, EXIT=0）: **ALL PASS 10/10
    ——判据面零扰动第三轮全绿**; L4 两 face 分层签名与 R53 同构（程序面
    state=DiscontinuityDeclared + Authority outcome=Preserved·v/a=
    Continuous·epoch=ProgramEpoch(0)·映射逐 ns 闭合 offset=32868814）;
    L5.1/5.2/5.3+故障域归因完整+Teardown 全 PASS。
- 累计切换样本: 本轮 +60 全 Preserved; **R53 语义后累计 80 切换
  NonMonotonic=0——闩锁事件未复现**（R53 前历史频率 30 切换 1 次）;
  闩锁解除真机样本仍未获得（不制造事件; rt_05 单测确定性锁定维持）。
- **D2 新测量事实: av_delta 振荡包络随会话推进增长**——run1 PRE
  1.2-117.9ms（SPAN 9.6-126.2, 均值 52.1）/ run2 PRE 1.8-126.8ms（SPAN
  均值 58.6）; 两跑同形态: ~2-7ms 起步 → #30 时 ~101-127ms, 局部回落但
  包络单调上升（run2 PRE 全序列 6.6→…→126.8→110.1ms 间隔递增可见
  形态）; R52 20 切换短窗（1.7-40.1ms·"无单调漂移"结论）未暴露该包络
  ——时长/切换数两变量在现有数据不可区分, 如实登记。候选=源内在 skew
  随运行时间漂移（电视 1080i25 vs ball 1080p25）; 登记非裁决, 阈值仍禁
  （T2 冻结: ns 可比性≠阈值授权）。
- 工件（零新增, 与 R53 原始日志逐一实测对照）: OBS 跑 pad_unlink ×4/
  跑·PortId ×2/跑·interlace 3-4/跑·MainContext ×1/跑（R54 实测复核
  R53 OBS 三跑同为 MainContext ×1/跑——§68.4 "MainContext WARN 0/4"
  表述与原始日志不符, 就地校准: 0/4 应为 OBS 域 1/跑·dual_input 域
  2/跑）; dual_input 跑 maincontext ×2·pad_unlink ×4·interlace ×6 与
  R53 run4 逐项相等。全为既有隔离债零新增。

### 11.2 T5 证据矩阵（正式化; OQ-T5 冻结形状: 六路×四模式·每格=证据 E·absence≠false·禁合成大布尔）

样本基数: R51 首采 1 + R52 20 + R53 20 + R54 60 = **101 次切换**
（每切换 PRE/SPAN/POST × {A,B} 六行投影; dual_input 全链三轮 10/10 另计）。

| 路＼模式 | rollback | discontinuity | D1 pad 分离 | D2 PTS 漂移 | starvation |
|---|---|---|---|---|---|
| in_v | E+: VM-only ×101 切换; NM 未观测（absence≠false） | 四态如实读出=VM; ingest 无声明源故无 DD 语义 | av_paired 域=watchdog fold 已检出+已测（mock）+R46 真机活体; 真机分离未观测（absence） | 不适用（D2=program 出口 \|v−a\|, 单路无定义） | E+: advanced=Some(true) 持续（含被切离路 SPAN 推进=跨切换不饿死首证 R51）; adv=Some(false)=0/101 |
| in_a | 同 in_v | 同 in_v | 同 in_v（pad 级双平面） | 不适用 | 同 in_v |
| br_v | E+: VM-only ×101; NM 未观测 | 四态=VM; bridge 无声明源 | 同 av_paired 域 | 不适用 | E+: bridge 帧计数推进+alive_in_window 观察时钟窗; Some(false)=0 |
| br_a | 同 br_v | 同 br_v | 同 br_v | 不适用 | 同 br_v |
| pr_v | E±: R52 首证 16/60 行（**旧闩锁语义·R53 前历史**）; R53 语义后 80 切换 NM=0; 生命周期=rt_05 五断言单测锁定 | **E+: DD 新基线格**（R53 34+2/58+2/N4 + R54 178+2×2: 首切前 PRE=VM, 此后 DD 边界事实保持——干净跑稳定签名）; undeclared_backward_jump=0（Authority 证据链）; PlaneContinuity=Continuous 全程 | 同 av_paired 域（program 双平面 selector） | **E-测量**: R51 7.15→15.48ms 首点; R52 振荡 1.7-40.1ms 短窗; R54 会话包络至 ~127ms（§11.1）——只测量无阈值 | E+: SPAN/POST 推进; adv=Some(false)=0 |
| pr_a | E+: NM 未观测（R52 0/60; R53/R54 =0） | 同 pr_v（DD 基线 V/A 对称成立） | 同 pr_v | 同 pr_v（delta 的另一端） | 同 pr_v |

**特殊格——闩锁解除（rollback×discontinuity 交叉生命周期）**: 真实
NM → 下一干净声明边界 → 解除全链 = **rt_05 单测确定性证明 + 真机样本
待采**（R53 语义后 80 切换 0 复现; 历史 30 切换 1 次为旧语义）。该格
状态 = "单测证明+真机待样本"（如实, 不制造事件, 不阻塞交接）。

**缺口格（如实登记, 均有独立归属）**: ①生产 InputPts.stalled 硬编码
false（R50 校准; R52/R53/R54 三轮登记 deferred）——六路逐平面 stalled
生产证据面=无, starvation 列现以 advanced/帧计数证据为据; ②S5
negotiated caps=None（异构 format 边界正证据缺席, 异构事实仍以二十五轮
canonical closure 为据）; ③switch_mock 行为分歧（mock 行 Declared-forever
+VM 硬编码, mock-sync 专门轮留后续, 不混入 A2-8-04 验收证据）。

### 11.3 矩阵交接判定与下一步（执行序不变）

- **T5 矩阵 = 已填充至可交接验收层状态**: 每格要么正证据（含首证/新
  基线）、要么单测锁定、要么如实 absence/缺口（各有独立归属轮次）;
  增量采集项（闩锁解除真机样本）opportunistic 续采不阻塞。
- 验收谓词由验收层在填充后的矩阵上定义（OQ-T5 冻结; 禁发明 PTS delta
  阈值——R54 D2 会话包络新事实恰好证明: 阈值必须由验收层据分布裁决,
  非实现层预设; 禁合成大布尔——各模式证据类型本不同）→ A2-8-04 Gate
  → A2-8-05。
- 红线全维持: L4 冻结判据零触碰; PtsMonotonicity 四态禁洗; absence≠
  false; sampled_at_ms 禁修 PTS; 首跑 FAIL 留证（run3 已执行）; 禁回头
  重复 R46-R49 旧活体验证。

## §12 第五十五轮（R55）: 验收谓词提案交付——OQ-P1..P7 待终裁（零代码）

- 交付: `docs/superpowers/reports/2026-09-05-a2-8-04-acceptance-predicate-
  proposal.md`（状态 PROPOSAL; 基线 2d66ab3; 严格 Evidence Matrix →
  Predicate → Gate——禁自己解释成 PASS）。
- 形状: 逐格谓词 P1-P9（rollback/discontinuity/D1/D2/starvation/
  闩锁解除特殊格/dual_input 回归/证据完整性/Gap 披露）, verdict 封闭
  词表 {Satisfied/Historical/UnitProven+FieldPending/Absent/Gap};
  组合仅 Gate 层固定合取; 证据粒度锚定 OBS 逐路 tally（non_advancing
  :189-207 四元定位）+ Authority 九项合取 + hw 单测 + av_paired 活体。
- 用户 R54 收口边界全部内嵌: pr_v×NM 双层不混（R52 历史=provenance）;
  闩锁解除=单测证明+真机待样本禁制造; av_delta 无阈值; 六路独立
  starvation（聚合 OR 显式排除）; S5=None/switch_mock/stalled=Gap 披露。
- **待裁: OQ-P1（D2 形状, 默认案 a 测量完备性·阈值外排专项轮）/ OQ-P2
  （FieldPending 阻塞, 默认 non-blocking）/ OQ-P3（证据窗, 默认案 b
  Gate 日新鲜确认集）/ OQ-P4（B 类重试政策）/ OQ-P5（工件漂移）/
  OQ-P6（Gap 阻塞化）/ OQ-P7（组合规则）**。
- 状态: A2-8-04 Gate 仍 OPEN——谓词终裁前不执行; A2-8-05 不提前。

## §13 第五十五轮修订（R55.1）: 终裁纠偏与冻结（2026-09-05, docs-only, 零代码）

- **终裁**: 验收层裁定 R55 = **ACCEPT WITH CORRECTIONS**——方向保留,
  四项语义纠偏后冻结; 不退回重做, R53 代码零改动, 裁决时点不执行
  Final Gate。谓词文档状态 PROPOSAL → **FROZEN (R55.1, 基线 89e2863)**。
- **终裁前只读核验**（裁决引用的代码事实逐条对到行号; 运行时代码自
  d1a4fc6 至 89e2863 零改动, 全部行号对两基线等价）: 全部属实, 且发现
  两处裁决未提及的结构性事实（见下"披露"）。
- **四项必改（已并入冻结稿 §2/§7）**:
  1. P1/P5 加最低观测完整性前置——每路 PRE≥1 ∧ POST≥1 且
     pts_state≠Unknown; 未达 = Unproven 不转 PASS; 观测违例 = Failed
     （下限值 = R55.1 补裁）。
  2. P2 重写为 outcome↔continuity 一致性——Preserved ⇔ 双 Continuous;
     NewEpoch 合法（声明性语义: Unproven 边界吸收/DeclaredDiscontinuity）
     不得伪装 Preserved; Violated/TransitionFailed = Failed。原 P2d
     "V/A 永远 Continuous" 弃用（会把合法 NewEpoch 全部误判失败）。
  3. P2c 弃用 `undeclared_backward_jump==None`——核验证实该字段全仓
     唯一构造点 program_timeline.rs:658 硬编码 None、BackwardJumpFact
     (:171) 从未实例化, 该断言只证 close_transition 没填字段, 不证
     运行时无回退。改三支合取: UndeclaredBackwardJump 事件==0（真
     检出链 on_program_pts :754-763 → fail_closed :776-786）∧ 程序面
     观测 NM==0（交叉引用 P1 不合并）∧ Violated 计数==0。
  4. P6: P6a UnitProven = Satisfied（blocking）; P6b FieldPending =
     Pending（non-blocking）, Gate 报告**永不写 Satisfied**。
- **两处结构性披露（核验新增, 本 change 不修, 归属候选后续 Domain 轮）**:
  ①死字段与恒真合取项——undeclared_backward_jump 恒 None →
  dual_input.rs:789 `is_none()` 合取项结构性恒真不提供信息; ②边界吸收
  语义——on_mapped_buffer（program_timeline.rs:621-624）将首帧 mapped
  边界上的回退吸收为 Unproven→NewEpoch, fail-closed 链不覆盖该相位
  （= P2c 语义边界, 如实登记）。
- **引用订正（语义不变）**: SixPathEvidence/PathEvidence/`advanced`
  派生 = program_execution.rs:204-247（a204_obs.rs:29 复用, 定位器
  non_advancing 仍在 a204_obs.rs:189-207）; av_paired fold =
  watchdog.rs:456; BridgeObservation = media_tap.rs:93-101。
- **OQ 全关**: R55 终裁关 OQ-P1（案 a）/OQ-P2（non-blocking 不伪装）/
  OQ-P5（零新增 blocking·漂移停查）/OQ-P6（披露 non-blocking）/OQ-P7
  （四层固定合取: Evidence Integrity(P8)/Semantic Correctness(P1,P2,
  P3,P5,P6a)/Regression Safety(P7 不替代 Timeline Gate））; R55.1 补裁
  关 OQ-P3（**案 b**: Gate 日新鲜确认集+累计背景）/OQ-P4（仅 signal 类
  可重试·全归档）/完整性下限（每路 PRE≥1∧POST≥1 非 Unknown）。
- **verdict 词表 v2**: {Satisfied, Failed, Unproven, Historical,
  UnitProven, FieldPending, Gap}; blocking 格 **Failed 或 Unproven 均阻断
  Gate PASS**（absence≠false: 证据不充分同样不放行）。
- 状态: 谓词已冻结——A2-8-04 Final Gate 待按案 b 证据窗执行（另行
  轮次）; A2-8-05 仍不提前; 本轮零代码零硬件, 账本纯追加。

## §14 第五十五轮修订二（R55.2）: 终裁补正与最终冻结（2026-09-05, docs-only）

- **终裁基线声明（诚实区分）**: 验收层按 GitHub 远端真实状态复裁
  ——远端分支 HEAD = **2f30d16**（父 d1a4fc6）; 本地链 2d66ab3/
  89e2863/5235df3/R55.2 未推送。代码裁决以远端为准; R55.x 文档按
  本地内容审查, 不称"远端已存在"; 推送延后至 Final Gate 完成后。
- **裁决 = ACCEPT WITH ONE REQUIRED CORRECTION**: R55.1 四项必改
  全部确认; 唯一必改 = **P2c 的"UndeclaredBackwardJump 事件计数
  ==0" 同属 vacuous**——该失败路径真实存在（on_program_pts
  program_timeline.rs:754-763 → fail_closed :776-786）但无独立生产
  证据出口（fail_closed 不填充证据载荷, 失败经 timeline_fail_
  closed→SwitchError program_execution.rs:751 走运行时错误面, 无
  事件计数/日志 sink）——"无事件生产 ≠ 没有发生事件"。已拆双通道:
  **P2c-1**（可观测通道, blocking）= 程序面 NM==0（交叉引用 P1）∧
  Violated==0（读出面 = OBS 逐切换 outcome 全 Preserved
  a204_obs.rs:537-545 + dual_input L4）; **P2c-2**（Authority 检出
  通道）= **Unproven/Structural Gap 披露不阻塞**, 禁"0 事件"包装成
  Satisfied, owner=后续 Domain/Observation change。
- **两项澄清入稿**: ①PRE≥1∧POST≥1 = evidence sufficiency minimum
  ≠ continuity completeness proof（采样证据非区间证明）; ②DD =
  declaration-bearing observation state 非异常, Gate 报告禁
  `DD>0→FAIL` 判法（否则把 R53 修复重新判成 failure）。P2 Failed
  子句收紧: Violated ∨ TransitionFailed ∨ 任何明确 TransitionFailure
  终态。首帧边界不连续与稳定态未声明回退=两个故障窗口禁混判
  （on_mapped_buffer :621-624 吸收语义维持登记）。
- 谓词文档状态 → **FROZEN-FINAL (R55.2)**（§8 = 本轮终裁登记）。
- 执行序: 本轮文字纠偏 → 最终冻结 → **直接进入 A2-8-04 Final Gate
  新鲜证据执行**（OBS N≥10 + dual_input 10/10 + hw 259 → 逐格
  verdict → P8 → Gate 层 AND）。运行时零改动; D2 无阈值; A2-8-05
  不提前。

## §15 第五十六轮（R56）: A2-8-04 Final Gate 执行——逐格裁决 = **FAIL**（新真机事实: pr_v 边界违例 NM 首现, R53 状态机如实显形）

- **执行身份链（P8）**: 冻结 bin target/debug/media-agent-gates md5
  `7a0ed95c…` == R53/R54 登记 ✓; 72/72 源 sha 盒==本地 HEAD（6ffff44,
  运行时自 d1a4fc6 零改动）✓; manifest v5 md5 `7521d17e…` ✓; 工件
  零新增（同模式实测: OBS 4/2/3/0==R54 run1·dual_input 4/2/6/0==R54
  run4——pad_unlink/collision/interlace/MainContext-acquired）✓。
  证据盒 `~/a2-8-02i-evidence/2026-09-05-r56-a204-final-gate/`（五件套
  +三日志+av_delta 全序列+NM 事件提取+md5s; 2026-09-05 17:06-17:17 CST）。
- **案 b 新鲜确认集三件**: run1 OBS N=30 dwell=1000ms **EXIT=0 30/30**
  全 Preserved·六路 adv=Some(false)=0; run2 dual_input **首跑 ALL PASS
  10/10 EXIT=0**（无需重试; L1c 双源 signal=true; L4 outcome=Preserved
  epoch=ProgramEpoch(0) V/A=Continuous）; run3 hw 矩阵 bmd,gstreamer
  **259/259 EXIT=0**。
- **核心事件——switch #8 B→A pr_v NonMonotonic=6 行（R53 语义后首次;
  此前 80 切换为 0; 累计 111 切换）**: 逐行还原（box nm-event 提取件）:
  #8 边界 mapped==program_start 锚 74137405051（offset=100000002）;
  NM 闩锁贯穿段 8 早窗（SPAN×2/POST×2 归 #8 + PRE×2 归 #9 共六行;
  期间 pr_v 帧计数 2299→2509 全程推进 adv=Some(true)·PTS 跨相位
  +3s/+4s 正常步进——**非停流/非帧损, 是 mapped 顺序事实**）;
  **#9 A→B 干净边界解除回 DD**（段 9+ 全 DD）——rt_05 五断言锁定的
  生命周期（违例边界传播 + 干净边界解除）**真机完整首证 = P6b
  FieldPending 样本落地**。候选成因=跨源 skew（PRE #8 in_v A 落后
  B 25.8ms·会话累积）与边界锚点交互; av_delta 本跑 SPAN 包络
  1.52→101.52ms·mean 31.72ms 振荡持续（R54 包络增长现象同形态延续）。
- **逐格 verdict（词表 v2; P1/P5 充分性前置六路全过——每路
  PRE/SPAN/POST 每 switch 均 pts_state≠Unknown）**:

  | 格 | verdict |
  |---|---|
  | P1 in_v/in_a/br_v/br_a | **Satisfied**（各 180 行全 VM·NM=0） |
  | P1 pr_a | **Satisfied**（178 DD+2 VM·NM=0） |
  | **P1 pr_v** | **Failed**（**NM=6**·行级定位 epoch=8 六行） |
  | P2 核心 outcome↔continuity | **Satisfied**（30/30 Preserved·Authority V/A=Continuous 逐切换·无 NewEpoch/FailClosed/TransitionFailed） |
  | P2a | Satisfied 记录性（in/br 无 DD 制造） |
  | **P2b pr_v face** | **Failed**（6 NM 行破 {PRE VM→此后 DD} 签名; pr_a face 保持） |
  | **P2c-1 可观测通道** | **Failed**（程序面 NM=6>0·plane Violated 伴随>0; Authority face 全 Preserved——两 face 分层如实呈现） |
  | P2c-2 | Gap 披露（Authority sink 缺失·Unproven·不阻塞） |
  | P3 D1 | Satisfied（窗内 av_paired 分离观测=0·absence 如实; 能力在证=rt_01∈259+R46 活体背景） |
  | P4 D2 | Satisfied 登记性（min/max/mean=1.52/101.52/31.72ms·全序列入盒·无阈值） |
  | P5 starvation 六路 | **Satisfied**（Some(false)=0·SPAN 含被切离路推进） |
  | P6a | Satisfied（rt_05∈259 绿） |
  | P6b | **field 首证落地**（NM→干净边界→reset 全链·非阻塞登记·待验收层确认升级） |
  | P7 | Satisfied（dual_input 10/10 首跑·EXIT=0） |
  | P8 | Satisfied（身份链全过·工件零新增） |
  | P9 | Gap 披露（stalled 硬编码/S5 caps=None/switch_mock 分歧/P2c-2） |

- **Gate 层固定合取: A2-8-04 Final Gate = FAIL**——三 blocking 格
  Failed（P1-pr_v/P2b-pr_v/P2c-1），其余 blocking 格全 Satisfied，
  Gap/增量格披露登记齐。**首败留证·判据零改动·禁为跑绿回改。**
- **交接验收层（实现层不自行解释）**: FAIL 驱动因子=单一新事实
  （跨源 skew 下切换边界 mapped 落于前段 last 之下 → R53 状态机如实
  标 NM 并于下一干净边界解除; 帧流全程健康）。两种待裁读法:
  (a) 维持现冻结谓词=rollback 违例（Gate 维持 FAIL, 修复/缓解后
  复跑）; (b) 参照 R52→R53 先例开边界 rebase 语义裁决轮（判定该类
  "边界 mapped 回退"属 declared-discontinuity 家族的测量事实还是
  correctness 缺口——P6b 真机首证同时是该项裁决的输入）。累计口径:
  R53 语义后 111 切换 pr_v NM 共 6 行（全部在本窗 #8 单事件）。
- 状态: A2-8-04 = **FAIL（待验收层裁决后续路径）**; A2-8-05 不进入;
  本轮零代码（运行时零改动）; 远端基线推送按终裁于 Gate 后执行。

## §16 第十六轮（R57）: 边界回退语义裁决支持——只读重建（零代码）

- 裁决授权: 用户 R56 终裁选 **(b) 严格版**——A2-8-04 = **FAIL / HOLD
  FOR BOUNDARY-REBASE SEMANTICS ADJUDICATION**; 三 blocking 不撤销
  不豁免不修改; FAIL 定性 = "Semantic adjudication required"（非
  "R53 已证伪"）; **P6a→Satisfied / P6b→FieldProven（v2.1 词汇增补,
  生命周期证据域, 不改 blocking）**; 本轮零代码零判据零 Gate。
  全文 = `2026-09-05-a2-8-04-r57-boundary-rebase-semantics.md`
  （+谓词文档 §10 登记）。
- **R57-A（六行 NM 产生链）**: 行 st= 唯一来源 = **PipelineHealth
  程序面健康弧**（selector src-pad 探针改写 mapped → appsink 逐缓冲
  plain :439 / 段首帧 declared note_declared_boundary :199-201→:259-282
  → pipeline.rs:323-333/:354-365 → observe() :1058-1067 → snapshot_row
  克隆共享 ProgramObservation a204_obs.rs:436-445 → pr 行 :339-340）。
  **TimelineAuthority 从未收到违例值**——喂入粒度 = 切换前 ①a 一次 +
  50ms 轮询（program_execution.rs:596-608/:641-668/:669-705）, 单缓冲
  瞬态在采样下不可见; 反证: 若收到则 fail_closed 终态, #9~#30 无法
  正常执行。**观察面逐缓冲 vs 控制面 50ms 采样 = "同 PTS 数学关系两
  个裁定"的接缝实锚（IMP-4 设计结果, 非 bug）。**
- **R57-B（事件重建）**: #8 锚 = 74137405051（=PRE8 末样本精确）×
  source_anchor 74037405049, offset=100000002=差值精确; 边界帧 mapped
  精确落旧段末值（零间隙声明）; **切换行 mapped=74304071718 不是锚**
  ——是 settle 期 50ms×3 轮推进后的 last_program_pts（R56 "锚点前跳
  +26.67ms" 读法就地修正）; av_epoch(7/8/9 执行计数) ≠ tl_epoch
  ProgramEpoch(0)（Preserve 同世代）; 段 8 行 pts 单调 +3s/+4s、帧计数
  推进、av_delta 随活跃源 15150956→1515711→6817622、输入面 skew
  25.8ms→反向 14.1ms、源 offset 差 131605.5ns 逐源稳定。
- **R57-C（六行=投影）**: 3 窗 × 双设备克隆同一 ProgramObservation
  （双行 pr 字段逐 ns 恒同即证）→ 6 次闩锁读数; 底层回退事件 ≥1、
  最简恰 1; **行数≠事件数**——P1 "NM 行==0" 度量闩锁投影数; 事件计数
  判据需证据面扩展（健康弧无计数字段）, 属 Domain/Observation change,
  登记不自行改。
- **机制排查（M1 唯一存活）**: **M2**（install→⑤ 未映射 raw 穿越 ≈
  基线−100ms）会被随后干净边界 DD 释放**洗除**（边界≈锚 > raw）→ 不
  可能产生行 NM, 排除; **M1** = [锚采样→install] 微秒窗内一枚出发段
  映射帧穿越（旧段 offset 仍生效）→ 基线推高 1 帧（40ms@25fps）→ A
  边界帧（=锚, 零间隙）成违例边界 → observe_video_pts_declared 置 NM
  （"声明不豁免回退" :353-356）→ 段作用域 sticky → #9 干净边界解除。
  概率 ≈ 窗(数十~数百µs)/40ms ≈ 0.1-0.6%/切换, 与 R53 后 ~170 切换
  1 例 + R52 run2（pre-R53 无解除→16/60 行到运行尾）相容; **#8 同索引
  复现**（两跑同节拍同相位）候选=周期瞬态拉长竞态窗, n=2 未证。
- **R57-D**: SixPathEvidence=六路行+program_av_delta_ns 仅此; 跨源
  表达=逐源 offset/AnchorPair（**skew 相消构造**: 两锚同瞬采样、offset
  =差、s(T) 相消——日志锚差精确==offset 到 ns 双证）; 无 source_skew/
  boundary_rebase/declared_rebase/drain 概念、无事件计数面 → **D2 不能
  裁决边界合法性确认; "跨源 skew 边界 rebase" 框架在证据下消解**。
- **锐化裁决问题（交验收层）**: 出发段在 [锚采样→新段映射生效] 微秒
  窗内继续推送的映射帧 = **(i) 连续性违例**（现行代码立场, FAIL=correctness
  gap → adapter/runtime 边界原子性修复→回归→Gate 复跑, 谓词不动）vs
  **(ii) 合法排水语义**（Domain contract 变更+事件计数证据面, 与
  "禁以声明洗回退" 冻结纪律正面冲突）。**执行层分析（非裁决）: 证据
  支持 (i)**——有界（≤1 帧）真实输出面回跳、健康弧正确检出、R53 按
  设计自愈、Authority 采样粒度设计性不可见; 缺口在切换协议原子性
  （锚快照与映射安装对旧段在途流不原子）, 不在任一状态机语义本身。
- 诚实边界: 违例帧未被直接观测（无逐缓冲轨迹; M1=排除法+算术自洽的
  推理结论, 幅度 ≈40ms 为机制推断值）; A 未推进(k=0)为最简推断;
  红线全维持（零代码零判据、首败留证、不为跑绿调整）。

### §16.1 R57 终裁复核（验收层 M1′ 修正裁决 × 源码逐条复核, 2026-09-05）

- **实体裁决全成立**: FAIL/HOLD 维持·三 blocking 维持·调用链与
  "③ pre-flip install" 注释（program_execution.rs:596-716/:624）·
  `!t.executed` 门控（switch_graph.rs:221-223）·executed=true 仅
  switch 后置位（:843-845）·锚读取面=健康弧末值+分支末值且
  +last_delta 外推已删（:946-962/:126-129）·clean 相对序判定
  （:265-268）·Authority 轮询两时间尺度（⑤-⑧ 只喂 facts :645-647）·
  R53 生命周期·offset 唯一生产·R57-A/B/C/D 分级·修复否决三项·
  边界原子性目标·Mutex 不足（状态已 Arc<Mutex>——缺口是协议定序）。
- **M1′ 替换案被反驳, M1 维持**: [①c→③] 窗内槽中为 #7 状态
  （executed=true; install :891 替换式写入, 运行期无清槽点）→出发源
  帧仍被旧段映射; raw 无映射窗实为 [③→④], 且 offset#7=+100131607>0
  ⇒ raw 帧落基线−100ms 被 #8 边界 clean→DD 覆写——与"跨 #9 才解除"
  签名矛盾。裁决自身算术（P→P+40ms→A=P）即映射帧签名（raw 位移=
  −offset7）。M1 = 唯一与源码结构和 #8 现场双相容机制。
- **接受的锐化**: §六 相对序（R57 §5.2 精确化为条件排除
  offset_prev>0, #8 成立）; §十七 mock 缺口加强（switch_mock.rs:396
  observe→tick_once; :406-415 program pts_state 硬编码
  ValidMonotonic——结构上不可复现竞态）。
- **R58 输入修正**: 修复不变量按"健康弧观测序"表述（appsink
  transit 晚于探针改写）; 确定性测试主缝=[①c→③] 映射帧注入, 次缝=
  [③→④] raw（符号条件一般化面）; mock 需增强为可注入交错模型。
- 红线全维持; 违例帧未直测的诚实边界维持; 全文 = R57 文档 §10。

### §16.2 R58-Design 终裁复核 + 步骤 4 执行（R58 unit 1, 测试先行零生产代码）

- **逐条复核 12 条全相容**: §1 槽生命周期（:891 唯一写点·运行期无
  清槽·executed=:844·锚=:946-949）→ M1 保留; §2 现有锁仅护槽
  （Arc<Mutex<Option<TES>>>）·弧=观测事实源（冻结=污染）·重采样=
  第二层; §3 fence 三实现条件——queue 在途须入 confirmed 语义（拓扑
  selector→queue→appsink: program-video-queue :508 / program-audio-
  queue :552 / 链接 :630-634）、pre-flip 拦截缓冲 DROP（不得 flush）、
  同 pad 探针添加序 mapping 先于 fence（弧写点在 appsink 故不变量不
  受影响）; §4 appsink 链 :422-446（:439 plain 写点）+探针挂 selector
  src :165-190; §5 双面 selector :1332/:2432+成对校验 :879-888; §6
  EVENT 门 :170-185+映射门 :221-227——fence 只拦 BUFFER 一致; §7
  草图顺序校正（锚必须先于窜帧——窜帧先行锚=P+40ms 不可复现）; §9
  mock 缺口确认（:396/:406-415）; §12 远端 2972fb8 `git ls-remote`
  实测。
- **步骤 4 执行（生产代码零改动, 仅测试模块追加）**: M1 确定性复现
  红测（S7→P→锚 P→窜帧 P+40ms Continuing→install #8→首枚映射 P→
  note_declared_boundary 违例边界→NM 断言→#9 干净边界→DD 断言;
  数值=R56 #8 实测锚 74137405051×74037405049·offset#7=+100131607）
  + 无窜帧差分对照（干净 DD）——`switch_graph_m1_*` 两测盒上 2/2 绿。
- **盒矩阵**: default 227/sim 227/gst 261=259+2（run-A 瞬态 1 失败
  未捕获名·零改动重跑 261/261 全绿——R51 flaky 同型）·clippy×2
  -D warnings 绿。红线全维持; 步骤 5-11 待执行; A2-8-05 不进入。

### §16.3 R58 独立远端裁决复核（验收层×GitHub 真实状态, docs-only 零代码）

- 远端事实独立实测与裁决一致: `git ls-remote`=2972fb80…（R56 FAIL=
  远端最新·证据完整）; `git branch -r --contains d1f1e58`=空（R58
  测试不在任何远端分支; tip=祖先⇒不可能含）; **口径纪律冻结**: 本地/
  远端分述·本地提交永不称远端落地·推送仅按明确指示。
- M1=源码闭环证实非推测（§16.2 锚点全数复认+TIMELINE_POLL_INTERVAL
  50ms :739——两时间尺度 ⇒ NM×Preserved 并存=观察粒度差非模块错误;
  fail-closed 链 program_timeline.rs:746/:755-763/:776 无声明豁免）;
  HOLD 维持定性=设计方向无错·生产修复未进远端·完成至「根因确定+
  测试先行」层级。
- **fence 不变量升格冻结**: INV-F1 queue-in-flight 入 confirmed
  （selector src fence≠output 静止——原条件 a 升格）·INV-F2 旧世代
  拦截帧不可复放/三段模型「排空→barrier→重开」（原条件 b 升格）·
  INV-F3 V+A Both-Confirmed 握手（新; switch() :822 video 先/:824
  audio 后/:828 回滚/:830-837 degraded）; BUFFER-only 与 Mock 交错
  增强维持。
- 步骤 5 边界树登记（V/A fence→confirmed→anchor→declare→install→
  switch→不可复放→executed→release; 二线 stale-baseline→fail-closed
  resample 非主机制）; A2-8-04 不得宣布恢复 PASS·A2-8-05 不进入。

### §16.4 R58 步骤 5 执行: Cutover Fence 生产实现（代码轮, 本地未推送）

- **INV-F1/F2/F3 编码进执行契约**: trait 增 arm/release 两法（无默认
  实现——4 个实现方编译期表态）; switch graph 层 FenceState/
  PlaneFence/FencePair（V+A 单字段成对承载）+ 两层门（selector src
  BUFFER 探针门 drop-first 先于映射 + appsink 消费门 V/A 对称不写
  弧——INV-F1 构造性覆盖在途帧, 无时间等待; INV-F2 Drop 无复放,
  计数交回）; Open=legacy 逐字节。
- **编排序**: ⓪ arm（①a 前）→①c 锚→②③④→switch executed 落点→
  Release（INV-F3）; `CutoverFenceGuard` Drop 兜底 ①-④ 任意错误路径
  必解除 barrier（不吞错误）。Mock=staging（真实交错模型=步骤 6）。
- **测试**: 契约测（双面同装同释/未知 graph fail-closed）+
  fence 闭合 M1 测（窜帧被处置→基线不推进→executed 落点 Release→
  新世代首帧干净 DD——对照无 fence 红测 NonMonotonic）。盒复跑全绿:
  default 227/sim 227/gst 263=259+2+2/clippy×2 -D warnings（首跑
  2 编译错: 闭包借期 E0599+参数 8/7→FencePair 重构修复, 如实登记）。
- Domain/谓词/Gate/阈值零字节; 步骤 6（Mock 交错）/7（真机 #8 复现
  NM 消失+生命周期仍立）/10（全回归）/11（新鲜 Gate）待执行;
  A2-8-04 仍 FAIL/HOLD·A2-8-05 不进入; 未推送（远端=dd263be）。
