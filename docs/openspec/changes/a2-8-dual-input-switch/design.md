# Design — a2-8-dual-input-switch（A2-8-00 SOT Probe）

## 1. 定位

探针首刀：回答"双输入 Program-level switch 的执行基础"六问；A2-8 转折 =
从 Domain 模型线转入 Execution Adapter 实现线。

## 2. 方法论

事实断言先复核（8 项全锚）；GStreamer 能力盒上实查（非文档推测）；缺口
如实披露（watchdog 单视角=Precondition Gate；AVSync 升级 A2-8 硬前置）。

## 3. 裁决面

OQ-1 Program graph 形态（inter 系倾向）· OQ-2 Switch Adapter 形态（独立
trait 倾向）· OQ-3 MultiInputWatchdog · OQ-4 AVSync 测量边界 · OQ-5 构图归属。

## 4. No-Build Gate

零 .rs diff；12 红线；禁 PipelinePlan 硬塞 A/B。

## 5. 后续（OQ 终裁后）

01 最小 FRAME_SWITCH Execution Switch（T1-T5 验收：ACTIVE/STANDBY 双向
显式切换；自动 failover 留后）→ 真机验证 → 收口。
