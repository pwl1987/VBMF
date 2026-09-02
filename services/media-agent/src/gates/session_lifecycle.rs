//! A2-0: P0-7A SESSION-RT-01 / RESOURCE-RT-01 真机门禁（VBMF_SESSION_LIFECYCLE 入口）。
//!
//! 自 main.rs 逐字节迁出（env 命中即 exit——语义不变）。
//! 全生命周期 create→start→观察 10s→stop→close 逐步 verdict + 第二会话冲突拒绝实证;
//! 含 COMMAND-CONTRACT / IDEMPOTENCY / ERROR-MODEL / EVENT-PROJECTION /
//! EVENT-INTEGRATION(E1-E7) / EXTERNAL-API 全部 RT-01 段。
//!
//! **Gate 语义纪律（用户裁定）**: 本入口**自建**诊断 world（manifest→probes→bindings→
//! registry→resources→ctrl→SessionManager——Gate-local diagnostic construction,
//! 非 production runtime 初始化路径）; 仅复用调用方传入的运行时基础件
//! (lm/sup/agent_state/双日志/sink)。A2-0 不合并 production runtime。

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::sync::Arc;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use uuid::Uuid;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::config::Config;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::backend::MediaBackend;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::provider::DiscoveredDevice;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::device::DeviceInfo;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::events::{RuntimeEventLog, RuntimeEventSink};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::health::AgentState;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::lease::{InMemoryLeaseManager, LeaseManager as _};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::supervisor::Supervisor;

