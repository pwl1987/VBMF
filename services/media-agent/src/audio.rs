//! Phase 0.7B-2B: Canonical Audio Semantics — 回答"这个音频是什么"，
//! **不回答"应该怎么处理"**。
//!
//! 契约锚点: `CANONICAL_MEDIA_MODEL.md` §4 — Audio 独立建模 (第 9 替换轴);
//! Audio 不当 Video 附属字段; embedding 语义 Embedded/De-embedded/Independent/
//! Mixed/External 显式化。
//!
//! **终审红线**: 无 AudioMixer / AudioBusManager / ChannelAllocator / Gain /
//! DelayCompensation / sample clock — 这些属后续 Runtime; Unknown 贯穿
//! (presence/role 绝不默认 Program); `AudioRouteIntent` 是 Semantic Intent,
//! 类型层面不可能产出 pipeline/backend/gst 引用 (纪律①同构)。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::normalize::CanonicalAudioDescription;

/// 语义角色 — 冻结词表。Main/Backup/Emergency/TX/RX 是业务词汇, 冻结禁止。
/// 默认 Unknown, 绝不默认 Program (与 0.7A absence≠evidence 同构)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioRole {
    Program,
    Commentary,
    Ambient,
    Auxiliary,
    Unknown,
}

/// 布局 — 只描述 (Downmix/Upmix/Normalize 是处理, 禁止)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioLayout {
    Mono,
    Stereo,
    FiveOne,
    SevenOne,
    Unknown,
}

/// 采样格式最小集 (0.7B-2B 无观测源 → 恒 None/Unknown)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSampleFormat {
    PcmS16,
    PcmS24,
    PcmS32,
    PcmFloat,
    Unknown,
}

/// 观测证据条目 (只增不解释 — 解释属 Policy 层)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationEvidence {
    pub code: String,
    pub detail: String,
}

/// Audio stream 语义 ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AudioStreamId(pub Uuid);

impl std::fmt::Display for AudioStreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "audio-{}", self.0.simple())
    }
}

/// Canonical Audio Stream — 回答"这个音频是什么"。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalAudioStream {
    pub id: AudioStreamId,
    /// 复用 normalize 既有三态 (Present{channels_hint}/NotPresent/Unknown) —
    /// 终审"保持三态不要改"; Absent ≡ NotPresent (既有命名)。
    pub presence: crate::normalize::AudioPresence,
    pub channels: Option<u32>,
    pub sample_rate: Option<u32>,
    pub sample_format: Option<AudioSampleFormat>,
    pub layout: AudioLayout,
    pub role: AudioRole,
    pub evidence: Vec<ObservationEvidence>,
}

impl CanonicalAudioStream {
    /// Unknown 贯穿构造器: 无观测时 presence/role 全 Unknown + evidence 记录
    /// (绝不默认 Program)。
    pub fn unknown(id: AudioStreamId) -> Self {
        Self {
            id,
            presence: crate::normalize::AudioPresence::Unknown,
            channels: None,
            sample_rate: None,
            sample_format: None,
            layout: AudioLayout::Unknown,
            role: AudioRole::Unknown,
            evidence: vec![ObservationEvidence {
                code: "no_audio_observation".into(),
                detail: "无 audio 观测; presence/role 保持 Unknown (绝不默认 Program)".into(),
            }],
        }
    }

    /// Normalize → Stream 桥 (D5 纯映射): presence 直映; 采样细节 0.7B-2B 无观测源 →
    /// None; role/layout → Unknown (语义角色属声明/Policy, 绝不由观测臆造)。
    pub fn from_description(id: AudioStreamId, desc: &CanonicalAudioDescription) -> Self {
        let mut evidence = Vec::new();
        let presence = match desc.presence {
            crate::normalize::AudioPresence::Present { channels_hint } => {
                evidence.push(ObservationEvidence {
                    code: "audio_observed".into(),
                    detail: format!("channels_hint={channels_hint:?}"),
                });
                crate::normalize::AudioPresence::Present { channels_hint }
            }
            other => other,
        };
        Self {
            id,
            presence,
            channels: None,
            sample_rate: None,
            sample_format: None,
            layout: AudioLayout::Unknown,
            role: AudioRole::Unknown,
            evidence,
        }
    }
}

// ── AudioRouteIntent (Semantic Intent; 纪律① 同构) ────────────────────────────

/// 路由目标 — 语义层。`Role` = 目标语义角色; `Named` = opaque 语义标签
/// (**非**路由配置; 命名 bus 属后续 Runtime/Control Plane)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSemanticTarget {
    Role(AudioRole),
    Named(String),
}

/// 路由策略 — 0.7B-2B 仅 required 标记; mix/duck/switch 等 policy 词汇属后续
/// Runtime/Control Plane。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutePolicy {
    pub required: bool,
}

