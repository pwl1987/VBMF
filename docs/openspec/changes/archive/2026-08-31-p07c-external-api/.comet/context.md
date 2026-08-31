# Comet Design Handoff

- Change: p07c-external-api
- Phase: design
- Mode: compact
- Context hash: 625ac30f7b9bf9a375ea86bcef8effafbb49d7fef3b302ea0bb9961bb1b94343

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07c-external-api/proposal.md

- Source: docs/openspec/changes/p07c-external-api/proposal.md
- Lines: 1-32
- SHA256: bf67fd66610b3d4ec016763d62e98d6b6aa9beb290fb467988a5440ada4c7e20

```md
# Change: Phase 0.7C-7 — p07c-external-api（External API Foundation：API Boundary Model + Idempotency 持久化边界 + 三平面映射）

## Why

0.7C-6 Merge Gate PASS（master=`9b475c1`）。终审裁定（2026-08-31）：
- **正确顺序**：Contract Probe（只读四问，已完成 `docs/superpowers/reports/2026-08-31-p07c7-external-api-contract-probe.md`）→ **API Boundary Model（独立资源模型，非 Runtime Resource Model 投影）** → Idempotency 持久化边界裁决 → Query/Command/Event 三平面映射 → transport 实现。
- **三 NOTE**：①EventProjection 不得取代 CanonicalRuntimeState ②禁 `REST handler → serialize Rust struct`（不暴露 Rust serde tag 习惯）③API Resource Model 独立定义；transport 保持 0.7A std-only 纪律（禁 axum/hyper/tower 大依赖）。
- **PR 规模判定**：生产代码 diff 按文件白名单审核，不按 PR 总行数（Comet 归档产物不计入 scope creep）。

## What Changes

- **`src/api_boundary.rs`（新）**——独立 API Resource Model（**非 Runtime Resource Model 投影**，禁万能 struct，加严红线）：
  - 资源五件套（API 视角）：`ApiDevice`、`ApiPort`、`ApiResource`、`ApiSession`、`ApiCapability`——字段仅由 API 消费语义驱动，**不绑回** `ResourceState`/`SessionPhase` 等 Runtime 内部枚举（serde 显式映射为 API 字符串）。
  - **`pub fn to_api_X(...)` 纯转换函数**（Query 平面）：`CanonicalRuntimeState` 子项 → `ApiX`——字段命名按 API 习惯（如 `state` 用 "available"/"reserved" 等 API 友好态），**与 Rust 内部 enum 名解耦**。
  - **Command API model**：`ApiCommandRequest`（含 `command_id: String`——客户端提供，**接受任意 string 后续持久化为 UUID 内部表示**）、`ApiCommandResponse { status: ApiCommandStatus, kind: ApiKind, classification: ApiErrorClass, outcome_equal, sessions, fingerprint_conflict?, detail? }`——**ApiCommandStatus ≠ CommandStatus**（"accepted" vs `Accepted`、"rejected" vs `Rejected` 等）独立 enum，避免暴露 serde tag。
  - **Event API model**：`ApiEventEnvelope { kind, session_id?, severity, ts_ms }` + `ApiProjectionResponse { total, kind_counts, session_states, session_failures, has_critical }`——`EventProjection` 是**辅助视图**，**不替代** `CanonicalRuntimeState`（API 暴露时标注"event projection snapshot, not authoritative state"）。
  - **红线白盒**：`api_boundary.rs` 零 runtime_query/command/idempotency 公开类型依赖（仅经 `to_api_*` 纯函数读 canonical 子项构造 API 模型）；serde 反向断言禁 vendor/执行词；allowlist `[to_api_device, to_api_port, to_api_resource, to_api_session, to_api_capabilities, to_api_event_envelope, to_api_projection]`。
- **Idempotency 持久化边界裁决**（**契约层，非实现**）：`ApiIdempotencyBoundary` 文档型结构（`pub` 字段）——**External 提交幂等键的契约承诺**（API 文档级，**不引入持久化实现**，避免越界做存储层）：
  - **承诺**：External API 端持久化 `command_id → outcome` 映射；提交同 `command_id` 重发**期望**返回原 outcome（**契约**）；
  - **当前实现边界**：进程内 `CommandIdempotency` 表——**仅承诺同进程内重放**，**跨重启/跨进程持久化**留待后续 change（契约层先冻结，实现层分步）；
  - **三选项对勘公开化**：①进程内（当前）②durable log/SQLite（External API 实现阶段二）③外部 KV/Redis（实现阶段三）——本 change 仅冻结**契约**与**选项标记**，不动存储实现。
- **门禁 EXTERNAL-API-RT-01（三层）**：Unit（API 资源模型与 Runtime 内部类型解耦——serde 反向断言禁 `ResourceState`/`SessionPhase`/`CommandStatus` 等字样；API 字段快照测试；allowlist 白盒）；Simulation（Query→API 五资源转换完整字段+值域；Command API 模型字段闭包；Idempotency 契约层三选项对勘）；Hardware（真机 SESSION_LIFECYCLE gate 段追加 API 资源快照打印 + 回归全部门禁）。
- **CI**：测试并入现有矩阵。

## Capabilities

（`skip_specs: true`——SoT 为终审 0.7C-6 Gate（External API 顺序+三 NOTE）+ probe 报告 + PHASE_IMPLEMENTATION_MAP §3。）

## Impact

- 编译：五套 feature 不回退；api_boundary.rs 零 vendor 依赖，零 transport 依赖。
- 受影响：新 `api_boundary.rs`；`main.rs`（mod + 真机 gate 段追加 API 资源快照）；Phase Map（0.7C-7 行）；债表（Idempotency 持久化条目登记）。
- **明确不做**：transport 实现（HTTP/RPC server——下一 change）；durable 持久化存储实现（SQLite/Redis/Kafka 禁令延续）；API Resource Model = Runtime Resource Model 投影（违反 NOTE-3）；REST handler → serialize Rust struct（违反 NOTE-2）；EventProjection 替代 CanonicalRuntimeState（违反 NOTE-1）；大依赖（axum/hyper/tower/grpc）；CommandId 持久化实现（仅契约冻结）；万能 API DTO（禁万能 struct 加严红线）；任何 Runtime 内部 enum 原样暴露。
```

