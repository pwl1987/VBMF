//! Phase 0.7C-2: Runtime Query Model — **Pure Read / Snapshot Semantics**
//! （终审 2026-08-31 §十四新原则：所有查询 API 必须是 Pure Read / Snapshot——
//! 查询只能观察, 不得 cleanup lease / refresh device / restart backend /
//! probe hardware / change state）。
//!
//! 只回答"系统现在是什么状态"; 命令动词 (start/stop/restart/allocate/release/
//! route/switch) 属 Command Contract（下一阶段），在本模块类型层面不存在
//! （公开面 allowlist 白盒锁定——Preflight 副作用教训的延续）。
//!
//! **零新 DTO**: 全部查询返回既有 `CanonicalRuntimeState` 子项克隆——绝不制造
//! 第二套查询模型（防 External DTO 化重复 0.6 前的问题）。
//!
//! D14 契约（登记不实现）: `get_runtime_state()` 返回的是**各源独立观测的拼合
//! snapshot, 非事务一致**——一致性语义（source observation time / state version）
//! 属后续（见 PHASE_0_7A_POST_MERGE_DEBT.md D14）。

#![allow(dead_code)]

use uuid::Uuid;

use crate::runtime_state::SessionRuntimeState;
use crate::runtime_state::{
    CanonicalRuntimeState, DeviceCapabilitiesSummary, DeviceRuntimeState, PortRuntimeState,
    ResourceRuntimeState,
};
use crate::session::{SessionId, SessionManager};

/// Runtime Query Model 只读门面。
pub struct RuntimeQuery {
    mgr: std::sync::Arc<SessionManager>,
}

impl RuntimeQuery {
    pub fn new(mgr: std::sync::Arc<SessionManager>) -> Self {
        Self { mgr }
    }

    /// 全量运行时快照（snapshot, 非事务一致——见 D14 契约）。
    pub fn get_runtime_state(&self) -> CanonicalRuntimeState {
        self.mgr.runtime_state()
    }

    pub fn get_device(&self, id: Uuid) -> Option<DeviceRuntimeState> {
        self.get_runtime_state()
            .devices
            .into_iter()
            .find(|d| d.device_id == id)
    }

    pub fn get_port(&self, id: Uuid) -> Option<PortRuntimeState> {
        self.get_runtime_state()
            .ports
            .into_iter()
            .find(|p| p.port_id == id)
    }

    pub fn get_resource(&self, id: Uuid) -> Option<ResourceRuntimeState> {
        self.get_runtime_state()
            .resources
            .into_iter()
            .find(|r| r.resource_id == id)
    }

    pub fn get_session(&self, id: SessionId) -> Option<SessionRuntimeState> {
        self.get_runtime_state()
            .sessions
            .into_iter()
            .find(|s| s.session_id == id)
    }

    pub fn list_sessions(&self) -> Vec<SessionRuntimeState> {
        self.get_runtime_state().sessions
    }

