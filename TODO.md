# VBMF TODO

> 当前主线：**V0.3-P1 Standalone Boot + Control API**
> 当前状态：**S2-C Read-only Final Audit NEXT**
> 维护规则：完成项必须有代码/测试/CI/文档证据；发现阻塞先修复再前进；不得绕过 Gate。

## 当前执行队列

### P1-S2-C · Implementation 前最终只读核验

- [ ] **S2-C-01** 核对 `services/media-agent/src/bin/media-agent.rs`：Production composition、mode 分流、tick thread、TransportContext 精确修改点
- [ ] **S2-C-02** 核对 `transport.rs`：503 条件、200 路径、Production/Diagnostic 差异
- [ ] **S2-C-03** 核对 `runtime_query.rs`：Pure Read/Snapshot 不变量
- [ ] **S2-C-04** 核对 `idempotency.rs`：同一 `Arc<SessionManager>` ownership
- [ ] **S2-C-05** 核对 `session.rs`：create/start/stop/close、rollback、状态迁移无需改变
- [ ] **S2-C-06** 定位 Production capability gate 的最小实现边界
- [ ] **S2-C-07** 定位现有 503 regression tests 与可迁移的 V0.3 assertions
- [ ] **S2-C-08** 定位 Event Projection / Start-Stop-Release 测试
- [ ] **S2-C-09** 定位最小 standalone integration/smoke fixture
- [ ] **S2-C-10** 输出“文件→函数→区段→行为→测试”实施表

### PR #31 · 文档基线收口

- [ ] **DOC-01** 修正 P0 Console 页面与 P2/P4 冲突：Recording/Incidents → P4
- [ ] **DOC-02** 修正 MVP “Event/Health/Incident”：P1/P2 仅 Event/Health，Incident → P4
- [ ] **DOC-03** 修正 P1 “clean shutdown”：限定 Session stop/release；进程级 graceful shutdown → P3
- [ ] **DOC-04** 重新运行 V0.3 文档一致性检查
- [ ] **DOC-05** 重新运行 PR #31 diff / CI / Merge Gate
- [ ] **DOC-06** PR #31 保持不合并，直到 P1 Implementation Go/No-Go

### P1 Implementation · 仅在 S2-C PASS 后执行

- [ ] **IMP-01** Production composition 保留 `api_mgr = Some(mgr.clone())`
- [ ] **IMP-02** Production 接入既有 Control API → Command → SessionManager 链路
- [ ] **IMP-03** 增加 single-input Production capability boundary
- [ ] **IMP-04** multi-input 在 Runtime create 前 `Rejected`
- [ ] **IMP-05** 保持 V0.2 Command/Idempotency/Error/Event semantics
- [ ] **IMP-06** Runtime Query 从 Production 503 恢复到正常 200
- [ ] **IMP-07** Start/Stop/Release Event Projection 回归
- [ ] **IMP-08** Idempotency replay/already_applied 回归
- [ ] **IMP-09** SwitchProgram P1 继续 honest Rejected
- [ ] **IMP-10** 不新增动态 watchdog；Watchdog 保持 P3

### P1 Verification Gates

- [ ] **TEST-01** P1-01 single-input StartSession
- [ ] **TEST-02** P1-02 multi-input Rejected + zero side effects
- [ ] **TEST-03** P1-03 empty input Rejected
- [ ] **TEST-04** P1-04 invalid binding Failed + rollback
- [ ] **TEST-05** P1-05 idempotency replay
- [ ] **TEST-06** P1-06 StopSession
- [ ] **TEST-07** P1-07 ReleaseSession
- [ ] **TEST-08** P1-08 SwitchProgram Rejected
- [ ] **TEST-09** P1-09 Production Runtime Query 200
- [ ] **TEST-10** P1-10 Event Projection
- [ ] **TEST-11** format / clippy / unit / integration / smoke / CI
- [ ] **TEST-12** P1 Standalone Control Loop Gate

## 后续阶段（不提前实施）

### P2 · Web Console P0

- [ ] Dashboard
- [ ] Sources
- [ ] Sessions
- [ ] Switcher
- [ ] Outputs
- [ ] Health
- [ ] Engineering

### P3 · Bug Fix + Runtime Hardening

- [ ] 全量 bug ledger
- [ ] Fault Injection
- [ ] 24h Stability
- [ ] Runtime Observer / Watchdog / Recovery evidence
- [ ] 进程级 graceful OS shutdown
- [ ] Metrics

### P4 · Broadcast Professionalization

- [ ] Recording / Replay / Timeshift
- [ ] Incident Timeline
- [ ] Graph / Preflight
- [ ] Capability Framework
- [ ] AVSync / Latency
- [ ] Redundancy / Failover
- [ ] Production multi-input / advanced switcher

### P5/P6

- [ ] Mother Contract mapping / Conformance
- [ ] Standalone Consumer Evidence
- [ ] `vbmf-sdk` stability evaluation
- [ ] Mother Platform Integration

## 红线检查

- [ ] master 未被 P1 直接修改
- [ ] PR #31 未提前合并
- [ ] 未创建 `v0.3.0`
- [ ] 未修改 V0.2 frozen Command Contract
- [ ] 未重写 Idempotency/Error/Event semantics
- [ ] 未创建第二个 SessionManager/Runtime owner
- [ ] 未让 Production 自动启动媒体管线
- [ ] 未把 multi-input 静默降级为 single-input
- [ ] 未引入 Universal CommandExecutor / Scheduler / HA 等提前架构

## 进度状态

```text
P0 V0.3 baseline              [x]
P1 S1 read-only audit         [x] GO
P1 S2-A capability boundary   [x] GO
P1 S2-B modification plan     [x] GO
P1 S2-C final read-only audit [ ] NEXT
P1 Implementation             [ ] HOLD
P1 Verification               [ ] HOLD
P1 Control Loop Gate          [ ] HOLD
PR #31 Merge Gate             [ ] HOLD
P2                            [ ] HOLD
v0.3.0 release                [ ] FORBIDDEN UNTIL RELEASE GATE
```

## 进度记录规则

每次推进至少更新：

1. 当前阶段与 Gate；
2. TODO 勾选状态；
3. 实际 commit SHA；
4. 测试/CI 证据；
5. 阻塞项与风险；
6. 下一步唯一动作。

**禁止用“已完成”替代证据。**