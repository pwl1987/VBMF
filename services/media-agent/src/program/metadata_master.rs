//! A2-4-01: Canonical Metadata Vocabulary —— Program Domain 第四块（V0.2 §3.7/§3.1）。
//!
//! **终裁形态（A2-4-00 SoT Probe §7, OQ-6）**: MetadataMaster 属
//! **fact aggregation + join declaration** 域 —— **无 Stage / 无 advance() /
//! 无迁移矩阵**（V0.2 §3.7 Metadata Graph 零中间处理节点: 三路并列源直汇
//! [Metadata Master Join]; 与 VideoMaster/AudioMaster 的 processing
//! progression 形态刻意不同, 三域差异表见下）。A2-4-01 冻结词表（§词表）;
//! A2-4-02 落地最小闭合模型: MetadataPresence / MetadataJoinDeclaration /
//! MetadataFact / MetadataMaster（字段终裁表 16 行全冻结, Design Doc §1.5）。
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

use crate::normalize::CanonicalSourceRef;
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

// ═══ A2-4-02: 最小闭合模型（SQ-1..SQ-5 + scope 终裁, Design Doc §1.5 锁定）═══

/// Join 声明受纳词表快照（SQ-2 终裁; 错误信息与测试共用）。
pub const JOIN_DECLARATIONS: &[&str] = &["PARTICIPATING", "NOT_PRESENT", "UNKNOWN"];

/// Metadata fact 在本对象中的**存在性**（SQ-1 终裁: 闭合 enum 最小三态）。
///
/// 语义边界: 表达 fact 存在性, **不是** metadata 内容健康状态——
/// `INVALID / DISCONTINUOUS / RECOVERED` 属 Timecode Observation 域语义
/// （#148）, 绝不复制进本词表（测试锁定拒收）。
/// 锚: `AudioPresence::{Present{..}, NotPresent, Unknown}` 三态同形。
/// 无 `Default`（构造 fact 必须显式 presence——"默认存在/不存在"均无依据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MetadataPresence {
    Present,
    NotPresent,
    Unknown,
}

/// Metadata Master 对 Master Join 的**声明**（SQ-2 终裁: enum, 禁裸 bool）。
///
/// 命名红线: 类型名不用 `Status`（Declaration ≠ Readiness ≠ Health ≠
/// Publication）; 无 `READY` 变体（readiness 属 Join/Projection）; 无
/// `JOINED/CONSUMED`（Join 消费态属 A2-5 侧）; 无 `NOT_APPLICABLE`
/// （业务裁决源不存在——控制面未建, 加法演进留口）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MetadataJoinDeclaration {
    /// Metadata 路正向声明参与 Program Master Join（facts 可空——SQ-4:
    /// "明确知道无额外 metadata 的节目" 是合法组合）。
    Participating,
    /// 已观测并声明本 Program 无该路 metadata（≠ 没观测; Unknown≠Absent 原则）。
    NotPresent,
    /// 观测不足以作出声明（无观测前态, 亦为 `Default`）。
    #[default]
    Unknown,
}

/// Canonical metadata fact —— 一个 metadata fact 的**存在与来源分类**声明
/// （Candidate B, A2-4-02-00 Probe §2）。
///
/// **不是 payload container**（SQ-5: 无 payload 字段——当前仓库零 canonical
/// payload 类型, 提前发明 = 创造第二 Metadata SoT）; 无 timecode/timestamp/
/// scope 字段（终裁表四不要: Timecode 归 Input Observation SoT / 时间语义
/// 不存在 / Program Domain 结构即 scope）。物理字段名 `kind` = 仓库压倒性
/// 惯例（api_boundary/command/clock 族）, 对应终裁表语义名 "fact.type"。
///
/// PartialEq+Eq only（无 Hash）: 声明面零 Hash 消费方; `CanonicalSourceRef`
/// 未实现 Hash, 为对称补 Hash 须改共享类型 normalize.rs——非本 change 职权
/// （与 SQ-3 "拒绝对称而对称" 同构; 需要时加法演进）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataFact {
    pub kind: MetadataType,
    /// 来源 canonical 引用（P02: 复用既有类型, 禁新建 MetadataSourceId）。
    pub source: CanonicalSourceRef,
    pub presence: MetadataPresence,
}

