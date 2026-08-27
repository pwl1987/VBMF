//! 信号探测 + 黑场检测 (Signal Content Analysis).
//!
//! HARD RULE (用户 §九/§十一):
//! - `No Signal` ≠ `Signal Present + Active Video Black`. 黑场是 "信号锁定 + 内容为黑", 不是无信号.
//! - 黑场检测必须用 **亮度统计** (Y 均值 + 方差 + 极值), 绝不是 `if brightness == 0: black`.
//! - 黑场检测属于 **Signal Content Analysis**, 与 Device Discovery / Port Discovery 严格分离,
//!   不得塞进 `DeviceManager` / `PortRegistry` 构建流程.
//! - 第一阶段仅做 luminance-based 分类 (Black / Active / Unknown); 不引入 OCR / CLIP / 场景检测 (§二十七).

#![allow(dead_code)]

use crate::fixture::{ExpectedSignal, Fixture};
use crate::port::{SignalState, VideoContentState, VideoFormat};
use serde::{Deserialize, Serialize};

/// 单批帧的亮度统计 (供黑场/内容分类).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LumaStats {
    /// Y 平均值 (0–255).
    pub mean: f64,
    /// Y 标准差.
    pub std: f64,
    /// Y 最小值.
    pub min: f64,
    /// Y 最大值.
    pub max: f64,
    /// 累计样本帧数.
    pub samples: u32,
}

/// 黑场判定阈值.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackThresholds {
    /// 平均亮度上限 (SDI 黑电平约 16/255; 留余量到 24).
    pub max_mean: f64,
    /// 亮度标准差上限 (黑场应近乎恒定; 有内容则方差大).
    pub max_std: f64,
}

impl Default for BlackThresholds {
    fn default() -> Self {
        Self {
            max_mean: 24.0,
            max_std: 4.0,
        }
    }
}

/// 基于亮度统计的黑场/活动分类 (纯函数, 可测).
///
/// 综合 Y 均值 + 方差: 两者都低于阈值 → `Black`; 否则 `Active`. 样本为 0 → `Unknown`.
pub fn classify_black(stats: &LumaStats, t: &BlackThresholds) -> VideoContentState {
    if stats.samples == 0 {
        return VideoContentState::Unknown;
    }
    if stats.mean <= t.max_mean && stats.std <= t.max_std {
        VideoContentState::Black
    } else {
        VideoContentState::Active
    }
}

/// 合并多批帧的亮度统计为整体统计 (用于跨样本聚合).
pub fn aggregate_luma(acc: &mut LumaStats, batch: &LumaStats) {
    if batch.samples == 0 {
        return;
    }
    let total = acc.samples + batch.samples;
    // 加权和 → 整体均值 (近似; 不保留每像素以省内存).
    acc.mean = (acc.mean * acc.samples as f64 + batch.mean * batch.samples as f64) / total as f64;
    // 方差近似取较大者 (保守, 避免低估内容方差).
    acc.std = acc.std.max(batch.std);
    acc.min = acc.min.min(batch.min);
    acc.max = acc.max.max(batch.max);
    acc.samples = total;
}

/// 实时信号探测结果 (Runtime State, 不入 Manifest).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalProbeResult {
    pub state: SignalState,
    pub video_format: Option<VideoFormat>,
    pub content: VideoContentState,
}

/// 信号探测验收失败原因 (失败闭合).
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SignalProbeError {
    /// 非 gstreamer 构建无法探测信号 (返回 Unsupported) — 必须显式拒, 绝不假装健康.
    #[error("信号探测不可用 (当前构建无 gstreamer): state={0:?}")]
    Unavailable(SignalState),
    /// 实际信号态未满足期望 (如期望 Locked 却 NoSignal/Unknown/ProbeFailed).
    #[error("信号态不满足期望: actual={actual:?} expected={expected:?}")]
    NotSatisfied {
        actual: SignalState,
        expected: SignalState,
    },
}

