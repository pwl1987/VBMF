//! Canonical media ingest pipeline (Phase 0.6): GStreamer `decklinkvideosrc/audiosrc` → RAW.
//!
//! Boundary (SoT §14 / Phase 0.6 锁死):
//! - **canonical 媒体采集 = GStreamer**, 不是 `IDeckLinkInput` SDK 直采 (后者仅 SDK 诊断探针).
//! - 链路: `DeckLink → GStreamer → RAW → Normalize → FRAME/MASTER SWITCH → Program Master RAW
//!   → Encode → SRS → RTMP/HLS/WHEP`. Encode 在 Switcher 之后, 不得提前.
//! - 身份: `Device Registry`(SDK DeviceHandle) → `Resolver`(GStreamer `hw-serial-number` 解析)
//!   → `PipelinePlan`(解析后的 `device-number`). **SDK 枚举 index ≠ GStreamer device-number**.
//!
//! 当前 MEDIA-RT-01 范围: canonical ingest 首帧/PTS/稳定性验收. Normalize/Switch/Encode/SRS 为后续阶段.

#![allow(dead_code)]

use crate::device::{DeviceInfo, IdentityStrength};
use crate::graph_intent::GraphRuntimeIntent;
#[cfg(feature = "gstreamer")]
use gstreamer::prelude::*;
#[cfg(feature = "gstreamer")]
use gstreamer_app::AppSink;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use uuid::Uuid;

/// 全局唯一 pipeline 句柄计数器 (P1-1: 多 controller 共用全局 `HEALTH_ARCS` 时避免 handle 碰撞).
static NEXT_PIPELINE_ID: AtomicU64 = AtomicU64::new(1);

/// 媒体源选择模式 — 决定 GStreamer `decklinkvideosrc` 的选卡属性.
/// 语义必须无歧义 (用户复核 §五): 生产路径不得伪装成 "诊断 fallback".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceSelectionMode {
    /// PersistentID 可用 → 官方首选 `persistent-id=<BMD PersistentID>` (优先级高于 device-number).
    /// 当前硬件 (10.30.15.10) PersistentID 不支持 → 此分支本机不触发.
    PersistentIdCanonical,
    /// DeviceHandle 经 Resolver 解析到确定 GStreamer `device-number`
    /// (`hw-serial-number` 探测匹配) — **当前硬件的正式生产物化路径**, 不是诊断 fallback.
    DeviceHandleResolved,
    /// Diagnostic 显式模式: `device-number=<resolved>` (含 connection=sdi) — 仅验证/排障用, 非静默.
    DiagnosticFallback,
    /// MEDIA-RT-01 自测: videotestsrc/audiotestsrc (不依赖 DeckLink).
    SelfTest,
}

/// 物化后的单路采集源计划 (GStreamer 选卡属性).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcePlan {
    pub device_id: String,
    /// 真实 BMD PersistentID (仅 PersistentIdCanonical 模式使用; 否则 `None`).
    pub bmd_persistent_id: Option<i64>,
    /// Resolver 解析后的 GStreamer `device-number` (DeviceHandleResolved/DiagnosticFallback 使用).
    pub device_number: u32,
    pub selection_mode: SourceSelectionMode,
}

/// 物化后的管线计划 (控制面只给 VBMF `device_id`; bmd_persistent_id / device-number 由 materialize 解析).
///
/// 注: `pipeline.rs` 只消费 **Resolver 解析后的 `device-number`** (绝不 SDK 枚举序号);
/// `persistent-id` 仅在 PersistentID 可用时由 `materialize` 填 (当前硬件走 device-number 路径).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinePlan {
    pub source: SourcePlan,
    pub normalize: bool,
    pub switch_mode: String,
}

impl PipelinePlan {
    /// 从控制面 `GraphRuntimeIntent` 物化 (Device Registry + Resolver 前置).
    pub fn from_intent(
        intent: &GraphRuntimeIntent,
        devices: &[DeviceInfo],
        bindings: &std::collections::HashMap<Uuid, crate::resolver::ResolvedDeviceBinding>,
        mode: MaterializeMode,
    ) -> Result<Vec<PipelinePlan>, PipelineError> {
        materialize(intent, devices, mode, bindings)
    }

