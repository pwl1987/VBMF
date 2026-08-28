//! Phase 0.6 C5: AdapterRegistry —— Provider/Backend 选择的单一收口点 (SPI 分层收口)。
//!
//! 把 main 中散落的 `Box<dyn HardwareProvider>` / `Arc<dyn MediaBackend>` 构造与
//! feature 优先级选择集中到此处。调用方(Domain / Graph / main)只拿到 trait 对象,
//! 绝不感知具体适配器 (Blackmagic / GStreamer / Mock / Simulated / Filesystem)。
//!
//! 选择优先级(与 C2c 接线一致):
//! - HardwareProvider: `mock` > `simulation` > `bmd-provider` > `default`(filesystem)。
//! - MediaBackend:     `mock` > `gstreamer-backend`。
//!
//! 此收口使 C6 BMD 迁移 / C7 GStreamer 迁移 / 运行时可插拔 (显式配置选择适配器)
//! 只需改动本模块, 不动 main 与 Domain/Graph, 进一步收敛 ARCH-PORTABILITY-01。
//! (显式配置覆盖 feature 选择为后续运行时可插拔扩展点, 本期未接。)

use std::boxed::Box;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::sync::Arc;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::backend::MediaBackend;

use crate::contracts::provider::HardwareProvider;

/// Provider/Backend 适配器注册表。仅暴露 trait 对象构造, 调用方不感知具体实现。
pub struct AdapterRegistry;

impl AdapterRegistry {
    /// 选择并构造 `HardwareProvider`。各实现返回相同 `Vec<DeviceInfo>` 契约。
    ///
    /// 优先级(高→低): `mock` > `simulation` > `bmd-provider` > `default`(filesystem)。
    pub fn build_provider() -> Box<dyn HardwareProvider> {
        #[cfg(feature = "mock")]
        let provider: Box<dyn HardwareProvider> =
            Box::new(crate::adapters::mock::MockProvider);
        #[cfg(all(not(feature = "mock"), feature = "simulation"))]
        let provider: Box<dyn HardwareProvider> =
            Box::new(crate::device::SimulatedDeviceManager::new());
        #[cfg(all(not(feature = "mock"), not(feature = "simulation"), feature = "bmd-provider"))]
        let provider: Box<dyn HardwareProvider> =
            Box::new(crate::adapters::blackmagic::DeckLinkDeviceManager::new());
        #[cfg(all(not(feature = "mock"), not(feature = "simulation"), not(feature = "bmd-provider")))]
        let provider: Box<dyn HardwareProvider> =
            Box::new(crate::device::FilesystemDeviceManager::new());
        provider
    }

    /// 选择并构造 `MediaBackend`。仅在 `bmd-provider,gstreamer-backend` 下编译
    /// (与 C2c 接线一致: 真机盒需 `--features bmd-provider,gstreamer-backend`)。
    ///
    /// 优先级(高→低): `mock` > `gstreamer-backend`。
    #[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
    pub fn build_media_backend() -> Arc<dyn MediaBackend> {
        #[cfg(feature = "mock")]
        let backend: Arc<dyn MediaBackend> =
            Arc::new(crate::adapters::mock::MockBackend);
        #[cfg(all(not(feature = "mock"), feature = "gstreamer-backend"))]
        let backend: Arc<dyn MediaBackend> =
            Arc::new(crate::adapters::gstreamer::GStreamerPipelineController::new());
        backend
    }
}
