//! A2-8-02-E: Program Execution Runtime——program 执行资源的**生命周期
//! 唯一 owner**（creator=destroyer, 第七轮终裁 §12.4）。
//!
//! A2-8-01 时组合根散持 group/switcher/graph/watchdog 四件（"创建的人
//! 不是销毁的人"风险）; 本对象统一治理并经 `SessionStopHook` 抽象缝接入
//! Session 停止链——**SessionManager 不理解 GStreamer/Program**（缝在
//! session.rs, 语义在.program_execution 层）。
//!
//! 生命周期序（终裁冻结）:
//! - 创建: attach taps（input 侧）→ build graph → start program →
//!   （组合根 spawn watchdog 后 `set_watchdog_stop` 注旗）;
//!   任一步失败 → **部分资源清理**（已挂 tap detach + 已建 graph stop）
//!   后返回 Err（组合根据此回滚整个会话——input/lease/resource 归
//!   SessionManager 既有机制）。
//! - 停止（`teardown`, 经 hook 于 Input 停止前触发）: watchdog 旗置位 →
//!   Program Stop → Tap Detach; **幂等**; 各步失败只记录不阻断其余步。
//!
//! 不做: 不切换（显式 Intent 链不变）·不恢复输入（Supervisor 链不变）·
//! 不持有 Session 语义（SessionInput 原样）。

use std::sync::{Arc, Mutex};

use crate::contracts::media_tap::{MediaTapPort, MediaTapRequest, TapPlanes};
use crate::contracts::switch::{ProgramExecutionObservation, SwitchExecutionAdapter};
use crate::pipeline::PipelineHandle;
use crate::program_timeline::{
    MediaPlane, TimelineAuthority, TimelineObservation, TimelinePhase, TransitionFailure,
    TransitionOutcome,
};
use crate::session::{SessionId, SessionStopHook};
use crate::switch_execution::{ExecutionGroup, SwitchError, SwitchIntent};

/// tap channel 派生约定（**唯一来源**, F-02/F-03）：DeviceId → execution
/// bridge address。**非新 identity**——仅 inter 桥接寻址（`intervideosink
/// .channel` ↔ `intervideosrc.channel`）；组合根挂 tap 与 program graph
/// 桥消费两侧均经本函数, 禁止内联重写格式（约定漂移=桥断）。
pub fn tap_channel(device_id: uuid::Uuid) -> String {
    format!("tap-{device_id}")
}

/// input 侧 tap 接线请求（channel 经 `tap_channel` 派生——唯一约定来源）。
pub struct TapWiring {
    pub input: PipelineHandle,
    pub channel: String,
}

impl TapWiring {
    /// 由 SessionInput 派生（channel=tap_channel(device_id)）。
    pub fn for_input(input: &crate::session::SessionInput) -> Self {
        Self {
            input: input.handle,
            channel: tap_channel(input.device_id),
        }
    }
}

// === A2-8-02-G/H: Observation & Timeline Evidence（第十四轮终裁） ===
// 只观测/只取证/绝不修 timestamp 行为。三列各自独立测量点 join。

/// 三列时间线采样行（Input/Bridge/Program——按 device 一行; program 列
/// 为整图共享）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimelineSample {
    pub sampled_at_ms: u64,
    pub device: uuid::Uuid,
    pub input_video_pts: Option<u64>,
    pub input_audio_pts: Option<u64>,
    pub bridge_video_pts: Option<u64>,
    pub bridge_audio_pts: Option<u64>,
    pub program_video_pts: Option<u64>,
    pub program_audio_pts: Option<u64>,
    pub input_video_state: crate::pipeline::PtsMonotonicity,
    pub input_audio_state: crate::pipeline::PtsMonotonicity,
    pub bridge_video_state: crate::pipeline::PtsMonotonicity,
    pub bridge_audio_state: crate::pipeline::PtsMonotonicity,
    pub program_video_state: crate::pipeline::PtsMonotonicity,
    pub program_audio_state: crate::pipeline::PtsMonotonicity,
    pub program_alive: bool,
}

/// 三列 join（各列独立测量: 输入=输入管线健康弧; 桥=BridgeObservation
/// 行[调用方按 tap_channel(device) 选行]; 程序=ProgramObservation）。
pub fn assemble_timeline_sample(
    device: uuid::Uuid,
    input_health: Option<&crate::pipeline::PipelineHealth>,
    bridge: Option<&crate::contracts::media_tap::BridgeObservation>,
    program: &crate::contracts::switch::ProgramObservation,
) -> TimelineSample {
    TimelineSample {
        sampled_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        device,
        input_video_pts: input_health.and_then(|h| h.video_last_pts),
        input_audio_pts: input_health.and_then(|h| h.audio_last_pts),
        bridge_video_pts: bridge.and_then(|b| b.video_last_pts),
        bridge_audio_pts: bridge.and_then(|b| b.audio_last_pts),
        program_video_pts: program.program_video_pts,
        program_audio_pts: program.program_audio_pts,
        input_video_state: input_health
            .map(|h| h.video_pts_state)
            .unwrap_or(crate::pipeline::PtsMonotonicity::Unknown),
        input_audio_state: input_health
            .map(|h| h.audio_pts_state)
            .unwrap_or(crate::pipeline::PtsMonotonicity::Unknown),
        bridge_video_state: bridge
            .map(|b| b.video_pts_state)
            .unwrap_or(crate::pipeline::PtsMonotonicity::Unknown),
        bridge_audio_state: bridge
            .map(|b| b.audio_pts_state)
            .unwrap_or(crate::pipeline::PtsMonotonicity::Unknown),
        program_video_state: program.program_video_pts_state,
        program_audio_state: program.program_audio_pts_state,
        program_alive: program.program_video_pts.is_some()
            && program.program_video_pts_state != crate::pipeline::PtsMonotonicity::NonMonotonic,
    }
}

/// recover 后桥健康结构化报告（RECOVER_PARTIAL_DEGRADED 观测化——
/// **不改 recover 返回类型**, 由观测查询组装: recover Ok + 簿记重放 ≠
/// 桥真实流通; degraded = 恢复成功但期望 channel 无实测数据流通）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeHealthReport {
    pub pipeline_recovered: bool,
    pub expected_channels: Vec<String>,
    pub observed_alive_channels: Vec<String>,
    pub bridge_degraded: bool,
}

/// alive = **当前推进性**（G/H-1, 第十五轮 §5-7: `frames>0` 只证"曾经
/// 活过"——必须以观察时钟窗口判定: now - last_observed ≤ window。
/// 历史证据 frames 与活性证据 last_observed 分层, 禁混）。
pub fn assemble_bridge_health(
    pipeline_recovered: bool,
    expected_channels: Vec<String>,
    liveness: &[crate::contracts::media_tap::BridgeChannelLiveness],
) -> BridgeHealthReport {
    let observed_alive_channels: Vec<String> = liveness
        .iter()
        .filter(|l| l.alive_in_window)
        .map(|l| l.channel.clone())
        .collect();
    let bridge_degraded = pipeline_recovered
        && expected_channels
            .iter()
            .any(|c| !observed_alive_channels.contains(c));
    BridgeHealthReport {
        pipeline_recovered,
        expected_channels,
        observed_alive_channels,
        bridge_degraded,
    }
}

