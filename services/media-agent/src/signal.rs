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
#[derive(Debug, thiserror::Error, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, thiserror::Error, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, thiserror::Error, PartialEq, Serialize, Deserialize)]
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

/// 单条 Fixture 的 loopback 验收结果 (失败闭合汇总, 供诊断/验收 JSON 输出).
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FixtureVerification {
    /// Fixture 标识.
    pub fixture_id: String,
    /// 是否通过 (信号态门 + 内容门 双通过).
    pub passed: bool,
    /// 实际信号态 (便于诊断输出).
    pub state: Option<SignalState>,
    /// 实际内容态 (便于诊断输出).
    pub content: Option<VideoContentState>,
    /// 是否采到 SMPTE 彩条特征 (`Some(true)`=检测到, 证明来源为本卡 render 的已知图案).
    pub test_pattern: Option<bool>,
    /// 加嵌音频存在性 (`Some(true)`=探测到音频缓冲, `Some(false)`=已探测无音频, `None`=无法探测).
    pub audio: Option<bool>,
    /// 采集格式与期望格式是否一致 (仅当 `ExpectedSignal.format` 与实测 `video_format` 均存在时判定;
    /// `Some(false)`=不一致, 纳入 `passed` 失败闭合; `None`=信息不足跳过).
    pub format_match: Option<bool>,
    /// 失败原因 (通过时为 None).
    pub error: Option<FixtureSignalError>,
}

/// 单条 Fixture 的探测产出 (信号态 + 加嵌音频存在性), 供 loopback 验收汇总.
///
/// 与 `SignalProbeResult` 分离, 以便在不改动已充分测试的信号态/内容分类类型的前提下扩展 loopback
/// 验收字段 (加嵌音频), 保持 sim/default 构建的单测不受影响.
#[derive(Debug, Clone)]
pub struct FixtureProbe {
    /// 信号探测结果 (信号态 + 内容分类).
    pub signal: SignalProbeResult,
    /// 是否采到 SMPTE 彩条特征 (`Some(true)`=检测到, 证明来源为本卡 render 的已知图案).
    pub test_pattern: Option<bool>,
    /// 加嵌音频存在性: `Some(true)`=探测到音频缓冲, `Some(false)`=已探测但无音频,
    /// `None`=无法探测 (非 gstreamer 构建 / 音频分支不可用).
    pub audio_present: Option<bool>,
}

/// loopback 验收主路径: 对每个 `Fixture`, 由 `probe` 注入取得其 source 端口的当前信号探测结果
/// (含加嵌音频存在性), 再走 `evaluate_fixture_signal` 对照 `ExpectedSignal` (信号态门 + 内容门,
/// 失败闭合), 汇总为 `FixtureVerification`.
///
/// `probe` 由调用方注入: gstreamer 构建注入真实 `probe_fixture_signal` (按 `fixture` 解析 gst
/// device-number + 在 source 渲染已知图案); 测试/诊断注入 stub. 验收编排与采集实现解耦, 故可在无硬件/
/// 无 gstreamer 的 sim/default 构建下完整测试.
pub fn verify_fixtures<F>(fixtures: &[Fixture], mut probe: F) -> Vec<FixtureVerification>
where
    F: FnMut(&Fixture) -> FixtureProbe,
{
    let mut out = Vec::with_capacity(fixtures.len());
    for f in fixtures {
        let probe_out = probe(f);
        let result = &probe_out.signal;
        let (mut passed, error) = match evaluate_fixture_signal(f, result) {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e)),
        };
        // 格式硬门: 期望格式与实测格式均存在时比对; 不一致 → 失败闭合 (并入 passed).
        // 仅报告/告警语义保留: 任一侧缺失则不判 (None), 不阻断.
        let format_match = match (&f.expected.format, &result.video_format) {
            (Some(exp), Some(vf)) => Some(vf.matches(exp)),
            _ => None,
        };
        if let Some(false) = format_match {
            passed = false;
        }
        out.push(FixtureVerification {
            fixture_id: f.fixture_id.clone(),
            passed,
            state: Some(result.state),
            content: Some(result.content),
            test_pattern: probe_out.test_pattern,
            audio: probe_out.audio_present,
            format_match,
            error,
        });
    }
    out
}

