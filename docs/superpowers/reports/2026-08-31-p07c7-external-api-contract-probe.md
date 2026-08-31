# 0.7C-7 External API Contract Probe（只读调查）

- 日期：2026-08-31 · 基线：master=`9b475c1`（0.7C-6 baseline）
- 性质：只读探针（零代码改动）——回答终审 0.7C-6 Gate 指定的 API 边界核心问题与开工前置检查，作为 p07c-external-api design 输入。
- 0.7C-6 Gate NOTE 锁定：①Health Reducer 完整实现 deferred ②Event persistence deferred ④Cross-restart idempotency deferred ⑤Projection 不得取代 CanonicalRuntimeState ⑥External API 开工前必须重新定义"对外模型"。

## Q1 现有 Query/Command/Event 三平面公开面全景

**Query 平面（`runtime_query.rs`，Pure Read 门面）**——已对外暴露 7 方法：
- `get_runtime_state() → CanonicalRuntimeState`
- `get_device(id) / get_port(id) / get_resource(id) / get_session(id) → Option<...>`
- `list_sessions() → Vec<SessionRuntimeState>`
- `get_capabilities() → Vec<(Uuid, DeviceCapabilitiesSummary)>`
- 零新 DTO（**全返回既有 CanonicalRuntimeState 子项**，终审已定）

**Command 平面（`command.rs`，不可执行性三重守护）**——对外暴露 6 类型 + 2 方法：
- `CommandKind` 封闭三命令 + `CommandId(Uuid)` + `CommandTarget`（Session/SessionById）+ `CommandEnvelope`（**零执行字段**）+ `CommandStatus` 四态 + `CommandOutcome`（含 classification `ErrorClassification`）+ `CommandRejection`
- `validate(envelope) → Result<(), CommandRejection>`（纯函数形状校验）
- `dispatch(mgr, envelope) → CommandOutcome`（薄映射，无 Executor）

**Idempotency 平面（`idempotency.rs`，D9 Foundation）**——对外暴露：
- `CommandFingerprint` + `fingerprint(envelope)` 纯函数
- `IdempotentDispatch` 四出口 {Executed, Replayed, Conflict, Rejected}
- `CommandIdempotency::new(mgr)` + `dispatch(envelope) → IdempotentDispatch`

**Error Model 平面（`error_model.rs`，三平面分离）**——对外暴露：
- `ErrorClassification` 五变体
- `classify_session_error(err) → ErrorClassification`（SessionError 九臂封闭映射）

**Event 平面（`event_projection.rs` + `events.rs`）**——对外暴露：
- `EventProjection` 五字段（total/kind_counts/session_states/session_failures/has_critical）
- `project(events) → EventProjection` 纯函数
- `RuntimeEventLog::drain() → Vec<RuntimeEvent>`（消费者唯一入口）+ 丢弃计数 `dropped_observations()/dropped_criticals()`

## Q2 Runtime Resource Model 当前公开面（候选 API 资源模型源）

`resource.rs` 公开： `ResourceState` 五状态 + `Resource/Reservation` + `ResourceStateError`；**无 ResourceRegistry 公开查询 API**（通过 `SharedResourceRegistry::derive_from_discovery` + SessionManager 内部聚合，不暴露给外层直接查）。**已对外的 Resource 视图仅经 `runtime_query::get_resource()`**——`ResourceRuntimeState`（CanonicalRuntimeState 子项）。

## Q3 CommandId 持久化现状（External API Idempotency 持久化的真正问题）

- `CommandId(pub Uuid)`——**随机 UUID**（每次调用方自己生成——`Uuid::new_v4()`）。
- `CommandIdempotency::records: Mutex<HashMap<CommandId, Record>>`——**进程内内存表**，无持久化（0.7C-4 终审 §10 已显式 deferred to External API stage）。
- 终结问题的边界：①进程重启后同 `CommandId` 视为**新命令**（"重启后同 command_id=新实例"——已写入债表 D9 措辞）；②容量上界/驱逐策略当前不做（**驱逐=replay 退化成重执行，破坏 D9-D 故意不驱逐**）；③跨进程/跨重启持久化属 External API 阶段闭环。

## Q4 RPC/HTTP transport 现状

