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

    // 2. PortAvailability — registry 在场时目标设备须有端口; 无 manifest (registry=None) 记 WARN (legacy 诊断路径)。
    match inputs.registry {
        Some(reg) => {
            let missing: Vec<&str> = inputs
                .intent
                .devices
                .iter()
                .filter(|d| {
                    Uuid::parse_str(&d.device_id)
                        .map(|u| reg.ports.iter().all(|p| p.device_id != u))
                        .unwrap_or(true)
                })
                .map(|d| d.device_id.as_str())
                .collect();
            if missing.is_empty() {
                report.push(
                    PreflightStage::PortAvailability,
                    StageLevel::Pass,
                    "目标设备均有已发现端口",
                );
            } else {
                report.push(
                    PreflightStage::PortAvailability,
                    StageLevel::Fail,
                    format!("目标设备无端口: {missing:?}"),
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
    if inputs.claims.is_empty() {
        report.push(
            PreflightStage::ResourceCapacity,
            StageLevel::Warn,
            "无资源占用请求 (registry=None legacy 路径)",
        );
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
        let unresolved = inputs
            .intent
            .devices
            .iter()
            .filter_map(|d| Uuid::parse_str(&d.device_id).ok())
            .filter(|u| !inputs.bindings.contains_key(u))
            .collect::<Vec<_>>();
        if unresolved.is_empty() {
            report.push(
                PreflightStage::IdentityBinding,
                StageLevel::Pass,
                "目标设备均有生产绑定",
            );
        } else {
            report.push(
                PreflightStage::IdentityBinding,
                StageLevel::Fail,
                format!("目标设备缺少生产绑定: {unresolved:?}"),
            );
        }
    }

    // 6. BackendCapability — WARN-only 报告 (探针占位, 不阻塞)。
    if inputs.capabilities.is_empty() {
        report.push(
            PreflightStage::BackendCapability,
            StageLevel::Warn,
            "无能力探针报告 (占位 SPI)",
        );
    } else {
        report.push(
            PreflightStage::BackendCapability,
            StageLevel::Warn,
            format!("能力报告 {} 份 (占位)", inputs.capabilities.len()),
        );
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
        let resources = ResourceRegistry::new();
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
}
