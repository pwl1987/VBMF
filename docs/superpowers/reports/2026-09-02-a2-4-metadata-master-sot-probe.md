# A2-4-00 — Metadata Master SoT / V0.2 Metadata Graph Probe

> Status: `PROBE ONLY / NO CODE CHANGE`
> Authority: `V0.2 Architecture Baseline — LOCK FINAL`（ARCHITECTURE_V0.2.md，含 V0.2.4 Errata 全部修订）
> Date: 2026-09-02
> Change: a2-4-metadata-master（phase=design，本报告为 A2-4-01 前置裁决输入）
> Base: master `b378a0d`（A2-3 Audio Master 已收口）

---

## 0. 探针任务定义（用户裁定冻结）

七项必答 + 一项强制证据检查，禁止自行补模型；发现 SoT 缺口原样进 Open Questions。
核心问题：**V0.2 的 Metadata Graph 是真正拥有独立生命周期/阶段语义的 Graph，还是仅为 Master Join 提供 canonical metadata facts 的语义层？**

---

## 1. Evidence（按问题编号）

### Q1 — Metadata Graph 节点（V0.2 §3.7 原文，L800-808）

```
Metadata Graph:
  Timecode
  Subtitle (SRT/ASS)
  SCTE-35
   ↓
  [Metadata Master Join]
   ↓
  Program-scope Master (METADATA)
```

**发现：Metadata Graph 与 Video/Audio Graph 形态根本不同。**

| Graph | 拓扑（§3.7 原文） | 中间处理节点数 |
|---|---|---|
| Video | Source→[Normalize]→[Switcher]→[Program Composition]→[Video Master Join]→Master(RAW_VIDEO) | 3（串行链） |
| Audio | Source→[Audio Mixer]→[Loudness]→[Audio Delay]→[Audio Master Join]→Master(RAW_AUDIO) | 3（串行链） |
| **Metadata** | **三路并列源（Timecode/Subtitle/SCTE-35）直接汇入 [Metadata Master Join]→Master(METADATA)** | **0（无串行链）** |

§1.20（L142-150）同一图式：`Metadata Graph: Timecode / Subtitle / SCTE-35 → Master Join → Program Master`。
决策 #29（L1908）：Program Master = Video + Audio + Metadata 三个独立 graph（锁定）。
§1.20 V0.2.3 措辞修正（L155）：三 graph **处理层独立隔离 + Master Join 处联合判定**——任一路 failed → Program Master `DEGRADED` 或触发 `FAILOVER`。

### Q2 — Canonical Data Plane（V0.2 §3.1，L331-404）

- 唯一定义规范 = **§3.1**（决策 #43：§1.13 等章节只引用不重复定义）。
- `METADATA` 是 Data Plane 四层之一：`layer: METADATA`，与 Elementary/Container/Control 并列（L337, L389-390）。
- 描述："同步/描述信息（与媒体帧时序绑定）"；examples: `scte-35, klv, timecode, sei, caption, vanc, cea-708`（L392）。
- 资源代价 `ZERO_OR_LOW`（V0.2.3 修正：不完全是 ZERO；VANC/CEA-708/字幕/SCTE-35 splice 需在 mux/demux 边界重注入）。
- **二级分类（V0.2.3 新增，L394-399）**：`metadata_types: [TIMECODE, CAPTION, SCTE35, KLV, SYSTEM]`。
- §1.13 TypeScript 两维表示（L69）与 §3.1 YAML **逐字一致**：`{ layer: "METADATA"; type: "METADATA"; metadata_type: "TIMECODE" | "CAPTION" | "SCTE35" | "KLV" | "SYSTEM" }`，UPPER_CASE 锁定。
- Program-scope Master 数据平面 = **`METADATA`**（L807）。**不存在** `RawMetadata` / `CanonicalMetadata` 等词——V0.2 词表即 `METADATA` + metadata_type 五值。

