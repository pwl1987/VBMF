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
use crate::port::{ConnectorType, PortDirection};
#[cfg(feature = "gstreamer")]
use gstreamer::prelude::*;
#[cfg(feature = "gstreamer")]
use gstreamer_app::AppSink;
use serde::{Deserialize, Serialize};
#[cfg(feature = "gstreamer")]
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "gstreamer")]
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, LazyLock, Mutex};
use uuid::Uuid;

/// 全局唯一 pipeline 句柄计数器 (P1-1: 多 controller 共用全局 `HEALTH_ARCS` 时避免 handle 碰撞).
static NEXT_PIPELINE_ID: AtomicU64 = AtomicU64::new(1);

/// Bus 事件因 channel 满被丢弃的累计计数 (P1 §十三: 溢出策略).
/// 关键 ERROR/EOS 事件溢出时仍存 sticky (`LAST_FATAL_BUS_EVENT`), 此处仅计数, 不静默丢失语义.
pub static DROPPED_BUS_EVENTS: AtomicU64 = AtomicU64::new(0);
/// 最近一次致命 Bus 事件 (Error/EOS) 的 sticky 副本 — 即便 channel 满也不丢失,
/// watchdog / 运维可通过 `last_fatal_bus_event()` 读取, 避免关键错误被溢出吞掉.
pub static LAST_FATAL_BUS_EVENT: Mutex<Option<PipelineBusEvent>> = Mutex::new(None);

/// Bus channel 累计丢弃事件数 (P1 §十三 溢出 metric), 由 `/health` 暴露.
pub fn dropped_bus_events() -> u64 {
    DROPPED_BUS_EVENTS.load(Ordering::SeqCst)
}

/// 最近一次 sticky 致命 Bus 事件 (Error/EOS); channel 溢出也不会丢失. `None` 表示尚未发生过.
pub fn last_fatal_bus_event() -> Option<PipelineBusEvent> {
    LAST_FATAL_BUS_EVENT.lock().unwrap().clone()
}

/// ClockLost 事件累计计数 (P1-4 最低策略: ClockLost = degraded, 不自动重启; 完整 Clock Recovery 属 V0.3/P2).
/// 即便 channel 不丢, 仍独立计数以便健康面暴露 "AV 同步降级" 信号.
pub static CLOCK_LOST_EVENTS: AtomicU64 = AtomicU64::new(0);

