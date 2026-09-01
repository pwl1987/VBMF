# Verification Report: p07d-event-integration（0.7D Event Integration — 事件内消费集成）

- 日期：2026-09-01
- Change：`docs/openspec/changes/p07d-event-integration`（spec-driven, skip_specs: true）
- 分支：`comet/p07d-event-integration` @ `1f5ecea`（base `456e753`，0.7C-8 master）
- 规模：13 tasks / 34 changed files → verify_mode=**full**（openspec CLI 实读：13/13 complete）
- 方法：实物 SoT（git diff 456e753..HEAD + 当前工作树代码实读 + 盒上 2026-09-01 真机/矩阵证据），不采信自报。

## Summary

| Dimension    | Status |
|--------------|--------|
| Completeness | 13/13 tasks [x]；无 delta spec（skip_specs，与前序 15 change 一致，行为契约锚定冻结文档） |
| Correctness  | proposal 6 项 What Changes + 6 项 Non-Goals 全部有代码实证；transport.rs 零 diff |
| Coherence    | design.md D1-D6 全部落实；Design Doc §5 锚点表与实际实现一致；Open Questions 两项均定稿 |

## 1. Completeness

- `openspec instructions apply --change p07d-event-integration --json`：progress 13/13 complete（tasks.md 与 openspec 解析一致）。
- 无 delta specs（proposal Capabilities 节 `skip_specs: true`，裁定理由=SoT 为 0.7C-6 归档 design §4 deferral 清单 + EVENT_CONTRACT §1/§2 + MEDIA_AGENT_STATE_MACHINE 8 态词汇）。规格覆盖检查按优雅降级跳过，正确性维度以 proposal 目标为覆盖基准。
- 4.2/4.3/4.4 三条 NOTE 完整记录覆盖归属与真机证据（Signal/Loopback 计数归 4.3；盒上矩阵 14/14 + CI 拆分）。

## 2. Correctness（proposal 目标 → 代码实证）

| # | 目标 | 实证 | 判定 |
|---|------|------|------|
| G1 | Health Reducer 完整实现 | `health.rs:119` `pub fn reduce(state: &HealthFold, events: &[RuntimeEvent]) -> HealthFold` 纯函数（无 I/O 参数）；110-118 行映射表注释逐态定稿（Released→Ready 重置边 / Running+SignalVerified→Capturing / ReservationExpired→Degraded / 微相位不偷改）；不足态诚实登记（Restarting/Backoff 无事件生产者、本期不派生）；8 个 unit test 含 `reduce_is_pure_deterministic`（同输入同输出） | ✓ |
| G2 | main.rs 散写收敛 | base 8 个可变写点中 watchdog 环内 3 个（1467/1483 ManualRequired + 1488 Capturing）全部删除，收敛为 `drain internal_log → health::reduce → 写回` 单一写点（main.rs:1500 注释 + 1648 写点）；保留 5 个均为 bootstrap/命令边（499 声明 / 573 selftest / 1388 auto_start / 1410,1415 失败边 / 1431 production ready），与映射表"SignalVerified 承载 selftest 无会话路径 Capturing 派生"自洽；E2 真机实证 derived_during_running=Capturing、final=Degraded（gate 段无旁路写入点） | ✓ |
| G3 | Supervisor 事件驱动消费 | `supervisor.rs` 新增纯函数 `fault_trigger_from_events`（+107 行含测试，diff 中唯一 pub fn 变化=新增；report_failure/begin_restart/report_recovered 调用面零变更）；watchdog tick 消费点接线（main.rs:843 注释链：internal drain→reduce→写回） | ✓ |
| G4 | 4 事件点亮 | IdentityResolved=session.rs:496（create() binding-verify）/ SignalVerified=main.rs:1635（watchdog a4 首帧+PTS 单调闩锁，恰一次）/ LoopbackVerified=main.rs:350（VBMF_LOOPBACK all_pass 验收点）/ ResourceReservationExpired=session.rs:869（tick 预留过期）；events.rs +108 纯新增（7 个 test fn，0 删除）→ 词表/serde tag/平面零改动 | ✓ |
| G5 | housekeeping 三项 | rpc.rs diff 纯注释（"No transport yet" → transport.rs 为当前 HTTP 边界 + 冻结 SoT §14 记录不在 wire 路径；非注释 diff=0）；p07c-{error-model,event-projection,external-api} 三目录已删（归档件 diff=仅 checkbox 状态差，15ac1cd）；Phase Map 0.7D 行再锚定 + 债表 D8 CLOSED @0.7C-6 + 0.7D Contract Probe 定层记录 | ✓ |
| G6 | EVENT-INTEGRATION-RT-01 三层 | Unit=health.rs 8 tests；Simulation=evt_int_rt_01 ×3（supervisor.rs:442 回声不自激 / session.rs:1761 真实 SessionManager 事件流经 FanoutSink 折叠 / session.rs:1843 预留过期派生 Degraded）；Hardware=main.rs gate 段（lifecycle E1-E3/E5-E8 + loopback E4），盒上双入口 ALL PASS exit 0（2026-09-01，证据见 tasks.md 4.3 NOTE） | ✓ |

