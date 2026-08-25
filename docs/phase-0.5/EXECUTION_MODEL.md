# VBMF Execution Model (V0.1 · 0.5D.3 锁)

> **目的:** 回答"谁创建谁 / 谁引用谁 / 谁实例化谁 / 谁产生谁 / 什么时候 Desired→Compiled / Provision / Reserve / Start / Take / Release" — 使 Phase 1 工程师无需脑补对象间时序。
>
> **关联:** `OBJECT_VOCABULARY.md` (15 对象) · `PRODUCT_OBJECT_MODEL.md` (3 层组合) · `RESOURCE_RESERVATION_SPEC.md` (Reservation 生命周期) · `ENCODE_MODEL_SPEC.md` (FILE/REALTIME) · `B-13-take-preflight.html` (Preflight 门禁)
>
> **状态:** 🟢 **SEMANTIC LOCKED** (0.5D.3 + §5 0.5D.4 + §5 0.5D.5 补强 + §6 0.5D.6 Click-Path Audit + §7 0.5F 双管线) — 与 §J/§K/§M/§N/§Q 对账一致

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
Session (lifecycle STARTING→RUNNING · readiness NOT_READY→READY_TO_TAKE · health UNKNOWN→HEALTHY)
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

**Media Session 三轴 (0.5F.2 P0 修正 — 删除 Session `RESERVED`):**
- **Reservation.state**: `PROVISIONED → RESERVED → IN_USE → RELEASED` (属于 Reservation, 不属于 Session)。
- **Media Session** 仅用 V0.2 Runtime 三轴:
  - `lifecycle`: `STOPPED → STARTING → RUNNING → STOPPING`
  - `readiness`: `NOT_READY → READY_TO_TAKE`
  - `health`: `HEALTHY / DEGRADED / FAILED / UNKNOWN`
- **Start**: lifecycle `STARTING → RUNNING` · readiness `NOT_READY → READY_TO_TAKE` (前置: `Reservation == RESERVED` 是真锁前提, 但 Session 状态本身无 `RESERVED`)。
- **TAKE 后**: lifecycle `RUNNING` · readiness `READY_TO_TAKE` · health `HEALTHY` · `active_source = PRIMARY`。
- ⛔ 禁止在 Session 上出现 `RESERVED` / `IN_USE` 等 Reservation 状态词 (Phase 1 会产生第二套 Runtime 状态机)。

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

## 6. 用户点击路径 → DB Object / Revision / Session / Audit Event 闭环审计 (0.5D.6)

> **目的:** 从用户实际点击路径**反推**每一步落到的 DB Object / Revision / Runtime Session / Audit Event (A-54 hash chain), 反向核对对象模型与执行时序无洞。审计通过 = Phase 0.5 Freeze 的前置条件 (用户第 9 轮要求)。
>
> **审计事件统一前缀 (A-54):** `SOURCE_*` / `PROFILE_*` / `BUNDLE_*` / `CHANNEL_*` / `VARIANT_*` / `CHANGESET_*` / `CONFIG_*` / `RESERVATION_*` / `SESSION_*` / `TAKE_*` / `FAILOVER_*` / `OUTPUT_RECOVERY_*` / `JOB_*` / `INCIDENT_*` / `AUDIO_*`。全量写入 `audit_logs` (append-only · hash chain)。

### 6.1 路径 A — Source 创建与验证 (E-40 → E-42 → VERIFIED)

