//! Phase 0.7C Foundation: CanonicalRuntimeState — Canonical 层与 Runtime 层之间的
//! **第一条生产聚合边**（0.7B Consolidation Audit 确认两子图不相交; 本模块补边）。
//!
//! **终审加严红线（本模块最高约束）**: `Canonical Media Semantics ≠ Runtime State`——
//! 媒体语义以 `CanonicalMediaDescriptor` **整值组合**（`PortMediaSemantics` 列表）,
//! Runtime 侧结构只存**运行事实**（bound/方向/状态/投影）; **绝不**把 descriptor
//! 字段平铺进 state 结构（组合性由测试锁定）。
//!
//! 位置: 编排层（可引用 canonical/port/resource/resolver 类型; 零 vendor 依赖）。
//! `assemble` 为纯函数（无 IO/锁/全局）; `SessionManager::runtime_state()` 是生产路径。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::device::{DeviceInfo, IdentityStrength};
use crate::port::{ConnectorType, PortDirection, PortInfo, PortRegistry};
use crate::resolver::{Confidence, ResolvedDeviceBinding, ResolverMatch};
use crate::resource::{ResourceRegistry, ResourceState};
use crate::session::{MediaSession, SessionId, SessionPhase, SessionState};

/// 设备运行态（运行事实; 媒体语义不在此 —— 见 `media_semantics` 组合）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceRuntimeState {
    pub device_id: Uuid,
    pub model: String,
    pub identity_strength: IdentityStrength,
    /// 仅 production_grade 绑定入列（D5: High + exact/ManifestVerified）。
    pub binding: Option<BindingStatus>,
    /// 能力投影（D6 BACKEND-CAPABILITY-01; None = 数据不在场——absence≠evidence）。
    pub capabilities: Option<DeviceCapabilitiesSummary>,
}

/// 绑定状态摘要（D5 实查后的投影）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingStatus {
    pub match_kind: ResolverMatch,
    pub confidence: Confidence,
}

/// 设备能力标志（直取 `CapabilityValue` 三态; D6 projection）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityFlag {
    Unknown,
    Supported,
    Unsupported,
}

/// 设备能力投影摘要（D6: Provider → Capability Probe → Runtime State）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCapabilitiesSummary {
    pub can_input: CapabilityFlag,
    pub can_output: CapabilityFlag,
    pub input_ports: Option<u32>,
    pub output_ports: Option<u32>,
}

/// 端口运行态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortRuntimeState {
    pub port_id: Uuid,
    pub device_id: Uuid,
    pub direction: PortDirection,
    pub connector: ConnectorType,
}

/// 资源运行态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRuntimeState {
    pub resource_id: Uuid,
    pub device_id: Uuid,
    pub capability: String,
    pub state: ResourceState,
}

/// 会话运行态投影（运行事实摘要; 非会话语义复制品）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRuntimeState {
    pub session_id: SessionId,
    pub state: SessionState,
    pub phase: SessionPhase,
    pub claims: usize,
    pub pipeline: Option<u64>,
}

/// 媒体语义组合（**整值组合, 绝不平铺**——终审加严红线）。
///
/// **D15 契约（登记不实现）**: `PortId` 是物理/逻辑绑定关系, **不等于**单一 media
/// flow——一个 Port 可对应 0/1/N flows（audio 多轨/timecode/metadata 属后续）；
/// `Vec` 结构已避免过度限制。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortMediaSemantics {
    pub port_id: Uuid,
    pub descriptor: crate::normalize::CanonicalMediaDescriptor,
}

/// D14 观察信封 —— 一次 `runtime_state()` 装配的观测元数据（Design Doc §2.1）。
///
/// **swept, non-transactional, start-ordered**:
/// - `revision`: 进程内单调递增观察序号, 起点 1（0 保留 = absent, 见 serde default）;
/// - `lineage`: 进程观察谱系（owner 构造时 UUIDv4 一次）, 重启换新;
/// - `observed_at_ms`: 观测时刻墙钟（毫秒; 新鲜度参考, **非**一致性锚点——锚点是 (lineage, revision)）。
///   在装配点读取（观测开始 fetch_add 之后、源采集完成之后）——与 revision 锚定时刻**不同一时刻**。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotObservation {
    pub revision: u64,
    pub lineage: Uuid,
    pub observed_at_ms: u64,
}

