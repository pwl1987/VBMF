//! A2-0: C1 Resolver 证据探针（VBMF_RESOLVER 入口, 含 HW-PORT-01 报告）。
//!
//! 自 main.rs 逐字节迁出（env 命中即 exit(0), 绝不进入媒体 launch——语义不变）。
//! C1: DeckLinkDeviceResolver —— DeviceHandle → GStreamer device-number 物化
//! (仅解析+证据, 不启动 pipeline). 输出每台 SDK 设备与 GStreamer 实例的交叉映射
//! 证据, 供现场核对 "CH01 怎么采到了另一张卡" / "device-number 与正确输入设备未对应"。
//! 与 CAP-01 生产路径严格隔离。

use crate::config::Config;
use crate::contracts::provider::DiscoveredDevice;

#[cfg(feature = "gstreamer-backend")]
pub fn run(_cfg: &Config, discovered: &[DiscoveredDevice]) {
    if std::env::var("VBMF_RESOLVER").is_ok() {
        let outcome = crate::resolver::probe_gstreamer_devices(
            crate::resolver::MAX_PROBE_DEVICES,
            _cfg.device_binding_path.is_none(),
        );
        match outcome {
            crate::resolver::GstProbeOutcome::Available { probes, errors } => {
                // 原始 GStreamer 枚举 (device-number / hw-serial-number / persistent-id / signal) —
                // 现场直接比对 SDK DeviceHandle / serial 是否等于 GStreamer hw-serial-number (C 设计待证关系).
                match serde_json::to_string_pretty(&probes) {
                    Ok(json) => {
                        println!("=== C1 Raw GStreamer Probes (decklinkvideosrc 直接探测: device-number / hw-serial-number / signal) ===");
                        println!("{json}");
                    }
                    Err(e) => eprintln!("gstreamer probes 序列化失败: {e}"),
                }
                // 各 device-number 的分类失败原因: 不再全压成 None 让现场误判为 "无此卡"
                // (用户 §⑥ / §九 P1). 例如 Busy=被别进程占用, NotFound=该序号无卡, OpenFailed=插件/运行时问题.
                if !errors.is_empty() {
                    println!("=== C1 Probe Errors (device-number -> 分类失败原因) ===");
                    for (n, e) in &errors {
                        println!("  device-number {n}: {e}");
                    }
                }
                // 解析: 若提供 DeviceBindingManifest (MEDIA_AGENT_DEVICE_BINDING) 走权威路径,
                // 否则回退 legacy runtime auto-resolver (生产应禁用, 用户 §11/§12).
                let manifest = _cfg.device_binding_path.as_deref().and_then(|p| {
                    match crate::resolver::DeviceBindingManifest::load(p) {
                        Ok(m) => {
                            // 诊断模式: 结构/主机校验仅报告, 不阻断证据输出.
                            if let Err(e) = m.validate_manifest() {
                                eprintln!(
                                    "DeviceBindingManifest 结构校验失败 (诊断模式仍输出证据): {e}"
                                );
                            }
                            if let Err(e) =
                                m.check_machine_identity(&crate::resolver::current_machine_id())
                            {
                                eprintln!(
                                    "DeviceBindingManifest 主机身份不符 (诊断模式仍输出证据): {e}"
                                );
                            } else {
                                // P1-2: 传入声明式 SDK 版本 (env) + 真实 GStreamer/decklink 版本, 真正校验.
                                let sdk_v = crate::resolver::declared_bmd_sdk_version();
                                let gst_v = crate::resolver::actual_gstreamer_version();
                                let plugin_v = crate::resolver::actual_decklink_plugin_version()
                                    .unwrap_or_else(|| "unknown".to_string());
                                // P1-1: 真实运行时 SDK 身份 (build include + libDeckLinkAPI.so) 作为 provenance 输出.
                                eprintln!(
                                    "BMD SDK runtime identity (provenance): {}",
                                    crate::resolver::detected_bmd_sdk_version()
                                );
                                for w in m.validate_environment(
                                    Some(&sdk_v),
                                    Some(&plugin_v),
                                    Some(&gst_v),
                                ) {
                                    eprintln!("device-binding manifest 版本校验: {w}");
                                }
                            }
                            Some(m)
                        }
                        Err(e) => {
                            eprintln!("DeviceBindingManifest 加载失败: {e}");
                            None
                        }
                    }
                });
                if manifest.is_none() {
                    eprintln!("WARNING: 未提供 DeviceBindingManifest, 回退 legacy runtime auto-resolution (C1 诊断模式允许; 生产应禁用)");
                }
                let evidence = match &manifest {
                    Some(m) => crate::resolver::resolve_with_manifest(&discovered, &probes, m),
                    None => crate::resolver::resolve(&discovered, &probes),
                };
                match serde_json::to_string_pretty(&evidence) {
                    Ok(json) => {
                        println!("=== C1 Resolver Evidence: VBMF DeviceHandle/serial → GStreamer device-number ===");
                        println!("{json}");
                    }
                    Err(e) => eprintln!("resolver evidence 序列化失败: {e}"),
                }
                let bindings = match &manifest {
                    Some(m) => {
                        crate::resolver::collect_bindings_from_manifest(&discovered, &probes, m)
                    }
                    None => crate::resolver::collect_bindings(&discovered, &probes),
                };
                match serde_json::to_string_pretty(&bindings) {
                    Ok(json) => {
                        println!("=== C1 Resolved Bindings (喂给 decklinkvideosrc/audiosrc 的 device-number) ===");
                        println!("{json}");
                    }
                    Err(e) => eprintln!("bindings 序列化失败: {e}"),
                }

                // HW-PORT-01 (用户下一阶段实施任务): 端口级绑定闭环验收.
                // Manifest 声明 + 实时 probe → PortRegistry → 验证
                // DeviceHandle → Device → Port → Direction → Connector → Gst address → Signal 闭环.
                // 绝不把当前 dn0/dn1/dn2 语义写死; 端口完全来自 Manifest + 运行时发现.
                if let Some(m) = &manifest {
                    let registry =
                        crate::port::PortRegistry::build(&discovered, &probes, m, &bindings)
                            .expect("端口发现与 manifest 不一致 (fail-closed 拒绝)");
                    let report = crate::hw_port_01::verify(&registry, m);
                    match serde_json::to_string_pretty(&report) {
                        Ok(json) => {
                            println!("=== C1: Runtime Port Binding Verification (HW-PORT-01) ===");
                            println!("{json}");
                            println!("=== HW-PORT-01 PASS = {} ===", report.pass);
                        }
                        Err(e) => eprintln!("HW-PORT-01 报告序列化失败: {e}"),
                    }
                }
            }
            crate::resolver::GstProbeOutcome::Unavailable(reason) => {
                // 探测方法不适用 (GStreamer/decklink 插件不可用) — 与 "设备未解析" 严格区分 (用户复核 §九).
                println!("=== C1 Probe UNAVAILABLE: 探测方法不适用本机 (非设备未解析, 不等同 Unresolved) ===");
                println!("{reason}");
            }
            crate::resolver::GstProbeOutcome::Empty => {
                println!("=== C1 Probe EMPTY: 探测正常执行但枚举到 0 个 DeckLink 实例 (本机无可用采集卡) ===");
            }
        }
        std::process::exit(0);
    }
}
