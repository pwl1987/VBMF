//! A2-5-04: ProgramMaster —— Program Domain 第六块（V0.2 §1.20 Composition
//! Root）。
//!
//! **定位（A2-5-00..04 终裁链）**: ProgramMaster = **Program Domain 组合根**，
//! 不是第四个 Master Pipeline——只把三路 Master + **已形成的** Join Result
//! 组合成一个 Program-scope Domain Object。Master Join 是判定者不是父类；
//! 本类型零判定零执行（compose 不重新 Join——PM-05）。
//!
//! **架构八 Gate（PM-01..08, 终裁 §11）**: 四字段存在 / 三 Master 整值组合
//! （"绝不平铺"——PortMediaSemantics 终审红线同律, wire 级由键集正反向测试
//! 锁）/ `join_result: Option<MasterJoinResult>`（None=尚未形成 Join Result）
//! / AVSyncClassification 不进入（双 SoT 禁——唯一家 master_join.rs）/
//! compose 纯组合 / serde 顶层四键禁平铺 / 零 serde(default)（Option 已表达
//! absence） / 零 Runtime/Health/Action/Time/Revision 污染。
//!
//! **禁入清单（终裁 §10）**: eligibility/ready/classification_input/
//! AVSyncClassification/timestamp/observation_revision/health/status/action/
//! recovery/channel_id/program_id/scope/stage/payload/measurement/threshold/
//! join() 内嵌/from_join()/serde(default)/Master trait。

use crate::program::audio_master::AudioMaster;
use crate::program::master_join::MasterJoinResult;
use crate::program::metadata_master::MetadataMaster;
use crate::program::video_master::VideoMaster;
use serde::{Deserialize, Serialize};

/// ProgramMaster —— Program-scope Composition Root（A2-5-04 终裁批准版）。
///
/// **Default 语义（终裁收紧 #1）**: `Default` **仅提供结构性零参构造便利**;
/// `join_result: None` 表示当前尚未形成 Join Result。Default 本身**不构成
/// 任何健康、就绪或故障语义**——不是"未初始化/未 Ready/UNKNOWN
/// ProgramMaster"（三 Master 的 Default 是各自声明对象的初始值, 如
/// VideoMaster::SourceRaw, 非对称是声明性有意设计）。
///
/// `join_result` 是**已形成的结果快照**（由 `join()` 产出后经 `compose` 组入）,
/// 不是本对象计算的事实。PartialEq-only（AudioMaster f32 同律）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProgramMaster {
    pub video: VideoMaster,
    pub audio: AudioMaster,
    pub metadata: MetadataMaster,
    pub join_result: Option<MasterJoinResult>,
}

