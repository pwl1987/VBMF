# A2-8-03-02 Recovery Contract 设计探针/冻结提案（Design Probe / Freeze Proposal）

状态: **CONTRACT FROZEN（R49, 2026-09-05 用户 OQ-R1..R5 终裁: R1=案 A
全维持现状 / R2/R3/R5 接受 / R4 已闭）** —— 六面（F-1..F-6）冻结=既有
行为+边界+"不新增语义"约束; **零运行时代码收口, 无独立实现轮**; 冻结
记录见 §9。
授权来源: R46 裁决 §17（先 Group Watchdog 真机活体, 再 03-02 Recovery Contract;
开发线=comet/a2-8-dual-input-switch@f5eedcb, master=7745968 勿混）

## §1 R46 活体证据基线（03-01-G Final 的输入, 先于本提案记录）

- 生产 bin 真机跑 9.5min（MEDIA_AGENT_MODE=diagnostic + v5 manifest
  sha 7a52b498 + VBMF_DIAG_INPUTS=2, VBMF_OUTPUT_* 全缺省 ⇒ fail-soft
  纯分析零外推）: **组 watchdog 线程连续 tick 0→1120（57 条活体观测行,
  每 20 tick 一行）**——双设备（4fa33dcb=SDI(1)/6ede00d0=SDI(2)）三列
  实时 observed=true/advancing=true/bridge=Some(true), program_advancing
  Some(true); **分类器真机活体**: tick 0 domain=None（首采样无证据, 诚实
  缺席）→ tick≥20 domain=Some(None)（三列齐备全健康臂, `FailureDomain
  ::None` 变体真机产出）。证据盒 ~/a2-8-02i-evidence/2026-09-05-r46-g2g
  -group-watchdog-live/（header 五件套+production-run.log, bin ab361801）。
- 观测行使能披露: 组 watchdog 健康路径原为静默（仅 spawn/异常日志）——
  R46 增加两处**仅诊断输出、零决策逻辑**观测行（周期活体行+决策输入
  指纹行）, G-2-G Final Gate 的使能面; 决策输入仍只在故障动作路径装配。
- 活体缺口（如实）: 窗口内零自然故障（TV 未抖动; ball 源 PID 992634
  勿杀, 生产无注入面=gate-only R35 红线）→ 故障动作路径的决策输入活体
  指纹=0——该路径现有证据=纯函数测试+gate L5d 真实故障注入分类+本轮
  线程/三列/分类器活体。

## §2 As-Is 实锚（提案只从这些向前推, 禁凭空发明）

- **决策面**: `Supervisor::report_failure`（attempts/circuit →
  {Restart, Escalate}, 词表冻结 R43; 决策输入记录面 last_domain/
  last_attributed=R45-F, 逐决策替换）; 触发面 `fault_trigger_from_events`
  （echo 排除/身份化精确触发/nil 保守匹配零放宽）。
- **执行面**: watchdog Restart 分支 → lease 重校 → backoff →
  `ctrl.recover(own handle)` → report_recovered（MEDIA-RT-01 闭环;
  真机 L5.2 tap 簿记重放已实证）; Escalate 分支 = HealthChanged{
  manual_required}（reducer 派生 ManualRequired）。
- **证据面**: FailureDomain{None,Input,Bridge,Program} 单故障优先序
  Input>Bridge>Program（gh_rt_01; **R45 §6 重申: 禁多故障多维归因**）;
  AttributedFailures 双路（SharedPipeline scope 单值——单路归因编译期
  不可构造）; 三列窗口 FAILURE_DOMAIN_LIVENESS_WINDOW_MS=3000。
- **冻结约束**: MediaBackend::recover SPI 冻结（R34: 诊断注入 view 之外
  零改）; stop 注销语义不可反转; custody 七不; 单故障分类器禁扩。

## §3 六面契约提案（F-1..F-6; R49 冻结=按案 A 成文, 见 §9）

