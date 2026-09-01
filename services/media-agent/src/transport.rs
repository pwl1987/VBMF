//! Phase 0.7C-8: Transport 实现 — **API Boundary Model → wire 的序列化边界**。
//!
//! 0.7C-7 终审 + Contract Probe 锁定:
//! - **std-only 纪律**: 仅 std + serde_json + uuid, 零新 transport 依赖 (禁 axum/hyper/tower)。
//! - **五端点不发明**: `GET /health` (行为不变, 回归锚点) / `GET /api/v1/runtime` /
//!   `POST /api/v1/commands` / `GET /api/v1/events/projection` /
//!   `GET /api/v1/idempotency/boundary`; 未知 404 / 方法错 405 / 无 mgr 503。
//! - **无持久连接**: 每连接一请求后关闭 (与 /health 既有单 accept 循环模型一致, 不偷升级)。
//! - **零触碰**: api_boundary / command / idempotency / runtime_query / event_projection /
//!   rpc 契约零改动; 本模块只做纯函数映射 + 路由 + 序列化。
//!
//! 红线: Observation≠Configuration / Semantic Intent≠Execution Plan / 0.7C-3 不可执行性
//! (map_command_request 零执行字段) / 0.7C-5 三平面分离 (status+classification 独立) /
//! 0.7C-7 NOTE (snapshot_kind 守门 / API 模型独立 / 不暴露 serde tag)。

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::api_boundary::{
    default_idempotency_boundary, to_api_query_snapshot, ApiCommandRequest, ApiCommandResponse,
    ApiErrorClass, ApiProjectionResponse, ApiQuerySnapshot,
};
use crate::command::{CommandEnvelope, CommandKind, CommandTarget};
use crate::graph_intent::GraphRuntimeIntent;
use crate::idempotency::{CommandIdempotency, IdempotentDispatch};
use crate::runtime_query::RuntimeQuery;
use crate::session::SessionId;

/// 请求体限长 (1 MiB, 防内存放大)。
pub const MAX_REQUEST_BYTES: usize = 1_048_576;

/// 非 UUID 字符串 command_id 的确定性 v5 派生命名空间 (幂等键稳定)。
/// 固定 128-bit 常量 (8-4-4-4-12 形态, 32 hex 数字)。
pub const COMMAND_ID_NAMESPACE: Uuid = Uuid::from_u128(0x62b79f8c_1a2e_4c3d_9f0b_5d6e7a8b9c0d);

/// Transport 依赖上下文。Query/Command 持 Option (生产路径无 mgr → 503 契约诚实);
/// events/agent_state/device_count 全路径可用。
#[derive(Clone)]
pub struct TransportContext {
    pub events: Arc<crate::events::RuntimeEventLog>,
    pub agent_state: Arc<Mutex<crate::health::AgentState>>,
    pub device_count: usize,
    pub query: Option<Arc<RuntimeQuery>>,
    pub idem: Option<Arc<CommandIdempotency>>,
}

/// 请求解析结果 (method, path, body)。
pub struct ParsedRequest {
    pub method: String,
    pub path: String,
    pub body: Vec<u8>,
}

/// **纯函数**: 从原始字节解析 HTTP 请求 (请求行 + Content-Length + body 限长)。
/// 畸形/超限/方法非 GET|POST → None (serve_connection 回 400)。
pub fn parse_request(buf: &[u8]) -> Option<ParsedRequest> {
    // 头部终止符 CRLFCRLF。
    let header_end = find_subslice(buf, b"\r\n\r\n")?;
    let head = std::str::from_utf8(&buf[..header_end]).ok()?;
    let lines: Vec<&str> = head.split("\r\n").collect();
    let request_line = lines.first()?.trim_end();
    let mut parts = request_line.splitn(3, ' ');
    let method = parts.next()?.to_ascii_uppercase();
    let path = parts.next()?.to_string();
    let _version = parts.next()?;
    if method != "GET" && method != "POST" {
        return None;
    }
    if path.is_empty() || !path.starts_with('/') {
        return None;
    }
    // Content-Length (仅取此头部; 缺失=空 body)。
    let mut content_length = 0usize;
    for line in lines.iter().skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse::<usize>().ok()?;
            }
        }
    }
    if content_length > MAX_REQUEST_BYTES {
        return None;
    }
    let body_start = header_end + 4;
    let body_part = &buf[body_start..];
    if body_part.len() < content_length {
        return None; // body 不完整 (单读模型: 视为畸形)。
    }
    Some(ParsedRequest {
        method,
        path,
        body: body_part[..content_length].to_vec(),
    })
}

