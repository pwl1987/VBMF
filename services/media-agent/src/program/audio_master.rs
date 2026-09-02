//! A2-3: Audio Master —— V0.2 §3.7 Audio Graph 的 Master 侧 Canonical 声明
//!（Program Domain 第三块, 严格遵守 A2-2 立规, 2026-09-02）。
//!
//! **声明性 only**（混合/响度/延迟执行属 Audio Engine 后续; GStreamer 属 A2-7+）。
//!
//! §3.7 Audio Graph（权威逐节点对应）:
//! ```text
//! Source ↓(RAW_AUDIO) → [Audio Mixer] → [Loudness] → [Audio Delay] (+80ms) → [Audio Master Join]
//!   ↓
//! Program-scope Master (RAW_AUDIO)
//! ```
//!
//! **V0.2.4 Errata-3 锁死**: Encode = delivery boundary; Program-scope Master = **RAW 域**;
//! 压缩域 Master 禁止——`AudioDataPlane` 唯一变体使违例在类型层不可构造。
//!
//! **A2-2 立规遵守**: `#[serde(default)]` 禁用（新生儿类型无旧实例; 缺字段 fail-closed）;
//! `advance_to(target)` 显式目标 API; 信任边界文档化; 产物随代码同步提交。

use serde::{Deserialize, Serialize};
use std::num::NonZeroU16;

use crate::program::switch_policy::ProgramDomainError;

/// V0.2 §3.7 Audio Graph 阶段（封闭词表; serde 名锁定, LOCK FINAL）。
/// 顺序: Source → Mixer → Loudness → Delay(+80ms) → Audio Master Join。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AudioMasterStage {
    /// Source 后（RAW_AUDIO 进入音频路径; A2-2 立规 #[default] = 首变体 = SourceRaw）。
    #[default]
    SourceRaw,
    /// [Audio Mixer] 后。
    Mixed,
    /// [Loudness] 后。
    LoudnessNormalized,
    /// [Audio Delay] 后（位于 Loudness 与 Audio Master Join 之间——V0.2 §3.7; 本层仅声明"已补偿"）。
    DelayCompensated,
    /// [Audio Master Join] 后（音频路 Program-scope Master 视角就绪）。
    MasterJoined,
}

/// Master 数据平面——**唯一 RawAudio**（Errata-3: 压缩域 Master 类型层不可表达）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AudioDataPlane {
    #[default]
    RawAudio,
}

/// 混合声道布局（声明面; 实际 mix 行为属 A2-7+）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MixLayout {
    #[default]
    Stereo,
    FiveOne,
    StereoAndSub,
}

/// V0.2 §3.7 锁定——Audio Delay 默认 +80ms。仅 const 锁（**不**通过 serde default
/// 引入, A2-2 立规）。
pub const DEFAULT_DELAY_MS: u16 = 80;

/// Audio Master —— 音频路径 Master 侧 Canonical Domain Object。
///
/// **信任边界（A2-2 立规 apply）**: 字段 pub 且 serde 可重建——**有意的**
/// （声明性数据对象需持久化/传输往返）; `advance_to` 是**语义守卫**, 不是唯一
/// 构造路径。A2-5 消费 `is_program_scope_master()` 前须在消费点重审信任来源或
/// 收紧为构造器模式。
///
/// derive 限制: `f32` 不实现 `Eq/Hash`——本类型只 derive `PartialEq`（值等比较语义足够;
/// `Hash` 在声明面无实际语义需求, 不强行包入。`Default` 锁首变体契约。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct AudioMaster {
    pub stage: AudioMasterStage,
    pub data_plane: AudioDataPlane,
    pub mix_layout: MixLayout,
    /// 延迟补偿声明（None = 未声明; Some = 具体 ms 值）。仅声明面——
    /// 实际延迟执行属 Audio Engine 后续。
    pub delay_ms: Option<NonZeroU16>,
    /// 响度归一化目标 LUFS（None = 未归一化; Some = 目标值）。
    /// 控制面设值; 实际归一化算法属 A2-7+。
    /// 单独 `PartialEq` 推导（f32 不实现 Eq/Hash, 声明显然不需要 Hash; 比较仅用于测试）。
    pub loudness_lufs: Option<f32>,
}

impl AudioMaster {
    /// Source 起点（RAW_AUDIO 进入即 Audio Master 生命周期开始; 默认布局+未声明+未归一化）。
    pub fn new() -> Self {
        Self {
            stage: AudioMasterStage::SourceRaw,
            data_plane: AudioDataPlane::RawAudio,
            mix_layout: MixLayout::Stereo,
            delay_ms: None,
            loudness_lufs: None,
        }
    }

