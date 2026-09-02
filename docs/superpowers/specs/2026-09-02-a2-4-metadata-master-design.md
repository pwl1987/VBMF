---
comet_change: a2-4-metadata-master
role: technical-design
canonical_spec: openspec
status: ruled-a2-4-01-approved
---

# Design Doc — a2-4-metadata-master（A2-4: Metadata Master）

> A2-4-00 SoT Probe 已收口（证据 = [sot-probe 报告 §1-6](../reports/2026-09-02-a2-4-metadata-master-sot-probe.md)）；
> 用户六项终裁已落盘（同报告 §7）。本文件自 §1' 起为**裁决后编码期设计**。

## 0'. 终裁要点（全文见 probe 报告 §7）

- **OQ-1**：Timecode = Input-local observation（timecode.rs 不动不搬）；AV Sync = Master Join property。
- **OQ-2**：CAPTION = canonical wire vocabulary；Subtitle = 源/载体语义。禁 `MetadataType::Subtitle`。
- **OQ-3**：Health Tree 张力 DEFERRED → A2-6/X5 reconciliation（A2-4 不扩修）。
- **OQ-4**：五值 taxonomy 全冻结；Graph source 只三路——**不凭 taxonomy 造节点**。
- **OQ-5**：三层边界 L1 Input-local Observation / L2 Canonical Metadata Fact / L3 Program-scope Metadata Master；extraction/parsing 不属 A2-4。
- **OQ-6**：**NO Stage / NO advance / NO transition matrix**；MetadataMaster = facts + join readiness/declaration；具体字段待 02/03 逐项证明。

### 设计 guard 红线（随裁决冻结，评审必查）

1. **三域差异**：VideoMaster=processing progression / AudioMaster=processing progression / **MetadataMaster=fact aggregation + join declaration**——形态不许趋同；Stage 化 Metadata = 创造 V0.3。
2. **Timecode ownership**：Ownership→Observation domain / Consumption→Metadata/Join 可引用 / Authority→Master Join 可用 / **Mutation→MetadataMaster 禁改写**（并禁 clock selection/sync/drift correction 进入）。
3. **VideoMaster/AudioMaster 零 diff**（含注释占位）。
4. **A2-5 预约束**：Join 判定用 readiness/joined facts，非 `all==MASTER_JOINED`。

## 1. A2-4-01 — Canonical Metadata Vocabulary Freeze（本轮已批范围）

**只冻结词表，不写 MetadataMaster**（domain shape 属 A2-4-02）：

- `MetadataType` 封闭五值：`Timecode/Caption/Scte35/Klv/System`，wire UPPER_CASE 逐字 `TIMECODE/CAPTION/SCTE35/KLV/SYSTEM`（V0.2 §3.1 L394-399 + §1.13 L69 两处一致，决策 #43）。
- `MetadataDataPlane` 单值 `Metadata`，wire `METADATA`（对齐 VideoDataPlane/AudioDataPlane 单值模式；Program-scope Master (METADATA) §3.7 L807）。
- Subtitle↔CAPTION 语义层级落 doc 注释：CAPTION=canonical taxonomy / Subtitle=Graph 源语义 / SRT/ASS=格式。
- 词表快照 const（同 SwitchPolicy ACCEPTED_LIST 纪律）+ serde fail-closed（拒 `SUBTITLE`/`SCTE_35`/未知串——测试锁定 OQ-2/OQ-4 裁决）。
- serde(default) 新生儿禁用（A2-2 立规；MetadataType 无天然默认值——taxonomy 不是阶段）。

## 2. 防伪需求三原则（不类推/不自创词/缺口原样上报）

见 openspec design.md §2。词表冻结与结构设计**全部延后**至裁决后。

## 3. 裁决面（交用户）

~~OQ-1..OQ-6~~ **已全部裁决**（probe 报告 §7）。

## 4. No-Build Gate（01 期边界）

零 .rs diff 于 VideoMaster/AudioMaster/timecode.rs/Master Join；不写 MetadataMaster struct；不冻结未证字段；A2-5 不碰。

## 5. 裁决后路线

A2-4-01 词表冻结（本文件 §1）→ 02 domain shape（facts+readiness，字段逐项证明）→
03 字段语义+serde（Option 边界/Unknown≠Absent）→ 04 Master Join boundary
review → 05 全回归 + architecture guard → 交付链
（review/verify/guards/archive/PR/CI/merge）。
