# Comet Design Handoff

- Change: a2-3-audio-master
- Phase: design
- Mode: compact
- Context hash: efc264ecc4baf507aa31c681397e0f4f5f32b6cfc0c0ca544b7a9a10253db411

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/a2-3-audio-master/proposal.md

- Source: docs/openspec/changes/a2-3-audio-master/proposal.md
- Lines: 1-50
- SHA256: 673203ddc331d2b4ea54169c79b59c0133d0c1ad4dc0a4746c7cfda475a1856c

```md
# Proposal — a2-3-audio-master

## Why

裁定链 A2-3（A2-1 SwitchPolicy @78a8319✅ / A2-2 Video Master @b68830c✅）。V0.2 §3.7
Audio Graph 是**独立 graph**（Source → Mixer → Loudness → **Delay(+80ms 补偿)**
→ Audio Master Join → Program-scope Master (RAW_AUDIO)），与 §1.20 锁定"故障域隔离 +
联合判定"——Audio Delay = Audio Graph 内部的事, 不是 Video Graph 的事。
A2-2 已立规（serde(default) 新生儿禁用 / 产物随代码同步 commit / advance_to 显式
目标矩阵 / 信任边界文档化），本 change 严格遵守。

## What Changes

- **`src/program/audio_master.rs`**（Program Domain 第三块, 与 video_master 同构）:
  - `AudioMasterStage` 封闭词表（`SOURCE_RAW → MIXED → LOUDNESS_NORMALIZED →
    DELAY_COMPENSATED → MASTER_JOINED`——§3.7 Audio Graph 节点逐一对应; serde 名锁）
  - `AudioMaster` { stage, data_plane, mix_layout, delay_ms, loudness_lufs }
    ——**Audio 路径特化字段（Video 无）**: `mix_layout`（mix 声道布局枚举; 与
    Normalize 同律, fail-closed 拒绝未声明布局）、`delay_ms`（延迟补偿; **A2-3
    不写死 +80**——V0.2 +80ms 是默认值; `Option<NonZeroU16>` 仅声明层"是否声明了 +
    多少"; 类型层带 `DEFAULT_DELAY_MS = 80` 常量）、`loudness_lufs`（响度归一化
    事实位; `Option<f32>` 默认 None=未做归一化——控制面设值）
  - `advance()` / `advance_to(target)` 白名单（无通配臂; A2-2 立规——所有 reject 由
    `ProgramDomainError::InvalidStageTransition` 携带真实 wire 词表名）
  - `data_plane` 仅 `AudioDataPlane::RawAudio`（Program-scope Master 一律 RAW——
    与 Video 同 Errata-3 纪律; 压缩域 Master 类型层不可表达）
  - `is_program_scope_master()` 终态判定（Audio 路视角）
- **Audio 独有测试**:
  - 5×5 advance_to 全组合矩阵（同 Video）
  - delay_ms/loudness/mix_layout 携带不变
  - DEFAULT_DELAY_MS 常量锁（== 80; 锁定 V0.2 §3.7 默认值）
  - mix_layout 词表快照（同 SwitchPolicy 纪律）+ 未知 fail-closed
- **serde(default) 禁用**（A2-2 立规: 新生儿类型无旧实例, 缺字段 fail-closed）
- 声明性 only: 零混合器/响度/延迟执行（属 Audio Engine, A2-7+）; 零行为变化

## Non-Goals

- 混合/响度/延迟执行; Loudness 算法; MixEngine; AV Sync 测量（Master Join 属性, A2-5）;
  Audio Master Join 实际逻辑; Variant Composition/Encode/Output Variant（delivery 侧）

## 验收场景

1. 阶段词表快照恰五词, serde 名锁定（§3.7 Audio Graph 节点对应）
2. advance_to 5×5 全组合矩阵（同 A2-2 纪律; `from/to` 载荷 wire 词表名）
3. data_plane 恒 RawAudio（Errata-3 纪律同 Video）
4. mix_layout 词表: STEREO/FIVE_ONE/STEREO_AND_SUB（声明面; 实际 mix 行为属 A2-7+;
   review Important#1 对账——FIVE_ONE 按 codebase SCREAMING_SNAKE_CASE 惯例, 非 5_1）
5. delay_ms `Option<NonZeroU16>`（None=未声明; Some=具体值; 默认 80 仅 const 锁）
6. loudness_lufs `Option<f32>`（None=未归一化; Some=目标 LUFS 值——控制面设值）
7. 全回归零退化（mock 265 基线 + Audio-specific 测试）

```

