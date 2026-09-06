//! R63-B4: Control Plane 并发正确性探针（B-T1..B-T7 + B3 直测）。
//!
//! 依 B0 契约（2026-09-06-a2-8-05-r63-b-transport-concurrency.md §2）验证:
//! - **HTTP concurrency ≠ switch concurrency**: 切换执行（真实 5s 证据超时
//!   常量窗——inner 长持）期间 /health·/runtime·/events 并发 200 且毫秒级;
//! - **B3 查询快照**: 切换窗内 observe_execution 回退最近一次已提交事实
//!   快照（observed=切换前已提交值·诚实时间戳在载荷）, 切换完成后新鲜
//!   （R63-A 恢复落定可见）;
//! - **切换串行边界**: 双反向并发经 inner 串行——后到者在窗后合法执行
//!   （恢复后连续切换=B6 语义锚）;
//! - **replay**: 同 command_id 二发等 InFlight 完成后逐字节回放;
//! - **慢读者隔离**: 部分请求挂住连接不再冻结管理面（对照 R62 实测
//!   单 accept 模型冻结 10.175s）。
//!
//! 注入面: 本文件私有 `SuppressFactsAdapter`（包装 mock——首次切换证据
//! 永不闭合=真实 5s 编译期常量窗, R63-A 同型）; 服务端=直接调用
//! `transport::serve_forever`（即 bin 新环路本体）; HTTP 客户端=std
//! TcpStream 最小实现。全程 std-only。
#![cfg(feature = "mock")]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use media_agent::adapters::mock::MockBackend;
use media_agent::adapters::switch_mock::MockSwitchExecutionAdapter;
use media_agent::contracts::backend::MediaBackend;
use media_agent::contracts::switch::{
    CutoverDrainEvidence, ProgramExecutionObservation, SwitchAnchors, SwitchExecuted,
    SwitchExecutionAdapter, TimelineExecutionFacts,
};
use media_agent::events::RuntimeEventLog;
use media_agent::health::AgentState;
use media_agent::idempotency::CommandIdempotency;
use media_agent::pipeline::{PipelineHandle, PipelinePlan};
use media_agent::program::SwitchPolicy;
use media_agent::program_execution::ProgramExecutionRuntime;
use media_agent::program_timeline::{ProgramEpoch, ProgramTimelinePlan};
use media_agent::runtime_query::RuntimeQuery;
use media_agent::session::{SessionId, SessionInput, SessionManager};
use media_agent::switch_dispatch_plane::RuntimeSwitchPlane;
use media_agent::switch_execution::{
    ExecutionGroup, SwitchError, SwitchExecutionPlan, SwitchIntent,
};
use media_agent::transport::TransportContext;
use uuid::Uuid;

// ── 注入适配器（切换 5s 证据超时窗——真实编译期常量路径）────────────────

struct SuppressFactsAdapter {
    inner: MockSwitchExecutionAdapter,
    /// 恰首次 switch 成功期间为 true → 该次 ⑤-⑧ 证据永不闭合（真实 5s
    /// 超时窗）; 第二次切换（==2）恢复供证——B-T5 恢复后连续切换需要。
    suppress: AtomicBool,
    switches_done: AtomicU32,
}

impl SwitchExecutionAdapter for SuppressFactsAdapter {
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
        self.inner.sample_switch_anchors(graph, target)
    }
    fn timeline_execution_facts(&self, graph: &PipelineHandle) -> Option<TimelineExecutionFacts> {
        if self.suppress.load(Ordering::SeqCst) && self.switches_done.load(Ordering::SeqCst) == 1 {
            return None;
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
        self.inner.release_cutover_fence(graph, timeout)
    }
    fn force_release_cutover_fence(&self, graph: &PipelineHandle) -> Result<u64, SwitchError> {
        self.inner.force_release_cutover_fence(graph)
    }
    fn observe(&self, graph: &PipelineHandle) -> ProgramExecutionObservation {
        self.inner.observe(graph)
    }
    fn stop_program(&self, graph: &PipelineHandle) -> Result<(), SwitchError> {
        self.inner.stop_program(graph)
    }
}

// ── 脚手架 ────────────────────────────────────────────────────────────

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
    let event_log = Arc::new(RuntimeEventLog::new());
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

struct Srv {
    addr: SocketAddr,
    runtime: Arc<ProgramExecutionRuntime>,
    adapter: Arc<SuppressFactsAdapter>,
    sid: SessionId,
    a: Uuid,
    b: Uuid,
}

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

