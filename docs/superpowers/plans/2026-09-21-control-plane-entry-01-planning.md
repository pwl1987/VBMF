# CONTROL-PLANE-ENTRY-01 — Planning / Authority Reconciliation（2026-09-21）

> **Packet**: `CONTROL-PLANE-ENTRY-01`（PLAN / RECONCILIATION ONLY·用户 2026-09-21 裁定；不写 Fastify 产品代码）
> **定位**: 在 RCE 全链（§3.68–§3.70：internal `/internal/v1/agent` JSON-RPC 已实机验证）与 RCE-D3（§3.71：bind/exposure 政策已显式化）之上，冻结 Fastify Control Plane 入场的全部边界决策，并拆 bounded implementation packets。
> **Authority 链**: 用户 2026-09-21 指令 > frozen `EXTERNAL_API_CONTRACT.md`（三平面/四类 API/幂等/错误模型/安全/观测）> frozen `EVENT_CONTRACT.md`（RuntimeEvent ≠ External Event；Projection 属 Control Plane）> frozen `TECHNOLOGY_STACK_AND_RUNTIME_OWNERSHIP.md`（Fastify+TS/PostgreSQL+Drizzle/JSON-RPC/F1–F12/GSTR-02/Gate A）> frozen `ARCHITECTURE_V0.2.md`（§4 进程角色/§5 数据模型/§8 状态机/§9 拓扑）> frozen `DEPLOYMENT_AND_DEV_RUNTIME.md`（Nginx 唯一入口/路由表/容器拓扑/§17 standalone lane）> RCE plan（§3.68 D1–D8 + D3-R）> 本 STATE。
> **Stop-condition 结论**: 本 planning **零 frozen Architecture/Contract 修改**——所有决策在既有冻结文档语义内推导（推导处标注出处）；无须 AUTHORITY CONFLICT 上报。

---

## 1. Baseline / live-tree audit（2026-09-21 @ `b2a2c5d`）

| # | 事实 | 锚点 |
|---|---|---|
| B1 | Node 控制面**代码不存在**：无 `apps/`/`packages/`；`ops/Dockerfile.fastify` 为 V0.2 占位模板；compose fastify service 仅 env/healthcheck 占位 | repo tree·`ops/docker-compose.yml:92-107` |
| B2 | media-agent 已具备唯一 canonical 控制面：`/internal/v1/agent` JSON-RPC 四方法，BMD 12/12 真实命令旅程验证（create/actual-state/stop/SIGTERM drain/replay/conflict/独占资源/lifecycle 守卫） | §3.69–§3.70 |
| B3 | Rust prototype `/api/v1/*` 五端点生产 503（诊断面独占）；Product `/api/v1/*` 尚无任何实现 | `transport.rs:228-285`·§3.68 R1 |
| B4 | bind/exposure 政策已显式化：canonical `MEDIA_AGENT_RPC_BIND` 默认 `127.0.0.1:50051`；full-stack = compose 私网（R-e：禁 host port publish；禁 Nginx `/internal/*` 路由；R-f 跨主机私网+mTLS）；compose 尚未给 media-agent 容器设 bind env（实现缺口登记） | §3.71 D3-R |
| B5 | 已知 wire 缝隙：查询面 session id 显示形态 `session-<hex>` vs 命令面 canonical UUID | §3.69/§3.70 披露 |
| B6 | 幂等两层已冻结：agent 进程内 `CommandIdempotency`（D9-A..E·四出口）+ `ApiCommandResponse` 不暴露 Failed（classification 传达） | `idempotency.rs`·`api_boundary.rs` |
| B7 | 错误模型三层：transport 状态码 / JSON-RPC error codes / `ErrorClassification`（Retryable/Permanent/Unknown）+ API 四出口（Executed/Replayed/Conflict/Rejected） | `internal_control.rs`·`error_model.rs` |
| B8 | Nginx frozen 路由表：`/api/* /ops/* /admin/* /ws/* /events/*` → Fastify；**无 `/internal` 路由**；media-agent 容器无 host port publish | `DEPLOYMENT_AND_DEV_RUNTIME.md §11`·`ops/compose.prod.yml` |
| B9 | `events.projection` = **单消费者 drain**（destructive read of in-memory log）——多实例消费会互相偷事件 | `transport.rs`·`events.rs::drain` |
| B10 | `CommandStatus::Accepted` 保留态未产出；Operation API（`GET /commands/{id}`）无内部对应（Product 面职责） | `command.rs:71-80`·§3.68 R4 |

## 2. Frozen decisions（C1–C14，逐项对应裁定域）

