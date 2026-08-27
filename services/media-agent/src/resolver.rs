//! C1 Resolver — Device Registry (硬件身份) → GStreamer runtime address 物化.
//!
//! 核心不变量 (Phase 0.6 + A0 实测): **SDK 枚举序号 ≠ GStreamer `device-number`**
//! (本机 SDK#0=SDI 但 GStreamer#0=MiniMonitor4K). 任何直接映射必须删除.
//!
//! 正确做法: 以 SDK **DeviceHandle** (经 `hw-serial-number` 属性) 在运行时探测 GStreamer
//! 实例, 匹配到确定的 `device-number`, 再喂 `decklinkvideosrc`.
//!
//! ⚠️ 关键: DeviceHandle == GStreamer `hw-serial-number` 这一关系**尚未最终证据**
//! (C1 待办). Resolver 必须**真机**探明关系, 绝不假设. 匹配失败 → `Unresolved`
//! (生产拒绝, 绝不回退 device-number=0).

#![allow(dead_code)]

use crate::device::DeviceInfo;
use crate::port::{ConnectorType, PortDirection, VerificationLevel};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// GStreamer 探测到的 DeckLink 实例 (经由直接创建 `decklinkvideosrc` 实例, READY 态读取只读属性).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GStreamerDeviceProbe {
    pub device_number: u32,
    /// GStreamer `hw-serial-number` — **只读** 硬件序列号 / hardware ID; Resolver 选卡关键属性.
    /// 官方定义为只读, 绝不可写.
    pub hw_serial_number: Option<String>,
    /// GStreamer `persistent-id` (`gint64`, 对应 BMD `PersistentID`).本硬件 PersistentID 不支持 → 多为 `None`/0.
    pub persistent_id: Option<i64>,
    /// GStreamer `signal` — 该卡当前是否锁定 SDI 信号 (与身份无关的独立维度; 见用户复核 §六/§十三).
    /// `signal=false` **不**等于身份失败; 身份与信号是两个维度.
    pub signal: Option<bool>,
    pub model: Option<String>,
    /// 协商后的视频格式 (仅 gstreamer 构建, 有信号时方可读; 否则 None).
    /// 属于 Runtime Signal State, 不入 Manifest 永久状态.
    pub caps: Option<crate::port::VideoFormat>,
}

/// 匹配种类 (置信度语义见 `Confidence`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolverMatch {
    /// `BMD PersistentID` == GStreamer `persistent-id` (HIGH).
    PersistentIdExact,
    /// SDK `serial_number` == GStreamer `hw-serial-number` (HIGH).
    SerialExact,
    /// SDK `DeviceHandle` == GStreamer `hw-serial-number` (HIGH) — 当前硬件路径.
    DeviceHandleExact,
    /// SDK `TopologicalID` 末段 == GStreamer `hw-serial-number` (MEDIUM, 拓扑敏感).
    TopologicalIdGuess,
    /// 未匹配 (Resolver 必须显式报告, 绝不静默回退 device-number=0).
    Unresolved,
    /// 同一 SDK 设备命中多个 HIGH 候选 → 必须拒绝 (广播不容许猜设备).
    Ambiguous,
    /// 经 DeviceBindingManifest 显式契约验证 (权威路径, 用户 §11/§12): 清单声明的
    /// device-number 被 GStreamer 探测确认可打开且身份吻合 → 高置信, 非 runtime 猜测.
    ManifestVerified,
}

/// 匹配置信度.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    None,
}

/// GStreamer Runtime Probe 结果 (用户复核 §九, P1: 必须区分失败原因, 不能全压成 `Unresolved`).
#[derive(Debug, Clone)]
pub enum GstProbeOutcome {
    /// 探测成功 — 拿到 `decklinkvideosrc` 实例运行时属性列表.
    /// `errors` 收集各 `device-number` 的**分类失败原因** (见 `ProbeError`); 即便 `probes` 为空,
    /// 只要 `errors` 非空即表示 "有卡但打不开", 与 `Empty` (本机确无卡) 严格区分.
    Available {
        probes: Vec<GStreamerDeviceProbe>,
        errors: Vec<(u32, ProbeError)>,
    },
    /// 探测方法本身不可用 (GStreamer 未初始化 或 `decklinkvideosrc` 工厂缺失 / decklink 插件未安装).
    /// 这是 "方法不可用", **不是** "设备未解析" — 绝不能等同 `Unresolved` 误导现场 (用户复核 §九).
    Unavailable(String),
    /// 探测正常执行但枚举到 0 个 DeckLink 实例 (本机确无可用采集卡). 与 `Unavailable` 完全不同.
    /// 仅当 `probes` 与 `errors` 同时为空时成立.
    Empty,
}

/// 单设备探测失败分类 (用户 §⑥ / §九 P1: 必须区分失败原因, 不能全压成 `None` 让调用方误判为 "无此卡").
///
/// - `OpenFailed`     : `decklinkvideosrc`/fakesink 创建或 Pipeline 装配失败 (插件/运行时问题, 非设备问题).
/// - `NotFound`       : 该 `device-number` 对应采集卡不存在 (set_state 失败且 GError 指向无设备/找不到).
/// - `Busy`           : 采集卡存在但被其它进程/会话占用, 打开失败.
/// - `StateFailed`    : 设备存在但进入 `Playing` 状态失败 (硬件/状态错误); GStreamer 未暴露明细时的兜底分类.
/// - `PropertyMissing`: 设备已打开, 但 `hw-serial-number`/`persistent-id`/`model` 全部缺失, 无法建立身份.
///
/// `NotFound`/`Busy` 由 pipeline bus 上的 GError 文案 best-effort 启发式归类; 无法判定时归 `StateFailed`
/// (携带原始错误文案), 绝不静默丢弃.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeError {
    OpenFailed(String),
    NotFound,
    Busy,
    StateFailed(String),
    PropertyMissing(Vec<String>),
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeError::OpenFailed(s) => write!(f, "OpenFailed: {s}"),
            ProbeError::NotFound => write!(f, "NotFound: 该 device-number 无对应采集卡"),
            ProbeError::Busy => write!(f, "Busy: 采集卡被其它进程/会话占用"),
            ProbeError::StateFailed(s) => write!(f, "StateFailed: {s}"),
            ProbeError::PropertyMissing(v) => write!(f, "PropertyMissing: 缺失关键身份属性 {v:?}"),
        }
    }
}

/// 由 GStreamer bus 上的 GError 文案 best-effort 归类 set_state 失败原因 (见 `ProbeError`).
#[cfg(feature = "gstreamer")]
fn classify_set_state_error(n: u32, msg: &str) -> ProbeError {
    let m = msg.to_ascii_lowercase();
    if m.contains("already in use")
        || m.contains("device or resource busy")
        || m.contains("in use")
        || m.contains("busy")
    {
        ProbeError::Busy
    } else if m.contains("no device")
        || m.contains("no such")
        || m.contains("not found")
        || m.contains("could not find")
        || m.contains("does not exist")
        || m.contains("no decklink")
    {
        ProbeError::NotFound
    } else {
        ProbeError::StateFailed(format!("device-number {n}: {msg}"))
    }
}

