# Comet Design Handoff

- Change: a2-6-program-projection
- Phase: design
- Mode: compact
- Context hash: ff1787242df06465449223d456d0802fa546439dc5423c04f0b2174c57fa6f7b

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/a2-6-program-projection/proposal.md

- Source: docs/openspec/changes/a2-6-program-projection/proposal.md
- Lines: 1-36
- SHA256: aa6176ec9826c043b1579fbba29ed40b88b2868e9ce9e2a222eb2bba1b0f5cbc

```md
# Proposal — a2-6-program-projection

## Why

裁定链 A2-6（A2-1..A2-5 全 CLOSED：SwitchPolicy 78a8319 / Video b68830c /
Audio b378a0d / Metadata 1779429 / Join+ProgramMaster 2166d25）。Program
Domain 六文件闭环后，第一个 Domain→Runtime→API 三层边界考验阶段。
**用户裁定：先 Ownership/SoT Probe 再 projection**——避免"为做 API
projection 在 Runtime 里复制/拼装 ProgramMaster"这一最危险错误。

## What Changes

- **A2-6-00 Ownership/SoT Probe 报告**（本 change 首产物，零代码）:
  `docs/superpowers/reports/2026-09-03-a2-6-program-projection-ownership-probe.md`
  - 八问逐项证据（Owner 零现状/生命周期零触发点/Snapshot 独立边界/
    API 命名不预设/None 投影禁坍缩/AVSync 透传禁 Health 化/failed 不暴露/
    inconsistency 默认不暴露）
  - **关键事实：`join()` 与三 Master writer 零生产调用者**——任何 projection
    设计在"三 Master 谁写"裁决前是空中楼阁（OQ-2 真前置）
  - OQ-1..5 交裁（Owner/写入时机/Snapshot 边界/API 命名/None 投影形态）
  - 禁止捷径红线落盘（禁从 RuntimeState 重建三 Master 再 compose）
- **A2-6-01+（Consumer/Shape/实现/Query/Transport/收口）在 OQ 裁决后按
  六刀链推进**，本刀到 design guard + handoff 为止

## Non-Goals

- API DTO / RuntimeState 修改 / RuntimeQuery 扩展 / transport 接线（六刀链
  顺序禁跳）；SessionManager 塞 ProgramMaster（禁）；从 Runtime facts 反推
  Program semantics（禁）；join() 生产 wiring（OQ-2 裁决后另立）

## 验收场景

1. 八问全有代码实锚（SessionManager 字段清点/assemble 唯一/allowlist 7 fn/
   api_boundary 禁令原文）
2. OQ-1..5 原样上报不预裁
3. 本刀零 .rs diff

```

## docs/openspec/changes/a2-6-program-projection/design.md

- Source: docs/openspec/changes/a2-6-program-projection/design.md
- Lines: 1-36
- SHA256: ecff9a6b32442960cbb6ec457e1902004f0a028e7fe234d76ffccb0682027b6b

```md
# Design — a2-6-program-projection（A2-6-00 Ownership/SoT Probe）

## 1. 定位

探针 change 首刀：产物 = Ownership/SoT Probe 报告，零代码。核心问题 =
"ProgramMaster 由谁拥有、从哪里产生"先于 projection 形态。

## 2. 方法论（A2-4/A2-5 已验证纪律复用 + 本阶段特有）

1. 不预设：API 资源命名/None 投影形态/Query 接线全部等消费者证据；
2. 所有权先于形态：owner 与 join() 触发时机联合裁决（OQ-1+OQ-2）；
3. 禁止捷径：Runtime facts 反推 Program semantics = 边界反向，全程打回。

## 3. 证据源

session.rs（SessionManager/MediaSession 字段全清点）· runtime_state.rs
（assemble 唯一装配点 + D14 SnapshotObservation）· runtime_query.rs
（纯读 allowlist）· api_boundary.rs（资源族 + 内部 DTO 禁令原文）·
transport.rs（五端点冻结）· master_join.rs（join() 零生产调用者）·
program/mod.rs（Canonical/Adapter 边界纪律）。

## 4. 裁决面

OQ-1 Owner（A/B/C/D 候选）· OQ-2 join()/三 Master 写入时机（真前置）·
OQ-3 Snapshot 边界（独立 vs 并入 D14）· OQ-4 API 命名与语义 · OQ-5 None
投影形态。Q6/Q7/Q8 已有终裁（透传禁 Health 化/不暴露/默认不暴露）。

## 5. No-Build Gate

零 .rs diff；六刀链顺序禁跳（00→01→02→03→04→05→06）；02 前禁制造
第二个 ProgramMaster。

## 6. 后续（OQ 裁决后）

01 Consumer+Shape 裁定 → 02 Projection 实现 → 03 Query 接线 → 04 API
Projection → 05 Transport → 06 收口交付链。

```

## docs/openspec/changes/a2-6-program-projection/tasks.md

- Source: docs/openspec/changes/a2-6-program-projection/tasks.md
- Lines: 1-21
- SHA256: fe156ec6ea0bfcc4b256c80b028af21569275995a14135342b7a5cbf2180d0fc

```md
# Tasks — a2-6-program-projection

> 四栏纪律。六刀链（用户裁定冻结）：00 Ownership/SoT Probe → 01 
> Consumer+Shape → 02 Projection 实现 → 03 Query 接线 → 04 API Projection
> → 05 Transport → 06 收口。02 前禁止制造第二个 ProgramMaster。

- [x] 1. A2-6-00 Ownership/SoT Probe: 八问逐项证据（Q1 Owner 零现状=
  SessionManager/MediaSession 字段全清点零 Program 引用 / Q2 **join() 与
  三 Master writer 零生产调用者——真前置** / Q3 Snapshot 独立边界=
  assemble 唯一装配点 + D14 绑定禁令 / Q4 API 命名不预设 / Q5 None 投影
  禁坍缩 / Q6 AVSync 透传禁 Health 化 / Q7 failed 不暴露 / Q8 
  inconsistency 默认不暴露）+ 禁止捷径红线落盘 + OQ-1..5 交裁; 报告=
  docs/superpowers/reports/2026-09-03-a2-6-program-projection-ownership-probe.md
  `Contract: A2-5 终裁六刀链+八问+禁止捷径` | `Implementation: 已` | 
  `Verification: 八问全有代码实锚·零 .rs diff` | `Gate: 无`
- [ ] 2. 用户对 OQ-1..5 逐项裁决（01 输入; OQ-2 真前置——join()/三 Master
  写入通道或显式 deferred 到 A2-7）
  `Contract: 用户裁定权` | `Implementation: 待` | `Verification: 裁决记录` | `Gate: 无`
- [ ] 3. A2-6-01..06 按 OQ 裁决推进（Consumer+Shape 裁定→Projection 实现→
  Query 接线→API→Transport→收口）
  `Contract: 六刀链` | `Implementation: 待` | `Verification: 后续核` | `Gate: 后续定`

```
