//! R62 / R63-A: 切换失败状态机探针 → 失败恢复矩阵。
//!
//! 演进（如实）: 本文件 R62 轮为只读探针, 钉死 begin 后各阶段失败（F1-F4）
//! 把组/时间线平面闩死、会话内不可再切换的硬事实; **R63-A 修复落地后,
//! 同一批注入的失败后断言更新为恢复语义**（契约: 2026-09-06 R63-A0——
//! Observed 优先, Desired 由 reconciliation 推进; absence ≠ false）:
//! - 翻转未生效失败 → Desired 回 Active(from) + Timeline 回 Stable{from};
//! - 已执行但证据/落定失败 → 命令仍 Failed（不伪装成功）+ 两平面按 observed
//!   落定（epoch+1 恒等重开 + DiscontinuityDeclared + 基线清空）;
//! - observed 未知 → RecoveryRequired 终态（不猜）+ 下次切换 Permanent 拒收;
//! - 每类均验证 Desired/Observed/switch_epoch/TimelinePhase 投影/下一次
//!   切换; replay 保真在 dispatch+幂等平面单测（同 command_id 逐字节）。
//!
//! 注入面 = 本文件私有 `FaultProbeAdapter`（包装 `MockSwitchExecutionAdapter`,
//! 实现 `SwitchExecutionAdapter` 契约——仅测试面存在）:
//! - F0: `sample_switch_anchors` 注入 Err（①b 失败——begin 前, 零状态变化）;
//! - F1/R1: `switch()` 注入 Err（④ 失败——begin 后, mock 未翻转）;
//! - F2/R2: `release_cutover_fence` 注入 Err（排空确认失败——switch 已执行）;
//! - F3/R3: switch 成功后 `timeline_execution_facts` 恒 None（证据超时——
//!   真实 5s 编译期常量墙钟路径）;
//! - F4/R4: switch 成功后 `observe()` 的 program_video_pts 注入回退（⑨ settle
//!   矛盾——UndeclaredBackwardJump FailClosed; R5"settle 失败"同类: settle
//!   段唯一 Err 源即 on_program_pts 矛盾, adapter observe 无 Err 通道）;
//! - degraded: `observe()` 的 observed_active 置 None（再观测未知 → 不猜）;
//! - 0a: 首次正常切换后 `observe()` 注入 pts 回退（①a 稳态闩锁——组未动而
//!   timeline 卡 TransitionFailed 的第三闩锁位点, R62 矩阵未列）。
//!
//! 边界（如实）:
//! - 五个编排超时为编译期常量（无 env/配置旋钮）, F2 以即时 Err 注入
//!   同类错误（与真机 5s 超时共用 program_execution 同一 `?` 传播点）;
//!   F3/F4 走真实超时/矛盾路径。
//! - watchdog 在 R63-A 零改动: 恢复由 switch_program 外层包装同步完成
//!   （watchdog 折叠对 RecoveryRequired → consistent=false 不动作, 单测钉
//!   于 watchdog.rs; 其线程为 hw feature 门控, 单测不可拉起）。
//! - --nocapture 打印矩阵行为证据; 不判架构。
#![cfg(feature = "mock")]

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use media_agent::adapters::mock::MockBackend;
use media_agent::adapters::switch_mock::MockSwitchExecutionAdapter;
use media_agent::command::{CommandEnvelope, CommandId, CommandKind, CommandStatus, CommandTarget};
use media_agent::contracts::backend::MediaBackend;
use media_agent::contracts::switch::{
    CutoverDrainEvidence, ProgramExecutionObservation, SwitchAnchors, SwitchExecuted,
    SwitchExecutionAdapter, TimelineExecutionFacts,
};
use media_agent::idempotency::{CommandIdempotency, IdempotentDispatch};
use media_agent::pipeline::{PipelineHandle, PipelinePlan};
use media_agent::program::SwitchPolicy;
use media_agent::program_execution::ProgramExecutionRuntime;
use media_agent::program_timeline::{ProgramEpoch, ProgramTimelinePlan};
use media_agent::session::{SessionId, SessionInput, SessionManager};
use media_agent::switch_dispatch_plane::RuntimeSwitchPlane;
use media_agent::switch_execution::{
    ExecutionGroup, SwitchDesired, SwitchError, SwitchExecutionPlan, SwitchIntent,
};
use uuid::Uuid;

