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
  ; **第二十六轮终裁（probe §34, 落账零代码）: C1=PASS/CLOSED·
  02-I 整体=FAIL-PENDING-CORRECTION[L0-L3 PASS·L4 双维记账:
  L4-SWITCH=PASS·L4-TIMELINE=FAIL-PENDING-CORRECTION·L4 overall
  同·L5=SKIPPED BY H1 不计独立失败·Teardown PASS·v5 VALID/v4
  INVALID/ARCHIVED·Identity CLOSED·Port collision 不再阻塞]——
  精确语义=基础设施完成真实双输入切换执行闭环但 Program Timeline
  Continuity 未实现, 非基础设施失败; C-TIMELINE-01 正式登记[Program
  Timeline Continuity Gap=Architecture/Execution Adapter Gap 真机
  首证·排除 C1/identity/PortRegistry/SwitchAdapter/Supervisor/硬件
  不稳定·设计十问[authority/PTS 映射/epoch 共享/discontinuity/
  settle/offset/双 clock/encoder/recover/observation 证明]未冻结禁
  写码·开工门=Program Timeline Authority+A→B 切换 Video/Audio PTS
  连续映射]; 四不批准[①禁直接实现 Normalize——PipelinePlan.normalize
  声明未消费·三层禁再耦合 ②禁 H1 例外——evidence purity ③禁降 L4
  判据 ④禁 SwitchGraph/ExecutionGroup 内做 Normalize]; L4 证据原则+
  H1+Gate 表面全不动[L4 子项拆分=验收记账模型非代码授权——现行单
  bool 输出 FAIL 与 overall 口径零改码一致]; 保留边界[sampled_at_ms
  wall-clock 禁修 PTS media-clock·Bridge liveness 观察时钟分层];
  下一刀=独立 Timeline/PTS Normalization change 设计裁决（十问未裁
  不开工）; 边界披露: 1c3032b 用户侧未独立核验源码[分支已推送·C1
  CLOSED 依真机证据非报告自证]**
  ; **第二十六轮终裁补正（probe §35, 落账零代码——维持+两处账面修正）:
  用户独立核验升级[直接核 470f1a0/1c3032b/d123b45+八源文件+两次真实
  compare: 470f1a0→d123b45 仅两账面文件零夹带·fe71b7c→1c3032b 运行时
  代码仅 resolver.rs——§34.7 边界披露解除]; 补正一=C1 变更范围表述限定
  ["运行时代码变更只有 resolver.rs, 架构/账面文档同步另计"]; 补正二=
  **C1-P1 登记**[signal polling window 内异步 Bus Error 未二次 drain——
  resolver.rs:243-245 恰一次 drain_bus_error[fn :149]后轮询闭包仅采样
  signal :268-272 零 bus 交互, 晚到 Error 表现为 Some(false) 而非
  StateFailed 分类; 非 blocker·不重开 C1·不阻塞 C-TIMELINE-01; 未来修=
  poll 内可选 bus check 禁重新设计 Resolver]; **C-TIMELINE-01=
  CONFIRMED**[三代码证据: switch_graph.rs 全文件零 timeline 层+零
  clock/base_time 设置·Bridged capsfilter None :231·L4=真实 appsink
  buffer PTS 非簿记]; 新增维度=A/B 异构 1080i25↔1080p25 video format
  continuity 未定义须进设计; 设计十问 v2[Authority/PTS mapping/epoch
  共享/异构策略/settle/discontinuity-segment/recover timeline/
  Execution Fact/Observation 证明/落点禁令]+反假修复红线[禁
  max(last+dur, incoming) 假闭合·第一问=Authority 结构非 element 选择];
  影响矩阵 19 行照录[SwitchGraph=Gap 边界⚠️·PipelinePlan.normalize=
  核心入口之一⚠️·Resolver/ExecutionGroup/Supervisor/MediaBackend/H1
  全❌]; 最终状态机+**A2-8 Switch Execution 基础能力=PASS 与 02-I=
  FAIL-PENDING-CORRECTION 并立**; 执行令: d123b45 保持·下一轮直接进入
  A2-8-C-TIMELINE-01: Program Timeline Authority & PTS Continuity
  Design（十项冻结前禁写实现）; **设计 SoT 探针已开（2026-09-04-
  c-timeline-01-program-timeline-authority-design-probe.md, 零代码）:
  代码/真机/V0.2 spec 三面证据+十问逐问选项空间+四方案 A-D 对照+OQ
  待裁清单**
  ; **第二十八轮=C-TIMELINE-01 十问终裁（设计探针 §11 落账+主账 §36
  跨账引用, 零代码）: OQ-1..12 全裁——架构方向冻结=Program Timeline
  Authority + Clock-Segment Timeline + Source Segment Mapping[B 为主+
  A 执行机制·C"出口再生成"正式废止·D 不采用]; OQ-1 Authority=
  Program Execution 层 TimelineAuthority[禁 ExecutionGroup/Supervisor/
  MediaBackend/单 pipeline/出口 muxer·不做大型独立 Engine·Domain
  拥有语义/Adapter 拥有执行·ProgramExecutionRuntime 四组件结构];
  OQ-2 PTS=Source Segment Offset Mapping[SourceSegment{source_id,
  program_epoch,source_start_pts,program_start_pts,offset}·mapping_B=
  Program anchor−Source B anchor·max(last+dur,incoming) 永久禁止];
  OQ-3 V/A 共享 Program Epoch 不共享数值序列·switch_epoch≠
  program_epoch[一次成功切换→timeline epoch+1·recover 可独立变];
  OQ-4 Timeline 与格式归一化解耦[当前=Switch Boundary Adaptation+
  Format Contract 显式声明不保证无缝 format continuity·格式策略=
  独立 Program Media Format Policy]; OQ-5 settle=状态语义[Stable→
  SwitchRequested→SwitchExecuted→TimelineTransition→Stable·settle
  期间 PTS 必须已属新 timeline·≠gap≠freeze]; OQ-6 Discontinuity
  Domain+Gst Segment/Event 双层+PtsState 四态[+DiscontinuityDeclared
  ·declared≠unexpected backward]; OQ-7 Recover 本轮不实现语义冻结
  [Soft/Hard 两类·Supervisor 只决定 recover 不拥有 Timeline·接口留
  A2-8-03]; OQ-8 TimelineMapped 结构化 Fact[≠TimelineHealthy];
  OQ-9 TimelineObservation 专门证据面[observed_at=wall clock 禁入
  program_pts·"真的完成"七条·pts>prev 永远不足]; OQ-10 删裸 bool
  normalize→TimelinePolicy[本轮零代码]; OQ-12 三时钟职权切开
  [Timeline Authority/AVSync Manager/Channel Reference Clock 互不
  越权]; 八红线 R1-R8[wall-clock 禁修 PTS/max 假闭合禁/Authority
  不进 ExecutionGroup·Supervisor·MediaBackend/Gst Segment=Adapter
  机制非 Authority/格式转换不偷做/Fact≠Healthy]; 不触碰=已 PASS
  各层+SwitchExecution/SessionManager/Resolver/PortRegistry/
  ResourceRegistry/Supervisor·只解 L4-TIMELINE·02-I 状态不变[设计
  ≠Gate PASS]; **Design Freeze 15 项已形成**[2026-09-04-c-timeline-
  01-design-freeze.md: TimelineAuthority Contract/ProgramTimeline/
  ProgramEpoch/SourceSegment/TimelineMapping/Discontinuity/
  TimelineMapped Fact/TimelineEvidence/V-A 双平面/状态转移/GStreamer
  Adapter contract/1080i-p 边界/Recover 接口/L4 复证/不变量+失败
  条件]; 下一动作=开 implementation change（冻结后）; C1-P1/
  converter interlace/PORT-IDENTITY/canonical UUID namespace 四独立
  债隔离禁顺手修**
  ; **第二十九轮=Design Freeze 复核通过+Implementation Change 正式
  开启（设计探针 §12+主账 §37, 零代码, commit f3158a0 复核有效）:
  用户实核 Freeze 核心内容与十问终裁一致——ProgramExecutionRuntime
  现有组合根增设 TimelineAuthority 零新 Engine 层/三时钟职权分离/
  epoch 拆分/映射语义+双禁/Fact≠Healthy/evidence 独立/四方案=B 核心+
  A 机制 C-D 淘汰; 工程状态表冻结[C1 CLOSED·C1-P1 隔离·L0-L3+
  L4-SWITCH PASS·L4-TIMELINE FAIL-PENDING-CORRECTION·L5 SKIPPED
  BY H1·02-I FAIL-PENDING-CORRECTION·Design FROZEN·Implementation
  =下一阶段·converter interlace+PortIdentity/UUID=独立队列]; 纪律=
  先实现前拓扑探针/Impact Map→最小变更面冻结→再写代码·十项落点
  钉死[4/5/6 项须真实 Rust/GStreamer API 实证不凭架构图猜]; **
  Implementation Impact Map 已交付**[2026-09-04-c-timeline-01-
  implementation-impact-map.md: As-Is 拓扑实锚=组合根两生产构造点
  [bin:464+gate:480]/Plan 面 8 构造点 2 生产[self_test+materialize]
  零消费/切换全序[plan_switch→begin→switcher.switch[set_active 成对
  +P1-1 回滚]→settle→observe→complete_switch·watchdog 只观测不驱动]/
  观测链[appsink→HEALTH_ARCS→PipelineHealth·桥=tap probe=全 adapter
  层唯一 probe]/GStreamer 高层 API 全仓零存量; 盒上 GStreamer 1.28.2
  实证=input-selector 零时间戳改写[drop-backwards=丢帧藏证禁入方案·
  sync-streams/sync-mode 行为面·pad 级 running-time 可读]/intersrc
  do-timestamp=false 原始终戳透传=双时钟域直通机制根源/**identity
  single-segment 真实存在**[eat segments appear as one segment=方案
  B 现成 primitive·精确数学留 sim 实验锚定]; gstreamer-0.23.7 crate
  实证=event::Segment::new+SegmentBuilder[crate event.rs:778/:2493]/
  Pad::send_event[pad.rs:369]/Element::send_event[element.rs:138]/
  PadProbeInfo::buffer_mut[pad.rs:68=in-probe PTS 重写可行]/
  EVENT_DOWNSTREAM probe; 十项逐项现状锚+候选+**OQ-IMP-1..7 待裁**
  [1 normalize 删 vs Policy+wire/2 timeline 入 adapter 路径/3 执行点
  组合须 sim 实验裁/4 evidence 读出面/5 adapter 微观序实验裁/6 失败
  三结局谓词/7 L4 新谓词]; 最小变更面候选 9 行[新 program_timeline.rs
  纯 Domain+contracts/switch 契约扩展+program_execution 挂点+
  switch_graph 执行面+pipeline.rs 字段处置+dual_input L4 谓词+bin
  接线+tests·controller MediaTap/watchdog/supervisor/resolver 零触碰];
  执行序=裁纯设计项+授权 sim 实验刀→Domain→契约+Mock→Adapter→Gate
  升级→真机复跑 §29.2]——待用户裁 OQ-IMP 后冻结最小变更面进实现**
  ; **第三十轮=OQ-IMP-1..7 裁决+SIM-01 实验刀完成（设计探针 §13+主账
  §38+SIM-01 报告, 落账零生产代码）: 5 ADOPT——IMP-1 normalize→
  TimelinePolicy[SourceNative/ProgramTimelineMapped·删 normalize 语义·
  禁含糊 bool]/IMP-2 走现有 Plan/materialization 链[禁新 Timeline
  trait/Port/SPI·ProgramEpoch authority 永在 ProgramExecutionRuntime/
  TimelineAuthority]/IMP-4 TimelineEvidence=Adapter 装配+Runtime 独立
  读取[禁塞 PipelineHealth·evidence 非 authority 禁自动接受]/IMP-6
  失败三结局 Preserve/NewEpoch/FailClosed[continuity 不可证→epoch++·
  禁硬接 PTS=R2 绝对禁区·禁第四种猜测成功]/IMP-7 L4-TIMELINE=Timeline
  Mapping Evidence 七合取[TimelineTransitionEvidence 结构·'B 是否按
  声明 SourceSegment 映射合法进入同一 Program Timeline 且 V/A 双连续
  成立']; IMP-3/IMP-5 授权 sim 实验; **SIM-01 已执行**[2026-09-04-
  c-timeline-01-sim-01-experiment.md·9 变体 2583 行·盒 ~/ct-sim-01
  sha256 归档·工程不入库]: F1 inter 桥按接收墙钟重定基=200ms 生产者
  基差跨桥后仅剩 0.1-0.3ms 相位差[真机 8-10ms 同源·NonMonotonic=相位
  回退]/F2 翻 active-pad 自然转发 stream-start(B)→caps→segment(B) 到
  appsink=免费边界标记/**F3 identity single-segment 只吃段不修 PTS
  [appsink 单 segment 但 PTS 回退 −0.155ms]=吞段假阳性实证 禁作机制
  或证明面**/**F4 控制线程 Pad::send_event(Segment) 两序均被拒**/
  **F5 selector 后 per-plane BUFFER probe+Domain 声明映射[anchor−
  B_anchor offset]=完整可行[vd-pre backward=0·B 首帧精确落 anchor=
  A 末帧+40ms·121/121 节拍规整·aud-map 162/162=V/A 双平面独立]**/
  F6 微观序 pre-flip 安装结构性无竞态[规范序候选]·post-flip ~1ms 赢
  竞态[窗口真实窄]+**附带发现 set_property 后立即 readback=旧值
  sink_0 9/9 而流已切→'已执行'证明禁立即 readback 生效边界=下一
  缓冲**/F7 基线复现生产 L4 签名[实验有效性锚]; 顺带发现 gstreamer-rs
  0.23 无公开 parse_launch[auto/functions crate-private·生产程序化构链
  不受影响]; IMP-3/IMP-5 候选结论待终裁[执行点=selector 后 per-plane
  probe 声明映射+F2 自然段边界·序=取锚→声明映射→pre-flip 安装→翻
  pad→生效边界=下一缓冲→Observed 走帧/事件序列]→冻结最小变更面→正式
  实现批次; 实验禁改清单全守[normalize/PipelineHealth/L4/SwitchGraph
  正式逻辑/Production graph 零触碰]**
  ; **第三十一轮=IMP-3/IMP-5 终裁+IMP-2 实现层纠偏+最小实现批次开工（设计
  探针 §14+主账 §39, 终裁落账零代码 commit 先行）: IMP 最终表=IMP-1 ADOPT
  [normalize 删·TimelinePolicy 取代]/IMP-2 ADOPT WITH CORRECTION[
  PipelinePlan=ingest 只承载声明·Program Timeline 走 ProgramExecution
  Runtime→TimelineAuthority→ProgramTimelinePlan→Adapter·实锚
  build_program_pipeline 不消费 PipelinePlan]/IMP-3 ADOPT[selector 后
  per-plane EVENT+BUFFER probe·selector 前不能证 Program 侧事实·identity
  =吞段假阳性禁 proof·F4 精确表述=控制线程外部注入 sent=false 非主注入
  机制]/IMP-4 ADOPT[ProgramExecutionObservation{program,timeline}=observe
  () 单一 observation surface 最小演进]/IMP-5 ADOPT[①-⑩ 微观序冻结·
  生效边界='事件确认+下一 Buffer'·active-pad readback 只能辅助·
  SwitchExecuted≠TimelineTransition complete]/IMP-6 ADOPT[Preserve=epoch
  N 保持·NewEpoch=N→N+1·FailClosed=failed 不得当 Stable]/IMP-7 ADOPT[
  L4=九项合取 TimelineTransition proof 非 PTS monotonicity test];
  PtsMonotonicity 升级四态+DiscontinuityDeclared[禁洗状态]; recover 本
  change 不碰[A2-8-03]; SwitchGraph 侧=adapter TimelineExecutionState 每
  plane 独立+共享 program_epoch[非 Authority]; source identity=声明+
  Event+Buffer 三件闭合禁瞬时 readback; V/A 双 mapping 独立共享 epoch;
  排除清单照录[SessionManager/Resolver/PortRegistry/ResourceRegistry/
  Supervisor/MediaBackend::recover/MediaTap/C1/switch correctness/
  1080i-1080p]; **SIM-01 足够无需二轮实验·Batch 1 Domain+contract+Mock
  开工→Batch 2 GStreamer Adapter+L4→真机复跑**; 实现纪律=Authority 声明
  +downstream Event/Buffer 证据+Runtime 闭合 TimelineMapped; 披露三项[
  observe() 契约演进机械波及 watchdog/registry/dual_input .program 路径
  ·epoch 口径按 §十一 实现[Preserve 保持·与 Freeze §3 字面差异已披露]
  ·install 路径=既有 trait 最小方法默认 fail-closed[GStreamer 实装=
  Batch 2]]**; **第三十一轮 Batch 1 已落地（设计探针 §15, 生产代码 commit）:
  program_timeline.rs 纯 Domain 全量[SourceSegment declare=anchor−anchor·
  TimelineAuthority ①-⑩ 状态机·三结局 Preserve=epoch 不变/NewEpoch=rebase
  不改 PTS/FailClosed 终态·四态·§8 恰十键 wire 锁·timeline_rt_01 ×12]+
  pipeline.rs[TimelinePolicy 取代 normalize 8 构造位·PtsMonotonicity
  +DiscontinuityDeclared 四态+declared 观测·既有 observe 语义零变化]+
  contracts/switch.rs[ProgramExecutionObservation{program,timeline}=observe
  () 单一组合面+install_timeline_transition 默认 fail-closed]+switch_mock
  [双模式出口 legacy 逐字节保持+映射后源流 F5 同构+边界 tick 无缓冲 F6
  同构+pre-flip 安装联动+switch_rt_02 ×3 含 Authority 全链 Preserve 闭环]+
  机械适配[watchdog 1/registry 2/dual_input 恰 7 绑定行 L4 判据零变化/
  switch_graph tests .program]+switch_graph observe=timeline no_evidence
  诚实边界[Batch 2 probe 前不伪造]; 盒矩阵 fmt OK·default **217**·mock
  **377**[+18 新测]·bmd+gst **236** 全过·clippy ×2 -D warnings PASS;
  Batch 2[switch_graph EVENT+BUFFER probe+TimelineExecutionState+GStreamer
  install+Runtime 挂 Authority+L4 九项合取+真机复跑]未动零越界**
  ; **第三十二轮=Batch 1 复核终裁 APPROVED+两前置+Batch 2 开工令（设计探针
  §16+主账 §40, 终裁落账零代码 commit 先行）: 四项成立[Domain/GStreamer
  分层·ExecutionGroup 零污染·observe 机械波及无隐藏语义扩散·GStreamer
  诚实缺席]+PipelinePlan 边界正式关闭不回头+SwitchExecution 链零污染
  [on_switch_executed 禁成第二 switch state machine·两状态机经 plan/
  executed 关联各自拥有]; install=只做 Plan→TimelineExecutionState 安装
  禁'install 完=TimelineMapped'; offset 只能源于 Authority 声明 AnchorPair
  禁 probe 重算覆盖; V/A 双 selector 各挂一套 state/probes 禁 audio=video;
  PtsMonotonicity≠PlaneContinuity 不合并; video 单行载体仅限 wire/evidence
  serialization 禁 audio 降格; Mock≠GStreamer 证明[Batch 2=风险高峰];
  **两前置直接处理: ①BLOCKER-DOC Freeze §3 epoch 文本统一[Preserve=同
  世代不变·NewEpoch/Hard Recover=+1·switch_epoch=执行事件/segment_id=
  段世代/program_epoch=不连续时间线世代三职权分离——否则 ProgramEpoch
  退化成另一 switch counter] ②BLOCKER-IMPL no_evidence 消除虚假 epoch=0
  [携带当前已知 epoch·十键形状不改 Option]**; 三风险[P2 i64 差值算法·P1
  no_evidence·P1 段历史累积不覆盖 Batch 2 锁测试]; Batch 2 十四步顺序
  锁定[1 docs epoch 统一/2 no_evidence/3 TimelineExecutionState/4-5 V+A
  EVENT probe/6-7 V+A BUFFER mapping probe/8 GStreamer install/9 Runtime
  挂 Authority/10 orchestration ①-⑩/11 timeline 真证据装配/12 L4 九项
  合取/13 双轨回归/14 真机复跑仅矩阵绿后]; 禁做照录[Authority 不入
  SwitchGraph/set_active 不产 epoch/readback 不判生效/identity 不用/
  send_event 主路径不用/recover·Supervisor 不碰]; 02-I 保持
  FAIL-PENDING-CORRECTION**; **第三十二轮 Batch 2 已落地（设计探针 §17,
  十四步全执行）: ①Freeze §3 统一 59aec43[docs-only·三计数器职权分离]
  ②no_evidence(epoch) 修正+段历史只增不改锁测试 ③⑧switch_graph
  TimelineExecutionState+install 实装[pre-flip 联动+V/A 一致性·只安装]
  +switch 联动拒收/成功仅置 executed 不产 epoch ④-⑦attach_plane_probes
  [EVENT_DOWNSTREAM Segment 声明驱动身份+BUFFER 施加声明冻结 offset
  make_mut set_pts·无声明透传零改写 legacy 保持]+sink pad 分支观察探针
  [锚证据] ①契约 sample_switch_anchors[纯观测 fail-closed]+timeline_
  execution_facts[同一 trait 两证据方法默认 fail-closed/None] ⑨⑩Inner.
  timeline TimelineAuthority+switch_program ①-⑩[基准+锚→declare 唯一
  offset 点→install→begin/switch 失败 abort→轮询 facts→Authority 校验
  闭合·超时 EvidenceInsufficient FailClosed→settle 3 轮·停滞超时归故障
  面→confirm+complete_switch Observed 驱动·非第二 switch state machine]
  ⑪observe_execution[program=adapter 平面+timeline=Authority snapshot
  Domain SoT·adapter 行=执行侧原始证据双行分工] ⑫L4=rt.switch_program+
  九项合取[L4-SWITCH 语义保持∧Preserve∧declared==observed∧V/A
  Continuous∧无未声明回退∧epoch 一致∧mapped>pre∧出口≥边界帧]; 盒矩阵
  fmt/default 217/mock 381[+4]/bmd+gst **237[+1 含真实 GStreamer 全链
  timeline 测试=Simulation 形态真实探针 Runtime ①-⑩ 2.18s Preserve——
  SIM-01 F2/F5/F6 在生产 switch_graph 实证]**/clippy×2 全绿; gstreamer-rs
  实锚 make_mut 返回 &mut BufferRef 非 Result; 披露[三 trait 证据方法·
  双 timeline 行分工（NewEpoch 后 adapter 行滞后 Domain 真值→裁决面恒
  Domain]·锚公式 F5 同构·settle 停滞不 FailClosed]; ⑭真机复跑=§18;
  **⑭ 已执行（2026-09-04 22:15 CST, 设计探针 §18+主账 §41）: HEAD=3ff66ad
  已 push·bin 31e294f4·68/68 源 sha==HEAD·v5 当日复核 2/2 production_grade
  ·双卡 signal=true·证据=盒 ~/a2-8-02i-evidence/2026-09-04-2230-batch2-
  ctimeline（run.log sha 5758c42d）; **L1a-d/L2a/L2b/L3/Teardown 8/10
  PASS——L4=switch_ok true ∧ outcome=Preserved[真 DeckLink 双输入全链:
  offset 118799ns 相位级·Segment(B) 观测·首枚映射缓冲过证据校验·V/A
  双平面 Continuous·无未声明回退·epoch 保持 0·post-switch prog
  ValidMonotonic=A2-8-01 确定性 NonMonotonic 签名消失]**; L4 overall
  FAIL 单点=九项合取转写 `mapped>pre` 严格大于 vs 真机零隙拼接精确相等
  [冻结语义=非回退 ≥·锚公式结构性保证 mapped∈[pv,pv+delta] 恒不回退]
  =**B 类 Gate 判据转写·未改码待用户裁决（单字符 >→>=）→复跑预期 L4
  PASS+L5 首次真机注入**; ffmpeg decklink 打不开=观察事实非 gate 依赖;
  interlace 断言/pad_unlink 工件同历跑（隔离队列）; 02-I 仍
  FAIL-PENDING-CORRECTION（8/10）——性质迁移=架构缺口→验收判据单点
  转写**; **第三十三轮终裁（设计探针 §19+主账 §42, 断言实物核验后落账）:
  Batch 2 ✅ APPROVED[14 项关闭——三职权分立/①-⑩/SIM-01 一致/声明→冻结
  →buffer 三段闭合/F6/真机 Preserve=核心问题实际解决/双面分工/消费面/
  Teardown-Recover 零污染]; L4 `>`→`>=` 正式批准[B 类 Gate-only 单字符·
  禁趁机重写其余八项·冻结语义非回退=≥·零隙拼接 equal≠backward]; **NewEpoch
  rebase 缺陷登记 P1[program_timeline.rs:682-688 rebase 沿用旧 plan
  offset 未按新 boundary 重算——不变量 new_segment.offset==
  program_start_pts−source_start_pts·回归四条=Preserve/NewEpoch/A→B→A
  history/append-only·不阻断本轮·C-TIMELINE-01 Final Close 前必修·不混入
  本次小修]+on_mapped_buffer 先行 DiscontinuityDeclared[616 先于连续性
  判定]=NewEpoch 修复时锁回归**; 令=修正后立即真机复跑·H1 开 L5 完整
  真实证据必拿[A fail→B alive/recover A→bridge real flow/B fail→A alive/
  failure-domain classification]; L5 全绿→02-I 具备正式收口评审条件**;
  **第三十三轮执行（设计探针 §20+主账 §43, commits b856a04+ d5059e2 fmt
  残留）: 盒矩阵 fmt/default 217/mock 381/bmd+gst 237/clippy×2 全绿·
  69/69 源 sha==HEAD·bin c0efdfad·v5 当日复核·证据=盒
  ~/a2-8-02i-evidence/2026-09-04-2340-l4fix-l5run（run.log sha 4616d680）;
  **L4 Timing/switch+timeline(A→B) 首次真机正式 PASS[九项合取全绿:
  Preserved epoch 0·映射闭合 6937849283+33301642==6971150925 逐 ns·V/A
  Continuous·declared==observed==SegmentId(1)·无未声明回退·mapped==pre_v
  再次精确相等（零隙拼接复现→>= 修正被真机证实必要且充分）·post prog≥
  mapped]**; **L5 首次真机执行（历史两跑均被 H1 跳过）FAIL=C 类候选留证
  未改码: L5.1 A-fail→B-alive=true 真机成立; L5.2 单一根因=stop/recover
  契约结构性冲突[MediaBackend::stop=终态注销（P0-2 防句柄泄漏·
  controller.rs:314-331）vs recover 第一步 instances.get 取 plan
  （controller.rs:220-227）——stop→recover 生产必败]; Mock stop/recover 均
  no-op Ok+L5 序列仅真机 gate 执行=Mock≠GStreamer 预警在 recover 契约面
  成真; L5.3/L5.4/Teardown session_stop=false 全为级联（Teardown 本体无
  独立缺陷）; 候选方向三选一待裁[L5 注入面改造/Session 层
  recover-from-plan/recover 语义归属 A2-8-03 supervision 面]; 红线=
  MediaBackend::recover 不改+stop 注销语义不可反转; 新工件=Bus watch
  MainContext already-acquired WARN（隔离队列）; 02-I 仍
  FAIL-PENDING-CORRECTION（8/10; 失败集迁移 {L4,L5-skip}→{L5,
  Teardown-级联}）**; **第三十四轮终裁（主账 §44+设计探针 §21 跨账,
  断言实物核验后落账——五断言全证实[watchdog.rs:212-233 生产恢复链/
  session.rs:193 SessionInput 恰两字段/controller.rs:217-299 同 handle
  原 plan 重建/mock.rs:129-134+228 no-op+bridge_stall 钩子/registry.rs
  :162-199 bundle 三 view 单构造]）: **方案 1 正式批准——A2-8-02-I —
  Diagnostic Runtime Fault Injection**[注入'运行故障'非'生命周期终止':
  真实执行面停流·handle+HEALTH_ARCS 保持登记·recover=生产行为同 handle
  原 plan 重建; 被证伪的是注入方式非生产恢复链]; 落点=GStreamerPipeline
  Controller **第四 trait view**（F-01 同源原则·禁入冻结 SPI·禁 Session/
  Supervisor 侧）; 方案 2 暂不批准[Session 无 plan 持久引用, 真做=Session
  重构修 Gate 错误]; 方案 3 不作替代[生产恢复链实存, 推迟=伪装未来功能];
  定性: recover 本体无阻断·stop→recover=非法组合·Teardown 本体 PASS=
  L5 注入级联不单独开缺陷; 红线七条[recover/stop 不改·Session 不换
  handle·Supervisor 不注入·SPI 不加·recover 不推 03·Timeline 不混修];
  第一版禁 Bus Error 合成事件[Observation Fact≠Synthetic Event]须作用
  实际执行面; Mock 禁假装真实 registry[bundle mock 分支 diagnostic=None];
  02-I 收口条件=13 项全 PASS→Final Close Review**; **第三十四轮执行
  （主账 §45+设计探针 §22 跨账, commits 374f5c0 账+bb1360c 实现）:
  交付=contracts/diagnostic.rs 新契约面[DiagnosticFaultInjection 单方法
  inject_runtime_stall·仅诊断·禁入冻结 SPI]+controller 第四 view
  [gstreamer-backend cfg·set_state(Paused) 真实执行面停流·instances/
  HEALTH_ARCS 登记保持·不合成 Bus Error]+MediaAdapterBundle 第四字段
  [同源第四 clone·mock 分支=None 诚实缺席]+gate L5 5.1/5.3 stop→
  inject_stall[观察仍唯一裁判·注入失败只打证据行]; diagnostic_rt×3
  [结构: 注入后 instances 保持+recover Ok/行为: self_test 真元素帧冻结
  →recover 复流/fail-closed: stop 后注入拒收]; 矩阵 fmt/default 217/
  mock 381/bmd+gst 240[+3]/clippy×2 全绿·70/70 sha==HEAD·bin 7e665e3b;
  **真机复跑（09-05 00:19 CST, 证据盒 ~/a2-8-02i-evidence/2026-09-05-
  0020-r34-diag-inject, run.log sha 83017553）: 9/10 历史最高——
  L4 PASS 连续第三次[Preserve·映射逐 ns 闭合·offset 452126ns·V/A
  Continuous·epoch 0]; **L5.1 PASS[注入=真实运行故障: inputA 停·
  bridgeB 活·program 走]+L5.2 PASS 首次真机[recovered=true·bridgeA
  复活·degraded=false·recover tap 簿记重放成功 handle=1——33 轮 C 类
  stop→recover 结构性缺口经方案 1 真机闭环】+L5.3 PASS[bridgeB 死·
  bridgeA 活]+Teardown PASS[session_stop=true·handle 全程在册=级联
  彻底消失】; 唯 L5.4 FAIL=B 类候选留证未改码[A行=None 期望 Program·
  B行=Input✓——根因=下游集料排空 runway（默认 queue≈200 buffers≈8s@
  25fps+inter 缓冲）与采样窗[B 注入后 8-11s]物理重叠·program 帧计数
  仍在增长→classify(true,true,true)=None 全健康臂=语义正确; 候选待裁:
  ①drain-wait 加长/采样推后 ≥12-15s ②相对注入时刻锚定 ③queue 水位
  读取=不推荐过度工程]**; 02-I 收口清单 14 项中 13 PASS 唯 L5.4 待裁;
  工件: interlace 断言同历跑·pad_unlink ×4 间歇复现·MainContext WARN
  同 recover 新管线建立（隔离队列）**; **第三十五轮（主账 §46+设计探针
  §23 跨账）: L5.4 终裁=方案②「相对故障注入时刻锚定」正式批准**
  [Fault t0→Drain Grace→q1→固定 GAP→q2 时序语义冻结·grace 成 Gate
  显式观测窗口参数 L5_PROGRAM_DRAIN_GRACE·wait_until 锚定非流水 sleep;
  方案① 12-15s 机械加长=不采纳为正式方案·方案③ queue 水位=不批准
  （FailureDomain 封闭四词表不扩）; classify (true,true,true)→None 语义
  冻结禁改; 裁决五项代码主张全实锚证实[classify 优先序 :186-199/L5.4
  现流 :785-808/双 queue 默认容量 :397·441/appsink sync·async=false
  :399-400·443-444/session hook 先于 Input Stop 且失败不截断 :782-798];
  执行边界=只改 gates/dual_input.rs·禁改九面[program_execution/
  diagnostic/controller/switch_graph/session/backend/program_timeline/
  Supervisor/SPI]; 后续序=修改→fmt→矩阵→bin rebuild→真机→核对 14/14→
  NewEpoch P1 独立刀→Final Close→A2-8-05 archive; grace 初值 15s=实测
  下界 t0+11s 仍推进+余量·不足则证据回裁; 隔离队列维持[pad_unlink ×4/
  MainContext WARN 不顺手修]]**; **执行（主账 §47+设计探针 §24）:
  3c0b2af 单文件 +18 行**[L5_PROGRAM_DRAIN_GRACE 15s 常量+5.3 t0 锚点+
  5.4 q1 前 wait_until 剩余等待·q1/GAP/q2/classify 判据零变化·禁改九面
  零触碰]; 盒矩阵 fmt 绿/default 217/mock 381/bmd+gst 240/clippy×2 绿·
  bin baf5f895·sha 80/81 唯 DIFF=Cargo.lock[盒 cargo v4 重写·Cargo.toml==
  HEAD·历史清单不含 lock·非本轮引入·披露]; **真机复跑（09-05 00:47 CST,
  证据盒 2026-09-05-0047-r35-l54-anchor, run.log sha 23a5f860）: 9/10
  复现——L4 PASS 连续第四次[Preserve·epoch 0·offset 130924ns 逐 ns
  6969781703+130924==6969912627·V/A Continuous·无未声明回退]; L5.1/5.2/
  5.3 PASS[recover(A) handle=1 tap 重放成功]; Teardown PASS; **唯 L5.4
  FAIL: runway 新下界 >18s**[t0+15..18 仍推进·时间线闭合 q1=t0+15.0/
  q2=t0+18.0/recover(B) handle=2 重放成功 00:48:01.147/Teardown 00:48:
  04.149——锚定机制精确执行; 机制实锚=inter sink 在输入管线内 tee 挂接
  controller.rs:645-666·B Paused 冻结属实·余流=inter shm 积压·容量由
  inter 插件内部语义决定仓库代码不可见; 与固定大积压或"积压≈冻结前
  B 生产窗（本跑 ~25.5s）"两假设均相容; 候选待裁 ①grace 15→30s
  ②①+q1/q2 帧计数 print（观测性一行·推荐）③eventually-stalled-deadline
  语义升级]**; 14 项中 13 PASS 维持; 工件 converter×6 间歇/pad_unlink×4/
  MainContext×2 隔离队列]**; **第三十六轮（主账 §48+设计探针 §25
  跨账）: L5.4 终裁=方案③「有界 eventual-stall」正式批准——三阶段
  观测器冻结**[Phase A 输入故障确认（现有 5.3 不变）→Phase B 最小排空
  grace（t0 锚定保留·第三十六轮真机时间线闭合已证精确）→Phase C 连续
  N=L5_PROGRAM_STALL_CONFIRM_ROUNDS 窗无增长=StalledConfirmed·t0+
  L5_PROGRAM_STALL_DEADLINE 仍未确认=StillAdvancingAtDeadline=FAIL/
  TIMEOUT·帧计数簿记回退=ObservationInvalid——结束原因三词表进
  evidence 禁静默超时; ①grace 15→30s ❌ 禁盲调（>11/>18s 只是下界
  非定值·经验 tuning≠failure-domain verification）②grace+帧计数
  print ❌ 已不足; classify (true,true,true)→None 与 FailureDomain
  封闭四词表冻结·Bridge liveness 与 Program 推进证据模型分离维持·
  queue 水位维持 ❌; 裁决六项代码主张全实锚[现行锚定链 dual_input.rs:
  793-827/Program Graph 拓扑 switch_graph.rs:397·399-400·441·443-444
  （3ff66ad 后未变）/classify program_execution.rs:186-200/session
  teardown :782-798（efc1b2a 后未变）/bridge liveness 分层 :131-143/
  诊断注入 controller 第四 view（bb1360c 后未变）]; 执行=只改 gates/
  dual_input.rs·三常量[GRACE 15s 维持不调参·ROUNDS=3 配 SAMPLE_GAP=3
  ⇒ 9s 确认窗·DEADLINE=60s=验证期限非通过常数]+循环+evidence·判据
  表达式零变化·禁改九面维持[program_execution/diagnostic/controller/
  switch_graph/session/backend/program_timeline/Supervisor/SPI];
  后续序=修改→fmt→矩阵→bin rebuild→真机→核对 14/14→NewEpoch P1
  独立刀→Final Close→A2-8-05 archive; 隔离队列维持[pad_unlink×4/
  MainContext WARN/interlace 断言]]**; **执行（主账 §49+设计探针 §26）:
  d7d4fc6 单文件 +68/−16**[三常量 GRACE 15s 维持/ROUNDS=3/DEADLINE=60s+
  三词表 outcome enum+Phase C 循环+evidence·判据表达式零变化·禁改九面
  零触碰]; 盒矩阵 fmt/default 217/mock 381/bmd+gst 240/clippy×2 绿·bin
  release 596a8bcc·sha 80/81 唯 Cargo.lock 既有; **真机两跑: run1
  （05:38, 1f0ea619）8/10=L4 首次 NewEpoch FAIL**[视频 mapped 6970509011<
  last 6970509012 1ns 级连续性竞态 :618-622/:658-679 触发→epoch 1·
  DeclaredDiscontinuity·无 undeclared jump·L5 H1 级联·C 类回裁三问
  (Preserve 声明保证 mapped≥last?/L4 接受良构 NewEpoch?/P1 排期)];
  **run2（05:40, ba2f1783）9/10=L5.4 观测器首执行 StillAdvancingAt
  Deadline @t0+60.0**[L4 Preserve 第五次·15/15 窗全速 30fps·停滞从未
  发生·**排空假设被定量否定**（B 预冻结生产窗 ~27s≪60s·shm 积压秒级
  撑不住）→领先假设=活跃输入死后程序仍被另一活输入全速 feeding=隔离
  前提待裁·R34>11s/R35>18s runway 解释追溯否定·回裁四选（推荐①观测
  归因探针:L5.4 期间 program PTS 与 A/B 源 PTS 对齐）]; 14/14 未达·
  02-I 维持 FAIL-PENDING-CORRECTION[run2 13/14 唯 L5.4+L4 NewEpoch
  间歇 1/5 并列未决]·零后续改码·双证归档**]**; **第三十七轮后即时诊断
  （用户拍板"截图比对", 主账 §50+设计探针 §27, 零仓库代码）:
  intervideosrc 断粮自造帧=插件级实锤**[E1 跨进程占位帧 320×240 暴露
  合成+inter=进程内通道实证/E2 无写入器 1080p 24 帧全同 md5/E3 真流
  6s→断流 23+s 恒 md5 连续不停/E4 Gate 同款 Paused 25s 全程每帧全同
  →恢复即真帧]; **L5.4 前提"活跃输入死⇒program 停"在 inter 拓扑
  结构性不可满足=待裁三选[①去 inter 化②活性信号换面（bridge
  liveness 已证诚实·与"不合并"旧裁构成再裁）③语义重定义]+L4
  NewEpoch 1ns 竞态并列**; R34/R35 runway 解释终修为合成非排空·
  §49.3"A 喂出口"假设证伪; 证据=盒 ~/vbmfp-r36+本地三截图]**]**; **第三
  十七轮（主账 §51+设计探针 §28, 双段裁决）: L5.4 正式重定义「故障域
  归因完整性」+R36 观测器撤销**[第一段@6759443: ①归因探针最高优先/
  deadline 非严格有界发现[sleep 越界+stall 先于 deadline·随撤销 moot]/
  L4 Preserve-only 冻结❌Preserve∨NewEpoch/P1-A 根因确认=连续性基准用
  动态 last_program_pts"用未来观测值判当前边界"/P1-B 代码直证/依赖图
  无 ownership 冲突; 第二段@6400639 终裁: **选③重定义**——①去 inter
  现在不批准[inter=带 starvation fallback 语义的桥≠错误架构·未来开
  PROGRAM-BRIDGE-TRANSPORT-SEMANTICS]②bridge_liveness 与 program_
  progress 合并维持 ❌[三事实分层不变]③L5.4=B input 不推进∧B bridge
  死∧A input 推进∧A bridge 活∧Program 输出非权威证据⇒A 行=None∧
  B 行=Input⇒PASS; 真 Program 域故障归 A2-8-03[注入在 03 设计禁塞
  DiagnosticFaultInjection]; **删除 grace/deadline/eventual-stall
  全套**; d7d4fc6=R36 实验实现保留历史·R37=semantic correction];
  裁决五项代码主张全实锚[Phase C 循环序/ProgramObservation :57-73/
  on_program_pts :768 动态基准/L4 match 单臂 :685-686/intervideosrc
  官方 timeout 1s 黑帧]; 执行=只改 gates/dual_input.rs[撤销观测器+
  5.4 归因完整性重写+5.3 陈旧注释修正]·classify/四词表/L4 判据/
  5.1-5.3/Diagnostic 契约全冻结; 后续序=修改→fmt→矩阵→bin→真机→
  14/14[10/10 亦不触发 Final Close——两 P1 未闭·A2-8-05 暂缓];
  **P1-A[冻结 transition boundary 替代动态 last_program_pts]+P1-B
  [rebase offset 不变量]=下一独立刀 program_timeline.rs 不混 commit·
  修后 L4 真机目标稳定 Preserve]**]**; **执行（主账 §52+设计探针 §29）:
  0d59ddb 单文件 +37/−82**[观测器撤销 grep 零残留+5.4 归因完整性重写+
  5.3 注释修正·classify/四词表/L4 判据/5.1-5.3/Diagnostic 契约零触碰];
  盒矩阵 fmt/217/381/240/clippy×2 绿·bin release 6e02ba57·sha 80/81
  唯 Cargo.lock 既有; **真机 09-05 06:38（证据盒 0638-r37-l54-
  attribution, run.log sha c1c296a6, EXIT=0, ~46s）: 10/10 ALL PASS
  02-I 历史首次**——L4 Preserve 连续第六次[epoch 0·offset 174161ns·
  6970673376+174161==6970847537 逐 ns·V/A Continuous]; **L5.4 新语义
  首跑 PASS**[A行=None B行=Input·Program 输出 advancing=true[合成帧]
  如实记录为非权威证据 v 1053->1143 a 1405->1525]; Teardown PASS;
  **14/14 达成但 Final Close 不触发**[两 P1 未闭·C-TIMELINE Final
  Close 暂缓·A2-8-05 暂缓]; 下一刀=P1-A[冻结 transition boundary 替代
  动态 last_program_pts]+P1-B[rebase offset 不变量]独立修→L4 复跑
  目标稳定 Preserve[当前 6 跑 5P+1NE]; 工件 interlace×6/pad_unlink×4/
  MainContext×2 隔离队列]**]**; **第三十八轮（主账 §53+设计探针 §30,
  核验+测试增强·生产代码零改动）: P1-B 正式撤销核验成立+P1-A 批准但
  实现期偏离回裁**[P1-B: on_mapped_buffer :597-606 映射校验先证
  boundary.1−boundary.0==seg.offset ⇒ NewEpoch rebase :682-691 沿用
  offset 不变量自动成立——R33 登记表述修正"测试不足非代码缺陷";
  P1-A 真问题维持（:618-622 动态基准+on_program_pts :768）; **偏离
  发现: sample_switch_anchors :852-867 双锚各加独立测量节拍 ⇒ 四跑
  program_start−mapped≡33,333,333ns 恒一帧 ⇒ 字面谓词 mapped≥
  program_start 将使一切健康 Preserve 跑翻 NewEpoch; ±1ns 竞态根源=
  双节拍测量差非 last_program_pts 推进; 方案 α=锚去节拍[switch_graph
  ·需扩授权·推荐·mapped==boundary 精确相等世界]/β=Domain 冻结基准+
  slack 魔数——待裁**; 交付=timeline_rt_01_new_epoch_rebase_offset_
  invariant[双平面 offset==program_start−source_start+NewEpoch 平面
  DiscontinuityDeclared 不洗·offset=i64 跨域可负·mock 车道 cfg]; 矩阵
  default 217 不变/mock 382/bmd+gst 240 不变/clippy×3; 真机无
  生产变更不复跑]**]**; **第三十九轮（主账 §54+设计探针 §31, 裁决
  落账零代码）: P1-A=方案 α「边界帧锚修正」批准·β 否决**[α:
  sample_switch_anchors 双锚去 saturating_add(节拍)——program_anchor=
  pv·source_anchor=target_v 原值（audio 同构）; 修改面=switch_graph.rs
  单函数+回归测试+注释+program_timeline.rs AnchorPair 注释语义统一
  （注释级）; 健康切换 mapped==program_start 精确相等+±1ns 竞态结构性
  根除; β=slack 魔数吸收非消除+吞真实 discontinuity 否决; P1-A 重定义
  =「已观测边界帧做未来一节拍外推致 program_start 与首枚 target buffer
  不在同一离散帧边界」; P1-B 维持撤销+不变量测试保留; last_delta 解耦=
  observation fact（allow(dead_code) 保留禁删）≠ declaration input;
  R39 回归项 1-4 被 α 取代（Domain 零改）/项 5 已交付/项 6-7 既有
  覆盖; 新回归锁=last_delta 不得改变声明 anchor（裁决例值）; 披露=
  switch_mock.rs:297-306 同 +STEP 外推——Mock 同构面同步与否独立待裁
  不阻塞; on_mapped_buffer/close_transition/declare/Runtime 编排链
  零改; 真机七项验收重点见主账 §54.2]**]**; **同轮实现+真机（5d61b97,
  主账 §55+设计探针 §32）: α 落地——锚=原值+rt_03 回归锁+AnchorPair
  注释统一; 矩阵 default 217/mock 382/bmd+gst 241(+1)/clippy×3/sha
  80/80/bin 83b9b695; 真机 ×2 EXIT=0 10/10——L4 Preserve epoch 0·
  **mapped==program_start 首次精确相等**（run1 6,973,081,228/offset
  14,415ns·run2 6,969,530,558/offset 53,969ns·历跑恒差一帧消失）·
  V/A Continuous·±1ns 条件性 NewEpoch 根除实证（5P+1NE→双 P）·L5.4
  归因 PASS·Teardown PASS; 证据 ~/a2-8-02i-evidence/2026-09-05-r40-
  anchor-fix/{run.log a67ef58a·run2.log be80906f}; **P1-A CLOSED**;
  02-I=10/10 第二次; Final Close/A2-8-05 待裁]**]**; **第四十一轮
  （主账 §56+设计探针 §33, 终裁落账零代码）: R40 复核 PASS·
  **C-TIMELINE-01/P1-A Final Close=APPROVED/CLOSED**[措辞限定: 专项
  Close≠A2-8 CLOSED——P1-B=REVOKED/CLOSED-AS-NON-ISSUE 不变量保留·
  Evidence/Hardware=PASS×2·R1-R8 未触碰·偏离回裁链闭合]; A2-8 总体
  OPEN·顺序 03→04→05 维持·A2-8-05 仅准备不可收口; **Mock +STEP 分叉
  正式立项 MOCK-ANCHOR-SEMANTIC-ALIGNMENT**[switch_mock.rs:299-306 vs
  观测原值 adapter·不回溯不阻塞·修复待独立裁决]; 下一刀=A2-8-03
  failure/supervision[探针先行·开工前须 SoT Probe/裁决授权·本轮
  未启动]**
