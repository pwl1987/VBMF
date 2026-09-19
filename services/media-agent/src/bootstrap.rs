//! A2-0 (A20-03, 用户裁定): Bootstrap — 唯一构造源（Dependency Construction）。
//!
//! **硬边界（用户 2026-09-02 裁定）: bootstrap 只构造, 不运行。**
//! 允许: Config / Provider / Discovery / EventLog+FanoutSink / LeaseManager
//! （含 bootstrap 占位租约与排他性自检——初始化态, 非媒体运行）/ Supervisor
//! 注册 / AgentState。
//! 禁止: Session::start / Pipeline::start / watchdog / recover / sleep /
//! Gate 断言 / process::exit（构造失败的 fail-closed 退出除外——无东西可运行）/ HTTP accept。
//!
//! 消费方（同源, 消灭"两套初始化语义"）:
//! - `bin/media-agent.rs`（生产 Composition Root）
//! - `bin/gates.rs`（Diagnostic Root——Gate 是 Consumer 不是 Bootstrapper）
//!
//! **不在本模块**（用户裁定: 诊断行为 ≠ 依赖构造）:
//! - SDK FFI probe（`probe_sdk`）——生产侧保留为 production diagnostic wiring
//!   （`bin/media-agent.rs` 内, 标记待后续独立变更裁决）; gate 侧在 `bin/gates.rs`。

use std::sync::Arc;

use crate::config::Config;
use crate::contracts::provider::{DiscoveredDevice, HardwareProvider};
use crate::device::DeviceInfo;
use crate::events::{RuntimeEventLog, RuntimeEventSink};
use crate::health::AgentState;
use crate::lease::{InMemoryLeaseManager, LeaseManager as _};
use crate::supervisor::Supervisor;

/// 共同构造件集合（仅两个入口都消费的对象; `provider` 在 build 内被 discover
/// 消费, 不留存——God Object 红线, 字段以实际使用为准）。
pub struct BootstrapContext {
    pub config: Config,
    pub discovered: Vec<DiscoveredDevice>,
    pub devices: Vec<DeviceInfo>,
    /// P0-7D D3 双日志: 投影（transport/gate 证据）与内消费（watchdog→reduce）。
    pub projection_log: Arc<RuntimeEventLog>,
    pub internal_log: Arc<RuntimeEventLog>,
    /// A2-8-03-01-B: internal 平面唯一 drain 边界（共享单实例——watchdog 族
    /// 经此消费, custody 在边界内全量恰一次累积; 生产线程不直接持 internal
    /// log——组合根接线级唯一 drain ownership, 非 Rust 类型系统绝对封锁
    /// （R45 复核纠偏: BootstrapContext.internal_log 仍公开, 禁未经 intake 直接 drain）。
    pub event_intake: Arc<std::sync::Mutex<crate::event_intake::InternalEventIntake>>,
    pub event_sink: Arc<dyn RuntimeEventSink>,
    /// bootstrap 占位租约已按设备全部持有（真实会话接管时让位——见各消费方）。
    pub lease_manager: Arc<InMemoryLeaseManager>,
    pub supervisor: Arc<std::sync::Mutex<Supervisor>>,
    pub agent_state: Arc<std::sync::Mutex<AgentState>>,
}

