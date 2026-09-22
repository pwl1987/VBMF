# RUNTIME-CONTROL-ENTRY-01 — Planning / Authority Reconciliation（2026-09-20）

> **Packet**: `RUNTIME-CONTROL-ENTRY-01`（PLAN / RECONCILIATION ONLY·用户 2026-09-20 裁定）
> **目标**: 把"standalone service 能运行，但只能靠 gate 演示媒体"推进到"真实 standalone `media-agent` 可被 canonical Runtime Control 创建 Session、启动、观察 actual state、停止、恢复，并保持 Runtime 唯一 truth"。
> ** Authority 链**: 用户 2026-09-20 指令 > frozen `EXTERNAL_API_CONTRACT.md`（三平面 `/api/v1/*` vs `/internal/v1/*`）> frozen `TECHNOLOGY_STACK_AND_RUNTIME_OWNERSHIP.md`（JSON-RPC / F1–F12 / GSTR-02）> frozen `EVENT_CONTRACT.md`（RuntimeEvent ≠ External Event / Projection 属 Control Plane）> Phase 0.7C 冻结契约族（command/idempotency/api_boundary/transport 词表快照测试）> live code > `.project/STATE.md`。
> **Stop-condition 检查（§2 结论）**: 本 reconciliation **不需要修改任何 frozen Architecture/Contract 文档** → 按用户指令，设计冻结后直接进入首个 bounded implementation sub-packet（RCE-01A），不再等普通确认。

---

## 1. Live-tree audit（只读事实，file:line 锚定）

| # | 事实 | 锚点 |
|---|---|---|
| A1 | Rust HTTP 面五端点冻结：`GET /health`、`GET /api/v1/runtime`、`POST /api/v1/commands`、`GET /api/v1/events/projection`、`GET /api/v1/idempotency/boundary`；未知 404 / 方法错 405 / 无 mgr 503 | `transport.rs:5-7,228-285` |
| A2 | `TransportContext.query/idem` 为 `Option`；**生产组合根不设 `api_mgr` → query/idem=None → `/api/v1/runtime`、`/api/v1/commands` 诚实 503**；仅 gstreamer 诊断路径（P1a/P1b demo）置 `api_mgr=Some` | `bin/media-agent.rs:100,464-468,707-724` |
| A3 | Device production：mgr 常驻 tick，"等待 Control Plane 显式 StartPipeline Intent (RPC transport 待接)"（P1-3）；Network-only production 同构 | `bin/media-agent.rs:666-688` |
| A4 | `rpc.rs` = frozen SoT §14 Node↔Rust 契约**骨架**（`AgentRequest::{DiscoverDevices,AcquireLease,ReleaseLease,StartPipeline,StopPipeline,Health}`，method-tagged JSON-RPC 形态），头注明确 "not on the wire path"；verb 集为 lease/pipeline-handle 旧模型，**早于** 0.7C 会话命令面 | `rpc.rs:1-48` |
| A5 | 命令面（0.7C-3 冻结）：封闭四词表 `StartSession/StopSession/ReleaseSession/SwitchProgram`；`CommandStatus::{Accepted,Rejected,Executed,Failed}`；dispatch 三臂薄映射（`StartSession = create()+start()`）；`SwitchProgram` 经 `SwitchDispatchPlane`（未装配 → Rejected 诚实拒绝） | `command.rs:23-80,199-261` |
| A6 | 幂等面（0.7C-4 冻结）：`IdempotentDispatch` 四出口封闭（Executed/Replayed/Conflict/Rejected）+ D9-A..E（fingerprint/原子 claim/replay/并发去重）；**进程内内存表，显式不做持久化** | `idempotency.rs:1-90` |
| A7 | 查询面：`RuntimeQuery::get_runtime_state()` → `CanonicalRuntimeState` → `to_api_query_snapshot`（`ApiSession.state/phase` 字符串化）；`SessionPhase` = Requested/Provisioning/Binding/Leased/Starting/Running/Stopping/Released + ProvisioningFailed/BindingFailed/StartFailed/Degraded/Recovery/Terminated | `runtime_query.rs`·`api_boundary.rs:66-80`·`session.rs:80-95` |
| A8 | 事件面：`RuntimeEventLog.drain()` → `event_projection::project`（内存投影）；EVENT_CONTRACT：External 投递归 Fastify，Valkey 仅外部队列 | `transport.rs:267-272`·`EVENT_CONTRACT.md §1-2` |
| A9 | 管理面绑定：单 listener `_cfg.health_bind`（默认 127.0.0.1:8080 回环，`MEDIA_AGENT_HEALTH_BIND` 可覆盖），per-connection std thread；`MEDIA_AGENT_RPC_BIND` UNWIRED（SE-01C 登记） | `bin/media-agent.rs:727-742` |
| A10 | Graph Compiler 边界：`CommandTarget::Session{intent: GraphRuntimeIntent}` 零执行字段（serde 反向断言）；GSTR-02/F10 materialize 归 Agent | `command.rs:41-66` |

