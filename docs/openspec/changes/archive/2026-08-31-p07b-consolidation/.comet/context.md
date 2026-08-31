# Comet Design Handoff

- Change: p07b-consolidation
- Phase: design
- Mode: compact
- Context hash: 2fe99fde558d82a69e03be6022f9e3d242316cf6b6c190e20fd22b06ef4b44e9

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07b-consolidation/proposal.md

- Source: docs/openspec/changes/p07b-consolidation/proposal.md
- Lines: 1-28
- SHA256: cb1c7e1da0edaf37470fec5bc0a17d13636b402ffe2eaedb9c671e38755f4f1a

```md
# Change: Phase 0.7B Consolidation — p07b-consolidation（文档对账 + Phase Map 冻结 + Canonical Integration Audit + 债务重分类）

## Why

0.7B-2C 已合并（master `c574238`，PR#1-#6 全 merged，六个 baseline tag 完整），但**文档 SoT 与实际实现发生漂移**（终审 P0）：`PHASE_0_6_MASTER_PRD.md` 仍标 CLOCK_TIMECODE/AUDIO_ROUTING = NOT_STARTED 且沿用旧 0.7A-G 标签；`README.md` 仍述"Phase 0.6 实施中/Gate NOT PASSED"；`ROADMAP.md` 仍把 Phase 0.6 标为 **Next**；Gap/Acceptance Matrix 的 SPI/Session/Resource 行停在 NOT_STARTED、ARCH-PORTABILITY-01 停在 FAIL。同时需要回答终审核心问题：**Canonical Model 是否已成为 Runtime 统一输入契约，还是有旁路**——本次做代码级 Integration Audit 给出权威答案。**本 change 零代码改动（src/ 不触碰）。**

## What Changes

- **`docs/architecture/PHASE_IMPLEMENTATION_MAP.md`（新）**：冻结 Implementation Phase Map（0.6 Runtime Abstraction / 0.7A Session Runtime / 0.7B Media Semantics(1/2A/2B/2C) / 0.7C External Integration / 0.7D Event Projection / 0.8 Federation），声明"Implementation Roadmap，不改变冻结 Architecture Contract"；收录 **0.7 全阶段最高架构红线**（Observation≠Configuration / Semantic Intent≠Execution Plan / Canonical 不绑回 Vendor）。
- **文档对账（只修状态与路线，不修改冻结语义/原文，历史口径以就地注记保留）**：
  - `PHASE_0_6_MASTER_PRD.md`：契约状态行（AUDIO_ROUTING/CLOCK_TIMECODE → PARTIAL 等）+ 旧 0.7A-G 段加"已被 PHASE_IMPLEMENTATION_MAP 取代"注记。
  - `README.md`：阶段导航更新为当前实态（0.6 完成 + 0.7A/0.7B 完成态 + 入口指向 Phase Map）。
  - `ROADMAP.md`：状态总览重构为三段式（Historical Architecture / Current Implementation（指向 Phase Map）/ Future Product），Phase 0.6 ✅。
  - `PHASE_0_6_IMPLEMENTATION_GAP_MATRIX.md`：SPI/Session/Resource/Preflight/Registry/Lint 等行 IMPLEMENTATION 列与 Gate 列对齐实态。
  - `PHASE_0_6_ACCEPTANCE_MATRIX.md`：ARCH-PORTABILITY-01 FAIL→PASS（CI 六/七 gate + remove-adapter proof）、补充 0.7A/0.7B 门禁行（SESSION-RT-01/RESOURCE-RT-01/NORMALIZE-RT-01/MEDIA-SEMANTICS-RT-01/AUDIO-SEMANTICS-RT-01/TIMECODE-SEMANTICS-RT-01）。
- **Integration Audit 报告**（`docs/superpowers/reports/2026-08-30-p07b-consolidation-integration-audit.md`）：三区审计结论（Provider→Canonical 统一 / Canonical→Runtime Intent 旁路 / Canonical→Backend 直连）+ 结构性事实（Canonical 与 Runtime 目前为不相交子图，红线平凡满足；待补 Canonical→Runtime/Policy 边）+ P2 观察（Mock 无生产 canonical 路径，unification 由测试证明）。
- **债务重分类**（`PHASE_0_7A_POST_MERGE_DEBT.md`）：0.7C 前必须（D2 derive_claims FAIL 化 / D4 PortAvailability 精确化 / D5 IdentityBinding 实查 / D6 BACKEND-CAPABILITY-01）vs 可延后（D1/D3/D7/D9/D10）；D11 优先级上调说明；**D13 新登记**（`observe_transitional` 的 debug_assert release 不强制 → Result 化，P1 semantic hardening）。
- **0.7C 前置顺序声明**（Phase Map 内）：Canonical Runtime State → Runtime Query Model → Command Contract → Idempotency → Error Model → Event Projection → External API（不直接做 REST）。

## Capabilities

（`skip_specs: true`——对账与地图文档，无新能力。）

## Impact

- **零代码改动**：`services/media-agent/` 不触碰；CI 七 checks 于 PR 实跑即全矩阵证明（盒上矩阵无代码可验，引用 master 基线 134/134/154/134）。
- 受影响：`docs/architecture/{PHASE_IMPLEMENTATION_MAP(新), PHASE_0_6_MASTER_PRD, README, PHASE_0_6_IMPLEMENTATION_GAP_MATRIX, PHASE_0_6_ACCEPTANCE_MATRIX, PHASE_0_7A_POST_MERGE_DEBT}`、`ROADMAP.md`、新 audit 报告。
- 明确不做：不改任何冻结契约语义/V0.2；不写功能代码；不提前做 0.7C。

```