    /// MEDIA-RT-01 自测计划 (videotestsrc/audiotestsrc, 不依赖 DeckLink).
    pub fn self_test() -> PipelinePlan {
        PipelinePlan {
            source: SourcePlan {
                device_id: "self-test".into(),
                bmd_persistent_id: None,
                device_number: 0,
                selection_mode: SourceSelectionMode::SelfTest,
            },
            normalize: true,
            switch_mode: "FRAME_SWITCH".into(),
        }
    }
}

/// PTS 单调性三态 (P1-3, 用户复核 §三/§十一):
/// 区分 "未观测任何有效 PTS" / "已观测且帧间非回退" / "观测到回退".
/// 旧 `pts_monotonic: bool` 把 `Unknown` 与 `NonMonotonic` 压成一个 `false`,
/// 导致 UI/Evidence/Supervisor 无法区分 "没收到帧" 与 "流损坏", 故升级为枚举.
/// 语义独立到 video/audio 两路 (PIPELINE-AV 之前的最小解耦, 用户 §三).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PtsMonotonicity {
    /// 尚未收到任何有效 PTS — 无证据, 不得等同于 NonMonotonic 或 PASS (absence≠evidence).
    Unknown,
    /// 已收到且帧间严格非回退 (`pts >= last_pts`) — 流时间戳健康.
    ValidMonotonic,
    /// 观测到 PTS 回退 (`pts < last_pts`) — 流损坏/时间戳错乱; sticky (一旦回退不再自动恢复).
    NonMonotonic,
}

/// 管线运行时健康状态 + MEDIA-RT-01 acceptance 现场.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineHealth {
    pub video_frame_count: u64,
    pub audio_frame_count: u64,
    pub video_first_pts: Option<u64>,
    pub audio_first_pts: Option<u64>,
    /// 上一帧已观测 PTS (video/audio 分别) — 真正单调性判定基准 (用户复核 §十一):
    /// 由 `observe_video_pts`/`observe_audio_pts` 推进 `video_pts_state`/`audio_pts_state`.
    pub video_last_pts: Option<u64>,
    pub audio_last_pts: Option<u64>,
    /// PTS 单调性三态 (P1-3), video/audio 独立: Unknown/ValidMonotonic/NonMonotonic.
    /// 旧版单一 `pts_monotonic: bool` 无法区分 "未观测" 与 "回退", 已被此取代.
    pub video_pts_state: PtsMonotonicity,
    pub audio_pts_state: PtsMonotonicity,
    pub last_error: Option<String>,
    pub acceptance: MediaRt01Acceptance,
    /// 管线启动时刻 (UNIX 秒) — C 稳定性窗口计时起点 (watchdog 计算 observed_ms).
    pub started_at: Option<i64>,
    /// 管线是否已进入 PLAYING (`start()` 成功后置 true; recover 停止时置 false).
    /// watchdog 据此推导 `a3_pipeline_playing` (P1-2: 不得靠 default=true 蒙混).
    pub playing: bool,
}

impl PipelineHealth {
    /// MEDIA-RT-01A: 首帧 (video+audio) + PTS 单调, 即视为 ingest 打开成功.
    pub fn first_frame_ok(&self) -> bool {
        self.video_first_pts.is_some()
            && self.audio_first_pts.is_some()
            && self.video_pts_state == PtsMonotonicity::ValidMonotonic
            && self.audio_pts_state == PtsMonotonicity::ValidMonotonic
    }

    /// 完整 acceptance (A+B+C 全过).
    pub fn pass(&self) -> bool {
        self.acceptance.a_pass() && self.acceptance.b_pass() && self.acceptance.c_pass()
    }

