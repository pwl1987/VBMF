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
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use uuid::Uuid;

// C7: 共享事件/健康类型与全局健康表 (HEALTH_ARCS/read_health/BusSeverity/PipelineBusEvent/
// PipelineBusEventKind/bus_event_recovery_policy) 已物理迁至中性模块 `pipeline_events.rs`
// (不依赖 vendor `gstreamer` crate, 在 default/simulation/mock 无 gstreamer 构建下也须编译).
// 消费方 (main.rs / contracts/backend.rs / adapters/mock.rs) 直接 `use crate::pipeline_events::*`,
// `pipeline.rs` 仅引用自身用到的 `PipelineBusEvent` (LAST_FATAL_BUS_EVENT / last_fatal_bus_event).
use crate::pipeline_events::PipelineBusEvent;

/// 全局唯一 pipeline 句柄计数器 (P1-1: 多 controller 共用全局 `HEALTH_ARCS` 时避免 handle 碰撞).
pub(crate) static NEXT_PIPELINE_ID: AtomicU64 = AtomicU64::new(1);

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
    /// Provider 侧持久标识 (P0-1 中立化: 经 Resolver 绑定从 ProviderIdentity 证据透传;
    /// PersistentIdCanonical 模式使用; 否则 `None`. 字段名不冠 vendor 专名).
    pub provider_persistent_id: Option<i64>,
    /// Resolver 解析后的 GStreamer `device-number` (DeviceHandleResolved/DiagnosticFallback 使用).
    pub device_number: u32,
    /// 物理连接器类型 (由 PortRegistry 经 Manifest 声明 + 运行时探测推导得到). 决定 GStreamer
    /// `decklinkvideosrc` 的 `connection=<...>` 属性; `None`/无对应枚举的连接器 → 不显式指定, 由插件默认探测.
    /// **绝不硬编码** 成 `sdi` (见 `src_props`).
    pub connector: Option<ConnectorType>,
    pub selection_mode: SourceSelectionMode,
}

/// 物化后的管线计划 (控制面只给 VBMF `device_id`; provider_persistent_id / device-number 由 materialize 解析).
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
                provider_persistent_id: None,
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


