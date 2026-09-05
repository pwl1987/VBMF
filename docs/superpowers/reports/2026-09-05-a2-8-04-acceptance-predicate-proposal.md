# A2-8-04 验收谓词（OQ-T5 兑现——Evidence Matrix → Predicate → Gate）

状态: **FROZEN-FINAL (R55.2, 2026-09-05)**——R55.1 终裁 ACCEPT WITH
CORRECTIONS 四项必改 + R55.2 终裁补正（唯一必改: P2c 拆双通道, §8）
均已并入: §2 内标 〔R55.1〕/〔R55.2〕 的条款为纠偏后冻结文本, §7/§8
为终裁登记（旧→新对照）; 未标记条款按提案原文冻结。冻结后 A2-8-04
Gate 按 §4 证据窗执行。

授权来源: R54 交接（04-探针 §11.2 矩阵已填充至可交接状态）+ 用户 R54
收口指令（"逐格定义 OQ-T5 acceptance predicates·严格 Evidence Matrix →
Predicate → Gate·禁自己解释成 PASS"）+ 用户 R55 终裁（ACCEPT WITH
CORRECTIONS·四项必改·四层 Gate 结构）+ R55.1 三项补裁（OQ-P3 证据窗/
OQ-P4 重试政策/完整性下限）。
基线: 提案 2d66ab3 → 冻结 89e2863（R55 提案后 HEAD）; 证据矩阵 =
04-探针 §11.2（101 切换基数 + dual_input 三轮全绿）。

## §1 设计原则（全部继承既有冻结, 本提案不新增语义）

1. **每格独立谓词, Gate 层组合**——禁格内/格间合成大布尔（OQ-T5 修订后
   冻结原文）。
2. 每格 verdict 取自封闭词表 v2〔R55.1 替换, 定义见 §7〕: `Satisfied` /
   `Failed`(观测违例) / `Unproven`(≠false·证据不充分) / `Historical`
   (旧语义·不计数) / `UnitProven` / `FieldPending` / `Gap`(独立归属·
   披露)——各模式证据类型本不同, 禁压平成 bool。
3. 证据源只认四类: ①OBS 逐路 tally（〔R55.1 引用订正〕结构定义
   SixPathEvidence/PathEvidence/`advanced`=Some(c>p) 双计数在场否则
   None 在 program_execution.rs:204-247, a204_obs.rs:29 复用不另造;
   `non_advancing` 逐路定位器 a204_obs.rs:189-207[#idx/phase/dev/path
   四元粒度·None 行不计=absence≠false 的结构保证]·av_delta 三相位
   序列）②
   dual_input L4 九项合取（Authority TimelineTransitionEvidence——
   Domain SoT）③hw 矩阵单测（rt_04×4/rt_05 五断言）④watchdog
   `av_paired` fold（mock 已测）+R46 生产线程活体。**显式排除**:
   `progress_since` 聚合 OR 语义与生产 `stalled` 硬编码 false 不可作
   六路证据（R50 校准冻结）。
4. 首跑 FAIL 留证; B 类前置重试政策=OQ-P4; 判据禁为跑绿修改。
5. 无魔法数: 除 OQ-P1 裁出的 D2 形状外, 全部谓词为计数==0 / 枚举相等 /
   结构断言。
6. `[pr_v×NonMonotonic]` 双层不可混（用户 R54 边界确认）: R52 首证
   16/60 行=旧闩锁语义历史（provenance 附录）; R53 MappedContinuation
   状态机后窗口=基线计数唯一来源。

## §2 逐格谓词 P[cell]（R55.1 纠偏后冻结; blocking 格 Failed 或
Unproven 均阻断 Gate PASS）

### P1 rollback——六路各一, 逐路独立报数〔R55.1 纠偏〕
- **充分性前置（新增）**: 每路先过最低观测完整性——窗口内该路
  PRE≥1 ∧ POST≥1 且 pts_state≠Unknown（VM/NM/DD 任一）; 未达 → 该路
  **Unproven**（不转 PASS、不计 Failed、不进入计数裁决）。下限是观测
  行数不是 PTS 数值, 不构成阈值发明。〔R55.2 语义澄清〕PRE≥1∧POST≥1
  = **Evidence Sufficiency minimum**, **不是 Continuity completeness
  proof**——SixPathEvidence 是采样证据, 两个观察点有证据不证明整窗
  全程无短暂 rollback/starvation; 二者禁混。