### C1 — Product `/api/v1/*` 与 Rust prototype `/api/v1/*` 隔离

- Product `/api/v1/*` = **Fastify 独占命名空间与独占实现**（EXTERNAL_API_CONTRACT §1/§2）。Fastify **绝不**反代/转发任何请求至 agent 的 prototype `/api/v1/*` 五端点；两实现零共享路由。
- Rust prototype 面 = diagnostic-only（生产 503 契约冻结·§3.68 R1）；P1b 静态 demo 页为诊断工件，非 Product UI。
- agent 对外唯一 consumed 面 = `/internal/v1/agent`（B2）；compose/Nginx 层按 B8 隔离（R-e 维持）。
- 命名空间冲突不可能运行时发生（不同进程/端口/路由层）；**规则冻结**：任何新 Product endpoint 不得以"代理 agent prototype 面"实现。

### C2 — Fastify ↔ `/internal/v1/agent` adapter 边界

- Fastify 侧**单一 adapter 模块**（`AgentControlClient`）：JSON-RPC 四方法、显式超时预算（entry 阶段 dispatch ≤10s、query ≤3s）、错误映射到 EXTERNAL_API_CONTRACT §5 error model（`DEPENDENCY_UNAVAILABLE` 等）。
- 禁止：Fastify 直连 agent 进程内存/文件/DB/SSH（Deployment SoT "SSH 不得作为业务 Runtime Control Protocol"）；第二协议；对 mutable runtime 状态做本地缓存——**Resource 读一律 live 内部查询**（防 stale truth）；快照自带 `generated_at_ms/observation_revision`（B2 wire 已有）透传给客户端作为新鲜度标注。

### C3 — Fastify 不拥有媒体生命周期（F1–F12 不变量重申）

- Session/Resource/Lease/Pipeline lifecycle 唯一 owner = agent `SessionManager`/`ResourceRegistry`/`LeaseManager`/Supervisor（F1–F4/F11）；Fastify 只 command + observe。
- Graph Compiler（Fastify 内，F7 非 Job）只产 `GraphRuntimeIntent`（GSTR-02/F10 零执行字段）；materialize 归 agent。
- F9 Gate A 落地：`apps/api`/控制面代码静态禁 `spawn/exec("ffmpeg"|"gst-launch")`（repo-check，与 Deployment SoT §Gate A 一致）。

### C4 — External command → durable idempotency → internal dispatch → actual state 全链

```
client POST /api/v1/... (Idempotency-Key/command_id)
  → Fastify: authn/authz/rate-limit/形状校验（拒绝 = 错误模型 4xx，不占幂等）
  → durable claim（PostgreSQL 唯一键 command_id；fingerprint 见 C6；ABA 防护见 C6）
  → internal command.dispatch（JSON-RPC；agent 内 D9 原子 claim）
  → IdempotentDispatch 四出口 → Fastify 落终态 Operation 记录
  → 客户端响应 = 命令裁决（executed/failed(classification)/conflict/rejected）
  → actual state：客户端/Fastify 经 Product Resource API **live 读** runtime.query（会话 phase/state）
```

- **红线**：Product API 200/executed ≠ Runtime success ≠ SignalVerified ≠ session running——Operation 记录与 Resource 读是两个显式平面（同 §3.68 R4 两平面纪律）。
- 快乐路径外的每条路径见 §4 failure matrix。

### C5 — RH-IDEM-01 durable boundary-of-record（解冻裁定）

- **解冻点 = CP-01B**（首个外部命令入口落地时）；scope = Fastify 外部边界（STATE queue 同步改 DEFER-UNTIL-CONTROL-PLANE → 由 CP-01B 清偿）。
- 分层非双 truth：外部 durable 表 = 客户端重放/冲突/跨 Fastify 重启的 boundary-of-record；agent 进程内 idem = 内部传输重试守卫（进程生命周期）。**Fastify 从不读 agent 幂等表做客户端决策；agent 从不读 Postgres**。

### C6 — command_id / Idempotency-Key / replay / conflict / durability / ABA / concurrent duplicate