/// 从 pipeline bus 抽取首个 `Error` 消息 (设备打开失败的 GError 在此异步送达). 仅消费已排队消息, 不阻塞.
#[cfg(feature = "gstreamer")]
fn drain_bus_error(pipeline: &gstreamer::Pipeline) -> Option<String> {
    use gstreamer::prelude::*;
    let bus = pipeline.bus()?;
    let mut first: Option<String> = None;
    while let Some(msg) = bus.pop() {
        if let gstreamer::MessageView::Error(e) = msg.view() {
            if first.is_none() {
                first = Some(format!("{}", e.error()));
            }
        }
    }
    first
}

/// 直接 probe `decklinkvideosrc` 实例, 物化 SDK DeviceHandle → GStreamer `device-number`.
///
/// 为什么不用 `GstDeviceMonitor` (用户复核, 撤回 `b52e2b6`): 当前 DeckLink 官方插件只暴露
/// `decklinkvideosrc` / `decklinkaudiosrc` **element**, 不提供 `GstDeviceProvider`; 实机验证
/// `gst-device-monitor` 不列出 DeckLink. 故枚举入口改成**直接创建 element 实例**, 按 `device-number`
/// 遍历, 打开到 PLAYING 读取只读身份属性: decklink 为 live source, 裸 Element 直接 set_state 不执行 start() 故身份属性恒 null, 须置于 Pipeline+fakesink 设 PLAYING 真正打开设备并填充属性 (即便无信号设备也已打开, 身份属性仍可读).
///
/// 每序号读取: `device-number`(guint) / `hw-serial-number`(只读硬件 ID) / `persistent-id`(gint64) /
/// `signal`(bool) / `model`(String). `connection`/`mode` 用 element 默认 (connection=auto 自动识别
/// SDI, mode=auto 自动探测) — 满足 binding probe; 显式设置 enum 需插件专属 enum 类型, 核心 gstreamer
/// crate 不暴露.
///
/// ⚠️ `device-number` **绝不默认 0**: GStreamer#0 在实机是 MiniMonitor 输出卡. 必须由 Resolver 经
/// `hw-serial-number` 命中后确定. 命中失败 → 该序号 `None` (不计入), 绝不回退 0.
///
/// 返回 `GstProbeOutcome` 以区分 `Unavailable` / `Empty` / `Available` (见 §九).
#[cfg(feature = "gstreamer")]
pub fn probe_gstreamer_devices(max: usize, require_identity: bool) -> GstProbeOutcome {
    // GStreamer 初始化 (幂等). 失败 → 探测方法不可用 (非设备失败).
    if let Err(e) = gstreamer::init() {
        return GstProbeOutcome::Unavailable(format!("gstreamer init 失败: {e}"));
    }
    // decklinkvideosrc 工厂存在 = decklink 插件安装; 否则探测方法不适用本机.
    if gstreamer::ElementFactory::find("decklinkvideosrc").is_none() {
        return GstProbeOutcome::Unavailable(
            "decklinkvideosrc 工厂不存在 (GStreamer decklink 插件未安装); 探测方法不可用"
                .to_string(),
        );
    }
    let mut probes = Vec::new();
    let mut errors: Vec<(u32, ProbeError)> = Vec::new();
    for n in 0..(max as u32) {
        match probe_one_device_number(n, require_identity) {
            Ok(p) => probes.push(p),
            Err(e) => errors.push((n, e)),
        }
    }
    if probes.is_empty() && errors.is_empty() {
        return GstProbeOutcome::Empty;
    }
    GstProbeOutcome::Available { probes, errors }
}

/// 探测单个 `device-number` 的 decklinkvideosrc 实例. 打开到 PLAYING 读只读属性; 失败按 `ProbeError`
/// 分类返回 (见枚举文档), **绝不** 把 "卡存在但打不开" 与 "无此卡" 混为一谈 (用户 §⑥).
#[cfg(feature = "gstreamer")]
fn probe_one_device_number(
    n: u32,
    require_identity: bool,
) -> Result<GStreamerDeviceProbe, ProbeError> {
    use gstreamer::prelude::*;
    let pipeline = gstreamer::Pipeline::default();
    // 元素创建/装配失败 = 插件或运行时问题, 与具体设备无关 → OpenFailed.
    let el = gstreamer::ElementFactory::make("decklinkvideosrc")
        .build()
        .map_err(|e| ProbeError::OpenFailed(format!("decklinkvideosrc 创建失败: {e}")))?;
    // 以 device-number 绑定目标采集卡 (GStreamer 运行时地址).
    el.set_property("device-number", n as i32);
    // live source 需置于 Pipeline (并接 fakesink) 才被驱动; 裸 Element 直接 set_state 不会执行
    // start(), 故 hw-serial-number 恒为 null. Pipeline 设 PLAYING 才 fully-active 并填充只读身份属性
    // (hw-serial-number 等); 即便无信号 (Signal lost) 设备也已打开, 身份属性应已可读.
    let sink = gstreamer::ElementFactory::make("fakesink")
        .build()
        .map_err(|e| ProbeError::OpenFailed(format!("fakesink 创建失败: {e}")))?;
    pipeline
        .add(&el)
        .map_err(|e| ProbeError::OpenFailed(format!("pipeline.add(src) 失败: {e}")))?;
    pipeline
        .add(&sink)
        .map_err(|e| ProbeError::OpenFailed(format!("pipeline.add(sink) 失败: {e}")))?;
    el.link(&sink)
        .map_err(|e| ProbeError::OpenFailed(format!("link 失败: {e}")))?;
    // 进 PLAYING 真正打开设备. set_state 同步结果 + 异步失败均可能在 bus 上报 GError:
    // 对 live source, 设备打开失败 (卡不存在/被占用/硬件错误) 通常以异步 Error message 送达 bus,
    // 故这里既看同步返回值也 drain bus.
    let playing = pipeline.set_state(gstreamer::State::Playing);
    // 少量延时兜底 live source 异步 preroll/错误上报, 确保设备已打开或错误已入队.
    std::thread::sleep(std::time::Duration::from_millis(300));
    let err_msg: Option<String> = match playing {
        Ok(_) => drain_bus_error(&pipeline),
        Err(_) => drain_bus_error(&pipeline)
            .or_else(|| Some("set_state(Playing) 同步返回失败 (无 bus 错误明细)".to_string())),
    };
    if let Some(msg) = err_msg {
        let _ = pipeline.set_state(gstreamer::State::Null);
        return Err(classify_set_state_error(n, &msg));
    }
    // 读取只读属性 (find_property 守卫防缺属性 panic; NULL 字符串用 Option<String> 归 None).
    let hw_serial_number = el.find_property("hw-serial-number").and_then(|_| {
        non_empty(
            el.property::<Option<String>>("hw-serial-number")
                .unwrap_or_default(),
        )
    });
    let persistent_id = el.find_property("persistent-id").and_then(|_| {
        let v = el.property::<i64>("persistent-id");
        if v > 0 {
            Some(v)
        } else {
            None
        }
    });
    let signal = el
        .find_property("signal")
        .map(|_| el.property::<bool>("signal"));
    let model = el
        .find_property("model")
        .and_then(|_| non_empty(el.property::<Option<String>>("model").unwrap_or_default()));
    // 设备已打开但无任何身份属性:
    // - 非清单模式 (require_identity=true): 无法建立身份 → 归 PropertyMissing (区别于 "无此卡").
    // - 清单模式 (require_identity=false): 身份由 DeviceBindingManifest 显式契约提供, 不依赖
    //   GStreamer 只读属性 (本硬件 hw-serial-number 等恒空串, 见 abda19f / device-binding.example.json),
    //   故只要卡能打开 (set_state Playing 成功) 即计入 probe, 身份匹配交由 resolve_with_manifest.
    if require_identity && hw_serial_number.is_none() && persistent_id.is_none() && model.is_none()
    {
        let missing = vec![
            "hw-serial-number".to_string(),
            "persistent-id".to_string(),
            "model".to_string(),
        ];
        let _ = pipeline.set_state(gstreamer::State::Null);
        return Err(ProbeError::PropertyMissing(missing));
    }
    // 释放设备. 只要 set_state(Playing) 成功 (=设备已打开=真实采集卡) 即计入 probe,
    // 即便 hw-serial-number 为空 (本硬件可能不暴露 serial); 最终匹配/Unresolved 由 resolve() 决定.
    let _ = pipeline.set_state(gstreamer::State::Null);
    let caps = read_negotiated_caps(&el);
    Ok(GStreamerDeviceProbe {
        device_number: n,
        hw_serial_number,
        persistent_id,
        signal,
        model,
        caps,
    })
}

