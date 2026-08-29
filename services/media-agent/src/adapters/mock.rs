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
fn mock_device(suffix: &str, model: &str, serial: &str, inputs: u64, outputs: u64) -> DiscoveredDevice {
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
        Ok(vec![mock_device("mock0", "mock-sdi-capture", "MOCK-A-0001", 1, 0)])
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
        Ok(PipelineHandle(crate::pipeline::NEXT_PIPELINE_ID.fetch_add(
            1,
            std::sync::atomic::Ordering::SeqCst,
        )))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_provider_a_discovers_single_device() {
        let devices = MockProvider.discover().expect("mock discover 应成功");
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device.identity_source, DeviceIdentitySource::Simulation);
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
}
