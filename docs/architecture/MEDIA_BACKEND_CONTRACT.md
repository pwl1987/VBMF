# MEDIA_BACKEND_CONTRACT — 媒体 Backend SPI 契约

> **[实现对齐注记 2026-08-29, p06-final-merge-hardening]** 本契约形状已在实现侧收口:
> `contracts/backend.rs::MediaBackend` = `instantiate/start/stop/recover/observe`
> (`prepare→instantiate`、`poll_bus→observe` 更名; `stop` 补齐)。trait **无条件编译**
> (P0-3: 契约不被 adapter feature 门控, 仅 `impl` 块门控)。错误类型暂用 `PipelineError`
> (不引入独立 `BackendError`, 避免二次 breaking; 语义映射: StartFailed/ConfigInvalid/... 即契约的 BackendError 分类)。
> `observe` 载荷为 vendor-neutral `PipelineBusEvent` (`pipeline_events.rs`), 非 GStreamer Bus Message 语义。

> Phase 0.6 门禁依据（P0）。综合论述见 [`IMPLEMENTATION_ADDENDUM.md §1,§8`](./IMPLEMENTATION_ADDENDUM.md)。

## 1. SPI 边界（trait 形状级，非实现）
```
trait MediaBackend {
    // 不决定 Resource / Session / Lease / Provider：
    // Runtime Orchestrator 已把 Resolved Execution Plan 传给 Backend。
    // `plan()` 属 Graph Compiler/Runtime 职责，Backend 只 instantiate/start/stop/recover/observe。
    // 真正的 lifecycle owner 是 Runtime Orchestrator / Session，不是 Backend。
    fn instantiate(&self, plan: &CanonicalPipelinePlan)
        -> Result<PipelineHandle, BackendError>;
    fn start(&self, handle: &PipelineHandle) -> Result<(), BackendError>;
    fn stop(&self, handle: &PipelineHandle) -> Result<(), BackendError>;
    fn recover(&self, handle: &PipelineHandle) -> Result<(), BackendError>;
    fn observe(&self, handle: &PipelineHandle) -> CanonicalRuntimeEvent; // 统一事件，非 GStreamer Bus Message
}
```
- Backend 必须消费 **Canonical** `CanonicalPipelinePlan`（由 Runtime Orchestrator 解析，仅含 Canonical 类型），不得依赖 GStreamer/BMD 字段。
- 上层（Session / Supervisor / Health）只认 Canonical 类型；所有 vendor 错误进入统一 `RuntimeEvent/RuntimeError` 模型（见 Addendum §8）。

## 1.1 资源获取边界（P0-8，冻结）

> **Backend 必须不自行获取未注册资源。**

- Runtime Orchestrator 负责：`Resource Reservation → Binding → Backend.instantiate`；Backend **只能使用已经解析和授权的 Runtime Resource**。
- Backend **不得**自行寻找 `/dev/video*` / `device-number` / GPU / encoder / 其他硬件；否则重新破坏 vendor-neutral 架构。
- 若 `instantiate` 发现所需 Resource 未在 Reservation 内，必须 **fail-closed**（报错返回），不得悄悄 acquire。

## 2. 共享契约（ARCH-BACKEND-01 判据）
`MockBackend` vs `GStreamerBackend` 必须共享：
- `CanonicalPipelinePlan`
- `CanonicalMediaFormat`
- `CanonicalRuntimeEvent`

## 3. 失败闭合
- Backend 失败 → `RuntimeEvent` → Health/Policy Reducer → Supervisor Decision；**而非** `GStreamer Error → Supervisor`。
- 禁止自动 Fallback：Backend 失败必须 `Policy + Capability + Preflight + 决策` 才能切换（见 VENDOR_NEUTRALITY_RULES）。

## 4. 门禁判据
- 换 GStreamer→FFmpeg：仅 Backend replaced + RuntimeBinding changed，**不改变** CanonicalPipelinePlan。
- `cargo build --features mock-only` 下，Domain / Runtime Contract / Simulation 仍能编译。
