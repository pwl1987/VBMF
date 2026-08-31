# Comet Design Handoff

- Change: p07c-idempotency
- Phase: design
- Mode: compact
- Context hash: f00290d8775f70d283020db4a291fdc27ec4cbf9b5403945d8375f2a09edf8a5

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07c-idempotency/proposal.md

- Source: docs/openspec/changes/p07c-idempotency/proposal.md
- Lines: 1-27
- SHA256: a1cd89bcdf43c3d1127ea8b05c7743eb338c6ba5d9f831dec9a341d1af2f7e85

```md
# Change: Phase 0.7C-4 — p07c-idempotency（Idempotency Foundation：什么叫"同一个命令"）

## Why

0.7C-3 交付 Command Contract（envelope 携带 `command_id` 幂等键**占位**）；终审（2026-08-31，master=7de969b）批准进入 **Idempotency Foundation**，并给出核心裁定：**重点不是"实现一个幂等存储"，而是先冻结"什么叫同一个命令"的语义，以及并发重复请求的裁决规则**。终审明确反模式：①`HashMap<CommandId, bool>` 式假关闭；②check-then-act 竞态（两个请求同时 `exists=false` → 双执行）；③同 id 不同 payload 简单返回 "already executed"（ID 复用/语义碰撞）。D9 在本 change 必须具体化为 **D9-A~E** 并全部锁死。

## What Changes

- **`src/idempotency.rs`（新，Command 平面）**——`CommandId → Canonical Command Fingerprint → Atomic Claim → Execute once → Persist Outcome → Duplicate → Replay/Conflict` 全链：
  - **D9-A command identity**：`CommandFingerprint = kind 判别式 + CommandTarget 的 canonical serde JSON`（确定性纯函数 `fingerprint(&envelope)`）。**参与**：kind + target（canonical payload）；**不参与**：`command_id`（查表键本身）、`issued_at_ms`（投递时刻元数据，重试会变，非命令语义）、`requested_by`（审计标签，非负载语义）。同 id + 同 fingerprint = 同一命令（重复投递）；同 id + 不同 fingerprint = **ID 复用/语义碰撞**。
  - **D9-B payload conflict**：`StartSession(id=X, intent=A)` 后收到 `StartSession(id=X, intent=B)` → **Conflict** 出口（携带 expected/actual fingerprint），绝不 replay、绝不执行第二个 payload；原记录原样保留（此后同 A 的重复仍可 replay）。
  - **D9-C atomic claim**：单锁 `Mutex<HashMap<CommandId, Record>>` + `Condvar`——claim（check-and-insert）在锁内**原子**完成，first claimant 在锁外独占执行，完成后落终态并 `notify_all`；**非 check-then-act**。validate 拒绝不写表不占 id（不可执行的请求未进入系统）。claimant panic 防御：`catch_unwind` → 终态 Failed（防等待者死等）；Mutex poison 用 `into_inner` 恢复。
  - **D9-D result replay**：`Record: InFlight{fingerprint} → Completed{fingerprint, outcome}`；重复请求等待/读取终态后 `Replayed(原 outcome)`——**Failed 结果同样 replay**（同一命令重放同一失败；重试语义属 Error Model/调用方新 command_id，本 change 不做）。
  - **D9-E concurrent duplicate**：N 线程同时 dispatch 同 envelope——恰一次执行 + 其余 replay 原结果（线程级击穿测试锁定）。
  - **两平面分层（不吞 Error Model）**：0.7C-3 冻结的 `CommandStatus` 四态**零改动**（执行状态平面）；新增 `IdempotentDispatch` 四出口 `{ Executed, Replayed, Conflict, Rejected }`（**幂等裁决平面**）——词表快照测试防静默加出口（InProgress/AlreadyApplied/Retryable 等细分属 0.7C-5 Error Model）。
- **门禁 IDEMPOTENCY-RT-01（三层）**：Unit（fingerprint 语义冻结/词表快照/allowlist 白盒）；Simulation（execute-once + replay + failed-replay + conflict + validate-不占 id + 8 线程并发击穿 + stop 重复 replay 非 re-execute）；Hardware（真机 SESSION_LIFECYCLE 追加幂等段：Start Executed → 同 envelope 重发 Replayed → 同 id 换 intent Conflict → observe 10s → Stop/Release 幂等路径 Executed）。
- **CI**：测试并入现有矩阵（不新增 required check）。

## Capabilities

（`skip_specs: true`——SoT 为终审 0.7C-3 Gate §8-§11（D9-A~E 定义）+ PHASE_IMPLEMENTATION_MAP §3。）

## Impact

- 编译：五套 feature 不回退；idempotency.rs 零 vendor 依赖、零 runtime_query 引用（Query/Command 分离延续）。
- 受影响：新 `idempotency.rs`；`main.rs`（mod + SESSION_LIFECYCLE 幂等段升级 COMMAND-CONTRACT-RT-01 → IDEMPOTENCY-RT-01）；Phase Map（0.7C-4 行）；债表（D9 → CLOSED@0.7C-4，按 D9-A~E 逐项引用证据）。Session/Resource/Lease 语义零变更（幂等层包住 command::dispatch 薄映射，不改 0.7C-3 冻结面）。
- **明确不做**：Error Model（Rejected/Failed/Duplicate/Conflict/InProgress/Retryable 的统一错误分类——0.7C-5）；Retry；Event/Event Projection；REST/WebSocket/External API；Scheduler/Command Bus/Kafka/NATS；跨进程/跨重启持久化幂等（进程内内存表，与 InMemoryLeaseManager 同决策级别；容量驱逐策略——幂等记录不可随意驱逐否则 replay 退化成重执行——留 External API 阶段决策）；不新增命令词表。

```

