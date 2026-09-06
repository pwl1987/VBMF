//! R64（用户裁决）: Control Plane / Switch Recovery 综合验收 Gate——真服务恢复矩阵。
//!
//! 载体 = **real Control Plane + test-controlled adapter**（用户 R64-1 裁决落点）:
//! - 真实 BMD 双输入 Session + 真实 `GStreamerSwitchAdapter::bridged()`
//!   （R53 correctness face 零触碰——wrapper 全方法委托, 仅显式旋钮置位时介入）;
//! - 真 transport（`serve_forever` per-connection 线程·R63-B）+ 真
//!   `CommandIdempotency` + `RuntimeQuery` + api_boundary——**矩阵全程经 HTTP 闭环**
//!   （POST /api/v1/commands + GET /runtime、/health、/events/projection）。
//!
//! 两阶段结构（首跑 2026-09-06 gate-run-attempt1.log 的发现固化于此设计）:
//! - **阶段一（干净世界·判据段）**: C0 控制 / C1 observed=from（switch 委托前
//!   Err——硬件未动, 全真实）/ C2 observed=to（**硬件真翻转**+证据抑制→真 5s
//!   编译期常量证据超时, R62 场景+恢复闭环, 全真实）/ STORM 并发综合+快照真值
//!   / C3 observed=None（`observe()` 接缝注入, 如实披露: 真机管线运行中无法
//!   自然产生 absent observed——恢复语义主证不变: 状态机拒绝猜测）→ teardown。
//! - **阶段二（新鲜世界·R65 确定性恢复段）**: C2b=F2 ×10 轮——`release_cutover_fence`
//!   注入 Err（与真机 5s 确认超时同传播点）→ 命令 Failed(unknown) →
//!   **期望感知稳定再观测**（R65-A 契约 2026-09-07: force_open 后物理翻转
//!   迟落窗内单发观测不可信——R64 四跑三态 L1/L2/L3 在案; executed=true 只接受
//!   连续 3 次 ==Some(to), 迟翻伪稳定 from 与 None 被拒, 界尽→RecoveryRequired
//!   诚实终态）**确定性**落 Active(to) + ProgramEpoch+1+DD → 反向再切成功;
//!   三态随机不得复现（否则如实 fail）。随后 F4 行（真翻转+真证据闭合+真
//!   release+⑨ PTS 回注矛盾 FailClosed → 同判据）→ teardown 出口。
//!
//! 纪律（gates 惯例继承）:
//! - Gate = 调用 Production Runtime, 绝不自造第二套; env 入口属 `bin/gates.rs`;
//!   生产 media-agent bin 对本 env 零 dispatch;
//! - **watchdog 不接线**（a204_obs 先例: 矩阵确定性; watchdog 对
//!   RecoveryRequired 的 fold 行为已有 R63-A 单测, watchdog-in-loop 由 30min
//!   真服务稳定基线覆盖）——如实披露;
//! - exit 契约: 0 = 阶段一矩阵全绿 + 阶段二采集完整（findings 不 gate exit——
//!   观察≠判据, R52 纪律）; 2 = 前置失败或阶段一断言失败（fail-closed, 失败行
//!   全量打印, 禁为跑绿改任何面）;
//! - 断言失败=立即停+如实报告（用户 R64 红线: 不在线修）; epoch 记账口径
//!   （首跑实测）: group.switch_epoch=**begin 即消费**（尝试数）;
//!   ProgramObservation.switch_epoch=**已委托 switch 的 plan epoch**（绝对值,
//!   失败重试可跳号——C1 后 av=2/group=3, 重试 av 直跳 4）。checker 只判
//!   `组≥执行` 与 `API==内部观测`, 不作三处相等。
//!
//! 行打印: `R64-MATRIX`（场景行 + sixplane 静息检查点行 + finding 行 + summary）。

// 全量 feature 门控（同 a204_obs/dual_input 惯例）: 默认构建零编译面。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::io::{Read, Write};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::net::{SocketAddr, TcpListener, TcpStream};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::sync::{Arc, Mutex};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::config::Config;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::media_tap::MediaTapPort;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::provider::DiscoveredDevice;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::switch::{
    CutoverDrainEvidence, ProgramExecutionObservation, SwitchAnchors, SwitchExecuted,
    SwitchExecutionAdapter, TimelineExecutionFacts,
};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::device::DeviceInfo;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::events::{RuntimeEventLog, RuntimeEventSink};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::health::AgentState;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::lease::{InMemoryLeaseManager, LeaseManager as _};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::port::{PortDirection, PortInfo};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::program_execution::{program_progress_since, ProgramExecutionRuntime, TapWiring};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::program_timeline::ProgramTimelinePlan;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::session::{SessionId, SessionInput};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::supervisor::Supervisor;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::switch_execution::{ExecutionGroup, SwitchDesired, SwitchError, SwitchExecutionPlan};

