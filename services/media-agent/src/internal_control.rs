//! RCE-01A (RUNTIME-CONTROL-ENTRY-01·D1/D2): Internal Runtime Control 面 ——
//! `/internal/v1/agent` 上的 JSON-RPC 2.0 四方法。
//!
//! 设计（frozen plan `2026-09-20-runtime-control-entry-01-planning.md`）:
//! - **R1/R2 裁定**: 现 `/api/v1/*` 五端点 = prototype/diagnostic（生产 503 契约不变）;
//!   Product `/api/v1/*` 归 Fastify; internal Runtime Control = 本模块
//!   （EXTERNAL_API_CONTRACT §1 `/internal/v1/*` 命名空间 × TECHNOLOGY_STACK
//!   "JSON-RPC" wire 形态, 两份 frozen 文档零修改）。
//! - **四方法封闭词表**（D2, 快照测试锁定）: `runtime.query` / `command.dispatch` /
//!   `events.projection` / `agent.health`。语义零新发明——全部为既有冻结三平面
//!   （RuntimeQuery / CommandIdempotency 包 command::dispatch / EventProjection）
//!   的薄封装, 经 api_boundary 独立资源模型出 wire。
//! - **两平面红线（R4）**: 命令响应 Executed/Replayed/Conflict/Rejected = 幂等裁决
//!   + 同步薄映射结果（失败经 classification 传达, 不暴露 failed——0.7C-7 冻结）。
//! - **actual state 唯一来源（R4）**: 查询面（SessionPhase / agent.health）;
//!   JSON-RPC 成功 ≠ SignalVerified ≠ Runtime healthy。
//! - **std-only 纪律延续**（无新依赖; per-connection thread + socket 超时, 同
//!   transport R62/R63 加固形态）; `Connection: close` 协议模型。
//! - **暴露边界（D3/用户 §二十二 P1-2）**: 经 `MEDIA_AGENT_RPC_BIND` 绑定
//!   （默认 `127.0.0.1:50051`; 仅 localhost/UDS, 由 Config 安全告警面约束）。
//!   Rust 不做 API Gateway/Auth（Fastify 职责）。
//! - **RH-IDEM-01 不触发（R5）**: 进程内 CommandIdempotency = 内部传输重试守卫;
//!   durable idempotency 解冻点 = external Fastify command entry。

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::api_boundary::{to_api_query_snapshot, ApiCommandRequest, ApiProjectionResponse};
use crate::idempotency::CommandIdempotency;
use crate::runtime_query::RuntimeQuery;
use crate::transport::{
    map_command_request, map_dispatch, parse_request, ParsedRequest, MAX_REQUEST_BYTES,
};

/// JSON-RPC 方法封闭词表（D2; 新方法须过架构评审并显式更新快照测试）。
pub const METHODS: &str = "runtime.query/command.dispatch/events.projection/agent.health";

/// Internal control 路径（EXTERNAL_API_CONTRACT §1 `/internal/v1/*` 命名空间）。
pub const AGENT_PATH: &str = "/internal/v1/agent";

/// Internal control 依赖上下文——与 prototype `TransportContext` 分离（生产组合根
/// 只装配本面; prototype `/api/v1/*` 的 query/idem 维持 None ⇒ 503 契约不变）。
/// query/idem 持 Option: 组合根构造完成前面未装配 ⇒ HTTP 503 诚实（F6）。
#[derive(Clone)]
pub struct InternalControlContext {
    pub events: Arc<crate::events::RuntimeEventLog>,
    pub agent_state: Arc<Mutex<crate::health::AgentState>>,
    pub device_count: usize,
    pub query: Option<Arc<RuntimeQuery>>,
    pub idem: Option<Arc<CommandIdempotency>>,
    /// program_switch 事实回读（Query Plane 通道; None ⇒ 投影块诚实缺席——与
    /// TransportContext 同语义, R60 裁决③）。
    pub switch_readback: Option<Arc<dyn crate::switch_dispatch_plane::SwitchReadbackPlane>>,
}

