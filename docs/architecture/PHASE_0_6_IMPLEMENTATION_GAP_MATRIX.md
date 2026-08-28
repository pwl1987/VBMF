# PHASE_0_6_IMPLEMENTATION_GAP_MATRIX — 实现缺口矩阵

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
| Hardware Provider SPI（`MediaHardwareProvider`） | `NOT_STARTED` | — | — | — | — | `ARCH-PORTABILITY-01` | `OPEN` |
| Media Backend SPI（`MediaBackend`） | `NOT_STARTED` | — | — | — | — | `ARCH-BACKEND-01` | `OPEN` |
| Session Model（`SessionId`/Lifecycle/Owner） | `NOT_STARTED` | — | — | — | — | `ARCH-PORTABILITY-01` | `OPEN` |
| Resource Model（Reservation/Lease/Allocation/四概念） | `NOT_STARTED` | — | — | — | — | — | `OPEN` |
| Runtime Binding（`BindingEntry`/Manifest 校验） | `PARTIAL` | Partial | — | Acceptance | `evidence/.../acceptance` | `ARCH-PORTABILITY-01` | `PARTIAL` |
| Topology（`PhysicalConnection`/`LogicalRoute`） | `NOT_STARTED` | — | — | — | — | `ARCH-TOPOLOGY-01` | `OPEN` |
| RuntimeEvent / RuntimeError（统一模型） | `PARTIAL` | Partial | — | MEDIA-RT-01 | `evidence/.../MEDIA-RT-01` | `ARCH-BACKEND-01` | `PARTIAL` |
| Preflight（Bind/Reservation 预检） | `NOT_STARTED` | — | — | — | — | — | `OPEN` |
| Provider Registry / Mock Provider+Backend | `NOT_STARTED` | — | — | — | — | `ARCH-PORTABILITY-01`/`ARCH-BACKEND-01` | `OPEN` |
| Architecture Lint（CI 防回归） | `NOT_STARTED` | — | — | — | — | CI | `OPEN` |
| 消除 11× `device_number` → Canonical `DeviceId` | `IN_PROGRESS` | Partial | — | HW-IDENT-02 | `evidence/.../HW-IDENT-02` | `ARCH-PORTABILITY-01` | `PARTIAL` |
| BMD / GStreamer Reference Adapter（降级为 Adapter） | `NOT_STARTED` | — | — | — | — | `ARCH-PORTABILITY-01`/`ARCH-BACKEND-01` | `OPEN` |
| Portability Gate（`ARCH-PORTABILITY-01` Test A 编译通过） | `NOT_PASSED` | — | — | — | — | `ARCH-PORTABILITY-01` | `BLOCKED` |
| Backend Gate（`ARCH-BACKEND-01` Mock↔GStreamer 共享 Plan） | `NOT_PASSED` | — | — | — | — | `ARCH-BACKEND-01` | `BLOCKED` |

## 结论
- **Architecture Contract 已全部 `FROZEN`**；但 **Implementation 绝大多数 `OPEN`/`PARTIAL`**。
- 两个阻塞项（`ARCH-PORTABILITY-01` / `ARCH-BACKEND-01`）是当前 P0 的硬缺口，须先于 Normalize 消除。
- 下一步是**纯实现**，不是继续设计（见 `IMPLEMENTATION_ADDENDUM.md` 推进顺序）。
