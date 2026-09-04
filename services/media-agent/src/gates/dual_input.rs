//! A2-8-02-I: VBMF_A2_8_DUAL_INPUT 真机五层 Gate（第十八轮 §十/§十五——正式入口）。
//!
//! 硬件形态冻结（十六轮 §四 / 十八轮 §十二）: **两块独立 DeckLink、每块恰一个
//! SDI Input**; 一块多输入卡不支持——N×M / 双工 Port Identity 属独立 change
//! `PORT-IDENTITY-AND-RESOURCE-ADDRESSING`（direction 入键/序号展开/PortId
//! 迁移/`derive_claims()` 精确 port 寻址一次闭合）, 不混入 02-I。
//!
//! 五层验收链（probe §21.3 + 十八轮 §十一 冻结）:
//! - **L1 Identity/Port/Capability/Signal**: manifest 双 Input port（含
//!   connector/ordinal 声明）→ SDK 位掩码能力证据 → 双信号 Locked。
//!   Capability / Direction / Signal **三列分记**——audio 能力当前由 video
//!   连接器推导（SDI 嵌入音频工程事实）, **不报独立 SDK audio 探针**;
//! - **L2 Execution**: 双输入 Session + MediaTap attach + Program Graph
//!   （Bridged inter 系真实消费双输入媒体面）;
//! - **L3 Output**: Program video/audio 帧计数与 PTS 真实增长（非 PLAYING 态）;
//! - **L4 Timing**: Input A/B + Bridge A/B + Program 三列 PTS 同采 + A→B
//!   切换 pre/post——**只测量, 不做 timestamp normalization**（04 冻结）;
//! - **L5 Failure**: A fail→B alive·B fail→A alive·recover 后桥真实复流·
//!   故障域分类不越域。**Supervisor = recovery decision 非 switch executor**
//!   ——本 Gate 切换经 SwitchExecutionAdapter 直驱（诊断消费方）, Supervisor
//!   不持有 switch 面（wiring 事实, 非 Supervisor 角色变更）。
//!   "Bridge fail≠Input fail" 的注入验证属 A2-8-03 supervision 面——本 Gate
//!   不伪造桥故障（无桥故障注入原语, 分类器分支以真实观测行覆盖）。
//!
//! Gate 语义纪律: 同 session_lifecycle——gate-local 诊断 world（manifest→
//! probes→bindings→registry→resources→bundle→SessionManager）, 复用调用方
//! 运行时基础件; env 未命中即返回不 exit。多输入 watchdog（b1-b4 观测面）
//! 由其单测与 A2-8-03 覆盖, 本 Gate 自行采样观测（确定性取证）。
//!
//! Hardening（第十九轮 §十六 H1-H4 + §十三 P1）:
//! - **H1 fail-stop**: L1a/b/c/d 任一 FAIL → 记录后立即终裁退出, 绝不进入
//!   L2; L2b/L3 同链——层间失败不进入下一层（仍走完整 Teardown 释放资源）。
//!   Gate 是验收 Gate 非"诊断脚本";
//! - **H2 Port↔Resource 闭环（L1d）**: 每设备恰一 Input Resource 且 ID ==
//!   Manifest Input Port 的规范派生（`resource::input_resource_id_for_port`
//!   单源）——`derive_claims()` 取首 "-input" 在唯一资源下必然命中它。
//!   **不动 SessionManager**; 精确 port 寻址属独立 change
//!   `PORT-IDENTITY-AND-RESOURCE-ADDRESSING`（多输入卡在 L1d 即 FAIL）;
//! - **H3 intent 携带 port_id**: `SourceIntent.port_id` 非"无副作用字段"——
//!   materialize 精确消费它定位端口（Some→registry 精确匹配出 connector,
//!   无匹配生产 fail-closed; None 才回退设备首端口）。Gate 携带已验证的
//!   Manifest Port, 闭合 Manifest→Registry→Intent→connector 定位链;
//! - **H4 每端口一行一一对应证据**: DeviceHandle/DeviceId/PortId/connector/
//!   ordinal/Direction/Capability/Signal/device_number 同行可审计;
//! - **P1: 本 Gate 不写 agent_state**——Gate verdict ≠ 生产 health state
//!   （session_lifecycle 同惯例: 状态由 reducer 从真实事件流派生, 诊断
//!   world 不直写）。

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::sync::Arc;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::config::Config;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::backend::MediaBackend;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::media_tap::{BridgeChannelLiveness, MediaTapPort};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::provider::DiscoveredDevice;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::switch::{ProgramObservation, SwitchExecutionAdapter};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::device::DeviceInfo;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::events::{RuntimeEventLog, RuntimeEventSink};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::health::AgentState;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::lease::{InMemoryLeaseManager, LeaseManager as _};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::pipeline::PtsMonotonicity;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::port::{CapabilityValue, PortDirection, PortInfo};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::session::{SessionInput, SessionPhase};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::supervisor::Supervisor;