/// MetadataMaster —— Program Domain 第四块, **fact aggregation + join
/// declaration** 域对象（三域差异红线: 与 Video/Audio 的 processing
/// progression 形态刻意不同——无 Stage/无 advance/无迁移矩阵, OQ-6 终裁）。
///
/// `facts` 与 `join_declaration` 两正交维度（SQ-4）: 空 Vec 合法, **禁止**
/// 以 Vec 是否为空推导 readiness/health/join——两组组合均不被结构禁止。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MetadataMaster {
    /// canonical Data Plane 自描述身份（SQ-3: domain contract value, 非对称
    /// 装饰; 唯一合法值 METADATA——EVENT 是 Control 平面, 禁入）。
    pub data_plane: MetadataDataPlane,
    pub facts: Vec<MetadataFact>,
    pub join_declaration: MetadataJoinDeclaration,
}

impl MetadataMaster {
    /// 无观测前态: 空聚合 + Unknown 声明（与 `Default` 同值; 显式构造入口）。
    pub fn new() -> Self {
        Self {
            data_plane: MetadataDataPlane::Metadata,
            facts: Vec::new(),
            join_declaration: MetadataJoinDeclaration::Unknown,
        }
    }
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

    // ═══ A2-4-02 测试（SQ 终裁红线测试级锁定）═══

    use uuid::Uuid;

    fn sample_source() -> CanonicalSourceRef {
        CanonicalSourceRef {
            device_id: Uuid::nil(),
            port_id: Some(Uuid::nil()),
        }
    }

    fn sample_fact(kind: MetadataType, presence: MetadataPresence) -> MetadataFact {
        MetadataFact {
            kind,
            source: sample_source(),
            presence,
        }
    }

    /// SQ-1: presence 三态词表锁 + 往返; **拒收 Timecode 域语义**
    /// （INVALID/DISCONTINUOUS/RECOVERED——绝不复制 Observation SoT）。
    #[test]
    fn program_rt_04_metadata_presence_vocabulary() {
        for (variant, wire) in [
            (MetadataPresence::Present, "PRESENT"),
            (MetadataPresence::NotPresent, "NOT_PRESENT"),
            (MetadataPresence::Unknown, "UNKNOWN"),
        ] {
            let s = serde_json::to_string(&variant).unwrap();
            assert_eq!(s, format!("\"{wire}\""));
            assert_eq!(
                serde_json::from_str::<MetadataPresence>(&s).unwrap(),
                variant
            );
        }
        for timecode_domain in ["INVALID", "DISCONTINUOUS", "RECOVERED", "ABSENT"] {
            assert!(
                serde_json::from_str::<MetadataPresence>(&format!("\"{timecode_domain}\""))
                    .is_err(),
                "presence ≠ Timecode 域语义: 拒收 {timecode_domain}"
            );
        }
    }

    /// SQ-2: declaration 三态词表快照锁 + Default=Unknown; **拒收 readiness/
    /// publication/bool 语义**（READY/JOINED/CONSUMED/TRUE——Declaration ≠
    /// Readiness ≠ Health ≠ Publication 红线）。
    #[test]
    fn program_rt_04_metadata_join_declaration_vocabulary() {
        assert_eq!(
            JOIN_DECLARATIONS,
            &["PARTICIPATING", "NOT_PRESENT", "UNKNOWN"]
        );
        for (variant, wire) in [
            (MetadataJoinDeclaration::Participating, "PARTICIPATING"),
            (MetadataJoinDeclaration::NotPresent, "NOT_PRESENT"),
            (MetadataJoinDeclaration::Unknown, "UNKNOWN"),
        ] {
            let s = serde_json::to_string(&variant).unwrap();
            assert_eq!(s, format!("\"{wire}\""));
            assert_eq!(
                serde_json::from_str::<MetadataJoinDeclaration>(&s).unwrap(),
                variant
            );
        }
        assert_eq!(
            MetadataJoinDeclaration::default(),
            MetadataJoinDeclaration::Unknown,
            "无观测前态为 Default"
        );
        for forbidden in [
            "READY",
            "JOINED",
            "CONSUMED",
            "NOT_APPLICABLE",
            "TRUE",
            "FALSE",
        ] {
            assert!(
                serde_json::from_str::<MetadataJoinDeclaration>(&format!("\"{forbidden}\""))
                    .is_err(),
                "declaration 禁 readiness/publication/bool 语义: 拒收 {forbidden}"
            );
        }
    }

