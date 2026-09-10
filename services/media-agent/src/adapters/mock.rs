//! Phase 0.6 C3 + Final Merge Hardening (P0-2/P1-2/P1-2): Mock Reference Adapter.
//!
//! **目标**：在不引用 BMD SDK / GStreamer 的前提下实现 SPI，证明 `HardwareProvider` / `MediaBackend`
//! 可由非 vendor 的 Reference Adapter 满足，从而解锁 ARCH-PORTABILITY-01 的 Mock 侧：
//! - Test B：Mock 与 GStreamer 后端共享同一 Graph / Session / Supervisor / Health；
//! - Test C：换 `MockProviderB` 不影响 Domain / Graph / UI 语义（任意两个 `HardwareProvider`
//!   实现按 SPI 可互换）。
//!
//! **hardening 变更**：
//! - `discover -> Result<Vec<DiscoveredDevice>, ProviderError>` (P1-2, SPI 统一);
//! - `MediaBackend` 方法对齐冻结契约 (`instantiate`/`observe`/`stop`) (P0-2);
//! - Mock 句柄走生产同源 `NEXT_PIPELINE_ID` (P1-1, 从 1 起, 绝不再取 `PipelineHandle(0)` 哨兵)。
//!
//! 仅在 `mock` feature 下编译（`cargo build --features mock`），不拉 GStreamer、不依赖真实硬件。

use uuid::Uuid;

use crate::contracts::backend::MediaBackend;
use crate::contracts::provider::{
    CapabilityReport, ConnectorConfig, DiscoveredDevice, HardwareProvider, ProviderError,
    ProviderIdentity,
};
use crate::device::{DeviceIdentitySource, DeviceInfo, IdentityStrength};
use crate::pipeline::{PipelineError, PipelineHandle, PipelinePlan};
use crate::pipeline_events::PipelineBusEvent;
use crate::port::DeviceCapabilities;

/// Mock 确定性 UUID 命名空间（与 FS/SIM/BMD 命名空间区分，避免设备 ID 漂移）。
const VBMF_MOCK_NS: Uuid = Uuid::from_u128(0x3c4d5e6f_7081_4290_bc1d_3e4f5a6b7c8d);

/// 构造一个合成发现结果（测试世界，绝不以 hash/合成值伪造真实硬件持久标识——
/// 证据进入 `ProviderIdentity{provider:"mock"}`, Domain 只见 `IdentitySource::Simulation`）。
fn mock_device(
    suffix: &str,
    model: &str,
    serial: &str,
    inputs: u64,
    outputs: u64,
) -> DiscoveredDevice {
    DiscoveredDevice {
        device: DeviceInfo {
            device_id: Uuid::new_v5(&VBMF_MOCK_NS, format!("vbmf:mock:{suffix}").as_bytes()),
            model: model.into(),
            display_name: suffix.into(),
            serial_number: Some(serial.into()),
            video_input_connections: inputs,
            video_output_connections: outputs,
            identity_strength: IdentityStrength::Enumeration,
            identity_source: DeviceIdentitySource::Simulation,
            capabilities: DeviceCapabilities::default(),
            ports: Vec::new(),
        },
        identity: Some(ProviderIdentity {
            provider: "mock",
            persistent_id: None,
            device_handle: Some(format!("mock-handle-{suffix}")),
            topological_id: None,
        }),
    }
}

// ── MockProvider A ────────────────────────────────────────────────────────────
/// Mock Provider（A）：1 路 SDI 采集，单设备。用于 Test B/C 的基准 Provider。
pub struct MockProvider;

impl HardwareProvider for MockProvider {
    fn discover(&self) -> Result<Vec<DiscoveredDevice>, ProviderError> {
        Ok(vec![mock_device(
            "mock0",
            "mock-sdi-capture",
            "MOCK-A-0001",
            1,
            0,
        )])
    }
    fn probe_capabilities(&self) -> Vec<CapabilityReport> {
        vec![CapabilityReport {
            source: "mock-a".into(),
            items: vec!["sdi-capture".into()],
        }]
    }
    fn probe_connector_config(&self) -> ConnectorConfig {
        ConnectorConfig {
            connectors: vec!["sdi".into()],
        }
    }
}

