//! A2-0 (裁定刀 3): Ingest Watchdog — 独立 Runtime Health/Recovery 模块。
//!
//! 自 main.rs 逐字节迁出（签名/行为零变, 2026-09-02 a2-0-runtime-repositioning）。
//! 职责链: observe → acceptance fold → signal 检出闩锁 → 事件 emission →
//! Supervisor 决策（只决策不碰 GStreamer——边界保留）→ backoff → recover。
//! 归属: Runtime 层（main.rs 组合根与 gates bin 同源引用）。
//!
//! MEDIA-RT-01 watchdog (Supervisor → PipelineController.recover 运行时接线):
//! 单向健康链 (回应 #9): GStreamer Bus → PipelineHealth → AgentState → Supervisor → Health API;
//! 周期真 bus 监控 (Error/EOS/StateChanged) + appsink 计数 → MEDIA-RT-01 A1-A4/B1-B4/C1-C4 →
//! 错误报告 Supervisor (决策引擎) → Restart → 重校 lease → recover;
//! Supervisor 仅决策不碰 GStreamer (硬边界); `ctrl` 为 `Arc<dyn MediaBackend>` (C2c)。

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::sync::Arc;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use uuid::Uuid;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::backend::MediaBackend;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::{events, health, lease, supervisor};

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
#[allow(clippy::too_many_arguments)] // P0-7D: +sink/+internal_log 事件内消费接线 (watchdog 装配参数, 非领域 API)
pub fn spawn_ingest_watchdog(
    ctrl: Arc<dyn MediaBackend>,
    handle: crate::pipeline::PipelineHandle,
    device_uuid: Uuid,
    sup: Arc<std::sync::Mutex<supervisor::Supervisor>>,
    lm: Arc<lease::InMemoryLeaseManager>,
    agent_state: Arc<std::sync::Mutex<health::AgentState>>,
    sink: Arc<dyn events::RuntimeEventSink>,
    internal_log: Arc<events::RuntimeEventLog>,
) {
    std::thread::spawn(move || {
        // A1/A2 在 start 前已由 materialize (身份解析) + lm.is_valid (租约) 保证, 否则不会进 watchdog.
        let _stability_window = std::time::Duration::from_secs(10); // MEDIA-RT-01C 验收窗口
        let mut prev_video = 0u64;
        let mut prev_audio = 0u64;
        let mut tick = 0u64;
        // P0-7D-1.3: reducer 折叠上下文 — bootstrap = 当前实际态 (构造期/乐观写入是输入初值);
        // 环内命令式 agent_state 散写全部收敛为 drain internal → reduce → 写回。
        let mut health_fold = crate::health::HealthFold::bootstrap(*agent_state.lock().unwrap());
        // P0-7D-2.1: SignalVerified 点亮闩锁 (a4 信号检出翻真只发一次)。
        let mut signal_latched = false;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            // 真实 GStreamer bus 监控 (Error/EOS/StateChanged) —— Supervisor 闭环数据源 (#8).
            let events = ctrl.observe(&handle);
            let mut bus_events: u64 = 0;
            // 在共享 Arc 上就地更新 acceptance 子项: 只读 live 状态→推导→写回 acceptance,
            // 绝不覆盖 appsink 回调写入的 video_frame_count/audio_frame_count/PTS/video_pts_state/audio_pts_state,
            // 否则每轮 snapshot 写回会把实时计数回退, 破坏 c4(计数增长) 判定 (#4 回归).
            let (pass, has_error, a4_signal) = if let Some(h) = crate::pipeline_events::HEALTH_ARCS
                .lock()
                .unwrap()
                .get(&handle)
            {
                let mut g = h.lock().unwrap();
                g.acceptance.a1_identity_resolved = true;
                g.acceptance.a2_lease_acquired = true;
                g.acceptance.a4_signal_detected = g.first_frame_ok();
                g.acceptance.b1_first_video = g.video_first_pts.is_some();
                g.acceptance.b2_first_audio = g.audio_first_pts.is_some();
                g.acceptance.b3_valid_pts = g.video_first_pts.is_some();
                g.acceptance.a3_pipeline_playing = g.playing;
                // b4 由两路 PTS 三态推导 (P1-3): 仅当 video 与 audio 均 ValidMonotonic 才视为 PTS 单调通过.
                // 绝不回退到单一 bool; Unknown/NonMonotonic 任一即不通过.
                g.acceptance.b4_pts_monotonic = g.video_pts_state
                    == crate::pipeline::PtsMonotonicity::ValidMonotonic
                    && g.audio_pts_state == crate::pipeline::PtsMonotonicity::ValidMonotonic;
                g.acceptance.c1_no_unexpected_eos = g.acceptance.c_unexpected_eos == 0;
                g.acceptance.c2_no_pipeline_error = g.last_error.is_none();
                g.acceptance.c3_no_repeated_reneg = g.acceptance.c_renegotiations == 0;
                let v = g.video_frame_count;
                let a = g.audio_frame_count;
                g.acceptance.c4_counters_continue = v > prev_video && a > prev_audio;
                prev_video = v;
                prev_audio = a;
                // C 稳定性窗口计时 + 测量字段 (用户复核 §十二).
                if let Some(started) = g.started_at {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    g.acceptance.c_observed_ms = Some(((now - started).max(0) as u64) * 1000);
                }
                g.acceptance.c_video_frames = g.video_frame_count;
                g.acceptance.c_audio_frames = g.audio_frame_count;
                let before = g.bus_event_count;
                g.bus_event_count += events.len() as u64;
                // P1-4 接线证据: 首次接获任意真实 Bus 事件时打一条 info (仅一次),
                // 证明 Bus watch → channel → poll_bus 链路端到端生效 (非 stub).
                if before == 0 && !events.is_empty() {
                    let kinds: Vec<&'static str> = events
                        .iter()
                        .map(|e| match e.kind {
                            crate::pipeline_events::PipelineBusEventKind::Error => "Error",
                            crate::pipeline_events::PipelineBusEventKind::Eos => "Eos",
                            crate::pipeline_events::PipelineBusEventKind::StateChanged => {
                                "StateChanged"
                            }
                            crate::pipeline_events::PipelineBusEventKind::Warning => "Warning",
                            crate::pipeline_events::PipelineBusEventKind::ClockLost => "ClockLost",
                        })
                        .collect();
                    tracing::info!(
                        handle = %handle.0,
                        kinds = ?kinds,
                        "MEDIA-RT-01 bus watch 首次接获真实 GStreamer Bus 事件 (P1-4 接线生效)"
                    );
                }
                for e in &events {
                    match e.kind {
                        crate::pipeline_events::PipelineBusEventKind::Error => {
                            g.acceptance.c_pipeline_errors += 1;
                        }
                        crate::pipeline_events::PipelineBusEventKind::Eos => {
                            g.acceptance.c_unexpected_eos += 1;
                        }
                        // P1-4 最低策略映射 (bus_event_recovery_policy): ClockLost = degraded, 不自动重启.
                        crate::pipeline_events::PipelineBusEventKind::ClockLost => {
                            crate::pipeline::CLOCK_LOST_EVENTS
                                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            tracing::warn!(
                                handle = %handle.0,
                                severity = ?e.severity,
                                detail = %e.detail,
                                policy = crate::pipeline_events::bus_event_recovery_policy(e.kind),
                                "Bus ClockLost: 标记 degraded, 不触发重启 (完整 Clock Recovery 属 V0.3/P2)"
                            );
                        }
                        crate::pipeline_events::PipelineBusEventKind::Warning => {
                            tracing::warn!(
                                handle = %handle.0,
                                severity = ?e.severity,
                                detail = %e.detail,
                                "Bus Warning (可恢复异常, 记录不重启)"
                            );
                        }
                        crate::pipeline_events::PipelineBusEventKind::StateChanged => {
                            tracing::info!(
                                handle = %handle.0,
                                detail = %e.detail,
                                "Bus StateChanged (生命周期事件)"
                            );
                        }
                    }
                }
                bus_events = g.bus_event_count;
                (
                    g.acceptance.a_pass() && g.acceptance.b_pass() && g.acceptance.c_pass(),
                    g.last_error.is_some(),
                    g.acceptance.a4_signal_detected,
                )
            } else {
                (false, false, false)
            };

            // P0-7D-1.4 (ingest 接线): 上游总线观测 → canonical 事件流 (Supervisor.ingest
            // 归一化, C2 契约首次接线; mapper 关键字: "error"→PipelineFault{retryable},
            // "device lost"/"hotplug"→HardwareFault)。ingest 先于本 tick drain — 事件在
            // 产生当 tick 即被消费 (drain 破坏性单次), 与轮询条件 OR 后同一 if 内至多一次
            // report_failure, 无跨 tick 双计。
            for e in events.iter() {
                if matches!(e.kind, crate::pipeline_events::PipelineBusEventKind::Error) {
                    sup.lock().unwrap().ingest(
                        events::EventSource::Upstream,
                        &format!("pipeline error: {}", e.detail),
                    );
                }
            }
            // P0-7D-2.1: SignalVerified 点亮 — a4 (信号检出) 翻真即语义时刻 (闩锁去重;
            // 经 FanoutSink 双日志可见, 投影端点 kind_counts 同步可观测)。
            if !signal_latched && a4_signal {
                signal_latched = true;
                sink.emit(events::RuntimeEvent::SignalVerified {
                    device_id: device_uuid,
                    port_id: None,
                });
            }
            // P0-7D-1.3 (事件内消费): drain internal → reduce → 写回 agent_state。
            let drained_internal = internal_log.drain();
            // P0-7D-1.4 (事件驱动故障输入): 谓词抽为 supervisor::fault_trigger_from_events
            // (纯函数, mock 面可测 — 见 evt_int_rt_01_fault_trigger_echo_never_retriggers);
            // 自回声排除/归属判定/平面分离语义在彼处锁定。
            let fault_from_events =
                supervisor::fault_trigger_from_events(&drained_internal, device_uuid);
            health_fold = crate::health::reduce(&health_fold, &drained_internal);
            *agent_state.lock().unwrap() = health_fold.agent;

            // 错误 / 总线错误 / 事件驱动故障 → Supervisor 决策引擎 (仅决策, 不碰 GStreamer).
            if has_error
                || fault_from_events
                || events.iter().any(|e| {
                    matches!(
                        e.kind,
                        crate::pipeline_events::PipelineBusEventKind::Error
                            | crate::pipeline_events::PipelineBusEventKind::Eos
                    )
                })
            {
                match sup.lock().unwrap().report_failure(&device_uuid) {
                    Ok(supervisor::SupervisorAction::Restart) => {
                        // Lease→Pipeline: recover 前必须重校租约仍在有效期内 (MEDIA-03 排他不变量).
                        if !lm.is_valid(&device_uuid) {
                            tracing::error!(device = %device_uuid, "recover 中止: lease 失效 (排他不变量)");
                            // P0-7D: 状态迁移必随事件 — 经 sink 发 HealthChanged (决策平面词表),
                            // 由 reducer 折叠派生 (替代原命令式直写)。
                            sink.emit(events::RuntimeEvent::HealthChanged {
                                from: "restarting".into(),
                                to: "manual_required".into(),
                            });
                            continue;
                        }
                        let backoff = sup.lock().unwrap().backoff(&device_uuid);
                        let _ = sup.lock().unwrap().begin_restart(&device_uuid);
                        std::thread::sleep(backoff);
                        match ctrl.recover(&handle) {
                            Ok(()) => {
                                sup.lock().unwrap().report_recovered(&device_uuid).ok();
                                tracing::warn!(handle = %handle.0, "MEDIA-RT-01 watchdog: recover 成功 (Supervisor→PipelineController.recover 闭环)");
                            }
                            Err(e) => tracing::error!(error = %e, "recover 失败"),
                        }
                    }
                    Ok(supervisor::SupervisorAction::Escalate) => {
                        tracing::error!(handle = %handle.0, "MEDIA-RT-01 watchdog: Escalate (MANUAL_REQUIRED)");
                        // P0-7D: report_failure Escalate 路径已发 HealthChanged{manual_required},
                        // ManualRequired 由 reducer 派生 (原命令式直写删除)。
                    }
                    Err(e) => tracing::error!(error = %e, "supervisor report_failure 失败"),
                }
            } else if pass {
                // P0-7D: Capturing 由 reducer 从 SignalVerified/SessionStateChanged{Running}
                // 派生 (原命令式直写删除); 本分支仅保留证据日志。
                tracing::info!(
                    handle = %handle.0,
                    video_frames = prev_video,
                    audio_frames = prev_audio,
                    bus_events,
                    "MEDIA-RT-01: A+B+C 全过 (canonical first-buffer 路径健康)"
                );
            } else if tick.is_multiple_of(20) {
                // 诊断: pass 未达成时打印各子项, 便于现场定位 (每 ~10s 一次, 防刷屏).
                let snap = crate::pipeline_events::read_health(&handle).unwrap_or_default();
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
                    c3 = snap.acceptance.c3_no_repeated_reneg,
                    c4 = snap.acceptance.c4_counters_continue,
                    cwin_ms = snap.acceptance.c_observed_ms.unwrap_or(0),
                    cwin_cfg = snap.acceptance.c_configured_window_ms,
                    vframes = snap.video_frame_count,
                    aframes = snap.audio_frame_count,
                    vpts = snap.video_first_pts.unwrap_or(0),
                    apts = snap.audio_first_pts.unwrap_or(0),
                    bus = snap.bus_event_count,
                    "MEDIA-RT-01 诊断 (未全过)"
                );
            }
            tick += 1;
        }
    });
}