## docs/openspec/changes/p07c-external-api/design.md

- Source: docs/openspec/changes/p07c-external-api/design.md
- Lines: 1-194
- SHA256: 4229d7816f4021edbfa4f429bbf771bc0b1257ad117d0afa772a70ab69e5f5bf

[TRUNCATED]

```md
# Design: Phase 0.7C-7 — p07c-external-api

## 0. 终审裁定落点 + probe 引用

| 终审裁定 + probe 结论 | 设计落点 |
|---|---|
| **API Resource Model 独立定义**（非 Runtime 投影，NOTE-3） | §1 api_boundary.rs 五大独立 API 类型 + to_api_* 纯转换 |
| **不暴露 Rust serde tag 习惯**（NOTE-2：verdict/retryable_failure 等） | §1 ApiCommandStatus ≠ CommandStatus；ApiErrorClass ≠ ErrorClassification；字段命名与 Rust enum 解耦 |
| **EventProjection ≠ CanonicalRuntimeState**（NOTE-1） | §1.3 ApiProjectionResponse 标注 "event projection snapshot, not authoritative state" |
| **顺序：Contract Probe ✓ → Boundary → Idempotency 边界 → 三平面映射 → transport** | §2 Idempotency 边界契约（仅契约，非实现）+ §3 三平面映射表；transport 留下一 change |
| **transport std-only 纪律**（0.7A） | 本 change 零 transport 依赖（README）；transport 实现属下一 change |

## 1. API 资源模型（api_boundary.rs 新）

### 1.1 Query 五资源独立 API 类型

```rust
/// 字段仅由 API 消费语义驱动 (加严红线: 禁万能 struct); 不绑回 Runtime 内部 enum。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiDevice {
    pub id: String,
    pub model: String,
    pub binding: Option<String>,       // canonical binding_status (high/manifest_verified/...) 字符串化
    pub capabilities: Option<Vec<ApiCapability>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiPort { pub id: String, pub device_id: String, pub direction: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiResource { pub id: String, pub device_id: String, pub state: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiSession { pub id: String, pub state: String, pub phase: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiCapability { pub flag: String, pub summary: String }
```

**关键解耦**：`ApiResource.state` 是 **API 字符串**（`"available"` / `"reserved"` / `"allocated"` / `"releasing"` / `"faulted"`），**不是** `ResourceState` enum 直接暴露；`ApiSession.state/phase` 同理。to_api_* 纯函数显式映射。

```rust
pub fn to_api_device(d: &DeviceRuntimeState) -> ApiDevice;
pub fn to_api_port(p: &PortRuntimeState) -> ApiPort;
pub fn to_api_resource(r: &ResourceRuntimeState) -> ApiResource;
pub fn to_api_session(s: &SessionRuntimeState) -> ApiSession;
pub fn to_api_capabilities(cs: &[(Uuid, DeviceCapabilitiesSummary)]) -> Vec<(String, ApiCapability)>;
```

### 1.2 Command API model（独立 enum 命名）

```rust
/// 接受客户端 command_id 字符串 (后续持久化为 UUID 内部表示); 不绑回 CommandId 内部类型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiCommandRequest {
    pub command_id: String,       // 客户端提供 (任意非空字符串); API 层负责规范化
    pub kind: String,             // API 友好: "start_session" / "stop_session" / "release_session"
    pub target: ApiCommandTarget, // 见下
    pub requested_by: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "target_type", rename_all = "snake_case")]
pub enum ApiCommandTarget {
    Session { intent: serde_json::Value },  // intent 不绑回 GraphRuntimeIntent 结构 (JSON 透传)
    SessionById { session_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ApiCommandStatus {
    Executed, Replayed, Rejected, Conflict,  // 4 态 (与 CommandStatus 4 态同构, 但命名 API 友好)
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorClass {
    Rejected, Conflict, Retryable, Permanent, Unknown,  // 5 类 (与 ErrorClassification 同构, 但无 _failure 后缀)
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiCommandResponse {
    pub command_id: String,
    pub status: ApiCommandStatus,
    pub kind: String,
    pub classification: Option<ApiErrorClass>,

```

Full source: docs/openspec/changes/p07c-external-api/design.md

## docs/openspec/changes/p07c-external-api/tasks.md

- Source: docs/openspec/changes/p07c-external-api/tasks.md
- Lines: 1-53
- SHA256: 896a7c2f88c5a7a0391e8ed429eae1058e91000c4985f634899318330c15b06b

```md
# Tasks: Phase 0.7C-7 — p07c-external-api

四栏纪律：`Contract: 已有(引用冻结文档节号) / Implementation / Verification / Gate`。

## 1. API Resource Model（design.md §1）
- [ ] api_boundary.rs 五大独立 API 类型（ApiDevice/Port/Resource/Session/Capability）+ to_api_* 纯函数（Query 平面转换）
      Contract: 终审 0.7C-6 NOTE-3（独立定义，非 Runtime 投影）+ 0.7B 加严红线（禁万能 struct）
      Implementation: api_boundary.rs
      Verification: `api_rt_01_to_api_query_models`
      Gate: EXTERNAL-API-RT-01 Unit 层
- [ ] Command API model（ApiCommandRequest/Response/Status/Target/ErrorClass）独立 enum 命名
      Contract: 终审 NOTE-2（不暴露 Rust serde tag 习惯）+ 0.7C-5 三平面分离
      Implementation: api_boundary.rs
      Verification: `api_rt_01_api_command_status_no_failed_state` + `api_rt_01_api_command_request_field_shape`
      Gate: EXTERNAL-API-RT-01 Unit 层

## 2. Event API model + Projection 红线守护（design.md §1.3）
- [ ] ApiEventEnvelope + ApiProjectionResponse + ApiProjectionKind 守门
      Contract: 终审 NOTE-1（EventProjection 不得取代 CanonicalRuntimeState）
      Implementation: api_boundary.rs
      Verification: `api_rt_01_api_projection_kind_enforced`（序列化必含 snapshot_kind）
      Gate: EXTERNAL-API-RT-01 Unit 层

## 3. Idempotency 持久化边界契约（design.md §2）
- [ ] ApiIdempotencyBoundary 公开契约 + 三选项对勘（ProcessLocal/DurableLogDeferred/ExternalKvDeferred + RestartBreaksReplay/RestartAllowsReplay）
      Contract: 终审 NOTE + probe Q3（CommandId 进程内）
      Implementation: api_boundary.rs（仅契约，无持久化逻辑）
      Verification: `api_rt_01_idempotency_boundary_contract`
      Gate: EXTERNAL-API-RT-01 Unit 层

## 4. 红线白盒（design.md §6）
- [ ] serde 反向断言：api_boundary.rs 零 `ResourceState`/`SessionPhase`/`CommandStatus`/`IdempotentDispatch`/`EventSeverity` 内部 enum 字样
      Contract: 终审 NOTE-2 + 0.7C-6 NOTE-1
      Implementation: 测试
      Verification: `api_rt_01_api_models_decoupled_from_runtime_types`
      Gate: EXTERNAL-API-RT-01 Unit 层

## 5. 真机与回归
- [ ] gate 段追加 API 资源快照打印（resource 五件套 + projection snapshot_kind）+ 全门禁回归
      Contract: 0.7 全阶段最高红线（Observation≠Configuration）
      Implementation: main.rs gate 段
      Verification: 盒上 VBMF_SESSION_LIFECYCLE=1 真机跑
      Gate: EXTERNAL-API-RT-01 Hardware 层 + SESSION/RESOURCE/IDEMPOTENCY/ERROR-MODEL/EVENT-PROJECTION-RT-01 回归
- [ ] 五套 feature 编译不回退 + 盒上全矩阵
      Contract: CI 七 checks 口径 + 0.7A std-only 纪律
      Verification: p07_verify.sh 全绿
      Gate: PR required checks

## 6. 文档与收尾
- [ ] Phase Map 0.7C-7 行 COMPLETE；0.7C 下一项 = Transport 实现（std-only）；债表 Idempotency 持久化条目登记
      Contract: PHASE_IMPLEMENTATION_MAP=唯一 SoT
      Verification: 文档对账
      Gate: verify
- [ ] verify（0 CRIT/0 IMP 目标）→ archive → PR → merge → tag phase-0.7C7-external-api → 删分支
```