/// ClockLost 累计计数 (P1-4 降级 metric), 由 `/health` 暴露.
pub fn clock_lost_events() -> u64 {
    CLOCK_LOST_EVENTS.load(Ordering::SeqCst)
}

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
    /// Diagnostic 显式模式: `device-number=<resolved>` (videosrc 含 connection=sdi; audiosrc 无此属性) — 仅验证/排障用, 非静默.
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
    /// 物理连接器类型 (由 PortRegistry 经 Manifest 声明 + 运行时探测推导得到). 决定 GStreamer
    /// `decklinkvideosrc` 的 `connection=<...>` 属性; `None`/无对应枚举的连接器 → 不显式指定, 由插件默认探测.
    /// **绝不硬编码** 成 `sdi` (见 `src_props`).
    pub connector: Option<ConnectorType>,
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
        registry: Option<&crate::port::PortRegistry>,
    ) -> Result<Vec<PipelinePlan>, PipelineError> {
        materialize(intent, devices, mode, bindings, registry)
    }

    /// MEDIA-RT-01 自测计划 (videotestsrc/audiotestsrc, 不依赖 DeckLink).
    pub fn self_test() -> PipelinePlan {
        PipelinePlan {
            source: SourcePlan {
                device_id: "self-test".into(),
                bmd_persistent_id: None,
                device_number: 0,
                connector: None,
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
    /// 累计接获的 GStreamer Bus 事件数 (P1-4: 证明 Bus watch→channel→poll_bus 链路真实生效;
    /// 运维可据此判断事件率, Supervisor 不依赖此计数做决策).
    pub bus_event_count: u64,
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
            bus_event_count: 0,
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
            .is_some_and(|o| o >= self.c_configured_window_ms);
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
///
/// P1-4: 由 **单个** `GstPipeline`(内含 video+audio 两路 branch) 取代原先分离的两个 `Bin`
/// (PIPELINE-AV-01 前置: 统一 Bus + 单一 Clock domain). Bus watch 运行在专用 GLib
/// MainContext 线程 (`thread` + `stop_flag`), 经 bounded mpsc (`bus_rx`) 把事件交给 `poll_bus`.
#[cfg(feature = "gstreamer")]
struct GstInstance {
    pipeline: gstreamer::Pipeline,
    plan: PipelinePlan,
    bus_rx: Receiver<PipelineBusEvent>,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(feature = "gstreamer")]
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
        #[cfg(not(feature = "gstreamer"))]
        {
            let _ = plan;
            Ok(PipelineHandle(1))
        }
    }

    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        #[cfg(feature = "gstreamer")]
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

/// GStreamer Bus 事件严重度 (喂 Supervisor 决策时判优先级).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BusSeverity {
    /// 致命: pipeline error / 解码失败 → Supervisor 必响应.
    Error,
    /// 告警: Warning / ClockLost 等可恢复异常.
    Warning,
    /// 信息: StateChanged / Eos / AsyncDone 等正常生命周期事件.
    Info,
}

/// GStreamer Bus 事件类型 (P1-4: 覆盖 Error/EOS/StateChanged/Warning/ClockLost, 真实接线到 Supervisor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineBusEventKind {
    Error,
    Eos,
    StateChanged,
    Warning,
    /// ClockLost 等 AV 同步相关 (PIPELINE-AV 后续消费; 当前仅记录).
    ClockLost,
}

/// Bus 事件 → Supervisor 恢复策略 (P1-4 最低策略映射, 用户复核 §十二):
/// - `Error` / `Eos`     : 致命 → 触发 Supervisor `report_failure` (重启/升级).
/// - `ClockLost`         : 降级 (degraded), **不**自动重启 (完整 Clock Recovery 属 V0.3/P2); 仅计数 + 健康降级.
/// - `Warning`           : 告警, 记录 + 日志, 不重启.
/// - `StateChanged`      : 信息, 仅生命周期日志.
pub fn bus_event_recovery_policy(kind: PipelineBusEventKind) -> &'static str {
    match kind {
        PipelineBusEventKind::Error | PipelineBusEventKind::Eos => "restart",
        PipelineBusEventKind::ClockLost => "degraded",
        PipelineBusEventKind::Warning => "warn",
        PipelineBusEventKind::StateChanged => "info",
    }
}