/// **纯函数**: command_id 字符串 → CommandId (合法 UUID 直用; 否则确定性 v5 派生)。
pub fn command_id_from_string(s: &str) -> crate::command::CommandId {
    match Uuid::parse_str(s) {
        Ok(u) => crate::command::CommandId(u),
        Err(_) => crate::command::CommandId(Uuid::new_v5(&COMMAND_ID_NAMESPACE, s.as_bytes())),
    }
}

/// **纯函数**: ApiCommandRequest → CommandEnvelope (形状映射, 零执行字段)。
/// Err = 400 detail (未触 Runtime); 形状对但语义拒绝由 dispatch 平面表达 (200+Rejected)。
pub fn map_command_request(req: &ApiCommandRequest) -> Result<CommandEnvelope, String> {
    // 封闭词表守卫 (0.7C-3 不可执行性: 形状层拒绝, 未触 Runtime)。
    const KIND_VOCAB: &str = "start_session/stop_session/release_session";
    if !matches!(
        req.kind.as_str(),
        "start_session" | "stop_session" | "release_session"
    ) {
        return Err(format!(
            "unknown_command_kind: {} (封闭词表: {KIND_VOCAB})",
            req.kind
        ));
    }
    let kind = match req.kind.as_str() {
        "start_session" => CommandKind::StartSession,
        "stop_session" => CommandKind::StopSession,
        _ => CommandKind::ReleaseSession, // 守卫已排除未知值, 此处必为 release_session
    };
    let target = match &req.target {
        crate::api_boundary::ApiCommandTarget::SessionById { session_id } => {
            let u = Uuid::parse_str(session_id)
                .map_err(|_| format!("invalid_session_id: {session_id} (须 canonical UUID)"))?;
            CommandTarget::SessionById {
                session_id: SessionId(u),
            }
        }
        crate::api_boundary::ApiCommandTarget::Session { intent } => {
            let intent = serde_json::from_value::<GraphRuntimeIntent>(intent.clone())
                .map_err(|e| format!("invalid_intent: {e}"))?;
            CommandTarget::Session { intent }
        }
    };
    Ok(CommandEnvelope {
        command_id: command_id_from_string(&req.command_id),
        kind,
        target,
        issued_at_ms: 0,
        requested_by: req.requested_by.clone(),
    })
}

