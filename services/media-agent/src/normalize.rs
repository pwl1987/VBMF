//! Phase 0.7B-1: Normalize Foundation — Raw Device Description → Canonical Media Description。
//!
//! 契约锚点: `CANONICAL_MEDIA_MODEL.md` §1 (冻结类型≠全量实现, 只填当前用到字段)、
//! §4 (Audio 独立建模: Embedded/De-embedded/Independent/Mixed/External)、
//! §5 (canonical 类型零 vendor 字段, 被 Domain/Graph/Session/Health 共享)。
//!
//! **三条设计纪律 (终审冻结)**:
//! ① Normalize 不吞 Runtime Intent — 本模块只描述"是什么", 返回类型层面即不可能
//!    构造 pipeline/intent; Execution Plan 属未来阶段 (`Input → Normalize → Canonical
//!    Media Model → Execution Plan → MediaBackend`)。
//! ② Audio 独立 Flow — `CanonicalAudioDescription` 与 video 平级, 非 Option 附属;
//!    embedding 五语义显式建模 (契约 §4)。
//! ③ Clock 不被 Backend 偷走 — `CanonicalClockRef` 仅引用 Domain, 本模块绝不决策。
//!
//! 纯函数保证: 无 IO、无锁、无全局; 同输入恒同输出 (NORMALIZE-RT-01 provider 无关性前提)。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::port::{ConnectorType, PortInfo};

// ── Canonical 类型 (契约 §1: 只填当前用到字段) ─────────────────────────────────

/// canonical 帧率 (num/den; 观测 "30000/1001" 结构化, 解析失败不丢观测 → 走 diagnostics)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalFrameRate {
    pub num: u32,
    pub den: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalVideoDescription {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<CanonicalFrameRate>,
    pub interlaced: Option<bool>,
    pub pixel_format: Option<String>,
}

impl CanonicalVideoDescription {
    fn unknown() -> Self {
        Self {
            width: None,
            height: None,
            frame_rate: None,
            interlaced: None,
            pixel_format: None,
        }
    }
}

/// Audio embedding 五语义 (契约 §4: Embedded/De-embedded/Independent/Mixed/External;
/// 绝不当 Video 的 Option 附属字段)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioEmbedding {
    Embedded,
    DeEmbedded,
    Independent,
    Mixed,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioPresence {
    Present { channels_hint: Option<u32> },
    NotPresent,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalAudioDescription {
    pub presence: AudioPresence,
    pub embedding: AudioEmbedding,
}

/// Clock 引用占位 (纪律③): 只引用 Domain, 本模块绝不决策 clock 策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalClockRef {
    pub domain: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalSourceRef {
    pub device_id: Uuid,
    pub port_id: Option<Uuid>,
}

/// Canonical 媒体描述 — Normalize Foundation 的核心产出。
/// 零 vendor 字段 (契约 §5); serde 可序列化 (证据输出)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalMediaDescriptor {
    pub source: CanonicalSourceRef,
    pub transport: String,
    pub video: CanonicalVideoDescription,
    pub audio: CanonicalAudioDescription,
    pub clock: CanonicalClockRef,
}

// ── Raw 输入侧 (provider 中立装配体; port.rs 观测类型不被替换, 仅转换) ──────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedVideo {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<(u32, u32)>,
    pub interlaced: Option<bool>,
    pub pixel_format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedMedia {
    pub video: Option<ObservedVideo>,
    pub audio_present: Option<bool>,
}

/// Raw 设备输入描述 (provider 中立; 由 PortInfo/SignalStatus 观测装配)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawInputDescription {
    pub device_id: Uuid,
    pub port_id: Option<Uuid>,
    pub transport: String,
    pub observed: Option<ObservedMedia>,
}