/// 空串/未设置串归 `None` (GStreamer 字符串属性未设置时返回空串).
#[cfg(feature = "gstreamer")]
fn non_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// 读取已协商的源 pad caps → 视频格式 (best-effort; 仅 gstreamer 构建, 有信号时可读).
/// 仅用 `caps.to_string()` 做轻量解析, 避免引入 GStreamer Fraction 类型依赖风险.
#[cfg(feature = "gstreamer")]
fn read_negotiated_caps(el: &gstreamer::Element) -> Option<crate::port::VideoFormat> {
    use gstreamer::prelude::*;
    let pad = el.static_pad("src")?;
    let caps = pad.current_caps()?;
    if caps.is_empty() {
        return None;
    }
    let text = caps.to_string();
    let width = parse_int(&text, "width=(int)")?;
    let height = parse_int(&text, "height=(int)")?;
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
    Some(crate::port::VideoFormat {
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

#[cfg(not(feature = "gstreamer"))]
pub fn probe_gstreamer_devices(_max: usize, _require_identity: bool) -> GstProbeOutcome {
    // 非 gstreamer 构建: 探测方法不适用 (无 GStreamer 运行时). 真实探测仅 `gstreamer` feature 构建.
    GstProbeOutcome::Unavailable(
        "gstreamer feature 未启用; 非 gstreamer 构建不物化运行时地址".to_string(),
    )
}

/// 从 SDK `DeviceHandle` 提取 `TopologicalID` 末段 (`:` 分隔最后一段) 作猜测键.
/// 例: `46:00000000:002e4500` → `002e4500`. 仅 TopologicalIdGuess 用 (MEDIUM).
fn topo_of(handle: &str) -> Option<String> {
    handle.rsplit(':').next().map(|s| s.to_string())
}

/// 在 GStreamer `hw-serial-number`(只读属性) 上, 为**单个 probe** 判定该 SDK 设备的最佳匹配种类.
/// 每 probe 至多返回一种 (按优先级 PersistentId > Serial > DeviceHandle > Topological);
/// Topological 为 MEDIUM (拓扑敏感, 仅诊断可用).
fn best_kind_for(
    sdk: &DeviceInfo,
    p: &GStreamerDeviceProbe,
) -> Option<(ResolverMatch, Confidence)> {
    // 1) PersistentID 精确 (HIGH)
    if let (Some(sdk_pid), Some(gst_pid)) = (sdk.bmd_persistent_id, p.persistent_id) {
        if sdk_pid == gst_pid {
            return Some((ResolverMatch::PersistentIdExact, Confidence::High));
        }
    }
    // 2) serial_number 精确 (HIGH)
    if let (Some(s), Some(g)) = (&sdk.serial_number, &p.hw_serial_number) {
        if s == g {
            return Some((ResolverMatch::SerialExact, Confidence::High));
        }
    }
    // 3) DeviceHandle 精确 (HIGH) — 当前硬件路径 (DeviceHandle ↔ hw-serial-number 待 C1 实机证据).
    if let (Some(h), Some(g)) = (&sdk.bmd_device_handle, &p.hw_serial_number) {
        if h == g {
            return Some((ResolverMatch::DeviceHandleExact, Confidence::High));
        }
    }
    // 4) TopologicalID 末段猜测 (MEDIUM, 拓扑敏感, 仅诊断).
    if let (Some(topo), Some(g)) = (
        sdk.bmd_device_handle.as_deref().and_then(topo_of).as_ref(),
        p.hw_serial_number.as_ref(),
    ) {
        if topo == g {
            return Some((ResolverMatch::TopologicalIdGuess, Confidence::Medium));
        }
    }
    None
}

/// 解析单个 SDK 设备 → GStreamer device instance 的绑定关系.
///
/// 关键守卫 (用户复核 §七/§八):
/// - **多重 HIGH 候选 → `Ambiguous`**: 同一 SDK 设备若匹配到 ≥2 个 HIGH 置信的 GStreamer 实例,
///   广播系统**必须拒绝** (宁可拒不变猜设备), 绝不默认选其中一个.
/// - MEDIUM (TopologicalIdGuess) 仅用于诊断; `collect_bindings` 在生产绑定中默认拒绝.
fn find_match<'a>(
    sdk: &DeviceInfo,
    probes: &'a [GStreamerDeviceProbe],
) -> (ResolverMatch, Confidence, Option<&'a GStreamerDeviceProbe>) {
    // 收集每个 probe 的最佳匹配 (每 probe 至多一种).
    let mut per_probe: Vec<(ResolverMatch, Confidence, usize)> = Vec::new();
    for (idx, p) in probes.iter().enumerate() {
        if let Some((k, c)) = best_kind_for(sdk, p) {
            per_probe.push((k, c, idx));
        }
    }
    // 多重 HIGH 候选 → Ambiguous (拒绝).
    let high_count = per_probe
        .iter()
        .filter(|(_, c, _)| *c == Confidence::High)
        .count();
    if high_count > 1 {
        return (ResolverMatch::Ambiguous, Confidence::None, None);
    }
    // 单一最佳 (HIGH 优先, 否则 MEDIUM).
    per_probe
        .into_iter()
        .max_by_key(|(k, _, _)| match k {
            ResolverMatch::PersistentIdExact => 4,
            ResolverMatch::SerialExact => 3,
            ResolverMatch::DeviceHandleExact => 2,
            ResolverMatch::TopologicalIdGuess => 1,
            _ => 0,
        })
        .map(|(k, c, idx)| (k, c, Some(&probes[idx])))
        .unwrap_or((ResolverMatch::Unresolved, Confidence::None, None))
}

/// 单个 SDK 设备解析证据 (C1 JSON 输出).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverEvidence {
    pub device_id: Uuid,
    pub model: Option<String>,
    pub bmd_device_handle: Option<String>,
    pub gst_device_number: Option<u32>,
    pub gst_hw_serial_number: Option<String>,
    /// 命中实例的 `signal` 维度 (独立; 不为 false 时 identity 仍 PASS, 见用户复核 §六).
    pub gst_signal: Option<bool>,
    pub match_kind: ResolverMatch,
    pub confidence: Confidence,
    pub note: String,
}

/// 解析所有 SDK 设备 → GStreamer 实例映射 (C1 证据).
pub fn resolve(devices: &[DeviceInfo], probes: &[GStreamerDeviceProbe]) -> Vec<ResolverEvidence> {
    let mut evidence = Vec::new();
    for dev in devices {
        let (kind, conf, matched) = find_match(dev, probes);
        let (gst_num, gst_serial, gst_sig, note) = match (matched, kind) {
            (_, ResolverMatch::Ambiguous) => (
                None,
                None,
                None,
                "AMBIGUOUS: same SDK device matches >=2 HIGH-confidence GStreamer instances; production MUST reject, never guess".to_string(),
            ),
            (Some(p), _) => (
                Some(p.device_number),
                p.hw_serial_number.clone(),
                p.signal,
                format!(
                    "match={:?} via gstreamer device-number={}",
                    kind, p.device_number
                ),
            ),
            (None, _) => (
                None,
                None,
                None,
                format!(
                    "no exact match (match={:?}); production MUST reject, never fallback to device-number=0",
                    kind
                ),
            ),
        };
        evidence.push(ResolverEvidence {
            device_id: dev.device_id,
            model: dev.serial_number.clone().or(Some(dev.model.clone())),
            bmd_device_handle: dev.bmd_device_handle.clone(),
            gst_device_number: gst_num,
            gst_hw_serial_number: gst_serial,
            gst_signal: gst_sig,
            match_kind: kind,
            confidence: conf,
            note,
        });
    }
    evidence
}

/// 解析后的稳定绑定 (喂给 `decklinkvideosrc/audiosrc` 的 `device-number`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDeviceBinding {
    pub device_number: u32,
    pub hw_serial_number: Option<String>,
    pub confidence: Confidence,
    pub match_kind: ResolverMatch,
}

