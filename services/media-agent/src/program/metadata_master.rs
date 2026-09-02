//! A2-4-01: Canonical Metadata Vocabulary —— Program Domain 第四块（V0.2 §3.7/§3.1）。
//!
//! **终裁形态（A2-4-00 SoT Probe §7, OQ-6）**: MetadataMaster 属
//! **fact aggregation + join declaration** 域 —— **无 Stage / 无 advance() /
//! 无迁移矩阵**（V0.2 §3.7 Metadata Graph 零中间处理节点: 三路并列源直汇
//! [Metadata Master Join]; 与 VideoMaster/AudioMaster 的 processing
//! progression 形态刻意不同, 三域差异表见下）。本文件 A2-4-01 只冻结词表;
//! MetadataMaster 结构属 A2-4-02（字段逐项证明, Option 边界 Unknown≠Absent）。
//!
//! 三域差异（设计 guard 红线 #1, 终裁 §7）:
//! - VideoMaster    = processing progression（五阶段链）
//! - AudioMaster    = processing progression（五阶段链）
//! - MetadataMaster = fact aggregation / join declaration（**本域**）
//!
//! 语义层级（OQ-2 终裁）:
//! - `CAPTION` = canonical `metadata_type` wire vocabulary（本模块词表）;
//! - `Subtitle` = §3.7 Graph 一路具体 metadata source 的源/载体语义
//!   （SRT/ASS 为其格式）——**不是** metadata_type 值; `SUBTITLE` wire 名
//!   不存在（测试锁定, 除 V0.3 改 SoT 外禁入词表）。
//!
//! 红线（终裁 §7, 评审必查）:
//! - **taxonomy ≠ topology**（OQ-4）: 五值是 canonical 分类; §3.7 图只明确
//!   三路 source（Timecode / Subtitle(SRT/ASS) / SCTE-35）——KLV/SYSTEM 属
//!   词表但**不因此**获得 Graph source 节点 / processing node / lifecycle
//!   （禁 KlvNormalizer/SystemMetadataProcessor 类臆造）;
//! - **Timecode ownership**（OQ-1）: `CanonicalTimecode` 归 Observation
//!   domain（`crate::timecode`, 本 change 零改动零搬运）; Metadata/Join 只可
//!   引用（consumption）, Master Join 可用（authority）, **MetadataMaster
//!   禁改写 observation**（mutation 禁; clock selection / sync correction /
//!   drift correction 行为一并不入本域）; AV Sync = Master Join property
//!   （≠ Timecode）;
//! - **VideoMaster / AudioMaster 零 diff**（Q8: 零占位即零迁移）。

use serde::{Deserialize, Serialize};

/// 受纳词表快照（V0.2 §3.1 L394-399 + §1.13 L69 两处逐字一致; 决策 #43:
/// §3.1 是 Data Plane 唯一定义规范, 其余章节只引用）。错误信息与测试共用。
pub const METADATA_TYPES: &[&str] = &["TIMECODE", "CAPTION", "SCTE35", "KLV", "SYSTEM"];

/// Canonical metadata taxonomy —— V0.2 §3.1 `metadata_type` 五值封闭词表。
///
/// 无 `Default`（A2-2 立规: 新生儿类型 serde(default) 禁用; 且 taxonomy 与
/// stage 不同——**无天然默认值**, 默认某个 metadata_type 无语义依据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MetadataType {
    /// 媒体帧时间标签（**载体是 Observation 侧 `CanonicalTimecode`**, 本词表值
    /// 仅作 taxonomy 标记——OQ-1: 引用不等于所有权）。
    Timecode,
    /// 字幕/ caption 类（OQ-2: Graph 源语义叫 Subtitle(SRT/ASS), taxonomy 值
    /// 叫 CAPTION——两层不同名是有意设计, 见模块注释）。
    Caption,
    /// 数字节目插入插播指令（V0.2 原文 "SCTE-35" 的 canonical wire 名无连字符）。
    Scte35,
    /// 键-长度-值封装（词表成员; §3.7 图未画 source 节点——OQ-4 纪律）。
    Klv,
    /// 系统级 metadata（词表成员; §3.7 图未画 source 节点——OQ-4 纪律）。
    System,
}