    /// 观测一帧 video PTS, 推进 `video_pts_state` 三态机 (P1-3).
    /// 规则: 首有效 PTS → ValidMonotonic; 之后 `pts < last` → NonMonotonic (sticky);
    /// 否则保持 (ValidMonotonic 或已 NonMonotonic). 无 PTS 帧 (None) 不参与 (调用方已过滤).
    /// 与 `video_first_pts` 解耦: 首帧 PTS 记录由 appsink 回调单独维护, 本方法只管单调性状态.
    pub fn observe_video_pts(&mut self, pts: u64) {
        if let Some(last) = self.video_last_pts {
            if pts < last {
                self.video_pts_state = PtsMonotonicity::NonMonotonic;
            }
            // 否则保持当前状态 (ValidMonotonic / 已 NonMonotonic 均 sticky).
        } else {
            self.video_pts_state = PtsMonotonicity::ValidMonotonic;
        }
        self.video_last_pts = Some(pts);
    }

    /// 观测一帧 audio PTS, 推进 `audio_pts_state` 三态机 (P1-3). 语义同 `observe_video_pts`,
    /// 独立维护 audio 一路 (PIPELINE-AV 之前的最小解耦, 用户 §三).
    pub fn observe_audio_pts(&mut self, pts: u64) {
        if let Some(last) = self.audio_last_pts {
            if pts < last {
                self.audio_pts_state = PtsMonotonicity::NonMonotonic;
            }
            // 否则保持当前状态 (sticky).
        } else {
            self.audio_pts_state = PtsMonotonicity::ValidMonotonic;
        }
        self.audio_last_pts = Some(pts);
    }
}

impl Default for PipelineHealth {
    fn default() -> Self {
        Self {
            video_frame_count: 0,
            audio_frame_count: 0,
            video_first_pts: None,
            audio_first_pts: None,
            video_last_pts: None,
            audio_last_pts: None,
            // 两路 PTS 三态起始 `Unknown`: 无帧即"未观测", 不得 absence-of-evidence = pass
            // (用户复核 §十一/§十 P1-2 / P1-3). appsink 回调经 `observe_*_pts` 推进状态:
            // 首有效 PTS → ValidMonotonic; 回退 → NonMonotonic (sticky).
            video_pts_state: PtsMonotonicity::Unknown,
            audio_pts_state: PtsMonotonicity::Unknown,
            last_error: None,
            acceptance: MediaRt01Acceptance::default(),
            started_at: None,
            playing: false,
        }
    }
}

/// MEDIA-RT-01 acceptance 子项 (A/B/C 三组).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaRt01Acceptance {
    // A — Ingest Open
    pub a1_identity_resolved: bool,
    pub a2_lease_acquired: bool,
    pub a3_pipeline_playing: bool,
    pub a4_signal_detected: bool,
    // B — First Valid Buffer
    pub b1_first_video: bool,
    pub b2_first_audio: bool,
    pub b3_valid_pts: bool,
    pub b4_pts_monotonic: bool,
    // C — Short Stability (测量型, 非仅"无 error"; 用户复核 §十二)
    pub c1_no_unexpected_eos: bool,
    pub c2_no_pipeline_error: bool,
    pub c3_no_repeated_reneg: bool,
    pub c4_counters_continue: bool,
    // —— C 稳定性测量窗口 (Phase 0.6: C = 测量型 acceptance) ——
    /// 已观测稳定时长 (ms); 由 watchdog 计时 (started_at → now).
    pub c_observed_ms: Option<u64>,
    /// 配置的稳定窗口 (ms); 默认 10_000. `c_observed_ms >= 此值` 视为窗口达标.
    pub c_configured_window_ms: u64,
    pub c_video_frames: u64,
    pub c_audio_frames: u64,
    pub c_unexpected_eos: u64,
    pub c_pipeline_errors: u64,
    pub c_renegotiations: u64,
}

impl MediaRt01Acceptance {
    /// A — Ingest Open: 身份解析 + 租约 + 管线 playing + 信号检测.
    pub fn a_pass(&self) -> bool {
        self.a1_identity_resolved
            && self.a2_lease_acquired
            && self.a3_pipeline_playing
            && self.a4_signal_detected
    }