    /// SQ-5/终裁表: fact wire 键集恰三（kind/source/presence——禁 payload/
    /// timecode/timestamp/scope 字段蔓延）; 缺任一字段 fail-closed。
    #[test]
    fn program_rt_04_metadata_fact_serde_and_key_set() {
        let fact = sample_fact(MetadataType::Scte35, MetadataPresence::Present);
        let json = serde_json::to_value(fact).unwrap();
        let keys: std::collections::BTreeSet<String> =
            json.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            ["kind", "presence", "source"]
                .into_iter()
                .map(String::from)
                .collect(),
            "fact wire 键集恰三——禁字段蔓延（终裁表: payload/timecode/timestamp/scope 四不要）"
        );
        // 完整往返恒等。
        let back: MetadataFact = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(back, fact);
        // 缺字段 fail-closed（A2-2 立规: 零 serde(default)）。
        for drop_key in ["kind", "source", "presence"] {
            let mut partial = json.as_object().unwrap().clone();
            assert!(partial.remove(drop_key).is_some());
            assert!(
                serde_json::from_value::<MetadataFact>(serde_json::Value::Object(partial)).is_err(),
                "缺 {drop_key} 必须 fail-closed"
            );
        }
    }

    /// SQ-4: facts 与 join_declaration 两正交维度——空 Vec+Participating 合法,
    /// 非空 facts+Unknown 同样不被结构禁止。
    #[test]
    fn program_rt_04_metadata_master_orthogonal_dimensions() {
        let empty_participating = MetadataMaster {
            join_declaration: MetadataJoinDeclaration::Participating,
            ..MetadataMaster::new()
        };
        assert!(empty_participating.facts.is_empty());
        assert_eq!(
            serde_json::to_string(&empty_participating).unwrap(),
            r#"{"data_plane":"METADATA","facts":[],"join_declaration":"PARTICIPATING"}"#,
            "空 facts + Participating = 明确知道无额外 metadata 的节目（合法组合）"
        );
        let facts_unknown = MetadataMaster {
            facts: vec![
                sample_fact(MetadataType::Timecode, MetadataPresence::Unknown),
                sample_fact(MetadataType::Caption, MetadataPresence::Present),
            ],
            ..MetadataMaster::new()
        };
        assert_eq!(facts_unknown.facts.len(), 2);
        assert_eq!(
            facts_unknown.join_declaration,
            MetadataJoinDeclaration::Unknown,
            "非空 facts + Unknown 不被结构禁止（正交维度）"
        );
    }

    /// SQ-3 + 立规: master wire 键集恰三（data_plane/facts/join_declaration,
    /// 禁对称装饰外的蔓延）; Default=new() 同值; 缺字段 fail-closed。
    #[test]
    fn program_rt_04_metadata_master_serde_default_fail_closed() {
        assert_eq!(MetadataMaster::default(), MetadataMaster::new());
        let master = MetadataMaster::new();
        let json = serde_json::to_value(&master).unwrap();
        let keys: std::collections::BTreeSet<String> =
            json.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            ["data_plane", "facts", "join_declaration"]
                .into_iter()
                .map(String::from)
                .collect(),
            "master wire 键集恰三——SQ-3 data_plane 为 canonical 身份非对称装饰"
        );
        assert_eq!(
            serde_json::to_string(&master).unwrap(),
            r#"{"data_plane":"METADATA","facts":[],"join_declaration":"UNKNOWN"}"#
        );
        for drop_key in ["data_plane", "facts", "join_declaration"] {
            let mut partial = json.as_object().unwrap().clone();
            assert!(partial.remove(drop_key).is_some());
            assert!(
                serde_json::from_value::<MetadataMaster>(serde_json::Value::Object(partial))
                    .is_err(),
                "缺 {drop_key} 必须 fail-closed（零 serde(default)）"
            );
        }
    }
}
