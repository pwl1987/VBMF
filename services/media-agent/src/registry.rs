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
use crate::contracts::backend::MediaBackend;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::sync::Arc;

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
    let backend = if cfg!(all(
        feature = "mock",
        any(feature = "bmd-provider", feature = "gstreamer-backend")
    )) {
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
        let provider: Box<dyn HardwareProvider> = Box::new(crate::adapters::mock::MockProvider);
        #[cfg(all(not(feature = "mock"), feature = "simulation"))]
        let provider: Box<dyn HardwareProvider> =
            Box::new(crate::device::SimulatedDeviceManager::new());
        #[cfg(all(
            not(feature = "mock"),
            not(feature = "simulation"),
            feature = "bmd-provider"
        ))]
        let provider: Box<dyn HardwareProvider> =
            Box::new(crate::adapters::blackmagic::DeckLinkDeviceManager::new());
        #[cfg(all(
            not(feature = "mock"),
            not(feature = "simulation"),
            not(feature = "bmd-provider")
        ))]
        let provider: Box<dyn HardwareProvider> =
            Box::new(crate::device::FilesystemDeviceManager::new());
        Ok(provider)
    }

    /// 选择并构造 `MediaBackend`（**单 view 委托面**）。仅在
    /// `bmd-provider,gstreamer-backend` 下编译。
    ///
    /// A2-8-02-F-02（第十轮终裁）: 本函数**不再独立构造** concrete
    /// controller——委托 `build_media_adapter_bundle()` 取 backend view
    /// （全仓库唯一 controller 构造路径= bundle; 旧双路径已封死, 不得
    /// 恢复独立 `Arc::new(GStreamerPipelineController)`——那会制造第二
    /// instances ownership 表）。仅需 backend 的调用方（gates/自检）经
    /// 此面; 组合根程序装配直接用 bundle。
    #[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
    pub fn build_media_backend() -> Result<Arc<dyn MediaBackend>, String> {
        Ok(Self::build_media_adapter_bundle()?.backend)
    }

    /// A2-8-02-F-01（第九轮终裁）: 同源 runtime adapter bundle——
    /// **同一 concrete `GStreamerPipelineController` 的双 trait view**
    /// （`MediaBackend` + `MediaTapPort` 指向同一对象; 禁二次构造——两个
    /// controller 即两个 instances ownership 表, tap attach 将得
    /// UnknownPipeline）。组合根经此取得双 view; SessionManager 仍只见
    /// `MediaBackend`（Session 抽象边界不破）。
    #[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
    pub fn build_media_adapter_bundle() -> Result<MediaAdapterBundle, String> {
        ensure_adapter_selection_safe()?;
        #[cfg(feature = "mock")]
        {
            // Mock 世界无共享 instances 表——独立实例语义等价。
            Ok(MediaAdapterBundle {
                backend: Arc::new(crate::adapters::mock::MockBackend),
                media_tap: Some(Arc::new(crate::adapters::mock::MockMediaTapPort::new())),
            })
        }
        #[cfg(all(not(feature = "mock"), feature = "gstreamer-backend"))]
        {
            // 单次构造 concrete controller → 两次 clone 各自 coerce——
            // 两个 trait object 同源同一对象（结构保证 + 行为证明见
            // registry_rt_01_bundle_dual_view_same_controller）。
            let controller =
                Arc::new(crate::adapters::gstreamer::GStreamerPipelineController::new());
            Ok(MediaAdapterBundle {
                backend: controller.clone(),
                media_tap: Some(controller),
            })
        }
    }
}

/// A2-8-02-F-01: 同源 adapter bundle（backend + media tap 双 trait view）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
pub struct MediaAdapterBundle {
    pub backend: Arc<dyn MediaBackend>,
    pub media_tap: Option<Arc<dyn crate::contracts::media_tap::MediaTapPort>>,
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