fn rpc_error(code: i32, message: impl Into<String>) -> Value {
    serde_json::json!({ "code": code, "message": message.into() })
}

fn rpc_response(id: Value, result: Result<Value, Value>) -> (u16, String) {
    let body = match result {
        Ok(r) => serde_json::json!({ "jsonrpc": "2.0", "result": r, "id": id }),
        Err(e) => serde_json::json!({ "jsonrpc": "2.0", "error": e, "id": id }),
    };
    (200, body.to_string())
}

/// **路由**: internal control 面 (method, path, body) → (status, json)。纯逻辑, 注入
/// ctx 可测（与 transport::route 同测试形态）。
pub fn route_internal(
    method: &str,
    path: &str,
    body: &[u8],
    ctx: &InternalControlContext,
) -> (u16, String) {
    match (method, path) {
        ("POST", AGENT_PATH) => handle_rpc(body, ctx),
        (_, AGENT_PATH) => (405, error_body("method_not_allowed")),
        _ => (404, error_body("not_found")),
    }
}

/// JSON-RPC 2.0 envelope 处理（Value 级解析以区分传输层 400 与协议层错误码, D1）:
/// 非_json → HTTP 400; jsonrpc 版本错 → `-32600`; 缺 method → `-32602`;
/// 未知 method → `-32601`; params 形状错 → `-32602`; 面未装配 → HTTP 503。
fn handle_rpc(body: &[u8], ctx: &InternalControlContext) -> (u16, String) {
    let v: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return (400, error_body("malformed_json")),
    };
    let id = v.get("id").cloned().unwrap_or(Value::Null);
    if let Some(ver) = v.get("jsonrpc") {
        if ver != "2.0" {
            return rpc_response(
                id,
                Err(rpc_error(
                    -32600,
                    "invalid request: jsonrpc must be \"2.0\"",
                )),
            );
        }
    }
    let Some(method) = v.get("method").and_then(Value::as_str) else {
        return rpc_response(
            id,
            Err(rpc_error(-32602, "invalid request: missing method")),
        );
    };
    match method {
        "runtime.query" => {
            let Some(query) = &ctx.query else {
                return (
                    503,
                    error_body("service_unavailable: runtime.query (session runtime not active)"),
                );
            };
            let mut snap = to_api_query_snapshot(&query.get_runtime_state());
            crate::transport::apply_program_switch(&mut snap, ctx.switch_readback.as_deref());
            let val = serde_json::to_value(&snap).unwrap_or(Value::Null);
            rpc_response(id, Ok(val))
        }
        "command.dispatch" => {
            let Some(idem) = &ctx.idem else {
                return (
                    503,
                    error_body(
                        "service_unavailable: command.dispatch (session runtime not active)",
                    ),
                );
            };
            let req: ApiCommandRequest =
                match serde_json::from_value(v.get("params").cloned().unwrap_or(Value::Null)) {
                    Ok(r) => r,
                    Err(e) => {
                        return rpc_response(
                            id,
                            Err(rpc_error(-32602, format!("invalid params: {e}"))),
                        )
                    }
                };
            let env = match map_command_request(&req) {
                Ok(e) => e,
                Err(detail) => {
                    return rpc_response(
                        id,
                        Err(rpc_error(-32602, format!("invalid params: {detail}"))),
                    )
                }
            };
            let dispatch = idem.dispatch(&env);
            let resp = map_dispatch(&dispatch);
            let val = serde_json::to_value(&resp).unwrap_or(Value::Null);
            rpc_response(id, Ok(val))
        }
        "events.projection" => {
            let drained = ctx.events.drain();
            let proj = crate::event_projection::project(&drained);
            let resp: ApiProjectionResponse = (&proj).into();
            let val = serde_json::to_value(&resp).unwrap_or(Value::Null);
            rpc_response(id, Ok(val))
        }
        "agent.health" => {
            let val = crate::transport::health_snapshot_json(&ctx.agent_state, ctx.device_count);
            rpc_response(id, Ok(val))
        }
        _ => rpc_response(
            id,
            Err(rpc_error(
                -32601,
                format!("method not found: {method} (封闭词表: {METHODS})"),
            )),
        ),
    }
}