impl RawInputDescription {
    /// 由 `PortInfo` 观测装配 (canonical 层内转换; runtime 探测类型保持不动)。
    /// `SignalStatus.video_format.frame_rate` ("30000/1001") 解析为 (num, den);
    /// 解析失败 → None + WARN diagnostic (不丢观测, 不臆造)。
    pub fn from_port(port: &PortInfo) -> Self {
        let video_format = port.signal.video_format.as_ref();
        let frame_rate = video_format
            .and_then(|v| v.frame_rate.as_deref())
            .and_then(parse_frame_rate);
        let observed = video_format.map(|v| ObservedMedia {
            video: Some(ObservedVideo {
                width: Some(v.width),
                height: Some(v.height),
                frame_rate,
                interlaced: v.interlaced,
                pixel_format: v.pixel_format.clone(),
            }),
            audio_present: port.signal.audio_locked,
        });
        Self {
            device_id: port.device_id,
            port_id: port.identity.port_id,
            transport: transport_label(port.identity.connector),
            observed,
        }
    }
}

fn transport_label(c: ConnectorType) -> String {
    match c {
        ConnectorType::Sdi => "sdi".into(),
        ConnectorType::Hdmi => "hdmi".into(),
        ConnectorType::DisplayPort => "displayport".into(),
        ConnectorType::Optical => "optical-sdi".into(),
        ConnectorType::Analog => "analog".into(),
        ConnectorType::Unknown => "unknown".into(),
    }
}

/// 解析 "30000/1001" 形状 → (num, den)。
fn parse_frame_rate(s: &str) -> Option<(u32, u32)> {
    let (num, den) = s.split_once('/')?;
    let num = num.trim().parse::<u32>().ok()?;
    let den = den.trim().parse::<u32>().ok()?;
    (den != 0).then_some((num, den))
}

