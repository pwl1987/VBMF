//! A2-0: HW-PORT-01D Loopback 真机验收闭环（VBMF_LOOPBACK 入口）。
//!
//! 自 main.rs 逐字节迁出（env 命中即 exit(0)——语义不变）。
//! 加载 DeviceBindingManifest → 探测 GStreamer → 解析绑定 → 构建 PortRegistry →
//! 加载 fixtures 目录 → 对每条 Fixture 在 source 渲染已知图案、在 sink 真实采集
//! (含加嵌音频探测) → verify_fixtures 双门 → 输出 FixtureVerification JSON。
//!
//! **Gate-local diagnostic construction（用户 2026-09-02 裁定显式标记）**:
//! 本入口自建 PortRegistry/bindings（诊断专用重建, **非** production runtime
//! 初始化路径）; event_sink/projection_log 为调用方传入的共享实例。
//! A2-0 不合并 production runtime——Gate = 调用 Production 组件做验收。

#[cfg(feature = "gstreamer-backend")]
use std::sync::Arc;

#[cfg(feature = "gstreamer-backend")]
use crate::config::Config;
#[cfg(feature = "gstreamer-backend")]
use crate::contracts::provider::DiscoveredDevice;
#[cfg(feature = "gstreamer-backend")]
use crate::events::{RuntimeEventLog, RuntimeEventSink};