// ── MockProvider B ────────────────────────────────────────────────────────────
/// Mock Provider（B）：2 设备（SDI + HDMI），拓扑与 A 不同。Test C 用它替换 A，
/// 验证 Domain / Graph / UI 语义 schema 无需改动即可切换 Provider 实现。
pub struct MockProviderB;

impl HardwareProvider for MockProviderB {
    fn discover(&self) -> Result<Vec<DiscoveredDevice>, ProviderError> {
        Ok(vec![
            mock_device("mock0", "mock-sdi-capture-b", "MOCK-B-0001", 1, 0),
            mock_device("mock1", "mock-hdmi-capture-b", "MOCK-B-0002", 1, 1),
        ])
    }
    fn probe_capabilities(&self) -> Vec<CapabilityReport> {
        vec![CapabilityReport {
            source: "mock-b".into(),
            items: vec!["sdi-capture".into(), "hdmi-capture".into()],
        }]
    }
    fn probe_connector_config(&self) -> ConnectorConfig {
        ConnectorConfig {
            connectors: vec!["sdi".into(), "hdmi".into()],
        }
    }
}

// ── MockBackend ───────────────────────────────────────────────────────────────
/// Mock Media Backend：不链接 GStreamer，instantiate/start/recover/stop 直接返回成功，
/// observe 返回空事件。证明 `MediaBackend` 可由非 GStreamer 实现满足（Test B）。
/// 句柄分配与生产同源 (`NEXT_PIPELINE_ID`, 从 1 起 — P1-1: 绝不取 `PipelineHandle(0)` 哨兵)。
pub struct MockBackend;