/// 起一个 per-connection 并发服务（=transport::serve_forever·bin 新环路本体）
/// + 慢切换注入 runtime。
fn server() -> Srv {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let sid = SessionId(Uuid::new_v4());
    let adapter = Arc::new(SuppressFactsAdapter {
        inner: MockSwitchExecutionAdapter::new(),
        suppress: AtomicBool::new(true),
        switches_done: AtomicU32::new(0),
    });
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
    let plane = Arc::new(RuntimeSwitchPlane::new(sid, runtime.clone()));
    let mgr = session_mgr();
    let ctx = TransportContext {
        events: Arc::new(RuntimeEventLog::new()),
        agent_state: Arc::new(Mutex::new(AgentState::Ready)),
        device_count: 2,
        query: Some(Arc::new(RuntimeQuery::new(mgr.clone()))),
        idem: Some(Arc::new(
            CommandIdempotency::new(mgr).with_switch_plane(plane.clone()),
        )),
        hls_dir: None,
        switch_readback: Some(plane),
    };
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || media_agent::transport::serve_forever(listener, ctx));
    Srv {
        addr,
        runtime,
        adapter,
        sid,
        a,
        b,
    }
}

fn intent(target: Uuid) -> SwitchIntent {
    SwitchIntent {
        target,
        policy: SwitchPolicy::FrameSwitch,
    }
}

/// 最小 HTTP 客户端: (status, body, elapsed)。8s 读超时（切换串行等待上限）。
fn http(addr: SocketAddr, method: &str, path: &str, body: Option<&str>) -> (u16, String, Duration) {
    let t = Instant::now();
    let mut s = TcpStream::connect(addr).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(8))).unwrap();
    let b = body.unwrap_or("");
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: t\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        b.len()
    );
    s.write_all(head.as_bytes()).expect("write head");
    if !b.is_empty() {
        s.write_all(b.as_bytes()).expect("write body");
    }
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
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
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    let body_part = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body_part, dt)
}

fn switch_body(sid: &SessionId, cid: Uuid, target: Uuid) -> String {
    format!(
        "{{\"command_id\":\"{cid}\",\"kind\":\"switch_program\",\"target\":{{\"target_type\":\"switch_program\",\"session_id\":\"{}\",\"target_device\":\"{target}\"}},\"requested_by\":\"r63b\"}}",
        sid.0
    )
}

/// 后台发起一次切换 POST（返回 join 句柄——响应体含 dispatch+outcome）。
fn spawn_switch(
    srv: &Srv,
    cid: Uuid,
    target: Uuid,
) -> std::thread::JoinHandle<(u16, String, Duration)> {
    let addr = srv.addr;
    let sid = srv.sid;
    std::thread::spawn(move || {
        http(
            addr,
            "POST",
            "/api/v1/commands",
            Some(&switch_body(&sid, cid, target)),
        )
    })
}

// ── B3 直测: 切换窗内 observe_execution 毫秒级回退已提交快照 ──────────

