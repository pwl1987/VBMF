---
comet_change: a2-3-audio-master
role: technical-design
canonical_spec: openspec
---

# Design Doc — a2-3-audio-master（A2-3: Audio Master）

基线 master `b68830c`（A2-2 Video Master 收口点）。Program Domain 第三块。

## 1. 权威语义锚（V0.2 probe 实证）

- §3.7 Audio Graph（行 788-796 完整）:
  ```
  Source ↓(RAW_AUDIO) → [Audio Mixer] → [Loudness] → [Audio Delay] (+80ms 补偿) → [Audio Master Join] → Program-scope Master (RAW_AUDIO)
  ```
- §1.20（行 153）: "Audio Delay = +80ms 是 Audio Graph 内部的事, 不是 Video Graph 的事"
- Errata-3（行 3037 等多处）: Encode = delivery boundary; Program-scope Master = **RAW 域**
- A2-2 立规（已应用）: serde(default) 新生儿禁用 / 产物随代码同步 commit / advance_to 显式目标
  / 信任边界文档化 / `{from,to}` 载荷 wire 词表名

## 2. 类型设计

```rust
// src/program/audio_master.rs
#[derive(...)] #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AudioMasterStage {
    SourceRaw, Mixed, LoudnessNormalized, DelayCompensated, MasterJoined,
    // §3.7 Audio Graph 节点链（声明性 — 不含执行）
}
#[derive(...)] pub enum AudioDataPlane { #[default] RawAudio }
#[derive(...)] pub enum MixLayout { #[default] Stereo, FiveOne, StereoAndSub }
pub struct AudioMaster {
    pub stage: AudioMasterStage,
    pub data_plane: AudioDataPlane,
    pub mix_layout: MixLayout,
    pub delay_ms: Option<NonZeroU16>,    // None=未声明
    pub loudness_lufs: Option<f32>,      // None=未归一化
}
pub const DEFAULT_DELAY_MS: u16 = 80;   // V0.2 §3.7 锁定（仅 const, 不引入 serde default）

impl AudioMaster {
    pub fn new() -> Self;                 // SourceRaw + RawAudio + Stereo + None + None
    pub fn advance_to(target) -> Result;   // 白名单相邻唯一
    pub fn advance() -> Result;            // next-step sugar
    pub fn is_program_scope_master() -> bool;  // stage == MasterJoined
}
```

- ProgramDomainError 复用（InvalidStageTransition from/to = as_wire 名）
- advance_to 白名单: SourceRaw→Mixed; Mixed→LoudnessNormalized;
  LoudnessNormalized→DelayCompensated; DelayCompensated→MasterJoined; MasterJoined=None
- mix_layout / delay_ms / loudness_lufs 在 advance 中**携带不变**（同 Video composition
  携带规则——独立的事实/声明字段不参与阶段迁移）

## 3. 测试策略

- 词表快照五词 + serde 锁 + advance_to 5×5 全组合矩阵
- RawAudio 唯一 + 压缩域 serde 拒绝（Errata-3）
- mix_layout 受纳+拒绝（含大小写敏感/空串/跨词表污染）
- DEFAULT_DELAY_MS 常量 == 80 锁; advance 携带 delay/loudness/mix_layout 不变
- 结构级 serde 往返 + 缺字段 fail-closed（立规: 新生儿类型不静默默认）
- is_program_scope_master 终态判定
- 全回归零退化: mock 265 基线 + 矩阵 + clippy 四组合

## 4. 冻结点

- 阶段词表 LOCK（§3.7 节点逐一对应）
- data_plane RAW 唯一（Errata-3 纪律）
- DEFAULT_DELAY_MS 仅 const 锁（不引入 serde default）
- advance_to 无通配臂; `{from,to}` 载荷 wire 词表名
- 声明性 only: 无 mix/loudness/delay 执行（A2-7+）/无 Join（A2-5）

## 5. CI（PR 后回填）

七 required context: **见 PR 检查记录（合并前全 green 为 Merge Gate 前置）。**
