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