/// 探测某 GStreamer device-number 的当前信号状态 + 内容 (gstreamer 构建).
///
/// 在单条 GStreamer 管线上同时采集视频(`decklinkvideosrc`)与加嵌音频(`decklinkaudiosrc`),
/// 避免对同一 DeckLink 设备号开两条独立管线造成争用 (canonical 采集路径即视频+音频同管线).
///
/// 返回 `(信号探测结果, 加嵌音频存在性)`: 信号态/内容/格式来自视频分支; 音频存在性由音频分支
/// `appsink` 限时拉取判定 (`Some(true)`=有缓冲, `Some(false)`=已探测无音频, `None`=管线启动失败无法探测).
#[cfg(feature = "gstreamer-backend")]
pub fn probe_combined(device_number: u32, sample_frames: u32) -> (SignalProbeResult, Option<bool>) {
    use gstreamer::prelude::*;
    // 强制 mode=1080i50 与对端 render 一致, 使采集格式确定 (验收硬门比对依据).
    let desc = format!(
        "decklinkvideosrc name=decklinkvideosrc0 device-number={dev} mode=1080i50 ! videoconvert ! video/x-raw,format=I420 ! appsink name=probe max-buffers=8 drop=false \
         decklinkaudiosrc name=decklinkaudiosrc0 device-number={dev} ! audioconvert ! appsink name=aprobe max-buffers=4 drop=false",
        dev = device_number,
    );
    let pipeline = match gstreamer::parse::launch(&desc) {
        Ok(p) => p,
        Err(_) => {
            return (
                SignalProbeResult {
                    state: SignalState::ProbeFailed,
                    video_format: None,
                    content: VideoContentState::Unknown,
                },
                None,
            );
        }
    };
    let pipeline = match pipeline.downcast::<gstreamer::Pipeline>() {
        Ok(p) => p,
        Err(_) => {
            return (
                SignalProbeResult {
                    state: SignalState::ProbeFailed,
                    video_format: None,
                    content: VideoContentState::Unknown,
                },
                None,
            );
        }
    };
    if pipeline.set_state(gstreamer::State::Playing).is_err() {
        let _ = pipeline.set_state(gstreamer::State::Null);
        return (
            SignalProbeResult {
                state: SignalState::ProbeFailed,
                video_format: None,
                content: VideoContentState::Unknown,
            },
            None,
        );
    }
    // 视频锁定 + 音频填充均需少许时间 (render 侧 SMPTE + 1kHz 启动亦需时间).
    std::thread::sleep(std::time::Duration::from_millis(1500));

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

    // 内容分类: 先 Black/Active, 命中 Active 再细分为 TestPattern (SMPTE 彩条特征),
    // 以证明采到的是 render 渲染的已知图案, 而非外部噪声/黑场.
    let content = if state == SignalState::Locked {
        match pull_luma(&pipeline, sample_frames) {
            Some(stats) => {
                let base = classify_black(&stats, &BlackThresholds::default());
                if base == VideoContentState::Active {
                    if let Some(fmt) = &caps {
                        let (w, h) = (fmt.width, fmt.height);
                        if w > 0 && h > 0 {
                            if let Some(y) = sample_frame_y(&pipeline, w, h) {
                                if detect_test_pattern(&y, w, h) {
                                    VideoContentState::TestPattern
                                } else {
                                    base
                                }
                            } else {
                                base
                            }
                        } else {
                            base
                        }
                    } else {
                        base
                    }
                } else {
                    base
                }
            }
            None => VideoContentState::Unknown,
        }
    } else {
        VideoContentState::NoSignal
    };

    // 音频: 用 RMS 能量区分"真实 1kHz 音"与静音, 而非仅判"有缓冲".
    let audio_present = audio_rms(&pipeline).map(|rms| rms > 50.0);

    let _ = pipeline.set_state(gstreamer::State::Null);
    (
        SignalProbeResult {
            state,
            video_format: caps,
            content,
        },
        audio_present,
    )
}

#[cfg(feature = "gstreamer-backend")]
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

#[cfg(feature = "gstreamer-backend")]
fn parse_int(text: &str, key: &str) -> Option<u32> {
    text.split(key)
        .nth(1)
        .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|s| s.parse::<u32>().ok())
}

