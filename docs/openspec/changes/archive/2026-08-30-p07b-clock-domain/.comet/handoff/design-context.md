# Comet Design Handoff

- Change: p07b-clock-domain
- Phase: design
- Mode: compact
- Context hash: a3ea207fc31b6e1b8328185ee6ff4ba950f8a9090a8319e8f6b5cee685139f69

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07b-clock-domain/proposal.md

- Source: docs/openspec/changes/p07b-clock-domain/proposal.md
- Lines: 1-32
- SHA256: 343c3f2d2b556044cb0ea8d107b8181bdcce17e97671910041599e36b9375873

```md
# Change: Phase 0.7B-2A — p07b-clock-domain（Clock Domain 建模：只描述观测，绝不决策）

## Why

0.7A 的 `CanonicalClockRef { domain: Option<Uuid> }` 过弱——Clock Domain 的 kind/reference/置信/证据没有 canonical 表达。冻结契约 `CLOCK_TIMECODE_CONTRACT.md` §1（#147）已定义 Clock **观测态**词表（Locked/Unlocked/Offset/Drift/Clock Lost/Clock Recovered）且钦定 **Observation≠Configuration**（R3：Clock 是运行时观测，绝不写回 Graph）。0.7B-2A 把它落成 canonical 类型，为 Audio Routing（依赖 Clock）与未来 Execution Plan 提供语义地基。**只描述"观察到什么"，绝不决定"应该使用什么 clock"**（那是 Runtime/Backend/Control Plane 的责任）。

## What Changes

- **`src/clock.rs`（新，canonical 层，零 vendor 依赖）**：
  - `CanonicalClockDomain { id: Uuid, kind, reference, state, confidence, evidence }`：
    - `kind: ClockKind { Internal, External, Unknown }`
    - `reference: ClockReference { FreeRunning, Locked, Unknown }`
    - `state: ClockObservationState { Locked, Unlocked, Offset, Drift, ClockLost, ClockRecovered, Unknown }`（**对齐冻结词表 #147**；Unknown 为 0.7B-2A 观测前置态）
    - `confidence: ClockConfidence { Observed, Inferred, Unknown }`
    - `evidence: Vec<ClockEvidence { code, detail }>`（观测证据条目，可序列化）
  - serde 全可序列化；`PartialEq`；**无任何决策方法**（类型层面不存在 `choose_master_clock` 之类）。
- **门禁 MEDIA-SEMANTICS-RT-01（Clock 部分，三层）**：
  - Unit：词表完备性（冻结 #147 六态全部可表达）/ serde roundtrip / Observation≠Configuration（类型层面无写回路径）/ 未知 kind/reference/state 的合法表达。
  - Simulation：MockProvider 世界装配 `CanonicalClockDomain`（Unknown kind + evidence 记录"无 clock 探针"）。
  - Hardware：盒上真机观测装配证据（当前硬件无 clock 探针 → Unknown kind + evidence，**Unknown 合法**——终审明确）。
- **`normalize.rs` 联动（最小）**：`CanonicalClockRef` 增补 `domain_description: Option<Box<CanonicalClockDomain>>`（引用升级为可携带观测描述；默认 None，normalize 恒 None + 既有 INFO 诊断不变——**不接 runtime，不接 clock 探针**）。
- **CI**：测试并入现有矩阵，无新 job。

## Capabilities

（`skip_specs: true`——SoT 为 `CLOCK_TIMECODE_CONTRACT.md`（#147 冻结词表）+ 终审裁定形状。）

## Impact

- 编译：五套 feature 不回退；零 vendor 依赖。
- 受影响：新 `clock.rs`；`normalize.rs`（CanonicalClockRef 最小增补）；`main.rs` mod 声明。Session/Resource/Lease/Pipeline 零触碰。
- 明确不做：不实现 clock 探针/源选择/master 仲裁/PTP/Genlock/恢复算法；不接 GStreamer/ALSA；不改 Session 语义；不做 Timecode（0.7B-2C）。

```

## docs/openspec/changes/p07b-clock-domain/design.md

- Source: docs/openspec/changes/p07b-clock-domain/design.md
- Lines: 1-36
- SHA256: e80907dff484996ce0956b5c104689281f4a6b578b0294343aedb1a9f32e289c

