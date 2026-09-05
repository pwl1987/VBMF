//! A2-8-04（R52）: VBMF_A2_8_04_OBS 多场景六路观测 Gate——observation only。
//!
//! 第五十二轮授权（Unit A = 多场景证据采集）: A→B / B→A 交替多次 · 连续切换
//! （DWELL=0 即 burst）· 长窗 N×switch（`_N` 调大）· 异构 format 边界
//! 1080i25↔1080p25——本仓硬件形态天然异构, 每设备 negotiated caps 随头部
//! 证据行打印（S5 场景内生, 无需另建源）。
//!
//! 纪律（R52 §五/§十 + 04-探针 §7 红线继承）:
//! - **只采集不判定**: 无 PASS/FAIL 裁决、无阈值（T2: ns 可比 ≠ 阈值授权）;
//!   L4/L5 判据面零触碰——本 Gate 与 dual_input 五层 Gate 正交, 不含任何
//!   判据代码, 汇总只打计数/序列（数据非裁决）;
//! - **零 Domain API 扩张**: 复用 R51 `SixPathEvidence`/`PathEvidence`/
//!   `EvidencePhase`/`assemble_six_path_evidence` 投影——不造第二套
//!   TimelineEvidence/TimelineProbe/PathHealth;
//! - **exit 契约=采集完整性, 非时间线裁决**: 0 = N 次切换全部执行且证据行
//!   打印（PTS/推进观察值本身不作 exit 依据）; 2 = 前置失败或任一切换 Err
//!   （证据链如实截断——首跑失败留证, 禁为跑绿改任何面）;
//! - **Gap B 不在本轮**: switch_graph.rs `first_mapped.is_some()→
//!   DiscontinuityDeclared` 语义过宽归 R53 C-TIMELINE correctness change——
//!   本 Gate 只如实打印 adapter 行 pts_state, 不解释不修。
//!
//! Gate 语义纪律同 dual_input: gate-local 诊断 world（manifest→probes→
//! bindings→registry→resources→bundle→SessionManager）, 复用调用方运行时
//! 基础件; env 未命中即返回不 exit。P1: 本 Gate 不写 agent_state。

// 全量 feature 门控（同 dual_input 惯例）: 本模块只服务真机观测, 默认构建
// 零编译面——纯聚合项亦不例外（否则 dead_code 击穿 clippy -D warnings）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::program_execution::SixPathEvidence;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::sync::Arc;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::config::Config;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::media_tap::MediaTapPort;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::provider::DiscoveredDevice;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::switch::SwitchExecutionAdapter;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::device::DeviceInfo;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::events::RuntimeEventSink;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::lease::{InMemoryLeaseManager, LeaseManager as _};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::port::{PortDirection, PortInfo};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::session::SessionInput;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::supervisor::Supervisor;

/// 起始稳定等待（帧累积; 与 dual_input 同值）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const SETTLE_SECS: u64 = 4;
/// 两次采样间隔（推进性判定; 与 dual_input 同值）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const SAMPLE_GAP_SECS: u64 = 3;
/// 切换返回后、SPAN 采样前的余量（switch_program 内部已 settle, 只留余量）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const POST_SWITCH_SECS: u64 = 2;
/// 默认切换次数（A→B→A…交替; `_N` 覆盖——长窗场景调大即场景 4）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const DEFAULT_SWITCHES: usize = 6;
/// 默认切换间歇（毫秒; `_DWELL_MS` 覆盖——0 即场景 3 连续切换）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const DEFAULT_DWELL_MS: u64 = 5000;

