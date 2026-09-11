# VBMF V0.3 P1 实施计划

> 状态：**LOCKED FOR IMPLEMENTATION PLANNING**
> 日期：2026-09-11
> 基线：`9bcbcb64f6e2c1c09b504ba5e8aa0ce859c57a4f`
> 范围：P1 Standalone Boot + Control API
>
> 本文是 P1 编码实施的唯一计划基线。实施前必须完成 S2-C 只读核验；未通过 Implementation Go/No-Go 不得修改 Runtime 代码。

## 1. P1 目标

P1 只解决一个核心问题：让 VBMF Standalone Production 使用**现有 V0.2 Control/Query/Idempotency/Event 语义**形成真实可用的 Runtime Control Loop。

目标闭环：

```text
Process Start
  -> Standalone Boot
  -> Runtime READY / IDLE
  -> GET /health = 200
  -> GET /api/v1/runtime = 200
  -> POST StartSession
  -> Production Capability Boundary
  -> SessionManager create/start
  -> Runtime Running
  -> Runtime Query
  -> Event Projection
  -> Idempotency replay
  -> Stop
  -> Release
```

Production 启动本身**不得自动启动媒体管线**；媒体执行必须由显式 Command/Intent 驱动。

## 2. 当前已确认的架构事实

### 2.1 Session ownership

`SessionManager` 是 Runtime Session 的唯一创建者/销毁者；`MediaSession` 持有 canonical `GraphRuntimeIntent`，不持有具体 GStreamer 对象。P1 不改变这一 ownership model。

### 2.2 Command boundary

`command.rs` 当前只负责 V0.2 Command Contract、形状 validation 与薄 dispatch。P1 不修改 `CommandKind`、`CommandStatus`、Idempotency 语义，也不建立 Universal CommandExecutor。

### 2.3 Production composition

`media-agent.rs` 已经在 Production 与 Diagnostic 共享构造 `SessionManager` 的 composition root；当前缺陷是 Production 将 `mgr` 交给 tick thread 后没有保留 Transport 所需的 `api_mgr` owner，导致 Query/Command context 缺席并产生 503。

### 2.4 Arc ownership

P1 使用同一个 `Arc<SessionManager>`：

```text
                 +-- Runtime tick
                 |
Arc<SessionManager> -- RuntimeQuery
                 |
                 +-- CommandIdempotency
```

不得创建第二个 SessionManager、第二个 Runtime 或额外 ownership plane。

## 3. S2-C 实施前只读核验

在任何 Rust 修改前逐项确认：

- [ ] `services/media-agent/src/bin/media-agent.rs`：确定 Production composition、mode 分流、tick thread、TransportContext 的精确修改点。
- [ ] `transport.rs`：确认 503 条件仅来自 Query/Idempotency context 缺席，并确认正常 200 路径。
- [ ] `runtime_query.rs`：确认 Runtime Query 仍为 Pure Read/Snapshot。
- [ ] `idempotency.rs`：确认与同一 `Arc<SessionManager>` 共享，无额外 ownership。
- [ ] `session.rs`：确认 `create/start/stop/close` 的错误、回滚、状态转换语义不需要改变。
- [ ] Production 测试 fixture：确定最小可复用测试入口。
- [ ] 现有 503 regression tests：确认哪些属于 V0.2 历史证据，哪些需要转为 V0.3 assertions。
- [ ] Event projection tests：确认 Start/Stop/Release 对应事件可观察。
- [ ] integration/smoke tests：确认无需引入新的测试框架或长期服务依赖。

**S2-C 输出：** 文件 → 函数 → 修改区段 → 调用关系 → 修改前后行为 → 对应测试。

## 4. Production StartSession Capability Boundary

P1 当前 Production 能力明确为：**single-input only**。

允许条件：

```text
GraphRuntimeIntent
  -> devices exactly 1
  -> manifest-resolved device
  -> valid input capability/resource/lease/binding
  -> existing SessionManager materialize/start succeeds
```

P1 不创建新的 `ProductionIntent` 类型，不创建大型 Capability Framework。

### 4.1 多输入

Production P1 的 `intent.devices.len() > 1` 在进入 Runtime 执行前拒绝：

```text
Rejected
classification = Rejected
reason = production_capability_not_supported
             / multi_input_not_supported_in_p1
```

`Failed` 只表示已经进入 Runtime 执行阶段后发生的执行错误。

### 4.2 边界位置

Capability check 放在 Production Control/Composition Boundary；不得放入 `command.rs` 的纯形状 validation，也不得让 `SessionManager` 感知 P1、Standalone 或产品阶段。

首版优先使用现有结构完成 predicate；只有在多个真实消费者出现后才抽取 capability helper。

## 5. P1 Runtime 修改范围

### M1 — Production Control Plane wiring

在共同 composition 创建完成后保留：

```text
api_mgr = Some(mgr.clone())
```

Production tick thread 使用自己的 `Arc` clone；Transport Query/Idempotency 使用同一实例。

### M2 — Production StartSession dispatch

让现有 `/api/v1/commands` 在 Production 模式进入既有 Command → SessionManager 链路。

不得绕过 `command.rs` 直接从 HTTP handler 操作 Runtime。

### M3 — Capability rejection

在 Production Control Boundary 增加 single-input capability gate；多输入在 Runtime 创建前 `Rejected`。

### M4 — Runtime Query/Event/Idempotency regression

确认：

- Runtime Query 从 503 恢复为正常 200；
- StartSession 可执行；
- Runtime state 变化可查询；
- Event Projection 可观察；
- 相同 command replay 保持既有 idempotency 结果；
- Stop/Release 使用既有 reverse teardown。

