#![allow(dead_code)]

//! GStreamer 媒体后端实现 (C7: 从 `pipeline.rs` 编排层迁入, 业务层不再直接 `use gstreamer`)。
//! 仅 `gstreamer`/`gstreamer_app`/`glib` 三个门面 crate 须在 `gstreamer-backend` 下才可用, 其余
//! (`std` 与 `crate::pipeline` 共享类型) 常驻; 非 gstreamer 构建下整块为 unused, 故 `allow(dead_code)`。
//! 共享事件/健康类型 (HEALTH_ARCS/BusSeverity/PipelineBusEvent 等) 已迁至中性模块 `pipeline_events`,
//! 不在此文件定义, 故 `unused_imports` 由 `adapters/gstreamer/mod.rs` 的 `#[allow(unused_imports)]`
//! 在门面层统一收敛, 此处不再重复声明以避免 duplicated attribute。

#[cfg(feature = "gstreamer-backend")]
use crate::contracts::backend::MediaBackend;
#[cfg(feature = "gstreamer-backend")]
use gstreamer::prelude::*;
#[cfg(feature = "gstreamer-backend")]
use gstreamer_app::AppSink;
#[cfg(feature = "gstreamer-backend")]
use glib;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::sync::Mutex;
use crate::pipeline::{
    DROPPED_BUS_EVENTS, LAST_FATAL_BUS_EVENT, NEXT_PIPELINE_ID, PipelineController, PipelineError,
    PipelineHandle, PipelineHealth, PipelinePlan, src_props,
};
// C7: 共享事件/健康类型已物理迁至中性模块 `pipeline_events` (不依赖 gstreamer crate),
// 此处仅 `use` 业务实现实际用到的子集 (HEALTH_ARCS/BusSeverity/PipelineBusEvent/PipelineBusEventKind);
// read_health/bus_event_recovery_policy 仅被 main/supervisor 经 `crate::pipeline` 重导出消费, 本实现不引用.
use crate::pipeline_events::{BusSeverity, HEALTH_ARCS, PipelineBusEvent, PipelineBusEventKind};
/// GStreamer 实现 (feature `gstreamer`).
pub struct GStreamerPipelineController {
    /// 运行时 pipeline 实例 (GStreamer Bin 对 + 物化计划), 供 start/recover 操作 (P0-2 修复核心:
    /// 旧 `launch()` 内部 Bin 未留存, start/recover 无对象可操作). 非 gstreamer 构建无此字段.
    #[cfg(feature = "gstreamer-backend")]
    instances: Mutex<HashMap<PipelineHandle, GstInstance>>,
}

/// 运行时 pipeline 实例 (仅 gstreamer 构建存在).
///
/// P1-4: 由 **单个** `GstPipeline`(内含 video+audio 两路 branch) 取代原先分离的两个 `Bin`
/// (PIPELINE-AV-01 前置: 统一 Bus + 单一 Clock domain). Bus watch 运行在专用 GLib
/// MainContext 线程 (`thread` + `stop_flag`), 经 bounded mpsc (`bus_rx`) 把事件交给 `poll_bus`.
#[cfg(feature = "gstreamer-backend")]
struct GstInstance {
    pipeline: gstreamer::Pipeline,
    plan: PipelinePlan,
    bus_rx: Receiver<PipelineBusEvent>,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(feature = "gstreamer-backend")]
impl GstInstance {
    /// 通知 Bus watch 线程退出并释放 DeckLink 设备 (recover / drop 前置).
    fn stop(&mut self) {
        self.stop_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        let _ = self.pipeline.set_state(gstreamer::State::Null);
    }
}

