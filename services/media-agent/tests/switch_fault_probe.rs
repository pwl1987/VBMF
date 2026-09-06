//! R62 / Step 16-0A: 切换失败状态机探针 —— 只读故障注入（零核心改动）。
//!
//! 目的（用户令 R62）: 钉死硬事实——begin_switch 之后各阶段失败（F1-F4）
//! 把 ExecutionGroup / TimelineAuthority 停在什么状态、会话内能否再次
//! 切换、恢复路径是什么; 并以 F0（pre-begin 失败）对照证明边界。
//!
//! 注入面 = 本文件私有 `FaultProbeAdapter`（包装 `MockSwitchExecutionAdapter`,
//! 实现 `SwitchExecutionAdapter` 契约——仅测试面存在, 生产代码零改动）:
//! - F0: `sample_switch_anchors` 注入 Err（①b 失败——begin 前, 零状态变化）;
//! - F1: `switch()` 注入 Err（④ 失败——begin 后, adapter 未翻转）;
//! - F2: `release_cutover_fence` 注入 Err（排空确认失败——switch 已执行）;
//! - F3: switch 成功后 `timeline_execution_facts` 恒 None（⑤-⑧ 证据超时
//!   —— 走真实 5s 编译期常量墙钟路径）;
//! - F4: switch 成功后 `observe()` 的 program_video_pts 注入回退（⑨ settle
//!   矛盾 —— UndeclaredBackwardJump FailClosed）。
//!
//! 边界（如实）:
//! - 五个编排超时为编译期常量（无 env/配置旋钮）, F2 以即时 Err 注入
//!   同类错误（与真机 5s 超时共用 program_execution 同一 `?` 传播点）;
//!   F3/F4 走真实超时/矛盾路径。
//! - watchdog 落定（watchdog.rs: "observed==Some(to) 时 complete_switch"）
//!   在本文件以直调 `complete_switch(to)` 等价模拟（其线程为 hw feature
//!   门控, 单测不可拉起; 语义同一段代码）。
//! - 本探针只产事实与证据（--nocapture 打印矩阵行）, 不修复、不判架构。
#![cfg(feature = "mock")]

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use media_agent::adapters::mock::MockBackend;
use media_agent::adapters::switch_mock::MockSwitchExecutionAdapter;
use media_agent::contracts::backend::MediaBackend;
use media_agent::contracts::switch::{
    CutoverDrainEvidence, ProgramExecutionObservation, SwitchAnchors, SwitchExecuted,
    SwitchExecutionAdapter, TimelineExecutionFacts,
};
use media_agent::pipeline::{PipelineHandle, PipelinePlan};
use media_agent::program::SwitchPolicy;
use media_agent::program_execution::ProgramExecutionRuntime;
use media_agent::program_timeline::ProgramTimelinePlan;
use media_agent::session::{SessionId, SessionInput};
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
    /// F1: 第 N 次 `switch` 注入 Err（0=off; 不委托——adapter 未翻转）。
    fail_switch_on: AtomicU32,
    switch_calls: AtomicU32,
    /// 已成功执行的 switch 次数（F3/F4 的注入门——仅 switch 成功后生效）。
    switches_done: AtomicU32,
    /// F2: `release_cutover_fence` 注入 Err（不委托; guard Drop → force 强释）。
    fail_release: AtomicBool,
    /// F3: switch 成功后 `timeline_execution_facts` 恒 None（证据超时）。
    suppress_facts: AtomicBool,
    /// F4: switch 成功后 observe() 注入 program_video_pts 硬回退（Some(1)）。
    regress_pts: AtomicBool,
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
            // F4: settle 窗 program_video_pts 硬回退（远小于 ①a 基准——
            // 证据循环不消费 pts, 该值只在 ⑨ settle 喂 on_program_pts）。
            obs.program.program_video_pts = Some(1);
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
    a: Uuid,
    b: Uuid,
    runtime: ProgramExecutionRuntime,
    adapter: Arc<FaultProbeAdapter>,
}

fn world() -> ProbeWorld {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let sid = SessionId(Uuid::new_v4());
    let adapter = Arc::new(FaultProbeAdapter::new());
    let runtime = ProgramExecutionRuntime::create(
        sid,
        dual_group(sid, a, b),
        adapter.clone(),
        None,
        Vec::new(),
    )
    .expect("runtime 创建");
    ProbeWorld {
        a,
        b,
        runtime,
        adapter,
    }
}

/// 失败后状态快照（矩阵行数据; --nocapture 打印为证据）。
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

