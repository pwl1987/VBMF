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

## 1.5 A2-4-02 词表与字段锁定（SQ-1..SQ-5 终裁后，编码前冻结）

> SQ 终裁（2026-09-02）：SQ-1 闭合 enum 最小三态 / SQ-2 enum declaration 禁裸 bool /
> SQ-3 data_plane 入字段（禁加 EVENT）/ SQ-4 空 Vec 合法 / SQ-5 无 payload；
> 补充裁决：**无 scope 字段**（Program Domain 结构即 scope，input scope 由
> CanonicalSourceRef 表达）。16 行字段终裁表全冻结（12 不要/绝对不要）。

### MetadataPresence（SQ-1）

```rust
pub enum MetadataPresence { Present, NotPresent, Unknown }
```
wire：`PRESENT / NOT_PRESENT / UNKNOWN`（SCREAMING_SNAKE_CASE，与 01 一致）。
**不收** `INVALID/DISCONTINUOUS/RECOVERED`（Timecode Observation 域语义，
绝不复制——测试锁定）。语义 = fact 在本对象中的存在性，非内容健康。
锚：`AudioPresence::{Present{..},NotPresent,Unknown}` 三态同形。无 Default
（fact 构造必须显式 presence，"默认存在/不存在"均无依据）。

### MetadataJoinDeclaration（SQ-2）

```rust
pub enum MetadataJoinDeclaration { Participating, NotPresent, Unknown }
```
wire：`PARTICIPATING / NOT_PRESENT / UNKNOWN`；词表快照 const `JOIN_DECLARATIONS`。
- `Participating` = Metadata 路正向声明参与 Program Master Join（facts 可空——
  SQ-4 合法组合：明确知道无额外 metadata 的节目）；
- `NotPresent` = 已观测并声明本 Program 无该路 metadata（≠ 没观测）；
- `Unknown` = 观测不足以声明。
命名论证：不叫 `READY`（Declaration≠Readiness 红线）；无 `JOINED/CONSUMED`
（Join 消费态属 A2-5 侧）；无 `NOT_APPLICABLE`（业务裁决源不存在——控制面
未建，加法演进留口）；类型名不用 `Status`（避免与 Health/readiness 混淆）。
锚：SQ-4 终裁例句直接以 NOT_PRESENT/UNKNOWN 作 declaration 值。
Default：`#[default] Unknown`（无观测前态）。

### MetadataFact（字段顺序与 wire 锁定）

```rust
pub struct MetadataFact {
    pub kind: MetadataType,          // 终裁表 "fact.type"; 物理名 kind=仓库压倒性惯例
    pub source: CanonicalSourceRef,  // 复用, 禁 MetadataSourceId
    pub presence: MetadataPresence,
}
```
serde 字段名 = 声明名（kind/source/presence，snake_case 与 descriptor 家族一致）。
无 timecode/timestamp/scope/payload 字段（终裁表四不要）。无 Default。

### MetadataMaster（字段顺序与 wire 锁定）

```rust
pub struct MetadataMaster {
    pub data_plane: MetadataDataPlane,            // SQ-3: canonical 自描述身份
    pub facts: Vec<MetadataFact>,                 // SQ-4: 空 Vec 合法, 禁推导
    pub join_declaration: MetadataJoinDeclaration,
}
```
derive `Default`（data_plane=Metadata 唯一值 / facts=[] / Unknown）+ `new()`
同值；**零字段级 `#[serde(default)]`**（A2-2 立规——缺字段 fail-closed，与
VideoMaster/AudioMaster 现状逐字一致）。全家族 Eq+Hash（CanonicalSourceRef
Copy+Eq 已备）。

### 测试级红线锁定（随编码交付）

拒收 `INVALID/DISCONTINUOUS/RECOVERED`（presence）；拒收 `READY/JOINED/
CONSUMED/TRUE`（declaration）；fact/master JSON 键集恰三键（禁字段蔓延的
wire 契约锁）；SQ-4 两组正交组合断言（空 facts+Participating 合法 /
非空 facts+Unknown 不禁）。

## 2. 防伪需求三原则（不类推/不自创词/缺口原样上报）

见 openspec design.md §2。词表已按 §1.5 锁定后编码。

## 3. 裁决面（交用户）

~~OQ-1..OQ-6~~ **已全部裁决**（probe 报告 §7）。

## 4. No-Build Gate（01 期边界）

零 .rs diff 于 VideoMaster/AudioMaster/timecode.rs/Master Join；不写 MetadataMaster struct；不冻结未证字段；A2-5 不碰。

## 5. 裁决后路线

A2-4-01 词表冻结（本文件 §1）→ 02 domain shape（facts+readiness，字段逐项证明）→
03 字段语义+serde（Option 边界/Unknown≠Absent）→ 04 Master Join boundary
review → 05 全回归 + architecture guard → 交付链
（review/verify/guards/archive/PR/CI/merge）。