// ── 脚手架（与 program_execution.rs 既有测试同型）──────────────────────

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

/// 故障注入适配器: 全方法委托 mock, 按 flag 在指定调用点注入失败。
struct FaultProbeAdapter {
    inner: MockSwitchExecutionAdapter,
    /// F0: 第 N 次 `sample_switch_anchors` 注入 Err（0=off）。
    fail_anchors_on: AtomicU32,
    anchor_calls: AtomicU32,
    /// F1: 第 N 次 `switch` 注入 Err（0=off; 不委托——mock 未翻转）。
    fail_switch_on: AtomicU32,
    switch_calls: AtomicU32,
    /// 已成功执行的 switch 次数（F3/F4/0a 的注入门——仅 switch 成功后生效）。
    switches_done: AtomicU32,
    /// F2: `release_cutover_fence` 注入 Err（不委托; guard Drop → force 强释）。
    fail_release: AtomicBool,
    /// F3: switch 成功后 `timeline_execution_facts` 恒 None（证据超时）。
    suppress_facts: AtomicBool,
    /// F4/0a: switch 成功后 observe() 注入 program_video_pts 硬回退（Some(1)）。
    regress_pts: AtomicBool,
    /// degraded: observe() 的 observed_active 置 None（再观测未知）。
    suppress_observed: AtomicBool,
}

impl FaultProbeAdapter {
    fn new() -> Self {
        Self {
            inner: MockSwitchExecutionAdapter::new(),
            fail_anchors_on: AtomicU32::new(0),
            anchor_calls: AtomicU32::new(0),
            fail_switch_on: AtomicU32::new(0),
            switch_calls: AtomicU32::new(0),
            switches_done: AtomicU32::new(0),
            fail_release: AtomicBool::new(false),
            suppress_facts: AtomicBool::new(false),
            regress_pts: AtomicBool::new(false),
            suppress_observed: AtomicBool::new(false),
        }
    }
}

impl SwitchExecutionAdapter for FaultProbeAdapter {
    fn build_program_graph(&self, group: &ExecutionGroup) -> Result<PipelineHandle, SwitchError> {
        self.inner.build_program_graph(group)
    }
    fn start_program(&self, graph: &PipelineHandle) -> Result<(), SwitchError> {
        self.inner.start_program(graph)
    }
    fn switch(
        &self,
        graph: &PipelineHandle,
        plan: &SwitchExecutionPlan,
    ) -> Result<SwitchExecuted, SwitchError> {
        self.switch_calls.fetch_add(1, Ordering::SeqCst);
        let n = self.switch_calls.load(Ordering::SeqCst);
        if self.fail_switch_on.load(Ordering::SeqCst) == n {
            return Err(SwitchError::Backend(format!(
                "F1 注入: adapter switch 失败（第 {n} 次——未翻转）"
            )));
        }
        let r = self.inner.switch(graph, plan)?;
        self.switches_done.fetch_add(1, Ordering::SeqCst);
        Ok(r)
    }
    fn install_timeline_transition(
        &self,
        graph: &PipelineHandle,
        plan: &ProgramTimelinePlan,
    ) -> Result<(), SwitchError> {
        self.inner.install_timeline_transition(graph, plan)
    }
    fn sample_switch_anchors(
        &self,
        graph: &PipelineHandle,
        target: Uuid,
    ) -> Result<SwitchAnchors, SwitchError> {
        self.anchor_calls.fetch_add(1, Ordering::SeqCst);
        let n = self.anchor_calls.load(Ordering::SeqCst);
        if self.fail_anchors_on.load(Ordering::SeqCst) == n {
            return Err(SwitchError::Backend(format!(
                "F0 注入: 锚采样失败（第 {n} 次——pre-begin）"
            )));
        }
        self.inner.sample_switch_anchors(graph, target)
    }
    fn timeline_execution_facts(&self, graph: &PipelineHandle) -> Option<TimelineExecutionFacts> {
        if self.suppress_facts.load(Ordering::SeqCst)
            && self.switches_done.load(Ordering::SeqCst) >= 1
        {
            return None; // F3: 证据永不闭合（⑤-⑧ 超时路径）
        }
        self.inner.timeline_execution_facts(graph)
    }
    fn arm_cutover_fence(&self, graph: &PipelineHandle) -> Result<(), SwitchError> {
        self.inner.arm_cutover_fence(graph)
    }
    fn release_cutover_fence(
        &self,
        graph: &PipelineHandle,
        timeout: Duration,
    ) -> Result<CutoverDrainEvidence, SwitchError> {
        let _ = timeout;
        if self.fail_release.load(Ordering::SeqCst) {
            // 与真机 5s 超时共用同一传播点（confirm_and_release `?`）;
            // 即时注入同类错误, fence 保持 Armed → guard Drop 强释。
            return Err(SwitchError::Backend(
                "F2 注入: 排空确认失败（真机=5s 超时同类）".into(),
            ));
        }
        self.inner.release_cutover_fence(graph, timeout)
    }
    fn force_release_cutover_fence(&self, graph: &PipelineHandle) -> Result<u64, SwitchError> {
        self.inner.force_release_cutover_fence(graph)
    }
    fn observe(&self, graph: &PipelineHandle) -> ProgramExecutionObservation {
        let mut obs = self.inner.observe(graph);
        if self.regress_pts.load(Ordering::SeqCst) && self.switches_done.load(Ordering::SeqCst) >= 1
        {
            // F4/0a: program_video_pts 硬回退（远小于既有基准——F4 在 ⑨ settle
            // / 0a 在下一次 ①a 喂 on_program_pts）。
            obs.program.program_video_pts = Some(1);
        }
        if self.suppress_observed.load(Ordering::SeqCst) {
            // degraded: 再观测未知（absence≠false——恢复不得猜）。
            obs.program.observed_active = None;
        }
        obs
    }
    fn stop_program(&self, graph: &PipelineHandle) -> Result<(), SwitchError> {
        self.inner.stop_program(graph)
    }
}