/// 失败闭合的信号探测验收: 实际态必须 == 期望态; 非 gstreamer 构建返回的 `Unsupported`
/// 必须显式拒 (绝不静默通过). `probe_signal` 的产出经此门对照 `Fixture::ExpectedSignal`.
pub fn evaluate_signal_probe(
    result: &SignalProbeResult,
    expected: &ExpectedSignal,
) -> Result<(), SignalProbeError> {
    if matches!(result.state, SignalState::Unsupported) {
        return Err(SignalProbeError::Unavailable(result.state));
    }
    if result.state == expected.state {
        Ok(())
    } else {
        Err(SignalProbeError::NotSatisfied {
            actual: result.state,
            expected: expected.state,
        })
    }
}

/// 信号内容分类验收失败原因 (失败闭合).
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SignalContentError {
    /// 信号态不足以信托内容 (NoSignal/Unknown/Unsupported/ProbeFailed) — 必须拒, 绝不假设内容.
    #[error("内容不可信 (信号态={0:?}): 不得据此断言内容满足期望")]
    Insufficient(SignalState),
    /// 实际内容未满足期望 (如期望 Active 却 Black/Frozen/Unknown).
    #[error("内容不满足期望: actual={actual:?} expected={expected:?}")]
    NotSatisfied {
        actual: VideoContentState,
        expected: VideoContentState,
    },
}

/// 失败闭合的内容分类验收: 内容仅在信号可信 (`Locked`) 时方有意义.
/// 当期望具体内容 (Black/Active/Frozen/TestPattern) 时, 实际内容须 == 期望, 否则拒;
/// 信号态不可信 (NoSignal/Unknown/Unsupported/ProbeFailed) 时绝不假设内容满足期望.
/// 与 `evaluate_signal_probe`(信号态维度) 互为 STEP 7/8 双门, 共同构成失败闭合验收.
pub fn evaluate_signal_content(
    result: &SignalProbeResult,
    expected: VideoContentState,
) -> Result<(), SignalContentError> {
    match result.state {
        SignalState::Locked => {
            if result.content == expected {
                Ok(())
            } else {
                Err(SignalContentError::NotSatisfied {
                    actual: result.content,
                    expected,
                })
            }
        }
        other => Err(SignalContentError::Insufficient(other)),
    }
}