/// Canonical Runtime State —— 运行时状态的统一聚合快照。
///
/// **D14 契约（CLOSED @ v03-d14-runtime-snapshot-consistency, 2026-09-02）**:
/// 一致性类 = **swept non-transactional** —— 各源（devices/ports/resources/sessions）
/// 在装配点各自加锁/共享点克隆, 各源观察时刻独立（= 其读取时刻 ≤ 装配完成时刻）,
/// 跨源无原子性; **start-ordered**: 进程内 revision 严格单调 +1、唯一, rev_a < rev_b
/// ⟺ S_b 观测开始（fetch_add）晚于 S_a; 无 per-field 单调承诺、无跨进程全序
/// （lineage 为随机 UUIDv4, 非时序）。revision 起点 1（0 = absent, serde default）,
/// 重启归 1 + 换新 lineage; 不承诺跨重启连续。`generated_at_ms` 保留为新鲜度参考
/// （墙钟, 可 NTP 回拨, 非一致性锚点——锚点 = (lineage, revision)）。
/// 三 "revision" 消歧: 本 `observation_revision` = 观察域; V0.2 §1.21 "Runtime
/// Revision N+1" = config-apply 域; API §4 `If-Match: "revision-N"` = 乐观并发域
/// —— 互不替代（Design Doc §4.3）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalRuntimeState {
    pub devices: Vec<DeviceRuntimeState>,
    pub ports: Vec<PortRuntimeState>,
    pub resources: Vec<ResourceRuntimeState>,
    pub sessions: Vec<SessionRuntimeState>,
    pub media_semantics: Vec<PortMediaSemantics>,
    pub generated_at_ms: u64,
    /// D14: 进程内单调观察序号（起点 1; 0 = absent, 见 serde default）。
    #[serde(default)]
    pub observation_revision: u64,
    /// D14: 进程观察谱系（UUIDv4; nil = absent）。
    #[serde(default)]
    pub observation_lineage: Uuid,
}

impl CanonicalRuntimeState {
    /// 纯装配（无 IO/锁/全局; 同输入——含 obs——恒同输出, D14 关闭）。媒体语义对每个有稳定
    /// port_id 的端口复用 0.7B 资产（`RawInputDescription::from_port` + `normalize_input`）。
    pub fn assemble(
        devices: &[DeviceInfo],
        registry: &PortRegistry,
        resources: &ResourceRegistry,
        bindings: &std::collections::HashMap<Uuid, ResolvedDeviceBinding>,
        sessions: &[MediaSession],
        obs: &SnapshotObservation,
    ) -> Self {
        let devs = devices
            .iter()
            .map(|d| DeviceRuntimeState {
                device_id: d.device_id,
                model: d.model.clone(),
                identity_strength: d.identity_strength,
                binding: bindings
                    .get(&d.device_id)
                    .filter(|b| b.is_production_grade())
                    .map(|b| BindingStatus {
                        match_kind: b.match_kind,
                        confidence: b.confidence,
                    }),
                capabilities: project_capabilities(&d.capabilities),
            })
            .collect();
        let ports = registry
            .ports
            .iter()
            .filter_map(PortRuntimeState::from_port_info)
            .collect();
        let res = resources
            .resources
            .iter()
            .map(|r| ResourceRuntimeState {
                resource_id: r.id,
                device_id: r.device_id,
                capability: r.capability.clone(),
                state: r.state,
            })
            .collect();
        let sess = sessions
            .iter()
            .map(|s| SessionRuntimeState {
                session_id: s.session_id,
                state: s.state,
                phase: s.phase,
                claims: s.resource_claims.len(),
                pipeline: s.pipeline.map(|h| h.0),
            })
            .collect();
        // 媒体语义组合: 仅稳定 port_id 的端口（与 ResourceRegistry 派生条件一致）。
        let media = registry
            .ports
            .iter()
            .filter_map(|p| {
                let pid = p.identity.port_id?;
                let raw = crate::normalize::RawInputDescription::from_port(p);
                let outcome = crate::normalize::normalize_input(&raw);
                Some(PortMediaSemantics {
                    port_id: pid,
                    descriptor: outcome.descriptor,
                })
            })
            .collect();
        Self {
            devices: devs,
            ports,
            resources: res,
            sessions: sess,
            media_semantics: media,
            generated_at_ms: obs.observed_at_ms,
            observation_revision: obs.revision,
            observation_lineage: obs.lineage,
        }
    }
}

/// `DeviceCapabilities`（三态 CapabilityValue）→ `DeviceCapabilitiesSummary` 投影。
/// ProbeFailed 视为 Unknown（探测失败 ≠ 不支持——absence≠evidence）。
fn project_capabilities(c: &crate::port::DeviceCapabilities) -> Option<DeviceCapabilitiesSummary> {
    use crate::port::CapabilityValue as Cv;
    let flag = |v: &Cv<bool>| match v {
        Cv::Supported(_) => CapabilityFlag::Supported,
        Cv::Unsupported => CapabilityFlag::Unsupported,
        Cv::Unknown | Cv::ProbeFailed(_) => CapabilityFlag::Unknown,
    };
    let count = |v: &Cv<u32>| match v {
        Cv::Supported(n) => Some(*n),
        _ => None,
    };
    Some(DeviceCapabilitiesSummary {
        can_input: flag(&c.input),
        can_output: flag(&c.output),
        input_ports: count(&c.input_port_count),
        output_ports: count(&c.output_port_count),
    })
}

