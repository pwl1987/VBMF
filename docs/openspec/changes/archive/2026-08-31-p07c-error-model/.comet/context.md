# Comet Design Handoff

- Change: p07c-error-model
- Phase: design
- Mode: compact
- Context hash: 50a35478c090abb9b239b5bf671e5e4a5cb3eb26005813d0d9e033609a33731a

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07c-error-model/proposal.md

- Source: docs/openspec/changes/p07c-error-model/proposal.md
- Lines: 1-27
- SHA256: 3b477b16f064258b0475ca2ab32fadb7e8cccb51d5a3a46d2cae0ad01ca9e68d

```md
# Change: Phase 0.7C-5 — p07c-error-model（Error Model Foundation：失败归因分类，非万能 CommandResult）

## Why

0.7C-4 交付 Idempotency Foundation（第四次 Merge Gate PASS，master=317d99d）；Phase Map §3 下一项 = **Error Model**。终审（2026-08-31）已把三平面事实摆清：`CommandStatus`（执行状态）与 `IdempotentDispatch`（幂等裁决）之外，失败细分（Retryable vs Permanent）尚无归宿。**0.7C-5 第一道架构红线（终审冻结）：`CommandStatus ≠ IdempotentDispatch ≠ ErrorClassification`——绝不把三维度合成"万能 CommandResult"**。Error Model 只补第三个独立平面：失败归因分类，并接线到命令面（有消费点才算实现——接线纪律）。

## What Changes

- **`src/error_model.rs`（新，分类平面）**：
  - **封闭词表 `ErrorClassification`（五变体）**：`Rejected`（形状/契约拒绝——改请求后重发）/ `Conflict`（幂等 ID 复用——修正 id 或 payload）/ `RetryableFailure`（换时机重试同请求有意义）/ `PermanentFailure`（重试无意义）/ `Unknown`（无法归因，不臆造——D6 三态先例）。词表快照测试锁定；**不纳入** InProgress（同步 dispatch 无此终态出口，属未来 async 查询面）/ AlreadyApplied（无对应场景）/ Duplicate（= Replayed 已由幂等平面表达）——理由显式记录于 design.md §2。
  - **`classify_session_error(&SessionError) -> ErrorClassification` 纯函数**：九变体→五分类**封闭映射**（match 无通配臂——新增 SessionError 变体时编译失败，强制架构评审）。映射详表（逐臂测试锁定）：PreflightFailed→Retryable / ResourceConflict→Retryable / ResourceState→Permanent / Lease(AlreadyLeased|Expired)→Retryable / Lease(NotFound)→Permanent / UnknownSession→Permanent / InvalidTransition→Permanent / Pipeline→Retryable / BackendUnavailable→Retryable。
  - **CommandOutcome 增 `classification: Option<ErrorClassification>`（嵌套独立 enum，非合并）**：`Failed ⇒ Some(Retryable|Permanent|Unknown)`、`Rejected ⇒ Some(Rejected)`、`Executed/Accepted ⇒ None`（不变量测试锁定）。这是分类能**在错误仍为类型态的错误边界处**（dispatch 三臂的 `Err(e)` 分支）产生的唯一干净路径——事后从 `detail` 字符串恢复分类 = 字符串匹配 = 脆弱且违背纪律（design.md §3 记录选项对勘）。0.7C-3 冻结的**语义**（四态封闭/零执行字段/不可执行性）零触碰；serde JSON 演进经本 change 架构评审（词表快照同步更新）。
  - **panic 兜底对齐**：idempotency.rs claimant panic 兜底的 Failed → `classification: Some(Unknown)`（诚实不臆造）。
  - **三平面分离白盒**：ErrorClassification 零字段单元变体（serde 只含分类标签，不嵌套其他平面数据）；CommandStatus/IdempotentDispatch 词表快照零改动回归；禁 From 互转。
- **门禁 ERROR-MODEL-RT-01（三层）**：Unit（词表快照/classify 封闭映射矩阵/outcome 分类不变量）；Simulation（dispatch 全失败路径分类正确：Rejected/ghost-Permanent 等）；Hardware（真机 SESSION_LIFECYCLE 幂等段追加 classification 输出 + ghost stop 探针实证 `PermanentFailure` + 正常链路 `classification=None`）。
- **文档搭车（终审 §11 措辞裁定）**：债表 D9 行收紧为 "D9-A~E **Foundation**: CLOSED（进程内）/ External API·持久化·跨重启语义 **deferred** to External API stage"。
- **CI**：测试并入现有矩阵（不新增 required check）。

## Capabilities

（`skip_specs: true`——SoT 为终审 0.7C-4 Gate（三平面分离红线）+ 0.7C-3 Gate §10（Error Model 范围）+ PHASE_IMPLEMENTATION_MAP §3。）

## Impact

- 编译：五套 feature 不回退；error_model.rs 零 vendor 依赖、零 runtime_query 引用。
- 受影响：新 `error_model.rs`；`command.rs`（outcome 字段 + Err 分支 classify + 快照测试更新）；`idempotency.rs`（panic 兜底 classification + outcome 构造补字段）；`main.rs`（mod + gate 段 classification 输出 + ghost 探针）；Phase Map（0.7C-5 行）；债表（D9 措辞收紧）。SessionManager/SessionError 本体**零改动**（分类是纯函数投影）。
- **明确不做**：Retry 执行器；HTTP/gRPC 状态码映射（External API 阶段）；错误事件/Event Projection（0.7D，D8 同期）；CommandStatus 与 IdempotentDispatch 词表改动；InProgress/AlreadyApplied/Duplicate 词表项；**万能 CommandResult**；EventSink 解耦（D8）；SessionError 类型改动；不做纯清债（D9 措辞是下一 change 文档对账搭车项）。

```

