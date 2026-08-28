# 架构决策日志 — Phase 0.6 Architecture Contract Coherence（Final Hardening）

> 一句话裁决：**先冻结四层契约，再过 ARCH-PORTABILITY-01 + ARCH-BACKEND-01 两门禁，通过后才把 BMD/GStreamer 降级为 Reference Adapter；此步完成前不进 Normalize、不进 BMD-specific 开发。**
> 详细载体：[`IMPLEMENTATION_ADDENDUM.md`](./IMPLEMENTATION_ADDENDUM.md)；讨论脉络：[`REVIEW_2026-08-28.md`](./REVIEW_2026-08-28.md)。

## 1. 决策上下文
- 输入 PRD 201 条混含四类（主架构/PRD/EngSpec/Acceptance），需逐条核对合理性。
- 约束：V0.2 LOCK FINAL，架构改动走 V0.3；当前 `services/media-agent` 应定位为 **Canonical Media Runtime**，BMD/GStreamer 仅是当前 Reference Implementation。

## 2. 201 条核对结论摘要
| 类别 | 结论 |
|---|---|
| 绝大多数功能条目 / 工程规格 | 成立（与已闭环硬件发现层一致） |
| 「Domain 语义全部已实现」 | 不成立（过头）—— V0.2 已定义但 Rust 层未落地 |
| 「201 条无一与 V0.2 冲突」 | 证据不足 → 严谨为「已知范围不冲突」，架构改动仍须 V0.3 |
| #20 Format「过度设计」 | 混淆冻结契约与完整实现；冻结契约必要 |

## 3. 二次评审 → 修正点
1. Domain 全实现过头（V0.2 语义已定义，Rust 实现未落地）。
2. 冲突证据不足，架构改动须 V0.3 拍板。
3. #20 冻结契约 ≠ 过度设计。
4. PRD 应拆 主架构/PRD/EngSpec/Acceptance 四类。
5. 「无消费方不建抽象」精确界：P0 冻结 / P1 定边界 / P2 暂缓。

## 4. 最终裁决要点
- **四层**：Canonical Domain → Runtime Contracts/SPI → Runtime Orchestration → Concrete Adapters。
- **P0**：Domain Boundary / HardwareProvider SPI / MediaBackend SPI / RuntimeBinding / Canonical Error-Event / Session Ownership(只冻结) / Portability Gate。
- **P0.5**：Resource Model（Capability/Capacity/Availability/Allocation/Reservation，对齐 V0.2 §3.11 Resource Vector，Resource≠Device）。
- **P1**：Clock / Timecode / Audio Backend-Routing / Capability Negotiation / Encoder / Gateway（只定 Contract）。
- **P2**：DB / Queue / ObjectStore / Auth / Deployment（连契约暂缓）。
- **门禁**：ARCH-PORTABILITY-01（删 BMD Provider 仍能编译——当前编译不过）、ARCH-BACKEND-01。
- **子阶段**：0.6A~0.6G（P0/P0.5）→ 0.7（P1：Audio/Clock/Timecode/Capability/Encoder/Gateway + External Integration）→ 0.8（P2 Multi-site/Federation）。

## 5. 影响范围
- 新增 6+1 契约文档（本目录）作为 Phase 0.6 门禁依据。
- Rust 实现层需消除 `main/resolver/signal/pipeline` 对 `decklink/gstreamer` 的直接依赖（当前编译不过，是 P0 缺口）。
- 不修改 `ARCHITECTURE_V0.2.md` 任何语义。

## 6. 关联索引
- 综合契约：`IMPLEMENTATION_ADDENDUM.md`
- 讨论留档：`REVIEW_2026-08-28.md`
- 契约门禁：`IMPLEMENTATION_BOUNDARIES.md` / `HARDWARE_PROVIDER_CONTRACT.md` / `MEDIA_BACKEND_CONTRACT.md` / `RUNTIME_RESOURCE_MODEL.md` / `CANONICAL_MEDIA_MODEL.md` / `TECHNOLOGY_PORTABILITY_MATRIX.md` / `VENDOR_NEUTRALITY_RULES.md` / `CANONICAL_IDENTITY.md` / `RUNTIME_TOPOLOGY_CONTRACT.md`

## 7. Final Hardening（2026-08-28 第三轮审查，用户裁决）
> 用户重新核对 GitHub master 上的 Master PRD / Addendum / Canonical Model / Session/Resource/Provider Contract / V0.2，结论：**方向正确、文档基本对齐，但仍差一轮 Architecture Contract Coherence Fix，不能直接宣布 Freeze**。
- **不推翻现有 PRD**，只修硬冲突（非重做）。
- 新建 `CANONICAL_IDENTITY.md`：统一 ID Vocabulary（`DeviceId`/`PortId`/`SessionId`/`ResourceId`/... + `IdentityStrength`/`Source`/`Scope`/`Stability`）；明确 `DeviceHandle`/`PersistentId` 是 **Provider Identity**，须经 Adapter 映射为 Canonical `DeviceId`（解决 BMD 身份泄漏风险，#1/#2）。
- 新建 `RUNTIME_TOPOLOGY_CONTRACT.md`：PhysicalConnection / LogicalRoute / Topology 分离（#6）。
- `SessionId` 由 `PersistentId` 改为 `SessionId(Uuid)`，冻结 Session Creator/Owner/Terminator（#3/#4）。
- Resource 加 `parent_resource_id`，严格区分 Resource/Vector/Constraint/Token 四概念（#5）。
- 统一阶段：0.6 = P0/P0.5 only，0.7 = P1（Audio/Clock/Timecode 从 0.6 移入 0.7，仅冻结 Contract 边界），0.8 = P2（#5 阶段矛盾）。
- Provider `open()` 收窄为 `open_input`/`open_output`（收 resolved request，不收整个 Manifest）；Backend `plan/build` 收窄为 `instantiate/start/stop/recover/observe`（#15/#17）。
- 统一术语：CONTRACT STATUS / IMPLEMENTATION STATUS / GATE STATUS（#13/#14）。
- **状态降级**：本文档标题由 "Runtime Abstraction Freeze" 改为 "Contract Coherence — Final Hardening"；待本轮硬冲突修复 + 过 `ARCH-PORTABILITY-01`/`ARCH-BACKEND-01` 门禁后，正式宣布 `PHASE-0.6-RUNTIME-ABSTRACTION-CONTRACT-FROZEN`，再进入 0.6 实施 → Normalize。