- **F-1 domain → recovery strategy**: 提案**不新造 Strategy 词表**——
  现有动作面已足够表达首版语义: Input 域 → 现状 recover(own handle)
  （=唯一有执行面的恢复）; Bridge/Program 域 → **不 recover**（无桥/节目
  平面执行面恢复能力, 禁凭空造）→ 维持 report_failure 判定走向 Escalate/
  人工面。**OQ-R1**: Bridge/Program 域是否需要显式"跳过 recover"语义
  （执行域读 last_decision_domain 分支）, 还是首版**全维持现状**（域
  证据仅记录不驱动执行分支——03-02 记账收口零代码）。
- **F-2 attribution → recovery target**: 双路 failed → 整管线 recover
  （target=own handle, 来自组输入表, 零跨设备——现状即此, 冻结）; 单路
  target 待 VideoPath/AudioPath scope 演进（禁猜, V0.3 债）。
- **F-3 retry/circuit interaction**: 提案 RestartPolicy 零变化（domain
  不参与 budget/circuit）。**OQ-R2**: 是否接受"域永不参与预算语义"冻结。
- **F-4 fail-closed semantics**: domain=None/证据缺席 → 既有 fault_trigger
  路径零变化（nil 保守匹配维持）; 无策略可执行 → 不新增自动动作。
- **F-5 Supervisor consumption point**: 提案=**执行域消费**（watchdog 读
  `last_decision_domain/attribution` 选择执行分支; Supervisor 判定/词表
  **零变化**）——与 R44 §7 红线一致（execution authority ≠ failure
  decision authority; Supervisor=recovery decision 非 strategy executor）。
  **OQ-R3**: 消费点放执行域是否可接受（vs Supervisor 判定域消费=需扩
  action 词表, 违反冻结, 提案不推荐）。
- **F-6 读取时序**: last_decision_* 写于 report_failure 内、执行分支读于
  同一调用栈其后——无跨线程竞态; 记录=该次决策的上下文（替换语义）。

## §4 OQ 汇总（待裁）

| OQ | 问题 | 提案默认 | 终裁（R49） |
|---|---|---|---|
| OQ-R1 | Bridge/Program 域执行分支: 显式跳 recover vs 全维持现状 | **全维持现状（03-02 记账收口零代码候选）** | **裁案 A（全维持现状）** |
| OQ-R2 | 域永不参与 restart budget/circuit 冻结 | 接受 | **接受** |
| OQ-R3 | 策略消费点=执行域（watchdog） | 接受 | **接受（R50 措辞校准: 预留消费边界, 非既存策略消费——见 §10）** |
| OQ-R4 | 故障动作路径决策输入活体指纹缺口: 接受"纯测试+gate L5d+线程活体"证据组合关闭 G-2-G, 还是要求自然故障长窗复跑 | 待裁 | **已闭（R47, 不重开）** |
| OQ-R5 | 03-02 若为零代码收口: 是否需独立实现轮 | 待裁 | **接受（无独立实现轮）** |

## §5 红线继承（实现期不可触碰）

MediaBackend::recover SPI / stop 注销语义 / Supervisor action 词表 /
单故障优先序分类器 / custody 七不 / 勿杀 ball 源 / 归因禁放宽（nil 不
自动绑定）/ EventLog 契约（P1-3 FIFO+丢弃计数 fail-closed）。

## §6 若 OQ-R1 裁"执行分支"的最小实现影响面（预估; **R49: 未启用**——OQ-R1 裁案 A, 保留为历史预估）

watchdog Restart 分支读 last_decision_domain → Input → recover(own
handle)（现状）; Bridge/Program → 跳 recover + 现有 escalate 面; +纯函数
测试; 矩阵+真机复跑。Supervisor 零改动。

## §7 R47 状态修正记录（2026-09-05, 用户裁决落账）

- **命名纠偏（R47 §八, 立即修正）**: 本文档曾以"设计冻结提案"为题——
  OQ-R1..R5 未裁决前**不是 Frozen Contract**; 准确名称=**设计探针/冻结
  提案（Design Probe / Freeze Proposal）**, 状态=DESIGN DELIVERED /
  FREEZE NOT YET COMPLETE。演进序: 设计提案 → OQ-R1..R5 用户裁决 →
  Contract Freeze → 实现。
