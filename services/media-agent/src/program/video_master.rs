//! A2-2: Video Master —— V0.2 §3.7 Video Graph 的 Master 侧 Canonical 声明
//!（Program Domain 第二块, 用户裁定链 A2-2, 2026-09-02）。
//!
//! **声明性 only**（合成/烧录执行属 Composition Engine 后续; GStreamer 属 A2-7+）。
//!
//! §3.7 Video Graph（权威逐节点对应）:
//! ```text
//! Source ↓(RAW_VIDEO) → [Normalize] → [Switcher] → [Program Composition]
//!   ↓(RAW_VIDEO) → [Video Master Join] → Program-scope Master(RAW_VIDEO)
//! ```
//!
//! **V0.2.4 Errata-3 锁死**: Encode = delivery boundary; Program-scope Master
//! = **RAW 域**; 禁止压缩域 Master（类型层不可表达——`VideoDataPlane` 唯一变体）;
//! "Clean Master" 术语删除（composition.applied 是事实位, 不存在"干净"概念）。

use serde::{Deserialize, Serialize};

use crate::program::switch_policy::ProgramDomainError;

/// V0.2 §3.7 Video Graph 阶段（封闭词表; serde 名锁定, LOCK FINAL）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VideoMasterStage {
    /// Source 后（RAW_VIDEO 进入视频路径）。
    #[default]
    SourceRaw,
    /// [Normalize] 后。
    Normalized,
    /// [Switcher] 后（switch_policy 声明在 A2-5 join 时接入）。
    Switched,
    /// [Program Composition] 后（节目级 Logo/字幕/版权烧录完成）。
    ProgramComposed,
    /// [Video Master Join] 后（视频路 Program-scope Master 视角就绪）。
    MasterJoined,
}

/// Master 数据平面——**唯一 RAW**（Errata-3: 压缩域 Master 禁止; 唯一变体使
/// 违例在类型层不可构造, 非运行时校验）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VideoDataPlane {
    #[default]
    RawElementary,
}

/// 节目级 Composition 在场性声明（事实位, 非执行）。
/// 默认 `bypassed` = 直通未烧录; `applied` = 已烧节目级包装。
/// （"Clean Master" 不存在——Master 一定处于二者之一的事实状态。）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ProgramComposition {
    pub applied: bool,
}

impl VideoMasterStage {
    /// Canonical wire 名（serde 同源; 错误载荷/诊断统一用词表名——review Minor#8）。
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::SourceRaw => "SOURCE_RAW",
            Self::Normalized => "NORMALIZED",
            Self::Switched => "SWITCHED",
            Self::ProgramComposed => "PROGRAM_COMPOSED",
            Self::MasterJoined => "MASTER_JOINED",
        }
    }
}

/// Video Master —— 视频路径 Master 侧 Canonical Domain Object。
///
/// **信任边界（review Important#2 记档）**: 字段 pub 且可 serde 重建——这是**有意的**
/// （声明性数据对象需持久化/传输往返）; `advance_to` 是**语义守卫**（迁移合法性）,
/// 不是唯一构造路径。`is_program_scope_master()` 将在 A2-5+ 消费——届时须在消费点
/// 重审: 信任构造来源或收紧为构造器模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct VideoMaster {
    pub stage: VideoMasterStage,
    pub data_plane: VideoDataPlane,
    pub composition: ProgramComposition,
}

impl VideoMaster {
    /// Source 起点（RAW_VIDEO 进入即 Master 生命周期开始; composition=bypassed）。
    pub fn new() -> Self {
        Self {
            stage: VideoMasterStage::SourceRaw,
            data_plane: VideoDataPlane::RawElementary,
            composition: ProgramComposition::default(),
        }
    }

