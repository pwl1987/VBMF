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
use crate::contracts::switch::SwitchExecutionAdapter;
use crate::pipeline::PipelineHandle;
use crate::session::{SessionId, SessionStopHook};
use crate::switch_execution::{ExecutionGroup, SwitchError};

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

struct Inner {
    group: Arc<Mutex<ExecutionGroup>>,
    switcher: Arc<dyn SwitchExecutionAdapter>,
    graph: PipelineHandle,
    /// 已挂 tap 簿记（input handle + channel）——teardown 时 detach。
    taps: Vec<(PipelineHandle, String)>,
    tap_port: Option<Arc<dyn MediaTapPort>>,
    watchdog_stop: Option<Arc<std::sync::atomic::AtomicBool>>,
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
}

impl SessionStopHook for ProgramExecutionRuntime {
    fn on_session_stopping(&self, id: &SessionId) -> Result<(), String> {
        if *id == self.session_id {
            self.teardown();
        }
        Ok(())
    }
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
}