/// 观察时钟活性窗口（ms）——G/H-1 语义: alive = 当前推进, 非"曾经有帧"。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const LIVENESS_WINDOW_MS: u64 = 3000;
/// 起始稳定等待（帧累积）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const SETTLE_SECS: u64 = 4;
/// 两次采样间隔（推进性判定）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const SAMPLE_GAP_SECS: u64 = 3;
/// 故障注入后等待（确保越过活性窗口 + 帧计数冻结可判）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const L5_WAIT_SECS: u64 = 5;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn sleep(sec: u64) {
    std::thread::sleep(std::time::Duration::from_secs(sec));
}

/// 记录单条 verdict（打印 + 入账）。独立 fn 而非捕获闭包——fail-stop 路径
/// 需在 record 之后立刻不可变借用 verdicts 终裁（H1）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn record(
    verdicts: &mut Vec<(&'static str, bool, String)>,
    name: &'static str,
    pass: bool,
    detail: String,
) {
    println!(
        "=== A2-8 {name}: {} === {detail}",
        if pass { "PASS" } else { "FAIL" }
    );
    verdicts.push((name, pass, detail));
}

/// 终裁输出 + 退出码（H1 fail-stop 与全链完成共用）。**不写 agent_state**
/// ——Gate verdict ≠ 生产 health state（P1, 第十九轮 §十三）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn finish(verdicts: &[(&'static str, bool, String)], stopped_at: &str) -> ! {
    let ok = verdicts.iter().all(|(_, p, _)| *p);
    println!(
        "=== A2-8 Dual Input Gate: {} ({}/{} verdicts; {stopped_at}) ===",
        if ok { "ALL PASS" } else { "FAIL" },
        verdicts.iter().filter(|(_, p, _)| *p).count(),
        verdicts.len()
    );
    std::process::exit(if ok { 0 } else { 2 })
}