/// 起始稳定等待（帧累积; 与 a204_obs/dual_input 同值）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const SETTLE_SECS: u64 = 4;
/// 推进性判定采样间隔（同 a204_obs）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const SAMPLE_GAP_SECS: u64 = 3;
/// 命令 POST 读超时（切换编排 + 5s 证据超时 + 恢复 + 排队串行的总上限）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const CMD_TIMEOUT_SECS: u64 = 90;
/// 查询类 GET 读超时（R63-B 已证毫秒级返回; 8s=宽裕上限）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
const PROBE_TIMEOUT_SECS: u64 = 8;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn sleep(sec: u64) {
    std::thread::sleep(Duration::from_secs(sec));
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// 墙钟毫秒（observed_at_ms 未来时间戳判定用）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ── 注入 wrapper: 真实适配器 + 场景旋钮（全方法委托, 旋钮默认 off）────────

/// R64-1 载体核心: 包装**真实** `GStreamerSwitchAdapter::bridged()`——11 个
/// trait 方法全委托（R53 correctness face 零触碰）; 旋钮按场景前置位/后清除,
/// 不置位时行为与裸真实适配器逐字节等价。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
struct FaultControlWrapper {
    inner: Arc<dyn SwitchExecutionAdapter>,
    /// 第 N 次 `switch` 调用注入 Err（**不委托**——硬件未动; 0=off）。C1/C3。
    fail_switch_on: AtomicU32,
    switch_calls: AtomicU32,
    /// 已成功委托的 switch 次数（诊断行用）。
    switches_done: AtomicU32,
    /// `release_cutover_fence` 注入 Err（不委托; 与真机 5s 确认超时同传播点,
    /// fence 保持 Armed → guard Drop 强释）。C2b（R65 阶段二确定性段）。
    fail_release: AtomicBool,
    /// `timeline_execution_facts` → None（真 5s 编译期常量证据超时窗——
    /// 硬件**已真实翻转**, 证据永不闭合）。C2。
    suppress_facts: AtomicBool,
    /// `observe()` 的 `observed_active` 置 None（**接缝注入**——再观测未知,
    /// absence≠false, 恢复不得猜）。C3。
    suppress_observed: AtomicBool,
    /// R65-F4: 第 N 次 `switch()` 已委托后（switches_done≥N）`observe()` 注入
    /// program_video_pts 硬回退（Some(1)）——⑨ settle `on_program_pts` 矛盾
    /// FailClosed 路径（真翻转+真证据闭合+真 release）。**阈值制是关键**:
    /// 门控世界携带前置切换史, 布尔门（switches_done≥1）会让回注在 **①a**
    /// 提前点火（= 0a pre-begin 形态, 首跑实测在案）——必须锚到本轮
    /// switch 委托之后。0=off。
    regress_pts_from: AtomicU32,
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
impl FaultControlWrapper {
    fn wrap(inner: Arc<dyn SwitchExecutionAdapter>) -> Self {
        Self {
            inner,
            fail_switch_on: AtomicU32::new(0),
            switch_calls: AtomicU32::new(0),
            switches_done: AtomicU32::new(0),
            fail_release: AtomicBool::new(false),
            suppress_facts: AtomicBool::new(false),
            suppress_observed: AtomicBool::new(false),
            regress_pts_from: AtomicU32::new(0),
        }
    }

    /// 下一次 switch 调用注入（按当前计数定向）。
    fn arm_fail_next_switch(&self) {
        let next = self.switch_calls.load(Ordering::SeqCst) + 1;
        self.fail_switch_on.store(next, Ordering::SeqCst);
    }

    fn clear_fail_switch(&self) {
        self.fail_switch_on.store(0, Ordering::SeqCst);
    }

    /// R65-F4: 在**下一次** switch 委托完成后激活 PTS 回注（⑨ 矛盾位点位）。
    fn arm_regress_after_next_switch(&self) {
        let next = self.switch_calls.load(Ordering::SeqCst) + 1;
        self.regress_pts_from.store(next, Ordering::SeqCst);
    }

    fn clear_regress_pts(&self) {
        self.regress_pts_from.store(0, Ordering::SeqCst);
    }
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
impl SwitchExecutionAdapter for FaultControlWrapper {
    fn build_program_graph(
        &self,
        group: &ExecutionGroup,
    ) -> Result<crate::pipeline::PipelineHandle, SwitchError> {
        self.inner.build_program_graph(group)
    }
    fn start_program(&self, graph: &crate::pipeline::PipelineHandle) -> Result<(), SwitchError> {
        self.inner.start_program(graph)
    }
    fn switch(
        &self,
        graph: &crate::pipeline::PipelineHandle,
        plan: &SwitchExecutionPlan,
    ) -> Result<SwitchExecuted, SwitchError> {
        self.switch_calls.fetch_add(1, Ordering::SeqCst);
        let n = self.switch_calls.load(Ordering::SeqCst);
        if self.fail_switch_on.load(Ordering::SeqCst) == n {
            return Err(SwitchError::Backend(format!(
                "R64-C1/C3 注入: adapter switch 失败（第 {n} 次调用——未委托, 硬件未动）"
            )));
        }
        let r = self.inner.switch(graph, plan)?;
        self.switches_done.fetch_add(1, Ordering::SeqCst);
        Ok(r)
    }
    fn install_timeline_transition(
        &self,
        graph: &crate::pipeline::PipelineHandle,
        plan: &ProgramTimelinePlan,
    ) -> Result<(), SwitchError> {
        self.inner.install_timeline_transition(graph, plan)
    }
    fn sample_switch_anchors(
        &self,
        graph: &crate::pipeline::PipelineHandle,
        target: uuid::Uuid,
    ) -> Result<SwitchAnchors, SwitchError> {
        self.inner.sample_switch_anchors(graph, target)
    }
    fn timeline_execution_facts(
        &self,
        graph: &crate::pipeline::PipelineHandle,
    ) -> Option<TimelineExecutionFacts> {
        if self.suppress_facts.load(Ordering::SeqCst) {
            return None; // C2: 证据永不闭合（真实 5s 超时路径——硬件已翻转）
        }
        self.inner.timeline_execution_facts(graph)
    }
    fn arm_cutover_fence(
        &self,
        graph: &crate::pipeline::PipelineHandle,
    ) -> Result<(), SwitchError> {
        self.inner.arm_cutover_fence(graph)
    }
    fn release_cutover_fence(
        &self,
        graph: &crate::pipeline::PipelineHandle,
        timeout: Duration,
    ) -> Result<CutoverDrainEvidence, SwitchError> {
        if self.fail_release.load(Ordering::SeqCst) {
            // 与真机 5s 确认超时共用同一传播点（confirm_and_release `?`）;
            // fence 保持 Armed → guard Drop 强释。首跑实测: 真机物理 cutover
            // 在 release 时完成——本注入即真机超时路径的状态同型（免 5s 等待）。
            return Err(SwitchError::Backend(
                "R64-C2b 注入: 排空确认失败（真机=5s 确认超时同类）".into(),
            ));
        }
        self.inner.release_cutover_fence(graph, timeout)
    }
    fn force_release_cutover_fence(
        &self,
        graph: &crate::pipeline::PipelineHandle,
    ) -> Result<u64, SwitchError> {
        self.inner.force_release_cutover_fence(graph)
    }
    fn observe(&self, graph: &crate::pipeline::PipelineHandle) -> ProgramExecutionObservation {
        let mut obs = self.inner.observe(graph);
        if self.suppress_observed.load(Ordering::SeqCst) {
            // C3 接缝注入: 再观测未知（absence≠false——恢复不得猜）。
            obs.program.observed_active = None;
        }
        let regress_from = self.regress_pts_from.load(Ordering::SeqCst);
        if regress_from != 0 && self.switches_done.load(Ordering::SeqCst) >= regress_from {
            // R65-F4: program_video_pts 硬回退（远小于真实 PTS 基准——
            // ⑨ settle on_program_pts 矛盾 FailClosed）。
            obs.program.program_video_pts = Some(1);
        }
        obs
    }
    fn stop_program(&self, graph: &crate::pipeline::PipelineHandle) -> Result<(), SwitchError> {
        self.inner.stop_program(graph)
    }
}

// ── 六平面静息一致性（R64-5——采集数据 + 纯函数判定, 零 API 扩张）──────────

/// 静息检查点六平面快照（①命令 ②幂等=命令 status 自带 executed/replayed
/// ③组 ④程序观测 ⑤时间线 ⑥API 投影）。absence 一律 None——缺席≠false。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
#[derive(Clone)]
struct SixPlaneSnapshot {
    /// ①②: 最近一次命令 (dispatch status, classification)。
    command: Option<(String, Option<String>)>,
    /// ③组平面（teardown 后 inner 已 take → 不可达=None）。
    desired: Option<SwitchDesired>,
    group_switch_epoch: Option<u64>,
    /// ④程序观测平面（经 observe_execution——与查询面同通路）。
    observed_active: Option<uuid::Uuid>,
    program_switch_epoch: Option<u64>,
    /// ⑤时间线平面（Authority snapshot）。
    timeline_epoch: Option<u64>,
    timeline_source: Option<uuid::Uuid>,
    /// ⑥API 投影平面（/runtime program_switch 块）。
    api_status: u16,
    api_present: bool,
    api_observed_active: Option<String>,
    api_switch_epoch: Option<u64>,
    api_timeline_epoch: Option<u64>,
}

/// 静息点期望（场景给出——checker 只判矛盾, 不猜状态）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
enum Quiescence {
    Active(uuid::Uuid),
    /// 终态闩锁: 观测面**不与 desired 作等值约束**（闩锁后观测可恢复——设计
    /// 语义: 状态机拒绝猜测, 仅 teardown 解锁）; 下一切换必须 Permanent 拒收。
    RecoveryRequiredTerminal {
        from: uuid::Uuid,
        to: uuid::Uuid,
    },
    Teardown,
}

/// 纯函数: 静息点跨平面矛盾判定（空=一致）。瞬态 Desired=B/Observed=A 仅
/// 允许存在于切换中——静息检查点不得出现。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn check_quiescent_consistency(s: &SixPlaneSnapshot, q: &Quiescence) -> Vec<String> {
    let mut e: Vec<String> = Vec::new();
    match q {
        Quiescence::Teardown => {
            if s.api_status != 200 {
                e.push(format!(
                    "api_status={} 期望 200（teardown 后查询仍须服务）",
                    s.api_status
                ));
            }
            if s.api_present {
                e.push("teardown 后 program_switch 必须为 null（诚实缺席）".into());
            }
            if s.observed_active.is_some()
                || s.program_switch_epoch.is_some()
                || s.timeline_epoch.is_some()
                || s.timeline_source.is_some()
            {
                e.push("teardown 后内部观测必须缺席（observed/epoch/source 全 None）".into());
            }
            if s.desired.is_some() || s.group_switch_epoch.is_some() {
                e.push("teardown 后 group 不可达（inner 已 take）".into());
            }
        }
        Quiescence::Active(_) | Quiescence::RecoveryRequiredTerminal { .. } => {
            // 活跃态公共: API 面健康 + 与内部观测同源一致（不读未来/不缺席）。
            if s.api_status != 200 {
                e.push(format!("api_status={} 期望 200", s.api_status));
            }
            if !s.api_present {
                e.push("program_switch 块缺席（活跃静息点必须可见）".into());
            }
            if s.program_switch_epoch.is_none() || s.timeline_epoch.is_none() {
                e.push("内部观测缺席（活跃静息点必须可读）".into());
            } else if s.program_switch_epoch != s.api_switch_epoch {
                e.push(format!(
                    "switch_epoch API({:?}) != 内部观测({:?})——投影读到了非当前事实",
                    s.api_switch_epoch, s.program_switch_epoch
                ));
            } else if s.timeline_epoch != s.api_timeline_epoch {
                e.push(format!(
                    "timeline_epoch API({:?}) != 内部({:?})",
                    s.api_timeline_epoch, s.timeline_epoch
                ));
            }
            if s.observed_active.map(|u| u.to_string()) != s.api_observed_active {
                e.push(format!(
                    "observed API({:?}) != 内部({:?})",
                    s.api_observed_active, s.observed_active
                ));
            }
            if s.timeline_source != s.observed_active {
                e.push(format!(
                    "Authority source({:?}) != adapter observed({:?})——静息点双源矛盾",
                    s.timeline_source, s.observed_active
                ));
            }
            if let (Some(g), Some(p)) = (s.group_switch_epoch, s.program_switch_epoch) {
                if g < p {
                    e.push(format!(
                        "group epoch({g}) < 执行 epoch({p})——尝试数不可能小于实际翻转数"
                    ));
                }
            }
        }
    }
    match q {
        Quiescence::Active(x) => {
            match &s.desired {
                Some(SwitchDesired::ActiveInput(d)) if d == x => {}
                other => e.push(format!("desired={other:?} 期望 ActiveInput({x})")),
            }
            if s.observed_active != Some(*x) {
                e.push(format!("observed={:?} 期望 Some({x})", s.observed_active));
            }
            if s.timeline_source != Some(*x) {
                e.push(format!("timeline source={:?} 期望 Some({x})", s.timeline_source));
            }
            if s.api_observed_active.as_deref() != Some(x.to_string().as_str()) {
                e.push(format!("API observed={:?} 期望 {x}", s.api_observed_active));
            }
        }
        Quiescence::RecoveryRequiredTerminal { from, to } => {
            match &s.desired {
                Some(SwitchDesired::RecoveryRequired { from: f, to: t }) if f == from && t == to => {}
                other => e.push(format!(
                    "desired={other:?} 期望 RecoveryRequired{{from:{from}, to:{to}}}（终态闩锁, 不自动恢复）"
                )),
            }
        }
        Quiescence::Teardown => {}
    }
    e
}

/// 断言收集（fail-closed: 全量收集后统一 exit, 不中断采集）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn chk(failures: &mut Vec<String>, cond: bool, msg: String) {
    if !cond {
        failures.push(msg);
    }
}

// ── 最小 HTTP 面（r63b 测试同型: std TcpStream + Connection: close）────────

/// (status, body, elapsed)。读到 EOF。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn http_req(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&str>,
    timeout: Duration,
) -> (u16, String, Duration) {
    let t = Instant::now();
    let mut s = match TcpStream::connect(addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("R64 gate: HTTP connect 失败 ({path}): {e}");
            std::process::exit(2);
        }
    };
    let _ = s.set_read_timeout(Some(timeout));
    let b = body.unwrap_or("");
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: r64-gate\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        b.len()
    );
    if s.write_all(head.as_bytes()).is_err()
        || (!b.is_empty() && s.write_all(b.as_bytes()).is_err())
    {
        eprintln!("R64 gate: HTTP write 失败 ({path})");
        std::process::exit(2);
    }
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match s.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(_) => break,
        }
    }
    let dt = t.elapsed();
    let text = String::from_utf8_lossy(&buf).into_owned();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|x| x.parse::<u16>().ok())
        .unwrap_or(0);
    let body_part = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body_part, dt)
}