// G/H-1（第十五轮 §11）: "曾经活过 ≠ 当前推进"——以采样增量分离历史
// 存在与当前推进（不改 ProgramObservation/PipelineHealth 契约）。

/// 程序出口当前推进（两次观测间帧计数增长）。
pub fn program_progress_since(
    prev: &crate::contracts::switch::ProgramObservation,
    cur: &crate::contracts::switch::ProgramObservation,
) -> bool {
    cur.program_video_frames > prev.program_video_frames
        || cur.program_audio_frames > prev.program_audio_frames
}

/// 输入管线当前推进（两次健康弧快照间帧计数增长）。
pub fn input_progress_since(
    prev: &crate::pipeline::PipelineHealth,
    cur: &crate::pipeline::PipelineHealth,
) -> bool {
    cur.video_frame_count > prev.video_frame_count || cur.audio_frame_count > prev.audio_frame_count
}

// === A2-8-04: 六路逐平面连续性取证（R51 Unit 1——observation only） ===
// R50 OQ-T3 修订后冻结: `program_progress_since`/`input_progress_since`
// 为聚合 A/V "或"——不能证六路逐平面推进。本面逐路独立记账, **只测量
// 不判定**（无阈值/无判据/不触 L4——判据属取证后的验收层）。absence≠
// false: `advanced=None`=无可比帧计数证据, 与 `Some(false)`=有证据未
// 推进严格分离。

/// 取样所处切换阶段标签（调用方时序标注, 纯数据——无判定语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidencePhase {
    PreSwitch,
    PostSwitch,
}

/// 单路证据行: PTS + 单调态 + 帧计数 + 相对前一采样的推进证据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathEvidence {
    pub pts: Option<u64>,
    pub pts_state: crate::pipeline::PtsMonotonicity,
    /// 该路帧计数（观测行缺席=None——如 bridge 无对应行）。
    pub frames: Option<u64>,
    /// 相对前一采样推进: None=无可比证据（absence≠false）。
    pub advanced: Option<bool>,
}

/// 六路证据行（input/bridge/program × video/audio——按 device 一行;
/// program 列为整图共享）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SixPathEvidence {
    pub sampled_at_ms: u64,
    pub device: uuid::Uuid,
    pub phase: EvidencePhase,
    /// 采样时 adapter 侧切换计数（`ProgramObservation.switch_epoch` 同源）。
    pub switch_epoch: u64,
    pub input_video: PathEvidence,
    pub input_audio: PathEvidence,
    pub bridge_video: PathEvidence,
    pub bridge_audio: PathEvidence,
    pub program_video: PathEvidence,
    pub program_audio: PathEvidence,
    /// T2（R50 冻结）: 只测量 `|program_v_pts − program_a_pts|`——ns 可比
    /// ≠阈值授权（阈值禁令维持）; 任一平面无 PTS=None。
    pub program_av_delta_ns: Option<u64>,
}

/// 三列原始观测借用面（与 `assemble_timeline_sample` 同源三面: 输入=
/// 输入管线健康弧; 桥=BridgeObservation 行[按 tap_channel 选行];
/// 程序=ProgramObservation）。
pub struct SixPathInputs<'a> {
    pub input: Option<&'a crate::pipeline::PipelineHealth>,
    pub bridge: Option<&'a crate::contracts::media_tap::BridgeObservation>,
    pub program: &'a crate::contracts::switch::ProgramObservation,
}

/// 单路装配: 行缺席→全 None/Unknown + advanced=None; 帧计数双方在场
/// 才产出推进证据（absence≠false）。
fn path_row(
    cur: Option<(Option<u64>, crate::pipeline::PtsMonotonicity, Option<u64>)>,
    prev_frames: Option<Option<u64>>,
) -> PathEvidence {
    PathEvidence {
        pts: cur.as_ref().and_then(|c| c.0),
        pts_state: cur
            .map(|c| c.1)
            .unwrap_or(crate::pipeline::PtsMonotonicity::Unknown),
        frames: cur.and_then(|c| c.2),
        advanced: match (prev_frames.flatten(), cur.and_then(|c| c.2)) {
            (Some(p), Some(c)) => Some(c > p),
            _ => None,
        },
    }
}

/// 六路证据装配（纯函数——两快照 join, 只取证不判定）。
pub fn assemble_six_path_evidence(
    device: uuid::Uuid,
    phase: EvidencePhase,
    prev: Option<&SixPathInputs<'_>>,
    cur: &SixPathInputs<'_>,
) -> SixPathEvidence {
    // 输入列: 行在=健康弧在（帧计数恒 u64, 非 Option 源）。
    let cur_input = |video: bool| {
        cur.input.map(|h| {
            if video {
                (
                    h.video_last_pts,
                    h.video_pts_state,
                    Some(h.video_frame_count),
                )
            } else {
                (
                    h.audio_last_pts,
                    h.audio_pts_state,
                    Some(h.audio_frame_count),
                )
            }
        })
    };
    let prev_input = |video: bool| {
        prev.and_then(|p| p.input).map(|h| {
            Some(if video {
                h.video_frame_count
            } else {
                h.audio_frame_count
            })
        })
    };
    // 桥列: 行缺席=无 bridge_observation 匹配行（frames None→advanced None）。
    let cur_bridge = |video: bool| {
        cur.bridge.map(|b| {
            if video {
                (b.video_last_pts, b.video_pts_state, Some(b.video_frames))
            } else {
                (b.audio_last_pts, b.audio_pts_state, Some(b.audio_frames))
            }
        })
    };
    let prev_bridge = |video: bool| {
        prev.and_then(|p| p.bridge).map(|b| {
            Some(if video {
                b.video_frames
            } else {
                b.audio_frames
            })
        })
    };
    // 程序列: 行恒在（ProgramObservation 必有）。
    let cur_prog = |video: bool| {
        Some(if video {
            (
                cur.program.program_video_pts,
                cur.program.program_video_pts_state,
                Some(cur.program.program_video_frames),
            )
        } else {
            (
                cur.program.program_audio_pts,
                cur.program.program_audio_pts_state,
                Some(cur.program.program_audio_frames),
            )
        })
    };
    let prev_prog = |video: bool| {
        prev.map(|p| {
            Some(if video {
                p.program.program_video_frames
            } else {
                p.program.program_audio_frames
            })
        })
    };
    SixPathEvidence {
        sampled_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        device,
        phase,
        switch_epoch: cur.program.switch_epoch,
        input_video: path_row(cur_input(true), prev_input(true)),
        input_audio: path_row(cur_input(false), prev_input(false)),
        bridge_video: path_row(cur_bridge(true), prev_bridge(true)),
        bridge_audio: path_row(cur_bridge(false), prev_bridge(false)),
        program_video: path_row(cur_prog(true), prev_prog(true)),
        program_audio: path_row(cur_prog(false), prev_prog(false)),
        program_av_delta_ns: match (cur.program.program_video_pts, cur.program.program_audio_pts) {
            (Some(v), Some(a)) => Some(v.abs_diff(a)),
            _ => None,
        },
    }
}

