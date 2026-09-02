# Comet Design Handoff

- Change: a2-7-execution-materialization
- Phase: design
- Mode: compact
- Context hash: 2e2a3bdf79ff8bdd830d56880c2f765f0071804407f022d145aeb9b6d7f7de0a

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/a2-7-execution-materialization/proposal.md

- Source: docs/openspec/changes/a2-7-execution-materialization/proposal.md
- Lines: 1-38
- SHA256: 1855a8a539df3e6504871801f8ee10d1bdf0190504a7955aa8791927c16706b9

```md
# Proposal — a2-7-execution-materialization

## Why

裁定链 A2-7（A2-1..A2-6 全 CLOSED @caab630）。**分账探针定调**：
Materialization 不是 A2-7 的空白（materialize/MediaBackend SPI/Session
冻结链/watchdog 观测链全已存在，禁重造）；**真正的空白 = Program Semantic
Lifecycle**——Execution Fact boundary（零）+ Program Runtime Custody（零）
+ 三 Master writer（零）+ Metadata declaration producer（零）+ JoinInput
装配（零）。核心问题 = "什么运行时事实、什么时刻、以什么 ownership/SoT/
语义等级，允许推动 Program Domain 状态前进"。

## What Changes

- **A2-7-00 SoT/Ownership Probe 报告**（本 change 首产物，零代码）:
  `docs/superpowers/reports/2026-09-03-a2-7-execution-materialization-sot-probe.md`
  - 分账表：已有八项能力实锚（禁重造）vs 缺失五项（真空白）
  - Execution Fact 候选七维裁表（9 候选逐一：来源/durable/observation/
    可推 stage/仅 Join input）
  - 六问 A-F 证据（**关键披露：SWITCHED/PROGRAM_COMPOSED 的执行事实当前
    不存在**——管线无独立 Switcher/Composition 节点，推进方式须裁）
  - OQ-1..5 交裁（stage 事实映射/Metadata producer/failed 转换边界/
    AVSync 上游/Custody 挂载层）
  - 十项禁止清单落盘（禁万能 ExecutionFact 巨型 struct 等）
- **A2-7-01+（Fact Shape/Custody/链路/mock 验证/真机前置）在 OQ 裁决后
  按 Gate 链推进**，本刀到 design guard + handoff 为止

## Non-Goals

- 重写 materialize/SessionManager/MediaBackend/GStreamer abstraction
- Watchdog/Supervisor 改造为 Program owner；新建 AVSync Engine
- Query/Transport API；双输入真机切换（A2-8）；万能 ExecutionFact

## 验收场景

1. 分账两侧全有代码实锚（已有八项 + 缺失五项）
2. 九候选七维逐个定位；六问全有证据锚
3. 本刀零 .rs diff

```

## docs/openspec/changes/a2-7-execution-materialization/design.md

- Source: docs/openspec/changes/a2-7-execution-materialization/design.md
- Lines: 1-30
- SHA256: dbb20c6b2acff3b11c0da415212f78bbb9b35f1b37b04b764bedaa05be4c160c

```md
# Design — a2-7-execution-materialization（A2-7-00 SoT/Ownership Probe）

## 1. 定位

探针 change 首刀：产物 = SoT/Ownership Probe 报告，零代码。A2-7 核心 ≠
"怎么启动 GStreamer"（已存在），= Execution Fact → Custody → advance →
join → ProgramMaster snapshot 的 Semantic Lifecycle 闭环。

## 2. 方法论

1. 分账先行：已有能力禁重造（八项实锚），缺失侧五项才是交付面；
2. 事实候选不预设：9 候选七维逐个裁，红线 = Session Running≠stage、
   Clock≠AVSync、pipeline 级≠节点级；
3. 缺口如实披露：SWITCHED/PROGRAM_COMPOSED 执行事实当前不存在——推进
   方式（deferred vs 声明性推进）交裁，不为闭环伪造事实。

## 3. 裁决面

OQ-1 stage 事实映射逐阶段 · OQ-2 Metadata producer · OQ-3 failed 转换
边界 · OQ-4 AVSync 上游 · OQ-5 Custody 挂载层与 SessionManager 协作。

## 4. No-Build Gate

零 .rs diff；十项禁止清单；01 前不定义 ExecutionFact 形态。

## 5. 后续（OQ 裁决后）

01 Execution Fact Shape/Ownership → 02 Program Runtime Custody →
03 Execution→Master→Join→Snapshot → 04 Mock/Simulation lifecycle 验证 →
05 真机前置验证 → A2-8 双输入真机切换。

```

## docs/openspec/changes/a2-7-execution-materialization/tasks.md

- Source: docs/openspec/changes/a2-7-execution-materialization/tasks.md
- Lines: 1-23
- SHA256: 5219e6f622ed6c6b77c19ad297b1bf1388348970672239a5e0b10783c5e42916

```md
# Tasks — a2-7-execution-materialization

> 四栏纪律。Gate 链（用户裁定冻结）：00 SoT/Ownership Probe → 01 Fact 
> Shape/Ownership → 02 Custody → 03 Execution→Master→Join→Snapshot → 
> 04 Mock/Simulation 验证 → 05 真机前置 → A2-8 双输入真机切换。

- [x] 1. A2-7-00 SoT/Ownership Probe: **分账**（已有八项实锚禁重造:
  materialize L529-549/MediaBackend SPI 五方法/Session 冻结链 L11+L530/
  watchdog b1-b3+a4 闩锁/Bus 词表/PipelineHandle+Health/outputs 投影/
  Production 等 Intent; 缺失五项=Execution Fact boundary/Custody/三 
  Master writer/Metadata producer/JoinInput 装配全零）+ 9 候选七维裁表 +
  六问 A-F 证据（**关键披露: SWITCHED/PROGRAM_COMPOSED 执行事实不存在**
  ——管线无独立 Switcher/Composition 节点）+ 十项禁止清单 + OQ-1..5 交裁;
  报告=docs/superpowers/reports/2026-09-03-a2-7-execution-materialization-
  sot-probe.md
  `Contract: A2-6 终裁 Gate 链+分账要求+十项禁止` | `Implementation: 已` | 
  `Verification: 分账两侧全实锚·零 .rs diff` | `Gate: 无`
- [ ] 2. 用户对 OQ-1..5 逐项裁决（01 输入; 含 SWITCHED/COMPOSED 推进方式:
  deferred vs 声明性推进; Metadata producer 归属）
  `Contract: 用户裁定权` | `Implementation: 待` | `Verification: 裁决记录` | `Gate: 无`
- [ ] 3. A2-7-01..05 按 OQ 裁决推进（Fact Shape/Ownership→Custody→链路→
  mock 验证→真机前置）
  `Contract: Gate 链` | `Implementation: 待` | `Verification: 后续核` | `Gate: 后续定`

```