## docs/openspec/changes/p07c-idempotency/design.md

- Source: docs/openspec/changes/p07c-idempotency/design.md
- Lines: 1-135
- SHA256: 4d46a983c60311f726af2e11dfa4826b74c615f0bfe91f92c83806bd6634f367

[TRUNCATED]

```md
# Design: Phase 0.7C-4 — p07c-idempotency

## 0. 冻结语义（终审 2026-08-31 裁定逐条落点）

| 终审裁定 | 设计落点 |
|---|---|
| "重点不是实现幂等存储，而是先冻结**什么叫同一个命令**" | §1 Fingerprint 语义（参与/不参与字段显式冻结 + 测试锁定） |
| 同 id + 不同 payload 绝不能简单返回 already executed | §3 Conflict 出口（独立于 Replayed，携带 expected/actual） |
| 禁 check-then-act（两请求同时 exists=false → 双执行） | §2 Atomic Claim（锁内 check-and-insert 原子；执行在锁外） |
| Atomic Claim → first claimant executes → result persisted → duplicates replay | §2 Record 状态机 + §4 replay |
| D9-A~E 具体化（防 "CLOSED 但只做了 HashMap<CommandId, bool>"） | §6 测试矩阵逐项对应 + 债表按 A~E 逐项引用证据关闭 |
| Error Model 不被顺手吞掉 | §5 两平面分层（CommandStatus 四态零改动；IdempotentDispatch 四出口） |

## 1. D9-A — Canonical Command Fingerprint（同一命令的定义）

```rust
/// canonical 命令指纹 — "什么叫同一个命令"的冻结语义 (D9-A)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandFingerprint(pub String);

/// 纯函数: envelope → fingerprint。确定性 (同输入同输出)。
pub fn fingerprint(env: &CommandEnvelope) -> CommandFingerprint;
```

组成 = `kind 的 serde 判别式（"start_session" 等 snake_case 冻结词表）` + `serde_json::to_string(&env.target)`（`CommandTarget` 为 tagged enum，字段按声明序序列化——serde struct/enum 序列化是确定性的，`Uuid`/`String` 无哈希随机性）。

**参与字段**：`kind` + `target`（canonical payload——终审原文 "command_id + command kind + canonical target/payload 的一致性"）。
**不参与字段（显式冻结）**：
- `command_id`——它是查表键本身（fingerprint 是**值**的同一性，command_id 是**请求实例**的同一性；同 id 异值 = 碰撞，见 §3）；
- `issued_at_ms`——投递时刻元数据：网络重试会重算时间戳，参与 fingerprint 会把全部重试判成冲突；
- `requested_by`——审计标签（opaque，非身份模型——0.7C-3 已冻结该注释），非负载语义。

裁决规则（冻结）：

```
同 command_id + 同 fingerprint  → 同一命令的重复投递 → Replay
同 command_id + 异 fingerprint  → ID 复用 / 语义碰撞  → Conflict（绝不 replay，绝不执行）
```

## 2. D9-C — Atomic Claim（并发安全的第一原则）

```rust
enum RecordState {
    InFlight,                          // claimant 执行中
    Completed(CommandOutcome),         // 终态（含 Failed——失败也是结果）
}
struct Record { fingerprint: CommandFingerprint, state: RecordState }

