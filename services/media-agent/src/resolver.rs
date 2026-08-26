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

/// GStreamer 探测到的 DeckLink 实例 (只读属性, 运行时采集).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GStreamerDeviceProbe {
    pub device_number: u32,
    /// GStreamer `hw-serial-number` (硬件 ID, 可读写; 本机选卡关键属性).
    pub hw_serial_number: Option<String>,
    /// GStreamer `persistent-id` (若 SDK/GStreamer 支持且硬件有值).
    pub persistent_id: Option<i64>,
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

/// 探测 GStreamer DeckLink 实例 (运行时; 仅 C1 证据/物化前置使用).
///
/// ⚠️ 此函数依赖系统 GStreamer + decklink 插件; 无 GStreamer 环境返回空.
/// 生产物化前必须先成功探测, 否则 `collect_bindings` 为空 → materialize 拒绝.
pub fn probe_gstreamer_devices(max: usize) -> Vec<GStreamerDeviceProbe> {
    // 真机由 `gstreamer` feature 的 decklinkvideosrc 枚举; 此处为基础设施占位.
    // 占位实现: 返回空 (无 GStreamer 环境). 真实实现见 `gstreamer` feature 构建.
    let _ = max;
    Vec::new()
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
        sdk.bmd_device_handle.as_deref().and_then(topo_of),
        &p.hw_serial_number,
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
        let (gst_num, gst_serial, note) = match (matched, kind) {
            (_, ResolverMatch::Ambiguous) => (
                None,
                None,
                "AMBIGUOUS: same SDK device matches >=2 HIGH-confidence GStreamer instances; production MUST reject, never guess".to_string(),
            ),
            (Some(p), _) => (
                Some(p.device_number),
                p.hw_serial_number.clone(),
                format!(
                    "match={:?} via gstreamer device-number={}",
                    kind, p.device_number
                ),
            ),
            (None, _) => (
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