/// 展示用短设备号（uuid 前 8 位——纯格式化, 无语义）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
pub(crate) fn short_device(dev: uuid::Uuid) -> String {
    dev.to_string().chars().take(8).collect()
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn sleep_ms(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn sleep(sec: u64) {
    std::thread::sleep(std::time::Duration::from_secs(sec));
}

/// 三列原始观测 owned 快照（与 dual_input R51 同构——本模块自持, 不动五层
/// Gate 的私有面）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
struct PathSnapshot {
    input: Option<crate::pipeline::PipelineHealth>,
    bridge: Option<crate::contracts::media_tap::BridgeObservation>,
    program: crate::contracts::switch::ProgramObservation,
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
impl PathSnapshot {
    fn inputs(&self) -> crate::program_execution::SixPathInputs<'_> {
        crate::program_execution::SixPathInputs {
            input: self.input.as_ref(),
            bridge: self.bridge.as_ref(),
            program: &self.program,
        }
    }
}

/// 单次切换的采集记录（gate-local 聚合载体——非 Domain 类型, 不入
/// program_execution; 汇总只产计数/序列）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
pub(crate) struct SwitchObsRecord {
    pub idx: usize,
    /// 方向短标签（"A→B" / "B→A"; A/B=started_inputs 序）。
    pub dir: String,
    pub av_epoch: u64,
    /// TransitionOutcome 变体短名（Preserved/NewEpoch/FailClosed）。
    pub outcome_name: String,
    /// 该切换全部证据行（PRE/SPAN/POST × A/B, 序同采集）。
    pub rows: Vec<(&'static str, SixPathEvidence)>,
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
impl SwitchObsRecord {
    /// PRE/SPAN/POST 三相位各自的 program A/V delta（program 列共享——取
    /// 任一设备行同值; None 如实）。
    pub fn av_delta_series(&self) -> Vec<(&'static str, Option<u64>)> {
        ["PRE", "SPAN", "POST"]
            .into_iter()
            .map(|ph| {
                (
                    ph,
                    self.rows
                        .iter()
                        .find(|(l, _)| l.starts_with(ph))
                        .and_then(|(_, e)| e.program_av_delta_ns),
                )
            })
            .collect()
    }
}

/// 六路名（与证据行装配序一致）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
pub(crate) const PATH_NAMES: [&str; 6] = ["in_v", "in_a", "br_v", "br_a", "pr_v", "pr_a"];

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn row_paths(e: &SixPathEvidence) -> [crate::program_execution::PathEvidence; 6] {
    [
        e.input_video,
        e.input_audio,
        e.bridge_video,
        e.bridge_audio,
        e.program_video,
        e.program_audio,
    ]
}

/// 汇总: 每路 PtsMonotonicity 计数（数据非判定——计数≠异常裁决）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
pub(crate) fn tally_path_states(records: &[SwitchObsRecord]) -> Vec<String> {
    PATH_NAMES
        .into_iter()
        .enumerate()
        .map(|(idx, name)| {
            let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
            for r in records {
                for (_, e) in &r.rows {
                    *counts
                        .entry(format!("{:?}", row_paths(e)[idx].pts_state))
                        .or_default() += 1;
                }
            }
            let body = if counts.is_empty() {
                "no-rows".to_string()
            } else {
                counts
                    .into_iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            format!("{name}: {body}")
        })
        .collect()
}

/// 汇总: adv=Some(false) 出现位（推进证据为负的行——数据非判定;
/// None 行不计, absence≠false）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
pub(crate) fn non_advancing(records: &[SwitchObsRecord]) -> Vec<String> {
    let mut hits = Vec::new();
    for r in records {
        for (label, e) in &r.rows {
            for (i, p) in row_paths(e).into_iter().enumerate() {
                if p.advanced == Some(false) {
                    hits.push(format!(
                        "#{} {} dev={} path={}",
                        r.idx,
                        label,
                        short_device(e.device),
                        PATH_NAMES[i]
                    ));
                }
            }
        }
    }
    hits
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
pub fn run(
    cfg: &Config,
    devices: &[DeviceInfo],
    discovered: &[DiscoveredDevice],
    lm: &Arc<InMemoryLeaseManager>,
    _sup: &Arc<std::sync::Mutex<Supervisor>>,
    event_sink: &Arc<dyn RuntimeEventSink>,
) {
    if std::env::var("VBMF_A2_8_04_OBS").is_err() {
        return;
    }
    let n_switches: usize = std::env::var("VBMF_A2_8_04_OBS_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_SWITCHES);
    let dwell_ms: u64 = std::env::var("VBMF_A2_8_04_OBS_DWELL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_DWELL_MS);
    println!(
        "=== A2-8-04 多场景六路观测（R52: observation only·不判定·无阈值·exit=采集完整性）=== \
         N={n_switches} dwell={dwell_ms}ms（PRE 对/SPAN/POST 对每切换·交替 A↔B）"
    );

    // ── 前置 0: manifest + discovery + registry（形态 fail-closed——采集
    //    前置非判据; 与 dual_input L0 同源）──
    let manifest_path = match &cfg.device_binding_path {
        Some(p) => p.clone(),
        None => {
            eprintln!("VBMF_A2_8_04_OBS 需要 DeviceBindingManifest (MEDIA_AGENT_DEVICE_BINDING)");
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
                eprintln!("GStreamer probe 不可用（{other:?}）——采集前置失败, fail-closed");
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
    let input_ports: Vec<&PortInfo> = registry
        .ports
        .iter()
        .filter(|p| p.direction == PortDirection::Input && p.identity.port_id.is_some())
        .collect();
    let mut device_ids: Vec<uuid::Uuid> = input_ports.iter().map(|p| p.device_id).collect();
    device_ids.sort();
    device_ids.dedup();
    if input_ports.len() != 2 || device_ids.len() != 2 {
        eprintln!(
            "采集前置 fail-closed: 需两块独立单输入卡（恰 2 个 Input port / 2 台设备）; \
             实测 ports={} devices={}",
            input_ports.len(),
            device_ids.len()
        );
        std::process::exit(2);
    }

    // S5 format 证据（每设备 dn/signal/negotiated caps 同行——异构边界在案;
    // caps 缺席=None 如实, 不臆测）。
    println!("=== A2-8-04 format 证据（S5: 异构边界在案——只记录）===");
    for d in &device_ids {
        let dn = bindings.get(d).map(|b| b.device_number);
        let probe = dn.and_then(|n| gst_probes.iter().find(|p| p.device_number == n));
        println!(
            "  {d}: dn={dn:?} signal={:?} caps={:?}",
            probe.and_then(|p| p.signal),
            probe.and_then(|p| p.caps.as_ref())
        );
    }

    // ── 接线（与 dual_input L2 同源: Diagnostic 诊断 world; 不写 agent_state）──
    let resource_registry = crate::resource::ResourceRegistry::derive_from_discovery(&registry);
    let resources = crate::resource::SharedResourceRegistry::new(resource_registry);
    let bundle = match crate::registry::AdapterRegistry::build_media_adapter_bundle() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("adapter feature 冲突 (fail-closed): {e}");
            std::process::exit(2);
        }
    };
    let ctrl: Arc<dyn crate::contracts::backend::MediaBackend> = bundle.backend.clone();
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
        // 纯分析（appsink）——观测 Gate 无输出 env 依赖; H3: 每设备携带已验证
        // Manifest Input Port（materialize 精确消费）。
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
            eprintln!("Session create/start 失败 (fail-closed): {e:?}");
            std::process::exit(2);
        }
    };
    let started_inputs: Vec<SessionInput> = mgr.status(&sid).map(|s| s.inputs).unwrap_or_default();
    if started_inputs.len() != 2 {
        eprintln!("双输入 Session 未成立: inputs={}", started_inputs.len());
        let _ = mgr.stop(&sid);
        std::process::exit(2);
    }
    let label_of = |dev: uuid::Uuid| -> String {
        started_inputs
            .iter()
            .position(|i| i.device_id == dev)
            .map(|i| char::from(b'A' + i as u8).to_string())
            .unwrap_or_else(|| format!("{dev}"))
    };
    let initial_active = started_inputs[0].device_id;
    let group = match crate::switch_execution::ExecutionGroup::new(
        sid,
        started_inputs.clone(),
        initial_active,
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("ExecutionGroup 构造失败: {e:?}");
            let _ = mgr.stop(&sid);
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
            eprintln!("ProgramExecutionRuntime 创建失败: {e:?}");
            let _ = mgr.stop(&sid);
            std::process::exit(2);
        }
    };
    mgr.register_stop_hook(&sid, rt.clone());
    let graph = rt.graph_handle().expect("active graph");
    let group_arc = rt.group_arc().expect("active group");
    sleep(SETTLE_SECS);

    // 采集前置（setup 健康非判据）: program 输出须在推进, 否则证据无意义。
    let pre1 = switcher.observe(&graph).program;
    sleep(SAMPLE_GAP_SECS);
    let pre2 = switcher.observe(&graph).program;
    if !crate::program_execution::program_progress_since(&pre1, &pre2) {
        println!("=== 采集前置未满足: program 输出未推进（如实截断, 非判据）===");
        let _ = mgr.stop(&sid);
        std::process::exit(2);
    }

    let snapshot_row =
        |input: &SessionInput, prog: &crate::contracts::switch::ProgramObservation| PathSnapshot {
            input: crate::pipeline_events::read_health(&input.handle),
            bridge: bridge_port.as_ref().and_then(|bp| {
                bp.bridge_observations(&input.handle)
                    .into_iter()
                    .find(|b| b.channel == crate::program_execution::tap_channel(input.device_id))
            }),
            program: prog.clone(),
        };
    let evidence_row = |label: &str, e: &SixPathEvidence| {
        let cell = |x: &crate::program_execution::PathEvidence| {
            format!(
                "pts={:?} st={:?} fr={:?} adv={:?}",
                x.pts, x.pts_state, x.frames, x.advanced
            )
        };
        println!(
            "  [{label}] dev={} phase={:?} epoch={} av_delta={:?}ns | in_v({}) in_a({}) br_v({}) br_a({}) pr_v({}) pr_a({})",
            short_device(e.device),
            e.phase,
            e.switch_epoch,
            e.program_av_delta_ns,
            cell(&e.input_video),
            cell(&e.input_audio),
            cell(&e.bridge_video),
            cell(&e.bridge_audio),
            cell(&e.program_video),
            cell(&e.program_audio),
        );
    };
    // 追加即打印（printed 游标——每相位只打新增行, 六行序 PRE×2/SPAN×2/POST×2）。
    let print_new_rows = |rows: &[(&'static str, SixPathEvidence)], printed: &mut usize| {
        for (label, e) in rows[*printed..].iter().copied() {
            evidence_row(label, &e);
        }
        *printed = rows.len();
    };

    let mut records: Vec<SwitchObsRecord> = Vec::new();
    let mut incomplete = false;
    for k in 1..=n_switches {
        // 当前活跃读 Desired（complete_switch 后回 ActiveInput——交替自校正）。
        let current = match group_arc.lock().unwrap().desired {
            crate::switch_execution::SwitchDesired::ActiveInput(id) => id,
            stuck @ crate::switch_execution::SwitchDesired::Switching { .. } => {
                println!("=== switch #{k}: group 卡在 Switching（{stuck:?}）——如实截断 ===");
                incomplete = true;
                break;
            }
        };
        let target = started_inputs
            .iter()
            .map(|i| i.device_id)
            .find(|d| *d != current)
            .expect("双输入必存另一端");
        let dir = format!("{}→{}", label_of(current), label_of(target));
        println!("=== switch #{k}/{n_switches} {dir}（PRE 对 → 切换 → SPAN → POST 对）===");
        let mut rows: Vec<(&'static str, SixPathEvidence)> = Vec::new();
        let mut printed = 0usize;

        // PRE 对（PreSwitch; pre1→pre2 推进证据——每次切换取新鲜 program 观测）。
        let pre1_obs = switcher.observe(&graph).program;
        let pre1_a = snapshot_row(&started_inputs[0], &pre1_obs);
        let pre1_b = snapshot_row(&started_inputs[1], &pre1_obs);
        sleep(SAMPLE_GAP_SECS);
        let pre_obs2 = switcher.observe(&graph).program;
        let pre2_a = snapshot_row(&started_inputs[0], &pre_obs2);
        let pre2_b = snapshot_row(&started_inputs[1], &pre_obs2);
        rows.push((
            "PRE A",
            crate::program_execution::assemble_six_path_evidence(
                started_inputs[0].device_id,
                crate::program_execution::EvidencePhase::PreSwitch,
                Some(&pre1_a.inputs()),
                &pre2_a.inputs(),
            ),
        ));
        rows.push((
            "PRE B",
            crate::program_execution::assemble_six_path_evidence(
                started_inputs[1].device_id,
                crate::program_execution::EvidencePhase::PreSwitch,
                Some(&pre1_b.inputs()),
                &pre2_b.inputs(),
            ),
        ));
        print_new_rows(&rows, &mut printed);

        // 切换（Runtime 全链直驱 ①-⑩——与 dual_input L4 同一 API 面）。
        let report = match rt.switch_program(&crate::switch_execution::SwitchIntent {
            target,
            policy: crate::program::SwitchPolicy::FrameSwitch,
        }) {
            Ok(r) => r,
            Err(e) => {
                println!("=== switch #{k} {dir} 失败: {e:?}（如实截断——SPAN/POST 不产生）===");
                incomplete = true;
                break;
            }
        };
        let outcome_name = match &report.outcome {
            crate::program_timeline::TransitionOutcome::Preserved { .. } => "Preserved",
            crate::program_timeline::TransitionOutcome::NewEpoch { .. } => "NewEpoch",
            _ => "FailClosed",
        };
        println!(
            "  switch #{k} {dir} 执行: av_epoch={} outcome={} | tl_epoch={:?} seg={:?} src_pts={:?} mapped={:?} offset={:?} v/a={:?}/{:?} disc={:?}",
            report.executed.av_epoch,
            outcome_name,
            report.observation.program_epoch,
            report.observation.segment_id,
            report.observation.input_pts,
            report.observation.mapped_program_pts,
            report.observation.mapping_offset,
            report.observation.video_continuity,
            report.observation.audio_continuity,
            report.observation.discontinuity_state,
        );

        // SPAN（跨切换: pre2→post1）+ POST 对（post1→post2）。
        sleep(POST_SWITCH_SECS);
        let post = switcher.observe(&graph).program;
        let post1_a = snapshot_row(&started_inputs[0], &post);
        let post1_b = snapshot_row(&started_inputs[1], &post);
        rows.push((
            "SPAN A",
            crate::program_execution::assemble_six_path_evidence(
                started_inputs[0].device_id,
                crate::program_execution::EvidencePhase::PostSwitch,
                Some(&pre2_a.inputs()),
                &post1_a.inputs(),
            ),
        ));
        rows.push((
            "SPAN B",
            crate::program_execution::assemble_six_path_evidence(
                started_inputs[1].device_id,
                crate::program_execution::EvidencePhase::PostSwitch,
                Some(&pre2_b.inputs()),
                &post1_b.inputs(),
            ),
        ));
        print_new_rows(&rows, &mut printed);
        sleep(SAMPLE_GAP_SECS);
        let post_obs2 = switcher.observe(&graph).program;
        let post2_a = snapshot_row(&started_inputs[0], &post_obs2);
        let post2_b = snapshot_row(&started_inputs[1], &post_obs2);
        rows.push((
            "POST A",
            crate::program_execution::assemble_six_path_evidence(
                started_inputs[0].device_id,
                crate::program_execution::EvidencePhase::PostSwitch,
                Some(&post1_a.inputs()),
                &post2_a.inputs(),
            ),
        ));
        rows.push((
            "POST B",
            crate::program_execution::assemble_six_path_evidence(
                started_inputs[1].device_id,
                crate::program_execution::EvidencePhase::PostSwitch,
                Some(&post1_b.inputs()),
                &post2_b.inputs(),
            ),
        ));
        print_new_rows(&rows, &mut printed);
        records.push(SwitchObsRecord {
            idx: k,
            dir,
            av_epoch: report.executed.av_epoch,
            outcome_name: outcome_name.to_string(),
            rows,
        });

        if k < n_switches && dwell_ms > 0 {
            sleep_ms(dwell_ms);
        }
    }

    // ── 汇总（数据非判定: 计数/序列——阈值与判据待验收层定义 T2/T5）──
    println!("=== A2-8-04 汇总（observation only——数据非裁决·无阈值）===");
    for r in &records {
        let deltas = r
            .av_delta_series()
            .iter()
            .map(|(ph, v)| format!("{ph}={v:?}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!(
            "  #{} {} epoch={} outcome={} av_delta_ns[{}] rows={}",
            r.idx,
            r.dir,
            r.av_epoch,
            r.outcome_name,
            deltas,
            r.rows.len()
        );
    }
    println!("  六路 pts_state 计数:");
    for line in tally_path_states(&records) {
        println!("    {line}");
    }
    let non_adv = non_advancing(&records);
    println!(
        "  adv=Some(false) 出现: {}{}",
        non_adv.len(),
        if non_adv.is_empty() {
            String::new()
        } else {
            format!(" -> [{}]", non_adv.join(", "))
        }
    );

    // Teardown 卫生（非 verdict——只打印停止链结果）。
    let stop_ok = mgr.stop(&sid).is_ok();
    sleep(1);
    println!(
        "=== Teardown（卫生打印非裁决）: session_stop={stop_ok} rt_active={} phase={:?} ===",
        rt.is_active(),
        mgr.status(&sid).map(|s| s.phase),
    );

    if incomplete || records.len() != n_switches {
        println!(
            "=== 采集不完整（{}/{} 切换执行）——exit=2（证据已留, 非判据）===",
            records.len(),
            n_switches
        );
        std::process::exit(2);
    }
    println!("=== 采集完整（{n_switches}/{n_switches}）——exit=0（采集完整性, 非时间线裁决）===");
    std::process::exit(0);
}

#[cfg(all(test, feature = "bmd-provider", feature = "gstreamer-backend"))]
mod tests {
    use super::*;
    use crate::pipeline::PtsMonotonicity;
    use crate::program_execution::{EvidencePhase, PathEvidence};

    fn path(state: PtsMonotonicity, advanced: Option<bool>) -> PathEvidence {
        PathEvidence {
            pts: Some(1000),
            pts_state: state,
            frames: Some(10),
            advanced,
        }
    }

    fn evidence(device: uuid::Uuid, advanced: Option<bool>) -> SixPathEvidence {
        SixPathEvidence {
            sampled_at_ms: 1,
            device,
            phase: EvidencePhase::PreSwitch,
            switch_epoch: 0,
            input_video: path(PtsMonotonicity::ValidMonotonic, advanced),
            input_audio: path(PtsMonotonicity::ValidMonotonic, Some(true)),
            bridge_video: path(PtsMonotonicity::NonMonotonic, Some(true)),
            bridge_audio: path(PtsMonotonicity::DiscontinuityDeclared, Some(true)),
            program_video: path(PtsMonotonicity::ValidMonotonic, Some(true)),
            program_audio: path(PtsMonotonicity::ValidMonotonic, Some(true)),
            program_av_delta_ns: Some(500),
        }
    }

    fn record(idx: usize, rows: Vec<(&'static str, SixPathEvidence)>) -> SwitchObsRecord {
        SwitchObsRecord {
            idx,
            dir: "A→B".into(),
            av_epoch: idx as u64,
            outcome_name: "Preserved".into(),
            rows,
        }
    }

    #[test]
    fn tally_counts_states_per_path_across_records() {
        let dev = uuid::Uuid::new_v4();
        let r = record(
            1,
            vec![
                ("PRE A", evidence(dev, Some(true))),
                ("POST A", evidence(dev, None)),
            ],
        );
        let tally = tally_path_states(&[r]);
        // in_v 两条 ValidMonotonic; br_v 两条 NonMonotonic; br_a 两条 Declared。
        assert!(tally[0].contains("in_v: ValidMonotonic=2"), "{}", tally[0]);
        assert!(tally[2].contains("br_v: NonMonotonic=2"), "{}", tally[2]);
        assert!(
            tally[3].contains("br_a: DiscontinuityDeclared=2"),
            "{}",
            tally[3]
        );
        // 空记录 → no-rows 如实。
        assert!(tally_path_states(&[])[0].contains("no-rows"));
    }

    #[test]
    fn non_advancing_lists_only_negative_evidence() {
        let dev = uuid::Uuid::new_v4();
        let rows = vec![
            ("PRE A", evidence(dev, Some(true))),
            ("SPAN B", evidence(dev, Some(false))),
        ];
        let hits = non_advancing(&[record(2, rows)]);
        // Some(false) 行恰一条（in_v）且带定位; None 行不计（absence≠false）。
        assert_eq!(hits.len(), 1);
        assert!(hits[0].contains("#2"), "{}", hits[0]);
        assert!(hits[0].contains("SPAN B"), "{}", hits[0]);
        assert!(hits[0].contains("path=in_v"), "{}", hits[0]);
        assert!(non_advancing(&[]).is_empty());
    }

    #[test]
    fn av_delta_series_reads_pre_span_post() {
        let dev = uuid::Uuid::new_v4();
        let mut pre = evidence(dev, None);
        pre.program_av_delta_ns = Some(100);
        let mut span = evidence(dev, None);
        span.program_av_delta_ns = Some(200);
        let mut post = evidence(dev, None);
        post.program_av_delta_ns = None;
        let series =
            record(1, vec![("PRE A", pre), ("SPAN A", span), ("POST A", post)]).av_delta_series();
        assert_eq!(
            series,
            vec![("PRE", Some(100)), ("SPAN", Some(200)), ("POST", None)]
        );
    }
}