/// 命令 POST 结果（①②平面数据: dispatch status + classification + detail）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
struct Cmd {
    status: u16,
    jstatus: String,
    classification: Option<String>,
    detail: Option<String>,
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn parse_cmd(status: u16, body: String) -> Cmd {
    let j: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
    Cmd {
        status,
        // ApiCommandStatus = internally tagged（wire: {"status":"replayed"}）。
        jstatus: j["status"]["status"].as_str().unwrap_or("").to_string(),
        classification: j["classification"].as_str().map(str::to_string),
        detail: j["detail"].as_str().map(str::to_string),
    }
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn post_switch(addr: SocketAddr, cid: uuid::Uuid, sid: &SessionId, target: uuid::Uuid) -> Cmd {
    let body = format!(
        "{{\"command_id\":\"{cid}\",\"kind\":\"switch_program\",\"target\":{{\"target_type\":\"switch_program\",\"session_id\":\"{}\",\"target_device\":\"{target}\"}},\"requested_by\":\"r64-gate\"}}",
        sid.0
    );
    let (s, b, _) = http_req(
        addr,
        "POST",
        "/api/v1/commands",
        Some(&body),
        Duration::from_secs(CMD_TIMEOUT_SECS),
    );
    parse_cmd(s, b)
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn post_stop(addr: SocketAddr, cid: uuid::Uuid, sid: &SessionId) -> Cmd {
    let body = format!(
        "{{\"command_id\":\"{cid}\",\"kind\":\"stop_session\",\"target\":{{\"target_type\":\"session_by_id\",\"session_id\":\"{}\"}},\"requested_by\":\"r64-gate\"}}",
        sid.0
    );
    let (s, b, _) = http_req(
        addr,
        "POST",
        "/api/v1/commands",
        Some(&body),
        Duration::from_secs(CMD_TIMEOUT_SECS),
    );
    parse_cmd(s, b)
}

/// /runtime 的 program_switch 块（缺席=Value::Null）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn api_program_switch(addr: SocketAddr) -> (u16, serde_json::Value) {
    let (s, b, _) = http_req(
        addr,
        "GET",
        "/api/v1/runtime",
        None,
        Duration::from_secs(PROBE_TIMEOUT_SECS),
    );
    let j: serde_json::Value = serde_json::from_str(&b).unwrap_or(serde_json::Value::Null);
    (s, j["program_switch"].clone())
}

/// 六平面采集（静息点调用——inner 空闲, observe_execution=真读+刷新发布）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn collect_six_planes(
    rt: &Arc<ProgramExecutionRuntime>,
    addr: SocketAddr,
    last_command: Option<(String, Option<String>)>,
) -> SixPlaneSnapshot {
    let (desired, group_switch_epoch) = match rt.group_arc() {
        Some(g) => {
            let g = g.lock().unwrap();
            (Some(g.desired), Some(g.switch_epoch))
        }
        None => (None, None),
    };
    let (observed_active, program_switch_epoch, timeline_epoch, timeline_source) =
        match rt.observe_execution() {
            Some(o) => (
                o.program.observed_active,
                Some(o.program.switch_epoch),
                Some(o.timeline.program_epoch.0),
                o.timeline.source_id,
            ),
            None => (None, None, None, None),
        };
    let (api_status, ps) = api_program_switch(addr);
    SixPlaneSnapshot {
        command: last_command,
        desired,
        group_switch_epoch,
        observed_active,
        program_switch_epoch,
        timeline_epoch,
        timeline_source,
        api_status,
        api_present: !ps.is_null(),
        api_observed_active: ps["observed_active"].as_str().map(str::to_string),
        api_switch_epoch: ps["switch_epoch"].as_u64(),
        api_timeline_epoch: ps["timeline"]["program_epoch"].as_u64(),
    }
}

/// 检查点: 采集 + 判定 + R64-MATRIX 行打印（矛盾计入 failures——判据段）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn checkpoint(
    failures: &mut Vec<String>,
    label: &str,
    expect: &Quiescence,
    rt: &Arc<ProgramExecutionRuntime>,
    addr: SocketAddr,
    last_command: Option<(String, Option<String>)>,
) {
    let s = collect_six_planes(rt, addr, last_command);
    let errs = check_quiescent_consistency(&s, expect);
    println!(
        "R64-MATRIX sixplane [{label}] cmd={:?} desired={:?} observed={:?} sw_epoch(g/p/api)={:?}/{:?}/{:?} tl(epoch/src/api)={:?}/{:?}/{:?} api(status/present)=({}/{}) verdict={}",
        s.command,
        s.desired,
        s.observed_active,
        s.group_switch_epoch,
        s.program_switch_epoch,
        s.api_switch_epoch,
        s.timeline_epoch,
        s.timeline_source,
        s.api_timeline_epoch,
        s.api_status,
        s.api_present,
        if errs.is_empty() { "OK" } else { "FAIL" },
    );
    for e in errs {
        failures.push(format!("sixplane[{label}]: {e}"));
    }
}

// ── Gate 世界（a204_obs 同源构造 + bin 同款控制面接线）────────────────────

/// 一个干净世界: 真实双输入 Session + 注入 wrapper runtime + 真控制面服务。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
struct GateWorld {
    #[allow(dead_code)]
    mgr: Arc<crate::session::SessionManager>,
    rt: Arc<ProgramExecutionRuntime>,
    wrapper: Arc<FaultControlWrapper>,
    sid: SessionId,
    a: uuid::Uuid,
    b: uuid::Uuid,
    addr: SocketAddr,
}

