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
  链; 盒上入口 smoke=真实 discovery→形态拒绝实证, probe §23））; 02-I
  真机 Gate 待用户双 SDI 窗口` | 
  `Verification: mock 356·bmd+gstreamer 226（含真实跨管线桥接/多切换
  一致性/G/H 三列证据/liveness 降级锁死/persistent-id=0 拒绝/碰撞防线
  测试）·clippy 双组合 -D warnings clean·fmt clean; resolver gate 真机
  复跑双工卡碰撞告警×2 落盘; 真机五层矩阵=02-I 执行（VBMF_A2_8_DUAL_
  INPUT+现场 v4 双 Input port 声明 manifest）` | `Gate: 02-A..02-H
  全 CLOSED（probe §19-20）·02-I OPEN[代码前置（十六轮三刀+十七轮两刀）
  +acceptance automation（十八轮正式 Gate 入口）全在仓, probe §21-23;
  硬件形态边界=两块独立单输入卡（collision closure/derive_claims/
  serial binding/audio 独立性=独立 change 不混入, 十八轮 §十二/§十三）]
  ——**唯一阻塞=用户侧双 SDI 信号源+两卡可占用窗口**`
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
