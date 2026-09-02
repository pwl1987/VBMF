---
comet_change: a2-7-execution-materialization
role: technical-design
canonical_spec: openspec
status: probe-stage
---

# Design Doc — a2-7-execution-materialization（A2-7: Execution Materialization — SoT/Ownership Probe Stage）

> 当前处于 **A2-7-00 SoT/Ownership Probe 阶段**（用户裁定 Gate 链第一刀，
> 分账特殊要求）。证据产物 =
> [sot-probe 报告](../reports/2026-09-03-a2-7-execution-materialization-sot-probe.md)。
> 编码期设计（Fact 形态/Custody 挂载/advance 映射）在 OQ-1..5 裁决后补入。

## 1. 分账定调（本阶段最重要结论）

**Materialization 不是 A2-7 的空白**——materialize（L529-549）/MediaBackend
SPI 五方法/Session 冻结链/watchdog b1-b3+a4 闩锁/Bus 词表全部已存在，禁重造。
**空白 = Program Semantic Lifecycle**：Execution Fact boundary / Custody /
三 Master writer / Metadata producer / JoinInput 装配，全零。

**核心问题**："什么运行时事实、什么时刻、以什么 ownership/SoT/语义等级，
允许推动 Program Domain 状态前进？"

## 2. 关键披露（探针新增）

当前管线拓扑 = `src → caps → (normalize 可选) → tee → appsink(+outputs)`，
**无独立 Switcher/Composition/Mixer 执行节点**——`SWITCHED/PROGRAM_COMPOSED`
（及 Audio MIXED）的执行事实当前不存在。推进方式（deferred vs Custody
声明性推进）交 OQ-1，不为闭环伪造事实。

## 3. 裁决面（交用户）

OQ-1 stage 事实映射 · OQ-2 Metadata producer · OQ-3 failed 转换边界 ·
OQ-4 AVSync 上游 · OQ-5 Custody 挂载层。

## 4. No-Build Gate

零 .rs diff；十项禁止清单（禁重写四模块/禁 Watchdog·Supervisor 改造/
禁 AVSync Engine/禁 Query·Transport/禁 A2-8/禁万能 ExecutionFact）；
01 前不定义 ExecutionFact 形态。

## 5. 裁决后路线（占位，勿执行）

01 Execution Fact Shape/Ownership → 02 Program Runtime Custody →
03 Execution→Master→Join→Snapshot → 04 Mock/Simulation lifecycle 验证 →
05 真机前置验证 → A2-8 双输入真机切换。
