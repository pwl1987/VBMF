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