- 断言: 通过前置的 path p, 证据窗内 `NonMonotonic` 行计数 == 0, p ∈
  {in_v, in_a, br_v, br_a, pr_v, pr_a}; Gate 报告六个独立数字,
  **禁求和成单值**。观测到任何 NM 行 = **Failed**。
- pr_v 特别条款: 仅对 R53 语义后窗口断言（§1.6）; R52 历史行永不计入。
- 违例呈现: 行级定位（与 adv 定位器同粒度: switch/phase/dev/path）。
- 冻结: **blocking**（该格 Failed 或 Unproven 均使 Gate 不能 PASS）。
- 现状证据: R53 后 80 切换六路 NM=0（R53 20 + R54 60）。

### P2 discontinuity——outcome↔continuity 一致性〔R55.1 重写〕
- **核心断言（原 P2d 并入）**: 证据窗内每次切换, Authority 报告的
  outcome 与双平面 continuity 状态必须**一致**（close_transition 推导
  的镜像校验: program_timeline.rs:660-662/675/710）:
  - `Preserved` ⇔ video==Continuous ∧ audio==Continuous;
  - `NewEpoch` 为**合法**结局 ⇔ 至少一平面 ∉{Continuous} 且属声明性
    语义（Unproven 边界吸收 [on_mapped_buffer, program_timeline.rs:
    621-624] 或 DeclaredDiscontinuity）——NewEpoch 不得伪装成
    Preserved, 平面不 Continuous 时不得报 Preserved;
  - 平面 `Violated` ∨ `TimelinePhase::TransitionFailed` ∨ 任何明确
    `TransitionFailure` 终态 = **Failed**〔R55.2 收紧〕。
- P2a（input/bridge 四路）: **仅对有 PTS 证据的行**断言四态如实 ==
  `ValidMonotonic`; 普通观测（pipeline.rs:323-333）无声明源, DD 不可
  产生——若出现即为异常（记录性定位）。无证据行 = Unknown →
  Unproven, 不判 PASS 也不判 failure。
- P2b（pr_v/pr_a 基线签名）: {首切前 PRE 行 = ValidMonotonic} ∧
  {其后行 = DiscontinuityDeclared}——R53/R54 四跑稳定签名（34+2/
  58+2/N4/178+2×2）。〔R55.2〕**DD 是 declaration-bearing
  observation state, 不是异常**——Gate 报告禁出现 `DD>0→FAIL` 类
  判法（否则把 R53 修复重新判成 failure）; P2b 的 DD = 期望基线签名
  （declaration 面）; 与稳定态 NM（failure 面）分属两个观测通道。
- P2c（未声明回退）〔R55.1 弃用字段值; R55.2 拆双通道〕: **不再以**
  `TimelineTransitionEvidence.undeclared_backward_jump == None` 作
  证据——该字段全仓唯一构造点 program_timeline.rs:658 硬编码 None,
  `BackwardJumpFact`(:171) 从未实例化, 故 dual_input.rs:789 的
  `is_none()` 合取项**结构性恒真、不提供信息**（披露事实, 本 change
  不修）。〔R55.2〕R55.1 三支合取中的"UndeclaredBackwardJump 事件
  计数==0"**同属 vacuous**——该失败路径真实存在（on_program_pts,
  program_timeline.rs:754-763 → fail_closed :776-786 → Transition
  Outcome::Failed + TimelinePhase::TransitionFailed）, 但**无独立生产
  证据出口**: fail_closed 不填充证据载荷, 失败经 `timeline_fail_
  closed → SwitchError`（program_execution.rs:751）走运行时错误面,
  无事件计数/日志 sink——"0 事件"不可观测即不可作证据（无事件生产
  ≠没有发生事件, absence≠evidence 同构）。拆为双通道:
  - **P2c-1（可观测通道, blocking）**: 窗口内 ①程序面观测 NM 行==0
    （与 P1 pr 路**交叉引用, 不合并计数**）②PlaneContinuity Violated
    观测==0——可观测面 = OBS 逐切换 outcome 全部 Preserved
    （a204_obs.rs:537-545 逐行打印, 任何 FailClosed/带 Violated 的
    NewEpoch 即 Failed）+ dual_input L4 Authority outcome 同检。
  - **P2c-2（Authority 检出通道 = Unproven/Structural Gap, 披露不
    阻塞）**: UndeclaredBackwardJump 检出能力 = 类型+触发路径真实
    存在但无生产证据 sink → **不以"0 事件"包装成 Satisfied**; 登记
    Gap(owner=后续 Domain/Observation change), A2-8-04 不修复, 仅
    真实能力披露。
  - 语义边界如实登记: 落在首帧 mapped 边界的回退被 on_mapped_buffer
    （program_timeline.rs:621-624）吸收为 Unproven→NewEpoch（合法
    声明路径）, fail-closed 检出链不覆盖该相位——首帧边界不连续与
    稳定态未声明回退是**两个不同故障窗口**, 禁混判。