/// Metadata 路 Data Plane —— 单值 `METADATA`（V0.2 §3.1 四层之一; §3.7 L807:
/// Program-scope Master (METADATA)）。类型层不可表达其他平面（对齐
/// VideoDataPlane::RawElementary / AudioDataPlane::RawAudio 单值纪律）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MetadataDataPlane {
    #[default]
    Metadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 词表快照恰五值, wire 名与 V0.2 §3.1/§1.13 逐字一致（含 SCTE35 无连字符）。
    #[test]
    fn program_rt_04_metadata_type_vocabulary_lock() {
        assert_eq!(METADATA_TYPES.len(), 5, "五值封闭词表");
        assert_eq!(
            METADATA_TYPES,
            &["TIMECODE", "CAPTION", "SCTE35", "KLV", "SYSTEM"]
        );
        for (variant, wire) in [
            (MetadataType::Timecode, "TIMECODE"),
            (MetadataType::Caption, "CAPTION"),
            (MetadataType::Scte35, "SCTE35"),
            (MetadataType::Klv, "KLV"),
            (MetadataType::System, "SYSTEM"),
        ] {
            assert_eq!(
                serde_json::to_string(&variant).unwrap(),
                format!("\"{wire}\"")
            );
        }
    }

    /// 每值 to_string → from_str 恒等往返（per-pair, 非平凡 matches! —— A2-3
    /// Minor#4 同律）。
    #[test]
    fn program_rt_04_metadata_type_roundtrip_per_pair() {
        let all = [
            (MetadataType::Timecode, "TIMECODE"),
            (MetadataType::Caption, "CAPTION"),
            (MetadataType::Scte35, "SCTE35"),
            (MetadataType::Klv, "KLV"),
            (MetadataType::System, "SYSTEM"),
        ];
        for (variant, wire) in all {
            let s = serde_json::to_string(&variant).unwrap();
            let back: MetadataType = serde_json::from_str(&s).unwrap();
            assert_eq!(back, variant, "{wire} 往返恒等");
        }
    }

    /// fail-closed: 未知串拒绝; **OQ-2/OQ-4 终裁锁定**——`SUBTITLE`（Subtitle 是
    /// 源语义非 taxonomy 值）与 `SCTE_35`（连字符变体）一并拒收; 大小写敏感。
    #[test]
    fn program_rt_04_metadata_type_fail_closed() {
        for bad in [
            "SUBTITLE", // OQ-2: Subtitle=源/载体语义, 非 metadata_type 值
            "Subtitle", // 驼峰变体同样拒
            "SCTE_35",  // 连字符变体——wire 名是无连字符的 SCTE35
            "scte35",   // 大小写敏感
            "UNKNOWN",  // 未知串
            "",         // 空串
        ] {
            assert!(
                serde_json::from_str::<MetadataType>(&format!("\"{bad}\"")).is_err(),
                "fail-closed: 拒收 {bad:?}"
            );
        }
    }

    /// Data Plane 单值锁: wire 恒 `METADATA`; 其他平面串（含 RAW_*/COMPRESSED_*）
    /// 一律拒收——Metadata 路不存在第二平面（对齐 Errata-3 单值纪律精神）。
    #[test]
    fn program_rt_04_metadata_data_plane_single_value() {
        assert_eq!(
            serde_json::to_string(&MetadataDataPlane::Metadata).unwrap(),
            "\"METADATA\""
        );
        assert_eq!(
            MetadataDataPlane::default(),
            MetadataDataPlane::Metadata,
            "单值平面的 Default 即唯一值"
        );
        for foreign in [
            "RAW_VIDEO",
            "RAW_AUDIO",
            "COMPRESSED_VIDEO",
            "EVENT",
            "MULTIPLEXED",
        ] {
            assert!(
                serde_json::from_str::<MetadataDataPlane>(&format!("\"{foreign}\"")).is_err(),
                "Metadata 平面不可表达 {foreign}"
            );
        }
    }
}