/// 构造失败 = fail-closed exit(2)（前置非判据——a204_obs 同纪律）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
#[allow(clippy::too_many_arguments)]
fn build_gate_world(
    cfg: &Config,
    devices: &[DeviceInfo],
    discovered: &[DiscoveredDevice],
    lm: &Arc<InMemoryLeaseManager>,
    _sup: &Arc<std::sync::Mutex<Supervisor>>,
    agent_state: &Arc<Mutex<AgentState>>,
    event_sink: &Arc<dyn RuntimeEventSink>,
    projection_log: &Arc<RuntimeEventLog>,
) -> GateWorld {
    let manifest_path = match &cfg.device_binding_path {
        Some(p) => p.clone(),
        None => {
            eprintln!("VBMF_A2_8_R64_CP 需要 DeviceBindingManifest (MEDIA_AGENT_DEVICE_BINDING)");
            std::process::exit(2);
        }
    };
    let manifest = match crate::resolver::DeviceBindingManifest::load(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("manifest 加载失败: {e}");
            std::process::exit(2);
        }
    };
    if manifest.validate_manifest().is_err() {
        eprintln!("manifest 结构校验失败");
        std::process::exit(2);
    }
    let gst_probes =
        match crate::resolver::probe_gstreamer_devices(crate::resolver::MAX_PROBE_DEVICES, false) {
            crate::resolver::GstProbeOutcome::Available { probes, .. } => probes,
            other => {
                eprintln!("GStreamer probe 不可用（{other:?}）——前置失败, fail-closed");
                std::process::exit(2);
            }
        };
    let bindings =
        crate::resolver::collect_bindings_from_manifest(discovered, &gst_probes, &manifest);
    let registry =
        match crate::port::PortRegistry::build(discovered, &gst_probes, &manifest, &bindings) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("PortRegistry 构建失败 (fail-closed): {e:?}");
                std::process::exit(2);
            }
        };
    let input_ports: Vec<&PortInfo> = registry
        .ports
        .iter()
        .filter(|p| p.direction == PortDirection::Input && p.identity.port_id.is_some())
        .collect();
    let mut device_ids: Vec<uuid::Uuid> = input_ports.iter().map(|p| p.device_id).collect();
    device_ids.sort();
    device_ids.dedup();
    if input_ports.len() != 2 || device_ids.len() != 2 {
        eprintln!(
            "前置 fail-closed: 需两块独立单输入卡（恰 2 Input port / 2 台设备）; 实测 ports={} devices={}",
            input_ports.len(),
            device_ids.len()
        );
        std::process::exit(2);
    }
    let resource_registry = crate::resource::ResourceRegistry::derive_from_discovery(&registry);
    let resources = crate::resource::SharedResourceRegistry::new(resource_registry);
    let bundle = match crate::registry::AdapterRegistry::build_media_adapter_bundle() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("adapter feature 冲突 (fail-closed): {e}");
            std::process::exit(2);
        }
    };
    let ctrl: Arc<dyn crate::contracts::backend::MediaBackend> = bundle.backend.clone();
    let media_tap_port: Option<Arc<dyn MediaTapPort>> = bundle.media_tap.clone();
    let mgr = Arc::new(crate::session::SessionManager::new(
        resources.clone(),
        lm.clone(),
        _sup.clone(),
        ctrl.clone(),
        Arc::new(devices.to_vec()),
        Arc::new(bindings.clone()),
        Some(registry.clone()),
        crate::pipeline::MaterializeMode::Diagnostic,
        crate::session::SessionTuning::default(),
        event_sink.clone(),
    ));
    for d in &device_ids {
        let _ = lm.release(&crate::lease::DeviceLease {
            device_id: *d,
            owner: "bootstrap".into(),
            acquired_at: chrono::Utc::now(),
            ttl: std::time::Duration::from_secs(60),
        });
    }
    let intent = crate::graph_intent::GraphRuntimeIntent {
        version: "1.0".into(),
        devices: device_ids
            .iter()
            .map(|id| {
                let port_id = input_ports
                    .iter()
                    .find(|p| p.device_id == *id)
                    .and_then(|p| p.identity.port_id);
                crate::graph_intent::DeviceIntent {
                    device_id: id.to_string(),
                    role: "CAPTURE".into(),
                    pipeline: crate::graph_intent::PipelineIntent {
                        source: crate::graph_intent::SourceIntent {
                            kind: "decklink".into(),
                            device_id: id.to_string(),
                            port_id: port_id.map(|u| u.to_string()),
                        },
                        sink: crate::graph_intent::SinkIntent {
                            kind: "appsink".into(),
                        },
                    },
                }
            })
            .collect(),
    };
    let session_res = mgr
        .create(intent)
        .and_then(|sid| mgr.start(&sid).map(|_| sid));
    let sid = match session_res {
        Ok(sid) => sid,
        Err(e) => {
            eprintln!("Session create/start 失败 (fail-closed): {e:?}");
            std::process::exit(2);
        }
    };
    let started_inputs: Vec<SessionInput> = mgr.status(&sid).map(|s| s.inputs).unwrap_or_default();
    if started_inputs.len() != 2 {
        eprintln!("双输入 Session 未成立: inputs={}", started_inputs.len());
        let _ = mgr.stop(&sid);
        std::process::exit(2);
    }
    let a = started_inputs[0].device_id;
    let b = started_inputs[1].device_id;
    let group = match ExecutionGroup::new(sid, started_inputs.clone(), a) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("ExecutionGroup 构造失败: {e:?}");
            let _ = mgr.stop(&sid);
            std::process::exit(2);
        }
    };
    // R64-1 核心: 真实 bridged 适配器 + gate 私有注入 wrapper（旋钮默认 off）。
    let real_switcher: Arc<dyn SwitchExecutionAdapter> =
        Arc::new(crate::adapters::gstreamer::GStreamerSwitchAdapter::bridged());
    let wrapper = Arc::new(FaultControlWrapper::wrap(real_switcher));
    let tap_wirings: Vec<TapWiring> = started_inputs
        .iter()
        .map(crate::program_execution::TapWiring::for_input)
        .collect();
    let rt = match ProgramExecutionRuntime::create(
        sid,
        group,
        wrapper.clone(),
        media_tap_port.clone(),
        tap_wirings,
    ) {
        Ok(rt) => Arc::new(rt),
        Err(e) => {
            eprintln!("ProgramExecutionRuntime 创建失败: {e:?}");
            let _ = mgr.stop(&sid);
            std::process::exit(2);
        }
    };
    mgr.register_stop_hook(&sid, rt.clone());
    let graph = rt.graph_handle().expect("active graph");
    sleep(SETTLE_SECS);
    // 采集前置: program 输出须在推进（旋钮 off=纯委托真实观测）。
    let pre1 = wrapper.observe(&graph).program;
    sleep(SAMPLE_GAP_SECS);
    let pre2 = wrapper.observe(&graph).program;
    if !program_progress_since(&pre1, &pre2) {
        eprintln!("=== 采集前置未满足: program 输出未推进（fail-closed 截断）===");
        let _ = mgr.stop(&sid);
        std::process::exit(2);
    }
    // 控制面（bin 同款七字段接线——真 transport/幂等/查询/投影）。
    let plane = Arc::new(crate::switch_dispatch_plane::RuntimeSwitchPlane::new(
        sid,
        rt.clone(),
    ));
    let ctx = crate::transport::TransportContext {
        events: projection_log.clone(),
        agent_state: agent_state.clone(),
        device_count: devices.len(),
        query: Some(Arc::new(crate::runtime_query::RuntimeQuery::new(
            mgr.clone(),
        ))),
        idem: Some(Arc::new(
            crate::idempotency::CommandIdempotency::new(mgr.clone())
                .with_switch_plane(plane.clone()),
        )),
        hls_dir: None,
        switch_readback: Some(plane),
    };
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("gate listener 绑定失败: {e}");
            let _ = mgr.stop(&sid);
            std::process::exit(2);
        }
    };
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || crate::transport::serve_forever(listener, ctx));
    GateWorld {
        mgr,
        rt,
        wrapper,
        sid,
        a,
        b,
        addr,
    }
}