fn error_body(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

/// 单连接处理（读循环/超限防放大/`Connection: close`——transport::serve_connection
/// 同形态; 无静态文件面）。
pub fn serve_connection_internal(mut stream: TcpStream, ctx: &InternalControlContext) {
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    let mut parsed: Option<ParsedRequest> = None;
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > MAX_REQUEST_BYTES + 8192 {
                    break;
                }
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    if let Some(req) = parse_request(&buf) {
                        parsed = Some(req);
                        break;
                    }
                }
            }
            Err(_) => break,
        }
    }
    let (status, body): (u16, String) = match parsed {
        Some(req) => route_internal(&req.method, &req.path, &req.body, ctx),
        None if buf.is_empty() => (400, error_body("empty_request")),
        None => (400, error_body("malformed_request")),
    };
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        503 => "Service Unavailable",
        _ => "OK",
    };
    let mut resp = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    resp.extend_from_slice(body.as_bytes());
    let _ = stream.write_all(&resp);
    let _ = stream.flush();
}

/// accept → per-connection std thread + socket 超时（R62/R63 加固形态同
/// transport::serve_forever: 读 10s/写 30s, 连接并发 ≠ 命令执行并发——
/// dispatch 串行边界在 CommandIdempotency 既有锁内）。
pub fn serve_forever_internal(listener: TcpListener, ctx: InternalControlContext) {
    for stream in listener.incoming().flatten() {
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(30)));
        let ctx = ctx.clone();
        std::thread::spawn(move || serve_connection_internal(stream, &ctx));
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::graph_intent::{
        DeviceIntent, GraphRuntimeIntent, PipelineIntent, SinkIntent, SourceIntent,
    };
    use crate::session::SessionManager;

    /// mock world（command.rs 测试同形态: MockProvider 单输入 registry → SessionManager）。
    fn world() -> Arc<SessionManager> {
        use crate::adapters::mock::MockProvider;
        use crate::contracts::provider::HardwareProvider as _;
        use crate::port::*;
        let devices: Vec<crate::device::DeviceInfo> = MockProvider
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
        let event_log = Arc::new(crate::events::RuntimeEventLog::new());
        Arc::new(SessionManager::new(
            crate::resource::SharedResourceRegistry::new(
                crate::resource::ResourceRegistry::derive_from_discovery(&registry),
            ),
            Arc::new(crate::lease::InMemoryLeaseManager::new()),
            Arc::new(std::sync::Mutex::new(crate::supervisor::Supervisor::new(
                crate::supervisor::RestartPolicy::default(),
                event_log.clone(),
            ))),
            Arc::new(crate::adapters::mock::MockBackend),
            Arc::new(devices),
            Arc::new(std::collections::HashMap::new()),
            None,
            Some(registry),
            crate::pipeline::MaterializeMode::Diagnostic,
            crate::session::SessionTuning::default(),
            event_log,
        ))
    }

    fn ctx(mgr: &Arc<SessionManager>) -> InternalControlContext {
        InternalControlContext {
            events: Arc::new(crate::events::RuntimeEventLog::new()),
            agent_state: Arc::new(Mutex::new(crate::health::AgentState::Ready)),
            device_count: 1,
            query: Some(Arc::new(RuntimeQuery::new(mgr.clone()))),
            idem: Some(Arc::new(CommandIdempotency::new(mgr.clone()))),
            switch_readback: None,
        }
    }

    fn start_intent(mgr: &Arc<SessionManager>) -> GraphRuntimeIntent {
        let dev = mgr.runtime_state().devices[0].device_id;
        GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![DeviceIntent {
                device_id: dev.to_string(),
                role: "CAPTURE".into(),
                pipeline: PipelineIntent {
                    source: SourceIntent::decklink(dev.to_string(), None),
                    sink: SinkIntent {
                        kind: "appsink".into(),
                    },
                },
            }],
        }
    }

    /// JSON-RPC 请求体构造 + 响应解析辅助。
    fn rpc(ctx: &InternalControlContext, body: &str) -> (u16, Value) {
        let (code, resp) = route_internal("POST", AGENT_PATH, body.as_bytes(), ctx);
        let v: Value = serde_json::from_str(&resp).unwrap_or(Value::Null);
        (code, v)
    }

    fn dispatch_params(command_id: &str, kind: &str, target: Value) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "command.dispatch",
            "id": 1,
            "params": {
                "command_id": command_id,
                "kind": kind,
                "target": target,
                "requested_by": "internal-control-test"
            }
        })
        .to_string()
    }

    fn query_body() -> String {
        serde_json::json!({ "jsonrpc": "2.0", "method": "runtime.query", "id": 2 }).to_string()
    }

    /// **词表快照（D2）**: 四方法封闭 + 未知 method → -32601。
    #[test]
    fn internal_control_rt_01_method_vocab_snapshot() {
        let mgr = world();
        let c = ctx(&mgr);
        for m in [
            "runtime.query",
            "command.dispatch",
            "events.projection",
            "agent.health",
        ] {
            let body = serde_json::json!({ "jsonrpc": "2.0", "method": m, "id": 9 }).to_string();
            let (code, v) = rpc(&c, &body);
            assert_eq!(code, 200, "{m} 必须命中封闭词表");
            assert!(v.get("result").is_some() || v.get("error").is_some());
        }
        let (_, v) = rpc(&c, r#"{"jsonrpc":"2.0","method":"no.such.method","id":9}"#);
        assert_eq!(v["error"]["code"], -32601);
        assert!(
            v["error"]["message"].as_str().unwrap().contains(METHODS),
            "未知方法错误必须携带封闭词表"
        );
    }

    /// F1: 非 JSON → 400; 缺 method → -32602; jsonrpc 版本错 → -32600; params 形状错 → -32602。
    #[test]
    fn internal_control_rt_01_envelope_shape_errors() {
        let mgr = world();
        let c = ctx(&mgr);
        let (code, _) = rpc(&c, "not json at all");
        assert_eq!(code, 400, "非 JSON = 传输层 400");
        let (_, v) = rpc(&c, r#"{"jsonrpc":"2.0","id":1}"#);
        assert_eq!(v["error"]["code"], -32602, "缺 method → invalid params 层");
        let (_, v) = rpc(&c, r#"{"jsonrpc":"1.0","method":"agent.health","id":1}"#);
        assert_eq!(v["error"]["code"], -32600);
        let (_, v) = rpc(
            &c,
            r#"{"jsonrpc":"2.0","method":"command.dispatch","id":1,"params":{"kind":"bogus"}}"#,
        );
        assert_eq!(v["error"]["code"], -32602);
    }

    /// F2/F3: 同 envelope 重放原 outcome; 同 command_id 异 payload → Conflict（第二 payload 绝不执行）。
    #[test]
    fn internal_control_rt_01_idempotency_replay_and_conflict() {
        let mgr = world();
        let c = ctx(&mgr);
        let target = serde_json::json!({ "target_type": "session", "intent": start_intent(&mgr) });
        let body = dispatch_params(
            "11111111-1111-1111-1111-111111111111",
            "start_session",
            target.clone(),
        );
        let (_, v1) = rpc(&c, &body);
        assert_eq!(v1["result"]["status"]["status"], "executed");
        let (_, v2) = rpc(&c, &body);
        assert_eq!(
            v2["result"]["status"]["status"], "replayed",
            "D9-D: 重复命令重放原 outcome"
        );
        assert_eq!(v1["result"]["command_id"], v2["result"]["command_id"]);
        // 同 id 异 payload（StopSession）→ Conflict。
        let other = serde_json::json!({ "target_type": "session_by_id", "session_id": "22222222-2222-2222-2222-222222222222" });
        let body_conflict = dispatch_params(
            "11111111-1111-1111-1111-111111111111",
            "stop_session",
            other,
        );
        let (_, v3) = rpc(&c, &body_conflict);
        assert_eq!(v3["result"]["status"]["status"], "conflict");
    }

    /// F4: 形状拒绝（空 intent devices / 未知 kind）→ Rejected（未触 Runtime、未占 id）。
    #[test]
    fn internal_control_rt_01_shape_rejection() {
        let mgr = world();
        let c = ctx(&mgr);
        let empty_intent = serde_json::json!({ "target_type": "session", "intent": { "version": "1.0", "devices": [] } });
        let body = dispatch_params(
            "33333333-3333-3333-3333-333333333333",
            "start_session",
            empty_intent,
        );
        let (_, v) = rpc(&c, &body);
        assert_eq!(v["result"]["status"]["status"], "rejected");
        // 未占 id: 同 id 再发合法 payload 不冲突（形状拒绝不进裁决表, D9 语义）。
        let target = serde_json::json!({ "target_type": "session", "intent": start_intent(&mgr) });
        let body2 = dispatch_params(
            "33333333-3333-3333-3333-333333333333",
            "start_session",
            target,
        );
        let (_, v2) = rpc(&c, &body2);
        assert_eq!(v2["result"]["status"]["status"], "executed");
    }

    /// F5: SwitchProgram 无执行平面装配 → Rejected（诚实拒绝, 非 Failed）。
    #[test]
    fn internal_control_rt_01_switch_plane_absent_rejected() {
        let mgr = world();
        let c = ctx(&mgr); // switch_readback/idem 无 plane
        let target = serde_json::json!({
            "target_type": "switch_program",
            "session_id": "44444444-4444-4444-4444-444444444444",
            "target_device": mgr.runtime_state().devices[0].device_id.to_string()
        });
        let body = dispatch_params(
            "55555555-5555-5555-5555-555555555555",
            "switch_program",
            target,
        );
        let (_, v) = rpc(&c, &body);
        // frozen 语义（0.7C-4 分层）: 能力级诚实拒绝经 classification 传达——
        // 幂等裁决 = executed（claimant 已执行一次"拒绝"）, 分类 = rejected。
        assert_eq!(v["result"]["classification"], "rejected");
        assert!(
            v["result"]["detail"]
                .as_str()
                .unwrap_or("")
                .contains("switch_plane_unavailable"),
            "detail 必须诚实说明平面缺席"
        );
    }

    /// F6: 组合根未装配（query/idem None）→ HTTP 503 诚实契约。
    #[test]
    fn internal_control_rt_01_unconfigured_503() {
        let c = InternalControlContext {
            events: Arc::new(crate::events::RuntimeEventLog::new()),
            agent_state: Arc::new(Mutex::new(crate::health::AgentState::Starting)),
            device_count: 0,
            query: None,
            idem: None,
            switch_readback: None,
        };
        let (code, _) = rpc(&c, &query_body());
        assert_eq!(code, 503);
        let (code, _) = rpc(
            &c,
            &dispatch_params(
                "a",
                "stop_session",
                serde_json::json!({"target_type":"session_by_id","session_id":"00000000-0000-0000-0000-000000000000"}),
            ),
        );
        assert_eq!(code, 503);
    }

    /// F7: requested→actual——命令 Executed 后, actual state 唯一来源 = 查询面
    /// （session running / health 独立观察）。
    #[test]
    fn internal_control_rt_01_actual_state_via_query_plane() {
        let mgr = world();
        let c = ctx(&mgr);
        let target = serde_json::json!({ "target_type": "session", "intent": start_intent(&mgr) });
        let body = dispatch_params(
            "66666666-6666-6666-6666-666666666666",
            "start_session",
            target,
        );
        let (_, v) = rpc(&c, &body);
        assert_eq!(v["result"]["status"]["status"], "executed");
        let (_, q) = rpc(&c, &query_body());
        let sessions = q["result"]["sessions"].as_array().expect("sessions");
        assert!(!sessions.is_empty(), "查询面必须看到会话");
        assert_eq!(sessions[0]["state"], "running");
        assert_eq!(sessions[0]["phase"], "running");
        // 停止后再查: actual state 经查询面变迁。
        // wire 缝隙（如实登记）: 查询面 id 为显示形态 "session-<hex>", 命令面
        // 要求 canonical UUID —— 消费方自行映射; Product API 资源形状归 Fastify 阶段。
        let sid = sessions[0]["id"]
            .as_str()
            .unwrap()
            .trim_start_matches("session-")
            .to_string();
        let stop = dispatch_params(
            "77777777-7777-7777-7777-777777777777",
            "stop_session",
            serde_json::json!({ "target_type": "session_by_id", "session_id": sid }),
        );
        let (_, sv) = rpc(&c, &stop);
        assert_eq!(sv["result"]["status"]["status"], "executed");
        let (_, q2) = rpc(&c, &query_body());
        let after = q2["result"]["sessions"].as_array().expect("sessions");
        // stop ≠ release: 会话以非 running 状态留驻快照（release 才移除）——
        // actual state 经查询面如实投影, 命令 Executed 不冒充终局。
        assert!(
            after.iter().all(|s| s["state"] != "running"),
            "停止后查询面不得再报 running: {after:?}"
        );
    }

    /// F8 红线: 命令 Executed ≠ agent Ready——两平面独立观察（agent_state 仍 Starting）。
    #[test]
    fn internal_control_rt_01_command_success_not_agent_health() {
        let mgr = world();
        let mut c = ctx(&mgr);
        c.agent_state = Arc::new(Mutex::new(crate::health::AgentState::Starting));
        let target = serde_json::json!({ "target_type": "session", "intent": start_intent(&mgr) });
        let body = dispatch_params(
            "88888888-8888-8888-8888-888888888888",
            "start_session",
            target,
        );
        let (_, v) = rpc(&c, &body);
        assert_eq!(v["result"]["status"]["status"], "executed");
        let (_, h) = rpc(&c, r#"{"jsonrpc":"2.0","method":"agent.health","id":3}"#);
        assert_eq!(
            h["result"]["state"], "Starting",
            "命令成功不得冒充 Runtime healthy"
        );
    }

    /// F10: 并发重复 envelope 恰一次执行（D9-E）——1×executed + 3×replayed。
    #[test]
    fn internal_control_rt_01_concurrent_duplicate_single_execution() {
        let mgr = world();
        let c = Arc::new(ctx(&mgr));
        let target = serde_json::json!({ "target_type": "session", "intent": start_intent(&mgr) });
        let body = dispatch_params(
            "99999999-9999-9999-9999-999999999999",
            "start_session",
            target,
        );
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let c = c.clone();
                let b = body.clone();
                std::thread::spawn(move || rpc(&c, &b))
            })
            .collect();
        let results: Vec<Value> = handles
            .into_iter()
            .map(|h| h.join().expect("join").1)
            .collect();
        let executed = results
            .iter()
            .filter(|v| v["result"]["status"]["status"] == "executed")
            .count();
        let replayed = results
            .iter()
            .filter(|v| v["result"]["status"]["status"] == "replayed")
            .count();
        assert_eq!((executed, replayed), (1, 3), "并发同 envelope 恰一次执行");
    }

    /// 路由: GET → 405; 未知 path → 404（与 transport 路由表同测试形态）。
    #[test]
    fn internal_control_rt_01_route_table() {
        let mgr = world();
        let c = ctx(&mgr);
        let (code, _) = route_internal("GET", AGENT_PATH, b"", &c);
        assert_eq!(code, 405);
        let (code, _) = route_internal("POST", "/internal/v1/other", b"{}", &c);
        assert_eq!(code, 404);
    }

    /// 命令路径经由 transport 既有纯函数（command_id_from_string 确定性 v5 派生锚定）。
    #[test]
    fn internal_control_rt_01_command_id_derivation_anchor() {
        use crate::transport::command_id_from_string;
        let a = command_id_from_string("not-a-uuid");
        let b = command_id_from_string("not-a-uuid");
        assert_eq!(a, b, "同字符串确定性派生");
        let real = command_id_from_string("11111111-1111-1111-1111-111111111111");
        assert_eq!(real.0.to_string(), "11111111-1111-1111-1111-111111111111");
    }
}
