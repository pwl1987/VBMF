---
comet_change: a2-8-dual-input-switch
role: technical-design
canonical_spec: openspec
status: probe-stage
---

# Design Doc — a2-8-dual-input-switch（A2-8: Dual-input Switch — SOT Probe Stage）

> A2-8-00 已交付（[sot-probe 报告](../reports/2026-09-03-a2-8-dual-input-switch-sot-probe.md)），
> OQ-1..5 待终裁。A2-8 定位转折：首次进入 Program Semantic → Execution
> Adapter → GStreamer Graph 实现层。

## 1. 探针决定性事实

- 多输入独立运行已具备（N-input Session + A1 真机 inputs=2）；**缺的是
  switch 执行基础**：GStreamer 无 Switch 节点（实链 src→caps→tee）/
  watchdog 单 Pipeline 视角（L403 首 handle）/ switch_mode=单路计划内
  预留 intent；
- 盒上能力实查：input-selector（active-pad/switch-mode=frame-boundary
  原生）/ intervideosink·src（跨管线隧道）/ audiomixer / valve 全在；
- 双路独立输出≠切换（12 红线 #11）；单输出承诺维持（#10 不做多输出）。

## 2. 裁决面（交用户）

OQ-1 Program graph 形态[inter 系倾向] · OQ-2 Switch Adapter[独立 trait
倾向] · OQ-3 MultiInputWatchdog[单 watchdog 服务 execution group 倾向] ·
OQ-4 AVSync 测量边界[A2-8 硬前置] · OQ-5 构图归属[与 OQ-1 联动]。

## 3. No-Build Gate

零 .rs diff；12 红线 + 禁 PipelinePlan 塞 A/B（Semantic Intent≠Execution
Plan≠Execution Fact）。

## 4. 裁决后路线

01 最小 FRAME_SWITCH Execution Switch（显式切换 T1-T5；PALETTE/MASTER
Deferred；自动 failover 留后——需 Runtime→Custody→Policy→Switch Intent
生产链）→ 真机验证 → 收口。