pub struct CommandIdempotency {
    mgr: Arc<SessionManager>,
    records: Mutex<HashMap<CommandId, Record>>,
    completed: Condvar,
}
```

dispatch 流程（顺序即契约）：

```
1. validate(env)            → Err: 返回 Rejected（不写表、不占 id——不可执行的请求未进入系统）
2. fp = fingerprint(env)
3. lock(records):
     a. 表无该 id            → 插入 {fp, InFlight} → 本请求是 claimant → 释放锁
     b. 有该 id 且 fp 相同:
          Completed(outcome) → 释放锁 → 返回 Replayed(outcome)
          InFlight           → condvar.wait 直到 Completed → 返回 Replayed(outcome)
     c. 有该 id 且 fp 不同    → 释放锁 → 返回 Conflict{command_id, expected(表内), actual(本次)}
                               （无论表内 InFlight/Completed——id 已被不同 payload 占用即碰撞）
4. claimant（锁外独占执行权）:
     outcome = catch_unwind(|| command::dispatch(&self.mgr, env))
                 .unwrap_or_else(|_| Failed("claimant panicked" 终态))
     lock(records) → 该 id 记录 → Completed(outcome) → notify_all
5. 返回 Executed(outcome)   （outcome.status 可为 Executed 或 Failed——执行失败也是"已执行一次"）
```

要点：
- **claim 是锁内原子 check-and-insert**——两个线程同时到达步骤 3，只有一个插入成功成为 claimant，另一个必然看到记录（Completed → replay / InFlight → 等待 replay）。不存在两个 "exists=false"。
- **执行在锁外**——claimant 执行期间不持 records 锁，无关命令的 dispatch 不被阻塞（锁只保护裁决表这个临界资源）。
- **catch_unwind + AssertUnwindSafe**——claimant 若 panic，记录仍落终态 Failed，等待者不会死等（防御性终态保障；SessionManager 各 API 返回 Result 不主动 panic，此为兜底）。
- **Mutex poison** 用 `unwrap_or_else(PoisonError::into_inner)` 恢复——记录表本身是纯数据，poison 不破坏不变量。


```

Full source: docs/openspec/changes/p07c-idempotency/design.md

## docs/openspec/changes/p07c-idempotency/tasks.md

- Source: docs/openspec/changes/p07c-idempotency/tasks.md
- Lines: 1-69
- SHA256: b7c095ad2f37f3e2ddeb702f97e574a82a26cf625213737a948d56cbff04f3e9