    /// 显式目标迁移——白名单: 仅"相邻下一阶段"唯一合法目标。
    /// 跳级/倒退/同阶段/终态后继一律 fail-closed（`{from, to}` 携带真实词表名）。
    pub fn advance_to(&self, target: AudioMasterStage) -> Result<Self, ProgramDomainError> {
        let legal_next = match self.stage {
            AudioMasterStage::SourceRaw => Some(AudioMasterStage::Mixed),
            AudioMasterStage::Mixed => Some(AudioMasterStage::LoudnessNormalized),
            AudioMasterStage::LoudnessNormalized => Some(AudioMasterStage::DelayCompensated),
            AudioMasterStage::DelayCompensated => Some(AudioMasterStage::MasterJoined),
            AudioMasterStage::MasterJoined => None,
        };
        match legal_next {
            Some(next) if next == target => Ok(Self {
                stage: next,
                data_plane: self.data_plane,
                mix_layout: self.mix_layout,
                delay_ms: self.delay_ms,
                loudness_lufs: self.loudness_lufs,
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
            AudioMasterStage::SourceRaw => AudioMasterStage::Mixed,
            AudioMasterStage::Mixed => AudioMasterStage::LoudnessNormalized,
            AudioMasterStage::LoudnessNormalized => AudioMasterStage::DelayCompensated,
            AudioMasterStage::DelayCompensated => AudioMasterStage::MasterJoined,
            AudioMasterStage::MasterJoined => {
                return Err(ProgramDomainError::InvalidStageTransition {
                    from: AudioMasterStage::MasterJoined.as_wire().to_string(),
                    to: "<terminal: 无后继>".to_string(),
                })
            }
        };
        self.advance_to(next)
    }

    /// 音频路 Program-scope Master 视角就绪（终态判定）。
    pub fn is_program_scope_master(&self) -> bool {
        self.stage == AudioMasterStage::MasterJoined
    }
}

impl AudioMasterStage {
    /// Canonical wire 名（serde 同源; 错误载荷/诊断统一用词表名）。
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::SourceRaw => "SOURCE_RAW",
            Self::Mixed => "MIXED",
            Self::LoudnessNormalized => "LOUDNESS_NORMALIZED",
            Self::DelayCompensated => "DELAY_COMPENSATED",
            Self::MasterJoined => "MASTER_JOINED",
        }
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;

    /// 阶段词表快照 + serde 名锁（§3.7 节点逐一对应, LOCK FINAL）。
    #[test]
    fn program_rt_03_audio_stage_vocabulary_and_serde_lock() {
        for (stage, wire) in [
            (AudioMasterStage::SourceRaw, "SOURCE_RAW"),
            (AudioMasterStage::Mixed, "MIXED"),
            (AudioMasterStage::LoudnessNormalized, "LOUDNESS_NORMALIZED"),
            (AudioMasterStage::DelayCompensated, "DELAY_COMPENSATED"),
            (AudioMasterStage::MasterJoined, "MASTER_JOINED"),
        ] {
            assert_eq!(
                serde_json::to_string(&stage).unwrap(),
                format!("\"{wire}\"")
            );
            let back: AudioMasterStage = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
            assert_eq!(back, stage);
        }
        // 未知串 fail-closed。
        assert!(serde_json::from_str::<AudioMasterStage>("\"ENCODED\"").is_err());
        assert!(serde_json::from_str::<AudioMasterStage>("\"RAW\"").is_err());
    }

    /// advance_to 5×5 全组合矩阵: 4 相邻 OK; 一切其他组合 fail-closed。
    #[test]
    fn program_rt_03_audio_advance_whitelist_matrix() {
        let chain = [
            AudioMasterStage::SourceRaw,
            AudioMasterStage::Mixed,
            AudioMasterStage::LoudnessNormalized,
            AudioMasterStage::DelayCompensated,
            AudioMasterStage::MasterJoined,
        ];
        let m = AudioMaster::new();
        for (i, from) in chain.iter().enumerate() {
            let cur = AudioMaster { stage: *from, ..m };
            for (j, to) in chain.iter().enumerate() {
                let r = cur.advance_to(*to);
                if j == i + 1 {
                    let ok = r.expect("相邻迁移必须通过");
                    assert_eq!(ok.stage, *to);
                    assert_eq!(
                        ok.data_plane,
                        AudioDataPlane::RawAudio,
                        "data_plane 携带不变"
                    );
                    assert_eq!(ok.mix_layout, cur.mix_layout, "mix_layout 携带不变");
                    assert_eq!(ok.delay_ms, cur.delay_ms, "delay_ms 携带不变");
                    assert_eq!(ok.loudness_lufs, cur.loudness_lufs, "loudness 携带不变");
                } else {
                    let err = r.expect_err("非相邻必须拒绝");
                    match &err {
                        ProgramDomainError::InvalidStageTransition { from: ef, to: et } => {
                            assert_eq!(ef, from.as_wire(), "from 载荷=wire 名");
                            assert_eq!(et, to.as_wire(), "to 载荷=wire 名");
                        }
                        _ => panic!("必须是 InvalidStageTransition: {err:?}"),
                    }
                }
            }
        }
        // no-arg advance = advance_to(next) sugar: 链式四步到终态。
        let done = m
            .advance()
            .unwrap()
            .advance()
            .unwrap()
            .advance()
            .unwrap()
            .advance()
            .unwrap();
        assert_eq!(done.stage, AudioMasterStage::MasterJoined);
        // 终态 advance 拒绝（载荷用 wire 词表名）。
        let err = done.advance().unwrap_err();
        assert!(
            err.to_string().contains("MASTER_JOINED"),
            "载荷用词表名: {err}"
        );
    }

    /// RawAudio 唯一 + 压缩域 serde fail-closed（Errata-3）。
    #[test]
    fn program_rt_03_audio_master_raw_plane_only() {
        assert_eq!(
            serde_json::to_string(&AudioDataPlane::RawAudio).unwrap(),
            "\"RAW_AUDIO\""
        );
        assert!(serde_json::from_str::<AudioDataPlane>("\"COMPRESSED\"").is_err());
        assert!(serde_json::from_str::<AudioDataPlane>("\"AAC\"").is_err());
        let mut m = AudioMaster::new();
        for _ in 0..4 {
            m = m.advance().unwrap();
            assert_eq!(m.data_plane, AudioDataPlane::RawAudio);
        }
    }

    /// mix_layout 词表快照 + 失败闭合（含大小写敏感 + 跨词表污染）。
    #[test]
    fn program_rt_03_audio_mix_layout_vocabulary() {
        // 受纳三词。
        for (layout, wire) in [
            (MixLayout::Stereo, "STEREO"),
            (MixLayout::FiveOne, "FIVE_ONE"),
            (MixLayout::StereoAndSub, "STEREO_AND_SUB"),
        ] {
            assert_eq!(
                serde_json::to_string(&layout).unwrap(),
                format!("\"{wire}\"")
            );
            let back: MixLayout = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
            assert_eq!(back, layout, "wire↔variant 恒等");
        }
        // 拒绝: 大小写敏感 + 空串 + 跨词表污染（VIDEO/RAW/RTMP/HLS 都不是 mix 布局）。
        for bad in [
            "stereo",
            "5_1",
            "FIVE1",
            "",
            "VIDEO",
            "RAW_AUDIO",
            "RTMP",
            "HLS",
        ] {
            assert!(
                serde_json::from_str::<MixLayout>(&format!("\"{bad}\"")).is_err(),
                "必须拒绝: {bad}"
            );
        }
    }

    /// DEFAULT_DELAY_MS const 锁 == 80（V0.2 §3.7 锁定值）。
    #[test]
    fn program_rt_03_audio_default_delay_ms_locked() {
        assert_eq!(DEFAULT_DELAY_MS, 80, "V0.2 §3.7 Audio Delay 默认 +80ms");
    }

    /// delay/loudness/mix_layout 三字段携带不变（独立事实位, advance 不参与迁移）。
    #[test]
    fn program_rt_03_audio_fact_fields_carried() {
        let mut m = AudioMaster::new();
        m.mix_layout = MixLayout::FiveOne;
        m.delay_ms = NonZeroU16::new(120);
        m.loudness_lufs = Some(-23.0);
        for _ in 0..4 {
            m = m.advance().unwrap();
            assert_eq!(m.mix_layout, MixLayout::FiveOne);
            assert_eq!(m.delay_ms, NonZeroU16::new(120));
            assert_eq!(m.loudness_lufs, Some(-23.0));
        }
    }

    /// 结构级 serde 往返 + 缺字段 fail-closed（立规: 新生儿类型无旧实例）。
    #[test]
    fn program_rt_03_audio_struct_serde_and_default() {
        let m = AudioMaster {
            mix_layout: MixLayout::StereoAndSub,
            delay_ms: NonZeroU16::new(80),
            loudness_lufs: Some(-14.0),
            ..AudioMaster::new()
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: AudioMaster = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        // Default == new()（#[default] 首变体契约锚）。
        assert_eq!(AudioMaster::default(), AudioMaster::new());
        // 缺字段 fail-closed: `{}` 不是合法 AudioMaster。
        assert!(
            serde_json::from_str::<AudioMaster>("{}").is_err(),
            "缺字段必须拒绝"
        );
    }

    /// is_program_scope_master 终态判定。
    #[test]
    fn program_rt_03_audio_program_scope_master_terminal() {
        let m = AudioMaster::new();
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
            "Audio 路 Program-scope Master 视角就绪"
        );
    }
}
