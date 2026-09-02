# Verify 报告 — A2-3 Audio Master（a2-3-audio-master）

- **Change**: `a2-3-audio-master`（full workflow, skip_specs:true）
- **分支**: `comet/a2-3-audio-master`（base `b68830c` = master, A2-2 收口点）
- **代码提交**: `455aa61`（design doc 顺序修正）→ `ca2d8e4`（代码+产物）→ `fe98dfa`（review 修复）→ `5dd6e56`（fmt 对齐）
- **日期**: 2026-09-02
- **Design Doc**: `docs/superpowers/specs/2026-09-02-a2-3-audio-master-design.md`

## 0. 结论

**Program Domain 第三块落地。** AudioMasterStage 阶段机（§3.7 Audio Graph 逐节点）+
`advance_to` 显式目标白名单（5×5 全组合矩阵, from/to 载荷 wire 名逐字断言）+
**RawAudio 类型层锁**（Errata-3）+ MixLayout 封闭词表 + **Audio 独有事实位**
（delay_ms/loudness_lufs/mix_layout 携带不变）+ DEFAULT_DELAY_MS=80 const 锁
（不通过 serde default 引入——A2-2 立规）。声明性 only, 零行为变化。

## 1. 交付内容

- `AudioMasterStage`（SOURCE_RAW/MIXED/LOUDNESS_NORMALIZED/DELAY_COMPENSATED/
  MASTER_JOINED——§3.7 Audio Graph 节点逐一对应, serde 名 LOCK FINAL）
- `advance_to(target)`: 白名单相邻唯一; 5×5 全组合矩阵测试; `from/to` 载荷逐字
  wire 名断言（含 reject 案例）; no-arg `advance()` sugar
- `AudioDataPlane` 唯一 `RawAudio`（Errata-3; serde `"COMPRESSED"`/`"AAC"` 拒绝）
- `MixLayout`（STEREO/FIVE_ONE/STEREO_AND_SUB; wire↔variant 恒等锁; 大小写敏感+
  跨词表污染 fail-closed）
- **Audio 独有事实位**: `delay_ms: Option<NonZeroU16>`（None=未声明; NonZero 使
  Some(0) 不可表达）; `loudness_lufs: Option<f32>`（None=未归一化; **PartialEq only**
  ——f32 不实现 Eq/Hash, 声明面无 Hash 需求, 文档记档）; `mix_layout`（Master 侧
  混合目标布局声明面——与 audio.rs `AudioLayout`（Source 侧观测布局）**不同概念**,
  映射姿态 design doc §6 记档, deferred to A2-7 裁定）
- `DEFAULT_DELAY_MS = 80`（const 锁 = V0.2 §3.7; 不通过 serde default 引入）
- `is_program_scope_master()` 终态判定
- **A2-2 立规全部遵守**: serde(default) 禁用（{} 与缺 mix_layout 拒绝测试锁）;
  信任边界文档化; advance_to 显式目标; 产物随代码同步提交

## 2. 测试证据

- **+8 测试**（mock 265 → **273**）: 词表+serde 锁 / 5×5 advance_to 矩阵（from/to
  载荷逐字断言）/ RawAudio 唯一 / MixLayout wire↔variant 恒等+拒绝 /
  DEFAULT_DELAY 常量锁 / 事实位携带不变 / 结构 serde+缺字段拒绝（含单字段）/
  终态判定
- **全回归零退化**: 矩阵 14/14; clippy 四组合零警; 硬件电池（lifecycle ALL PASS
  via gates bin / P1a 12 / P1b 11 / transport 19/0）

## 3. Review Gate（standard, subagent 全 change @ca2d8e4）

裁决 **With fixes**: 0 Critical / 2 Important / 6 Minor——全部处置:
- **Important#1（proposal 验收场景 4 词表 `5_1` 与代码 `FIVE_ONE` 矛盾——LOCK FINAL
  契约对齐）**: 已修——proposal 修正为 FIVE_ONE + 注明 codebase SCREAMING_SNAKE_CASE
  惯例（V0.2 无强制格式, prose "5.1" 非词表锚）。
- **Important#2（MixLayout 与 audio.rs AudioLayout 词汇重叠无文档映射——A2-7
  materialization 时会踩坑）**: 已修——design doc §6 记档**不同概念**（Source 侧
  观测 vs Master 侧混合目标）+ 映射缺口（Mono/SevenOne/StereoAndSub）显式
  deferred to A2-7 裁定。
- Minor#3（Audio Delay 节点位置 doc 注释错）: 已修; #4（MixLayout matches!
  同义反复）: 已修——per-pair to_string 恒等锁; #5（矩阵载荷仅验类型未验内容）:
  已修——reject 案例逐字断言 from/to; #6（advance 双写 next_stage 建议）:
  **接受记档**——与 VideoMaster 对称保持一致性, DRY 建议统一在 A2-5 Join 时评估;
  #7（终态 arm 硬编码）: 已修——as_wire(); #8（缺单字段测试不如 Video 深）:
  已补——mix_layout 单字段移除断言。

## 4. 冻结点

- 阶段词表 LOCK（§3.7 节点逐一对应; serde 名 = wire 契约锚）
- data_plane RawAudio 唯一（Errata-3 纪律同 Video）
- DEFAULT_DELAY_MS 仅 const 锁; MixLayout 三词 LOCK FINAL
- AudioLayout↔MixLayout 映射 deferred to A2-7
- 声明性 only: 无 mix/loudness/delay 执行（A2-7+）/无 Metadata Master（A2-4）/
  无 Join（A2-5）

## 5. CI（PR 后回填）

七 required context: **见 PR 检查记录（合并前全 green 为 Merge Gate 前置）。**
