# VBMF Execution Model (V0.1 · 0.5D.3 锁)

> **目的:** 回答"谁创建谁 / 谁引用谁 / 谁实例化谁 / 谁产生谁 / 什么时候 Desired→Compiled / Provision / Reserve / Start / Take / Release" — 使 Phase 1 工程师无需脑补对象间时序。
>
> **关联:** `OBJECT_VOCABULARY.md` (15 对象) · `PRODUCT_OBJECT_MODEL.md` (3 层组合) · `RESOURCE_RESERVATION_SPEC.md` (Reservation 生命周期) · `ENCODE_MODEL_SPEC.md` (FILE/REALTIME) · `B-13-take-preflight.html` (Preflight 门禁)
>
> **状态:** 🟢 **SEMANTIC LOCKED** (0.5D.3 + §5 0.5D.4 + §5 0.5D.5 补强) — 与 §J/§K/§M 对账一致

---

## 1. 核心执行链 (REALTIME — 播出域)

```text
Template (CHANNEL_TEMPLATE, 工厂, 不进运行态)
   │  instantiate (D1/D2)
   ▼
Channel (DRAFT) ──────────────┐
   │  snapshot 默认 7 Profile  │  source assignment (02-sources / D1)
   ▼                          │  E-40 create → E-42 VERIFIED → ASSIGN PRIMARY/BACKUP
Profile Bundle (B-v2, immutable)   ▼
   │  resolve 7 Profile refs   Source Assignment (role/priority/standby)
   ▼                          │
Profiles (P-21 ENC-v3 REALTIME / P-22 / P-23..)  │
   │  compile                 │
   ▼                          │
GraphRuntime (DESIRED → COMPILED → EFFECTIVE)    │
   │  provision               │
   ▼                          │
Reservation (PROVISIONED → RESERVED → IN_USE → RELEASED)
   │  start (H2 Scheduler, 资源已 RESERVED)
   ▼
Session (STARTING → RESERVED → READY_TO_TAKE)
   │  operator action (CD-01)
   ▼
TAKE ──► RUNNING (Program Master → Output Variant → Adapter)
   │
   ├─ FAILOVER: PACKET/FRAME/MASTER Decision → 备源/备机 (Reservation 已预占)
   └─ OUTPUT RECOVERY: OutputResilience (retry/backoff/zombie) → 不切源 → Incident/Replay
```

### 时序判定 (REALTIME)

| 事件 | 谁执行 | 前置条件 | 后置状态 |
|---|---|---|---|
| `instantiate` | D1/D2 向导 | Template 存在, 7 Profile 默认 | Channel DRAFT + Bundle B-v2 快照 |
| `resolve` | 编译期 | Bundle 快照有效 | 7 Profile 引用 resolved |
| `compile` | Graph Runtime | Profiles 兼容 (P-21 REALTIME) | DESIRED → COMPILED |
| `provision` | H2 Scheduler | Preflight A (Config) PASS | Reservation PROVISIONED |
| `reserve` | H2 Scheduler | 9-dim vector 可满足 | Reservation RESERVED (HOT 备机同锁) |
| `start` | Session Manager | Reservation == RESERVED | Session STARTING → READY_TO_TAKE |
| `take` | Operator (CD-01) | B-13: READY/CONDITIONAL + Reservation RESERVED | TAKE → RUNNING |
| `release` | Session stop / 退役 | 主备切换完成 / 显式释放 | Reservation RELEASED → 触发仲裁 |

> ⛔ **不变量:** `Apply ≠ Start` · `Start ≠ Take` · `Take ≠ Failover` · **TAKE 绝不触发资源抢占** (只验证 RESERVED)。

---

## 2. 核心执行链 (FILE — 媒资域)

```text
Asset (源)
   │  submit
   ▼
Job (FILE_TRANSCODE) ──── FILE_PROFILE (P-21, profile_type=FILE_PROFILE, 只读引用)
   │  schedule
   ▼
Queue / Worker (Concurrency/Priority/Retry — Job Policy, 不是 Profile 字段)
   │  run
   ▼
Output Asset Version (ENC-v12 / ENC-v22 ...) ──► QC (可选) ──► 发布/归档
```

### 时序判定 (FILE)