/// Audio 路由**语义意图** — 从 `CanonicalAudioStream` 语义映射 (如 Embedded 相机音
/// → Role(Program))。类型层面不可能产出 pipeline/backend/gst 引用 (纪律①同构)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioRouteIntent {
    pub source: AudioStreamId,
    pub destination: AudioSemanticTarget,
    pub policy: RoutePolicy,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::{AudioPresence as NormalizePresence, CanonicalAudioDescription};

    /// 构造 normalize 侧 audio description (BMD 形状: present + embedded)。
    fn bmd_audio_description() -> CanonicalAudioDescription {
        CanonicalAudioDescription {
            presence: crate::normalize::AudioPresence::Present {
                channels_hint: None,
            },
            embedding: crate::normalize::AudioEmbedding::Embedded,
        }
    }

    fn mock_audio_description() -> CanonicalAudioDescription {
        bmd_audio_description() // 同一逻辑媒体 (相同观测语义)
    }

    #[test]
    fn audio_semantics_rt_01_provider_independent_same_media_same_stream() {
        // 不同 Provider (BMD 形状 vs Mock 形状) 的同一逻辑音频 → 同一 canonical stream
        // 媒体语义 (provider 装配差异不得渗入)。
        let a = AudioStreamId(Uuid::new_v4());
        let stream_a = CanonicalAudioStream::from_description(a, &bmd_audio_description());
        let stream_b = CanonicalAudioStream::from_description(a, &mock_audio_description());
        assert_eq!(stream_a, stream_b);
        assert_eq!(
            stream_a.presence,
            NormalizePresence::Present {
                channels_hint: None
            }
        );
        // embedding 语义来自 description。
        assert!(
            stream_a.evidence.iter().any(|e| e.code == "audio_observed"),
            "from_description 应记录 audio_observed 证据"
        );
    }

    #[test]
    fn audio_semantics_rt_01_unknown_throughout_never_defaults_program() {
        // 终审 Unknown 测试: 无 audio observation → presence=Unknown + role=Unknown,
        // **禁止默认 Program** (与 0.7A absence≠evidence 同构)。
        let id = AudioStreamId(Uuid::new_v4());
        let s = CanonicalAudioStream::unknown(id);
        assert_eq!(s.presence, NormalizePresence::Unknown);
        assert_eq!(s.role, AudioRole::Unknown);
        assert_eq!(s.layout, AudioLayout::Unknown);
        assert_eq!(s.channels, None);
        assert_eq!(s.sample_rate, None);
        assert_eq!(s.sample_format, None);
        assert_eq!(s.evidence.len(), 1);
        assert_eq!(s.evidence[0].code, "no_audio_observation");
        // from_description 的 Unknown presence 同样不产 Program。
        let desc = CanonicalAudioDescription {
            presence: NormalizePresence::Unknown,
            embedding: crate::normalize::AudioEmbedding::Embedded,
        };
        let from_desc = CanonicalAudioStream::from_description(id, &desc);
        assert_eq!(from_desc.role, AudioRole::Unknown);
    }

    #[test]
    fn audio_semantics_rt_01_route_intent_has_no_pipeline_refs() {
        // 终审 Route 测试: A/B streams → AudioRouteIntent — serde JSON 反向断言
        // 零 pipeline/backend/gst 引用 (Semantic Intent ≠ Execution)。
        let a = AudioStreamId(Uuid::new_v4());
        let b = AudioStreamId(Uuid::new_v4());
        let intent = AudioRouteIntent {
            source: a,
            destination: AudioSemanticTarget::Role(AudioRole::Program),
            policy: RoutePolicy { required: true },
        };
        let json = serde_json::to_string(&intent).expect("serialize");
        for banned in ["gst", "pipeline", "backend", "mixer", "gain"] {
            assert!(
                !json.to_lowercase().contains(banned),
                "禁止字样渗入: {banned}"
            );
        }
        // source 未被消费 — B stream 仍可生成自己的 intent (语义意图互不排斥)。
        let intent_b = AudioRouteIntent {
            source: b,
            destination: AudioSemanticTarget::Named("ambient-bus".into()),
            policy: RoutePolicy { required: false },
        };
        assert_ne!(intent.source, intent_b.source);
        // serde roundtrip。
        let back: AudioRouteIntent =
            serde_json::from_str(&serde_json::to_string(&intent).unwrap()).expect("deserialize");
        assert_eq!(intent, back);
    }

    #[test]
    fn audio_role_frozen_vocabulary_snapshot() {
        // Role 冻结词表快照 (防静默增删; 新变体须对照终审裁定 — 业务词禁止)。
        assert_eq!(
            serde_json::to_string(&AudioRole::Program).unwrap(),
            "\"program\""
        );
        assert_eq!(
            serde_json::to_string(&AudioRole::Commentary).unwrap(),
            "\"commentary\""
        );
        assert_eq!(
            serde_json::to_string(&AudioRole::Ambient).unwrap(),
            "\"ambient\""
        );
        assert_eq!(
            serde_json::to_string(&AudioRole::Auxiliary).unwrap(),
            "\"auxiliary\""
        );
        assert_eq!(
            serde_json::to_string(&AudioRole::Unknown).unwrap(),
            "\"unknown\""
        );
    }
}