/// Fixture 信号验收失败原因 (信号态门 + 内容门 双门汇总, 失败闭合).
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FixtureSignalError {
    /// 信号态门失败 (STEP 7 验收).
    #[error("信号态验收失败: {0}")]
    State(#[from] SignalProbeError),
    /// 内容门失败 (STEP 8 验收).
    #[error("内容验收失败: {0}")]
    Content(#[from] SignalContentError),
}

/// 失败闭合的 Fixture 信号验收: 把 STEP 7 信号态门与 STEP 8 内容门接入 `Fixture::ExpectedSignal`,
/// 作为 loopback / 信号维度验收的汇总结算. 信号态门始终生效; 内容门仅当 `ExpectedSignal.content`
/// 为 `Some` 时强制, `None` 跳过. 任一门拒即整体拒 (绝不部分通过假装健康).
pub fn evaluate_fixture_signal(
    fixture: &Fixture,
    result: &SignalProbeResult,
) -> Result<(), FixtureSignalError> {
    evaluate_signal_probe(result, &fixture.expected)?;
    if let Some(expected_content) = fixture.expected.content {
        evaluate_signal_content(result, expected_content)?;
    }
    Ok(())
}

/// 探测某 GStreamer device-number 的当前信号状态 + 内容 (gstreamer 构建).
///
/// 先读 `signal` 属性与协商 caps; 若信号锁定, 拉取少量 I420 样本做亮度统计并分类黑场/活动.
/// 非 gstreamer 构建返回 `Unsupported`.
#[cfg(feature = "gstreamer")]
pub fn probe_signal(device_number: u32, sample_frames: u32) -> SignalProbeResult {
    use gstreamer::prelude::*;

    let desc = format!(
        "decklinkvideosrc device-number={device} ! videoconvert ! video/x-raw,format=I420 ! appsink name=probe max-buffers=8 drop=false",
        device = device_number,
    );
    let pipeline = match gstreamer::parse::launch(&desc) {
        Ok(p) => p,
        Err(_) => {
            return SignalProbeResult {
                state: SignalState::ProbeFailed,
                video_format: None,
                content: VideoContentState::Unknown,
            }
        }
    };
    let pipeline = match pipeline.downcast::<gstreamer::Pipeline>() {
        Ok(p) => p,
        Err(_) => {
            return SignalProbeResult {
                state: SignalState::ProbeFailed,
                video_format: None,
                content: VideoContentState::Unknown,
            }
        }
    };
    if pipeline.set_state(gstreamer::State::Playing).is_err() {
        let _ = pipeline.set_state(gstreamer::State::Null);
        return SignalProbeResult {
            state: SignalState::ProbeFailed,
            video_format: None,
            content: VideoContentState::Unknown,
        };
    }
    std::thread::sleep(std::time::Duration::from_millis(800));

    let src = pipeline.by_name("decklinkvideosrc0");
    let signal = src.as_ref().and_then(|el| {
        el.find_property("signal")
            .map(|_| el.property::<bool>("signal"))
    });
    let caps = src
        .as_ref()
        .and_then(|el| el.static_pad("src"))
        .and_then(|pad| pad.current_caps())
        .filter(|c| !c.is_empty())
        .and_then(|c| parse_caps(&c.to_string()));

    let state = match signal {
        Some(true) => SignalState::Locked,
        Some(false) => SignalState::NoSignal,
        None => SignalState::Unknown,
    };

    let content = if state == SignalState::Locked {
        match pull_luma(&pipeline, sample_frames) {
            Some(stats) => {
                let t = BlackThresholds::default();
                classify_black(&stats, &t)
            }
            None => VideoContentState::Unknown,
        }
    } else {
        VideoContentState::NoSignal
    };

    let _ = pipeline.set_state(gstreamer::State::Null);
    SignalProbeResult {
        state,
        video_format: caps,
        content,
    }
}

#[cfg(feature = "gstreamer")]
fn parse_caps(text: &str) -> Option<VideoFormat> {
    let width = parse_int(text, "width=(int)")?;
    let height = parse_int(text, "height=(int)")?;
    let frame_rate = text
        .split("framerate=(fraction)")
        .nth(1)
        .and_then(|s| s.split([',', ')', ' ']).next())
        .map(|s| s.to_string());
    let pixel_format = text
        .split("format=(string)")
        .nth(1)
        .and_then(|s| s.split([',', ')', ' ']).next())
        .map(|s| s.to_string());
    let interlaced = text.split("interlace-mode=(string)").nth(1).map(|s| {
        let v = s.split([',', ')', ' ']).next().unwrap_or("");
        v == "interleaved" || v == "mixed"
    });
    Some(VideoFormat {
        width,
        height,
        frame_rate,
        interlaced,
        pixel_format,
    })
}

#[cfg(feature = "gstreamer")]
fn parse_int(text: &str, key: &str) -> Option<u32> {
    text.split(key)
        .nth(1)
        .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|s| s.parse::<u32>().ok())
}

/// 拉取少量 I420 样本, 仅取 Y 平面做亮度统计.
#[cfg(feature = "gstreamer")]
fn pull_luma(pipeline: &gstreamer::Pipeline, sample_frames: u32) -> Option<LumaStats> {
    use gstreamer::prelude::*;
    let sink = pipeline.by_name("probe")?;
    let sink = sink.downcast::<gstreamer_app::AppSink>().ok()?;
    sink.set_property("timeout", gstreamer::ClockTime::from_seconds(1));

    let mut acc = LumaStats::default();
    for _ in 0..sample_frames.max(1) {
        match sink.pull_sample() {
            Ok(sample) => {
                let buf = sample.buffer()?;
                if let Ok(map) = buf.map_readable() {
                    let data = map.as_slice();
                    // I420: Y 平面在前 width*height 字节.
                    // 尺寸从 caps 获取; 这里用 buffer 大小近似 (Y 平面 = 总字节 * 2/3 因 4:2:0).
                    let y_len = (data.len() * 2 / 3).max(1);
                    let (mean, std, min, max) = luma_stats(&data[..y_len]);
                    let batch = LumaStats {
                        mean,
                        std,
                        min,
                        max,
                        samples: 1,
                    };
                    aggregate_luma(&mut acc, &batch);
                }
            }
            Err(_) => break,
        }
    }
    if acc.samples == 0 {
        None
    } else {
        Some(acc)
    }
}

#[cfg(feature = "gstreamer")]
fn luma_stats(y: &[u8]) -> (f64, f64, f64, f64) {
    let n = y.len() as f64;
    let mut sum: u64 = 0;
    let mut min = u8::MAX;
    let mut max = 0u8;
    for &b in y {
        sum += b as u64;
        if b < min {
            min = b;
        }
        if b > max {
            max = b;
        }
    }
    let mean = sum as f64 / n;
    let mut var = 0f64;
    for &b in y {
        let d = b as f64 - mean;
        var += d * d;
    }
    var /= n;
    (mean, var.sqrt(), min as f64, max as f64)
}

/// 非 gstreamer 构建: 信号探测不可用.
#[cfg(not(feature = "gstreamer"))]
pub fn probe_signal(_device_number: u32, _sample_frames: u32) -> SignalProbeResult {
    SignalProbeResult {
        state: SignalState::Unsupported,
        video_format: None,
        content: VideoContentState::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_when_low_mean_and_low_variance() {
        let t = BlackThresholds::default();
        // SDI 黑电平约 16, 近乎恒定 → Black.
        let stats = LumaStats {
            mean: 16.0,
            std: 1.0,
            min: 15.0,
            max: 18.0,
            samples: 8,
        };
        assert_eq!(classify_black(&stats, &t), VideoContentState::Black);
    }

    #[test]
    fn active_when_high_mean() {
        let t = BlackThresholds::default();
        let stats = LumaStats {
            mean: 128.0,
            std: 40.0,
            min: 0.0,
            max: 255.0,
            samples: 8,
        };
        assert_eq!(classify_black(&stats, &t), VideoContentState::Active);
    }

    #[test]
    fn active_when_high_variance_even_low_mean() {
        // HARD RULE: 不能用 "brightness==0 => black". 低均值但高方差 = 有内容 (非纯黑).
        let t = BlackThresholds::default();
        let stats = LumaStats {
            mean: 18.0,
            std: 30.0,
            min: 0.0,
            max: 60.0,
            samples: 8,
        };
        assert_eq!(classify_black(&stats, &t), VideoContentState::Active);
    }

    #[test]
    fn unknown_with_no_samples() {
        let t = BlackThresholds::default();
        let stats = LumaStats::default();
        assert_eq!(classify_black(&stats, &t), VideoContentState::Unknown);
    }

    #[test]
    fn aggregate_combines_stats() {
        let mut acc = LumaStats {
            mean: 16.0,
            std: 1.0,
            min: 15.0,
            max: 18.0,
            samples: 4,
        };
        let batch = LumaStats {
            mean: 17.0,
            std: 1.5,
            min: 16.0,
            max: 19.0,
            samples: 4,
        };
        aggregate_luma(&mut acc, &batch);
        assert_eq!(acc.samples, 8);
        // 均值应为 (16*4 + 17*4)/8 = 16.5
        assert!((acc.mean - 16.5).abs() < 1e-6);
        // 方差取较大者 = 1.5
        assert!((acc.std - 1.5).abs() < 1e-9);
    }

    #[test]
    #[cfg(not(feature = "gstreamer"))]
    fn probe_signal_unsupported_without_gstreamer() {
        // default/simulation/bmd 构建返回 Unsupported (不 panic, 不触碰真实 GStreamer).
        let r = probe_signal(0, 1);
        assert_eq!(r.state, SignalState::Unsupported);
    }

    #[test]
    fn evaluate_accepts_locked_when_expected_locked() {
        // 回归: 实际 Locked == 期望 Locked ⇒ 通过.
        let r = SignalProbeResult {
            state: SignalState::Locked,
            video_format: None,
            content: VideoContentState::Unknown,
        };
        let exp = ExpectedSignal {
            state: SignalState::Locked,
            format: None,
            content: None,
        };
        assert!(evaluate_signal_probe(&r, &exp).is_ok());
    }

    #[test]
    fn evaluate_rejects_nosignal_when_expected_locked() {
        // TDD(RED→GREEN): 期望 Locked 但实际 NoSignal ⇒ 失败闭合 (绝不假装健康).
        let r = SignalProbeResult {
            state: SignalState::NoSignal,
            video_format: None,
            content: VideoContentState::NoSignal,
        };
        let exp = ExpectedSignal {
            state: SignalState::Locked,
            format: None,
            content: None,
        };
        assert!(matches!(
            evaluate_signal_probe(&r, &exp),
            Err(SignalProbeError::NotSatisfied { .. })
        ));
    }

    #[test]
    fn evaluate_rejects_unsupported_fails_closed() {
        // TDD(RED→GREEN): 非 gstreamer 构建返回 Unsupported ⇒ 显式拒, 绝不通过 (无假健康).
        let r = SignalProbeResult {
            state: SignalState::Unsupported,
            video_format: None,
            content: VideoContentState::Unknown,
        };
        let exp = ExpectedSignal {
            state: SignalState::Locked,
            format: None,
            content: None,
        };
        assert!(matches!(
            evaluate_signal_probe(&r, &exp),
            Err(SignalProbeError::Unavailable(_))
        ));
    }

    #[test]
    fn evaluate_rejects_probe_failed() {
        // TDD(RED→GREEN): 探测失败 (decklinkvideosrc 打不开等) ⇒ 失败闭合.
        let r = SignalProbeResult {
            state: SignalState::ProbeFailed,
            video_format: None,
            content: VideoContentState::Unknown,
        };
        let exp = ExpectedSignal {
            state: SignalState::Locked,
            format: None,
            content: None,
        };
        assert!(matches!(
            evaluate_signal_probe(&r, &exp),
            Err(SignalProbeError::NotSatisfied { .. })
        ));
    }

    #[test]
    fn evaluate_content_accepts_active_when_expected_active() {
        // 回归: Locked + 实测 Active == 期望 Active ⇒ 通过.
        let r = SignalProbeResult {
            state: SignalState::Locked,
            video_format: None,
            content: VideoContentState::Active,
        };
        assert!(evaluate_signal_content(&r, VideoContentState::Active).is_ok());
    }

    #[test]
    fn evaluate_content_accepts_black_when_expected_black() {
        // 回归: Locked + 实测 Black == 期望 Black ⇒ 通过.
        let r = SignalProbeResult {
            state: SignalState::Locked,
            video_format: None,
            content: VideoContentState::Black,
        };
        assert!(evaluate_signal_content(&r, VideoContentState::Black).is_ok());
    }

    #[test]
    fn evaluate_content_rejects_active_when_actual_black() {
        // TDD(RED→GREEN): 期望 Active 但实测 Black ⇒ 失败闭合.
        let r = SignalProbeResult {
            state: SignalState::Locked,
            video_format: None,
            content: VideoContentState::Black,
        };
        assert!(matches!(
            evaluate_signal_content(&r, VideoContentState::Active),
            Err(SignalContentError::NotSatisfied { .. })
        ));
    }

    #[test]
    fn evaluate_content_rejects_nosignal_state_fails_closed() {
        // 失败闭合: 无信号时内容不可信 ⇒ 拒 (绝不假设内容满足期望).
        let r = SignalProbeResult {
            state: SignalState::NoSignal,
            video_format: None,
            content: VideoContentState::NoSignal,
        };
        assert!(matches!(
            evaluate_signal_content(&r, VideoContentState::Active),
            Err(SignalContentError::Insufficient(_))
        ));
    }

    #[test]
    fn evaluate_content_rejects_unknown_state_fails_closed() {
        // 失败闭合: 信号态未知时内容不可信 ⇒ 拒.
        let r = SignalProbeResult {
            state: SignalState::Unknown,
            video_format: None,
            content: VideoContentState::Unknown,
        };
        assert!(matches!(
            evaluate_signal_content(&r, VideoContentState::Black),
            Err(SignalContentError::Insufficient(_))
        ));
    }

    #[test]
    fn evaluate_content_rejects_unsupported_fails_closed() {
        // 失败闭合: 非 gstreamer 构建返回 Unsupported ⇒ 内容不可信 ⇒ 拒.
        let r = SignalProbeResult {
            state: SignalState::Unsupported,
            video_format: None,
            content: VideoContentState::Unknown,
        };
        assert!(matches!(
            evaluate_signal_content(&r, VideoContentState::Active),
            Err(SignalContentError::Insufficient(_))
        ));
    }

    use crate::fixture::default_sdi_loopback;

    #[test]
    fn fixture_signal_accepts_locked_active() {
        // 回归: 期望 Locked+Active, 实测 Locked+Active ⇒ 信号态门+内容门双通过.
        let f = default_sdi_loopback(); // expected.state=Locked, content=Some(Active)
        let r = SignalProbeResult {
            state: SignalState::Locked,
            video_format: None,
            content: VideoContentState::Active,
        };
        assert!(evaluate_fixture_signal(&f, &r).is_ok());
    }

    #[test]
    fn fixture_signal_rejects_state_nosignal() {
        // TDD(RED→GREEN): 期望 Locked 但实测 NoSignal ⇒ 信号态门失败闭合.
        let f = default_sdi_loopback();
        let r = SignalProbeResult {
            state: SignalState::NoSignal,
            video_format: None,
            content: VideoContentState::NoSignal,
        };
        assert!(matches!(
            evaluate_fixture_signal(&f, &r),
            Err(FixtureSignalError::State(_))
        ));
    }

    #[test]
    fn fixture_signal_rejects_content_black_when_expected_active() {
        // TDD(RED→GREEN): 期望 Active 但实测 Black ⇒ 内容门失败闭合.
        let f = default_sdi_loopback();
        let r = SignalProbeResult {
            state: SignalState::Locked,
            video_format: None,
            content: VideoContentState::Black,
        };
        assert!(matches!(
            evaluate_fixture_signal(&f, &r),
            Err(FixtureSignalError::Content(_))
        ));
    }

    #[test]
    fn fixture_signal_skips_content_when_none() {
        // 期望 content=None 时跳过内容门 (仅信号态门生效); 实测 Black 仍通过.
        let mut f = default_sdi_loopback();
        f.expected.content = None;
        let r = SignalProbeResult {
            state: SignalState::Locked,
            video_format: None,
            content: VideoContentState::Black,
        };
        assert!(evaluate_fixture_signal(&f, &r).is_ok());
    }

    #[test]
    fn fixture_signal_rejects_unsupported_fails_closed() {
        // 失败闭合: 非 gstreamer 构建返回 Unsupported ⇒ 信号态门显式拒.
        let f = default_sdi_loopback();
        let r = SignalProbeResult {
            state: SignalState::Unsupported,
            video_format: None,
            content: VideoContentState::Unknown,
        };
        assert!(matches!(
            evaluate_fixture_signal(&f, &r),
            Err(FixtureSignalError::State(_))
        ));
    }
}