## docs/openspec/changes/p07c-error-model/design.md

- Source: docs/openspec/changes/p07c-error-model/design.md
- Lines: 1-87
- SHA256: c7d5aad4ad3a26299258dfd4acf1d9aeca13bc68be264da7ba5f58f81574afb9

[TRUNCATED]

```md
# Design: Phase 0.7C-5 — p07c-error-model

## 0. 红线与裁定落点

| 终审裁定 | 设计落点 |
|---|---|
| **第一道红线：`CommandStatus ≠ IdempotentDispatch ≠ ErrorClassification`，禁万能 CommandResult** | §1 三平面正交表；ErrorClassification 零字段单元变体（不嵌套其他平面） |
| Error classification 维度：Rejected / Conflict / InProgress / Retryable / Permanent | §2 词表五项 + 不纳入项的显式理由 |
| 错误分类须有真实消费点（接线纪律：只写 helper 不接线=未实现） | §3 classification 接线到 CommandOutcome（错误边界处产生） |
| "重复命令是 Rejected/Executed/新状态"的疑问（0.7C-3 Gate §10） | 0.7C-4 已答：Replayed（幂等平面）；本 change 只归因**失败**，不重表达成功 |
| D9 措辞收紧（0.7C-4 Gate §11） | §5 债表搭车任务（非纯清债——随本 change 文档对账） |

## 1. 三平面正交（红线守护结构）

```
CommandStatus        (0.7C-3, 零改动)   命令生命周期:  Accepted / Rejected / Executed / Failed
IdempotentDispatch   (0.7C-4, 零改动)   本请求裁决:    Executed / Replayed / Conflict / Rejected
ErrorClassification  (0.7C-5, 新)       失败归因:      Rejected / Conflict / RetryableFailure
                                                      / PermanentFailure / Unknown