/// 故障域分类（G/H ④: Input/Bridge/Program 组合观测——单故障假设,
/// 优先序 Input>Bridge>Program; 多重并发故障如实报首因不做多维归因）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureDomain {
    None,
    Input,
    Bridge,
    Program,
}

pub fn classify_failure_domain(
    input_advancing: bool,
    bridge_alive: bool,
    program_advancing: bool,
) -> FailureDomain {
    if !input_advancing {
        FailureDomain::Input
    } else if !bridge_alive {
        FailureDomain::Bridge
    } else if !program_advancing {
        FailureDomain::Program
    } else {
        FailureDomain::None
    }
}

/// 03-01-E（R45）: 运行时故障域分类的桥 liveness 观察窗——与 gate L5
/// （gates/dual_input.rs `LIVENESS_WINDOW_MS`=3000）同值同义, 分类器喂入
/// 口径一致。常量在本模块定义（gates→runtime 依赖禁反转; gate 局部常量
/// 保持不动）。
pub const FAILURE_DOMAIN_LIVENESS_WINDOW_MS: u64 = 3000;

struct Inner {
    group: Arc<Mutex<ExecutionGroup>>,
    switcher: Arc<dyn SwitchExecutionAdapter>,
    graph: PipelineHandle,
    /// 已挂 tap 簿记（input handle + channel）——teardown 时 detach。
    taps: Vec<(PipelineHandle, String)>,
    tap_port: Option<Arc<dyn MediaTapPort>>,
    watchdog_stop: Option<Arc<std::sync::atomic::AtomicBool>>,
    /// C-TIMELINE-01 ⑨（Freeze §1 四组件并列）: Program Timeline Authority
    /// ——Domain 状态机; **不是第二个 switch state machine**（switch_epoch
    /// 归 ExecutionGroup, program_epoch/segment 归本 Authority——经
    /// SwitchExecutionPlan/SwitchExecuted 关联, 各自拥有自己的状态）。
    timeline: TimelineAuthority,
}

/// Program Execution Runtime（组合根装配后为 program 资源唯一 owner）。
pub struct ProgramExecutionRuntime {
    session_id: SessionId,
    inner: Mutex<Option<Inner>>,
}

impl std::fmt::Debug for ProgramExecutionRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgramExecutionRuntime")
            .field("session_id", &self.session_id)
            .field("active", &self.is_active())
            .finish()
    }
}

impl ProgramExecutionRuntime {
    /// 组合根装配（creator=destroyer）。任一步失败 → 部分资源清理后 Err。
    pub fn create(
        session_id: SessionId,
        group: ExecutionGroup,
        switcher: Arc<dyn SwitchExecutionAdapter>,
        tap_port: Option<Arc<dyn MediaTapPort>>,
        tap_wirings: Vec<TapWiring>,
    ) -> Result<Self, SwitchError> {
        // 第八轮终裁 P1: session/group 身份一致性 fail-closed（组合根当前
        // 同源 sid 不会错——类型面强制不变量, 防未来调用方构造
        // Runtime=A/Group=B 的分裂态; 不引入新 identity 类型）。
        if session_id != group.session_id {
            return Err(SwitchError::Backend(format!(
                "session/group 身份不一致 (runtime={} group={})——fail-closed",
                session_id.0, group.session_id.0
            )));
        }
        // 步 1: input 侧 tap attach（失败 → 已挂部分全部 detach）。
        let mut attached: Vec<(PipelineHandle, String)> = Vec::new();
        if let Some(port) = tap_port.as_ref() {
            for w in &tap_wirings {
                let req = MediaTapRequest {
                    channel: w.channel.clone(),
                    planes: TapPlanes::Both,
                };
                match port.attach_media_tap(&w.input, &req) {
                    Ok(()) => attached.push((w.input, w.channel.clone())),
                    Err(e) => {
                        for (h, ch) in &attached {
                            let _ = port.detach_media_tap(h, ch);
                        }
                        return Err(SwitchError::Backend(format!(
                            "tap attach 失败（部分已清理）: {e}"
                        )));
                    }
                }
            }
        }
        // 步 2+3: graph 物化 + 启动（失败 → tap 清理 + 已建 graph 停止）。
        // C-TIMELINE-01 ⑨: Authority 以初始 active 源锚定（epoch 0 恒等段）。
        let initial_active = match group.desired {
            crate::switch_execution::SwitchDesired::ActiveInput(a) => a,
            switching @ crate::switch_execution::SwitchDesired::Switching { .. } => {
                return Err(SwitchError::NotActiveSource(switching))
            }
        };
        let timeline = TimelineAuthority::new(initial_active);
        let group = Arc::new(Mutex::new(group));
        let graph = {
            let g = group.lock().unwrap();
            switcher.build_program_graph(&g)
        };
        match graph {
            Ok(graph) => match switcher.start_program(&graph) {
                Ok(()) => Ok(Self {
                    session_id,
                    inner: Mutex::new(Some(Inner {
                        group,
                        switcher,
                        graph,
                        taps: attached,
                        tap_port,
                        watchdog_stop: None,
                        timeline,
                    })),
                }),
                Err(e) => {
                    Self::cleanup_partial(&switcher, Some(&graph), &attached, tap_port.as_ref());
                    Err(e)
                }
            },
            Err(e) => {
                Self::cleanup_partial(&switcher, None, &attached, tap_port.as_ref());
                Err(e)
            }
        }
    }

    /// 部分资源清理（创建失败路径）: 已建 graph 停止 + 已挂 tap detach。
    fn cleanup_partial(
        switcher: &Arc<dyn SwitchExecutionAdapter>,
        graph: Option<&PipelineHandle>,
        attached: &[(PipelineHandle, String)],
        tap_port: Option<&Arc<dyn MediaTapPort>>,
    ) {
        if let Some(g) = graph {
            if let Err(e) = switcher.stop_program(g) {
                tracing::warn!(error = ?e, "A2-8-02-E 创建失败清理: graph 停止失败（残留风险已记录）");
            }
        }
        if let Some(port) = tap_port {
            for (h, ch) in attached {
                if let Err(e) = port.detach_media_tap(h, ch) {
                    tracing::warn!(error = ?e, channel = %ch, "A2-8-02-E 创建失败清理: tap detach 失败");
                }
            }
        }
    }

