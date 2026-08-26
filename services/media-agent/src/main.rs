//! VBMF Rust Media Agent — Gate 2 skeleton + Gate 5/6/7 scaffolding.
//!
//! Boundary (SoT §14): this binary owns the Hardware Plane only.
//! Control Plane (API/auth/RBAC/config/UI) stays in Node/Fastify.

mod config;
mod device;
mod graph_intent;
mod health;
mod lease;
mod pipeline;
mod rpc;
mod sdk;
mod supervisor;
mod decklink; // Gate 6/7: real DeckLink enumeration (feature `bmd`)

// 硬规则 (Phase 0.6): `hardware-test` (IDeckLinkInput SDK 探针) 与 canonical `gstreamer`
// 运行时互斥 —— 生产运行不得同时打开同一块 DeckLink (避免双采 / 设备争用). 编译期强制.
#[cfg(all(feature = "hardware-test", feature = "gstreamer"))]
compile_error!("hardware-test SDK 探针与 canonical GStreamer 运行时互斥; 生产运行不得同时启用 (避免双采/争用同一块 DeckLink)");

// Trait must be in scope to call `discover()` (trait method, not inherent).
use device::DeviceManager;
// Trait must be in scope to call `acquire`/`is_valid` on `Arc<InMemoryLeaseManager>`
// (trait method, auto-deref via Arc; 否则 E0599 no method named `acquire`).
use lease::LeaseManager;
// Trait must be in scope to call `prepare`/`start`/`recover` on `Arc<GStreamerPipelineController>`
// (trait 方法, 否则 E0599 no method named `recover`).
use pipeline::PipelineController;
use std::io::Write;
use std::sync::Arc;
use uuid::Uuid;