/// 收集生产绑定: 仅接受 HIGH 置信 (PersistentId/Serial/DeviceHandle 精确匹配).
/// Ambiguous / Unresolved / MEDIUM(TopologicalIdGuess) 一律不进入生产绑定 → 触发
/// materialize IdentityUnresolved, 绝不盲开 device 0 (用户复核 §七/§八).
pub fn collect_bindings(
    devices: &[DeviceInfo],
    probes: &[GStreamerDeviceProbe],
) -> HashMap<Uuid, ResolvedDeviceBinding> {
    let mut map = HashMap::new();
    for dev in devices {
        // 解析严格模式: 仅接受 HIGH 置信 (PersistentId/Serial/DeviceHandle 精确匹配).
        // Ambiguous / Unresolved / MEDIUM(TopologicalIdGuess) 一律不进入生产绑定 → 触发
        // materialize IdentityUnresolved, 绝不盲开 device 0. (用户复核 §七/§八)
        let (kind, _conf, matched) = find_match(dev, probes);
        let Some(p) = matched else { continue };
        // 生产绑定只接受 HIGH; MEDIUM (TopologicalIdGuess) 与 Ambiguous/Unresolved 拒绝,
        // 广播宁可拒不变猜设备.
        match kind {
            ResolverMatch::PersistentIdExact
            | ResolverMatch::SerialExact
            | ResolverMatch::DeviceHandleExact => {}
            _ => continue,
        }
        map.insert(
            dev.device_id,
            ResolvedDeviceBinding {
                device_number: p.device_number,
                hw_serial_number: p.hw_serial_number.clone(),
                confidence: Confidence::High,
                match_kind: kind,
            },
        );
    }
    map
}

/// 严格解析: 确保所有 `required` 设备都能解析到 HIGH 绑定, 否则报错.
/// (C1 物化前置; 调用方据此拒绝 IdentityUnresolved.)
pub fn resolve_strict(
    devices: &[DeviceInfo],
    probes: &[GStreamerDeviceProbe],
    required: &[Uuid],
) -> Result<HashMap<Uuid, ResolvedDeviceBinding>, String> {
    let bindings = collect_bindings(devices, probes);
    for rid in required {
        if !bindings.contains_key(rid) {
            return Err(format!(
                "设备 {rid} 无法解析到 HIGH 置信 GStreamer 绑定 (Ambiguous/Unresolved/MEDIUM); 生产拒绝"
            ));
        }
    }
    Ok(bindings)
}

/// C1 最大探测设备数 (防无限枚举; 真机通常 1-3 块卡).
pub const MAX_PROBE_DEVICES: usize = 8;

/// 当前主机标识: 优先 `VBMF_MACHINE_ID`, 否则 `HOSTNAME`; 均空则空串
/// (调用方据此跳过 machine_id 校验, 而非误报).
pub fn current_machine_id() -> String {
    std::env::var("VBMF_MACHINE_ID")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default()
}