fn intent(target: Uuid) -> SwitchIntent {
    SwitchIntent {
        target,
        policy: SwitchPolicy::FrameSwitch,
    }
}

struct ProbeWorld {
    sid: SessionId,
    a: Uuid,
    b: Uuid,
    runtime: Arc<ProgramExecutionRuntime>,
    adapter: Arc<FaultProbeAdapter>,
}

fn world() -> ProbeWorld {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let sid = SessionId(Uuid::new_v4());
    let adapter = Arc::new(FaultProbeAdapter::new());
    let runtime = Arc::new(
        ProgramExecutionRuntime::create(
            sid,
            dual_group(sid, a, b),
            adapter.clone(),
            None,
            Vec::new(),
        )
        .expect("runtime 创建"),
    );
    ProbeWorld {
        sid,
        a,
        b,
        runtime,
        adapter,
    }
}

/// 失败后组平面快照（矩阵行数据）。
fn snap(w: &ProbeWorld) -> (SwitchDesired, u64, Option<Uuid>, bool) {
    let group_arc = w.runtime.group_arc().expect("group");
    let g = group_arc.lock().unwrap();
    let observed = w
        .runtime
        .observe_execution()
        .expect("observe_execution 失败后仍可用（查询面活着）")
        .program
        .observed_active;
    (g.desired, g.switch_epoch, observed, w.runtime.is_active())
}

/// 时间线投影快照: (program_epoch, source_id, discontinuity_state)——
/// 恢复落定的"epoch+1 + DiscontinuityDeclared"契约面。
fn tl(w: &ProbeWorld) -> (ProgramEpoch, Option<Uuid>, String) {
    let t = w
        .runtime
        .observe_execution()
        .expect("observe_execution")
        .timeline;
    let disc = format!("{:?}", t.discontinuity_state);
    (t.program_epoch, t.source_id, disc)
}

fn row(
    tag: &str,
    err: &SwitchError,
    desired: &SwitchDesired,
    epoch: u64,
    observed: Option<Uuid>,
    active: bool,
) {
    println!(
        "R63-MATRIX {tag:<10} err_class={err_class} desired={desired:?} switch_epoch={epoch} observed={observed:?} runtime_active={active}",
        err_class = match err {
            SwitchError::Backend(_) => "Backend",
            SwitchError::NotActiveSource(_) => "NotActiveSource",
            SwitchError::RecoveryRequired(_) => "RecoveryRequired",
            _ => "Other",
        },
    );
}