    /// 停止序: watchdog 旗 → Program Stop → Tap Detach。幂等（已 teardown
    /// = no-op）。各步失败只记录——**不因 Program 停止失败截断 Session
    /// 停止链**（hook 调用方保证; 本函数不向上传播错误）。
    pub fn teardown(&self) {
        let Some(inner) = self.inner.lock().unwrap().take() else {
            return;
        };
        if let Some(flag) = &inner.watchdog_stop {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if let Err(e) = inner.switcher.stop_program(&inner.graph) {
            tracing::warn!(error = ?e, "A2-8-02-E teardown: Program Stop 失败（记录不阻断 Tap Detach）");
        }
        if let Some(port) = inner.tap_port.as_ref() {
            for (h, ch) in &inner.taps {
                if let Err(e) = port.detach_media_tap(h, ch) {
                    tracing::warn!(error = ?e, channel = %ch, "A2-8-02-E teardown: Tap Detach 失败");
                }
            }
        }
        tracing::info!(
            session = %self.session_id.0,
            graph = inner.graph.0,
            "A2-8-02-E Program Execution Runtime teardown 完成（Program Stop→Tap Detach）"
        );
    }

    /// program 执行资源是否仍存活。
    pub fn is_active(&self) -> bool {
        self.inner.lock().unwrap().is_some()
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// 组合根接线访问器（watchdog spawn 需要 graph/group/switcher）。
    pub fn graph_handle(&self) -> Option<PipelineHandle> {
        self.inner.lock().unwrap().as_ref().map(|i| i.graph)
    }

    pub fn group_arc(&self) -> Option<Arc<Mutex<ExecutionGroup>>> {
        self.inner.lock().unwrap().as_ref().map(|i| i.group.clone())
    }

    pub fn switcher_arc(&self) -> Option<Arc<dyn SwitchExecutionAdapter>> {
        self.inner
            .lock()
            .unwrap()
            .as_ref()
            .map(|i| i.switcher.clone())
    }

    /// watchdog spawn 后注入停止旗（teardown 置位）。
    pub fn set_watchdog_stop(&self, flag: Arc<std::sync::atomic::AtomicBool>) {
        if let Some(inner) = self.inner.lock().unwrap().as_mut() {
            inner.watchdog_stop = Some(flag);
        }
    }

    // ── C-TIMELINE-01 ⑩⑪: timeline orchestration（①-⑩ 全链 owner）────

    /// ①-⑩ 全链切换（timeline orchestration）。实现纪律（第三十一轮 §十六
    /// 照录）: **TimelineAuthority 产生"应该怎样映射"的声明; selector
    /// downstream Event/Buffer 产生"实际上发生了什么"的证据; 两者在本
    /// Runtime 中闭合成 TimelineMapped**。
    ///
    /// - ① 基准+锚采样（Adapter 观测——offset 归 Authority 声明）;
    /// - ② Authority 声明（fail-closed）; ③ pre-flip install（只安装）;
    /// - ④ group begin→adapter switch（失败→timeline abort+传播）;
    /// - ⑤⑥⑦⑧ 证据收集（adapter facts→Authority 校验闭合; 超时/矛盾=
    ///   FailClosed——"evidence 不足"不猜测成功）;
    /// - ⑨ settle 稳定窗（映射后连续观测; 停滞超时不 FailClosed——停滞=
    ///   Observation 事实归 watchdog/Gate 故障面, 时间线证据已闭合）;
    /// - ⑩ confirm_settled + Desired 推进（Observed 驱动——非命令回显）。
    pub fn switch_program(
        &self,
        intent: &SwitchIntent,
    ) -> Result<ProgramSwitchReport, SwitchError> {
        let mut inner_guard = self.inner.lock().unwrap();
        let Some(inner) = inner_guard.as_mut() else {
            return Err(SwitchError::Backend(
                "runtime 未激活（已 teardown）——切换拒收".into(),
            ));
        };
        // ①a 连续性基准（pre-flip program 实测位置——Authority 连续性校验基准）。
        let pre = inner.switcher.observe(&inner.graph).program;
        if let Some(v) = pre.program_video_pts {
            inner
                .timeline
                .on_program_pts(MediaPlane::Video, v)
                .map_err(timeline_fail_closed)?;
        }
        if let Some(a) = pre.program_audio_pts {
            inner
                .timeline
                .on_program_pts(MediaPlane::Audio, a)
                .map_err(timeline_fail_closed)?;
        }
        // ①b 执行计划（零状态变化——与既有显式链同语义）+ 锚采样。
        let execution_plan = inner.group.lock().unwrap().plan_switch(intent)?;
        let anchors = inner
            .switcher
            .sample_switch_anchors(&inner.graph, intent.target)?;
        // ② Authority 声明（唯一 offset 生产点）。
        let plan = inner
            .timeline
            .declare_transition(
                intent.target,
                execution_plan.epoch,
                anchors.video,
                anchors.audio,
            )
            .map_err(|e| SwitchError::Backend(format!("timeline ② 声明 fail-closed: {e}")))?;
        // ③ pre-flip install（Adapter 执行态——非 TimelineMapped）。
        inner
            .switcher
            .install_timeline_transition(&inner.graph, &plan)?;
        // ④ 执行（既有显式链不动: begin→switch; 失败→abort+传播）。
        inner.group.lock().unwrap().begin_switch(&execution_plan)?;
        let executed = match inner.switcher.switch(&inner.graph, &execution_plan) {
            Ok(ex) => ex,
            Err(e) => {
                let _ = inner.timeline.abort_transition();
                return Err(e);
            }
        };
        inner
            .timeline
            .on_switch_executed(executed.av_epoch)
            .map_err(|e| SwitchError::Backend(format!("timeline ④ 联动 fail-closed: {e}")))?;
        // ⑤⑥⑦⑧ 证据收集（poll adapter observe[驱动 Mock tick]+facts→Authority）。
        let evidence_deadline = std::time::Instant::now() + TIMELINE_EVIDENCE_TIMEOUT;
        loop {
            let _ = inner.switcher.observe(&inner.graph);
            let facts = inner.switcher.timeline_execution_facts(&inner.graph);
            if let Some(facts) = facts {
                feed_authority(inner, &facts, intent.target);
            }
            match inner.timeline.phase() {
                TimelinePhase::TimelineTransition { .. } => break,
                TimelinePhase::TransitionFailed { reason } => {
                    return Err(SwitchError::Backend(format!(
                        "timeline FailClosed（证据/矛盾）: {reason}"
                    )));
                }
                _ => {}
            }
            if std::time::Instant::now() >= evidence_deadline {
                let reason = TransitionFailure::EvidenceInsufficient {
                    pending: pending_planes(inner),
                };
                let _ = inner.timeline.fail_closed(reason.clone());
                return Err(SwitchError::Backend(format!(
                    "timeline 证据超时 FailClosed: {reason}"
                )));
            }
            std::thread::sleep(TIMELINE_POLL_INTERVAL);
        }
        // ⑨ settle: 映射后连续观测稳定窗（回退→Authority FailClosed）。
        let settle_deadline = std::time::Instant::now() + TIMELINE_SETTLE_TIMEOUT;
        let mut stable_rounds = 0u32;
        let mut last: Option<(Option<u64>, Option<u64>)> = None;
        loop {
            let obs = inner.switcher.observe(&inner.graph).program;
            if let Some(v) = obs.program_video_pts {
                inner
                    .timeline
                    .on_program_pts(MediaPlane::Video, v)
                    .map_err(timeline_fail_closed)?;
            }
            if let Some(a) = obs.program_audio_pts {
                inner
                    .timeline
                    .on_program_pts(MediaPlane::Audio, a)
                    .map_err(timeline_fail_closed)?;
            }
            let cur = (obs.program_video_pts, obs.program_audio_pts);
            let advancing = match (last, cur) {
                (Some((lp, la)), (cp, ca)) => {
                    cp.is_some_and(|v| lp.is_some_and(|l| v > l))
                        || ca.is_some_and(|a| la.is_some_and(|l| a > l))
                }
                _ => true, // 首观测轮计稳定（无前值——absence≠停滞）
            };
            if advancing {
                stable_rounds += 1;
            }
            last = Some(cur);
            if stable_rounds >= TIMELINE_SETTLE_ROUNDS
                || std::time::Instant::now() >= settle_deadline
            {
                break; // 停滞超时不 FailClosed（停滞归故障面; 时间线证据已闭合）
            }
            std::thread::sleep(TIMELINE_POLL_INTERVAL);
        }
        // ⑩ settle 落定 + Desired 推进（Observed 驱动）。
        let outcome = inner
            .timeline
            .confirm_settled()
            .map_err(|e| SwitchError::Backend(format!("timeline ⑩ settle fail-closed: {e}")))?;
        {
            let mut g = inner.group.lock().unwrap();
            if let Some(o) = inner.switcher.observe(&inner.graph).program.observed_active {
                g.complete_switch(o);
            }
        }
        let observation = inner.timeline.snapshot(now_observed_ms());
        Ok(ProgramSwitchReport {
            executed,
            outcome,
            observation,
        })
    }

    /// ⑪ 裁决级 observation 组合面: program=adapter 既有平面; timeline=
    /// **Authority snapshot**（Domain SoT——epoch/段/连续性恒当前; adapter
    /// 行=执行侧原始证据）。
    pub fn observe_execution(&self) -> Option<ProgramExecutionObservation> {
        let inner = self.inner.lock().unwrap();
        let inner = inner.as_ref()?;
        Some(ProgramExecutionObservation {
            program: inner.switcher.observe(&inner.graph).program,
            timeline: inner.timeline.snapshot(now_observed_ms()),
        })
    }
}

/// C-TIMELINE-01 轮询参数（观察层节流——非时间线语义; 常量不 IO 等待媒体）。
const TIMELINE_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
const TIMELINE_EVIDENCE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const TIMELINE_SETTLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const TIMELINE_SETTLE_ROUNDS: u32 = 3;

fn now_observed_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn timeline_fail_closed(e: TransitionFailure) -> SwitchError {
    SwitchError::Backend(format!("timeline FailClosed（观测/状态机）: {e}"))
}

/// ⑤⑥⑦ 证据喂入（幂等——每轮只推进尚在途的平面; FailClosed 由相位检查上抛）。
fn feed_authority(
    inner: &mut Inner,
    facts: &crate::contracts::switch::TimelineExecutionFacts,
    target: uuid::Uuid,
) {
    let feed_plane = |inner: &mut Inner,
                      plane: MediaPlane,
                      f: &crate::contracts::switch::PlaneExecutionFacts| {
        if f.segment_observed
            && matches!(
                inner.timeline.plane(plane).transition,
                Some(crate::program_timeline::PlaneTransitionState::AwaitSegmentEvent)
            )
        {
            let _ = inner.timeline.on_segment_event(plane, target);
        }
        if let Some((src, mapped)) = f.first_mapped {
            if matches!(
                inner.timeline.plane(plane).transition,
                Some(crate::program_timeline::PlaneTransitionState::AwaitFirstMappedBuffer)
            ) {
                let _ = inner.timeline.on_mapped_buffer(plane, target, src, mapped);
            }
        }
    };
    feed_plane(inner, MediaPlane::Video, &facts.video);
    feed_plane(inner, MediaPlane::Audio, &facts.audio);
}

fn pending_planes(inner: &Inner) -> Vec<MediaPlane> {
    [MediaPlane::Video, MediaPlane::Audio]
        .into_iter()
        .filter(|p| {
            !matches!(
                inner.timeline.plane(*p).transition,
                Some(crate::program_timeline::PlaneTransitionState::Mapped)
            )
        })
        .collect()
}

impl SessionStopHook for ProgramExecutionRuntime {
    fn on_session_stopping(&self, id: &SessionId) -> Result<(), String> {
        if *id == self.session_id {
            self.teardown();
        }
        Ok(())
    }
}

/// C-TIMELINE-01 ⑩: 全链切换报告（switch 执行证据 + timeline 结局 +
/// Authority 快照行——L4-TIMELINE 九项合取的输入）。
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramSwitchReport {
    pub executed: crate::contracts::switch::SwitchExecuted,
    pub outcome: TransitionOutcome,
    pub observation: TimelineObservation,
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::adapters::mock::{MockBackend, MockMediaTapPort};
    use crate::adapters::switch_mock::MockSwitchExecutionAdapter;
    use crate::contracts::backend::MediaBackend;
    use crate::contracts::media_tap::MediaTapPort;
    use crate::pipeline::PipelinePlan;
    use crate::session::{SessionId, SessionInput};
    use crate::switch_execution::SwitchDesired;
    use uuid::Uuid;

    fn dual_group(session_id: SessionId, a: Uuid, b: Uuid) -> ExecutionGroup {
        let backend = MockBackend;
        let h1 = backend.instantiate(&PipelinePlan::self_test()).unwrap();
        let h2 = backend.instantiate(&PipelinePlan::self_test()).unwrap();
        ExecutionGroup::new(
            session_id,
            vec![
                SessionInput {
                    device_id: a,
                    handle: h1,
                },
                SessionInput {
                    device_id: b,
                    handle: h2,
                },
            ],
            a,
        )
        .unwrap()
    }

    struct FailingSwitcher;
    impl SwitchExecutionAdapter for FailingSwitcher {
        fn build_program_graph(
            &self,
            _group: &ExecutionGroup,
        ) -> Result<PipelineHandle, SwitchError> {
            Err(SwitchError::Backend("注入: graph 物化失败".into()))
        }
        fn start_program(&self, _g: &PipelineHandle) -> Result<(), SwitchError> {
            Ok(())
        }
        fn switch(
            &self,
            _g: &PipelineHandle,
            _p: &crate::switch_execution::SwitchExecutionPlan,
        ) -> Result<crate::contracts::switch::SwitchExecuted, SwitchError> {
            unreachable!("失败注入不用于切换")
        }
        fn observe(
            &self,
            _g: &PipelineHandle,
        ) -> crate::contracts::switch::ProgramExecutionObservation {
            unreachable!("失败注入不用于观测")
        }
        fn stop_program(&self, _g: &PipelineHandle) -> Result<(), SwitchError> {
            Ok(())
        }
    }

    #[test]
    fn program_exec_rt_01_create_teardown_idempotent() {
        // 正常全序: create[taps attach→graph→start] → teardown[Program
        // Stop→Tap Detach] 幂等; 观测面可证（tap 簿记清空+graph 停止后
        // observe 归零——非仅内部标志）。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let sid = SessionId(Uuid::new_v4());
        let group = dual_group(sid, a, b);
        let h1 = group.inputs[0].handle;
        let h2 = group.inputs[1].handle;
        let switcher = Arc::new(MockSwitchExecutionAdapter::new());
        let taps = Arc::new(MockMediaTapPort::new());
        let runtime = ProgramExecutionRuntime::create(
            sid,
            group,
            switcher.clone(),
            Some(taps.clone()),
            vec![
                TapWiring {
                    input: h1,
                    channel: format!("dev-{}-raw", a),
                },
                TapWiring {
                    input: h2,
                    channel: format!("dev-{}-raw", b),
                },
            ],
        )
        .expect("创建成功");
        assert!(runtime.is_active());
        assert_eq!(
            taps.tap_attachments(&h1).len() + taps.tap_attachments(&h2).len(),
            2
        );
        let graph = runtime.graph_handle().expect("graph 在");
        assert!(
            switcher.observe(&graph).program.observed_active.is_some(),
            "program 运行中（观测面）"
        );

        runtime.teardown();
        assert!(!runtime.is_active(), "teardown 后失活");
        assert!(
            taps.tap_attachments(&h1).is_empty() && taps.tap_attachments(&h2).is_empty(),
            "Tap Detach 完成（簿记清空）"
        );
        assert!(
            switcher.observe(&graph).program.observed_active.is_none(),
            "Program Stop 完成（observe 归零）"
        );
        runtime.teardown(); // 幂等
        assert!(!runtime.is_active());
    }

    #[test]
    fn program_exec_rt_01_create_failure_cleans_partial_taps() {
        // 创建失败（graph 物化注入失败）: 已 attach 的 tap 必须全部清理
        // ——零部分资源残留。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let sid = SessionId(Uuid::new_v4());
        let group = dual_group(sid, a, b);
        let h1 = group.inputs[0].handle;
        let h2 = group.inputs[1].handle;
        let taps = Arc::new(MockMediaTapPort::new());
        let err = ProgramExecutionRuntime::create(
            sid,
            group,
            Arc::new(FailingSwitcher),
            Some(taps.clone()),
            vec![
                TapWiring {
                    input: h1,
                    channel: "dev-f1".into(),
                },
                TapWiring {
                    input: h2,
                    channel: "dev-f2".into(),
                },
            ],
        )
        .expect_err("注入失败应传播");
        assert!(matches!(err, SwitchError::Backend(_)));
        assert!(
            taps.tap_attachments(&h1).is_empty() && taps.tap_attachments(&h2).is_empty(),
            "部分资源已清理（tap 零残留）"
        );
    }

    #[test]
    fn program_exec_rt_01_stop_hook_scoped_to_own_session() {
        // hook 仅对本 session 触发 teardown（他 session 停止零副作用）。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let sid = SessionId(Uuid::new_v4());
        let runtime = ProgramExecutionRuntime::create(
            sid,
            dual_group(sid, a, b),
            Arc::new(MockSwitchExecutionAdapter::new()),
            None,
            Vec::new(),
        )
        .expect("创建");
        let own = *runtime.session_id();
        let other = SessionId(Uuid::new_v4());
        SessionStopHook::on_session_stopping(&runtime, &other).unwrap();
        assert!(runtime.is_active(), "他 session 停止不触发");
        SessionStopHook::on_session_stopping(&runtime, &own).unwrap();
        assert!(!runtime.is_active(), "本 session 停止触发 teardown");
        let _ = SwitchDesired::ActiveInput(a); // 引用锚（模块语义完整性）
    }

    #[test]
    fn program_exec_rt_01_session_group_identity_mismatch_rejected() {
        // 第八轮终裁 P1: runtime.session_id ≠ group.session_id → fail-closed
        // 拒收（防未来调用方构造 Runtime=A/Group=B 分裂态; 不引入新
        // identity 类型——仅既有身份的一致性强制）。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let group = dual_group(SessionId(Uuid::new_v4()), a, b);
        let mismatch = SessionId(Uuid::new_v4()); // ≠ group.session_id
        let err = ProgramExecutionRuntime::create(
            mismatch,
            group,
            Arc::new(MockSwitchExecutionAdapter::new()),
            None,
            Vec::new(),
        )
        .expect_err("身份不一致必须拒收");
        assert!(matches!(err, SwitchError::Backend(_)));
        assert!(
            err.to_string().contains("身份不一致"),
            "错误信息可观测: {err}"
        );
    }

    // ── A2-8-02-G/H: 三列采样/桥健康报告/故障域分类（第十四轮） ──────

    #[test]
    fn gh_rt_01_timeline_sample_three_independent_columns() {
        // 三列各自独立测量点 join: 六 PTS 列互异（非复制——第十四轮 §4
        // "三列实际两份数据"反证）; 缺席列=Unknown 非伪造。
        use crate::contracts::media_tap::BridgeObservation;
        use crate::contracts::switch::ProgramObservation;
        use crate::pipeline::{PipelineHealth, PtsMonotonicity};
        let device = Uuid::new_v4();
        let input = PipelineHealth {
            video_last_pts: Some(1111),
            audio_last_pts: Some(2222),
            video_pts_state: PtsMonotonicity::ValidMonotonic,
            audio_pts_state: PtsMonotonicity::ValidMonotonic,
            ..Default::default()
        };
        let bridge = BridgeObservation {
            channel: tap_channel(device),
            video_last_pts: Some(3333),
            audio_last_pts: Some(4444),
            video_pts_state: PtsMonotonicity::ValidMonotonic,
            audio_pts_state: PtsMonotonicity::ValidMonotonic,
            video_frames: 99,
            audio_frames: 198,
        };
        let program = ProgramObservation {
            observed_active: Some(device),
            video_active: Some(device),
            audio_active: Some(device),
            switch_epoch: 0,
            input_pts: Vec::new(),
            program_video_pts: Some(5555),
            program_audio_pts: Some(6666),
            program_video_pts_state: PtsMonotonicity::ValidMonotonic,
            program_audio_pts_state: PtsMonotonicity::ValidMonotonic,
            program_video_frames: 42,
            program_audio_frames: 84,
        };
        let s = assemble_timeline_sample(device, Some(&input), Some(&bridge), &program);
        let cols = [
            s.input_video_pts,
            s.input_audio_pts,
            s.bridge_video_pts,
            s.bridge_audio_pts,
            s.program_video_pts,
            s.program_audio_pts,
        ];
        assert!(cols.iter().all(|c| c.is_some()), "六列全在场");
        let vals: Vec<u64> = cols.iter().map(|c| c.unwrap()).collect();
        let mut sorted = vals.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 6, "六列值互异——独立测量非复制");
        assert!(s.program_alive);
        // 桥缺席列 = Unknown（absence≠evidence）。
        let s2 = assemble_timeline_sample(device, Some(&input), None, &program);
        assert_eq!(s2.bridge_video_state, PtsMonotonicity::Unknown);
        assert_eq!(s2.bridge_video_pts, None);
    }

    #[test]
    fn gh_rt_01_bridge_health_degraded_detection() {
        // RECOVER_PARTIAL_DEGRADED 观测化（G/H-1 升级 liveness 基）:
        // "曾经活过"（frames>0 但窗口外）≠ 当前流通——b 有历史帧但断流
        // → degraded=true（帧基判定会漏报, 第十五轮 §5 实证场景）。
        use crate::contracts::media_tap::BridgeChannelLiveness;
        let now = 10_000u64;
        let row =
            |ch: &str, frames: u64, last: Option<u64>, window_alive: bool| BridgeChannelLiveness {
                channel: ch.into(),
                frames,
                last_observed_at_ms: last,
                alive_in_window: window_alive,
            };
        // 健康: 双 channel 窗口内流通。
        let ok = assemble_bridge_health(
            true,
            vec!["a".into(), "b".into()],
            &[
                row("a", 100, Some(now - 100), true),
                row("b", 200, Some(now - 200), true),
            ],
        );
        assert!(!ok.bridge_degraded);
        assert_eq!(ok.observed_alive_channels.len(), 2);
        // **历史曾经活过≠当前流通**: b frames=10_000（曾在流）但最后实测
        // 在窗口外（断流）→ degraded（帧基判定此处置 false=漏报根因）。
        let degraded = assemble_bridge_health(
            true,
            vec!["a".into(), "b".into()],
            &[
                row("a", 100, Some(now - 100), true),
                row("b", 10_000, Some(now - 9_000), false),
            ],
        );
        assert!(
            degraded.bridge_degraded,
            "pipeline Ok + bridge 当前断流（虽有历史帧）degraded 可见"
        );
        assert_eq!(degraded.observed_alive_channels, vec!["a".to_string()]);
        // 从未观测（重放失败/无数据）→ 亦降级。
        let never = assemble_bridge_health(true, vec!["a".into()], &[row("a", 0, None, false)]);
        assert!(never.bridge_degraded);
        // recover 本体失败 → 不声称桥降级（管线平面事实优先）。
        let failed = assemble_bridge_health(false, vec!["a".into()], &[row("a", 0, None, false)]);
        assert!(!failed.bridge_degraded);
    }

    #[test]
    fn gh_rt_01_failure_domain_matrix() {
        // ④: 组合观测分类——单故障假设, 优先序 Input>Bridge>Program。
        use FailureDomain::*;
        assert_eq!(classify_failure_domain(true, true, true), None);
        assert_eq!(classify_failure_domain(false, true, true), Input);
        assert_eq!(
            classify_failure_domain(false, false, false),
            Input,
            "输入死为首因"
        );
        assert_eq!(classify_failure_domain(true, false, true), Bridge);
        assert_eq!(classify_failure_domain(true, false, false), Bridge);
        assert_eq!(classify_failure_domain(true, true, false), Program);
    }

    // ── C-TIMELINE-01 Batch 2: Runtime orchestration ①-⑩（Mock 闭环）────

    /// Mock runtime 构造 helper（双输入 + Mock switcher）。
    fn runtime_with_mock() -> (
        Uuid,
        Uuid,
        SessionId,
        ProgramExecutionRuntime,
        std::sync::Arc<MockSwitchExecutionAdapter>,
    ) {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let sid = SessionId(Uuid::new_v4());
        let adapter = std::sync::Arc::new(MockSwitchExecutionAdapter::new());
        let runtime = ProgramExecutionRuntime::create(
            sid,
            dual_group(sid, a, b),
            adapter.clone(),
            None,
            Vec::new(),
        )
        .expect("runtime 创建");
        (a, b, sid, runtime, adapter)
    }

    #[test]
    fn timeline_rt_02_runtime_switch_program_full_chain_preserved() {
        // Runtime 全链（⑨ 挂 Authority + ⑩ orchestration）: ①基准+锚→②声明
        // →③install→④switch→⑤-⑧证据闭合→⑨settle→⑩Stable——Preserve
        // （epoch 不变）+ Desired Observed 落定 + observe_execution 权威行。
        let (a, b, _sid, runtime, adapter) = runtime_with_mock();
        let report = runtime
            .switch_program(&SwitchIntent {
                target: b,
                policy: crate::program::SwitchPolicy::FrameSwitch,
            })
            .expect("全链切换");
        assert_eq!(report.executed.av_epoch, 1);
        let TransitionOutcome::Preserved { epoch, mapped } = &report.outcome else {
            panic!("连续性成立应 Preserve, 得 {:?}", report.outcome);
        };
        assert_eq!(
            *epoch,
            crate::program_timeline::ProgramEpoch(0),
            "Preserve=同世代"
        );
        assert_eq!(mapped.source_id, b);
        assert!(mapped.evidence.mapped_program_pts > 0);
        assert_eq!(
            mapped.evidence.video_continuity,
            crate::program_timeline::PlaneContinuity::Continuous
        );
        // Desired=Observed 落定（非命令回显）。
        {
            let group_arc = runtime.group_arc().expect("group");
            let g = group_arc.lock().unwrap();
            assert_eq!(g.desired, SwitchDesired::ActiveInput(b));
        }
        // ⑪ observe_execution: program 平面 + Authority snapshot（Domain SoT）。
        let obs = runtime.observe_execution().expect("obs");
        assert_eq!(obs.program.observed_active, Some(b));
        assert_eq!(obs.timeline.source_id, Some(b));
        assert!(obs.timeline.mapped_program_pts.is_some());
        assert_eq!(
            obs.timeline.discontinuity_state,
            crate::pipeline::PtsMonotonicity::DiscontinuityDeclared
        );
        // adapter 行（执行侧原始证据）与权威行同一声明映射（offset 恒等;
        // pts 因 mock 观测即推进允许权威行落后若干 tick——单调 ≥）。
        let graph = runtime.graph_handle().expect("graph");
        let adapter_row = adapter.observe(&graph).timeline;
        assert_eq!(adapter_row.mapping_offset, obs.timeline.mapping_offset);
        assert!(
            adapter_row.mapped_program_pts >= obs.timeline.mapped_program_pts,
            "adapter={:?} authority={:?}",
            adapter_row.mapped_program_pts,
            obs.timeline.mapped_program_pts
        );
        runtime.teardown();
        let _ = a;
    }

    #[test]
    fn timeline_rt_02_runtime_switch_aborts_timeline_on_backend_failure() {
        // ④ 失败路径: adapter.switch 失败 → timeline abort（回 Stable 旧源,
        // 零时间线变化）+ 错误传播; 组停留 Switching（与既有显式链一致——
        // Observed 未确认即不落定）。
        struct FailingSwitchOnly(MockSwitchExecutionAdapter);
        impl crate::contracts::switch::SwitchExecutionAdapter for FailingSwitchOnly {
            fn build_program_graph(
                &self,
                g: &ExecutionGroup,
            ) -> Result<PipelineHandle, SwitchError> {
                self.0.build_program_graph(g)
            }
            fn start_program(&self, g: &PipelineHandle) -> Result<(), SwitchError> {
                self.0.start_program(g)
            }
            fn install_timeline_transition(
                &self,
                g: &PipelineHandle,
                plan: &crate::program_timeline::ProgramTimelinePlan,
            ) -> Result<(), SwitchError> {
                self.0.install_timeline_transition(g, plan)
            }
            fn sample_switch_anchors(
                &self,
                g: &PipelineHandle,
                target: uuid::Uuid,
            ) -> Result<crate::contracts::switch::SwitchAnchors, SwitchError> {
                self.0.sample_switch_anchors(g, target)
            }
            fn timeline_execution_facts(
                &self,
                g: &PipelineHandle,
            ) -> Option<crate::contracts::switch::TimelineExecutionFacts> {
                self.0.timeline_execution_facts(g)
            }
            fn switch(
                &self,
                _g: &PipelineHandle,
                _p: &crate::switch_execution::SwitchExecutionPlan,
            ) -> Result<crate::contracts::switch::SwitchExecuted, SwitchError> {
                Err(SwitchError::Backend("注入: switch 执行失败".into()))
            }
            fn observe(
                &self,
                g: &PipelineHandle,
            ) -> crate::contracts::switch::ProgramExecutionObservation {
                self.0.observe(g)
            }
            fn stop_program(&self, g: &PipelineHandle) -> Result<(), SwitchError> {
                self.0.stop_program(g)
            }
        }
        let (a, b, sid, _runtime, _adapter) = runtime_with_mock();
        _runtime.teardown();
        let failing = std::sync::Arc::new(FailingSwitchOnly(MockSwitchExecutionAdapter::new()));
        let rt =
            ProgramExecutionRuntime::create(sid, dual_group(sid, a, b), failing, None, Vec::new())
                .expect("创建");
        let err = rt
            .switch_program(&SwitchIntent {
                target: b,
                policy: crate::program::SwitchPolicy::FrameSwitch,
            })
            .expect_err("注入失败必须传播");
        assert!(err.to_string().contains("switch 执行失败"), "err={err}");
        // timeline abort 回 Stable 旧源——⑪ 行诚实（恒等段: source=A,
        // offset=0, 位置=已观测 program 实测——映射事实缺席非伪 None）。
        let obs = rt.observe_execution().expect("obs");
        assert_eq!(obs.timeline.source_id, Some(a), "abort 回 Stable 旧源");
        assert_eq!(obs.timeline.mapping_offset, Some(0), "恒等段 offset 0");
        assert!(
            obs.timeline.mapped_program_pts.is_some(),
            "位置=实测 program pts（恒等映射下 mapped==observed）"
        );
        rt.teardown();
    }
}

// === A2-8-04 六路取证面纯函数测试（R51 Unit 1——observation only） ===
#[cfg(test)]
mod six_path_tests {
    use super::*;