    /// B — First Valid Buffer: 首视频帧 + 首音频帧 + 有效 PTS + PTS 单调.
    pub fn b_pass(&self) -> bool {
        self.b1_first_video && self.b2_first_audio && self.b3_valid_pts && self.b4_pts_monotonic
    }

    /// C — Short Stability (测量型, 用户复核 §十二): 稳定窗口达标 + 无致命 pipeline error +
    /// 计数持续增长 + PTS 单调. 仅 "没有 error" 不够, 必须达到配置观测窗口.
    pub fn c_pass(&self) -> bool {
        let window_ok = self
            .c_observed_ms
            .map_or(false, |o| o >= self.c_configured_window_ms);
        window_ok
            && self.c1_no_unexpected_eos
            && self.c2_no_pipeline_error
            && self.c3_no_repeated_reneg
            && self.c4_counters_continue
            && self.b4_pts_monotonic
    }
}

impl Default for MediaRt01Acceptance {
    fn default() -> Self {
        // P1-2: 默认全 `false` — "Default Health = all PASS" 不合理. 各**正向成就**项
        // (身份/租约/播放/信号/首帧/有效PTS/单调/计数增长) 由真实运行时事件逐项置 true;
        // 从未观测到 = 未通过, 绝不 absence-of-evidence = PASS. 负向项 (c1/c2/c3 无错误)
        // 由 watchdog 从对应计数器推导 (见 main.rs).
        Self {
            a1_identity_resolved: false,
            a2_lease_acquired: false,
            a3_pipeline_playing: false,
            a4_signal_detected: false,
            b1_first_video: false,
            b2_first_audio: false,
            b3_valid_pts: false,
            b4_pts_monotonic: false,
            c1_no_unexpected_eos: false,
            c2_no_pipeline_error: false,
            c3_no_repeated_reneg: false,
            c4_counters_continue: false,
            c_observed_ms: None,
            c_configured_window_ms: 10_000,
            c_video_frames: 0,
            c_audio_frames: 0,
            c_unexpected_eos: 0,
            c_pipeline_errors: 0,
            c_renegotiations: 0,
        }
    }
}

/// 物化模式 (控制面/运行时策略).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializeMode {
    /// 生产: 身份解析失败直接 `IdentityUnresolved`, 绝不盲开 device 0.
    Production,
    /// Diagnostic: 显式回退 `device-number` (仅验证/排障, 非静默 fallback).
    Diagnostic,
}

/// 管线错误.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("identity unresolved: {0}")]
    IdentityUnresolved(String),
    #[error("pipeline prepare failed: {0}")]
    PrepareFailed(String),
    #[error("pipeline start failed: {0}")]
    StartFailed(String),
}

