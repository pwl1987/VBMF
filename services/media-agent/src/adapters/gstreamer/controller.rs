#![allow(dead_code)]

//! GStreamer 媒体后端实现 (C7: 从 `pipeline.rs` 编排层迁入, 业务层不再直接 `use gstreamer`)。
//! 仅 `gstreamer`/`gstreamer_app`/`glib` 三个门面 crate 须在 `gstreamer-backend` 下才可用, 其余
//! (`std` 与 `crate::pipeline` 共享类型) 常驻; 非 gstreamer 构建下整块为 unused, 故 `allow(dead_code)`。
//! 共享事件/健康类型 (HEALTH_ARCS/BusSeverity/PipelineBusEvent 等) 已迁至中性模块 `pipeline_events`,
//! 不在此文件定义, 故 `unused_imports` 由 `adapters/gstreamer/mod.rs` 的 `#[allow(unused_imports)]`
//! 在门面层统一收敛, 此处不再重复声明以避免 duplicated attribute。

#[cfg(feature = "gstreamer-backend")]
use crate::contracts::backend::MediaBackend;
use crate::pipeline::{
    src_props, PipelineController, PipelineError, PipelineHandle, PipelineHealth, PipelinePlan,
    DROPPED_BUS_EVENTS, LAST_FATAL_BUS_EVENT, NEXT_PIPELINE_ID,
};
#[cfg(feature = "gstreamer-backend")]
use glib;
#[cfg(feature = "gstreamer-backend")]
use gstreamer::prelude::*;
#[cfg(feature = "gstreamer-backend")]
use gstreamer_app::AppSink;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::sync::Mutex;
// C7: 共享事件/健康类型已物理迁至中性模块 `pipeline_events` (不依赖 gstreamer crate),
// 此处仅 `use` 业务实现实际用到的子集 (HEALTH_ARCS/BusSeverity/PipelineBusEvent/PipelineBusEventKind);
// read_health/bus_event_recovery_policy 仅被 main/supervisor 经 `crate::pipeline` 重导出消费, 本实现不引用.
use crate::pipeline_events::{BusSeverity, PipelineBusEvent, PipelineBusEventKind, HEALTH_ARCS};
/// GStreamer 实现 (feature `gstreamer`).
pub struct GStreamerPipelineController {
    /// 运行时 pipeline 实例 (GStreamer Bin 对 + 物化计划), 供 start/recover 操作 (P0-2 修复核心:
    /// 旧 `launch()` 内部 Bin 未留存, start/recover 无对象可操作). 非 gstreamer 构建无此字段.
    #[cfg(feature = "gstreamer-backend")]
    instances: Mutex<HashMap<PipelineHandle, GstInstance>>,
    /// A2-8-02-G/H: Bridge 观测统计——分流分支 pad probe 实测（帧计数/
    /// 最后 PTS/三态单调, 按 (handle, channel, plane)）。runtime
    /// observation fact, 与 GstInstance.media_taps 静态簿记严格分层
    /// （第十四轮 §5）; detach 摘分支即移除条目（absence≠evidence）。
    #[cfg(feature = "gstreamer-backend")]
    bridge_stats: BridgeStatsMap,
    /// A2-8-02-G/H-1: 桥**观察时钟**原点（liveness 判定的 wall clock——
    /// 观察时序, 与媒体时序 PTS 严格分离, 第十五轮 §8）。
    #[cfg(feature = "gstreamer-backend")]
    bridge_clock_origin: std::time::Instant,
}

/// A2-8-02-G/H: 桥观测统计表类型别名（(handle, channel, plane) → 单平面
/// 统计; probe 写入 / port 查询读取）。
#[cfg(feature = "gstreamer-backend")]
type BridgeStatsMap =
    std::sync::Arc<Mutex<HashMap<(PipelineHandle, String, &'static str), BridgeStat>>>;

/// A2-8-02-G/H: 单平面桥统计（probe 侧维护——三态与 PipelineHealth
/// observe_*_pts 同语义: Unknown 起步/回退 sticky NonMonotonic）。
#[cfg(feature = "gstreamer-backend")]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BridgeStat {
    pub last_pts: Option<u64>,
    pub state: crate::pipeline::PtsMonotonicity,
    pub frames: u64,
    /// A2-8-02-G/H-1: 最后实测时刻（观察时钟 ms——liveness 证据;
    /// 与历史证据 frames 分层, 第十五轮 §7）。
    pub last_observed_ms: Option<u64>,
}

