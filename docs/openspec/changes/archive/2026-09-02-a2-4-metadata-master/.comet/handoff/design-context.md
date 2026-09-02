# Comet Design Handoff

- Change: a2-4-metadata-master
- Phase: design
- Mode: compact
- Context hash: d2dc340e6e59bd46b657b12dc182a1a1b761e1c4a86ad6ef986be8ad8245527c

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/a2-4-metadata-master/proposal.md

- Source: docs/openspec/changes/a2-4-metadata-master/proposal.md
- Lines: 1-40
- SHA256: 96da415fc33fa240c5bb4720b115449add3eb520b32827f7f80e070867920c5a

```md
# Proposal — a2-4-metadata-master

## Why

裁定链 A2-4（A2-1 SwitchPolicy @78a8319✅ / A2-2 Video Master @b68830c✅ /
A2-3 Audio Master @b378a0d✅）。V0.2 §3.7 Metadata Graph 是三独立 graph 之一
（决策 #29），但**用户裁定 A2-4 第一刀 = SoT Probe，非编码**：
Metadata Graph 拓扑（三路并列源 Timecode/Subtitle/SCTE-35 → [Metadata
Master Join] → Program-scope Master (METADATA)）与 Video/Audio（各 3 中间
处理节点的串行链）**形态根本不同**——禁止类推五阶段模型。探针须先证明
Metadata 是什么，再决定它长什么样。

## What Changes

- **A2-4-00 SoT Probe 报告**（本 change 唯一产物，零代码）:
  `docs/superpowers/reports/2026-09-02-a2-4-metadata-master-sot-probe.md`
  - 七项必答 + 一项强制证据检查（Q1 节点/Q2 Data Plane/Q3 Timecode 定位/
    Q4 Program-Input 边界/Q5 Option vs fail-closed/Q6 serde wire/Q7 阶段
    语义/Q8 既有占位扫描）逐项以 V0.2 节号/行号 + 代码实锚作答
  - Evidence → Open Questions（OQ-1..OQ-6）→ Proposed Decisions（PD-1..PD-4,
    全部待裁决）→ No-Build Gate
- **裁决输入就绪**: OQ-1（Timecode=Metadata 源[V0.2 原文] vs Join 属性[用户
  先前口头]证据冲突）、OQ-2（Subtitle vs CAPTION 词汇张力）、OQ-6（A2-4
  形态——无阶段链读法下的组合声明模型）等六问交用户裁决
- **A2-4-01+（词表冻结/domain object）在裁决后另立任务**，本 change 到
  design 阶段为止（phase=design, STOP）

## Non-Goals

- Rust domain code（MetadataMaster struct/stage enum/transition matrix/serde）
- VideoMaster/AudioMaster/Master Join 任何修改（含 deprecated/仅注释标记）
- canonical vocabulary 猜测性冻结
- Subtitle/SCTE-35 解析执行面（A2-7+）

## 验收场景

1. Probe 报告七问全部有 V0.2 节号级证据（无类推、无自创词表）
2. Q8 证据确认 VideoMaster/AudioMaster 零 metadata/timecode 占位
3. 全部 SoT 缺口/张力原样进 Open Questions（不自行补模型）
4. 本 change 零 .rs diff

```

## docs/openspec/changes/a2-4-metadata-master/design.md

- Source: docs/openspec/changes/a2-4-metadata-master/design.md
- Lines: 1-48
- SHA256: f72c4d666dcb5c810ba11139bd7428f66fc5403902e18f0f3036656b983a057e