```

- 三个独立 enum，互不 impl `From`，互不嵌套（ErrorClassification 全部为**零字段单元变体**——serde 断言序列化只含分类标签，无任何其他平面数据）。
- `CommandOutcome.classification: Option<ErrorClassification>` 是**嵌入**而非**合并**：status 语义不变；分类是失败归因的附加视图。
- 幂等层 `Replayed` 重放的 outcome 含 claimant 时的 classification——**同一命令重放同一归因**（与 D9-D 逐字节重放语义一致，天然成立）。

## 2. 词表（封闭五项）与不纳入理由

| 变体 | 语义 | 调用方动作 |
|---|---|---|
| `Rejected` | 形状/契约不满足（validate 拒绝） | 修改请求后重发 |
| `Conflict` | command_id 被不同 payload 占用（D9-B） | 修正 id 或 payload |
| `RetryableFailure` | 换时机重试同请求有意义 | 稍后重试（可结合 Query 观察） |
| `PermanentFailure` | 重试无意义（时序/状态机错误、目标不存在） | 不要再试（查 Query 修正认知） |
| `Unknown` | 无法归因（claimant panic 兜底） | 不臆造；走运维/日志（D6 三态 Unknown 先例） |

**不纳入（显式理由，防词表膨胀成万能分类）**：
- `InProgress`——0.7C-4 dispatch 是同步等待语义（Condvar 等到 Completed 才返回），当前**不存在** InProgress 终态出口；属未来 async/查询面（External API 阶段若有异步提交再过评审引入）。
- `AlreadyApplied`——无对应场景（create 幂等键命中已存在资源的场景不存在：Start 的 replay 是 D9-D 结果重放，不是效果重算）。
- `Duplicate`——不是错误；= `IdempotentDispatch::Replayed`，已由幂等平面表达。分类平面**只归因失败**，绝不重复表达成功/幂等成功。

## 3. 分类产生点：错误边界（决策 D-1 对勘）

**问题**：Retryable vs Permanent 需要结构化错误；而 0.7C-3 的 `CommandOutcome.detail` 是 `Option<String>`（类型擦除）。

- 选项 A（**选定**）：outcome 增 `classification` 字段，在 dispatch 三臂的 `Err(e: SessionError)` 分支处分类——**错误仍为类型态**，纯函数投影，零字符串匹配。
- 选项 B（否决）：事后 `classify(outcome)` 从 detail 字符串恢复——脆弱、vendor-neutral 假象、违背"文档语义>实现行为"。
- 选项 C（否决）：只提供 `classify_session_error` 纯函数库不接线——违背接线纪律（无消费点=未实现）。
- 选项 D（否决）：`IdempotentDispatch` 增分类出口——改 0.7C-4 词表平面，红线。

选项 A 触碰 command.rs 的**类型定义**（加字段），不触碰其**冻结语义**（四态封闭/零执行字段/不可执行性/validate 纯函数/dispatch 薄映射）——语义冻结 vs serde JSON 演进分开对待，本 change 即架构评审事件（两平面词表快照测试同步更新为含 classification 的断言）。

## 4. `classify_session_error` 封闭映射（九臂→五类，无通配臂）

| SessionError 变体 | 分类 | 理由（逐臂记录，测试锁定） |
|---|---|---|
| `PreflightFailed(_)` | RetryableFailure | 预检=现实 vs 请求判定（judge-only）；端口无信号/资源暂占等随时间变化。注：能力 Unsupported 子情形本质 Permanent，但分类粒度按报告整体保守判 Retryable——细化属演进项，不臆造 |
| `ResourceConflict(_)` | RetryableFailure | 资源被他会话占用，释放后可重试 |
| `ResourceState(_)` | PermanentFailure | 资源状态机拒绝（时序错误），重试同序列仍错 |
| `Lease(AlreadyLeased(_))` | RetryableFailure | 设备被占，租约释放后可再 acquire |
| `Lease(Expired)` | RetryableFailure | TTL 到期可重新 acquire |
| `Lease(NotFound(_))` | PermanentFailure | 释放不存在的租约=调用方记账错误 |
| `UnknownSession(_)` | PermanentFailure | 目标会话不存在（ghost）；真机探针实证项 |
| `InvalidTransition(_)` | PermanentFailure | 会话状态机白名单拒绝（如 close Released） |
| `Pipeline(_)` | RetryableFailure | 管线执行期错误（Supervisor 领地，0.6 起有 recover 语义） |
| `BackendUnavailable(_)` | RetryableFailure | 后端暂时不可用 |

**panic 兜底**（idempotency.rs claimant catch_unwind）→ `Unknown`（没有理由臆造 Retryable 或 Permanent）。

封闭性保障：match **无 `_` 通配臂**——SessionError 新增变体时编译失败，强制逐臂评审（与"新命令=架构评审事件"同构）。

## 5. 不变量与接线面

- `CommandOutcome` 不变量（测试锁定）：`Failed ⇒ Some(RetryableFailure|PermanentFailure|Unknown)`；`Rejected ⇒ Some(Rejected)`；`Executed/Accepted ⇒ None`。
- 接线点：①command.rs dispatch 三臂 Err 分支；②idempotency.rs panic 兜底；③main.rs gate 段输出 `classification=?` + 新增 ghost 探针步（真机实证 PermanentFailure）。
- 白盒：ErrorClassification allowlist `[classify_session_error]`；无 get_/list_/execute 动词；零 runtime_query 引用。
- 债表搭车（终审 §11）：D9 行改写为 "D9-A~E Foundation: CLOSED（进程内 Command Idempotency）/ External API·持久化·跨重启语义: deferred to External API stage"。

## 6. 测试矩阵（err-model-rt-01_*，feature=mock）

| 测试 | 覆盖 |

```

