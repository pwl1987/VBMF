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
- ~~`CompressedMasterForbidden` 错误变体~~（**review 对账删除**——压缩域 Master 由
  `VideoDataPlane` 唯一变体在**类型层**杜绝, 运行时错误变体不可达亦无需）
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
