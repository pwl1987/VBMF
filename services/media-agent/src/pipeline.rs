//! Media Pipeline Lifecycle Orchestrator (PipelineController).
//!
//! Frozen interface per SoT §15.2 / Phase 0.6 canonical-ingest contract.
//!
//! 本文件实现 **canonical ingest 的真实物化**:
//!   * `PipelinePlan` 是 `GraphRuntimeIntent` 的 materialization, 不是第二套 Graph Model;
//!   * 唯一 canonical 媒体采集通道 = GStreamer `decklinkvideosrc` + `decklinkaudiosrc`;
//!   * `IDeckLinkInput` (Rust FFI) 仅作 Device Capability / 模式探测 / 诊断探针, **不**作
//!     生产视频数据通道 (否则双采 / 设备争用).
//!
//! GStreamer 真实 launch 由 `feature = "gstreamer"` 门控 (需要宿主机 GStreamer 1.22+ 与
//! `decklinkvideosrc`/`decklinkaudiosrc` 插件). 未启用该 feature 时, trait 仍提供骨架实现,
//! 保证 `default` / `simulation` / `bmd` 构建均可编译并通过单元/集成测试.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::device::DeviceInfo;
use crate::graph_intent::GraphRuntimeIntent;

/// VBMF canonical 设备身份 = DeckLink `DeviceHandle` 派生 UUID
/// (见 `device.rs` `DeckLinkDeviceManager`, UUIDv5(serial)).
/// 跨进程/重启/拓扑变化稳定; 与 GStreamer `device-number` 索引解耦.
pub type CanonicalDeviceId = String;

/// materialize 模式: 决定 `bmd_persistent_id` 解析失败时的行为.
///
/// 硬规则 (Phase 0.6 锁死): **生产路径禁止**悄悄把 `device-number` 当作 PersistentID 兜底.
/// 只有显式 Diagnostic/Compatibility 模式才允许用 `device-number` 兜底, 且必须在证据中标注.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MaterializeMode {
    /// 生产路径: `bmd_persistent_id` 解析失败 → `IdentityUnresolved`, 绝不盲开.
    Production,
    /// 诊断/兼容模式: `bmd_persistent_id` 缺失时显式退回 `device-number` (仅探测/回退用).
    Diagnostic,
}

/// 物化后 GStreamer 实际选用哪张卡的选择策略.
///
/// **注意**: `bmd_persistent_id` (BMDDeckLinkPersistentID) 与 `DeviceHandle`
/// (RevisionID:PersistentID:TopologicalID) 是两回事; `Canonical` 模式把前者映射到
/// GStreamer `hw-serial-number` (本机 gst-inspect 实测属性名; 部分 GStreamer 版本为
/// `persistent-id`, gint64), 优先级高于 `device-number`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceSelectionMode {
    /// 生产路径: 用 BMD PersistentID → GStreamer `hw-serial-number` (首选; 本机 gst-inspect
    /// 实测属性名为 `hw-serial-number`, 部分 GStreamer 版本为 `persistent-id`).
    Canonical,
    /// 仅 Diagnostic/Compatibility: BMD PersistentID 解析失败, 显式退回 `device-number`.
    DiagnosticFallback,
    /// MEDIA-RT-01 自测: 用 `videotestsrc`/`audiotestsrc` 代替 DeckLink, 验证媒体运行时链路,
    /// 不依赖真实 SDI 信号 (信号待接入时用于证明采集/健康闭环在运行时层面正确).
    SelfTest,
}

