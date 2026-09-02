//! A2-1: Canonical SwitchPolicy —— V0.2 §1.17 Switch Mode 封闭词表
//!（Program Domain 第一个 Canonical Domain Object, 2026-09-02 用户裁定链 A2-1）。
//!
//! **只描述, 不执行**（Observation≠Configuration 纪律同源）: 切换执行属
//! GStreamer Materialization（A2-7）; 本类型是 Intent/Plan 层的**声明**。
//!
//! 词表 LOCK FINAL（V0.2 §1.17 逐字）:
//! - `PACKET_SWITCH`: 压缩码流层切（GOP 对齐 / SPS/PPS / 时间戳连续性）
//! - `FRAME_SWITCH`:  主备都先 decode → RAW_VIDEO 层切 → 重新 encode
//! - `MASTER_SWITCH`: 主备都先 normalize → 统一输出格式 → 切
//!
//! IO 平面（V0.2 §313-315）: PACKET=COMPRESSED_*→COMPRESSED_*; FRAME=RAW_*→RAW_*;
//! MASTER=RAW_*(post-normalize)→RAW_*。

use serde::{Deserialize, Serialize};

/// Program Domain 错误类型（A2-2+ Masters/MasterJoin/ProgramMaster 复用）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProgramDomainError {
    #[error("未知 SwitchPolicy {0:?}: 受纳词表={ACCEPTED_LIST:?} (V0.2 §1.17, fail-closed)")]
    UnknownSwitchPolicy(String),
}

/// 受纳词表快照（错误信息与测试共用; 与 V0.2 §1.17 逐字一致）。
pub const ACCEPTED_LIST: &[&str] = &["PACKET_SWITCH", "FRAME_SWITCH", "MASTER_SWITCH"];

/// V0.2 §1.17 Switch Mode —— Canonical 封闭词表（序列化名逐字一致, LOCK FINAL）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwitchPolicy {
    /// 压缩码流层切（GOP 对齐 / SPS/PPS / 时间戳连续性）; 主备 codec+profile 完全一致。
    PacketSwitch,
    /// 主备都先 decode → RAW_VIDEO 层切 → 重新 encode; codec 不同 / 跨格式。
    FrameSwitch,
    /// 主备都先 normalize → 统一输出格式 → 切; 不同设备 / 不同色域 / 异构。
    MasterSwitch,
}

/// V0.2 §313-315 各切换模式的 IO 平面。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwitchIoPlane {
    /// PACKET: COMPRESSED_* → COMPRESSED_*（同源 codec）。
    CompressedToCompressed,
    /// FRAME: RAW_* → RAW_*（跨 codec）。
    RawToRaw,
    /// MASTER: RAW_*（post-normalize）→ RAW_*（异构主备）。
    NormalizedRawToRaw,
}

impl SwitchPolicy {
    /// 词表解析——未知值 fail-closed（生产/诊断一致, 绝不静默回退; sink.kind 同纪律）。
    pub fn parse(s: &str) -> Result<Self, ProgramDomainError> {
        match s {
            "PACKET_SWITCH" => Ok(Self::PacketSwitch),
            "FRAME_SWITCH" => Ok(Self::FrameSwitch),
            "MASTER_SWITCH" => Ok(Self::MasterSwitch),
            other => Err(ProgramDomainError::UnknownSwitchPolicy(other.to_string())),
        }
    }

    /// V0.2 §313-315 IO 平面。
    pub fn io_plane(&self) -> SwitchIoPlane {
        match self {
            Self::PacketSwitch => SwitchIoPlane::CompressedToCompressed,
            Self::FrameSwitch => SwitchIoPlane::RawToRaw,
            Self::MasterSwitch => SwitchIoPlane::NormalizedRawToRaw,
        }
    }

