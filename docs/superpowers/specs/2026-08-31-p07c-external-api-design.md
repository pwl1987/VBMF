---
comet_change: p07c-external-api
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-31-p07c-external-api
status: final
---

# Design Doc — p07c-external-api（Phase 0.7C-7: External API Foundation）

> open design.md §1-§6 实现级细化。锚点：终审 0.7C-6 Gate（External API 顺序+三 NOTE）+ Contract Probe `docs/superpowers/reports/2026-08-31-p07c7-external-api-contract-probe.md`。
> **本 change 仅冻结 API Boundary Model + Idempotency 契约；三平面映射通过纯转换函数；transport 实现属下一 change（std-only 纪律）**。

## 1. api_boundary.rs 类型与纯函数（design.md §1）

```rust
// Query 五资源独立 API 类型（不绑回 Runtime 内部 enum）
pub struct ApiDevice { id, model, binding: Option<String>, capabilities: Option<Vec<ApiCapability>> }
pub struct ApiPort { id, device_id, direction }
pub struct ApiResource { id, device_id, state: String }   // state 字符串化（available/reserved/...）
pub struct ApiSession { id, state, phase: String }
pub struct ApiCapability { flag, summary }
pub fn to_api_device / port / resource / session / capabilities(&...) -> ApiX;

// Command API model（独立 enum 命名）
pub struct ApiCommandRequest { command_id: String, kind: String, target: ApiCommandTarget, requested_by: String }
pub enum ApiCommandTarget { Session { intent: serde_json::Value }, SessionById { session_id: String } }
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ApiCommandStatus { Executed, Replayed, Rejected, Conflict }  // 4 态；不暴露 Failed
#[serde(rename_all = "snake_case")]
pub enum ApiErrorClass { Rejected, Conflict, Retryable, Permanent, Unknown }  // 去 _failure 后缀
pub struct ApiCommandResponse { command_id, status, kind, classification: Option<ApiErrorClass>, detail: Option<String> }

// Event API model（Projection ≠ State 红线守护）
pub struct ApiEventEnvelope { kind, session_id: Option<String>, severity: String, ts_ms: u64 }
pub struct ApiProjectionResponse {
    pub snapshot_kind: ApiProjectionKind,   // 必含 "event_projection_snapshot" 守门
    pub total: usize, pub kind_counts: BTreeMap<String, usize>,
    pub session_states: BTreeMap<String, String>, pub session_failures: BTreeMap<String, usize>,
    pub has_critical: bool,
}
#[serde(rename_all = "snake_case")]
pub enum ApiProjectionKind { EventProjectionSnapshot }
```

## 2. Idempotency 边界契约（design.md §2，仅契约）

```rust
pub struct ApiIdempotencyBoundary {
    pub current_backend: ApiIdempotencyBackend,           // 当前=ProcessLocal
    pub durable_persistence: ApiPersistenceOption,        // DurableLogDeferred / ExternalKvDeferred
    pub cross_restart_semantics: ApiCrossRestartSemantics, // 当前=RestartBreaksReplay
}
pub enum ApiIdempotencyBackend { ProcessLocal }
pub enum ApiPersistenceOption { DurableLogDeferred, ExternalKvDeferred }
pub enum ApiCrossRestartSemantics { RestartBreaksReplay, RestartAllowsReplay }
```

## 3. 红线白盒（design.md §4）

测试断言（feature=mock）：serde 反向断言 `Api*` JSON 字符串禁出现 `ResourceState`/`SessionPhase`/`CommandStatus`/`IdempotentDispatch`/`EventSeverity` 字样；`ApiProjectionResponse` 序列化必含 `snapshot_kind: "event_projection_snapshot"`；`ApiCommandStatus` 不出现 `failed` 变体（验证 NOTE-2 命名解耦）。

## 4. 测试矩阵（api_rt_01_*）

1. `api_models_decoupled_from_runtime_types` — serde 反向断言红线白盒。
2. `api_command_status_no_failed_state` — 验证命名解耦（不暴露 Failed）。
3. `api_projection_kind_enforced` — 验证 snapshot_kind 必出现。
4. `to_api_query_models` — Query 五资源转换完整字段+值域。
5. `api_command_request_field_shape` — command_id 非空、kind 三词表封闭、target 二选一。
6. `idempotency_boundary_contract` — 三选项对勘公开化。
7. 真机 EXTERNAL-API-RT-01 — gate 段 API 资源快照打印 + 全门禁回归。

## 5. 触碰面

`api_boundary.rs`（新）/ `main.rs`（mod + 真机 gate 段 API 资源快照追加）。其他模块零触碰。