- **键**：客户端 `Idempotency-Key` header（优先）或 body `command_id` → 统一为 command_id 字符串；进 agent 时沿用既有 v5 确定性派生（`command_id_from_string` 冻结规则）。
- **fingerprint**（什么算"同一个命令"）：D9-A 字段（kind + canonical target JSON；排除 issued_at/requested_by）**+ 认证主体 id**（同键不同主体 = Conflict，防键跨主体劫持重放）。
- **replay**：同键同 fingerprint 重复提交 → 返回首次终态 outcome（含 failed；逐字节重放首次响应体语义）；未终态（in-flight）→ 短窗口等待后返回 `pending`（Operation 状态，非阻塞长轮询）。
- **conflict**：同键异 fingerprint → `409 RESOURCE_CONFLICT`（错误模型），第二 payload 绝不执行、绝不重放。
- **durability（Fastify 重启）**：claim/终态皆在 PostgreSQL；重启后 in-flight 记录按租约回收（超龄 claimed → `timeout(retryable)` 终态——绝不猜成功）。
- **ABA 防护**：Operation 状态机单向（claimed → completed|failed|timeout|conflict|rejected 单次条件迁移 `UPDATE ... WHERE state='claimed'`）；终态记录不可变，无覆写路径。
- **concurrent duplicate**：`command_id` 唯一索引——首插者 claim，并发者走 replay/pending 分支；恰一次外部终态。
- **诚实边界（如实冻结）**：跨 **agent** 重启的执行级 exactly-once 不宣称——agent 幂等表进程内（§3.68 R5）；dispatch 已达 agent 但 verdict 未及持久化时 Fastify 崩溃 → 恢复后标 `timeout(retryable)`，**不自动重发** `start_session`（session 创建非 agent 重启幂等；重试为显式操作员/客户端动作，新 command_id）。agent 侧 durable idem 若未来证据需要 → 独立 bounded packet 再裁（不进 RH-IDEM-01 外部边界 scope）。

### C7 — Resource / Command / Operation / Event 四类 API 责任边界（§2.1 对齐）

| API 类 | 责任 | 数据源 |
|---|---|---|
| Resource（GET/PUT） | 只读现状投影（entry 阶段 GET-only）；PUT/ChangeSet 流程后续包 | live `runtime.query`（无本地 truth） |
| Command（POST） | 触发动作 + durable 幂等入口 | → `command.dispatch` |
| Operation（`GET /commands/{id}`） | 命令旅程持久记录查询 | PostgreSQL Operation 记录（**非** Runtime 现状） |
| Event（SSE/webhook） | 外部投递投影 | agent `events.projection` drain → outbox（EVENT_CONTRACT §2 归属 Fastify） |

### C8 — `GET /commands/{id}` 与 Runtime actual state 的关系

- Operation 记录状态词表（冻结）：`pending`（claimed 未终态）/ `completed`（裁决 executed）/ `failed`（classification=retryable|permanent|unknown）/ `timeout`（retryable）/ `conflict` / `rejected`。
- Operation = **命令旅程事实**；session/resource 现状 = Resource API live 读。两者在 Product wire 上是不同资源、不同端点、不同语义；文档与 UI 必须分开呈现（C13）。禁止用 Operation `completed` 推断 "session running"。

### C9 — session canonical UUID ↔ `session-<hex>` 缝隙的 Product 映射

- entry 阶段：**Fastify adapter 规范化**——解析 `session-<hex>` → canonical UUID 作为 Product `id`（URL/响应统一 UUID 形态）；显示串仅作 label。零 Rust 改动。
- 登记后续项：agent `ApiSession` 增补 canonical UUID 字段（additive wire）= 独立小 Rust packet（不阻塞 CP；落地后 adapter 规范化退役）。

### C10 — PostgreSQL 记录分类（什么是 CP 持久记录 / 什么绝不能成为 Runtime truth）

- **是 CP persistent record**：commands/operations（durable idempotency + 命令旅程）、audit（每外部命令 who/what/when/command_id/verdict）、authn 身份与 RBAC 授权、（后续包）config_revisions/change_sets、event outbox、V0.2 §5 其余历史/分析表（health_trees 等 = **历史/审计**，非现状）。
- **绝不成为 Runtime truth**：session 活动态、resource/lease 分配态、device health、pipeline/supervisor/recovery 现状——一律 live 读 agent（Runtime owns truth；TECHNOLOGY_STACK §3 "Health (runtime truth) = Rust"）。DB 快照类数据一律标注历史语义，禁止作为现状响应源。

### C11 — Auth / RBAC / Audit / Rate-limit ownership

- 全部 Fastify（EXTERNAL_API_CONTRACT §6：默认 authenticated/authorized/audited/rate-limited；authz 在应用层；Better Auth + CASL）。Nginx 只 TLS/routing/IP filter/粗粒度限流。
- agent internal 面**无认证**（D3-R：网络边界即安全边界）；`/internal/*` 永不进 Nginx 公网路由（B8 维持）。
- 审计 = Fastify 持久写（C10）；Operation/audit 记录含 command_id 贯穿链。