    fn health(v_fr: u64, a_fr: u64) -> crate::pipeline::PipelineHealth {
        crate::pipeline::PipelineHealth {
            video_frame_count: v_fr,
            audio_frame_count: a_fr,
            ..Default::default()
        }
    }

    fn bridge(v_fr: u64, a_fr: u64) -> crate::contracts::media_tap::BridgeObservation {
        crate::contracts::media_tap::BridgeObservation {
            channel: "ch".into(),
            video_last_pts: Some(1_000),
            audio_last_pts: Some(1_200),
            video_pts_state: crate::pipeline::PtsMonotonicity::ValidMonotonic,
            audio_pts_state: crate::pipeline::PtsMonotonicity::ValidMonotonic,
            video_frames: v_fr,
            audio_frames: a_fr,
        }
    }

    fn program_obs(
        v_fr: u64,
        a_fr: u64,
        v_pts: Option<u64>,
        a_pts: Option<u64>,
    ) -> crate::contracts::switch::ProgramObservation {
        crate::contracts::switch::ProgramObservation {
            observed_active: None,
            video_active: None,
            audio_active: None,
            switch_epoch: 0,
            input_pts: Vec::new(),
            program_video_pts: v_pts,
            program_audio_pts: a_pts,
            program_video_pts_state: crate::pipeline::PtsMonotonicity::ValidMonotonic,
            program_audio_pts_state: crate::pipeline::PtsMonotonicity::ValidMonotonic,
            program_video_frames: v_fr,
            program_audio_frames: a_fr,
        }
    }

