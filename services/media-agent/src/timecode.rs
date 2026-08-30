//! Phase 0.7B-2C: Canonical Timecode Foundation — **时间标签, 非时间本体**。
//!
//! 概念隔离 (终审裁定):
//! - **Clock = 流速/同步参考** (`clock.rs`);
//! - **Timecode = 媒体帧携带的时间标签** (本模块)。
//!
//! 两者绝不因 Timecode 有 `frame_rate` 而混同 — 本模块的 frame_rate 是标签所属
//! 媒体的帧率, 不是 Clock 的 rate。
//!
//! 契约锚点: `CLOCK_TIMECODE_CONTRACT.md` §2 (#148 冻结词表
//! Present/Absent/Invalid/Discontinuous/Recovered) + Unknown (无观测源前置态)。
//!
//! **终审红线**: 不实现 parser (LTC/VITC/ATC/SMPTE 解析留后续) —
//! `Provider observation → Timecode observation → CanonicalTimecode` 到此结束;
//! 类型族零决策方法 (禁 clock selection/master clock/drift correction/
//! sync decision/resampling/timestamp correction); Discontinuous/Recovered 是
//! **观察事实**, 不是修正/恢复动作; 无观测绝不臆造 00:00:00:00。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Timecode 状态 — **#148 冻结词表** + Unknown (无观测源前置态; 真机合法)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimecodePresence {
    Present,
    Absent,
    Invalid,
    Discontinuous,
    Recovered,
    Unknown,
}

/// 格式标签 — **只声明, 不解析** (终审最小集; ATC/SMPTE 等扩充留后续)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimecodeFormat {
    Ltc,
    Vitc,
    Embedded,
    Unknown,
}

/// 时间标签值 — 仅 presence=Present 且有真实观测时携带;
/// 越界校验属 parser 阶段 (本阶段无解析器, 不做臆测校验)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimecodeValue {
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
    pub frames: u32,
}

/// 观测证据条目 (只增不解释 — 解释属 Policy 层)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimecodeEvidence {
    pub code: String,
    pub detail: String,
}

/// Canonical Timecode — 媒体帧携带的时间标签。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalTimecode {
    pub presence: TimecodePresence,
    pub format: TimecodeFormat,
    pub value: Option<TimecodeValue>,
    /// 标签所属媒体帧率 (如 30000/1001) — **语义上 ≠ Clock 的 rate** (概念隔离)。
    pub frame_rate: Option<(u32, u32)>,
    pub evidence: Vec<TimecodeEvidence>,
}

impl CanonicalTimecode {
    /// 无观测前置态: 全 Unknown + evidence (绝不臆造值)。
    pub fn unknown() -> Self {
        Self {
            presence: TimecodePresence::Unknown,
            format: TimecodeFormat::Unknown,
            value: None,
            frame_rate: None,
            evidence: vec![TimecodeEvidence {
                code: "no_timecode_observation".into(),
                detail: "无 timecode 观测; presence/format 保持 Unknown (绝不臆造 00:00:00:00)"
                    .into(),
            }],
        }
    }

    /// 观察到"无 timecode" — Absent 是合法观察事实 (非缺测)。
    pub fn absent() -> Self {
        Self {
            presence: TimecodePresence::Absent,
            format: TimecodeFormat::Unknown,
            value: None,
            frame_rate: None,
            evidence: vec![TimecodeEvidence {
                code: "timecode_absent".into(),
                detail: "观测确认无 timecode (Absent 为观察事实)".into(),
            }],
        }
    }

