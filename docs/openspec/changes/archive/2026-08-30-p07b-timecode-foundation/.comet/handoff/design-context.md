# Comet Design Handoff

- Change: p07b-timecode-foundation
- Phase: design
- Mode: compact
- Context hash: 068ed7d50a310fba113a7f33d1d2b3b2dfb18569159fdac24a7dab41e3359d2e

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07b-timecode-foundation/proposal.md

- Source: docs/openspec/changes/p07b-timecode-foundation/proposal.md
- Lines: 1-34
- SHA256: c2f89eba686129293a894fb14e245e7d0b4e446dabe990581deeba278633812a

```md
# Change: Phase 0.7B-2C — p07b-timecode-foundation（Timecode Foundation：时间标签，非时间本体）

## Why

Canonical Media Model 三基础（Video✅/Audio✅/Clock✅）缺最后一块：Timecode。**Clock = 流速/同步参考；Timecode = 媒体帧携带的时间标签**——两者是不同概念，绝不因 Timecode 有 frame_rate 就与 Clock 的 rate 混同。冻结契约 `CLOCK_TIMECODE_CONTRACT.md` §2（#148）只冻结了状态词表 `Present/Absent/Invalid/Discontinuous/Recovered`，**没有冻结格式族**——按终审裁定：只实现状态与最小描述结构，**不做 LTC/VITC/ATC/SMPTE 解析器**。

## What Changes

- **`src/timecode.rs`（新，canonical 层，零 vendor 依赖）**：
  - `TimecodePresence { Present, Absent, Invalid, Discontinuous, Recovered, Unknown }`——#148 冻结五态 + Unknown 观测前置态（与 0.7B-2A Clock 同构；无观测源时恒 Unknown，真机合法）。
  - `TimecodeFormat { Ltc, Vitc, Embedded, Unknown }`——**只声明格式标签，不实现解析**（终审建议的最小格式族，只作 canonical 标签存在）。
  - `TimecodeValue { hours, minutes, seconds, frames }`（u32 四元组）——**只在 presence=Present 且有真实观测时携带**；无观测绝不臆造 `00:00:00:00`。
  - `CanonicalTimecode { presence, format, value: Option<TimecodeValue>, frame_rate: Option<(u32,u32)>（标签所属媒体帧率, 语义上≠Clock rate）, evidence: Vec<TimecodeEvidence> }`。
  - `CanonicalTimecode::unknown()` / `absent()` 构造器——Unknown/Absent 不臆造。
  - `observe_invalid(code, detail)` 构造路径——**Invalid 保留证据，绝不悄悄转成合法 Timecode**。
  - **零决策红线**：类型族零方法（构造器除外）——clock selection / master clock / drift correction / sync decision / resampling / timestamp correction 全部类型层面不存在（白盒 allowlist 测试，同 0.7B-2A 先例）；**Timecode 变更不可能影响 CanonicalClockDomain**（隔离测试：类型无互相引用路径）。
- **`normalize.rs` 联动（最小）**：`CanonicalMediaDescriptor` 增 `timecode: CanonicalTimecode` 平级字段（四基础齐备；normalize 恒 `unknown()`——无观测源）。既有测试同步（构造点+断言）。
- **`main.rs`（最小）**：`mod timecode;` + loopback 证据挂点（timecode 段真机装配输出）。
- **门禁 TIMECODE-SEMANTICS-RT-01（三层）**：
  - Unit：**词表快照**（#148 五态+Unknown serde 往返）；**Clock/Timecode 隔离**（公开面无 clock/master/sync 决策 API + Timecode 类型与 CanonicalClockDomain 无引用路径）；**Unknown/Absent 不臆造**（无观测 → Unknown/Absent，value=None，绝不 00:00:00:00）；**Invalid 保留证据**（observe_invalid → presence=Invalid + evidence，不转合法值）；**Discontinuous/Recovered 语义保持**（构造为观察事实，无"修正"路径）；Vendor independence（BMD/Mock 相同 canonical observation → 相同 CanonicalTimecode；serde 零 vendor 字样）。
  - Simulation：Mock observation → canonical timecode。
  - Hardware：真机 loopback timecode 段证据输出（Unknown 合法——只证明"能观察/描述"，不证明"能解析全部格式"）。
- **CI**：测试并入现有矩阵。

## Capabilities

（`skip_specs: true`——SoT 为 `CLOCK_TIMECODE_CONTRACT.md` §2（#148 冻结词表）+ 终审裁定。）

## Impact

- 编译：五套 feature 不回退；零 vendor 依赖。
- 受影响：新 `timecode.rs`；`normalize.rs`（descriptor 增 timecode 字段 + 既有测试同步）；`main.rs`（mod + loopback 挂点）。
- 明确不做：**Timecode parser（LTC/VITC/ATC/SMPTE 解析）**；帧号计算/PTS 推导/Clock 校正/Graph 修改/pipeline 启动；Clock 策略；不触碰 session/resource/lease/pipeline/backend 五文件。
- 后续：2C 合并后先做 **0.7B Media Semantics Consolidation Review**（终审裁定），通过后再进 0.7C External API。

```