// ── 00 对照基线: 注入全关 = 全链健康（R62/R63 同形）───────────────────

#[test]
fn r62_probe_00_control_normal_switch_preserved() {
    let w = world();
    let report = w
        .runtime
        .switch_program(&intent(w.b))
        .expect("对照基线: 全链切换");
    assert_eq!(report.executed.av_epoch, 1);
    assert!(matches!(
        report.outcome,
        media_agent::program_timeline::TransitionOutcome::Preserved { .. }
    ));
    let (desired, epoch, observed, active) = snap(&w);
    assert_eq!(desired, SwitchDesired::ActiveInput(w.b));
    assert_eq!(epoch, 1);
    assert_eq!(observed, Some(w.b));
    assert!(active);
    let (tl_epoch, tl_source, _) = tl(&w);
    assert_eq!(tl_epoch, ProgramEpoch(0), "Preserved 同世代");
    assert_eq!(tl_source, Some(w.b));
    println!("R63-MATRIX ctrl outcome=Preserved desired=ActiveInput(b) epoch=1 observed=Some(b) tl_epoch=0");
    w.runtime.teardown();
    assert!(!w.runtime.is_active());
}

// ── F0 pre-begin 失败: 零状态变化、完全可恢复（R62/R63 同形）──────────

#[test]
fn r62_probe_f0_pre_begin_failure_recoverable() {
    let w = world();
    w.adapter.fail_anchors_on.store(1, Ordering::SeqCst);
    let err = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("F0: 锚采样注入失败");
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("F0")),
        "{err:?}"
    );
    let (desired, epoch, observed, active) = snap(&w);
    assert_eq!(desired, SwitchDesired::ActiveInput(w.a), "F0 零状态变化");
    assert_eq!(epoch, 0);
    assert_eq!(observed, Some(w.a));
    assert!(active);
    let (tl_epoch, tl_source, _) = tl(&w);
    assert_eq!(tl_epoch, ProgramEpoch(0), "时间线零变化");
    assert_eq!(tl_source, Some(w.a));
    row("F0", &err, &desired, epoch, observed, active);
    // 解除注入后同 intent 立即可切换成功（完全恢复——无残留闩锁）。
    w.adapter.fail_anchors_on.store(0, Ordering::SeqCst);
    let report = w
        .runtime
        .switch_program(&intent(w.b))
        .expect("F0 后恢复切换");
    assert!(matches!(
        report.outcome,
        media_agent::program_timeline::TransitionOutcome::Preserved { .. }
    ));
    let (desired, epoch, observed, _) = snap(&w);
    assert_eq!(desired, SwitchDesired::ActiveInput(w.b));
    assert_eq!(epoch, 1);
    assert_eq!(observed, Some(w.b));
    println!("R63-MATRIX F0-recover next_switch=Preserved desired=ActiveInput(b)");
    w.runtime.teardown();
}

// ── R1 adapter 失败（observed=from）: 回旧源 + 立即可再计划（R7）──────

#[test]
fn r63_r1_adapter_error_recovers_to_from_then_replan() {
    let w = world();
    w.adapter.fail_switch_on.store(1, Ordering::SeqCst);
    let err = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("R1: adapter switch 注入失败");
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("F1")),
        "{err:?}"
    );
    // R63-A 恢复: observed=from → Desired 回 Active(from); epoch 已消费不变。
    let (desired, epoch, observed, active) = snap(&w);
    assert_eq!(
        desired,
        SwitchDesired::ActiveInput(w.a),
        "R1 恢复: 回旧源（不再闩锁 Switching）"
    );
    assert_eq!(epoch, 1);
    assert_eq!(observed, Some(w.a));
    assert!(active);
    // 时间线: SwitchRequested + observed≠to → abort 语义（epoch/世代不变）。
    let (tl_epoch, tl_source, _) = tl(&w);
    assert_eq!(tl_epoch, ProgramEpoch(0), "未翻转→时间线零变化");
    assert_eq!(tl_source, Some(w.a));
    row("R1", &err, &desired, epoch, observed, active);
    // R7: 恢复后再次 A→B 立即成功（注入已按次数消耗）。
    let report = w
        .runtime
        .switch_program(&intent(w.b))
        .expect("R7: 恢复后 A→B 应成功");
    assert!(matches!(
        report.outcome,
        media_agent::program_timeline::TransitionOutcome::Preserved { .. }
    ));
    let (desired, epoch, observed, _) = snap(&w);
    assert_eq!(desired, SwitchDesired::ActiveInput(w.b));
    assert_eq!(epoch, 2);
    assert_eq!(observed, Some(w.b));
    println!("R63-MATRIX R1+R7 recovered_next=Preserved epoch=2 observed=Some(b)");
    w.runtime.teardown();
}