// ── 阶段一: 干净世界判据段（C0/C1/C2/STORM/C3 → teardown）──────────────────

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn phase_matrix(failures: &mut Vec<String>, findings: &mut Vec<String>, w: &GateWorld) {
    let (rt, addr, sid, a, b) = (&w.rt, w.addr, &w.sid, w.a, w.b);
    let cmd_last = |c: &Cmd| Some((c.jstatus.clone(), c.classification.clone()));

    println!("R64-MATRIX phase1 world A={a} B={b} sid={}", sid.0);
    checkpoint(failures, "init", &Quiescence::Active(a), rt, addr, None);

    // ── C0 控制行: 零注入 A→B→A（Preserved + R53 签名复核）──
    // 记账: g=begin 尝试数, av=已委托 plan epoch（绝对值）, tl=NewEpoch 落定数。
    let c0a = post_switch(addr, uuid::Uuid::new_v4(), sid, b);
    println!(
        "R64-MATRIX c0a A→B status={} class={:?} detail={:?}",
        c0a.jstatus, c0a.classification, c0a.detail
    );
    chk(
        failures,
        c0a.status == 200,
        format!("c0a http={}", c0a.status),
    );
    chk(
        failures,
        c0a.jstatus == "executed",
        "c0a 期望 executed".into(),
    );
    chk(
        failures,
        c0a.detail
            .as_deref()
            .is_some_and(|d| d.contains("outcome=preserved")),
        format!("c0a 期望 preserved: {:?}", c0a.detail),
    );
    let (_, ps) = api_program_switch(addr);
    chk(
        failures,
        ps["observed_active"].as_str() == Some(b.to_string().as_str()),
        format!("c0a observed={ps}"),
    );
    chk(
        failures,
        ps["switch_epoch"].as_u64() == Some(1),
        "c0a av_epoch=1".into(),
    );
    chk(
        failures,
        ps["timeline"]["program_epoch"].as_u64() == Some(0),
        "c0a tl=0".into(),
    );
    chk(
        failures,
        ps["timeline"]["video_continuity"].as_str() == Some("continuous"),
        format!("c0a R53 签名 Authority=continuous: {ps}"),
    );
    let c0b = post_switch(addr, uuid::Uuid::new_v4(), sid, a);
    println!(
        "R64-MATRIX c0b B→A status={} class={:?} detail={:?}",
        c0b.jstatus, c0b.classification, c0b.detail
    );
    chk(
        failures,
        c0b.jstatus == "executed"
            && c0b
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("outcome=preserved")),
        "c0b preserved".into(),
    );
    checkpoint(
        failures,
        "c0-after",
        &Quiescence::Active(a),
        rt,
        addr,
        cmd_last(&c0b),
    );

    // ── C1 observed=from（全真实: switch 委托前 Err——硬件未动; av 不推进,
    //    group begin 已消费——首跑实测记账: 落定后 g=3/av=2, 重试 av 直跳 4）──
    let cid_c1 = uuid::Uuid::new_v4();
    w.wrapper.arm_fail_next_switch();
    let c1 = post_switch(addr, cid_c1, sid, b);
    w.wrapper.clear_fail_switch();
    println!(
        "R64-MATRIX c1 A→B(注入 pre-flip Err) status={} class={:?} detail={:?}",
        c1.jstatus, c1.classification, c1.detail
    );
    chk(failures, c1.status == 200, format!("c1 http={}", c1.status));
    chk(
        failures,
        c1.jstatus == "executed" && c1.classification.as_deref() == Some("unknown"),
        format!(
            "c1 期望 dispatch executed + class unknown（Backend→Unknown·不伪装成功）: {:?}",
            c1.detail
        ),
    );
    chk(
        failures,
        c1.detail
            .as_deref()
            .is_some_and(|d| d.contains("R64-C1/C3 注入")),
        format!("c1 detail 应含注入标记: {:?}", c1.detail),
    );
    // 落定: Observed 优先 → Active(from=A); timeline abort 回 Stable(A)·epoch 不变。
    checkpoint(
        failures,
        "c1-landed",
        &Quiescence::Active(a),
        rt,
        addr,
        cmd_last(&c1),
    );
    let (_, ps) = api_program_switch(addr);
    chk(
        failures,
        ps["switch_epoch"].as_u64() == Some(2),
        format!("c1 av=2 保持（switch 未委托不推进; g=3 已消费）: {ps}"),
    );
    chk(
        failures,
        ps["timeline"]["program_epoch"].as_u64() == Some(0),
        "c1 tl=0（abort）".into(),
    );
    // 同 id replay: 原样 Failed（不因恢复改写）。
    let c1r = post_switch(addr, cid_c1, sid, b);
    println!(
        "R64-MATRIX c1-replay status={} class={:?} detail={:?}",
        c1r.jstatus, c1r.classification, c1r.detail
    );
    chk(
        failures,
        c1r.jstatus == "replayed",
        "c1-replay 期望 replayed".into(),
    );
    chk(
        failures,
        c1r.detail == c1.detail && c1r.classification == c1.classification,
        "c1-replay 原样（detail/classification 逐字节）".into(),
    );
    // 再切（新 id）: A→B 成功——恢复后合法再切换（R7 真机版）。
    let c1x = post_switch(addr, uuid::Uuid::new_v4(), sid, b);
    println!(
        "R64-MATRIX c1-retry(A→B 新 id) status={} class={:?} detail={:?}",
        c1x.jstatus, c1x.classification, c1x.detail
    );
    chk(
        failures,
        c1x.jstatus == "executed"
            && c1x
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("outcome=preserved")),
        format!("c1-retry 期望成功: {:?}", c1x.detail),
    );
    chk(
        failures,
        c1x.detail
            .as_deref()
            .is_some_and(|d| d.contains("av_epoch=4")),
        format!("c1-retry av 直跳 plan epoch 4: {:?}", c1x.detail),
    );
    checkpoint(
        failures,
        "c1-after-retry",
        &Quiescence::Active(b),
        rt,
        addr,
        cmd_last(&c1x),
    );

    // ── C2 observed=to（全真实: 硬件真翻转 + 证据抑制→真 5s 超时; R62 场景+恢复闭环）──
    w.wrapper.suppress_facts.store(true, Ordering::SeqCst);
    let cid_c2 = uuid::Uuid::new_v4();
    let t_c2 = Instant::now();
    let c2 = post_switch(addr, cid_c2, sid, a);
    let c2_elapsed = t_c2.elapsed();
    w.wrapper.suppress_facts.store(false, Ordering::SeqCst);
    println!(
        "R64-MATRIX c2 B→A(硬件真翻转+证据抑制) elapsed={c2_elapsed:?} status={} class={:?} detail={:?}",
        c2.jstatus, c2.classification, c2.detail
    );
    chk(
        failures,
        c2.jstatus == "executed" && c2.classification.as_deref() == Some("unknown"),
        "c2 dispatch executed + unknown（命令 Failed 不伪装成功）".into(),
    );
    chk(
        failures,
        c2.detail.as_deref().is_some_and(|d| d.contains("超时")),
        format!("c2 detail 应含证据超时: {:?}", c2.detail),
    );
    chk(
        failures,
        c2_elapsed >= Duration::from_secs(4),
        format!("c2 应经历真实 5s 证据窗: {c2_elapsed:?}"),
    );
    // 落定: observed=A（真实）→ Active(to=A) + ProgramEpoch+1 + DD + 新 identity 段。
    checkpoint(
        failures,
        "c2-landed",
        &Quiescence::Active(a),
        rt,
        addr,
        cmd_last(&c2),
    );
    let (_, ps) = api_program_switch(addr);
    chk(
        failures,
        ps["switch_epoch"].as_u64() == Some(5),
        format!("c2 av=5: {ps}"),
    );
    chk(
        failures,
        ps["timeline"]["program_epoch"].as_u64() == Some(1),
        "c2 tl=1（NewEpoch 落定）".into(),
    );
    chk(
        failures,
        ps["timeline"]["discontinuity_state"].as_str() == Some("discontinuity_declared"),
        format!("c2 DD 边界事实: {ps}"),
    );
    // 同 id replay 原样。
    let c2r = post_switch(addr, cid_c2, sid, a);
    println!(
        "R64-MATRIX c2-replay status={} class={:?} detail={:?}",
        c2r.jstatus, c2r.classification, c2r.detail
    );
    chk(
        failures,
        c2r.jstatus == "replayed",
        "c2-replay 期望 replayed".into(),
    );
    chk(
        failures,
        c2r.detail == c2.detail && c2r.classification == c2.classification,
        "c2-replay 原样（detail/classification 逐字节）".into(),
    );
    // 恢复后反向再切: A→B 成功（新纪元上 Preserved）。
    let c2x = post_switch(addr, uuid::Uuid::new_v4(), sid, b);
    println!(
        "R64-MATRIX c2-next(A→B) status={} class={:?} detail={:?}",
        c2x.jstatus, c2x.classification, c2x.detail
    );
    chk(
        failures,
        c2x.jstatus == "executed"
            && c2x
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("timeline_epoch=1")),
        format!("c2-next 期望 preserved@epoch1: {:?}", c2x.detail),
    );
    checkpoint(
        failures,
        "c2-after-next",
        &Quiescence::Active(b),
        rt,
        addr,
        cmd_last(&c2x),
    );
    let (_, ps) = api_program_switch(addr);
    chk(
        failures,
        ps["timeline"]["video_continuity"].as_str() == Some("continuous"),
        format!("c2-next R53 签名 re-proved continuous: {ps}"),
    );

    // ── STORM（R64-3/4 真机并发综合: 无注入——真实切换窗）──
    // 记账: 窗前 g=6/av=6; first(→a) g=7/av=7; queued(→b) g=8/av=8; tl 保持 1。
    println!("R64-MATRIX storm 开始（B→A 后台 + 并发查询 + 同 id replay + 异 id 排队）");
    let cid_s1 = uuid::Uuid::new_v4();
    let storm_addr = addr;
    let storm_sid = *sid;
    let first = std::thread::spawn(move || post_switch(storm_addr, cid_s1, &storm_sid, a));
    sleep_ms(500); // 进入切换编排窗（inner 被长持）
                   // 快照真值轮询（R64-4）: observed 全程=旧已提交值 B, 直至出口发布。
    let poll_stop = Arc::new(AtomicBool::new(false));
    let poll_flag = poll_stop.clone();
    let poll_addr = addr;
    let poller = std::thread::spawn(move || {
        let mut series: Vec<(String, Option<u64>, u64, bool)> = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(CMD_TIMEOUT_SECS);
        while !poll_flag.load(Ordering::SeqCst) && Instant::now() < deadline {
            let (s, b, _) = http_req(
                poll_addr,
                "GET",
                "/api/v1/runtime",
                None,
                Duration::from_secs(PROBE_TIMEOUT_SECS),
            );
            if s == 200 {
                let j: serde_json::Value =
                    serde_json::from_str(&b).unwrap_or(serde_json::Value::Null);
                let ps = &j["program_switch"];
                let observed = ps["observed_active"].as_str().unwrap_or("").to_string();
                let sw = ps["switch_epoch"].as_u64();
                let at = ps["timeline"]["observed_at_ms"].as_u64().unwrap_or(0);
                let future = at > now_ms().saturating_add(2_000);
                series.push((observed, sw, at, future));
            }
            sleep_ms(50); // 真机净切换 ~400ms（首跑实测 storm 全程 0.87s）——快轮询
        }
        series
    });
    // 并发查询四路（T2×2/T4/T5 同窗）。
    let mut probes = Vec::new();
    for path in [
        "/api/v1/runtime",
        "/api/v1/runtime",
        "/health",
        "/api/v1/events/projection",
    ] {
        let p_addr = addr;
        probes.push(std::thread::spawn(move || {
            http_req(
                p_addr,
                "GET",
                path,
                None,
                Duration::from_secs(PROBE_TIMEOUT_SECS),
            )
        }));
    }
    for (i, p) in probes.into_iter().enumerate() {
        let (s, _b, d) = p.join().expect("probe join");
        chk(failures, s == 200, format!("storm probe#{i} http={s}"));
        chk(
            failures,
            d < Duration::from_secs(2),
            format!("storm probe#{i} 延迟 {d:?} 须 <2s"),
        );
    }
    // T6 同 id replay（等 InFlight 完成后原样）+ T7 异 id 反向排队（inner 串行）。
    let replay_addr = addr;
    let replay_sid = *sid;
    let replay = std::thread::spawn(move || post_switch(replay_addr, cid_s1, &replay_sid, a));
    let queued_addr = addr;
    let queued_sid = *sid;
    let cid_s2 = uuid::Uuid::new_v4();
    let queued = std::thread::spawn(move || post_switch(queued_addr, cid_s2, &queued_sid, b));
    let (first_c, replay_c, queued_c) = (
        first.join().expect("first join"),
        replay.join().expect("replay join"),
        queued.join().expect("queued join"),
    );
    println!(
        "R64-MATRIX storm first={} class={:?} | replay={} | queued={} class={:?}",
        first_c.jstatus,
        first_c.classification,
        replay_c.jstatus,
        queued_c.jstatus,
        queued_c.classification
    );
    chk(
        failures,
        first_c.jstatus == "executed"
            && first_c
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("outcome=preserved")),
        format!("storm first 期望 preserved: {:?}", first_c.detail),
    );
    chk(
        failures,
        replay_c.jstatus == "replayed",
        "storm replay 期望 replayed".into(),
    );
    chk(
        failures,
        replay_c.detail == first_c.detail && replay_c.classification == first_c.classification,
        "storm replay 原样".into(),
    );
    chk(
        failures,
        queued_c.jstatus == "executed"
            && queued_c
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("outcome=preserved")),
        format!("storm queued 期望串行后执行: {:?}", queued_c.detail),
    );
    poll_stop.store(true, Ordering::SeqCst);
    let series = poller.join().expect("poller join");
    // 真值分析: 窗内（av==6）样本 observed 必须=B（不读未来）; 单调不回退; 无未来时间戳。
    let mut at_prev = 0u64;
    for (observed, sw, at, future) in &series {
        chk(
            failures,
            !*future,
            format!("storm 快照未来时间戳 observed_at_ms={at} > now"),
        );
        chk(
            failures,
            *at >= at_prev,
            format!("storm observed_at_ms 回退: {at} < {at_prev}"),
        );
        at_prev = *at;
        if *sw == Some(6) {
            chk(
                failures,
                observed == &b.to_string(),
                format!("storm 窗内(av=6)样本 observed={observed} 须=旧已提交 B（不读未来状态）"),
            );
        }
        chk(
            failures,
            observed == &a.to_string() || observed == &b.to_string(),
            format!("storm 样本 observed 异常值: {observed}"),
        );
    }
    chk(failures, !series.is_empty(), "storm 真值序列非空".into());
    println!(
        "R64-MATRIX storm 真值序列 {} 样本: av6(窗内=B)→7(A)→8(B) 演进=快照只读已提交事实",
        series.len()
    );
    checkpoint(
        failures,
        "storm-after",
        &Quiescence::Active(b),
        rt,
        addr,
        cmd_last(&queued_c),
    );
    let (_, ps) = api_program_switch(addr);
    chk(
        failures,
        ps["switch_epoch"].as_u64() == Some(8),
        format!("storm av=8: {ps}"),
    );
    chk(
        failures,
        ps["timeline"]["program_epoch"].as_u64() == Some(1),
        "storm tl 保持 1".into(),
    );
    // 查询只读: 连续两次 GET epoch 不变。
    let (_, ps2) = api_program_switch(addr);
    chk(
        failures,
        ps2["switch_epoch"].as_u64() == Some(8),
        "storm 后连续 GET epoch 不变（查询只读）".into(),
    );

    // ── C3 observed=None（接缝注入: 拒绝猜测 → RecoveryRequired 终态 → 拒收 → teardown）──
    w.wrapper.arm_fail_next_switch();
    w.wrapper.suppress_observed.store(true, Ordering::SeqCst);
    let cid_c3 = uuid::Uuid::new_v4();
    let c3 = post_switch(addr, cid_c3, sid, a);
    w.wrapper.clear_fail_switch();
    w.wrapper.suppress_observed.store(false, Ordering::SeqCst);
    println!(
        "R64-MATRIX c3 B→A(注入 pre-flip Err + observed=None) status={} class={:?} detail={:?}",
        c3.jstatus, c3.classification, c3.detail
    );
    chk(
        failures,
        c3.jstatus == "executed" && c3.classification.as_deref() == Some("unknown"),
        "c3 dispatch executed + unknown".into(),
    );
    // 闩锁后观测已恢复(真实 observed=B)——状态机拒绝猜测, 不自动恢复。
    checkpoint(
        failures,
        "c3-landed",
        &Quiescence::RecoveryRequiredTerminal { from: b, to: a },
        rt,
        addr,
        cmd_last(&c3),
    );
    let (_, ps) = api_program_switch(addr);
    chk(
        failures,
        ps["timeline"]["program_epoch"].as_u64() == Some(1),
        "c3 tl 保持 1（abort）".into(),
    );
    chk(
        failures,
        ps["observed_active"].as_str() == Some(b.to_string().as_str()),
        format!("c3 闩锁后 API observed=真实 B: {ps}"),
    );
    // 新 id 再切: 拒收（状态机裁决, 不触硬件）。R65-B 入口早卫兵后**确定性
    // 单形态**: permanent + recovery required（⓪ fence 装甲/①a PTS 喂入之前
    // 拒收——R64 发现③ 的 ①a PTS 伪影遮蔽 [unknown 形态] 不得复现; 若现=
    // 回归信号如实 fail+finding）。不变量: 命令被拒 + 零委托 + 状态不变。
    let calls_before = w.wrapper.switch_calls.load(Ordering::SeqCst);
    let c3x = post_switch(addr, uuid::Uuid::new_v4(), sid, a);
    let calls_after = w.wrapper.switch_calls.load(Ordering::SeqCst);
    println!(
        "R64-MATRIX c3-next(新 id) status={} class={:?} detail={:?}",
        c3x.jstatus, c3x.classification, c3x.detail
    );
    chk(
        failures,
        c3x.jstatus == "executed" && c3x.classification.as_deref() == Some("permanent"),
        "c3-next 必须确定性 permanent（R65-B 早卫兵——不被 ①a FailClosed 遮蔽成 unknown）".into(),
    );
    chk(
        failures,
        c3x.detail
            .as_deref()
            .is_some_and(|d| d.contains("recovery required")),
        format!("c3-next 拒收形态=recovery required: {:?}", c3x.detail),
    );
    chk(
        failures,
        c3x.detail
            .as_deref()
            .is_some_and(|d| !d.contains("FailClosed")),
        format!("c3-next 不得再现 ①a FailClosed 遮蔽形态（R64 发现③）: {:?}", c3x.detail),
    );
    chk(
        failures,
        calls_before == calls_after,
        "c3-next 拒收不得触硬件（switch_calls 不变）".into(),
    );
    if c3x
        .detail
        .as_deref()
        .is_some_and(|d| d.contains("FailClosed"))
    {
        findings.push(format!(
            "c3-next 回归信号: ①a PTS 回跳 FailClosed 遮蔽形态再现（R64 发现③——R65-B 早卫兵后不得出现）: {:?}",
            c3x.detail
        ));
    }
    // 拒收不改变状态。
    checkpoint(
        failures,
        "c3-after-reject",
        &Quiescence::RecoveryRequiredTerminal { from: b, to: a },
        rt,
        addr,
        cmd_last(&c3x),
    );
    // 同 id replay 原样。
    let c3r = post_switch(addr, cid_c3, sid, a);
    chk(
        failures,
        c3r.jstatus == "replayed" && c3r.detail == c3.detail,
        "c3-replay 原样".into(),
    );
    // 唯一恢复通道 = Session teardown（stop_session → hook → runtime teardown）。
    let cstop = post_stop(addr, uuid::Uuid::new_v4(), sid);
    println!(
        "R64-MATRIX c3-stop status={} class={:?} detail={:?}",
        cstop.jstatus, cstop.classification, cstop.detail
    );
    chk(
        failures,
        cstop.jstatus == "executed",
        "stop_session executed".into(),
    );
    sleep(2);
    chk(
        failures,
        !rt.is_active(),
        "teardown 后 is_active=false".into(),
    );
    chk(
        failures,
        rt.observe_execution().is_none(),
        "teardown 后 observe_execution=None（诚实缺席）".into(),
    );
    checkpoint(
        failures,
        "teardown",
        &Quiescence::Teardown,
        rt,
        addr,
        cmd_last(&cstop),
    );
}