### Q3 — Timecode 定位（三重证据交叉）

**(a) V0.2 §3.7 L801：Timecode 是 Metadata Graph 的三路源之一**（输入侧，与 Subtitle/SCTE-35 并列）。

**(b) CLOCK_TIMECODE_CONTRACT.md（FROZEN）**：Timecode 状态 = `Present / Absent / Invalid / Discontinuous / Recovered`（#148 冻结词表）；Clock 是运行时观测、不写回 Graph（R3 Observation≠Configuration）。

**(c) 代码现状（0.7B-2C 已交付）**：`src/timecode.rs::CanonicalTimecode`（presence/format/value/frame_rate/evidence）已存在且被 `src/normalize.rs::CanonicalMediaDescriptor.timecode` 消费——**Timecode 已实现为 Source 侧 per-input 观测事实**（normalize 恒 unknown，无观测绝不臆造 00:00:00:00）。

**(d) 关键区分——AV Sync ≠ Timecode**：V0.2 L830 "**AV Sync 测量在 Master Join 处**。AV Sync 不再是普通 Process Node——**它是 Master Join 的属性**"；§3.8 AVSync Manager = 横切管理器（决策 #37）；§2.4 引擎表 L319：AV Sync 接受 `RAW_* (in master join)`、输出 METADATA。

> ⚠️ **证据冲突（进 User Decision Required）**：用户此前口头裁定"时码元数据 = Master Join 属性"；V0.2 原文锁定的是 **AV Sync = Master Join 属性**，而 **Timecode = Metadata Graph 输入源**（观测事实）。两者是不同概念。探针不预裁决，原样上报。

### Q4 — Program-scope vs Input-local Metadata 边界

V0.2 **未给出显式边界定义**，仅有间接锚点：

- Input 侧：Source (SRT/RTMP) 产出 `COMPRESSED_VIDEO + COMPRESSED_AUDIO + METADATA`（L305）——压缩源携带 METADATA；Source (SDI) 仅产出 RAW_VIDEO+RAW_AUDIO（L304），SDI 的 metadata 提取路径（VANC/CEA-708 嵌入式）在 §3.7 图中**未画节点**。
- Normalize (stream) 三子能力含 `METADATA_REWRITE`（COMPRESSED 域内，L312）——唯一的 metadata 处理节点，且属 stream normalize 不属 Metadata Graph。
- QC 接受 METADATA、产出 METADATA/EVENT（L320）——QC 是 METADATA 生产者（只读引擎）。
- Program 侧：Metadata Master Join 产物 = Program-scope Master (METADATA)（L807）。
- 代码侧事实：`CanonicalMediaDescriptor.timecode` = per-input（input-local）已存在；`runtime_state.rs:106`（D15 契约注释）显式登记 "audio 多轨/timecode/metadata 属后续"。

### Q5 — Option vs fail-closed

V0.2 对 Metadata 字段级 Option/fail-closed **无规定**（无此粒度）。可循既有先例（不预裁决，供 A2-4-01/02 参考）：
- `TimecodeValue = Option`（无观测不臆造值，timecode.rs L45-46）；
- `TimecodePresence` 封闭词表 fail-closed（未知串 serde 拒收）；
- A2-2/A2-3 立规：新生儿类型禁 `serde(default)`，缺字段 fail-closed。

### Q6 — serde / wire vocabulary 现状

- **代码层 Metadata 零 wire 面**：`metadata` 全库 grep 仅 4 处（device.rs 注释 / resolver.rs `std::fs::metadata` 文件系统 API / runtime_state.rs D15 注释 / program/mod.rs 路线图注释）。api_boundary.rs / transport.rs / events.rs / api 投影**零引用**。
- V0.2 已锁定的 metadata 词表：`METADATA`（Data Plane）+ `metadata_type` 五值 `TIMECODE/CAPTION/SCTE35/KLV/SYSTEM`（UPPER_CASE，两处一致）。
- **词汇张力（进 Open Questions）**：§3.7 图示与 H5 术语表用 "**Subtitle** (SRT/ASS)"（L802, L283, 决策 #5 L1884），而 metadata_type 词表用 "**CAPTION**"（L69, L396）——字幕概念的图示词与分类词不同名，V0.2 未显式对账。
- Timecode 已有 wire 面：经 `CanonicalMediaDescriptor` serde（runtime_state/api_boundary 投影链已存在）。

