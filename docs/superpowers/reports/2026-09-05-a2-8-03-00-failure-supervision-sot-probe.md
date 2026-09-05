# A2-8-03-00 Failure / Supervision Boundary SoT Probe（2026-09-05）

- **授权**: 第四十二轮终裁 ①（主账 §57.2）——批准 A2-8-03 开工, 第一步
  必须 SoT Probe, **仅探针零代码**。十二问照录裁决 §十二; 本探针逐问
  实锚作答, 全部基于 `3eff2e8` 真实仓库代码。
- **定位（裁决 §十一/§十五）**: 03 = 把已有 02/G/H/I supervision observation
  能力收敛成可恢复、可验证的 supervision contract——**不是**重造
  watchdog/liveness/FailureDomain（代码现实已在册）。
- **裁面链**: Observation → RuntimeEvent → Custody → Supervisor →
  Recovery 是否形成**唯一、无旁路、无重复归因**闭环。
- **硬红线（裁决 §十）**: Supervisor 禁 `switch()` /
  `ExecutionGroup.begin_switch()`——违反即触 A2-8 冻结「Supervisor ≠
  switch executor」。

## §1 十二问实锚作答

### Q1 当前 watchdog / liveness 的全部实际实现在哪里？

**watchdog.rs（767 行, 三件套）**:

1. `spawn_ingest_watchdog`（:34-283, 单管线, `bmd+gstreamer` hw 门控）:
   500ms tick; `ctrl.observe`（bus Error/Eos/StateChanged/Warning/
   ClockLost）→ HEALTH_ARCS 上就地折叠 acceptance a1-a4/b1-b4/c1-c4
   （:63-167, 不覆写 appsink 回调计数）; bus 策略映射（Error/Eos 计入
   c 项, ClockLost=degraded 不重启 :131-141, Warning 记录不重启）;
   bus Error→`Supervisor.ingest`（mapper 归一化 :174-181）; a4 信号
   闩锁→`SignalVerified`（:184-190）; drain internal→`HealthFold
   ::reduce`→写回 agent_state（:192-199, P0-7D-1.3）; 故障→
   `report_failure`→Restart→lease 重校（:215-224 排他不变量）→backoff
   →`begin_restart`→`ctrl.recover`→`report_recovered`（:202-242）。
2. `execution_group_observe_fold`（:374-452, **纯函数零 IO**, mock 车道
   ×5 测试 :644-766）: 四观测面 Input A/B+Switch+Program;
   `GroupAction` 封闭词表**仅** `ReportInputFailure{device_id, reason}`
   （:317-322, T10/T12 类型级反证——无切换变体）;
   `InputFailureReason`={HealthAbsent, CountersFrozen, PipelineError}
   （:324-332, 互斥优先级 缺席>冻结>错误, 恰好归因本设备）;
   `InputHealthFold`/`SwitchFold{av_paired, consistent}`/`program_alive`
   （:336-440）。
3. `spawn_execution_group_watchdog`（:463-581, hw 门控薄壳）: 单实例服务
   整组（禁 for 循环多 spawn）; 停止旗由 `ProgramExecutionRuntime
   ::teardown` 置位（:476-478/:486-489, A2-8-02-E——线程随 program
   生命周期退出）; `switcher.observe(&graph).program`→fold（:494-517）;
   每设备 SignalVerified 闩锁（:518-529）; **Observed 确认→
   `complete_switch`（Desired 落定, 不发起切换, :530-535）**; 故障
   action→Supervisor→recover **仅故障输入自身 handle**（:540-577,
   归因恰好该设备——跨设备污染不可构造）。

**Bridge liveness（G/H-1, 第十五轮交付）**: `contracts/media_tap.rs`
`BridgeChannelLiveness{frames, last_observed, alive_in_window}` +
`MediaTapPort::bridge_liveness(handle, window)`; controller.rs
`BridgeStat`+`bridge_clock_origin`（观察时钟与 PTS 严格分离）;
`program_execution.rs assemble_bridge_health`（:134-154, degraded=
pipeline_recovered ∧ 期望 channel 无实测流通）。

