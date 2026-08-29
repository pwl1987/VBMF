//! Phase 0.6 C5 + Final Merge Hardening (P0-4): AdapterRegistry —— Provider/Backend 选择的单一收口点。
//!
//! 把 main 中散落的 `Box<dyn HardwareProvider>` / `Arc<dyn MediaBackend>` 构造与
//! feature 优先级选择集中到此处。调用方(Domain / Graph / main)只拿到 trait 对象,
//! 绝不感知具体适配器 (Blackmagic / GStreamer / Mock / Simulated / Filesystem)。
//!
//! 选择优先级(与 C2c 接线一致):
//! - HardwareProvider: `mock` > `simulation` > `bmd-provider` > `default`(filesystem)。
//! - MediaBackend:     `mock` > `gstreamer-backend`。
//!
//! **P0-4 fail-closed**: `mock` 与真实实现 (`bmd-provider`/`gstreamer-backend`) 同时编译时,
//! **生产模式**下不得静默按优先级取 Mock (历史部署事故来源) —— 构造函数返回 `Err`,
//! main 拒启并列出冲突 feature。显式测试模式 (`MEDIA_AGENT_MODE=simulation|diagnostic`
//! 或 `VBMF_ALLOW_MOCK=1`) 放行 (测试/诊断需要 mock+真实组合时必须明示)。
//!
//! 此收口使后续运行时可插拔 (显式配置选择适配器) 只需改动本模块, 不动 main 与 Domain/Graph。

use std::boxed::Box;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::sync::Arc;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::backend::MediaBackend;

use crate::contracts::provider::HardwareProvider;

/// mock 与真实实现同时编译时的冲突描述 (无冲突 → `None`)。
pub fn mock_real_conflict() -> Option<&'static str> {
    #[cfg(feature = "mock")]
    {
        #[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
        {
            return Some("mock + bmd-provider + gstreamer-backend");
        }
        #[cfg(all(feature = "bmd-provider", not(feature = "gstreamer-backend")))]
        {
            return Some("mock + bmd-provider");
        }
        #[cfg(all(not(feature = "bmd-provider"), feature = "gstreamer-backend"))]
        {
            return Some("mock + gstreamer-backend");
        }
        #[allow(unreachable_code)]
        {
            None
        }
    }
    #[cfg(not(feature = "mock"))]
    {
        None
    }
}

/// 显式测试/诊断模式: 允许 mock+真实组合共存 (必须明示, 不许静默)。
pub fn test_mode_allows_mock() -> bool {
    matches!(
        std::env::var("MEDIA_AGENT_MODE").as_deref(),
        Ok("simulation") | Ok("diagnostic")
    ) || std::env::var("VBMF_ALLOW_MOCK").is_ok()
}

/// P0-4 生产 fail-closed 检查: 冲突且非显式测试模式 → `Err` (main 据此拒启)。
pub fn ensure_adapter_selection_safe() -> Result<(), String> {
    match mock_real_conflict() {
        None => Ok(()),
        Some(conflict) => {
            if test_mode_allows_mock() {
                tracing::warn!(
                    conflict,
                    "mock 与真实 adapter 组合已由显式测试/诊断模式放行 (VBMF_ALLOW_MOCK/MEDIA_AGENT_MODE)"
                );
                Ok(())
            } else {
                Err(format!(
                    "adapter feature 冲突: {conflict} 同时编译; 生产模式拒绝静默选择 Mock。\
                     如确为测试/诊断场景, 请显式设置 MEDIA_AGENT_MODE=simulation|diagnostic 或 VBMF_ALLOW_MOCK=1"
                ))
            }
        }
    }
}

/// 生效 adapter 摘要 (启动日志打印, 运维可见当前选择)。
pub fn active_adapters() -> (&'static str, &'static str) {
    let provider = if cfg!(feature = "mock") {
        "mock"
    } else if cfg!(feature = "simulation") {
        "simulation"
    } else if cfg!(feature = "bmd-provider") {
        "blackmagic"
    } else {
        "filesystem"
    };
    let backend = if cfg!(all(feature = "mock", any(feature = "bmd-provider", feature = "gstreamer-backend"))) {
        "mock"
    } else if cfg!(feature = "gstreamer-backend") {
        "gstreamer"
    } else if cfg!(feature = "mock") {
        "mock"
    } else {
        "none"
    };
    (provider, backend)
}

/// Provider/Backend 适配器注册表。仅暴露 trait 对象构造, 调用方不感知具体实现。
pub struct AdapterRegistry;

impl AdapterRegistry {
    /// 选择并构造 `HardwareProvider`。各实现返回相同 `Result<Vec<DiscoveredDevice>>` 契约。
    ///
    /// 优先级(高→低): `mock` > `simulation` > `bmd-provider` > `default`(filesystem)。
    /// **P0-4**: 生产模式下 mock+真实组合 → `Err` (拒启)。
    pub fn build_provider() -> Result<Box<dyn HardwareProvider>, String> {
        ensure_adapter_selection_safe()?;
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
        Ok(provider)
    }

    /// 选择并构造 `MediaBackend`。仅在 `bmd-provider,gstreamer-backend` 下编译
    /// (与 C2c 接线一致: 真机盒需 `--features bmd-provider,gstreamer-backend`)。
    ///
    /// 优先级(高→低): `mock` > `gstreamer-backend`。**P0-4**: 生产 fail-closed 见上。
    #[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
    pub fn build_media_backend() -> Result<Arc<dyn MediaBackend>, String> {
        ensure_adapter_selection_safe()?;
        #[cfg(feature = "mock")]
        let backend: Arc<dyn MediaBackend> =
            Arc::new(crate::adapters::mock::MockBackend);
        #[cfg(all(not(feature = "mock"), feature = "gstreamer-backend"))]
        let backend: Arc<dyn MediaBackend> =
            Arc::new(crate::adapters::gstreamer::GStreamerPipelineController::new());
        Ok(backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_fail_closed_gate_consistent_with_feature_set() {
        // P0-4: 检查函数与 feature 组合自洽 — 无冲突恒 Ok; 有冲突时必须显式测试模式才 Ok.
        match mock_real_conflict() {
            None => assert!(ensure_adapter_selection_safe().is_ok()),
            Some(_) => {
                // 测试进程环境通常未设置显式模式 → 拒启 (行为即门禁; 放行路径由运行时行为验证).
                if !test_mode_allows_mock() {
                    assert!(ensure_adapter_selection_safe().is_err());
                }
            }
        }
    }
}