### Q7 — 阶段迁移语义（独立推导，不套用 Video/Audio 五阶段）

**证据结论：Metadata Graph 无阶段链。**

- §3.7 图中 Metadata 路径无任何中间处理节点（Video/Audio 各 3 个）——不存在 Normalize/Switcher/Mixer 对应物，因此**不存在相邻阶段迁移**；强行造 5×5 advance 矩阵 = 伪需求。
- V0.2 为 Metadata 路定义的唯一动态语义是**联合判定**（§1.20 L155）：任一路 failed → Program Master DEGRADED/FAILOVER——这是 **Master Join 层（A2-5）** 的语义，不是 Metadata Graph 内部 stage 语义。
- §3.9 Health Tree（L901-916）节点表列有 `Video Master Join ●` / `Audio Master Join ●` / `Program Master ●`，**无 Metadata Master Join 节点**——文档内部张力（§3.7 图有、健康树示例无；示例是否穷尽未说明）。

### Q8 —（强制）现有 VideoMaster / AudioMaster 的 metadata/timecode 占位

- `VideoMaster { stage, data_plane, composition }`（video_master.rs L56-64）——**零 metadata/timecode 字段**。
- `AudioMaster { stage, data_plane, mix_layout, delay_ms, loudness_lufs }`（audio_master.rs）——**零 metadata/timecode 字段**。
- 两类型当前消费方 = 仅声明面单元测试 + `src/program/mod.rs` re-export；无 API/serde-wire/持久化/事件引用（A2-2/A2-3 均为 declaration-only，GStreamer 属 A2-7+）。
- `SwitchPolicy`/`SwitchIoPlane` 亦零 metadata 引用。
- 结论：**无任何既有占位需要迁移或标注；不存在污染已 CLOSED domain model 的风险**——"Timecode=Join 属性"若最终裁决成立，落点在 A2-5 新类型，不动 VideoMaster/AudioMaster。

---

## 2. 对核心问题的证据回答（供裁决，非裁决本身）

> V0.2 的 Metadata Graph 是真正拥有独立生命周期/阶段语义的 Graph，还是仅为 Master Join 提供 canonical metadata facts 的语义层？

证据形态：**是三独立 graph 之一（决策 #29 + 故障域隔离 + 联合判定），但拓扑是"多源在场 → 单 Join 点"，无独立阶段链**。即：
- "独立性"体现在**故障域与 Join 判定**（任一路 failed 影响 Program Master），不在内部 stage 推进；
- 它是 Graph（有源、有 Join、有 Master 产物），但其唯一内部结构 = **源的在场性**，无处理管线。

若此读法获裁决确认，A2-4-02 的自然形态是**源在场性事实 + Join 就绪事实**的组合声明模型（类似 ProgramComposition 事实位），而非 VideoMasterStage 式阶段机——最终形态待用户裁决。

---

## 3. Open Questions / User Decision Required

