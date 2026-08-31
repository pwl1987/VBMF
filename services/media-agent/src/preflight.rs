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
use crate::resolver::ResolvedDeviceBinding;
use crate::resource::{preflight, AcquisitionRequest, ResourceRegistry};

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
    /// 已解析绑定 (无 gstreamer/manifest 路径为空 map → IdentityBinding 记 WARN)。
    pub bindings: &'a HashMap<Uuid, ResolvedDeviceBinding>,
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
                !inputs
                    .devices
                    .iter()
                    .any(|x| x.device_id.to_string() == d.device_id)
            })
            .map(|d| d.device_id.as_str())
            .collect();
        if unknown.is_empty() {
            report.push(
                PreflightStage::Graph,
                StageLevel::Pass,
                format!("{} 设备引用均已在 Registry", inputs.intent.devices.len()),
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
                let Ok(u) = Uuid::parse_str(&d.device_id) else {
                    failures.push(format!("设备 {} id 不可解析", d.device_id));
                    continue;
                };
                match &d.pipeline.source.port_id {
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
        let Ok(u) = Uuid::parse_str(&d.device_id) else {
            continue;
        };
        if !inputs
            .resources
            .resources
            .iter()
            .any(|r| r.device_id == u && r.capability.ends_with("-input"))
        {
            resolution_failures.push(format!(
                "设备 {u} 无派生 input 资源 (declared capability missing)"
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

    // 4. LeaseConflict — 目标设备未被其他 owner 持有 (本会话尚未 acquire, 任何现存租约即冲突)。
    // P0-4 judge-only: 用 list_active 纯读 (health() 会清扫过期租约 = 副作用)。
    let held: Vec<Uuid> = inputs
        .leases
        .list_active()
        .into_iter()
        .map(|l| l.device_id)
        .collect();
    let conflicts: Vec<Uuid> = inputs
        .intent
        .devices
        .iter()
        .filter_map(|d| Uuid::parse_str(&d.device_id).ok())
        .filter(|u| held.contains(u))
        .collect();
    if conflicts.is_empty() {
        report.push(
            PreflightStage::LeaseConflict,
            StageLevel::Pass,
            "目标设备无现存租约冲突",
        );
    } else {
        report.push(
            PreflightStage::LeaseConflict,
            StageLevel::Fail,
            format!("设备已被租约持有: {conflicts:?}"),
        );
    }

    // 5. IdentityBinding — 生产绑定在场; 无绑定 (非 gstreamer/legacy) 记 WARN 不阻塞。
    if inputs.bindings.is_empty() {
        report.push(PreflightStage::IdentityBinding, StageLevel::Warn, "无已解析绑定 (legacy/非 gstreamer 路径); materialize 将按 identity_strength fail-closed");
    } else {
        // **D5 (IDENTITY-BINDING-01, p07c-runtime-state): 实查强度**——key-existence
        // 不等于 verified: 须 is_production_grade()（HIGH + 精确匹配/ManifestVerified）。
        let unresolved = inputs
            .intent
            .devices
            .iter()
            .filter_map(|d| Uuid::parse_str(&d.device_id).ok())
            .filter(|u| {
                !inputs
                    .bindings
                    .get(u)
                    .is_some_and(|b| b.is_production_grade())
            })
            .collect::<Vec<_>>();
        if unresolved.is_empty() {
            report.push(
                PreflightStage::IdentityBinding,
                StageLevel::Pass,
                "目标设备均有生产级绑定 (HIGH + 精确匹配/ManifestVerified)",
            );
        } else {
            report.push(
                PreflightStage::IdentityBinding,
                StageLevel::Fail,
                format!("目标设备缺少生产级绑定 (非 HIGH/非精确匹配): {unresolved:?}"),
            );
        }
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
                    source: crate::graph_intent::SourceIntent {
                        kind: "decklink".into(),
                        device_id: id.to_string(),
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
            bindings: &bindings,
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
            bindings: &bindings,
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
            bindings: &bindings,
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
        bindings: &HashMap<Uuid, crate::resolver::ResolvedDeviceBinding>,
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
            bindings,
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
        it_bad.devices[0].pipeline.source.port_id = Some(pid.to_string());
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
        it_good.devices[0].pipeline.source.port_id = Some(in_pid.to_string());
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
        it_ghost.devices[0].pipeline.source.port_id = Some(Uuid::new_v4().to_string());
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
            crate::resolver::ResolvedDeviceBinding {
                device_number: 3,
                hw_serial_number: None,
                persistent_id: None,
                confidence: crate::resolver::Confidence::Medium,
                match_kind: crate::resolver::ResolverMatch::TopologicalIdGuess,
            },
        );
        let it = intent(&id);
        let r = with_inputs(&it, &devices, &resources, None, &bindings, run);
        assert!(r.stages.iter().any(|s| {
            s.stage == PreflightStage::IdentityBinding
                && s.level == StageLevel::Fail
                && s.detail.contains("非 HIGH/非精确匹配")
        }));
        bindings.insert(
            id,
            crate::resolver::ResolvedDeviceBinding {
                device_number: 3,
                hw_serial_number: None,
                persistent_id: None,
                confidence: crate::resolver::Confidence::High,
                match_kind: crate::resolver::ResolverMatch::ManifestVerified,
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
}
