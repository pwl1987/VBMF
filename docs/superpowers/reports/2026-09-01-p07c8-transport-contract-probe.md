# 0.7C-8 Transport 实现 Contract Probe（只读调查）

- 日期：2026-09-01 · 基线：master=`70d3ed1`（0.7C-7 baseline）
- 性质：只读探针（零代码改动）——回答 Transport 实现（API Boundary Model → wire 的序列化边界）的开工前置问题。
- 边界（终审 0.7C-6/0.7C-7 已冻结）：std-only 纪律（禁 axum/hyper/tower/grpc）；API Boundary Model 已冻结（api_boundary.rs）；本 change 只做模型到 wire 的序列化边界，不改 Command/Query/Event 内部契约。

## Q1 既有 transport 基础设施

**main.rs:1267-1297（health endpoint）**——唯一既有 wire 面：
- `std::thread::spawn` + `TcpListener::bind(cfg.health_bind)`（默认 `127.0.0.1:8080` 仅回环；生产由 `MEDIA_AGENT_HEALTH_BIND` 覆盖/经 Fastify 反代+认证）
- **单线程 accept 循环**（`for mut s in listener.incoming().flatten()`），**无请求解析**（任何连接一律回 health JSON）
- 响应格式：`HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: N\r\n\r\n{body}`
- health body：`{state, devices, active_pipelines, dropped_bus_events, clock_lost_events}`（与 Supervisor 状态机对齐，Gate 2.6）

**结论**：API 路由应**扩展同一 listener**（管理面单端口，不新开端口/线程模型），把"无解析一律回 health"升级为"解析请求行→路由"。/health 响应体行为不变（回归锚点）。

## Q2 rpc.rs 与 External API 的边界

`rpc.rs` 是 **Rust→Node/Fastify Control Plane 的冻结接口**（`AgentRequest/AgentResponse`，"No transport yet"；SoT §14：Node=Control, Rust=Hardware；Rust 禁实现 API gateway/auth/RBAC/config UI）。**与 External API（API 消费者面）是两个边界，本 change 不触碰 rpc.rs 冻结接口**。External API 的认证/身份属 Fastify 反代层（与 health 注释一致：生产经反向代理+认证），agent 本体只做 opaque `requested_by` 标签。

## Q3 三平面接线点（全部已冻结，零改动）

| 平面 | 链 | 可用性 |
|---|---|---|
| Query | `RuntimeQuery::new(mgr).get_runtime_state()` → `to_api_query_snapshot()` | mgr **仅诊断路径+gstreamer-backend**（main.rs:1196，在 `if auto_start`+cfg 块内）；生产路径无 mgr |
| Command | `ApiCommandRequest` → `map_command_request()` → `CommandEnvelope` → `CommandIdempotency::new(mgr).dispatch()` → `map_dispatch()` → `ApiCommandResponse` | 同上（需 mgr） |
| Event | `event_log.drain()`（**全局 line 486, 全路径可用**）→ `project()` → `ApiProjectionResponse` | 全路径 |
| Idempotency 契约 | `default_idempotency_boundary()`（纯静态） | 全路径 |

**关键约束**：`mgr` 在 health 线程 spawn 点（line 1267）**不在作用域**（诊断路径 mgr 在 `if auto_start`+`#[cfg(gstreamer-backend)]` 块内创建，且被 line 1234 tick 线程 `move` 走）。

## Q4 设计裁决输入

1. **TransportContext 持 Option**：`{ events: Arc<RuntimeEventLog>, agent_state, device_count, query: Option<Arc<RuntimeQuery>>, idem: Option<Arc<CommandIdempotency>> }`——生产路径（无 mgr）对 runtime/commands 端点返回 **503**（契约诚实：能力边界暴露非隐藏，与 Idempotency 边界三选项同风格）；/health、/api/v1/events/projection、/api/v1/idempotency/boundary 全路径可用。
2. **诊断路径接线**：`let api_mgr: Option<Arc<SessionManager>> = None;` 提升到 `if auto_start` 外；cfg 块内 `let mgr = Arc::new(SessionManager::new(...))`（Arc 包装——原 mgr 会被 tick 线程 move，共享须 Arc 化；既有 `mgr.xxx()` 调用经 Arc 透传零语义变化）；`api_mgr = Some(mgr.clone())`。
3. **command_id 字符串→CommandId(Uuid) 映射**：合法 UUID 字符串→`Uuid::parse_str` 直用；否则→`Uuid::new_v5(&固定命名空间, bytes)` 确定性派生（uuid crate 内置 v5，零新依赖，幂等键语义稳定）。
4. **kind/target 映射**：`"start_session"/"stop_session"/"release_session"`→CommandKind 三词表（封闭）；`SessionById{session_id}`→UUID parse（失败 400）；`Session{intent: Value}`→`serde_json::from_value::<GraphRuntimeIntent>()`（失败 400）。**形状错=400（malformed request，未触 Runtime）；形状对但语义拒绝=200+ApiCommandStatus::Rejected**（Command 平面语义，0.7C-3 不可执行性红线）。
5. **IdempotentDispatch→ApiCommandStatus**：Executed→Executed（outcome Failed 时 classification=Some）/ Replayed→Replayed / Conflict→Conflict / Rejected→Rejected（4 态不暴露 Failed，NOTE-2 命名解耦在 map_dispatch 落地）。
6. **HTTP 解析**：读请求行 `METHOD SP path SP HTTP/1.1` + 头部（仅取 Content-Length）+ body（限长，如 1 MiB 防 DoS 式内存放大）；无持久连接（每连接一请求后关闭，与 /health 既有模型一致——并发模型不偷升级）。
7. **路由表**：`GET /health`（行为不变）/ `GET /api/v1/runtime` / `POST /api/v1/commands` / `GET /api/v1/events/projection` / `GET /api/v1/idempotency/boundary`；未知 404、方法错 405（JSON body）。
8. **门禁 TRANSPORT-RT-01 三层**：Unit（parse_request/路由表/map_command_request 封闭词表/map_dispatch 四出口/404-405/503 语义，纯函数级）；Simulation（mock feature 下以 std TcpListener 起真实 loopback 端口打真实 HTTP 请求断言响应——std-only 无依赖）；Hardware（真机 gate：curl/wget 打真机端点，EXTERNAL-API 端点真实响应 + 回归全门禁）。

## 明确不做

- 新 transport 依赖（axum/hyper/tower/tonic 禁令延续）
- 持久连接/keep-alive、TLS、认证/RBAC（Fastify 反代层职责）
- 并发模型升级（线程池/async——单 accept 循环既有模型）
- 修改 rpc.rs 冻结接口 / Command/Query/Event 内部契约
- 新端点发明（仅 Q4-7 路由表五端点）
- EventProjection 暴露为权威态（snapshot_kind 守门已在 API 模型层冻结）