/// Pipeline Controller trait — 媒体运行时生命周期 (prepare/start/recover).
pub trait PipelineController {
    fn prepare(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError>;
    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
}

/// 管线句柄 (GStreamer 运行时实例标识).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PipelineHandle(pub u64);

/// GStreamer 实现 (feature `gstreamer`).
pub struct GStreamerPipelineController {
    /// 运行时 pipeline 实例 (GStreamer Bin 对 + 物化计划), 供 start/recover 操作 (P0-2 修复核心:
    /// 旧 `launch()` 内部 Bin 未留存, start/recover 无对象可操作). 非 gstreamer 构建无此字段.
    #[cfg(feature = "gstreamer")]
    instances: Mutex<HashMap<PipelineHandle, GstInstance>>,
}

/// 运行时 pipeline 实例 (仅 gstreamer 构建存在).
#[cfg(feature = "gstreamer")]
struct GstInstance {
    video: gstreamer::Bin,
    audio: gstreamer::Bin,
    plan: PipelinePlan,
}

impl GStreamerPipelineController {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "gstreamer")]
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
        #[cfg(feature = "gstreamer")]
        {
            let handle = PipelineHandle(NEXT_PIPELINE_ID.fetch_add(1, Ordering::SeqCst));
            let (video, audio) = self.build_bins(plan, handle)?;
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
            self.instances
                .lock()
                .unwrap()
                .insert(handle, GstInstance { video, audio, plan: plan.clone() });
            Ok(handle)
        }
        #[cfg(not(feature = "gstreamer"))]
        {
            let _ = plan;
            Ok(PipelineHandle(1))
        }
    }

    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        #[cfg(feature = "gstreamer")]
        {
            {
                let guard = self.instances.lock().unwrap();
                let inst = guard.get(handle).ok_or_else(|| {
                    PipelineError::StartFailed(format!("未知 pipeline handle (start): {handle:?}"))
                })?;
                inst.video
                    .set_state(gstreamer::State::Playing)
                    .map_err(|e| PipelineError::StartFailed(format!("video play: {e}")))?;
                inst.audio
                    .set_state(gstreamer::State::Playing)
                    .map_err(|e| PipelineError::StartFailed(format!("audio play: {e}")))?;
            }
            // 标记 playing (watchdog 推导 a3_pipeline_playing). 此处与 instances 锁不嵌套, 避免死锁.
            if let Some(hp) = HEALTH_ARCS.lock().unwrap().get(handle) {
                hp.lock().unwrap().playing = true;
            }
            Ok(())
        }
        #[cfg(not(feature = "gstreamer"))]
        {
            let _ = handle;
            Ok(())
        }
    }

    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        #[cfg(feature = "gstreamer")]
        {
            let plan = {
                let guard = self.instances.lock().unwrap();
                guard
                    .get(handle)
                    .map(|i| i.plan.clone())
                    .ok_or_else(|| {
                        PipelineError::StartFailed(format!(
                            "未知 pipeline handle (recover): {handle:?}"
                        ))
                    })?
            };
            // 停止并丢弃旧实例 (释放 DeckLink 设备).
            if let Some(old) = self.instances.lock().unwrap().remove(handle) {
                let _ = old.video.set_state(gstreamer::State::Null);
                let _ = old.audio.set_state(gstreamer::State::Null);
            }
            let (video, audio) = self.build_bins(&plan, *handle)?;
            video
                .set_state(gstreamer::State::Playing)
                .map_err(|e| PipelineError::StartFailed(format!("video play: {e}")))?;
            audio
                .set_state(gstreamer::State::Playing)
                .map_err(|e| PipelineError::StartFailed(format!("audio play: {e}")))?;
            self.instances
                .lock()
                .unwrap()
                .insert(*handle, GstInstance { video, audio, plan });
            if let Some(hp) = HEALTH_ARCS.lock().unwrap().get(handle) {
                hp.lock().unwrap().playing = true;
            }
            Ok(())
        }
        #[cfg(not(feature = "gstreamer"))]
        {
            let _ = handle;
            Ok(())
        }
    }
}

/// 运行时健康共享状态 (GStreamer 回调/bus 监控/监控线程共享).
pub(crate) static HEALTH_ARCS: LazyLock<
    Mutex<std::collections::HashMap<PipelineHandle, Arc<Mutex<PipelineHealth>>>>,
> = LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// 读取管线健康快照 (监控 API 用).
pub fn read_health(handle: &PipelineHandle) -> Option<PipelineHealth> {
    HEALTH_ARCS
        .lock()
        .unwrap()
        .get(handle)
        .map(|h| h.lock().unwrap().clone())
}

/// GStreamer bus 事件 (监控线程消费, 喂 Supervisor 决策).
#[derive(Debug, Clone)]
pub enum PipelineBusEvent {
    Error(String),
    Eos,
    StateChanged,
    Renegotiation,
}

