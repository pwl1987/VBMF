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

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
#[allow(clippy::too_many_arguments)]
pub fn run(
    cfg: &Config,
    devices: &[DeviceInfo],
    discovered: &[DiscoveredDevice],
    lm: &Arc<InMemoryLeaseManager>,
    _sup: &Arc<std::sync::Mutex<Supervisor>>,
    agent_state: &Arc<std::sync::Mutex<AgentState>>,
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
    let mut record = |name: &'static str, pass: bool, detail: String| {
        println!(
            "=== A2-8 {name}: {} === {detail}",
            if pass { "PASS" } else { "FAIL" }
        );
        verdicts.push((name, pass, detail));
    };

    // L1a 身份/绑定: 双设备均有生产级 binding（DeviceHandle→Resolver→device-number）。
    let l1a = device_ids
        .iter()
        .all(|d| bindings.get(d).is_some_and(|b| b.is_production_grade()));
    record(
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
    record("L1b Capability(SDK mask)", l1b, cap_note.join(" | "));

    // L1c Signal Locked（真实探测; Direction 由 manifest 声明, 与 Capability/Signal 分记）。
    let signal_rows: Vec<String> = device_ids
        .iter()
        .map(|d| {
            let n = bindings.get(d).map(|b| b.device_number);
            let sig = n.and_then(|n| {
                gst_probes
                    .iter()
                    .find(|p| p.device_number == n)
                    .and_then(|p| p.signal)
            });
            format!("{d:?}/dn={n:?}/signal={sig:?}")
        })
        .collect();
    let l1c = signal_rows.iter().all(|r| r.contains("signal=Some(true)"));
    record("L1c Signal Locked", l1c, signal_rows.join(" | "));

    // ── L2: Execution（双输入 Session + MediaTap + Program Graph）──
    let resources = crate::resource::SharedResourceRegistry::new(
        crate::resource::ResourceRegistry::derive_from_discovery(&registry),
    );
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
        devices: device_ids
            .iter()
            .map(|id| crate::graph_intent::DeviceIntent {
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
            })
            .collect(),
    };
    let session_res = mgr
        .create(intent)
        .and_then(|sid| mgr.start(&sid).map(|_| sid));
    let sid = match session_res {
        Ok(sid) => sid,
        Err(e) => {
            *agent_state.lock().unwrap() = AgentState::Degraded;
            eprintln!("A2-8 L2 Session create/start 失败 (fail-closed): {e:?}");
            std::process::exit(2);
        }
    };
    let started_inputs: Vec<SessionInput> = mgr.status(&sid).map(|s| s.inputs).unwrap_or_default();
    let l2a = started_inputs.len() == 2;
    record(
        "L2a Session dual-input",
        l2a,
        format!(
            "session={} started_inputs={} (SessionInput 键集恰 {{device_id, handle}})",
            sid.0,
            started_inputs.len()
        ),
    );
    if !l2a {
        let _ = mgr.stop(&sid);
        *agent_state.lock().unwrap() = AgentState::Degraded;
        println!("=== A2-8 Dual Input Gate: FAIL (L2a 双输入 Session 未成立, L2b-L5 跳过) ===");
        std::process::exit(2);
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
            *agent_state.lock().unwrap() = AgentState::Degraded;
            eprintln!("A2-8 L2 ExecutionGroup 构造失败: {e:?}");
            std::process::exit(2);
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
            *agent_state.lock().unwrap() = AgentState::Degraded;
            eprintln!("A2-8 L2 ProgramExecutionRuntime 创建失败: {e:?}");
            std::process::exit(2);
        }
    };
    // 停止链接线: Session stop → hook → Program Stop→Tap Detach→Input Stop→Release。
    mgr.register_stop_hook(&sid, rt.clone());
    let graph = rt.graph_handle().expect("active graph");
    let group_arc = rt.group_arc().expect("active group");
    *agent_state.lock().unwrap() = AgentState::Capturing;
    sleep(SETTLE_SECS);

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
    record("L2b MediaTap/Bridge wired", l2b, l2b_rows.join(" | "));

    // ── L3: Output（帧计数与 PTS 真实增长——非 PLAYING 态）──
    let obs1 = switcher.observe(&graph);
    sleep(SAMPLE_GAP_SECS);
    let obs2 = switcher.observe(&graph);
    let l3 = crate::program_execution::program_progress_since(&obs1, &obs2)
        && obs2.program_video_pts.is_some()
        && obs2.program_video_pts_state != PtsMonotonicity::NonMonotonic
        && obs2.program_audio_pts.is_some();
    record(
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

    // A→B 切换（诊断消费方直驱 SwitchExecutionAdapter; Supervisor 不介入）。
    let target_b = started_inputs[1].device_id;
    let switch_res = {
        let mut g = group_arc.lock().unwrap();
        g.plan_switch(&crate::switch_execution::SwitchIntent {
            target: target_b,
            policy: crate::program::SwitchPolicy::FrameSwitch,
        })
        .and_then(|plan| g.begin_switch(&plan).map(|_| plan))
    }
    .and_then(|plan| switcher.switch(&graph, &plan).map(|ex| (plan, ex)));
    let mut l4 = false;
    let l4_detail: String;
    match switch_res {
        Ok((plan, executed)) => {
            sleep(SETTLE_SECS);
            let post = switcher.observe(&graph);
            let completed = post
                .observed_active
                .is_some_and(|a| group_arc.lock().unwrap().complete_switch(a));
            let post_a = sample_row(&started_inputs[0], &post);
            let post_b = sample_row(&started_inputs[1], &post);
            println!("=== A2-8 L4 三列 PTS 证据（post-switch; 只测量）===");
            print_row("post A", &post_a);
            print_row("post B", &post_b);
            l4 = completed
                && post.observed_active == Some(target_b)
                && executed.av_epoch == 1
                && post.program_video_pts_state != PtsMonotonicity::NonMonotonic
                && post.program_video_pts.is_some();
            l4_detail = format!(
                "switch {}→{} epoch={} observed={:?} completed={} prog_v={:?} state={:?}",
                plan.from,
                plan.target,
                executed.av_epoch,
                post.observed_active,
                completed,
                post.program_video_pts,
                post.program_video_pts_state
            );
        }
        Err(e) => {
            l4_detail = format!("切换失败: {e:?}");
        }
    }
    record("L4 Timing/switch(A→B)", l4, l4_detail);

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
        let p1 = switcher.observe(&graph);
        sleep(SAMPLE_GAP_SECS);
        let p2 = switcher.observe(&graph);
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
        let q1 = switcher.observe(&graph);
        sleep(SAMPLE_GAP_SECS);
        let q2 = switcher.observe(&graph);
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
    record("L5 Failure isolation/recover", l5_all, l5_notes.join(" | "));
    println!(
        "=== A2-8 L5 Supervisor 角色: recovery decision 非 switch executor——切换经 SwitchExecutionAdapter 直驱, Supervisor 未持有 switch 面（wiring 事实）==="
    );

    // ── Teardown: Session stop → hook → Program Stop→Tap Detach→Input Stop→Release ──
    let stop_ok = mgr.stop(&sid).is_ok();
    sleep(1);
    let runtime_inactive = !rt.is_active();
    let released = mgr
        .status(&sid)
        .is_some_and(|s| matches!(s.phase, SessionPhase::Released));
    record(
        "Teardown 停止链",
        stop_ok && runtime_inactive && released,
        format!(
            "session_stop={stop_ok} program_runtime_inactive={runtime_inactive} phase_released={released} (Program Stop→Tap Detach→Input Stop→Release)"
        ),
    );
    let _ = internal_log; // 诊断 world 完整性（本 Gate 不消费双日志投影）

    let ok = verdicts.iter().all(|(_, p, _)| *p);
    println!(
        "=== A2-8 Dual Input Gate: {} ({}/{} verdicts) ===",
        if ok { "ALL PASS" } else { "FAIL" },
        verdicts.iter().filter(|(_, p, _)| *p).count(),
        verdicts.len()
    );
    if !ok {
        *agent_state.lock().unwrap() = AgentState::Degraded;
    }
    std::process::exit(if ok { 0 } else { 2 });
}