/// GStreamer 选卡属性 (decklinkvideosrc/audiosrc).
///
/// 选卡属性语义 (用户复核 §十, 纠正旧注释 "hw-serial-number=<BMD PersistentID>" 的错误表达):
/// - PersistentID 可用 → GStreamer `persistent-id=<BMD PersistentID>` (官方首选, 优先级高于 device-number).
/// - PersistentID 不可用 → Resolver 探测 `hw-serial-number` 匹配 SDK DeviceHandle, 解析出
///   确定的 GStreamer `device-number` → `decklinkvideosrc device-number=<n>`.
/// - `hw-serial-number` 是 **GStreamer 侧** 硬件序列号/硬件 ID 探测属性 (只读), 与 "BMD PersistentID"
///   是两回事; 它不是 PersistentID 的别名. SDK 枚举 index 绝不直接当 device-number.
pub(crate) fn src_props(plan: &PipelinePlan) -> (String, String) {
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
                plan.source.provider_persistent_id.unwrap_or(0),
                connection
            ),
            // 注: decklinkaudiosrc 无 `connection` 属性 (音频内嵌于 SDI 视频流, 跟随视频连接),
            // 绝不可像 videosrc 那样设 `connection=sdi`, 否则 launch 串解析失败 (MEDIA-RT-01 真机实测).
            format!(
                "decklinkaudiosrc persistent-id={}",
                plan.source.provider_persistent_id.unwrap_or(0)
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
/// 身份层级状态机 (用户复核 §二/§三/§十九): 严格按 `identity_strength` 判定.
/// P0-1: 强度由 Provider 在 discovery 时按自身证据自证 (Domain 无 vendor 字段可伪造,
/// filesystem 合成身份强度恒为 Enumeration); 合成身份在生产路径必须拒绝.
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

        let binding = bindings.get(&info.device_id);
        let resolved_device_number = binding.map(|b| b.device_number);
        // 身份层级状态机: 严格按 identity_strength (provider 自证), 绝不默认.
        let selection_mode = match info.identity_strength {
            IdentityStrength::PersistentId => {
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
                            "{}: 身份未解析 (identity_strength={:?}, Resolver 绑定={:?}); 生产拒绝 device 0",
                            d.device_id, info.identity_strength, resolved_device_number
                        )));
                    }
                    MaterializeMode::Diagnostic => SourceSelectionMode::DiagnosticFallback,
                }
            }
        };

        // 连接类型: 优先按 Control Plane 显式声明的 `port_id` 精确定位端口; 否则回退到该设备
        // 的首个输入端口. 显式 `port_id` 在 Discovery 无对应端口 ⇒ 生产失败闭合 (绝不静默回退 auto 探测).
        let connector = match &d.pipeline.source.port_id {
            Some(pid) => {
                let u = match Uuid::parse_str(pid) {
                    Ok(u) => u,
                    Err(e) => {
                        return Err(PipelineError::IdentityUnresolved(format!(
                            "{}: port_id 解析失败: {e}",
                            d.device_id
                        )))
                    }
                };
                match registry.and_then(|r| {
                    r.ports
                        .iter()
                        .find(|p| p.identity.port_id == Some(u))
                        .map(|p| p.identity.connector)
                }) {
                    Some(c) => Some(c),
                    None => {
                        if matches!(mode, MaterializeMode::Production) {
                            return Err(PipelineError::IdentityUnresolved(format!(
                                "{}: 显式 port_id {} 在 Discovery 端口中无匹配 (生产拒绝静默回退 auto)",
                                d.device_id, pid
                            )));
                        } else {
                            None // Diagnostic: 回退 auto 探测
                        }
                    }
                }
            }
            None => registry.and_then(|r| {
                r.ports
                    .iter()
                    .find(|p| p.device_id == info.device_id && p.direction == PortDirection::Input)
                    .map(|p| p.identity.connector)
            }),
        };

        let source = SourcePlan {
            device_id: d.device_id.clone(),
            provider_persistent_id: binding.and_then(|b| b.persistent_id),
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
    use crate::device::DeviceIdentitySource;
    use crate::graph_intent::{
        DeviceIntent, GraphRuntimeIntent, PipelineIntent, SinkIntent, SourceIntent,
    };
    use crate::port::{
        PortIdentity, PortInfo, PortOrdinal, PortRegistry, SignalStatus, VideoContentState,
    };
    use crate::resolver::{Confidence, ResolvedDeviceBinding, ResolverMatch};
    use uuid::Uuid;

    fn dev_handle(strength: IdentityStrength) -> DeviceInfo {
        DeviceInfo {
            device_id: Uuid::new_v4(),
            model: "DeckLink SDI".to_string(),
            display_name: "dv".to_string(),
            serial_number: None,
            identity_strength: strength,
            identity_source: DeviceIdentitySource::RealBmd,
            capabilities: crate::port::DeviceCapabilities::default(),
            video_input_connections: 0,
            video_output_connections: 0,
            ports: Vec::new(),
        }
    }

    fn intent_with_port(device_id: &str, port_id: Option<&str>) -> GraphRuntimeIntent {
        GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![DeviceIntent {
                device_id: device_id.into(),
                role: "CAPTURE".into(),
                pipeline: PipelineIntent {
                    source: SourceIntent {
                        kind: "decklink".into(),
                        device_id: device_id.into(),
                        port_id: port_id.map(|s| s.into()),
                    },
                    sink: SinkIntent {
                        kind: "appsink".into(),
                    },
                },
            }],
        }
    }

    fn registry_with_port(dev_id: Uuid, pid: Uuid, connector: ConnectorType) -> PortRegistry {
        PortRegistry {
            ports: vec![PortInfo {
                device_id: dev_id,
                provider_binding_ref: Some("h".into()),
                identity: PortIdentity {
                    port_id: Some(pid),
                    connector,
                    ordinal: PortOrdinal::Known(1),
                },
                direction: PortDirection::Input,
                capabilities: crate::port::PortCapabilities::default(),
                runtime_binding: None,
                signal: SignalStatus::default(),
                content: VideoContentState::Unknown,
            }],
        }
    }

    #[test]
    fn src_props_sdi_uses_connection_sdi() {
        // canonical 边界回归: SDI ⇒ decklinkvideosrc connection=sdi, audiosrc 无 connection.
        let plan = PipelinePlan {
            source: SourcePlan {
                device_id: "d".into(),
                provider_persistent_id: None,
                device_number: 2,
                connector: Some(ConnectorType::Sdi),
                selection_mode: SourceSelectionMode::DeviceHandleResolved,
            },
            normalize: true,
            switch_mode: "FRAME_SWITCH".into(),
        };
        let (v, a) = src_props(&plan);
        assert!(
            v.contains("decklinkvideosrc device-number=2 connection=sdi"),
            "video={v}"
        );
        assert!(a.contains("decklinkaudiosrc device-number=2"), "audio={a}");
        assert!(!a.contains("connection"), "audio 不得含 connection: {a}");
    }

    #[test]
    fn src_props_optical_uses_connection_optical_sdi() {
        // canonical 边界回归: Optical ⇒ connection=optical-sdi (绝非 optical).
        let plan = PipelinePlan {
            source: SourcePlan {
                device_id: "d".into(),
                provider_persistent_id: None,
                device_number: 3,
                connector: Some(ConnectorType::Optical),
                selection_mode: SourceSelectionMode::DeviceHandleResolved,
            },
            normalize: true,
            switch_mode: "FRAME_SWITCH".into(),
        };
        let (v, _) = src_props(&plan);
        assert!(
            v.contains("connection=optical-sdi"),
            "video={v} (不得 optical)"
        );
    }

    #[test]
    fn src_props_unknown_has_no_connection() {
        // canonical 边界回归: Unknown 连接器 ⇒ 不显式指定 connection (由插件 auto 探测).
        let plan = PipelinePlan {
            source: SourcePlan {
                device_id: "d".into(),
                provider_persistent_id: None,
                device_number: 4,
                connector: Some(ConnectorType::Unknown),
                selection_mode: SourceSelectionMode::DeviceHandleResolved,
            },
            normalize: true,
            switch_mode: "FRAME_SWITCH".into(),
        };
        let (v, _) = src_props(&plan);
        assert!(
            !v.contains("connection"),
            "Unknown 不得显式 connection: {v}"
        );
    }

    #[test]
    fn materialize_rejects_explicit_port_id_missing_in_registry_production() {
        // TDD(RED→GREEN): Control Plane 显式 port_id 但 Discovery 无匹配 ⇒ 生产失败闭合 (绝不静默回退 auto).
        let dev = dev_handle(IdentityStrength::DeviceHandle);
        let dev_id = dev.device_id;
        let missing_pid = Uuid::new_v4();
        let intent = intent_with_port(&dev_id.to_string(), Some(&missing_pid.to_string()));
        let mut bindings = std::collections::HashMap::new();
        bindings.insert(
            dev_id,
            ResolvedDeviceBinding {
                device_number: 2,
                hw_serial_number: None,
                persistent_id: None,
                confidence: Confidence::High,
                match_kind: ResolverMatch::SerialExact,
            },
        );
        let other = Uuid::new_v4();
        let registry = registry_with_port(dev_id, other, ConnectorType::Sdi);
        let res = materialize(
            &intent,
            &[dev],
            MaterializeMode::Production,
            &bindings,
            Some(&registry),
        );
        assert!(res.is_err(), "显式 port_id 缺失须被生产拒绝: {res:?}");
    }

    #[test]
    fn materialize_resolves_explicit_port_id_in_registry() {
        // 回归: 显式 port_id 匹配 Discovery ⇒ Ok 且 connector 取该端口 (精确绑定, 不猜).
        let dev = dev_handle(IdentityStrength::DeviceHandle);
        let dev_id = dev.device_id;
        let pid = Uuid::new_v4();
        let intent = intent_with_port(&dev_id.to_string(), Some(&pid.to_string()));
        let mut bindings = std::collections::HashMap::new();
        bindings.insert(
            dev_id,
            ResolvedDeviceBinding {
                device_number: 2,
                hw_serial_number: None,
                persistent_id: None,
                confidence: Confidence::High,
                match_kind: ResolverMatch::SerialExact,
            },
        );
        let registry = registry_with_port(dev_id, pid, ConnectorType::Sdi);
        let plans = materialize(
            &intent,
            &[dev],
            MaterializeMode::Production,
            &bindings,
            Some(&registry),
        )
        .unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].source.connector, Some(ConnectorType::Sdi));
    }

    #[test]
    fn materialize_rejects_unresolved_identity_production() {
        // 回归: 合成身份 + 无 Resolver 绑定 ⇒ 生产拒绝 (绝不盲开 device 0).
        let dev = dev_handle(IdentityStrength::Enumeration);
        let dev_id = dev.device_id;
        let intent = intent_with_port(&dev_id.to_string(), None);
        let bindings = std::collections::HashMap::new();
        let registry = PortRegistry::default();
        let res = materialize(
            &intent,
            &[dev],
            MaterializeMode::Production,
            &bindings,
            Some(&registry),
        );
        assert!(res.is_err(), "未解析身份须被生产拒绝: {res:?}");
    }

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

    // ── p06-hi MEDIA-RT-01 acceptance gate ─────────────────────────────────────────
    // 门禁不变量: (1) Default = absence-of-evidence, 绝不默认假过 (P1-2);
    // (2) PTS 三态区分 Unknown 与 NonMonotonic, 只在真实回退时置 false;
    // (3) B/C 子项语义: B 为帧证据, C 为测量窗口 (仅"无 error"不够);
    // (4) self_test plan 是 canonical 自测源 (device_number=0 为自测哨兵, 非真实卡默认).

    #[test]
    fn media_rt_01_health_default_is_absence_not_pass() {
        let h = PipelineHealth::default();
        assert_eq!(h.video_pts_state, PtsMonotonicity::Unknown);
        assert_eq!(h.audio_pts_state, PtsMonotonicity::Unknown);
        assert!(!h.playing);
        assert!(h.video_first_pts.is_none());
        // 未观测 = 未通过: pass() / first_frame_ok() 默认必须为 false, 绝不默认假过.
        assert!(!h.pass());
        assert!(!h.first_frame_ok());
        // acceptance 正向成就项默认全 false (P1-2): absence-of-evidence ≠ PASS.
        let a = MediaRt01Acceptance::default();
        assert!(!a.a_pass() && !a.b_pass() && !a.c_pass());
    }

    #[test]
    fn media_rt_01_pts_only_false_on_real_regression() {
        let mut h = PipelineHealth::default();
        // Unknown ≠ NonMonotonic: 未收帧前状态为 Unknown, 不得当作 "置 false" 的失败证据.
        assert_eq!(h.video_pts_state, PtsMonotonicity::Unknown);
        h.observe_video_pts(1000);
        h.observe_video_pts(2000);
        assert_eq!(h.video_pts_state, PtsMonotonicity::ValidMonotonic);
        // 只在真实回退时置 false: NonMonotonic (sticky, 不自动恢复).
        h.observe_video_pts(1500);
        assert_eq!(h.video_pts_state, PtsMonotonicity::NonMonotonic);
        h.observe_video_pts(3000);
        assert_eq!(h.video_pts_state, PtsMonotonicity::NonMonotonic);
    }

    #[test]
    fn media_rt_01_b_and_c_pass_semantics() {
        // B: 首视频帧 + 首音频帧 + 有效 PTS + PTS 单调 — 四项全真才 pass.
        let b_ok = MediaRt01Acceptance {
            b1_first_video: true,
            b2_first_audio: true,
            b3_valid_pts: true,
            b4_pts_monotonic: true,
            ..MediaRt01Acceptance::default()
        };
        assert!(b_ok.b_pass());
        // C: 测量型 — 观测窗口未达标即不过, 即使当前无任何错误.
        assert!(!b_ok.c_pass());
        // 窗口达标 + 无致命项 → c_pass.
        let c_ok = MediaRt01Acceptance {
            b4_pts_monotonic: true,
            c_observed_ms: Some(10_000),
            c1_no_unexpected_eos: true,
            c2_no_pipeline_error: true,
            c3_no_repeated_reneg: true,
            c4_counters_continue: true,
            ..MediaRt01Acceptance::default()
        };
        assert!(c_ok.c_pass());
    }

    #[test]
    fn media_rt_01_self_test_plan_is_canonical() {
        let plan = PipelinePlan::self_test();
        assert_eq!(plan.source.device_id, "self-test");
        assert_eq!(plan.source.selection_mode, SourceSelectionMode::SelfTest);
        assert!(plan.normalize);
        assert_eq!(plan.switch_mode, "FRAME_SWITCH");
        // 自测哨兵: 无真实设备, `device_number: 0` 是占位, 不违反
        // "device-number 绝不默认 0" (该约束针对真实选卡不得静默落到 DeckLink 0 号).
        assert_eq!(plan.source.device_number, 0);
        assert!(plan.source.provider_persistent_id.is_none());
    }

    // ── p06-hi ARCH-BACKEND-01 gate (Test C 延伸到 Backend 侧) ─────────────────────
    // 门禁不变量: `MediaBackend` 实现以 trait-object 级可互换 — Domain/Graph/Session/
    // Supervisor/Health 只依赖 `dyn MediaBackend` 与 canonical `PipelinePlan`,
    // 不依赖具体 backend 类型; canonical plan 是不可变输入, backend 不得回写.

    #[cfg(feature = "mock")]
    #[test]
    fn arch_backend_01_mock_backend_implements_media_backend() {
        let backend: Box<dyn crate::contracts::backend::MediaBackend> =
            Box::new(crate::adapters::mock::MockBackend);
        let plan = PipelinePlan::self_test();
        let handle = backend
            .instantiate(&plan)
            .expect("MockBackend instantiate 应接受 canonical plan");
        backend.start(&handle).expect("MockBackend start 应成功");
        backend.recover(&handle).expect("MockBackend recover 应成功");
        backend.stop(&handle).expect("MockBackend stop 应成功");
        assert!(backend.observe(&handle).is_empty());
        // P1-1: 句柄与生产同源分配 (NEXT_PIPELINE_ID, 从 1 起), 绝不为 0 哨兵.
        assert_ne!(handle, PipelineHandle(0));
        // canonical 字段未被 backend 回写.
        assert_eq!(plan.source.selection_mode, SourceSelectionMode::SelfTest);
        assert!(plan.normalize);
    }

    #[cfg(feature = "gstreamer-backend")]
    #[test]
    fn arch_backend_01_gstreamer_backend_implements_media_backend() {
        // 同一 trait 对象 + 同一 canonical plan: GStreamer Reference Backend 与 Mock
        // 在 SPI 层面可互换. prepare 构建 videotestsrc/audiotestsrc 管线 (无需硬件;
        // 该断言在 GStreamer 运行时构建下执行, CI 无 GStreamer 时仅 Mock 侧运行).
        let backend: Box<dyn crate::contracts::backend::MediaBackend> =
            Box::new(crate::adapters::gstreamer::GStreamerPipelineController::new());
        let plan = PipelinePlan::self_test();
        let handle = backend
            .instantiate(&plan)
            .expect("GStreamerBackend instantiate 应接受同一 canonical plan (self_test)");
        // 句柄为运行时实例标识, 不得与 Mock 固定哨兵冲突.
        assert_ne!(handle, PipelineHandle(0));
        // canonical 字段未被 backend 回写.
        assert_eq!(plan.source.selection_mode, SourceSelectionMode::SelfTest);
        assert!(plan.normalize);
    }
}