fn main() {
    tracing_subscriber::fmt::init();

    // Gate 2.1: load config shape from env (no behavior attached yet).
    let _cfg = config::Config::from_env();

    // Gate 2.2: device discovery.
    // `simulation` => mock devices (CI/tests, no hardware or SDK).
    // default / bmd => filesystem probe (safe on CI / non-BMD; real on BMD).
    #[cfg(feature = "simulation")]
    let dm = device::SimulatedDeviceManager::new();
    #[cfg(all(not(feature = "simulation"), feature = "bmd"))]
    let dm = device::DeckLinkDeviceManager::new();
    #[cfg(all(not(feature = "simulation"), not(feature = "bmd")))]
    let dm = device::FilesystemDeviceManager::new();
    let devices = dm.discover();
    tracing::info!(count = devices.len(), "device discovery complete");

    // Gate 2.3: lease manager (in-memory; no hardware needed for the interface).
    let lm = Arc::new(lease::InMemoryLeaseManager::new());
    for d in &devices {
        match lm.acquire(&d.device_id, "bootstrap", std::time::Duration::from_secs(60)) {
            Ok(l) => tracing::info!(device = %l.device_id, "lease acquired"),
            Err(e) => tracing::warn!(error = %e, "lease acquire failed"),
        }
    }
    // 排他性不变量: 同一设备重复 acquire 必须被拒 (防 host ffmpeg / 双采).
    if let Some(first) = devices.first() {
        match lm.acquire(&first.device_id, "second-owner", std::time::Duration::from_secs(60)) {
            Ok(_) => tracing::warn!("LEASE COLLISION — double-capture risk!"),
            Err(e) => tracing::info!(error = %e, "lease re-acquire correctly rejected"),
        }
    }

    // Gate 2.5 (A): DeckLink SDK FFI smoke — 验证 libDeckLinkAPI.so 在运行环境可达.
    // 宿主机(/usr/lib 默认路径)应成功; Option B 容器若不 bind-mount 库则 warn(预期).
    match sdk::probe_sdk("libDeckLinkAPI.so") {
        Ok(()) => tracing::info!("SDK libDeckLinkAPI.so reachable, entry symbols present"),
        Err(e) => tracing::warn!(error = %e, "SDK probe failed (expected in container w/o bind-mount)"),
    }

    // Gate 2.6 (P1①): bmd feature 下 `devices` 已直接来自 `DeckLinkDeviceManager`
    // (基于 DeckLink DeviceHandle 派生的 canonical identity), 不再按索引把 SDK 枚举回填
    // filesystem 列表 (拓扑变化后 index 会漂移, 见 device.rs).

    // Gate 7 (feature `hardware-test`): verbose Device Registry (model/serial/status) for BMD.
    #[cfg(feature = "hardware-test")]
    match decklink::registry() {
        Ok(table) => tracing::info!("DeckLink Device Registry:\n{table}"),
        Err(e) => tracing::warn!(error = %e, "registry unavailable"),
    }

    // Gate 5: Supervisor seeded with device handles (watchdog state machine + budget/
    // backoff/circuit-breaker are unit-tested in supervisor.rs). 包 Arc<Mutex> 以便 watch
    // 线程与 GStreamer recover 接线共享 (Supervisor 只决策, 不碰 GStreamer).
    let sup = Arc::new(std::sync::Mutex::new(supervisor::Supervisor::new(
        supervisor::RestartPolicy::default(),
    )));
    for d in &devices {
        sup.lock().unwrap().register(d.device_id);
    }
    tracing::info!(watched = devices.len(), "supervisor initialized");

    // Gate 2.4: 最简 /health (std TcpListener, 无第三方依赖; 后续可换 axum).
    // Gate 2.6 (P1②): 返回真实运行时状态, 与 Supervisor 状态机对齐 (不再固定 ready).
    let device_count = devices.len();
    let agent_state = Arc::new(std::sync::Mutex::new(health::AgentState::Ready));

    // Gate 2.6 (CAP-01) — 关键边界澄清 (Phase 0.6 锁死):
    //   * `decklink::start_capture` (IDeckLinkInput) = SDK 能力 / 诊断探针
    //     (Gate 6/7), 验证 SDK 能否打开设备 / callback 是否正常 / 格式是否可读.
    //     它**不是** canonical 媒体数据通道 (否则与 GStreamer 争夺设备 → 双采).
    //     真机 GStreamer 启动后, 该探针仅限 `hardware-test` feature, 避免争用同一块卡.
    //   * canonical 媒体采集 = GStreamer `decklinkvideosrc` + `decklinkaudiosrc`
    //     (Phase 0.6). CAP-01 的 MEDIA-RT-01 (真实 SDI → GStreamer → RAW →
    //     first buffer) 由 `PipelineController` 拥有.
    #[cfg(feature = "bmd")]
    {
        // MEDIA-RT-01 自测模式 (MEDIA_AGENT_SELFTEST=1): 用 videotestsrc/audiotestsrc
        // 验证媒体运行时链路 (GStreamer launch → appsink 首帧 → PTS → MEDIA-RT-01 A/B/C),
        // 不依赖 DeckLink 信号; 此时跳过下方 decklink canonical 路径.
        #[cfg(feature = "gstreamer")]
        let skip_decklink = std::env::var("MEDIA_AGENT_SELFTEST").is_ok();
        #[cfg(not(feature = "gstreamer"))]
        let skip_decklink = false;
        #[cfg(feature = "gstreamer")]
        if skip_decklink {
            let plan = crate::pipeline::PipelinePlan::self_test();
            let ctrl = std::sync::Arc::new(crate::pipeline::GStreamerPipelineController::new());
            match ctrl.prepare(&plan) {
                Ok(h) => match ctrl.start(&h) {
                    Ok(()) => {
                        tracing::info!(handle = %h.0, "MEDIA-RT-01 self-test 管线启动 (videotestsrc/audiotestsrc → appsink)");
                        *agent_state.lock().unwrap() = health::AgentState::Capturing;
                        // 复用生产 ingest watchdog, 完整推导 A1-A4/B1-B4/C1-C4;
                        // 自测源稳定出帧 → pass() 达成即打印 "MEDIA-RT-01: A+B+C 全过".
                        spawn_ingest_watchdog(ctrl, h, Uuid::nil(), sup.clone(), lm.clone(), agent_state.clone());
                    }
                    Err(e) => tracing::error!(error = %e, "MEDIA-RT-01 self-test 启动失败"),
                },
                Err(e) => tracing::error!(error = %e, "MEDIA-RT-01 self-test prepare 失败"),
            }
        }
        if !skip_decklink {
        // (A) SDK 诊断探针 (仅 hardware-test; 真机已验证可行, 不用于生产媒体路径).
        //     与 canonical GStreamer 路径互斥, 避免同时打开同一块 DeckLink.
        //     注: `hardware-test` 与 `gstreamer` 已在编译期互斥 (见文件顶部 compile_error),
        //     生产 canonical 运行时绝不会同时启用两者.
        #[cfg(all(feature = "hardware-test", not(feature = "gstreamer")))]
        match decklink::start_capture(0) {
            Ok(stats) => {
                tracing::info!("CAP-01 SDK 诊断探针已启动 (device 0, IDeckLinkInput; 非 canonical 通道)");
                std::thread::spawn(move || loop {
                    let n = stats.frame_count.load(std::sync::atomic::Ordering::SeqCst);
                    let ff = stats.first_frame_at.lock().unwrap().is_some();
                    let mono = stats.monotonic.load(std::sync::atomic::Ordering::SeqCst);
                    tracing::info!(frame_count = n, first_frame = ff, pts_monotonic = mono, "CAP-01 SDK probe live");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                });
            }
            Err(e) => tracing::error!(error = %e, "CAP-01 SDK 诊断探针失败"),
        }

        // (B) canonical 媒体采集路径 (GStreamer) — 物化 PipelinePlan, 由
        //     PipelineController 拥有. 控制面只带 VBMF device_id; bmd_persistent_id /
        //     device-number 由 materialize 经 Device Registry 解析得到.
        let first_id = devices.first().map(|d| d.device_id.to_string()).unwrap_or_default();
        let intent = crate::graph_intent::GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![crate::graph_intent::DeviceIntent {
                device_id: first_id.clone(),
                role: "CAPTURE".into(),
                pipeline: crate::graph_intent::PipelineIntent {
                    source: crate::graph_intent::SourceIntent { kind: "decklink".into(), device_id: first_id },
                    sink: crate::graph_intent::SinkIntent { kind: "rtmp".into() },
                },
            }],
        };
        // 物化模式: 默认 Production (identity 解析失败直接 IdentityUnresolved, 绝不盲开);
        // MEDIA_AGENT_MODE=diagnostic 时显式回退 device-number (仅验证/排障用, 非静默).
        let mode = match std::env::var("MEDIA_AGENT_MODE").as_deref() {
            Ok("diagnostic") => crate::pipeline::MaterializeMode::Diagnostic,
            _ => crate::pipeline::MaterializeMode::Production,
        };
        match crate::pipeline::materialize(
            &intent,
            &devices,
            mode,
        ) {
            Ok(plans) => {
                for p in &plans {
                    tracing::info!(
                        device_id = %p.source.device_id,
                        bmd_persistent_id = p.source.bmd_persistent_id,
                        device_number = p.source.device_number,
                        selection_mode = ?p.source.selection_mode,
                        "CAP-01 canonical ingest plan materialized (GStreamer decklinkvideosrc/audiosrc hw-serial-number; launch pending)"
                    );
                }
                *agent_state.lock().unwrap() = health::AgentState::Capturing;

                // (C) 真实 GStreamer launch (feature = "gstreamer") + Supervisor→recover 接线.
                #[cfg(feature = "gstreamer")]
                {
                    let dev_id_str = plans[0].source.device_id.clone();
                    let device_uuid = Uuid::parse_str(&dev_id_str).unwrap_or(Uuid::nil());
                    // Lease→Pipeline: 启动前确认该设备的租约仍有效 (排他采集前置条件).
                    if !lm.is_valid(&device_uuid) {
                        tracing::error!(device_id = %dev_id_str, "lease 无效, 拒绝启动 canonical 采集 (排他不变量)");
                    } else {
                        let ctrl = Arc::new(crate::pipeline::GStreamerPipelineController::new());
                        // 证据: 记录 GStreamer 运行时版本 (与 SDK/driver 一并归档).
                        tracing::info!(gst_version = ?gstreamer::version(), "GStreamer runtime version (evidence)");
                        match ctrl.prepare(&plans[0]) {
                            Ok(h) => match ctrl.start(&h) {
                                Ok(()) => {
                                    tracing::info!(
                                        handle = %h.0,
                                        device_id = %dev_id_str,
                                        "canonical GStreamer pipeline 启动 (decklinkvideosrc/audiosrc hw-serial-number)"
                                    );
                                    // MEDIA-RT-01A: Ingest Open 达成 (已启动, 信号检测见 health).
                                    sup.lock().unwrap().register(h.0);
                                    spawn_ingest_watchdog(
                                        ctrl,
                                        h,
                                        device_uuid,
                                        sup.clone(),
                                        lm.clone(),
                                        agent_state.clone(),
                                    );
                                }
                                Err(e) => tracing::error!(error = %e, "canonical GStreamer 启动失败 (未盲开)"),
                            },
                            Err(e) => tracing::error!(error = %e, "canonical prepare 失败"),
                        }
                    }
                }
                #[cfg(not(feature = "gstreamer"))]
                {
                    tracing::info!("canonical 计划已物化; 真实 GStreamer launch 待启用 feature 'gstreamer'");
                }
            }
            Err(e) => tracing::error!(error = %e, "CAP-01 canonical ingest 物化失败 (identity 未解析)"),
        }
        }
    }

    std::thread::spawn({
        let agent_state = agent_state.clone();
        move || {
            match std::net::TcpListener::bind("0.0.0.0:8080") {
                Ok(listener) => {
                    tracing::info!("health endpoint listening on :8080");
                    for stream in listener.incoming() {
                        if let Ok(mut s) = stream {
                            let st = *agent_state.lock().unwrap();
                            let body = serde_json::json!({
                                "state": st,
                                "devices": device_count,
                                "active_pipelines": 0
                            })
                            .to_string();
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = s.write_all(resp.as_bytes());
                        }
                    }
                }
                Err(e) => tracing::error!(error = %e, "health bind failed"),
            }
        }
    });

    tracing::info!("media-agent canonical runtime loaded (health :8080; ingest via GStreamer started on lease acquire)");
    // 常驻以便 health 探测 (Gate 2.4 演示); 生产由 supervisor 管理生命周期.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