impl GStreamerPipelineController {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "gstreamer-backend")]
            instances: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for GStreamerPipelineController {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineController for GStreamerPipelineController {
    fn prepare(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError> {
        #[cfg(feature = "gstreamer-backend")]
        {
            let handle = PipelineHandle(NEXT_PIPELINE_ID.fetch_add(1, Ordering::SeqCst));
            // 每个 pipeline 实例独占一条 bounded mpsc channel: Bus watch 投递, poll_bus 非阻塞 drain.
            let (bus_tx, bus_rx) = sync_channel::<PipelineBusEvent>(256);
            let (pipeline, thread, stop_flag) = self.build_pipeline(plan, handle, bus_tx)?;
            // 注册健康 (默认全 false — P1-2; 由真实事件逐项置 true).
            HEALTH_ARCS
                .lock()
                .unwrap()
                .insert(handle, Arc::new(Mutex::new(PipelineHealth::default())));
            if let Some(hp) = HEALTH_ARCS.lock().unwrap().get(&handle) {
                hp.lock().unwrap().started_at = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0),
                );
            }
            self.instances.lock().unwrap().insert(
                handle,
                GstInstance {
                    pipeline,
                    plan: plan.clone(),
                    bus_rx,
                    stop_flag,
                    thread: Some(thread),
                },
            );
            Ok(handle)
        }
        #[cfg(not(feature = "gstreamer-backend"))]
        {
            let _ = plan;
            Ok(PipelineHandle(1))
        }
    }

    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        #[cfg(feature = "gstreamer-backend")]
        {
            // 统一 GstPipeline: 单一 set_state(Playing) 同时启动 video+audio 两路 (P1-4).
            let res = {
                let guard = self.instances.lock().unwrap();
                let inst = guard.get(handle).ok_or_else(|| {
                    PipelineError::StartFailed(format!("未知 pipeline handle (start): {handle:?}"))
                })?;
                inst.pipeline.set_state(gstreamer::State::Playing)
            };
            res.map_err(|e| PipelineError::StartFailed(format!("pipeline play: {e}")))?;
            // 标记 playing (watchdog 推导 a3_pipeline_playing). 与 instances 锁不嵌套, 避免死锁.
            if let Some(hp) = HEALTH_ARCS.lock().unwrap().get(handle) {
                hp.lock().unwrap().playing = true;
            }
            Ok(())
        }
        #[cfg(not(feature = "gstreamer-backend"))]
        {
            let _ = handle;
            Ok(())
        }
    }

    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        #[cfg(feature = "gstreamer-backend")]
        {
            let plan = {
                let guard = self.instances.lock().unwrap();
                guard.get(handle).map(|i| i.plan.clone()).ok_or_else(|| {
                    PipelineError::StartFailed(format!(
                        "未知 pipeline handle (recover): {handle:?}"
                    ))
                })?
            };
            // 停止并丢弃旧实例: 通知 GLib 线程退出 + join + 释放 DeckLink 设备.
            if let Some(mut old) = self.instances.lock().unwrap().remove(handle) {
                old.stop();
            }
            // 重建统一 GstPipeline (新 bus channel).
            let (bus_tx, bus_rx) = sync_channel::<PipelineBusEvent>(256);
            let (pipeline, thread, stop_flag) = self.build_pipeline(&plan, *handle, bus_tx)?;
            pipeline
                .set_state(gstreamer::State::Playing)
                .map_err(|e| PipelineError::StartFailed(format!("pipeline play: {e}")))?;
            self.instances.lock().unwrap().insert(
                *handle,
                GstInstance {
                    pipeline,
                    plan,
                    bus_rx,
                    stop_flag,
                    thread: Some(thread),
                },
            );
            if let Some(hp) = HEALTH_ARCS.lock().unwrap().get(handle) {
                hp.lock().unwrap().playing = true;
            }
            Ok(())
        }
        #[cfg(not(feature = "gstreamer-backend"))]
        {
            let _ = handle;
            Ok(())
        }
    }
}

