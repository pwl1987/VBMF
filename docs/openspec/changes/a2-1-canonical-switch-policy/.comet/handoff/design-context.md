# Comet Design Handoff

- Change: a2-1-canonical-switch-policy
- Phase: design
- Mode: compact
- Context hash: 9c79a3def5e1a950a6a0f7fe62f7f5a01a946ca4b1de633a049435ed6a8c3a8a

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/a2-1-canonical-switch-policy/proposal.md

- Source: docs/openspec/changes/a2-1-canonical-switch-policy/proposal.md
- Lines: 1-40
- SHA256: f9a8e093bef9a22b6621c989d19ad2c0e06846f21c614bb91f862361d2cd5f4b

```md
# Proposal — a2-1-canonical-switch-policy

## Why

用户裁定的 A2-1 起点（A2-0 结构债务清零后 Program Domain 第一个 Canonical Domain Object）。
Reality Audit 实证: `PipelinePlan.switch_mode: String = "FRAME_SWITCH"` 是**从未被消费的占位
字符串**（V0.2 词表在代码中零实现）——与 P1a 前 `sink.kind` 同病（占位无牙齿）。

V0.2 §1.17 权威语义（LOCK FINAL）: 主备切换**必须显式按切换粒度分模式**——
- `PACKET_SWITCH`: 压缩码流层切（GOP 对齐/SPS/PPS/时间戳连续性; 主备 codec+profile 完全一致）
- `FRAME_SWITCH`: 主备都先 decode → RAW_VIDEO 层切 → 重新 encode（codec 不同/跨格式）
- `MASTER_SWITCH`: 主备都先 normalize → 统一输出格式 → 切（不同设备/不同色域/异构）
且 V0.2 §313-315 锁定各模式的 IO 平面（PACKET: COMPRESSED_*→COMPRESSED_*;
FRAME: RAW_*→RAW_*; MASTER: RAW_* post-normalize→RAW_*）。

## What Changes

- **新 `program` 模块**（lib.rs A2-0 腾位锚落位——Program Domain 第一块）:
  `SwitchPolicy` 封闭 enum（三模式词表快照 + 未知 fail-closed——同 sink.kind 词表纪律）
  + 模式语义访问器（IO 平面/前置约束, 回答"是什么"不"怎么执行"）。
- **`PipelinePlan.switch_mode: String` → 类型化**: 占位字符串被 `SwitchPolicy` 取代——
  词表第一次有牙齿（materialize 侧 fail-closed; 序列化名与 V0.2 §1.17 逐字对齐）。
- **Program Domain 骨架锚**: `program` 模块只含 SwitchPolicy（Channel/Masters/MasterJoin/
  ProgramMaster 属 A2-2+——不提前实现）。
- 零执行变化: 本 change 不实现任何切换执行（GStreamer Materialization 属 A2-7）;
  单输入行为逐字节不变。

## Non-Goals

- 切换执行/GStreamer compositor（A2-7）; Video/Audio/Metadata Master/Master Join（A2-2..5）;
  Channel 完整模型（控制面 A4 线）; Hot-Standby 三级/failover 语义（Alpha-5/V0.3）;
  双输入真机切换验收（A2-8）

## 验收场景

1. 词表快照: 恰三词, serde 序列化名与 V0.2 §1.17 逐字一致（PACKET_SWITCH/FRAME_SWITCH/MASTER_SWITCH）
2. 未知值 fail-closed（生产/诊断一致拒绝, 绝不静默回退 FRAME_SWITCH）
3. IO 平面访问器与 §313-315 一致; 各模式前置约束可查询（"是什么"）
4. `PipelinePlan` 类型化后序列化兼容（wire 上 switch_mode 字段值不变）
5. 单输入运行时行为零变化（全回归: 矩阵/mock 251/P1a/P1b/gates bin）

```

## docs/openspec/changes/a2-1-canonical-switch-policy/design.md

- Source: docs/openspec/changes/a2-1-canonical-switch-policy/design.md
- Lines: 1-26
- SHA256: 0283f62188df4092d57b81842559c204d2e2ad5bb4e02cbb2cba78647f202605

```md
# Design — a2-1-canonical-switch-policy（高层框架）

## D1 SwitchPolicy = Canonical Domain Object（program 模块首块）

```
src/program/mod.rs        // Program Domain（A2-2+ 续: Masters/MasterJoin/ProgramMaster/Channel）
src/program/switch_policy.rs
```

- 封闭 enum 三变体, serde rename_all = "SCREAMING_SNAKE_CASE"（wire 名与 V0.2 逐字一致）
- `SwitchPolicy::parse(&str) -> Result<Self, _>`: 词表外 fail-closed（错误信息含受纳词表——
  同 sink.kind 纪律）; serde 反序列化路径同 fail-closed
- 语义访问器（只描述不执行, Observation≠Configuration 纪律同源）:
  `io_plane()`（§313-315 IO 平面）, `precondition()`（§1.17 适用条件摘要）

## D2 PipelinePlan 类型化

`switch_mode: String` → `switch_mode: SwitchPolicy`（默认 `FRAME_SWITCH` = 现占位值不变,
wire 兼容）; `materialize` 无 SwitchPolicy 输入源（intent 尚无该字段——A2-6 投影时接入）,
故本期默认值即全部来源; 测试字面量同步。

## D3 边界

- 不触碰 pipeline 执行/GStreamer/输出——switch_mode 仍是**声明**（V0.2: Intent 是声明,
  执行是 Plan/Backend 的事; 本 change 让声明从字符串升为类型）
- lib.rs 腾位锚注释更新为真实模块声明

```

## docs/openspec/changes/a2-1-canonical-switch-policy/tasks.md

- Source: docs/openspec/changes/a2-1-canonical-switch-policy/tasks.md
- Lines: 1-17
- SHA256: 6ea44576af42cf970240b75618a693f017bca5142820fc4062a5aea0e506787e

```md
# Tasks — a2-1-canonical-switch-policy

> 四栏纪律。TDD; cargo 经盒; 基线 mock 251。

## 1. program 模块 + SwitchPolicy（TDD）

- [ ] 1.1 RED: 词表快照（恰三词/serde 名逐字/parse 受纳+拒绝含大小写敏感）/ IO 平面+前置约束访问器 / serde 反序列化未知值 fail-closed `Contract: V0.2 §1.17+§313-315` | `Implementation: 待` | `Verification: Unit` | `Gate: 无`
- [ ] 1.2 GREEN: `src/program/{mod,switch_policy}.rs` + lib.rs 锚转真实声明 `Contract: design D1` | `Implementation: 待` | `Verification: Unit 全绿` | `Gate: 无`

## 2. PipelinePlan 类型化

- [ ] 2.1 RED+GREEN: `switch_mode: SwitchPolicy`（默认 FRAME_SWITCH 不变）; 测试字面量同步; wire 序列化值不变断言 `Contract: design D2` | `Implementation: 待` | `Verification: Unit + mock 251 零回退` | `Gate: 无`

## 3. 回归 + 交付

- [ ] 3.1 全回归（矩阵/gates bin 双 gate/P1a/P1b/transport）零退化 `Contract: 验收口径` | `Implementation: 待` | `Verification: 盒上全 PASS` | `Gate: BOX`
- [ ] 3.2 review + verify 报告 + 双 guard + archive + PR + CI + merge + memory `Contract: 交付纪律` | `Implementation: 待` | `Verification: PR merged` | `Gate: CI/RELEASE`

```