/// **纯函数**: IdempotentDispatch → ApiCommandResponse (四出口封闭, **不暴露 Failed**;
/// 失败归因经 classification 传达 — 0.7C-5 三平面分离 + 0.7C-7 NOTE-2)。
pub fn map_dispatch(d: &IdempotentDispatch) -> ApiCommandResponse {
    let kind_str = |k: &CommandKind| match k {
        CommandKind::StartSession => "start_session",
        CommandKind::StopSession => "stop_session",
        CommandKind::ReleaseSession => "release_session",
    };
    match d {
        IdempotentDispatch::Executed(o) => ApiCommandResponse {
            command_id: o.command_id.0.to_string(),
            status: crate::api_boundary::ApiCommandStatus::Executed,
            kind: kind_str(&o.kind).into(),
            classification: o.classification.as_ref().map(ApiErrorClass::from),
            detail: o.detail.clone(),
        },
        IdempotentDispatch::Replayed(o) => ApiCommandResponse {
            command_id: o.command_id.0.to_string(),
            status: crate::api_boundary::ApiCommandStatus::Replayed,
            kind: kind_str(&o.kind).into(),
            classification: o.classification.as_ref().map(ApiErrorClass::from),
            detail: o.detail.clone(),
        },
        IdempotentDispatch::Conflict { .. } => ApiCommandResponse {
            command_id: String::new(),
            status: crate::api_boundary::ApiCommandStatus::Conflict,
            kind: String::new(),
            classification: Some(crate::api_boundary::ApiErrorClass::Conflict),
            detail: Some("same command_id with different payload".into()),
        },
        IdempotentDispatch::Rejected(r) => ApiCommandResponse {
            command_id: String::new(),
            status: crate::api_boundary::ApiCommandStatus::Rejected,
            kind: String::new(),
            classification: Some(crate::api_boundary::ApiErrorClass::Rejected),
            detail: Some(format!("{}: {}", r.code, r.detail)),
        },
    }
}

/// **路由**: (method, path, body) → (status, json_body)。纯逻辑, 注入 ctx 可测。
pub fn route(method: &str, path: &str, body: &[u8], ctx: &TransportContext) -> (u16, String) {
    match (method, path) {
        ("GET", "/health") => {
            let st = *ctx.agent_state.lock().unwrap();
            let active = crate::pipeline_events::HEALTH_ARCS.lock().unwrap().len();
            let dropped = crate::pipeline::dropped_bus_events();
            let json = serde_json::json!({
                "state": st,
                "devices": ctx.device_count,
                "active_pipelines": active,
                "dropped_bus_events": dropped,
                "clock_lost_events": crate::pipeline::clock_lost_events(),
            });
            (200, json.to_string())
        }
        ("GET", "/api/v1/runtime") => {
            let Some(query) = &ctx.query else {
                return not_available("runtime");
            };
            let snap: ApiQuerySnapshot = to_api_query_snapshot(&query.get_runtime_state());
            (200, serde_json::to_string(&snap).unwrap_or_default())
        }
        ("POST", "/api/v1/commands") => {
            let Some(idem) = &ctx.idem else {
                return not_available("commands");
            };
            let req: ApiCommandRequest = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => return (400, error_body(&format!("malformed_command_request: {e}"))),
            };
            let env = match map_command_request(&req) {
                Ok(env) => env,
                Err(detail) => return (400, error_body(&detail)),
            };
            let dispatch = idem.dispatch(&env);
            let resp = map_dispatch(&dispatch);
            (200, serde_json::to_string(&resp).unwrap_or_default())
        }
        ("GET", "/api/v1/events/projection") => {
            let drained = ctx.events.drain();
            let proj = crate::event_projection::project(&drained);
            let resp: ApiProjectionResponse = (&proj).into();
            (200, serde_json::to_string(&resp).unwrap_or_default())
        }
        ("GET", "/api/v1/idempotency/boundary") => {
            let boundary = default_idempotency_boundary();
            (200, serde_json::to_string(&boundary).unwrap_or_default())
        }
        // 已知 path 错误 method → 405。
        (_, "/health")
        | (_, "/api/v1/runtime")
        | (_, "/api/v1/events/projection")
        | (_, "/api/v1/idempotency/boundary") => (405, error_body("method_not_allowed")),
        ("GET", "/api/v1/commands") => (405, error_body("method_not_allowed")),
        _ => (404, error_body("not_found")),
    }
}

fn not_available(endpoint: &str) -> (u16, String) {
    (
        503,
        error_body(&format!(
            "service_unavailable: {endpoint} (session runtime not active)"
        )),
    )
}