#[cfg(feature = "gstreamer-backend")]
impl Default for BridgeStat {
    fn default() -> Self {
        // 无证据起步 = Unknown（absence≠evidence——与三态语义一致）。
        Self {
            last_pts: None,
            state: crate::pipeline::PtsMonotonicity::Unknown,
            frames: 0,
            last_observed_ms: None,
        }
    }
}

/// 三态推进（与 PipelineHealth::observe_video_pts 同律）+ 观察时刻。
#[cfg(feature = "gstreamer-backend")]
fn bridge_observe_pts(stat: &mut BridgeStat, pts: u64, observed_at_ms: u64) {
    stat.frames += 1;
    stat.last_observed_ms = Some(observed_at_ms);
    match (stat.last_pts, stat.state) {
        (Some(last), crate::pipeline::PtsMonotonicity::NonMonotonic)
        | (Some(last), crate::pipeline::PtsMonotonicity::ValidMonotonic) => {
            if pts < last {
                stat.state = crate::pipeline::PtsMonotonicity::NonMonotonic; // sticky
            } else {
                stat.state = crate::pipeline::PtsMonotonicity::ValidMonotonic;
            }
        }
        _ => stat.state = crate::pipeline::PtsMonotonicity::ValidMonotonic,
    }
    stat.last_pts = Some(pts);
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
    /// A2-8-02-D: MediaTap attachment bookkeeping——recover 重建后重放
    /// attach 的**唯一事实源**（第六轮终裁: execution resource bookkeeping,
    /// 非第二 identity/execution registry; tap branch 生命周期由
    /// MediaTapPort 控制, 本字段只做簿记）。
    media_taps: Vec<crate::contracts::media_tap::MediaTapAttachment>,
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
            #[cfg(feature = "gstreamer-backend")]
            bridge_stats: std::sync::Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "gstreamer-backend")]
            bridge_clock_origin: std::time::Instant::now(),
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
                    media_taps: Vec::new(),
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
            // A2-8-02-D: 旧实例销毁前捕获 tap 簿记（重建后重放——唯一事实源）。
            let saved_taps: Vec<crate::contracts::media_tap::MediaTapAttachment> = {
                let guard = self.instances.lock().unwrap();
                guard
                    .get(handle)
                    .map(|i| i.media_taps.clone())
                    .unwrap_or_default()
            };
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
                    media_taps: Vec::new(),
                },
            );
            if let Some(hp) = HEALTH_ARCS.lock().unwrap().get(handle) {
                hp.lock().unwrap().playing = true;
            }
            // A2-8-02-D: attachment replay——新管线上按簿记重放 tap（失败
            // 不阻断 recover 本体: 管线已恢复, tap 恢复失败诚实记录降级）。
            {
                let mut guard = self.instances.lock().unwrap();
                if let Some(inst) = guard.get_mut(handle) {
                    for tap in saved_taps {
                        let req = crate::contracts::media_tap::MediaTapRequest {
                            channel: tap.channel.clone(),
                            planes: tap.planes,
                        };
                        match Self::attach_tap_to_instance(
                            *handle,
                            inst,
                            &req,
                            &self.bridge_stats,
                            self.bridge_clock_origin,
                        ) {
                            Ok(()) => tracing::info!(
                                handle = handle.0,
                                channel = %tap.channel,
                                "A2-8-02-D recover: tap 簿记重放成功 (新管线)"
                            ),
                            Err(e) => tracing::warn!(
                                handle = handle.0,
                                channel = %tap.channel,
                                error = ?e,
                                "A2-8-02-D recover: tap 重放失败 (管线本体已恢复, tap 降级待重挂)"
                            ),
                        }
                    }
                }
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
        crate::pipeline_events::HEALTH_ARCS
            .lock()
            .unwrap()
            .remove(handle);
        Ok(())
    }
    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        <Self as PipelineController>::recover(self, handle)
    }
    fn observe(&self, handle: &PipelineHandle) -> Vec<PipelineBusEvent> {
        GStreamerPipelineController::poll_bus(self, handle)
    }
}

