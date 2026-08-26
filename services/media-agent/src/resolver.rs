//! # DeckLinkDeviceResolver (C1)
//!
//! **职责边界 (V0.2/0.5/0.6 已焊死)**:
//! - `DeviceRegistry` (SDK) 产出 **硬件身份** (`DeviceHandle` / `PersistentID`)。
//! - `Resolver` 把硬件身份物化为 **GStreamer 运行时地址** (`device-number`)。
//! - `PipelinePlan` 只消费 `device-number`, 绝不自己重新猜设备。
//!
//! 这两者是**不同层次的对象**: "设备身份" 跨进程/重启稳定, "运行时地址" 随 GStreamer
//! 枚举顺序变化 (Duplex/connector 映射会改变 `device-number`, 且与 SDK 枚举序号无关 —
//! 已在 10.30.15.10 实机证明)。因此 SDK `device_number` 字段**绝不能直接**当 GStreamer
//! `device-number` 使用, 必须经本 Resolver 解析。
//!
//! ## 匹配策略 (优先级 + confidence, 杜绝静默回退)
//! 1. `PersistentID` 精确匹配 → `PersistentIdExact` (HIGH) —— Blackmagic 官方最高优先级。
//! 2. `serial_number` 精确匹配 GStreamer `hw-serial-number` → `SerialExact` (HIGH)。
//! 3. `DeviceHandle` 精确匹配 GStreamer `hw-serial-number` → `DeviceHandleExact` (HIGH)。
//! 4. `TopologicalID` (DeviceHandle 末段) 匹配 → `TopologicalIdGuess` (MEDIUM, 拓扑敏感, 仅诊断)。
//! 5. 以上皆无 → `Unresolved` (生产路径**必须拒绝**, 绝不回退 `device-number=0`)。
//!
//! 当前 10.30.15.10 / SDK 16.0 三台设备 `PersistentID`/`TopologicalID` 均不支持,
//! `DeviceHandle` 是唯一可用身份; Resolver 须运行于真机, 通过 GStreamer `hw-serial-number`
//! 探针动态探得取值后再与 `DeviceHandle` 交叉匹配 (证据见 `ResolverEvidence`)。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::device::{DeviceInfo, IdentityStrength};

/// GStreamer 单实例探测快照 (C1 运行时经 `decklinkvideosrc` 动态探得, 非硬编码)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GStreamerDeviceProbe {
    pub device_number: u32,
    /// GStreamer `hw-serial-number` (="The serial number (hardware ID) of the Decklink card")。
    /// 与 SDK `DeviceHandle` 的关系**必须运行时探明**, 本结构只忠实记录。
    pub hw_serial_number: Option<String>,
    /// GStreamer `persistent-id` (此硬件恒 0/不支持)。
    pub persistent_id: Option<i64>,
    /// GStreamer `signal` (只读: 当前是否有有效输入信号)。身份与信号是**两个维度**, 不混淆。
    pub signal: Option<bool>,
}

/// Resolver 物化出的确定性绑定 (喂给 GStreamer `decklinkvideosrc`/`decklinkaudiosrc` 的运行时地址)。
///
/// `decklinkvideosrc` 与 `decklinkaudiosrc` **必须共用同一 `device_number`** → 保证 A/V 同一硬件实例。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDeviceBinding {
    pub device_number: u32,
    pub hw_serial_number: Option<String>,
    pub model: Option<String>,
    /// VBMF 输入契约 (本项目 = SDI capture), 不是 GStreamer 自动探测。
    pub connection: Option<String>,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
    None,
}

/// Resolver 实际使用的匹配方式 (写入证据, 杜绝"静默回退到 device-number=0")。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResolverMatch {
    PersistentIdExact,
    SerialExact,
    DeviceHandleExact,
    TopologicalIdGuess,
    Unresolved,
}

/// 单台 SDK 设备的解析证据。**C1 必须完整产出**, 供现场核对
/// "CH01 怎么采到了另一张卡" / "device-number 与正确输入设备未对应" 等问题。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverEvidence {
    pub vbmf_device_id: String,
    pub bmd_device_handle: Option<String>,
    pub model: String,
    pub sdk_serial_number: Option<String>,
    pub identity_strength: IdentityStrength,
    pub gstreamer_device_number: Option<u32>,
    pub gstreamer_hw_serial_number: Option<String>,
    pub match_kind: ResolverMatch,
    pub confidence: Confidence,
    pub note: String,
}

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("identity unresolved for device {0} ({1}): {2}")]
    IdentityUnresolved(String, String, String),
}

