---
comet_change: a2-4-metadata-master
role: technical-design
canonical_spec: openspec
status: probe-stage
---

# Design Doc — a2-4-metadata-master（A2-4: Metadata Master — SoT Probe Stage）

> 本 change 当前处于 **A2-4-00 SoT Probe 阶段**（用户裁定：先证明 Metadata 是
> 什么，再决定它长什么样）。技术设计 = 探针方法论；证据产物 =
> [sot-probe 报告](../reports/2026-09-02-a2-4-metadata-master-sot-probe.md)。
> 编码期设计（词表/结构/迁移语义）在用户对 OQ-1..OQ-6 裁决后补入本文件。

## 1. 探针结论摘要（证据见报告）

- **拓扑**：Metadata Graph = 三路并列源（Timecode/Subtitle/SCTE-35）→
  [Metadata Master Join] → Program-scope Master (METADATA)。**零中间处理节点**
  ——与 Video/Audio（各 3 节点串行链）形态根本不同，五阶段模型不适用（PD-1）。
- **Data Plane**：`METADATA`（§3.1 四层之一，决策 #43 唯一定义）；二级
  `metadata_type: TIMECODE/CAPTION/SCTE35/KLV/SYSTEM`（UPPER_CASE 两处一致锁定）。
- **Timecode**：V0.2 原文 = Metadata Graph 输入源（§3.7 L801）；代码已实现
  Source 侧观测（timecode.rs CanonicalTimecode，#148）；AV Sync（≠Timecode）
  才是 Master Join 属性（L830）——证据冲突进 OQ-1 待裁决。
- **既有占位**：VideoMaster/AudioMaster 零 metadata/timecode 字段（Q8），
  无迁移风险。

## 2. 防伪需求三原则（不类推/不自创词/缺口原样上报）

见 openspec design.md §2。词表冻结与结构设计**全部延后**至裁决后。

## 3. 裁决面（交用户）

OQ-1 Timecode 归属 · OQ-2 Subtitle vs CAPTION · OQ-3 Health Tree 缺节点 ·
OQ-4 KLV/SYSTEM 未入图 · OQ-5 Program/Input 边界 · OQ-6 A2-4 形态
（"源在场性 + joined 事实"组合模型 vs 其他）。

## 4. No-Build Gate

零 .rs diff；不冻结词表；不改 A2-2/A2-3 已 CLOSED 类型；A2-5 Master Join
不碰。本阶段产物仅 reports/ 探针报告 + 本设计文档。

## 5. 裁决后路线（占位，勿执行）

A2-4-01 词表冻结 → 02 domain object → 03 迁移/fail-closed/serde（形态依
OQ-6）→ 04 Master Join boundary review → 05 全回归 → 交付链
（review/verify/guards/archive/PR/CI/merge）。