// A2-8-02-I 第三十四轮终裁: Diagnostic Runtime Fault Injection——同一
// concrete controller 的第四 trait view（仅诊断消费, 禁入 MediaBackend
// 冻结 SPI）。注入"运行故障"非"生命周期终止": 真实执行面停流
// （set_state(Paused)——源/分支停止产出 buffer, frames/PTS 冻结,
// liveness 窗口自然过期）而 instances/HEALTH_ARCS 登记**保持**; 随后
// `MediaBackend::recover(handle)` 即为生产行为（同 handle 原 plan 重建）。
// 红线: 不注销 handle（那是 stop 的 P0-2 终态语义）; 不合成 Bus Error
// 事件（Observation Fact ≠ Synthetic Event——只作用于实际执行面）。
#[cfg(feature = "gstreamer-backend")]
impl crate::contracts::diagnostic::DiagnosticFaultInjection for GStreamerPipelineController {
    fn inject_runtime_stall(&self, handle: &PipelineHandle) -> Result<(), String> {
        let guard = self.instances.lock().unwrap();
        let inst = guard
            .get(handle)
            .ok_or_else(|| format!("未知 pipeline handle (inject stall): {handle:?}"))?;
        inst.pipeline
            .set_state(gstreamer::State::Paused)
            .map(|_| ())
            .map_err(|e| format!("diagnostic stall set_state(Paused) 失败: {e}"))
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
        let (video_src, audio_src) = src_props(plan)?;
        // P1a: 有输出段 ⇒ plan.output_launch 全串（tee 双分支: 分析 + 编码输出）;
        // 无输出段 ⇒ 今日串逐字节不变（纯分析, 向后兼容承诺）。controller 纯拼接执行,
        // 不在此出现任何编码/输出 element 名（用户边界修正: 输出物化在 pipeline.rs domain 层）。
        let launch = {
            let with_outputs = plan.output_launch(&video_src, &audio_src);
            if with_outputs.is_empty() {
                // A2-8-02-A: 纯分析形态同样构造**命名 tee = 通用 tap 点**
                // （MediaTapPort attach 的物化前提; tee 单消费分支行为等价
                // =透传, appsink 名/async 语义不变; **假实现禁令**: 构造期
                // 只建 tap 点, 不预塞 tap branch——branch 生命周期归
                // MediaTapPort）。pipeline.rs 零 diff（本组装在 controller 侧）。
                let video_branch = format!(
                    "{video_src} ! video/x-raw ! tee name=v \
                     v. ! queue ! appsink name=videosink async=false"
                );
                let audio_branch = format!(
                    "{audio_src} ! audio/x-raw ! tee name=a \
                     a. ! queue ! appsink name=audiosink async=false"
                );
                format!("{video_branch} {audio_branch}")
            } else {
                with_outputs
            }
        };
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

    /// tap branch 元素名（detach 定位/簿记外独立验证锚）。
    #[cfg(feature = "gstreamer-backend")]
    fn tap_element_name(plane: &str, channel: &str) -> String {
        format!("tap-{plane}-{channel}")
    }

    /// 在实例管线上物化一个 tap branch（tee request pad → inter sink →
    /// sync_state_with_parent），并记入簿记。构造期命名 tee（"v"/"a"）
    /// = 通用 tap 点（两形态皆有, §02-A）; branch 本体在此动态创建
    /// （**非构造期预设**——假实现禁令, probe §11.2）。
    #[cfg(feature = "gstreamer-backend")]
    fn attach_tap_to_instance(
        handle: PipelineHandle,
        inst: &mut GstInstance,
        req: &crate::contracts::media_tap::MediaTapRequest,
        bridge_stats: &BridgeStatsMap,
        clock_origin: std::time::Instant,
    ) -> Result<(), crate::contracts::media_tap::TapError> {
        use crate::contracts::media_tap::{TapError, TapPlanes};
        use gstreamer::prelude::*;
        let wants_video = matches!(req.planes, TapPlanes::Video | TapPlanes::Both);
        let wants_audio = matches!(req.planes, TapPlanes::Audio | TapPlanes::Both);
        // (tee 名, 是否需要, inter sink 工厂, 平面前缀)——video/audio 平面
        // 独立命名空间, 同 channel 两平面互不冲突。
        // **事务式物化（第七轮终裁 P2 债修复）**: 成功项入 staged, 任一平面
        // 失败 → 全部回滚（释放 tee request pad + Null + remove）——保证
        // Gst 真实图与 media_taps 簿记**原子一致**（簿记=recover 唯一事实
        // 源, Reality≠bookkeeping 不可发生）。
        let mut staged: Vec<(gstreamer::Element, gstreamer::Pad, gstreamer::Element)> = Vec::new();
        let materialize = (|| -> Result<(), TapError> {
            for (tee_name, want, factory, plane) in [
                ("v", wants_video, "intervideosink", "v"),
                ("a", wants_audio, "interaudiosink", "a"),
            ] {
                if !want {
                    continue;
                }
                let tee = inst.pipeline.by_name(tee_name).ok_or_else(|| {
                    TapError::TapPointUnavailable(format!(
                        "命名 tee {tee_name} 不在管线（tap 点缺失）"
                    ))
                })?;
                let el = gstreamer::ElementFactory::make(factory)
                    .name(Self::tap_element_name(plane, &req.channel))
                    .build()
                    .map_err(|e| TapError::Backend(format!("{factory} 构造失败: {e}")))?;
                el.set_property("channel", req.channel.as_str());
                inst.pipeline
                    .add(&el)
                    .map_err(|e| TapError::Backend(format!("tap 元素入管线: {e}")))?;
                el.sync_state_with_parent()
                    .map_err(|e| TapError::Backend(format!("tap 元素状态同步: {e}")))?;
                let tee_src = tee
                    .request_pad_simple("src_%u")
                    .ok_or_else(|| TapError::Backend("tee request pad 失败".into()))?;
                let el_sink = el
                    .static_pad("sink")
                    .ok_or_else(|| TapError::Backend("tap 元素 sink pad 缺失".into()))?;
                tee_src
                    .link(&el_sink)
                    .map_err(|e| TapError::Backend(format!("tap 链接: {e:?}")))?;
                // A2-8-02-G/H: 桥观测 probe——分流分支 sink pad 实测缓冲
                //（tap→inter 段真实经过的数据; 非输入/程序观测复制——
                // 三列各自独立测量, 第十四轮 §4）。键含 plane（"v"/"a"）。
                // G/H-1: probe 同时记录观察时刻（liveness 证据——观察时钟
                // 与媒体时序 PTS 分离, 第十五轮 §8）。
                {
                    let key = (handle, req.channel.clone(), plane);
                    let stats_for_probe = std::sync::Arc::clone(bridge_stats);
                    el_sink.add_probe(gstreamer::PadProbeType::BUFFER, move |_pad, info| {
                        if let Some(buf) = info.buffer() {
                            if let Some(pts) = buf.pts().map(|c| c.nseconds()) {
                                let now_ms = clock_origin.elapsed().as_millis() as u64;
                                let mut stats = stats_for_probe.lock().unwrap();
                                let st = stats.entry(key.clone()).or_default();
                                bridge_observe_pts(st, pts, now_ms);
                            }
                        }
                        gstreamer::PadProbeReturn::Ok
                    });
                }
                staged.push((tee, tee_src, el));
            }
            Ok(())
        })();
        match materialize {
            Ok(()) => {
                inst.media_taps
                    .push(crate::contracts::media_tap::MediaTapAttachment {
                        channel: req.channel.clone(),
                        planes: req.planes,
                    });
                Ok(())
            }
            Err(e) => {
                // 回滚（逆序）: 已物化分支全部移除, 簿记零增加。
                for (tee, tee_src, el) in staged.into_iter().rev() {
                    if let Some(sink_pad) = el.static_pad("sink") {
                        let _ = sink_pad.unlink(&tee_src);
                        tee.release_request_pad(&tee_src);
                    }
                    let _ = el.set_state(gstreamer::State::Null);
                    let _ = inst.pipeline.remove(&el);
                }
                tracing::warn!(
                    channel = %req.channel,
                    error = ?e,
                    "A2-8-02-C attach 部分失败已整体回滚（图与簿记原子一致）"
                );
                Err(e)
            }
        }
    }
}

// === A2-8-02-C: MediaTapPort 物化（同 ownership——无第二 registry, probe §11.3） ===
#[cfg(feature = "gstreamer-backend")]
impl crate::contracts::media_tap::MediaTapPort for GStreamerPipelineController {
    fn attach_media_tap(
        &self,
        handle: &PipelineHandle,
        req: &crate::contracts::media_tap::MediaTapRequest,
    ) -> Result<(), crate::contracts::media_tap::TapError> {
        use crate::contracts::media_tap::TapError;
        let mut guard = self.instances.lock().unwrap();
        let inst = guard
            .get_mut(handle)
            .ok_or(TapError::UnknownPipeline(*handle))?;
        if inst.media_taps.iter().any(|a| a.channel == req.channel) {
            return Err(TapError::AlreadyAttached(req.channel.clone()));
        }
        let stats = std::sync::Arc::clone(&self.bridge_stats);
        Self::attach_tap_to_instance(*handle, inst, req, &stats, self.bridge_clock_origin)
    }