| 事件 | 谁执行 | 前置条件 | 后置状态 |
|---|---|---|---|
| `submit` | Editor/API | Asset 入库 + FILE_PROFILE 选择 (禁 REALTIME_PROFILE) | Job QUEUED |
| `schedule` | Job Scheduler | Queue/Priority/Worker 策略 | Job RUNNING |
| `run` | Worker | FILE_PROFILE 编译通过 | Output Asset Version (每个版本一个 ENC-v) |
| `retry/resume/cancel` | Job Policy | — | Job 状态迁移 (File: Retry/Resume/Requeue) |

> **Job ≠ Session:** File = Job + Asset Version (可重试/续传); Realtime = Session + Reservation (Restart/Failover/Keep Program)。

---

## 3. 谁创建谁 / 谁引用谁 (对象关系总表)

| 对象 | 由谁创建 | 引用谁 | 被谁引用 | 实例化/产生 |
|---|---|---|---|---|
| Channel Template | Engineer (CH-02B) | 默认 Bundle 模板 / 默认源策略 | Channel (Used By) | Channel |
| Profile | Engineer (P-20/P-21/P-22..) | 无 | Bundle / Channel | — |
| Profile Bundle | Engineer (P-28) / Template 快照 | 7 Profile (revision 引用) | Channel (1:1) | Output Variant 派生 |
| Channel | Operator (D1) | Template / Bundle / Source Assignment | Session / Reservation | Session |
| Source | Engineer (E-40) | Adapter / Endpoint / Contract | Channel (ASSIGN) | — |
| Output Variant | Bundle 派生 (D1/D3) | P-22 Output Profile | Channel (交付) | — |
| Destination | Engineer (CD-01 Detail) | Variant | Adapter | — |
| Adapter | Runtime / Device Registry (E-35/E-38) | 无 | Destination | — |
| Reservation | H2 Scheduler (Apply→Provision) | Channel / Session / resource_vector | Session (门禁) | — |
| Session | Session Manager | Channel / P-21 REALTIME / Reservation | RUNNING | — |
| Job | Editor/API | Asset / FILE_PROFILE | Worker | Output Asset Version |
| ChangeSet | 任何 Configure 动作 | items (before/after revision) | Apply | — |

---

## 4. 状态机对照 (为什么这样切)

- **Desired → Compiled:** 配置修改后由 Graph Runtime 编译 (相容性/资源校验)。
- **Compiled → Effective:** Apply 后运行态采用 (3-Layer 一致才算生效)。
- **Provision vs Reserve:** Provision = 预算/计划 (PROVISIONED); Reserve = 实际锁定 (RESERVED, 锁 9-dim vector + device_tokens)。HOT 备机必须同为 RESERVED 才算真锁。
- **Start vs Take:** Start = Session 拉起 (STARTING→READY_TO_TAKE); Take = Operator 切出 (READY_TO_TAKE→RUNNING)。Start 后可停在 READY_TO_TAKE 等指令。
- **Release:** STOP / 故障切换完成后 → RELEASED → 触发 PENDING 仲裁 (抢占走 PREEMPT_PENDING→DRAINING→RELEASED, 不直接 FAILED)。

---

## 5. Configuration Change ≠ Operational TAKE (0.5D.4 焊死)

```text
Configuration Change                Operational TAKE
─────────────────────              ─────────────────────
Draft → Validate → ChangeSet       Prepared Session → Readiness
→ Approve → Apply → Runtime Rev     → Operator Intent → TAKE (Runtime Event)
                                        → Incident / Audit
```

- **Configuration Change** 走 ChangeSet (Logical Atomic), 进 Runtime Revision。
- **Operational TAKE** 是运行时事件 (Operator Intent → Runtime Event), 可引用 ChangeSet / Runtime Revision, 但**不是所有 TAKE 都创建 ChangeSet**。
- B-13 验证对象 = 运行时就绪态 (Source / Clock / Output / Reservation); 与配置 ChangeSet 解耦 — **TAKE ≠ 一次配置变更**。
- 模型层: **Configuration Surface ≠ Runtime Surface ≠ Operational Surface** — Desired/Compiled/Effective 是对象状态层; Config/Runtime/Operate 是用户工作层 (0.5D.4)。

---

**VBMF Contributors** · Execution Model V0.1 · Phase 0.5D.3 Object/State/Execution Closure