    /// 显式目标迁移——白名单: 仅"相邻下一阶段"唯一合法目标。
    /// 跳级/倒退/同阶段/终态后继一律 fail-closed（`{from, to}` 携带真实目标——
    /// 控制面实际使用形态; review Important#3）。
    pub fn advance_to(&self, target: VideoMasterStage) -> Result<Self, ProgramDomainError> {
        let legal_next = match self.stage {
            VideoMasterStage::SourceRaw => Some(VideoMasterStage::Normalized),
            VideoMasterStage::Normalized => Some(VideoMasterStage::Switched),
            VideoMasterStage::Switched => Some(VideoMasterStage::ProgramComposed),
            VideoMasterStage::ProgramComposed => Some(VideoMasterStage::MasterJoined),
            // 终态无后继（显式 None 而非通配放行）。
            VideoMasterStage::MasterJoined => None,
        };
        match legal_next {
            Some(next) if next == target => Ok(Self {
                stage: next,
                data_plane: self.data_plane,
                composition: self.composition,
            }),
            _ => Err(ProgramDomainError::InvalidStageTransition {
                from: self.stage.as_wire().to_string(),
                to: target.as_wire().to_string(),
            }),
        }
    }

    /// 相邻下一步迁移（advance_to(next) sugar——链式推进惯用法）。
    pub fn advance(&self) -> Result<Self, ProgramDomainError> {
        let next = match self.stage {
            VideoMasterStage::SourceRaw => VideoMasterStage::Normalized,
            VideoMasterStage::Normalized => VideoMasterStage::Switched,
            VideoMasterStage::Switched => VideoMasterStage::ProgramComposed,
            VideoMasterStage::ProgramComposed => VideoMasterStage::MasterJoined,
            VideoMasterStage::MasterJoined => {
                return Err(ProgramDomainError::InvalidStageTransition {
                    from: "MASTER_JOINED".to_string(),
                    to: "<terminal: 无后继>".to_string(),
                })
            }
        };
        self.advance_to(next)
    }