fn row(
    tag: &str,
    err: &SwitchError,
    desired: &SwitchDesired,
    epoch: u64,
    observed: Option<Uuid>,
    active: bool,
) {
    println!(
        "R62-MATRIX {tag:<4} err_class={err_class} desired={desired:?} switch_epoch={epoch} observed={observed:?} runtime_active={active}",
        err_class = match err {
            SwitchError::Backend(_) => "Backend",
            SwitchError::NotActiveSource(_) => "NotActiveSource",
            _ => "Other",
        },
    );
}

/// 轮询等待 observed 到达期望源（mock observe 驱动 tick; 上限 2s）。
fn wait_observed(w: &ProbeWorld, want: Uuid) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if w.runtime
            .observe_execution()
            .expect("obs")
            .program
            .observed_active
            == Some(want)
        {
            return;
        }
        assert!(Instant::now() < deadline, "observed 未到达 {want}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

// ── 00 对照基线: 注入全关 = 全链健康 ──────────────────────────────────

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
    println!("R62-MATRIX ctrl outcome=Preserved desired=ActiveInput(b) epoch=1 observed=Some(b)");
    w.runtime.teardown();
    assert!(!w.runtime.is_active());
}

// ── F0 pre-begin 失败: 零状态变化、完全可恢复 ─────────────────────────

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
    println!("R62-MATRIX F0-recover next_switch=Preserved desired=ActiveInput(b)");
    w.runtime.teardown();
}

// ── F1 begin 后 adapter 失败: 组平面闩锁 Switching（watchdog 不救）────

#[test]
fn r62_probe_f1_adapter_err_group_latched_switching() {
    let w = world();
    w.adapter.fail_switch_on.store(1, Ordering::SeqCst);
    let err = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("F1: adapter switch 注入失败");
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("F1")),
        "{err:?}"
    );
    let (desired, epoch, observed, active) = snap(&w);
    assert_eq!(
        desired,
        SwitchDesired::Switching { from: w.a, to: w.b },
        "F1: 组平面闩锁 Switching（begin 已推进、无回退路径）"
    );
    assert_eq!(epoch, 1);
    assert_eq!(observed, Some(w.a), "F1: adapter 未翻转——observed 停旧源");
    assert!(active);
    row("F1", &err, &desired, epoch, observed, active);
    // watchdog 条件不满足（observed==Some(to) 才落定）: 直调 complete_switch(旧源)
    // 模拟 watchdog 不触发——组保持闩锁。
    {
        let g = w.runtime.group_arc().expect("group");
        assert!(!g.lock().unwrap().complete_switch(w.a), "旧源回显不落定");
    }
    let (desired, ..) = snap(&w);
    assert!(matches!(desired, SwitchDesired::Switching { .. }));
    // 会话内重试同 intent: 永久 NotActiveSource（重放确定性——两次错误逐字节一致）。
    let e2 = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("F1 后重试 #1");
    let e3 = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("F1 后重试 #2");
    assert!(matches!(&e2, SwitchError::NotActiveSource(_)), "{e2:?}");
    assert_eq!(format!("{e2}"), format!("{e3}"), "runtime 级重放确定性");
    row("F1-next", &e2, &desired, 1, Some(w.a), true);
    // 恢复路径实证: 会话级 teardown（唯一恢复面）仍工作。
    w.runtime.teardown();
    assert!(!w.runtime.is_active());
    println!("R62-MATRIX F1-recover teardown=Ok(session 级唯一恢复面)");
}

// ── F2 排空确认失败: 组可被 watchdog 落定, timeline 闩锁 → 下次 declare InvalidPhase ──

