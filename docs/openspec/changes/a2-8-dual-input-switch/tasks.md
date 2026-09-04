# Tasks — a2-8-dual-input-switch

> 四栏纪律。Gate 链（用户两轮终裁冻结）：00 Probe（CLOSED）→ OQ 终裁 +
> Pre-Implementation 十项冻结 → 01 FRAME_SWITCH Execution Group MVP（T1-T12）
> → 02 真机 → 03 failure/supervision → 04 AV continuity → 05 archive+CI+merge。
> **A2-8 NOT CLOSED until 05。**

- [x] 1. A2-8-00 SOT Probe: 裁决事实断言复核（8 项全锚）+ 六问逐答
  （Q1 双 Pipeline 真机=A1 已证 inputs=2·Q2 三候选形态[inter 系倾向]·
  Q3 input-selector frame-boundary 原生·Q4 独立 trait 倾向·Q5 
  MultiInputWatchdog=Precondition Gate·Q6 观测点=selector active-pad+双
  PTS+首版只显式切换）+ 盒上元素实查（input-selector/inter 系/
  audiomixer 全在）+ 12 红线+禁塞 A/B 落盘 + OQ-1..5 交裁; 报告=
  docs/superpowers/reports/2026-09-03-a2-8-dual-input-switch-sot-probe.md
  `Contract: A2-8 终裁[12 红线+六问+否决直接编码]` | `Implementation: 已` | 
  `Verification: 六问全实锚·零 .rs diff` | `Gate: 无`
- [x] 2. 用户两轮终裁落盘（2026-09-03）: 第一轮 OQ-1..5 批准 + 01 批准;
  第二轮修正=**不批准直接编码，批准 Pre-Implementation Gate**——十项冻结
  [ExecutionGroup=Program execution boundary/Switch≠Backend SPI/
  SessionManager≠graph builder/Supervisor≠switch executor/topology=实现
  细节/FRAME first/**Video+Audio 成对切换=方案 A**/AV continuity mandatory/
  MASTER Deferred/failover Deferred] + OQ-1 降格[inter 系=候选
  Materialization 非架构合同] + T1-T12 验收矩阵（替代 T1-T5） +
  Desired≠Execution≠Observed 三分离 + 禁 Session.active_input/
  SessionInput.is_active + Event Identity Debt 不修[PipelineFault.pipeline
  兼容层维持，新增代码不扩大歧义] + watchdog 四观测非 God Object +
  01 完成标准=真实 Execution Graph+真实 A/B 切换+MultiInputWatchdog 落地
  （不停在设计完成）; A2-8-00 正式 CLOSED; 落 probe §7
  `Contract: 用户裁定权` | `Implementation: 已` | `Verification: probe §7` | `Gate: 无`
- [x] 3. A2-8-01 最小 FRAME_SWITCH Execution Group MVP（"最小、可验证、
  可监督的 Program-level FRAME_SWITCH Execution Group"，非"input-selector+
  双 Pipeline"）: SwitchIntent→SwitchExecutionPlan→SwitchExecutionAdapter
  链（独立执行面）+ ExecutionGroup 概念（inputs/switch/program output/
  supervision；SessionInput 原样）+ Program graph 构建（topology=实现细节，
  归 Program Execution/Switch 层）+ **Video/Audio 成对显式切换** + 六路 PTS
  观测点（A/B/Program×video/audio）+ MultiInputWatchdog（修正 bin L403+
  gates L165 首 handle 用法为 ExecutionGroup 四视角单实例，禁 for 循环双
  spawn）+ T1-T12 落地（mock 层先行，盒上 cargo 验证）; 12 红线全程
  `Contract: 00 终裁+Gate 十项冻结（probe §7）` | `Implementation: 已
  （五提交 0ee8ae2/4a07ca6/585ac23/72d9aa0/337a6b6: switch_execution 纯
  模型·contracts/switch SPI+Mock·GStreamerSwitchAdapter 真实物化·
  execution_group_observe_fold+薄壳·bin 双输入接线; gates L165 单输入
  gate 保持原样——单输入路径不动）` | `Verification: 盒上矩阵全绿
  [default 200·sim 200·mock 330(307+23)·bmd+gstreamer 202 含真实双测
  2/2]·clippy 四组合 -D warnings 全 exit 0·fmt clean·边界门禁[冻结面
  backend/session/events/supervisor/program/pipeline 零 diff·契约面签名
  零拓扑耦合]; **T5 边界实证（第三轮终裁拆分记录）**: 回切 selector 原生
  透传源时间戳可现 <1 帧 PTS 后跳，三态机如实检出 NonMonotonic——
  **T5 = 观测能力 PASS / 连续时间线 NOT YET PASS**（01 状态=FRAME_SWITCH
  execution PASS; Program timeline continuity DEFERRED/FAIL-PENDING-
  CORRECTION——架构级事实: source switching ≠ Program Timeline
  continuity, 真实 GStreamer 实证）` | `Gate: T1-T12 mock 层全落地+
  真实 GStreamer 切换实证; **A2-8-01 = IMPLEMENTATION COMPLETE +
  APPROVED（第三轮终裁, probe §8）**; **A2-8 NOT CLOSED**`