## docs/openspec/changes/p07b-consolidation/design.md

- Source: docs/openspec/changes/p07b-consolidation/design.md
- Lines: 1-27
- SHA256: 588fd1a20d5e413149899d12bbdd884602d968508d712adba9c51e8f4f8cd7fe

```md
# Design: Phase 0.7B Consolidation — p07b-consolidation

## Context

终审确认：代码主线健康（PR#1-#6 全 merged、六 tag 完整、0.7B 四基础齐备），但文档 SoT 漂移为 P0。Integration Audit 已完成（代码级，三区全 CLEAN）。DOCUMENT_STATUS_MODEL 的铁律是"文档存在≠代码实现；实现进度 SoT=Gap Matrix"——本次把该铁律反向应用：**实现前进后，状态 SoT 必须回写**。

## Goals / Non-Goals

**Goals:** Phase Map 冻结 + 五文档状态对账 + Audit 报告落盘 + 债务重分类 + 0.7C 前置顺序声明。
**Non-Goals:** 改冻结语义/V0.2；写代码；做 0.7C；清偿债务（只分类）。

## Decisions

- **D1 PHASE_IMPLEMENTATION_MAP.md 为唯一 Implementation 路线 SoT**：旧 PRD §5 的 0.7A-G 标签**保留原文 + 就地注记被取代**（同 P0-6 勘误先例：历史可追溯，不删改）；新地图为活文档（状态列随阶段推进更新）。
- **D2 对账只动状态字段**：PRD 契约表 / Gap Matrix IMPLEMENTATION+Gate 列 / Acceptance Matrix 结果列 / README 阶段导航 / ROADMAP 状态总览——均为状态词（NOT_STARTED→PARTIAL/IMPLEMENTED；FAIL→PASS），不触碰需求语义行。
- **D3 ROADMAP 三段式**：`Historical Architecture Roadmap`（Phase 0/0.5，LOCK FINAL 不动）/ `Current Implementation Roadmap`（0.6→0.8，指向 Phase Map，本段仅存概要+链接防双源）/ `Future Product Roadmap`（Phase 1-5/V0.3-V1.0 原文保留）。单一事实源=Phase Map；ROADMAP 不再复制细节。
- **D4 Acceptance Matrix 增补 0.7 门禁行**：SESSION-RT-01/RESOURCE-RT-01（Hardware PASS，0.7A）、NORMALIZE-RT-01（三层 PASS，0.7B-1）、MEDIA-SEMANTICS-RT-01 Clock（0.7B-2A）、AUDIO-SEMANTICS-RT-01（0.7B-2B）、TIMECODE-SEMANTICS-RT-01（0.7B-2C）——证据列指向各 verify 报告。
- **D5 Audit 报告落 reports/ 而非 architecture/**：architecture/ 只放契约与地图；审计是时点快照证据，归 `docs/superpowers/reports/`（comet 惯例），Phase Map 引用其结论。
- **D6 债务重分类标注**（不改 D 编号，加"优先级"列分组）：0.7C 前必须 = D2/D4/D5/D6（External API 依赖：资源解析 fail 硬化/端口精确化/身份绑定实查/后端能力门禁）；延后 = D1 LifecycleJournal/D3 per-claim TTL/D7 OnceLock/D9 幂等键/D10 多 Pipeline；D11 Clock Timeline 优先级上调（广播时钟天然是时间序列）；D8 EventSink 与 0.7D Event Projection 合并考虑。**D13 新登记**：timecode `observe_transitional` debug_assert（release 不强制）→ 后续 Result 化（P1 semantic hardening，不单独开 change，随 timecode 下一触碰点处理）。
- **D7 0.7C 前置顺序写死在 Phase Map**：Canonical Runtime State → Runtime Query Model → Command Contract → Idempotency → Error Model → Event Projection → External API。
- **D8 验证口径**：零代码改动 → 盒上矩阵引用 master 基线（134/134/154/134）；PR CI 七 checks 实跑为 CI 层证明；`python scripts/check_docs.py` 跑通（文档链接/数字口径校验，若规则覆盖新增文件）。

## Risks / Trade-offs

- ROADMAP 概要段与 Phase Map 存在双源风险 → D3 以"仅链接不复制"化解。
- Gap Matrix 行多、手改易漏 → 用精确锚点批量替换 + 逐行 grep 复核。
- PRD 状态行修改可能被误读为"解冻" → 每处注记显式写明"CONTRACT 仍 FROZEN；仅 IMPLEMENTATION 状态对账"。

```