    /// V0.2 §1.17 适用条件摘要（"是什么", 非执行检查——执行前置校验属 A2-7 preflight）。
    pub fn precondition(&self) -> &'static str {
        match self {
            Self::PacketSwitch => {
                "主备 codec+profile 完全一致（GOP 边界/timecode 连续/audio timestamp 对齐）"
            }
            Self::FrameSwitch => "codec 不同 / 跨格式（两侧 decode 到 RAW 层后切, 重新 encode）",
            Self::MasterSwitch => "不同设备 / 不同色域 / 异构（两侧 normalize 到统一输出格式后切）",
        }
    }

    /// Canonical 序列化名（serde 同源; 显式提供供非 serde 路径使用）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PacketSwitch => "PACKET_SWITCH",
            Self::FrameSwitch => "FRAME_SWITCH",
            Self::MasterSwitch => "MASTER_SWITCH",
        }
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;

    #[test]
    fn program_rt_01_switch_policy_vocabulary_snapshot() {
        // 词表快照: 恰三词, 与 V0.2 §1.17 逐字一致。
        assert_eq!(ACCEPTED_LIST.len(), 3);
        assert_eq!(
            ACCEPTED_LIST,
            &["PACKET_SWITCH", "FRAME_SWITCH", "MASTER_SWITCH"]
        );
    }

    #[test]
    fn program_rt_01_switch_policy_parse_roundtrip_identity() {
        // review Minor#2: parse↔variant 恒等（词表快照与 serde 锁之外的缺环——
        // match 臂交换也逃不过此断言）。
        for p in [
            SwitchPolicy::PacketSwitch,
            SwitchPolicy::FrameSwitch,
            SwitchPolicy::MasterSwitch,
        ] {
            assert_eq!(SwitchPolicy::parse(p.as_str()).unwrap(), p);
        }
    }

    #[test]
    fn program_rt_01_switch_policy_serde_names_lock() {
        // serde 序列化名逐字锁定（wire 契约锚——未来任何 rename 都是破坏性变更）。
        for (policy, wire) in [
            (SwitchPolicy::PacketSwitch, "PACKET_SWITCH"),
            (SwitchPolicy::FrameSwitch, "FRAME_SWITCH"),
            (SwitchPolicy::MasterSwitch, "MASTER_SWITCH"),
        ] {
            assert_eq!(
                serde_json::to_string(&policy).unwrap(),
                format!("\"{wire}\"")
            );
            let back: SwitchPolicy = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
            assert_eq!(back, policy);
            assert_eq!(policy.as_str(), wire);
        }
    }

    #[test]
    fn program_rt_01_switch_policy_parse_accepted_and_rejected() {
        for w in ACCEPTED_LIST {
            assert!(SwitchPolicy::parse(w).is_ok(), "受纳: {w}");
        }
        // 拒绝: 大小写敏感 / 跨词表污染 / 空串 / 简写 / 旧占位别名不算词。
        for bad in [
            "packet_switch",
            "frame_switch",
            "master_switch",
            "SWITCH",
            "PACKET",
            "FRAME",
            "MASTER",
            "",
            "RTMP",
            "HLS",
            "APPSINK",
            "FRAME_SWITCH ",
            " FRAME_SWITCH",
        ] {
            let err = SwitchPolicy::parse(bad).unwrap_err();
            assert!(
                matches!(err, ProgramDomainError::UnknownSwitchPolicy(_)),
                "{bad:?} 必须拒绝: {err:?}"
            );
            assert!(
                err.to_string().contains("fail-closed"),
                "错误信息含纪律声明: {err}"
            );
        }
    }

    #[test]
    fn program_rt_01_switch_policy_serde_unknown_fails_closed() {
        // serde 反序列化路径与 parse 同牙齿（无 default 回退）。
        let r: Result<SwitchPolicy, _> = serde_json::from_str("\"PACKET_SWITCH_LEGACY\"");
        assert!(r.is_err(), "serde 未知串必须 fail-closed");
        let r: Result<SwitchPolicy, _> = serde_json::from_str("\"frame_switch\"");
        assert!(r.is_err(), "serde 大小写敏感");
    }

    #[test]
    fn program_rt_01_switch_policy_io_plane_matches_v02() {
        // V0.2 §313-315: PACKET=COMPRESSED→COMPRESSED; FRAME=RAW→RAW; MASTER=RAW(post-norm)→RAW。
        assert_eq!(
            SwitchPolicy::PacketSwitch.io_plane(),
            SwitchIoPlane::CompressedToCompressed
        );
        assert_eq!(
            SwitchPolicy::FrameSwitch.io_plane(),
            SwitchIoPlane::RawToRaw
        );
        assert_eq!(
            SwitchPolicy::MasterSwitch.io_plane(),
            SwitchIoPlane::NormalizedRawToRaw
        );
        // IO 平面 serde 名锁定（wire 稳定）。
        assert_eq!(
            serde_json::to_string(&SwitchIoPlane::NormalizedRawToRaw).unwrap(),
            "\"NORMALIZED_RAW_TO_RAW\""
        );
    }

    #[test]
    fn program_rt_01_switch_policy_precondition_nonempty() {
        for p in [
            SwitchPolicy::PacketSwitch,
            SwitchPolicy::FrameSwitch,
            SwitchPolicy::MasterSwitch,
        ] {
            assert!(!p.precondition().is_empty(), "适用条件摘要在场: {p:?}");
        }
    }
}
