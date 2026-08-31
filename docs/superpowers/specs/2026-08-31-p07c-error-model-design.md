---
comet_change: p07c-error-model
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-31-p07c-error-model
status: final
---

# Design Doc — p07c-error-model（Phase 0.7C-5: Error Model Foundation）

> open design.md §1-§6 实现级细化。锚点：终审 0.7C-4 Gate（**第一道红线：`CommandStatus ≠ IdempotentDispatch ≠ ErrorClassification`，禁万能 CommandResult**）+ 0.7C-3 Gate §10。

## 1. `src/error_model.rs` — 类型与纯函数

```rust
/// 错误分类平面 — 失败归因（独立 enum, 零字段单元变体, 不嵌套任何其他平面数据）。
/// 第一道红线: CommandStatus ≠ IdempotentDispatch ≠ ErrorClassification。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClassification {
    /// 形状/契约拒绝 (validate) — 调用方动作: 修改请求。
    Rejected,
    /// command_id 被不同 payload 占用 (D9-B) — 修正 id 或 payload。
    Conflict,
    /// 换时机重试同请求有意义 (preflight FAIL/资源冲突/租约占用或过期/pipeline/backend)。
    RetryableFailure,
    /// 重试无意义 (状态机拒绝/目标不存在/记账错误)。
    PermanentFailure,
    /// 无法归因 (claimant panic 兜底) — 不臆造 (D6 三态 Unknown 先例)。
    Unknown,
}

use crate::session::SessionError;

/// 纯函数: SessionError → 分类。**match 无通配臂** — SessionError 新增变体时
/// 编译失败, 强制逐臂架构评审 (与"新命令=架构评审事件"同构)。
pub fn classify_session_error(err: &SessionError) -> ErrorClassification {
    match err {
        SessionError::PreflightFailed(_) => RetryableFailure,  // 预检=现实vs请求; 现实随时间变 (Unsupported 子情形注释记录, 粒度保守)
        SessionError::ResourceConflict(_) => RetryableFailure, // 释放后可重试
        SessionError::ResourceState(_) => PermanentFailure,    // 状态机拒绝, 重试同序列仍错
        SessionError::Lease(LeaseError::AlreadyLeased(_)) => RetryableFailure,
        SessionError::Lease(LeaseError::Expired) => RetryableFailure,
        SessionError::Lease(LeaseError::NotFound(_)) => PermanentFailure, // 记账错误
        SessionError::UnknownSession(_) => PermanentFailure,   // ghost; 真机探针实证
        SessionError::InvalidTransition(_) => PermanentFailure,
        SessionError::Pipeline(_) => RetryableFailure,         // Supervisor 领地, 0.6 起有 recover
        SessionError::BackendUnavailable(_) => RetryableFailure,
    }
}
```

## 2. outcome 接线（决策 D-1：错误边界处分类）

```rust
// command.rs (语义冻结零触碰; serde 演进=本 change 评审事件)
pub struct CommandOutcome {
    pub command_id: CommandId,
    pub kind: CommandKind,
    pub status: CommandStatus,
    /// 失败归因 (独立分类平面, 嵌入非合并): Failed⇒Some(Retryable|Permanent|Unknown),
    /// Rejected⇒Some(Rejected), Executed/Accepted⇒None (不变量测试锁定)。
    pub classification: Option<ErrorClassification>,
}
```

- dispatch `Rejected` 分支 → `Some(Rejected)`；三臂 `Err(e)` → `Some(classify_session_error(&e))`；`Ok` → `None`。
- idempotency.rs panic 兜底 → `Some(Unknown)`。
- 重放语义：`Replayed(outcome)` 重放 claimant 的 outcome **含 classification**——同一命令重放同一归因（D9-D 逐字节重放天然涵盖）。
- 幂等 `Conflict` 出口**不改**（0.7C-4 词表零改动）；Conflict 分类经由 outcome 之外的表达已存在（`IdempotentDispatch::Conflict`），如未来需要统一视图由 External API 阶段对账。

## 3. 白盒与测试（err_model_rt_01_*）

1. `vocabulary_snapshot` — 五变体 serde `"rejected"/"conflict"/"retryable_failure"/"permanent_failure"/"unknown"` 快照；序列化 JSON 仅分类标签（零字段单元变体——禁嵌套平面数据）。
2. `classify_matrix_closed_mapping` — design.md §4 详表 10 case 逐臂断言。
3. `outcome_invariant` — Failed⇒Some(非Rejected/Conflict) / Rejected⇒Some(Rejected) / Executed⇒None。
4. `three_plane_separation` — CommandStatus 4 变体 + IdempotentDispatch 4 出口快照零改动回归；ErrorClassification allowlist `[classify_session_error]`（禁 get_/list_/execute 动词）；禁 From 互转。
5. `dispatch_failure_classification` — Simulation：validate 拒绝→Rejected / ghost stop→PermanentFailure(UnknownSession 臂) / 正常链→None / replay outcome 含原 classification。
6. 真机 ERROR-MODEL-RT-01 — gate 段逐步打印 `classification=?`；新增 ghost 探针步输出 `classification=permanent_failure`；回归 SESSION/RESOURCE/IDEMPOTENCY-RT-01 + COMMAND-CONTRACT-RT-01。

## 4. 触碰面清单（防 scope 蔓延）

`error_model.rs`（新）/ `command.rs`（outcome 字段+Err 分支+快照测试更新）/ `idempotency.rs`（panic 兜底+构造补字段）/ `main.rs`（mod+gate 段）。**SessionManager/SessionError/SessionManager API 零改动**（分类是纯函数投影）；runtime_query.rs 零改动。

## 5. 不做

Retry 执行器 / HTTP 状态码映射（External API）/ 错误事件投影（0.7D, D8 同期）/ InProgress·AlreadyApplied·Duplicate 词表项（design §2 理由）/ 万能 CommandResult / SessionError 类型改动 / D8 EventSink。
