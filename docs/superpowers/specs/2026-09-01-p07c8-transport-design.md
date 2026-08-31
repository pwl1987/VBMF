---
comet_change: p07c8-transport
role: technical-design
canonical_spec: openspec
archived-with: 2026-09-01-p07c8-transport
status: final
---

# Design Doc — p07c8-transport（Phase 0.7C-8: Transport 实现 — API Boundary Model → wire）

> open design.md §1-§5 实现级细化。锚点：0.7C-7 终审裁定 + Contract Probe `docs/superpowers/reports/2026-09-01-p07c8-transport-contract-probe.md`。
> **本 change 仅做模型到 wire 的序列化边界（五端点）；std-only 零新依赖；不改 api_boundary/command/idempotency/runtime_query/event_projection/rpc 任何契约**。

## 1. transport.rs 实现契约

```rust
pub struct TransportContext {
    pub events: Arc<crate::events::RuntimeEventLog>,
    pub agent_state: Arc<std::sync::Mutex<crate::health::AgentState>>,
    pub device_count: usize,
    pub query: Option<Arc<crate::runtime_query::RuntimeQuery>>,
    pub idem: Option<Arc<crate::idempotency::CommandIdempotency>>,
}

pub const MAX_REQUEST_BYTES: usize = 1_048_576; // 1 MiB 限长

pub fn parse_request(buf: &[u8]) -> Option<(String /*method*/, String /*path*/, Vec<u8> /*body*/)>;
// 请求行 "METHOD SP path SP HTTP/1.1" + 头部仅取 Content-Length + body 精确长度;
// buf 无 CRLFCRLF 分隔 / 方法非 GET|POST / body 超 Content-Length / Content-Length 非数字 / 超 MAX → None

pub const COMMAND_ID_NAMESPACE: Uuid = Uuid::from_u128(0x62b79f8c1a2e4c3d9f0b5d6e7a8b9c0d);

pub fn map_command_request(req: &ApiCommandRequest) -> Result<CommandEnvelope, String>;
// command_id: Uuid::parse_str 成功→直用; 失败→Uuid::new_v5(&COMMAND_ID_NAMESPACE, bytes)
// kind: "start_session"→StartSession / "stop_session"→StopSession / "release_session"→ReleaseSession / 其他→Err
// target: SessionById{session_id}→Uuid::parse_str→SessionId; Session{intent}→from_value::<GraphRuntimeIntent>
// issued_at_ms: 0 (信封形状字段; 幂等指纹不含 issued_at_ms — D9-A)

pub fn map_dispatch(d: &IdempotentDispatch) -> ApiCommandResponse;
// Executed(o)→{status:Executed, classification:o.classification.map(ApiErrorClass::from)}
// Replayed(o)→{status:Replayed, ...}
// Conflict{..}→{status:Conflict, classification:Some(Conflict)}
// Rejected(r)→{status:Rejected, classification:Some(Rejected), detail:Some(r.detail)}

pub fn route(method: &str, path: &str, body: &[u8], ctx: &TransportContext) -> (u16, String);
pub fn serve_connection(mut stream: TcpStream, ctx: &TransportContext);
```

**响应写**：`HTTP/1.1 {code} {reason}\r\nContent-Type: application/json\r\nContent-Length: N\r\nConnection: close\r\n\r\n{body}`。reason: 200 OK / 400 Bad Request / 404 Not Found / 405 Method Not Allowed / 503 Service Unavailable。

**/health body（与 main.rs 原逻辑逐字段一致，回归锚点）**：
`{"state": st, "devices": device_count, "active_pipelines": HEALTH_ARCS.len(), "dropped_bus_events": dropped_bus_events(), "clock_lost_events": clock_lost_events()}`

## 2. main.rs 接线（最小 diff）

1. `let mut api_mgr: Option<Arc<crate::session::SessionManager>> = None;`（`if auto_start` 前）
2. cfg(gstreamer-backend) 块：`let mgr: Arc<SessionManager> = Arc::new(SessionManager::new(...));` + 块首 `api_mgr = Some(mgr.clone());`
3. health 线程：构造 `TransportContext`（query/idem 由 api_mgr map）+ `serve_connection` 循环替换原内联响应。

## 3. 测试矩阵

- Unit（feature 无关）：parse_request 形状 / route 表（404/405/503/200+health 形状）/ map_command_request 封闭词表+v5 确定性 / map_dispatch 四出口。
- Simulation（feature=mock）：`transport_rt_01_loopback_http`——`TcpListener::bind("127.0.0.1:0")` 取随机端口 + 线程 serve + 客户端 TcpStream 打真实 HTTP 字节断言响应（runtime 200 / commands POST 200 / health 200 / 404 / 405 / 503 无 mgr 用 `TransportContext{query:None,idem:None,...}`）。
- Hardware：gate 段 TRANSPORT-RT-01（真机 curl 打端点 + 全门禁回归）。

## 4. 红线

- std-only：Cargo.toml 零新依赖。
- 零触碰：api_boundary/command/idempotency/runtime_query/event_projection/rpc/preflight/resource/lease/session/supervisor 零 diff。
- 0.7C-3 不可执行性：map_command_request 零执行字段。
- 0.7C-5 三平面分离：map_dispatch status+classification 独立。
- 0.7C-7 NOTE：snapshot_kind 守门（projection 端点经 ApiProjectionResponse）/ API 模型独立 / 不暴露 serde tag。