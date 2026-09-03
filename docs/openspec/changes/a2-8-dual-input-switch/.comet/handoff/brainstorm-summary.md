# Brainstorm Summary

- Change: a2-8-dual-input-switch
- Date: 2026-09-03

## 确认的技术方案

用户两轮终裁确认（第一轮 OQ-1..5 批准；第二轮修正=Pre-Implementation
Gate 十项冻结，全部落 probe §7 / design doc §2-§5）：

- A2-8-00 SOT Probe = 唯一探针刀（CLOSED @c3d3e23，零代码）；00 与 01
  不混为一个 change 节点。
- A2-8-01 目标 = **最小、可验证、可监督的 Program-level FRAME_SWITCH
  Execution Group**（非"input-selector+双 Pipeline"）。
- 依赖链冻结：SwitchPolicy→SwitchIntent→SwitchExecutionPlan→Switch
  Execution Adapter→{Input A/B Pipeline, Program Graph}→Program Output→
  Observation/PTS→RuntimeEvent→Watchdog→Health/Custody。
- OQ-1：inter 系 = 候选 Execution Materialization **非架构合同**——
  GStreamer topology 属实现细节，换拓扑不得触及 Program Domain。
- OQ-2：独立 Switch Execution Adapter（不塞 MediaBackend 生命周期五方法）。
- OQ-3：ExecutionGroup 概念冻结（Session=生命周期 /
  ExecutionGroup=inputs+active source+switch+program graph+group
  observation / Switch Execution=A↔B / Watchdog=四观测）；MultiInputWatchdog
  单实例服务 execution group，修正 bin L403+gates L165 首 handle 用法，
  禁 for 循环双 spawn；非 God Object，喂现有 RuntimeEvent→Custody→Health 链。
- OQ-4：六路 PTS observation only（A/B/Program × video/audio）；禁
  AvSyncEngine、禁 threshold 进 MasterJoin。
- OQ-5：Program pipeline 归 Program Execution/Switch 层构建；SessionManager
  lifecycle only；Supervisor recovery only。
- 状态三分离 Desired（ACTIVE_A/B/SWITCHING）≠ Execution（selector pad）≠
  Observed（actual pad/PTS/frames）；禁 Session.active_input /
  SessionInput.is_active；SessionInput{device_id,handle} 原样保留。

## 关键取舍与风险

- **Audio=最大隐藏风险**：终裁采方案 A（Video/Audio 成对切换），audio 不得
  顺手 audiomixer 冒充已解决；Video=B/Audio=A 分离态直接进 Master Join 问题。
- T4：element property（active-pad/switch-mode）≠ PASS——须实证 A active→
  switch(B)→B 成为 program source→output 存活 + PTS 无 rollback/异常跳变。
- Event Identity Debt（PipelineFault.pipeline legacy DeviceId 双语义）不修，
  新增代码不得扩大歧义（V0.3 Event Contract change）。
- MASTER_SWITCH Deferred（normalize Gap 不顺手补）；auto-failover /
  PACKET_SWITCH / HLS+RTMP 多输出全程 Deferred。
- 12 红线 + 禁 PipelinePlan 塞 A/B 三案全程生效。

## 测试策略

- T1-T12 验收矩阵（probe §7.5，替代第一轮 T1-T5）：mock 层先行 + 盒上
  cargo（真机留 02）。
- 01 完成标准 = 真实 Execution Graph + 真实 A/B 切换 + MultiInputWatchdog
  架构落地（不停在"设计完成"）。
- 链：01 实现→02 真机→03 failure/supervision→04 AV continuity→05
  archive+CI+merge；A2-8 NOT CLOSED until 05。

## Spec Patch

无（本 change 无 specs/*/spec.md delta；验收矩阵以 tasks.md + probe §7 为准）。