## docs/openspec/changes/p07b-consolidation/tasks.md

- Source: docs/openspec/changes/p07b-consolidation/tasks.md
- Lines: 1-35
- SHA256: bbd451d6a1bf1a93c90cb41371e78ff52c9aba776667692e59c2a1def2f4b6c2

```md
# Tasks: Phase 0.7B Consolidation — p07b-consolidation

> 纪律（§十五）：每项标注 `Contract / Implementation / Verification / Gate`。本 change 零代码改动。

## 1. Phase Map 冻结

- [ ] 1.1 新建 `PHASE_IMPLEMENTATION_MAP.md`（0.6→0.8 阶段表 + 各阶段状态/基线 tag + 0.7 最高架构红线 + 0.7C 前置顺序）
  - Contract: 终审 §十三/§十七 | Implementation: Not Started | Verification: Docs | Gate: Pending

## 2. 文档对账（只修状态，不改冻结语义）

- [ ] 2.1 `PHASE_0_6_MASTER_PRD.md`：契约状态行（AUDIO_ROUTING/CLOCK_TIMECODE→PARTIAL 等）+ 旧 0.7A-G 段取代注记
  - Contract: DOCUMENT_STATUS_MODEL（状态 SoT=实现实态） | Implementation: Not Started | Verification: Docs | Gate: Pending
- [ ] 2.2 `README.md` 阶段导航更新（0.6/0.7A/0.7B 完成态 + 指向 Phase Map）
  - Contract: 同上 | Implementation: Not Started | Verification: Docs | Gate: Pending
- [ ] 2.3 `ROADMAP.md` 三段式重构（Historical/Current(链接 Phase Map)/Future）
  - Contract: 终审 §四 | Implementation: Not Started | Verification: Docs | Gate: Pending
- [ ] 2.4 `PHASE_0_6_IMPLEMENTATION_GAP_MATRIX.md`：SPI/Session/Resource/Preflight/Registry/Lint/RuntimeEvent 行状态对齐
  - Contract: 同上 | Implementation: Not Started | Verification: Docs+grep 复核 | Gate: Pending
- [ ] 2.5 `PHASE_0_6_ACCEPTANCE_MATRIX.md`：ARCH-PORTABILITY-01 FAIL→PASS + 0.7 门禁行增补（六个 RT-01）
  - Contract: 同上 | Implementation: Not Started | Verification: Docs | Gate: Pending

## 3. Integration Audit 与债务重分类

- [ ] 3.1 Audit 报告落盘（三区 CLEAN 结论 + 结构性事实 + P2 观察）
  - Contract: 终审 §七 | Implementation: Not Started | Verification: 报告 | Gate: Pending
- [ ] 3.2 `PHASE_0_7A_POST_MERGE_DEBT.md` 重分类（0.7C 前必须 D2/D4/D5/D6；延后组；D11 上调；D13 登记）
  - Contract: 终审 §八/§十二 | Implementation: Not Started | Verification: Docs | Gate: Pending

## 4. 交付

- [ ] 4.1 `python scripts/check_docs.py` 通过 + 零代码改动核验（src/ 零触碰）+ PR CI 七 checks
  - Contract: 盒上绿≠CI绿（本 change 以 CI 为准，盒上矩阵引用 master 基线） | Implementation: Not Started | Verification: CI | Gate: Pending
- [ ] 4.2 verify（full）→ archive → PR#7 → merge → 删分支
  - Contract: 分支纪律 | Implementation: Not Started | Verification: CI+Review | Gate: Pending

```