    fn detach_media_tap(
        &self,
        handle: &PipelineHandle,
        channel: &str,
    ) -> Result<(), crate::contracts::media_tap::TapError> {
        use crate::contracts::media_tap::{TapError, TapPlanes};
        use gstreamer::prelude::*;
        let mut guard = self.instances.lock().unwrap();
        let inst = guard
            .get_mut(handle)
            .ok_or(TapError::UnknownPipeline(*handle))?;
        let planes = inst
            .media_taps
            .iter()
            .find(|a| a.channel == channel)
            .map(|a| a.planes)
            .ok_or_else(|| TapError::NotAttached(channel.into()))?;
        let wants_video = matches!(planes, TapPlanes::Video | TapPlanes::Both);
        let wants_audio = matches!(planes, TapPlanes::Audio | TapPlanes::Both);
        for (tee_name, want, plane) in [("v", wants_video, "v"), ("a", wants_audio, "a")] {
            if !want {
                continue;
            }
            let name = Self::tap_element_name(plane, channel);
            let Some(el) = inst.pipeline.by_name(&name) else {
                continue;
            };
            // 解链 + 释放 tee request pad + 移除元素（分支生命周期归本端口）。
            if let Some(sink_pad) = el.static_pad("sink") {
                if let Some(peer) = sink_pad.peer() {
                    let _ = sink_pad.unlink(&peer);
                    if let Some(tee) = inst.pipeline.by_name(tee_name) {
                        tee.release_request_pad(&peer);
                    }
                }
            }
            let _ = el.set_state(gstreamer::State::Null);
            let _ = inst.pipeline.remove(&el);
        }
        inst.media_taps.retain(|a| a.channel != channel);
        // A2-8-02-G/H: 分支已摘——桥观测条目随之移除（absence≠evidence,
        // 非冻结零值伪装）。
        self.bridge_stats
            .lock()
            .unwrap()
            .retain(|(h, ch, _), _| !(*h == *handle && ch == channel));
        Ok(())
    }