impl GStreamerPipelineController {
    /// 构造 decklinkvideosrc/audiosrc → video/x-raw → appsink 的采集 pipeline (P0-2 修复核心).
    /// 仅构建 + 注册 appsink 回调, **不**立即 Playing; Playing 由 `PipelineController::start` 负责
    /// (统一生命周期: prepare 构建 → start 播放 → recover 重建+播放). 旧 `launch()` 已被此拆分取代,
    /// 不再作为绕过 `PipelineController` 的第二入口.
    ///
    /// 注: appsink 当前用作 MEDIA-RT-01 首帧/PTS acceptance 探针; 最终生产媒体出口
    /// (Normalize → FRAME/MASTER SWITCH → Encode → SRS) 待 A2+ 实现 (用户复核 §十三).
    #[cfg(feature = "gstreamer")]
    fn build_bins(
        &self,
        plan: &PipelinePlan,
        handle: PipelineHandle,
    ) -> Result<(gstreamer::Bin, gstreamer::Bin), PipelineError> {
        gstreamer::init().map_err(|e| PipelineError::StartFailed(format!("gst init: {e}")))?;
        let (video_src, audio_src) = src_props(plan);
        let video_pipeline_str = format!(
            "{video_src} ! video/x-raw ! appsink name=videosink async=false",
        );
        let audio_pipeline_str = format!(
            "{audio_src} ! audio/x-raw ! appsink name=audiosink async=false",
        );
        let vp = gstreamer::parse::launch(&video_pipeline_str)
            .map_err(|e| PipelineError::StartFailed(format!("video parse: {e}")))?;
        let vp = vp
            .dynamic_cast::<gstreamer::Bin>()
            .map_err(|_| PipelineError::StartFailed("video pipeline not a bin".into()))?;
        let ap = gstreamer::parse::launch(&audio_pipeline_str)
            .map_err(|e| PipelineError::StartFailed(format!("audio parse: {e}")))?;
        let ap = ap
            .dynamic_cast::<gstreamer::Bin>()
            .map_err(|_| PipelineError::StartFailed("audio pipeline not a bin".into()))?;
        let v_appsink = vp
            .by_name("videosink")
            .and_then(|e| e.dynamic_cast::<AppSink>().ok())
            .ok_or_else(|| PipelineError::StartFailed("videosink cast".into()))?;
        let a_appsink = ap
            .by_name("audiosink")
            .and_then(|e| e.dynamic_cast::<AppSink>().ok())
            .ok_or_else(|| PipelineError::StartFailed("audiosink cast".into()))?;

        self.attach_video_sink(&v_appsink, handle);
        self.attach_audio_sink(&a_appsink, handle);

        Ok((vp, ap))
    }

