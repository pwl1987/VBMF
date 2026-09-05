# A2-8-04 验收谓词提案（OQ-T5 兑现——Evidence Matrix → Predicate → Gate）

状态: **提案（PROPOSAL）——待验收层终裁; 终裁前零 Gate 执行、零实现**。

授权来源: R54 交接（04-探针 §11.2 矩阵已填充至可交接状态）+ 用户 R54
收口指令（"逐格定义 OQ-T5 acceptance predicates·严格 Evidence Matrix →
Predicate → Gate·禁自己解释成 PASS"）。
基线: 2d66ab3（R54 账本后 HEAD）; 证据矩阵 = 04-探针 §11.2（101 切换
基数 + dual_input 三轮全绿）。

## §1 设计原则（全部继承既有冻结, 本提案不新增语义）

1. **每格独立谓词, Gate 层组合**——禁格内/格间合成大布尔（OQ-T5 修订后
   冻结原文）。
2. 每格 verdict 取自封闭词表: `Satisfied(E+)` / `Historical(首证·旧
   语义·不计数)` / `UnitProven+FieldPending` / `Absent(≠false·登记)` /
   `Gap(独立归属·披露)`——各模式证据类型本不同, 禁压平成 bool。
3. 证据源只认四类: ①OBS 逐路 tally（a204_obs.rs: pts_state 计数·
   `non_advancing` 逐路定位器 :189-207[#idx/phase/dev/path 四元粒度·
   None 行不计=absence≠false 的结构保证]·av_delta 三相位序列）②
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

## §2 逐格谓词 P[cell]（提案默认; blocking=违例即 Gate FAIL）

### P1 rollback——六路各一, 逐路独立报数
- 断言: 证据窗内 path p 的 `NonMonotonic` 行计数 == 0, p ∈ {in_v, in_a,
  br_v, br_a, pr_v, pr_a}; Gate 报告六个独立数字, **禁求和成单值**。
- pr_v 特别条款: 仅对 R53 语义后窗口断言（§1.6）; R52 历史行永不计入。
- 违例呈现: 行级定位（与 adv 定位器同粒度: switch/phase/dev/path）。
- 提案默认: **blocking**。
- 现状证据: R53 后 80 切换六路 NM=0（R53 20 + R54 60）。

### P2 discontinuity——declared vs unexpected 判别面
- P2a（input/bridge 四路）: 四态如实读出 == `ValidMonotonic`; 结构备注:
  ingest/bridge 无声明源, DD 不可产生——缺席=结构事实（如实记录, 非
  证据性"通过"）。
- P2b（pr_v/pr_a 基线签名）: {首切前 PRE 行 = ValidMonotonic} ∧ {其后
  行 = DiscontinuityDeclared}——R53/R54 四跑稳定签名（34+2/58+2/N4/
  178+2×2）。
- P2c（未声明回退）: Authority `TimelineTransitionEvidence.
  undeclared_backward_jump == None`（证据窗全部切换）。
- P2d（双平面连续）: `PlaneContinuity(v)==Continuous ∧ (a)==
  Continuous`（Authority 证据链）。
- 提案默认: P2b/P2c/P2d **blocking**; P2a 记录性。

### P3 D1 pad 分离
- 断言: 证据窗内 `av_paired` 分离真机观测计数 == 0; 检出能力在证 =
  `group_fold_rt_01_av_divergence_detected`（hw 矩阵绿）+ R46 生产线程
  活体。
- 语义: >0 = FAIL; ==0 = 窗口内 Satisfied（absence 如实, 不外推
  "结构上不可能"）。
- 提案默认: **blocking**。

### P4 D2 PTS 漂移——无阈值谓词（形状=OQ-P1 待裁）
- 提案默认（案 a·测量完备性）: 证据窗内 av_delta 三相位分布（min/max/
  mean + 全序列入证据盒, N=窗口切换数）**已登记** = Satisfied
  ("measured+registered"); **阈值显式排除在 04 Gate 语义外**, 定值归
  后续专项分布裁决轮（R54 会话包络增长事实 2-7ms→101-127ms = 该轮
  输入; T2 冻结"ns 可比性≠阈值授权"维持）。
- 备选案 b: 验收层现在基于 R52+R54 分布直接给值/给程序（需验收层
  提供, 本提案不预设）。
- 备选案 c: 程序化阈值（分位数+余量类）——仍属实现层发明, 不推荐。
- 提案默认: **案 a; non-blocking（登记性）**。

### P5 starvation——六路各一, 逐路独立
- 断言: 证据窗内 path p 的 `advanced==Some(false)` 计数 == 0（定位器
  逐路四元粒度）; `advanced==None` 行 = absence 登记非 false（不计
  违例, 须报数）。