/// 拉取少量 I420 样本, 仅取 Y 平面做亮度统计.
#[cfg(feature = "gstreamer-backend")]
fn pull_luma(pipeline: &gstreamer::Pipeline, sample_frames: u32) -> Option<LumaStats> {
    use gstreamer::prelude::*;
    let sink = pipeline.by_name("probe")?;
    let sink = sink.downcast::<gstreamer_app::AppSink>().ok()?;

    let mut acc = LumaStats::default();
    for _ in 0..sample_frames.max(1) {
        match sink.try_pull_sample(gstreamer::ClockTime::from_seconds(1)) {
            Some(sample) => {
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
            None => break,
        }
    }
    if acc.samples == 0 {
        None
    } else {
        Some(acc)
    }
}

/// 从视频 appsink 拉取单帧, 返回 I420 的 Y 平面 (前 width*height 字节). 用于内容细分类 (SMPTE 彩条检测).
#[cfg(feature = "gstreamer-backend")]
fn sample_frame_y(pipeline: &gstreamer::Pipeline, width: u32, height: u32) -> Option<Vec<u8>> {
    use gstreamer::prelude::*;
    let sink = pipeline
        .by_name("probe")?
        .downcast::<gstreamer_app::AppSink>()
        .ok()?;
    let sample = sink.try_pull_sample(gstreamer::ClockTime::from_seconds(1))?;
    let buf = sample.buffer()?;
    let map = buf.map_readable().ok()?;
    let data = map.as_slice();
    let need = (width as usize) * (height as usize);
    if data.len() < need {
        return None;
    }
    Some(data[..need].to_vec())
}

/// SMPTE 彩条特征检测: 在顶带采样一行, 统计量化后的亮度等级数与水平跳变数.
/// SMPTE 顶 2/3 为 7 条竖直彩条 → 约 7 个 distinct 等级 + 6 次跳变; 据此判定"采到的是我们渲染的彩条".
/// 阈值留余量 (>=5 等级, >=6 跳变), 容忍 SDI 往返的轻微亮度漂移.
#[cfg(feature = "gstreamer-backend")]
fn detect_test_pattern(y: &[u8], width: u32, height: u32) -> bool {
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 {
        return false;
    }
    // 取顶带 (SMPTE 彩条位于画面顶部约 2/3 处) 中部一行.
    let row = (h / 3).min(h - 1);
    let start = row * w;
    if start + w > y.len() {
        return false;
    }
    let mut levels: std::collections::HashSet<u8> = std::collections::HashSet::new();
    let mut prev: Option<u8> = None;
    let mut transitions: u32 = 0;
    for col in 0..w {
        let v = y[start + col] / 16; // 量化到 16 级桶, 容忍漂移
        if let Some(p) = prev {
            if p != v {
                transitions += 1;
            }
        }
        prev = Some(v);
        levels.insert(v);
    }
    levels.len() >= 5 && transitions >= 6
}

/// 从音频 appsink 拉取单样本, 计算 RMS 能量 (S16LE). 用于区分"真实 1kHz 音"与静音.
/// 返回 None 表示无音频分支/无样本.
#[cfg(feature = "gstreamer-backend")]
fn audio_rms(pipeline: &gstreamer::Pipeline) -> Option<f64> {
    use gstreamer::prelude::*;
    let sink = pipeline
        .by_name("aprobe")?
        .downcast::<gstreamer_app::AppSink>()
        .ok()?;
    let sample = sink.try_pull_sample(gstreamer::ClockTime::from_seconds(1))?;
    let buf = sample.buffer()?;
    let map = buf.map_readable().ok()?;
    let data = map.as_slice();
    if data.len() < 2 {
        return None;
    }
    let mut sum_sq: f64 = 0.0;
    let mut n: u64 = 0;
    for i in (0..data.len() - 1).step_by(2) {
        let s = i16::from_le_bytes([data[i], data[i + 1]]) as f64;
        sum_sq += s * s;
        n += 1;
    }
    if n == 0 {
        return None;
    }
    Some((sum_sq / n as f64).sqrt())
}

#[cfg(feature = "gstreamer-backend")]
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
#[cfg(not(feature = "gstreamer-backend"))]
pub fn probe_signal(_device_number: u32, _sample_frames: u32) -> SignalProbeResult {
    SignalProbeResult {
        state: SignalState::Unsupported,
        video_format: None,
        content: VideoContentState::Unknown,
    }
}

/// 按 Fixture 解析出的 **sink(输入采集端口)** gst device-number 真实探测信号态 + 内容,
/// 并先在 **source(输出端口)** 渲染已知测试图案, 使 loopback 受控 (信号来源确为本卡输出, 非外部源),
/// 同时探测加嵌音频存在性. 汇总为 `FixtureProbe` 供 `verify_fixtures` 走双门验收.
///
/// 仅 gstreamer 构建可用; 非 gstreamer 构建由 `probe_signal`(Unsupported) 兜底, 本函数不存在.
/// 失败闭合: 若 fixture 无法解析到 sink gst device-number, 返回 `ProbeFailed` + `audio_present=None`,
/// 绝不开 device 0 盲采.
#[cfg(feature = "gstreamer-backend")]
pub fn probe_fixture_signal(
    fixture: &Fixture,
    registry: &crate::port::PortRegistry,
    sample_frames: u32,
) -> FixtureProbe {
    use gstreamer::prelude::*;
    use uuid::Uuid;
    // 解析 sink(输入采集端口) 的 gst device-number:
    //  - 显式声明 port_id → 查该端口 runtime_binding;
    //  - 模板 fixture (port_id 为 None, 如本机 v1 manifest 端口 identity.port_id 为 None) →
    //    回退到第一个带 runtime_binding 的输入端口 (host-specific, 确定性).
    // 直接读 runtime_binding.gst_device_number, 绕开 resolve() 的 port_id 回退 (v1 manifest
    // 端口无 port_id, 走 port_id 必落空 → ProbeFailed).
    let sink_dev = fixture
        .sink
        .port_id
        .as_ref()
        .and_then(|pid| Uuid::parse_str(pid).ok())
        .and_then(|u| registry.get(&u))
        .and_then(|p| p.runtime_binding.as_ref())
        .map(|b| b.gst_device_number)
        .or_else(|| {
            registry
                .input_ports()
                .iter()
                .find_map(|p| p.runtime_binding.as_ref().map(|b| b.gst_device_number))
        });
    let sink_dev = match sink_dev {
        Some(n) => n,
        None => {
            return FixtureProbe {
                signal: SignalProbeResult {
                    state: SignalState::ProbeFailed,
                    video_format: None,
                    content: VideoContentState::Unknown,
                },
                test_pattern: None,
                audio_present: None,
            }
        }
    };
    // source(输出渲染端口) gst device-number: 同上, 用于渲染已知图案驱动 loopback (确认信号来源为本卡输出).
    let src_dev = fixture
        .source
        .port_id
        .as_ref()
        .and_then(|pid| Uuid::parse_str(pid).ok())
        .and_then(|u| registry.get(&u))
        .and_then(|p| p.runtime_binding.as_ref())
        .map(|b| b.gst_device_number)
        .or_else(|| {
            registry
                .output_ports()
                .iter()
                .find_map(|p| p.runtime_binding.as_ref().map(|b| b.gst_device_number))
        });
    // 在 source 渲染已知图案驱动 loopback (失败则退化为直接探测 sink, 仍输出可诊断结果).
    let render = src_dev.and_then(start_loopback_render);
    std::thread::sleep(std::time::Duration::from_millis(500));
    let (signal, audio_present) = probe_combined(sink_dev, sample_frames);
    if let Some(p) = render {
        let _ = p.set_state(gstreamer::State::Null);
    }
    let test_pattern = Some(matches!(signal.content, VideoContentState::TestPattern));
    FixtureProbe {
        signal,
        test_pattern,
        audio_present,
    }
}

/// 在输出卡 (gst device-number) 渲染已知测试图案 (SMPTE 彩条) + 1kHz 音频, 驱动 loopback 闭环.
///
/// 返回运行中的 Pipeline 句柄 (调用方 probe 完成后置 Null 停止); 启动失败 (元素缺失/设备占用) 返回 None,
/// 由 `probe_fixture_signal` 退化为直接探测 sink. 仅 gstreamer 构建可用.
#[cfg(feature = "gstreamer-backend")]
fn start_loopback_render(device_number: u32) -> Option<gstreamer::Pipeline> {
    use gstreamer::prelude::*;
    // 关键: 强制 mode=1080i50 + 喂入匹配 caps, 否则 decklinkvideosink 默认回退到 SD(720x486),
    //       导致对端 Duo 采集到的是 SD 而非验收所需的 1080i50.
    // 音频显式 2 通道 / 48k (SDI 内嵌音频标准), 与 decklinkaudiosink 期望一致.
    let desc = format!(
        "videotestsrc pattern=smpte is-live=true ! video/x-raw,format=I420,width=1920,height=1080,framerate=25/1,interlace-mode=interleaved ! videoconvert ! decklinkvideosink device-number={device} mode=1080i50 audiotestsrc is-live=true volume=0.2 ! audioconvert ! audio/x-raw,format=S16LE,channels=2,rate=48000 ! decklinkaudiosink device-number={device}",
        device = device_number,
    );
    let pipeline = gstreamer::parse::launch(&desc).ok()?;
    let pipeline = pipeline.downcast::<gstreamer::Pipeline>().ok()?;
    if pipeline.set_state(gstreamer::State::Playing).is_err() {
        let _ = pipeline.set_state(gstreamer::State::Null);
        return None;
    }
    Some(pipeline)
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
    #[cfg(not(feature = "gstreamer-backend"))]
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

    // ---- STEP 10: loopback 验收主路径 (verify_fixtures) ----
    #[test]
    fn verify_fixtures_accepts_locked_active() {
        // 回归: 期望 Locked+Active, 注入 probe 返回 Locked+Active ⇒ 单条 fixture 通过.
        let fixtures = vec![default_sdi_loopback()];
        let results = verify_fixtures(&fixtures, |_f| FixtureProbe {
            signal: SignalProbeResult {
                state: SignalState::Locked,
                video_format: None,
                content: VideoContentState::Active,
            },
            test_pattern: None,
            audio_present: None,
        });
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
        assert!(results[0].error.is_none());
    }

    #[test]
    fn verify_fixtures_rejects_content_black() {
        // TDD(RED→GREEN): 注入 probe 返回 Black ⇒ 内容门失败闭合, passed=false, error=Content.
        let fixtures = vec![default_sdi_loopback()];
        let results = verify_fixtures(&fixtures, |_f| FixtureProbe {
            signal: SignalProbeResult {
                state: SignalState::Locked,
                video_format: None,
                content: VideoContentState::Black,
            },
            test_pattern: None,
            audio_present: None,
        });
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
        assert!(matches!(
            results[0].error,
            Some(FixtureSignalError::Content(_))
        ));
    }

    #[test]
    fn verify_fixtures_rejects_state_nosignal() {
        // TDD(RED→GREEN): 注入 probe 返回 NoSignal ⇒ 信号态门失败闭合, error=State.
        let fixtures = vec![default_sdi_loopback()];
        let results = verify_fixtures(&fixtures, |_f| FixtureProbe {
            signal: SignalProbeResult {
                state: SignalState::NoSignal,
                video_format: None,
                content: VideoContentState::NoSignal,
            },
            test_pattern: None,
            audio_present: None,
        });
        assert!(!results[0].passed);
        assert!(matches!(
            results[0].error,
            Some(FixtureSignalError::State(_))
        ));
    }

    #[test]
    fn verify_fixtures_multiple_mixed() {
        // 多条 fixture: 一条通过、一条内容门拒 ⇒ 结果按序对应, 互不掩盖.
        let pass = default_sdi_loopback();
        let mut fail = default_sdi_loopback();
        fail.fixture_id = "FAIL-FIXTURE".into();
        let fixtures = vec![pass, fail];
        let results = verify_fixtures(&fixtures, |f| {
            if f.fixture_id == "FAIL-FIXTURE" {
                FixtureProbe {
                    signal: SignalProbeResult {
                        state: SignalState::Locked,
                        video_format: None,
                        content: VideoContentState::Black,
                    },
                    test_pattern: None,
                    audio_present: None,
                }
            } else {
                FixtureProbe {
                    signal: SignalProbeResult {
                        state: SignalState::Locked,
                        video_format: None,
                        content: VideoContentState::Active,
                    },
                    test_pattern: None,
                    audio_present: None,
                }
            }
        });
        assert_eq!(results.len(), 2);
        assert!(results[0].passed);
        assert!(!results[1].passed);
        assert_eq!(results[1].fixture_id, "FAIL-FIXTURE");
    }
}
