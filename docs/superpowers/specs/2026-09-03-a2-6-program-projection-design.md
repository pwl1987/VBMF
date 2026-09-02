---
comet_change: a2-6-program-projection
role: technical-design
canonical_spec: openspec
status: probe-stage
---

# Design Doc — a2-6-program-projection（A2-6: Program Runtime Projection — Ownership Probe Stage）

> 当前处于 **A2-6-00 Ownership/SoT Probe 阶段**（用户裁定：先解决
> "ProgramMaster 由谁拥有、从哪里产生"，再谈 projection）。证据产物 =
> [ownership-probe 报告](../reports/2026-09-03-a2-6-program-projection-ownership-probe.md)。
> 编码期设计（Owner/写入通道/投影形态）在 OQ-1..5 裁决后补入。

## 1. 探针决定性事实

- **ProgramMaster 当前无处产生**：`join()` 零生产调用者、三 Master writer
  零——SessionManager/MediaSession 字段全清点零 Program 引用。所有权问题
  的真正源头 = "join() 由谁在什么时机调用"（OQ-1+OQ-2 联合裁决）。
- **Snapshot 边界独立**：assemble 唯一装配点 + D14 swept 语义与 C′ 矛盾
  快照冲突——Program 语义快照禁并入 CanonicalRuntimeState。
- **API Boundary 先例完整**：内部 DTO 禁作 API DTO（api_boundary 禁令
  ❿原文）+ to_api_* 纯映射族——A2-6-01+ 沿用。
- **transport 五端点冻结**；RuntimeQuery 纯读 allowlist 7 fn 在 02 前零扩展。

## 2. 禁止捷径（终裁 §九全程生效）

禁从 SessionRuntimeState/GraphRuntimeIntent/CanonicalRuntimeState 重建
三 Master 再 compose——Runtime facts 反推 Program semantics = 边界反向。

## 3. 裁决面（交用户）

OQ-1 Owner 四候选 · OQ-2 写入时机（真前置）· OQ-3 Snapshot 边界 ·
OQ-4 API 命名 · OQ-5 None 投影形态。Q6/Q7/Q8 终裁已有。

## 4. No-Build Gate

零 .rs diff；六刀链禁跳；02 前禁制造第二个 ProgramMaster。

## 5. 裁决后路线（占位，勿执行）

01 Consumer+Shape 裁定 → 02 实现 → 03 Query 接线 → 04 API → 05 Transport
→ 06 收口（矩阵/verify/guards/archive/PR/CI/merge/memory）。