Full source: docs/openspec/changes/p07c-error-model/design.md

## docs/openspec/changes/p07c-error-model/tasks.md

- Source: docs/openspec/changes/p07c-error-model/tasks.md
- Lines: 1-56
- SHA256: 2aff173c56e68ac47829b7e2c5008775309d1a7fab2cd653d67052929d2a79c7

```md
# Tasks: Phase 0.7C-5 — p07c-error-model

四栏纪律：`Contract: 已有(引用冻结文档节号) / Implementation / Verification / Gate`。

## 1. 分类平面（design.md §1/§2）
- [ ] ErrorClassification 封闭词表五项 + 不纳入项理由（InProgress/AlreadyApplied/Duplicate）
      Contract: 终审 0.7C-4 Gate（第一道红线三平面分离；维度示例）+ 0.7C-3 Gate §10
      Implementation: `error_model.rs` 词表 + design.md §2
      Verification: `err_model_rt_01_vocabulary_snapshot`
      Gate: ERROR-MODEL-RT-01 Unit 层
- [ ] 三平面正交结构：零字段单元变体 / 禁 From 互转 / CommandStatus+IdempotentDispatch 词表零改动回归
      Contract: 终审 0.7C-4 Gate（禁万能 CommandResult）
      Implementation: 词表定义 + 白盒断言
      Verification: `err_model_rt_01_three_plane_separation`
      Gate: ERROR-MODEL-RT-01 Unit 层

## 2. 封闭映射（design.md §4）
- [ ] classify_session_error 九臂→五类封闭映射（match 无通配臂，编译级防漏）
      Contract: 接线纪律（新增变体强制评审）+ D6 Unknown 先例
      Implementation: 纯函数逐臂映射
      Verification: `err_model_rt_01_classify_matrix_closed_mapping`（10 case）
      Gate: ERROR-MODEL-RT-01 Unit 层

## 3. outcome 接线（design.md §3/§5）
- [ ] CommandOutcome 增 classification: Option<ErrorClassification>（错误边界处产生；决策 D-1）
      Contract: 终审 0.7C-3 Gate §10（Idempotency+Error Model 联合设计结果形态）
      Implementation: command.rs dispatch Err 分支 + idempotency.rs panic 兜底(Unknown)
      Verification: `err_model_rt_01_outcome_invariant`（三不变量）
      Gate: ERROR-MODEL-RT-01 Unit 层
- [ ] Simulation：dispatch 失败路径分类正确 + replay 重放含原 classification
      Contract: D9-D 逐字节重放语义延续
      Implementation: 测试
      Verification: `err_model_rt_01_dispatch_failure_classification`
      Gate: ERROR-MODEL-RT-01 Simulation 层

## 4. 真机（design.md §6）
- [ ] main.rs gate 段 classification 输出 + ghost 探针步（PermanentFailure 实证）+ 回归
      Contract: PHASE_IMPLEMENTATION_MAP §3（Error Model 项）
      Implementation: SESSION_LIFECYCLE 段升级
      Verification: 盒上 VBMF_SESSION_LIFECYCLE=1 真机跑
      Gate: ERROR-MODEL-RT-01 Hardware 层 + SESSION/RESOURCE/IDEMPOTENCY-RT-01 回归
- [ ] 五套 feature 编译不回退 + 盒上全矩阵
      Contract: CI 七 checks 口径
      Verification: p07_verify.sh 全绿
      Gate: PR required checks

## 5. 文档与收尾
- [ ] 债表 D9 措辞收紧：Foundation CLOSED（进程内）/ External·持久化语义 deferred
      Contract: 终审 0.7C-4 Gate §11
      Verification: PHASE_0_7A_POST_MERGE_DEBT.md 对账
      Gate: verify
- [ ] Phase Map：0.7C-5 行 COMPLETE；0.7C 下一项 = Event Projection → External API
      Contract: PHASE_IMPLEMENTATION_MAP=唯一 SoT
      Verification: 文档对账
      Gate: verify
- [ ] verify（0 CRIT/0 IMP 目标）→ archive → PR → merge → tag phase-0.7C5-error-model → 删分支

```
