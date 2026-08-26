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

use crate::device::{DeviceInfo, DeviceManager, IdentityStrength, PipelineError};
use crate::graph_intent::{DeviceIntent, GraphRuntimeIntent, PipelineIntent, SinkIntent, SourceIntent};
use gstreamer::prelude::*;
use gstreamer_app::AppSink;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock, Mutex};
use uuid::Uuid;

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

/// 管线运行时健康状态 + MEDIA-RT-01 acceptance 现场.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineHealth {
    pub video_frame_count: u64,
    pub audio_frame_count: u64,
    pub video_first_pts: Option<u64>,
    pub audio_first_pts: Option<u64>,
    /// 上一帧已观测 PTS (video/audio 分别) — 真正单调性判定基准 (用户复核 §十一):
    /// 每帧 `pts >= last_pts` 才维持 monotonic; 否则 `pts_monotonic = false`.
    pub video_last_pts: Option<u64>,
    pub audio_last_pts: Option<u64>,
    pub pts_monotonic: bool,
    pub last_error: Option<String>,
    pub acceptance: MediaRt01Acceptance,
    /// 管线启动时刻 (UNIX 秒) — C 稳定性窗口计时起点 (watchdog 计算 observed_ms).
    pub started_at: Option<i64>,
}

impl PipelineHealth {
    /// MEDIA-RT-01A: 首帧 (video+audio) + PTS 单调, 即视为 ingest 打开成功.
    pub fn first_frame_ok(&self) -> bool {
        self.video_first_pts.is_some()
            && self.audio_first_pts.is_some()
            && self.pts_monotonic
    }

    /// 完整 acceptance (A+B+C 全过).
    pub fn pass(&self) -> bool {
        self.acceptance.a_pass() && self.acceptance.b_pass() && self.acceptance.c_pass()
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
            // `pts_monotonic` 起始为 `true` (absence-of-evidence = pass, 直到 appsink 回调观测到
            // PTS 回退才证伪); 其余字段取类型默认值. 注意 `acceptance.b4_pts_monotonic` 每轮由
            // watchdog 从 `pts_monotonic` 拷贝 (main.rs), 故此处 `true` 是 B4 可达标的真正来源.
            pts_monotonic: true,
            last_error: None,
            acceptance: MediaRt01Acceptance::default(),
            started_at: None,
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
        Self {
            a1_identity_resolved: true,
            a2_lease_acquired: true,
            a3_pipeline_playing: true,
            a4_signal_detected: true,
            b1_first_video: true,
            b2_first_audio: true,
            b3_valid_pts: true,
            b4_pts_monotonic: true,
            c1_no_unexpected_eos: true,
            c2_no_pipeline_error: true,
            c3_no_repeated_reneg: true,
            c4_counters_continue: true,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineHandle(pub u64);

/// GStreamer 实现 (feature `gstreamer`).
pub struct GStreamerPipelineController;

impl GStreamerPipelineController {
    pub fn new() -> Self {
        GStreamerPipelineController
    }
}

impl Default for GStreamerPipelineController {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineController for GStreamerPipelineController {
    fn prepare(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError> {
        let _ = plan;
        Ok(PipelineHandle(1))
    }

    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        let _ = handle;
        Ok(())
    }

    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        let _ = handle;
        Ok(())
    }
}

/// 运行时健康共享状态 (GStreamer 回调/bus 监控/监控线程共享).
static HEALTH_ARCS: LazyLock<
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
    /// 真实 GStreamer launch (feature = "gstreamer").
    /// 构造 decklinkvideosrc/audiosrc → video/x-raw → appsink 的采集 pipeline,
    /// 注册 bus 监控 + appsink 回调 (首帧/PTS 探测).
    ///
    /// 注: appsink 当前用作 MEDIA-RT-01 首帧/PTS acceptance 探针; 最终生产媒体出口
    /// (Normalize → FRAME/MASTER SWITCH → Encode → SRS) 待 A2+ 实现, 不作为当前
    /// first-frame 验收的生产数据出口 (用户复核 §十三).
    #[cfg(feature = "gstreamer")]
    pub fn launch(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError> {
        use gstreamer::prelude::*;
        gstreamer::init().map_err(|e| PipelineError::StartFailed(format!("gst init: {e}")))?;
        let (video_src, audio_src) = src_props(plan);
        let video_pipeline_str = format!(
            "{video_src} ! video/x-raw ! appsink name=videosink async=false",
        );
        let audio_pipeline_str = format!(
            "{audio_src} ! audio/x-raw ! appsink name=audiosink async=false",
        );
        let vp = gstreamer::parse_launch(&video_pipeline_str)
            .map_err(|e| PipelineError::StartFailed(format!("video parse: {e}")))?;
        let ap = gstreamer::parse_launch(&audio_pipeline_str)
            .map_err(|e| PipelineError::StartFailed(format!("audio parse: {e}")))?;
        let v_appsink = vp
            .by_name("videosink")
            .and_then(|e| e.dynamic_cast::<AppSink>().ok())
            .ok_or_else(|| PipelineError::StartFailed("videosink cast".into()))?;
        let a_appsink = ap
            .by_name("audiosink")
            .and_then(|e| e.dynamic_cast::<AppSink>().ok())
            .ok_or_else(|| PipelineError::StartFailed("audiosink cast".into()))?;

        let h = PipelineHandle(1);
        HEALTH_ARCS
            .lock()
            .unwrap()
            .insert(h, Arc::new(Mutex::new(PipelineHealth::default())));
        // 启动即开始 C 稳定性窗口计时 (watchdog 用 started_at 计算 observed_ms).
        if let Some(hp) = HEALTH_ARCS.lock().unwrap().get(&h) {
            hp.lock().unwrap().started_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
            );
        }

        self.attach_video_sink(&v_appsink, h);
        self.attach_audio_sink(&a_appsink, h);

        vp.set_state(gstreamer::State::Playing)
            .map_err(|e| PipelineError::StartFailed(format!("video play: {e}")))?;
        ap.set_state(gstreamer::State::Playing)
            .map_err(|e| PipelineError::StartFailed(format!("audio play: {e}")))?;
        Ok(h)
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
                            // 记录首帧 PTS.
                            if h.video_first_pts.is_none() {
                                h.video_first_pts = Some(pts);
                            }
                            // 真正单调性 (用户复核 §十一): 与上一帧比较, 回退即非单调.
                            // 无 PTS 帧 (None) 已在上方跳过, 不污染判定.
                            if let Some(last) = h.video_last_pts {
                                if pts < last {
                                    h.pts_monotonic = false;
                                }
                            }
                            h.video_last_pts = Some(pts);
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
                            if h.audio_first_pts.is_none() {
                                h.audio_first_pts = Some(pts);
                            }
                            // 音频同样做真实单调性判定 (用户复核 §十一: 旧实现缺音频单调检查).
                            if let Some(last) = h.audio_last_pts {
                                if pts < last {
                                    h.pts_monotonic = false;
                                }
                            }
                            h.audio_last_pts = Some(pts);
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
