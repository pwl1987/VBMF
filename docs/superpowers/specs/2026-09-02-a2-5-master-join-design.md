---
comet_change: a2-5-master-join
role: technical-design
canonical_spec: openspec
status: probe-stage
---

# Design Doc — a2-5-master-join（A2-5: Master Join — SoT Probe Stage）

> 当前处于 **A2-5-00 SoT Probe 阶段**（用户裁定七刀链第一刀）。技术设计 =
> 探针方法论；证据产物 = [sot-probe 报告](../reports/2026-09-02-a2-5-master-join-sot-probe.md)。
> 编码期设计（Join 形态/词表/输入输出模型）在 OQ-A..E 裁决后补入。

## 1. 探针结论摘要

- **联合判定唯一权威句**：§1.20 L155——三 graph 处理层隔离 + Master Join
  一致性判定；任一路 **failed** → Program Master `DEGRADED` 或 `FAILOVER`。
- **§8.9 Master 是 7 故障域之一**（Program Master 失败 → Filler/Emergency，
  切源✅垫片✅）；由 Safety+Watchdog+Health Tree 执行不新增 Engine。
- **§8.10**：AV Sync red（>250ms）先 classify_failure_domain 后动作（消费
  §8.9；PLAYER 绝不切源；UNKNOWN→SAFE_DEGRADE）；绝对规则已删。
- **§8.11 三轴**：health 轴含 UNKNOWN 独立合法值。
- **Errata-9**：AVSync Manager=Measurement+Correction+Classification，
  不做 Recovery（§8.9 是 Recovery SoT；识别/决策分离）。
- **代码现状**：Join/ProgramMaster/AVSync/FAILOVER/READY_TO_TAKE 全零
  （A2-4-04 J1-J9 @1779429 复核未变）；failed 唯一来源 Runtime 平面。

## 2. 十危险点双锚 + OQ-A..E + PD-1..4

见 probe 报告 §3-§5。十危险点全部 V0.2+代码双证据锚定；五问
（Join 输出×§8.9 / ProgramMaster 形态 / AVSync 范围 / classify 归属 /
三路不对称就绪输入）交用户裁决。

## 3. No-Build Gate

零 .rs diff；不动三 Master/Runtime/Event/Health；不冻结词表；D14 语义
禁引用；GStreamer 执行面（A2-7+）不碰。

## 4. 裁决后路线（占位，勿执行）

01 Domain Shape Probe → 02 输入/输出模型裁定 → 03 实现 → 04 ProgramMaster
聚合 + AVSync 边界 → 05 Semantic Deep Review → 06 Verification & Delivery
Closure（矩阵/guards/archive/PR/CI/merge）。
