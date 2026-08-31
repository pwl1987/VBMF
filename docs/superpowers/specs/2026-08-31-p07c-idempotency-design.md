---
comet_change: p07c-idempotency
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-31-p07c-idempotency
status: final
---

# Design Doc — p07c-idempotency（Phase 0.7C-4: Idempotency Foundation）

> open design.md §1-§8 实现级细化。锚点：终审 0.7C-3 Gate §8-§11（**重点不是实现幂等存储，而是先冻结"什么叫同一个命令"的语义与并发裁决规则**；D9-A~E 具体化）。

## 1. `src/idempotency.rs` — 类型与纯函数（D9-A）

```rust
use crate::command::{CommandEnvelope, CommandId, CommandOutcome, CommandRejection, dispatch};
use crate::session::SessionManager;

/// canonical 命令指纹 — "什么叫同一个命令"的冻结语义 (D9-A)。
/// 组成 = kind 判别式 (snake_case 冻结词表) + CommandTarget 的 canonical serde JSON。
/// 不参与: command_id (查表键本身) / issued_at_ms (投递时刻元数据, 重试会变)
///         / requested_by (审计标签, 非负载语义)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandFingerprint(pub String);

/// 纯函数 (确定性): envelope → fingerprint。
pub fn fingerprint(env: &CommandEnvelope) -> CommandFingerprint {
    CommandFingerprint(format!(
        "{}|{}",
        serde_json::to_string(&env.kind).expect("kind serde"),
        serde_json::to_string(&env.target).expect("target serde"),
    ))
}

/// 幂等裁决平面 — 本请求 dispatch 的结果 (与 CommandStatus 执行状态平面分层,
/// 终审 §10: 细分状态 InProgress/AlreadyApplied/Retryable 属 0.7C-5 Error Model)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum IdempotentDispatch {
    /// 本请求是 claimant, 已执行一次 (outcome.status 可为 Executed 或 Failed)。
    Executed(CommandOutcome),
    /// 重复命令 — 重放 claimant 的原 outcome (逐字节相等; Failed 同样 replay)。
    Replayed(CommandOutcome),
    /// 同 command_id 异 payload — ID 复用/语义碰撞: 绝不 replay, 绝不执行。
    Conflict {
        command_id: CommandId,
        expected: CommandFingerprint, // 表内已占用 (先到 payload)
        actual: CommandFingerprint,   // 本次请求 (后到异值 payload)
    },
    /// 形状校验拒绝 (未触 Runtime, 未占 id — 不可执行的请求未进入系统)。
    Rejected(CommandRejection),
}
```

确定性依据：serde 对 struct/enum 按声明序序列化；`Uuid`/`String` 无哈希随机性；`CommandTarget` 是 internally tagged enum——同一值恒产生同一 JSON。

## 2. 原子 claim 结构（D9-C）

```rust
enum RecordState {
    InFlight,                    // claimant 执行中 (锁外)
    Completed(CommandOutcome),   // 终态 — 失败也是结果 (D9-D)
}
struct Record { fingerprint: CommandFingerprint, state: RecordState }

pub struct CommandIdempotency {
    mgr: Arc<SessionManager>,
    records: Mutex<HashMap<CommandId, Record>>,
    completed: Condvar,   // notify_all 于 Completed 落表时
}
```

`dispatch(&self, env: &CommandEnvelope) -> IdempotentDispatch`（顺序即契约，见 open design.md §2 五步）：

