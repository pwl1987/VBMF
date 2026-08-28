# RUNTIME_RESOURCE_MODEL — 运行时资源模型契约

> Phase 0.6 门禁依据（P0.5，优先级高于 Clock/Timecode）。综合论述见 [`IMPLEMENTATION_ADDENDUM.md §5`](./IMPLEMENTATION_ADDENDUM.md)。**必须与 V0.2 §3.11 九维 Resource Vector 对齐，不得另起一套语义。**

## 1. 必须回答的 4 个问题
```
Capability    = 能不能做
Capacity      = 最多能做多少
Availability  = 现在还有多少
Allocation    = 当前谁占用了多少
```
示例：BMD Device → Capability 4 SDI Input / Capacity 4 input sessions / Availability 2 / Allocation Session-A→Port1, Session-B→Port2。

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

## 4. Resource 树（只定义模型，不实现 Scheduler）
```
Resource
├── DeviceResource / PortResource / BackendResource
├── EncoderResource / GPUResource / NetworkResource
├── StorageResource / ClockResource
```

## 5. 门禁判据
- Resource 状态机与 V0.2 §3.11 九维 Resource Vector 一致，无重复语义。
- `Resource Registry` 是 Resource State 的唯一 owner（见 IMPLEMENTATION_BOUNDARIES §5）。
- P0.5 只冻结模型，不实现 Scheduler / Placement / 全局资源仲裁。