| # | 用户点击 | DB Object | Revision | Runtime Session | Audit Event (A-54) | 状态迁移 |
|---|---|---|---|---|---|---|
| A1 | E-40 创建 (Source Kind→Transport→Delivery Mode→Endpoint Schema) | `sources` (NETWORK) | `src-001@d0` | — | `SOURCE_CREATE` (who/why) | SourceLifecycle `DRAFT` |
| A2 | E-40 Run Source Test Bench → E-42 | `preflight_runs` (E-42 验证台) | — | — | `SOURCE_TEST_RUN` | `TESTING` |
| A3 | E-42 7 层 PASS | Source Contract 回写 (Media Contract) | `src-001@v1` (Contract) | — | `SOURCE_VERIFIED` | `TESTING → VERIFIED` |
| A4 | E-40 SAVE VERIFIED / ASSIGN | `source_assignments` (role/priority/standby) | — | — | `SOURCE_ASSIGN` | `VERIFIED → ASSIGNED` |
| A5 | D1 Step2 选 VERIFIED 源 | Channel `source_refs` (PRIMARY/BACKUP) | — | — | `SOURCE_ATTACH_CHANNEL` | `ASSIGNED → ACTIVE` (上线后) |

**关键:** Source 创建入口**唯一** = E-40; 02-sources / CD-01 / E-42 / Health Tree 只读/查看。无第二套 source 创建逻辑。

### 6.2 路径 B — Profile / Bundle 创建 (P-21 / P-22 / P-28)

| # | 用户点击 | DB Object | Revision | Runtime Session | Audit Event | 状态迁移 |
|---|---|---|---|---|---|---|
| B1 | P-21 Create (FILE_PROFILE / REALTIME_PROFILE) | `profiles` (ENCODING) | `ENC-v3` | — | `PROFILE_CREATE` | Profile Revision 不可变 |
| B2 | P-22 Create (Delivery Policy) | `profiles` (OUTPUT) | `OUT-v4` | — | `PROFILE_CREATE` | 不落 Destination/Adapter |
| B3 | P-28 Bundle (7 Profile refs) | `profile_bundles` | `B-v2` | — | `BUNDLE_CREATE` | 引用 7 Profile Rev |

### 6.3 路径 C — Channel 创建 (D1 CH-02 向导)

| # | 用户点击 | DB Object | Revision | Runtime Session | Audit Event | 状态迁移 |
|---|---|---|---|---|---|---|
| C1 | CH-02 Step1 (Template/类型) | `channels` (DRAFT) | 模板 `T-v3` 快照 | — | `CHANNEL_CREATE` | ChannelLifecycle `DRAFT` |
| C2 | CH-02 Step2 源分配 | Channel `source_assignments` | — | — | `SOURCE_ATTACH` | — |
| C3 | CH-02 Step5 输出 (选 Profile+Destination) | `output_variants` × N | — | — | `VARIANT_ATTACH` | Variant 引用 Profile + Destination (`adapter_ref` 在 Destination) |
| C4 | CH-02 Step7 提交 | ChangeSet (E-33) | `CS-001` | — | `CHANGESET_CREATE` | CS `DRAFT → VALIDATED` |
| C5 | E-33 Review/Approve (L2) | ChangeSet | `CS-001` | — | `CHANGESET_APPROVE` | `APPROVED` |
| C6 | Apply | Channel Config Rev / Runtime Revision +1 | `cfg-rev N→N+1` | — | `CONFIG_APPLY` | Channel `COMPILED` |
| C7 | Runtime Provision/Reserve (H2) | `reservations` (9-dim + device_tokens) | — | — | `RESERVATION_RESERVE` | `PROVISIONED → RESERVED` |
| C8 | Session start (H2 Scheduler) | `media_sessions` (MEDIA_SESSION) | — | Session `s-001` | `SESSION_START` | `STARTING → READY_TO_TAKE` |

### 6.4 路径 D — Channel Workspace 日常操作 (CD-01)

| # | 用户点击 | DB Object | Revision | Runtime Session | Audit Event | 状态迁移 |
|---|---|---|---|---|---|---|
| D1 | CD-01 切源 PGM←B (TAKE) | — (运行时事件) | — | Session `s-001` | → 路径 E | `READY_TO_TAKE → RUNNING` |
| D2 | CD-01 添加 Output Variant (选 Profile+Destination) | `output_variants` (新) | — | — | `CONFIG_CHANGE` (走 ChangeSet) | Variant refs 更新 |
| D3 | CD-01 Audio MUTE / DIM | Session runtime | — | — | `AUDIO_CONTROL` (即时生效, 不进 ChangeSet) | runtime 状态 |
| D4 | CD-01 Open Audio 深页 / Health | 查看 | — | — | 无需 audit | — |