```md
# Design — a2-4-metadata-master（A2-4-00 SoT Probe）

## 1. 定位

本 change 是**探针 change**：产物 = SoT Probe 报告一份，零代码。
设计 = 探针方法论本身（用户裁定冻结的执行纪律）：

```
comet-open → A2-4-00 SoT Probe → reports/2026-09-02-a2-4-metadata-master-sot-probe.md
→ design guard → handoff → STOP（等用户逐项裁决）
```

## 2. 探针方法论（防伪需求三原则）

1. **不类推**：Metadata Graph 节点/语义只从 V0.2 §3.7 原文推导，禁止"Video/Audio
   这样所以 Metadata 也这样"——探针已证实三 graph 拓扑不同（Metadata 零中间
   处理节点）。
2. **不自创词**：Data Plane/类型词表只认 V0.2 §3.1（决策 #43 唯一定义规范）+
   metadata_type 五值；`RawMetadata`/`CanonicalMetadata` 等词不存在即禁用。
3. **缺口原样上报**：V0.2 未定义处（Program/Input 边界、字段级 fail-closed
   粒度、Health Tree 缺 Metadata Master Join 节点、Subtitle vs CAPTION）进
   Open Questions，不为"代码完整"人为补齐。

## 3. 证据源与采集范围

| 源 | 采集点 |
|---|---|
| ARCHITECTURE_V0.2.md（LOCK FINAL） | §1.13 L69 / §1.20 L138-155 / §2.4 L302-323 / §3.1 L331-404 / §3.7 L759-872 / §3.8 / §3.9 L901-933 / 决策 #5/#24/#29/#37/#43 |
| CLOCK_TIMECODE_CONTRACT.md（FROZEN） | §1 #147 Clock / §2 #148 Timecode / §3 替换不变量 |
| 代码（master b378a0d） | timecode.rs / normalize.rs / runtime_state.rs L106 / program/{video,audio}_master.rs 结构体 / 全库 metadata grep |

## 4. 裁决面（报告 §3，六问）

OQ-1 Timecode 归属（源 vs Join 属性，证据冲突）· OQ-2 Subtitle vs CAPTION ·
OQ-3 Health Tree 缺节点 · OQ-4 KLV/SYSTEM 未入图 · OQ-5 Program/Input 边界 ·
OQ-6 A2-4 形态（无阶段链读法下的组合声明模型）。

## 5. No-Build Gate

零 .rs diff；不冻结词表；不改已 CLOSED 的 A2-2/A2-3 类型；报告入 reports/
（不建 probes/ 目录——用户裁定）。

## 6. 后续（裁决后）

A2-4-01 词表冻结 → A2-4-02 MetadataMaster domain object → A2-4-03 迁移/
fail-closed/serde（形态依 OQ-6 裁决，可能是"在场性+joined 事实"而非 5×5）→
A2-4-04 Master Join boundary review → A2-4-05 全回归 → 交付链。
A2-5 Master Join 本 change 不碰。

```

## docs/openspec/changes/a2-4-metadata-master/tasks.md

- Source: docs/openspec/changes/a2-4-metadata-master/tasks.md
- Lines: 1-17
- SHA256: 049b651e404c4c762757e042d1207c6573819910b70b425d923a5e7767ccea8f

```md
# Tasks — a2-4-metadata-master

> 四栏纪律。本 change = A2-4-00 SoT Probe（零代码）。执行纪律（用户裁定冻结）:
> comet-open → probe 报告 → design guard → handoff → STOP 等裁决。

- [x] 1. A2-4-00 SoT Probe: 七项必答+Q8 强制检查全证据落袋（V0.2 §3.7 拓扑原文/
  §3.1 METADATA+metadata_type 五值/Timecode 三重证据交叉/边界锚点/词表现状/
  阶段语义独立推导/VideoMaster+AudioMaster 零占位确认）+ 报告落
  `docs/superpowers/reports/2026-09-02-a2-4-metadata-master-sot-probe.md`
  （Evidence/Open Questions/Proposed Decisions/No-Build Gate 全节）
  `Contract: V0.2 §3.7+§3.1+§1.20+决策#29/#43+CLOCK_TIMECODE #148` | 
  `Implementation: 已` | `Verification: 报告七问全有节号级证据` | `Gate: 无`
- [ ] 2. 用户对 OQ-1..OQ-6 逐项裁决（A2-4-01+ 的输入; 本 change 内不执行）
  `Contract: 用户裁定权` | `Implementation: 待` | `Verification: 裁决记录` | `Gate: 无`
- [ ] 3. A2-4-01+ 按 A2-4 裁决链推进（01 词表冻结→02 domain object→03 语义→
  04 Join boundary review→05 全回归→交付链; **形态依 OQ-6, 不预设 5×5**）
  `Contract: 裁决后另核` | `Implementation: 待` | `Verification: 后续核` | `Gate: 后续定`

```
