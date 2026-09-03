---
comet_change: a2-8-dual-input-switch
role: technical-design
canonical_spec: openspec
status: gate-frozen
---

# Design Doc — a2-8-dual-input-switch（A2-8: Dual-input Switch — Probe CLOSED / Gate Frozen）

> A2-8-00 已交付并 **CLOSED**（[sot-probe 报告](../reports/2026-09-03-a2-8-dual-input-switch-sot-probe.md)）；
> OQ-1..5 已两轮终裁，A2-8-01 经 **Pre-Implementation Gate 十项冻结**后
> APPROVED（全部落 [probe §7](../reports/2026-09-03-a2-8-dual-input-switch-sot-probe.md)）。
> A2-8 定位转折：首次进入 Program Semantic → Execution Adapter →
> GStreamer Graph 实现层。

## 1. 探针决定性事实

- 多输入独立运行已具备（N-input Session + A1 真机 inputs=2）；**缺的是
  switch 执行基础**：GStreamer 无 Switch 节点（实链 src→caps→tee）/
  watchdog 单 Pipeline 视角（L403 首 handle）/ switch_mode=单路计划内
  预留 intent；
- 盒上能力实查：input-selector（active-pad/switch-mode=frame-boundary
  原生）/ intervideosink·src（跨管线隧道）/ audiomixer / valve 全在；
- 双路独立输出≠切换（12 红线 #11）；单输出承诺维持（#10 不做多输出）。

## 2. 裁决面（已终裁 2026-09-03）

OQ-1 inter 系=**候选 Materialization 非架构合同**（topology=实现细节，
换拓扑不得触及 Program Domain）· OQ-2 独立 Switch Execution Adapter ·
OQ-3 ExecutionGroup 级 MultiInputWatchdog（概念冻结：Session=生命周期 /
ExecutionGroup=inputs+active source+switch+program graph+group observation /
Watchdog=四观测）· OQ-4 六路 PTS observation only（禁 Engine/禁 threshold
进 MasterJoin）· OQ-5 Program pipeline 归 Program Execution/Switch 层。

## 3. No-Build Gate（00 已定格）

零 .rs diff；12 红线 + 禁 PipelinePlan 塞 A/B（Semantic Intent≠Execution
Plan≠Execution Fact）。

## 4. A2-8-01 设计边界（Gate 十项冻结 + T1-T12）

- 依赖链：SwitchPolicy→SwitchIntent→SwitchExecutionPlan→Switch Execution
  Adapter→{Input A/B Pipeline, Program Graph}→Program Output→Observation/
  PTS→RuntimeEvent→Watchdog→Health/Custody；SessionManager lifecycle
  only / Supervisor recovery only；
- 状态三分离：Desired（ACTIVE_A/B/SWITCHING）≠ Execution（selector pad）≠
  Observed（actual pad/PTS/frames）；禁 Session.active_input /
  SessionInput.is_active；
- **Video/Audio 成对切换（方案 A）**；audio ≠ 顺手 audiomixer 冒充已解决；
- T4：element property ≠ PASS，须实证切换+B 成为 source+output 存活；
- Event Identity Debt（PipelineFault.pipeline）不修，新增代码不扩大歧义；
- 01 完成标准=真实 Execution Graph+真实 A/B 切换+MultiInputWatchdog 落地。

## 5. 收口链

01 实现 → 02 真机 → 03 failure/supervision → 04 AV continuity →
05 archive+CI+merge；**A2-8 NOT CLOSED until 05**（MASTER_SWITCH /
auto-failover / PACKET_SWITCH 全程 Deferred）。