/// H2（第十九轮 §十六）: 单设备 Port 证据闭环检查——该设备恰一 Input
/// Resource 且其 ID == Manifest Input Port 的规范派生。多 Input Resource
/// （多输入卡）或 ID 不对应（跨端口污染）即 false; 与 L0 形态检查互为纵深。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn device_input_resource_closure(
    resources: &crate::resource::ResourceRegistry,
    device_id: uuid::Uuid,
    manifest_port: Option<uuid::Uuid>,
) -> bool {
    let expected = manifest_port.map(crate::resource::input_resource_id_for_port);
    let mut hits = resources
        .resources
        .iter()
        .filter(|r| r.device_id == device_id && r.capability.ends_with("-input"));
    match (hits.next(), hits.next()) {
        (Some(only), None) => Some(only.id) == expected,
        _ => false,
    }
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
#[allow(clippy::too_many_arguments)]
pub fn run(
    cfg: &Config,
    devices: &[DeviceInfo],
    discovered: &[DiscoveredDevice],
    lm: &Arc<InMemoryLeaseManager>,
    _sup: &Arc<std::sync::Mutex<Supervisor>>,
    // P1（§十三）: 本 Gate 不写 agent_state（verdict ≠ 生产 health state）;
    // 保留参数维持 gate 签名一致（bin/gates.rs 传位不变）。
    _agent_state: &Arc<std::sync::Mutex<AgentState>>,
    event_sink: &Arc<dyn RuntimeEventSink>,
    internal_log: &Arc<RuntimeEventLog>,
) {
    if std::env::var("VBMF_A2_8_DUAL_INPUT").is_err() {
        return;
    }
    println!("=== A2-8 Dual Input Gate: L1→L5 真机五层验收开始 ===");

    // ── 前置 0: manifest + discovery + registry（形态 fail-closed）──
    let manifest_path = match &cfg.device_binding_path {
        Some(p) => p.clone(),
        None => {
            eprintln!(
                "VBMF_A2_8_DUAL_INPUT 需要 DeviceBindingManifest (MEDIA_AGENT_DEVICE_BINDING)"
            );
            std::process::exit(2);
        }
    };
    let manifest = match crate::resolver::DeviceBindingManifest::load(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("manifest 加载失败: {e}");
            std::process::exit(2);
        }
    };
    if manifest.validate_manifest().is_err() {
        eprintln!("manifest 结构校验失败");
        std::process::exit(2);
    }
    let gst_probes =
        match crate::resolver::probe_gstreamer_devices(crate::resolver::MAX_PROBE_DEVICES, false) {
            crate::resolver::GstProbeOutcome::Available { probes, .. } => probes,
            other => {
                eprintln!(
                    "GStreamer probe 不可用（{other:?}）——L1 Signal 证据无从取得, fail-closed"
                );
                std::process::exit(2);
            }
        };
    let bindings =
        crate::resolver::collect_bindings_from_manifest(discovered, &gst_probes, &manifest);
    let registry =
        match crate::port::PortRegistry::build(discovered, &gst_probes, &manifest, &bindings) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("PortRegistry 构建失败 (fail-closed): {e:?}");
                std::process::exit(2);
            }
        };

    // ── L1: Identity / Port / Capability / Signal ──
    let input_ports: Vec<&PortInfo> = registry
        .ports
        .iter()
        .filter(|p| p.direction == PortDirection::Input && p.identity.port_id.is_some())
        .collect();
    // 02-I 硬件形态: 恰 2 个 Input port 且分属 2 台设备（一块多输入卡拒绝）。
    let device_ids: Vec<uuid::Uuid> = {
        let mut v: Vec<uuid::Uuid> = input_ports.iter().map(|p| p.device_id).collect();
        v.sort();
        v.dedup();
        v
    };
    if input_ports.len() != 2 || device_ids.len() != 2 {
        eprintln!(
            "02-I 硬件形态 fail-closed: 需两块独立单输入卡（恰 2 个 Input port / 2 台设备）; \
             实测 ports={} devices={}（多输入卡/双工双声明不在 02-I 形态——N×M 见独立 change）",
            input_ports.len(),
            device_ids.len()
        );
        std::process::exit(2);
    }
    let mut verdicts: Vec<(&'static str, bool, String)> = Vec::new();

    // 每设备 (device_number, signal) 一次采样——H4 证据行与 L1c 判定共用。
    let dn_sig: Vec<(uuid::Uuid, Option<u32>, Option<bool>)> = device_ids
        .iter()
        .map(|d| {
            let dn = bindings.get(d).map(|b| b.device_number);
            let sig = dn.and_then(|n| {
                gst_probes
                    .iter()
                    .find(|p| p.device_number == n)
                    .and_then(|p| p.signal)
            });
            (*d, dn, sig)
        })
        .collect();

    // H4（第十九轮 §十六）: 每端口一行一一对应证据——DeviceHandle / DeviceId /
    // PortId / connector / ordinal / Direction / Capability / Signal / GStreamer
    // device_number 同行, 现场验收报告的最小可审计行（非"device X signal=true"
    // 式散点）。
    let l1_evidence: Vec<String> = device_ids
        .iter()
        .zip(&dn_sig)
        .map(|(d, (_, dn, sig))| {
            let port = input_ports
                .iter()
                .find(|p| p.device_id == *d)
                .expect("形态已验: 每设备恰一 Input port");
            let handle = discovered
                .iter()
                .find(|dev| dev.device.device_id == *d)
                .and_then(crate::resolver::identity_handle);
            format!(
                "{d}: handle={handle:?} port_id={:?} conn={:?} ordinal={:?} dir=Input \
                 cap.input={:?} cap.audio={:?}(video-推导) dn={dn:?} signal={sig:?} prod_binding={}",
                port.identity.port_id,
                port.identity.connector,
                port.identity.ordinal,
                port.capabilities.input,
                port.capabilities.audio_input,
                bindings.get(d).is_some_and(|b| b.is_production_grade()),
            )
        })
        .collect();
    println!("=== A2-8 L1 端口证据（一一对应, H4）===");
    for r in &l1_evidence {
        println!("  {r}");
    }

    // L1a 身份/绑定: 双设备均有生产级 binding（DeviceHandle→Resolver→device-number）。
    let l1a = device_ids
        .iter()
        .all(|d| bindings.get(d).is_some_and(|b| b.is_production_grade()));
    record(
        &mut verdicts,
        "L1a Identity/Binding",
        l1a,
        format!(
            "devices={:?} bindings={}/{} production_grade",
            device_ids,
            device_ids
                .iter()
                .filter(|d| bindings.get(d).is_some_and(|b| b.is_production_grade()))
                .count(),
            device_ids.len()
        ),
    );

    // L1b Capability（SDK 位掩码证据; 三列分记——audio=video 推导工程事实不报独立探针）。
    let l1b = input_ports
        .iter()
        .all(|p| matches!(p.capabilities.input, CapabilityValue::Supported(true)));
    let cap_note: Vec<String> = input_ports
        .iter()
        .map(|p| {
            format!(
                "{:?}/{:?}/input={:?} audio_input={:?}(video-推导工程事实)",
                p.device_id, p.identity.connector, p.capabilities.input, p.capabilities.audio_input
            )
        })
        .collect();
    record(
        &mut verdicts,
        "L1b Capability(SDK mask)",
        l1b,
        cap_note.join(" | "),
    );

    // L1c Signal Locked（真实探测; Direction 由 manifest 声明, 与 Capability/Signal 分记）。
    let l1c = dn_sig.iter().all(|(_, _, sig)| *sig == Some(true));
    let l1c_detail = dn_sig
        .iter()
        .map(|(d, dn, sig)| format!("{d:?}/dn={dn:?}/signal={sig:?}"))
        .collect::<Vec<_>>()
        .join(" | ");
    record(&mut verdicts, "L1c Signal Locked", l1c, l1c_detail);

    // L1d Port↔Resource 闭环（H2, 第十九轮 §十六）: 当前拓扑下 Manifest Input
    // Port → Registry Port → 唯一 Input Resource → Session 证据闭合。每设备恰一
    // Input Resource 且 ID == manifest port 规范派生——`derive_claims()` 取首
    // "-input" 在唯一资源下必然命中它（不动 SessionManager; 精确 port 寻址属
    // PORT-IDENTITY-AND-RESOURCE-ADDRESSING; 多输入卡在此即 FAIL）。
    let resource_registry = crate::resource::ResourceRegistry::derive_from_discovery(&registry);
    let l1d_rows: Vec<(bool, String)> = device_ids
        .iter()
        .map(|d| {
            let port_id = input_ports
                .iter()
                .find(|p| p.device_id == *d)
                .and_then(|p| p.identity.port_id);
            let ok = device_input_resource_closure(&resource_registry, *d, port_id);
            (
                ok,
                format!("{d}: manifest_port={port_id:?} 唯一InputResource+ID对应={ok}"),
            )
        })
        .collect();
    let l1d = l1d_rows.iter().all(|(ok, _)| *ok);
    record(
        &mut verdicts,
        "L1d Port↔Resource closure",
        l1d,
        l1d_rows
            .iter()
            .map(|(_, s)| s.clone())
            .collect::<Vec<_>>()
            .join(" | "),
    );

    // H1 fail-stop（第十九轮 §三/§十六）: Gate 非"诊断脚本"——L1 任一 FAIL 即停,
    // 绝不进入 L2（此点前无会话/租约让位, 零清理直接终裁）。
    if !(l1a && l1b && l1c && l1d) {
        finish(&verdicts, "L1 fail-stop——L2-L5 不执行（H1）");
    }

    // ── L2: Execution（双输入 Session + MediaTap + Program Graph）──
    let resources = crate::resource::SharedResourceRegistry::new(resource_registry);
    let bundle = match crate::registry::AdapterRegistry::build_media_adapter_bundle() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("adapter feature 冲突 (fail-closed): {e}");
            std::process::exit(2);
        }
    };
    let ctrl: Arc<dyn MediaBackend> = bundle.backend.clone();
    let media_tap_port: Option<Arc<dyn MediaTapPort>> = bundle.media_tap.clone();
    let bridge_port = bundle.bridge_observation.clone();
    let mgr = Arc::new(crate::session::SessionManager::new(
        resources.clone(),
        lm.clone(),
        _sup.clone(),
        ctrl.clone(),
        Arc::new(devices.to_vec()),
        Arc::new(bindings.clone()),
        Some(registry.clone()),
        crate::pipeline::MaterializeMode::Diagnostic,
        crate::session::SessionTuning::default(),
        event_sink.clone(),
    ));
    // bootstrap 占位租约让位（双设备; 真实会话租约接管排他性）。
    for d in &device_ids {
        let _ = lm.release(&crate::lease::DeviceLease {
            device_id: *d,
            owner: "bootstrap".into(),
            acquired_at: chrono::Utc::now(),
            ttl: std::time::Duration::from_secs(60),
        });
    }
    let intent = crate::graph_intent::GraphRuntimeIntent {
        version: "1.0".into(),
        // 纯分析（appsink）——Gate 无输出 env 依赖; 双设备 intent。
        // H3（第十九轮 §十六）: 每设备携带已验证的 Manifest Input Port——
        // materialize 精确消费该字段定位端口（Some→registry 精确匹配出
        // connector, 无匹配生产 fail-closed）; None 才回退设备首输入端口。
        devices: device_ids
            .iter()
            .map(|id| {
                let port_id = input_ports
                    .iter()
                    .find(|p| p.device_id == *id)
                    .and_then(|p| p.identity.port_id);
                crate::graph_intent::DeviceIntent {
                    device_id: id.to_string(),
                    role: "CAPTURE".into(),
                    pipeline: crate::graph_intent::PipelineIntent {
                        source: crate::graph_intent::SourceIntent {
                            kind: "decklink".into(),
                            device_id: id.to_string(),
                            port_id: port_id.map(|u| u.to_string()),
                        },
                        sink: crate::graph_intent::SinkIntent {
                            kind: "appsink".into(),
                        },
                    },
                }
            })
            .collect(),
    };
    let session_res = mgr
        .create(intent)
        .and_then(|sid| mgr.start(&sid).map(|_| sid));
    let sid = match session_res {
        Ok(sid) => sid,
        Err(e) => {
            eprintln!("A2-8 L2 Session create/start 失败 (fail-closed): {e:?}");
            finish(
                &verdicts,
                "L2 Session create/start fail-stop——L2b-L5 不执行",
            );
        }
    };
    let started_inputs: Vec<SessionInput> = mgr.status(&sid).map(|s| s.inputs).unwrap_or_default();
    let l2a = started_inputs.len() == 2;
    record(
        &mut verdicts,
        "L2a Session dual-input",
        l2a,
        format!(
            "session={} started_inputs={} port_id=manifest port 已携带（H3: materialize 精确定位, 非 None 回退首端口）",
            sid.0,
            started_inputs.len()
        ),
    );
    if !l2a {
        let _ = mgr.stop(&sid);
        finish(
            &verdicts,
            "L2a fail-stop——双输入 Session 未成立, L2b-L5 不执行",
        );
    }
    let initial_active = started_inputs[0].device_id;
    let group = match crate::switch_execution::ExecutionGroup::new(
        sid,
        started_inputs.clone(),
        initial_active,
    ) {
        Ok(g) => g,
        Err(e) => {
            let _ = mgr.stop(&sid);
            eprintln!("A2-8 L2 ExecutionGroup 构造失败: {e:?}");
            finish(&verdicts, "L2 ExecutionGroup fail-stop——L2b-L5 不执行");
        }
    };
    let switcher: Arc<dyn SwitchExecutionAdapter> =
        Arc::new(crate::adapters::gstreamer::GStreamerSwitchAdapter::bridged());
    let tap_wirings: Vec<crate::program_execution::TapWiring> = started_inputs
        .iter()
        .map(crate::program_execution::TapWiring::for_input)
        .collect();
    let rt = match crate::program_execution::ProgramExecutionRuntime::create(
        sid,
        group,
        switcher.clone(),
        media_tap_port.clone(),
        tap_wirings,
    ) {
        Ok(rt) => Arc::new(rt),
        Err(e) => {
            let _ = mgr.stop(&sid);
            eprintln!("A2-8 L2 ProgramExecutionRuntime 创建失败: {e:?}");
            finish(
                &verdicts,
                "L2 ProgramExecutionRuntime fail-stop——L2b-L5 不执行",
            );
        }
    };
    // 停止链接线: Session stop → hook → Program Stop→Tap Detach→Input Stop→Release。
    mgr.register_stop_hook(&sid, rt.clone());
    let graph = rt.graph_handle().expect("active graph");
    let group_arc = rt.group_arc().expect("active group");
    sleep(SETTLE_SECS);

    // H1 链尾共用: 层间 fail-stop 仍走完整 Teardown（验证停止链 + 释放资源）,
    // 但不进入下一层; Teardown verdict 如常入账。
    let teardown = |verdicts: &mut Vec<(&'static str, bool, String)>| -> bool {
        let stop_ok = mgr.stop(&sid).is_ok();
        sleep(1);
        let runtime_inactive = !rt.is_active();
        let released = mgr
            .status(&sid)
            .is_some_and(|s| matches!(s.phase, SessionPhase::Released));
        record(
            verdicts,
            "Teardown 停止链",
            stop_ok && runtime_inactive && released,
            format!(
                "session_stop={stop_ok} program_runtime_inactive={runtime_inactive} phase_released={released} (Program Stop→Tap Detach→Input Stop→Release)"
            ),
        );
        stop_ok && runtime_inactive && released
    };

    // L2b: 双 MediaTap 真实存在（attach 簿记 + 桥观测行可查）。
    let bridge_rows_of = |input: &SessionInput| -> Option<BridgeChannelLiveness> {
        bridge_port
            .as_ref()?
            .bridge_liveness(&input.handle, LIVENESS_WINDOW_MS)
            .into_iter()
            .find(|l| l.channel == crate::program_execution::tap_channel(input.device_id))
    };
    let l2b_rows: Vec<String> = started_inputs
        .iter()
        .map(|i| {
            format!(
                "{}: frames={:?}",
                crate::program_execution::tap_channel(i.device_id),
                bridge_rows_of(i).map(|l| l.frames)
            )
        })
        .collect();
    let l2b = l2b_rows.iter().all(|r| !r.contains("frames=None"));
    record(
        &mut verdicts,
        "L2b MediaTap/Bridge wired",
        l2b,
        l2b_rows.join(" | "),
    );
    if !l2b {
        // H1 fail-stop（§三链）: L2b 失败 → 完整 Teardown 后终裁, 不进入 L3-L5。
        teardown(&mut verdicts);
        finish(&verdicts, "L2b fail-stop——L3-L5 不执行（H1）");
    }

    // ── L3: Output（帧计数与 PTS 真实增长——非 PLAYING 态）──
    let obs1 = switcher.observe(&graph).program;
    sleep(SAMPLE_GAP_SECS);
    let obs2 = switcher.observe(&graph).program;
    let l3 = crate::program_execution::program_progress_since(&obs1, &obs2)
        && obs2.program_video_pts.is_some()
        && obs2.program_video_pts_state != PtsMonotonicity::NonMonotonic
        && obs2.program_audio_pts.is_some();
    record(
        &mut verdicts,
        "L3 Program output advancing",
        l3,
        format!(
            "video_frames {}→{} audio_frames {}→{} pts v={:?}/a={:?} state v={:?}",
            obs1.program_video_frames,
            obs2.program_video_frames,
            obs1.program_audio_frames,
            obs2.program_audio_frames,
            obs2.program_video_pts,
            obs2.program_audio_pts,
            obs2.program_video_pts_state
        ),
    );
    if !l3 {
        // H1 fail-stop（§三链）: L3 失败 → 完整 Teardown 后终裁, 不进入 L4/L5。
        teardown(&mut verdicts);
        finish(&verdicts, "L3 fail-stop——L4-L5 不执行（H1）");
    }

    // ── L4: Timing（三列 PTS 同采, 只测量不 normalize）──
    let sample_row = |input: &SessionInput,
                      prog: &ProgramObservation|
     -> crate::program_execution::TimelineSample {
        let health = crate::pipeline_events::read_health(&input.handle);
        let bridge = bridge_port.as_ref().and_then(|bp| {
            bp.bridge_observations(&input.handle)
                .into_iter()
                .find(|b| b.channel == crate::program_execution::tap_channel(input.device_id))
        });
        crate::program_execution::assemble_timeline_sample(
            input.device_id,
            health.as_ref(),
            bridge.as_ref(),
            prog,
        )
    };
    let print_row = |label: &str, s: &crate::program_execution::TimelineSample| {
        println!(
            "  [{label}] {} in(v/a)={:?}/{:?} bridge(v/a)={:?}/{:?} prog(v/a)={:?}/{:?} states in={:?}/{:?} bridge={:?}/{:?} prog={:?}/{:?} alive={}",
            s.device,
            s.input_video_pts, s.input_audio_pts,
            s.bridge_video_pts, s.bridge_audio_pts,
            s.program_video_pts, s.program_audio_pts,
            s.input_video_state, s.input_audio_state,
            s.bridge_video_state, s.bridge_audio_state,
            s.program_video_state, s.program_audio_state,
            s.program_alive
        );
    };
    let pre_a = sample_row(&started_inputs[0], &obs2);
    let pre_b = sample_row(&started_inputs[1], &obs2);
    println!("=== A2-8 L4 三列 PTS 证据（pre-switch; 只测量）===");
    print_row("pre A", &pre_a);
    print_row("pre B", &pre_b);

    // A→B 切换（诊断消费方经 Runtime 全链直驱——timeline orchestration ①-⑩;
    // Supervisor 不介入。C-TIMELINE-01 ⑫: L4=switch 正确 ∧ Timeline 九项合取）。
    let target_b = started_inputs[1].device_id;
    let switch_res = rt.switch_program(&crate::switch_execution::SwitchIntent {
        target: target_b,
        policy: crate::program::SwitchPolicy::FrameSwitch,
    });
    let mut l4 = false;
    let l4_detail: String;
    match switch_res {
        Ok(report) => {
            sleep(SETTLE_SECS);
            let post = switcher.observe(&graph).program;
            // L4-SWITCH 语义保持（既有判据——Desired=Observed 落定）。
            let completed = post.observed_active.is_some_and(|a| {
                matches!(
                    group_arc.lock().unwrap().desired,
                    crate::switch_execution::SwitchDesired::ActiveInput(id) if id == a
                )
            });
            let post_a = sample_row(&started_inputs[0], &post);
            let post_b = sample_row(&started_inputs[1], &post);
            println!("=== A2-8 L4 三列 PTS 证据（post-switch; 只测量）===");
            print_row("post A", &post_a);
            print_row("post B", &post_b);
            let l4_switch = completed
                && post.observed_active == Some(target_b)
                && report.executed.av_epoch == 1
                && post.program_video_pts_state != PtsMonotonicity::NonMonotonic
                && post.program_video_pts.is_some();
            // L4-TIMELINE 九项合取（IMP-7/第三十一轮 §十三——Preserve 证据
            // 链=transition declared∧Segment(B) observed∧首枚映射缓冲观测∧
            // mapped 连续∧V 连续∧A 连续∧epoch 一致∧无未声明回退）。
            let l4_timeline = match &report.outcome {
                crate::program_timeline::TransitionOutcome::Preserved { mapped, .. } => {
                    let ev = &mapped.evidence;
                    let pre_v = obs2.program_video_pts.unwrap_or(0);
                    ev.observed_segment == ev.declared_segment
                        && ev.video_continuity
                            == crate::program_timeline::PlaneContinuity::Continuous
                        && ev.audio_continuity
                            == crate::program_timeline::PlaneContinuity::Continuous
                        && ev.undeclared_backward_jump.is_none()
                        && ev.discontinuity_state != PtsMonotonicity::NonMonotonic
                        && ev.program_epoch == report.observation.program_epoch
                        && ev.mapped_program_pts >= pre_v
                        && post
                            .program_video_pts
                            .is_some_and(|p| p >= ev.mapped_program_pts)
                        && report.observation.mapped_program_pts.is_some()
                }
                other => {
                    println!("=== A2-8 L4 Timeline 结局非 Preserve: {other:?} ===");
                    false
                }
            };
            l4 = l4_switch && l4_timeline;
            l4_detail = format!(
                "switch epoch={} observed={:?} completed={} prog_v={:?} state={:?} switch_ok={l4_switch} | timeline_ok={l4_timeline} outcome={:?} epoch={:?} seg={:?} src_pts={:?} mapped={:?} offset={:?} v/a={:?}/{:?} disc={:?}",
                report.executed.av_epoch,
                post.observed_active,
                completed,
                post.program_video_pts,
                post.program_video_pts_state,
                report.outcome,
                report.observation.program_epoch,
                report.observation.segment_id,
                report.observation.input_pts,
                report.observation.mapped_program_pts,
                report.observation.mapping_offset,
                report.observation.video_continuity,
                report.observation.audio_continuity,
                report.observation.discontinuity_state,
            );
        }
        Err(e) => {
            l4_detail = format!("切换失败: {e:?}");
        }
    }
    record(
        &mut verdicts,
        "L4 Timing/switch+timeline(A→B)",
        l4,
        l4_detail,
    );

    // ── L5: Failure isolation + recover + teardown ──
    let (h_a, h_b) = (started_inputs[0].handle, started_inputs[1].handle);
    let ch_a = crate::program_execution::tap_channel(started_inputs[0].device_id);
    let ch_b = crate::program_execution::tap_channel(started_inputs[1].device_id);
    let mut l5_all = true;
    let mut l5_notes: Vec<String> = Vec::new();

    if l4 {
        // 5.1 A fail → B alive（active=B: program 不受牵连）。
        let a_health_pre = crate::pipeline_events::read_health(&h_a);
        let _ = ctrl.stop(&h_a);
        sleep(L5_WAIT_SECS);
        let a_health_post = crate::pipeline_events::read_health(&h_a);
        let a_advancing = match (&a_health_pre, &a_health_post) {
            (Some(p), Some(c)) => crate::program_execution::input_progress_since(p, c),
            _ => false,
        };
        let b_alive = bridge_rows_of(&started_inputs[1]).is_some_and(|l| l.alive_in_window);
        let p1 = switcher.observe(&graph).program;
        sleep(SAMPLE_GAP_SECS);
        let p2 = switcher.observe(&graph).program;
        let prog_adv = crate::program_execution::program_progress_since(&p1, &p2);
        let l5a = !a_advancing && b_alive && prog_adv;
        l5_all &= l5a;
        l5_notes.push(format!(
            "A-fail→B-alive={} (inputA_advancing={b_alive_sig} bridgeB_alive={b_alive} program_advancing={prog_adv})",
            l5a,
            b_alive_sig = a_advancing,
        ));

        // 5.2 recover A → 桥真实复流（观察恢复非簿记重放）。
        let rec = ctrl.recover(&h_a).is_ok();
        sleep(SETTLE_SECS + SAMPLE_GAP_SECS);
        let liveness_a: Vec<BridgeChannelLiveness> = bridge_port
            .as_ref()
            .map(|bp| bp.bridge_liveness(&h_a, LIVENESS_WINDOW_MS))
            .unwrap_or_default();
        let report = crate::program_execution::assemble_bridge_health(
            rec,
            vec![ch_a.clone(), ch_b.clone()],
            &liveness_a
                .iter()
                .chain(
                    bridge_port
                        .as_ref()
                        .map(|bp| bp.bridge_liveness(&h_b, LIVENESS_WINDOW_MS))
                        .unwrap_or_default()
                        .iter(),
                )
                .cloned()
                .collect::<Vec<_>>(),
        );
        let a_bridge_back = liveness_a
            .iter()
            .find(|l| l.channel == ch_a)
            .is_some_and(|l| l.alive_in_window);
        let l5b = rec && a_bridge_back && !report.bridge_degraded;
        l5_all &= l5b;
        l5_notes.push(format!(
            "recover-A→桥复流={l5b} (recovered={rec} bridgeA_alive={a_bridge_back} degraded={})",
            report.bridge_degraded
        ));

        // 5.3 B fail → A alive（active=B: program 诚实停滞——隔离证据=A 不受牵连）。
        let _ = ctrl.stop(&h_b);
        sleep(L5_WAIT_SECS);
        let b_bridge_alive = bridge_rows_of(&started_inputs[1]).is_some_and(|l| l.alive_in_window);
        let a_bridge_alive = bridge_rows_of(&started_inputs[0]).is_some_and(|l| l.alive_in_window);
        let l5c = !b_bridge_alive && a_bridge_alive;
        l5_all &= l5c;
        l5_notes.push(format!(
            "B-fail→A-alive={} (bridgeB_alive={b_bridge_alive} bridgeA_alive={a_bridge_alive})",
            l5c
        ));

        // 5.4 故障域分类（真实观测行; 单故障分类器不越域）:
        //     A 行 input 活+桥活+program 停滞 → Program 域（不归因输入）;
        //     B 行 input 停滞 → Input 域。
        let a1 = crate::pipeline_events::read_health(&h_a);
        sleep(SAMPLE_GAP_SECS);
        let a2 = crate::pipeline_events::read_health(&h_a);
        let a_input_adv = match (&a1, &a2) {
            (Some(p), Some(c)) => crate::program_execution::input_progress_since(p, c),
            _ => false,
        };
        let q1 = switcher.observe(&graph).program;
        sleep(SAMPLE_GAP_SECS);
        let q2 = switcher.observe(&graph).program;
        let prog_adv2 = crate::program_execution::program_progress_since(&q1, &q2);
        let row_a = crate::program_execution::classify_failure_domain(
            a_input_adv,
            a_bridge_alive,
            prog_adv2,
        );
        let row_b = crate::program_execution::classify_failure_domain(false, false, prog_adv2);
        let l5d = matches!(row_a, crate::program_execution::FailureDomain::Program)
            && matches!(row_b, crate::program_execution::FailureDomain::Input);
        l5_all &= l5d;
        l5_notes.push(format!(
            "故障域不越域={} (A行={row_a:?} B行={row_b:?}; Program 停滞不归因存活输入)",
            l5d
        ));

        // 恢复 B（teardown 前还原运行态, best-effort）。
        let _ = ctrl.recover(&h_b);
        sleep(SAMPLE_GAP_SECS);
    } else {
        l5_all = false;
        l5_notes.push("L4 未过——L5 注入序列跳过".into());
    }
    record(
        &mut verdicts,
        "L5 Failure isolation/recover",
        l5_all,
        l5_notes.join(" | "),
    );
    println!(
        "=== A2-8 L5 Supervisor 角色: recovery decision 非 switch executor——切换经 SwitchExecutionAdapter 直驱, Supervisor 未持有 switch 面（wiring 事实）==="
    );

    // ── Teardown: Session stop → hook → Program Stop→Tap Detach→Input Stop→Release ──
    let _ = internal_log; // 诊断 world 完整性（本 Gate 不消费双日志投影）
    teardown(&mut verdicts);
    finish(&verdicts, "全链完成 L0→L5+Teardown");
}