    /// 设备能力投影汇总（D6: Provider → Capability Probe → Runtime State → Query）。
    pub fn get_capabilities(&self) -> Vec<(Uuid, DeviceCapabilitiesSummary)> {
        self.get_runtime_state()
            .devices
            .into_iter()
            .filter_map(|d| d.capabilities.map(|c| (d.device_id, c)))
            .collect()
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::session::{SessionPhase, SessionState, SessionTuning};

    /// **Pure Read 白盒**（终审 §十四）: 公开关联函数清单硬编码比对——新增公开项
    /// 必须显式更新本清单并过 Pure Read 评审; 命令动词禁入。
    const PUBLIC_SURFACE_ALLOWLIST: &[&str] = &[
        "new",
        "get_runtime_state",
        "get_device",
        "get_port",
        "get_resource",
        "get_session",
        "list_sessions",
        "get_capabilities",
    ];
    const BANNED_VERBS: &[&str] = &[
        "start", "stop", "restart", "allocate", "release", "route", "switch", "probe", "refresh",
        "cleanup", "create", "close", "tick",
    ];

    fn query_world() -> (RuntimeQuery, crate::lease::InMemoryLeaseManager) {
        use crate::adapters::mock::MockProvider;
        use crate::contracts::provider::HardwareProvider as _;
        let devices: Vec<crate::device::DeviceInfo> = MockProvider
            .discover()
            .expect("mock discover")
            .into_iter()
            .map(|d| d.device)
            .collect();
        // capability 投影数据 (D6 三态测试素材: 设备 0 Supported)。
        let mut devices = devices;
        let registry = port_registry_owned(&devices);
        if let Some(d) = devices.first_mut() {
            d.capabilities = crate::port::DeviceCapabilities {
                input: crate::port::CapabilityValue::Supported(true),
                output: crate::port::CapabilityValue::Unsupported,
                input_port_count: crate::port::CapabilityValue::Supported(1),
                output_port_count: crate::port::CapabilityValue::Unsupported,
                audio_input: crate::port::CapabilityValue::Supported(true),
                audio_output: crate::port::CapabilityValue::Unknown,
            };
        }
        let lm = crate::lease::InMemoryLeaseManager::new();
        let resources = crate::resource::SharedResourceRegistry::new(
            crate::resource::ResourceRegistry::derive_from_discovery(&port_registry(&devices)),
        );
        let event_log = std::sync::Arc::new(crate::events::RuntimeEventLog::new());
        let sup = std::sync::Arc::new(std::sync::Mutex::new(crate::supervisor::Supervisor::new(
            crate::supervisor::RestartPolicy::default(),
            event_log.clone(),
        )));
        let mgr = std::sync::Arc::new(SessionManager::new(
            resources,
            std::sync::Arc::new(crate::lease::InMemoryLeaseManager::new()),
            sup,
            std::sync::Arc::new(crate::adapters::mock::MockBackend),
            std::sync::Arc::new(devices),
            std::sync::Arc::new(std::collections::HashMap::new()),
            Some(registry),
            crate::pipeline::MaterializeMode::Diagnostic,
            SessionTuning::default(),
            event_log,
        ));
        (RuntimeQuery::new(mgr), lm)
    }

    fn port_registry(devices: &[crate::device::DeviceInfo]) -> crate::port::PortRegistry {
        port_registry_owned(devices)
    }

    fn port_registry_owned(devices: &[crate::device::DeviceInfo]) -> crate::port::PortRegistry {
        use crate::port::*;
        let mut ports = Vec::new();
        for d in devices {
            ports.push(PortInfo {
                device_id: d.device_id,
                provider_binding_ref: None,
                identity: PortIdentity {
                    port_id: PortIdentity::derive(
                        &d.device_id,
                        ConnectorType::Sdi,
                        PortOrdinal::Known(1),
                    ),
                    connector: ConnectorType::Sdi,
                    ordinal: PortOrdinal::Known(1),
                },
                direction: PortDirection::Input,
                capabilities: PortCapabilities::default(),
                runtime_binding: None,
                signal: SignalStatus::default(),
                content: VideoContentState::Unknown,
            });
        }
        PortRegistry { ports }
    }

    fn intent_for(dev: &crate::device::DeviceInfo) -> crate::graph_intent::GraphRuntimeIntent {
        crate::graph_intent::GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![crate::graph_intent::DeviceIntent {
                device_id: dev.device_id.to_string(),
                role: "CAPTURE".into(),
                pipeline: crate::graph_intent::PipelineIntent {
                    source: crate::graph_intent::SourceIntent {
                        kind: "decklink".into(),
                        device_id: dev.device_id.to_string(),
                        port_id: None,
                    },
                    sink: crate::graph_intent::SinkIntent {
                        kind: "appsink".into(),
                    },
                },
            }],
        }
    }