/// Media Agent 在运行时对 `GraphRuntimeIntent` 的物化执行计划.
///
/// 关键边界 (Phase 0.6 锁死):
///   * `PipelinePlan` **不是** 第二套 Graph Model; 它是 `GraphRuntimeIntent` 的物化.
///   * GStreamer `decklinkvideosrc`/`decklinkaudiosrc` 选卡属性: 本机 gst-inspect 实测为
///     `hw-serial-number` (String, 对应硬件 ID) / `device-number` (Integer); 部分版本为
///     `persistent-id`. 物化链路: VBMF `device_id` → Device Registry → BMD PersistentID →
///     GStreamer `hw-serial-number`; `device-number` 仅作 Diagnostic 回退.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinePlan {
    pub source: SourcePlan,
    pub video: VideoPlan,
    pub audio: AudioPlan,
    pub switch: SwitchPlan,
    pub output: OutputPlan,
}

impl PipelinePlan {
    /// MEDIA-RT-01 自测源计划: 用 `videotestsrc`/`audiotestsrc` 代替 DeckLink,
    /// 验证媒体运行时链路 (GStreamer launch → appsink 首帧 → PTS → MEDIA-RT-01 A/B/C),
    /// 不依赖真实 SDI 信号.
    pub fn self_test() -> Self {
        PipelinePlan {
            source: SourcePlan {
                device_id: "self-test".into(),
                bmd_persistent_id: 0,
                device_number: 0,
                selection_mode: SourceSelectionMode::SelfTest,
                resolved_input: ResolvedInputContract {
                    mode: "self-test".into(),
                    pixel_format: "I420".into(),
                    fps: 25.0,
                    interlace: false,
                },
            },
            video: VideoPlan {
                normalize: true,
                mode: "self-test".into(),
                pixel_format: "I420".into(),
                fps: 25.0,
                interlace: false,
            },
            audio: AudioPlan {
                enabled: true,
                channels: 2,
                sample_rate: 48_000,
            },
            switch: SwitchPlan { mode: "FRAME_SWITCH".into() },
            output: OutputPlan { sink: "rtmp".into() },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcePlan {
    /// VBMF canonical identity (DeviceHandle 派生 UUID). 主键.
    /// **命名澄清**: 此字段是 VBMF 规范身份, **不是** BMD PersistentID.
    /// BMD PersistentID 见 `selection_mode` + `bmd_persistent_id`.
    pub device_id: CanonicalDeviceId,
    /// BMD 硬件持久身份 (BMDDeckLinkPersistentID), 物化自 Device Registry.
    /// 对应 GStreamer `decklinkvideosrc`/`decklinkaudiosrc` 的 `persistent-id` (gint64).
    pub bmd_persistent_id: u32,
    /// GStreamer `device-number` 属性 (仅 Diagnostic 回退 / 诊断). 与 `bmd_persistent_id`
    /// 指向 **同一块** 已解析设备, 不会盲开 device 0.
    pub device_number: u32,
    /// GStreamer 实际选卡策略 (Canonical 优先; DiagnosticFallback 仅显式诊断).
    pub selection_mode: SourceSelectionMode,
    /// 模式协商后的输入契约 (Capability Match 产物).
    pub resolved_input: ResolvedInputContract,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedInputContract {
    pub mode: String,         // e.g. "1080i50"
    pub pixel_format: String, // e.g. "UYVY" (8-bit YUV)
    pub fps: f64,
    pub interlace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoPlan {
    /// RAW → RAW (Signal Contract / clock / PTS). 在 Encode 之前, 切换之内.
    pub normalize: bool,
    /// 协商后的输入格式 (当前最小字段集; CAP-03/04 再逐步补全 ClockPlan/NormalizationPlan).
    pub mode: String,
    pub pixel_format: String,
    pub fps: f64,
    pub interlace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioPlan {
    pub enabled: bool,
    pub channels: u32,    // 2 / 8 / 16
    pub sample_rate: u32, // 48000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchPlan {
    /// FRAME_SWITCH | MASTER_SWITCH. 决策由 Graph Compiler 给出, Media Agent 执行.
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPlan {
    /// 媒体出口 (SRS 负责 RTMP/HLS/WHEP 分发; 非 Encoder).
    pub sink: String,
}

/// 管线句柄 (物化后由 `prepare` 返回, 供 start/stop/recover 引用).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PipelineHandle(pub Uuid);

/// 管线运行时健康 (Pipeline Health / GStreamer bus 监控载体).
///
/// 对应 Phase 0.6 "24h stability 需落到 Runtime Harness": 此处先提供 MEDIA-RT-01 所需的
/// 最小健康指标 (首帧 / PTS / 单调性 / 错误), 后续 G-RUNTIME 再扩展 EOS/restart/输出健康.
#[derive(Debug, Clone, Default)]
pub struct PipelineHealth {
    pub video_first_pts: Option<u64>,
    pub audio_first_pts: Option<u64>,
    pub video_frame_count: u64,
    pub audio_frame_count: u64,
    /// PTS 单调非减 (经 `GetHardwareReferenceClock`/GStreamer clock 校验).
    pub pts_monotonic: bool,
    pub last_error: Option<String>,
    pub running: bool,
    pub started_at: Option<i64>,
    /// MEDIA-RT-01 接受判定 (A1-A4 / B1-B4 / C1-C4), 由 watchdog 从 bus + 计数推导.
    pub acceptance: MediaRt01Acceptance,
}

impl PipelineHealth {
    /// MEDIA-RT-01B: 真实 video+audio 首帧已到, 且 PTS 有效.
    pub fn first_frame_ok(&self) -> bool {
        self.video_first_pts.is_some()
            && self.audio_first_pts.is_some()
            && self.video_frame_count > 0
            && self.audio_frame_count > 0
            && self.pts_monotonic
            && self.last_error.is_none()
    }
}

/// MEDIA-RT-01 接受判定 (Phase 0.6: 不是 happy path, 必须确定性观测, 防"PLAYING 但无信号"误判).
/// 拆为三层, 每层再细分可观测子项:
///   * A — Ingest Open: A1 身份解析 / A2 租约获取 / A3 管线 PLAYING / A4 信号检测.
///   * B — First Frame: B1 首视频帧 / B2 首音频帧 / B3 有效 PTS / B4 PTS 单调.
///   * C — Short Stability: C1 无意外 EOS / C2 无 pipeline error / C3 无重复重协商 /
///         C4 帧/音频计数持续增长.
/// 默认 = "尚无失败证据" 语义: C 层 (无意外 EOS / 无 pipeline error / 无重复重协商)
/// 与 B4 (PTS 单调) 起始为 `true` (absence-of-evidence = pass, 直到被 bus 事件 / 帧数据证伪);
/// 其余子项需显式观测到位, 故起始 `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaRt01Acceptance {
    // A — Ingest Open
    pub a1_identity_resolved: bool,
    pub a2_lease_acquired: bool,
    pub a3_pipeline_playing: bool,
    pub a4_signal_detected: bool,
    // B — First Frame
    pub b1_first_video: bool,
    pub b2_first_audio: bool,
    pub b3_valid_pts: bool,
    pub b4_pts_monotonic: bool,
    // C — Short Stability
    pub c1_no_unexpected_eos: bool,
    pub c2_no_pipeline_error: bool,
    pub c3_no_repeated_reneg: bool,
    pub c4_counters_continue: bool,
}

impl MediaRt01Acceptance {
    pub fn a_pass(&self) -> bool {
        self.a1_identity_resolved && self.a2_lease_acquired && self.a3_pipeline_playing && self.a4_signal_detected
    }
    pub fn b_pass(&self) -> bool {
        self.b1_first_video && self.b2_first_audio && self.b3_valid_pts && self.b4_pts_monotonic
    }
    pub fn c_pass(&self) -> bool {
        self.c1_no_unexpected_eos && self.c2_no_pipeline_error && self.c3_no_repeated_reneg && self.c4_counters_continue
    }
    /// MEDIA-RT-01 = A + B + C 全过. 单 first-frame 不足以判定整个媒体运行时健康.
    pub fn pass(&self) -> bool {
        self.a_pass() && self.b_pass() && self.c_pass()
    }
}

impl Default for MediaRt01Acceptance {
    /// C1/C2/C3 (无失败证据) 与 B4 (PTS 单调, 直到帧证伪) 起始为 true; 其余需观测置位.
    fn default() -> Self {
        Self {
            a1_identity_resolved: false,
            a2_lease_acquired: false,
            a3_pipeline_playing: false,
            a4_signal_detected: false,
            b1_first_video: false,
            b2_first_audio: false,
            b3_valid_pts: false,
            b4_pts_monotonic: true,
            c1_no_unexpected_eos: true,
            c2_no_pipeline_error: true,
            c3_no_repeated_reneg: true,
            c4_counters_continue: false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("device lease invalid")]
    LeaseInvalid,
    #[error("identity resolution failed: {0}")]
    IdentityUnresolved(String),
    #[error("gstreamer init failed: {0}")]
    GstreamerInit(String),
    #[error("pipeline launch failed: {0}")]
    LaunchFailed(String),
    #[error("appsink element missing: {0}")]
    AppsinkMissing(String),
}

/// GStreamer bus 事件 (Supervisor 只看事件决策, 不碰 GStreamer).
/// 由 `GStreamerPipelineController::poll_bus` 从 `pipeline.bus()` 抽取.
#[derive(Debug, Clone)]
pub enum PipelineBusEvent {
    Error(String),
    Eos,
    StatePlaying,
}

/// Media Agent = 媒体运行时生命周期 owner: 创建/配置/启动/停止/恢复 GStreamer.
/// **不**重新实现 `IDeckLinkInput` 帧搬运.
pub trait PipelineController {
    /// 物化 GraphRuntimeIntent → PipelinePlan, 校验 Device Lease, 解析
    /// canonical identity → GStreamer device-number, 构造管线 (尚未 launch).
    fn prepare(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError>;
    /// 启动真实 GStreamer 采集 (decklinkvideosrc + decklinkaudiosrc → RAW).
    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    fn stop(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    /// MEDIA-03: 崩溃/挂起后的恢复 (revalidate lease → restart GStreamer).
    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
}

/// 把控制面 `GraphRuntimeIntent` 物化为 Media Agent 执行计划.
///
/// 物化链路 (Control Plane 不感知 GStreamer property):
///   VBMF `device_id` → Device Registry (`DeviceInfo`) → `bmd_persistent_id`
///   → GStreamer `hw-serial-number` (首选) / `device-number` (Diagnostic 回退).
///
/// **硬规则**: `MaterializeMode::Production` 下, 若 `device_id` 在 registry 中找不到,
/// 或找到但 `bmd_persistent_id == 0` (PersistentID 解析失败), 直接 `IdentityUnresolved`
/// 失败 —— 绝不 `unwrap_or(0)` 盲开 device 0. 广播系统最危险的是打开*错误*的输入
/// (以为开了 SDI-A, 实际开了 SDI-B), 而不是打不开.
pub fn materialize(
    intent: &GraphRuntimeIntent,
    devices: &[DeviceInfo],
    mode: MaterializeMode,
) -> Result<Vec<PipelinePlan>, PipelineError> {
    intent
        .devices
        .iter()
        .map(|d| {
            // 1) 用 VBMF device_id 在 Device Registry 中定位确定设备.
            let info = devices
                .iter()
                .find(|i| i.device_id.to_string() == d.device_id)
                .ok_or_else(|| PipelineError::IdentityUnresolved(d.device_id.clone()))?;

            // 2) 硬规则: 生产路径要求 BMD PersistentID 已解析 (non-zero).
            let selection_mode = if info.bmd_persistent_id != 0 {
                SourceSelectionMode::Canonical
            } else {
                match mode {
                    MaterializeMode::Production => {
                        // 生产路径: 解析失败即失败, 不盲开.
                        return Err(PipelineError::IdentityUnresolved(format!(
                            "{}: BMD PersistentID 未解析 (bmd_persistent_id=0)",
                            d.device_id
                        )));
                    }
                    MaterializeMode::Diagnostic => SourceSelectionMode::DiagnosticFallback,
                }
            };

            Ok(PipelinePlan {
                source: SourcePlan {
                    device_id: d.device_id.clone(),
                    bmd_persistent_id: info.bmd_persistent_id,
                    device_number: info.device_number,
                    selection_mode,
                    resolved_input: ResolvedInputContract {
                        mode: "auto".into(), // 由 DoesSupportVideoMode 协商, 此处占位
                        pixel_format: "UYVY".into(),
                        fps: 0.0,
                        interlace: true,
                    },
                },
                video: VideoPlan {
                    normalize: true,
                    mode: "auto".into(),
                    pixel_format: "UYVY".into(),
                    fps: 0.0,
                    interlace: true,
                },
                audio: AudioPlan {
                    enabled: d.pipeline.source.kind == "decklink",
                    channels: 2,
                    sample_rate: 48_000,
                },
                switch: SwitchPlan {
                    mode: "FRAME_SWITCH".into(),
                },
                output: OutputPlan {
                    sink: d.pipeline.sink.kind.clone(),
                },
            })
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// 真实 GStreamer launch (feature = "gstreamer")
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "gstreamer")]
mod gst_runtime {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, LazyLock, Mutex};

    use gstreamer::prelude::*;
    use gstreamer as gst;
    use gstreamer_app::AppSink;

    /// 跨线程共享的健康 Arc 表 (appsink 回调写入, watchdog/health 读取).
    /// `Mutex::new(HashMap::new())` 非 const, 不能用于 `static`, 用 `LazyLock` 懒初始化.
    pub static HEALTH_ARCS: LazyLock<Mutex<HashMap<PipelineHandle, Arc<Mutex<PipelineHealth>>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    /// 读取某管线最新健康 (跨线程, 经共享 Arc).
    pub fn read_health(handle: &PipelineHandle) -> Option<PipelineHealth> {
        HEALTH_ARCS
            .lock()
            .unwrap()
            .get(handle)
            .map(|a| a.lock().unwrap().clone())
    }

    /// 真实 GStreamer 管线控制器.
    ///
    /// 生命周期: `prepare` 存 plan; `start` 经 `gst::parse::launch`
    /// 构造 `decklinkvideosrc/audiosrc hw-serial-number=<BMD PersistentID> → appsink`
    /// (SelfTest 模式则为 `videotestsrc`/`audiotestsrc` → appsink),
    /// 设 Playing, 由 appsink 回调捕获首帧/PTS; `recover` 先由调用方重校 lease 再重 launch.
    pub struct GStreamerPipelineController {
        plans: Mutex<HashMap<PipelineHandle, PipelinePlan>>,
        pipelines: Mutex<HashMap<PipelineHandle, gst::Pipeline>>,
    }

    impl GStreamerPipelineController {
        pub fn new() -> Self {
            Self {
                plans: Mutex::new(HashMap::new()),
                pipelines: Mutex::new(HashMap::new()),
            }
        }

        pub fn health(&self, handle: &PipelineHandle) -> Option<PipelineHealth> {
            read_health(handle)
        }

        /// 巡检 GStreamer bus (Error / EOS / StateChanged), 写回 PipelineHealth.last_error
        /// 与 acceptance 子项; 返回事件序列供 watchdog 决策 (Supervisor 仅决策, 不碰 GStreamer).
        /// 这是 #8/#9 的闭环关键: 没有真实 bus 监控, Supervisor 无法知道管线何时坏.
        pub fn poll_bus(&self, handle: &PipelineHandle) -> Vec<PipelineBusEvent> {
            let mut events = Vec::new();
            if let Some(p) = self.pipelines.lock().unwrap().get(handle) {
                if let Some(bus) = p.bus() {
                    // 必须用非阻塞 `pop()`, 不能用 `pop_filtered()` —— 后者会阻塞等到匹配类型
                    // 的消息到达; 稳定 PLAYING 的管线不再产生 Error/Eos/StateChanged, 会永久阻塞
                    // 看门狗线程 (MEDIA-RT-01 永不判定). `pop()` 队列空时立即返回 None.
                    while let Some(msg) = bus.pop() {
                        match msg.view() {
                            gst::MessageView::Error(e) => {
                                let err = e.error().to_string();
                                if let Some(h) = HEALTH_ARCS.lock().unwrap().get(handle) {
                                    let mut g = h.lock().unwrap();
                                    g.last_error = Some(err.clone());
                                    g.acceptance.c2_no_pipeline_error = false;
                                }
                                events.push(PipelineBusEvent::Error(err));
                            }
                            gst::MessageView::Eos(_) => {
                                if let Some(h) = HEALTH_ARCS.lock().unwrap().get(handle) {
                                    h.lock().unwrap().acceptance.c1_no_unexpected_eos = false;
                                }
                                events.push(PipelineBusEvent::Eos);
                            }
                            gst::MessageView::StateChanged(s) => {
                                if s.current() == gst::State::Playing {
                                    if let Some(h) = HEALTH_ARCS.lock().unwrap().get(handle) {
                                        h.lock().unwrap().acceptance.a3_pipeline_playing = true;
                                    }
                                    events.push(PipelineBusEvent::StatePlaying);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            events
        }

        /// 构造 decklinkvideosrc/audiosrc 源串 (Canonical 用 persistent-id, Diagnostic 用 device-number).
        fn src_props(plan: &PipelinePlan) -> (String, String) {
            match plan.source.selection_mode {
                SourceSelectionMode::Canonical => (
                    format!("decklinkvideosrc hw-serial-number={}", plan.source.bmd_persistent_id),
                    format!("decklinkaudiosrc hw-serial-number={}", plan.source.bmd_persistent_id),
                ),
                SourceSelectionMode::DiagnosticFallback => (
                    format!("decklinkvideosrc device-number={}", plan.source.device_number),
                    format!("decklinkaudiosrc device-number={}", plan.source.device_number),
                ),
                SourceSelectionMode::SelfTest => (
                    "videotestsrc is-live=true pattern=ball ! videoconvert".to_string(),
                    "audiotestsrc is-live=true".to_string(),
                ),
            }
        }

        /// appsink 回调: 抓首帧 + 记录 PTS + 校验单调 (video).
        fn attach_video_sink(
            pipeline: &gst::Pipeline,
            health: Arc<Mutex<PipelineHealth>>,
        ) -> Result<(), PipelineError> {
            let sink = pipeline
                .by_name("videosink")
                .ok_or_else(|| PipelineError::AppsinkMissing("videosink".into()))?
                .downcast::<AppSink>()
                .map_err(|_| PipelineError::AppsinkMissing("videosink not AppSink".into()))?;
            sink.set_callbacks(
                gstreamer_app::AppSinkCallbacks::builder()
                    .new_sample(move |appsink| {
                        let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                        if let Some(buf) = sample.buffer() {
                            let mut h = health.lock().unwrap();
                            h.video_frame_count += 1;
                            // 仅当 buffer 携带真实 PTS 时才参与首帧 / 单调判定.
                            // 无 PTS 的帧 (如部分 SDI 黑场/内嵌) 若记 0, 会与首帧的大时钟值比较
                            // 误判为非单调, 进而把 b4 翻成 false. 此处保持 monotonic=true, 不污染 B4.
                            if let Some(pts) = buf.pts().map(|c| c.nseconds()) {
                                if h.video_first_pts.is_none() {
                                    h.video_first_pts = Some(pts);
                                }
                                // 单调性: 与首帧 PTS 比较 (PTS 回退即非单调).
                                if let Some(first) = h.video_first_pts {
                                    if pts < first {
                                        h.pts_monotonic = false;
                                    }
                                }
                            }
                        }
                        Ok(gst::FlowSuccess::Ok)
                    })
                    .build(),
            );
            Ok(())
        }

        /// appsink 回调: 抓首帧 + 记录 PTS (audio).
        fn attach_audio_sink(
            pipeline: &gst::Pipeline,
            health: Arc<Mutex<PipelineHealth>>,
        ) -> Result<(), PipelineError> {
            let sink = pipeline
                .by_name("audiosink")
                .ok_or_else(|| PipelineError::AppsinkMissing("audiosink".into()))?
                .downcast::<AppSink>()
                .map_err(|_| PipelineError::AppsinkMissing("audiosink not AppSink".into()))?;
            sink.set_callbacks(
                gstreamer_app::AppSinkCallbacks::builder()
                    .new_sample(move |appsink| {
                        let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                        if let Some(buf) = sample.buffer() {
                            let mut h = health.lock().unwrap();
                            h.audio_frame_count += 1;
                            // 同上: 仅当携带真实 PTS 时才记录, 避免无 PTS 帧记 0 污染首帧时间.
                            if let Some(pts) = buf.pts().map(|c| c.nseconds()) {
                                if h.audio_first_pts.is_none() {
                                    h.audio_first_pts = Some(pts);
                                }
                            }
                        }
                        Ok(gst::FlowSuccess::Ok)
                    })
                    .build(),
            );
            Ok(())
        }
    }

    impl Default for GStreamerPipelineController {
        fn default() -> Self {
            Self::new()
        }
    }

    impl PipelineController for GStreamerPipelineController {
        fn prepare(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError> {
            gst::init().map_err(|e| PipelineError::GstreamerInit(e.to_string()))?;
            let handle = PipelineHandle(Uuid::new_v4());
            self.plans.lock().unwrap().insert(handle, plan.clone());
            Ok(handle)
        }

        fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
            let plan = self
                .plans
                .lock()
                .unwrap()
                .get(handle)
                .ok_or(PipelineError::LeaseInvalid)?
                .clone();
            let (vsrc, asrc) = Self::src_props(&plan);
            // canonical 链路: DeckLink → GStreamer → RAW (appsink 仅采样, 不提前 Encode).
            let desc = format!(
                "{vsrc} ! video/x-raw ! appsink name=videosink async=false \
                 {asrc} ! audio/x-raw ! appsink name=audiosink async=false"
            );
            let pipeline = gst::parse::launch(&desc)
                .map_err(|e| PipelineError::LaunchFailed(e.to_string()))?
                .downcast::<gst::Pipeline>()
                .map_err(|_| PipelineError::LaunchFailed("not a pipeline".into()))?;

            let health_arc = Arc::new(Mutex::new(PipelineHealth {
                running: true,
                started_at: Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0),
                ),
                ..Default::default()
            }));
            Self::attach_video_sink(&pipeline, health_arc.clone())?;
            Self::attach_audio_sink(&pipeline, health_arc.clone())?;

            pipeline
                .set_state(gst::State::Playing)
                .map_err(|e| PipelineError::LaunchFailed(format!("set_state Playing: {e:?}")))?;

            self.pipelines.lock().unwrap().insert(*handle, pipeline);
            HEALTH_ARCS.lock().unwrap().insert(*handle, health_arc);
            Ok(())
        }

        fn stop(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
            if let Some(p) = self.pipelines.lock().unwrap().remove(handle) {
                let _ = p.set_state(gst::State::Null);
            }
            HEALTH_ARCS.lock().unwrap().remove(handle);
            Ok(())
        }

        fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
            // 调用方须先重校 lease; 此处先停后起 (重 launch GStreamer).
            self.stop(handle)?;
            self.start(handle)
        }
    }
}

#[cfg(feature = "gstreamer")]
pub use gst_runtime::{read_health, GStreamerPipelineController, HEALTH_ARCS};