- 反面结构证据: SPAN 窗含被切离路推进（R51 首证+101 切换保持）= 跨
  切换不饿死。
- 显式排除（重申）: 聚合 OR / 生产 stalled 硬编码不可作证据。
- 提案默认: **blocking**（任一路 Some(false)>0 即 FAIL）。
- 现状证据: 101 切换全六路 Some(false)=0。

### P6 闩锁解除生命周期（特殊格, rollback×discontinuity 交叉）
- 断言: rt_05 五断言（①段内回退→NM ②普通帧不自动恢复 ③干净声明边界
  解除 ④违例边界 NM 传播 ⑤V/A 独立）hw 矩阵绿 = **UnitProven**;
  真机 NM→下一干净声明边界→reset 全链 = **FieldPending**（禁人为制造,
  增量采集; 历史 30 切换 1 次, R53 语义后 80 切换 0 次）。
- 提案默认: UnitProven=**blocking**; FieldPending=**non-blocking 增量
  项**（阻塞与否=OQ-P2 待裁——若裁 blocking, 04 Gate 须待真机样本）。

### P7 判据面零扰动回归（dual_input）
- 断言: 冻结 bin 下 ALL PASS 10/10（L0→L5+Teardown）; 首跑 B 类 FAIL
  留证不被重跑覆盖（R54 run3/run4 先例即本谓词的执行形态）; 重试政策
  =OQ-P4。
- 提案默认: **blocking**。
- 现状证据: 三轮全绿（R51 首轮 / R53 run4 / R54 run4）。

### P8 证据完整性（盒纪律）
- 断言: 五件套（date/date -u/timedatectl/REV/git status）+ 源 sha 盒==
  HEAD + bin/manifest md5 + 各 run 日志 md5 归档; 工件计数与上轮登记
  逐项对照（pad_unlink/PortId/interlace/MainContext——零新增; 漂移处理
  =OQ-P5）。
- 提案默认: **blocking**（哈希与五件套部分）。

### P9 Gap 格——披露性, 不满足不假装
- `stalled` 生产硬编码 false / S5 negotiated caps=None / switch_mock
  行为分歧: Gate verdict 以 `Gap(owner=独立轮)` 披露——不计入
  Satisfied、不静默略过（阻塞化与否=OQ-P6）。

## §3 Gate 组合规则（OQ-P7 待裁确认）

提案默认: **A2-8-04 Gate PASS ⇔ 全部 blocking 格 Satisfied ∧ 全部 Gap
格已披露（owner 在册）∧ 全部增量格登记（FieldPending 在册）∧ P8 证据
完整**。组合仅发生在 Gate 层的固定合取; 各格 verdict 独立保留于 Gate
报告（任一违例可定位到 switch/phase/path/行——可审计、可复现）。

## §4 证据窗定义（OQ-P3 待裁）

- 案 a: 累计账本证据（R51-R54, 101 切换 + 三轮 dual_input）即证据窗。
- 案 b（提案默认）: Gate 日以冻结 bin 新鲜确认集 = 一次 OBS 场景
  （N≥10）+ dual_input 10/10 + hw 矩阵（259）绿; 累计矩阵为背景。
  理由: 排除"历史绿 + Gate 日环境已变"的审计缺口; 冻结 bin md5 逐字节
  可比（R54 已实证零代码轮 bin 复现性）。

## §5 OQ-P1..P7 终裁清单（提案默认汇总）

| OQ | 问题 | 提案默认 |
|---|---|---|
| OQ-P1 | D2 谓词形状 | 案 a: 测量完备性登记; 阈值显式外排至专项分布裁决轮 |
| OQ-P2 | 闩锁解除 FieldPending 阻塞与否 | non-blocking 增量项（UnitProven 仍 blocking） |
| OQ-P3 | 证据窗 | 案 b: Gate 日新鲜确认集 + 累计矩阵背景 |
| OQ-P4 | B 类前置重试政策 | 仅 signal 类允许重试; 全部尝试日志归档; 判据零改动 |
| OQ-P5 | 工件漂移处理 | 零新增=blocking; 漂移=停下调查非自动 FAIL |
| OQ-P6 | Gap 格阻塞化 | 披露性 non-blocking（维持三轮登记口径） |
| OQ-P7 | 组合规则确认 | Gate 层固定合取; 格 verdict 独立保留 |

## §6 边界与红线（不变）

零代码提案轮; L4 冻结判据零触碰; PtsMonotonicity 四态禁洗; absence≠
false; sampled_at_ms 禁修 PTS; 首跑 FAIL 留证; R52/R53/R54 历史零回改;
A2-8-04 Gate 在谓词终裁前不执行; A2-8-05 不提前; switch_mock/stalled/
S5 不借谓词轮顺手修。