- [ ] 4. A2-8-02 Real Dual-Input Program Execution **Integration**（第五轮
  终裁重定义, probe §10）: 三件事一个完整集成——**MediaTap + Program
  Graph Lifecycle + Recover Reattachment**; 五层验收 **L1 Input**[
  DeckLink A/B 真实 RAW+PTS+health+bus]·**L2 Execution**[A/B 真实进入
  Program Graph]·**L3 Output**[Program output 真 frames+PTS]·**L4
  Timing**[A/B/Program 三列 PTS·切换前后 monotonic/continuous]·**L5
  Supervision**[A fail→B alive·B fail→A alive·Program fail 不误判 A·
  echo 不成第二物理 fact]; **G1 升级必修 Gate**（Session.stop 只停
  SessionInput 句柄 session.rs:726-763·stop_program 零接线=Program
  orphan 实证）; **C1 修正裁定**: 否决强制 HLS/RTMP output 获得 tee
  （内部 tap≠业务 OutputPlan）; 方向 A>C>B（Generic MediaTap 构造期能
  力>intervideo 桥>动态手术）; **C2 必修**: MediaTapAttachment 簿记
  （execution resource bookkeeping 非新 Device Identity Registry）+
  recover 重放 attach; 停止序 Program Stop→Tap Detach→Inputs Stop→
  Release; **模拟边界**: 01 videotestsrc=仅 GStreamer switch execution
  证明; FrameAligned≠TimelineContinuous 冻结; 执行序 **02-A Controller/
  Session 生命周期接线→02-B Generic MediaTap contract→02-C MediaTap
  materialization→02-D recover re-attach→02-E Program Graph 入 Session
  生命周期→02-F intervideo A/B 真机桥接→02-G Program Output
  observation→02-H Timing/PTS measurement→02-I 真机双 DeckLink 验证**
  `Contract: 第四轮五层+第五轮 Integration 重定义（probe §9.3+§10）` | 
  `Implementation: 02-A..02-H 已（十六刀提交链至 19326e8: E-1..E-6
  SessionStopHook+ProgramExecutionRuntime[creator=destroyer·close-path
  E-6]→F-01 唯一构造 bundle 三 trait view[backend/media_tap/bridge_
  observation 同一 Arc controller]→F-02 组合根接线+双输入回滚→F-03/
  F-04 Bridged inter 真机桥接+十项证据链→F-05 多切换+TargetAlready
  Active 真纵深修复→G/H BridgeObservation 一等事实+三列 PTS+recover
  降级+故障域[probe §19]→G/H-1 liveness 观察时钟窗口语义+tap_channel
  唯一来源收尾[probe §20]）; 第十六轮 02-I 代码前置三刀+serial 档+日期修正
  （P0-1 生产组合根 PortRegistry/P0-2 Capability SDK 位掩码证据/P1-1 双平面
  补偿 degraded/P1-2 IdentityStrength::Serial, probe §21）; 第十七轮两刀
  （①PersistentId 证据门+src_props Result belt——persistent-id=0 盲开
  路径封死·②PortId 碰撞双层防线=证据面告警+registry 装配层 fail-closed,
  probe §22; **实证: 盒上两张 DeckLink SDI 双工卡 in/out 同 port_id
  （e43d8f5a/f0f53b80）——十八轮终裁: collision closure=P1 架构债务非
  02-I 阻塞（Manifest 只声明 Input 时 registry 无别名可继续）**; 新遗漏
  实锤: derive_claims() 不消费 port_id 只取首个 "-input" resource=
  P1/N×M debt; **登记独立后续 change PORT-IDENTITY-AND-RESOURCE-
  ADDRESSING**: direction+connector+ordinal+PortId 迁移+Manifest+
  PortRegistry.get()+derive_claims()+Resource addressing 一次闭合,
  禁只修 UUID 不修 claims; 第十八轮: **VBMF_A2_8_DUAL_INPUT 正式 Gate
  入口落地**（gates/dual_input.rs——L0 形态 fail-closed/L1 三列分记/
  L2 双输入 Session+ProgramRuntime+Tap 桥/L3 帧增长非 PLAYING/L4 三列
  PTS 只测量+切换全序/L5 隔离+recover 复流+故障域不越域/Teardown 停止
  链; 盒上入口 smoke=真实 discovery→形态拒绝实证, probe §23）; 第十九轮
  **A2-8 Gate Hardening H1-H4+P1**（probe §24: H1 全链 fail-stop——L1a/
  b/c/d 任一 FAIL 即终裁不进 L2·L2b/L3 失败走完整 Teardown 后不进下一
  层·L4 FAIL 跳 L5; H2 L1d Port↔Resource closure——每设备恰一 Input
  Resource 且 ID==manifest port 规范派生[input_resource_id_for_port
  单源, derive_from_discovery 同源调用零行为变化], 4 纯函数测试, 零改
  SessionManager/derive_claims; H3 intent 携带已验证 port_id——实锚
  materialize 精确消费该字段[Some→registry 精确 connector 定位/无匹配
  生产 fail-closed], 原 None 回退路径弃用; H4 每端口一行一一对应证据
  [handle/port_id/conn/ordinal/cap/signal/dn 同行]; P1 删 6 处
  agent_state 直写——Gate verdict≠生产 health state）; 02-I 真机 Gate
  待用户双 SDI 窗口` | 
  `Verification: mock 356·bmd+gstreamer 230（含真实跨管线桥接/多切换
  一致性/G/H 三列证据/liveness 降级锁死/persistent-id=0 拒绝/碰撞防线
  测试/十九轮 L1d closure 四测试）·clippy 双组合 -D warnings clean·fmt
  clean; resolver gate 真机
  复跑双工卡碰撞告警×2 落盘; 真机五层矩阵=02-I 执行（VBMF_A2_8_DUAL_
  INPUT+现场 v4 双 Input port 声明 manifest）` | `Gate: 02-A..02-H
  全 CLOSED（probe §19-20）·02-I OPEN[**代码前置 CLOSED——第十九轮
  终裁 APPROVED（probe §25）: fe71b7c 冻结为 A2-8 验收候选基线;
  子项 Gate automation/H1 fail-stop/H2 Port↔Resource/H3 Intent↔Port/
  H4 evidence/Health-state isolation 全 CLOSED; SessionManager/
  derive_claims/PortIdentity/PTS normalization/N-input/Supervisor/
  recover SPI 零越界**; 仅剩 Real hardware=双 DeckLink+双 SDI
  L0→L5+Teardown; 硬件形态边界=两块独立单输入卡（collision closure/
  derive_claims/serial binding/audio 独立性/UUID namespace 统一=
  独立 change 不混入, 十八轮 §十二/§十三+十九轮 §11 定级）;
  **第二十轮 APPROVED/FROZEN/GO（probe §26）: fe71b7c=实现冻结基线·
  019f89e=裁决账本基线·禁再动 A2-8 代码·§9 验收矩阵已逐项映射
  Gate 实锚·首跑 FAIL 先留证按 A/B/C 分类（硬件/证据/代码）禁为
  跑绿改码·v4 manifest 由真实 Discovery 据实生成不手工美化;
  **02-I 真机首跑已执行（2026-09-04, probe §27 零代码）: v4 manifest
  已据实生成; fe71b7c bin 下 L0/H4/L1b/L1d PASS（H2 闭环真机成立）
  + L1a/L1c FAIL → H1 fail-stop 精确触发零会话创建; §11 裁决=B 类
  Real Hardware / Runtime Environment Preconditions（SDI-IN-1 无信号·
  SDI-IN-2 gst 输入稳态不可开[仅 device 0/1 可开, 08-27 时代 device 2
  可开]; 二十一轮精度修正: 根因未证明, 候选 B1..B8）; run1 陈旧
  cb78adc bin 对照=十九轮
  §3 P0 真机活体演示（教训: gates 真机复跑前必须 cargo build bin）;
  证据归档盒 ~/a2-8-02i-evidence/**
  ]——**硬件前置细化: ①双 SDI 信号源接入两卡输入 ②SDI-IN-2 gst 输入
  可开性恢复（候选 B1..B8 未定, 用户侧排查）; 恢复后无需修改代码,
  但必须以当日 Discovery 核验 runtime binding, device_number 变化
  则据实更新 v4（device-number=Runtime instance address 非 Device
  Identity）再复跑**; **第二十一轮 APPROVED/FROZEN/GO 维持
  （probe §28, 零代码）: 02-I≠"代码失败"而是 B 类前置条件未满足
  ——A 类 NONE FOUND, C 类十项 OPEN 禁为 02-I 临时修; d0ffff9 记
  2026-09-04/仓库 2026-09-03=evidence host clock/timezone mismatch
  （不影响技术裁决, 影响时间线审计）——复跑证据须同录 date -u/date/
  timedatectl/git rev-parse HEAD; 复跑执行序①-⑧=probe §28.3（⑥显式
  cargo build --bin media-agent-gates 必须）; **第二十二轮 APPROVED/
  FROZEN/GO 维持（probe §29, 零代码）: 主线切换"02-I 真机条件恢复
  与证据验收"——无新代码裁决无新架构决策; 环境证据包纪律（§29.2）:
  证据头五件套[date/date -u/timedatectl/git rev-parse HEAD/
  git status --short]·build 后 HEAD 复核=实际执行确为冻结版源·
  六问 Evidence Package[何时/时区/commit/是否冻结 bin/两卡 Discovery
  状态/A-B-C 归类]——比增加 Gate 断言更有价值**; **第二十三轮
  APPROVED/FROZEN/GO 维持（probe §30, 零代码）: 02-I 阻塞点重定义=
  Runtime Address/Provisioning Identity 闭环——现场推断 gst 序
  [dn0≈SDI(1),dn1≈SDI(2),dn2≈Mini]=correlation evidence 非 canonical
  proof[resolver.rs:903/:939-985/:528-535/:1022 独立复核:
  ManifestVerified=dn 可开+可选 serial/model 校验, hw-serial NULL+同
  model 下不证 Handle↔dn 同一硬件——语义边界登记不修, identity closure
  冻结]; Gate 无写死 dn[dual_input.rs:198/:232-249]; B4 占用降级为
  观察事实[dn2=Mini output-only 被 ball sink 用→自然开不了 input];
  旧 v4 正式作废; 两路输入=A 类已证[ffmpeg 1080i25/1080p25 双拉流];
  下一步=身份闭环核验[Discovery→Handle↔物理 BNC↔SDI(1)/(2)↔runtime
  probe→人工/物理/官方工具交叉确认→新 v4[Provisioning 意义]→frozen
  build→L0→L5], 禁猜 dn 写 v4**; **第二十四轮执行（probe §31, 零代码,
  02-I Provisioning Identity Closure Step 0/1/2 已跑）: Step 0 环境
  证据+sha256 68/68 盒==本地 8fea7ea(=fe71b7c 冻结)·Step 1 当日
  Discovery[dn0/dn1 PropertyMissing 可开无身份·dn2-7 StateFailed·
  legacy 全 Unresolved fail-closed——与首跑形态一致=常态非新故障]·
  Step 2 视觉指纹[dn0=电视临沂频道 1080i25→BNC#2·dn1=ball 1080p25→
  BNC#4]+杀源差分[**BNC#4 ball 源独立于 PID 577061/Mini 输出——
  'BNC#4←4K 卡'证伪, 对端设备现场待核**; 电视分钟级抖动三证]+复原
  [PID 992634 原命令行]; 映射 PROVEN=dn↔内容↔ffmpeg 名, CORRELATION
  ONLY=handle↔(1)/(2) iterator 序[待用户裁决/照片/官方侧证];
  "人工/物理/官方工具交叉确认"=Provisioning/Evidence 层非 Runtime
  前提; 候选 v4[SDI-IN-1→gst0·SDI-IN-2→gst1]待裁决不写死**;
  **第二十五轮执行（probe §32, 零代码）: canonical closure 零代码
  闭合[碰撞告警 port_id↔display_name × H4 handle↔port_id（VBMF
  确定性联结）×内核 PCI canonical[dv0=44:00.0/dv1=45:00.0·Mini
  芯片序列交叉验证]×内容指纹 ⇒ 4fa33dcb=SDI(1)=dn0=BNC#2=电视·
  6ede00d0=SDI(2)=dn1=BNC#4=ball; 实证 SDK 序≠dv 序≠PCI 序——旧
  v4=错绑作废正确]; v5 据实生成+02-I 第二次验收: L0/L1a[2/2
  production_grade 首次]/L1b/L1d PASS+L1c FAIL 双卡 signal=false→
  H1 fail-stop 零会话; **L1c 根因=Gate probe 采样窗口[resolver.rs:
  230-232 set_state 后仅 300ms 即读 signal·检测器锁定需 1-3s=
  结构性假阴性; ffmpeg 同分钟双输入出帧+gst 12s 手动双卡 false→
  true 翻转——A 类证据自动化候选, 冻结未修, probe 不修 L1c 确定性
  false=02-I 唯一代码级阻塞待裁决]**;
  **第二十六轮（probe §33, **A2-8-C1 授权落地**——APPROVED/FROZEN/
  CHANGE REQUIRED）: C1=Resolver signal 观察窗最小修[仅 resolver.rs
  +86/−3, commit 1c3032b: PROBE_SIGNAL_WINDOW=3000ms 自 PLAYING 起算/
  INTERVAL=100ms 重采样/锁定提前结束/超时 Some(false) fail-closed;
  Option<bool> 契约零变更·错误分类与生产绑定语义原样·gate 零改动=
  单一设备打开者; 3 单测=transient false×2→true 锁定/全窗 false
  fail-closed 重采样≥2/首采 true 恰 1 次]; 盒矩阵: sha256 68/68 盒源
  ==HEAD·fmt OK·mock 211→214·bmd+gst 233 全过·clippy ×2 -D warnings
  OK; **第三次 02-I 验收（v5, 15:59:24 CST, 五件套+bin sha 入 log）:
  L0/L1a 2/2/L1b/L1c PASS[dn0/dn1 signal=true——C1 真机成立]/L1d/
  L2a[双输入 session·H3 精确]/L2b[双 tap 83 帧]/L3[120→210·
  ValidMonotonic] PASS·**L4 FAIL**·L5 FAIL[H1 设计性跳过非独立失败]·
  Teardown PASS——8/10 verdicts EXIT=2 全链首次完成**; **L4 FAIL=
  确定性签名复跑 2 逐项复现[判据锚 dual_input.rs:644-648 唯一失败项
  =prog pts NonMonotonic; 切换机制 completed/observed=B/epoch=1 全对;
  A/B in 列互差 8-10ms·in/bridge 各列 ValidMonotonic 仅 prog 翻转·
  alive=false=复合字段推论 program_execution.rs:111-112]=A2-8-01
  第三轮已裁架构硬事实[switching≠Program Timeline continuity·
  Timestamp Normalization 四方案未裁]的真机表达——初步 §11 归 C 类
  候选待终裁; 工件: converter interlace 断言两跑各 9 条未定性]`;
  v4=INVALID/ARCHIVED·v5 保留**`
- [ ] 5. A2-8-03 failure/supervision 验证: watchdog 四视角观测穿
  RuntimeEvent→Custody 无跨设备污染 + Supervisor 边界（recovery only）
  `Contract: 02` | `Implementation: 待` | `Verification: 待` | `Gate: 待`
- [ ] 6. A2-8-04 Program Timeline / AV continuity 验证（第三轮终裁更名）:
  六路 PTS before/after switch 无 rollback/discontinuity/divergence/
  starvation; Program Timeline Continuity / Timestamp Normalization 方案
  裁决与验证（observation only，无 Engine——方案设计裁决属 02/04）
  `Contract: 03` | `Implementation: 待` | `Verification: 待` | `Gate: 待`
- [ ] 7. A2-8-05 archive+CI+merge（A2-8 收口唯一入口; 01-04 任一完成不宣布
  CLOSED）
  `Contract: 04` | `Implementation: 待` | `Verification: CI+归档` | `Gate: 待`