// ── 阶段二: R65 确定性恢复段（C2b=F2 ×10 + F4 settle 矛盾）────────────────

/// R65-A2（用户裁决: F2 真机回归 N≥10——三态随机不得复现）。每轮:
/// 注入 release Err（真机 5s 确认超时同型传播点）→ 命令 Failed(unknown) →
/// 期望感知稳定再观测**确定性**落 Active(to)（迟翻窗口内稳定 from 伪稳定被拒
/// ——R64 四跑三态 L1/L2 不复现）→ ProgramEpoch+1+DD → 反向再切成功。
/// 记账: 第 i 轮后 av=2i / g=2i / tl=i（失败轮委托 plan epoch=2i−1, 反向=2i）。
/// 随后 F4（真翻转+真证据闭合+⑨ PTS 回注矛盾 FailClosed）同判据一遍 + teardown。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn phase2_c2b_f2_deterministic_loop(
    failures: &mut Vec<String>,
    findings: &mut Vec<String>,
    w: &GateWorld,
) {
    let (rt, addr, sid, a, b) = (&w.rt, w.addr, &w.sid, w.a, w.b);
    let cmd_last = |c: &Cmd| Some((c.jstatus.clone(), c.classification.clone()));
    println!(
        "R64-MATRIX phase2(R65 F2×10 确定性) world A={a} B={b} sid={}",
        sid.0
    );
    checkpoint(failures, "p2-init", &Quiescence::Active(a), rt, addr, None);

    let mut landed_to = 0u32;
    let mut other_shapes: Vec<String> = Vec::new();
    for i in 1u64..=10 {
        w.wrapper.fail_release.store(true, Ordering::SeqCst);
        let cid = uuid::Uuid::new_v4();
        let t0 = Instant::now();
        let c = post_switch(addr, cid, sid, b);
        let elapsed = t0.elapsed();
        w.wrapper.fail_release.store(false, Ordering::SeqCst);
        println!(
            "R64-MATRIX c2b[{i}] A→B(release Err) elapsed={elapsed:?} status={} class={:?} detail={:?}",
            c.jstatus, c.classification, c.detail
        );
        chk(
            failures,
            c.status == 200,
            format!("c2b[{i}] http={}", c.status),
        );
        chk(
            failures,
            c.jstatus == "executed" && c.classification.as_deref() == Some("unknown"),
            format!("c2b[{i}] dispatch executed + unknown（命令 Failed 不伪装成功）"),
        );
        chk(
            failures,
            c.detail
                .as_deref()
                .is_some_and(|d| d.contains("R64-C2b 注入")),
            format!("c2b[{i}] detail 应含注入标记: {:?}", c.detail),
        );
        // R65 确定性落定: Active(to=B)——唯一合法形态（期望感知协议拒绝迟翻
        // 伪稳定 from; 界尽/稳定 None → RecoveryRequired 诚实终态, 若现=违
        // 契约如实 fail——R64 L1/L2 三态随机不得复现）。
        let desired_now = rt.group_arc().map(|g| g.lock().unwrap().desired);
        match desired_now {
            Some(SwitchDesired::ActiveInput(x)) if x == b => landed_to += 1,
            other => {
                other_shapes.push(format!("round{i}={other:?}"));
                failures.push(format!(
                    "c2b[{i}] R65 后落定必须确定性 Active(to)——三态随机不得复现: {other:?}"
                ));
            }
        }
        let (_, ps) = api_program_switch(addr);
        chk(
            failures,
            ps["switch_epoch"].as_u64() == Some(2 * i - 1),
            format!(
                "c2b[{i}] av={}（失败轮委托 plan epoch 2i−1）: {ps}",
                2 * i - 1
            ),
        );
        chk(
            failures,
            ps["timeline"]["program_epoch"].as_u64() == Some(i),
            format!("c2b[{i}] tl={}（每轮 NewEpoch 落定）: {ps}", i),
        );
        chk(
            failures,
            ps["timeline"]["discontinuity_state"].as_str() == Some("discontinuity_declared"),
            format!("c2b[{i}] DD 边界事实: {ps}"),
        );
        if i == 1 {
            // 同 id replay 原样 + 落定六平面检查点（仅首轮——其余轮由逐项断言覆盖）。
            let cr = post_switch(addr, cid, sid, b);
            chk(
                failures,
                cr.jstatus == "replayed" && cr.detail == c.detail,
                "c2b[1]-replay 原样".into(),
            );
            checkpoint(
                failures,
                "c2b1-landed",
                &Quiescence::Active(b),
                rt,
                addr,
                Some((c.jstatus.clone(), c.classification.clone())),
            );
        }
        // 反向再切: B→A 真实成功（Preserved·新纪元连续——恢复后合法再切换）。
        let rn = post_switch(addr, uuid::Uuid::new_v4(), sid, a);
        println!(
            "R64-MATRIX c2b[{i}] B→A(反向) status={} class={:?} detail={:?}",
            rn.jstatus, rn.classification, rn.detail
        );
        chk(
            failures,
            rn.jstatus == "executed"
                && rn
                    .detail
                    .as_deref()
                    .is_some_and(|d| d.contains("outcome=preserved")),
            format!("c2b[{i}] 反向再切期望 preserved: {:?}", rn.detail),
        );
        chk(
            failures,
            rn.detail
                .as_deref()
                .is_some_and(|d| d.contains(&format!("timeline_epoch={i}"))),
            format!("c2b[{i}] 反向在新纪元 {}: {:?}", i, rn.detail),
        );
        let (_, ps) = api_program_switch(addr);
        chk(
            failures,
            ps["switch_epoch"].as_u64() == Some(2 * i),
            format!("c2b[{i}] 反向后 av={}: {ps}", 2 * i),
        );
        chk(
            failures,
            ps["timeline"]["video_continuity"].as_str() == Some("continuous"),
            format!("c2b[{i}] R53 签名 re-proved continuous: {ps}"),
        );
    }
    checkpoint(
        failures,
        "c2b10-after",
        &Quiescence::Active(a),
        rt,
        addr,
        None,
    );
    let (_, ps) = api_program_switch(addr);
    chk(
        failures,
        ps["switch_epoch"].as_u64() == Some(20),
        format!("c2b 10 轮后 av=20: {ps}"),
    );
    chk(
        failures,
        ps["timeline"]["program_epoch"].as_u64() == Some(10),
        "c2b 10 轮后 tl=10".into(),
    );
    findings.push(format!(
        "c2b F2×10 确定性: landed_to={landed_to}/10 other={other_shapes:?}（R64 四跑三态 L1/L2/L3 在案——R65 后不得复现）"
    ));

    // ── F4 settle 矛盾（真翻转+真证据闭合+真 release+⑨ PTS 回注 FailClosed）──
    // 记账: av 21（失败轮）→ 22（反向）, tl 10→11。回注锚定本轮 switch 委托后
    // （阈值制——避免 ①a 提前点火成 0a pre-begin 形态, 首跑实测教训在案）。
    w.wrapper.arm_regress_after_next_switch();
    let cid_f4 = uuid::Uuid::new_v4();
    let c4 = post_switch(addr, cid_f4, sid, b);
    w.wrapper.clear_regress_pts();
    println!(
        "R64-MATRIX f4 A→B(⑨ PTS 回注矛盾) status={} class={:?} detail={:?}",
        c4.jstatus, c4.classification, c4.detail
    );
    chk(
        failures,
        c4.jstatus == "executed" && c4.classification.as_deref() == Some("unknown"),
        "f4 dispatch executed + unknown".into(),
    );
    chk(
        failures,
        c4.detail
            .as_deref()
            .is_some_and(|d| d.contains("backward jump")),
        format!(
            "f4 detail 应含 backward jump（⑨ settle 矛盾）: {:?}",
            c4.detail
        ),
    );
    // 确定性落定: executed=true → 稳定 Some(to) → Active(B) + NewEpoch(11)。
    let desired_now = rt.group_arc().map(|g| g.lock().unwrap().desired);
    chk(
        failures,
        matches!(desired_now, Some(SwitchDesired::ActiveInput(x)) if x == b),
        format!("f4 恢复落定必须 Active(to): {desired_now:?}"),
    );
    checkpoint(
        failures,
        "f4-landed",
        &Quiescence::Active(b),
        rt,
        addr,
        Some((c4.jstatus.clone(), c4.classification.clone())),
    );
    let (_, ps) = api_program_switch(addr);
    chk(
        failures,
        ps["switch_epoch"].as_u64() == Some(21),
        format!("f4 av=21: {ps}"),
    );
    chk(
        failures,
        ps["timeline"]["program_epoch"].as_u64() == Some(11),
        "f4 tl=11（NewEpoch 落定）".into(),
    );
    // 同 id replay 原样。
    let c4r = post_switch(addr, cid_f4, sid, b);
    chk(
        failures,
        c4r.jstatus == "replayed" && c4r.detail == c4.detail,
        "f4-replay 原样".into(),
    );
    // 反向再切成功（av=22）。
    let r4 = post_switch(addr, uuid::Uuid::new_v4(), sid, a);
    chk(
        failures,
        r4.jstatus == "executed"
            && r4
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("outcome=preserved")),
        format!("f4 反向再切期望 preserved: {:?}", r4.detail),
    );
    // 唯一兜底出口: stop_session teardown（此路径必须可用——gate exit 判据）。
    let cstop = post_stop(addr, uuid::Uuid::new_v4(), sid);
    println!(
        "R64-MATRIX phase2-stop status={} class={:?} detail={:?}",
        cstop.jstatus, cstop.classification, cstop.detail
    );
    chk(
        failures,
        cstop.jstatus == "executed",
        "phase2 stop_session executed（兜底出口可用）".into(),
    );
    sleep(2);
    chk(
        failures,
        !rt.is_active(),
        "phase2 teardown 后 is_active=false".into(),
    );
    checkpoint(
        failures,
        "p2-teardown",
        &Quiescence::Teardown,
        rt,
        addr,
        cmd_last(&cstop),
    );
}

