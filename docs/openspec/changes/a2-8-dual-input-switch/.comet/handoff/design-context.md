# Comet Design Handoff

- Change: a2-8-dual-input-switch
- Phase: design
- Mode: compact
- Context hash: 83e69346561f0425d70913637a9c1b13a6a36584e6648ccbb3b2173205d8fcea

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/a2-8-dual-input-switch/proposal.md

- Source: docs/openspec/changes/a2-8-dual-input-switch/proposal.md
- Lines: 1-35
- SHA256: 31d65d3e05c7bbb3878559a8e3bdc5f7e970c4b5ff08355759a159d533d0093d

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
- **A2-8-01（最小 FRAME_SWITCH Execution Switch）在 OQ 终裁后实现**——
  PACKET/MASTER Switch Deferred（终裁预裁）

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
- Lines: 1-25
- SHA256: 4db53edf273ad6aca6ea1d83cadec5ca731b14da847571a728d9cfc4701e94a3

```md
# Design — a2-8-dual-input-switch（A2-8-00 SOT Probe）

## 1. 定位

探针首刀：回答"双输入 Program-level switch 的执行基础"六问；A2-8 转折 =
从 Domain 模型线转入 Execution Adapter 实现线。

## 2. 方法论

事实断言先复核（8 项全锚）；GStreamer 能力盒上实查（非文档推测）；缺口
如实披露（watchdog 单视角=Precondition Gate；AVSync 升级 A2-8 硬前置）。

## 3. 裁决面

OQ-1 Program graph 形态（inter 系倾向）· OQ-2 Switch Adapter 形态（独立
trait 倾向）· OQ-3 MultiInputWatchdog · OQ-4 AVSync 测量边界 · OQ-5 构图归属。

## 4. No-Build Gate

零 .rs diff；12 红线；禁 PipelinePlan 硬塞 A/B。

## 5. 后续（OQ 终裁后）

01 最小 FRAME_SWITCH Execution Switch（T1-T5 验收：ACTIVE/STANDBY 双向
显式切换；自动 failover 留后）→ 真机验证 → 收口。

```

## docs/openspec/changes/a2-8-dual-input-switch/tasks.md

- Source: docs/openspec/changes/a2-8-dual-input-switch/tasks.md
- Lines: 1-19
- SHA256: 41591e7ebc0f3129740467b0c4e527b922608afdf793242242e48143965de099

```md
# Tasks — a2-8-dual-input-switch

> 四栏纪律。00 Probe → 01 最小 FRAME_SWITCH Execution Switch（OQ 终裁后）
> → 真机验证 → 收口。12 红线全程。

- [x] 1. A2-8-00 SOT Probe: 裁决事实断言复核（8 项全锚）+ 六问逐答
  （Q1 双 Pipeline 真机=A1 已证 inputs=2·Q2 三候选形态[inter 系倾向]·
  Q3 input-selector frame-boundary 原生·Q4 独立 trait 倾向·Q5 
  MultiInputWatchdog=Precondition Gate·Q6 观测点=selector active-pad+双
  PTS+首版只显式切换）+ 盒上元素实查（input-selector/inter 系/
  audiomixer 全在）+ 12 红线+禁塞 A/B 落盘 + OQ-1..5 交裁; 报告=
  docs/superpowers/reports/2026-09-03-a2-8-dual-input-switch-sot-probe.md
  `Contract: A2-8 终裁[12 红线+六问+否决直接编码]` | `Implementation: 已` | 
  `Verification: 六问全实锚·零 .rs diff` | `Gate: 无`
- [ ] 2. 用户对 OQ-1..5 终裁（01 前置）
  `Contract: 用户裁定权` | `Implementation: 待` | `Verification: 裁决记录` | `Gate: 无`
- [ ] 3. A2-8-01 最小 FRAME_SWITCH Execution Switch（T1-T5 显式切换验收；
  PACKET/MASTER Deferred）→ 真机验证 → 收口交付链
  `Contract: 00 终裁+OQ 裁决` | `Implementation: 待` | `Verification: 后续核` | `Gate: 后续定`

```
