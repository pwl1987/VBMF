# Comet Design Handoff

- Change: p07c8-transport
- Phase: design
- Mode: compact
- Context hash: fe26ea7a48813e5c540b6ab609df9f5584829088aadad1e1c918ea25f493d26c

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07c8-transport/proposal.md

- Source: docs/openspec/changes/p07c8-transport/proposal.md
- Lines: 1-38
- SHA256: b5df3a1d414801c95a39c210bcbc96361f1155440eee0048c3a33298f2a42535

```md
# Change: Phase 0.7C-8 — p07c8-transport（Transport 实现：API Boundary Model → wire 的序列化边界）

## Why

0.7C-7 Merge Gate PASS（master=`70d3ed1`）。0.7C-7 终审裁定 + 本阶段 Contract Probe（`docs/superpowers/reports/2026-09-01-p07c8-transport-contract-probe.md` 只读四问）：

- **顺序**：API Boundary Model（0.7C-7 ✅）→ **Transport 实现**（本 change）——只做模型到 wire 的序列化边界，不发明新端点、不改内部契约。
- **std-only 纪律**（0.7A）：禁 axum/hyper/tower/grpc。既有唯一 wire 面 = main.rs health（std TcpListener 单线程无解析）。
- **关键约束（probe Q3/Q4）**：`mgr` 仅诊断路径+gstreamer-backend 存在；生产路径无 mgr → runtime/commands 端点返回 503（契约诚实，能力边界暴露非隐藏）；/health、events/projection、idempotency/boundary 全路径可用。
- **PR 规模判定**：生产代码 diff 按文件白名单审核（transport.rs 新 + main.rs 接线），不按 PR 总行数。

## What Changes

- **`src/transport.rs`（新）**——std-only HTTP 序列化边界（零新依赖，仅 std + serde_json + uuid）：
  - **`TransportContext`**：`{ events: Arc<RuntimeEventLog>, agent_state: Arc<Mutex<AgentState>>, device_count: usize, query: Option<Arc<RuntimeQuery>>, idem: Option<Arc<CommandIdempotency>> }`——Query/Command 持 Option（生产路径 None→503），Event/agent_state/device_count 全路径。
  - **HTTP 解析（纯函数）**：`parse_request(buf) -> Option<(method, path, body)>`——请求行 `METHOD SP path SP HTTP/1.1` + 头部（仅取 Content-Length）+ body（**限长 1 MiB** 防内存放大）；无持久连接（每连接一请求后关闭，与 /health 既有模型一致）。
  - **路由（五端点，不发明）**：`GET /health`（行为不变，回归锚点）/ `GET /api/v1/runtime` / `POST /api/v1/commands` / `GET /api/v1/events/projection` / `GET /api/v1/idempotency/boundary`；未知 404 / 方法错 405 / 无 mgr 503（JSON body）。
  - **映射（纯函数，Unit 可测）**：
    - `map_command_request(&ApiCommandRequest) -> Result<CommandEnvelope, String>`——command_id 字符串：合法 UUID→`Uuid::parse_str`，否则→`Uuid::new_v5(&固定命名空间, bytes)` 确定性派生；kind 三词表封闭（start/stop/release_session，未知→Err=400）；`SessionById{session_id}`→UUID parse（失败 Err）；`Session{intent: Value}`→`from_value::<GraphRuntimeIntent>`（失败 Err）。**形状错=400（未触 Runtime）；形状对但语义拒绝=200+ApiCommandStatus::Rejected**（0.7C-3 不可执行性红线）。
    - `map_dispatch(&IdempotentDispatch) -> ApiCommandResponse`——Executed→Executed / Replayed→Replayed / Conflict→Conflict / Rejected→Rejected（**4 态不暴露 Failed**，NOTE-2；outcome Failed 经 classification=Some 传达）。
  - **`serve_connection(stream, ctx)`**：读请求→路由→写 `HTTP/1.1 {code} {reason}\r\nContent-Type: application/json\r\nContent-Length: N\r\n\r\n{body}`；单 accept 循环既有并发模型（不偷升级线程池/async）。
- **`main.rs` 接线**：
  - 诊断路径：`let api_mgr: Option<Arc<SessionManager>> = None;` 提升到 `if auto_start` 外；cfg(gstreamer-backend) 块内 `let mgr = Arc::new(SessionManager::new(...))`（**Arc 包装**——原 mgr 被 tick 线程 move，共享须 Arc；既有 `mgr.xxx()` 经 Arc 透传零语义变化）+ `api_mgr = Some(mgr.clone())`。
  - health 线程 spawn 点：构造 `TransportContext`（query/idem 由 api_mgr map 出 Option）+ `for s in listener.incoming().flatten() { transport::serve_connection(s, &ctx) }` 替换原"无解析一律回 health"（/health 响应体不变）。
- **门禁 TRANSPORT-RT-01（三层）**：
  - **Unit**（纯函数级，feature 无关）：parse_request 合法/畸形/超长按/无 Content-Length；路由表 404/405/503 语义；map_command_request 封闭词表（kind 三值+未知、UUID 合法/非法→v5 派生、target 二选一、intent 反序列化失败）；map_dispatch 四出口映射；/health body 形状不变。
  - **Simulation**（feature=mock）：std TcpListener 起**真实 loopback 端口**打真实 HTTP 请求断言响应（runtime 200 含 devices / commands POST 200 含 status / 404 / 405 / 无 mgr 503）——std-only 无依赖。
  - **Hardware**（真机）：gate 段以 curl 打真机端点，断言 /api/v1/runtime 200 + /api/v1/commands POST 200 + /health 回归 + 全门禁回归。
- **CI**：测试并入现有矩阵（mock 组含 loopback 集成测试）。

## Capabilities

（`skip_specs: true`——SoT 为 0.7C-7 终审裁定 + Transport Contract Probe + PHASE_IMPLEMENTATION_MAP §3。）

## Impact

- 编译：五套 feature 不回退；transport.rs 零 vendor/零 transport 依赖（仅 std + serde_json + uuid）。
- 受影响：新 `transport.rs`；`main.rs`（mod + 诊断路径 Arc 化 mgr + health 线程接线）；Phase Map（0.7C-8 行）；债表（无新增，Transport 收口 0.7C 顺序）。
- **明确不做**：新 transport 依赖（axum/hyper/tower/tonic）；持久连接/keep-alive、TLS、认证/RBAC（Fastify 反代层职责）；并发模型升级（线程池/async）；修改 rpc.rs 冻结接口；修改 Command/Query/Event 内部契约；新端点发明（仅五端点）；EventProjection 暴露为权威态（snapshot_kind 守门已冻结）。
```