/// 唯一构造入口。构造失败 = fail-closed 进程退出（无媒体行为可泄漏）。
pub fn build() -> BootstrapContext {
    // Gate 2.1: load config shape from env.
    let config = Config::from_env();

    // P1-2 (用户 §二十二): RPC 绑定安全校验 — Rust 不负责 Auth, RPC 须 localhost/UDS.
    for w in config.rpc_bind_security_warnings() {
        tracing::warn!(warning = %w, "rpc_bind 安全");
    }

    // Gate 2.2: adapter 选择收口 (P0-4; mock > simulation > bmd-provider > default).
    let provider: Box<dyn HardwareProvider> = crate::registry::AdapterRegistry::build_provider()
        .unwrap_or_else(|e| {
            eprintln!("adapter feature 冲突 (fail-closed): {e}");
            std::process::exit(2);
        });
    let (active_provider, active_backend) = crate::registry::active_adapters();
    tracing::info!(
        provider = active_provider,
        backend = active_backend,
        mode = std::env::var("MEDIA_AGENT_MODE")
            .as_deref()
            .unwrap_or("production"),
        "adapter selection (P0-4 运维可见)"
    );
    // P1-2: discover fail-closed — SDK/驱动失败显式拒启, 绝不与"无设备"混淆.
    let discovered = match provider.discover() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("device discovery 失败 (fail-closed): {e}");
            std::process::exit(2);
        }
    };
    let devices: Vec<DeviceInfo> = discovered.iter().map(|d| d.device.clone()).collect();
    tracing::info!(count = devices.len(), "device discovery complete");

    // P0-7D D3 (双日志分流定稿): FanoutSink 同序双写, emit 永不阻塞永不失败。
    let projection_log = Arc::new(RuntimeEventLog::new());
    let internal_log = Arc::new(RuntimeEventLog::new());
    let event_sink: Arc<dyn RuntimeEventSink> = Arc::new(crate::events::FanoutSink::new(
        projection_log.clone(),
        internal_log.clone(),
    ));

    // A2-8-03-01-B: internal 平面唯一 drain 边界（BS-01 构造源; 唯一事实
    // 消费点——R44 §5/§8 裁决方向）。
    let event_intake = Arc::new(std::sync::Mutex::new(
        crate::event_intake::InternalEventIntake::new(internal_log.clone()),
    ));

    // Gate 2.3: lease manager + bootstrap 占位租约（初始化态; 真实会话让位语义在各消费方）。
    let lease_manager = Arc::new(InMemoryLeaseManager::new());
    for d in &devices {
        match lease_manager.acquire(
            &d.device_id,
            "bootstrap",
            std::time::Duration::from_secs(60),
        ) {
            Ok(l) => tracing::info!(device = %l.device_id, "lease acquired"),
            Err(e) => tracing::warn!(error = %e, "lease acquire failed"),
        }
    }
    // 排他性不变量自检: 同一设备重复 acquire 必须被拒 (防 host ffmpeg / 双采).
    if let Some(first) = devices.first() {
        match lease_manager.acquire(
            &first.device_id,
            "second-owner",
            std::time::Duration::from_secs(60),
        ) {
            Ok(_) => tracing::warn!("LEASE COLLISION — double-capture risk!"),
            Err(e) => tracing::info!(error = %e, "lease re-acquire correctly rejected"),
        }
    }

    // Gate 5: Supervisor 注册设备（只决策, 不碰 GStreamer; 持 FanoutSink）。
    let supervisor = Arc::new(std::sync::Mutex::new(Supervisor::new(
        crate::supervisor::RestartPolicy::default(),
        event_sink.clone(),
    )));
    for d in &devices {
        supervisor.lock().unwrap().register(d.device_id);
    }
    tracing::info!(watched = devices.len(), "supervisor initialized");

    let agent_state = Arc::new(std::sync::Mutex::new(AgentState::Ready));

    BootstrapContext {
        config,
        discovered,
        devices,
        projection_log,
        internal_log,
        event_intake,
        event_sink,
        lease_manager,
        supervisor,
        agent_state,
    }
}

#[cfg(feature = "ffmpeg-backend")]
pub struct FfmpegSessionComposition {
    /// Canonical Session lifecycle owner. Production control commands must enter here.
    pub manager: Arc<crate::session::SessionManager>,
    /// Backend instance injected into SessionManager; exposed for
    /// acceptance/observation only, not as a second lifecycle owner.
    pub backend: Arc<dyn crate::contracts::backend::MediaBackend>,
    /// Acceptance-only process observation view of the same backend instance.
    pub process_inspector: Arc<dyn crate::contracts::backend::BackendProcessInspector>,
    /// Backend-neutral Port/Resource authorization view.
    pub registry: crate::port::PortRegistry,
    pub authorizations:
        Arc<std::collections::HashMap<uuid::Uuid, crate::resolver::BindingAuthorization>>,
}