    fn tap_attachments(
        &self,
        handle: &PipelineHandle,
    ) -> Vec<crate::contracts::media_tap::MediaTapAttachment> {
        self.instances
            .lock()
            .unwrap()
            .get(handle)
            .map(|i| i.media_taps.clone())
            .unwrap_or_default()
    }
}

// === A2-8-02-G/H: Bridge 观测查询面（pad probe 实测, 第十四轮 §4） ===
#[cfg(feature = "gstreamer-backend")]
impl crate::contracts::media_tap::BridgeObservationPort for GStreamerPipelineController {
    fn bridge_observations(
        &self,
        handle: &PipelineHandle,
    ) -> Vec<crate::contracts::media_tap::BridgeObservation> {
        use crate::contracts::media_tap::BridgeObservation;
        // 按簿记 channel 分组——attached 才有观测行（摘除=absence≠evidence）。
        let attachments = {
            self.instances
                .lock()
                .unwrap()
                .get(handle)
                .map(|i| i.media_taps.clone())
                .unwrap_or_default()
        };
        let stats = self.bridge_stats.lock().unwrap();
        attachments
            .into_iter()
            .map(|a| {
                let v = stats
                    .get(&(*handle, a.channel.clone(), "v"))
                    .copied()
                    .unwrap_or_default();
                let s = stats
                    .get(&(*handle, a.channel.clone(), "a"))
                    .copied()
                    .unwrap_or_default();
                BridgeObservation {
                    channel: a.channel,
                    video_last_pts: v.last_pts,
                    video_pts_state: v.state,
                    video_frames: v.frames,
                    audio_last_pts: s.last_pts,
                    audio_pts_state: s.state,
                    audio_frames: s.frames,
                }
            })
            .collect()
    }