    /// T3 实锚（R50 修订）: input video 冻结、audio 推进——聚合
    /// `input_progress_since` 会报"推进", 逐平面证据必须分记
    /// Some(false)/Some(true)。
    #[test]
    fn per_plane_independence_input() {
        let dev = uuid::Uuid::new_v4();
        let prog = program_obs(10, 10, Some(5_000), Some(5_500));
        let prev = SixPathInputs {
            input: Some(&health(10, 10)),
            bridge: None,
            program: &prog,
        };
        let cur = SixPathInputs {
            input: Some(&health(10, 12)),
            bridge: None,
            program: &prog,
        };
        let e = assemble_six_path_evidence(dev, EvidencePhase::PreSwitch, Some(&prev), &cur);
        assert_eq!(
            e.input_video.advanced,
            Some(false),
            "video 冻结=有证据未推进"
        );
        assert_eq!(e.input_audio.advanced, Some(true), "audio 推进");
        assert_eq!(e.input_video.frames, Some(10));
        assert_eq!(e.program_av_delta_ns, Some(500));
    }

    /// absence≠false: bridge/input 行缺席→frames/advanced=None;
    /// PTS 缺一平面→av_delta=None; prev=None→全路 advanced=None。
    #[test]
    fn absence_is_not_false() {
        let dev = uuid::Uuid::new_v4();
        let prog = program_obs(10, 10, Some(5_000), None);
        let cur = SixPathInputs {
            input: None,
            bridge: None,
            program: &prog,
        };
        let e = assemble_six_path_evidence(dev, EvidencePhase::PreSwitch, None, &cur);
        assert_eq!(e.bridge_video.frames, None);
        assert_eq!(e.bridge_video.advanced, None);
        assert_eq!(
            e.bridge_video.pts_state,
            crate::pipeline::PtsMonotonicity::Unknown
        );
        assert_eq!(e.input_video.frames, None);
        assert_eq!(
            e.program_av_delta_ns, None,
            "audio PTS 缺席→delta=None 非差值"
        );
        assert_eq!(e.program_video.advanced, None, "无 prev=无可比证据");
    }