/// 授权绑定清单: 显式 `SDK-handle → GStreamer device-number` 运行时契约.
///
/// 这是用户 §11/§12 的 "下一阶段明确设计决策": **停止 runtime 猜测**, 改由 Provisioning
/// 维护的**显式**映射作为绑定权威. GStreamer 运行时探测降级为**校验器** (确认清单声明的
/// device-number 真实可打开且身份吻合), 任何不符 → 失败闭合 (`Unresolved`), 绝不猜.
///
/// 关键不变量:
/// - 仅按 `bmd_device_handle` (canonical 真实身份) 索引; 绝不按枚举序号/拓扑猜测.
/// - `gst_device_number` 来自运营/Provisioning 的**现场核实** (如 `VBMF_RESOLVER=1` 原始
///   探测 + 物理确认), 不在 runtime 反推.
/// - `expected_hw_serial_number` / `expected_model` 为可选交叉校验; 当前硬件 serial 恒空,
///   留空 (=不校验) 仍可工作, 但建议随驱动升级回填以强化闭合.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceBindingManifest {
    /// 清单 schema 版本 (如 "1.0").
    pub manifest_version: String,
    /// 绑定主机标识 (hostname 或显式机器 ID): 清单仅对声明主机有效, 误投到别机 → 失败闭合.
    pub machine_id: String,
    /// 生成/校验工具或操作员标识.
    pub generated_by: String,
    /// 生成时间 (ISO8601).
    pub generated_at: String,
    /// BMD SDK 版本 (如 "16.0"); 与运行环境不符 → 告警.
    pub bmd_sdk_version: Option<String>,
    /// GStreamer decklink 插件版本; 实际运行时版本已接入软校验 (用户 §五 P1-2), 不符 → 告警.
    pub gst_decklink_plugin_version: Option<String>,
    /// GStreamer 运行时核心版本 (major.minor.micro); 实际运行时版本已接入软校验, 不符 → 告警.
    /// 留空表示不校验 (旧清单向后兼容).
    pub gst_runtime_version: Option<String>,
    /// 备注 (可选).
    pub notes: Option<String>,
    /// 绑定条目.
    pub bindings: Vec<BindingEntry>,
}

/// 端口级绑定 (v2 schema): 把 "某块设备的某个物理端口" 显式声明, 而非仅 device-level。
///
/// HARD RULE: 绝不把 `SDI1` 定义成 `ConnectorType`; 物理 SDI #1/#2 是 `ConnectorType=SDI`
/// + `ordinal=1/2`。`required=true` 的端口若 HW-PORT-01 验收时 signal 未 Locked → 失败闭合。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PortBinding {
    pub connector: ConnectorType,
    /// 物理端口序号 (1-based; 与 `ConnectorType` 共同构成端口身份).
    pub ordinal: u32,
    /// 端口方向 (由 SDK 能力/Manifest 声明, 绝不由 device-number/信号推断).
    pub direction: PortDirection,
    /// 是否参与 HW-PORT-01 验收必需项 (required 且 signal 未 Locked → 验收失败).
    pub required: bool,
    /// 该端口在 HW-PORT-01 验收中所需最低验证等级 (§十八): 声明/运行时打开/信号锁定/环回验证.
    /// 缺省 `Declared` (仅声明); 越高越严, 验收须收集到对应等级运行时证据 (fail-closed).
    #[serde(default)]
    pub verification: VerificationLevel,
}

/// 单条 `SDK-handle → GStreamer device-number` 绑定.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BindingEntry {
    /// 人类可读标签 (如 "SDI-IN-1").
    pub label: Option<String>,
    /// canonical 真实身份: BMD `DeviceHandle` (来自 `BmdDeviceIdentity.device_handle`).
    pub bmd_device_handle: String,
    /// GStreamer `decklinkvideosrc` 的 device-number (运行时地址).
    pub gst_device_number: u32,
    /// 可选交叉校验: 期望的 GStreamer `hw-serial-number`. 若插件暴露 serial 则校验, 不符 → 失败闭合.
    pub expected_hw_serial_number: Option<String>,
    /// 可选交叉校验: 期望型号.
    pub expected_model: Option<String>,
    /// v2: 端口级绑定 (ConnectorType + ordinal + direction + required). 缺省 = device-level (v1) 兼容.
    /// 绝不得含除运行时地址外的硬编码语义; 物理 SDI #1/#2 = `SDI` + ordinal.
    #[serde(default)]
    pub port: Option<PortBinding>,
}

