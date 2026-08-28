//! BMD (Blackmagic DeckLink) Reference Adapter.
//!
//! 仅此模块（及其子模块 `decklink` / `sdk`）引用 vendor `decklink` / `sdk` crate 顶层。
pub mod decklink;
pub mod sdk;

/// BMD 真实设备发现 (Reference Adapter). 仅在 `bmd-provider` 下编译,
/// 使 Domain (`device.rs`) 不依赖 BMD, 满足 ARCH-PORTABILITY-01 Test A.
#[cfg(feature = "bmd-provider")]
pub mod device_manager;
#[cfg(feature = "bmd-provider")]
pub use device_manager::DeckLinkDeviceManager;