    /// bridge v 冻结/a 推进 + program 双平面推进 + epoch/phase 透传;
    /// av_delta=|v−a| 只测量。
    #[test]
    fn bridge_program_paths_independent_and_epoch_passthrough() {
        let dev = uuid::Uuid::new_v4();
        let b1 = bridge(7, 7);
        let b2 = bridge(7, 9);
        let p1 = program_obs(100, 100, Some(50_000), Some(50_900));
        let mut p2 = program_obs(125, 125, Some(51_000), Some(51_900));
        p2.switch_epoch = 1;
        let h = health(5, 5);
        let prev = SixPathInputs {
            input: Some(&h),
            bridge: Some(&b1),
            program: &p1,
        };
        let cur = SixPathInputs {
            input: Some(&h),
            bridge: Some(&b2),
            program: &p2,
        };
        let e = assemble_six_path_evidence(dev, EvidencePhase::PostSwitch, Some(&prev), &cur);
        assert_eq!(e.bridge_video.advanced, Some(false));
        assert_eq!(e.bridge_audio.advanced, Some(true));
        assert_eq!(e.program_video.advanced, Some(true));
        assert_eq!(e.program_audio.advanced, Some(true));
        assert_eq!(e.input_video.advanced, Some(false));
        assert_eq!(e.switch_epoch, 1);
        assert_eq!(e.phase, EvidencePhase::PostSwitch);
        assert_eq!(e.program_av_delta_ns, Some(900));
    }
}
