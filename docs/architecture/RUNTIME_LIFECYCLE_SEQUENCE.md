# RUNTIME_LIFECYCLE_SEQUENCE — 运行时生命周期时序

> Phase 0.6 P0 契约（TD-02）。把分散在 Session / Resource / Lease / Binding / Backend / Pipeline 文档里的时序**合成一张图**，明确「谁先谁后、失败时谁释放」。状态词见 [`DOCUMENT_STATUS_MODEL.md`](./DOCUMENT_STATUS_MODEL.md)。
> 关联：[`IMPLEMENTATION_ADDENDUM.md §4/§5/§9`](./IMPLEMENTATION_ADDENDUM.md)、[`RUNTIME_RESOURCE_MODEL.md`](./RUNTIME_RESOURCE_MODEL.md)、[`RUNTIME_SESSION_MODEL.md`](./RUNTIME_SESSION_MODEL.md)。

## 1. 主时序（成功路径）

```
Control Plane
  │  Intent (Manifest / GraphRuntimeIntent)
  ▼
Runtime Session Manager
  │  ── 唯一 creator ──
  ▼
Preflight  ──────────────►  (Bind 预检 / Reservation 冲突预检 / Capability 预检)
  │  PASS
  ▼
Resource Registry
  │  Reserve Port / DMA / Session-capacity   (Reservation 进入 Reserved)
  ▼
Runtime Session Manager
  │  Create Session (SessionId = Uuid)
  ▼
Lease Manager
  │  Acquire Lease (Exclusive Runtime Claim)  (Reservation → Leased)
  ▼
Binding Resolver
  │  Verify Physical→Provider→Runtime Binding  (achieved_verification 提升)
  ▼
Runtime Orchestrator
  │  ── 已解析 Resolved Execution Plan ──
  ▼
MediaBackend.instantiate(plan)
  │  PipelineHandle 创建
  ▼
Resource Registry
  │  Allocation (Reservation/Lease → Allocated；占用 Capacity)
  ▼
MediaBackend.start(handle)
  │  Pipeline 在线
  ▼
Session  ──►  Running  (Health aggregator 开始观测)
  │
  │  (运行期)
  ▼
MediaBackend.stop(handle)
  ▼
Resource Registry
  │  Release Allocation → Release Lease → Release Reservation
  ▼
Runtime Session Manager
  │  Destroy Session (唯一 terminator)
  ▼
Resource Registry  ──►  Available (资源回收)
```

## 2. 失败 / 异常分支（谁释放）

| 失败点 | 已发生 | 释放责任 |
|---|---|---|
| Preflight FAIL | 无 Reservation / 无 Session | 无释放（Preflight 在 Reserve 之前） |
| Reserve FAIL（Reservation conflict） | Reservation 未创建 | Resource Registry 拒绝后到者，无残留 |
| Create Session FAIL | Reservation 已建 | Session Manager 或 Resource Registry **回滚 Reservation**（Reserved→Available） |
| Acquire Lease FAIL（Lease conflict） | Reservation 已建、Session 已建 | **释放 Lease 竞争方**；Reservation 由 Session Manager 回滚 |
| Binding Verify FAIL | Reservation+Lease 已建 | Binding Resolver 拒绝；Session Manager 释放 Lease+Reservation，销毁 Session |
| `Backend.instantiate` FAIL | Reservation+Lease 已建、Session 已建 | **Session 不立即销毁**：Session Manager 标记 `ProvisioningFailed`，释放 Lease+Reservation，销毁 Session（fail-closed，绝不留半吊子 Pipeline） |
| `Backend.start` FAIL（Pipeline 起不来） | Allocation 已建 | Backend `stop` 兜底；Session Manager 标记 `StartFailed`，回滚 Allocation→Lease→Reservation，销毁 Session |
| Runtime crash / Supervisor Recovery | Running 中 | Supervisor 触发 `Backend.recover`；**不重新 Reservation**（已在 Lease 内）；若 Lease 过期 → `RECOVERING` → 重新 Reservation（见 `MEDIA_AGENT_STATE_MACHINE.md` 租约重校验不变量） |
| Stop / 正常结束 | Running | 正常顺序释放（见 §1） |

> **铁律**：任何失败分支都**不允许**留下「Reservation/Lease/Allocation 已占但 Session/Pipeline 不存在」的孤儿状态。释放责任唯一归属 **Runtime Session Manager**（创建者即销毁者）。

## 3. Ownership 与生命周期的关系
- `Session` 拥有其生命周期，但**不拥有资源释放的底层操作**——它**委派** Resource Registry / Lease Manager / Binding Resolver / Backend 执行，自身只做编排与失败回滚决策。
- `Backend` 只 `instantiate/start/stop/recover/observe`，**不得**自行 `acquire` 未注册资源（见 `MEDIA_BACKEND_CONTRACT.md` P0-8）。
- `Supervisor` 拥有 Recovery Decision，但 Recovery 不改变 Reservation/Lease 的所有权（仍在 Session Manager 内）。