## docs/openspec/changes/p07b-timecode-foundation/design.md

- Source: docs/openspec/changes/p07b-timecode-foundation/design.md
- Lines: 1-27
- SHA256: c4cc24728449cc74287c36266ace21603b6ec895388fd61f3aa7bcf725e07012

```md
# Design: Phase 0.7B-2C — p07b-timecode-foundation（Timecode Foundation）

## Context

冻结契约 `CLOCK_TIMECODE_CONTRACT.md` §2（#148）：Timecode 状态 `Present/Absent/Invalid/Discontinuous/Recovered`；§3 替换不变量（Clock/Timecode 源替换 GraphIntent 不变）。Gap Matrix 无 Timecode 实现项（纯新落地）。终审红线：Timecode 只描述"时间标签"——禁止 clock selection/master clock/drift correction/sync decision/resampling/timestamp correction/pipeline 引用；**不实现 parser**（Provider observation → Timecode observation → CanonicalTimecode 到此结束）；2C 合并后先 Consolidation Review 再 0.7C。

## Goals / Non-Goals

**Goals:** `CanonicalTimecode` + `TimecodePresence`（#148 词表+Unknown）+ `TimecodeFormat`（标签不解析）+ `TimecodeValue`（仅真实观测携带）+ 证据；TIMECODE-SEMANTICS-RT-01 三层；descriptor 四基础齐备。
**Non-Goals:** parser（LTC/VITC/ATC/SMPTE）；帧号计算/PTS 推导/Clock 校正；格式族扩充；Session/五文件触碰。

## Decisions

- **D1 presence 词表**：#148 五态 + `Unknown`（无观测源前置态，真机合法——与 0.7B-2A Clock 的 Unknown 同构处理；词表快照测试防静默增删）。
- **D2 格式族最小化**：`TimecodeFormat { Ltc, Vitc, Embedded, Unknown }`——终审建议的最小集，**只作 canonical 标签**；ATC/SMPTE 等格式族扩充留后续（不做擅自扩充）。
- **D3 value 防臆造**：`value: Option<TimecodeValue>`——仅 presence=Present 且有真实观测时 Some；`unknown()/absent()` 恒 None。**绝不**在无观测时生成 00:00:00:00（测试锁定）。
- **D4 Invalid 保证据**：`observe_invalid(code, detail)` → presence=Invalid + evidence——解析/观测异常**不得**悄悄转成合法 Timecode。
- **D5 Discontinuous/Recovered 是观察事实**：仅作为 presence 值存在（构造自观测），类型层无"修正/恢复动作"方法（Recovered ≠ 修复操作）。
- **D6 frame_rate 语义隔离**：`frame_rate: Option<(u32,u32)>` = 标签所属媒体的帧率（如 30000/1001 drop-frame 场景需要），**语义上 ≠ Clock 的 rate**——文档锁定 + 字段注释 + 隔离测试（Timecode 类型与 CanonicalClockDomain 零引用路径，serde 互不含对方字段）。
- **D7 零决策红线（白盒）**：公开面 allowlist 硬编码清单（构造器除外），防 clock/sync/resample/correct 类 API 静默进入——同 0.7B-2A 先例。
- **D8 normalize 联动**：`CanonicalMediaDescriptor` 增 `timecode: CanonicalTimecode` 平级字段（四基础齐备：video/audio/clock/timecode）；`normalize_input` 恒 `CanonicalTimecode::unknown()` + 既有诊断不变；0.7B-1/2A/2B 既有测试构造点同步（编译级小改）。

## Risks / Trade-offs

- descriptor 增字段 → 0.7B 系列既有测试构造点需同步（机械小改，风险低）。
- `TimecodeValue` 裸 u32 四元组无越界校验（23:59:59:xx 上界）：0.7B-2C 无解析器即无校验依据；校验属 parser 阶段（登记不必要——parser 本身就是后续阶段的显式范围）。
- Hardware 层只证明"能观察/描述"——READY 态无 timecode 观测 → Unknown 输出（与 0.7B-2A/2B 同边界）。

```

