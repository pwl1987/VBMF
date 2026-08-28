# MEDIA_BACKEND_CONTRACT — 媒体 Backend SPI 契约

> Phase 0.6 门禁依据（P0）。综合论述见 [`IMPLEMENTATION_ADDENDUM.md §1,§8`](./IMPLEMENTATION_ADDENDUM.md)。

## 1. SPI 边界（trait 形状级，非实现）
```
trait MediaBackend {
    fn plan(&self, intent: &GraphRuntimeIntent) -> CanonicalPipelinePlan;
    fn build(&self, plan: &CanonicalPipelinePlan) -> Result<BackendRuntime, BackendError>;
    fn observe(&self) -> CanonicalRuntimeEvent;   // 统一事件，非 GStreamer Bus Message
}
```
- Backend 必须消费 **Canonical** `GraphRuntimeIntent`（仅 DeviceId + PortId + Media Semantics），不得依赖 GStreamer/BMD 字段。
- 上层（Session / Supervisor / Health）只认 Canonical 类型；所有 vendor 错误进入统一 `RuntimeEvent/RuntimeError` 模型（见 Addendum §8）。

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