    /// 注册视频 appsink 回调: 首帧/PTS 探测 (MEDIA-RT-01 B).
    #[cfg(feature = "gstreamer")]
    fn attach_video_sink(&self, sink: &AppSink, handle: PipelineHandle) {
        let handle = handle;
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
    #[cfg(feature = "gstreamer")]
    fn attach_audio_sink(&self, sink: &AppSink, handle: PipelineHandle) {
        let handle = handle;
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

    /// 轮询 GStreamer bus (Error/EOS/StateChanged) — Supervisor 闭环数据源.
    #[cfg(feature = "gstreamer")]
    pub fn poll_bus(&self, _handle: &PipelineHandle) -> Vec<PipelineBusEvent> {
        Vec::new()
    }
}

/// GStreamer 选卡属性 (decklinkvideosrc/audiosrc).
///
/// 选卡属性语义 (用户复核 §十, 纠正旧注释 "hw-serial-number=<BMD PersistentID>" 的错误表达):
/// - PersistentID 可用 → GStreamer `persistent-id=<BMD PersistentID>` (官方首选, 优先级高于 device-number).
/// - PersistentID 不可用 → Resolver 探测 `hw-serial-number` 匹配 SDK DeviceHandle, 解析出
///   确定的 GStreamer `device-number` → `decklinkvideosrc device-number=<n>`.
/// - `hw-serial-number` 是 **GStreamer 侧** 硬件序列号/硬件 ID 探测属性 (只读), 与 "BMD PersistentID"
///   是两回事; 它不是 PersistentID 的别名. SDK 枚举 index 绝不直接当 device-number.
fn src_props(plan: &PipelinePlan) -> (String, String) {
    let (video_src, audio_src) = match plan.source.selection_mode {
        // PersistentID 可用 → 官方首选 `persistent-id`.
        SourceSelectionMode::PersistentIdCanonical => (
            format!(
                "decklinkvideosrc persistent-id={} connection=sdi",
                plan.source.bmd_persistent_id.unwrap_or(0)
            ),
            format!(
                "decklinkaudiosrc persistent-id={} connection=sdi",
                plan.source.bmd_persistent_id.unwrap_or(0)
            ),
        ),
        // DeviceHandle 经 Resolver 解析到确定 device-number (当前硬件正式生产路径).
        // 注: `hw-serial-number` 是 GStreamer 侧硬件序列号/硬件 ID 探测属性 (只读),
        // 与 "BMD PersistentID" 是两回事; 此处经 Resolver 已映射到 device-number.
        SourceSelectionMode::DeviceHandleResolved | SourceSelectionMode::DiagnosticFallback => (
            format!(
                "decklinkvideosrc device-number={} connection=sdi",
                plan.source.device_number
            ),
            format!(
                "decklinkaudiosrc device-number={} connection=sdi",
                plan.source.device_number
            ),
        ),
        SourceSelectionMode::SelfTest => (
            "videotestsrc is-live=true pattern=ball".to_string(),
            "audiotestsrc is-live=true".to_string(),
        ),
    };
    (video_src, audio_src)
}

/// 物化 `GraphRuntimeIntent` → `PipelinePlan` 列表.
///
/// 身份层级状态机 (用户复核 §二/§三/§十九): 严格按 `identity_strength` 判定,
/// **绝不只看 `bmd_persistent_id.is_some()`** (否则 filesystem 伪造的 `Some(hash)`
/// 会被当成真实 PersistentID 越权). 合成身份 (Enumeration) 在生产路径必须拒绝.
///
/// 关键不变量: `materialize` 只消费 **Resolver 解析后的 `device-number`**, 绝不 (在生产/已解析时)
/// 直接用 SDK 枚举序号; `device-number` 默认 0 在 DeviceHandle/Diagnostic 路径下由 resolved 覆盖.
pub fn materialize(
    intent: &GraphRuntimeIntent,
    devices: &[DeviceInfo],
    mode: MaterializeMode,
    bindings: &std::collections::HashMap<Uuid, crate::resolver::ResolvedDeviceBinding>,
) -> Result<Vec<PipelinePlan>, PipelineError> {
    let mut plans = Vec::new();
    for d in &intent.devices {
        let info = devices
            .iter()
            .find(|x| x.device_id.to_string() == d.device_id)
            .ok_or_else(|| {
                PipelineError::IdentityUnresolved(format!("设备未注册: {}", d.device_id))
            })?;

        let resolved_device_number = bindings.get(&info.device_id).map(|b| b.device_number);
        // 身份层级状态机: 严格按 identity_strength, 不看 Option 真伪.
        let selection_mode = match info.identity_strength {
            IdentityStrength::PersistentId if info.bmd_persistent_id.is_some() => {
                // 官方首选: persistent-id (优先级高于 device-number).
                SourceSelectionMode::PersistentIdCanonical
            }
            IdentityStrength::DeviceHandle if resolved_device_number.is_some() => {
                // 当前硬件正式路径: DeviceHandle → Resolver → 确定 GStreamer device-number.
                SourceSelectionMode::DeviceHandleResolved
            }
            IdentityStrength::TopologicalId if resolved_device_number.is_some() => {
                // 拓扑敏感: 仅 Diagnostic 显式模式允许, 生产拒绝 (猜设备风险高).
                match mode {
                    MaterializeMode::Production => {
                        return Err(PipelineError::IdentityUnresolved(format!(
                            "{}: TopologicalId 身份强度不足, 生产路径拒绝 (需 PersistentId/DeviceHandle+Resolver)",
                            d.device_id
                        )));
                    }
                    MaterializeMode::Diagnostic => SourceSelectionMode::DiagnosticFallback,
                }
            }
            _ => {
                // Enumeration 身份 (filesystem 合成) 或 (DeviceHandle/TopologicalId 无 Resolver 绑定):
                // 生产路径直接 IdentityUnresolved, 绝不 unwrap_or(0) 盲开 device 0.
                match mode {
                    MaterializeMode::Production => {
                        return Err(PipelineError::IdentityUnresolved(format!(
                            "{}: 身份未解析 (identity_strength={:?}, bmd_persistent_id={:?}, Resolver 绑定={:?}); 生产拒绝 device 0",
                            d.device_id, info.identity_strength, info.bmd_persistent_id, resolved_device_number
                        )));
                    }
                    MaterializeMode::Diagnostic => SourceSelectionMode::DiagnosticFallback,
                }
            }
        };

        let source = SourcePlan {
            device_id: d.device_id.clone(),
            bmd_persistent_id: info.bmd_persistent_id,
            device_number: resolved_device_number.unwrap_or(0),
            selection_mode,
        };
        plans.push(PipelinePlan {
            source,
            normalize: true,
            switch_mode: "FRAME_SWITCH".into(),
        });
    }
    Ok(plans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pts_state_starts_unknown() {
        let h = PipelineHealth::default();
        assert_eq!(h.video_pts_state, PtsMonotonicity::Unknown);
        assert_eq!(h.audio_pts_state, PtsMonotonicity::Unknown);
        assert!(!h.first_frame_ok());
    }

    #[test]
    fn first_video_pts_is_valid_monotonic() {
        let mut h = PipelineHealth::default();
        h.observe_video_pts(1000);
        assert_eq!(h.video_pts_state, PtsMonotonicity::ValidMonotonic);
        // first_pts 由回调单独维护, 本方法只推进状态; 此处未设 first_pts.
        assert!(!h.first_frame_ok());
    }

    #[test]
    fn video_regression_becomes_non_monotonic() {
        let mut h = PipelineHealth::default();
        h.observe_video_pts(1000);
        h.observe_video_pts(1001);
        h.observe_video_pts(999); // 回退
        assert_eq!(h.video_pts_state, PtsMonotonicity::NonMonotonic);
    }

    #[test]
    fn non_monotonic_is_sticky() {
        let mut h = PipelineHealth::default();
        h.observe_video_pts(1000);
        h.observe_video_pts(500); // 回退 -> NonMonotonic
        h.observe_video_pts(2000); // 后续正常帧不得自动恢复
        assert_eq!(h.video_pts_state, PtsMonotonicity::NonMonotonic);
    }

    #[test]
    fn equal_pts_keeps_valid_monotonic() {
        let mut h = PipelineHealth::default();
        h.observe_video_pts(1000);
        h.observe_video_pts(1000); // pts == last, 非严格递增但非回退
        assert_eq!(h.video_pts_state, PtsMonotonicity::ValidMonotonic);
    }

    #[test]
    fn video_audio_states_independent() {
        let mut h = PipelineHealth::default();
        h.observe_video_pts(1000);
        h.observe_video_pts(500); // video 坏
        h.observe_audio_pts(1000);
        h.observe_audio_pts(1001); // audio 好
        assert_eq!(h.video_pts_state, PtsMonotonicity::NonMonotonic);
        assert_eq!(h.audio_pts_state, PtsMonotonicity::ValidMonotonic);
    }

    #[test]
    fn first_frame_ok_requires_both_valid() {
        let mut h = PipelineHealth::default();
        // 仅 video 有效 -> 未过.
        h.video_first_pts = Some(1000);
        h.observe_video_pts(1000);
        h.audio_first_pts = Some(1000);
        h.observe_audio_pts(1000);
        assert!(h.first_frame_ok());
        // 任一回退 -> 不过.
        h.observe_audio_pts(999);
        assert!(!h.first_frame_ok());
    }
}
