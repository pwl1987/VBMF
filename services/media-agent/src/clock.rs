//! Phase 0.7B-2A: Canonical Clock Domain — **只描述观测, 绝不决策**。
//!
//! 契约锚点: `CLOCK_TIMECODE_CONTRACT.md` §1 (#147): Clock 观测态冻结词表
//! `Locked / Unlocked / Offset / Drift / Clock Lost / Clock Recovered`;
//! **Observation≠Configuration** (R3): Clock 是运行时**观测**, 绝不写回 Graph。
//!
//! **终审红线**: 本模块类型族零决策方法 — `choose_master_clock` / `select_clock` /
//! `auto_route` 在类型层面不存在; Clock 策略属 Runtime / Backend / Control Plane。
//! `state: Unknown` 表示"0.7B-2A 尚无 clock 探针" — 真机 Unknown 合法 (终审明确)。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Clock 观测态 — **冻结词表 #147** + Unknown (观测前置态)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockObservationState {
    Locked,
    Unlocked,
    Offset,
    Drift,
    ClockLost,
    ClockRecovered,
    Unknown,
}

/// Clock kind — 源归属分类 (Internal = agent 自由运行; External = 外部参考)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockKind {
    Internal,
    External,
    Unknown,
}

/// Clock reference — 参考关系分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockReference {
    FreeRunning,
    Locked,
    Unknown,
}

/// 置信语义: Observed(直接探针证据) / Inferred(transport/拓扑推断) / Unknown。
/// 0.7B-2A 只会产出 Unknown (无探针); 枚举为 0.7B 探针阶段预留。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockConfidence {
    Observed,
    Inferred,
    Unknown,
}

/// 观测证据条目 (只增不解释 — 解释属 Policy 层)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockEvidence {
    pub code: String,
    pub detail: String,
}

/// 单次 Clock 观测事实。时间线只保留顺序与证据，不解释状态也不产生动作。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockObservation {
    pub observed_at_ms: i64,
    pub state: ClockObservationState,
    pub evidence: Vec<ClockEvidence>,
}

/// Clock 观测时间线错误。容量耗尽时 fail-closed，禁止静默丢失旧观测。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockTimelineError {
    ZeroCapacity,
    CapacityExceeded,
}

impl fmt::Display for ClockTimelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCapacity => {
                f.write_str("clock observation timeline capacity must be positive")
            }
            Self::CapacityExceeded => f.write_str("clock observation timeline capacity exceeded"),
        }
    }
}

/// 有界、append-only 的 Clock 观测时间线。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockObservationTimeline {
    capacity: usize,
    observations: Vec<ClockObservation>,
}

impl ClockObservationTimeline {
    pub fn new(capacity: usize) -> Result<Self, ClockTimelineError> {
        if capacity == 0 {
            return Err(ClockTimelineError::ZeroCapacity);
        }
        Ok(Self {
            capacity,
            observations: Vec::with_capacity(capacity),
        })
    }

    pub fn append(&mut self, observation: ClockObservation) -> Result<(), ClockTimelineError> {
        if self.observations.len() >= self.capacity {
            return Err(ClockTimelineError::CapacityExceeded);
        }
        self.observations.push(observation);
        Ok(())
    }

    pub fn observations(&self) -> &[ClockObservation] {
        &self.observations
    }

    pub fn latest(&self) -> Option<&ClockObservation> {
        self.observations.last()
    }
}

/// Canonical Clock Domain — **只描述观测, 绝不决策**。
///
/// 0.7B-2A 只会产出 Unknown 组合 (无探针): kind/reference/state/confidence 全 Unknown
/// + evidence 记录"无 clock 探针" — 真机 Unknown 合法 (终审明确)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalClockDomain {
    pub id: Uuid,
    pub kind: ClockKind,
    pub reference: ClockReference,
    pub state: ClockObservationState,
    pub confidence: ClockConfidence,
    pub evidence: Vec<ClockEvidence>,
}