#[test]
fn r62_probe_f2_drain_confirm_err_timeline_latched() {
    let w = world();
    w.adapter.fail_release.store(true, Ordering::SeqCst);
    let err = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("F2: 排空确认注入失败");
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("F2")),
        "{err:?}"
    );
    let (desired, epoch, observed, active) = snap(&w);
    assert!(
        matches!(desired, SwitchDesired::Switching { .. }),
        "F2: 组平面 Switching"
    );
    assert_eq!(epoch, 1);
    assert_eq!(observed, Some(w.b), "F2: adapter 已翻转（switch 成功在先）");
    assert!(active);
    row("F2", &err, &desired, epoch, observed, active);
    // watchdog 等价落定（watchdog.rs observed==Some(to) → complete_switch(to)）。
    wait_observed(&w, w.b);
    {
        let g = w.runtime.group_arc().expect("group");
        assert!(
            g.lock().unwrap().complete_switch(w.b),
            "watchdog 等价: observed==to 落定"
        );
    }
    let (desired, epoch, observed, _) = snap(&w);
    assert_eq!(
        desired,
        SwitchDesired::ActiveInput(w.b),
        "组平面已恢复 Active"
    );
    assert_eq!(observed, Some(w.b));
    // 但 timeline 闩锁: 下一次合法切换在 ② declare 失败（Stable-only）。
    let e2 = w
        .runtime
        .switch_program(&intent(w.a))
        .expect_err("F2 后下次切换: timeline InvalidPhase");
    assert!(
        matches!(&e2, SwitchError::Backend(s) if s.contains("declare_transition")),
        "{e2:?}"
    );
    row("F2-next", &e2, &desired, epoch, observed, true);
    let e3 = w.runtime.switch_program(&intent(w.a)).expect_err("重试");
    assert_eq!(format!("{e2}"), format!("{e3}"), "runtime 级重放确定性");
    w.runtime.teardown();
    assert!(!w.runtime.is_active());
    println!("R62-MATRIX F2-recover group=watchdog-settle(Active) timeline=latched(SwitchExecuted) teardown=Ok");
}

// ── F3 证据超时 FailClosed: 真实 5s 墙钟路径, timeline 终态闩锁 ────────

#[test]
fn r62_probe_f3_evidence_timeout_fail_closed_latched() {
    let w = world();
    w.adapter.suppress_facts.store(true, Ordering::SeqCst);
    let t0 = Instant::now();
    let err = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("F3: 证据超时 FailClosed");
    let elapsed = t0.elapsed();
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("证据超时")),
        "{err:?}"
    );
    assert!(
        elapsed >= Duration::from_secs(4),
        "F3 走真实证据超时常量路径: {elapsed:?}"
    );
    let (desired, epoch, observed, active) = snap(&w);
    assert!(matches!(desired, SwitchDesired::Switching { .. }));
    assert_eq!(epoch, 1);
    assert_eq!(observed, Some(w.b), "F3: adapter 已翻转");
    assert!(active);
    row("F3", &err, &desired, epoch, observed, active);
    wait_observed(&w, w.b);
    {
        let g = w.runtime.group_arc().expect("group");
        assert!(g.lock().unwrap().complete_switch(w.b));
    }
    let (desired, _, _, _) = snap(&w);
    assert_eq!(desired, SwitchDesired::ActiveInput(w.b));
    let e2 = w
        .runtime
        .switch_program(&intent(w.a))
        .expect_err("F3 后下次切换: TransitionFailed 终态 → declare InvalidPhase");
    assert!(
        matches!(&e2, SwitchError::Backend(s) if s.contains("declare_transition")),
        "{e2:?}"
    );
    row("F3-next", &e2, &desired, 1, Some(w.b), true);
    w.runtime.teardown();
    assert!(!w.runtime.is_active());
    println!("R62-MATRIX F3-recover group=watchdog-settle(Active) timeline=latched(TransitionFailed 终态) teardown=Ok");
}

// ── F4 settle 矛盾: UndeclaredBackwardJump FailClosed, timeline 闩锁 ───

#[test]
fn r62_probe_f4_settle_contradiction_fail_closed_latched() {
    let w = world();
    w.adapter.regress_pts.store(true, Ordering::SeqCst);
    let err = w
        .runtime
        .switch_program(&intent(w.b))
        .expect_err("F4: settle 回退矛盾 FailClosed");
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("backward jump")),
        "{err:?}"
    );
    let (desired, epoch, observed, active) = snap(&w);
    assert!(matches!(desired, SwitchDesired::Switching { .. }));
    assert_eq!(epoch, 1);
    assert_eq!(observed, Some(w.b), "F4: adapter 已翻转");
    assert!(active);
    row("F4", &err, &desired, epoch, observed, active);
    wait_observed(&w, w.b);
    {
        let g = w.runtime.group_arc().expect("group");
        assert!(g.lock().unwrap().complete_switch(w.b));
    }
    let (desired, _, _, _) = snap(&w);
    assert_eq!(desired, SwitchDesired::ActiveInput(w.b));
    let e2 = w
        .runtime
        .switch_program(&intent(w.a))
        .expect_err("F4 后下次切换: declare InvalidPhase");
    assert!(
        matches!(&e2, SwitchError::Backend(s) if s.contains("declare_transition")),
        "{e2:?}"
    );
    row("F4-next", &e2, &desired, 1, Some(w.b), true);
    w.runtime.teardown();
    assert!(!w.runtime.is_active());
    println!("R62-MATRIX F4-recover group=watchdog-settle(Active) timeline=latched(TransitionFailed) teardown=Ok");
}
