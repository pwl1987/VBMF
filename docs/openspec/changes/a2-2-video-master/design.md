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