impl DeviceBindingManifest {
    /// 从 JSON 文件加载清单. 失败 (文件缺失/格式错) → `Err`, 调用方据此失败闭合.
    pub fn load(path: &str) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("DeviceBindingManifest 读取失败 ({path}): {e}"))?;
        serde_json::from_str(&text)
            .map_err(|e| format!("DeviceBindingManifest 解析失败 ({path}): {e}"))
    }

    /// 结构完整性校验 (用户 §六 / Manifest v2, STEP 4): 加载后立即调用, 失败即拒绝 (ManifestInvalid).
    /// 检查项:
    /// - `manifest_version` 须能解析为版本号, 且 **v2 (major>=2) 强制端口级绑定** (`port` 字段非空);
    ///   v1 (major<2) 保持 device-level 向后兼容 (port 可空).
    /// - `machine_id` 非空 / 至少一条绑定 / `bmd_device_handle` 唯一且非空 / `gst_device_number` 唯一.
    /// - 端口级绑定 (`port`) 内部一致: connector 不得 Unknown、ordinal>=1、direction 不得 Unknown.
    pub fn validate_manifest(&self) -> Result<(), String> {
        let major = self
            .manifest_version
            .trim()
            .split('.')
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| {
                "ManifestInvalid: manifest_version 无法解析为版本号 (应为 \"2\" / \"2.0\" 等)"
                    .to_string()
            })?;
        if self.machine_id.trim().is_empty() {
            return Err("ManifestInvalid: machine_id 为空 (无法绑定主机, 拒绝)".to_string());
        }
        if self.bindings.is_empty() {
            return Err("ManifestInvalid: bindings 为空 (无绑定条目, 拒绝)".to_string());
        }
        let is_v2 = major >= 2;
        let mut seen_handle = HashSet::new();
        for b in &self.bindings {
            if b.bmd_device_handle.trim().is_empty() {
                return Err("ManifestInvalid: 存在空 bmd_device_handle 条目".to_string());
            }
            if !seen_handle.insert(b.bmd_device_handle.clone()) {
                return Err(format!(
                    "ManifestInvalid: 重复的 bmd_device_handle '{}' (设备身份冲突, 拒绝)",
                    b.bmd_device_handle
                ));
            }
            if is_v2 && b.port.is_none() {
                return Err(format!(
                    "ManifestInvalid: v2 清单绑定 '{}' 缺少端口级 port 声明 (拒绝 device-level-only)",
                    b.bmd_device_handle
                ));
            }
            if let Some(p) = &b.port {
                if p.connector == ConnectorType::Unknown {
                    return Err(format!(
                        "ManifestInvalid: 绑定 '{}' port.connector=Unknown (不得声明未知连接器)",
                        b.bmd_device_handle
                    ));
                }
                if p.ordinal < 1 {
                    return Err(format!(
                        "ManifestInvalid: 绑定 '{}' port.ordinal={} < 1",
                        b.bmd_device_handle, p.ordinal
                    ));
                }
                if p.direction == PortDirection::Unknown {
                    return Err(format!(
                        "ManifestInvalid: 绑定 '{}' port.direction=Unknown (必须显式 Input/Output)",
                        b.bmd_device_handle
                    ));
                }
            }
        }
        let mut seen_num = HashSet::new();
        for b in &self.bindings {
            if !seen_num.insert(b.gst_device_number) {
                return Err(format!(
                    "ManifestInvalid: 重复的 gst_device_number {} (运行时地址冲突, 拒绝)",
                    b.gst_device_number
                ));
            }
        }
        Ok(())
    }

    /// 主机身份校验 (用户 §五): 运行时主机标识非空且与声明 machine_id 不符 → 失败闭合
    /// (拒绝, 非 warning). machine_id 不一致 = 误投/串机器, 绝不能只是警告.
    /// 运行环境无法判定主机 (未设 VBMF_MACHINE_ID/HOSTNAME) → 跳过校验而非误报.
    pub fn check_machine_identity(&self, runtime_machine_id: &str) -> Result<(), String> {
        if runtime_machine_id.is_empty() {
            return Ok(());
        }
        if self.machine_id != runtime_machine_id {
            return Err(format!(
                "ManifestEnvironmentMismatch: 清单 machine_id='{}' 与当前主机 '{}' 不符 (误投/串机器, 生产拒绝)",
                self.machine_id, runtime_machine_id
            ));
        }
        Ok(())
    }

    /// 校验运行环境版本一致性 (软校验: 仅返回告警, 不阻断启动).
    ///
    /// 用户 §五 P1-2: **必须传入真实 runtime 版本**, 不能再用 `None`. 此前调用方传 `(None, None)`
    /// 使 `bmd_sdk_version` / `gst_decklink_plugin_version` 形同虚设. 现三参数分别接:
    /// - `sdk_version`    : **声明式** BMD SDK 版本 (`declared_bmd_sdk_version`, Provisioning 经 env 声明, 与清单同为 ops 控制; 真实运行时身份见 `detected_bmd_sdk_version`, P1-1 整改)
    /// - `plugin_version` : 真实 GStreamer decklink 插件版本 (`actual_decklink_plugin_version`)
    /// - `gst_version`    : 真实 GStreamer 运行时核心版本 (`actual_gstreamer_version`)
    pub fn validate_environment(
        &self,
        sdk_version: Option<&str>,
        plugin_version: Option<&str>,
        gst_version: Option<&str>,
    ) -> Vec<String> {
        let mut warns = Vec::new();
        if let (Some(decl), Some(actual)) = (&self.bmd_sdk_version, sdk_version) {
            if decl != actual {
                warns.push(format!("清单 BMD SDK 版本 '{decl}' 与运行 '{actual}' 不符"));
            }
        }
        if let (Some(decl), Some(actual)) = (&self.gst_decklink_plugin_version, plugin_version) {
            if decl != actual {
                warns.push(format!(
                    "清单 GStreamer decklink 插件版本 '{decl}' 与运行 '{actual}' 不符"
                ));
            }
        }
        if let (Some(decl), Some(actual)) = (&self.gst_runtime_version, gst_version) {
            if decl != actual {
                warns.push(format!(
                    "清单 GStreamer 运行时版本 '{decl}' 与运行 '{actual}' 不符"
                ));
            }
        }
        warns
    }

    /// 在清单中按 SDK DeviceHandle 查绑定条目.
    pub fn lookup(&self, bmd_device_handle: &str) -> Option<&BindingEntry> {
        self.bindings
            .iter()
            .find(|b| b.bmd_device_handle == bmd_device_handle)
    }
}

/// 声明式 (ops 在 Provisioning 时经 env 显式声明) 的 BMD SDK 版本, 用于与 Manifest `bmd_sdk_version`
/// 做一致性软校验. 这不是运行时真实探测 —— 真实探测见 `detected_bmd_sdk_version`
/// (P1-1 整改: 二者概念不能都叫 "actual"). 默认 "unknown" (未声明).
pub fn declared_bmd_sdk_version() -> String {
    std::env::var("VBMF_BMD_SDK_VERSION").unwrap_or_else(|_| "unknown".to_string())
}

/// 真实运行时 SDK 身份探测 (P1-1 整改): 不依赖 env 声明, 而是基于编译期 SDK include 路径
/// (`DECKLINK_SDK_INCLUDE`, 即 "SDK build identity") 与运行时实际可解析到的 `libDeckLinkAPI.so`
/// 路径及字节大小 (即 "libDeckLinkAPI.so identity"). 返回紧凑身份串; 无法探测时返回 "unknown".
/// 这是 provenance, 用于健康/证据归档, 不与 Manifest 版本号做误比 (版本号一致性用 `declared_bmd_sdk_version` 比对).
pub fn detected_bmd_sdk_version() -> String {
    let build = option_env!("DECKLINK_SDK_INCLUDE").unwrap_or("n/a");
    let lib_identity = match resolve_decklink_lib() {
        Some(lib) => match std::fs::metadata(&lib) {
            Ok(m) => format!("lib={lib};lib_bytes={}", m.len()),
            Err(_) => format!("lib={lib};lib_bytes=?"),
        },
        None => "lib=unresolved".to_string(),
    };
    format!("sdk_include={build};{lib_identity}")
}