    /// A2-8-02-G/H-1: 当前推进性——观察时钟窗口判定（now - last_observed
    /// ≤ window; frames=历史证据, last_observed=活性证据, 严格分层）。
    fn bridge_liveness(
        &self,
        handle: &PipelineHandle,
        window_ms: u64,
    ) -> Vec<crate::contracts::media_tap::BridgeChannelLiveness> {
        use crate::contracts::media_tap::BridgeChannelLiveness;
        let attachments = {
            self.instances
                .lock()
                .unwrap()
                .get(handle)
                .map(|i| i.media_taps.clone())
                .unwrap_or_default()
        };
        let now_ms = self.bridge_clock_origin.elapsed().as_millis() as u64;
        let stats = self.bridge_stats.lock().unwrap();
        attachments
            .into_iter()
            .map(|a| {
                let v = stats
                    .get(&(*handle, a.channel.clone(), "v"))
                    .copied()
                    .unwrap_or_default();
                let s = stats
                    .get(&(*handle, a.channel.clone(), "a"))
                    .copied()
                    .unwrap_or_default();
                // 活性证据取双平面最近实测时刻。
                let last_observed = v.last_observed_ms.max(s.last_observed_ms);
                let alive_in_window = last_observed
                    .map(|t| now_ms.saturating_sub(t) <= window_ms)
                    .unwrap_or(false);
                BridgeChannelLiveness {
                    channel: a.channel,
                    frames: v.frames.max(s.frames),
                    last_observed_at_ms: last_observed,
                    alive_in_window,
                }
            })
            .collect()
    }
}

// —— 真实 GStreamer tap 验证（盒上 bmd+gstreamer; self_test 计划=纯分析形态） ——
#[cfg(all(test, feature = "gstreamer-backend"))]
mod media_tap_tests {
    use super::*;
    use crate::contracts::backend::MediaBackend;
    use crate::contracts::media_tap::{MediaTapPort, MediaTapRequest, TapError, TapPlanes};
    use crate::pipeline::PipelinePlan;

    fn started_analysis_pipeline() -> (GStreamerPipelineController, PipelineHandle) {
        let ctrl = GStreamerPipelineController::new();
        let h =
            MediaBackend::instantiate(&ctrl, &PipelinePlan::self_test()).expect("纯分析管线物化");
        MediaBackend::start(&ctrl, &h).expect("启动");
        (ctrl, h)
    }

    fn element_present(ctrl: &GStreamerPipelineController, h: &PipelineHandle, name: &str) -> bool {
        ctrl.instances
            .lock()
            .unwrap()
            .get(h)
            .map(|i| i.pipeline.by_name(name).is_some())
            .unwrap_or(false)
    }