## docs/openspec/changes/p07c8-transport/design.md

- Source: docs/openspec/changes/p07c8-transport/design.md
- Lines: 1-114
- SHA256: 32617b95d81e18c138dd39932917a14aefdcdc60e0cc7d5a9aaa61cb70867e62

[TRUNCATED]

```md
# Design: Phase 0.7C-8 — p07c8-transport（Transport 实现）

## 0. 终审裁定 + probe 引用落点

| 裁定/probe 结论 | 设计落点 |
|---|---|
| std-only 纪律（禁 axum/hyper/tower/grpc） | transport.rs 仅 std + serde_json + uuid（Cargo 零新依赖） |
| API Boundary Model 已冻结（0.7C-7） | 复用 `api_boundary.rs` 全部类型 + `to_api_*` 纯函数；**零改动** api_boundary.rs |
| 扩展既有 health listener，/health 行为不变 | 同 `cfg.health_bind` 单端口单 accept 循环；/health body 形状回归锚点 |
| mgr 仅诊断路径+gst → 生产 503 契约诚实 | `TransportContext.query/idem: Option<...>`；None→503 |
| 诊断路径 Arc 化 mgr | `let mgr = Arc::new(SessionManager::new(...))` + `api_mgr: Option<Arc<SessionManager>>` 提升 |
| command_id 字符串→UUID | 合法 parse；非法 `Uuid::new_v5(&NS, bytes)` 确定性派生 |
| 形状错=400 vs 语义拒绝=200+Rejected | map_command_request Err→400；dispatch Rejected→200+ApiCommandStatus::Rejected |
| 四出口不暴露 Failed | map_dispatch 穷举四出口 |
| 无持久连接/单请求/1MiB 限长 | parse_request + serve_connection |

## 1. transport.rs 结构

```rust
pub struct TransportContext {
    pub events: Arc<crate::events::RuntimeEventLog>,
    pub agent_state: Arc<std::sync::Mutex<crate::health::AgentState>>,
    pub device_count: usize,
    pub query: Option<Arc<crate::runtime_query::RuntimeQuery>>,
    pub idem: Option<Arc<crate::idempotency::CommandIdempotency>>,
}