Non-Goals 核验：transport.rs 零 diff ✓；`/health` 字段逐字段不变（transport_ctx 回归锚 + EXTERNAL-API-RT-01 真机 OK）✓；零新 crate（diff 面 7 文件全在 src/ 内）✓；不重做 Projection/不做外部投递/不做持久化 ✓。

真机证据（4.3 NOTE，盒上 2026-09-01 bmd,gstreamer dev 二进制）：E1 identity_resolved=5/session_created=5；E2 Capturing→Degraded；E3 signal_verified=1（A+B+C 全过 236 帧）；E4 loopback_verified=1（期望 1）；E5 expired=1+phase Terminated；E6 pipeline_fault=0；E7 投影 60 条完整 + internal 残留 10 条干净 drain（dropped 0/0）；回归 SESSION/RESOURCE/COMMAND-CONTRACT/IDEMPOTENCY/ERROR-MODEL/EVENT-PROJECTION/EXTERNAL-API 全 OK。改动后矩阵 14/14 exit 0、零警告、155/155/215/155。

## 3. Coherence（design.md 决策 → 实现）

| 决策 | 实现形态 | 判定 |
|------|----------|------|
| D1 reducer=纯函数 | `reduce(&HealthFold, &[RuntimeEvent]) -> HealthFold`；消费侧只读，不写回 Graph/Backend/Command 路径 | ✓ |
| D2 消费点=watchdog tick 接线层 | 无新线程（沿用既有 5s tick 节拍模式）；health.rs 只持纯语义 | ✓ |
| D3 单日志多消费者（design 阶段定稿） | **双日志分流**：组合根 FanoutSink 同序双写 projection_log + internal_log（main.rs:260-268），各日志恰一消费者（投影端点 / watchdog），drain 竞争结构性消除；transport 零改动 | ✓ |
| D4 Supervisor 输入侧演进 | 故障类事件视图 → report_failure 语义等价；调用面/决策纯度不变 | ✓ |
| D5 4 事件锚定真实语义路径 | 四个 emit 站点全部锚定语义真实触发点（G4 实证），Simulation kind_counts 精确 + 真机计数精确 | ✓ |
| D6 红线守护 | reducer 输出仅进 /health（transport_ctx.agent_state）+ watchdog 观测面；未进任何 Command/配置路径 | ✓ |
| Open Questions | D3 形态（双日志分流）+ 8 态×事件映射表——均已在 Design Doc/health.rs 注释定稿并有测试锁定 | ✓ |

## 4. Issues

### CRITICAL
无。

### WARNING
无。

### SUGGESTION
1. `health.rs:14` 模块级 `#![allow(dead_code)]` 保留（task 1.2 字面为"去"）。实际已收窄：注释显式声明仅覆盖冻结 8 态词表中无生产构造点的词汇完整性项（Starting/Restarting/Backoff/Escalated，SoT §15.2 词表冻结，不伪造构造点），reducer 活路径不再被 blanket 覆盖——任务意图（skeleton allow 不再罩住死骨架）已达成。如需字面收紧，可改为对四个 variant 逐点 `#[allow(dead_code)]`。不阻塞。

## Final Assessment

**0 CRITICAL / 0 WARNING / 1 SUGGESTION。** 全生命周期证据链完整（design 定稿 → 三层门禁 → 真机 E1-E8 → 盒上矩阵 → tasks NOTE），transport/词表/Supervisor 调用面三条红线均有零 diff/零变更实证。Ready for archive（SUGGESTION 项可留待后续，不阻塞）。