#[cfg(feature = "gstreamer-backend")]
pub fn run(
    _cfg: &Config,
    discovered: &[DiscoveredDevice],
    event_sink: &Arc<dyn RuntimeEventSink>,
    projection_log: &Arc<RuntimeEventLog>,
) {
    if std::env::var("VBMF_LOOPBACK").is_ok() {
        let manifest_path = match &_cfg.device_binding_path {
            Some(p) => p.clone(),
            None => {
                eprintln!("VBMF_LOOPBACK 需要 DeviceBindingManifest (MEDIA_AGENT_DEVICE_BINDING)");
                std::process::exit(2);
            }
        };
        let manifest = match crate::resolver::DeviceBindingManifest::load(&manifest_path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("DeviceBindingManifest 加载失败: {e}");
                std::process::exit(2);
            }
        };
        if let Err(e) = manifest.validate_manifest() {
            eprintln!("DeviceBindingManifest 结构校验失败: {e}");
            std::process::exit(2);
        }
        let probes = match crate::resolver::probe_gstreamer_devices(
            crate::resolver::MAX_PROBE_DEVICES,
            false,
        ) {
            crate::resolver::GstProbeOutcome::Available { probes, errors } => {
                for (n, e) in &errors {
                    tracing::warn!(device_number = n, error = %e, "GStreamer 单设备探测失败");
                }
                probes
            }
            _ => Vec::new(),
        };
        let bindings =
            crate::resolver::collect_bindings_from_manifest(discovered, &probes, &manifest);
        let registry =
            match crate::port::PortRegistry::build(discovered, &probes, &manifest, &bindings) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("PortRegistry 构建失败 (fail-closed): {e:?}");
                    std::process::exit(2);
                }
            };
        let fixtures_dir = std::env::var("VBMF_FIXTURES_DIR")
            .unwrap_or_else(|_| "evidence/bmd-10.30.15.10/fixtures".to_string());
        let fixtures = match crate::fixture::Fixture::load_dir(std::path::Path::new(&fixtures_dir))
        {
            Ok(f) if !f.is_empty() => f,
            Ok(_) => {
                eprintln!("fixtures 目录为空: {fixtures_dir}");
                std::process::exit(2);
            }
            Err(e) => {
                eprintln!("fixtures 加载失败 ({fixtures_dir}): {e}");
                std::process::exit(2);
            }
        };
        let sample_frames = std::env::var("VBMF_LOOPBACK_FRAMES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(8);
        let verifications = crate::signal::verify_fixtures(&fixtures, |f| {
            crate::signal::probe_fixture_signal(f, &registry, sample_frames)
        });
        match serde_json::to_string_pretty(&verifications) {
            Ok(json) => {
                println!("=== HW-PORT-01D Loopback Verification (真机闭环) ===");
                println!("{json}");
                let all_pass = verifications.iter().all(|v| v.passed);
                println!("=== LOOPBACK ALL PASS = {all_pass} ===");
                // P0.7B-1 NORMALIZE-RT-01 (Hardware 层): 渲染已建立信号 → 新鲜探测重建
                // registry → 真机观测装配 → CanonicalMediaDescriptor 证据输出
                // (judge-only; 纪律① — descriptor 不进任何 pipeline)。
                if all_pass {
                    // P0-7D-2.1: LoopbackVerified 点亮 (词表在册, 原生产) — loopback 验收
                    // 双门 (信号态+内容) 通过即语义时刻。fixture 级验证: 设备归属未在
                    // FixtureVerification 携带 → nil=未归属 (观测记账, 不改 reducer 主态)。
                    for _ in verifications.iter().filter(|v| v.passed) {
                        event_sink.emit(crate::events::RuntimeEvent::LoopbackVerified {
                            device_id: uuid::Uuid::nil(),
                            port_id: None,
                        });
                    }
                    // P0-7D-4.3 (E4, 方案 A): 投影端计数闭环 — 每条通过 verification 恰好
                    // 发一条, FanoutSink 双写经投影 drain 可精确计数 (loopback 为独立入口,
                    // 本段日志仅含本段事件; 计数失配 = 生产接线缺陷, fail-closed)。
                    let drained_lb = projection_log.drain();
                    let p_lb = crate::event_projection::project(&drained_lb);
                    let loopback_count = p_lb
                        .kind_counts
                        .get("loopback_verified")
                        .copied()
                        .unwrap_or(0);
                    println!(
                        "EVENT-INTEGRATION-RT-01 E4 loopback_verified={loopback_count} (期望 {})",
                        verifications.len()
                    );
                    if loopback_count != verifications.len() {
                        eprintln!("EVENT-INTEGRATION-RT-01 E4 verdict=FAIL (计数失配)");
                        std::process::exit(2);
                    }
                    let fresh_probes = match crate::resolver::probe_gstreamer_devices(
                        crate::resolver::MAX_PROBE_DEVICES,
                        false,
                    ) {
                        crate::resolver::GstProbeOutcome::Available { probes, .. } => probes,
                        _ => Vec::new(),
                    };
                    let fresh_registry = crate::port::PortRegistry::build(
                        discovered,
                        &fresh_probes,
                        &manifest,
                        &bindings,
                    );
                    let fresh_registry = match fresh_registry {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("NORMALIZE-RT-01: registry 重建失败 (fail-closed): {e:?}");
                            std::process::exit(2);
                        }
                    };
                    if let Some(input_port) = fresh_registry
                        .ports
                        .iter()
                        .find(|p| p.direction == crate::port::PortDirection::Input)
                    {
                        let raw = crate::normalize::RawInputDescription::from_port(input_port);
                        let outcome = crate::normalize::normalize_input(&raw);
                        match serde_json::to_string_pretty(&outcome.descriptor) {
                            Ok(json) => {
                                println!(
                                    "=== NORMALIZE-RT-01 Canonical Descriptor (真机观测, 渲染中) ==="
                                );
                                println!("{json}");
                                println!(
                                    "=== NORMALIZE-RT-01 diagnostics = {:?} ===",
                                    outcome
                                        .diagnostics
                                        .iter()
                                        .map(|d| (d.level, d.code.as_str()))
                                        .collect::<Vec<_>>()
                                );
                            }
                            Err(e) => eprintln!("descriptor 序列化失败: {e}"),
                        }
                        // P0.7B-2A MEDIA-SEMANTICS-RT-01 (Clock 部分, Hardware 层):
                        // 真机装配 CanonicalClockDomain — 当前无 clock 探针 →
                        // Unknown 组合 + evidence (终审明确: Unknown 合法; Observation≠Configuration)。
                        let clock_domain =
                            crate::clock::CanonicalClockDomain::unknown(uuid::Uuid::nil());
                        match serde_json::to_string_pretty(&clock_domain) {
                            Ok(json) => {
                                println!(
                                    "=== MEDIA-SEMANTICS-RT-01 Canonical Clock Domain (真机装配) ==="
                                );
                                println!("{json}");
                            }
                            Err(e) => eprintln!("clock domain 序列化失败: {e}"),
                        }
                        // P0.7B-2B MEDIA-SEMANTICS-RT-01 (Audio 部分, Hardware 层):
                        // Normalize → CanonicalAudioStream 证据输出 (role=Unknown 合法;
                        // 回答"是什么", 不产 pipeline)。
                        let audio_stream = crate::audio::CanonicalAudioStream::from_description(
                            crate::audio::AudioStreamId(uuid::Uuid::nil()),
                            &outcome.descriptor.audio,
                        );
                        match serde_json::to_string_pretty(&audio_stream) {
                            Ok(json) => {
                                println!(
                                    "=== MEDIA-SEMANTICS-RT-01 Canonical Audio Stream (真机装配) ==="
                                );
                                println!("{json}");
                            }
                            Err(e) => eprintln!("audio stream 序列化失败: {e}"),
                        }
                        // P0.7B-2C TIMECODE-SEMANTICS-RT-01 (Hardware 层): 真机装配
                        // CanonicalTimecode — 无 timecode 探针 → Unknown + evidence
                        // (Unknown 合法; 只证明"能观察/描述", 不证明"能解析全部格式")。
                        let tc = crate::timecode::CanonicalTimecode::unknown();
                        match serde_json::to_string_pretty(&tc) {
                            Ok(json) => {
                                println!(
                                    "=== TIMECODE-SEMANTICS-RT-01 Canonical Timecode (真机装配) ==="
                                );
                                println!("{json}");
                            }
                            Err(e) => eprintln!("timecode 序列化失败: {e}"),
                        }
                    }
                }
            }
            Err(e) => eprintln!("loopback verification 序列化失败: {e}"),
        }
        std::process::exit(0);
    }
}
