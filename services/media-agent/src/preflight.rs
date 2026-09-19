//! Phase 0.7A: Preflight 分级判定 — **只判断, 不执行** (V0.2 §1.2 三层 Preflight 语义;
//! 与运行期 QC 解耦)。FAIL ⇒ create 拒绝且**零预留零回滚** (RUNTIME_LIFECYCLE_SEQUENCE §2)。
//!
//! 分层: 判定级 (FAIL 阻塞) / WARN 级 (报告不阻塞) / Report-only 占位。
//! 本模块是 SessionManager create 的第一步; 绝不触碰媒体操作/资源占用。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::contracts::provider::CapabilityReport;
use crate::device::DeviceInfo;
use crate::graph_intent::GraphRuntimeIntent;
use crate::lease::LeaseManager;
use crate::port::PortRegistry;
use crate::resolver::BindingAuthorization;
use crate::resource::{preflight, AcquisitionRequest, ResourceRegistry};
use crate::source::{LeaseKey, ResourceOwner};

/// 设备输入能力三态（D6 判定用; ProbeFailed→Unknown, absence≠evidence）。
fn project_input_capability(
    c: &crate::port::DeviceCapabilities,
) -> crate::runtime_state::CapabilityFlag {
    use crate::port::CapabilityValue as Cv;
    match &c.input {
        Cv::Supported(_) => crate::runtime_state::CapabilityFlag::Supported,
        Cv::Unsupported => crate::runtime_state::CapabilityFlag::Unsupported,
        Cv::Unknown | Cv::ProbeFailed(_) => crate::runtime_state::CapabilityFlag::Unknown,
    }
}

/// Preflight 阶段 (判定顺序即报告顺序)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightStage {
    /// 意图形状: intent 可解析、设备引用非空且 canonical。
    Graph,
    /// 端口可用性: 目标设备在 PortRegistry 中存在可用端口 (有 manifest 时)。
    PortAvailability,
    /// 资源容量/状态: 目标 Resource 存在、能力匹配、Available (复用 resource::preflight)。
    ResourceCapacity,
    /// 租约冲突: 目标设备未被其他 owner 持有。
    LeaseConflict,
    /// 身份/绑定: 目标设备已有 HIGH/ManifestVerified 绑定 (无 gstreamer 路径则 WARN 跳过)。
    IdentityBinding,
    /// Backend 能力报告 (WARN-only: 占位探针, 不阻塞)。
    BackendCapability,
    /// 拓扑影响 (report-only 占位, 0.7B+)。
    Topology,
    /// 风险/影响面 (report-only 占位, 0.7B+)。
    Risk,
}

/// 单阶段结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOutcome {
    pub stage: PreflightStage,
    pub level: StageLevel,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageLevel {
    Pass,
    Warn,
    Fail,
}

/// 总裁决: 任一 Fail ⇒ Fail; 否则任一 Warn ⇒ Warn; 否则 Pass。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Warn,
    Fail,
}

/// 分级报告 (SessionCreated 事件的证据附件; 可序列化)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightReport {
    pub stages: Vec<StageOutcome>,
    pub verdict: Verdict,
}

impl PreflightReport {
    pub fn is_ok(&self) -> bool {
        self.verdict != Verdict::Fail
    }
    fn push(&mut self, stage: PreflightStage, level: StageLevel, detail: impl Into<String>) {
        self.stages.push(StageOutcome {
            stage,
            level,
            detail: detail.into(),
        });
    }
}

/// 判定输入 (全部只读引用; 本模块不执行任何操作)。
pub struct PreflightInputs<'a> {
    pub intent: &'a GraphRuntimeIntent,
    pub devices: &'a [DeviceInfo],
    pub resources: &'a ResourceRegistry,
    /// 目标资源占用请求 (由 SessionManager 依 intent→registry 解析得出)。
    pub claims: &'a [AcquisitionRequest],
    pub leases: &'a dyn LeaseManager,
    /// Backend-neutral production authorization；不携带任何 concrete runtime address。
    pub authorizations: &'a HashMap<Uuid, BindingAuthorization>,
    /// RF-SRC-RTMP-02 (plan D11): 生产 RTMP 准入唯一授权源。Production 模式下
    /// RTMP source 必须在此完成五元组精确授权；None = 无网络授权（Production
    /// RTMP fail-closed，Diagnostic 保持 loopback fixture 语义）。
    pub network_binding: Option<&'a crate::network_binding::NetworkSourceBinding>,
    /// Production 必须在 Preflight 阶段已有 authorization；Diagnostic 可显式 WARN fallback。
    pub require_authorization: bool,
    /// Backend 能力报告 (无 provider 时为空切片 → WARN)。
    pub capabilities: &'a [CapabilityReport],
    pub registry: Option<&'a PortRegistry>,
}

