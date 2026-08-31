# PHASE_0_6_IMPLEMENTATION_GAP_MATRIX — 实现缺口矩阵

> **[状态对账 2026-08-30, p07b-consolidation]** 本表 IMPLEMENTATION/Gate 列已与 master `c574238` 实态对齐（0.6 系列全部落地：SPI/Session/Resource/Preflight/Registry/Mint/RuntimeEvent；Canonical Media Semantics 四基础随 0.7B 落地，门禁见 Acceptance Matrix 0.7 节）。其余未提及行状态不变。

> **唯一回答「文档要求 vs 代码实际完成度」的 SoT。** 状态词见 [`DOCUMENT_STATUS_MODEL.md`](./DOCUMENT_STATUS_MODEL.md)。本文件防止「文档存在 = 功能完成」的误读。
> 关联：各 Contract 文档、[`PHASE_0_6_ACCEPTANCE_MATRIX.md`](./PHASE_0_6_ACCEPTANCE_MATRIX.md)、`evidence/`。

## 状态词（摘要）
- Implementation：`NOT_STARTED` / `IN_PROGRESS` / `PARTIAL` / `IMPLEMENTED`
- Verification：`NOT_VERIFIED` / `LAB_VERIFIED` / `HARDWARE_VERIFIED` / `ACCEPTED`
- Gap 总括：`OPEN`（未启动）/ `PARTIAL`（部分）/ `DONE`（已落地待验证）/ `BLOCKED`（被门禁阻塞）

## 矩阵

| Contract | Implementation | Unit Test | Simulation | Real Hardware | Evidence | Gate | Gap |
|---|---|---|---|---|---|---|---|
| Canonical Identity（`DeviceId`/`PortId`/`SessionId` 类型 + `IdentityStrength`） | `PARTIAL` | Partial | — | HW-IDENT-02 | `evidence/.../HW-IDENT-02` | `ARCH-PORTABILITY-01` | `PARTIAL` |
| Provider Identity Adapter（BMD→`DeviceId` 收敛） | `PARTIAL` | Partial | — | HW-IDENT-02 | 同上 | `ARCH-PORTABILITY-01` | `PARTIAL` |
| Hardware Provider SPI（`HardwareProvider`: discover→Result + ProviderIdentity 配对） | `IMPLEMENTED` | Complete | — | HW-IDENT-02 | `evidence/.../HW-IDENT-02` | `ARCH-PORTABILITY-01` | `PASS` |
| Media Backend SPI（`MediaBackend`: instantiate/start/stop/recover/observe, 无条件编译） | `IMPLEMENTED` | Complete | — | MEDIA-RT-01 / ARCH-BACKEND-01 | `evidence/.../MEDIA-RT-01` | `ARCH-BACKEND-01` | `PASS` |
| Session Model（`SessionId`/两级状态机/SessionManager 唯一 owner） | `IMPLEMENTED` | Complete（0.7A, 四轮 Hardening） | — | SESSION-RT-01 / RESOURCE-RT-01 | 真机 ALL PASS（verify 报告 2026-08-29-p07-session-runtime） | `ARCH-PORTABILITY-01` | `PASS` |
| Resource Model（Reservation/Lease/Allocation/状态机/原子 acquire/回滚） | `IMPLEMENTED` | Complete（0.7A） | — | RESOURCE-RT-01 | 同上 | — | `PASS` |
| Runtime Binding（`BindingEntry`/Manifest 校验） | `PARTIAL` | Partial | — | Acceptance | `evidence/.../acceptance` | `ARCH-PORTABILITY-01` | `PARTIAL` |
| Topology（`PhysicalConnection`/`LogicalRoute`） | `NOT_STARTED` | — | — | — | — | `ARCH-TOPOLOGY-01` | `OPEN` |
| RuntimeEvent / RuntimeError（统一模型） | `PARTIAL` | Partial | — | MEDIA-RT-01 | `evidence/.../MEDIA-RT-01` | `ARCH-BACKEND-01` | `PARTIAL` |
| Preflight（分级判定: Graph/Port/Resource/Lease/Identity/Backend; judge-only FAIL 零预留） | `IMPLEMENTED` | Complete（0.7A） | — | SESSION-RT-01 前置 | verify 报告 p07-session-runtime | — | `PASS` |
| Provider Registry / Mock Provider+Backend | `IMPLEMENTED` | Complete（fail-closed mock+真实冲突拒启） | — | ARCH-BACKEND-01 | CI session-lifecycle job | `ARCH-PORTABILITY-01`/`ARCH-BACKEND-01` | `PASS` |
| Architecture Lint + Proof（词法 Lint + remove-adapter 编译证明, CI required） | `IMPLEMENTED` | Complete | — | CI | `architecture-portability` job | CI | `PASS` |
| 消除 11× `device_number` → Canonical `DeviceId` | `IN_PROGRESS` | Partial | — | HW-IDENT-02 | `evidence/.../HW-IDENT-02` | `ARCH-PORTABILITY-01` | `PARTIAL` |
| BMD / GStreamer Reference Adapter（降级为 Adapter） | `IMPLEMENTED` | Complete（C6/C7 迁移 + hardening 收口: adapters/{blackmagic,gstreamer} + remove-adapter proof） | — | ARCH-PORTABILITY/BACKEND | CI | `ARCH-PORTABILITY-01`/`ARCH-BACKEND-01` | `PASS` |
| Portability Gate（`ARCH-PORTABILITY-01` Test A 编译通过） | `NOT_PASSED` | — | — | — | — | `ARCH-PORTABILITY-01` | `BLOCKED` |
| Backend Gate（`ARCH-BACKEND-01` Mock↔GStreamer 共享 Plan） | `NOT_PASSED` | — | — | — | — | `ARCH-BACKEND-01` | `BLOCKED` |

## 结论
- **Architecture Contract 已全部 `FROZEN`**；但 **Implementation 绝大多数 `OPEN`/`PARTIAL`**。
- 两个阻塞项（`ARCH-PORTABILITY-01` / `ARCH-BACKEND-01`）是当前 P0 的硬缺口，须先于 Normalize 消除。
- 下一步是**纯实现**，不是继续设计（见 `IMPLEMENTATION_ADDENDUM.md` 推进顺序）。