```md
# Design: Phase 0.7B-2A — p07b-clock-domain（Clock Domain 建模）

## Context

冻结契约 `CLOCK_TIMECODE_CONTRACT.md` §1（#147）：Clock 观测态词表 `Locked/Unlocked/Offset/Drift/Clock Lost/Clock Recovered`；**Observation≠Configuration**（R3：Clock 观测绝不写回 Graph）。0.7A 遗留 `CanonicalClockRef { domain: Option<Uuid> }` 过弱。终审裁定形状：`CanonicalClockDomain { id, kind, reference, confidence, evidence }`——只描述观测，类型层面不存在任何决策方法。

## Goals / Non-Goals

**Goals:** canonical Clock Domain 类型族（kind/reference/state/confidence/evidence）+ serde + 单测；`CanonicalClockRef` 最小增补（可携带 domain 描述）；MEDIA-SEMANTICS-RT-01 Clock 部分三层。
**Non-Goals:** clock 探针/源选择/master 仲裁/PTP/Genlock/恢复算法；GStreamer/ALSA 接线；Session 语义变更；Timecode（0.7B-2C）；Audio Routing（0.7B-2B）。

## Decisions

- **D1 形状（终审裁定 + #147 词表合并）**：
  ```rust
  pub struct CanonicalClockDomain {
      pub id: Uuid,
      pub kind: ClockKind,            // Internal | External | Unknown
      pub reference: ClockReference,  // FreeRunning | Locked | Unknown
      pub state: ClockObservationState, // 冻结 #147: Locked/Unlocked/Offset/Drift/ClockLost/ClockRecovered + Unknown(观测前置)
      pub confidence: ClockConfidence, // Observed | Inferred | Unknown
      pub evidence: Vec<ClockEvidence>, // { code: String, detail: String }
  }
  ```
  `state` 采用 #147 冻结词表（终审形状未列，系契约对齐的**加法**；`Unknown` 表"0.7B-2A 尚无 clock 探针"——真机 Unknown 合法，终审明确）。
- **D2 无决策（终审红线）**：类型族**零方法**（除派生 trait）；`choose_master_clock`/`select_clock`/`auto_route` 在本模块类型层面不存在。Clock 策略属 Runtime/Backend/Control Plane。
- **D3 Observation≠Configuration**：无任何 `-> GraphIntent` / 写回路径；evidence 只增不解释（Policy 层职责）。
- **D4 `CanonicalClockRef` 增补**：`domain_description: Option<Box<CanonicalClockDomain>>`（Box 防递归膨胀；默认 None → 既有 normalize 行为与诊断不变——`normalize_input` 恒 None + INFO，**不接探针**）。
- **D5 Confidence 语义**：`Observed`（有直接探针证据）/`Inferred`（从 transport/拓扑推断）/`Unknown`——0.7B-2A 只会产出 Unknown（无探针），枚举为 0.7B 探针阶段预留。
- **D6 门禁 MEDIA-SEMANTICS-RT-01（Clock 部分）**：Unit（#147 六态+Unknown 全可表达 / serde roundtrip / 无决策方法的存在性断言——编译期类型核查+白盒）/ Simulation（MockProvider 世界装配 Unknown domain + evidence）/ Hardware（盒上装配证据输出：Unknown kind + evidence "无 clock 探针"——**Unknown 合法**）。

## Risks / Trade-offs

- `state` 字段为终审形状的加法（#147 对齐）：若 0.7B 探针阶段发现词表不够，只能**加变体**不能改义（冻结词表约束）。
- `evidence` 用字符串对而非强类型：0.7B-2A 无探针 schema 依据，强类型留探针阶段（演进空间，登记债务不必要——枚举化属探针实现细节）。
- `CanonicalClockRef` 增字段 → normalize 既有断言（`clock == CanonicalClockRef{domain:None}`）需同步更新（0.7B-1 测试为编译+断言级小改）。

```

## docs/openspec/changes/p07b-clock-domain/tasks.md

- Source: docs/openspec/changes/p07b-clock-domain/tasks.md
- Lines: 1-31
- SHA256: 56011aded15fbbb073e07deb5ba17fc3cc59124a174a9d796ee9ed7d3afef9d3

```md
# Tasks: Phase 0.7B-2A — p07b-clock-domain

> 纪律（§十五）：每项标注 `Contract / Implementation / Verification / Gate`。

## 1. clock.rs 类型族（canonical 层，零 vendor 依赖）

- [ ] 1.1 `CanonicalClockDomain` / `ClockKind` / `ClockReference` / `ClockObservationState`(#147 词表+Unknown) / `ClockConfidence` / `ClockEvidence` + serde
  - Contract: CLOCK_TIMECODE_CONTRACT §1(#147 词表; Observation≠Configuration) + 终审裁定形状 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 1.2 零决策红线: 类型族无任何选择/仲裁方法; 无 Graph 写回路径
  - Contract: CLOCK_TIMECODE_CONTRACT §1 (Observation≠Configuration, R3); 终审红线 (禁 choose_master_clock/select_clock/auto_route) | Implementation: Not Started | Verification: Test(编译期存在性白盒) | Gate: Pending

## 2. normalize.rs 联动（最小）

- [ ] 2.1 `CanonicalClockRef.domain_description: Option<Box<CanonicalClockDomain>>`（默认 None；normalize 恒 None + 既有 INFO 诊断不变；0.7B-1 测试同步）
  - Contract: CANONICAL_MEDIA_MODEL §2 (Clock 属 Media 实体关系) | Implementation: Not Started | Verification: Test(0.7B-1 测试不回退) | Gate: Pending

## 3. 门禁 MEDIA-SEMANTICS-RT-01（Clock 部分，三层）

- [ ] 3.1 Unit: #147 六态+Unknown 全可表达 / serde roundtrip / 白名单语义
  - Contract: #147 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 3.2 Simulation: MockProvider 世界装配 Unknown domain + evidence 记录"无 clock 探针"
  - Contract: Observation≠Configuration | Implementation: Not Started | Verification: Simulation | Gate: Pending
- [ ] 3.3 Hardware: 盒上装配证据输出（Unknown kind + evidence——Unknown 合法）
  - Contract: 终审"Unknown 合法" | Implementation: Not Started | Verification: Hardware | Gate: Pending

## 4. 交付

- [ ] 4.1 盒上全矩阵 + CI 七 checks 不回退（首提交仅类型+serde+单测，不接 runtime）
  - Contract: 终审"第一提交只允许新类型/serde/unit test/canonical contract" | Implementation: Not Started | Verification: Box+CI | Gate: Pending
- [ ] 4.2 verify（full）→ archive → PR#4 → merge → 删分支
  - Contract: 分支纪律 | Implementation: Not Started | Verification: CI+Review | Gate: Pending

```