impl PortRuntimeState {
    fn from_port_info(p: &PortInfo) -> Option<Self> {
        Some(Self {
            port_id: p.identity.port_id?,
            device_id: p.device_id,
            direction: p.direction,
            connector: p.identity.connector,
        })
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::contracts::provider::HardwareProvider as _;
    use crate::port::{
        PortCapabilities, PortIdentity, PortOrdinal, SignalStatus, VideoContentState,
    };
    use std::collections::HashMap;

    /// D14: 既有测试共用观察信封（revision=1 固定值——测试只关心装配, 不关心序列）。
    const TEST_OBS: SnapshotObservation = SnapshotObservation {
        revision: 1,
        lineage: uuid::Uuid::from_u128(0),
        observed_at_ms: 0,
    };

    fn input_port(device_id: Uuid, direction: PortDirection) -> PortInfo {
        PortInfo {
            device_id,
            provider_binding_ref: None,
            identity: PortIdentity {
                port_id: PortIdentity::derive(
                    &device_id,
                    ConnectorType::Sdi,
                    PortOrdinal::Known(1),
                ),
                connector: ConnectorType::Sdi,
                ordinal: PortOrdinal::Known(1),
            },
            direction,
            capabilities: PortCapabilities::default(),
            runtime_binding: None,
            signal: SignalStatus::default(),
            content: VideoContentState::Unknown,
        }
    }

    fn world() -> (Vec<DeviceInfo>, PortRegistry, ResourceRegistry) {
        let devices: Vec<DeviceInfo> = crate::adapters::mock::MockProvider
            .discover()
            .expect("mock discover")
            .into_iter()
            .map(|d| d.device)
            .collect();
        let registry = PortRegistry {
            ports: vec![input_port(devices[0].device_id, PortDirection::Input)],
        };
        let resources = ResourceRegistry::derive_from_discovery(&registry);
        (devices, registry, resources)
    }

    /// D14: 空 world（零端口零设备——观察信封测试不需要世界内容）。
    fn empty_world() -> (Vec<DeviceInfo>, PortRegistry, ResourceRegistry) {
        let registry = PortRegistry { ports: vec![] };
        let resources = ResourceRegistry::derive_from_discovery(&registry);
        (Vec::new(), registry, resources)
    }

    #[test]
    fn runtime_state_rt_01_composition_descriptor_not_flattened() {
        // 终审加严红线: descriptor 字段只存在于 media_semantics[].descriptor 命名空间,
        // 绝不平铺到 state 顶层 (顶层键集合 == 八个固定键, D14 additive 两字段)。
        let (devices, registry, resources) = world();
        let state = CanonicalRuntimeState::assemble(
            &devices,
            &registry,
            &resources,
            &HashMap::new(),
            &[],
            &TEST_OBS,
        );
        let json = serde_json::to_value(&state).expect("serialize");
        let mut top_keys: Vec<&str> = json
            .as_object()
            .expect("object")
            .keys()
            .map(|k| k.as_str())
            .collect();
        top_keys.sort_unstable(); // serde_json Value 为字典序; 语义断言用集合比较。
        assert_eq!(top_keys, {
            let mut expect = vec![
                "devices",
                "ports",
                "resources",
                "sessions",
                "media_semantics",
                "generated_at_ms",
                "observation_revision",
                "observation_lineage",
            ];
            expect.sort_unstable();
            expect
        });
        // descriptor 整值组合在场 (媒体语义 ≠ 运行事实, 分离存放)。
        assert_eq!(state.media_semantics.len(), 1);
        // 运行事实结构无媒体语义字段。
        let dev_json = serde_json::to_value(&state.devices[0]).unwrap();
        for banned in ["width", "frame_rate", "role", "presence", "descriptor"] {
            assert!(
                !dev_json.to_string().contains(banned),
                "运行事实结构不得含媒体语义字段: {banned}"
            );
        }
    }

    #[test]
    fn runtime_state_rt_01_binding_only_production_grade() {
        // D5: 仅 production_grade 绑定入 DeviceRuntimeState。
        let (devices, registry, resources) = world();
        let dev = devices[0].device_id;
        let mut bindings = HashMap::new();
        bindings.insert(
            dev,
            ResolvedDeviceBinding {
                device_number: 1,
                hw_serial_number: None,
                persistent_id: None,
                confidence: Confidence::Medium, // 非 High
                match_kind: ResolverMatch::TopologicalIdGuess,
            },
        );
        let state = CanonicalRuntimeState::assemble(
            &devices,
            &registry,
            &resources,
            &bindings,
            &[],
            &TEST_OBS,
        );
        assert!(
            state.devices[0].binding.is_none(),
            "非 production_grade 不得投影"
        );
        bindings.insert(
            dev,
            ResolvedDeviceBinding {
                device_number: 1,
                hw_serial_number: None,
                persistent_id: None,
                confidence: Confidence::High,
                match_kind: ResolverMatch::ManifestVerified,
            },
        );
        let state = CanonicalRuntimeState::assemble(
            &devices,
            &registry,
            &resources,
            &bindings,
            &[],
            &TEST_OBS,
        );
        assert!(
            state.devices[0].binding.is_some(),
            "production_grade 应投影"
        );
    }

    #[test]
    fn runtime_state_rt_01_session_projection_and_resource_states() {
        // 聚合投影: 会话/资源运行事实可见 (Simulation 层在 session 集成测试扩展)。
        let (devices, registry, resources) = world();
        let state = CanonicalRuntimeState::assemble(
            &devices,
            &registry,
            &resources,
            &HashMap::new(),
            &[],
            &TEST_OBS,
        );
        assert_eq!(state.resources.len(), 1); // sdi-input 派生
        assert_eq!(state.resources[0].state, ResourceState::Available);
        assert!(state.sessions.is_empty());
        assert_eq!(state.ports.len(), 1);
        assert_eq!(state.ports[0].direction, PortDirection::Input);
    }

    #[test]
    fn runtime_state_rt_01_observation_envelope_serde_roundtrip() {
        let (devices, registry, resources) = empty_world();
        let obs = SnapshotObservation {
            revision: 7,
            lineage: Uuid::new_v4(),
            observed_at_ms: 1_700_000_000_123,
        };
        let state = CanonicalRuntimeState::assemble(
            &devices,
            &registry,
            &resources,
            &HashMap::new(),
            &[],
            &obs,
        );
        assert_eq!(state.observation_revision, 7, "revision 必来自 obs 输入");
        assert_eq!(state.observation_lineage, obs.lineage);
        assert_eq!(
            state.generated_at_ms, obs.observed_at_ms,
            "时间戳必来自 obs, 非墙钟"
        );
        let json = serde_json::to_string(&state).expect("serialize 8 字段");
        let back: CanonicalRuntimeState = serde_json::from_str(&json).expect("roundtrip");
        assert_eq!(back.observation_revision, 7);
        assert_eq!(back.observation_lineage, obs.lineage);
        assert_eq!(back.generated_at_ms, obs.observed_at_ms);
    }

    #[test]
    fn runtime_state_rt_01_legacy_six_key_json_deserializes_with_defaults() {
        let legacy = r#"{
            "devices": [],
            "ports": [],
            "resources": [],
            "sessions": [],
            "media_semantics": [],
            "generated_at_ms": 1700000000000
        }"#;
        let state: CanonicalRuntimeState =
            serde_json::from_str(legacy).expect("旧 6 键 JSON 必须可反序列化 (additive)");
        assert_eq!(
            state.observation_revision, 0,
            "缺字段 default = 0 (=absent)"
        );
        assert_eq!(
            state.observation_lineage,
            Uuid::nil(),
            "缺字段 default = nil UUID"
        );
        assert_eq!(state.generated_at_ms, 1_700_000_000_000);
    }

    #[test]
    fn runtime_state_rt_01_assemble_pure_same_obs_same_output() {
        let (devices, registry, resources) = empty_world();
        let bindings = std::collections::HashMap::new();
        let sessions: &[MediaSession] = &[];
        let obs = SnapshotObservation {
            revision: 3,
            lineage: Uuid::new_v4(),
            observed_at_ms: 1_700_000_000_456,
        };
        let a = CanonicalRuntimeState::assemble(
            &devices, &registry, &resources, &bindings, sessions, &obs,
        );
        let b = CanonicalRuntimeState::assemble(
            &devices, &registry, &resources, &bindings, sessions, &obs,
        );
        assert_eq!(a, b, "同输入恒同输出 (PartialEq 逐字段)");
        assert_eq!(a.generated_at_ms, obs.observed_at_ms, "无隐式墙钟读");
        let other = SnapshotObservation { revision: 4, ..obs };
        let c = CanonicalRuntimeState::assemble(
            &devices, &registry, &resources, &bindings, sessions, &other,
        );
        assert_ne!(
            a.observation_revision, c.observation_revision,
            "obs 是显式输入"
        );
    }
}