// ── normalize_input (纯函数; judge-only 描述层) ────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Info,
    Warn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizeDiagnostic {
    pub level: DiagnosticLevel,
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizeOutcome {
    pub descriptor: CanonicalMediaDescriptor,
    pub diagnostics: Vec<NormalizeDiagnostic>,
}

/// Raw → Canonical 归一化。**纯函数** (无 IO/锁/全局; 同输入恒同输出)。
/// 纪律①: 观测缺失 → `Unknown`/`None` + WARN diagnostic, **绝不臆造、绝不回退默认格式**;
/// 纪律③: clock 恒引用占位 + INFO (策略属 0.7B Clock 阶段)。
pub fn normalize_input(raw: &RawInputDescription) -> NormalizeOutcome {
    let mut diagnostics: Vec<NormalizeDiagnostic> = Vec::new();

    let (video, audio, obs_warn) = match &raw.observed {
        None => (
            CanonicalVideoDescription::unknown(),
            CanonicalAudioDescription {
                presence: AudioPresence::Unknown,
                embedding: AudioEmbedding::Embedded,
            },
            Some("无观测媒体 (observed=None); video/audio 全 Unknown, 绝不臆造默认格式"),
        ),
        Some(obs) => {
            let video = match &obs.video {
                Some(v) => CanonicalVideoDescription {
                    width: v.width,
                    height: v.height,
                    frame_rate: v
                        .frame_rate
                        .map(|(num, den)| CanonicalFrameRate { num, den }),
                    interlaced: v.interlaced,
                    pixel_format: v.pixel_format.clone(),
                },
                None => {
                    diagnostics.push(NormalizeDiagnostic {
                        level: DiagnosticLevel::Warn,
                        code: "video_unobserved".into(),
                        detail: "观测存在但无 video 形状".into(),
                    });
                    CanonicalVideoDescription::unknown()
                }
            };
            let presence = match obs.audio_present {
                Some(true) => AudioPresence::Present {
                    channels_hint: None,
                },
                Some(false) => AudioPresence::NotPresent,
                None => AudioPresence::Unknown,
            };
            // 0.7B-1: SDI 内嵌音频现状显式化 (契约 §4); MADI/AES/Dante 等由未来 Audio Provider 声明。
            let embedding = AudioEmbedding::Embedded;
            (
                video,
                CanonicalAudioDescription {
                    presence,
                    embedding,
                },
                None,
            )
        }
    };
    if let Some(detail) = obs_warn {
        diagnostics.push(NormalizeDiagnostic {
            level: DiagnosticLevel::Warn,
            code: "observed_missing".into(),
            detail: detail.into(),
        });
    }

    diagnostics.push(NormalizeDiagnostic {
        level: DiagnosticLevel::Info,
        code: "clock_policy_deferred".into(),
        detail: "clock 策略属 0.7B Clock 阶段; 本描述仅持有 Domain 引用占位 (纪律③)".into(),
    });

    NormalizeOutcome {
        descriptor: CanonicalMediaDescriptor {
            source: CanonicalSourceRef {
                device_id: raw.device_id,
                port_id: raw.port_id,
            },
            transport: raw.transport.clone(),
            video,
            audio,
            clock: CanonicalClockRef { domain: None },
        },
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{PortIdentity, PortOrdinal, SignalStatus, VideoContentState, VideoFormat};

    /// 构造 BMD 真机 loopback 观测形状 (720x486 / 30000÷1001 / interlaced / v210 + audio)。
    fn bmd_shape_port(device_id: Uuid) -> PortInfo {
        PortInfo {
            device_id,
            provider_binding_ref: None,
            identity: PortIdentity {
                port_id: PortIdentity::derive(
                    &device_id,
                    ConnectorType::Sdi,
                    PortOrdinal::Known(1),
                ),
                connector: ConnectorType::Sdi,
                ordinal: PortOrdinal::Known(1),
            },
            direction: crate::port::PortDirection::Input,
            capabilities: crate::port::PortCapabilities::default(),
            runtime_binding: None,
            signal: SignalStatus {
                state: crate::port::SignalState::Locked,
                video_locked: Some(true),
                audio_locked: Some(true),
                video_format: Some(VideoFormat {
                    width: 720,
                    height: 486,
                    frame_rate: Some("30000/1001".into()),
                    interlaced: Some(true),
                    pixel_format: Some("v210".into()),
                }),
                last_seen: None,
            },
            content: VideoContentState::Unknown,
        }
    }

    /// Mock 形状: 同一逻辑媒体 (相同观测), 不同装配路径 (非 BMD 来源)。
    fn mock_shape_port(device_id: Uuid) -> PortInfo {
        let mut p = bmd_shape_port(device_id);
        p.provider_binding_ref = Some("mock-binding".into());
        p
    }

    fn raw_from(port: &PortInfo) -> RawInputDescription {
        RawInputDescription::from_port(port)
    }

    // ── NORMALIZE-RT-01 ─────────────────────────────────────────────────────────

    #[test]
    fn normalize_rt_01_provider_independent_same_media_same_descriptor() {
        // 不同 Provider (BMD 形状 vs Mock 形状) 的同一逻辑媒体 → 同一 canonical 表征
        // (逐字段相等; provider 装配差异不得渗入 descriptor)。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let out_a = normalize_input(&raw_from(&bmd_shape_port(a)));
        let out_b = normalize_input(&raw_from(&mock_shape_port(b)));
        // source 反映各自身份; 媒体语义部分必须完全一致。
        assert_eq!(out_a.descriptor.video, out_b.descriptor.video);
        assert_eq!(out_a.descriptor.audio, out_b.descriptor.audio);
        assert_eq!(out_a.descriptor.clock, out_b.descriptor.clock);
        assert_eq!(out_a.descriptor.transport, out_b.descriptor.transport);
        // provider 绑定引用绝不出现在 descriptor。
        let json = serde_json::to_string(&out_a.descriptor).unwrap();
        assert!(
            !json.contains("mock-binding"),
            "provider 绑定引用不得进入 canonical 描述"
        );
    }

    #[test]
    fn normalize_rt_01_missing_observed_unknown_not_fabricated() {
        // 纪律①: observed=None → 全 Unknown + WARN, 绝不默认 1080i50/绝不回退。
        let id = Uuid::new_v4();
        let raw = RawInputDescription {
            device_id: id,
            port_id: None,
            transport: "sdi".into(),
            observed: None,
        };
        let out = normalize_input(&raw);
        assert_eq!(out.descriptor.video.width, None);
        assert_eq!(out.descriptor.video.height, None);
        assert_eq!(out.descriptor.video.frame_rate, None);
        assert_eq!(out.descriptor.video.pixel_format, None);
        assert_eq!(out.descriptor.audio.presence, AudioPresence::Unknown);
        assert!(out
            .diagnostics
            .iter()
            .any(|d| d.code == "observed_missing" && d.level == DiagnosticLevel::Warn));
    }

    #[test]
    fn normalize_rt_01_bmd_loopback_shape_matches_expected_canonical() {
        // 真机 loopback 实证形状 → 期望 canonical 形状 (Hardware 门的 Unit 级锚点)。
        let id = Uuid::new_v4();
        let out = normalize_input(&raw_from(&bmd_shape_port(id)));
        let v = &out.descriptor.video;
        assert_eq!(v.width, Some(720));
        assert_eq!(v.height, Some(486));
        assert_eq!(
            v.frame_rate,
            Some(CanonicalFrameRate {
                num: 30000,
                den: 1001
            })
        );
        assert_eq!(v.interlaced, Some(true));
        assert_eq!(v.pixel_format, Some("v210".into()));
        assert_eq!(
            out.descriptor.audio.presence,
            AudioPresence::Present {
                channels_hint: None
            }
        );
        assert_eq!(out.descriptor.audio.embedding, AudioEmbedding::Embedded);
        assert_eq!(out.descriptor.transport, "sdi");
        assert_eq!(out.descriptor.clock, CanonicalClockRef { domain: None });
    }

    #[test]
    fn normalize_input_is_pure_and_parse_failure_does_not_drop_observation() {
        // 纯函数: 同输入恒同输出; frame_rate 解析失败 → None + 其余观测保留 (不丢观测)。
        let id = Uuid::new_v4();
        let mut port = bmd_shape_port(id);
        port.signal.video_format.as_mut().unwrap().frame_rate = Some("not-a-rate".into());
        let raw = raw_from(&port);
        let out1 = normalize_input(&raw);
        let out2 = normalize_input(&raw);
        assert_eq!(out1, out2, "同输入恒同输出");
        assert_eq!(out1.descriptor.video.frame_rate, None, "解析失败 → None");
        assert_eq!(out1.descriptor.video.width, Some(720), "其余观测保留");
        assert_eq!(out1.descriptor.video.pixel_format, Some("v210".into()));
    }

    #[test]
    fn normalize_descriptor_serde_roundtrip_and_no_vendor_fields() {
        let id = Uuid::new_v4();
        let out = normalize_input(&raw_from(&bmd_shape_port(id)));
        let json = serde_json::to_string(&out.descriptor).expect("serialize");
        let back: CanonicalMediaDescriptor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(out.descriptor, back);
        for banned in ["bmd", "decklink", "gstreamer", "device_number"] {
            assert!(
                !json.to_lowercase().contains(banned),
                "vendor 字样渗入: {banned}"
            );
        }
    }

    #[test]
    fn normalize_audio_presence_semantics() {
        // 契约 §4: audio_present 三态映射 + embedding 显式化 (非 Option 附属)。
        let base = bmd_shape_port(Uuid::new_v4());
        for (audio_locked, expected) in [
            (
                Some(true),
                AudioPresence::Present {
                    channels_hint: None,
                },
            ),
            (Some(false), AudioPresence::NotPresent),
            (None, AudioPresence::Unknown),
        ] {
            let mut port = base.clone();
            port.signal.audio_locked = audio_locked;
            let out = normalize_input(&raw_from(&port));
            assert_eq!(out.descriptor.audio.presence, expected);
            assert_eq!(out.descriptor.audio.embedding, AudioEmbedding::Embedded);
        }
    }
}