#[test]
fn r63b_b3_observe_snapshot_during_switch() {
    let srv = server();
    let rt = srv.runtime.clone();
    let (a, b) = (srv.a, srv.b);
    let handle = std::thread::spawn(move || rt.switch_program(&intent(b)));
    // 进入证据窗（switch 已执行、inner 被编排长持）。
    let deadline = Instant::now() + Duration::from_secs(2);
    while srv.adapter.switches_done.load(Ordering::SeqCst) < 1 {
        assert!(Instant::now() < deadline, "switch 未执行");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(200));
    let mut worst = Duration::ZERO;
    for _ in 0..10 {
        let t = Instant::now();
        let obs = srv
            .runtime
            .observe_execution()
            .expect("切换窗内查询必须返回（快照回退）");
        let dt = t.elapsed();
        worst = worst.max(dt);
        assert_eq!(
            obs.program.observed_active,
            Some(a),
            "窗内=上一次已提交事实（初始快照·observed=旧源）"
        );
        assert!(
            dt < Duration::from_millis(500),
            "B3: 查询不得等待切换编排: {dt:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    println!("R63B-MATRIX b3 worst_query_latency={worst:?} (5s switch window)");
    let err = handle.join().unwrap().expect_err("5s 证据超时");
    assert!(
        matches!(&err, SwitchError::Backend(s) if s.contains("证据超时")),
        "{err:?}"
    );
    // 切换完成+R63-A 恢复后: 新鲜读——落定 B（已执行证据失败按观测落 B）。
    let obs = srv.runtime.observe_execution().expect("obs");
    assert_eq!(obs.program.observed_active, Some(b), "恢复落定可见");
    assert_eq!(obs.timeline.program_epoch, ProgramEpoch(1));
    srv.runtime.teardown();
    assert_eq!(srv.runtime.observe_execution(), None, "teardown 后诚实缺席");
}

// ── B-T1/B-T2/B-T3: 切换窗内查询端点并发 200 且 <1s ───────────────────

#[test]
fn r63b_t1_health_unblocked_during_switch() {
    let srv = server();
    let sw = spawn_switch(&srv, Uuid::new_v4(), srv.b);
    let deadline = Instant::now() + Duration::from_secs(2);
    while srv.adapter.switches_done.load(Ordering::SeqCst) < 1 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(100));
    let mut worst = Duration::ZERO;
    for _ in 0..3 {
        let (status, body, dt) = http(srv.addr, "GET", "/health", None);
        assert_eq!(status, 200, "body={body}");
        worst = worst.max(dt);
        assert!(dt < Duration::from_secs(1), "B-T1: {dt:?}");
    }
    let (status, _, _) = sw.join().unwrap();
    assert_eq!(status, 200);
    println!("R63B-MATRIX t1 health_worst={worst:?} during 5s switch");
    srv.runtime.teardown();
}

#[test]
fn r63b_t2_runtime_snapshot_during_switch() {
    let srv = server();
    let sw = spawn_switch(&srv, Uuid::new_v4(), srv.b);
    let deadline = Instant::now() + Duration::from_secs(2);
    while srv.adapter.switches_done.load(Ordering::SeqCst) < 1 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(100));
    let (status, body, dt) = http(srv.addr, "GET", "/api/v1/runtime", None);
    assert_eq!(status, 200, "body={body}");
    assert!(dt < Duration::from_secs(1), "B-T2: {dt:?}");
    assert!(body.contains("\"program_switch\""), "投影块在: {body}");
    assert!(
        body.contains(&format!("\"observed_active\":\"{}\"", srv.a)),
        "窗内=已提交快照（observed=切换前旧源·非执行中实时值）: {body}"
    );
    let (status, _, _) = sw.join().unwrap();
    assert_eq!(status, 200);
    // 切换完成后: 新鲜读=B（恢复落定）。
    let (_, body2, _) = http(srv.addr, "GET", "/api/v1/runtime", None);
    assert!(
        body2.contains(&format!("\"observed_active\":\"{}\"", srv.b)),
        "{body2}"
    );
    println!("R63B-MATRIX t2 runtime_during={dt:?} snapshot_observed=a then=b");
    srv.runtime.teardown();
}

#[test]
fn r63b_t3_events_unblocked_during_switch() {
    let srv = server();
    let sw = spawn_switch(&srv, Uuid::new_v4(), srv.b);
    let deadline = Instant::now() + Duration::from_secs(2);
    while srv.adapter.switches_done.load(Ordering::SeqCst) < 1 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(100));
    let (status, body, dt) = http(srv.addr, "GET", "/api/v1/events/projection", None);
    assert_eq!(status, 200, "body={body}");
    assert!(dt < Duration::from_secs(1), "B-T3: {dt:?}");
    let (status, _, _) = sw.join().unwrap();
    assert_eq!(status, 200);
    println!("R63B-MATRIX t3 events_during={dt:?}");
    srv.runtime.teardown();
}

// ── B-T4: 切换进行中同 command_id 二发 = 等待完成后逐字节回放 ──────────

#[test]
fn r63b_t4_replay_during_switch_byte_identical() {
    let srv = server();
    let cid = Uuid::new_v4();
    let first = spawn_switch(&srv, cid, srv.b);
    let deadline = Instant::now() + Duration::from_secs(2);
    while srv.adapter.switches_done.load(Ordering::SeqCst) < 1 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    // 切换仍在窗内: 同 id 二发应等待 InFlight 完成后回放（不二次执行）。
    let second = spawn_switch(&srv, cid, srv.b);
    let (s1, b1, d1) = first.join().unwrap();
    let (s2, b2, d2) = second.join().unwrap();
    assert_eq!(s1, 200);
    assert_eq!(s2, 200);
    assert!(
        d2 >= Duration::from_secs(3),
        "二发应等待 InFlight（~5s 证据超时）完成: {d2:?}"
    );
    let v1: serde_json::Value = serde_json::from_str(&b1).expect("json");
    let v2: serde_json::Value = serde_json::from_str(&b2).expect("json");
    assert_eq!(v1["detail"], v2["detail"], "detail 逐字节");
    assert_eq!(v1["classification"], v2["classification"]);
    assert_eq!(v1["command_id"], v2["command_id"]);
    assert!(
        b1.contains("证据超时"),
        "原 outcome=Failed（不伪装成功）: {b1}"
    );
    println!("R63B-MATRIX t4 replay_detail_identical first={d1:?} second={d2:?}");
    srv.runtime.teardown();
}