## 2. Authority reconciliation（五项裁定）

### R1 — `/api/v1/*` 漂移裁定（用户点名）

**裁定：现有 Rust `/api/v1/*` 五端点 = prototype/diagnostic surface，不是 Product External API 实现，也不是未来 internal Runtime Control。**

- EXTERNAL_API_CONTRACT §1 冻结三平面：Product `/api/v1/*`（业务消费方，Fastify 实现——CONTROL-PLANE packet 家族）；Diagnostics 独立命名空间；Internal Agent `/internal/v1/*`。
- 现 Rust 五端点源自 0.7C-8/R60 std-only 纪律的 prototype 传输层 + P1b 诊断 demo console（`/` 静态页轮询 `/api/v1/runtime`）。它在生产组合根 query/idem=None ⇒ 503（A2），从未作为生产可写面。
- **处置**：不迁移、不删除（五端点为 R60 冻结词表 + 回归锚点）；生产路径维持 503 契约不变；Product `/api/v1/*` 未来由 Fastify 独立实现。**禁止**把 prototype 端点直接接到 production SessionManager 后改称 Product API 或 internal control——本设计引入的内部面使用独立命名空间（R2），两个可写面不存在。

### R2 — internal Runtime Control transport 裁定

**裁定：Fastify↔Media Agent internal Runtime Control = JSON-RPC 2.0 method-tagged envelope，经 HTTP POST `/internal/v1/agent`，由 media-agent 进程承载；standalone 阶段同一面即 operator/control client 的 canonical Runtime Control 入口。**

- 满足 TECHNOLOGY_STACK 冻结行 "Media Runtime = Rust Media Agent (Host, **JSON-RPC**)"（wire 形态）与 EXTERNAL_API_CONTRACT §1 "Internal (Agent) API = `/internal/v1/*`"（命名空间）——两份 frozen 文档零修改、同时满足。
- 语义面**零新发明**：方法 = 既有冻结三平面的薄封装——`runtime.query`（RuntimeQuery 快照 + switch readback 投影合并）、`command.dispatch`（CommandIdempotency 包 command::dispatch）、`events.projection`（drain+project）。请求/响应 DTO 复用 api_boundary 的 `Api*` 独立资源模型（0.7C-7 红线：不直接暴露内部 enum）。
- std-only 纪律延续（无新依赖；per-connection thread 复用 transport 模式）。

### R3 — `rpc.rs` SoT §14 骨架 reconciliation

**裁定：`rpc.rs` 旧 verb 集（lease/pipeline-handle 模型）为历史契约记录，被 Phase 0.7C 冻结的会话命令面取代（Authority 顺序：Phase 0.6/0.7 契约族 > V0.1 SoT §14 旧表述——TECHNOLOGY_STACK §0 冲突裁决规则）。**

- `rpc.rs` 头注本就声明 "not on the wire path"；其 `StartPipeline{intent}→PipelineHandle` 模型与现行 `StartSession`（SessionId + 幂等 command_id + SessionManager 唯一 owner）冲突，以后者为准。
- 处置：`rpc.rs` 保留为历史记录并更新头注指向本 reconciliation；新 internal control 面的 JSON-RPC envelope 落独立模块（动词 = R2 方法集）。**不修改任何 frozen 文档**。

### R4 — requested→actual-state 语义裁定（用户点名）

**裁定：两平面分层已足以表达九段旅程，禁止为 Web UI 在命令面造异步中间态。冻结映射表：**

| 旅程段 | 权威平面 | 载体 |
|---|---|---|
| requested | 调用方 | JSON-RPC `command.dispatch` 请求（`command_id` + canonical fingerprint） |
| accepted / rejected（形状） | 命令面 | `IdempotentDispatch::Rejected`（未触 Runtime、未占 id） |
| rejected（能力/冲突） | 命令面 | `CommandStatus::Rejected`（如 switch_plane_unavailable）/ `Conflict`（同 id 异 payload，绝不 replay） |
| executing→acknowledged | 命令面 | `Executed`/`Failed` = dispatch 薄映射**同步完成**（create+start 等 SessionManager 公共 API 返回）；`Accepted` 为保留态（dispatch 现路径不产出——如实登记，不虚造） |
| **actual state** | **查询面（唯一 truth）** | `runtime.query` → `ApiSession.state/phase`（Requested→…→Running→…+Degraded/Recovery/…）+ `/health`（Agent 八态）+ supervisor/recovery 状态 |
| succeeded/failed | 命令面 + 查询面 | `Failed ⇒ classification=Retryable/Permanent/Unknown`（错误边界处类型态产生）；终局以查询面 SessionPhase 失败相位为准 |
| timeout / reconciled | 查询面 + 事件面 | supervisor 预算化恢复 / RecoveryMonitor / `events.projection`；无命令面异步状态 |