// ── R2 排空确认失败（observed=to）: 落 B + epoch+1 不连续 + 可再切（R8）──

#[test]
fn r63_r2_fence_confirm_error_lands_observed_to() {
    let w = world();
    w.adapter.fail_release.store(true, Ordering::SeqCst);
    let err = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("R2: 排空确认注入失败");
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("F2")),
        "{err:?}"
    );
    // R63-A 恢复: observed=to → Desired 落 Active(to)（不伪装成功——命令已 Failed）。
    let (desired, epoch, observed, active) = snap(&w);
    assert_eq!(
        desired,
        SwitchDesired::ActiveInput(w.b),
        "R2 恢复: 按观测落 B（不再依赖 watchdog 等价落定）"
    );
    assert_eq!(epoch, 1);
    assert_eq!(observed, Some(w.b), "R2: adapter 已翻转（switch 成功在先）");
    assert!(active);
    // 时间线: SwitchExecuted → reconcile 落 Stable{b} + epoch+1 + DD + 恒等重开。
    let (tl_epoch, tl_source, disc) = tl(&w);
    assert_eq!(tl_epoch, ProgramEpoch(1), "已执行→世代推进（不伪造连续）");
    assert_eq!(tl_source, Some(w.b));
    assert!(
        disc.contains("DiscontinuityDeclared"),
        "切换发生了但证据不完整——如实记不连续: {disc}"
    );
    row("R2", &err, &desired, epoch, observed, active);
    // R8: 恢复后 B→A 立即合法（旧断言 declare InvalidPhase 已废——闩锁解除）。
    w.adapter.fail_release.store(false, Ordering::SeqCst);
    let report = w
        .runtime
        .switch_program(&intent(w.a))
        .expect("R8: 恢复后 B→A 应成功");
    assert!(matches!(
        report.outcome,
        media_agent::program_timeline::TransitionOutcome::Preserved { .. }
    ));
    let (desired, epoch, observed, _) = snap(&w);
    assert_eq!(desired, SwitchDesired::ActiveInput(w.a));
    assert_eq!(epoch, 2);
    assert_eq!(observed, Some(w.a));
    println!(
        "R63-MATRIX R2+R8 recovered_next=Preserved epoch=2 observed=Some(a) tl_epoch=1(同世代延续)"
    );
    w.runtime.teardown();
}

// ── R3 证据超时（真实 5s 常量路径·observed=to）: 落 B 不伪装成功（R6）──

