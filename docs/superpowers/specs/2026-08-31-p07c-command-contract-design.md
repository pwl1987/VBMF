---
comet_change: p07c-command-contract
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-31-p07c-command-contract
status: final
---

# Design Doc — p07c-command-contract（Phase 0.7C-3: Command Contract Foundation）

> open design.md D1-D8 实现级细化。锚点：PHASE_IMPLEMENTATION_MAP §3；终审执行令（**第一红线：Command 不可执行性**；`Query = What is true now? Command = What do I request to change?`）。

## 1. `src/command.rs` — 类型（编排层，零 vendor 依赖）

```rust
/// 命令词表 — **封闭枚举**（三命令; 新命令须过架构评审并显式更新词表快照测试）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandKind { StartSession, StopSession, ReleaseSession }

pub struct CommandId(pub Uuid);   // 幂等键占位: 携带不实现 (D9 幂等语义属下一 change)

/// 命令目标 — 仅 canonical 类型 (GraphRuntimeIntent / SessionId)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum CommandTarget {
    Session { intent: crate::graph_intent::GraphRuntimeIntent },  // Start 用 (canonical intent)
    SessionById { session_id: crate::session::SessionId },         // Stop/Release 用
}

/// 命令信封 — **零执行字段** (无 gst/pipeline/device_number/backend/handle)。
pub struct CommandEnvelope {
    pub command_id: CommandId,
    pub kind: CommandKind,
    pub target: CommandTarget,
    pub issued_at_ms: u64,
    pub requested_by: String,      // opaque 请求方标签 (非身份模型)
}

/// 命令生命周期结果 (非 Runtime 状态投影)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus { Accepted, Rejected, Executed, Failed }

pub struct CommandOutcome { pub command_id: CommandId, pub kind: CommandKind,
    pub status: CommandStatus, pub detail: Option<String> }

pub struct CommandRejection { pub code: String, pub detail: String }
```

## 2. validate（纯函数，validation/execution 分离）

```rust
pub fn validate(env: &CommandEnvelope) -> Result<(), CommandRejection>
```
形状规则（**不触 Runtime**、不依赖 runtime_query）：
- `requested_by` 非空（`"empty_requester"`）；
- kind/target 形状匹配：`StartSession ⇒ Session{..}`、`StopSession|ReleaseSession ⇒ SessionById{..}`（`"kind_target_mismatch"`）；
- `Session{intent}` 的 `intent.devices` 非空（`"empty_intent"`）；
- `SessionById` 的 `session_id.0 != Uuid::nil()`（`"nil_session_id"`）。

## 3. dispatch（薄映射 boundary，非 Executor）

```rust
pub fn dispatch(mgr: &crate::session::SessionManager, env: &CommandEnvelope) -> CommandOutcome
```
- 先 `validate` → `Rejected{code}`（不触 runtime）；
- `match env.kind` 三臂，各调 SessionManager **公共 API**：
  - `StartSession`：`mgr.create(intent)` → `mgr.start(&sid)`（失败回滚由 SessionManager 既有 hardening 保证；command 层报 `Failed`）；
  - `StopSession`：`mgr.stop(&sid)`；`ReleaseSession`：`mgr.close(&sid)`；
- 成功 → `Executed`；SessionError → `Failed(reason)`。
- **无**命令循环/插件/注册机制/命令总线（终审禁令——match 三臂即全部）。

## 4. 红线守护（三重 + 分离）

| 守护 | 实现 |
|---|---|
| ①类型层 | envelope/target/outcome 字段仅 canonical 类型（Uuid/String/GraphRuntimeIntent/SessionId/枚举） |
| ②serde 反向断言 | envelope JSON 禁字样：`gst/pipeline/device_number/backend/handle/ffmpeg/alsa/kafka/nats`（测试） |
| ③公开面 | allowlist `[validate, dispatch]`（+类型构造）；denylist 动词：`execute_pipeline/configure_backend/run_backend/build_gst/emit/send/publish`（测试断言 allowlist 恒等于该二集） |
| Query/Command 分离 | command.rs 零 `use crate::runtime_query`；runtime_query.rs 零 `use crate::command`（编译结构保证；测试断言 allowlist 无对方方法） |

## 5. COMMAND-CONTRACT-RT-01（三层）

| 层 | 测试 |
|----|------|
| Unit | 词表快照（三 kind serde 字符串）；不可执行性 serde 断言；allowlist 自检 + denylist；validate 四拒绝路径 + 通过路径 |
| Simulation | mock 世界：Start（envelope→Executed + 会话 Running 经 RuntimeQuery 可见）→ Stop（Executed + Released）→ Release（Executed + 会话不可查询）；Rejected（nil_session_id / kind_target_mismatch）与 Failed（Stop 未存在会话）路径 |
| Hardware | SESSION_LIFECYCLE 追加 command 段：直接路径跑完后，再以 envelope 驱动 Start→observe→Stop→Release，输出各步 CommandStatus（与直接路径等价性 = 会话生命周期门禁保持 ALL PASS） |

## 6. 实施顺序

command.rs（类型→validate→dispatch）→ Unit 测试 → Simulation → main.rs mod+gate 段 → 盒上矩阵 → 真机 → Phase Map。