- 冻结: 核心/P2b/P2c-1 **blocking**; P2a 记录性; P2c-2 Gap 披露。

### P3 D1 pad 分离
- 断言: 证据窗内 `av_paired` 分离真机观测计数 == 0; 检出能力在证 =
  `group_fold_rt_01_av_divergence_detected`（hw 矩阵绿）+ R46 生产线程
  活体。
- 语义: >0 = FAIL; ==0 = 窗口内 Satisfied（absence 如实, 不外推
  "结构上不可能"）。
- 冻结: **blocking**（独立验收层谓词; `program_av_delta_ns`/
  `av_paired` 测量不反哺 Timeline Authority 内部状态〔R55.1 终裁
  确认〕）。

### P4 D2 PTS 漂移——无阈值谓词（形状=OQ-P1 待裁）
- 提案默认（案 a·测量完备性）: 证据窗内 av_delta 三相位分布（min/max/
  mean + 全序列入证据盒, N=窗口切换数）**已登记** = Satisfied
  ("measured+registered"); **阈值显式排除在 04 Gate 语义外**, 定值归
  后续专项分布裁决轮（R54 会话包络增长事实 2-7ms→101-127ms = 该轮
  输入; T2 冻结"ns 可比性≠阈值授权"维持）。
- 备选案 b: 验收层现在基于 R52+R54 分布直接给值/给程序（需验收层
  提供, 本提案不预设）。
- 备选案 c: 程序化阈值（分位数+余量类）——仍属实现层发明, 不推荐。
- 冻结〔OQ-P1 终裁=案 a〕: non-blocking（登记性）。

### P5 starvation——六路各一, 逐路独立〔R55.1 加充分性前置〕
- **充分性前置（新增, 与 P1 同一）**: 该路 PRE≥1 ∧ POST≥1 且
  pts_state≠Unknown; 未达 → Unproven——六路全 None 时
  Some(false)==0 **不得**转 PASS。〔R55.2〕前置 = sufficiency
  minimum ≠ continuity completeness proof（同 P1 澄清）。
- 断言: 通过前置的 path p, 证据窗内 `advanced==Some(false)` 计数 == 0
  （定位器逐路四元粒度）; 观测到 Some(false) = **Failed**;
  `advanced==None` 行 = absence 登记非 false（不计违例, 须报数）。
- 反面结构证据: SPAN 窗含被切离路推进（R51 首证+101 切换保持）= 跨
  切换不饿死。
- 显式排除（重申）: 聚合 OR / 生产 stalled 硬编码不可作证据。
- 冻结: **blocking**（任一路 Failed 或 Unproven 均使 Gate 不能 PASS）。
- 现状证据: 101 切换全六路 Some(false)=0。

### P6 闩锁解除生命周期（特殊格, rollback×discontinuity 交叉）
- P6a 单测层: rt_05 五断言（①段内回退→NM ②普通帧不自动恢复 ③干净
  声明边界解除 ④违例边界 NM 传播 ⑤V/A 独立）hw 矩阵绿 = **Satisfied**
  〔R55.1: UnitProven 即 Satisfied, blocking〕。
- P6b 真机层: NM→下一干净声明边界→reset 全链 = **FieldPending**（禁
  人为制造, 增量采集; 历史 30 切换 1 次, R53 语义后 80 切换 0 次）。
  〔R55.1〕FieldPending = Pending 登记项, non-blocking, **Gate 报告中
  永不写 Satisfied**——与 P6a 分行呈现, 禁合并。
- 冻结: P6a **blocking**; P6b non-blocking 增量（OQ-P2 终裁）。