- **hygiene 修正（R47 §九, defer）**: 主账/tasks 中"五面 F-1..F-6"计数
  有误（实为**六面** contract proposal faces）——按用户裁定**不在本轮
  单独制造提交**, 于下一次 03-02 正式冻结时顺手统一为"六面（F-1..F-6）"。
- **OQ-R4 已由 R47 裁决关闭**: 用户裁定=接受三层组合证据（生产线程活体
  +生产线程分类器活体+gate L5d 真机注入分类）关闭 G-2-G, **不要求自然
  故障长窗复跑**; Gate 分层模型: G-2-G-LIVE=PASS / G-2-G-CLASSIFY=PASS /
  G-2-G-FAULT=NOT OBSERVED（不阻塞）/ G-2-G-E2E=属后续 03-02 Recovery
  与 A2-8-04 范围。**G-2 Final=CLOSED（R47）**; 03-01-G=COMPLETE。
- 红线重申（R47 §十三）: **先裁 OQ-R1..R5 后按冻结 Contract 实现**——
  禁实现先于 Contract; 当前分支基线=f5eedcb。

## §8 R48 复核确认 + OQ-R1..R5 冻结包（2026-09-05, 用户裁决落账; 待裁版）

- **R47 复核=PASS**（用户四层复核: 用户报告→GitHub 实际提交→当前分支
  状态→语义一致性）——本 doc 命名/状态纠偏经 GitHub 实文核验属实
  （6f5735e）; 无运行时代码越界。
- **状态语言规则（R48 §十三, 永久锁）**: 本 doc 在 OQ-R1..R5 裁决冻结前
  **不得**称 Frozen Contract（状态行已载）; **G-2 Final=CLOSED ≠
  G-2-G-E2E 完成**——E2E 属 03-02/A2-8-04 且未实现未验证, 永久不入
  G-2 Final 记账; 四层 Gate LIVE/CLASSIFY/FAULT/E2E 边界永久保持。
- **OQ-R1..R5 冻结包（R48 §十五指令: 下一轮=逐项最终裁决; 最关键=
  OQ-R1）**:
  - **OQ-R1（关键）——两案**:
    - **案 A=全维持现状（用户维持推荐）**: FailureDomain/
      AttributedFailures 只作 evidence/attribution/决策输入记录, 不驱动
      新 Recovery Strategy 分支; 不新造 Strategy 词表; Bridge/Program 域
      不新造 skip-recover 语义。后果: 03-02=**零运行时代码 Contract
      close-out**（§3 F-1..F-6 成文冻结既有行为+边界, 非新行为）, 收口
      后直进 A2-8-04。
    - **案 B=执行分支（§6 预估面）**: watchdog Restart 分支读
      last_decision_domain → Input→recover(own handle)（现状）;
      Bridge/Program→既有 escalate 面跳 recover; Supervisor 零改。后果:
      最小实现+纯函数测试+矩阵+真机, 再收口。
  - **OQ-R2**: 域永不参与 restart budget/circuit（RestartPolicy 零变化）
    ——裁=接受/否决（提案默认=接受）。
  - **OQ-R3**: 策略消费点=执行域（watchdog 读 last_decision_*;
    Supervisor 判定/词表零变化, 与 R44 §7 红线一致）——裁=接受/否决
    （提案默认=接受）。
  - **OQ-R4**: 已 CLOSED（R47 组合证据裁决; R48 复核确认, 不重开）。
  - **OQ-R5**: 若 OQ-R1 裁案 A（零代码收口）: 03-02 不设独立实现轮,
    收口即直进 A2-8-04——裁=接受/否决（提案默认=接受）。
  - 随冻结顺手修（R47 §九已授权）: §3 标题"五面契约提案"→
    "**六面契约提案（F-1..F-6）**"。
- 裁决后状态语（届时解锁）: 裁案 A ⇒ Contract Freeze 完成, 本 doc 状态
  改 FROZEN, item-5 的 03-02 子项收口, A2-8-04（item 6）开启; 裁案 B ⇒
  先最小实现+矩阵+真机再收口。