**推进性判定**: `program_progress_since`/`input_progress_since`
（program_execution.rs:160-174——帧计数增量, "曾经活过≠当前推进"）。

**装配点**: bin/media-agent.rs:106（ingest）/:479（group）/:529（ingest）;
gates/session_lifecycle.rs:166（ingest）。

### Q2 RuntimeEvent 的产生点有哪些？

词表 = events.rs:50-107（14 变体; kind :109-128; is_fault/severity
:130-146）。生产发射点（按发射者）:

- **SessionManager**（session.rs, 经组合根注入的 RuntimeEventSink——
  FanoutSink 双日志 projection+internal 单表单锁）: SessionCreated :482·
  LeaseGranted :510·IdentityResolved :551·SourceMaterialized :632·
  ResourceAllocated :697·ResourceReservationExpired :1009·
  SessionStateChanged :748/:833/:1090·SessionFailed
  :431/:600/:623/:658/:713/:730/:1030/:1080。
- **Supervisor**（supervisor.rs）: PipelineFault 回声（:210, summary=
  RESTART_ECHO_SUMMARY :34）·HealthChanged :217/:250/:274;
  `ingest`→`DefaultRuntimeEventMapper`（events.rs:164-189）产
  PipelineFault(nil)/HardwareFault(nil)/AmbiguousIdentity(nil)——
  **mapper 产物恒 `Uuid::nil()` 归属**（无身份证据）。
- **Watchdogs**: SignalVerified（watchdog.rs:186/:523）·HealthChanged
  （:219/:547）。
- **消费面**: health.rs `HealthFold::reduce`（:123-149→AgentState 派生）·
  supervisor `fault_trigger_from_events`（:45-58, 事件驱动故障触发谓词）·
  custody `observations_from_events`（:136-158）·event_projection.rs
  （API 投影）。

### Q3 RuntimeEvent → Custody 的唯一 ownership 是什么？

- **唯一桥** = custody.rs `observations_from_events`（:136-158）:
  只提取 `PipelineFault{pipeline≠nil ∧ summary≠echo}`; HardwareFault/
  SessionFailed/HealthChanged/ClockLost 不提取（等 attribution
  contract）; avsync 恒 Unknown。
- Custody 本体 = 纯函数（`attribute_failures` :98-113 identity
  correlation 三重联合匹配; `custody_snapshot` :184-205）——不订阅不持
  Runtime 引用, 消费点自行 drain 后调用; 七不红线（:11-13: 不猜测完成/
  不创建 Runtime Health/不改 Supervisor/不读 GStreamer/不改 Plan/不执行
  recovery/不产 metadata truth）。
- **关键现状（:119-123 注释原文）**: 「本桥已实现但**尚无生产调用者**——
  真实生产故障链现状为 mapper 产 PipelineFault(nil)（本桥拒收）+
  Supervisor echo（本桥再拒收）, 即真实故障尚未经本桥进入 Custody」。
  生产闭合（真实 drain→桥→custody 周期）deferred（A2-7-04 只做了 mock
  lifecycle 验证 custody_08/09）。
- 已登记债: `PipelineFault.pipeline` 当前承载 **device identity**
  （legacy misnamed; 同 enum 内 SourceMaterialized.pipeline 是 Pipeline
  identity——同名双语义, custody.rs:61-68）= V0.3 Event Contract 债。

⇒ **缺口 G-1（03 核心）**: Observation→RuntimeEvent→Custody 生产链未
闭合——真实故障不进 Custody 快照; `custody_snapshot` 无生产调用者
（当前唯一合法 ProgramMaster=join_result:None）。

### Q4 FailureDomain 当前在哪里计算？

- 定义+分类器: program_execution.rs:179-200
  `classify_failure_domain(input_advancing, bridge_alive,
  program_advancing)`, 单故障假设优先序 Input>Bridge>Program（:176-177,
  多重并发故障如实报首因）。