## docs/openspec/changes/a2-3-audio-master/design.md

- Source: docs/openspec/changes/a2-3-audio-master/design.md
- Lines: 1-39
- SHA256: 85cea08e94358cbcae0ccf4b97f91b45b7958f0c6f24f780fe6a8f85c242de62

```md
# Design — a2-3-audio-master（高层框架）

## D1 AudioMaster 形态（与 VideoMaster 对称 + Audio 独有字段）

```
AudioMasterStage（封闭词表, §3.7 Audio Graph 节点对应, serde SCREAMING_SNAKE_CASE）
  SourceRaw → Mixed → LoudnessNormalized → DelayCompensated → MasterJoined
AudioMaster {
  stage: AudioMasterStage,
  data_plane: AudioDataPlane::RawAudio,  // 唯一变体（Errata-3 纪律）
  mix_layout: MixLayout,                   // Audio 独有（混合声道布局）
  delay_ms: Option<NonZeroU16>,             // Audio 独有（None=未声明）
  loudness_lufs: Option<f32>,              // Audio 独有（None=未归一化）
}
AudioDataPlane = RawAudio（仅 — 压缩域禁止类型层）
MixLayout: Stereo / FiveOne / StereoAndSub（封闭词表; 未知 fail-closed）
DEFAULT_DELAY_MS = 80u16  // V0.2 §3.7 锁定常量
```

## D2 立规遵循（A2-2 立规）

- `#[serde(default)]` 禁用（新生儿类型无旧实例）
- `advance()` / `advance_to(target)`: 白名单无通配臂; 终态拒绝; `{from,to}` 载荷 wire 词表名
- 信任边界文档化（pub + serde = 声明性对象有意设计; 消费者须重审）
- 产物随代码 commit 同步提交

## D3 测试

词表快照 / serde 名锁 / 5×5 advance_to 全组合矩阵 / RawAudio 类型层锁 /
mix_layout 受纳+拒绝（含大小写敏感）/ DEFAULT_DELAY_MS 常量锁 == 80 /
delay/loudness 携带不变 / 结构级 serde 往返 + 缺字段 fail-closed。
全回归: mock 265 基线零退化 + 矩阵 + clippy 四组合。

## D4 冻结点

- 阶段词表 LOCK（§3.7 节点逐一对应, serde 名 = wire 契约锚）
- data_plane RAW 唯一（Errata-3 纪律同 Video）
- delay_ms 默认值仅 const 锁（**不**通过 serde default 引入——A2-2 立规）
- 声明性 only: 无 mix/loudness/delay 执行（A2-7+）/无 Join（A2-5）

```

## docs/openspec/changes/a2-3-audio-master/tasks.md

- Source: docs/openspec/changes/a2-3-audio-master/tasks.md
- Lines: 1-6
- SHA256: eb32d3aeefd89e05c42d804ca5a9d7b5db48485024ee085f5fa2237a4a76e86a

```md
# Tasks — a2-3-audio-master

> 四栏纪律。TDD; cargo 经盒; 基线 mock 265。立规（A2-2 立）: serde(default) 新生儿禁用 / 产物随代码同步 commit / advance_to 显式目标矩阵 / 信任边界文档化。

- [x] 1. RED+GREEN: `audio_master.rs`（阶段词表快照/serde 名锁/advance_to 5×5 矩阵/RawAudio 类型层锁/mix_layout 词表+拒绝/DEFAULT_DELAY_MS 常量锁=80/advance 携带 delay+loudness/mix_layout 不变/结构级 serde+缺字段 fail-closed/is_program_scope_master 终态判定）+ mod.rs 声明 `Contract: V0.2 §3.7+§1.20+A2-2 立规` | `Implementation: 已` | `Verification: Unit + mock 265 零回退` | `Gate: 无`
- [x] 2. 全回归（矩阵/clippy 四组合）+ review + verify + 双 guard + archive + PR + CI + merge + memory `Contract: 交付纪律` | `Implementation: 已` | `Verification: PR merged` | `Gate: CI/RELEASE`

```