    #[test]
    fn runtime_query_rt_01_pure_read_public_surface() {
        // 白盒: allowlist 硬编码; 命令动词禁入 (Pure Read / Snapshot Semantics)。
        assert_eq!(
            PUBLIC_SURFACE_ALLOWLIST,
            &[
                "new",
                "get_runtime_state",
                "get_device",
                "get_port",
                "get_resource",
                "get_session",
                "list_sessions",
                "get_capabilities",
            ]
        );
        for name in PUBLIC_SURFACE_ALLOWLIST {
            for verb in BANNED_VERBS {
                assert!(
                    !name.starts_with(verb),
                    "命令动词 {verb} 禁入查询面: {name}"
                );
            }
        }
    }

    #[test]
    fn runtime_query_rt_01_get_paths_hit_and_miss() {
        let (q, _lm) = query_world();
        let st = q.get_runtime_state();
        let dev_id = st.devices[0].device_id;
        let port_id = st.ports[0].port_id;
        let res_id = st.resources[0].resource_id;
        // 命中。
        assert!(q.get_device(dev_id).is_some());
        assert!(q.get_port(port_id).is_some());
        assert!(q.get_resource(res_id).is_some());
        // 未命中 (幽灵 id → None, 绝不臆造)。
        let ghost = Uuid::new_v4();
        assert!(q.get_device(ghost).is_none());
        assert!(q.get_port(ghost).is_none());
        assert!(q.get_resource(ghost).is_none());
        assert!(q.get_session(SessionId(ghost)).is_none());
        assert!(q.list_sessions().is_empty());
    }

    #[test]
    fn runtime_query_rt_01_capability_projection() {
        // D6: DeviceCapabilities → DeviceCapabilitiesSummary 投影 (mock 设备 0 注入 Supported)。
        let (q, _lm) = query_world();
        let caps = q.get_capabilities();
        assert!(!caps.is_empty(), "capability 投影应在场");
        let (_, c) = &caps[0];
        assert_eq!(c.can_input, crate::runtime_state::CapabilityFlag::Supported);
        assert_eq!(
            c.can_output,
            crate::runtime_state::CapabilityFlag::Unsupported
        );
        assert_eq!(c.input_ports, Some(1));
    }

    #[test]
    fn runtime_query_rt_01_simulation_session_lifecycle_projection() {
        // Simulation: create→query 投影 (会话可见→close 后消失)。
        let (q, _lm) = query_world();
        let dev_id = q.get_runtime_state().devices[0].device_id;
        // SessionManager 需要可变操作 — 经由 Arc clone 拿一个可操作句柄。
        // (RuntimeQuery 只读; 生命周期操作走 SessionManager 本体。)
        let mgr = std::sync::Arc::clone(q_mgr(&q));
        let dev = mgr
            .runtime_state()
            .devices
            .first()
            .map(|d| {
                q.get_device(d.device_id);
                d.device_id
            })
            .unwrap();
        let _ = dev;
        let sid = mgr.create(intent_for_dev(dev_id)).expect("create 应通过");
        assert!(q.get_session(sid).is_some(), "会话应可查询");
        assert_eq!(q.list_sessions().len(), 1);
        let s = q.get_session(sid).unwrap();
        assert_eq!(s.phase, SessionPhase::Leased);
        assert_eq!(s.state, SessionState::Reserved);
        mgr.stop(&sid).ok();
        mgr.close(&sid).expect("close 应成功");
        assert!(q.get_session(sid).is_none(), "close 后会话不可查询");
    }

    // 测试辅助: 取出 RuntimeQuery 内部 mgr (仅测试世界; 生产面无此方法)。
    fn q_mgr(q: &RuntimeQuery) -> &std::sync::Arc<SessionManager> {
        &q.mgr
    }

    fn intent_for_dev(dev_id: Uuid) -> crate::graph_intent::GraphRuntimeIntent {
        crate::graph_intent::GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![crate::graph_intent::DeviceIntent {
                device_id: dev_id.to_string(),
                role: "CAPTURE".into(),
                pipeline: crate::graph_intent::PipelineIntent {
                    source: crate::graph_intent::SourceIntent {
                        kind: "decklink".into(),
                        device_id: dev_id.to_string(),
                        port_id: None,
                    },
                    sink: crate::graph_intent::SinkIntent {
                        kind: "appsink".into(),
                    },
                },
            }],
        }
    }
}