/// 参数即原 main 词法依赖（用户裁定: 显式参数即可, 不过度抽象; bootstrap 收口在 A20-03）。
/// cfg 精确复刻原位（main.rs bmd-provider 外层块 + gstreamer 内层 = all(bmd, gst)）。
#[allow(clippy::too_many_arguments)]
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
pub fn run(
    _cfg: &Config,
    devices: &[DeviceInfo],
    discovered: &[DiscoveredDevice],
    lm: &Arc<InMemoryLeaseManager>,
    sup: &Arc<std::sync::Mutex<Supervisor>>,
    agent_state: &Arc<std::sync::Mutex<AgentState>>,
    event_sink: &Arc<dyn RuntimeEventSink>,
    projection_log: &Arc<RuntimeEventLog>,
    internal_log: &Arc<RuntimeEventLog>,
) {
    if std::env::var("VBMF_SESSION_LIFECYCLE").is_ok() {
        let first_id = devices
            .first()
            .map(|d| d.device_id.to_string())
            .unwrap_or_default();
        let manifest_path = match &_cfg.device_binding_path {
            Some(p) => p.clone(),
            None => {
                eprintln!("VBMF_SESSION_LIFECYCLE 需要 DeviceBindingManifest (MEDIA_AGENT_DEVICE_BINDING)");
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
        let gst_probes = match crate::resolver::probe_gstreamer_devices(
            crate::resolver::MAX_PROBE_DEVICES,
            false,
        ) {
            crate::resolver::GstProbeOutcome::Available { probes, .. } => probes,
            _ => Vec::new(),
        };
        let bindings = crate::resolver::collect_bindings_from_manifest(
            discovered,
            &gst_probes,
            &manifest,
        );
        let registry = match crate::port::PortRegistry::build(
            discovered,
            &gst_probes,
            &manifest,
            &bindings,
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("PortRegistry 构建失败 (fail-closed): {e:?}");
                std::process::exit(2);
            }
        };
        let resources = crate::resource::SharedResourceRegistry::new(
            crate::resource::ResourceRegistry::derive_from_discovery(&registry),
        );
        let ctrl: std::sync::Arc<dyn MediaBackend> =
            crate::registry::AdapterRegistry::build_media_backend().unwrap_or_else(|e| {
                eprintln!("adapter feature 冲突 (fail-closed): {e}");
                std::process::exit(2);
            });
        let mgr = std::sync::Arc::new(crate::session::SessionManager::new(
            resources,
            lm.clone(),
            sup.clone(),
            ctrl.clone(),
            std::sync::Arc::new(devices.to_vec()),
            std::sync::Arc::new(bindings.clone()),
            Some(registry),
            crate::pipeline::MaterializeMode::Diagnostic,
            crate::session::SessionTuning::default(),
            event_sink.clone(),
        ));
        // P0.7C-2: Runtime Query (Pure Read) 门面 — 硬件证据冒烟。
        let _rq = crate::runtime_query::RuntimeQuery::new(std::sync::Arc::clone(&mgr));
        // P1a: 诊断主会话 sink kind —— `VBMF_OUTPUT_KIND` 覆盖（hls/rtmp gate 用）;
        // 默认 "rtmp" 与 P1a 前逐字节一致。无任何 VBMF_OUTPUT_* ⇒ materialize
        // fail-soft 降级纯分析（向后兼容承诺, Design Doc §6）。
        let out_cfg = crate::config::PrototypeOutputConfig::from_env();
        let diag_sink_kind = out_cfg.sink_kind_override.unwrap_or_else(|| "rtmp".into());
        let intent = crate::graph_intent::GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![crate::graph_intent::DeviceIntent {
                device_id: first_id.clone(),
                role: "CAPTURE".into(),
                pipeline: crate::graph_intent::PipelineIntent {
                    source: crate::graph_intent::SourceIntent {
                        kind: "decklink".into(),
                        device_id: first_id.clone(),
                        port_id: None,
                    },
                    sink: crate::graph_intent::SinkIntent {
                        kind: diag_sink_kind,
                    },
                },
            }],
        };
        let mut ok = true;
        // bootstrap 占位租约让位: 真实会话租约 (owner=session) 接管排他性。
        let _ = lm.release(&crate::lease::DeviceLease {
            device_id: Uuid::parse_str(&first_id).unwrap_or(Uuid::nil()),
            owner: "bootstrap".into(),
            acquired_at: chrono::Utc::now(),
            ttl: std::time::Duration::from_secs(60),
        });
        // RUNTIME-STATE-RT-01 (Hardware): create 前 CanonicalRuntimeState。
        match serde_json::to_string_pretty(&mgr.runtime_state()) {
            Ok(json) => {
                println!("=== RUNTIME-STATE-RT-01 CanonicalRuntimeState (create 前) ===");
                println!("{json}");
            }
            Err(e) => eprintln!("runtime state 序列化失败: {e}"),
        }
        println!("SESSION-RT-01 step=create ...");
        match mgr.create(intent.clone()) {
            Ok(sid) => {
                println!("SESSION-RT-01 step=create verdict=OK session={sid}");
                println!("SESSION-RT-01 step=start ...");
                match mgr.start(&sid) {
                    Ok(()) => {
                        println!("SESSION-RT-01 step=start verdict=OK (pipeline Running)");
                        // P0-7D-4.3 EVENT-INTEGRATION-RT-01 (E2/E3): gate 复用生产
                        // ingest watchdog (与 auto_start 1256 同款装配, 非 gate 专用路径) —
                        // SignalVerified 点亮 / internal drain→reduce→agent_state 写回 /
                        // 回声谓词 全部走生产代码。device_uuid 与 IdentityResolved 的
                        // device_id 同源 (first_id 解析), 保证回声归属匹配。
                        let gate_dev_uuid = Uuid::parse_str(&first_id).unwrap_or(Uuid::nil());
                        if let Some(h) = mgr.status(&sid).and_then(|s| s.pipeline) {
                            crate::watchdog::spawn_ingest_watchdog(
                                ctrl,
                                h,
                                gate_dev_uuid,
                                sup.clone(),
                                lm.clone(),
                                agent_state.clone(),
                                event_sink.clone(),
                                internal_log.clone(),
                            );
                        }
                        // P0-7D-4.3 (E5): 生产同款 5s tick 线程 — 真实驱动 lease 续期 /
                        // 预留过期 (expire_reservations_of → ResourceReservationExpired)。
                        let tick_mgr = mgr.clone();
                        std::thread::spawn(move || loop {
                            std::thread::sleep(std::time::Duration::from_secs(5));
                            tick_mgr.tick();
                        });
                        println!("SESSION-RT-01 step=observe 10s ...");
                        std::thread::sleep(std::time::Duration::from_secs(10));
                        let running = mgr
                            .status(&sid)
                            .map(|s| s.phase == crate::session::SessionPhase::Running)
                            .unwrap_or(false);
                        println!(
                            "SESSION-RT-01 step=observe verdict={} (running={running})",
                            if running { "OK" } else { "FAIL" }
                        );
                        ok &= running;
                        // P0-7D-4.3 (E2): 观察窗内 AgentState 必须由 reducer 从真实事件流
                        // (SessionCreated/SessionStateChanged{running}/SignalVerified →
                        // watchdog 每 tick drain internal → reduce) 派生为 Capturing —
                        // gate 段无其他 agent_state 写入点, 排除旁路赋值。
                        let derived_running = *agent_state.lock().unwrap();
                        println!(
                            "EVENT-INTEGRATION-RT-01 step=E2 derived_during_running={:?}",
                            derived_running
                        );
                        ok &= matches!(derived_running, crate::health::AgentState::Capturing);
                    }
                    Err(e) => {
                        println!("SESSION-RT-01 step=start verdict=FAIL error={e}");
                        ok = false;
                    }
                }
                println!("SESSION-RT-01 step=stop ...");
                match mgr.stop(&sid) {
                    Ok(()) => println!("SESSION-RT-01 step=stop verdict=OK"),
                    Err(e) => {
                        println!("SESSION-RT-01 step=stop verdict=FAIL error={e}");
                        ok = false;
                    }
                }
                let _ = mgr.close(&sid);
                // P0-7D-4.3 EVENT-INTEGRATION-RT-01 (E5): 真实 tick 驱动预留过期 —
                // 第二会话只 create 不 start (停留 Provisioning, 持真实预留/租约),
                // 超过 reservation_window (默认 30s) 由上方 5s tick 线程驱动
                // expire_reservations_of → 逐资源发射 ResourceReservationExpired,
                // 随后 Terminated 零孤儿 (RESOURCE-RT-01 crash cleanup 同路径)。
                println!(
                    "EVENT-INTEGRATION-RT-01 step=E5 create (不 start, 等 tick 过期, ~42s) ..."
                );
                let e5_intent = crate::graph_intent::GraphRuntimeIntent {
                    version: "1.0".into(),
                    devices: vec![crate::graph_intent::DeviceIntent {
                        device_id: first_id.clone(),
                        role: "CAPTURE".into(),
                        pipeline: crate::graph_intent::PipelineIntent {
                            source: crate::graph_intent::SourceIntent {
                                kind: "decklink".into(),
                                device_id: first_id.clone(),
                                port_id: None,
                            },
                            sink: crate::graph_intent::SinkIntent {
                                kind: "appsink".into(),
                            },
                        },
                    }],
                };
                match mgr.create(e5_intent) {
                    Ok(e5) => {
                        // 42s = 30s 预留窗口 + 5s tick 对齐余量 + 2s 边界裕量
                        // (首会话 create 时刻在真机上有 ±5s 漂移, 断言必须晚于
                        //  过期后首个 5s tick, 否则与 tick 线程竞态)。
                        std::thread::sleep(std::time::Duration::from_secs(42));
                        let e5_phase = mgr.status(&e5).map(|s| s.phase);
                        println!(
                            "EVENT-INTEGRATION-RT-01 step=E5 expired_phase={:?} (期望 Terminated)",
                            e5_phase
                        );
                        ok &= matches!(
                            e5_phase,
                            Some(crate::session::SessionPhase::Terminated)
                        );
                        let _ = mgr.close(&e5);
                    }
                    Err(e) => {
                        println!(
                            "EVENT-INTEGRATION-RT-01 step=E5 create verdict=FAIL error={e}"
                        );
                        ok = false;
                    }
                }
            }
            Err(e) => {
                println!("SESSION-RT-01 step=create verdict=FAIL error={e}");
                ok = false;
            }
        }
        // RUNTIME-STATE-RT-01 (Hardware): 会话生命周期后 CanonicalRuntimeState
        // (资源回落 Available / 会话终态投影可见)。
        match serde_json::to_string_pretty(&mgr.runtime_state()) {
            Ok(json) => {
                println!("=== RUNTIME-STATE-RT-01 CanonicalRuntimeState (生命周期后) ===");
                println!("{json}");
            }
            Err(e) => eprintln!("runtime state 序列化失败: {e}"),
        }
        // P0.7C-3 COMMAND-CONTRACT-RT-01 (Hardware): envelope 驱动段——
        // Start→observe→Stop→Release 经 Command Contract (与直接路径等价)。
        {
            use crate::command::{
                dispatch, CommandEnvelope, CommandId, CommandKind, CommandTarget,
            };
            let cmd_env = CommandEnvelope {
                command_id: CommandId(uuid::Uuid::new_v4()),
                kind: CommandKind::StartSession,
                target: CommandTarget::Session {
                    intent: intent.clone(),
                },
                issued_at_ms: 0,
                requested_by: "vbmf-session-lifecycle-gate".into(),
            };
            let out = dispatch(&mgr, &cmd_env);
            println!(
                "COMMAND-CONTRACT-RT-01 step=start status={:?} detail={:?}",
                out.status, out.detail
            );
            let sid = mgr.list().first().map(|s| s.session_id);
            if let Some(sid) = sid {
                println!("COMMAND-CONTRACT-RT-01 step=observe 10s ...");
                std::thread::sleep(std::time::Duration::from_secs(10));
                let running = mgr
                    .status(&sid)
                    .map(|s| s.phase == crate::session::SessionPhase::Running)
                    .unwrap_or(false);
                println!("COMMAND-CONTRACT-RT-01 step=observe running={running}");
                let stop_env = CommandEnvelope {
                    command_id: CommandId(uuid::Uuid::new_v4()),
                    kind: CommandKind::StopSession,
                    target: CommandTarget::SessionById { session_id: sid },
                    issued_at_ms: 0,
                    requested_by: "vbmf-session-lifecycle-gate".into(),
                };
                let out = dispatch(&mgr, &stop_env);
                println!(
                    "COMMAND-CONTRACT-RT-01 step=stop status={:?} detail={:?}",
                    out.status, out.detail
                );
                let rel_env = CommandEnvelope {
                    command_id: CommandId(uuid::Uuid::new_v4()),
                    kind: CommandKind::ReleaseSession,
                    target: CommandTarget::SessionById { session_id: sid },
                    issued_at_ms: 0,
                    requested_by: "vbmf-session-lifecycle-gate".into(),
                };
                let out = dispatch(&mgr, &rel_env);
                println!(
                    "COMMAND-CONTRACT-RT-01 step=release status={:?} detail={:?}",
                    out.status, out.detail
                );
            }
        }
        // P0.7C-4 IDEMPOTENCY-RT-01 (Hardware): 幂等裁决段——
        // 同 envelope 重发 Replayed / 同 id 换 intent Conflict / 会话数不增。
        {
            use crate::command::{CommandEnvelope, CommandId, CommandKind, CommandTarget};
            use crate::idempotency::{CommandIdempotency, IdempotentDispatch};
            let idem = CommandIdempotency::new(std::sync::Arc::clone(&mgr));
            let cmd_env = CommandEnvelope {
                command_id: CommandId(uuid::Uuid::new_v4()),
                kind: CommandKind::StartSession,
                target: CommandTarget::Session {
                    intent: intent.clone(),
                },
                issued_at_ms: 0,
                requested_by: "vbmf-idempotency-gate".into(),
            };
            let start_out = match idem.dispatch(&cmd_env) {
                IdempotentDispatch::Executed(o) => Some(o),
                other => {
                    println!("IDEMPOTENCY-RT-01 step=start verdict=UNEXPECTED {other:?}");
                    None
                }
            };
            if let Some(o) = &start_out {
                println!(
                    "IDEMPOTENCY-RT-01 step=start verdict=executed status={:?} classification={:?} detail={:?} sessions={}",
                    o.status,
                    o.classification,
                    o.detail,
                    mgr.list().len()
                );
            }
            // 重复投递 → Replayed (会话数不增)。
            let sessions_before = mgr.list().len();
            match idem.dispatch(&cmd_env) {
                IdempotentDispatch::Replayed(o) => {
                    let replay_same = start_out.as_ref() == Some(&o);
                    println!(
                        "IDEMPOTENCY-RT-01 step=duplicate verdict=replayed status={:?} classification={:?} sessions={sessions_before} outcome_equal={replay_same}",
                        o.status, o.classification
                    );
                }
                other => {
                    println!(
                        "IDEMPOTENCY-RT-01 step=duplicate verdict=UNEXPECTED {other:?}"
                    );
                }
            }
            // 同 command_id 换 intent → Conflict (零执行)。
            // (改 version 而非 sink.kind: 真机 intent 的 sink 恒为 rtmp,
            //  改成 rtmp 等于没改 → 指纹不变 → 误判 Replayed; version 是
            //  canonical 字段且真机恒为 "1.0", 必产生指纹差。)
            let mut conflict_env = cmd_env.clone();
            if let CommandTarget::Session { intent } = &mut conflict_env.target {
                intent.version = "idempotency-conflict-probe".into();
            }
            let sessions_before = mgr.list().len();
            match idem.dispatch(&conflict_env) {
                IdempotentDispatch::Conflict { .. } => {
                    println!(
                        "IDEMPOTENCY-RT-01 step=conflict verdict=conflict sessions={sessions_before}"
                    );
                }
                other => {
                    println!(
                        "IDEMPOTENCY-RT-01 step=conflict verdict=UNEXPECTED {other:?}"
                    );
                }
            }
            // observe: 幂等段创建的会话仍在运行。
            if let Some(sid) = mgr.list().first().map(|s| s.session_id) {
                println!("IDEMPOTENCY-RT-01 step=observe 10s ...");
                std::thread::sleep(std::time::Duration::from_secs(10));
                let running = mgr
                    .status(&sid)
                    .map(|s| s.phase == crate::session::SessionPhase::Running)
                    .unwrap_or(false);
                println!("IDEMPOTENCY-RT-01 step=observe running={running}");
                let stop_env = CommandEnvelope {
                    command_id: CommandId(uuid::Uuid::new_v4()),
                    kind: CommandKind::StopSession,
                    target: CommandTarget::SessionById { session_id: sid },
                    issued_at_ms: 0,
                    requested_by: "vbmf-idempotency-gate".into(),
                };
                match idem.dispatch(&stop_env) {
                    IdempotentDispatch::Executed(o) => println!(
                        "IDEMPOTENCY-RT-01 step=stop verdict=executed status={:?} classification={:?}",
                        o.status, o.classification
                    ),
                    other => {
                        println!(
                            "IDEMPOTENCY-RT-01 step=stop verdict=UNEXPECTED {other:?}"
                        );
                    }
                }
                let rel_env = CommandEnvelope {
                    command_id: CommandId(uuid::Uuid::new_v4()),
                    kind: CommandKind::ReleaseSession,
                    target: CommandTarget::SessionById { session_id: sid },
                    issued_at_ms: 0,
                    requested_by: "vbmf-idempotency-gate".into(),
                };
                match idem.dispatch(&rel_env) {
                    IdempotentDispatch::Executed(o) => println!(
                        "IDEMPOTENCY-RT-01 step=release verdict=executed status={:?} classification={:?}",
                        o.status, o.classification
                    ),
                    other => {
                        println!(
                            "IDEMPOTENCY-RT-01 step=release verdict=UNEXPECTED {other:?}"
                        );
                    }
                }
                // P0.7C-5 ERROR-MODEL-RT-01 (Hardware): ghost 探针——
                // 对已 release 的会话再发 Stop (新 command_id) → Failed +
                // classification=PermanentFailure (UnknownSession 臂)。
                let ghost_env = CommandEnvelope {
                    command_id: CommandId(uuid::Uuid::new_v4()),
                    kind: CommandKind::StopSession,
                    target: CommandTarget::SessionById { session_id: sid },
                    issued_at_ms: 0,
                    requested_by: "vbmf-error-model-gate".into(),
                };
                match idem.dispatch(&ghost_env) {
                    IdempotentDispatch::Executed(o) => println!(
                        "ERROR-MODEL-RT-01 step=ghost-stop status={:?} classification={:?} detail={:?}",
                        o.status, o.classification, o.detail
                    ),
                    other => {
                        println!(
                            "ERROR-MODEL-RT-01 step=ghost-stop verdict=UNEXPECTED {other:?}"
                        );
                    }
                }
            }
        }
        // RESOURCE-RT-01: 第二会话争同资源必须被拒 (首会话已释放 → 先占住再争)。
        println!("RESOURCE-RT-01 step=conflict ...");
        let sid_a = mgr.create(crate::graph_intent::GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![crate::graph_intent::DeviceIntent {
                device_id: first_id.clone(),
                role: "CAPTURE".into(),
                pipeline: crate::graph_intent::PipelineIntent {
                    source: crate::graph_intent::SourceIntent {
                        kind: "decklink".into(),
                        device_id: first_id.clone(),
                        port_id: None,
                    },
                    sink: crate::graph_intent::SinkIntent {
                        kind: "appsink".into(),
                    },
                },
            }],
        });
        match sid_a {
            Ok(a) => {
                let conflict = mgr.create(crate::graph_intent::GraphRuntimeIntent {
                    version: "1.0".into(),
                    devices: vec![crate::graph_intent::DeviceIntent {
                        device_id: first_id.clone(),
                        role: "CAPTURE".into(),
                        pipeline: crate::graph_intent::PipelineIntent {
                            source: crate::graph_intent::SourceIntent {
                                kind: "decklink".into(),
                                device_id: first_id.clone(),
                                port_id: None,
                            },
                            sink: crate::graph_intent::SinkIntent {
                                kind: "appsink".into(),
                            },
                        },
                    }],
                });
                println!(
                    "RESOURCE-RT-01 step=conflict verdict={}",
                    if conflict.is_err() {
                        "OK (第二会话被拒)"
                    } else {
                        "FAIL (资源被超卖!)"
                    }
                );
                ok &= conflict.is_err();
                let _ = mgr.close(&a);
            }
            Err(e) => {
                println!("RESOURCE-RT-01 step=conflict verdict=FAIL error={e}");
                ok = false;
            }
        }
        // P0.7C-6 EVENT-PROJECTION-RT-01 (Hardware): 消费接线实证——
        // 全生命周期事件 drain → project → 只读快照 (Observation, 不写回)。
        let drained_events = projection_log.drain();
        let p = crate::event_projection::project(&drained_events);
        {
            println!(
                "EVENT-PROJECTION-RT-01 total={} kinds={:?}",
                p.total, p.kind_counts
            );
            println!(
                "EVENT-PROJECTION-RT-01 session_states={:?} session_failures={:?} has_critical={}",
                p.session_states, p.session_failures, p.has_critical
            );
            println!(
                "EVENT-PROJECTION-RT-01 dropped_obs={} dropped_crit={}",
                projection_log.dropped_observations(),
                projection_log.dropped_criticals()
            );
        }
        // P0-7D-4.3 EVENT-INTEGRATION-RT-01 (Hardware): 事件内消费闭环真机实证。
        // 计数证据取上方投影全量 drain (internal log 已被生产 watchdog 逐 tick 消费并
        // 折叠进 reducer — 其可观测输出 = E2 Capturing 断言 [观察窗内] + 终态断言 [此处];
        // 两路共享同一事件流, 投影端计数完整 = 内消费未破坏外送平面):
        //   E1 IdentityResolved/SessionCreated 真实生命周期产生;
        //   E3 SignalVerified 由生产 watchdog 点亮 (a4 双路首帧+PTS 单调翻真, 闩锁恰好一次);
        //   E5 ResourceReservationExpired 经上方真实 tick 过期路径发射 (首设备=1 资源, 精确计数);
        //   E6 Supervisor 回声不自激 — 无故障窗口 PipelineFault 计数为零 (若自激必以
        //      tick 频率故障事件显现; 回声谓词排除语义 4.2 已证);
        //   E7 双日志互不破坏 — 投影全量计数完整 + internal 残留 drain 干净;
        //   E2 终态: E5 真实降级后 reducer 派生 Degraded (无更高级 pending);
        //   E4 LoopbackVerified 由 VBMF_LOOPBACK 入口独立实跑闭环 (方案 A, 不在此重复验收)。
        {
            let e1 = p.kind_counts.get("identity_resolved").copied().unwrap_or(0) >= 1
                && p.kind_counts.get("session_created").copied().unwrap_or(0) >= 1;
            let e3 = p.kind_counts.get("signal_verified").copied().unwrap_or(0) == 1;
            let e5 = p
                .kind_counts
                .get("resource_reservation_expired")
                .copied()
                .unwrap_or(0)
                == 1;
            let e6 = p.kind_counts.get("pipeline_fault").copied().unwrap_or(0) == 0;
            let final_state = *agent_state.lock().unwrap();
            let e7_residue = internal_log.drain();
            let e2_final = matches!(final_state, crate::health::AgentState::Degraded);
            println!(
                "EVENT-INTEGRATION-RT-01 E1={e1} E3_signal_verified={e3} E5_expired_count={e5} E6_pipeline_fault_absent={e6} E7_internal_residue={:?} E2_final_state={final_state:?}",
                e7_residue.len()
            );
            ok &= e1 && e3 && e5 && e6 && e2_final;
        }
        // P0.7C-7 EXTERNAL-API-RT-01 (Hardware): API Boundary Model 实证——
        // 真机 Runtime State → API 模型纯转换 + 序列化往返 (非 Web Server;
        // 零 transport / 零持久化; 禁清单 11 项)。
        {
            use crate::api_boundary::{
                default_idempotency_boundary, to_api_query_snapshot, ApiProjectionResponse,
                ApiQuerySnapshot,
            };
            let state = _rq.get_runtime_state();
            let snap = to_api_query_snapshot(&state);
            let snap_json =
                serde_json::to_string(&snap).expect("ApiQuerySnapshot 必须可序列化");
            let _roundtrip: ApiQuerySnapshot =
                serde_json::from_str(&snap_json).expect("ApiQuerySnapshot 必须可反序列化");
            let api_proj: ApiProjectionResponse = (&p).into();
            let proj_json = serde_json::to_string(&api_proj)
                .expect("ApiProjectionResponse 必须可序列化");
            let boundary = default_idempotency_boundary();
            let boundary_json = serde_json::to_string(&boundary)
                .expect("ApiIdempotencyBoundary 必须可序列化");
            let api_ok = snap.devices.len() == state.devices.len()
                && snap.sessions.len() == state.sessions.len()
                && snap_json.contains("\"devices\"")
                && proj_json.contains("\"event_projection_snapshot\"")
                && boundary_json.contains("\"process_local\"")
                && boundary_json.contains("\"durable_log_deferred\"")
                && boundary_json.contains("\"restart_breaks_replay\"");
            println!(
                "EXTERNAL-API-RT-01 verdict={} devices={} sessions={} resources={} boundary=process_local/durable_log_deferred/restart_breaks_replay",
                if api_ok { "OK" } else { "FAIL" },
                snap.devices.len(),
                snap.sessions.len(),
                snap.resources.len()
            );
            println!(
                "EXTERNAL-API-RT-01 projection_total={} snapshot_kind=event_projection_snapshot",
                p.total
            );
            ok &= api_ok;
        }
        println!(
            "=== SESSION-RT-01/RESOURCE-RT-01/EXTERNAL-API-RT-01 ALL {} ===",
            if ok { "PASS" } else { "FAIL" }
        );
        std::process::exit(if ok { 0 } else { 2 });
    }
}