```rust
pub fn dispatch(&self, env: &CommandEnvelope) -> IdempotentDispatch {
    // 1. 形状校验 (纯函数, 无锁): 拒绝不写表不占 id。
    if let Err(rej) = crate::command::validate(env) {
        return IdempotentDispatch::Rejected(rej);
    }
    let fp = fingerprint(env);
    // 2. 锁内原子 claim (check-and-insert 单临界区 — 非 check-then-act)。
    let mut guard = self.lock();
    match guard.get(&env.command_id) {
        None => {
            guard.insert(env.command_id, Record { fingerprint: fp, state: RecordState::InFlight });
            drop(guard);
            // 3. claimant 锁外独占执行; panic 兜底落终态 Failed (防等待者死等)。
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::command::dispatch(&self.mgr, env)
            }))
            .unwrap_or_else(|_| CommandOutcome {
                command_id: env.command_id, kind: env.kind,
                status: CommandStatus::Failed,
                detail: Some("claimant panicked during execution".into()),
            });
            // 4. 终态落表 + 唤醒等待者。
            let mut guard = self.lock();
            if let Some(rec) = guard.get_mut(&env.command_id) {
                rec.state = RecordState::Completed(outcome.clone());
            }
            drop(guard);
            self.completed.notify_all();
            IdempotentDispatch::Executed(outcome)
        }
        Some(rec) if rec.fingerprint == fp => {
            // 5a. 同一命令重复投递 → 等待/读取终态后 replay。
            loop {
                let rec = self.lock().get(&env.command_id).cloned()… // 状态拷贝后释放锁再判断
                match rec.state {
                    Completed(outcome) => return IdempotentDispatch::Replayed(outcome),
                    InFlight => { guard = self.completed.wait(guard); } // 循环重查
                }
            }
        }
        Some(rec) => {
            // 5b. 同 id 异 payload → Conflict (无论 InFlight/Completed; 记录零改写)。
            IdempotentDispatch::Conflict {
                command_id: env.command_id,
                expected: rec.fingerprint.clone(),
                actual: fp,
            }
        }
    }
}
```

（实现时 Record 需 `Clone` 或以值拷贝状态；等待循环以持 guard 的 `Condvar::wait` 为准——锁释放/重取由 `wait` 语义保证。）

要点：claim 原子性 = 锁内 match(None→insert)；执行不持 records 锁（无关命令不被阻塞）；poison 恢复 `unwrap_or_else(|e| e.into_inner())`；Conflict 分支不等待、不改写。

## 3. Replay / Conflict 契约（D9-B/D）

- `Replayed(outcome)` 与 claimant 的 `Executed(outcome)` 中 outcome **逐字节相等**（`assert_eq!` 级锁定）。
- Failed replay：ghost stop 首次 `Failed`，同 envelope 重发 `Replayed(Failed 同值)`——重试 = 调用方发**新 command_id**（Error Model 阶段的分类）。
- 重复≠重执行：stop `Executed` 后同 envelope 重发 → `Replayed(Executed)`，非对 Released 会话再 stop 的 `Failed`。
- Conflict：本次 payload 零执行（会话表不变）；原记录零改写（此后同原 fingerprint 重发仍 `Replayed`）。

## 4. 测试（`idem_rt_01_*`，feature=mock；`world()` 夹具复用 command.rs 模式）

8 项 Simulation/Unit + 真机 IDEMPOTENCY-RT-01（矩阵见 open design.md §8）：
1. `fingerprint_semantics` — D9-A 冻结（含 issued_at_ms/requested_by 变化不变）。
2. `vocabulary_snapshot` — 四出口 serde 快照（防静默加出口）。
3. `non_executability_surface` — allowlist `[fingerprint, dispatch]`；banned 词表沿用；无 get_/list_。
4. `validate_rejected_does_not_claim` — Rejected 后同 id valid 仍 Executed。
5. `execute_once_and_replay` — Executed→Replayed（outcome 相等/会话数不变/Failed 也 replay）。
6. `stop_replay_not_reexecute` — 重复 stop 重放 Executed。
7. `payload_conflict` — 同 id 异 intent → Conflict；原记录保留可继续 replay。
8. `concurrent_duplicate_single_execution` — 8 线程 barrier：Executed×1 + Replayed×7 + 会话数 1。

## 5. main.rs 真机段（IDEMPOTENCY-RT-01）

SESSION_LIFECYCLE gate：`CommandIdempotency::new(Arc::clone(&mgr))` 包住原 command 段——
Start envelope → `Executed`（打印 verdict+status）→ 同 envelope 重发 → `Replayed`（会话数仍 1）→ 同 command_id 换 intent → `Conflict`（会话数仍 1）→ observe 10s running=true → Stop/Release（幂等路径，各自新 command_id）→ `Executed`。每步打印 `verdict=/status=/sessions=`。回归：SESSION-RT-01 / RESOURCE-RT-01 / RUNTIME-STATE-RT-01 不动。

## 6. 边界

进程内内存表（InMemoryLeaseManager 同决策级别）；不做持久化/TTL/容量驱逐/时间窗（design.md §7）；command.rs 0.7C-3 冻结面零改动；零 runtime_query 引用；不新增命令词表。