    /// 视频路 Program-scope Master 视角就绪（终态判定; join 语义完整实现属 A2-5）。
    pub fn is_program_scope_master(&self) -> bool {
        self.stage == VideoMasterStage::MasterJoined
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;

    /// 阶段词表快照 + serde 名锁（§3.7 逐节点对应, LOCK FINAL）。
    #[test]
    fn program_rt_02_video_stage_vocabulary_and_serde_lock() {
        // §3.7 节点 → 阶段名逐一对应对应:
        // Source→SOURCE_RAW / [Normalize]→NORMALIZED / [Switcher]→SWITCHED /
        // [Program Composition]→PROGRAM_COMPOSED / [Video Master Join]→MASTER_JOINED
        for (stage, wire) in [
            (VideoMasterStage::SourceRaw, "SOURCE_RAW"),
            (VideoMasterStage::Normalized, "NORMALIZED"),
            (VideoMasterStage::Switched, "SWITCHED"),
            (VideoMasterStage::ProgramComposed, "PROGRAM_COMPOSED"),
            (VideoMasterStage::MasterJoined, "MASTER_JOINED"),
        ] {
            assert_eq!(
                serde_json::to_string(&stage).unwrap(),
                format!("\"{wire}\"")
            );
            let back: VideoMasterStage = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
            assert_eq!(back, stage);
        }
        // 未知串 fail-closed。
        assert!(serde_json::from_str::<VideoMasterStage>("\"ENCODED\"").is_err());
        assert!(serde_json::from_str::<VideoMasterStage>("\"RAW\"").is_err());
    }

    /// advance_to 白名单全组合矩阵: 恰四相邻迁移 OK; 跳级/倒退/同阶段/终态后继全部拒绝。
    /// （review Important#3: 显式目标 API 使全组合矩阵真实可测; `{from,to}` 携带真实词表名。）
    #[test]
    fn program_rt_02_video_advance_whitelist_matrix() {
        let chain = [
            VideoMasterStage::SourceRaw,
            VideoMasterStage::Normalized,
            VideoMasterStage::Switched,
            VideoMasterStage::ProgramComposed,
            VideoMasterStage::MasterJoined,
        ];
        let m = VideoMaster::new();
        // 全组合: from × to 的 5×5 矩阵逐一断言。
        for (i, from) in chain.iter().enumerate() {
            let cur = VideoMaster { stage: *from, ..m };
            for (j, to) in chain.iter().enumerate() {
                let r = cur.advance_to(*to);
                if j == i + 1 {
                    // 唯一合法: 相邻下一阶段。
                    let ok = r.expect("相邻迁移必须通过");
                    assert_eq!(ok.stage, *to);
                    assert_eq!(ok.data_plane, VideoDataPlane::RawElementary, "携带不变");
                    assert_eq!(ok.composition, cur.composition, "携带不变");
                } else {
                    // 同阶段(j==i)/跳级(j>i+1)/倒退(j<i)/终态后继(from 终态的一切目标)。
                    let err = r.expect_err("非相邻必须拒绝");
                    assert!(
                        matches!(err, ProgramDomainError::InvalidStageTransition { .. }),
                        "{from:?}→{to:?} 须拒: {err:?}"
                    );
                }
            }
        }
        // no-arg advance() = advance_to(next) sugar: 链式四步到终态。
        let done = m
            .advance()
            .unwrap()
            .advance()
            .unwrap()
            .advance()
            .unwrap()
            .advance()
            .unwrap();
        assert_eq!(done.stage, VideoMasterStage::MasterJoined);
        // 终态 advance() 拒绝（from=MASTER_JOINED 词表名载荷）。
        let err = done.advance().unwrap_err();
        assert!(
            err.to_string().contains("MASTER_JOINED"),
            "错误载荷用词表名: {err}"
        );
    }

    /// RAW 域唯一（Errata-3）: data_plane 构造恒 RawElementary, serde 名锁定。
    #[test]
    fn program_rt_02_video_master_raw_plane_only() {
        assert_eq!(
            serde_json::to_string(&VideoDataPlane::RawElementary).unwrap(),
            "\"RAW_ELEMENTARY\""
        );
        // 压缩域变体在类型层不存在（编译期保证; 未知串 serde fail-closed）。
        assert!(serde_json::from_str::<VideoDataPlane>("\"COMPRESSED\"").is_err());
        assert!(serde_json::from_str::<VideoDataPlane>("\"H264\"").is_err());
        // 全链各阶段 data_plane 恒 RAW（advance 携带不变）。
        let mut m = VideoMaster::new();
        for _ in 0..4 {
            m = m.advance().expect("全链四步");
            assert_eq!(m.data_plane, VideoDataPlane::RawElementary);
        }
    }

    /// composition 两态: 默认 bypassed（直通）; applied 事实位可声明。
    #[test]
    fn program_rt_02_video_composition_fact_bit() {
        let m = VideoMaster::new();
        assert_eq!(
            m.composition,
            ProgramComposition { applied: false },
            "默认直通"
        );
        let burned = VideoMaster {
            composition: ProgramComposition { applied: true },
            ..m
        };
        assert!(burned.composition.applied, "已烧录事实位");
        // advance 携带 composition 不变。
        assert!(
            burned.advance().unwrap().composition.applied,
            "advance 携带 applied 不变"
        );
    }

    /// 结构级 serde 往返 + 缺字段 fail-closed（review Minor#7——新生儿类型无旧实例,
    /// serde(default) 已按 Important#1 移除, 缺字段必须报错而非静默默认）。
    #[test]
    fn program_rt_02_video_master_struct_serde_and_default() {
        let m = VideoMaster::new();
        let json = serde_json::to_string(&m).unwrap();
        let back: VideoMaster = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        // Default == new()（#[default] 首变体 = SourceRaw/Raw/bypassed 契约锚）。
        assert_eq!(VideoMaster::default(), VideoMaster::new());
        // 缺字段 fail-closed: `{}` 不是合法 VideoMaster。
        assert!(
            serde_json::from_str::<VideoMaster>("{}").is_err(),
            "缺 stage/data_plane/composition 必须拒绝"
        );
        let no_comp = json.replace(",\"composition\":{\"applied\":false}", "");
        assert!(
            serde_json::from_str::<VideoMaster>(&no_comp).is_err(),
            "缺 composition 必须拒绝: {no_comp}"
        );
    }

    /// is_program_scope_master 终态判定。
    #[test]
    fn program_rt_02_video_program_scope_master_terminal() {
        let m = VideoMaster::new();
        assert!(!m.is_program_scope_master());
        let done = m
            .advance()
            .unwrap()
            .advance()
            .unwrap()
            .advance()
            .unwrap()
            .advance()
            .unwrap();
        assert!(
            done.is_program_scope_master(),
            "恰四步后为 Program-scope Master 视角"
        );
    }
}