    // A2-8-02-F-01 同源双 view **行为证明**（盒上 bmd+gstreamer 非 mock）:
    // 经 backend view 实例化的 handle, 经 tap view attach 成功——若为两个
    // controller（二次构造）, instances 表分裂 → UnknownPipeline（反证）。
    #[cfg(all(
        feature = "bmd-provider",
        feature = "gstreamer-backend",
        not(feature = "mock")
    ))]
    #[test]
    fn registry_rt_01_bundle_dual_view_same_controller() {
        use crate::contracts::media_tap::{MediaTapRequest, TapPlanes};
        use crate::pipeline::PipelinePlan;

        let bundle = AdapterRegistry::build_media_adapter_bundle().expect("bundle 构造");
        let tap = bundle.media_tap.expect("tap view 在");
        let h = bundle
            .backend
            .instantiate(&PipelinePlan::self_test())
            .expect("物化");
        bundle.backend.start(&h).expect("启动");
        tap.attach_media_tap(
            &h,
            &MediaTapRequest {
                channel: "bundle-proof".into(),
                planes: TapPlanes::Both,
            },
        )
        .expect("同源双 view: tap 可见 backend 实例化的 handle（分裂即 UnknownPipeline）");
        assert_eq!(tap.tap_attachments(&h).len(), 1, "簿记在（同一 ownership）");
        let _ = bundle.backend.stop(&h);
    }

    // A2-8-02-F-02: **真接 MediaTap 的 Runtime 生命周期**（盒上真实
    // GStreamer）——bundle 双 view → 双输入管线 → Runtime::create 真挂
    // tap（簿记在真实管线上）→ teardown 真摘（簿记清空）。channel=
    // device_id 派生桥接地址。
    #[cfg(all(
        feature = "bmd-provider",
        feature = "gstreamer-backend",
        not(feature = "mock")
    ))]
    #[test]
    fn registry_rt_01_runtime_tap_lifecycle_on_same_controller() {
        use crate::pipeline::PipelinePlan;
        use crate::program_execution::{ProgramExecutionRuntime, TapWiring};
        use crate::session::{SessionId, SessionInput};
        use crate::switch_execution::ExecutionGroup;
        use uuid::Uuid;

        let bundle = AdapterRegistry::build_media_adapter_bundle().expect("bundle");
        let tap = bundle.media_tap.clone().expect("tap view");
        let h1 = bundle
            .backend
            .instantiate(&PipelinePlan::self_test())
            .expect("管线 A");
        let h2 = bundle
            .backend
            .instantiate(&PipelinePlan::self_test())
            .expect("管线 B");
        bundle.backend.start(&h1).expect("启动 A");
        bundle.backend.start(&h2).expect("启动 B");
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let sid = SessionId(Uuid::new_v4());
        let group = ExecutionGroup::new(
            sid,
            vec![
                SessionInput {
                    device_id: a,
                    handle: h1,
                },
                SessionInput {
                    device_id: b,
                    handle: h2,
                },
            ],
            a,
        )
        .expect("组");
        let switcher =
            std::sync::Arc::new(crate::adapters::gstreamer::GStreamerSwitchAdapter::default());
        let runtime = ProgramExecutionRuntime::create(
            sid,
            group,
            switcher,
            Some(tap.clone()),
            vec![
                TapWiring {
                    input: h1,
                    channel: format!("tap-{a}"),
                },
                TapWiring {
                    input: h2,
                    channel: format!("tap-{b}"),
                },
            ],
        )
        .expect("Runtime 真接 tap 创建");
        assert!(runtime.is_active());
        assert_eq!(tap.tap_attachments(&h1).len(), 1, "A 管线 tap 真挂");
        assert_eq!(tap.tap_attachments(&h2).len(), 1, "B 管线 tap 真挂");

        runtime.teardown();
        assert!(!runtime.is_active());
        assert!(tap.tap_attachments(&h1).is_empty(), "teardown 真摘 A");
        assert!(tap.tap_attachments(&h2).is_empty(), "teardown 真摘 B");
        let _ = bundle.backend.stop(&h1);
        let _ = bundle.backend.stop(&h2);
    }
}
