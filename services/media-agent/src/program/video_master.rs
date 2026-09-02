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
    #[serde(default)]
    pub applied: bool,
}

/// Video Master —— 视频路径 Master 侧 Canonical Domain Object。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct VideoMaster {
    #[serde(default)]
    pub stage: VideoMasterStage,
    #[serde(default)]
    pub data_plane: VideoDataPlane,
    #[serde(default)]
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

    /// 相邻阶段白名单迁移（恰四迁移, match 无通配臂——新增阶段编译期强制评审）。
    /// 跳级/倒退/同阶段一律 fail-closed。
    pub fn advance(&self) -> Result<Self, ProgramDomainError> {
        let next = match self.stage {
            VideoMasterStage::SourceRaw => VideoMasterStage::Normalized,
            VideoMasterStage::Normalized => VideoMasterStage::Switched,
            VideoMasterStage::Switched => VideoMasterStage::ProgramComposed,
            VideoMasterStage::ProgramComposed => VideoMasterStage::MasterJoined,
            // 终态无后继——显式拒绝而非通配放行。
            VideoMasterStage::MasterJoined => {
                return Err(ProgramDomainError::InvalidStageTransition {
                    from: format!("{:?}", self.stage),
                    to: "<terminal: MasterJoined 无后继>".to_string(),
                })
            }
        };
        Ok(Self {
            stage: next,
            data_plane: self.data_plane,
            composition: self.composition,
        })
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

    /// advance 白名单全组合矩阵: 恰四相邻迁移 OK; 一切其他组合 fail-closed。
    #[test]
    fn program_rt_02_video_advance_whitelist_matrix() {
        let chain = [
            VideoMasterStage::SourceRaw,
            VideoMasterStage::Normalized,
            VideoMasterStage::Switched,
            VideoMasterStage::ProgramComposed,
            VideoMasterStage::MasterJoined,
        ];
        // 相邻迁移恰四且逐一通过; data_plane/composition 携带不变。
        for (i, from) in chain.iter().enumerate().take(4) {
            let m = VideoMaster {
                stage: *from,
                ..VideoMaster::new()
            };
            let next = m.advance().expect("相邻迁移必须通过");
            assert_eq!(next.stage, chain[i + 1], "{from:?} 的后继");
            assert_eq!(next.data_plane, VideoDataPlane::RawElementary);
            assert_eq!(next.composition, m.composition);
        }
        // 终态: 无后继。
        let m = VideoMaster::new();
        assert!(
            VideoMaster {
                stage: VideoMasterStage::MasterJoined,
                ..m
            }
            .advance()
            .is_err(),
            "终态无后继"
        );
        assert!(
            VideoMaster {
                stage: VideoMasterStage::SourceRaw,
                ..m
            }
            .advance()
            .unwrap() // Normalized
            .advance()
            .unwrap() // Switched
            .advance()
            .unwrap() // ProgramComposed
            .advance()
            .unwrap() // MasterJoined
            .advance()
            .is_err(),
            "全链后再 advance 终态拒绝"
        );
        // 倒退: 从 Normalized 的后继只能是 Switched, 绝非回 SourceRaw。
        let norm = VideoMaster {
            stage: VideoMasterStage::Normalized,
            ..m
        };
        assert_eq!(norm.advance().unwrap().stage, VideoMasterStage::Switched);
        // 跳级不可表达: API 唯一迁移入口是 advance（相邻一步）;
        // 从 SourceRaw 到 MasterJoined 恰需四步（见 terminal 测试）——不存在一步路径。
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