#[cfg(feature = "gstreamer-backend")]
// P0-2: MediaBackend 方法形状对齐冻结契约 (instantiate/start/stop/recover/observe);
// 实现委托旧 `PipelineController` trait 方法 (prepare/recover 语义不变) 与固有 poll_bus。
impl MediaBackend for GStreamerPipelineController {
    fn instantiate(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError> {
        <Self as PipelineController>::prepare(self, plan)
    }
    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        <Self as PipelineController>::start(self, handle)
    }
    /// P0-2 补齐契约 `stop`: 通知 Bus watch 退出 + `set_state(Null)` 释放 DeckLink +
    /// 从实例表/健康表移除 (防句柄与健康条目泄漏)。
    fn stop(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        let inst = {
            let mut guard = self.instances.lock().unwrap();
            guard.remove(handle)
        };
        if let Some(mut inst) = inst {
            inst.stop();
        } else {
            return Err(PipelineError::StartFailed(format!(
                "未知 pipeline handle (stop): {handle:?}"
            )));
        }
        crate::pipeline_events::HEALTH_ARCS.lock().unwrap().remove(handle);
        Ok(())
    }
    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        <Self as PipelineController>::recover(self, handle)
    }
    fn observe(&self, handle: &PipelineHandle) -> Vec<PipelineBusEvent> {
        GStreamerPipelineController::poll_bus(self, handle)
    }
}