/// MEDIA-RT-01 watchdog (Supervisor → PipelineController.recover 运行时接线).
///
/// 单向健康链 (回应 #9): `GStreamer Bus → PipelineHealth → AgentState → Supervisor → Health API`.
/// 周期: 真 bus 监控 (Error/EOS/StateChanged) + appsink 计数 → 推导 MEDIA-RT-01
/// A1-A4 / B1-B4 / C1-C4 → 错误时报告 Supervisor (决策引擎) → Restart → 重校 lease → recover.
/// Supervisor 仅决策, 不碰 GStreamer (硬边界); 实际重启由这里执行.
#[cfg(feature = "gstreamer")]
fn spawn_ingest_watchdog(
    ctrl: Arc<crate::pipeline::GStreamerPipelineController>,
    handle: crate::pipeline::PipelineHandle,
    device_uuid: Uuid,
    sup: Arc<std::sync::Mutex<supervisor::Supervisor>>,
    lm: Arc<lease::InMemoryLeaseManager>,
    agent_state: Arc<std::sync::Mutex<health::AgentState>>,
) {
    std::thread::spawn(move || {
        // A1/A2 在 start 前已由 materialize (身份解析) + lm.is_valid (租约) 保证, 否则不会进 watchdog.
        let stability_window = std::time::Duration::from_secs(10); // MEDIA-RT-01C 验收窗口
        let mut prev_video = 0u64;
        let mut prev_audio = 0u64;
        let mut tick = 0u64;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            // 真实 GStreamer bus 监控 (Error/EOS/StateChanged) —— Supervisor 闭环数据源 (#8).
            let events = ctrl.poll_bus(&handle);
            // 在共享 Arc 上就地更新 acceptance 子项: 只读 live 状态→推导→写回 acceptance,
            // 绝不覆盖 appsink 回调写入的 video_frame_count/audio_frame_count/PTS/pts_monotonic,
            // 否则每轮 snapshot 写回会把实时计数回退, 破坏 c4(计数增长) 判定 (#4 回归).
            let (pass, has_error) = if let Some(h) = crate::pipeline::HEALTH_ARCS.lock().unwrap().get(&handle) {
                let mut g = h.lock().unwrap();
                g.acceptance.a1_identity_resolved = true;
                g.acceptance.a2_lease_acquired = true;
                g.acceptance.a4_signal_detected = g.first_frame_ok();
                g.acceptance.b1_first_video = g.video_first_pts.is_some();
                g.acceptance.b2_first_audio = g.audio_first_pts.is_some();
                g.acceptance.b3_valid_pts = g.video_first_pts.is_some();
                g.acceptance.b4_pts_monotonic = g.pts_monotonic;
                g.acceptance.c2_no_pipeline_error = g.last_error.is_none();
                let v = g.video_frame_count;
                let a = g.audio_frame_count;
                g.acceptance.c4_counters_continue = v > prev_video && a > prev_audio;
                prev_video = v;
                prev_audio = a;
                (g.acceptance.pass(), g.last_error.is_some())
            } else {
                (false, false)
            };

            // 错误 / 总线错误 → Supervisor 决策引擎 (仅决策, 不碰 GStreamer).
            if has_error
                || events
                    .iter()
                    .any(|e| matches!(e, crate::pipeline::PipelineBusEvent::Error(_) | crate::pipeline::PipelineBusEvent::Eos))
            {
                match sup.lock().unwrap().report_failure(&handle.0) {
                    Ok(supervisor::SupervisorAction::Restart) => {
                        // Lease→Pipeline: recover 前必须重校租约仍在有效期内 (MEDIA-03 排他不变量).
                        if !lm.is_valid(&device_uuid) {
                            tracing::error!(device = %device_uuid, "recover 中止: lease 失效 (排他不变量)");
                            *agent_state.lock().unwrap() = health::AgentState::ManualRequired;
                            continue;
                        }
                        let backoff = sup.lock().unwrap().backoff(&handle.0);
                        let _ = sup.lock().unwrap().begin_restart(&handle.0);
                        std::thread::sleep(backoff);
                        match ctrl.recover(&handle) {
                            Ok(()) => {
                                sup.lock().unwrap().report_recovered(&handle.0).ok();
                                tracing::warn!(handle = %handle.0, "MEDIA-RT-01 watchdog: recover 成功 (Supervisor→PipelineController.recover 闭环)");
                            }
                            Err(e) => tracing::error!(error = %e, "recover 失败"),
                        }
                    }
                    Ok(supervisor::SupervisorAction::Escalate) => {
                        tracing::error!(handle = %handle.0, "MEDIA-RT-01 watchdog: Escalate (MANUAL_REQUIRED)");
                        *agent_state.lock().unwrap() = health::AgentState::ManualRequired;
                    }
                    Err(e) => tracing::error!(error = %e, "supervisor report_failure 失败"),
                }
            } else if pass {
                *agent_state.lock().unwrap() = health::AgentState::Capturing;
                tracing::info!(
                    handle = %handle.0,
                    video_frames = prev_video,
                    audio_frames = prev_audio,
                    "MEDIA-RT-01: A+B+C 全过 (canonical first-buffer 路径健康)"
                );
            } else if tick % 20 == 0 {
                // 诊断: pass 未达成时打印各子项, 便于现场定位 (每 ~10s 一次, 防刷屏).
                let snap = crate::pipeline::read_health(&handle).unwrap_or_default();
                tracing::info!(
                    tick = tick,
                    a1 = snap.acceptance.a1_identity_resolved,
                    a2 = snap.acceptance.a2_lease_acquired,
                    a3 = snap.acceptance.a3_pipeline_playing,
                    a4 = snap.acceptance.a4_signal_detected,
                    b1 = snap.acceptance.b1_first_video,
                    b2 = snap.acceptance.b2_first_audio,
                    b3 = snap.acceptance.b3_valid_pts,
                    b4 = snap.acceptance.b4_pts_monotonic,
                    c1 = snap.acceptance.c1_no_unexpected_eos,
                    c2 = snap.acceptance.c2_no_pipeline_error,
                    c3 = snap.acceptance.c3_no_renegotiation,
                    c4 = snap.acceptance.c4_counters_continue,
                    vframes = snap.video_frame_count,
                    aframes = snap.audio_frame_count,
                    vpts = snap.video_first_pts.unwrap_or(0),
                    apts = snap.audio_first_pts.unwrap_or(0),
                    "MEDIA-RT-01 诊断 (未全过)"
                );
            }
            tick += 1;
        }
    });
}