/// RF-FF-01E: construct the production FFmpeg Session path without starting media.
///
/// This is dependency construction only: manifest + live Provider identity are
/// validated fail-closed, resources are derived from the neutral PortRegistry,
/// and the existing SessionManager remains the sole lifecycle owner.
#[cfg(feature = "ffmpeg-backend")]
pub fn build_ffmpeg_session_composition(
    world: &BootstrapContext,
) -> Result<FfmpegSessionComposition, String> {
    let manifest_path = world.config.device_binding_path.as_deref().ok_or_else(|| {
        "RF-FF-01E production FFmpeg requires MEDIA_AGENT_DEVICE_BINDING".to_string()
    })?;
    let manifest = crate::resolver::DeviceBindingManifest::load(manifest_path)?;
    manifest.validate_manifest()?;
    let runtime_machine_id = crate::resolver::current_machine_id();
    manifest.check_machine_identity(&runtime_machine_id)?;

    let authorizations = crate::resolver::collect_authorizations_from_manifest(
        &world.discovered,
        &manifest,
        &runtime_machine_id,
    )?;
    let registry = crate::port::PortRegistry::build_authorized(&world.discovered, &manifest)
        .map_err(|e| format!("RF-FF-01E authorized PortRegistry build failed: {e:?}"))?;
    let resources = crate::resource::SharedResourceRegistry::new(
        crate::resource::ResourceRegistry::derive_from_discovery(&registry),
    );
    let (backend, process_inspector) =
        crate::registry::AdapterRegistry::build_backend_with_manifest(
            &world.discovered,
            &manifest,
        )?;
    let authorizations = Arc::new(authorizations);
    let manager = Arc::new(crate::session::SessionManager::new(
        resources,
        world.lease_manager.clone(),
        world.supervisor.clone(),
        backend.clone(),
        Arc::new(world.devices.clone()),
        authorizations.clone(),
        None,
        Some(registry.clone()),
        crate::pipeline::MaterializeMode::Production,
        crate::session::SessionTuning {
            default_lease_ttl: world.config.default_lease_ttl,
            lease_renew_window: world.config.lease_renew_window,
            ..crate::session::SessionTuning::default()
        },
        world.event_sink.clone(),
    ));

    Ok(FfmpegSessionComposition {
        manager,
        backend,
        process_inspector,
        registry,
        authorizations,
    })
}

/// RF-SRC-RTMP-02 TG-2: production network-only composition result.
///
/// Same runtime primitives as the device composition (single SessionManager
/// lifecycle ownership, in-memory lease manager, Supervisor, FanoutSink) —
/// but constructed WITHOUT device discovery, WITHOUT the
/// DeviceBindingManifest and WITHOUT bootstrap device leases (plan D10).
#[cfg(feature = "ffmpeg-backend")]
pub struct FfmpegNetworkComposition {
    /// Canonical Session lifecycle owner. Production control commands must enter here.
    pub manager: Arc<crate::session::SessionManager>,
    /// Backend instance injected into SessionManager; acceptance/observation only.
    pub backend: Arc<dyn crate::contracts::backend::MediaBackend>,
    /// Acceptance-only process observation view of the same backend instance.
    pub process_inspector: Arc<dyn crate::contracts::backend::BackendProcessInspector>,
    /// Same Supervisor instance wired into the manager (acceptance/recovery
    /// wiring; NOT a second decision owner).
    pub supervisor: Arc<std::sync::Mutex<Supervisor>>,
    /// Same lease-manager instance wired into the manager (acceptance
    /// assertions only; lifecycle stays owned by the manager).
    pub lease_manager: Arc<InMemoryLeaseManager>,
    /// Same resource registry wired into the manager (TG-4 recovery claim
    /// revalidation input; read-only observation, not a second truth).
    pub resources: Arc<crate::resource::SharedResourceRegistry>,
    /// Startup-loaded, D5-verified binding — the sole Network authorization
    /// truth for this manager (plan D3/D11).
    pub binding: Arc<crate::network_binding::NetworkSourceBinding>,
}