| # | 问题 | 证据位置 | 影响的后续步骤 |
|---|---|---|---|
| OQ-1 | **Timecode 归属**：V0.2 原文 = Metadata Graph 输入源（观测事实）；用户先前口头 = "Master Join 属性"。二者冲突，须裁决 A2-4 采用哪个（或区分：Timecode=源、AV Sync=Join 属性，两者并存） | §3.7 L801 vs L830；CLOCK_TIMECODE_CONTRACT #148；timecode.rs 已实现 Source 侧 | A2-4-01/02 词表与结构 |
| OQ-2 | **Subtitle vs CAPTION 词汇**：图示/H5/决策#5 用 Subtitle(SRT/ASS)，metadata_type 词表用 CAPTION——A2-4-01 冻结 wire 词表时以哪个为准 | L802/L283/L1884 vs L69/L396 | A2-4-01 |
| OQ-3 | **Metadata Master Join 是否入 Health Tree**：§3.7 图有该节点，§3.9 示例无 | L805 vs L901-916 | A2-6 Runtime Projection / X5 |
| OQ-4 | **KLV / SYSTEM 两类 metadata_type 未出现在 §3.7 图**：图示仅三源（Timecode/Subtitle/SCTE-35），词表五值——A2-4 声明面覆盖三源还是五值 | L394-399 vs L800-803 | A2-4-01/02 |
| OQ-5 | **Program/Input 边界**：V0.2 未定义哪些 metadata 进 program scope；SDI 源的 metadata 提取（VANC/CEA-708）无图示节点 | §2.4 L304-305；§3.1 examples | A2-4-02 / A2-6 |
| OQ-6 | **A2-4 形态**：若"无阶段链"读法确认，MetadataMaster 是否 = 源在场性 + joined 事实的组合模型（无 advance/5×5）？还是用户另立形态 | §1 全部 | A2-4-02/03 |

## 4. Proposed Decisions（仅为提案，全部待裁决后才生效）

- **PD-1**：A2-4 不实现 VideoMasterStage 式阶段枚举与 advance 矩阵（证据：零中间节点）。若 OQ-6 确认，声明面 = `MetadataMaster { 源在场性(按裁决后的词表), joined 事实位 }`，Data Plane 单值 `METADATA`（类型层不可构造其他平面，对齐 Errata-3 精神）。
- **PD-2**：wire 词表沿用 V0.2 已锁 UPPER_CASE（`METADATA` + `metadata_type`），Subtitle/CAPTION 二选一待 OQ-2 裁决；不自创新词。
- **PD-3**：`CanonicalTimecode`（已存在，Source 侧）不被 A2-4 改动；Metadata Graph 若引用 Timecode 源，以组合（含 CanonicalTimecode）非复制表达。
- **PD-4**：VideoMaster/AudioMaster 零改动（Q8 证据：无占位即无迁移）。

## 5. No-Build Gate（本刀禁止清单——用户裁定冻结）

- Rust domain code（MetadataMaster struct / stage enum / transition matrix / serde 实现）
- VideoMaster / AudioMaster / Master Join 任何修改（含 deprecated/仅注释标记）
- canonical vocabulary 猜测性冻结（词表须待 OQ-1/2/4/6 裁决）
- GStreamer / 执行面（A2-7+）
- 本报告仅入 `reports/`，不创建 `probes/` 目录

## 6. 证据文件清单

| 文件 | 引用节/行 |
|---|---|
| docs/architecture/ARCHITECTURE_V0.2.md | §1.13 L69；§1.20 L138-155；§2.4 L302-323（L305/L312/L319/L320）；§3.1 L331-404；§3.7 L759-872；§3.8 L874-895；§3.9 L901-933；术语表 L852-858；决策 #5/#24/#29/#37/#43；L1884/L1908/L1916/L1922；DB L1480-1483 |
| docs/architecture/CLOCK_TIMECODE_CONTRACT.md | §1 Clock #147；§2 Timecode #148；§3 替换不变量 |
| services/media-agent/src/timecode.rs | L1-80（CanonicalTimecode 全结构，#148+Unknown 词表） |
| services/media-agent/src/normalize.rs | L95-105（CanonicalMediaDescriptor.timecode 消费点） |
| services/media-agent/src/runtime_state.rs | L100-112（D15 deferral 注释） |
| services/media-agent/src/program/{video_master,audio_master,switch_policy}.rs | 结构体定义（Q8 零占位证据） |