**零 transport 依赖**（Cargo.toml 无 axum/hyper/warp/actix/grpc/tower）。`main.rs:495` 注释"最简 /health (std TcpListener, 无第三方依赖; 后续可换 axum)"——**当前无 HTTP/RPC server**。**External API 阶段首次引入 transport 依赖，是新依赖红线（license/编译期/binary 大小）**。

## Q5 0.7C-6 NOTE 对应 Architecture Probe 子项（开工前置确认）

| NOTE | Probe 结论 |
|---|---|
| **NOTE-1 Projection ≠ RuntimeState** | 当前实现：Projection 由 event 流构造（**纯只读**），无写回路径。`RuntimeQuery::get_runtime_state() → CanonicalRuntimeState` 仍是**权威**状态。两条路径解耦：`RuntimeState`（拉式聚合 by SessionManager）vs `EventProjection`（推式快照 by event sink）——**同源不同象，不可互替**。Probe 风险点：External API 阶段若同时暴露两者，必须明确"哪个是 What is true now"（Query=true north star），Projection非非"快照视图"——>本 change 开工须白盒锁定（**API 不暴露 `EventProjection` 当作权威状态**——只可作辅助视图） |
| **NOTE-2 不暴露 Rust 内部结构** | 现状：Query 返回类型已是 canonical DTO（`DeviceRuntimeState`/`PortRuntimeState`/`ResourceRuntimeState`/`SessionRuntimeState`），**不是裸 SessionManager 内部**——`0.7C-2` 终审已锁定零新 DTO 原则（复用 CanonicalRuntimeState 子项）。但 `CommandEnvelope/CommandOutcome/IdempotentDispatch/ErrorClassification/EventProjection` **当前是 Rust 内部枚举**（serde-tagged）——若原样暴露给外部，等同暴露 Rust serde tag 习惯（"rejected"/"retryable_failure"/"verdict"/"verdict":"replayed"）——**与"不暴露 Rust 内部结构"原则有裂痕** |
| **NOTE-3 Resource Model 独立** | 现状：Runtime 内部 `Resource/ResourceState/Reservation` 与 Query 外部 `ResourceRuntimeState` 已分裂（前者带状态机动词，后者只读快照）。External API 阶段应明确"API 资源模型**独立定义**于 Runtime Resource Model"——不是投影，** |
| **NOTE-4 Idempotency 持久化边界** | 现状：CommandId 进程内，重启=新实例。External API 阶段须显式裁决：①持久化存储（durable log/SQLite/外部 KV）边界 ②跨重启后**已知 CommandId 是否仍可重放**——**三选项对勘留待 design 阶段展开** |

## 0.7C-7 范围裁定输入（probe 结论）

### 推荐顺序（终审建议）
```
External API Contract Probe (本探针 ✓)
  ↓
API Boundary Model（独立资源模型——非 Runtime Resource Model 投影）
  ↓
Idempotency 持久化边界裁决（durable storage vs 进程内 vs 重跨即新）
  ↓
Query/Command/Event 三平面映射（每个平面→API 资源模型的转换规则）
  ↓
Transport 实现（首推 std HTTP 或 thin RPC；非 axum 大依赖）
```

### 探针级核心问题（design 阶段须逐一回答）
1. **API 资源模型 = Runtime Resource Model 的投影？还是独立？**——probe 推荐独立（避免暴露 Rust 内部结构）
2. **CommandId 持久化如何裁决？**——三选项对勘留待 design
3. **transport 选型？**——probe 推荐 std HTTP（与 0.7A "std TcpListener 无第三方依赖"纪律一致）——**非 axum 大依赖**
4. **三平面映射规则**：Query=true north star / Command=mutation 请求（含 classification+fingerprint）/ Event=历史轨迹（不替代权威态）
5. **Projection 在 API 暴露的角色**——辅助视图（"事件流快照"），**绝不替代 CanonicalRuntimeState**

### 明确不做（probe 锁定）
- 直接 `REST handler → serialize Rust struct`（违反 NOTE-2）
- 大依赖（axum/tower/grpc）——破坏 0.7A "std-only 纪律"
- 把 EventProjection 当作权威 RuntimeState（违反 NOTE-1）
- 重做 Runtime Resource Model（违反 0.7 终审：Canonical 不绑回 Vendor；本模型已锁定）
- Idempotency 持久化存储实现（仅 boundary 契约——实现属后续 change）
- 任何"万能 API DTO"（违反 0.7B 加严红线：禁万能 struct）