### P7 判据面零扰动回归（dual_input）
- 断言: 冻结 bin 下 ALL PASS 10/10（L0→L5+Teardown）; 首跑 B 类 FAIL
  留证不被重跑覆盖（R54 run3/run4 先例即本谓词的执行形态）; 重试政策
  =OQ-P4 终裁（仅 signal 类可重试, 全部尝试日志归档）。
- 冻结: **blocking**（回归证据; **不得替代 Timeline Gate 裁决**——
  dual_input 为诊断/验收证据面, 不写 agent_state, gates/dual_input.rs:
  45/:175-177）。
- 现状证据: 三轮全绿（R51 首轮 / R53 run4 / R54 run4）。

### P8 证据完整性（盒纪律）
- 断言: 五件套（date/date -u/timedatectl/REV/git status）+ 源 sha 盒==
  HEAD + bin/manifest md5 + 各 run 日志 md5 归档; 工件计数与上轮登记
  逐项对照（pad_unlink/PortId/interlace/MainContext——零新增; 漂移处理
  =OQ-P5）。
- 冻结: **blocking**〔R55.1 确认并提升为 Gate 基础设施谓词——最终
  PASS 必须可回答"哪个源码版本/哪个 binary/哪个硬件运行所得"; 不能
  回答则技术全绿也不构成 release-grade acceptance evidence〕。

### P9 Gap 格——披露性, 不满足不假装
- `stalled` 生产硬编码 false / S5 negotiated caps=None / switch_mock
  行为分歧: Gate verdict 以 `Gap(owner=独立轮)` 披露——不计入
  Satisfied、不静默略过（阻塞化与否=OQ-P6）。

## §3 Gate 组合规则〔R55.1 四层结构·OQ-P7 终裁冻结〕

**A2-8-04 Gate PASS ⇔ 全部 blocking 格 Satisfied ∧ 全部 Gap 格已披露
（owner/reason/scope 在册）∧ 全部增量格登记（FieldPending 在册且不改写
为 Satisfied）∧ P8 证据完整**。blocking 格出现 **Failed 或 Unproven**
均使 Gate 不能 PASS（absence≠false: 证据不充分同样不放行）。

组合分层（仅 Gate 层固定合取; 各格 verdict 独立保留于 Gate 报告, 任一
违例可定位到 switch/phase/path/行——可审计、可复现）:

| 层 | 成员 |
|---|---|
| Evidence Integrity | P8（源 sha 盒==HEAD/bin/manifest md5/日志 md5/工件对照） |
| Semantic Correctness | P1, P2（核心/P2b/P2c; P2a 记录性）, P3, P5, P6a |
| Regression Safety | P7（dual_input=L4 回归证据, **不得替代 Timeline Gate 裁决**） |

P4（D2 案 a·登记性）、P9（Gap 披露）与 P2c-2（Authority 检出通道
Gap）不进入 blocking 合取集合, 按登记/披露义务呈现。

## §4 证据窗定义〔OQ-P3 终裁=案 b, 冻结〕

- **案 b（冻结）**: Gate 日以冻结 bin 新鲜确认集 = 一次 OBS 场景
  （N≥10）+ dual_input 10/10 + hw 矩阵（259）绿; 累计矩阵为背景。
  理由: 排除"历史绿 + Gate 日环境已变"的审计缺口; 冻结 bin md5 逐字节
  可比（R54 已实证零代码轮 bin 复现性）。
- 案 a（未采纳）: 累计账本证据（R51-R54, 101 切换 + 三轮 dual_input）
  即证据窗——存在环境漂移审计缺口, 弃用。

## §5 OQ-P1..P7 终裁表（R55 终裁 + R55.1 补裁, 全部关闭）

| OQ | 问题 | 终裁 |
|---|---|---|
| OQ-P1 | D2 谓词形状 | 案 a: 测量完备性登记; 阈值显式外排至专项分布裁决轮 |
| OQ-P2 | 闩锁解除 FieldPending 阻塞与否 | non-blocking 增量; UnitProven=blocking; FieldPending 永不写 Satisfied |
| OQ-P3 | 证据窗 | 案 b: Gate 日新鲜确认集 + 累计矩阵背景（R55.1 补裁） |
| OQ-P4 | B 类前置重试政策 | 仅 signal 类允许重试; 全部尝试日志归档; 判据零改动（R55.1 补裁） |
| OQ-P5 | 工件漂移处理 | 零新增=blocking; 漂移=停下调查非自动 FAIL |
| OQ-P6 | Gap 格阻塞化 | 披露性 non-blocking（维持三轮登记口径） |
| OQ-P7 | 组合规则确认 | Gate 层四层固定合取（§3）; 格 verdict 独立保留 |
| — | P1/P5 最低观测完整性下限 | 每路 PRE≥1 ∧ POST≥1 且 pts_state≠Unknown; 未达=Unproven（R55.1 补裁） |
| — | P2c 通道拆分 | P2c-1 可观测通道 blocking（程序面 NM==0 ∧ Violated==0）; P2c-2 Authority sink 缺失 = Unproven/Gap 披露, 禁"0 事件"=Satisfied（R55.2 必改） |