/// DeviceHandle 末段 (TopologicalID) 候选匹配键。
fn topo_of(handle: &str) -> Option<&str> {
    handle.rsplit(':').next().filter(|s| !s.is_empty())
}

/// 为单台 SDK 设备挑选最佳 GStreamer 探针匹配。
fn find_match<'a>(
    sdk: &DeviceInfo,
    probes: &'a [GStreamerDeviceProbe],
) -> (ResolverMatch, Confidence, Option<&'a GStreamerDeviceProbe>) {
    let mut best: Option<(ResolverMatch, Confidence, usize)> = None;
    let mut consider = |kind: ResolverMatch, conf: Confidence, idx: usize| {
        let rank = match kind {
            ResolverMatch::PersistentIdExact => 4,
            ResolverMatch::SerialExact => 3,
            ResolverMatch::DeviceHandleExact => 2,
            ResolverMatch::TopologicalIdGuess => 1,
            ResolverMatch::Unresolved => 0,
        };
        if let Some((_, best_rank, _)) = best {
            if rank > best_rank {
                best = Some((kind, conf, idx));
            }
        } else {
            best = Some((kind, conf, idx));
        }
    };

    for (idx, p) in probes.iter().enumerate() {
        // 1) PersistentID 精确
        if let (Some(sdk_pid), Some(gst_pid)) = (sdk.bmd_persistent_id, p.persistent_id) {
            if sdk_pid as i64 == gst_pid {
                consider(ResolverMatch::PersistentIdExact, Confidence::High, idx);
                continue;
            }
        }
        // 2) serial_number 精确
        if let (Some(s), Some(g)) = (&sdk.serial_number, &p.hw_serial_number) {
            if s == g {
                consider(ResolverMatch::SerialExact, Confidence::High, idx);
                continue;
            }
        }
        // 3) DeviceHandle 精确 (GStreamer `hw-serial-number` 恰为该字符串)
        if let (Some(h), Some(g)) = (&sdk.bmd_device_handle, &p.hw_serial_number) {
            if h == g {
                consider(ResolverMatch::DeviceHandleExact, Confidence::High, idx);
                continue;
            }
        }
        // 4) TopologicalID 末段猜测 (拓扑敏感, 仅 MEDIUM)
        if let (Some(topo), Some(g)) = (sdk.bmd_device_handle.as_deref().and_then(topo_of), &p.hw_serial_number) {
            if topo == g {
                consider(ResolverMatch::TopologicalIdGuess, Confidence::Medium, idx);
            }
        }
    }

    match best {
        Some((kind, conf, idx)) => (kind, conf, Some(&probes[idx])),
        None => (ResolverMatch::Unresolved, Confidence::None, None),
    }
}

/// C1 主入口: 对每台 SDK 设备产出 `ResolverEvidence` (完整映射, 供核对; 不启动 pipeline)。
pub fn resolve(sdk: &[DeviceInfo], probes: &[GStreamerDeviceProbe]) -> Vec<ResolverEvidence> {
    sdk.iter()
        .map(|dev| {
            let (kind, conf, matched) = find_match(dev, probes);
            let (gst_num, gst_serial, note) = match matched {
                Some(p) => (
                    Some(p.device_number),
                    p.hw_serial_number.clone(),
                    format!("match={:?} via gstreamer device-number={}", kind, p.device_number),
                ),
                None => (
                    None,
                    None,
                    format!("no exact match (match={:?}); production MUST reject, never fallback to device-number=0", kind),
                ),
            };
            ResolverEvidence {
                vbmf_device_id: dev.device_id.to_string(),
                bmd_device_handle: dev.bmd_device_handle.clone(),
                model: dev.model.clone(),
                sdk_serial_number: dev.serial_number.clone(),
                identity_strength: dev.identity_strength,
                gstreamer_device_number: gst_num,
                gstreamer_hw_serial_number: gst_serial,
                match_kind: kind,
                confidence: conf,
                note,
            }
        })
        .collect()
}