// ── Gate 入口 ────────────────────────────────────────────────────────────

/// 入口（bin/gates.rs 派发; env `VBMF_A2_8_R64_CP` 存在即跑）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
#[allow(clippy::too_many_arguments)]
pub fn run(
    cfg: &Config,
    devices: &[DeviceInfo],
    discovered: &[DiscoveredDevice],
    lm: &Arc<InMemoryLeaseManager>,
    _sup: &Arc<std::sync::Mutex<Supervisor>>,
    agent_state: &Arc<Mutex<AgentState>>,
    event_sink: &Arc<dyn RuntimeEventSink>,
    projection_log: &Arc<RuntimeEventLog>,
) {
    if std::env::var("VBMF_A2_8_R64_CP").is_err() {
        return;
    }
    println!(
        "=== R64 Control Plane / Switch Recovery 综合验收（real Control Plane + test-controlled adapter）==="
    );
    println!(
        "披露: C1/C2=真实硬件行为（注入仅在适配器调用点）; C3 observed=None 为接缝注入\
         （真机管线运行中无法自然产生 absent observed）——恢复语义主证不变: 状态机拒绝猜测"
    );
    println!(
        "披露: C2b=R65 阶段二确定性段（F2×10——期望感知稳定再观测协议在案; R64 四跑三态不得复现）+ F4 settle 矛盾行"
    );
    println!(
        "披露: watchdog 未接线（a204_obs 先例·矩阵确定性; watchdog-in-loop 由 30min 真服务基线覆盖）"
    );
    println!(
        "epoch 口径: group.switch_epoch=begin 即消费（尝试数）; ProgramObservation.switch_epoch=已委托 plan epoch（绝对值, 失败重试可跳号）"
    );

    let mut failures: Vec<String> = Vec::new();
    let mut findings: Vec<String> = Vec::new();

    // 阶段一: 干净世界判据段。
    let w1 = build_gate_world(
        cfg,
        devices,
        discovered,
        lm,
        _sup,
        agent_state,
        event_sink,
        projection_log,
    );
    phase_matrix(&mut failures, &mut findings, &w1);

    // 阶段二: 新鲜世界 R65 确定性段（C2b=F2×10 + F4）。
    let w2 = build_gate_world(
        cfg,
        devices,
        discovered,
        lm,
        _sup,
        agent_state,
        event_sink,
        projection_log,
    );
    phase2_c2b_f2_deterministic_loop(&mut failures, &mut findings, &w2);

    println!(
        "R64-MATRIX summary phase1_switches_delegated={} phase2_switches_delegated={} findings={} failures={}",
        w1.wrapper.switches_done.load(Ordering::SeqCst),
        w2.wrapper.switches_done.load(Ordering::SeqCst),
        findings.len(),
        failures.len()
    );
    for f in &findings {
        println!("R64-MATRIX FINDING: {f}");
    }
    if failures.is_empty() {
        println!(
            "R64-MATRIX verdict PASS-AND-FINDINGS（阶段一判据全绿 + 阶段二在案采集完整——exit 0; FINDING 归裁决）"
        );
        std::process::exit(0);
    }
    eprintln!(
        "R64-MATRIX verdict FAIL（{} 项断言失败——fail-closed exit 2）:",
        failures.len()
    );
    for f in &failures {
        eprintln!("  FAIL: {f}");
    }
    std::process::exit(2);
}

