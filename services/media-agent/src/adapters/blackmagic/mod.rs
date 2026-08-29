//! BMD (Blackmagic DeckLink) Reference Adapter.
//!
//! 仅此模块（及其子模块 `decklink` / `sdk`）引用 vendor `decklink` / `sdk` crate 顶层。
pub mod decklink;
pub mod sdk;

/// BMD 真实设备发现 (Reference Adapter). 仅在 `bmd-provider` 下编译,
/// 使 Domain (`device.rs`) 不依赖 BMD, 满足 ARCH-PORTABILITY-01 Test A.
#[cfg(feature = "bmd-provider")]
pub mod device_manager;
// 模块门面 re-export. 在 `bmd-provider,gstreamer-backend,mock` 组合下, main 经
// `dyn HardwareProvider` / `dyn MediaBackend` 接线会优先选用 Mock, 导致真实适配器不被引用;
// 此 re-export 在该组合下看似未用, 但属架构门面 (Concrete Adapters 公共 API), 故允许.
#[cfg(feature = "bmd-provider")]
#[allow(unused_imports)]
pub use device_manager::DeckLinkDeviceManager;

// C6: 诊断探针门面 re-export —— 将 `decklink::`/`sdk::` 子模块内部诊断函数提升到 `blackmagic`
// 模块表面, 使 runtime 层 (main.rs) 不再点名 vendor 子模块 (`decklink::`/`sdk::`), 收敛
// ARCH-PORTABILITY-01 边界 (对齐 gstreamer 经 `AdapterRegistry::build_media_backend` 收敛).
// 这些函数本质 BMD-specific 诊断, 无法 vendor-neutral, 但调用点统一收敛到 `crate::adapters::blackmagic::*`.
// 各 re-export 的 cfg 与 main.rs 调用点严格一致, 保证任意 feature 组合下符号可达且不被误判 unused.
#[cfg(feature = "bmd-provider")]
pub use decklink::probe_connector_config;
#[cfg(feature = "hardware-test")]
pub use decklink::registry;
#[cfg(all(feature = "hardware-test", not(feature = "gstreamer-backend")))]
pub use decklink::start_capture;
pub use sdk::probe_sdk;