#[test]
fn r63_r3_evidence_timeout_lands_observed_not_faking_success() {
    let w = world();
    w.adapter.suppress_facts.store(true, Ordering::SeqCst);
    let t0 = Instant::now();
    let err = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("R3: 证据超时 FailClosed");
    let elapsed = t0.elapsed();
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("证据超时")),
        "{err:?}"
    );
    assert!(
        elapsed >= Duration::from_secs(4),
        "R3 走真实证据超时常量路径: {elapsed:?}"
    );
    // R6: 命令 Failed（不伪装成功）; 状态按观测落 B。
    let (desired, epoch, observed, active) = snap(&w);
    assert_eq!(
        desired,
        SwitchDesired::ActiveInput(w.b),
        "R3+R6: observed=B 已生效 → 状态如实落 B（命令仍 Failed）"
    );
    assert_eq!(epoch, 1);
    assert_eq!(observed, Some(w.b), "R3: adapter 已翻转");
    assert!(active);
    let (tl_epoch, tl_source, disc) = tl(&w);
    assert_eq!(
        tl_epoch,
        ProgramEpoch(1),
        "TransitionFailed → reconcile 世代推进"
    );
    assert_eq!(tl_source, Some(w.b));
    assert!(disc.contains("DiscontinuityDeclared"), "{disc}");
    row("R3", &err, &desired, epoch, observed, active);
    // 恢复后 B→A 立即成功（R8 同锚）。
    w.adapter.suppress_facts.store(false, Ordering::SeqCst);
    let report = w
        .runtime
        .switch_program(&intent(w.a))
        .expect("R3 恢复后 B→A 应成功");
    assert!(matches!(
        report.outcome,
        media_agent::program_timeline::TransitionOutcome::Preserved { .. }
    ));
    let (desired, epoch, observed, _) = snap(&w);
    assert_eq!(desired, SwitchDesired::ActiveInput(w.a));
    assert_eq!(epoch, 2);
    assert_eq!(observed, Some(w.a));
    println!("R63-MATRIX R3+R6 recovered_next=Preserved epoch=2 observed=Some(a)");
    w.runtime.teardown();
}

// ── R4 settle 矛盾（R5 同类: settle 唯一 Err 源=on_program_pts 矛盾）──

#[test]
fn r63_r4_settle_contradiction_lands_observed() {
    let w = world();
    w.adapter.regress_pts.store(true, Ordering::SeqCst);
    let err = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("R4: settle 回退矛盾 FailClosed");
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("backward jump")),
        "{err:?}"
    );
    // R63-A 恢复: 再观测 observed=to（regress 只改 pts）→ 落 B + 不连续。
    let (desired, epoch, observed, active) = snap(&w);
    assert_eq!(
        desired,
        SwitchDesired::ActiveInput(w.b),
        "R4 恢复: 按观测落 B（矛盾证据保留在段史, 不洗）"
    );
    assert_eq!(epoch, 1);
    assert_eq!(observed, Some(w.b), "R4: adapter 已翻转");
    assert!(active);
    let (tl_epoch, tl_source, disc) = tl(&w);
    assert_eq!(
        tl_epoch,
        ProgramEpoch(1),
        "矛盾 FailClosed → reconcile 世代推进"
    );
    assert_eq!(tl_source, Some(w.b));
    assert!(disc.contains("DiscontinuityDeclared"), "{disc}");
    row("R4", &err, &desired, epoch, observed, active);
    // 恢复后 B→A 立即成功。
    w.adapter.regress_pts.store(false, Ordering::SeqCst);
    let report = w
        .runtime
        .switch_program(&intent(w.a))
        .expect("R4 恢复后 B→A 应成功");
    assert!(matches!(
        report.outcome,
        media_agent::program_timeline::TransitionOutcome::Preserved { .. }
    ));
    let (desired, epoch, observed, _) = snap(&w);
    assert_eq!(desired, SwitchDesired::ActiveInput(w.a));
    assert_eq!(epoch, 2);
    assert_eq!(observed, Some(w.a));
    println!("R63-MATRIX R4(+R5-class) recovered_next=Preserved epoch=2 observed=Some(a)");
    w.runtime.teardown();
}

// ── degraded: 再观测未知 → RecoveryRequired 终态（不猜）──────────────