### C12 — Failure semantics 矩阵（见 §4 全表）

覆盖：Runtime unavailable / dispatch timeout / stale observation / Fastify restart / Agent restart / duplicate event / reordered event。

### C13 — Web Console 契约（未来包的硬边界）

- Web Console 只消费 Fastify Product API；禁直连 agent；禁本地 optimistic success——命令结果必须呈现 Operation 终态 + Resource live 现状两平面；UI 不发明成功状态（WEB-CONSOLE queue 验收标准引用本条）。

### C14 — standalone first / compatible-not-dependent 维持

- standalone lane 零变化：media-agent 无 Fastify/Postgres 依赖照常全功能（internal 面 = operator canonical 入口，BMD 已证）；Fastify = additive full-stack lane。
- CP 包不得引入 agent → 控制面方向依赖；compose prod overlay 叠加在既有冻结拓扑上（B8 路由/端口规则不变）。

## 3. Command / actual-state contract（时序冻结）

| 阶段 | 权威 | 载体 | 失败分支 |
|---|---|---|---|
| requested | client | POST + Idempotency-Key | 4xx 形状/authz/rate（不占幂等） |
| claimed | Fastify+PG | commands 行（唯一键） | 冲突→409；in-flight→pending |
| dispatching | agent | 内部 JSON-RPC（D9 原子 claim） | 503/超时→timeout(retryable) |
| verdict | agent→Fastify | 四出口 + classification | failed 语义经 classification |
| recorded | Fastify+PG | Operation 终态（单向迁移） | ABA 不可能（条件更新） |
| **actual state** | **agent（唯一）** | Product Resource API live 读 | stale 由无缓存策略消除 |
| events | Fastify 投影 | events.projection drain → outbox | 单实例约束（B9） |

## 4. Failure-first matrix（实现包必须逐项覆盖）

> **2026-09-23 correction（用户指令·最小修正，不重开设计）**：F13 原文"非法形态 4xx"未区分错误方向。修正为双向责任归属——客户端输入非法 UUID 是客户端错误（4xx `VALIDATION_ERROR`）；agent 返回非法 session-id wire 是 Fastify↔agent 内部契约违例，绝不能返回客户端 4xx，映射为既有 taxonomy 的 `INTERNAL_ERROR`（5xx、`retryable` 显式、诊断只进 server log、响应不泄漏内部细节）。

| # | 注入 | 期望行为 |
|---|---|---|
| F1 | agent 停机/503 | Product 返回 `DEPENDENCY_UNAVAILABLE`(retryable)；已 claim 命令 → `timeout(retryable)`；绝不假成功 |
| F2 | dispatch 超预算 | 同 F1 timeout 语义；不自动重发 start_session |
| F3 | 同键重复提交（终态后） | 逐字节重放首次 outcome（含 failed） |
| F4 | 同键异 payload | 409 conflict；第二 payload 零执行 |
| F5 | 同键并发 | 恰一次 claim；其余 pending/replay |
| F6 | Fastify 崩溃于 dispatch 中 | 重启后超龄 claimed → timeout(retryable)（租约回收，不猜成功） |
| F7 | agent 重启（会话 drain） | /health Starting→Ready；Resource 读跟随；Operation 记录不受影响 |
| F8 | stale observation 攻击面 | 无 mutable 状态缓存；每次读带 agent `generated_at_ms` |
| F9 | 事件重复消费 | 单实例 drain 约束（多实例禁用事件面直至 outbox 包）；outbox 投递幂等（consumer 幂等键） |
| F10 | 事件乱序 | 投影聚合以事件内容为准（计数/状态集）；SSE 带 cursor（无全局序时标注弱序，如实） |
| F11 | Nginx 误配 `/internal` 路由 | 部署断言/文档红线测试（compose 断言无该路由、无 50051 publish） |
| F12 | Fastify 试图 spawn ffmpeg / 直连设备 | 静态 Gate A repo-check FAIL（C3） |
| F13 | Resource 读映射 session id | **责任归属双向（2026-09-23 用户指令修正）**：**客户端输入**非法 UUID → 4xx `VALIDATION_ERROR`；**agent `runtime.query` 返回非法 session-id wire** = Fastify↔agent 内部契约违例 → 5xx `INTERNAL_ERROR`（明确 `retryable=false`；server log 保留内部诊断且 Product 响应不泄漏内部实现细节；不猜测、不生成伪 UUID、不把 malformed upstream 当成"没有 session"） |