/// Build the production network-only composition from configuration.
///
/// The `NetworkSourceBinding` manifest is loaded exactly once here via the
/// production entry (`getifaddrs` D5 snapshot + machine pin + 0600/race-free
/// single-fd checks — all fail-closed; plan D3/D5). There is no reload, no
/// watch and no second endpoint truth; any manifest or interface change
/// requires a service restart.
#[cfg(feature = "ffmpeg-backend")]
pub fn build_ffmpeg_network_only_composition() -> Result<FfmpegNetworkComposition, String> {
    let config = Config::from_env();
    let binding_path = config.network_binding_path.as_deref().ok_or_else(|| {
        "RF-SRC-RTMP-02 production network composition requires MEDIA_AGENT_NETWORK_BINDING (fail-closed)"
            .to_string()
    })?;
    let binding =
        crate::network_binding::NetworkSourceBinding::load(std::path::Path::new(binding_path))
            .map_err(|e| format!("network binding manifest rejected (fail-closed): {e}"))?;
    build_ffmpeg_network_only_composition_with(&config, Arc::new(binding))
}

/// Deterministic core with a pre-verified binding (test seam; production
/// goes through [`build_ffmpeg_network_only_composition`]).
///
/// Plan D10 invariants hold by construction: no provider discovery, no
/// `DeviceBindingManifest`, no bootstrap placeholder device leases; only
/// Network `rtmp-input` Resources — one per authorized source — are
/// registered, and only `LeaseKey::Network` can ever be acquired through
/// this manager.
#[cfg(feature = "ffmpeg-backend")]
pub fn build_ffmpeg_network_only_composition_with(
    config: &Config,
    binding: Arc<crate::network_binding::NetworkSourceBinding>,
) -> Result<FfmpegNetworkComposition, String> {
    let authorized = binding.authorized_sources();
    let mut resources = crate::resource::ResourceRegistry::new();
    for source_id in &authorized {
        resources.register_network_source(*source_id);
    }
    let resources = crate::resource::SharedResourceRegistry::new(resources);

    // Shared runtime primitives (same types as build()), fresh instances:
    // the network plane owns no device state.
    let projection_log = Arc::new(RuntimeEventLog::new());
    let internal_log = Arc::new(RuntimeEventLog::new());
    let event_sink: Arc<dyn RuntimeEventSink> = Arc::new(crate::events::FanoutSink::new(
        projection_log.clone(),
        internal_log.clone(),
    ));
    let lease_manager = Arc::new(InMemoryLeaseManager::new());
    let supervisor = Arc::new(std::sync::Mutex::new(Supervisor::new(
        crate::supervisor::RestartPolicy::default(),
        event_sink.clone(),
    )));
    for source_id in &authorized {
        // RecoveryMonitor uses the same Supervisor namespace for both source domains.
        supervisor.lock().unwrap().register(source_id.0);
    }

    let (backend, process_inspector) =
        crate::registry::AdapterRegistry::build_network_process_backend()?;
    let manager = Arc::new(crate::session::SessionManager::new(
        resources.clone(),
        lease_manager.clone(),
        supervisor.clone(),
        backend.clone(),
        // No devices by construction: network-only composition performs no
        // hardware discovery and cannot touch DeckLink input 0/1 or output
        // device-number 2.
        Arc::new(Vec::new()),
        // Device authorizations stay empty: Network admission NEVER flows
        // through the device BindingAuthorization map (plan D11).
        Arc::new(std::collections::HashMap::new()),
        Some(binding.clone()),
        // PortRegistry is a device-port concept; the network plane has none.
        None,
        crate::pipeline::MaterializeMode::Production,
        crate::session::SessionTuning {
            default_lease_ttl: config.default_lease_ttl,
            lease_renew_window: config.lease_renew_window,
            ..crate::session::SessionTuning::default()
        },
        event_sink,
    ));

    Ok(FfmpegNetworkComposition {
        manager,
        backend,
        process_inspector,
        supervisor,
        lease_manager,
        resources: Arc::new(resources),
        binding,
    })
}