#[test]
fn r63_degraded_observed_none_recovery_required_terminal() {
    let w = world();
    w.adapter.fail_release.store(true, Ordering::SeqCst);
    w.adapter.suppress_observed.store(true, Ordering::SeqCst);
    let err = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("degraded: 排空确认失败 + 再观测未知");
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("F2")),
        "{err:?}"
    );
    // R63-A: observed=None → 组降级 RecoveryRequired（唯一入口）; 时间线
    // 诚实停留（epoch 不推进——不落地）。
    let (desired, epoch, observed, active) = snap(&w);
    assert_eq!(
        desired,
        SwitchDesired::RecoveryRequired { from: w.a, to: w.b },
        "observed 未知 → 不猜（absence≠false）"
    );
    assert_eq!(epoch, 1);
    assert_eq!(observed, None, "再观测未知如实投影");
    assert!(active);
    let (tl_epoch, tl_source, _) = tl(&w);
    assert_eq!(
        tl_epoch,
        ProgramEpoch(0),
        "时间线诚实停留（未落地不推进世代）"
    );
    assert_eq!(tl_source, Some(w.a));
    row("degraded", &err, &desired, epoch, observed, active);
    // 下一次切换: 专用词表 Permanent 拒收（确定性——两次逐字节一致）。
    let e2 = w
        .runtime
        .switch_program(&intent(w.a))
        .expect_err("degraded 后切换应拒收");
    let e3 = w
        .runtime
        .switch_program(&intent(w.a))
        .expect_err("degraded 后重试");
    assert!(matches!(&e2, SwitchError::RecoveryRequired(_)), "{e2:?}");
    assert_eq!(format!("{e2}"), format!("{e3}"), "降级拒收确定性");
    row("degraded-next", &e2, &desired, 1, None, true);
    // 恢复路径=会话级 teardown（自动恢复不在 R63-A 范围）。
    w.runtime.teardown();
    assert!(!w.runtime.is_active());
    println!("R63-MATRIX degraded recovery=teardown(会话级·契约内)");
}

// ── 0a ①a 稳态 PTS 闩锁（组未动而 timeline 卡 TransitionFailed）──────

#[test]
fn r63_0a_steady_state_pts_latch_pre_begin_recovers() {
    let w = world();
    // 先正常切换一次（建立 program PTS 基线; Preserved 同世代）。
    let report = w.runtime.switch_program(&intent(w.b)).expect("基线切换");
    assert!(matches!(
        report.outcome,
        media_agent::program_timeline::TransitionOutcome::Preserved { .. }
    ));
    // 注入 ①a 回退: 组从未进入 Switching, timeline 稳态 FailClosed。
    w.adapter.regress_pts.store(true, Ordering::SeqCst);
    let err = w
        .runtime
        .switch_program(&intent(w.a))
        .expect_err("0a: 稳态回退闩锁");
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("backward jump")),
        "{err:?}"
    );
    // R63-A: 组平面无恙（未进入 Switching——reconcile 跳过）; 时间线
    // TransitionFailed → reconcile 落当前源 + epoch+1 + 不连续。
    let (desired, epoch, observed, active) = snap(&w);
    assert_eq!(
        desired,
        SwitchDesired::ActiveInput(w.b),
        "0a: 组平面未被污染（失败发生在 plan/begin 之前）"
    );
    assert_eq!(epoch, 1, "组 epoch 不变（begin 未发生）");
    assert_eq!(observed, Some(w.b));
    assert!(active);
    let (tl_epoch, tl_source, disc) = tl(&w);
    assert_eq!(tl_epoch, ProgramEpoch(1), "稳态闩锁 → reconcile 世代推进");
    assert_eq!(tl_source, Some(w.b));
    assert!(disc.contains("DiscontinuityDeclared"), "{disc}");
    row("0a", &err, &desired, epoch, observed, active);
    // 闩锁解除: 解除注入后 B→A 立即成功（旧语义=会话内永久 InvalidPhase）。
    w.adapter.regress_pts.store(false, Ordering::SeqCst);
    let report = w
        .runtime
        .switch_program(&intent(w.a))
        .expect("0a 恢复后 B→A 应成功");
    assert!(matches!(
        report.outcome,
        media_agent::program_timeline::TransitionOutcome::Preserved { .. }
    ));
    let (desired, epoch, observed, _) = snap(&w);
    assert_eq!(desired, SwitchDesired::ActiveInput(w.a));
    assert_eq!(epoch, 2);
    assert_eq!(observed, Some(w.a));
    println!("R63-MATRIX 0a-recover next_switch=Preserved epoch=2 observed=Some(a)");
    w.runtime.teardown();
}

// ── replay: 恢复改变状态后, 同 command_id 重放 ≡ 原始 outcome ─────────