### 6.5 路径 E — TAKE (CD-01 TAKE → B-13 → Runtime Event)

| # | 用户点击 | DB Object | Revision | Runtime Session | Audit Event | 状态迁移 |
|---|---|---|---|---|---|---|
| E1 | CD-01 TAKE → B-13 模态 | `preflight_runs` (B-13) | — | — | `PREFLIGHT_RUN` | TakePreflightResult `READY/CONDITIONAL/BLOCKED` |
| E2 | B-13 READY → Operator Intent | TakeIntent (operator intent) | — | — | `OPERATOR_INTENT` | — |
| E3 | TAKE 执行 | — (Runtime Event `evt-take-...`) | — | Session `READY_TO_TAKE → RUNNING` | `TAKE_RECORD` (**非 ChangeSet**) | Reservation `IN_USE` |
| E4 | Audit / Incident Timeline | `audit_logs` + `incident_timeline` | — | — | `TAKE_RECORD` 入链 | — |

**关键 (0.5D.5 焊死):** TAKE **不生成 ChangeSet**。仅当同次操作还修改了 Bundle/Profile/Route/Output/Runtime Config 时, 才**另发** ChangeSet (E-33), 与 TAKE 事件解耦。

### 6.6 路径 F — Failover / Output Recovery (M-17 / CD-01)

| # | 用户点击 | DB Object | Revision | Runtime Session | Audit Event | 状态迁移 |
|---|---|---|---|---|---|---|
| F1 | M-17/CD-01 FAILOVER | — | — | 备 Session (HOT RESERVED) | `FAILOVER_TRIGGER` | Graph Compiler Decision → Effective Switch Mode |
| F2 | Output 故障恢复 | `output_resilience` (retry/backoff/zombie) | — | — | `OUTPUT_RECOVERY` | 不切源 → 恢复 |
| F3 | 恢复失败 → Incident | `incidents` | — | — | `INCIDENT_OPEN` | Incident `#1248` |

**关键:** Failover 切换依赖**已预占** Reservation (HOT 备机 RESERVED); 绝不临时抢资源。

### 6.7 路径 G — FILE Job (M-14)

| # | 用户点击 | DB Object | Revision | Runtime Session | Audit Event | 状态迁移 |
|---|---|---|---|---|---|---|
| G1 | M-14 Wizard | `media_jobs` (FILE_TRANSCODE) | — | — | `JOB_CREATE` | JobState `PENDING` |
| G2 | Worker 调度 | Job | — | — | `JOB_START` | `QUEUED → RUNNING` |
| G3 | 完成 | Job + Asset Version | `asset v→v+1` | — | `JOB_COMPLETE` | `COMPLETED` (FAILED/CANCELLED 分路) |

### 6.8 路径 H — ChangeSet 配置变更 (E-33 / D7)

| # | 用户点击 | DB Object | Revision | Runtime Session | Audit Event | 状态迁移 |
|---|---|---|---|---|---|---|
| H1 | 改 Bundle/Profile/Route/Output/Runtime Config | ChangeSet | — | — | `CHANGESET_CREATE` | `DRAFT` |
| H2 | Validate | ChangeSet | — | — | `CHANGESET_VALIDATE` | `VALIDATED` |
| H3 | Approve (L2 / D7 独立面) | ChangeSet | — | — | `CHANGESET_APPROVE` | `APPROVED` |
| H4 | Apply | Runtime Revision +1 | `cfg-rev +1` | — | `CONFIG_APPLY` | `APPLIED` → 触发 Provision (C7) |

### 6.9 单一创建入口映射 (0.5D.5 原则 → 落点)