#[cfg(all(test, feature = "ffmpeg-backend"))]
mod rf_ff_01e_tests {
    use super::*;
    use crate::contracts::provider::ProviderIdentity;
    use crate::device::{DeviceIdentitySource, IdentityStrength};
    use crate::events::FanoutSink;
    use crate::port::{ConnectorType, DeviceCapabilities, PortDirection, VerificationLevel};
    use crate::resolver::{BindingEntry, DeviceBindingManifest, PortBinding};
    use uuid::Uuid;

    fn manifest(machine_id: String, handle: &str) -> DeviceBindingManifest {
        DeviceBindingManifest {
            manifest_version: "2.0".into(),
            machine_id,
            generated_by: "rf-ff-01e-test".into(),
            generated_at: "2026-09-18T00:00:00Z".into(),
            bmd_sdk_version: None,
            gst_decklink_plugin_version: None,
            gst_runtime_version: None,
            notes: None,
            bindings: vec![BindingEntry {
                label: Some("SDI-IN-1".into()),
                bmd_device_handle: handle.into(),
                gst_device_number: 9,
                expected_hw_serial_number: None,
                expected_model: None,
                port: Some(PortBinding {
                    connector: ConnectorType::Sdi,
                    ordinal: 1,
                    direction: PortDirection::Input,
                    required: true,
                    verification: VerificationLevel::Declared,
                }),
            }],
        }
    }
    fn world(manifest_path: Option<String>, handle: &str) -> BootstrapContext {
        let device_id = Uuid::new_v5(&Uuid::nil(), handle.as_bytes());
        let device = DeviceInfo {
            device_id,
            model: "DeckLink test".into(),
            display_name: "test-input".into(),
            serial_number: None,
            video_input_connections: 0,
            video_output_connections: 0,
            identity_strength: IdentityStrength::DeviceHandle,
            identity_source: DeviceIdentitySource::RealBmd,
            capabilities: DeviceCapabilities::default(),
            ports: Vec::new(),
        };
        let discovered = vec![DiscoveredDevice {
            device: device.clone(),
            identity: Some(ProviderIdentity {
                provider: "blackmagic",
                persistent_id: None,
                device_handle: Some(handle.into()),
                topological_id: None,
            }),
        }];
        let projection_log = Arc::new(RuntimeEventLog::new());
        let internal_log = Arc::new(RuntimeEventLog::new());
        let event_sink: Arc<dyn RuntimeEventSink> = Arc::new(FanoutSink::new(
            projection_log.clone(),
            internal_log.clone(),
        ));
        let event_intake = Arc::new(std::sync::Mutex::new(
            crate::event_intake::InternalEventIntake::new(internal_log.clone()),
        ));
        let lease_manager = Arc::new(InMemoryLeaseManager::new());
        let supervisor = Arc::new(std::sync::Mutex::new(Supervisor::new(
            crate::supervisor::RestartPolicy::default(),
            event_sink.clone(),
        )));
        supervisor.lock().unwrap().register(device_id);
        BootstrapContext {
            config: Config {
                device_binding_path: manifest_path,
                ..Config::default()
            },
            discovered,
            devices: vec![device],
            projection_log,
            internal_log,
            event_intake,
            event_sink,
            lease_manager,
            supervisor,
            agent_state: Arc::new(std::sync::Mutex::new(AgentState::Ready)),
        }
    }

