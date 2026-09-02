---
comet_change: a2-2-video-master
role: technical-design
canonical_spec: openspec
---

# Design Doc — a2-2-video-master（A2-2: Video Master）

基线 master `78a8319`（A2-1 SwitchPolicy 收口点）。Program Domain 第二块。

## 1. 权威语义锚（V0.2 probe 实证）

- §1.20: Video Graph **独立 graph**（SDI → Normalize → Switch → Compose → Encode 摘要）;
  §3.7 详图: `Source ↓(RAW_VIDEO) → [Normalize] → [Switcher] → [Program Composition]
  （烧录节目级 Logo/Bug/字幕）↓(RAW_VIDEO) → [Video Master Join] → Program-scope Master(RAW_VIDEO)`。
- **V0.2.4 Errata-3 锁死**: Encode = delivery boundary; Program-scope Master = **RAW 域**;
  禁止把 Program Master 实现为 H.264/AAC 压缩域; "Clean Master" 术语删除
  （Master 一定含 Program Scope Composition——applied/bypassed 是事实位, 不存在"干净"概念）。
- A2-1 已落: `program/` 模块 + ProgramDomainError + SwitchPolicy（本 change 复用错误类型扩展）。

## 2. 类型设计

```rust
// src/program/video_master.rs
#[derive(...)] #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VideoMasterStage {
    SourceRaw,       // §3.7 Source 后（RAW_VIDEO 入）
    Normalized,      // [Normalize] 后
    Switched,       // [Switcher] 后（switch_policy 在场声明留给 A2-5 join 时接）
    ProgramComposed, // [Program Composition] 后（节目级烧录完成）
    MasterJoined,    // [Video Master Join] 后（= Program-scope Master 视频路）
}

#[derive(...)] pub enum VideoDataPlane { RawElementary }  // 唯一——压缩域类型层不可表达

pub struct ProgramComposition { pub applied: bool }  // 默认 bypassed（直通）; applied=已烧录

pub struct VideoMaster {
    pub stage: VideoMasterStage,
    pub data_plane: VideoDataPlane,        // 构造恒 RawElementary
    pub composition: ProgramComposition,
}

impl VideoMaster {
    pub fn new() -> Self;                          // SourceRaw 起点 + bypassed
    pub fn advance(&self) -> Result<Self, ProgramDomainError>;  // 相邻白名单（match 无通配）
    pub fn is_program_scope_master(&self) -> bool;  // stage==MasterJoined
}
```

- `ProgramDomainError` 扩展: `InvalidStageTransition { from, to }`（Debug 承载枚举）
- advance 白名单恰四迁移: SourceRaw→Normalized→Switched→ProgramComposed→MasterJoined;
  其余一切组合（跳级/倒退/同阶段）fail-closed。

## 3. 冻结点

- 阶段词表 LOCK（§3.7 逐节点对应, serde 名锁定）; 白名单无通配臂。
- data_plane RAW 唯一（Errata-3）; 不构造压缩域 Master 的能力在类型层不存在。
- 声明性 only: 无合成执行/无 GStreamer/runtime 零接线（行为零变化）。

## 4. 测试策略

Unit: 词表快照恰五词 + serde 名锁 + 阶段↔§3.7 节点对照注释锁定 + advance 全组合矩阵
（4 迁移 OK / 跳级×3 / 倒退×4 / 同阶段×5 拒绝）+ identity parse 往返 + RAW 域恒定 +
composition 默认 bypassed/applied 两态 + is_program_scope_master。
全回归: mock 259 基线零回退 + 矩阵 + clippy 四组合。