## 6. P1 不实施

- 不修改 Command vocabulary/status。
- 不重写 Idempotency。
- 不新增 Universal CommandExecutor。
- 不实现完整 V0.3 101+ API namespaces。
- 不实现 Scheduler/Placement/Workload/Allocation。
- 不实现 HA/Cluster。
- 不实现 SSE/WebSocket。
- 不实现 Prometheus Metrics。
- 不实现 Incident subsystem。
- 不实现 Recording/Replay/Timeshift 产品。
- 不实现 Production 双输入/专业 Switcher。
- 不动态为 P1 command-created session 重构 Watchdog ownership。
- 不实现进程级 graceful OS shutdown。
- 不接入 Mother Framework。
- 不建设 `vbmf-sdk`。
- 不创建 `v0.3.0` tag。

## 7. Watchdog 归属

P1 不新增动态 watchdog spawn。

当前 watchdog 属完整 Runtime health/recovery mechanism，涉及 observation、failure detection、recovery、Supervisor、event 等多个 ownership 面。将其在 P1 临时接入会扩大 Runtime lifecycle 设计，因此统一归入 P3 Runtime Hardening。

除非 S2-C 发现一个**已经存在的硬正确性依赖**，否则不得在 P1 改 Watchdog。

## 8. 测试矩阵

| ID | 场景 | 预期 |
|---|---|---|
| P1-01 | Production + single input | StartSession 执行成功，Session Running |
| P1-02 | Production + multi input | Rejected；无 Session/Lease/Allocation/Pipeline 副作用 |
| P1-03 | Production + empty input | Rejected |
| P1-04 | single input + invalid binding | Failed；沿既有 Runtime rollback |
| P1-05 | StartSession replay | 既有 idempotency replay/already_applied 语义 |
| P1-06 | StopSession | Executed，Session 停止 |
| P1-07 | ReleaseSession | Executed，Session Released |
| P1-08 | SwitchProgram in P1 single-input Production | 继续 Rejected，不虚构双输入执行平面 |
| P1-09 | Runtime Query | Production 不再因 api_mgr 缺席返回 503 |
| P1-10 | Event Projection | Start/Stop/Release 对应事件可观察 |

P1-02 必须证明 capability rejection 发生在 SessionManager.create 之前，而不是先创建再回滚。

## 9. V0.2 → V0.3 测试迁移原则

V0.2 Production 503 测试不得简单删除。它们是历史行为证据。

V0.3 新断言：

```text
Production boot
  -> /health 200
  -> /api/v1/runtime 200
  -> /api/v1/commands 不因 query/idem context 缺席而 503
```

文档必须明确：这是 V0.3 Standalone Control Plane 的版本化行为变化，而不是偷偷修改 V0.2 冻结语义。

## 10. 文档收口项

PR #31 合并前必须修正三处一致性问题：

1. **P0 Console 页面冲突**：Recording、Incidents 从 P0 页面列表调整为 P4；P1/P2 Core Console 保留 Dashboard/Sources/Sessions/Switcher/Outputs/Health/Engineering。
2. **MVP Incident 冲突**：P1/P2 MVP 的可观察面写为 Event/Health；Incident Timeline 明确归 P4。
3. **clean shutdown 表述**：P1 的“正常关闭”限定为 Session stop/release clean teardown；进程级 graceful OS shutdown 明确归 P3。

修正文档后再重新运行文档一致性与 PR Gate。

## 11. Diff budget

生产 Rust 目标：**约 20–60 行净新增/修改**。

若生产代码净变更超过约 150 行，自动触发 P1 Scope Review；除非有证据证明是既有架构缺陷修复，否则停止扩张。

文档/测试可适度增加，但不得借测试引入新的 Runtime architecture。

## 12. Gate 顺序

```text
S2-A  Capability Boundary          PASS
   ↓
S2-B  Modification-point Plan      PASS
   ↓
S2-C  Read-only final audit        NEXT
   ↓
Implementation Go/No-Go
   ↓
P1 Implementation
   ↓
Unit / Integration / Smoke
   ↓
CI
   ↓
P1 Control Loop Gate
   ↓
PR Review / Merge Gate
   ↓
P2
```

任何 Gate FAIL：停止向后推进，先修复/重新审计。

## 13. 红线

- `master` 在 P1 实施期间不得被直接修改。
- PR #31 在 P1 Implementation Go/No-Go 前不合并。
- 不在 P1 偷改 V0.2 frozen Command/Idempotency/Error/Event semantics。
- 不允许 Browser 直连 GStreamer/FFmpeg/DeckLink/Rust internal object。
- 不允许 Production 自动启动媒体管线。
- 不允许多输入在 P1 静默降级成单输入。
- 不允许 Rejected 场景先创建 Session 再回滚来“模拟支持”。
- 不允许创建第二个 SessionManager/Runtime owner。
- 不允许为未来 Capability/Scheduler/HA 提前建复杂基础设施。
- `v0.3.0` 只有 V0.3 实现完整、合并并通过发布 Gate 后才能创建。

## 14. 当前状态

```text
P0 V0.3 baseline                  PASS
P1 S1 read-only audit             GO
P1 S2-A capability boundary       GO
P1 S2-B modification plan         GO
P1 S2-C read-only audit           NEXT
Implementation                     HOLD
PR #31 merge                       HOLD
master                             PROTECTED / NO TOUCH
v0.3.0 tag                         FORBIDDEN UNTIL RELEASE GATE
```