/// 解析运行时实际可加载的 libDeckLinkAPI.so 路径 (best-effort, 不新增依赖).
fn resolve_decklink_lib() -> Option<String> {
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(ld) = std::env::var("LD_LIBRARY_PATH") {
        for d in ld.split(':') {
            if !d.is_empty() {
                candidates.push(format!("{d}/libDeckLinkAPI.so"));
            }
        }
    }
    candidates.push("/usr/lib/blackmagic/libDeckLinkAPI.so".into());
    candidates.push("/usr/lib/libDeckLinkAPI.so".into());
    candidates.push("/usr/lib/x86_64-linux-gnu/libDeckLinkAPI.so".into());
    candidates.push("/Library/Blackmagic/DeckLink/libDeckLinkAPI.so".into());
    if let Some(inc) = option_env!("DECKLINK_SDK_INCLUDE") {
        if let Some(parent) = std::path::Path::new(inc).parent() {
            candidates.push(
                parent
                    .join("lib/libDeckLinkAPI.so")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    candidates
        .into_iter()
        .find(|p| std::path::Path::new(p).exists())
}

/// 实际 GStreamer 运行时核心版本 (major.minor.micro); 用于 Manifest `gst_runtime_version` 软校验.
#[cfg(feature = "gstreamer")]
pub fn actual_gstreamer_version() -> String {
    let (maj, min, mic, _) = gstreamer::version();
    format!("{maj}.{min}.{mic}")
}

/// 实际 GStreamer decklink 插件版本 (从已加载插件元数据读取); 读不到 (插件未加载) → `None`.
#[cfg(feature = "gstreamer")]
pub fn actual_decklink_plugin_version() -> Option<String> {
    use gstreamer::prelude::*;
    gstreamer::ElementFactory::find("decklinkvideosrc")
        .and_then(|f| f.plugin())
        .map(|p| p.version().to_string())
}

/// 基于 `DeviceBindingManifest` 解析 (权威路径). GStreamer 探测仅作**校验**:
/// - 清单声明 device-number → 在 probe 中必须存在且曾真实打开 (`Available` 含该序号);
/// - 若清单记录 `expected_hw_serial_number` / `expected_model`, 须与 probe 读到的吻合;
/// - 不符 → 该设备 `Unresolved` (失败闭合, 绝不猜);
/// - 设备不在清单 → `Unresolved` (runtime 猜测已禁用, 用户 §11/§12).
pub fn resolve_with_manifest(
    devices: &[DeviceInfo],
    probes: &[GStreamerDeviceProbe],
    manifest: &DeviceBindingManifest,
) -> Vec<ResolverEvidence> {
    let mut evidence = Vec::new();
    for dev in devices {
        let handle = dev.bmd_device_handle.as_deref().unwrap_or("");
        let (gst_num, gst_serial, gst_sig, kind, conf, note) = match manifest.lookup(handle) {
            None => (
                None,
                None,
                None,
                ResolverMatch::Unresolved,
                Confidence::None,
                "device not present in DeviceBindingManifest; runtime auto-resolution disabled by design (用户 §11/§12)".to_string(),
            ),
            Some(b) => {
                let probe = probes.iter().find(|p| p.device_number == b.gst_device_number);
                match probe {
                    None => (
                        None,
                        None,
                        None,
                        ResolverMatch::Unresolved,
                        Confidence::None,
                        format!(
                            "manifest claims gst_device_number={} for handle='{}' but GStreamer probe did not open that device \
                             (misconfig or card missing); fail closed",
                            b.gst_device_number, handle
                        ),
                    ),
                    Some(p) => {
                        // 交叉校验: expected_* 为可选强化项. 运行时未暴露该属性 (actual=None,
                        // 本硬件 hw-serial-number/model 恒空串) 视为 "无法校验" -> 跳过 (不失败闭合);
                        // 仅当运行时实测值存在且与期望矛盾 (Some(x) != Some(exp)) 才失败闭合.
                        let serial_ok = match &b.expected_hw_serial_number {
                            Some(exp) => match &p.hw_serial_number {
                                Some(act) => act == exp,
                                None => true,
                            },
                            None => true,
                        };
                        let model_ok = match &b.expected_model {
                            Some(exp) => match &p.model {
                                Some(act) => act == exp,
                                None => true,
                            },
                            None => true,
                        };
                        if serial_ok && model_ok {
                            (
                                Some(p.device_number),
                                p.hw_serial_number.clone(),
                                p.signal,
                                ResolverMatch::ManifestVerified,
                                Confidence::High,
                                format!(
                                    "manifest-verified: handle='{}' -> gst_device_number={} (probe open OK{})",
                                    handle,
                                    p.device_number,
                                    if b.expected_hw_serial_number.is_some() {
                                        ", serial cross-check OK"
                                    } else {
                                        ""
                                    }
                                ),
                            )
                        } else {
                            (
                                None,
                                None,
                                None,
                                ResolverMatch::Unresolved,
                                Confidence::None,
                                format!(
                                    "manifest-verified FAILED cross-check: handle='{}' gst_device_number={} \
                                     expected_serial={:?} actual={:?} expected_model={:?} actual={:?}; fail closed",
                                    handle,
                                    b.gst_device_number,
                                    b.expected_hw_serial_number,
                                    p.hw_serial_number,
                                    b.expected_model,
                                    p.model
                                ),
                            )
                        }
                    }
                }
            }
        };
        evidence.push(ResolverEvidence {
            device_id: dev.device_id,
            model: dev.serial_number.clone().or(Some(dev.model.clone())),
            bmd_device_handle: dev.bmd_device_handle.clone(),
            gst_device_number: gst_num,
            gst_hw_serial_number: gst_serial,
            gst_signal: gst_sig,
            match_kind: kind,
            confidence: conf,
            note,
        });
    }
    evidence
}

/// 仅接受 `ManifestVerified` 高置信绑定 (喂给 `materialize`). 未验证/不符 → 不进入 (失败闭合).
pub fn collect_bindings_from_manifest(
    devices: &[DeviceInfo],
    probes: &[GStreamerDeviceProbe],
    manifest: &DeviceBindingManifest,
) -> HashMap<Uuid, ResolvedDeviceBinding> {
    let mut map = HashMap::new();
    let evidence = resolve_with_manifest(devices, probes, manifest);
    for (dev, ev) in devices.iter().zip(evidence.iter()) {
        if ev.match_kind == ResolverMatch::ManifestVerified && ev.confidence == Confidence::High {
            if let Some(n) = ev.gst_device_number {
                map.insert(
                    dev.device_id,
                    ResolvedDeviceBinding {
                        device_number: n,
                        hw_serial_number: ev.gst_hw_serial_number.clone(),
                        confidence: Confidence::High,
                        match_kind: ResolverMatch::ManifestVerified,
                    },
                );
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{DeviceIdentitySource, IdentityStrength};
    use uuid::Uuid;

    fn dev(handle: &str) -> DeviceInfo {
        DeviceInfo {
            device_id: Uuid::new_v4(),
            model: "DeckLink SDI".to_string(),
            display_name: format!("dv-{handle}"),
            serial_number: None,
            bmd_device_handle: Some(handle.to_string()),
            bmd_persistent_id: None,
            bmd_topological_id: None,
            identity_strength: IdentityStrength::DeviceHandle,
            identity_source: DeviceIdentitySource::RealBmd,
            capabilities: crate::port::DeviceCapabilities::default(),
            video_input_connections: 0,
            video_output_connections: 0,
            ports: Vec::new(),
        }
    }

    fn probe(num: u32, serial: Option<&str>) -> GStreamerDeviceProbe {
        GStreamerDeviceProbe {
            device_number: num,
            hw_serial_number: serial.map(|s| s.to_string()),
            persistent_id: None,
            signal: Some(true),
            model: Some("DeckLink SDI".to_string()),
            caps: None,
        }
    }

    fn manifest_entry(handle: &str, num: u32) -> BindingEntry {
        BindingEntry {
            label: None,
            bmd_device_handle: handle.to_string(),
            gst_device_number: num,
            expected_hw_serial_number: None,
            expected_model: None,
            port: None,
        }
    }

    fn base_manifest(entries: Vec<BindingEntry>) -> DeviceBindingManifest {
        DeviceBindingManifest {
            manifest_version: "1.0".into(),
            machine_id: "box-a".into(),
            generated_by: "ops".into(),
            generated_at: "2026-08-27".into(),
            bmd_sdk_version: None,
            gst_decklink_plugin_version: None,
            gst_runtime_version: None,
            notes: None,
            bindings: entries,
        }
    }

    #[test]
    fn manifest_verified_when_probe_opens_declared_number() {
        let devices = vec![dev("46:00000000:002e4500")];
        let probes = vec![probe(2, None)];
        let manifest = base_manifest(vec![manifest_entry("46:00000000:002e4500", 2)]);
        let ev = resolve_with_manifest(&devices, &probes, &manifest);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].match_kind, ResolverMatch::ManifestVerified);
        assert_eq!(ev[0].confidence, Confidence::High);
        assert_eq!(ev[0].gst_device_number, Some(2));
        let binds = collect_bindings_from_manifest(&devices, &probes, &manifest);
        assert_eq!(binds.len(), 1);
    }

    #[test]
    fn manifest_fails_closed_when_declared_number_not_in_probe() {
        let devices = vec![dev("46:00000000:002e4500")];
        let probes = vec![probe(5, None)]; // 清单声明 2, 探测只有 5
        let manifest = base_manifest(vec![manifest_entry("46:00000000:002e4500", 2)]);
        let ev = resolve_with_manifest(&devices, &probes, &manifest);
        assert_eq!(ev[0].match_kind, ResolverMatch::Unresolved);
        assert_eq!(ev[0].gst_device_number, None);
    }

    #[test]
    fn manifest_fails_closed_on_serial_crosscheck_mismatch() {
        let devices = vec![dev("46:00000000:002e4500")];
        let probes = vec![probe(2, Some("REAL-123"))];
        let mut manifest = base_manifest(vec![manifest_entry("46:00000000:002e4500", 2)]);
        manifest.bindings[0].expected_hw_serial_number = Some("EXPECTED-999".into());
        let ev = resolve_with_manifest(&devices, &probes, &manifest);
        assert_eq!(ev[0].match_kind, ResolverMatch::Unresolved);
    }

    #[test]
    fn manifest_fails_closed_when_device_absent_from_manifest() {
        let devices = vec![dev("46:00000000:002e4500")];
        let probes = vec![probe(2, None)];
        let manifest = base_manifest(vec![]); // 空清单
        let ev = resolve_with_manifest(&devices, &probes, &manifest);
        assert_eq!(ev[0].match_kind, ResolverMatch::Unresolved);
    }

    #[test]
    fn manifest_serial_crosscheck_passes_when_declared() {
        let devices = vec![dev("46:00000000:002e4500")];
        let probes = vec![probe(2, Some("REAL-123"))];
        let mut manifest = base_manifest(vec![manifest_entry("46:00000000:002e4500", 2)]);
        manifest.bindings[0].expected_hw_serial_number = Some("REAL-123".into());
        let ev = resolve_with_manifest(&devices, &probes, &manifest);
        assert_eq!(ev[0].match_kind, ResolverMatch::ManifestVerified);
        assert_eq!(ev[0].gst_device_number, Some(2));
    }

    #[test]
    fn manifest_validate_rejects_duplicate_handle() {
        let m = base_manifest(vec![
            manifest_entry("46:00000000:002e4500", 1),
            manifest_entry("46:00000000:002e4500", 2),
        ]);
        assert!(m.validate_manifest().is_err());
    }

    #[test]
    fn manifest_validate_rejects_duplicate_device_number() {
        let m = base_manifest(vec![
            manifest_entry("46:00000000:002e4500", 1),
            manifest_entry("46:00000000:002e4400", 1),
        ]);
        assert!(m.validate_manifest().is_err());
    }

    #[test]
    fn manifest_validate_rejects_empty_machine_id() {
        let mut m = base_manifest(vec![manifest_entry("46:00000000:002e4500", 1)]);
        m.machine_id = String::new();
        assert!(m.validate_manifest().is_err());
    }

    #[test]
    fn manifest_validate_ok_on_well_formed() {
        let m = base_manifest(vec![manifest_entry("46:00000000:002e4500", 1)]);
        assert!(m.validate_manifest().is_ok());
    }

    #[test]
    fn manifest_check_machine_identity_rejects_mismatch() {
        let m = base_manifest(vec![manifest_entry("46:00000000:002e4500", 1)]);
        // 主机一致 → Ok
        assert!(m.check_machine_identity("box-a").is_ok());
        // 主机不一致 → 失败闭合 (拒绝, 非 warning)
        assert!(m.check_machine_identity("box-b").is_err());
        // 运行环境无法判定主机 (空) → 跳过而非误报
        assert!(m.check_machine_identity("").is_ok());
    }

    // ---- STEP 4: Manifest v2 版本化 + 端口级绑定 fail-closed ----

    #[test]
    fn manifest_v2_requires_port_level_binding() {
        // v2 (major>=2) 清单若仍是 device-level (port: None) → 失败闭合拒绝.
        let mut m = base_manifest(vec![manifest_entry("46:00000000:002e4500", 1)]);
        m.manifest_version = "2".into();
        assert!(m.validate_manifest().is_err());
    }

    #[test]
    fn manifest_v2_with_valid_port_ok() {
        let mut m = base_manifest(vec![BindingEntry {
            port: Some(PortBinding {
                connector: ConnectorType::Sdi,
                ordinal: 1,
                direction: PortDirection::Input,
                required: true,
                verification: VerificationLevel::SignalVerified,
            }),
            ..manifest_entry("46:00000000:002e4500", 1)
        }]);
        m.manifest_version = "2".into();
        assert!(m.validate_manifest().is_ok());
    }

    #[test]
    fn manifest_v2_rejects_unknown_direction() {
        let mut m = base_manifest(vec![BindingEntry {
            port: Some(PortBinding {
                connector: ConnectorType::Sdi,
                ordinal: 1,
                direction: PortDirection::Unknown,
                required: true,
                verification: VerificationLevel::Declared,
            }),
            ..manifest_entry("46:00000000:002e4500", 1)
        }]);
        m.manifest_version = "2".into();
        assert!(m.validate_manifest().is_err());
    }

    #[test]
    fn manifest_v1_device_level_still_ok() {
        // v1 (major<2) 仍允许 device-level 绑定 (向后兼容, port 可空).
        let m = base_manifest(vec![manifest_entry("46:00000000:002e4500", 1)]);
        assert!(m.validate_manifest().is_ok());
    }

    #[test]
    fn manifest_version_unparseable_rejected() {
        let mut m = base_manifest(vec![manifest_entry("46:00000000:002e4500", 1)]);
        m.manifest_version = "v2".into();
        assert!(m.validate_manifest().is_err());
    }
}