- **红线重申**：JSON-RPC 成功响应或 `Executed` ≠ SignalVerified ≠ Runtime healthy；A/V 验证属 acceptance gates / 信号探测面（signal.rs），不经命令状态冒充。
- 缺口如实登记：命令面无 in-flight 查询方法（Operation API `GET /commands/{id}` 是 Product 面 Fastify 职责，EXTERNAL_API_CONTRACT §2.1）；内部面以幂等 replay 语义替代（D9-D）。

### R5 — RH-IDEM-01 触发裁定

**裁定：RH-IDEM-01（durable idempotency）维持 DEFER-UNTIL-CONTROL-PLANE——解冻点 = external Fastify command entry 落地时；internal Runtime Control 面不触发解冻。**

- 分层不并存：内部面 `CommandIdempotency`（进程内）= 内部传输重试的执行守卫；未来外部 durable store = 客户端重放的 boundary-of-record（重启/replay/conflict 持久性）。同一 `command_id` 贯穿，语义域不同（进程内 vs 边界），非第二幂等 truth。
- media-agent 单进程重启 = Runtime 生命周期事件（会话 drain/重建经 SessionManager），进程内幂等表随之失效是诚实语义；跨重启重放权威留给外部边界。

### R6 — Owner map（零新 owner）

| 能力 | Owner（不变） | 本设计 |
|---|---|---|
| Session/Resource/Lease lifecycle | `SessionManager`/`ResourceRegistry`/`LeaseManager`（Rust） | internal 面只经 command::dispatch 薄映射调用 |
| Switch 执行/回读 | `SwitchDispatchPlane`/`ProgramExecutionRuntime` | `command.dispatch` 透传（未装配 ⇒ Rejected 诚实） |
| 媒体进程 spawn/监督 | Media Agent（F1–F4/F9/F11） | 零触碰 |
| Graph 编译 | Control Plane（未来 Fastify）·只产 `GraphRuntimeIntent`（F10） | internal 面接受 intent 为**不透明 canonical 载荷** |
| External Event 投递 | Fastify Projection 层（EVENT_CONTRACT §2） | internal `events.projection` 仅 drain 内存投影（诊断/桥接用） |
| Health truth | Runtime（/health） | 透传，不改 health.rs |

## 3. 冻结设计（D1–D8）

- **D1 命名空间/面隔离**：internal control = `/internal/v1/agent`（HTTP POST，JSON-RPC 2.0 envelope：`jsonrpc/"2.0"`、`method`、`params`、`id`）；prototype `/api/v1/*` 五端点与 `/health` 行为零变化；未知 method ⇒ JSON-RPC error `-32601`；形状错误 ⇒ `-32602`；面未装配（生产前）⇒ HTTP 503（诚实契约延续）。
- **D2 方法集（封闭）**：`runtime.query` / `command.dispatch` / `events.projection` / `agent.health`（四方法；词表快照测试锁定，新方法须过架构评审）。
- **D3 绑定与暴露**：~~新 listener `MEDIA_AGENT_CONTROL_BIND`，默认 `127.0.0.1:8081`；`MEDIA_AGENT_RPC_BIND` UNWIRED 登记维持~~ **→ 已由 D3-R reconciliation amendment 修正（见 §3 末），canonical = `MEDIA_AGENT_RPC_BIND` / `127.0.0.1:50051`**。原 D3 文本保留于此作为漂移记录：新 listener `MEDIA_AGENT_CONTROL_BIND`，默认 `127.0.0.1:8081`（回环；与 health_bind 同纪律——不裸露公网，生产内网暴露经反向代理+认证，用户 §二十二 P1）。写入口默认仅回环；`MEDIA_AGENT_RPC_BIND` UNWIRED 登记维持（不同键，不复活）。
- **D4 生产接线**：Device production 与 Network-only production 两组合根均构造 internal 面（mgr 常驻 tick 循环保留；P1-3 "等待显式 Intent" 由本面承接——session 的创建/启动仅经 `command.dispatch`）；`api_mgr` 仍不设（prototype `/api/v1/*` 生产 503 不变）。零媒体自动启动红线不变。
- **D5 幂等/错误模型**：`command.dispatch` 参数 = `ApiCommandRequest`（复用）；响应 = `ApiCommandResponse`（Executed/Replayed/Conflict/Rejected 四出口 + classification）；错误体走 api_boundary 既有映射，无 `ApiResponse<T>` 万能包装（0.7C-7 禁项维持）。
- **D6 事件面**：`events.projection` = drain+project（内存），语义同现行 `GET /api/v1/events/projection`；不建 External 投递（Fastify 职责）。
- **D7 关停语义**：internal listener 线程为 daemon 性质（不 join 阻塞退出）；SIGTERM drain 顺序不变（SessionManager 唯一 owner；SE-01A 契约零变化）。
- **D8 文档面**：`STANDALONE_MEDIA_AGENT_OPERATIONS.md` 增补 internal control 面一节（endpoints/绑定/退出码语义引用）；EXTERNAL_API_CONTRACT / TECHNOLOGY_STACK **零修改**（IMPLEMENTATION_STATUS 状态词按 DOCUMENT_STATUS_MODEL 属各文档自身维护策略，不在本包擅动）。

