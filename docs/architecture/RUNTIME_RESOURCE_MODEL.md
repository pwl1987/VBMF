# RUNTIME_RESOURCE_MODEL — 运行时资源模型契约

> Phase 0.6 门禁依据（P0.5，优先级高于 Clock/Timecode）。综合论述见 [`IMPLEMENTATION_ADDENDUM.md §5`](./IMPLEMENTATION_ADDENDUM.md)。**必须与 V0.2 §3.11 九维 Resource Vector 对齐，不得另起一套语义。**

## 1. 必须回答的 4 个问题
```
Capability    = 能不能做
Capacity      = 最多能做多少
Availability  = 现在还有多少
Allocation    = 当前谁占用了多少
```
示例：`Media Device`（厂商无关）→ Capability N INPUT ports / Capacity N concurrent input sessions / Availability M / Allocation Session-A→Port-X, Session-B→Port-Y。具体厂商数值（如 BMD 4 SDI Input）只放在 PROVIDER/HOST evidence，不进 Canonical 示例（见 [`CANONICAL_IDENTITY.md`](./CANONICAL_IDENTITY.md) 的 vendor 中立原则）。

## 2. Resource ≠ Device
- Resource 是**逻辑可消耗资源**；Device 是**物理身份/容器**。
```
Device ── Port resource / DMA resource / Input session capacity / Output session capacity
GPU    ── decode session / encode session / memory resource
```

## 3. Resource 状态（不止 Available/Allocated）
```
Available → Reserved → Allocated → Releasing → Faulted
```
- `Reservation` = 计划占用（Preflight："准备用这个资源，但 Pipeline 还没启动"）。
- `Reservation ≠ Lease ≠ Allocation`：`Preflight → Reserve Port → Create Session → Acquire Lease → Start Pipeline → Allocated`。
- **竞争关系（三种不同错误，不得混为一谈）**：
  - `Reservation conflict`：Preflight 阶段两个计划争同一 Resource（尚未占用）→ 拒绝后到者，或 Policy 仲裁。
  - `Lease conflict`：两个 Session 同时 `Acquire Lease` 同一 Resource（已 Reserved）→ 拒绝重复 Lease。
  - `Allocation conflict`：运行时实际占用超额（Capacity < 已 Allocation）→ 失败闭合，绝不静默超卖。

## 4. Resource 树（只定义模型，不实现 Scheduler）
```
Resource
├── resource_id: ResourceId
├── parent_resource_id: Option<ResourceId>   // 允许嵌套：Device → Port / DMA / Session-capacity
├── resource_type: Device | Port | Backend | Encoder | GPU | Network | Storage | Clock
├── capacity / availability / allocation
├── DeviceResource ── PortResource A / PortResource B / DMA / Session Capacity
├── EncoderResource / GPUResource / NetworkResource
├── StorageResource / ClockResource
```
- **Resource 允许嵌套**：申请一个 `PortResource` 不会自动占用整个 `DeviceResource`；占用关系由 `parent_resource_id` + Allocation 显式表达（多路卡最易踩的坑）。

## 4.1 Resource / Vector / Constraint / Token 四概念（严格区分）
> 用户 Final Hardening 审查（2026-08-28）第 ⑤ 项：CPU/GPU/PCIe/BMD Port 不得当成同一种 Resource。
- **Resource** = 可消费实体（有 `resource_id`/`parent_resource_id`/状态机）。
- **Resource Vector** = 定量成本/预算（V0.2 §3.11 九维：`cpu`/`gpu`/`pcie`/`mem`/`net`/`bw`/`storage`/`license`/`port`）。
- **Constraint** = 约束条件（如 `device-type=blackmagic`、`clock-domain=ptp`、`exclusive=true`）。
- **Token** = 独占能力（如某 BMD Port 的物理独占权，表达为 Constraint `exclusive=true` + Allocation 排他）。
- 四者正交：Vector 计成本，Constraint 定条件，Token 表独占，Resource 是载体。

## 4.2 Reservation 生命周期状态机（TD-10，冻结 Contract）

> 不仅定义 Resource 状态，还要冻结 Reservation 生命周期（不实现完整 scheduler）。

```
Reserve   → 创建 Reservation（Reserved）
Renew     → 续期 TTL（防止过期回收）
Expire    → TTL 到达且无续期 → Reservation 失效（Reserved→Available）
Release   → 主动释放（Session 结束）
Abort     → 异常中止（Preflight 失败 / Session 创建失败回滚）
Recover   → Supervisor 重建（Lease 仍有效时，不重新 Reserve）
```

- **Reservation TTL / Lease TTL**：必须有上限；超时未进入 Allocated 则自动 `Expire`/`Abort`。
- **Crash cleanup**：进程崩溃后，孤儿 Reservation/Lease 由 Recovery 扫描 TTL 回收（绝不残留「占而未用」）。
- 状态转移唯一归属 Resource Registry / Lease Manager（见 IMPLEMENTATION_BOUNDARIES §5.2）。

## 4.3 Resource 层级 mutation boundary（TD-11）

- `parent_resource_id` 形成树；**子 Resource 的 Allocation 变更不得绕过父 Resource Registry**。
- 占用关系由 Resource Registry 统一校验（多路卡：占 `PortResource` 不自动占 `DeviceResource`，但校验 Device 总 Capacity 不被超额）。
- 禁止 Backend / Provider 自行修改 Resource 状态（见 MEDIA_BACKEND_CONTRACT P0-8）。

## 5. 门禁判据
- Resource 状态机与 V0.2 §3.11 九维 Resource Vector 一致，无重复语义。
- `Resource Registry` 是 Resource State 的唯一 owner（见 IMPLEMENTATION_BOUNDARIES §5）。
- P0.5 只冻结模型，不实现 Scheduler / Placement / 全局资源仲裁。
