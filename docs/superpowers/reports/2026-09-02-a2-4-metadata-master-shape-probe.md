# A2-4-02-00 — Metadata Master Domain Shape Probe

> Status: `PROBE ONLY / NO CODE CHANGE`
> Authority: 用户终裁（A2-4-02 = APPROVED TO PROBE, NOT YET APPROVED TO CODE）
> Date: 2026-09-02 · Base: `b8dbf88`（A2-4-01 词表已 CLOSED）
> 任务：代码级字段来源调查（10 项必查）+ Candidate A/B/C 证据对比 → 交用户字段级终裁

---

## 0. 本轮裁决复认（12 条 NO-CODE 冻结清单）

NO MetadataStage / NO advance / NO transition matrix / NO duplicated CanonicalTimecode /
NO metadata payload God Object / NO five-field-by-taxonomy assumption / NO generic
timestamp invention / NO new source identity unless现有不满足 / NO forced data_plane
symmetry / NO `joined: bool` / NO VideoMaster modification / NO AudioMaster modification。
**本 probe 零 .rs diff，清单完好。** 新增原则复认：`taxonomy ≠ object field schema`
（01 冻结的 taxonomy enum 是 discriminator，不是 struct 字段选择器）；
`joined ≠ readiness ≠ health ≠ publication`。

---

## 1. 十项必查证据（P01-P10）

### P01 — canonical metadata payload/fact 类型现状
全库 grep（Scte/Caption/Klv/metadata payload）：**零 payload、零 fact 结构**。唯一
存在 = A2-4-01 taxonomy（`MetadataType`/`MetadataDataPlane`，纯判别式）。
→ 任何"typed payload 字段"方案（Candidate A）在当前仓库**无类型可填**——
`Option<KlvMetadata>` 等需从零发明 payload 类型，违反"缺口原样上报"。

### P02 — source/input identity 正式类型（关键正面证据）
- **`CanonicalSourceRef { device_id: Uuid, port_id: Option<Uuid> }`**（normalize.rs
  L89-92）：Copy + Eq + serde，已服役（CanonicalMediaDescriptor.source 全链消费）。
  **成熟 canonical source reference 已存在**——fact 的 source 维度零新建。
- `SessionInput { device_id, handle }`（session.rs L193）：Alpha-1 输入句柄表。
- identity 家族全 `Uuid`（device/port/resource/clock-domain），**无 wrapper 类型**
  （grep `pub struct DeviceId` 零命中）。
→ **不新建 `MetadataSourceId`**（§十二裁决"除非现有不满足"——现有满足）。

### P03 — channel/program identity 类型
零 `ProgramId`/`Channel` identity 类型。Channel = 控制台侧显示规约（CH+序，
状态不携带序——Alpha-1 裁定）；program scope 当前唯一载体 = 会话/runtime_state
顶层。→ A2-4-02 **不需要** program identity 字段（Program-scope 由对象所在的
聚合层表达，V0.2 未定义 program_id 概念）。

### P04 — metadata/timecode 的 scope 表达现状
无显式 scope 类型。唯一 scope 信号 = 结构位置：`CanonicalMediaDescriptor` =
per-input（经 `PortMediaSemantics{port_id, descriptor}` 挂 port）；顶层
CanonicalRuntimeState = session 级。→ scope 三层（L1/L2/L3）当前靠**归属结构**
非字段表达——02 设计应沿用"结构即 scope"，不发明 scope enum。

### P05 — event/segment/time semantics（§十三裁决的实锚）
- `events.rs` 时间字段计数 = **0**：RuntimeEvent 零 timestamp（D14 已登记
  EVENT_CONTRACT 偏差 B：文档声称有 timestamp 而 struct 无）。
- SCTE-35 事件性/ Caption 连续性：代码零实现。
- Timecode ≠ presentation timestamp ≠ wall clock ≠ monotonic（timecode.rs 概念
  隔离已锁）。
→ **generic timestamp 确不存在**——MetadataFact 不带时间字段（时间语义属未来
payload contract / A2-5 AV Sync 域）。

### P06 — CanonicalMediaDescriptor 携带 Timecode 的方式
六字段之一：`timecode: CanonicalTimecode`（normalize.rs L104），normalize 恒
`unknown()`（无观测源绝不臆造）。**组合非复制**先例——descriptor 引用
Observation 域产物，不复制其状态机。→ MetadataMaster 若引 timecode fact，
同律：**引用/汇聚，绝无第二 SoT**。

### P07 — generic metadata container 是否已存在
无（P01 复证 Q8 结论）。→ 无既有容器可复用，也无容器需要迁移。

### P08 — MetadataDataPlane 是否应成为 MetadataMaster 字段（§十五待审项）
- Video/Audio 存 `data_plane` 字段的**证据理由**：Errata-3 是 V0.2 反复锁定的
  纠偏史边界（"Master 一定 RAW / 禁压缩域 Master"），字段化使其可测试
  （wire 锁 + 拒收压缩域串的测试存在）。
- Metadata 路：§3.7 图平面唯一（METADATA），**无同等纠偏史**；类型身份
  （`MetadataDataPlane` enum，01 已冻结）与对象名已双重锁定平面。
- 两案并存待裁：(a) 不入字段——拒绝为对称而对称；(b) 入字段作 wire 一致性锚
  （随 Master 序列化时平面自证）。**本 probe 不预裁决**。