fn error_body(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

/// 连接处理: 读→解析→路由→写 (无持久连接, 处理完关闭)。
/// 读模型: 累积至 `parse_request` 成功 (头部+body 完整) 或连接关闭/超限;
/// 与 /health 既有单 accept 循环并发模型一致 (不偷升级线程池/async)。
pub fn serve_connection(mut stream: TcpStream, ctx: &TransportContext) {
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    let mut parsed: Option<ParsedRequest> = None;
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break, // 对端关闭。
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > MAX_REQUEST_BYTES + 8192 {
                    break; // 超限 (含头部), 防内存放大。
                }
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    if let Some(req) = parse_request(&buf) {
                        parsed = Some(req);
                        break;
                    }
                    // 头部完整但 body 未齐 (或畸形): 继续读至完整/关闭/超限。
                }
            }
            Err(_) => break,
        }
    }
    let (status, body) = match parsed {
        Some(req) => route(&req.method, &req.path, &req.body, ctx),
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
    let resp = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

/// 子串查找 (避免引入 memchr 依赖; 数据量小, 线性扫描足够)。
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

// === 单元测试 (feature 无关: 纯函数级) ========================================

#[cfg(test)]
mod tests {
    use super::*;

    /// parse_request: 合法 GET (无 body) / 合法 POST + body / 无 Content-Length /
    /// 超限 / 畸形请求行 / 未知方法。
    #[test]
    fn transport_rt_01_parse_request_shapes() {
        let get = b"GET /health HTTP/1.1\r\nHost: x\r\n\r\n";
        let r = parse_request(get).expect("GET 应解析");
        assert_eq!(r.method, "GET");
        assert_eq!(r.path, "/health");
        assert!(r.body.is_empty());

        let body = b"\"hello\"";
        let post = format!(
            "POST /api/v1/commands HTTP/1.1\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        );
        let r = parse_request(post.as_bytes()).expect("POST+body 应解析");
        assert_eq!(r.method, "POST");
        assert_eq!(r.path, "/api/v1/commands");
        assert_eq!(r.body, body);

        // body 不完整 (Content-Length 声明大于实际) → None。
        let short = b"POST /x HTTP/1.1\r\nContent-Length: 100\r\n\r\n\"ab\"";
        assert!(parse_request(short).is_none());

        // 超限 Content-Length → None。
        let huge = format!(
            "POST /x HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            MAX_REQUEST_BYTES + 1
        );
        assert!(parse_request(huge.as_bytes()).is_none());

        // 无头部终止符 → None。
        assert!(parse_request(b"GET /health HTTP/1.1").is_none());

        // 未知方法 → None。
        let del = b"DELETE /health HTTP/1.1\r\n\r\n";
        assert!(parse_request(del).is_none());

        // 空 path → None。
        let nopath = b"GET  HTTP/1.1\r\n\r\n";
        assert!(parse_request(nopath).is_none());
    }

    /// command_id 字符串映射: 合法 UUID 直用 (确定性) / 非法 v5 派生 (同串同值, 幂等稳定)。
    #[test]
    fn transport_rt_01_command_id_mapping_deterministic() {
        let a = command_id_from_string("6f9d1c2e-3b4a-5c6d-7e8f-9a0b1c2d3e4f");
        let b = command_id_from_string("6f9d1c2e-3b4a-5c6d-7e8f-9a0b1c2d3e4f");
        assert_eq!(a.0, b.0, "同串同值 (幂等键稳定)");
        // 非法 UUID → v5 派生 (非 nil, 确定性)。
        let c = command_id_from_string("client-abc-123");
        let d = command_id_from_string("client-abc-123");
        assert_eq!(c.0, d.0);
        assert_ne!(c.0, Uuid::nil());
        // 合法与派生不同值。
        assert_ne!(a.0, c.0);
    }

    /// map_command_request: kind 三词表封闭 (未知→Err) / target 二选一 /
    /// SessionById UUID 非法→Err / Session intent 反序列化失败→Err / 合法全链。
    #[test]
    fn transport_rt_01_map_command_request() {
        let ok_by_id = ApiCommandRequest {
            command_id: "client-1".into(),
            kind: "stop_session".into(),
            target: crate::api_boundary::ApiCommandTarget::SessionById {
                session_id: "6f9d1c2e-3b4a-5c6d-7e8f-9a0b1c2d3e4f".into(),
            },
            requested_by: "ops".into(),
        };
        let env = map_command_request(&ok_by_id).expect("合法 SessionById 应成功");
        assert_eq!(env.kind, CommandKind::StopSession);

        let ok_session = ApiCommandRequest {
            command_id: "client-2".into(),
            kind: "start_session".into(),
            target: crate::api_boundary::ApiCommandTarget::Session {
                intent: serde_json::json!({"version": "1.0", "devices": []}),
            },
            requested_by: "ops".into(),
        };
        let env2 = map_command_request(&ok_session).expect("合法 Session 应成功");
        assert_eq!(env2.kind, CommandKind::StartSession);

        // 未知 kind → Err (400)。
        let bad_kind = ApiCommandRequest {
            command_id: "x".into(),
            kind: "delete_session".into(),
            target: crate::api_boundary::ApiCommandTarget::SessionById {
                session_id: "6f9d1c2e-3b4a-5c6d-7e8f-9a0b1c2d3e4f".into(),
            },
            requested_by: "ops".into(),
        };
        assert!(map_command_request(&bad_kind).is_err());

        // SessionById UUID 非法 → Err。
        let bad_sid = ApiCommandRequest {
            command_id: "x".into(),
            kind: "stop_session".into(),
            target: crate::api_boundary::ApiCommandTarget::SessionById {
                session_id: "not-a-uuid".into(),
            },
            requested_by: "ops".into(),
        };
        assert!(map_command_request(&bad_sid).is_err());
    }

    /// map_dispatch: 四出口封闭 (Executed/Replayed/Conflict/Rejected) +
    /// classification 映射 + 不暴露 failed (serde 断言)。
    #[test]
    fn transport_rt_01_map_dispatch_four_exits() {
        let outcome = crate::command::CommandOutcome {
            command_id: crate::command::CommandId(Uuid::nil()),
            kind: CommandKind::StopSession,
            status: crate::command::CommandStatus::Executed,
            detail: None,
            classification: None,
        };
        let resp = map_dispatch(&IdempotentDispatch::Executed(outcome.clone()));
        assert_eq!(resp.status, crate::api_boundary::ApiCommandStatus::Executed);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("\"failed\""), "不暴露 failed");

        let resp_replayed = map_dispatch(&IdempotentDispatch::Replayed(outcome.clone()));
        assert_eq!(
            resp_replayed.status,
            crate::api_boundary::ApiCommandStatus::Replayed
        );

        let resp_conflict = map_dispatch(&IdempotentDispatch::Conflict {
            command_id: crate::command::CommandId(Uuid::nil()),
            expected: crate::idempotency::CommandFingerprint("a".into()),
            actual: crate::idempotency::CommandFingerprint("b".into()),
        });
        assert_eq!(
            resp_conflict.status,
            crate::api_boundary::ApiCommandStatus::Conflict
        );
        assert_eq!(
            resp_conflict.classification,
            Some(crate::api_boundary::ApiErrorClass::Conflict)
        );

        let resp_rejected = map_dispatch(&IdempotentDispatch::Rejected(
            crate::command::CommandRejection {
                code: "empty_requester".into(),
                detail: "requested_by 不得为空".into(),
            },
        ));
        assert_eq!(
            resp_rejected.status,
            crate::api_boundary::ApiCommandStatus::Rejected
        );
    }

    /// route 路由表: 404 未知 / 405 方法错 / 503 无 mgr (query/idem None) /
    /// 200 /health 形状不变 / 200 boundary。
    #[test]
    fn transport_rt_01_route_table() {
        let ctx = TransportContext {
            events: Arc::new(crate::events::RuntimeEventLog::new()),
            agent_state: Arc::new(Mutex::new(crate::health::AgentState::Ready)),
            device_count: 3,
            query: None,
            idem: None,
        };
        // 404 未知。
        let (code, body) = route("GET", "/nope", &[], &ctx);
        assert_eq!(code, 404);
        assert!(body.contains("not_found"));

        // 405 方法错 (POST /health)。
        let (code, _) = route("POST", "/health", &[], &ctx);
        assert_eq!(code, 405);

        // 503 无 mgr (runtime/commands)。
        let (code, body) = route("GET", "/api/v1/runtime", &[], &ctx);
        assert_eq!(code, 503);
        assert!(body.contains("service_unavailable"));
        let (code, _) = route("POST", "/api/v1/commands", b"{}", &ctx);
        assert_eq!(code, 503);

        // 200 /health 形状不变 (回归锚点: 五字段齐全)。
        let (code, body) = route("GET", "/health", &[], &ctx);
        assert_eq!(code, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(v.get("state").is_some());
        assert!(v.get("devices").is_some());
        assert_eq!(v["devices"], 3);
        assert!(v.get("active_pipelines").is_some());
        assert!(v.get("dropped_bus_events").is_some());
        assert!(v.get("clock_lost_events").is_some());

        // 200 boundary (全路径, 无 mgr 依赖)。
        let (code, body) = route("GET", "/api/v1/idempotency/boundary", &[], &ctx);
        assert_eq!(code, 200);
        assert!(body.contains("process_local"));
    }

    /// TRANSPORT-RT-01 Simulation 层: serve_connection 端到端 (真实 loopback TCP) —
    /// bind 127.0.0.1:0 (临时端口) → accept 单连接 → 客户端发 GET /health →
    /// 断言 200 + Content-Type + 响应体五字段齐全 (/health 回归锚点, 无硬件依赖)。
    #[test]
    fn transport_rt_01_loopback_http() {
        use std::io::{Read, Write};
        // 临时端口 (127.0.0.1:0 → 内核分配), 避免端口冲突。
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local_addr");
        let ctx = TransportContext {
            events: Arc::new(crate::events::RuntimeEventLog::new()),
            agent_state: Arc::new(Mutex::new(crate::health::AgentState::Ready)),
            device_count: 42,
            query: None,
            idem: None,
        };
        // 服务端: accept 单连接 → serve_connection (处理完关闭, 无持久连接)。
        let server = std::thread::spawn(move || {
            let (stream, _peer) = listener.accept().expect("accept");
            serve_connection(stream, &ctx);
        });
        // 客户端: 连接 → 发 GET /health → 读至 EOF (服务端写完关闭)。
        let mut client = std::net::TcpStream::connect(addr).expect("connect");
        client
            .write_all(b"GET /health HTTP/1.1\r\nHost: loopback-test\r\n\r\n")
            .expect("write request");
        let mut resp = String::new();
        client.read_to_string(&mut resp).expect("read response");
        // 状态行 200 + JSON Content-Type。
        assert!(resp.starts_with("HTTP/1.1 200 OK"), "应 200: {resp}");
        assert!(resp.contains("Content-Type: application/json"));
        // 响应体 (CRLFCRLF 之后) 五字段齐全 (回归锚点)。
        let body = resp.split("\r\n\r\n").nth(1).expect("response body 存在");
        let v: serde_json::Value = serde_json::from_str(body).expect("body 可解析 JSON");
        assert!(v.get("state").is_some());
        assert_eq!(v["devices"], 42);
        assert!(v.get("active_pipelines").is_some());
        assert!(v.get("dropped_bus_events").is_some());
        assert!(v.get("clock_lost_events").is_some());
        let _ = server.join();
    }
}