## docs/openspec/changes/p07b-timecode-foundation/tasks.md

- Source: docs/openspec/changes/p07b-timecode-foundation/tasks.md
- Lines: 1-31
- SHA256: e44fcac8b7c0ad2733284aba4faaa2a720b56756e1cf545cb9711cc01206dbda

```md
# Tasks: Phase 0.7B-2C — p07b-timecode-foundation

> 纪律（§十五）：每项标注 `Contract / Implementation / Verification / Gate`。

## 1. timecode.rs 类型族（canonical 层，零 vendor 依赖）

- [ ] 1.1 `TimecodePresence`(#148 五态+Unknown) / `TimecodeFormat`(标签不解析) / `TimecodeValue`(仅真实观测携带) / `TimecodeEvidence` + serde
  - Contract: CLOCK_TIMECODE_CONTRACT §2(#148 冻结词表) + 终审最小格式族 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 1.2 `CanonicalTimecode`（unknown()/absent() 不臆造; observe_invalid 保证据; Discontinuous/Recovered 为观察事实）+ 零决策红线（公开面 allowlist 白盒）
  - Contract: 终审红线 (禁 clock/sync/resample/correct; 不实现 parser) | Implementation: Not Started | Verification: Test | Gate: Pending

## 2. normalize.rs 联动（最小）

- [ ] 2.1 `CanonicalMediaDescriptor` 增 `timecode` 平级字段（四基础齐备; normalize 恒 unknown(); 既有测试同步）
  - Contract: CANONICAL_MEDIA_MODEL §2 (Media—Signal/MediaFormat Observed; Timecode P1 Contract) | Implementation: Not Started | Verification: Test(既有测试不回退) | Gate: Pending

## 3. 门禁 TIMECODE-SEMANTICS-RT-01（三层）

- [ ] 3.1 Unit: 词表快照 / Clock·Timecode 隔离（无决策 API + 无引用路径）/ Unknown·Absent 不臆造 / Invalid 保证据 / Discontinuous·Recovered 语义 / Vendor independence
  - Contract: #148 + 终审 6 项测试要求 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 3.2 Simulation: Mock observation → canonical timecode
  - Contract: 同上 | Implementation: Not Started | Verification: Simulation | Gate: Pending
- [ ] 3.3 Hardware: 真机 loopback timecode 段证据输出（Unknown 合法; 只证明"能观察/描述"）
  - Contract: 终审 Hardware 要求 | Implementation: Not Started | Verification: Hardware | Gate: Pending

## 4. 交付

- [ ] 4.1 禁改五文件核验 + 盒上全矩阵（fmt/test×4/clippy×4/build×3/PROOF）+ CI 七 checks 不回退
  - Contract: 盒上绿≠CI绿 铁律 | Implementation: Not Started | Verification: Box+CI | Gate: Pending
- [ ] 4.2 verify（full）→ archive → PR#6 → merge → 删分支 →（后续 Consolidation Review, 不直接进 0.7C）
  - Contract: 分支纪律 + 终审"2C 后先 Consolidation" | Implementation: Not Started | Verification: CI+Review | Gate: Pending

```
