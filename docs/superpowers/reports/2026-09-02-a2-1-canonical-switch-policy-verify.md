# Verify 报告 — A2-1 Canonical SwitchPolicy（a2-1-canonical-switch-policy）

- **Change**: `a2-1-canonical-switch-policy`（full workflow, skip_specs:true）
- **分支**: `comet/a2-1-canonical-switch-policy`（base `eb337dc` = master, A2-0 收口点）
- **代码提交**: `4bf1325`（+review 修复 `680d4e7`）
- **日期**: 2026-09-02
- **Design Doc**: `docs/superpowers/specs/2026-09-02-a2-1-canonical-switch-policy-design.md`

## 0. 结论

**Program Domain 第一个 Canonical Domain Object 落位。** V0.2 §1.17 三模式词表从
"从未被消费的占位字符串"（Reality Audit Engine-7 实证 ~5%）升级为**有牙齿的类型**:
封闭 enum + serde 名逐字锁定 + parse/serde 双路径 fail-closed + §313-315 IO 平面访问器。
声明性 only——零切换执行、零 GStreamer 触碰、单输入行为逐字节不变（全回归实证）。

## 1. 交付内容

- `src/program/{mod,switch_policy}.rs`: SwitchPolicy（PacketSwitch/FrameSwitch/MasterSwitch;
  serde `SCREAMING_SNAKE_CASE` wire 名逐字 = V0.2 §1.17 LOCK FINAL）; `parse` 词表外
  fail-closed（错误含受纳词表——sink.kind 同纪律）; serde 反序列化同牙齿（无 default）;
  `SwitchIoPlane`（§313-315: COMPRESSED→COMPRESSED / RAW→RAW / NORMALIZED-RAW→RAW）;
  `precondition()` 适用条件摘要（"是什么"非执行）; `ProgramDomainError`（A2-2+ 复用）;
  `ACCEPTED_LIST`（错误信息与测试共用快照）
- `lib.rs`: A2-0 腾位锚注释 → 真实 `pub mod program;`——**Program Domain 落位**
- `PipelinePlan.switch_mode: String → SwitchPolicy`（默认 FrameSwitch = 旧占位值;
  6 构造字面量 + 1 断言类型化; wire JSON 值不变——兼容锚测试锁 `"FRAME_SWITCH"` 逐字）

## 2. 测试证据

- **+8 测试**（mock 251 → **259**）: 词表快照恰三词 / serde 名逐字锁 / **parse↔variant
  恒等**（review Minor#2 补——match 臂交换逃不过）/ parse 受纳+拒绝（含大小写敏感/空串/
  跨词表污染 RTMP/HLS/APPSINK）/ serde 未知串 fail-closed / io_plane 对照 §313-315 /
  precondition 非空 / **PipelinePlan wire 兼容锚**（字段名+值+往返+复合 fail-closed）
- **全回归零退化**: 矩阵 14/14（fmt×2/test×4/clippy×4 零警/build×3/proof OK）;
  双 gate 经 media-agent-gates bin PASS（lifecycle ALL PASS / loopback ALL PASS=true）;
  P1a 12 + P1b 11 + transport 19/0

## 3. Review Gate（standard, subagent 全 change @4bf1325）

裁决 **With fixes**: 0 Critical / 1 Important / 4 Minor——
- **Important#1（Design Doc 未提交——契约必须进历史）**: 已修（680d4e7 提交）。
- Minor#2（parse↔variant 恒等缺环）: 已修（identity 测试, +1）。
- Minor#3（字面量计数 5→6+1）: 已修（Design Doc 对账修正）。
- Minor#4（tasks.md 未勾选）: 已修。
- Minor#5（SwitchIoPlane serde 推测性）: **接受**——词汇表对象语义自洽预锁, review 亦认可
  "保持现状即可, 不要加门控"。
- Review 独立核验确认: V0.2 §1.17 逐字保真（无虚构语义）; io_plane 与 §313-315 1:1;
  wire 兼容真实（全 crate 序列化面 grep——PipelinePlan 唯一序列化点即锚测试, 无其他消费方）;
  fail-closed 双面+大小写敏感; 范围纪律（无执行逻辑/无 Masters/Channel 提前实现）。

## 4. 冻结点

- 三词表 LOCK（V0.2 §1.17 逐字; serde 名 = wire 契约锚, 未来 rename = 破坏性变更）
- 未知值 fail-closed 不豁免（parse/serde 同牙齿）
- wire 兼容: `switch_mode` 字段值域不变（默认 FRAME_SWITCH）
- 不实现切换执行（A2-7）/ 不加 Hot-Standby/failover（Alpha-5/V0.3）

## 5. CI（PR 后回填）

七 required context: **见 PR 检查记录（合并前全 green 为 Merge Gate 前置）。**