/// 幂等测试用 SessionManager（与 command/idempotency 既有测试同型装配——
/// switch 命令不触 mgr, 形状在即可）。
fn session_mgr() -> Arc<SessionManager> {
    use media_agent::adapters::mock::MockProvider;
    use media_agent::contracts::provider::HardwareProvider as _;
    use media_agent::port::*;
    let devices: Vec<media_agent::device::DeviceInfo> = MockProvider
        .discover()
        .expect("mock discover")
        .into_iter()
        .map(|d| d.device)
        .collect();
    let pid = PortIdentity::derive(
        &devices[0].device_id,
        ConnectorType::Sdi,
        PortOrdinal::Known(1),
    );
    let registry = PortRegistry {
        ports: vec![PortInfo {
            device_id: devices[0].device_id,
            provider_binding_ref: None,
            identity: PortIdentity {
                port_id: pid,
                connector: ConnectorType::Sdi,
                ordinal: PortOrdinal::Known(1),
            },
            direction: PortDirection::Input,
            capabilities: PortCapabilities::default(),
            runtime_binding: None,
            signal: SignalStatus::default(),
            content: VideoContentState::Unknown,
        }],
    };
    let event_log = Arc::new(media_agent::events::RuntimeEventLog::new());
    Arc::new(SessionManager::new(
        media_agent::resource::SharedResourceRegistry::new(
            media_agent::resource::ResourceRegistry::derive_from_discovery(&registry),
        ),
        Arc::new(media_agent::lease::InMemoryLeaseManager::new()),
        Arc::new(Mutex::new(media_agent::supervisor::Supervisor::new(
            media_agent::supervisor::RestartPolicy::default(),
            event_log.clone(),
        ))),
        Arc::new(MockBackend),
        Arc::new(devices),
        Arc::new(std::collections::HashMap::new()),
        Some(registry),
        media_agent::pipeline::MaterializeMode::Diagnostic,
        media_agent::session::SessionTuning::default(),
        event_log,
    ))
}

#[test]
fn r63_replay_after_recovery_returns_original_outcome() {
    let w = world();
    let plane = Arc::new(RuntimeSwitchPlane::new(w.sid, w.runtime.clone()));
    let idem = CommandIdempotency::new(session_mgr()).with_switch_plane(plane);
    let env = |cid, target| CommandEnvelope {
        command_id: CommandId(cid),
        kind: CommandKind::SwitchProgram,
        target: CommandTarget::SwitchProgram {
            session_id: w.sid,
            target_device: target,
        },
        issued_at_ms: 0,
        requested_by: "r63-replay".into(),
    };
    // claimant 执行: A→B 注入 F2 失败 → outcome Failed; 恢复已把状态落 B。
    w.adapter.fail_release.store(true, Ordering::SeqCst);
    let IdempotentDispatch::Executed(first) = idem.dispatch(&env(Uuid::new_v4(), w.b)) else {
        panic!("claimant 应为 Executed(outcome)");
    };
    assert_eq!(first.status, CommandStatus::Failed);
    assert!(first.detail.as_deref().unwrap().contains("F2"));
    let (desired, _, observed, _) = snap(&w);
    assert_eq!(desired, SwitchDesired::ActiveInput(w.b));
    assert_eq!(observed, Some(w.b), "恢复先于 replay 发生（前提成立）");
    // 同 command_id 重放（恢复已改变状态）: 逐字节回放原始 outcome——
    // 恢复只动状态平面, 绝不改写已记录应答, 也绝不触发再执行。
    let cid = first.command_id.0;
    let IdempotentDispatch::Replayed(second) = idem.dispatch(&env(cid, w.b)) else {
        panic!("同 id 二发应为 Replayed");
    };
    assert_eq!(second.command_id, first.command_id);
    assert_eq!(second.kind, first.kind);
    assert_eq!(
        second.status, first.status,
        "Failed 同样 replay（不洗成成功）"
    );
    assert_eq!(second.detail, first.detail, "detail 逐字节");
    assert_eq!(second.classification, first.classification);
    // 对照: 新 command_id 目标=A（恢复后 B→A）→ 真实再执行成功。
    w.adapter.fail_release.store(false, Ordering::SeqCst);
    let IdempotentDispatch::Executed(ok) = idem.dispatch(&env(Uuid::new_v4(), w.a)) else {
        panic!("新 id 应为 Executed");
    };
    assert_eq!(ok.status, CommandStatus::Executed, "恢复后 B→A 真实成功");
    println!("R63-MATRIX replay(original=Failed) == replayed(byte-identical); new-id=Executed");
    w.runtime.teardown();
}