## §6 边界与红线（不变, R55.1 增补）

零代码提案轮; L4 冻结判据零触碰; PtsMonotonicity 四态禁洗; absence≠
false; sampled_at_ms 禁修 PTS; 首跑 FAIL 留证; R52/R53/R54 历史零回改;
A2-8-04 Gate 在谓词终裁前不执行（现已冻结, Gate 按案 b 窗执行）; A2-8-05
不提前; switch_mock/stalled/S5 不借谓词轮顺手修。〔R55.1 增〕验收谓词
不得反向改变 PipelineHealth / SwitchGraph Adapter / TimelineAuthority /
SixPathEvidence 四层既有职责与状态机; 不因 101 切换累计好看提前判
PASS——当前缺的是 Authority/Adapter/Observation/Acceptance 四层证明
关系的钉死, 不是更多切换样本。

## §7 R55.1 终裁登记（ACCEPT WITH CORRECTIONS → FROZEN）

**终裁**: 验收层 2026-09-05 裁定 R55 = ACCEPT WITH CORRECTIONS——方向
保留, 四项语义纠偏后冻结; 不退回重做, 不改 R53 代码, 裁决时点不执行
Final Gate。终裁前另经只读核验把裁决引用的代码事实逐条对到行号
（运行时代码自 d1a4fc6 至 89e2863 零改动, 全部行号对两基线等价）。

### 四项必改对照（旧 → 新）

| # | 旧（提案 v1） | 新（冻结 v2） |
|---|---|---|
| 1 | P1/P5 以计数==0 直接判 | 加最低观测完整性前置（每路 PRE≥1∧POST≥1 非 Unknown）; 未达=Unproven 不转 PASS; 观测违例=Failed |
| 2 | P2d 要求 V/A 永远 Continuous | 改 outcome↔continuity 一致性: Preserved⇔双 Continuous; NewEpoch 合法（声明性语义）不得伪装 Preserved; Violated=Failed |
| 3 | P2c 用 `undeclared_backward_jump==None` | 弃用（字段硬编码 None 无信息）; 改三支合取: UndeclaredBackwardJump 事件==0 ∧ 程序面 NM==0（交叉引用 P1 不合并）∧ Violated==0 |
| 4 | P6 FieldPending 未显式禁止伪装 | P6a UnitProven=Satisfied(blocking); P6b FieldPending=Pending(non-blocking), Gate 报告永不写 Satisfied |

### 两处结构性披露（核验发现, 本 change 不修）

1. **死字段与恒真合取项**: `undeclared_backward_jump` 唯一构造点
   program_timeline.rs:658 硬编码 None, `BackwardJumpFact`(:171) 从未
   实例化 → dual_input.rs:789 `is_none()` 合取项结构性恒真。归属:
   候选后续 Domain 轮（让 fail_closed 填充该字段或移除合取项）, 不进
   A2-8-04。
2. **边界吸收语义**: on_mapped_buffer（program_timeline.rs:621-624）
   将落在首帧 mapped 边界的回退吸收为 Unproven→NewEpoch; fail-closed
   检出链仅存在于 on_program_pts（:754-763）。此为合法声明性路径,
   已登记为 P2c 语义边界。

### verdict 封闭词表 v2（替换 §1.2 旧词表）

`Satisfied`(证据充分∧断言成立) / `Failed`(观测到违例) /
`Unproven`(充分性前置未满足——不转 PASS、不计 Failed) /
`Historical`(旧语义历史, 仅 provenance) / `UnitProven`(单测证明) /
`FieldPending`(真机待样本) / `Gap`(独立归属披露)。blocking 格要求
Satisfied; **Failed 与 Unproven 均阻断 Gate PASS**。

### 引用订正（核验订正, 语义不变）