impl ProgramMaster {
    /// 纯组合器——**唯一构造入口**（终裁收紧 #2）。
    ///
    /// 不内嵌 `join()`/`is_ready()`/`validate_join()`/任何 avsync 或 metadata
    /// consistency 判断（PM-05: 结果由 `join()` 单一判定入口产出, 本方法只
    /// 组合）; 不提供 `from_join()`（防第二判定入口——A2-6 真实消费者出现
    /// 后再反推）。
    pub fn compose(
        video: VideoMaster,
        audio: AudioMaster,
        metadata: MetadataMaster,
        join_result: Option<MasterJoinResult>,
    ) -> Self {
        Self {
            video,
            audio,
            metadata,
            join_result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::program::metadata_master::MetadataJoinDeclaration;

    /// PM-01/02/03/05: compose 纯组合——四字段逐一恒等, 且**不校正不重算**
    /// （传入与三 Master 状态"看似矛盾"的 join_result 仍原样保留, 证明
    /// compose 不内嵌任何 Join 判定）。
    #[test]
    fn program_rt_06_program_master_compose_is_pure() {
        let video = VideoMaster::new();
        let audio = AudioMaster::new();
        let metadata = MetadataMaster::new();
        let pm = ProgramMaster::compose(
            video,
            audio,
            metadata.clone(),
            Some(MasterJoinResult::Acceptable),
        );
        assert_eq!(pm.video, VideoMaster::new());
        assert_eq!(pm.audio, AudioMaster::new());
        assert_eq!(pm.metadata, metadata);
        assert_eq!(pm.join_result, Some(MasterJoinResult::Acceptable));

        // PM-05 行为证: 三 Master 全在 Source 起点（远非 Ready）+ Some(Acceptable)
        // ——compose 原样保留, 零重算零校正。
        let odd = ProgramMaster::compose(
            VideoMaster::new(),
            AudioMaster::new(),
            MetadataMaster::new(),
            Some(MasterJoinResult::Degraded),
        );
        assert_eq!(odd.join_result, Some(MasterJoinResult::Degraded));
        assert_eq!(odd.video, VideoMaster::default());
    }

    /// PM-06: serde 顶层键**正向**必存在四键; **反向**不得出现平铺键与
    /// 污染键（终裁收紧 #3——正反向表达架构意图, 不绑死 serde 实现细节）。
    #[test]
    fn program_rt_06_program_master_serde_top_keys() {
        let pm = ProgramMaster::compose(
            VideoMaster::new(),
            AudioMaster::new(),
            MetadataMaster {
                join_declaration: MetadataJoinDeclaration::Participating,
                ..MetadataMaster::new()
            },
            None,
        );
        let json = serde_json::to_value(&pm).unwrap();
        let obj = json.as_object().unwrap();
        for must in ["video", "audio", "metadata", "join_result"] {
            assert!(obj.contains_key(must), "顶层必须存在 {must}");
        }
        // 平铺键（三 Master 字段直译）与污染键（跨平面概念）一律禁入。
        for banned in [
            "video_stage",
            "video_data_plane",
            "audio_stage",
            "audio_data_plane",
            "audio_delay_ms",
            "metadata_facts",
            "metadata_join_declaration",
            "avsync",
            "health",
            "status",
            "ready",
            "revision",
            "timestamp",
            "channel_id",
            "program_id",
            "scope",
            "action",
        ] {
            assert!(!obj.contains_key(banned), "顶层禁入 {banned}（平铺/污染）");
        }
        // 整值组合: video/audio 是嵌套对象（非平铺标量）。
        assert!(obj.get("video").unwrap().is_object());
        assert!(obj.get("metadata").unwrap().is_object());
    }

    /// PM-07 + Default 收紧语义: 三 Master 字段缺失 fail-closed（零
    /// serde(default)）; `join_result` 缺失 = **serde Option 内建 absence
    /// 语义**（→None——终裁 §8 "Option 本身已经表达 absence", 非标注
    /// serde(default) 所致）; Default = 结构性零参便利（join_result==None
    /// + 三 Master==各自 Default, 不构成健康/就绪/故障语义）。
    #[test]
    fn program_rt_06_program_master_no_serde_default_and_default_semantics() {
        let pm = ProgramMaster::compose(
            VideoMaster::new(),
            AudioMaster::new(),
            MetadataMaster::new(),
            None,
        );
        let json = serde_json::to_value(&pm).unwrap();
        // 三 Master 字段（非 Option）缺失必须 fail-closed——零 serde(default)。
        for drop_key in ["video", "audio", "metadata"] {
            let mut partial = json.as_object().unwrap().clone();
            assert!(partial.remove(drop_key).is_some());
            assert!(
                serde_json::from_value::<ProgramMaster>(serde_json::Value::Object(partial))
                    .is_err(),
                "缺 {drop_key} 必须 fail-closed（零 serde(default)）"
            );
        }
        // join_result 缺失 = Option 内建 absence（→None）, 与显式 None 等价。
        let mut no_result = json.as_object().unwrap().clone();
        assert!(no_result.remove("join_result").is_some());
        let back: ProgramMaster =
            serde_json::from_value(serde_json::Value::Object(no_result)).unwrap();
        assert_eq!(
            back.join_result, None,
            "Option absence 语义（非 serde(default)）"
        );
        // Default 收紧语义（终裁 #1）: 结构性便利, 非任何状态宣称。
        let d = ProgramMaster::default();
        assert_eq!(d.join_result, None, "Default 的 join_result=None=尚未形成");
        assert_eq!(d.video, VideoMaster::default());
        assert_eq!(d.audio, AudioMaster::default());
        assert_eq!(d.metadata, MetadataMaster::default());
    }

    /// PM-03/04/08 联动: join_result 三态（None/Some(Degraded)/Some(Failed)）
    /// 携带不变; Some(Degraded) 往返恒等（Result 词表在 master_join 已锁,
    /// 此处锁组合根携带不丢失）。
    #[test]
    fn program_rt_06_program_master_result_roundtrip_carried() {
        for result in [
            None,
            Some(MasterJoinResult::Degraded),
            Some(MasterJoinResult::Failed),
        ] {
            let pm = ProgramMaster::compose(
                VideoMaster::new(),
                AudioMaster::new(),
                MetadataMaster {
                    join_declaration: MetadataJoinDeclaration::NotPresent,
                    ..MetadataMaster::new()
                },
                result,
            );
            let back: ProgramMaster =
                serde_json::from_value(serde_json::to_value(&pm).unwrap()).unwrap();
            assert_eq!(back.join_result, result, "join_result 携带不变");
            assert_eq!(back, pm);
        }
    }
}
