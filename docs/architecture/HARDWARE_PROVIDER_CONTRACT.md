# HARDWARE_PROVIDER_CONTRACT — 硬件 Provider SPI 契约

> Phase 0.6 门禁依据（P0）。综合论述见 [`IMPLEMENTATION_ADDENDUM.md §1,§6,§9`](./IMPLEMENTATION_ADDENDUM.md)。

## 1. SPI 边界（trait 形状级，非实现）
```
trait MediaHardwareProvider {
    fn discover(&self) -> Vec<CanonicalDevice>;          // 返回 Canonical Device/Port，不返回 vendor handle
    fn translate(&self, physical: PhysicalResource) -> ProviderResource;
    fn capabilities(&self, dev: &DeviceId) -> CapabilitySnapshot;
    // 不接收整个 Manifest：由 Binding 层先完成 Manifest → ResolvedBinding，
    // Provider 只收已解析的 Provider 资源请求（见 CANONICAL_IDENTITY.md）。
    // Provider 最多：discover/resolve/open resource/close resource/observe；
    // 绝不包含 create_session/create_lease/modify_health（否则破坏 Session Ownership）。
    fn open_input(&self, request: &ProviderInputRequest)
        -> Result<ProviderInputSession, ProviderError>;
    fn open_output(&self, request: &ProviderOutputRequest)
        -> Result<ProviderOutputSession, ProviderError>;
    fn close(&self, session: &ProviderSessionRef) -> Result<(), ProviderError>;
    fn observe(&self) -> ProviderObservation;
}
```
- Provider 负责 **Vendor Resource ↔ Physical Resource ↔ Provider Resource** 翻译。
- Domain / Graph / Session **不得** `use gstreamer::*` / `use decklink::*`（当前 `pipeline.rs`/`signal.rs` 直接依赖，是 P0 缺口）。

## 2. 身份与失败闭合
- **Provider Identity（非 Canonical）**：`DeviceHandle`（经 `IDeckLinkProfileAttributes::GetString(BMDDeckLinkDeviceHandle)`，详见 memory FFI 常量）是 **BMD Provider 内部**的稳定句柄——属于 Provider Identity，**不是** VBMF Canonical Identity（见 [`CANONICAL_IDENTITY.md`](./CANONICAL_IDENTITY.md)）。Provider 内部优先级 PersistentId > DeviceHandle > TopologicalId > EnumerationOnly；解析结果统一收敛为 Canonical `DeviceId` + `IdentityStrength`。
- `device-number` 绝不默认 0；SDK 枚举序 ≠ GStreamer `device-number`（见 Canonical Ingest 边界）。Canonical 主身份是 `DeviceId`，`device-number` 仅是运行时地址。
- Identity / Capability / Binding 冲突必须拒绝；绝不盲开 device 0 / 自动换卡。

## 3. 门禁判据（ARCH-PORTABILITY-01 Test A）
- 删除/禁用 BMD Provider 后，要求 **Domain / Graph / Session / Supervisor / Health / Acceptance 仍能编译**。
- **当前状态：编译不过**（main/resolver/signal/pipeline 直接依赖 decklink/gstreamer 模块）→ 这是 P0 必须消除的缺口，列为门禁目标。
- CI：`cargo build --no-default-features --features simulation` 与 `--features mock-only` 必须通过。