### D3-R — D3 Reconciliation Amendment（RCE-D3-BIND-RECONCILIATION·2026-09-21）

**性质**：planning→implementation 之间**未记录的命名漂移**修正（非设计变更）。RCE-01A 实施（`06bc01a`）与 RCE-01B BMD 12/12 验证实际采用下述形态；原 D3 的 `MEDIA_AGENT_CONTROL_BIND`/`8081` 从未进入任何代码。按"不回退已验证实现、不做双配置 Authority"裁定如下（经核对 EXTERNAL_API_CONTRACT / TECHNOLOGY_STACK / DEPLOYMENT_AND_DEV_RUNTIME / STANDALONE_MEDIA_AGENT_OPERATIONS / config.rs / internal_control.rs / bin/media-agent.rs / RCE focused tests / RCE-01B evidence——**零 frozen Architecture/Contract 修改，无 stop condition**）：

1. **canonical bind 配置名 = `MEDIA_AGENT_RPC_BIND`**（`config.rs` 既有字段；单一配置 Authority；`MEDIA_AGENT_CONTROL_BIND` 永不创建）。
2. **canonical default = `127.0.0.1:50051`**（standalone lane 本机 control entry；BMD 实证）。
3. **standalone lane exposure policy（显式四规则，替代模糊"localhost-only"）**：
   - R-a 默认回环 `127.0.0.1:50051` = canonical standalone 形态（operator/control client 同机）。
   - R-b **显式私有地址 bind（LAN/内网）= 允许的显式运维动作**（同网段控制面/排障；该面**无认证**——Rust 不拥有 Auth，运维须自担网络隔离：host firewall / VPN / 内网分段）。
   - R-c 通配 `0.0.0.0`/`::` 于**宿主机进程形态 = 禁止暴露公网**（现实现 = 启动告警 `rpc_bind_security_warnings`；升级 fail-closed 登记为 CONTROL-PLANE 阶段 hardening 候选，本包零行为变化）；UDS 为允许形态但 TCP-only 实现下未支持（§3.69 已登记缝隙）。
   - R-d 该面绝不直接公网；不经 Nginx 对普通客户端路由；`/internal/v1/agent` 不是 Product API。
4. **full-stack lane Fastify ↔ Media-Agent transport policy**：
   - 唯一通道 = JSON-RPC `/internal/v1/agent` over **dedicated compose private service network**（frozen Deployment SoT "React → Fastify → JSON-RPC → Media Agent"；两容器不同 netns，宿主 127.0.0.1-only 政策不适用于容器间）。
   - media-agent 容器**容器内**绑定 service-network 可达地址（容器 netns 内 `0.0.0.0:50051` 为可接受形态——仅私网可达，受 R-e 约束）。
   - R-e **host port publish 50051 = 禁止**（frozen compose 现状即无 media-agent ports 映射，维持）；**Nginx 禁止新增 `/internal/*` 公网路由**（frozen 路由表 /api /ops /admin /ws /events 无 internal，维持）。
   - R-f 跨主机（Agent 与 Fastify 不同机）：明文公网禁止；须经私有网络（VPN/专线）+ mTLS（EXTERNAL_API_CONTRACT §6 #110 既有冻结允许的加密通道形态）；具体部署 Authority 属 CONTROL-PLANE 阶段，不在本包实现。
   - Runtime 唯一 truth 不变：Fastify 只 command + observe（F1–F12 / 本 plan R6）。