impl GStreamerPipelineController {
    /// 构造 decklinkvideosrc/audiosrc → video/x-raw → appsink 的采集 pipeline (P0-2 修复核心).
    /// 仅构建 + 注册 appsink 回调, **不**立即 Playing; Playing 由 `PipelineController::start` 负责
    /// (统一生命周期: prepare 构建 → start 播放 → recover 重建+播放). 旧 `launch()` 已被此拆分取代,
    /// 不再作为绕过 `PipelineController` 的第二入口.
    ///
    /// 注: appsink 当前用作 MEDIA-RT-01 首帧/PTS acceptance 探针; 最终生产媒体出口
    /// (Normalize → FRAME/MASTER SWITCH → Encode → SRS) 待 A2+ 实现 (用户复核 §十三).
    /// 构造 **单个** `GstPipeline`(video+audio 两路 branch 同处一个 pipeline) — P1-4 前置
    /// (PIPELINE-AV-01: 统一 Bus + 单一 Clock domain). 挂载 appsink 回调(首帧/PTS 探针),
    /// 并在专用 GLib MainContext 线程上挂 Bus watch, 把 Error/EOS/StateChanged/Warning/ClockLost
    /// 经 bounded mpsc 投递; `poll_bus` 非阻塞 drain.
    ///
    /// 旧 `build_bins` 把 video/audio 拆成两个独立 `Bin`(各自无统一 Bus/Clock), 导致 `poll_bus`
    /// 只能 stub. 现统一为单 `GstPipeline`, Bus watch 才能真实生效.
    /// GLib main loop 必须运行 (用户复核 §五): watch 回调在 `MainLoop` 迭代的 MainContext 上分发,
    /// 故 spawn 专用线程持有 `MainContext`+`MainLoop` 并 `run()`.
    #[cfg(feature = "gstreamer-backend")]
    fn build_pipeline(
        &self,
        plan: &PipelinePlan,
        handle: PipelineHandle,
        bus_tx: SyncSender<PipelineBusEvent>,
    ) -> Result<
        (
            gstreamer::Pipeline,
            std::thread::JoinHandle<()>,
            Arc<std::sync::atomic::AtomicBool>,
        ),
        PipelineError,
    > {
        gstreamer::init().map_err(|e| PipelineError::StartFailed(format!("gst init: {e}")))?;
        let (video_src, audio_src) = src_props(plan);
        // 单一 launch 串内含两条 branch → 同属一个 GstPipeline (共享 Bus/Clock).
        let video_branch =
            format!("{video_src} ! video/x-raw ! appsink name=videosink async=false");
        let audio_branch =
            format!("{audio_src} ! audio/x-raw ! appsink name=audiosink async=false");
        let launch = format!("{video_branch} {audio_branch}");
        let pipeline = gstreamer::parse::launch(&launch)
            .map_err(|e| PipelineError::StartFailed(format!("pipeline parse: {e}")))?
            .dynamic_cast::<gstreamer::Pipeline>()
            .map_err(|_| PipelineError::StartFailed("launch 结果非 GstPipeline".into()))?;
        let v_appsink = pipeline
            .by_name("videosink")
            .and_then(|e| e.dynamic_cast::<AppSink>().ok())
            .ok_or_else(|| PipelineError::StartFailed("videosink cast".into()))?;
        let a_appsink = pipeline
            .by_name("audiosink")
            .and_then(|e| e.dynamic_cast::<AppSink>().ok())
            .ok_or_else(|| PipelineError::StartFailed("audiosink cast".into()))?;

        self.attach_video_sink(&v_appsink, handle);
        self.attach_audio_sink(&a_appsink, handle);

        // —— Bus watch: 专用 GLib MainContext/MainLoop 线程 (用户复核 §五/§六) ——
        // 注意: Bus watch 回调只在 MainLoop 迭代其 MainContext 时才分发. 因此必须有独立线程
        // 持有 MainContext 并 run MainLoop; 否则编译通过但事件永远不到 (用户 §五 风险点).
        let stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_for_thread = stop_flag.clone();
        let p = pipeline.clone();
        let thread = std::thread::spawn(move || {
            let ctx = glib::MainContext::default();
            // 把 ctx 设为该线程 thread-default, 使 MainLoop(Some(&ctx)) 与 add_watch/timeout 都使用同一 ctx
            // (GLib main loop 必须运行, 否则 watch 回调永不分发 — 用户 §五 风险点).
            let res = ctx.with_thread_default(|| {
                let ml = std::rc::Rc::new(glib::MainLoop::new(Some(&ctx), false));
                let bus = match p.bus() {
                    Some(b) => b,
                    None => {
                        tracing::warn!(handle = %handle.0, "pipeline 无 bus, Bus watch 未挂载");
                        return;
                    }
                };
                let tx = bus_tx.clone();
                let h = handle;
                // watch 回调: 把消息翻译为结构化 PipelineBusEvent 投递进 channel.
                let _watch = bus
                    .add_watch(move |_, msg| {
                        if let Some(evt) = GStreamerPipelineController::translate_bus(msg, h) {
                            // 致命事件 (Error/EOS) 永不静默丢弃: 先存 sticky 副本 (即便 channel 满也能被读取).
                            if evt.kind == PipelineBusEventKind::Error
                                || evt.kind == PipelineBusEventKind::Eos
                            {
                                *LAST_FATAL_BUS_EVENT.lock().unwrap() = Some(evt.clone());
                            }
                            // 非阻塞投递: 满则计数丢弃 (不阻塞 GLib watch 线程); 但致命事件已留存 sticky.
                            match tx.try_send(evt) {
                                Ok(()) => {}
                                Err(std::sync::mpsc::TrySendError::Full(_)) => {
                                    DROPPED_BUS_EVENTS.fetch_add(1, Ordering::SeqCst);
                                    tracing::warn!(handle = %h.0, "Bus channel 溢出: 事件被计数为丢弃 (ERROR/EOS 已存 sticky, 不静默丢失)");
                                }
                                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                                    tracing::warn!(handle = %h.0, "Bus channel 已断开 (controller dropped)");
                                }
                            }
                        }
                        glib::ControlFlow::Continue
                    })
                    .expect("bus add_watch 失败");
                // 周期检查 stop_flag, 置位则退出 MainLoop (recover/stop 时通知线程结束, 避免线程泄漏).
                let ml_timeout = ml.clone();
                let stop = stop_for_thread.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
                    if stop.load(std::sync::atomic::Ordering::SeqCst) {
                        ml_timeout.quit();
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
                ml.run();
                tracing::debug!(handle = %h.0, "GStreamer Bus watch 线程退出");
            });
            if let Err(e) = res {
                tracing::warn!(error = %e, "Bus watch MainContext 推送失败");
            }
        });
        Ok((pipeline, thread, stop_flag))
    }

