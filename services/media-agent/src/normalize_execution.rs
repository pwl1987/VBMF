//! RF-NORM-01 Phase A: explicit RAW Normalize execution intent/evidence.
//!
//! This module is deliberately separate from normalize.rs:
//! - normalize.rs describes observed input as CanonicalMediaDescriptor.
//! - this module describes an explicit Program RAW target and records execution evidence.
//!
//! No GStreamer, FFmpeg, wall-clock, or vendor runtime address belongs here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawVideoTarget {
    pub width: u32,
    pub height: u32,
    pub frame_rate_num: u32,
    pub frame_rate_den: u32,
    pub pixel_format: RawPixelFormat,
    pub interlaced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawPixelFormat {
    I420,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawAudioTarget {
    pub format: RawAudioFormat,
    pub channels: u16,
    pub sample_rate: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawAudioFormat {
    S16le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizeTarget {
    pub video: RawVideoTarget,
    pub audio: RawAudioTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizePlanError {
    MissingTarget,
    InvalidVideoRate,
    InvalidVideoDimensions,
    InvalidAudioChannels,
    InvalidAudioRate,
}

impl NormalizeTarget {
    pub fn validate(self) -> Result<Self, NormalizePlanError> {
        if self.video.width == 0 || self.video.height == 0 {
            return Err(NormalizePlanError::InvalidVideoDimensions);
        }
        if self.video.frame_rate_num == 0 || self.video.frame_rate_den == 0 {
            return Err(NormalizePlanError::InvalidVideoRate);
        }
        if self.audio.channels == 0 {
            return Err(NormalizePlanError::InvalidAudioChannels);
        }
        if self.audio.sample_rate == 0 {
            return Err(NormalizePlanError::InvalidAudioRate);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizePlan {
    pub target: NormalizeTarget,
}

impl NormalizePlan {
    pub fn require(target: Option<NormalizeTarget>) -> Result<Self, NormalizePlanError> {
        let target = target
            .ok_or(NormalizePlanError::MissingTarget)?
            .validate()?;
        Ok(Self { target })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedRawVideo {
    pub width: u32,
    pub height: u32,
    pub frame_rate_num: u32,
    pub frame_rate_den: u32,
    pub pixel_format: RawPixelFormat,
    pub interlaced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedRawAudio {
    pub format: RawAudioFormat,
    pub channels: u16,
    pub sample_rate: u32,
}

impl RawVideoTarget {
    fn matches(self, observed: ObservedRawVideo) -> bool {
        self.width == observed.width
            && self.height == observed.height
            && self.frame_rate_num == observed.frame_rate_num
            && self.frame_rate_den == observed.frame_rate_den
            && self.pixel_format == observed.pixel_format
            && self.interlaced == observed.interlaced
    }
}

impl RawAudioTarget {
    fn matches(self, observed: ObservedRawAudio) -> bool {
        self.format == observed.format
            && self.channels == observed.channels
            && self.sample_rate == observed.sample_rate
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalizeEvidenceState {
    Unobserved,
    ObservedExact,
    ObservedMismatch,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizeEvidence {
    pub target: NormalizeTarget,
    pub video: NormalizeEvidenceState,
    pub audio: NormalizeEvidenceState,
}

impl NormalizeEvidence {
    pub fn new(target: NormalizeTarget) -> Result<Self, NormalizePlanError> {
        Ok(Self {
            target: target.validate()?,
            video: NormalizeEvidenceState::Unobserved,
            audio: NormalizeEvidenceState::Unobserved,
        })
    }

    pub fn observe_video(&mut self, observed: ObservedRawVideo) {
        self.video = if self.target.video.matches(observed) {
            NormalizeEvidenceState::ObservedExact
        } else {
            NormalizeEvidenceState::ObservedMismatch
        };
    }

    pub fn observe_audio(&mut self, observed: ObservedRawAudio) {
        self.audio = if self.target.audio.matches(observed) {
            NormalizeEvidenceState::ObservedExact
        } else {
            NormalizeEvidenceState::ObservedMismatch
        };
    }

    pub fn complete(&self) -> bool {
        self.video == NormalizeEvidenceState::ObservedExact
            && self.audio == NormalizeEvidenceState::ObservedExact
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> NormalizeTarget {
        NormalizeTarget {
            video: RawVideoTarget {
                width: 1920,
                height: 1080,
                frame_rate_num: 25,
                frame_rate_den: 1,
                pixel_format: RawPixelFormat::I420,
                interlaced: true,
            },
            audio: RawAudioTarget {
                format: RawAudioFormat::S16le,
                channels: 2,
                sample_rate: 48_000,
            },
        }
    }

    fn video() -> ObservedRawVideo {
        ObservedRawVideo {
            width: 1920,
            height: 1080,
            frame_rate_num: 25,
            frame_rate_den: 1,
            pixel_format: RawPixelFormat::I420,
            interlaced: true,
        }
    }

    fn audio() -> ObservedRawAudio {
        ObservedRawAudio {
            format: RawAudioFormat::S16le,
            channels: 2,
            sample_rate: 48_000,
        }
    }
    #[test]
    fn missing_target_fails_closed_without_default() {
        assert_eq!(
            NormalizePlan::require(None),
            Err(NormalizePlanError::MissingTarget)
        );
    }

    #[test]
    fn invalid_target_fails_closed() {
        let mut t = target();
        t.video.frame_rate_den = 0;
        assert_eq!(
            NormalizePlan::require(Some(t)),
            Err(NormalizePlanError::InvalidVideoRate)
        );
    }

    #[test]
    fn evidence_needs_both_independent_planes() {
        let mut e = NormalizeEvidence::new(target()).expect("target");
        e.observe_video(video());
        assert_eq!(e.video, NormalizeEvidenceState::ObservedExact);
        assert_eq!(e.audio, NormalizeEvidenceState::Unobserved);
        assert!(!e.complete());
        e.observe_audio(audio());
        assert!(e.complete());
    }

    #[test]
    fn mismatch_is_not_treated_as_completion() {
        let mut e = NormalizeEvidence::new(target()).expect("target");
        let mut wrong = video();
        wrong.width = 1280;
        e.observe_video(wrong);
        e.observe_audio(audio());
        assert_eq!(e.video, NormalizeEvidenceState::ObservedMismatch);
        assert!(!e.complete());
    }
    #[test]
    fn evidence_serde_roundtrip_preserves_explicit_target() {
        let e = NormalizeEvidence::new(target()).expect("target");
        let json = serde_json::to_string(&e).expect("serialize");
        let back: NormalizeEvidence = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e, back);
        assert!(json.contains("I420"));
        assert!(json.contains("S16le"));
    }
}