/// H2 语义锁（第十九轮 §十六）: L1d 单设备闭环检查的纯函数面——
/// 多输入卡/跨端口污染/零资源三路 fail-closed + 唯一对应正路。
#[cfg(all(test, feature = "bmd-provider", feature = "gstreamer-backend"))]
mod tests {
    use super::*;

    fn res(id: uuid::Uuid, cap: &str, dev: uuid::Uuid) -> crate::resource::Resource {
        let mut r = crate::resource::Resource::new(id, format!("{cap}-{id}"), cap, 1);
        r.device_id = dev;
        r
    }

    #[test]
    fn unique_input_resource_matching_port_derivation_passes() {
        let dev = uuid::Uuid::new_v4();
        let port = uuid::Uuid::new_v4();
        let rr = crate::resource::ResourceRegistry {
            resources: vec![
                res(
                    crate::resource::input_resource_id_for_port(port),
                    "sdi-input",
                    dev,
                ),
                res(uuid::Uuid::new_v4(), "sdi-output", dev), // 非 -input 平面不计入
            ],
        };
        assert!(device_input_resource_closure(&rr, dev, Some(port)));
    }

    #[test]
    fn multiple_input_resources_fail_closed() {
        // 多输入卡（一设备两 Input Resource）——即使其一对应 manifest port 也拒绝。
        let dev = uuid::Uuid::new_v4();
        let port = uuid::Uuid::new_v4();
        let rr = crate::resource::ResourceRegistry {
            resources: vec![
                res(
                    crate::resource::input_resource_id_for_port(port),
                    "sdi-input",
                    dev,
                ),
                res(
                    crate::resource::input_resource_id_for_port(uuid::Uuid::new_v4()),
                    "hdmi-input",
                    dev,
                ),
            ],
        };
        assert!(!device_input_resource_closure(&rr, dev, Some(port)));
    }

    #[test]
    fn mismatched_derivation_fails() {
        // 唯一 Input Resource 但 ID ≠ manifest port 规范派生（跨端口污染防线）。
        let dev = uuid::Uuid::new_v4();
        let rr = crate::resource::ResourceRegistry {
            resources: vec![res(uuid::Uuid::new_v4(), "sdi-input", dev)],
        };
        assert!(!device_input_resource_closure(
            &rr,
            dev,
            Some(uuid::Uuid::new_v4())
        ));
    }

    #[test]
    fn zero_input_resource_fails() {
        // 零 Input Resource（manifest 声明与 Resource 派生脱节）同样 fail-closed。
        let dev = uuid::Uuid::new_v4();
        let rr = crate::resource::ResourceRegistry {
            resources: vec![res(uuid::Uuid::new_v4(), "sdi-output", dev)],
        };
        assert!(!device_input_resource_closure(
            &rr,
            dev,
            Some(uuid::Uuid::new_v4())
        ));
    }
}