/// 收集所有**已解析** (非 Unresolved) 设备的绑定, 供 `materialize` 物化使用。
/// 不报错: 未解析设备直接不出现在 map 中 (生产路径遇到缺失即拒绝)。
pub fn collect_bindings(
    sdk: &[DeviceInfo],
    probes: &[GStreamerDeviceProbe],
) -> HashMap<Uuid, ResolvedDeviceBinding> {
    let mut map = HashMap::new();
    for dev in sdk {
        let (kind, _conf, matched) = find_match(dev, probes);
        let Some(p) = matched else { continue };
        if kind == ResolverMatch::Unresolved {
            continue;
        }
        let confidence = match kind {
            ResolverMatch::PersistentIdExact
            | ResolverMatch::SerialExact
            | ResolverMatch::DeviceHandleExact => Confidence::High,
            ResolverMatch::TopologicalIdGuess => Confidence::Medium,
            ResolverMatch::Unresolved => Confidence::None,
        };
        map.insert(
            dev.device_id,
            ResolvedDeviceBinding {
                device_number: p.device_number,
                hw_serial_number: p.hw_serial_number.clone(),
                model: Some(dev.model.clone()),
                connection: Some("sdi".to_string()), // VBMF 输入契约 (本项目 = SDI)
                confidence,
            },
        );
    }
    map
}

/// 生产路径严格解析: 对 `required` 中每个 device_id 都必须有 HIGH/MEDIUM 绑定,
/// 任一缺失/未解析 → `IdentityUnresolved` (整个生产 pipeline 不启动)。
pub fn resolve_strict(
    sdk: &[DeviceInfo],
    probes: &[GStreamerDeviceProbe],
    required: &[Uuid],
) -> Result<HashMap<Uuid, ResolvedDeviceBinding>, ResolverError> {
    let bindings = collect_bindings(sdk, probes);
    for id in required {
        match bindings.get(id) {
            Some(b) if b.confidence != Confidence::None => {}
            _ => {
                let dev = sdk.iter().find(|d| &d.device_id == id);
                let handle = dev.and_then(|d| d.bmd_device_handle.clone()).unwrap_or_default();
                let model = dev.map(|d| d.model.clone()).unwrap_or_default();
                return Err(ResolverError::IdentityUnresolved(
                    id.to_string(),
                    handle,
                    format!("model={model}: no exact GStreamer match found; refusing production binding"),
                ));
            }
        }
    }
    Ok(bindings)
}

/// 默认探测上界 (GStreamer device-number 从 0 递增; 越界设备 hw-serial-number 为空则停止)。
pub const MAX_PROBE_DEVICES: u32 = 16;

/// GStreamer 运行时探针: 枚举 `device-number = 0..MAX`, 为每个实例读取
/// `hw-serial-number` / `persistent-id` / `signal`。仅在 `gstreamer` feature 下可用 (需真机)。
///
/// 注意: 须在 READY 状态才能读到硬件属性; 读取后回到 NULL。属性缺失/类型不符用
/// `try_property` 兜底为 `None`, 不 panic。真实属性取值由真机运行决定, 本函数只忠实采集。
#[cfg(feature = "gstreamer")]
pub fn probe_gstreamer_devices(max_devices: u32) -> Vec<GStreamerDeviceProbe> {
    use gstreamer::prelude::*;
    let _ = gstreamer::init();
    let mut out = Vec::new();
    for n in 0..max_devices {
        let elem = match gstreamer::ElementFactory::make("decklinkvideosrc").build() {
            Ok(e) => e,
            Err(_) => break,
        };
        elem.set_property("device-number", n);
        // 进入 READY 才能读到硬件属性; 失败说明该 device-number 无效, 停止探测。
        if elem.set_state(gstreamer::State::Ready).is_err() {
            let _ = elem.set_state(gstreamer::State::Null);
            break;
        }
        let hw_serial_number = elem
            .try_property::<String>("hw-serial-number")
            .ok()
            .filter(|s| !s.is_empty());
        let persistent_id = elem.try_property::<i64>("persistent-id").unwrap_or(0);
        let signal = elem.try_property::<bool>("signal").unwrap_or(false);
        let _ = elem.set_state(gstreamer::State::Null);
        out.push(GStreamerDeviceProbe {
            device_number: n,
            hw_serial_number,
            persistent_id: if persistent_id == 0 { None } else { Some(persistent_id) },
            signal: Some(signal),
        });
    }
    out
}