    fn write_manifest(m: &DeviceBindingManifest) -> String {
        let path = std::env::temp_dir().join(format!(
            "vbmf-rf-ff-01e-{}-{}.json",
            std::process::id(),
            Uuid::new_v4()
        ));
        std::fs::write(&path, serde_json::to_vec(m).unwrap()).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn rf_ff_01e_valid_manifest_builds_production_session_composition() {
        let handle = "46:test:01e";
        let runtime_machine_id = crate::resolver::current_machine_id();
        let manifest_machine_id = if runtime_machine_id.is_empty() {
            "test-host".to_string()
        } else {
            runtime_machine_id
        };
        let path = write_manifest(&manifest(manifest_machine_id, handle));
        let w = world(Some(path.clone()), handle);
        let composition =
            build_ffmpeg_session_composition(&w).expect("production FFmpeg composition");
        assert_eq!(composition.authorizations.len(), 1);
        assert_eq!(composition.registry.input_ports().len(), 1);
        assert!(composition
            .registry
            .ports
            .iter()
            .all(|p| p.runtime_binding.is_none()));
        let state = composition.manager.runtime_state();
        assert_eq!(state.resources.len(), 1);
        assert_eq!(state.sessions.len(), 0);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn rf_ff_01e_missing_manifest_fails_closed() {
        let w = world(None, "46:test:missing");
        let err = build_ffmpeg_session_composition(&w)
            .err()
            .expect("missing manifest must fail");
        assert!(err.contains("MEDIA_AGENT_DEVICE_BINDING"), "{err}");
    }

    #[test]
    fn rf_ff_01e_invalid_empty_machine_manifest_fails_closed() {
        let handle = "46:test:invalid-machine";
        let path = write_manifest(&manifest(String::new(), handle));
        let w = world(Some(path.clone()), handle);
        let err = build_ffmpeg_session_composition(&w)
            .err()
            .expect("empty manifest machine identity must fail");
        assert!(err.contains("machine_id"), "{err}");
        std::fs::remove_file(path).ok();
    }
}

#[cfg(all(test, feature = "ffmpeg-backend"))]
mod rf_src_rtmp_02_tg2_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::PathBuf;

    /// Minimal temp-dir helper (mirrors network_binding tests; no tempfile
    /// dev-dependency in this crate).
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "vbmf-tg2-{}-{}-{}",
                tag,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write_binding(dir: &TempDir, entries: &[(uuid::Uuid, &str)]) -> PathBuf {
        let entries_text = entries
            .iter()
            .map(|(id, url)| format!(r#"{{"source_id":"{id}","endpoint":"{url}"}}"#))
            .collect::<Vec<_>>()
            .join(",");
        let body = format!(r#"{{"version":1,"machine_id":"box-a","entries":[{entries_text}]}}"#);
        let path = dir.0.join("network-binding.json");
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        path
    }

    fn loaded_binding(
        path: &std::path::Path,
    ) -> std::sync::Arc<crate::network_binding::NetworkSourceBinding> {
        let local: Vec<std::net::IpAddr> = vec!["127.0.0.1".parse().unwrap()];
        std::sync::Arc::new(
            crate::network_binding::NetworkSourceBinding::load_verified(path, "box-a", &local)
                .expect("valid test binding"),
        )
    }

    fn rtmp_intent(
        source_id: crate::source::NetworkSourceId,
        port: u16,
    ) -> crate::graph_intent::GraphRuntimeIntent {
        crate::graph_intent::GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![crate::graph_intent::DeviceIntent {
                device_id: "network-source-node".into(),
                role: "CAPTURE".into(),
                pipeline: crate::graph_intent::PipelineIntent {
                    source: crate::graph_intent::SourceIntent::rtmp(
                        source_id,
                        crate::source::NetworkEndpoint {
                            protocol: crate::source::NetworkProtocol::Rtmp,
                            host: "127.0.0.1".into(),
                            port,
                            path: "/live/source".into(),
                        },
                    ),
                    sink: crate::graph_intent::SinkIntent {
                        kind: "appsink".into(),
                    },
                },
            }],
        }
    }

    #[test]
    fn rf_src_rtmp_02_network_only_composition_is_device_side_effect_free() {
        // Plan D10 (baseline-difference assertions): construction registers
        // ONLY Network Resources, holds zero Device leases, exposes no
        // devices, and never touches the binding file.
        let dir = TempDir::new("d10");
        let source_a = uuid::Uuid::new_v4();
        let source_b = uuid::Uuid::new_v4();
        let path = write_binding(
            &dir,
            &[
                (source_a, "rtmp://127.0.0.1:19350/live/a"),
                (source_b, "rtmp://127.0.0.1:19351/live/b"),
            ],
        );
        let before = std::fs::read(&path).unwrap();

        let composition =
            build_ffmpeg_network_only_composition_with(&Config::default(), loaded_binding(&path))
                .expect("network-only composition");

        let state = composition.manager.runtime_state();
        // zero device plane: no devices, no device-owned resources
        assert!(state.devices.is_empty(), "no discovery, no device plane");
        assert_eq!(
            state.resources.len(),
            2,
            "one Network Resource per authorized source"
        );
        for resource in &state.resources {
            assert_eq!(resource.capability, "rtmp-input");
            assert_eq!(resource.device_id, uuid::Uuid::nil());
            assert_eq!(resource.state, crate::resource::ResourceState::Available);
        }
        assert!(state.sessions.is_empty());
        // zero device leases (bootstrap placeholder leases never existed here)
        assert!(
            composition.lease_manager.list_active().is_empty(),
            "network-only composition must hold zero DeviceLease"
        );
        // binding file untouched by construction
        let after = std::fs::read(&path).unwrap();
        assert_eq!(before, after, "manifest digest must be unchanged");
    }

    #[test]
    fn rf_src_rtmp_02_network_only_composition_create_uses_five_tuple_only() {
        // D10 + D11 through a real create on the production composition:
        // exact five-tuple authorizes and takes ONLY LeaseKey::Network.
        // (No start here: under ffmpeg-backend a real start would spawn a
        // listener child — that belongs to TG-3/BMD gates, not this test.)
        let dir = TempDir::new("create");
        let source_id = uuid::Uuid::new_v4();
        let path = write_binding(&dir, &[(source_id, "rtmp://127.0.0.1:19350/live/source")]);
        let composition =
            build_ffmpeg_network_only_composition_with(&Config::default(), loaded_binding(&path))
                .expect("network-only composition");
        let sid = crate::source::NetworkSourceId(source_id);

        let session = composition
            .manager
            .create(rtmp_intent(sid, 19350))
            .expect("exact five-tuple authorizes");
        let status = composition
            .manager
            .status(&session)
            .expect("session exists");
        assert_eq!(status.runtime_leases.len(), 1);
        assert!(
            status.leases.is_empty(),
            "no DeviceLease on the network plane"
        );
        assert!(composition
            .lease_manager
            .is_key_active(&crate::source::LeaseKey::Network(sid)));
        assert!(composition.lease_manager.list_active().is_empty());
        let state = composition.manager.runtime_state();
        assert!(state.resources.iter().all(|r| r.capability == "rtmp-input"));
    }

    #[test]
    fn rf_src_rtmp_02_network_only_composition_rejects_mismatch_fail_closed() {
        // Same composition, fresh state: an endpoint mismatch fails closed —
        // no lease, its Network Resource stays Available, zero device leases.
        let dir = TempDir::new("mismatch");
        let source_id = uuid::Uuid::new_v4();
        let path = write_binding(&dir, &[(source_id, "rtmp://127.0.0.1:19350/live/source")]);
        let composition =
            build_ffmpeg_network_only_composition_with(&Config::default(), loaded_binding(&path))
                .expect("network-only composition");
        let sid = crate::source::NetworkSourceId(source_id);

        let err = composition
            .manager
            .create(rtmp_intent(sid, 19351))
            .expect_err("mismatched endpoint must fail closed");
        assert!(matches!(
            err,
            crate::session::SessionError::PreflightFailed(_)
        ));
        assert!(!composition
            .lease_manager
            .is_key_active(&crate::source::LeaseKey::Network(sid)));
        assert!(composition.lease_manager.list_active().is_empty());
        let state = composition.manager.runtime_state();
        let resource_id = crate::resource::network_resource_id_for_source(sid);
        let resource = state
            .resources
            .iter()
            .find(|r| r.resource_id == resource_id)
            .expect("network resource registered");
        assert_eq!(resource.state, crate::resource::ResourceState::Available);
    }

    #[test]
    fn rf_src_rtmp_02_network_only_composition_requires_binding_config() {
        // fail-closed startup wiring: no MEDIA_AGENT_NETWORK_BINDING → refuse
        std::env::remove_var("MEDIA_AGENT_NETWORK_BINDING");
        let err = match build_ffmpeg_network_only_composition() {
            Ok(_) => panic!("missing network binding config must fail closed"),
            Err(e) => e,
        };
        assert!(err.contains("MEDIA_AGENT_NETWORK_BINDING"), "{err}");
    }
}