| 对象 | 唯一创建入口 | 查看入口 (只读/引用) | 禁入口 |
|---|---|---|---|
| Source | E-40 → E-42 VERIFIED | 02-sources · CD-01 · E-42 · Health Tree | 频道向导内联创建 ✗ |
| Destination | CD-01 Output / 06-output Wizard | 06-output · CD-01 · E-41 | P-22 配 URL ✗ |
| Encoding Profile | P-21 | M-17 · CD-01 · P-28 | 运行页改参数 ✗ |
| Output Profile | P-22 | CD-01 · P-28 · 06-output | 运行页改参数 ✗ |
| Bundle | P-28 | Channel (D3) · 模板 (D2) | — |
| ChangeSet | 配置变更动作 (E-33) | D7 Review · B-13 引用 | TAKE 生成 ✗ |
| Adapter | E-35 Device Registry (Runtime) | P-22 3-Tier · 06-output | Profile 页定义 ✗ |

### 6.10 审计结论 (无洞检查)

- **对象 ↔ 页面一致性**: 每条用户点击都能反推唯一 DB Object + Revision + (如需) Session + Audit Event; 无"点击落在模型外"。
- **状态机一致性**: `SourceLifecycle` / `JobState` / `ChangeSet 三层` / `Reservation` / `Session 三轴` 各自闭合, 交叉引用无冲突。
- **TAKE 语义**: TAKE = Runtime Event, 不产生 ChangeSet (E 路径与 H 路径完全分离)。
- **Revision 链**: 配置改 → ChangeSet → Runtime Revision +1; 运行事件不 bump Revision (只写 Audit/Incident)。
- **剩余 (0.5E/0.5G)**: Impact Preview / Configuration Diff / Command Palette / Global Risk / Critical field blocking 未进 HTML (SURFACE_SPEC 标 Spec, Phase 4 实施); Player Capability 由 Capability Registry 推导 (P2)。

---

## 7. UI 双动作管线 — Configuration Pipeline vs Runtime Operation Pipeline (0.5F F3 焊死)

```text
A. CONFIGURATION PIPELINE (对象配置变更)
   Edit → Desired → Impact Preview (E-50) → Diff (E-51)
   → Preflight → ChangeSet (E-33) → Approve → Transactional Cutover
   → Compiled → Effective
   Runtime Actions 绝不进入本管线。

B. RUNTIME OPERATION PIPELINE (运行操作)
   Observe → Preflight / Readiness (B-13) → Operator Intent
   → TAKE / FAILOVER / RESTART / RETRY / OUTPUT RECOVERY
   → Runtime Event → Audit (A-54) → Incident Timeline
   不生成 ChangeSet (仅引用 Runtime Revision / Config Revision)。
```

| Runtime Action | 是否进 ChangeSet | 走哪条链 | Audit Event |
|---|---|---|---|
| TAKE | ❌ 否 | B | `TAKE_RECORD` (Runtime Event) |
| FAILOVER | ❌ 否 | B | `FAILOVER_TRIGGER` |
| RESTART | ❌ 否 | B | `SESSION_RESTART` |
| RETRY | ❌ 否 | B | `JOB_RETRY` / `OUTPUT_RETRY` |
| OUTPUT RECOVERY | ❌ 否 | B | `OUTPUT_RECOVERY` (retry/backoff/zombie) |

**UI 边界 (0.5F):**
- **E-50 / E-51** = Configuration Pipeline 的确认面 (Continue to Diff → ChangeSet); 展示 Operational Consequence, 但不执行 Runtime 操作。
- **E-52 Command Palette**: L2/L3 Action 一律**跳转**到对应确认入口 (TAKE → CD-01 + B-13), 不在 Palette 内直接执行; 命令必须携带 Context (surface/object/channel/session, 0.5F F5)。
- **B-13** = Runtime Pipeline 的 TAKE 门禁 (9 项 Preflight + Operator Intent → Runtime Event)。
- 两条链共享 **Audit (A-54)** 与 **Incident Timeline**, 但对象不同: **ChangeSet** (配置链) vs **Runtime Event** (运行链), 绝不复用同一抽象 (0.5D.5 §5 延续)。

---

**VBMF Contributors** · Execution Model V0.1 · Phase 0.5D.3 Object/State/Execution Closure