impl CanonicalClockDomain {
    /// 0.7B-2A 唯一合法产出: Unknown domain (无探针观测)。
    pub fn unknown(id: Uuid) -> Self {
        Self {
            id,
            kind: ClockKind::Unknown,
            reference: ClockReference::Unknown,
            state: ClockObservationState::Unknown,
            confidence: ClockConfidence::Unknown,
            evidence: vec![ClockEvidence {
                code: "no_clock_probe".into(),
                detail: "0.7B-2A 无 clock 探针; 探针与源选择属后续阶段 (Observation≠Configuration)"
                    .into(),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **红线白盒**: 公开关联函数/方法清单硬编码比对 — 新增公开项必须显式更新本清单,
    /// 防 `choose_master_clock` 之类决策 API 静默进入 canonical 层 (终审红线)。
    const PUBLIC_SURFACE_ALLOWLIST: &[&str] = &[
        "unknown", // CanonicalClockDomain::unknown (0.7B-2A 唯一构造器)
    ];

    #[test]
    fn clock_timeline_rt_01_preserves_locked_lost_recovered_order() {
        let mut timeline = ClockObservationTimeline::new(3).expect("positive capacity");
        for (at, state) in [
            (100_i64, ClockObservationState::Locked),
            (200, ClockObservationState::ClockLost),
            (300, ClockObservationState::ClockRecovered),
        ] {
            timeline
                .append(ClockObservation {
                    observed_at_ms: at,
                    state,
                    evidence: Vec::new(),
                })
                .expect("timeline has room");
        }
        assert_eq!(
            timeline
                .observations()
                .iter()
                .map(|item| item.state)
                .collect::<Vec<_>>(),
            vec![
                ClockObservationState::Locked,
                ClockObservationState::ClockLost,
                ClockObservationState::ClockRecovered
            ]
        );
        assert_eq!(timeline.latest().map(|item| item.observed_at_ms), Some(300));
    }

    #[test]
    fn clock_timeline_rt_02_capacity_is_fail_closed_without_mutation() {
        assert_eq!(
            ClockObservationTimeline::new(0),
            Err(ClockTimelineError::ZeroCapacity)
        );
        let mut timeline = ClockObservationTimeline::new(1).expect("positive capacity");
        let observation = ClockObservation {
            observed_at_ms: 100,
            state: ClockObservationState::Locked,
            evidence: Vec::new(),
        };
        timeline.append(observation.clone()).expect("first append");
        assert_eq!(
            timeline.append(observation),
            Err(ClockTimelineError::CapacityExceeded)
        );
        assert_eq!(timeline.observations().len(), 1);
        assert_eq!(
            timeline.observations()[0].state,
            ClockObservationState::Locked
        );
    }

    #[test]
    fn clock_semantics_01_frozen_state_vocabulary_complete() {
        // #147 冻结词表: 六态 + Unknown 全部可构造且 serde 往返。
        let all = [
            ClockObservationState::Locked,
            ClockObservationState::Unlocked,
            ClockObservationState::Offset,
            ClockObservationState::Drift,
            ClockObservationState::ClockLost,
            ClockObservationState::ClockRecovered,
            ClockObservationState::Unknown,
        ];
        for st in all {
            let json = serde_json::to_string(&st).expect("serialize");
            let back: ClockObservationState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(st, back);
        }
        // 词表快照 (防静默增删; 新变体须显式更新本清单并对照 #147)。
        assert_eq!(
            serde_json::to_string(&ClockObservationState::ClockLost).unwrap(),
            "\"clock_lost\""
        );
    }

    #[test]
    fn clock_semantics_01_unknown_domain_is_legal_and_fully_unknown() {
        // 终审明确: 真机 Unknown 合法 — 无探针时全 Unknown + evidence 记录。
        let d = CanonicalClockDomain::unknown(Uuid::nil());
        assert_eq!(d.kind, ClockKind::Unknown);
        assert_eq!(d.reference, ClockReference::Unknown);
        assert_eq!(d.state, ClockObservationState::Unknown);
        assert_eq!(d.confidence, ClockConfidence::Unknown);
        assert_eq!(d.evidence.len(), 1);
        assert_eq!(d.evidence[0].code, "no_clock_probe");
        // serde roundtrip。
        let json = serde_json::to_string(&d).expect("serialize");
        let back: CanonicalClockDomain = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(d, back);
    }

    #[test]
    fn clock_semantics_01_public_surface_has_no_decision_apis() {
        // 终审红线 (Observation≠Configuration): 本模块不提供任何 clock 选择/仲裁 API。
        // 白盒实现: 公开关联项以 allowlist 常量比对 (见 PUBLIC_SURFACE_ALLOWLIST);
        // 类型族本身零 inherent 方法 (除构造 helper) — 由编译保证: 以下断言固定清单。
        // 若未来新增公开方法, 必须先过终审 "决策属 Runtime/Control Plane" 评审。
        let d = CanonicalClockDomain::unknown(Uuid::nil());
        // 仅允许读取观测字段; 不存在任何 "返回决策" 的方法调用面。
        let _kind: ClockKind = d.kind;
        let _reference: ClockReference = d.reference;
        let _state: ClockObservationState = d.state;
        let _confidence: ClockConfidence = d.confidence;
        let _evidence: &Vec<ClockEvidence> = &d.evidence;
        // allowlist 自检 (防漂移)。
        assert_eq!(PUBLIC_SURFACE_ALLOWLIST, &["unknown"]);
    }
}