// ── B-T5: 双反向并发——inner 串行边界 + 恢复后连续切换（B6 锚）─────────

#[test]
fn r63b_t5_double_opposite_serialized_by_inner() {
    let srv = server();
    // 先发 A→B（5s 证据超时窗·长持 inner）; 窗内后发 B→A——后者在 inner
    // 串行边界排队; 前者失败+R63-A 恢复落 B 后, 后者合法执行（恢复后
    // 连续切换=B6 语义锚）。
    let to_b = spawn_switch(&srv, Uuid::new_v4(), srv.b);
    let deadline = Instant::now() + Duration::from_secs(2);
    while srv.adapter.switches_done.load(Ordering::SeqCst) < 1 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(100));
    let to_a = spawn_switch(&srv, Uuid::new_v4(), srv.a);
    let (s1, b1, _) = to_b.join().unwrap();
    let (s2, b2, d2) = to_a.join().unwrap();
    assert_eq!(s1, 200);
    assert_eq!(s2, 200);
    assert!(b1.contains("证据超时"), "首个=Failed: {b1}");
    assert!(
        b2.contains("switch executed") && b2.contains("preserved"),
        "第二个在恢复后合法执行: {b2}"
    );
    assert!(
        d2 >= Duration::from_secs(3),
        "第二个应被 inner 串行（等待首个完成）: {d2:?}"
    );
    // 终态: 双向都消费——desired=Active(a)·epoch=2。
    let g = srv.runtime.group_arc().expect("group");
    let g = g.lock().unwrap();
    assert_eq!(
        g.desired,
        media_agent::switch_execution::SwitchDesired::ActiveInput(srv.a)
    );
    assert_eq!(g.switch_epoch, 2, "两次 begin 各消费一个 epoch");
    println!("R63B-MATRIX t5 serialized both_consumed epoch=2 final=Active(a)");
    srv.runtime.teardown();
}

// ── B-T6: 慢读者隔离——部分请求挂住连接不冻结管理面 ─────────────────────

#[test]
fn r63b_t6_slow_reader_isolated() {
    let srv = server();
    // 停滞读者: 连接后只发部分请求（无空行终止头）, 挂住。
    let mut slow = TcpStream::connect(srv.addr).expect("slow connect");
    slow.write_all(b"GET /health HTTP/1.1\r\nHost: t\r\n")
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    // 对照 R62: 单 accept 模型此处在慢读者超时前整管理面冻结（10.175s 实测）。
    let (status, body, dt) = http(srv.addr, "GET", "/health", None);
    assert_eq!(status, 200, "body={body}");
    assert!(
        dt < Duration::from_secs(2),
        "B-T6: 慢读者不得冻结管理面: {dt:?}"
    );
    println!("R63B-MATRIX t6 health_during_slow_reader={dt:?} (R62 同形=10.175s 冻结)");
    drop(slow);
    srv.runtime.teardown();
}

// ── B-T7: N=16 并发 /runtime 全 200 且 <2s ────────────────────────────

#[test]
fn r63b_t7_sixteen_concurrent_runtime_queries() {
    let srv = server();
    let mut handles = Vec::new();
    for _ in 0..16 {
        let addr = srv.addr;
        handles.push(std::thread::spawn(move || {
            http(addr, "GET", "/api/v1/runtime", None)
        }));
    }
    let mut worst = Duration::ZERO;
    for h in handles {
        let (status, body, dt) = h.join().unwrap();
        assert_eq!(status, 200, "body={body}");
        worst = worst.max(dt);
    }
    assert!(worst < Duration::from_secs(2), "B-T7 worst={worst:?}");
    println!("R63B-MATRIX t7 n=16 worst={worst:?}");
    srv.runtime.teardown();
}