- **生产调用 = 唯 gates/dual_input.rs:827-832（L5.4 两行 A/B）**;
  switch_graph.rs:1956-1969（测试）; api_boundary.rs:406（注释——A4
  控制面计划消费点 §8.10）; master_join.rs:24（注释）。
- 同族观测装配器同样 gate-only: `assemble_timeline_sample` 生产调用
  gates/dual_input.rs:596; `assemble_bridge_health` 生产调用
  gates/dual_input.rs:758。

⇒ **缺口 G-2**: supervision 观测/分类面（classify/TimelineSample/
BridgeHealthReport）**无 runtime 常驻消费链**——仅真机 gate 消费。

### Q5 Supervisor 当前消费什么？

- `report_failure(device_id)`——单管线 watchdog（watchdog.rs:212）与组
  watchdog（:543）的故障上报; 决策句柄 = device_id（register/
  report_failure/begin_restart/report_recovered/backoff/escalate,
  supervisor.rs:149-280）。
- `fault_trigger_from_events(&drained, device)`（watchdog.rs:197）——
  事件驱动故障输入谓词（自回声排除/归属判定/平面分离, supervisor.rs
  :45-58）。
- `ingest(source, observation)`（watchdog.rs:176）——上游 vendor 观测
  归一化。
- **不消费** ProgramObservation/TimelineSample/FailureDomain/
  BridgeHealthReport——Supervisor 决策面以设备为单位, 无 Program 域
  视角（Program 域观测现归 watchdog fold 的 program_alive 事实位 +
  gate）。

### Q6 Supervisor 是否已经存在 recovery-only 边界？

**存在, 四层**:

1. **类型级**: `SupervisorAction={Restart, Escalate}` 封闭词表
   （supervisor.rs:119-125）——无任何切换变体。
2. **模块硬边界**（:1-19）: "ONLY *decides* a recovery strategy…
   never touches GStreamer/FFmpeg/DeckLink"; 决策经 SupervisorAction
   交 Controller 执行。
3. **上游词表级**: `GroupAction` 无切换变体（watchdog.rs:314-322）+
   组 watchdog 注释「切换永不在此发生（T10）」。
4. **Gate 证据行**: dual_input.rs:862-864（「Supervisor 角色: recovery
   decision 非 switch executor——切换经 SwitchExecutionAdapter 直驱,
   Supervisor 未持有 switch 面（wiring 事实）」）。

全仓 supervisor API 面（register/status/report_failure/begin_restart/
report_recovered/backoff/escalate/ingest）**零** switch/begin_switch
入口（supervisor.rs 全读 + grep 证实）。
`complete_switch` 唯一生产调用 = 组 watchdog 观测确认（watchdog.rs:
530-535, ExecutionGroup Desired 落定语义, 非 Supervisor 面）。
✔ 硬红线现状成立。

### Q7 recover() 当前真正能恢复什么？

`MediaBackend::recover` → `GStreamerPipelineController::recover`
（controller.rs:217-299）——**输入采集管线级**:

- plan 取自 instances 登记（:220-227; 未注册即 fail-closed——R33 证
  stop→recover 结构性必败的根因面）;
- tap 簿记捕获（:230-236）→ 旧实例 stop+remove（:237-239）→ **同
  handle 同 plan** 重建管线 + set Playing（:241-259）→ tap 重放
  （:260-291）。
- **不能恢复**: Program graph（无 recover 面——只有 stop_program/新
  Runtime 重建）; switch graph/桥拓扑（无 recover 面）; Session（stop
  =终态 P0-2 :314-331 instances+HEALTH_ARCS 双注销）; 设备级硬件故障
  （HardwareFault→Escalate 人工）。
- 诊断注入（R34）: `DiagnosticFaultInjection::inject_runtime_stall`
  （controller.rs:349-360, set_state(Paused) 停流·登记保持）——注入
  "运行故障"非生命周期终止, 随后 recover 即生产行为; 红线 :346（不
  注销 handle/不合成 Bus Error）。