    #[test]
    fn tap_rt_01_analysis_form_attach_detach_cycle() {
        // 02-A/C: 纯分析形态（原无 tee）现具通用 tap 点; attach 动态物化
        // 分支（簿记外独立验证元素实存——非"登记式假实现"）。
        let (ctrl, h) = started_analysis_pipeline();
        let req = MediaTapRequest {
            channel: "tap-t1".into(),
            planes: TapPlanes::Both,
        };
        ctrl.attach_media_tap(&h, &req)
            .expect("纯分析形态 attach（tee tap 点在）");
        assert_eq!(
            ctrl.tap_attachments(&h),
            vec![crate::contracts::media_tap::MediaTapAttachment {
                channel: "tap-t1".into(),
                planes: TapPlanes::Both,
            }],
            "簿记恰一行保真"
        );
        assert_eq!(
            ctrl.attach_media_tap(&h, &req),
            Err(TapError::AlreadyAttached("tap-t1".into())),
            "重复 attach fail-closed"
        );
        assert!(element_present(&ctrl, &h, "tap-v-tap-t1"), "video 分支实存");
        assert!(element_present(&ctrl, &h, "tap-a-tap-t1"), "audio 分支实存");

        ctrl.detach_media_tap(&h, "tap-t1").expect("detach");
        assert!(ctrl.tap_attachments(&h).is_empty());
        assert!(!element_present(&ctrl, &h, "tap-v-tap-t1"), "分支已移除");
        assert!(!element_present(&ctrl, &h, "tap-a-tap-t1"));
        assert_eq!(
            ctrl.detach_media_tap(&h, "tap-t1"),
            Err(TapError::NotAttached("tap-t1".into()))
        );
        let _ = ctrl.stop(&h);
    }

    #[test]
    fn tap_rt_01_video_only_planes_and_unknown_pipeline() {
        let (ctrl, h) = started_analysis_pipeline();
        ctrl.attach_media_tap(
            &h,
            &MediaTapRequest {
                channel: "tap-vonly".into(),
                planes: TapPlanes::Video,
            },
        )
        .expect("video-only attach");
        assert!(element_present(&ctrl, &h, "tap-v-tap-vonly"));
        assert!(
            !element_present(&ctrl, &h, "tap-a-tap-vonly"),
            "video-only 不建 audio 分支"
        );
        let _ = ctrl.stop(&h);

        let unknown = PipelineHandle(987_654);
        assert_eq!(
            ctrl.attach_media_tap(
                &unknown,
                &MediaTapRequest {
                    channel: "x".into(),
                    planes: TapPlanes::Both
                }
            ),
            Err(TapError::UnknownPipeline(unknown))
        );
    }

    #[test]
    fn tap_rt_01_recover_replays_attachments() {
        // 02-D: recover 销毁重建管线→簿记重放→tap 在新管线上恢复
        // （元素实存于**新** pipeline 对象——C2 闭环实证）。
        let (ctrl, h) = started_analysis_pipeline();
        ctrl.attach_media_tap(
            &h,
            &MediaTapRequest {
                channel: "tap-rc".into(),
                planes: TapPlanes::Both,
            },
        )
        .expect("attach");
        MediaBackend::recover(&ctrl, &h).expect("recover 重建");
        assert_eq!(ctrl.tap_attachments(&h).len(), 1, "簿记等值恢复");
        assert_eq!(ctrl.tap_attachments(&h)[0].channel, "tap-rc");
        assert!(
            element_present(&ctrl, &h, "tap-v-tap-rc"),
            "新管线上 video tap 分支重挂"
        );
        assert!(element_present(&ctrl, &h, "tap-a-tap-rc"), "audio 分支重挂");
        let _ = ctrl.stop(&h);
    }