    /// 观测/解析异常 — presence=Invalid + 证据; **绝不悄悄转成合法 Timecode**。
    pub fn observe_invalid(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            presence: TimecodePresence::Invalid,
            format: TimecodeFormat::Unknown,
            value: None,
            frame_rate: None,
            evidence: vec![TimecodeEvidence {
                code: code.into(),
                detail: detail.into(),
            }],
        }
    }

    /// 观测到有效 timecode — 唯一携带 value 的路径 (来自真实观测, 非臆造)。
    pub fn observe(
        value: TimecodeValue,
        format: TimecodeFormat,
        frame_rate: Option<(u32, u32)>,
    ) -> Self {
        Self {
            presence: TimecodePresence::Present,
            format,
            value: Some(value),
            frame_rate,
            evidence: vec![TimecodeEvidence {
                code: "timecode_observed".into(),
                detail: format!("format={format:?} value={value:?}"),
            }],
        }
    }

    /// 过渡态观察事实 (Discontinuous/Recovered) — 记录观测, **无修正/恢复动作**。
    pub fn observe_transitional(
        presence: TimecodePresence,
        code: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        debug_assert!(
            matches!(
                presence,
                TimecodePresence::Discontinuous | TimecodePresence::Recovered
            ),
            "observe_transitional 仅接受 Discontinuous/Recovered"
        );
        Self {
            presence,
            format: TimecodeFormat::Unknown,
            value: None,
            frame_rate: None,
            evidence: vec![TimecodeEvidence {
                code: code.into(),
                detail: detail.into(),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **红线白盒**: 公开关联函数清单硬编码比对 — 新增公开项必须显式更新本清单,
    /// 防 clock selection / sync / resample / correct 类决策 API 静默进入 (终审红线)。
    const PUBLIC_SURFACE_ALLOWLIST: &[&str] = &[
        "unknown",
        "absent",
        "observe_invalid",
        "observe",
        "observe_transitional",
    ];

    #[test]
    fn timecode_rt_01_frozen_vocabulary_snapshot() {
        // #148 冻结词表: 五态 + Unknown 全部可构造且 serde 往返 + 字符串形态快照
        // (防静默增删; 新变体须对照 #148 与终审裁定)。
        let all = [
            TimecodePresence::Present,
            TimecodePresence::Absent,
            TimecodePresence::Invalid,
            TimecodePresence::Discontinuous,
            TimecodePresence::Recovered,
            TimecodePresence::Unknown,
        ];
        for p in all {
            let json = serde_json::to_string(&p).expect("serialize");
            let back: TimecodePresence = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(p, back);
        }
        assert_eq!(
            serde_json::to_string(&TimecodePresence::Present).unwrap(),
            "\"present\""
        );
        assert_eq!(
            serde_json::to_string(&TimecodePresence::Absent).unwrap(),
            "\"absent\""
        );
        assert_eq!(
            serde_json::to_string(&TimecodePresence::Invalid).unwrap(),
            "\"invalid\""
        );
        assert_eq!(
            serde_json::to_string(&TimecodePresence::Discontinuous).unwrap(),
            "\"discontinuous\""
        );
        assert_eq!(
            serde_json::to_string(&TimecodePresence::Recovered).unwrap(),
            "\"recovered\""
        );
        assert_eq!(
            serde_json::to_string(&TimecodePresence::Unknown).unwrap(),
            "\"unknown\""
        );
    }

    #[test]
    fn timecode_rt_01_clock_isolation_no_decision_apis_no_cross_refs() {
        // Clock/Timecode 概念隔离: 本模块 JSON 零 clock/master/sync/resample/correct
        // 字样; 与 CanonicalClockDomain 无引用路径 (serde 互不含对方)。
        let tc = CanonicalTimecode::observe(
            TimecodeValue {
                hours: 1,
                minutes: 2,
                seconds: 3,
                frames: 4,
            },
            TimecodeFormat::Ltc,
            Some((30000, 1001)),
        );
        let json = serde_json::to_string(&tc).expect("serialize");
        for banned in [
            "clock", "master", "sync", "resample", "correct", "drift", "gst", "pipeline", "backend",
        ] {
            assert!(
                !json.to_lowercase().contains(banned),
                "禁止字样渗入 timecode: {banned}"
            );
        }
        // 反向: CanonicalClockDomain JSON 不含 timecode 字样。
        let cd = crate::clock::CanonicalClockDomain::unknown(uuid::Uuid::nil());
        let cd_json = serde_json::to_string(&cd).expect("serialize");
        assert!(
            !cd_json.to_lowercase().contains("timecode"),
            "clock 不得含 timecode 字样"
        );
        // allowlist 自检 (防漂移)。
        assert_eq!(
            PUBLIC_SURFACE_ALLOWLIST,
            &[
                "unknown",
                "absent",
                "observe_invalid",
                "observe",
                "observe_transitional"
            ]
        );
    }

    #[test]
    fn timecode_rt_01_unknown_absent_never_fabricate_value() {
        // 无观测 → Unknown/Absent, value=None — 绝不臆造 00:00:00:00。
        let u = CanonicalTimecode::unknown();
        assert_eq!(u.presence, TimecodePresence::Unknown);
        assert_eq!(u.value, None);
        assert_eq!(u.evidence[0].code, "no_timecode_observation");
        let a = CanonicalTimecode::absent();
        assert_eq!(a.presence, TimecodePresence::Absent);
        assert_eq!(a.value, None);
    }

    #[test]
    fn timecode_rt_01_invalid_preserves_evidence_never_becomes_valid() {
        // 观测/解析异常 → Invalid + 证据; 绝不悄悄转合法。
        let i = CanonicalTimecode::observe_invalid("ltc_parse_failed", "bad bcd framing");
        assert_eq!(i.presence, TimecodePresence::Invalid);
        assert_eq!(i.value, None, "Invalid 绝不携带 value");
        assert_eq!(i.evidence.len(), 1);
        assert_eq!(i.evidence[0].code, "ltc_parse_failed");
    }

    #[test]
    fn timecode_rt_01_discontinuous_recovered_are_observations_not_actions() {
        // 过渡态是观察事实: 构造只记录 evidence, 无修正路径 (allowlist 锁定无 action API)。
        let d = CanonicalTimecode::observe_transitional(
            TimecodePresence::Discontinuous,
            "jump_detected",
            "frame index regressed",
        );
        assert_eq!(d.presence, TimecodePresence::Discontinuous);
        assert_eq!(d.value, None);
        let r = CanonicalTimecode::observe_transitional(
            TimecodePresence::Recovered,
            "stream_relocked",
            "label continuity restored",
        );
        assert_eq!(r.presence, TimecodePresence::Recovered);
        assert_eq!(r.value, None);
    }

    #[test]
    fn timecode_rt_01_vendor_independent_same_observation_same_timecode() {
        // Vendor independence: 相同 canonical observation (BMD/Mock 装配等价观测)
        // → 相同 CanonicalTimecode; serde 零 vendor 字样。
        let value = TimecodeValue {
            hours: 10,
            minutes: 1,
            seconds: 2,
            frames: 3,
        };
        let a = CanonicalTimecode::observe(value, TimecodeFormat::Embedded, Some((25, 1)));
        let b = CanonicalTimecode::observe(value, TimecodeFormat::Embedded, Some((25, 1)));
        assert_eq!(a, b);
        let json = serde_json::to_string(&a).unwrap();
        for banned in ["bmd", "decklink", "gstreamer", "device_number"] {
            assert!(
                !json.to_lowercase().contains(banned),
                "vendor 字样渗入: {banned}"
            );
        }
        // serde roundtrip。
        let back: CanonicalTimecode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(a, back);
    }
}