/// 请求行解析结果。
pub struct ParsedRequest { pub method: String, pub path: String, pub body: Vec<u8> }

/// 纯函数: 从原始字节解析 HTTP 请求 (请求行 + Content-Length + body 限长 1 MiB)。
/// 畸形/超限 → None (serve_connection 回 400)。
pub fn parse_request(buf: &[u8]) -> Option<ParsedRequest>;

/// 固定命名空间: 非 UUID 字符串 command_id → 确定性 v5 派生 (幂等键稳定)。
pub const COMMAND_ID_NAMESPACE: uuid::Uuid = uuid::Uuid::from_u128(0x62b79f8c_1a2e_4c3d_9f0b_5d6e7a8b9c0d);

/// 纯函数: ApiCommandRequest → CommandEnvelope (形状校验; Err=400 detail)。
pub fn map_command_request(req: &crate::api_boundary::ApiCommandRequest) -> Result<crate::command::CommandEnvelope, String>;

/// 纯函数: IdempotentDispatch → ApiCommandResponse (四出口封闭, 不暴露 Failed)。
pub fn map_dispatch(d: &crate::idempotency::IdempotentDispatch) -> crate::api_boundary::ApiCommandResponse;

/// 路由 + 响应 (纯逻辑, 可注入 ctx 测试)。返回 (status, json_body)。
pub fn route(method: &str, path: &str, body: &[u8], ctx: &TransportContext) -> (u16, String);