SixPathEvidence/PathEvidence/`advanced` 派生 = program_execution.rs:
204-247（a204_obs.rs:29 复用）; `av_paired` fold = watchdog.rs:456
（execution_group_observe_fold :398）; BridgeObservation =
contracts/media_tap.rs:93-101（controller.rs:841-849/684-694 独立
喂入）; dual_input 不写 agent_state = gates/dual_input.rs:45/
:133-134/:175-177（`_agent_state` 下划线未用=结构证明）; close_
transition 为模块私有（program_timeline.rs:639）, Failed 从不在其内
派生（:637-638"矛盾已在证据入口 FailClosed"）。

## §8 R55.2 终裁补正（ACCEPT WITH ONE REQUIRED CORRECTION → 最终冻结）

**终裁基线声明**: 验收层按 GitHub 远端真实状态复裁——远端分支
`comet/a2-8-dual-input-switch` HEAD = **2f30d16**（R53 账本单元, 父
d1a4fc6）; 本地链 2d66ab3（R54）/89e2863（R55）/5235df3（R55.1）+
本轮 R55.2 **未推送**。代码裁决以远端 2f30d16→d1a4fc6 为准; 文档
裁决按本地冻结内容审查, **不称"远端已存在"**; 推送按终裁延后至
Final Gate 完成后（否则最终闭环无法在仓库层面形成可追溯基线）。

**裁决**: R55.1 四项必改全部确认（P1/P5 充分性 ✅ / P2 outcome↔
continuity ✅ / P6 FieldPending ✅——含原 P2c 字段弃用方向 ✅）;
**唯一必改 = P2c 的"UndeclaredBackwardJump 事件计数==0"**——与被
弃用的字段断言同属 vacuous: 无生产证据出口的通道, "0 事件"≠"无
事件"。已拆双通道（P2c-1 可观测通道 blocking / P2c-2 Unproven·
Structural Gap 披露, owner=后续 Domain/Observation change）。

**两项澄清一并入稿**: ①PRE≥1∧POST≥1 = evidence sufficiency
minimum ≠ continuity completeness proof（采样证据非区间证明, P1/P5
内注明）; ②DD = declaration-bearing observation state 非异常, Gate
报告禁 `DD>0→FAIL` 判法（P2b 内注明; declaration 面 vs failure 面
两通道分立）。P2 Failed 子句收紧: Violated ∨ TransitionFailed ∨
任何明确 TransitionFailure 终态。

**fail_closed 传播面实锚（本轮核验）**: on_program_pts 失败经
`timeline_fail_closed(e) → SwitchError`（program_execution.rs:751）
走运行时错误面; `TimelinePhase::TransitionFailed{reason}` 处理点
:651; OBS gate 逐切换打印 outcome 短名 Preserved/NewEpoch/FailClosed
（a204_obs.rs:115-116/:537-545/:626-630）= P2c-1 可观测通道的窗口
读出面。

**执行序（终裁确认, 本轮后直接进入）**: R55.2 文字纠偏 → 谓词最终
冻结 → 新鲜证据窗（OBS N≥10 + dual_input 10/10 + hw 矩阵 259）→
逐格 verdict → P8 完整性 → Gate 层 AND → A2-8-04 PASS/FAIL。
运行时零改动清单维持: PipelineHealth/SwitchGraph/TimelineAuthority/
SixPathEvidence/dual_input/Supervisor/L4 全不动; D2 不加阈值; 不因
101 次累计证据漂亮提前 PASS; A2-8-05 不提前。

## §9 Gate 执行结果登记（R56, 2026-09-05）

按本冻结谓词+案 b 窗执行（证据盒 2026-09-05-r56-a204-final-gate;
细节 = 04-探针 §15/主账 §73）: 新鲜三件全 EXIT=0（OBS 30/30 全
Preserved·dual_input 首跑 10/10·hw 259/259）; 逐格: 除 **P1-pr_v /
P2b-pr_v / P2c-1 三 blocking 格 Failed**（switch #8 B→A pr_v NM=6
行——R53 语义后首现, 帧流全程健康, #9 干净边界自解除=P6b 真机
首证）外全 Satisfied/披露齐 → **Gate 层固定合取 = A2-8-04 Final
Gate = FAIL**。首败留证; 判据零改动; 后续路径（维持违例读法 vs
边界 rebase 语义裁决轮）归验收层。