```md
# Tasks: Phase 0.7C-4 — p07c-idempotency

四栏纪律：`Contract: 已有(引用冻结文档节号) / Implementation / Verification / Gate`。

## 1. 语义冻结（design.md §1/§5）
- [ ] D9-A fingerprint 组成冻结：参与=kind+target；不参与=command_id/issued_at_ms/requested_by（含理由）
      Contract: 终审 0.7C-3 Gate §8（"command_id + command kind + canonical target/payload 一致性"）
      Implementation: design.md §1 + `CommandFingerprint`/`fingerprint()` 纯函数
      Verification: `idem_rt_01_fingerprint_semantics`
      Gate: IDEMPOTENCY-RT-01 Unit 层
- [ ] 两平面分层冻结：CommandStatus 四态零改动；IdempotentDispatch 四出口 {Executed, Replayed, Conflict, Rejected}
      Contract: 终审 0.7C-3 Gate §10（Error Model 不被吞——0.7C-5）
      Implementation: design.md §5
      Verification: `idem_rt_01_vocabulary_snapshot` + command.rs 词表快照零改动回归
      Gate: IDEMPOTENCY-RT-01 Unit 层

## 2. Atomic Claim 与执行（design.md §2）
- [ ] D9-C 原子 claim：锁内 check-and-insert；执行锁外；catch_unwind 终态兜底；poison 恢复
      Contract: 终审 0.7C-3 Gate §9（禁 check-then-act；Atomic Claim → first claimant executes）
      Implementation: `CommandIdempotency::dispatch` 步骤 1-5
      Verification: `idem_rt_01_validate_rejected_does_not_claim` + `idem_rt_01_concurrent_duplicate_single_execution`
      Gate: IDEMPOTENCY-RT-01 Simulation 层
- [ ] D9-E 并发裁决：8 线程 barrier 击穿——恰一次执行 + 其余 replay + 会话数 1
      Contract: 同上（§9 竞态图）
      Implementation: design.md §5
      Verification: `idem_rt_01_concurrent_duplicate_single_execution`
      Gate: IDEMPOTENCY-RT-01 Simulation 层

## 3. Replay 与 Conflict（design.md §3/§4）
- [ ] D9-D result replay：原 outcome 逐字节重放；Failed 同样 replay；重复≠重新执行（stop 案例）
      Contract: 终审 0.7C-3 Gate §9（duplicates replay result）
      Implementation: `RecordState::Completed` + `Replayed` 出口
      Verification: `idem_rt_01_execute_once_and_replay` + `idem_rt_01_stop_replay_not_reexecute`
      Gate: IDEMPOTENCY-RT-01 Simulation 层
- [ ] D9-B payload conflict：同 id 异 payload → Conflict（不执行/不改写原记录）
      Contract: 终审 0.7C-3 Gate §8（ID reuse 绝不能 already executed）
      Implementation: `Conflict{command_id, expected, actual}` 出口
      Verification: `idem_rt_01_payload_conflict`
      Gate: IDEMPOTENCY-RT-01 Simulation 层

## 4. 红线延续（design.md §6）
- [ ] 不可执行性 + Query/Command 分离 + 无万能 Executor 白盒
      Contract: 0.7C-3 三重守护 + 0.7 三红线
      Implementation: 类型仅 canonical；零 runtime_query 引用；包住 command::dispatch 不侵入
      Verification: `idem_rt_01_non_executability_surface`
      Gate: IDEMPOTENCY-RT-01 Unit 层

## 5. 接线与真机（design.md §8）
- [ ] main.rs：mod idempotency + SESSION_LIFECYCLE 幂等段（COMMAND-CONTRACT-RT-01 → IDEMPOTENCY-RT-01：Executed/Replayed/Conflict 逐步输出 + 会话数断言）
      Contract: PHASE_IMPLEMENTATION_MAP §3（Idempotency 项）
      Implementation: main.rs gate 段升级
      Verification: 盒上 `VBMF_SESSION_LIFECYCLE=1` 真机跑
      Gate: IDEMPOTENCY-RT-01 Hardware 层 + SESSION/RESOURCE-RT-01 回归
- [ ] 五套 feature 编译不回退 + 盒上全矩阵（fmt/test×4/clippy×4/build×3/PROOF）
      Contract: CI 七 checks 口径
      Implementation: 无 feature 门控新依赖
      Verification: p07_verify.sh 全绿
      Gate: PR required checks

## 6. 文档与收尾
- [ ] Phase Map：0.7C-4 行 COMPLETE（tag）；0.7C 行下一项 = Error Model → Event Projection → External API
      Contract: PHASE_IMPLEMENTATION_MAP=唯一 SoT（文档漂移=P0）
      Verification: 文档对账
      Gate: verify
- [ ] 债表：D9 → CLOSED@0.7C-4，按 D9-A/B/C/D/E 逐项引用测试证据（防假关闭）
      Contract: 终审 0.7C-3 Gate §11
      Verification: PHASE_0_7A_POST_MERGE_DEBT.md
      Gate: verify
- [ ] verify（0 CRIT/0 IMP 目标）→ archive 7/7 → PR → merge → tag phase-0.7C4-idempotency → 删分支

```