- 基线: 开发线 6f5735e（R48 账单元后新头）·master=7745968。

## §9 R49 Contract Freeze 记录（2026-09-05, 用户 OQ-R1..R5 终裁落账）

- **OQ 终裁（用户 R49 裁决）**: OQ-R1=**裁案 A（全维持现状）**——
  FailureDomain/AttributedFailures=evidence / attribution /
  decision-input, **≠** 新 Recovery Strategy selector / 新 restart
  语义 / 新 escalation 词表; **不新增 Recovery runtime code**（用户原语:
  "不是少做一点, 而是更严格地保持已经形成的架构边界"）。OQ-R2=接受
  （域永不参与 restart budget/circuit; RestartPolicy unchanged）;
  OQ-R3=接受（**R50 措辞校准**: 预留消费边界非既存策略消费——当前唯一
  消费=Supervisor 记录 decision evidence; domain→recovery strategy=
  NOT USED / NOT NEEDED; 详见 §10）; OQ-R4=已闭（R47, 不重开——E2E 永不塞回 G-2 Final）;
  OQ-R5=接受（案 A 下无独立 Recovery implementation round）。
- **状态提升**: DESIGN DELIVERED / FREEZE NOT YET COMPLETE →
  **CONTRACT FROZEN（R49）**。六面（F-1..F-6）按案 A 冻结——**冻结的是
  既有行为+边界+"不新增语义"的约束本身**（零运行时代码收口）: F-1=域只
  作证据面不驱动执行分支; F-5=消费点契约边界**潜伏**（未来若消费=执行
  域, 本轮零新代码）; F-2/F-3/F-4/F-6=既有行为成文冻结。
- **术语一次性统一（R47 §九授权, 本节执行）**: 非引用处"五面"→"六面"
  恰三处修正——本 doc §3 标题 + 主账 §61 行 + tasks R46 段; 引用纠错
  原文处（本 doc §7/§8、主账 §62.4、tasks R47/R48 段中的引号内引用）
  **保留不改动**（改动=伪造被纠错对象）。
- **Recovery runtime code 状态语**: NOT NEEDED（案 A 裁决——非仅
  NOT STARTED）。
- **后续**: 03-02 收口直进 A2-8-04（SoT 探针 R49 同轮开启, 独立单元）;
  G-2-G-E2E 语义归属 03-02/A2-8-04 边界不变, 状态语言规则持续生效
  （"03-02=FROZEN" 自本轮起可说; "G-2-G E2E=PASS"/"Recovery E2E=
  COMPLETE" 仍然禁说）。

## §10 R50 修正记录（2026-09-05, OQ-R3 措辞校准——用户二层代码真相审计 §四; 零代码）

- **修正对象**: §4 表 OQ-R3 终裁格与 §9 OQ-R3 句原文"消费点=执行域既有
  last_decision_*"——可误读为"既存策略消费"。
- **修正后终版语义**: domain 生产点=execution/watchdog
  （execution_group_observe_fold→assemble_decision_input→
  classify_failure_domain/attribute_failures）; **当前唯一消费=
  Supervisor 记录 decision evidence**（report_failure 写 Status.
  last_domain/last_attributed, supervisor.rs:229-241; 读取面
  last_decision_domain/last_decision_attribution :203-210——当前零
  调用者=潜伏读取面）; **domain→recovery strategy=NOT USED /
  NOT NEEDED**（案 A=不新增该消费分支）。实锚: report_failure 体零
  domain 条件分支——决策仅由 attempts/circuit_threshold/
  RestartPolicy.should_retry（:93, 无 FailureDomain 参数）驱动;
  docstring :226-228 自证"本轮无分支消费（有证据与无证据同判）"。
- **禁表述**: "watchdog 已消费 FailureDomain 选择恢复策略"——与真实
  代码不符。§3 F-5/§6/§8 为提案/预估/待裁历史文本保留原文, 冻结语义
  以 §9+本节为准（F-5=消费点契约边界**潜伏**与本节一致）。
- 裁决不变: OQ-R3=接受; **CONTRACT FROZEN 状态不变**; 零运行时代码。