- **Mock 面**: MockBackend stop/recover 均 no-op Ok（session.rs:1595-
  1605 测试桩）——recover 契约真实语义仅真机 gate 覆盖（R33 结论
  「Mock≠GStreamer 在 recover 契约面」维持）。

### Q8 recover 后 MediaTap 是否重挂？

**是**——controller.rs:260-291 attachment replay: `saved_taps` 簿记
（:230-236 捕获）→ 新管线上 `attach_tap_to_instance` 逐条重放; 失败
不阻断 recover 本体（管线已恢复, tap 降级待重挂, warn :282-287）。
真机证据: L5.2 recover→`bridge_liveness` 真实复流 PASS ×3（R35/R37/
R40）。

### Q9 ProgramExecutionRuntime 是否需要参与 recovery？

- **现状零参与**: Runtime API 面 = create（:234+）/ teardown（:335-357）/
  switch_program（:407）/ observe_execution（:550）/ is_active/
  set_watchdog_stop（:386-390, 停观测线程非恢复机制）。输入级恢复不经
  Runtime; **Program graph 故障无任何恢复路径**（只能 teardown→新
  session 重建）。
- L5.4 明注预留: dual_input.rs:821-822「真正的 Program 域故障验证归
  A2-8-03 专项」。
- 边界约束: C-TIMELINE-01 Freeze——Recover Soft/Hard **语义冻结本轮
  不实现**, Supervisor 只决定 recover 不拥有 Timeline。⇒ 03 若裁
  Program 域恢复, 只能裁 **supervision contract**（谁观测/谁归因/谁
  决定/走哪级既有 lifecycle）, 不得实现 Timeline recover 语义。

⇒ **缺口 G-3**: Program 域故障（graph 停滞/死）无观测→归因→恢复
lifecycle——02 L5.4 显式 deferred 项。

### Q10 SessionStopHook 是否可能与 recovery 形成竞态？

- 结构: session.rs `stop()`（:758-841）——double-stop 防护（:762-769,
  Released/Stopping→InvalidTransition）→ phase Stopping → **hook 先行**
  （:787-798; `ProgramExecutionRuntime::on_session_stopping`
  program_execution.rs:619-626, session-scoped→teardown: 停止旗→
  Program Stop→Tap Detach）→ backend.stop(inputs) 逆序（:804-809, P0-2
  失败只记录不截断释放链）→ 资源释放 → hook 条目移除（:840）。
- 竞态面（组 watchdog in-flight 恢复 × session.stop 并发）: 停止旗只在
  loop 头检查（watchdog.rs:486）, backoff sleep/`ctrl.recover` 在途时
  stop 可并发。结局两分支: recover 在 backend.stop **后**执行→instances
  已注销→fail-closed "未知 pipeline handle"（controller.rs:222-226,
  error 日志, 无状态破坏）; recover 在 stop **前**完成→重建实例被
  backend.stop 正常注销。**无状态破坏路径; 一切失败方向=fail-closed**。
- 顺序组合面: stop→recover 已证结构性必败（R33 L5.2 根因）; R34 诊断
  注入已绕开（gate 面）。
- 残余: recover 重建的新 bus channel/GLib 线程与 stop 交错的理论窗口
  表现为 error 日志——03-01 候选加固项（登记, 非缺陷）。

### Q11 A/B failure 是否已经有真实 Gate？

**有**——gates/dual_input.rs L5（:708-864, 真机 env
`VBMF_A2_8_DUAL_INPUT`）:

- L5.1 A-fail→B-alive（:729-749: !inputA_advancing ∧ bridgeB_alive ∧
  program_advancing）;
- L5.2 recover→桥真实复流（:751-782: `ctrl.recover`+`bridge_liveness`
  +`assemble_bridge_health` !degraded——观察恢复非簿记重放）;
- L5.3 B-fail→A-alive（:784-795: !bridgeB_alive ∧ bridgeA_alive）;
- L5.4 归因完整性（:797-847: row_a=None ∧ row_b=Input; Program 输出=
  非权威证据[inter 合成帧语义]）;