    /// 将 GStreamer `Message` 翻译为结构化 `PipelineBusEvent` (P1-4).
    /// 仅保留 Supervisor 关心的 Error/EOS/StateChanged/Warning/ClockLost; 其余消息忽略.
    #[cfg(feature = "gstreamer-backend")]
    fn translate_bus(msg: &gstreamer::Message, handle: PipelineHandle) -> Option<PipelineBusEvent> {
        let (kind, severity, detail) = match msg.view() {
            gstreamer::MessageView::Error(e) => (
                PipelineBusEventKind::Error,
                BusSeverity::Error,
                format!("{} | debug={:?}", e.error(), e.debug()),
            ),
            gstreamer::MessageView::Eos(_) => (
                PipelineBusEventKind::Eos,
                BusSeverity::Info,
                "end-of-stream".to_string(),
            ),
            gstreamer::MessageView::Warning(w) => (
                PipelineBusEventKind::Warning,
                BusSeverity::Warning,
                format!("{}", w.error()),
            ),
            gstreamer::MessageView::StateChanged(sc) => (
                PipelineBusEventKind::StateChanged,
                BusSeverity::Info,
                format!("{:?} -> {:?}", sc.old(), sc.current()),
            ),
            gstreamer::MessageView::ClockLost(_) => (
                PipelineBusEventKind::ClockLost,
                BusSeverity::Warning,
                "clock-lost".to_string(),
            ),
            _ => return None,
        };
        let source = msg.src().map(|s| s.name().to_string()).unwrap_or_default();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Some(PipelineBusEvent {
            handle,
            kind,
            source,
            timestamp,
            detail,
            severity,
        })
    }

    /// 注册视频 appsink 回调: 首帧/PTS 探测 (MEDIA-RT-01 B).
    #[cfg(feature = "gstreamer-backend")]
    fn attach_video_sink(&self, sink: &AppSink, handle: PipelineHandle) {
        sink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gstreamer::FlowError::Eos)?;
                    let buf = sample.buffer().ok_or(gstreamer::FlowError::Error)?;
                    if let Some(h) = HEALTH_ARCS.lock().unwrap().get(&handle) {
                        let mut h = h.lock().unwrap();
                        h.video_frame_count += 1;
                        let pts = buf.pts().map(|c| c.nseconds());
                        if let Some(pts) = pts {
                            // 记录首帧 PTS (与单调性状态解耦, P1-3).
                            if h.video_first_pts.is_none() {
                                h.video_first_pts = Some(pts);
                            }
                            // 真正单调性三态机 (用户复核 §十一/§三 P1-3): 首有效 PTS → ValidMonotonic,
                            // 回退 → NonMonotonic (sticky). 无 PTS 帧 (None) 已在上方跳过, 不污染判定.
                            h.observe_video_pts(pts);
                        }
                    }
                    Ok(gstreamer::FlowSuccess::Ok)
                })
                .build(),
        );
    }

    /// 注册音频 appsink 回调: 首帧/PTS 探测 (MEDIA-RT-01 B, 含真实单调判定).
    #[cfg(feature = "gstreamer-backend")]
    fn attach_audio_sink(&self, sink: &AppSink, handle: PipelineHandle) {
        sink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gstreamer::FlowError::Eos)?;
                    let buf = sample.buffer().ok_or(gstreamer::FlowError::Error)?;
                    if let Some(h) = HEALTH_ARCS.lock().unwrap().get(&handle) {
                        let mut h = h.lock().unwrap();
                        h.audio_frame_count += 1;
                        let pts = buf.pts().map(|c| c.nseconds());
                        if let Some(pts) = pts {
                            // 记录首帧 PTS (与单调性状态解耦, P1-3).
                            if h.audio_first_pts.is_none() {
                                h.audio_first_pts = Some(pts);
                            }
                            // 音频同样做真实单调性三态判定 (用户复核 §十一/§三 P1-3).
                            h.observe_audio_pts(pts);
                        }
                    }
                    Ok(gstreamer::FlowSuccess::Ok)
                })
                .build(),
        );
    }

    /// 非阻塞 drain 当前 GStreamer Bus 事件 (Bus watch 线程已投递进 bounded mpsc).
    /// watchdog 每 500ms 调用一次, 不阻塞媒体/GStreamer 线程 (用户复核 §六: 解耦节拍).
    #[cfg(feature = "gstreamer-backend")]
    pub fn poll_bus(&self, handle: &PipelineHandle) -> Vec<PipelineBusEvent> {
        let guard = self.instances.lock().unwrap();
        match guard.get(handle) {
            Some(inst) => inst.bus_rx.try_iter().collect(),
            None => Vec::new(),
        }
    }
}