### P09 — Option / Unknown / Absent 现有惯例（§十四四分法的仓库先例）
- **观测状态 = enum（含 Unknown）**：`TimecodePresence`（六值）、
  `AudioPresence::{Present{channels_hint: Option<u32>}, NotPresent, Unknown}`
  ——Present 内嵌 Option 的"enum 状态 + Option 细节"复合先例。
- **细节值 = Option**：`CanonicalVideoDescription` 全 Option、
  `CanonicalClockRef.domain: Option<Uuid>`、`AudioMaster.delay_ms:
  Option<NonZeroU16>`。
- 无裸 `Option<bool>`/双态裸 bool 表状态的反例。
→ 02/03 字段设计沿用：**presence 语义用 enum，数值/引用细节用 Option**；
四分法（Option/Enum 状态/Unknown/Absent）仓库已有全部先例支撑。

### P10 — A2-5 Master Join 已有/预留接口
零代码预留。唯一前瞻锚 = video_master.rs 注释"switch_policy 声明在 A2-5 join
时接入" + program/mod.rs 路线图注释。→ A2-5 对 02 字段**无既有约束**，仅
§十七概念图（readiness/declaration/AV Sync/cross-domain 四件在 Join 层）——
即 02 只需产出"可被 Join 消费的 declaration"，不需实现 Join。

---

## 2. Candidate A/B/C 证据对比

| 维度 | A: Typed fixed fields | **B: Fact Set + Join Declaration** | C: Join References |
|---|---|---|---|
| 依托的已有类型 | 无（P01 零 payload——需发明 KlvMetadata 等 5 个） | `MetadataType`(01) + `CanonicalSourceRef`(P02) + enum 惯例(P09) 全就绪 | `CanonicalSourceRef`(P02) 就绪 |
| taxonomy≠field-schema | ❌ 五值直译五字段（或三字段漏 KLV/SYSTEM） | ✅ MetadataType 作 discriminator | ✅ 不涉及 |
| 多源/多实例（SCTE-35 事件性、双输入 timecode） | ❌ 单字段单实例，改 Vec 即破格式 | ✅ `Vec<MetadataFact>` 天然吸收 | ⚠️ 有 source 维度但丢 type/presence fact |
| Timecode 第二 SoT 风险 | ❌ `timecode: Option<...>` 必然复制状态 | ✅ fact 引用/汇聚，P06 组合先例 | ✅ 同 B |
| "有什么/参与什么/Join 结果"三问（§六） | ⚠️ 只答"有什么" | ✅ facts 答前两问 + declaration 答第三问 | ⚠️ 只答"从哪来" |
| God Object 风险 | ❌ 高（payload 入 Master） | ✅ payload 留 canonical contracts（§十） | ✅ 低 |
| 与 SessionInput 重复 | — | — | ⚠️ 纯引用退化成输入清单 |
| 02 之后的扩展面 | 破坏性（格式演进=改字段） | 加法（新 fact 类型/新 declaration 语义） | 加法但表达力不足 |

**结论（供裁决）：Candidate B 为基座，且天然吸收 C 的 source 维度**
（`MetadataFact` 内嵌 `CanonicalSourceRef`，即 B ⊃ C 的表达力）；A 被仓库证据
否决（P01 无类型可填 + §九/§十结构性缺陷）。

**B 的概念骨架（不批准、仅示意——字段细节属 02 编码前逐项证明）**：
```
MetadataMaster
 ├── facts: Vec<MetadataFact>          // 判别式=MetadataType; source=CanonicalSourceRef;
 │                                      // presence=enum(P09 惯例); 无时间字段(P05); 无 payload(P01)
 └── join declaration                   // 非裸 bool(§七); 形态待 02 设计
```

---

## 3. Open Questions（字段级，交用户终裁）

| # | 问题 | 证据 | 倾向（非裁决） |
|---|---|---|---|
| SQ-1 | `MetadataFact.presence` 词表形态：复用式新词表（如 presence enum per-fact）vs 通用三态（Present/Absent/Unknown 对齐 AudioPresence）vs #148 分型 | P09 先例三种并存 | 02 设计给逐词表证明 |
| SQ-2 | join declaration 形态：enum（如 NotJoined/Ready/Joined+语义化变体）vs 结构体 | §七 joined≠readiness≠health≠publication；P09 enum 惯例 | enum 优先，词表 02 提案 |
| SQ-3 | `MetadataDataPlane` 入不入 MetadataMaster 字段 | P08 两案 | 待裁（(a) 不入 / (b) wire 锚） |
| SQ-4 | facts 空 Vec 语义：合法（无 metadata 节目）vs 需 declaration 区分 | V0.2 无"必须有 metadata"规定 | 空 Vec 合法 + declaration 表 Join 状态 |
| SQ-5 | fact 是否携带 canonical payload 引用（当前零 payload，P01） | P01/§十 | 02 不带；payload 属未来 contract |

## 4. No-Build Gate（本刀产物）

仅本报告 + tasks 勾选。零 .rs diff；12 条 NO-CODE 清单完好；SQ-1..SQ-5 未裁
决前不写 MetadataMaster/MetadataFact/join declaration 任何结构。

## 5. 证据文件清单

services/media-agent/src: normalize.rs L28-105（descriptor 家族+CanonicalSourceRef+timecode 携带）·
session.rs L191-197（SessionInput）· events.rs（时间字段计数 0；RuntimeEventLog L200-205）·
timecode.rs L23-71（#148 词表+Option 细节）· program/metadata_master.rs（01 词表）·
program/{video,audio}_master.rs（data_plane 字段先例+零 diff 确认）；
ARCHITECTURE_V0.2.md §3.7/§1.20/§3.8（A2-5 概念约束）。