- 注入 = `inject_runtime_stall`（:719-726）; H1 fail-stop 链; 真机
  PASS ×3（R35 9/10 历史首通→R37/R40 10/10）。

### Q12 Program failure 是否仍可能误归因 Input？

- 分类器语义: `!input_advancing → Input` 恒先判（program_execution.rs:
  191-192）——单故障假设下 input 故障+program 故障并发只报 Input
  （:176-177 documented「如实报首因不做多维归因」）。**方向=漏报
  Program（首因遮蔽）, 非把 Program 故障错报为 Input**（无故障 input
  时 Program 停滞正确落 Program 域 :195-196）。
- L5.4 已重定义（R37/R38）: inter 拓扑下 Program 输出非权威
  （intervideosrc 断粮合成黑帧 advancing 恒真）——**输入故障不伪装
  Program 健康**; 反向（Program 真域故障检测）= 双输入健康+bridge 活+
  program 停滞 → classify 落 Program 域——**无真机 gate 覆盖**
  （:821-822 明注 deferred 03）。
- 残余: ①Program 域故障真机验证缺（G-3）; ②分类器无 runtime 常驻
  调用（G-2）——runtime 若不调 classify, "误归因"无从发生也无从服务;
  ③单故障首因遮蔽=设计语义（并发多故障归因显式 out of scope）。

## §2 红线核验（裁决 §十）

- Supervisor→`switch()`/`begin_switch()`: **零存在**（Q6 四层证据）。
- 切换唯一链 = Intent→`ProgramExecutionRuntime::switch_program`（①-⑩
  timeline orchestration, program_execution.rs:392-406）→
  ExecutionGroup/Adapter——Supervisor/GroupAction 词表类型级不可表达
  切换。
- 正确方向已在位: Observation→failure classification（fold/classify）→
  RuntimeEvent/Custody（桥在册未接线）→Supervisor（决策）→Recovery
  intent→既有 lifecycle（watchdog recover / 人工 Escalate）。

## §3 03-01 建议裁面（缺口清单——待用户裁决, 本探针不代选）

| # | 缺口 | 实锚 | 性质 |
|---|------|------|------|
| G-1 | 事件→Custody 生产链未闭合（真实故障不进 Custody; custody_snapshot 无生产调用者） | custody.rs:119-123 | A2-7 债务「生产链」项——03 须裁装配点+身份语义（PipelineFault.pipeline=device legacy 双语义先裁） |
| G-2 | 分类器/三列观测/桥健康无 runtime 常驻消费（gate-only） | program_execution.rs:179-200; dual_input.rs:596/:758/:827 | supervision contract 收敛点 |
| G-3 | Program 域故障无观测→归因→恢复 lifecycle（02 L5.4 显式预留） | dual_input.rs:821-822; program_execution.rs API 面无 recover | 03 专项核心; 受 C-TIMELINE Freeze「Recover 语义冻结不实现」约束 |
| G-4 | Mock stop/recover no-op——supervision 链 mock 验证受限（recover 契约面 Mock≠GStreamer） | session.rs:1595-1605 | 03 验证面设计输入 |
| 登记项 | SessionStopHook×recovery 理论交错窗口（fail-closed 方向, 无破坏路径） | §1 Q10 | 03-01 候选加固, 非缺陷 |

**不新造清单（裁决 §八）**: Input/Bridge/Program watchdog、FailureDomain、
liveness、progress 判定、Supervisor 决策引擎、recover+tap 重放——全部
在册, 03 只做收敛/接线/验证, 不重复实现。

## §4 探针边界披露

- 本探针零代码、零矩阵（无生产变更）; 全部行号锚于 `3eff2e8`。
- 未覆盖（非授权面）: PORT-IDENTITY-AND-RESOURCE-ADDRESSING·canonical
  UUID·C1-P1·audio capability 独立性·serial production binding·
  converter interlace·Cargo.lock 分叉——隔离队列不动。
- CONTRACT-ANCHOR-DOC-SYNC（契约注释漂移两处）另册登记（主账 §57.2 ④）,
  非 03-00 裁面。