/// 连接处理: 读→解析→路由→写 (无持久连接, 处理完关闭)。
pub fn serve_connection(mut stream: std::net::TcpStream, ctx: &TransportContext);
```

**路由表（route() 内）**：
| method+path | 处理 | 状态 |
|---|---|---|
| GET /health | 既有 body {state,devices,active_pipelines,dropped_bus_events,clock_lost_events} | 200 |
| GET /api/v1/runtime | `ctx.query`→`get_runtime_state()`→`to_api_query_snapshot`；None→503 | 200/503 |
| POST /api/v1/commands | body→`ApiCommandRequest`（serde）→`map_command_request`→`ctx.idem.dispatch`→`map_dispatch`；解析/映射 Err→400；None→503 | 200/400/503 |
| GET /api/v1/events/projection | `events.drain()`→`project`→`ApiProjectionResponse` | 200 |
| GET /api/v1/idempotency/boundary | `default_idempotency_boundary()` | 200 |
| 已知 path 错误 method | 405 {error} | 405 |
| 未知 path | 404 {error} | 404 |
| 解析失败 | 400 {error} | 400 |

**/health body 不变性**：`route` 的 /health 分支与 main.rs 原逻辑逐字段一致（state 经 `agent_state` 序列化、active_pipelines 经 `pipeline_events::HEALTH_ARCS`、dropped/clock 经 `pipeline::` 计数）——回归锚点。

## 2. main.rs 接线（最小 diff）

```rust
// if auto_start 外:
let api_mgr: Option<Arc<crate::session::SessionManager>> = None;
if auto_start {
    #[cfg(feature = "gstreamer-backend")]
    {
        let mgr = Arc::new(crate::session::SessionManager::new(...));  // Arc 包装 (原 move 进 tick 线程, 共享须 Arc)
        let api_mgr_for_ctx = Some(mgr.clone());
        ... // 既有 create/start/watchdog/tick 零语义变化 (Arc 透传)
        // 块末: *api_mgr_slot = api_mgr_for_ctx;  — 用 FnOnce 闭包或 Cell 传递 (见实现)
    }
}
// health 线程:
let ctx = crate::transport::TransportContext {

```

Full source: docs/openspec/changes/p07c8-transport/design.md

## docs/openspec/changes/p07c8-transport/tasks.md

- Source: docs/openspec/changes/p07c8-transport/tasks.md
- Lines: 1-51
- SHA256: 1ec33ee1e00cc4869bc4b53b6d909b8275422e8a6ebfada8e2d7fe353e2b87de

```md
# Tasks: Phase 0.7C-8 — p07c8-transport

四栏纪律：`Contract: 已有(引用冻结文档节号) / Implementation / Verification / Gate`。

## 1. HTTP 解析 + 路由（design.md §1）
- [ ] transport.rs：`ParsedRequest` + `parse_request`（请求行/Content-Length/1MiB 限长）+ `TransportContext` + `route()` 五端点路由表（404/405/503/200）
      Contract: probe Q1（扩展既有 listener）+ Q4-7（五端点不发明）+ /health 行为不变
      Implementation: transport.rs
      Verification: `transport_rt_01_parse_request_shapes` + `transport_rt_01_route_table`
      Gate: TRANSPORT-RT-01 Unit 层
- [ ] `serve_connection`（读→解析→路由→写, 无持久连接）
      Contract: probe Q1（单 accept 循环既有并发模型）
      Implementation: transport.rs
      Verification: `transport_rt_01_loopback_http`（Simulation）
      Gate: TRANSPORT-RT-01 Simulation 层

## 2. 三平面映射（design.md §1）
- [ ] `map_command_request`（command_id UUID 直用/v5 派生 + kind 三词表 + target 二选一 + intent 反序列化）
      Contract: 0.7C-7 design §1.2（ApiCommandRequest 模型）+ probe Q4-3/Q4-4
      Implementation: transport.rs
      Verification: `transport_rt_01_map_command_request`
      Gate: TRANSPORT-RT-01 Unit 层
- [ ] `map_dispatch`（四出口封闭, 不暴露 Failed, classification 映射）
      Contract: 0.7C-7 NOTE-2（Status/Error 独立性）+ 0.7C-5 三平面分离
      Implementation: transport.rs
      Verification: `transport_rt_01_map_dispatch_four_exits`
      Gate: TRANSPORT-RT-01 Unit 层

## 3. main.rs 接线（design.md §2）
- [ ] 诊断路径 Arc 化 mgr + `api_mgr: Option<Arc<SessionManager>>` 提升 + TransportContext 构造 + health 线程接线
      Contract: probe Q3/Q4-1/Q4-2（生产 503 契约诚实 / Arc 透传零语义变化）
      Implementation: main.rs
      Verification: 五套 feature 编译不回退 + 盒上矩阵
      Gate: TRANSPORT-RT-01 Hardware 层 + PR required checks

## 4. 真机与回归
- [ ] gate 段 TRANSPORT-RT-01（真机 curl 打 /api/v1/runtime + /api/v1/commands + /health 回归 + 全门禁回归）
      Contract: 0.7 全阶段最高红线（Observation≠Configuration）
      Implementation: main.rs gate 段
      Verification: 盒上 VBMF_SESSION_LIFECYCLE=1 真机跑
      Gate: TRANSPORT-RT-01 Hardware 层 + SESSION/RESOURCE/IDEMPOTENCY/ERROR-MODEL/EVENT-PROJECTION/EXTERNAL-API-RT-01 回归
- [ ] 五套 feature 编译不回退 + 盒上全矩阵（mock 组含 loopback 集成测试）
      Contract: CI 七 checks 口径 + 0.7A std-only 纪律（Cargo 零新依赖）
      Verification: p07_verify.sh 全绿
      Gate: PR required checks

## 5. 文档与收尾
- [ ] Phase Map 0.7C-8 行 COMPLETE；0.7C §3 Transport 完成（**0.7C 全段收口**）；债表对账
      Contract: PHASE_IMPLEMENTATION_MAP=唯一 SoT
      Verification: 文档对账
      Gate: verify
- [ ] verify（0 CRIT/0 IMP 目标）→ archive → PR → merge → tag phase-0.7C8-transport → 删分支
```
