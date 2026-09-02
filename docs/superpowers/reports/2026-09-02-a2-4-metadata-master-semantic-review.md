# A2-4-03 — MetadataMaster Semantic Deep Review（六组合语义矩阵 + 四问审查）

> Status: `SEMANTIC REVIEW ONLY / NO CODE CHANGE`
> Authority: 用户终裁（A2-4-02 全项通过; 03 = Semantic Deep Review, 禁改 .rs）
> Date: 2026-09-02 · Base: `7342cdd`
> 纪律：任何组合无明确业务含义 → **暂停上报，不靠测试合法化**。

---

## 0. 复查记录

`7342cdd` diff 边界实查：`payload/timestamp/scope/MetadataSourceId/ready/healthy/
stage/advance` 全部命中为**禁令注释与测试断言文本**，零实际字段/方法——
"未越界"判定复核通过。四文件 diff（+354/−8）与提交信息一致。

---

## 1. 语义基座（矩阵解释的前提，先行钉死）

- **`MetadataMaster` 是聚合快照声明**（swept, non-transactional——与 D14
  CanonicalRuntimeState 同一致性类），不是终局裁决；
- **`facts` = 已聚合的条级 canonical 证据**（每条声明"某 source 某类型
  metadata 的存在性"）；**`join_declaration` = 路级单方声明**（Metadata
  Domain 对 Master Join 说的话）；
- 三态互斥（单字段单值）：`Participating` = 参与声明（内容待续/可有）；
  `NotPresent` = **基于充分观测的结论性负声明**（参与性隐含在"已观测"中
  ——已观测即已参与观测过程，故不另设"参与+确认无"复合态）；
  `Unknown` = 观测不足以作出任何路级结论；
- **条级证据 ≠ 路级结论**：部分 fact 到位不等于路级声明可下（此即组合 6
  的语义基础）。

---

## 2. 六组合语义解释矩阵（用户指定交付）

| # | declaration | facts | 业务含义 | 判定 |
|---|---|---|---|---|
| 1 | `Participating` | `[]` | Metadata 路参与 Program Join，**本次聚合快照无 fact**（observation 未回流/暂无事件——瞬时态）。**快照语义消解歧义**（用户 §2 四解收束为一）：`[]` 恒指"本快照无"，"确认无"的规范表达是 `NotPresent`，二者不混同 | ✅ 自洽 |
| 2 | `NotPresent` | `[]` | 已充分观测、确认本 Program 无 metadata，聚合自然为空（纯净节目：无字幕无 SCTE-35） | ✅ 自洽 |
| 3 | `Unknown` | `[]` | 观测不足，无证据亦无结论（冷启动初始态 = `Default`/`new()`） | ✅ 自洽 |
| 4 | `Participating` | `[fact]` | 参与且有已聚合证据（常态） | ✅ 自洽 |
| 5 | `NotPresent` | `[fact]` | **分两型**：(5a) fact.presence ∈ {NotPresent, Unknown}——"有观测记录且记录为'无'/'未定'"，**自洽**（负 fact 本身就是证据）；(5b) 任一 fact.presence = Present——**证据说有、声明说无，语义矛盾** | ⚠️ **5b 矛盾 → TQ-1 待裁** |
| 6 | `Unknown` | `[fact]` | 条级证据部分到位，但整体不足以作路级结论（如 Caption 已观测、SCTE-35 未完成——观测进行中） | ✅ 自洽（条级≠路级） |

**结论：六组合中五组合半自洽；唯一张力 = 组合 5b**（`NotPresent` 声明 +
`presence: Present` fact 的跨字段矛盾）。按纪律不靠测试合法化，原样上报。

---

## 3. 四问审查（用户指定范围）

### Q-1 三态语义自洽性
三态覆盖"参与吗 / 有吗 / 知道吗"三问且单值互斥；`NotPresent` 的参与性
隐含已由 §1 精化（不再需要复合态）；`Participating+[]` 的四解歧义由**快照
语义**收束（§2 组合 1）。**自洽成立**，但精化结论须回写 Design §1.5（见
§5 落实清单 D1-D3）。

### Q-2 facts 与 declaration 完全正交性
结构性正交已由 SQ-4 测试锁定；本审查补**语义性禁推导清单**（须入 Design）：
禁 `facts.is_empty() → NotPresent`（结构推导冒充观测结论）；禁
`!facts.is_empty() → Participating`（证据存在不等于参与声明）；组合 5b 表明
**正交 ≠ 一致**——矛盾可能存在，定性见 TQ-1。

### Q-3 MetadataFact 是否隐形 payload container
字段逐一审查：`kind`（taxonomy 判别）/`source`（canonical 引用，非值拷贝）/
`presence`（存在性）。零值域、零时间、零内容载荷；JSON 键集恰三键测试已
锁蔓延路径。**结论：纯 canonical fact declaration，非容器**。

### Q-4 serde wire 与 A2-5 Master Join 消费边界兼容性
- A2-5 消费面 = Join 读 declaration + facts 作联合判定输入（V0.2 §1.20
  L155：**任一路 failed → DEGRADED/FAILOVER**）。wire 契约（三键 JSON/
  三态 SCREAMING_SNAKE/fail-closed）满足"Join 拿到可判定输入"；
- **A2-5 前瞻约束（须随 A2-5 设计带走）**：`Unknown ≠ failed`（观测不足
  不是故障——Join 侧保守消费，禁把 Unknown 直接升格为 DEGRADED 依据）；
  `Participating + []` 不阻断（空 ≠ failed）；
- 矛盾声明（组合 5b）在 A2-5 的处置属 TQ-1 裁决范围。

---

## 4. 待裁项（TQ）

### TQ-1 组合 5b 跨字段矛盾的处置（唯一上报项）

`facts 含 presence=Present 条目` 与 `join_declaration=NotPresent` 矛盾。
三个候选（**不预裁决**）：
- **案 A（推荐）**：结构层不禁止 + 语义文档定性"矛盾 = 生产者（未来
  A2-6 聚合侧）责任"；A2-5 Join 消费时对矛盾声明 fail-closed 拒绝
  （"结构承载事实，不裁决事实"——与 assemble 不校验业务矛盾同律）；
- **案 B**：A2-4-04/05 增加 `is_consistent()` 纯函数谓词（无状态、
  零执行——但属加代码，须用户批准后入 04）；
- **案 C**：收窄 `NotPresent` 定义为"由聚合器背书的结论，结构不承载
  背书义务"（仅文档收窄，代码零改）。

---

## 5. 落实清单（本轮已执行，零 .rs diff）

- **D1**（修用户 §2 指出的 Design 级问题）：Design §1.5 补 `Participating+[]`
  快照语义收束（四解歧义 → "本快照无"，确认无 = `NotPresent` 规范表达）；
- **D2**：Design §1.5 补 `NotPresent` 可观测语义锁死（Unknown≠NotPresent；
  禁结构推导清单两条）；
- **D3**：Design §1.5 补 `NotPresent` 参与性隐含说明 + 条级/路级区分；
- **D4**：A2-5 前瞻约束两条（Unknown≠failed / Participating+空不阻断）入
  Design，供 A2-5 设计时消费；
- **D5**：本报告 + tasks 更新；TQ-1 待用户裁决。

## 6. No-Build Gate 复认

本轮零 .rs diff；未加 stage/advance/payload/scope/Hash/serde(default)；
组合 5b 未被任何测试"合法化"（现有测试仅断言正交组合不被结构禁止，
未断言矛盾组合语义正确——两事有别的记录已在 SQ-4 测试注释）。
