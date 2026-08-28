# CANONICAL_IDENTITY — Canonical 身份标识契约

> 状态：🔧 待建 → ✅ 已建（Phase 0.6, P0）
> 来源：API PRD 评审 §3.5-P（#93/#94/#147 与 Portability #8/#11 应共享"稳定身份"定义）；用户 Final Hardening 审查（2026-08-28）第 ①/② 优先项。
> 关联：`CANONICAL_MEDIA_MODEL.md`、`HARDWARE_PROVIDER_CONTRACT.md`、`RUNTIME_SESSION_MODEL.md`、`DEVICE_INTEGRATION_CONTRACT.md`

## 1. 核心判据（用户审查第 ①/② 项）

> **Provider Identity ≠ Canonical Identity。**
>
> `DeviceHandle` / `PersistentId` / `TopologicalId` / `SDK GUID` 本质是 **Provider-specific identity mechanism**，不是 VBMF Canonical Identity。
> BMD 的 `DeviceHandle` 是当前硬件非常好的稳定身份，但**不能因此把 BMD 的身份机制升级成 VBMF 的 Canonical Identity**——否则 AJA 的 serial/UUID 来了又得改 Canonical Domain。

正确模型：

```
Provider Identity (vendor-specific)
        ↓  Provider Identity Adapter
Canonical DeviceId
        ↓
Domain 只知道 DeviceId + IdentityStrength
```

## 2. Canonical ID Vocabulary（统一命名）

| Canonical ID | 类型形态 | 说明 |
|---|---|---|
| `DeviceId` | `struct DeviceId(String)` | 设备稳定身份（经 Provider Adapter 映射，非 vendor handle） |
| `PortId` | `struct PortId(String)` | 端口稳定身份 |
| `SessionId` | `struct SessionId(Uuid)` | 会话身份，**独立**于硬件 PersistentId |
| `ResourceId` | `struct ResourceId(String)` | 逻辑资源身份 |
| `BindingId` | `struct BindingId(String)` | Runtime Binding 身份 |
| `LeaseId` | `struct LeaseId(String)` | Lease 身份 |
| `PipelineId` | `struct PipelineId(String)` | Pipeline 身份（opaque handle） |
| `IntegrationId` | `struct IntegrationId(String)` | 外部集成身份 |
| `AgentId` | `struct AgentId(String)` | 外部 Agent 身份（非 PID/container/IP） |
| `EndpointId` | `struct EndpointId(String)` | 集成端点身份 |
| `CommandId` | `struct CommandId(Uuid)` | 幂等命令身份 |

- 所有 Canonical ID **必须**用新类型包裹（`struct XxxId(...)`），禁止裸 `String`/整数作主身份（防止 device-number / PID / IP 泄漏成业务主身份）。
- `SessionId` 用 `Uuid`，**绝不使用**硬件 `PersistentId` 作 Session 身份（见 `RUNTIME_SESSION_MODEL.md`）。

## 3. Identity 属性（统一维度）

| 属性 | 取值 | 说明 |
|---|---|---|
| `IdentityStrength` | `Declared` / `Topological` / `Persistent` / `Verified` | 身份置信度（与 `VerificationLevel` 对应，不混淆） |
| `IdentitySource` | `ProviderAdapter` / `Manifest` / `Discovery` / `Enumeration` | 身份来自哪一层 |
| `IdentityScope` | `Device` / `Port` / `Resource` / `Session` / `Integration` | 身份作用域 |
| `IdentityStability` | `Ephemeral` / `Session` / `Persistent` / `Permanent` | 跨重启/跨进程稳定性 |

## 4. Provider Identity（vendor-specific，非 Canonical）

Provider 内部可使用任意厂商机制，**但必须经由 Provider Identity Adapter 映射为 Canonical `DeviceId`**：

| Provider | Provider Identity 机制 | 映射 |
|---|---|---|
| BMD (Blackmagic) | `DeviceHandle`（经 `IDeckLinkProfileAttributes::GetString(BMDDeckLinkDeviceHandle)`） | → `DeviceId` + `IdentityStrength` |
| AJA | `Serial` / `UUID` / SDK GUID | → `DeviceId` |
| Vendor X | PCIe persistent ID | → `DeviceId` |

- Canonical Domain **只知道** `DeviceId` + `IdentityStrength` + `IdentitySource`。
- `PersistentId` / `TopologicalId` / `DeviceHandle` 是 Provider 层概念，可在 evidence / provider-host 层出现，**不得**出现在 Canonical Domain / Graph / UI schema。
- 身份优先级（Provider 内部解析用）：`PersistentId > DeviceHandle > TopologicalId > EnumerationOnly`；但解析结果统一收敛为 Canonical `DeviceId`。

## 5. 失败闭合

- Identity / Capability / Binding 冲突必须拒绝；绝不盲开 device 0 / 自动换卡。
- `device-number` 仅是运行时地址，**绝不**作为业务主身份，绝不默认 0。
- 多重 HIGH 身份 → `Ambiguous`（拒）；Unresolved → `IdentityUnresolved`。

## 6. 验收

- `HW-IDENT-02`：稳定身份（非 device-number）在 Canonical 层以 `DeviceId` 表达。
- `ARCH-API-BOUNDARY-01`：External API 暴露 `DeviceId`/`PortId`，**不**暴露 `DeviceHandle`/`PersistentId`/vendor handle。
- `ARCH-PORTABILITY-01`：换 BMD→AJA，Domain 仅见 `DeviceId`，零变化。
