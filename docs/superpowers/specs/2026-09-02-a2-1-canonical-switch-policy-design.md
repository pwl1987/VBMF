---
comet_change: a2-1-canonical-switch-policy
role: technical-design
canonical_spec: openspec
archived-with: 2026-09-02-a2-1-canonical-switch-policy
status: final
---

# Design Doc — a2-1-canonical-switch-policy（A2-1: Canonical SwitchPolicy）

基线 master `eb337dc`（A2-0 结构债务清零点）。Program Domain 第一个 Canonical Domain Object。

## 1. 现状锚点（probe 实证）

- `pipeline.rs:136 pub switch_mode: String` + 6 处 `"FRAME_SWITCH".into()` 字面量 + 1 处断言字面量——
  **从未被消费**（Reality Audit §Engine-7: 占位 ~5%）。V0.2 §1.17 三模式词表代码零实现。
- V0.2 §1.17（LOCK FINAL）: `PACKET_SWITCH`（压缩码流层切; GOP 对齐/SPS/PPS/时间戳连续;
  主备 codec+profile 完全一致）/ `FRAME_SWITCH`（decode→RAW 切→re-encode; codec 不同/跨格式）/
  `MASTER_SWITCH`（normalize→统一输出格式→切; 异构）。
- V0.2 §313-315 IO 平面: PACKET=COMPRESSED_*→COMPRESSED_*; FRAME=RAW_*→RAW_*;
  MASTER=RAW_*(post-normalize)→RAW_*。
- lib.rs A2-0 腾位锚注释（`// A2-1+ 腾位锚: pub mod program; ...`）。
- sink.kind 词表先例（P1a）: parse fail-closed + 受纳词表错误信息 + 快照测试。

## 2. 类型设计

```rust
// src/program/switch_policy.rs
/// V0.2 §1.17 Switch Mode —— Canonical 封闭词表（LOCK FINAL, 序列化名逐字一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwitchPolicy {
    PacketSwitch,  // PACKET_SWITCH
    FrameSwitch,   // FRAME_SWITCH
    MasterSwitch,  // MASTER_SWITCH
}

impl SwitchPolicy {
    pub fn parse(s: &str) -> Result<Self, ProgramDomainError>;  // 词表外 fail-closed
    pub fn io_plane(&self) -> SwitchIoPlane;      // §313-315（枚举: CompressedToCompressed / RawToRaw / NormalizedRawToRaw）
    pub fn precondition(&self) -> &'static str;   // §1.17 适用条件摘要
    pub const ACCEPTED: &'static [&'static str];  // 受纳词表快照（错误信息/测试共用）
}
```

- serde 反序列化未知串: 默认 error（不 custom default——fail-closed 与 parse 同牙齿）。
- `ProgramDomainError::UnknownSwitchPolicy(String)`（program 模块自己的错误类型——A2-2+ 复用）。

## 3. PipelinePlan 类型化

- `switch_mode: String` → `switch_mode: crate::program::SwitchPolicy`
- 默认 `SwitchPolicy::FrameSwitch`（= 现占位值; wire 上 `"FRAME_SWITCH"` 不变——serde 兼容断言）
- 6 处构造字面量 + 1 处断言 → 类型化（语义零变; review 对账后修正计数）
- materialize 不新增输入源（intent 无该字段——A2-6 投影时接入; 本期默认值即全量来源）

## 4. 模块布局

```
src/program/mod.rs           // pub mod switch_policy;（A2-2+ 续: masters/master_join/program_master/channel）
src/program/switch_policy.rs
lib.rs: 腾位锚注释 → 真实 pub mod program;
```

## 5. 测试策略

- Unit: 词表快照恰三词 + serde 名逐字（序列化断言字符串相等）+ parse 受纳/拒绝
  （含 `PACKET_SWITCH` 小写变体/`"SWITCH"`/空串/`RTMP` 类跨词表污染）+ serde 未知串
  fail-closed + io_plane/precondition 与 §1.17/§313-315 对照表断言
  + PipelinePlan 序列化 wire 值 `"FRAME_SWITCH"` 不变（兼容锚）
- 全回归: mock 251 + 矩阵 + gates bin 双 gate + P1a/P1b/transport（零变化证明——
  本 change 纯类型化, 无执行路径触碰）

## 6. 冻结点

- 三词表 LOCK（与 V0.2 §1.17 逐字）; 未知 fail-closed 不豁免。
- 不实现切换执行（A2-7）; 不加 Hot-Standby/failover 语义（Alpha-5/V0.3）。
- wire 兼容: `switch_mode` 字段值域不变（默认 FRAME_SWITCH）。