    #[test]
    fn tap_rt_01_attach_partial_failure_rolls_back() {
        // 02-C P2 债（第七轮终裁）: Both 时 video 成功 + audio 失败 →
        // video 分支必须整体回滚, 簿记零增加——Gst 图与 media_taps 原子
        // 一致（簿记=recover 唯一事实源, Reality≠bookkeeping 不可发生）。
        // 故障注入: 从运行中管线移除 audio tee（模拟 tap 点缺失）。
        let (ctrl, h) = started_analysis_pipeline();
        {
            let mut guard = ctrl.instances.lock().unwrap();
            let inst = guard.get_mut(&h).unwrap();
            if let Some(a_tee) = inst.pipeline.by_name("a") {
                let _ = inst.pipeline.remove(&a_tee);
            }
        }
        let err = ctrl
            .attach_media_tap(
                &h,
                &MediaTapRequest {
                    channel: "tap-pf".into(),
                    planes: TapPlanes::Both,
                },
            )
            .expect_err("audio tap 点缺失应失败");
        assert!(matches!(err, TapError::TapPointUnavailable(_)));
        assert!(
            ctrl.tap_attachments(&h).is_empty(),
            "簿记零增加（无半条 attachment）"
        );
        assert!(
            !element_present(&ctrl, &h, "tap-v-tap-pf"),
            "已物化的 video 分支被回滚（图与簿记一致）"
        );
        assert!(!element_present(&ctrl, &h, "tap-a-tap-pf"));
        let _ = ctrl.stop(&h);
    }
}

// —— A2-8-02-I 第三十四轮: Diagnostic Runtime Fault Injection 真实验证 ——
#[cfg(all(test, feature = "gstreamer-backend"))]
mod diagnostic_tests {
    use super::*;
    use crate::contracts::backend::MediaBackend;
    use crate::contracts::diagnostic::DiagnosticFaultInjection;
    use crate::pipeline::PipelinePlan;

    fn started() -> (GStreamerPipelineController, PipelineHandle) {
        let ctrl = GStreamerPipelineController::new();
        let h =
            MediaBackend::instantiate(&ctrl, &PipelinePlan::self_test()).expect("纯分析管线物化");
        MediaBackend::start(&ctrl, &h).expect("启动");
        (ctrl, h)
    }

    fn frames(h: &PipelineHandle) -> Option<u64> {
        crate::pipeline_events::read_health(h).map(|x| x.video_frame_count)
    }

    // 契约证明（结构面）: 注入"运行故障"≠注销——instance 登记保持;
    // 随后 recover=生产行为（同 handle 原 plan 重建）。33 轮 stop→recover
    // 非法组合的反面: 注入后 recover 必须成功。
    #[test]
    fn diagnostic_rt_01_stall_keeps_registration_and_recovers() {
        let (ctrl, h) = started();
        ctrl.inject_runtime_stall(&h)
            .expect("注入成功（handle 在册）");
        assert!(
            ctrl.instances.lock().unwrap().get(&h).is_some(),
            "注入≠注销: instance 登记保持（终态注销是 stop 的 P0-2 语义）"
        );
        MediaBackend::recover(&ctrl, &h).expect("注入后 recover=生产行为（原 plan 重建）");
        assert!(
            ctrl.instances.lock().unwrap().get(&h).is_some(),
            "recover 后同 handle 在册"
        );
        let _ = MediaBackend::stop(&ctrl, &h);
    }

    // 契约证明（行为面）: 注入=真实执行面停流（帧冻结）→ recover 后复流。
    #[test]
    fn diagnostic_rt_02_stall_freezes_media_and_resumes_after_recover() {
        let (ctrl, h) = started();
        std::thread::sleep(std::time::Duration::from_millis(800));
        ctrl.inject_runtime_stall(&h).expect("注入");
        // 深入 Paused 稳态后采样（避开翻转在途帧的单帧噪声）。
        std::thread::sleep(std::time::Duration::from_millis(500));
        let f1 = frames(&h);
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let f2 = frames(&h);
        assert_eq!(f2, f1, "停流: 注入期间帧冻结（观测面仍在——absence≠注销）");
        MediaBackend::recover(&ctrl, &h).expect("recover");
        std::thread::sleep(std::time::Duration::from_millis(800));
        let f3 = frames(&h);
        assert!(
            f1.is_some_and(|a| f3.is_some_and(|b| b > a)),
            "复流: recover 后帧重新推进 (stalled={f1:?} recovered={f3:?})"
        );
        let _ = MediaBackend::stop(&ctrl, &h);
    }

    // 同一注册事实源: stop 终态注销后注入面诚实 fail-closed（与 recover
    // 报 UnknownPipeline 同源——非法组合的错误在入口即暴露）。
    #[test]
    fn diagnostic_rt_03_unknown_handle_fail_closed() {
        let (ctrl, h) = started();
        let _ = MediaBackend::stop(&ctrl, &h);
        assert!(
            ctrl.inject_runtime_stall(&h).is_err(),
            "已注销 handle 拒收（无第二注册表）"
        );
    }
}