- [ ] 5. A2-8-03 failure/supervision 验证: watchdog 四视角观测穿
  RuntimeEvent→Custody 无跨设备污染 + Supervisor 边界（recovery only）
  `Contract: 02` | `Implementation: 待` | `Verification: 待` | `Gate: 待`
  **[第四十二轮（主账 §57, 裁决落账零代码）: A2-8-03 批准开工·第一步
  =SoT Probe 仅探针零代码——不重造 watchdog/liveness/FailureDomain
  （代码现实已在册: watchdog 三件套+Bridge liveness+progress_since+
  FailureDomain+SupervisorAction 封闭）; 硬红线=Supervisor 禁 switch()/
  begin_switch(); 核验扩面发现=契约注释漂移两处[contracts/switch.rs:
  91-92+switch_mock.rs:271-273 同源"位置+步长"]正式登记 CONTRACT-
  ANCHOR-DOC-SYNC 下一次文档/契约同步轮处理禁留归档]**;
  **同轮 Probe 交付（A2-8-03-00 SoT Probe=
  2026-09-05-a2-8-03-00-failure-supervision-sot-probe.md 十二问全锚+
  红线核验[Supervisor 禁 switch/begin_switch 零存在·四层证据]+缺口
  G-1 事件→Custody 生产链未闭合[custody.rs:119-123 零生产调用者]/
  G-2 分类器三列观测 gate-only 无 runtime 常驻消费/G-3 Program 域故障
  无恢复 lifecycle[dual_input.rs:821-822 显式预留·受 Freeze'Recover
  语义冻结'约束]/G-4 Mock recover 契约面; 不新造清单在册; 下一刀=
  03-01 待用户对 Probe 裁决后授权]**
  **[第四十三轮（主账 §58, 裁决落账零代码）: R42=PASS 收紧表述[Probe/
  ledger round·no runtime implementation authorized or introduced];
  **实施序正式冻结=G-1→G-2→Failure Attribution→Recovery Contract→
  G-3→G-4 依赖 DAG 禁并列开工**; 授权 03-01 第一阶段=G-1 Identity/
  Custody+G-2 Runtime Consumption 设计/实现探针（已交付
  2026-09-05-a2-8-03-01-g1-g2-custody-consumption-design-probe.md:
  internal 平面多消费者竞争 drain 新事实[watchdog.rs:192/:537 共享
  world.internal_log 破坏性 drain——单一 drain 点假设不成立]+身份
  丢失机制根因[ingest→mapper 边界归零]+custody 双零生产调用+组
  watchdog 无 MediaTapPort 依赖+OQ-G1-1..7/OQ-G2-1..6 十三问待裁];
  G-3 暂不授权; CONTRACT-ANCHOR-DOC-SYNC+Mock A/B 合并同一同步轮一次
  统一[用户倾向 B 非现在改·禁半同步中间态]; 五误区禁令落账[禁
  'Supervisor 已有⇒03 完成'等五句]; Mimosa 后置维持; 本轮零代码零
  矩阵——R40 runtime 证据继续 baseline; 本项保持未勾]**
  **[第四十四轮（主账 §59+03-01 探针 §8/§9, 裁决+实现落地）: R43=PASS
  [Probe/架构裁决轮]; R44 裁定=P0 两问直接裁[OQ-G1-1 身份语义=形式化
  设备身份承载·字段名不动·类型级修正留 V0.3/OQ-G1-2 拓扑=禁 custody
  第三 drain·「单一事实消费点+非破坏性 fan-out」新解]; 实施序收紧
  03-01-A..G→03-02→03-03→03-04; 授权=A/B/C+矩阵+binary gate; 新红线=
  [G-2 禁改 ProgramExecutionRuntime 切换逻辑/归因禁放宽 nil→NO
  ATTRIBUTION/EventLog 契约 FIFO·两级丢弃·计数·fail-closed 禁绕开/
  watchdog 重接线为周期驱动器]; **A/B/C 已落地 9 文件**: A=Supervisor.
  ingest 签名扩展携带设备身份[watchdog.rs:177 生产唯一调用点]+mapper
  携身份入口 map_upstream_for_device[词面零变化·trait 面 nil=未归属
  维持]/B=event_intake.rs InternalEventIntake 唯一生产 drain 实现
  [生产 internal 平面 drain 全仓普查仅一处·生产 watchdog 不再持
  internal log 类型级排他·bootstrap 共享单实例 BS-01]/C=consume 边界内
  observations_from_events 全量恰一次累积[A2-7 桥规则原样·零新增
  消费者·零 advance·快照调用点不加=OQ-G1-5 留 D/E/F]; +6 测试全绿;
  盒矩阵 fmt/default 223[217+6]/mock 388[382+6]/bmd+gst 247[241+6]/
  clippy×2/bin 全绿·盒源 9/9 sha8==本地; 行为变化披露两处[生产故障
  事件 nil→真实设备身份·fault_trigger 收敛为只触归属设备(nil 保守
  匹配维持)/watchdog 本地 fold 分区语义不变]; custody「双零生产调用」
  闭合其一, 归因/快照生产消费仍零[03-01-D/E/F 待授权]; 本项保持未勾
  ——A/B/C≠03-01 完成]**
  **[第四十五轮（主账 §60, 03-01 探针 §10, R44 复核裁决+G-2 stage-1 实现）:
  R44=PASS[A/B/C 实现轮]; 两措辞降级采信[①A=运行时身份修复≠类型语义锁死
  (PipelineFault.pipeline 双语义留 V0.3) ②B="类型级排他"→"组合根接线级唯一
  drain ownership"[internal_log 仍 pub, 注释已纠偏, 强类型封锁留治理轮]];
  **E 前提纠偏: FailureDomain 并非不存在**[program_execution.rs:179
  {None,Input,Bridge,Program}+classify_failure_domain 三列进度观测既有,
  消费=dual_input L5d gate-only=恰 03-00 G-2 缺口原文; §8.10 消费面
  master_join/api_boundary 预留]——依用户红线复用现有 contract 生产化,
  禁新造第二同名类型[SharedPipeline scope 与 FailureDomain 两族证据禁融合];
  授权 R45 全序 G-2-00→D→E→F→G 已执行: G-2-00 预检[report_failure 生产
  调用者恰 2/桥 liveness=bundle 第三 view 现成/bin composition 扩 4 元透传];
  D=assemble_decision_input 纯函数装配[attribute_failures 首个生产调用者,
  ingest/group 两 tick 同临界区; 空 custody→None=absence≠evidence];
  E=组 tick 三列生产喂入[fold advancing+桥 liveness view(窗口 3000 与 gate
  同值同义常量)+program_progress_since 两采样; 三列齐备才分类, 缺席不分类
  ≠gate L5d 缺席→false 口径差异披露]; F=report_failure(+domain,+attributed)
  决策输入面[Status 逐决策替换记录+只读访问器; **决策判定逻辑零变化**
  (无分支消费, 域→恢复策略选择=03-02); Custody→Supervisor→switch 禁式
  不可构造维持; 四红线全守]; 测试+5[证据记录/替换/读取 fail-closed/装配
  规则三列齐备缺席不分类空 custody 不归因]; 盒矩阵 fmt/default 224/mck
  390/bmd+gst 248/clippy×2/盒源 7/7 sha8 全绿[hw 门控闭包作用域 bug 盒上
  抓到修复=default/mock 不编译该段的分层实证]; **真机双 gate: session_
  lifecycle ALL PASS EXIT=0[ingest tick D/F 接线活体]+dual_input ALL PASS
  10/10 EXIT=0 零回归[L4 Preserved epoch0·L5d 归因完整; 基线校准: R36/R37
  已闭环 L5.4, R40 起 10/10]**; 证据盒 2026-09-05-r45-g2-decision-input/;
  披露: 组 watchdog tick 生产活体证据缺[唯一 spawn=bin:479, 编译级+分类器
  同源 L5d 真机复核, 活体留 A2-8-04 bin 轮]·G-2 PASS 不自宣待用户复核;
  本项保持未勾——D/E/F 落地≠03-01 完成[G-2 终审+组 tick 活体+03-02 待裁]]**
  **[第四十六轮（主账 §61, 03-01 探针 §11+03-02 新探针, R45 复核裁决
  G-2 Stage-1 PASS/G-2 Final OPEN + G-2-G 真机活体 + 03-02 设计提案零
  实现）: 开发线纪律=comet/a2-8-dual-input-switch@ff864d2 恒定
  [master=7745968 旧头禁混]; 单故障优先序分类器语义锁死重申; group
  custody batch group-wide+逐 action device-scoped attribution 双防线
  边界 03-02 沿用; **组 watchdog 真机活体已获得**: 活体观测行使能披露
  [健康路径原静默→两处仅诊断输出零决策逻辑观测行·矩阵 224/390/248 计数
  零变化]+生产 bin 双输入诊断会话 9.5min[fail-soft 纯分析零外推·bin
  ab361801·v5 manifest]——**线程连续 tick 0→1120（57 行）+双设备三列
  实时全健康+分类器真机活体 tick0=None→tick≥20=Some(None)**[证据盒
  2026-09-05-r46-g2g-group-watchdog-live]; 仍缺如实记档: 故障动作路径
  决策输入活体指纹=0[窗口零自然故障·ball 源勿杀·生产注入面 gate-only
  红线]——OQ-R4 待裁[证据组合关闭 G-2-G vs 自然故障长窗复跑]; 03-02
  Recovery Contract 设计冻结提案已交付[五面 F-1..F-6+OQ-R1..R5 待裁;
  提案默认=不新造 Strategy 词表+消费点=执行域读 last_decision_*+
  Supervisor 判定/词表零变化+OQ-R1 全维持现状零代码收口候选]; 本项
  保持未勾——G-2 Final/03-02 冻结/A2-8-04 专项均待裁待做]**
  **[第四十七轮（主账 §62, 03-01 探针 §12, R46 复核裁决+OQ-R4 关闭+
  G-2 Final CLOSE+03-02 命名纠偏; 零运行时代码零矩阵）: R46=PASS
  G-2-G LIVE EVIDENCE[12 行复核: 1-6/8-9/11-12 PASS·7 有条件[LIVE
  PASS/E2E 未触发]·10=DESIGN DELIVERED/FREEZE NOT YET COMPLETE];
  **OQ-R4=组合证据关闭**[Layer1 生产线程活体+Layer2 生产线程分类器活体
  +Layer3 gate L5d 真机注入分类; 长窗复跑=概率性证据非确定性软件证据;
  **Gate 分层记账: G-2-G-LIVE=PASS·G-2-G-CLASSIFY=PASS·G-2-G-FAULT=NOT
  OBSERVED 不阻塞·G-2-G-E2E=属 03-02/A2-8-04·LIVE 与 E2E 禁混**];
  **G-2 Final=CLOSED（R47）**·03-01-A..G 全 COMPLETE·03-01-G 收口注记
  [观测行=诊断零决策语义·E2E 触发证据随 03-02/A2-8-04 补齐]; **03-02
  命名纠偏: 设计探针/冻结提案[Design Probe / Freeze Proposal]·DESIGN
  DELIVERED/FREEZE NOT YET COMPLETE·禁称已冻结 Contract·文档标题/状态
  行已就地修正+§7 修正记录**; hygiene defer: "五面"→六面[F-1..F-6]于
  03-02 冻结时顺手修不单独提交; **纪律重申: 禁先写 Recovery 代码——先
  OQ-R1..R5 用户裁决→Contract Freeze→最小实现→matrix+真机→A2-8-04**;
  本项保持未勾——03-02/03-03[G-3]/03-04[G-4] 未做]**
- [ ] 6. A2-8-04 Program Timeline / AV continuity 验证（第三轮终裁更名）:
  六路 PTS before/after switch 无 rollback/discontinuity/divergence/
  starvation; Program Timeline Continuity / Timestamp Normalization 方案
  裁决与验证（observation only，无 Engine——方案设计裁决属 02/04）
  `Contract: 03` | `Implementation: 待` | `Verification: 待` | `Gate: 待`
- [ ] 7. A2-8-05 archive+CI+merge（A2-8 收口唯一入口; 01-04 任一完成不宣布
  CLOSED）
  `Contract: 04` | `Implementation: 待` | `Verification: CI+归档` | `Gate: 待`