impl MediaBackend for MockBackend {
    fn instantiate(&self, _plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError> {
        Ok(PipelineHandle(
            crate::pipeline::NEXT_PIPELINE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        ))
    }
    fn start(&self, _handle: &PipelineHandle) -> Result<(), PipelineError> {
        Ok(())
    }
    fn stop(&self, _handle: &PipelineHandle) -> Result<(), PipelineError> {
        Ok(())
    }
    fn recover(&self, _handle: &PipelineHandle) -> Result<(), PipelineError> {
        Ok(())
    }
    fn observe(&self, _handle: &PipelineHandle) -> Vec<PipelineBusEvent> {
        Vec::new()
    }
}

/// A2-8-02: Mock MediaTap——确定性簿记实现（attach/detach/查询,
/// 供 02-D recover 重放逻辑的契约级验证）。
#[derive(Default)]
pub struct MockMediaTapPort {
    taps: std::sync::Mutex<
        std::collections::HashMap<
            PipelineHandle,
            Vec<crate::contracts::media_tap::MediaTapAttachment>,
        >,
    >,
    /// A2-8-02-G/H: 桥观测仿真 tick（每次查询推进——确定性递增流）。
    bridge_tick: std::sync::atomic::AtomicU64,
    /// G/H-1: 桥停滞注入集合（liveness 测试钩子）。
    bridge_stalled: std::sync::Mutex<std::collections::HashSet<String>>,
}

impl MockMediaTapPort {
    pub fn new() -> Self {
        Self::default()
    }
}

/// A2-8-02-G/H: Mock 桥观测——确定性仿真（每次查询 tick+1: 帧计数/
/// PTS 递增/ValidMonotonic——attached channel 各一行; 摘除即无行）。
impl crate::contracts::media_tap::BridgeObservationPort for MockMediaTapPort {
    fn bridge_observations(
        &self,
        handle: &PipelineHandle,
    ) -> Vec<crate::contracts::media_tap::BridgeObservation> {
        use crate::contracts::media_tap::BridgeObservation;
        use crate::pipeline::PtsMonotonicity;
        let tick = self
            .bridge_tick
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        let taps = self.taps.lock().unwrap();
        taps.get(handle)
            .map(|rows| {
                rows.iter()
                    .map(|a| BridgeObservation {
                        channel: a.channel.clone(),
                        video_last_pts: Some(1000 + tick * 40),
                        audio_last_pts: Some(800 + tick * 20),
                        video_pts_state: PtsMonotonicity::ValidMonotonic,
                        audio_pts_state: PtsMonotonicity::ValidMonotonic,
                        video_frames: tick * 25,
                        audio_frames: tick * 50,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// G/H-1: Mock liveness——attached 即视为窗口内流通（观察时钟仿真:
    /// last_observed=now; 独立 stall 注入见 `bridge_stall`）。
    fn bridge_liveness(
        &self,
        handle: &PipelineHandle,
        _window_ms: u64,
    ) -> Vec<crate::contracts::media_tap::BridgeChannelLiveness> {
        use crate::contracts::media_tap::BridgeChannelLiveness;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let taps = self.taps.lock().unwrap();
        let stalled = self.bridge_stalled.lock().unwrap();
        taps.get(handle)
            .map(|rows| {
                rows.iter()
                    .map(|a| BridgeChannelLiveness {
                        channel: a.channel.clone(),
                        frames: 100,
                        last_observed_at_ms: Some(if stalled.contains(&a.channel) {
                            now_ms.saturating_sub(10_000) // 远超任何窗口
                        } else {
                            now_ms
                        }),
                        alive_in_window: !stalled.contains(&a.channel),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl MockMediaTapPort {
    /// G/H-1 测试钩子: 注入桥停滞（liveness 窗口外——当前断流仿真）。
    pub fn bridge_stall(&self, handle: &PipelineHandle, channel: &str) {
        let _ = handle;
        self.bridge_stalled
            .lock()
            .unwrap()
            .insert(channel.to_string());
    }
}

impl crate::contracts::media_tap::MediaTapPort for MockMediaTapPort {
    fn attach_media_tap(
        &self,
        handle: &PipelineHandle,
        req: &crate::contracts::media_tap::MediaTapRequest,
    ) -> Result<(), crate::contracts::media_tap::TapError> {
        use crate::contracts::media_tap::{MediaTapAttachment, TapError};
        let mut taps = self.taps.lock().unwrap();
        let rows = taps.entry(*handle).or_default();
        if rows.iter().any(|a| a.channel == req.channel) {
            return Err(TapError::AlreadyAttached(req.channel.clone()));
        }
        rows.push(MediaTapAttachment {
            channel: req.channel.clone(),
            planes: req.planes,
        });
        Ok(())
    }

    fn detach_media_tap(
        &self,
        handle: &PipelineHandle,
        channel: &str,
    ) -> Result<(), crate::contracts::media_tap::TapError> {
        use crate::contracts::media_tap::TapError;
        let mut taps = self.taps.lock().unwrap();
        let rows = taps
            .get_mut(handle)
            .ok_or(TapError::NotAttached(channel.into()))?;
        let before = rows.len();
        rows.retain(|a| a.channel != channel);
        if rows.len() == before {
            return Err(TapError::NotAttached(channel.into()));
        }
        Ok(())
    }

    fn tap_attachments(
        &self,
        handle: &PipelineHandle,
    ) -> Vec<crate::contracts::media_tap::MediaTapAttachment> {
        self.taps
            .lock()
            .unwrap()
            .get(handle)
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_provider_a_discovers_single_device() {
        let devices = MockProvider.discover().expect("mock discover 应成功");
        assert_eq!(devices.len(), 1);
        assert_eq!(
            devices[0].device.identity_source,
            DeviceIdentitySource::Simulation
        );
        assert_eq!(devices[0].device.video_input_connections, 1);
        // 确定性：同输入应得同 device_id（无随机漂移）。
        let again = MockProvider.discover().expect("mock discover 应成功");
        assert_eq!(devices[0].device.device_id, again[0].device.device_id);
    }

    #[test]
    fn mock_provider_b_discovers_two_devices() {
        let devices = MockProviderB.discover().expect("mock discover 应成功");
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].device.model, "mock-sdi-capture-b");
        assert_eq!(devices[1].device.video_output_connections, 1);
    }

    #[test]
    fn mock_backend_lifecycle_ok() {
        let backend = MockBackend;
        let plan = PipelinePlan::self_test();
        let handle = backend.instantiate(&plan).expect("instantiate 应成功");
        // P1-1: 句柄与生产同源分配, 绝不为 0 哨兵。
        assert_ne!(handle, PipelineHandle(0));
        backend.start(&handle).expect("start 应成功");
        backend.recover(&handle).expect("recover 应成功");
        backend.stop(&handle).expect("stop 应成功");
        assert!(backend.observe(&handle).is_empty());
    }

    #[test]
    fn media_tap_rt_01_attach_records_bookkeeping() {
        // 02-B: attach → 簿记唯一事实源（恰一行, channel/planes 保真）。
        use crate::contracts::media_tap::{MediaTapPort, MediaTapRequest, TapPlanes};
        let tap = MockMediaTapPort::new();
        let h = PipelineHandle(424_242);
        tap.attach_media_tap(
            &h,
            &MediaTapRequest {
                channel: "dev-a-raw".into(),
                planes: TapPlanes::Both,
            },
        )
        .expect("attach 应成功");
        assert_eq!(
            tap.tap_attachments(&h),
            vec![crate::contracts::media_tap::MediaTapAttachment {
                channel: "dev-a-raw".into(),
                planes: TapPlanes::Both,
            }],
            "簿记恰一行且保真"
        );
        assert!(
            tap.tap_attachments(&PipelineHandle(1)).is_empty(),
            "无关管线零簿记"
        );
    }

    #[test]
    fn media_tap_rt_01_double_attach_fail_closed() {
        use crate::contracts::media_tap::{MediaTapPort, MediaTapRequest, TapError, TapPlanes};
        let tap = MockMediaTapPort::new();
        let h = PipelineHandle(424_243);
        let req = MediaTapRequest {
            channel: "dev-b-raw".into(),
            planes: TapPlanes::Video,
        };
        tap.attach_media_tap(&h, &req).expect("首次 attach");
        assert_eq!(
            tap.attach_media_tap(&h, &req),
            Err(TapError::AlreadyAttached("dev-b-raw".into())),
            "同 channel 重复 attach fail-closed（不静默重定义）"
        );
        // 不同 channel 可并存（双平面分列属合法簿记形态）。
        tap.attach_media_tap(
            &h,
            &MediaTapRequest {
                channel: "dev-b-raw-audio".into(),
                planes: TapPlanes::Audio,
            },
        )
        .expect("异 channel 并存");
        assert_eq!(tap.tap_attachments(&h).len(), 2);
    }

    #[test]
    fn media_tap_rt_01_detach_removes_and_unknown_rejected() {
        use crate::contracts::media_tap::{MediaTapPort, MediaTapRequest, TapError, TapPlanes};
        let tap = MockMediaTapPort::new();
        let h = PipelineHandle(424_244);
        tap.attach_media_tap(
            &h,
            &MediaTapRequest {
                channel: "dev-c-raw".into(),
                planes: TapPlanes::Both,
            },
        )
        .expect("attach");
        tap.detach_media_tap(&h, "dev-c-raw").expect("detach");
        assert!(tap.tap_attachments(&h).is_empty(), "摘除后簿记清空");
        assert_eq!(
            tap.detach_media_tap(&h, "dev-c-raw"),
            Err(TapError::NotAttached("dev-c-raw".into())),
            "重复 detach 拒收"
        );
    }

    #[test]
    fn media_tap_rt_01_bookkeeping_is_replay_source() {
        // 02-D 契约级预演（C2 裁定形式）: 簿记=恢复重放唯一事实源——
        // 模拟 recover 丢失 tap（detach 全部）后, 仅凭 tap_attachments()
        // 快照重放 attach → 能力恢复。recover 内"裸调 attach"禁令的
        // 替代路径即此: 簿记驱动重放。
        use crate::contracts::media_tap::{MediaTapPort, MediaTapRequest, TapPlanes};
        let tap = MockMediaTapPort::new();
        let h = PipelineHandle(424_245);
        tap.attach_media_tap(
            &h,
            &MediaTapRequest {
                channel: "dev-d-raw".into(),
                planes: TapPlanes::Both,
            },
        )
        .expect("attach");
        // 快照（恢复前唯一可得事实）。
        let snapshot = tap.tap_attachments(&h);
        // 模拟 recover: 管线重建, tap 全失。
        for a in &snapshot {
            tap.detach_media_tap(&h, &a.channel).expect("模拟丢失");
        }
        assert!(tap.tap_attachments(&h).is_empty());
        // 仅凭快照重放（02-D controller 恢复钩的契约依据）。
        for a in &snapshot {
            tap.attach_media_tap(
                &h,
                &MediaTapRequest {
                    channel: a.channel.clone(),
                    planes: a.planes,
                },
            )
            .expect("簿记重放 attach");
        }
        assert_eq!(tap.tap_attachments(&h), snapshot, "重放后簿记等值恢复");
    }
}
