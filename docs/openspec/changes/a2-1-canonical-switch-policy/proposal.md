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