## 4. Failure-first matrix（首个实现子包必须覆盖）

| # | 注入 | 期望 |
|---|---|---|
| F1 | 非 JSON / 缺 method / 未知 method / 错参数形状 | `-32602`/`-32601`/400，零 Runtime 触碰 |
| F2 | `command.dispatch` 重复同 envelope | `Replayed`（逐字节 outcome 重放，D9-D） |
| F3 | 同 `command_id` 异 payload | `Conflict`，第二 payload 绝不执行 |
| F4 | `StartSession` 空 intent / nil session_id | `Rejected`（形状层，未触 Runtime、未占 id） |
| F5 | `SwitchProgram`（无 plane 装配） | `Rejected`（switch_plane_unavailable，诚实拒绝非 Failed） |
| F6 | mgr 未构造（面早于组合根） | HTTP 503 诚实契约 |
| F7 | `runtime.query` 在 create 后未 start 前后 | phase 变迁可观察（Requested→…→Running），**actual state 唯一来源 = 查询面** |
| F8 | 命令 Executed 但 /health 未 Ready | 两平面并读：命令成功 ≠ Runtime healthy（红线锚定测试） |
| F9 | listener bind 失败 | 日志 error、进程继续（health 面不受扰——与 health bind 同语义） |
| F10 | 并发 N 线程同 envelope | 恰一次执行（D9-E） |

## 5. Security / exposure boundary

- 默认回环绑定；无认证（standalone 单机边界 = 回环；跨主机/内网暴露 = 显式 env + 反向代理 + 认证，归 CONTROL-PLANE 阶段，EXTERNAL_API_CONTRACT §6 authz 在应用层的最终归属不变）。
- 不新增：TLS 终止（Nginx/Fastify 职责）、RBAC、审计持久化（Control Plane 职责）。
- Secret 红线不变：internal 面不携带 secret/credential（endpoint redaction 语义沿 D9 既有负向套件）。

## 6. Device / Network-only parity

两生产组合根对称接线（D4）；Network-only 面上 `StartSession` 的 intent 消费 NetworkSourceBinding 授权五元组（既有 admission 不放宽）；Device 面 discovery/lease 语义零变化。诊断/gates 路径零变化。

## 7. Bounded implementation sub-packets

| 子包 | 范围 | Allowed files | Forbidden | 验收 |
|---|---|---|---|---|
| **RCE-01A** internal control surface | 新 internal control 模块（JSON-RPC envelope + 四方法）+ `rpc.rs` 头注 reconciliation + 两生产组合根接线 + `MEDIA_AGENT_CONTROL_BIND` 配置 + focused tests（F1–F10） | `services/media-agent/src/`（新模块、`rpc.rs` 注释、`bin/media-agent.rs`、`config.rs`）+ 语义页增补 | frozen 文档、prototype `/api/v1/*` 行为、SessionManager/Supervisor/Lease/Resource 语义、Fastify/DB/SDK/Web Console、新依赖 | focused + 全矩阵回归 + clippy×4 + fmt + architecture lint + remove-adapters + CI 7/7 |
| **RCE-01B** BMD 真机命令旅程 | standalone service（systemd）经 internal 面：创建 Session → 启动 → `runtime.query` 观察 phase/actual → 停止 → SIGTERM 带活会话 drain；零自动启动回归；device-2/`/opt/vbmf-dev` 边界 | BMD 操作 + evidence + 报告 | 同上 + 不做 A/V 冒充（A/V 验证仍属 gates/信号面） | 真实命令旅程 evidence 链 + STATE 收口 |

依赖：RCE-01A → RCE-01B。RCE-01A 冻结本设计后**直接开工**（用户裁定免普通确认）。

## 8. Acceptance matrix（planning 包自身）

live-tree audit ✓（§1）· Authority reconciliation 五裁定 ✓（§2）· owner map ✓（R6）· transport boundary ✓（R1/R2）· requested→actual sequence ✓（R4）· failure-first matrix ✓（§4）· security/exposure ✓（§5）· parity ✓（§6）· sub-packets ✓（§7）· STATE 更新（§3.68）。

## 9. Forbidden（planning 与实现期通用）

Fastify / PostgreSQL / BullMQ / Web Console / `vbmf-sdk` / SRS 建设；新 Runtime owner / 新 Graph compiler / 新 wire shape（方法集四词表除外——零执行语义）；修改 EXTERNAL_API_CONTRACT / TECHNOLOGY_STACK / EVENT_CONTRACT 文本；prototype `/api/v1/*` 端点语义变更；`/health` wire 变更；RH-IDEM-01 提前解冻；GraphRuntimeIntent 执行字段解禁。
