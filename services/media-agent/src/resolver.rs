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
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    Available(Vec<GStreamerDeviceProbe>),
    /// 探测方法本身不可用 (GStreamer 未初始化 或 `decklinkvideosrc` 工厂缺失 / decklink 插件未安装).
    /// 这是 "方法不可用", **不是** "设备未解析" — 绝不能等同 `Unresolved` 误导现场 (用户复核 §九).
    Unavailable(String),
    /// 探测正常执行但枚举到 0 个 DeckLink 实例 (本机确无可用采集卡). 与 `Unavailable` 完全不同.
    Empty,
}

/// 直接 probe `decklinkvideosrc` 实例, 物化 SDK DeviceHandle → GStreamer `device-number`.
///
/// 为什么不用 `GstDeviceMonitor` (用户复核, 撤回 `b52e2b6`): 当前 DeckLink 官方插件只暴露
/// `decklinkvideosrc` / `decklinkaudiosrc` **element**, 不提供 `GstDeviceProvider`; 实机验证
/// `gst-device-monitor` 不列出 DeckLink. 故枚举入口改成**直接创建 element 实例**, 按 `device-number`
/// 遍历, 打开到 READY (不 PLAYING, 不拉真实帧) 后读取只读属性.
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
pub fn probe_gstreamer_devices(max: usize) -> GstProbeOutcome {
    use gstreamer::prelude::*;
    // GStreamer 初始化 (幂等). 失败 → 探测方法不可用 (非设备失败).
    if let Err(e) = gstreamer::init() {
        return GstProbeOutcome::Unavailable(format!("gstreamer init 失败: {e}"));
    }
    // decklinkvideosrc 工厂存在 = decklink 插件安装; 否则探测方法不适用本机.
    if gstreamer::ElementFactory::find("decklinkvideosrc").is_none() {
        return GstProbeOutcome::Unavailable(
            "decklinkvideosrc 工厂不存在 (GStreamer decklink 插件未安装); 探测方法不可用".to_string(),
        );
    }
    let mut probes = Vec::new();
    for n in 0..(max as u32) {
        if let Some(p) = probe_one_device_number(n) {
            probes.push(p);
        }
    }
    if probes.is_empty() {
        return GstProbeOutcome::Empty;
    }
    GstProbeOutcome::Available(probes)
}

/// 探测单个 `device-number` 的 decklinkvideosrc 实例. 打开到 READY 读只读属性; 不存在/打不开返回 `None`
/// (视为该序号无可用采集卡, 不计入 probes, 避免 ghost 设备导致 `Ambiguous`).
#[cfg(feature = "gstreamer")]
fn probe_one_device_number(n: u32) -> Option<GStreamerDeviceProbe> {
    use gstreamer::prelude::*;
    let el = gstreamer::ElementFactory::make("decklinkvideosrc")
        .build()
        .ok()?;
    // 以 device-number 绑定目标采集卡 (GStreamer 运行时地址). 只读属性前提是设备已打开 (READY).
    el.try_set_property("device-number", n as i32)?;
    // 打开设备 (READY 即打开; 无需 PLAYING, 不拉真实帧). 失败 = 该序号无此卡.
    if el.set_state(gstreamer::State::Ready).is_err() {
        let _ = el.set_state(gstreamer::State::Null);
        return None;
    }
    // 读取只读属性 (hw-serial-number 等设备在 READY 后才可靠填充). 各属性用 `find_property` 守卫:
    // 本机 GStreamer 1.28.2 decklink 插件的 `gst-inspect` 显示**只有 `device-number` 与 `hw-serial-number`
    // 两个选卡属性**, 没有 `persistent-id` (且 `signal`/`model` 因版本而异). 缺属性直接读取会 panic,
    // 故先确认存在再按类型读 (String/i64/bool 均实现 HasParamSpec, 编译稳定).
    let hw_serial_number = el
        .find_property("hw-serial-number")
        .and_then(|_| non_empty(el.property::<String>("hw-serial-number")));
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
        .and_then(|_| Some(el.property::<bool>("signal")));
    let model = el
        .find_property("model")
        .and_then(|_| non_empty(el.property::<String>("model")));
    // 释放设备.
    let _ = el.set_state(gstreamer::State::Null);
    // ghost 判定: 无任何身份线索说明该序号并非真实采集卡, 不计入 (防误判 Ambiguous).
    if hw_serial_number.is_none() && model.is_none() && persistent_id.is_none() {
        return None;
    }
    Some(GStreamerDeviceProbe {
        device_number: n,
        hw_serial_number,
        persistent_id,
        signal,
        model,
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

#[cfg(not(feature = "gstreamer"))]
pub fn probe_gstreamer_devices(_max: usize) -> GstProbeOutcome {
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
fn best_kind_for(sdk: &DeviceInfo, p: &GStreamerDeviceProbe) -> Option<(ResolverMatch, Confidence)> {
    // 1) PersistentID 精确 (HIGH)
    if let (Some(sdk_pid), Some(gst_pid)) = (sdk.bmd_persistent_id, p.persistent_id) {
        if sdk_pid as i64 == gst_pid {
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
pub fn resolve(
    devices: &[DeviceInfo],
    probes: &[GStreamerDeviceProbe],
) -> Vec<ResolverEvidence> {
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