## 5. Persistence / durable idempotency contract（CP-01B 落地基线）

- 表（对齐 V0.2 §5 命名族，CP-01B 做 schema reconciliation）：`commands`（command_id 唯一·principal·fingerprint·state·verdict·classification·timestamps·request/response digest）；`audit_entries`；（CP-01D）`event_outbox`。
- 不变量：state 单向迁移条件更新；终态不可变；fingerprint 含 principal（C6）；审计与命令同事务落终态。
- 明确非目标（后续包）：ChangeSet/Preflight 持久流、webhook 签名投递、多实例事件分发。

## 6. Owner map（零新 Runtime owner）

| 能力 | Owner | CP 侧形态 |
|---|---|---|
| 媒体生命周期/健康 truth | Rust agent（不变） | 只经 internal 面消费 |
| Graph 编译 | Fastify（F7/F10） | 只产 `GraphRuntimeIntent` |
| 幂等（外部边界） | Fastify+PostgreSQL | durable boundary-of-record |
| 幂等（进程内） | agent（不变） | 传输重试守卫 |
| Auth/RBAC/审计/限流 | Fastify | Better Auth + CASL |
| External Event 投影/投递 | Fastify（EVENT_CONTRACT §2） | drain→outbox→SSE/webhook |
| Nginx | TLS/路由/粗限流 | `/internal` 永不路由 |

## 7. Implementation packet decomposition（bounded；禁一次铺开）

| 子包 | 范围 | 明确不做 | 验证 |
|---|---|---|---|
| **CP-01A** skeleton + adapter | Fastify 应用骨架、`AgentControlClient`（四方法/超时/错误映射）、Product 读 API v0（`GET /api/v1/runtime` 投影、`/healthz`）、compose dev overlay agent bind env 接线、F11/F12/F13 静态与路由断言、adapter 契约测试（对 mock JSON-RPC server） | DB/命令写入/Auth/事件 | VM compose 起停 + 契约测试 + CI（新 lane，暂非 required context——required 化随首包收口裁定） |
| **CP-01B** durable command entry | PostgreSQL/Drizzle migration、commands/audit 表、`POST` 命令 API（C4/C6 全语义）、`GET /commands/{id}`、F1–F8 | Auth 真实集成（开发态局部鉴权桩）、事件、Web Console | VM 集成测试（含崩溃/并发注入）+ CI |
| **CP-01C** Auth/RBAC/限流/审计硬化 | Better Auth + CASL + rate limit + 审计全覆盖 | 事件、Console | VM + CI + 安全检查单 |
| **CP-01D** Event plane | internal drain→outbox→SSE（单实例约束）、cursor | webhook 签名投递、多实例 | VM + CI |
| **CP-01E** 全栈/BMD 集成验收 | compose prod overlay 全链（Nginx→Fastify→internal→agent）、failure drill 矩阵重放、standalone 零回归（agent 无 CP 照常）、BMD 真机一轮 | — | BMD + VM + CI |
| 显式 deferred | BullMQ/Worker 异步面、Web Console、`vbmf-sdk`、跨主机 mTLS、agent 侧 durable idem、agent UUID wire 增补（C9 后续项） | — | — |

依赖序：CP-01A → CP-01B → (CP-01C ∥ CP-01D) → CP-01E。每个子包独立 STATE 收口；Rust 侧改动（若有，如 C9 后续项）走独立 Rust packet 不混入 CP 包。

## 8. Verification requirements（闭环口径）

- **Dev VM**：各包单测/契约测试（adapter 对 mock JSON-RPC；PG 用 ephemeral 实例跑迁移与并发/崩溃注入）；compose dev 起停。
- **CI**：新 `control-plane` lane（lint/typecheck/test/build；是否入 required contexts 在 CP-01A 收口时按既有灰度纪律裁定，不擅改 branch protection）；media-agent 7/7 required 不受影响。
- **BMD**：CP-01E 一轮（真实 systemd agent + compose Fastify；命令旅程 × Product API；failure drill；standalone 零回归；device-2/`/opt/vbmf-dev` 边界）。

## 9. Forbidden（planning 与各实现包通用）

一次铺开 Fastify+PostgreSQL+BullMQ+Auth+Web Console；Fastify 拥有媒体生命周期/直连设备/spawn ffmpeg（F1–F12）；Product API 代理 agent prototype 面；Nginx 路由 `/internal`；50051 host publish；DB 成为 Runtime truth；agent 依赖控制面启动/运行/恢复；UI 本地 optimistic success；跨主机明文 internal；未经裁定的 required context 变更。