// ── 单元测试（纯函数 checker; 入盒 hw 测试腿）────────────────────────────

#[cfg(all(test, feature = "bmd-provider", feature = "gstreamer-backend"))]
mod tests {
    use super::*;

    fn snap(desired: Option<SwitchDesired>, observed: Option<uuid::Uuid>) -> SixPlaneSnapshot {
        let observed_str = observed.map(|u| u.to_string());
        SixPlaneSnapshot {
            command: Some(("executed".into(), Some("unknown".into()))),
            desired,
            group_switch_epoch: Some(3),
            observed_active: observed,
            program_switch_epoch: Some(3),
            timeline_epoch: Some(1),
            timeline_source: observed,
            api_status: 200,
            api_present: true,
            api_observed_active: observed_str,
            api_switch_epoch: Some(3),
            api_timeline_epoch: Some(1),
        }
    }

    #[test]
    fn r64_checker_active_consistent_ok() {
        let x = uuid::Uuid::new_v4();
        let s = snap(Some(SwitchDesired::ActiveInput(x)), Some(x));
        assert!(check_quiescent_consistency(&s, &Quiescence::Active(x)).is_empty());
    }

    #[test]
    fn r64_checker_active_cross_plane_contradiction_detected() {
        let x = uuid::Uuid::new_v4();
        let y = uuid::Uuid::new_v4();
        // desired=x 而 observed=y（静息点跨平面矛盾）。
        let mut s = snap(Some(SwitchDesired::ActiveInput(x)), Some(y));
        s.timeline_source = Some(y);
        s.api_observed_active = Some(y.to_string());
        let errs = check_quiescent_consistency(&s, &Quiescence::Active(x));
        assert!(!errs.is_empty(), "静息点 desired≠observed 必须判矛盾");
        // API 读到非当前事实（epoch 与内部观测不等）。
        let mut s2 = snap(Some(SwitchDesired::ActiveInput(x)), Some(x));
        s2.api_switch_epoch = Some(9);
        assert!(check_quiescent_consistency(&s2, &Quiescence::Active(x))
            .iter()
            .any(|e| e.contains("switch_epoch")));
    }

    #[test]
    fn r64_checker_group_epoch_below_executed_is_contradiction() {
        let x = uuid::Uuid::new_v4();
        let mut s = snap(Some(SwitchDesired::ActiveInput(x)), Some(x));
        s.group_switch_epoch = Some(2); // 尝试数 < 实际翻转数——不可能
        let errs = check_quiescent_consistency(&s, &Quiescence::Active(x));
        assert!(errs.iter().any(|e| e.contains("尝试数")));
        // 相等与大于均合法（分叉记账: 尝试数≥执行数）。
        s.group_switch_epoch = Some(3);
        assert!(check_quiescent_consistency(&s, &Quiescence::Active(x)).is_empty());
        s.group_switch_epoch = Some(4);
        assert!(check_quiescent_consistency(&s, &Quiescence::Active(x)).is_empty());
    }

    #[test]
    fn r64_checker_recovery_terminal_allows_observed_divergence() {
        let from = uuid::Uuid::new_v4();
        let to = uuid::Uuid::new_v4();
        // 闩锁后观测可恢复（observed=from 真实值）——设计语义: 拒绝猜测。
        let s = snap(
            Some(SwitchDesired::RecoveryRequired { from, to }),
            Some(from),
        );
        assert!(
            check_quiescent_consistency(&s, &Quiescence::RecoveryRequiredTerminal { from, to })
                .is_empty(),
            "终态闩锁后观测与 desired 分叉=设计语义, 非矛盾"
        );
        // 但 desired 退化回 ActiveInput = 丧失终态——矛盾。
        let s2 = snap(Some(SwitchDesired::ActiveInput(from)), Some(from));
        assert!(!check_quiescent_consistency(
            &s2,
            &Quiescence::RecoveryRequiredTerminal { from, to }
        )
        .is_empty());
    }

    #[test]
    fn r64_checker_teardown_requires_all_absent() {
        let s = SixPlaneSnapshot {
            command: Some(("executed".into(), None)),
            desired: None,
            group_switch_epoch: None,
            observed_active: None,
            program_switch_epoch: None,
            timeline_epoch: None,
            timeline_source: None,
            api_status: 200,
            api_present: false,
            api_observed_active: None,
            api_switch_epoch: None,
            api_timeline_epoch: None,
        };
        assert!(check_quiescent_consistency(&s, &Quiescence::Teardown).is_empty());
        // 任一平面残留 = 矛盾。
        let mut s2 = s.clone();
        s2.api_present = true;
        assert!(!check_quiescent_consistency(&s2, &Quiescence::Teardown).is_empty());
        let mut s3 = s.clone();
        s3.observed_active = Some(uuid::Uuid::new_v4());
        assert!(!check_quiescent_consistency(&s3, &Quiescence::Teardown).is_empty());
    }
}