/// 执行分级判定 (纯函数; 绝不修改任何输入)。
pub fn run(inputs: &PreflightInputs<'_>) -> PreflightReport {
    let mut report = PreflightReport {
        stages: Vec::new(),
        verdict: Verdict::Pass,
    };

    // 1. Graph — intent 形状与设备引用。
    if inputs.intent.devices.is_empty() {
        report.push(
            PreflightStage::Graph,
            StageLevel::Fail,
            "intent 无设备 (空 devices)",
        );
    } else {
        let unknown: Vec<&str> = inputs
            .intent
            .devices
            .iter()
            .filter(|d| {
                match &d.pipeline.source {
                    // Network and SelfTest sources intentionally do not have a
                    // hardware DeviceInfo entry.
                    crate::graph_intent::SourceIntent::Rtmp { .. }
                    | crate::graph_intent::SourceIntent::SelfTest => false,
                    crate::graph_intent::SourceIntent::Decklink { .. } => !inputs
                        .devices
                        .iter()
                        .any(|x| x.device_id.to_string() == d.device_id),
                }
            })
            .map(|d| d.device_id.as_str())
            .collect();
        if unknown.is_empty() {
            report.push(
                PreflightStage::Graph,
                StageLevel::Pass,
                format!(
                    "{} source reference(s) are structurally resolvable",
                    inputs.intent.devices.len()
                ),
            );
        } else {
            report.push(
                PreflightStage::Graph,
                StageLevel::Fail,
                format!("intent 引用未注册设备: {unknown:?}"),
            );
        }
    }

    // 2. PortAvailability — **D4 (p07c-runtime-state): 端口级精确化**（镜像 materialize
    //    冻结语义 pipeline.rs:485-523）: port_id 显式 ⇒ 精确端口必须存在且方向为
    //    Input/Bidirectional; port_id 缺省 ⇒ 设备须有 ≥1 Input 方向端口（不再接受
    //    "任意端口", 修 Output 端口混过 Capture intent 的漏洞）。registry=None 记 WARN。
    match inputs.registry {
        Some(reg) => {
            let has_input_port = |u: Uuid| {
                reg.ports.iter().any(|p| {
                    p.device_id == u
                        && (p.direction == crate::port::PortDirection::Input
                            || p.direction == crate::port::PortDirection::Bidirectional)
                })
            };
            let mut failures: Vec<String> = Vec::new();
            for d in &inputs.intent.devices {
                if matches!(
                    d.pipeline.source,
                    crate::graph_intent::SourceIntent::Rtmp { .. }
                        | crate::graph_intent::SourceIntent::SelfTest
                ) {
                    continue;
                }
                let Ok(u) = Uuid::parse_str(&d.device_id) else {
                    failures.push(format!("设备 {} id 不可解析", d.device_id));
                    continue;
                };
                match d.pipeline.source.port_id() {
                    Some(pid) => {
                        let parsed = Uuid::parse_str(pid).ok();
                        let matched = parsed.and_then(|pu| {
                            reg.ports
                                .iter()
                                .find(|p| p.identity.port_id == Some(pu) && p.device_id == u)
                        });
                        match matched {
                            None => failures.push(format!(
                                "设备 {u} 显式 port_id {pid} 在 Discovery 端口中无匹配 (生产拒绝静默回退)"
                            )),
                            Some(port) => {
                                if port.direction != crate::port::PortDirection::Input
                                    && port.direction != crate::port::PortDirection::Bidirectional
                                {
                                    failures.push(format!(
                                        "设备 {u} port_id {pid} 方向为 {:?} (Capture intent 需 Input/Bidirectional)",
                                        port.direction
                                    ));
                                }
                            }
                        }
                    }
                    None => {
                        if !has_input_port(u) {
                            failures.push(format!("设备 {u} 无 Input 方向端口"));
                        }
                    }
                }
            }
            if failures.is_empty() {
                report.push(
                    PreflightStage::PortAvailability,
                    StageLevel::Pass,
                    "目标端口全部可用 (端口级: 精确 port_id 匹配或 ≥1 Input 端口)",
                );
            } else {
                report.push(
                    PreflightStage::PortAvailability,
                    StageLevel::Fail,
                    failures.join("; "),
                );
            }
        }
        None => report.push(
            PreflightStage::PortAvailability,
            StageLevel::Warn,
            "无 PortRegistry (无 manifest 路径); 端口可用性延后到运行期判定",
        ),
    }

    // 3. ResourceCapacity — 复用 resource::preflight (存在/能力/Available/容量)。
    // **D2 (RESOURCE-RESOLUTION-01, p07c-runtime-state): 三态 Resolution**——
    // intent 设备在 ResourceRegistry 中无派生 input 资源 ⇒ FAIL（declared capability
    // missing 不再 WARN 降级; 自动化控制面不得误判可创建）。
    let registry_empty = inputs.resources.resources.is_empty();
    let mut resolution_failures: Vec<String> = Vec::new();
    for d in &inputs.intent.devices {
        let resource_present = match &d.pipeline.source {
            crate::graph_intent::SourceIntent::Rtmp { source_id, .. } => {
                inputs.resources.resources.iter().any(|r| {
                    r.owner == ResourceOwner::Network(*source_id) && r.capability == "rtmp-input"
                })
            }
            crate::graph_intent::SourceIntent::Decklink { .. } => {
                let Ok(u) = Uuid::parse_str(&d.device_id) else {
                    continue;
                };
                inputs.resources.resources.iter().any(|r| {
                    (r.owner == ResourceOwner::Device(u) || r.device_id == u)
                        && r.capability.ends_with("-input")
                })
            }
            crate::graph_intent::SourceIntent::SelfTest => true,
        };
        if !resource_present {
            resolution_failures.push(format!(
                "source {} 无派生 input 资源 (declared capability missing)",
                d.device_id
            ));
        }
    }
    if !resolution_failures.is_empty() {
        report.push(
            PreflightStage::ResourceCapacity,
            StageLevel::Fail,
            resolution_failures.join("; "),
        );
    } else if inputs.claims.is_empty() {
        if registry_empty {
            report.push(
                PreflightStage::ResourceCapacity,
                StageLevel::Warn,
                "无资源占用请求 (registry=None legacy 路径)",
            );
        } else {
            report.push(
                PreflightStage::ResourceCapacity,
                StageLevel::Warn,
                "资源已解析但无占用请求 (诊断路径)",
            );
        }
    } else {
        let failures: Vec<String> = inputs
            .claims
            .iter()
            .filter_map(|c| {
                preflight(inputs.resources, c)
                    .err()
                    .map(|e| format!("{}: {e}", c.resource_id))
            })
            .collect();
        if failures.is_empty() {
            report.push(
                PreflightStage::ResourceCapacity,
                StageLevel::Pass,
                format!("{} 项资源占用请求全部可满足", inputs.claims.len()),
            );
        } else {
            report.push(
                PreflightStage::ResourceCapacity,
                StageLevel::Fail,
                failures.join("; "),
            );
        }
    }

    // 4. LeaseConflict — typed Device/Network keys are judged read-only before acquire.
    let conflicts: Vec<String> = inputs
        .intent
        .devices
        .iter()
        .filter_map(|d| match &d.pipeline.source {
            crate::graph_intent::SourceIntent::Rtmp { source_id, .. } => {
                let key = LeaseKey::Network(*source_id);
                inputs
                    .leases
                    .is_key_active(&key)
                    .then(|| format!("network source {source_id}"))
            }
            crate::graph_intent::SourceIntent::Decklink { .. } => {
                let Ok(device_id) = Uuid::parse_str(&d.device_id) else {
                    return None;
                };
                let key = LeaseKey::Device(device_id);
                inputs
                    .leases
                    .is_key_active(&key)
                    .then(|| format!("device {device_id}"))
            }
            crate::graph_intent::SourceIntent::SelfTest => None,
        })
        .collect();
    if conflicts.is_empty() {
        report.push(
            PreflightStage::LeaseConflict,
            StageLevel::Pass,
            "目标 source key 无现存租约冲突",
        );
    } else {
        report.push(
            PreflightStage::LeaseConflict,
            StageLevel::Fail,
            format!("source key 已被租约持有: {conflicts:?}"),
        );
    }

    // 5. IdentityBinding — hardware sources need a production device binding;
    //    RTMP network sources need exact five-tuple admission via the
    //    NetworkSourceBinding (RF-SRC-RTMP-02 plan D11). SelfTest needs nothing.
    let mut identity_failures: Vec<String> = Vec::new();
    let mut identity_details: Vec<String> = Vec::new();
    let mut identity_warn: Option<String> = None;

    // 5a. Hardware (Decklink) sources: existing production-grade logic.
    let needs_hardware_binding = inputs.intent.devices.iter().any(|d| {
        matches!(
            d.pipeline.source,
            crate::graph_intent::SourceIntent::Decklink { .. }
        )
    });
    if needs_hardware_binding {
        if inputs.authorizations.is_empty() {
            if inputs.require_authorization {
                identity_failures
                    .push("Production 缺少 binding authorization；Preflight fail-closed".into());
            } else {
                identity_warn =
                    Some("Diagnostic 缺少 binding authorization；显式 WARN fallback".into());
            }
        } else {
            let unresolved = inputs
                .intent
                .devices
                .iter()
                .filter(|d| {
                    matches!(
                        d.pipeline.source,
                        crate::graph_intent::SourceIntent::Decklink { .. }
                    )
                })
                .filter_map(|d| Uuid::parse_str(&d.device_id).ok())
                .filter(|u| {
                    !inputs
                        .authorizations
                        .get(u)
                        .is_some_and(|a| a.is_production_grade())
                })
                .collect::<Vec<_>>();
            if unresolved.is_empty() {
                identity_details.push("目标设备均有 production-grade binding authorization".into());
            } else {
                identity_failures.push(format!(
                    "目标设备缺少 production-grade binding authorization: {unresolved:?}"
                ));
            }
        }
    }

    // 5b. RTMP network sources: exact five-tuple admission
    //     (source_id, protocol, canonical_ip, port, exact_path) against the
    //     startup binding. Production without a binding is fail-closed;
    //     Diagnostic keeps the loopback-fixture semantics.
    let rtmp_sources: Vec<(
        &crate::source::NetworkSourceId,
        &crate::source::NetworkEndpoint,
    )> = inputs
        .intent
        .devices
        .iter()
        .filter_map(|d| match &d.pipeline.source {
            crate::graph_intent::SourceIntent::Rtmp {
                source_id,
                endpoint,
            } => Some((source_id, endpoint)),
            _ => None,
        })
        .collect();
    if !rtmp_sources.is_empty() {
        match inputs.network_binding {
            Some(binding) => {
                let mut rejected = 0usize;
                for (source_id, endpoint) in &rtmp_sources {
                    // Redaction-safe: NetworkBindingError carries no endpoint content.
                    if let Err(e) = binding.authorize(source_id, endpoint) {
                        rejected += 1;
                        identity_failures.push(format!("network source admission rejected: {e}"));
                    }
                }
                if rejected == 0 {
                    identity_details.push(format!(
                        "{} network source(s) authorized by exact five-tuple admission",
                        rtmp_sources.len()
                    ));
                }
            }
            None => {
                if inputs.require_authorization {
                    identity_failures.push(
                        "Production RTMP source requires NetworkSourceBinding admission (fail-closed)"
                            .into(),
                    );
                } else {
                    identity_details.push(
                        "Diagnostic network source keeps loopback fixture semantics (no binding)"
                            .into(),
                    );
                }
            }
        }
    }

    // 5c. Combine into exactly one stage outcome: Fail beats Warn beats Pass;
    //     details join so mixed hardware/network intents stay one verdict.
    if !identity_failures.is_empty() {
        report.push(
            PreflightStage::IdentityBinding,
            StageLevel::Fail,
            identity_failures.join("; "),
        );
    } else if let Some(warn) = identity_warn {
        report.push(PreflightStage::IdentityBinding, StageLevel::Warn, warn);
    } else if !identity_details.is_empty() {
        report.push(
            PreflightStage::IdentityBinding,
            StageLevel::Pass,
            identity_details.join("; "),
        );
    }

    // 6. BackendCapability — **D6 (BACKEND-CAPABILITY-01, p07c-runtime-query): 硬判定**——
    // 设备输入能力 Unsupported ⇒ FAIL（硬决策）; Unknown ⇒ WARN（absence≠evidence）;
    // Supported ⇒ Pass。
    {
        let mut cap_failures: Vec<String> = Vec::new();
        let mut unknown_devices = 0usize;
        for d in &inputs.intent.devices {
            let Ok(u) = Uuid::parse_str(&d.device_id) else {
                continue;
            };
            let Some(dev) = inputs.devices.iter().find(|x| x.device_id == u) else {
                continue;
            };
            match project_input_capability(&dev.capabilities) {
                crate::runtime_state::CapabilityFlag::Unsupported => {
                    cap_failures.push(format!("设备 {u} 无输入能力 (capability=unsupported)"));
                }
                crate::runtime_state::CapabilityFlag::Unknown => unknown_devices += 1,
                crate::runtime_state::CapabilityFlag::Supported => {}
            }
        }
        if !cap_failures.is_empty() {
            report.push(
                PreflightStage::BackendCapability,
                StageLevel::Fail,
                cap_failures.join("; "),
            );
        } else if unknown_devices > 0 {
            report.push(
                PreflightStage::BackendCapability,
                StageLevel::Warn,
                format!("{unknown_devices} 台设备输入能力 Unknown (未探测/未暴露; 不臆造)"),
            );
        } else {
            report.push(
                PreflightStage::BackendCapability,
                StageLevel::Pass,
                "目标设备输入能力全部 Supported",
            );
        }
    }

    // 7/8. Report-only 占位 (0.7B+)。
    report.push(
        PreflightStage::Topology,
        StageLevel::Warn,
        "拓扑影响判定 0.7B+ 提供 (report-only 占位)",
    );
    report.push(
        PreflightStage::Risk,
        StageLevel::Warn,
        "风险/影响面判定 0.7B+ 提供 (report-only 占位)",
    );

    report.verdict = if report.stages.iter().any(|s| s.level == StageLevel::Fail) {
        Verdict::Fail
    } else if report.stages.iter().any(|s| s.level == StageLevel::Warn) {
        Verdict::Warn
    } else {
        Verdict::Pass
    };
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lease::InMemoryLeaseManager;
    use std::os::unix::fs::PermissionsExt as _;

    fn device(id: Uuid) -> DeviceInfo {
        DeviceInfo {
            device_id: id,
            model: "m".into(),
            display_name: "d".into(),
            serial_number: None,
            identity_strength: crate::device::IdentityStrength::Enumeration,
            identity_source: crate::device::DeviceIdentitySource::Simulation,
            capabilities: crate::port::DeviceCapabilities::default(),
            video_input_connections: 0,
            video_output_connections: 0,
            ports: Vec::new(),
        }
    }

    fn intent(id: &Uuid) -> GraphRuntimeIntent {
        crate::graph_intent::GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![crate::graph_intent::DeviceIntent {
                device_id: id.to_string(),
                role: "CAPTURE".into(),
                pipeline: crate::graph_intent::PipelineIntent {
                    source: crate::graph_intent::SourceIntent::decklink(id.to_string(), None),
                    sink: crate::graph_intent::SinkIntent {
                        kind: "appsink".into(),
                    },
                },
            }],
        }
    }

    #[test]
    fn preflight_passes_on_clean_inputs_and_fail_on_unknown_device() {
        let id = Uuid::new_v4();
        let devices = vec![device(id)];
        let it = intent(&id);
        // D2: clean case 需注册派生 input 资源 (无派生资源 ⇒ FAIL, 不再 WARN 降级)。
        let mut resources = ResourceRegistry::new();
        // Resource::new 的 device_id 默认 nil (仅 derive_from_discovery 会设) —
        // 手动设置以模拟派生资源 (D2 per-device 检查按 device_id 匹配)。
        let mut res = crate::resource::Resource::new(id, "r", "sdi-input", 1);
        res.device_id = id;
        resources.resources.push(res);
        let leases = InMemoryLeaseManager::new();
        let bindings = HashMap::new();
        let caps: Vec<CapabilityReport> = Vec::new();
        let claims: Vec<AcquisitionRequest> = Vec::new();
        let inputs = PreflightInputs {
            intent: &it,
            devices: &devices,
            resources: &resources,
            claims: &claims,
            leases: &leases,
            authorizations: &bindings,
            network_binding: None,
            require_authorization: false,
            capabilities: &caps,
            registry: None,
        };
        let r = run(&inputs);
        assert!(
            r.is_ok(),
            "干净输入应为 Warn/Pass (WARN: legacy 路径): {:?}",
            r.verdict
        );
        assert_eq!(r.verdict, Verdict::Warn, "无绑定/无 registry ⇒ Warn 不阻塞");

        // 未知设备 → Graph FAIL。
        let ghost = Uuid::new_v4();
        let bad_intent = intent(&ghost);
        let inputs2 = PreflightInputs {
            intent: &bad_intent,
            devices: &devices,
            resources: &resources,
            claims: &claims,
            leases: &leases,
            authorizations: &bindings,
            network_binding: None,
            require_authorization: false,
            capabilities: &caps,
            registry: None,
        };
        let r2 = run(&inputs2);
        assert!(!r2.is_ok());
        assert_eq!(r2.verdict, Verdict::Fail);
        assert!(r2
            .stages
            .iter()
            .any(|s| s.stage == PreflightStage::Graph && s.level == StageLevel::Fail));
    }

    #[test]
    fn rf_ff_01d_production_missing_authorization_fails_preflight() {
        let id = Uuid::new_v4();
        let devices = vec![device(id)];
        let it = intent(&id);
        let mut resources = ResourceRegistry::new();
        let mut res = crate::resource::Resource::new(id, "r", "sdi-input", 1);
        res.device_id = id;
        resources.resources.push(res);
        let leases = InMemoryLeaseManager::new();
        let authorizations = HashMap::new();
        let caps: Vec<CapabilityReport> = Vec::new();
        let claims: Vec<AcquisitionRequest> = Vec::new();
        let inputs = PreflightInputs {
            intent: &it,
            devices: &devices,
            resources: &resources,
            claims: &claims,
            leases: &leases,
            authorizations: &authorizations,
            network_binding: None,
            require_authorization: true,
            capabilities: &caps,
            registry: None,
        };
        let report = run(&inputs);
        assert_eq!(report.verdict, Verdict::Fail);
        assert!(report.stages.iter().any(|s| {
            s.stage == PreflightStage::IdentityBinding
                && s.level == StageLevel::Fail
                && s.detail.contains("Production")
        }));
    }

    #[test]
    fn preflight_fails_on_lease_conflict_and_resource_unavailable() {
        let id = Uuid::new_v4();
        let devices = vec![device(id)];
        let intent = intent(&id);
        let mut resources = ResourceRegistry::new();
        resources.resources.push(crate::resource::Resource::new(
            Uuid::new_v4(),
            "r",
            "sdi-input",
            1,
        ));
        let leases = InMemoryLeaseManager::new();
        // 他人已持有目标设备租约 → LeaseConflict FAIL。
        leases
            .acquire(&id, "other-owner", std::time::Duration::from_secs(60))
            .unwrap();
        let bindings = HashMap::new();
        let caps: Vec<CapabilityReport> = Vec::new();
        let claims: Vec<AcquisitionRequest> = Vec::new();
        let inputs = PreflightInputs {
            intent: &intent,
            devices: &devices,
            resources: &resources,
            claims: &claims,
            leases: &leases,
            authorizations: &bindings,
            network_binding: None,
            require_authorization: false,
            capabilities: &caps,
            registry: None,
        };
        let r = run(&inputs);
        assert_eq!(r.verdict, Verdict::Fail);
        assert!(r
            .stages
            .iter()
            .any(|s| s.stage == PreflightStage::LeaseConflict && s.level == StageLevel::Fail));
    }

    // ── RUNTIME-STATE-RT-01 (Unit): D2/D4/D5 FAIL 路径 + side-effect 补测 ────────

    fn port_of(device_id: Uuid, direction: crate::port::PortDirection) -> crate::port::PortInfo {
        crate::port::PortInfo {
            device_id,
            provider_binding_ref: None,
            identity: crate::port::PortIdentity {
                port_id: crate::port::PortIdentity::derive(
                    &device_id,
                    crate::port::ConnectorType::Sdi,
                    crate::port::PortOrdinal::Known(1),
                ),
                connector: crate::port::ConnectorType::Sdi,
                ordinal: crate::port::PortOrdinal::Known(1),
            },
            direction,
            capabilities: crate::port::PortCapabilities::default(),
            runtime_binding: None,
            signal: crate::port::SignalStatus::default(),
            content: crate::port::VideoContentState::Unknown,
        }
    }

    /// 闭包式装配 (局部 leases/caps/claims 生命周期随闭包作用域)。
    fn with_inputs<T>(
        intent: &crate::graph_intent::GraphRuntimeIntent,
        devices: &[DeviceInfo],
        resources: &ResourceRegistry,
        registry: Option<&crate::port::PortRegistry>,
        bindings: &HashMap<Uuid, crate::resolver::BindingAuthorization>,
        f: impl FnOnce(&PreflightInputs<'_>) -> T,
    ) -> T {
        let leases = InMemoryLeaseManager::new();
        let caps: Vec<CapabilityReport> = Vec::new();
        let claims: Vec<AcquisitionRequest> = Vec::new();
        let inputs = PreflightInputs {
            intent,
            devices,
            resources,
            claims: &claims,
            leases: &leases,
            authorizations: bindings,
            network_binding: None,
            require_authorization: false,
            capabilities: &caps,
            registry,
        };
        f(&inputs)
    }

    #[test]
    fn runtime_state_rt_01_d2_missing_resource_fails_not_warn() {
        // D2: 设备存在但无派生 input 资源 ⇒ ResourceCapacity FAIL (不再 WARN)。
        let id = Uuid::new_v4();
        let devices = vec![device(id)];
        let it = intent(&id);
        let resources = ResourceRegistry::new();
        let bindings = HashMap::new();
        let r = with_inputs(&it, &devices, &resources, None, &bindings, run);
        assert_eq!(r.verdict, Verdict::Fail);
        assert!(r.stages.iter().any(|s| {
            s.stage == PreflightStage::ResourceCapacity
                && s.level == StageLevel::Fail
                && s.detail.contains("declared capability missing")
        }));
    }

    #[test]
    fn runtime_state_rt_01_d4_port_level_precision() {
        let id = Uuid::new_v4();
        let devices = vec![device(id)];
        let mut resources = ResourceRegistry::new();
        let mut res = crate::resource::Resource::new(id, "r", "sdi-input", 1);
        res.device_id = id;
        resources.resources.push(res);
        let bindings = HashMap::new();

        // (a) 设备仅有 Output 端口 + port_id=None ⇒ FAIL (不再 any-port 混过)。
        let out_only = crate::port::PortRegistry {
            ports: vec![port_of(id, crate::port::PortDirection::Output)],
        };
        let it_none = intent(&id);
        let r = with_inputs(
            &it_none,
            &devices,
            &resources,
            Some(&out_only),
            &bindings,
            run,
        );
        assert!(
            r.stages
                .iter()
                .any(|s| s.stage == PreflightStage::PortAvailability && s.level == StageLevel::Fail),
            "仅 Output 端口不得满足 Capture intent"
        );

        // (b) 显式 port_id 指向 Output 端口 ⇒ FAIL。
        let pid = out_only.ports[0].identity.port_id.unwrap();
        let mut it_bad = intent(&id);
        it_bad.devices[0].pipeline.source =
            crate::graph_intent::SourceIntent::decklink(id.to_string(), Some(pid.to_string()));
        let r2 = with_inputs(
            &it_bad,
            &devices,
            &resources,
            Some(&out_only),
            &bindings,
            run,
        );
        assert!(r2.stages.iter().any(|s| {
            s.stage == PreflightStage::PortAvailability
                && s.level == StageLevel::Fail
                && s.detail.contains("Input/Bidirectional")
        }));

        // (c) 显式 port_id 精确匹配 Input 端口 ⇒ PASS; 指向不存在端口 ⇒ FAIL。
        let in_reg = crate::port::PortRegistry {
            ports: vec![port_of(id, crate::port::PortDirection::Input)],
        };
        let in_pid = in_reg.ports[0].identity.port_id.unwrap();
        let mut it_good = intent(&id);
        it_good.devices[0].pipeline.source =
            crate::graph_intent::SourceIntent::decklink(id.to_string(), Some(in_pid.to_string()));
        let r3 = with_inputs(
            &it_good,
            &devices,
            &resources,
            Some(&in_reg),
            &bindings,
            run,
        );
        assert!(r3
            .stages
            .iter()
            .any(|s| s.stage == PreflightStage::PortAvailability && s.level == StageLevel::Pass));
        let mut it_ghost = intent(&id);
        it_ghost.devices[0].pipeline.source = crate::graph_intent::SourceIntent::decklink(
            id.to_string(),
            Some(Uuid::new_v4().to_string()),
        );
        let r4 = with_inputs(
            &it_ghost,
            &devices,
            &resources,
            Some(&in_reg),
            &bindings,
            run,
        );
        assert!(r4.stages.iter().any(|s| {
            s.stage == PreflightStage::PortAvailability
                && s.level == StageLevel::Fail
                && s.detail.contains("无匹配")
        }));
    }

    #[test]
    fn runtime_state_rt_01_d5_binding_strength_checked() {
        // D5: binding 在场但非 production_grade ⇒ FAIL (key-existence 不再算通过)。
        let id = Uuid::new_v4();
        let devices = vec![device(id)];
        let mut resources = ResourceRegistry::new();
        let mut res = crate::resource::Resource::new(id, "r", "sdi-input", 1);
        res.device_id = id;
        resources.resources.push(res);
        let mut bindings = HashMap::new();
        bindings.insert(
            id,
            crate::resolver::BindingAuthorization {
                confidence: crate::resolver::Confidence::Medium,
                match_kind: crate::resolver::ResolverMatch::TopologicalIdGuess,
                persistent_identity: false,
            },
        );
        let it = intent(&id);
        let r = with_inputs(&it, &devices, &resources, None, &bindings, run);
        assert!(r.stages.iter().any(|s| {
            s.stage == PreflightStage::IdentityBinding
                && s.level == StageLevel::Fail
                && s.detail.contains("production-grade binding authorization")
        }));
        bindings.insert(
            id,
            crate::resolver::BindingAuthorization {
                confidence: crate::resolver::Confidence::High,
                match_kind: crate::resolver::ResolverMatch::ManifestVerified,
                persistent_identity: false,
            },
        );
        let r2 = with_inputs(&it, &devices, &resources, None, &bindings, run);
        assert!(r2
            .stages
            .iter()
            .any(|s| s.stage == PreflightStage::IdentityBinding && s.level == StageLevel::Pass));
    }

    #[test]
    fn preflight_is_side_effect_free() {
        // 0.7A R1 补测落盘 (当时补丁脚本中断未写入, 本 change 补齐): Preflight 只判断
        // 不执行 — list_active 纯读; 过期租约经 Preflight 后仍在存储 (health 才清扫)。
        let id = Uuid::new_v4();
        let devices = vec![device(id)];
        let it = intent(&id);
        let mut resources = ResourceRegistry::new();
        let mut res = crate::resource::Resource::new(id, "r", "sdi-input", 1);
        res.device_id = id;
        resources.resources.push(res);
        let leases = InMemoryLeaseManager::new();
        leases
            .acquire(&id, "stale", std::time::Duration::ZERO)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(!leases.is_valid(&id), "租约确已过期");
        assert_eq!(
            leases.list_active().len(),
            1,
            "list_active 纯读: 含未清扫的过期租约"
        );
        let bindings = HashMap::new();
        let _ = with_inputs(&it, &devices, &resources, None, &bindings, run);
        assert_eq!(leases.list_active().len(), 1, "Preflight 不得修改租约存储");
        assert!(leases.health().is_empty(), "health() 才负责清扫 (职责分离)");
    }

    // ── RF-SRC-RTMP-02 TG-2 (plan D11): Production 五元组准入 ──────────────────

    fn temp_manifest(body: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vbmf-preflight-tg2-{}-{}.json",
            std::process::id(),
            Uuid::new_v4()
        ));
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        path
    }

    fn network_binding_for(
        source_id: Uuid,
        endpoint: &str,
    ) -> (
        std::path::PathBuf,
        crate::network_binding::NetworkSourceBinding,
    ) {
        let body = format!(
            r#"{{"version":1,"machine_id":"box-a","entries":[{{"source_id":"{source_id}","endpoint":"{endpoint}"}}]}}"#
        );
        let path = temp_manifest(&body);
        let local: Vec<std::net::IpAddr> = vec!["127.0.0.1".parse().unwrap()];
        let binding =
            crate::network_binding::NetworkSourceBinding::load_verified(&path, "box-a", &local)
                .expect("valid test binding");
        (path, binding)
    }

    fn rtmp_intent_for(source_id: Uuid, host: &str, port: u16, path: &str) -> GraphRuntimeIntent {
        GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![crate::graph_intent::DeviceIntent {
                device_id: "network-source-node".into(),
                role: "CAPTURE".into(),
                pipeline: crate::graph_intent::PipelineIntent {
                    source: crate::graph_intent::SourceIntent::rtmp(
                        crate::source::NetworkSourceId(source_id),
                        crate::source::NetworkEndpoint {
                            protocol: crate::source::NetworkProtocol::Rtmp,
                            host: host.into(),
                            port,
                            path: path.into(),
                        },
                    ),
                    sink: crate::graph_intent::SinkIntent {
                        kind: "appsink".into(),
                    },
                },
            }],
        }
    }

    fn identity_stage(report: &PreflightReport) -> &StageOutcome {
        report
            .stages
            .iter()
            .find(|s| s.stage == PreflightStage::IdentityBinding)
            .expect("IdentityBinding stage present")
    }

    #[test]
    fn rf_src_rtmp_02_preflight_production_rtmp_without_binding_fails_closed() {
        let source_id = Uuid::new_v4();
        let it = rtmp_intent_for(source_id, "127.0.0.1", 19350, "/live/source");
        let devices: Vec<DeviceInfo> = Vec::new();
        let resources = ResourceRegistry::new();
        let mut resources = resources;
        resources.register_network_source(crate::source::NetworkSourceId(source_id));
        let leases = InMemoryLeaseManager::new();
        let bindings = HashMap::new();
        let caps: Vec<CapabilityReport> = Vec::new();
        let claims: Vec<AcquisitionRequest> = Vec::new();
        let inputs = PreflightInputs {
            intent: &it,
            devices: &devices,
            resources: &resources,
            claims: &claims,
            leases: &leases,
            authorizations: &bindings,
            network_binding: None,
            require_authorization: true,
            capabilities: &caps,
            registry: None,
        };
        let report = run(&inputs);
        assert_eq!(report.verdict, Verdict::Fail);
        let stage = identity_stage(&report);
        assert_eq!(stage.level, StageLevel::Fail);
        assert!(
            stage.detail.contains("NetworkSourceBinding"),
            "{}",
            stage.detail
        );
    }

    #[test]
    fn rf_src_rtmp_02_preflight_diagnostic_rtmp_without_binding_keeps_loopback_fixture() {
        // Loopback regression preserved: Diagnostic network fixtures stay
        // Pass-with-note when no binding is wired.
        let source_id = Uuid::new_v4();
        let it = rtmp_intent_for(source_id, "127.0.0.1", 19350, "/live/source");
        let devices: Vec<DeviceInfo> = Vec::new();
        let mut resources = ResourceRegistry::new();
        resources.register_network_source(crate::source::NetworkSourceId(source_id));
        let leases = InMemoryLeaseManager::new();
        let bindings = HashMap::new();
        let caps: Vec<CapabilityReport> = Vec::new();
        let claims: Vec<AcquisitionRequest> = Vec::new();
        let inputs = PreflightInputs {
            intent: &it,
            devices: &devices,
            resources: &resources,
            claims: &claims,
            leases: &leases,
            authorizations: &bindings,
            network_binding: None,
            require_authorization: false,
            capabilities: &caps,
            registry: None,
        };
        let report = run(&inputs);
        let stage = identity_stage(&report);
        assert_eq!(stage.level, StageLevel::Pass);
        assert!(
            stage.detail.contains("loopback fixture"),
            "{}",
            stage.detail
        );
    }

    #[test]
    fn rf_src_rtmp_02_preflight_production_five_tuple_exact_admission_matrix() {
        let source_id = Uuid::new_v4();
        let (_path, binding) = network_binding_for(source_id, "rtmp://127.0.0.1:19350/live/source");
        let devices: Vec<DeviceInfo> = Vec::new();

        let run_identity = |host: &str, port: u16, path: &str, sid: Uuid| {
            let it = rtmp_intent_for(sid, host, port, path);
            let mut resources = ResourceRegistry::new();
            resources.register_network_source(crate::source::NetworkSourceId(sid));
            let leases = InMemoryLeaseManager::new();
            let bindings = HashMap::new();
            let caps: Vec<CapabilityReport> = Vec::new();
            let claims: Vec<AcquisitionRequest> = Vec::new();
            let inputs = PreflightInputs {
                intent: &it,
                devices: &devices,
                resources: &resources,
                claims: &claims,
                leases: &leases,
                authorizations: &bindings,
                network_binding: Some(&binding),
                require_authorization: true,
                capabilities: &caps,
                registry: None,
            };
            identity_stage(&run(&inputs)).clone()
        };

        // exact five-tuple passes
        let ok = run_identity("127.0.0.1", 19350, "/live/source", source_id);
        assert_eq!(ok.level, StageLevel::Pass, "{}", ok.detail);
        assert!(ok.detail.contains("five-tuple"), "{}", ok.detail);

        // any single component mismatch fails, redaction-safe (no endpoint
        // literals in the detail)
        for (host, port, path, label) in [
            ("127.0.0.1", 19351, "/live/source", "port"),
            ("127.0.0.1", 19350, "/live/other", "path"),
            ("127.0.0.2", 19350, "/live/source", "host"),
        ] {
            let stage = run_identity(host, port, path, source_id);
            assert_eq!(stage.level, StageLevel::Fail, "{label}: {}", stage.detail);
            assert!(
                stage.detail.contains("admission rejected"),
                "{label}: {}",
                stage.detail
            );
            assert!(
                !stage.detail.contains("19350"),
                "redaction: {}",
                stage.detail
            );
            assert!(
                !stage.detail.contains("/live"),
                "redaction: {}",
                stage.detail
            );
        }

        // unknown source_id is not authorized
        let unknown = run_identity("127.0.0.1", 19350, "/live/source", Uuid::new_v4());
        assert_eq!(unknown.level, StageLevel::Fail);
        assert!(
            unknown.detail.contains("not authorized"),
            "{}",
            unknown.detail
        );

        // non-canonical wire endpoint rejects as invalid before matching
        let invalid = run_identity("localhost", 19350, "/live/source", source_id);
        assert_eq!(invalid.level, StageLevel::Fail);
        assert!(invalid.detail.contains("invalid"), "{}", invalid.detail);
    }
}
