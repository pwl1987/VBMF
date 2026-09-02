# Comet Design Handoff

- Change: a2-2-video-master
- Phase: design
- Mode: compact
- Context hash: 3343ea262d24fab9eccff8bc0d64a306ee32fc55ae57070ea2653f92b314c942

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/a2-2-video-master/proposal.md

- Source: docs/openspec/changes/a2-2-video-master/proposal.md
- Lines: 1-39
- SHA256: fd15e35079c4ee0da76223c7f28ee7a78dc4a408a75d710d013e92e9140e2288

```md
# Proposal — a2-2-video-master

## Why

裁定链 A2-2（A2-1 SwitchPolicy 已收口 @78a8319）。V0.2 §1.20 + §3.7 权威语义:
Video Graph 是**独立 graph**（SDI → Normalize → Switch → Program Composition →
Video Master Join → Program-scope Master），且 **Program-scope Master = RAW 域**
（V0.2.4 Errata-3 锁死: Encode 是 delivery boundary, 绝不在 Master Join 之前;
"Clean Master" 术语已删除——Master 一定含 Program Scope Composition）。
代码现状: Video Master 概念零实现（Reality Audit Engine 表 Composition 0%）。

## What Changes

- **`src/program/video_master.rs`**（Program Domain 第二块）:
  `VideoMaster` Canonical Domain Object——视频路径的 Master 侧**声明**:
  - `VideoMasterStage` 封闭阶段词表（`SOURCE_RAW / NORMALIZED / SWITCHED /
    PROGRAM_COMPOSED / MASTER_JOINED`——§3.7 Video Graph 逐节点对应; serde 名锁定）
  - `VideoMaster` { stage, data_plane }——data_plane 锁 `RAW_ELEMENTARY`
    （**Master 永远 RAW 域**——压缩域 Master 属 Errata-3 禁止, 类型层面不可表达）
  - `advance()` 白名单迁移（相邻阶段唯一; 跳级/倒退拒绝——阶段机非自由态）
  - `ProgramComposition` 声明（节目级 Logo/字幕/版权**在场性**声明——
    Composition Engine 执行属后续; 本 change 只声明"已烧录/未烧录"事实位）
- **阶段↔§3.7 图节点对照**测试锁定; ProgramDomainError 复用扩展
  （`InvalidStageTransition` / `CompressedMasterForbidden` 不可达但语义在档）。
- 声明性 only: 零合成执行（GStreamer compositor 属 A2-7+）; 零行为变化。

## Non-Goals

- 合成执行/烧录引擎（Composition Engine）;/ Audio/Metadata Master（A2-3/4）;
  Master Join / ProgramMaster（A2-5/6）;/ Variant Composition/Encode/Output Variant
  （delivery 侧后续）;/ AV Sync（Master Join 属性, A2-5）

## 验收场景

1. 阶段词表快照恰五词, serde 名锁定, 与 §3.7 图节点逐节点对应
2. data_plane 恒 RAW_ELEMENTARY（类型层不可构造压缩域 Master）
3. advance 白名单: 相邻唯一可迁; 跳级/倒退/重复 fail-closed
4. ProgramComposition 在场性声明（applied/bypassed 两态, 默认 bypassed = 直通）
5. 全回归零退化（mock 259 基线）

```

## docs/openspec/changes/a2-2-video-master/design.md

- Source: docs/openspec/changes/a2-2-video-master/design.md
- Lines: 1-22
- SHA256: 0b320a7bb322712cfe0143b161806cf82cbf31361328378b9d823008dfe0c788

```md
# Design — a2-2-video-master（高层框架）

## D1 VideoMaster = Canonical 声明（program 域第二块）

```
VideoMasterStage（封闭阶段词表, §3.7 Video Graph 节点对应）
  SourceRaw → Normalized → Switched → ProgramComposed → MasterJoined
VideoMaster { stage, data_plane: VideoDataPlane, composition: ProgramComposition }
  VideoDataPlane = RAW_ELEMENTARY（唯一变体——压缩域 Master 类型层不可表达, Errata-3）
```

- `advance(&self) -> Result<Self>`: 仅相邻前一阶段可迁（白名单 match, 无通配臂）;
  跳级/倒退/重复 → `ProgramDomainError::InvalidStageTransition`
- `ProgramComposition { applied: bool }`（默认 bypassed = 直通未烧录;
  applied = 已烧节目级包装——事实位非执行）
- 构造: `VideoMaster::new()` = SourceRaw 起点（source 进 RAW 域即 Master 生命周期起点）

## D2 冻结点

- 阶段词表 LOCK（§3.7 逐节点）; RAW 域唯一（Errata-3 禁止压缩域 Master）;
  advance 白名单无通配（新增阶段 = 编译期强制评审）
- 声明性 only: 无合成执行/无 GStreamer/无 runtime 接线（A2-6 投影时接）

```

## docs/openspec/changes/a2-2-video-master/tasks.md

- Source: docs/openspec/changes/a2-2-video-master/tasks.md
- Lines: 1-6
- SHA256: 3afdd68d46a22bf93dc3f49681f074e7580d3344f6a8cb7fa7a4440381b9f95a

```md
# Tasks — a2-2-video-master

> 四栏纪律。TDD; cargo 经盒; 基线 mock 259。

- [ ] 1. RED+GREEN: `video_master.rs`（阶段词表快照/serde 锁/advance 白名单含跳级倒退拒绝/RAW 域唯一/composition 两态）+ mod.rs 声明 `Contract: V0.2 §1.20+§3.7+Errata-3` | `Implementation: 待` | `Verification: Unit + mock 259 零回退` | `Gate: 无`
- [ ] 2. 全回归（矩阵/clippy 四组合）+ review + verify + 双 guard + archive + PR + CI + merge `Contract: 交付纪律` | `Implementation: 待` | `Verification: PR merged` | `Gate: CI/RELEASE`

```