/// GStreamer Bus 事件 (监控线程消费, 喂 Supervisor 决策).
///
/// P1-4 改造 (用户复核 §七): 之前只有 `Error(String)` 等薄枚举, 多 pipeline 后无法诊断
/// "哪一路出的错". 现结构化携带 `handle`(哪条管线) / `source`(哪个 element 发出) /
/// `timestamp`(观测墙钟 ms) / `detail`(错误串/状态转移) / `severity`. 事件经专门 GLib
/// MainContext 线程的 Bus watch 投递进 bounded mpsc channel, `poll_bus` 非阻塞 drain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineBusEvent {
    pub handle: PipelineHandle,
    pub kind: PipelineBusEventKind,
    pub source: String,
    pub timestamp: i64,
    pub detail: String,
    pub severity: BusSeverity,
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
    #[cfg(feature = "gstreamer")]
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
    #[cfg(feature = "gstreamer")]
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
    #[cfg(feature = "gstreamer")]
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
    #[cfg(feature = "gstreamer")]
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
    #[cfg(feature = "gstreamer")]
    pub fn poll_bus(&self, handle: &PipelineHandle) -> Vec<PipelineBusEvent> {
        let guard = self.instances.lock().unwrap();
        match guard.get(handle) {
            Some(inst) => inst.bus_rx.try_iter().collect(),
            None => Vec::new(),
        }
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
    // 连接类型 → GStreamer `connection=` 属性. 仅 decklinkvideosrc 需要; decklinkaudiosrc 无此属性
    // (音频内嵌于视频 SDI/HDMI 流, 跟随视频连接). `None` 或无对应枚举的连接器 → 不显式指定, 由插件默认
    // (auto) 探测, 绝不硬编码 `connection=sdi`.
    let connection = match plan.source.connector {
        Some(ConnectorType::Sdi) => " connection=sdi",
        Some(ConnectorType::Hdmi) => " connection=hdmi",
        // GStreamer `decklinkvideosrc` 连接枚举 nick 为 "optical-sdi" (对应 BMD bmdVideoConnectionOpticalSDI), 绝非 "optical".
        Some(ConnectorType::Optical) => " connection=optical-sdi",
        Some(ConnectorType::DisplayPort)
        | Some(ConnectorType::Analog)
        | Some(ConnectorType::Unknown)
        | None => "",
    };
    let (video_src, audio_src) = match plan.source.selection_mode {
        // PersistentID 可用 → 官方首选 `persistent-id`.
        SourceSelectionMode::PersistentIdCanonical => (
            format!(
                "decklinkvideosrc persistent-id={}{}",
                plan.source.bmd_persistent_id.unwrap_or(0),
                connection
            ),
            // 注: decklinkaudiosrc 无 `connection` 属性 (音频内嵌于 SDI 视频流, 跟随视频连接),
            // 绝不可像 videosrc 那样设 `connection=sdi`, 否则 launch 串解析失败 (MEDIA-RT-01 真机实测).
            format!(
                "decklinkaudiosrc persistent-id={}",
                plan.source.bmd_persistent_id.unwrap_or(0)
            ),
        ),
        // DeviceHandle 经 Resolver 解析到确定 device-number (当前硬件正式生产路径).
        // 注: `hw-serial-number` 是 GStreamer 侧硬件序列号/硬件 ID 探测属性 (只读),
        // 与 "BMD PersistentID" 是两回事; 此处经 Resolver 已映射到 device-number.
        SourceSelectionMode::DeviceHandleResolved | SourceSelectionMode::DiagnosticFallback => (
            format!(
                "decklinkvideosrc device-number={}{}",
                plan.source.device_number, connection
            ),
            // 同上: decklinkaudiosrc 无 `connection` 属性, 仅 device-number 选卡, 音频跟随视频 SDI 连接.
            format!(
                "decklinkaudiosrc device-number={}",
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
    registry: Option<&crate::port::PortRegistry>,
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

        // 连接类型: 优先按 Control Plane 显式声明的 `port_id` 精确定位端口; 否则回退到该设备
        // 的首个输入端口. 二者皆无 (无 registry / 端口未注册) → `None`, `src_props` 不显式指定 connection.
        let connector = registry.and_then(|r| {
            if let Some(pid) = &d.pipeline.source.port_id {
                if let Ok(u) = Uuid::parse_str(pid) {
                    return r
                        .ports
                        .iter()
                        .find(|p| p.identity.port_id == Some(u))
                        .map(|p| p.identity.connector);
                }
            }
            r.ports
                .iter()
                .find(|p| p.device_id == info.device_id && p.direction == PortDirection::Input)
                .map(|p| p.identity.connector)
        });

        let source = SourcePlan {
            device_id: d.device_id.clone(),
            bmd_persistent_id: info.bmd_persistent_id,
            device_number: resolved_device_number.unwrap_or(0),
            connector,
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
        let mut h = PipelineHealth {
            video_first_pts: Some(1000),
            audio_first_pts: Some(1000),
            ..PipelineHealth::default()
        };
        // 仅 video 有效 -> 未过.
        h.observe_video_pts(1000);
        h.observe_audio_pts(1000);
        assert!(h.first_frame_ok());
        // 任一回退 -> 不过.
        h.observe_audio_pts(999);
        assert!(!h.first_frame_ok());
    }
}