// ── R64-3/4: 并发综合 storm——同一 5s 证据窗内 T1 切换 + T2×2 查询 + T4/T5
//    端点 + T6 同 id replay + T7 异 id 反向排队; 快照真值（窗内只读旧已提交
//    值·不读未来状态）; 查询只读。mock 确定性主证, 真机同型场景在 R64 gate。──

#[test]
fn r64_storm_switch_query_replay_matrix() {
    let srv = server();
    let cid = Uuid::new_v4();
    // T1: 后台首切（5s 证据窗·inner 被编排长持——suppress 仅首次切换）。
    let first = spawn_switch(&srv, cid, srv.b);
    let deadline = Instant::now() + Duration::from_secs(2);
    while srv.adapter.switches_done.load(Ordering::SeqCst) < 1 {
        assert!(Instant::now() < deadline, "switch 未执行");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(100));
    // 窗内并发查询四路（T2×2 + T4 + T5）: 全 200 且 <1s。
    let addr = srv.addr;
    let mut probes = Vec::new();
    for path in [
        "/api/v1/runtime",
        "/api/v1/runtime",
        "/health",
        "/api/v1/events/projection",
    ] {
        let a = addr;
        probes.push(std::thread::spawn(move || http(a, "GET", path, None)));
    }
    for (i, p) in probes.into_iter().enumerate() {
        let (s, _b, dt) = p.join().unwrap();
        assert_eq!(s, 200, "storm probe#{i}");
        assert!(dt < Duration::from_secs(1), "storm probe#{i} 延迟 {dt:?}");
    }
    // 快照真值: 窗内 GET observed==旧已提交 a（不得读到未来 b）。
    let (s, body, _) = http(addr, "GET", "/api/v1/runtime", None);
    assert_eq!(s, 200);
    assert!(
        body.contains(&format!("\"observed_active\":\"{}\"", srv.a)),
        "窗内=上一次已提交事实（observed=旧源 a）: {body}"
    );
    // T6 同 id replay（等待 InFlight 完成后原样）+ T7 异 id 反向排队（inner 串行）。
    let replay = spawn_switch(&srv, cid, srv.b);
    let queued = spawn_switch(&srv, Uuid::new_v4(), srv.a);
    let (s1, b1, _) = first.join().unwrap();
    let (sr, br, _) = replay.join().unwrap();
    let (sq, bq, dq) = queued.join().unwrap();
    assert_eq!(s1, 200);
    assert!(b1.contains("证据超时"), "首个=Failed（不伪装成功）: {b1}");
    assert_eq!(sr, 200);
    let j1: serde_json::Value = serde_json::from_str(&b1).expect("json");
    let jr: serde_json::Value = serde_json::from_str(&br).expect("json");
    assert_eq!(
        jr["status"]["status"], "replayed",
        "同 id 二发=Replayed: {br}"
    );
    assert_eq!(j1["detail"], jr["detail"], "replay detail 逐字节");
    assert_eq!(j1["classification"], jr["classification"]);
    assert_eq!(j1["command_id"], jr["command_id"]);
    assert_eq!(sq, 200);
    assert!(
        bq.contains("switch executed") && bq.contains("preserved"),
        "异 id 在恢复后串行合法执行: {bq}"
    );
    assert!(
        dq >= Duration::from_secs(3),
        "异 id 应被 inner 串行（等待首个完成）: {dq:?}"
    );
    // 终态: 双消费 epoch=2、唯一 Active(a)——与 t5 同锚。
    let g = srv.runtime.group_arc().expect("group");
    let g = g.lock().unwrap();
    assert_eq!(
        g.desired,
        media_agent::switch_execution::SwitchDesired::ActiveInput(srv.a)
    );
    assert_eq!(g.switch_epoch, 2, "两次 begin 各消费一个 epoch");
    drop(g);
    // 查询只读: 连续 observe 不改变 epoch。
    let e1 = srv
        .runtime
        .observe_execution()
        .expect("obs")
        .program
        .switch_epoch;
    let _ = srv.runtime.observe_execution();
    let e2 = srv
        .runtime
        .observe_execution()
        .expect("obs")
        .program
        .switch_epoch;
    assert_eq!(e1, e2, "查询只读");
    println!("R64-MATRIX storm(mock) queries_fast observed_committed replay_identical queued_serialized epoch=2 final=Active(a)");
    srv.runtime.teardown();
}
