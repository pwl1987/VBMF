# Comet Design Handoff

- Change: a2-8-dual-input-switch
- Phase: design
- Mode: compact
- Context hash: 1f4b201892a6ae95b09c3938f75e914b5e3ce6b46c2dce51427a1670da24adce

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/a2-8-dual-input-switch/proposal.md

- Source: docs/openspec/changes/a2-8-dual-input-switch/proposal.md
- Lines: 1-37
- SHA256: ea8a72420b588b21ba85a83b67c90f61897d7e9ed970b6b78eee2371bae91734

```md
# Proposal — a2-8-dual-input-switch

## Why

裁定链 A2-8（A2-0..A2-7 全 CLOSED @7745968）。用户终裁：**否决直接编码，
批准 A2-8-00 Dual-input Switch Execution SOT Probe**——A2-8 是 VBMF 首次进入
"Program Semantic → Execution Adapter → GStreamer Graph"实现层的关键节点；
当前具备"两个输入独立运行"（A1/A2-7 已证）但**不具备"输入间 Program-level
switch"执行基础**（无 Switch 节点/watchdog 单视角/switch_mode 是预留 intent）。

## What Changes

- **A2-8-00 Probe 报告**（零代码）:
  docs/superpowers/reports/2026-09-03-a2-8-dual-input-switch-sot-probe.md
  - 裁决事实断言复核（8 项全实锚确认：N-input 已备/first-pipeline 迁移
    边界/watchdog 单视角/无 Switch 节点/switch_mode 预留/单输出/双路独立
    输出≠切换/identity 三层）
  - 六问逐答（含盒上 GStreamer 元素实查：input-selector[active-pad/
    switch-mode]/inter 系/audiomixer 全在）
  - 12 红线 + PipelinePlan 禁塞 A/B 三案落盘；OQ-1..5 交裁
- **A2-8-01（最小、可验证、可监督的 Program-level FRAME_SWITCH Execution
  Group）经 Pre-Implementation Gate 后实现**——十项冻结 + T1-T12 验收矩阵 +
  Video/Audio 成对切换（方案 A）落 probe §7；PACKET/MASTER Switch 与
  auto-failover Deferred（终裁预裁）

## Non-Goals

12 红线全项（不改 V0.2/Event contract/不建 registry/SwitchPolicy 非执行器/
Switcher 不进 SessionManager/Supervisor 不切换/不虚构 Metadata/Normalize
非 Fact/不碰 V0.3/不做多输出/双路独立≠切换/无 continuity 证据不宣布完成）；
自动 failover（首版只做显式切换）。

## 验收场景

1. 六问全有实锚（代码/盒上 gst-inspect/既有 Gate 记录）
2. OQ-1..5 原样交裁不预裁决
3. 零 .rs diff

```

## docs/openspec/changes/a2-8-dual-input-switch/design.md

- Source: docs/openspec/changes/a2-8-dual-input-switch/design.md
- Lines: 1-32
- SHA256: f43d9d3341682c9b3d91b3b11883ae04429326d47ae7ed29b8462ccc74053fa3

```md
# Design — a2-8-dual-input-switch（A2-8-00 SOT Probe）

## 1. 定位

探针首刀：回答"双输入 Program-level switch 的执行基础"六问；A2-8 转折 =
从 Domain 模型线转入 Execution Adapter 实现线。

## 2. 方法论

事实断言先复核（8 项全锚）；GStreamer 能力盒上实查（非文档推测）；缺口
如实披露（watchdog 单视角=Precondition Gate；AVSync 升级 A2-8 硬前置）。

## 3. 裁决面（已终裁，2026-09-03 两轮）

OQ-1 inter 系=**候选 Materialization 非架构合同**（topology=实现细节）·
OQ-2 独立 Switch Execution Adapter（不塞 Backend 五方法）· OQ-3
ExecutionGroup 级 MultiInputWatchdog 单实例（概念正式冻结）· OQ-4 六路
PTS 观测 only（无 Engine/无 threshold）· OQ-5 Program pipeline 归 Program
Execution/Switch 层。全部终裁与十项冻结落 probe §7。

## 4. No-Build Gate

零 .rs diff；12 红线；禁 PipelinePlan 硬塞 A/B。

## 5. A2-8-01 范围（Gate 后）

目标=**最小、可验证、可监督的 Program-level FRAME_SWITCH Execution
Group**（非"input-selector+双 Pipeline"）。T1-T12 验收矩阵；Video/Audio
成对切换（方案 A）；Desired≠Execution≠Observed 三分离；Event Identity
Debt 不修；完成标准=真实 Execution Graph+真实 A/B 切换+MultiInputWatchdog
落地（不停在设计完成）。链：01 实现→02 真机→03 failure/supervision→
04 AV continuity→05 archive+CI+merge；A2-8 NOT CLOSED until 05。

```

## docs/openspec/changes/a2-8-dual-input-switch/tasks.md

- Source: docs/openspec/changes/a2-8-dual-input-switch/tasks.md
- Lines: 1-52
- SHA256: b50d845a2c060165baf08f57e9e938eb50023a13dc6e354544906e9b930d1325

```md
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
- [ ] 3. A2-8-01 最小 FRAME_SWITCH Execution Group MVP（"最小、可验证、
  可监督的 Program-level FRAME_SWITCH Execution Group"，非"input-selector+
  双 Pipeline"）: SwitchIntent→SwitchExecutionPlan→SwitchExecutionAdapter
  链（独立执行面）+ ExecutionGroup 概念（inputs/switch/program output/
  supervision；SessionInput 原样）+ Program graph 构建（topology=实现细节，
  归 Program Execution/Switch 层）+ **Video/Audio 成对显式切换** + 六路 PTS
  观测点（A/B/Program×video/audio）+ MultiInputWatchdog（修正 bin L403+
  gates L165 首 handle 用法为 ExecutionGroup 四视角单实例，禁 for 循环双
  spawn）+ T1-T12 落地（mock 层先行，盒上 cargo 验证）; 12 红线全程
  `Contract: 00 终裁+Gate 十项冻结（probe §7）` | `Implementation: 待` | 
  `Verification: T1-T12 mock 层+盒上 cargo` | `Gate: 后续定`
- [ ] 4. A2-8-02 真机验证: 双 SDI inputs=2 同 Session 双 Pipeline + A→B→A
  显式切换 + frame boundary 实证 + Program output 存活; 真机 Gate 记录
  `Contract: 01 交付链` | `Implementation: 待` | `Verification: 真机 Gate` | `Gate: 待`
- [ ] 5. A2-8-03 failure/supervision 验证: watchdog 四视角观测穿
  RuntimeEvent→Custody 无跨设备污染 + Supervisor 边界（recovery only）
  `Contract: 02` | `Implementation: 待` | `Verification: 待` | `Gate: 待`
- [ ] 6. A2-8-04 AV continuity 验证: 六路 PTS before/after switch 无
  rollback/discontinuity/divergence/starvation（observation only，无 Engine）
  `Contract: 03` | `Implementation: 待` | `Verification: 待` | `Gate: 待`
- [ ] 7. A2-8-05 archive+CI+merge（A2-8 收口唯一入口; 01-04 任一完成不宣布
  CLOSED）
  `Contract: 04` | `Implementation: 待` | `Verification: CI+归档` | `Gate: 待`

```
