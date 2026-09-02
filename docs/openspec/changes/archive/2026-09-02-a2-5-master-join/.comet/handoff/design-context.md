# Comet Design Handoff

- Change: a2-5-master-join
- Phase: design
- Mode: compact
- Context hash: fb99cf1e62b222a3345f0878cc73daacef545ae32602a4c440dbf047ca25eea3

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/a2-5-master-join/proposal.md

- Source: docs/openspec/changes/a2-5-master-join/proposal.md
- Lines: 1-36
- SHA256: a248347bc6859991e37a981a2b6f12f1ee097e5145e199293251e7b8a4019d0b

```md
# Proposal — a2-5-master-join

## Why

裁定链 A2-5（A2-1..A2-4 全 CLOSED：SwitchPolicy 78a8319 / Video b68830c /
Audio b378a0d / Metadata 1779429）。三 Master 齐，V0.2 §1.20 L155 联合判定
（任一路 failed → DEGRADED/FAILOVER）是 Program Domain 的最后一块拼图。
用户裁定七刀链（00 Probe → 01 Shape → 02 输入/输出模型 → 03 实现 →
04 ProgramMaster+AVSync → 05 Semantic Review → 06 收口），第一刀只探针；
十危险点（Unknown≠NotPresent ≠Failed / facts≠declaration / Join fail-closed
五条 / failed 唯一来源 Runtime / D14 不偷渡 / Timecode SoT / AVSync 归 Join /
MetadataMaster 禁八字段）全部证据锚定后方可进 01。

## What Changes

- **A2-5-00 SoT Probe 报告**（本 change 首产物，零代码）:
  `docs/superpowers/reports/2026-09-02-a2-5-master-join-sot-probe.md`
  - V0.2 Join 侧全景（§1.20 联合判定/§8.9 Master=7 故障域且切源+垫片/
    §8.10 AVSync 先分类后动作/§8.11 三轴 health 含 UNKNOWN/§3.8+§3.13
    Errata-9 识别决策分离/11 处 Master Join 出现点）
  - 十危险点 V0.2+代码双锚表；OQ-A..OQ-E 五问交裁（Join 输出与 §8.9
    Master 域关系 / ProgramMaster 形态 / AVSync 落地范围 / classify 归属 /
    三路不对称就绪输入）；PD-1..PD-4 提案
- **A2-5-01+ 在 OQ 裁决后另立**，本刀到 design guard + handoff 为止

## Non-Goals

- master_join.rs / program_master.rs / avsync 任何生产代码
- 三 Master / Runtime / Event / Health 任何修改
- 词表冻结（01/02 后）；D14 语义引用；GStreamer 执行面（A2-7+）

## 验收场景

1. 十危险点全有 V0.2 节号 + 代码实锚双证据
2. OQ-A..E 原样上报不预裁决
3. 本刀零 .rs diff

```

## docs/openspec/changes/a2-5-master-join/design.md

- Source: docs/openspec/changes/a2-5-master-join/design.md
- Lines: 1-34
- SHA256: 38e9b08303cb136959ce83054a79d26ce67bd03849689f0ef9bdb7db3791ca75

```md
# Design — a2-5-master-join（A2-5-00 SoT Probe）

## 1. 定位

探针 change 首刀：产物 = SoT Probe 报告，零代码。设计 = 探针方法论
（A2-4 已验证纪律的复用）+ A2-4 Boundary Contract 作为既有输入。

## 2. 方法论（同 A2-4-00 三原则 + Join 特有边界）

1. 不类推：Join 语义只从 V0.2 散布锚点（§1.20/§3.7/§3.8/§3.13/§8.9-8.11）
   汇总，禁"三 Master 这样所以 Join 也这样"；
2. 不自创词：V0.2 无 MasterJoin 独立词表——需新词表处全部进 OQ 待裁；
3. 缺口原样上报：ProgramMaster 形态/Join 输出落点/classify 归属等缺口
   不为"完整"补模型。

## 3. 证据源

V0.2 §1.20 L155（联合判定唯一权威句）/§8.9（Master 故障域）/§8.10（AVSync
决策链）/§8.11（三轴 UNKNOWN）/§3.8+§3.13（Errata-9）+ 代码 @1779429
（A2-4-04 探针结论复核未变）+ A2-4 归档契约（Design §1.5a-1.5b）。

## 4. 裁决面

OQ-A Join 输出×§8.9 Master 域 · OQ-B ProgramMaster 形态 · OQ-C AVSync
范围 · OQ-D classify 归属 · OQ-E 三路不对称就绪输入。

## 5. No-Build Gate

零 .rs diff；不动三 Master/Runtime/Event/Health；不冻结词表。

## 6. 后续（OQ 裁决后）

01 Domain Shape Probe → 02 输入/输出模型裁定 → 03 实现 → 04 ProgramMaster
聚合+AVSync 边界 → 05 Semantic Review → 06 Verification & Delivery Closure。

```

## docs/openspec/changes/a2-5-master-join/tasks.md

- Source: docs/openspec/changes/a2-5-master-join/tasks.md
- Lines: 1-16
- SHA256: cca6298a744bf9cd547f56ef9b09d86fd1be4b627376db812ff420b5c3871993

```md
# Tasks — a2-5-master-join

> 四栏纪律。七刀链（用户裁定）：00 Probe → 01 Shape → 02 输入/输出模型 →
> 03 实现 → 04 ProgramMaster+AVSync → 05 Semantic Review → 06 收口。

- [x] 1. A2-5-00 SoT Probe: V0.2 Join 侧全景（§1.20/§8.9/§8.10/§8.11/
  §3.8+§3.13/11 出现点）+ 代码现状复核（Join/ProgramMaster/AVSync 零代码
  未变）+ 十危险点双锚表 + OQ-A..E/PD-1..4 + 报告落
  docs/superpowers/reports/2026-09-02-a2-5-master-join-sot-probe.md
  `Contract: V0.2 §1.20 L155+§8.9-8.11+Errata-9+A2-4 Boundary Contract` | 
  `Implementation: 已` | `Verification: 十危险点全双锚·零 .rs diff` | `Gate: 无`
- [ ] 2. 用户对 OQ-A..E 逐项裁决（A2-5-01+ 输入）
  `Contract: 用户裁定权` | `Implementation: 待` | `Verification: 裁决记录` | `Gate: 无`
- [ ] 3. A2-5-01..06 七刀链按裁决推进（形态/输入输出模型/实现/AVSync 边界/
  语义深审/收口——每刀边界待裁后定）
  `Contract: 裁决后另核` | `Implementation: 待` | `Verification: 后续核` | `Gate: 后续定`

```
