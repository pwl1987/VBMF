//! VBMF Rust Media Agent — Gate 2 skeleton + Gate 5/6/7 scaffolding.
//!
//! Boundary (SoT §14): this binary owns the Hardware Plane only.
//! Control Plane (API/auth/RBAC/config/UI) stays in Node/Fastify.

mod adapters;
mod api_boundary; // P0.7C-7: External API Foundation (API Boundary Model + Idempotency 契约; 非 Web Server)
mod audio; // P0.7B-2B: Canonical Audio Semantics (是什么, 非怎么处理)
mod clock; // P0.7B-2A: Canonical Clock Domain (只描述观测, 绝不决策; #147)
mod command; // P0.7C-3: Command Contract (请求语义非执行计划; 不可执行性三重守护)
mod config;
mod contracts;
mod device;
mod error_model; // P0.7C-5: Error Model (失败归因分类平面; 三平面分离 CommandStatus≠IdempotentDispatch≠ErrorClassification)
mod event_projection; // P0.7C-6: Event Projection Foundation (Runtime→Event→Projection 生产边; 四语义零偷改)
mod events; // 0.6D: RuntimeEvent canonical 事件契约 + 归一化映射 + 有界事件日志
mod fixture; // HW-PORT-01 / MEDIA-RT-01 复用的 BMD-SDI-LOOPBACK Fixture (host-specific 证据)
mod graph_intent;
mod health;
mod hw_port_01; // HW-PORT-01 Gate: 端口级绑定闭环验收
mod idempotency; // P0.7C-4: Idempotency (D9-A~E: 同一命令语义 + 原子 claim + replay/conflict)
mod lease;
mod normalize; // P0.7B-1: Normalize Foundation — Raw → CanonicalMediaDescriptor (纯函数; 纪律①②③)
mod pipeline;
mod pipeline_events; // C7: 中性共享事件/健康类型模块 (不依赖 gstreamer crate)
mod port; // 五层模型: Device → Port → Capability → Runtime Binding → Signal
mod preflight; // P0-7A: Preflight 分级判定 (judge-only; V0.2 §1.2)
mod registry;
mod resolver;
mod resource; // 0.6E: Resource 模型 + 状态机 + Preflight 闸门 (防自动 Fallback)
mod rpc;
mod runtime_query; // P0.7C-2: Runtime Query Model (Pure Read / Snapshot Semantics; 命令动词禁入)
mod runtime_state; // P0.7C-F: Canonical Runtime State 聚合 (组合非展开; 第一条 Canonical→Runtime 生产边)
mod session;
mod timecode; // P0.7B-2C: Canonical Timecode (时间标签, 非时间本体; #148) // P0-7A: MediaSession + SessionManager (RUNTIME_SESSION_MODEL 唯一 owner)
mod transport; // P0.7C-8: Transport 实现 (API Boundary Model → wire 序列化边界; std-only 五端点)

mod signal; // 信号探测 + 亮度黑场检测
mod supervisor; // Gate 6/7: real DeckLink enumeration (feature `bmd-provider`)

// 硬规则 (Phase 0.6): `hardware-test` (IDeckLinkInput SDK 探针) 与 canonical `gstreamer`
// 运行时互斥 —— 生产运行不得同时打开同一块 DeckLink (避免双采 / 设备争用). 编译期强制.
#[cfg(all(feature = "hardware-test", feature = "gstreamer-backend"))]
compile_error!("hardware-test SDK 探针与 canonical GStreamer 运行时互斥; 生产运行不得同时启用 (避免双采/争用同一块 DeckLink)");

// Trait must be in scope to call `discover()` (trait method, not inherent).
use crate::contracts::provider::HardwareProvider;
use crate::device::DeviceInfo;
// Trait must be in scope to call `acquire`/`is_valid` on `Arc<InMemoryLeaseManager>`
// (trait method, auto-deref via Arc; 否则 E0599 no method named `acquire`).
use lease::LeaseManager;
// Trait must be in scope to call `prepare`/`start`/`recover` on `Arc<dyn MediaBackend>`
// (trait 方法, 否则 E0599 no method named `recover`). C2c 经 `dyn MediaBackend` 接线; 调用点均在
// `#[cfg(feature = "bmd-provider")]` 块内 → bmd && gstreamer 才编译.
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::backend::MediaBackend;
use std::sync::Arc;
// Uuid 使用点 (selftest/canonical watchdog/P0-7A SessionManager auto-start) 全部位于
// bmd && gstreamer 块内; hardware-test (bmd, 无 gst) 路径已无直接 Uuid 使用
// (旧内联 preflight 已被 SessionManager 取代)。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use uuid::Uuid;

// MediaBackend 构造已收口至 `registry::AdapterRegistry::build_media_backend` (C5)。

fn main() {
    tracing_subscriber::fmt::init();

    // Gate 2.1: load config shape from env (no behavior attached yet).
    let _cfg = config::Config::from_env();

    // P1-2 (用户 §二十二): RPC 绑定安全校验 — 即便 RPC transport 尚未实现 (rpc.rs "No transport yet"),
    // 也须防止未来一启用即暴露公网. Rust 不负责 Auth, RPC 须 localhost / Unix socket, 由 Fastify 反向代理.
    for w in _cfg.rpc_bind_security_warnings() {
        tracing::warn!(warning = %w, "rpc_bind 安全");
    }

    // Gate 2.2: device discovery.
    // `simulation` => mock devices (CI/tests, no hardware or SDK).
    // default / bmd => filesystem probe (safe on CI / non-BMD; real on BMD).
    // Phase 0.6 C5: Provider 选择收口至 `registry::AdapterRegistry` (Domain/Graph 不感知具体适配器)。
    // 选择优先级(mock > simulation > bmd-provider > default)见 registry.rs。
    // P0-4: adapter 选择收口 + fail-closed (mock+真实组合在生产模式拒启, 见 registry.rs).
    let provider: Box<dyn HardwareProvider> = crate::registry::AdapterRegistry::build_provider()
        .unwrap_or_else(|e| {
            eprintln!("adapter feature 冲突 (fail-closed): {e}");
            std::process::exit(2);
        });
    let (active_provider, active_backend) = crate::registry::active_adapters();
    tracing::info!(
        provider = active_provider,
        backend = active_backend,
        mode = std::env::var("MEDIA_AGENT_MODE")
            .as_deref()
            .unwrap_or("production"),
        "adapter selection (P0-4 运维可见)"
    );
    // P1-2: discover fail-closed — SDK/驱动失败显式拒启, 绝不与"无设备"混淆.
    let discovered = match provider.discover() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("device discovery 失败 (fail-closed): {e}");
            std::process::exit(2);
        }
    };
    let devices: Vec<DeviceInfo> = discovered.iter().map(|d| d.device.clone()).collect();
    tracing::info!(count = devices.len(), "device discovery complete");

    // B 选项 (用户 2026-08-28): Connector Mode / 各子设备配置方向探针.
    // 纯 SDK 读取 (IDeckLinkConfiguration), 不进媒体, 不依赖 GStreamer; 命中即 exit(0).
    // 用于无桌面环境 (无 Blackmagic Desktop Video Setup) 时直接回答
    // "每个子设备当前是 In 还是 Out、物理端口如何按 Connector Mode 分组".
    #[cfg(feature = "bmd-provider")]
    if std::env::var("VBMF_CONFIG_PROBE").is_ok() {
        match crate::adapters::blackmagic::probe_connector_config() {
            Ok(rows) => match serde_json::to_string_pretty(&rows) {
                Ok(json) => {
                    println!("=== B Connector Config Probe (IDeckLinkConfiguration) ===");
                    println!("{json}");
                }
                Err(e) => eprintln!("config probe 序列化失败: {e}"),
            },
            Err(e) => eprintln!("config probe 失败: {e}"),
        }
        std::process::exit(0);
    }

    // C1: DeckLinkDeviceResolver —— DeviceHandle → GStreamer device-number 物化 (仅解析+证据, 不启动 pipeline).
    // 设置 VBMF_RESOLVER=1 运行: 输出每台 SDK 设备与 GStreamer 实例的交叉映射证据,
    // 供现场核对 "CH01 怎么采到了另一张卡" / "device-number 与正确输入设备未对应"。
    // 与 CAP-01 生产路径严格隔离: 命中即 exit(0), 绝不进入媒体 launch。
    #[cfg(feature = "gstreamer-backend")]
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

    // HW-PORT-01D (Loopback): 真机 loopback 验收闭环 (STEP 11). 设置 VBMF_LOOPBACK=1 运行:
    // 加载 DeviceBindingManifest → 探测 GStreamer → 解析绑定 → 构建 PortRegistry → 加载 fixtures 目录
    // → 对每条 Fixture 在 source 渲染已知图案、在 sink 真实采集 (含加嵌音频探测) → verify_fixtures 双门
    // → 输出 FixtureVerification JSON. 命中即 exit(0), 绝不进入生产媒体 launch.
    #[cfg(feature = "gstreamer-backend")]
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
            crate::resolver::collect_bindings_from_manifest(&discovered, &probes, &manifest);
        let registry =
            match crate::port::PortRegistry::build(&discovered, &probes, &manifest, &bindings) {
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
                    let fresh_probes = match crate::resolver::probe_gstreamer_devices(
                        crate::resolver::MAX_PROBE_DEVICES,
                        false,
                    ) {
                        crate::resolver::GstProbeOutcome::Available { probes, .. } => probes,
                        _ => Vec::new(),
                    };
                    let fresh_registry = crate::port::PortRegistry::build(
                        &discovered,
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

    // Gate 2.3: lease manager (in-memory; no hardware needed for the interface).
    let lm = Arc::new(lease::InMemoryLeaseManager::new());
    for d in &devices {
        match lm.acquire(
            &d.device_id,
            "bootstrap",
            std::time::Duration::from_secs(60),
        ) {
            Ok(l) => tracing::info!(device = %l.device_id, "lease acquired"),
            Err(e) => tracing::warn!(error = %e, "lease acquire failed"),
        }
    }
    // 排他性不变量: 同一设备重复 acquire 必须被拒 (防 host ffmpeg / 双采).
    if let Some(first) = devices.first() {
        match lm.acquire(
            &first.device_id,
            "second-owner",
            std::time::Duration::from_secs(60),
        ) {
            Ok(_) => tracing::warn!("LEASE COLLISION — double-capture risk!"),
            Err(e) => tracing::info!(error = %e, "lease re-acquire correctly rejected"),
        }
    }

    // Gate 2.5 (A): DeckLink SDK FFI smoke — 验证 libDeckLinkAPI.so 在运行环境可达.
    // 宿主机(/usr/lib 默认路径)应成功; Option B 容器若不 bind-mount 库则 warn(预期).
    match crate::adapters::blackmagic::probe_sdk("libDeckLinkAPI.so") {
        Ok(()) => tracing::info!("SDK libDeckLinkAPI.so reachable, entry symbols present"),
        Err(e) => {
            tracing::warn!(error = %e, "SDK probe failed (expected in container w/o bind-mount)")
        }
    }

    // Gate 2.6 (P1①): bmd feature 下 `devices` 已直接来自 `DeckLinkDeviceManager`
    // (基于 DeckLink DeviceHandle 派生的 canonical identity), 不再按索引把 SDK 枚举回填
    // filesystem 列表 (拓扑变化后 index 会漂移, 见 device.rs).

    // Gate 7 (feature `hardware-test`): verbose Device Registry (model/serial/status) for BMD.
    #[cfg(feature = "hardware-test")]
    match crate::adapters::blackmagic::registry() {
        Ok(table) => tracing::info!("DeckLink Device Registry:\n{table}"),
        Err(e) => tracing::warn!(error = %e, "registry unavailable"),
    }
    // A0 纯身份核对模式: 仅打印注册表后退出, 不进入 SDK 采集探针 / 生产 materialize。
    #[cfg(feature = "hardware-test")]
    if std::env::var("VBMF_REGISTRY_ONLY").is_ok() {
        tracing::info!("VBMF_REGISTRY_ONLY 已设置: 仅输出注册表, 进程退出。");
        std::process::exit(0);
    }

    // Gate 5: Supervisor seeded with device handles (watchdog state machine + budget/
    // backoff/circuit-breaker are unit-tested in supervisor.rs). 包 Arc<Mutex> 以便 watch
    // 线程与 GStreamer recover 接线共享 (Supervisor 只决策, 不碰 GStreamer).
    // 0.7C-6 D8: 组合根唯一事件表 — SessionManager 与 Supervisor 共享 (单表单锁全局 FIFO)。
    let event_log = Arc::new(events::RuntimeEventLog::new());
    let sup = Arc::new(std::sync::Mutex::new(supervisor::Supervisor::new(
        supervisor::RestartPolicy::default(),
        event_log.clone(),
    )));
    for d in &devices {
        sup.lock().unwrap().register(d.device_id);
    }
    tracing::info!(watched = devices.len(), "supervisor initialized");

    // Gate 2.4: 最简 /health (std TcpListener, 无第三方依赖; 后续可换 axum).
    // Gate 2.6 (P1②): 返回真实运行时状态, 与 Supervisor 状态机对齐 (不再固定 ready).
    let device_count = devices.len();
    let agent_state = Arc::new(std::sync::Mutex::new(health::AgentState::Ready));
    // P0.7C-8: 诊断路径 SessionManager 提升到 transport 上下文 (生产路径 None → 503 契约诚实)。
    // 声明在 main body 顶层 (4-space), 使下方 cfg 块内赋值 (Arc 化 mgr) 与 main body 的
    // transport_ctx 构造 (health 线程) 共享同一作用域 — 块内声明无法被 main body 级引用。
    // `mut` 仅在 gstreamer 构建下被消费 (诊断路径 Arc 化 mgr 赋值); 非 gstreamer 构建
    // 该赋值被 cfg 移除 → `unused_mut` 属预期, 显式 allow (clippy -D 门禁)。
    #[allow(unused_mut)]
    let mut api_mgr: Option<std::sync::Arc<crate::session::SessionManager>> = None;

    // Gate 2.6 (CAP-01) — 关键边界澄清 (Phase 0.6 锁死):
    //   * `decklink::start_capture` (IDeckLinkInput) = SDK 能力 / 诊断探针
    //     (Gate 6/7), 验证 SDK 能否打开设备 / callback 是否正常 / 格式是否可读.
    //     它**不是** canonical 媒体数据通道 (否则与 GStreamer 争夺设备 → 双采).
    //     真机 GStreamer 启动后, 该探针仅限 `hardware-test` feature, 避免争用同一块卡.
    //   * canonical 媒体采集 = GStreamer `decklinkvideosrc` + `decklinkaudiosrc`
    //     (Phase 0.6). CAP-01 的 MEDIA-RT-01 (真实 SDI → GStreamer → RAW →
    //     first buffer) 由 `PipelineController` 拥有.
    #[cfg(feature = "bmd-provider")]
    {
        // MEDIA-RT-01 自测模式 (MEDIA_AGENT_SELFTEST=1): 用 videotestsrc/audiotestsrc
        // 验证媒体运行时链路 (GStreamer launch → appsink 首帧 → PTS → MEDIA-RT-01 A/B/C),
        // 不依赖 DeckLink 信号; 此时跳过下方 decklink canonical 路径.
        #[cfg(feature = "gstreamer-backend")]
        let skip_decklink = std::env::var("MEDIA_AGENT_SELFTEST").is_ok();
        #[cfg(not(feature = "gstreamer-backend"))]
        let skip_decklink = false;
        #[cfg(feature = "gstreamer-backend")]
        if skip_decklink {
            let plan = crate::pipeline::PipelinePlan::self_test();
            let ctrl: Arc<dyn MediaBackend> =
                crate::registry::AdapterRegistry::build_media_backend().unwrap_or_else(|e| {
                    eprintln!("adapter feature 冲突 (fail-closed): {e}");
                    std::process::exit(2);
                });
            match ctrl.instantiate(&plan) {
                Ok(h) => match ctrl.start(&h) {
                    Ok(()) => {
                        tracing::info!(handle = %h.0, "MEDIA-RT-01 self-test 管线启动 (videotestsrc/audiotestsrc → appsink)");
                        *agent_state.lock().unwrap() = health::AgentState::Capturing;
                        // 复用生产 ingest watchdog, 完整推导 A1-A4/B1-B4/C1-C4;
                        // 自测源稳定出帧 → pass() 达成即打印 "MEDIA-RT-01: A+B+C 全过".
                        spawn_ingest_watchdog(
                            ctrl,
                            h,
                            Uuid::nil(),
                            sup.clone(),
                            lm.clone(),
                            agent_state.clone(),
                        );
                    }
                    Err(e) => tracing::error!(error = %e, "MEDIA-RT-01 self-test 启动失败"),
                },
                Err(e) => tracing::error!(error = %e, "MEDIA-RT-01 self-test prepare 失败"),
            }
        }
        if !skip_decklink {
            // (A) SDK 诊断探针 (仅 hardware-test; 真机已验证可行, 不用于生产媒体路径).
            //     与 canonical GStreamer 路径互斥, 避免同时打开同一块 DeckLink.
            //     注: `hardware-test` 与 `gstreamer` 已在编译期互斥 (见文件顶部 compile_error),
            //     生产 canonical 运行时绝不会同时启用两者.
            #[cfg(all(feature = "hardware-test", not(feature = "gstreamer-backend")))]
            match crate::adapters::blackmagic::start_capture(0) {
                Ok(stats) => {
                    tracing::info!(
                        "CAP-01 SDK 诊断探针已启动 (device 0, IDeckLinkInput; 非 canonical 通道)"
                    );
                    std::thread::spawn(move || loop {
                        let n = stats.frame_count.load(std::sync::atomic::Ordering::SeqCst);
                        let ff = stats.first_frame_at.lock().unwrap().is_some();
                        let mono = stats.monotonic.load(std::sync::atomic::Ordering::SeqCst);
                        tracing::info!(
                            frame_count = n,
                            first_frame = ff,
                            pts_monotonic = mono,
                            "CAP-01 SDK probe live"
                        );
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    });
                }
                Err(e) => tracing::error!(error = %e, "CAP-01 SDK 诊断探针失败"),
            }

            // (B) canonical 媒体采集路径 (GStreamer). 控制面只带 VBMF device_id;
            // provider_persistent_id / device-number 由 materialize 经 Resolver 绑定解析得到.
            // 物化模式: 默认 Production; MEDIA_AGENT_MODE=diagnostic 时显式回退 (仅验证/排障).
            let mode = match std::env::var("MEDIA_AGENT_MODE").as_deref() {
                Ok("diagnostic") => crate::pipeline::MaterializeMode::Diagnostic,
                _ => crate::pipeline::MaterializeMode::Production,
            };
            // Resolver 绑定 (物化前置): gstreamer 构建下探测并解析; 非 gstreamer 构建为空 map.
            #[cfg(feature = "gstreamer-backend")]
            let gst_probes = match crate::resolver::probe_gstreamer_devices(
                crate::resolver::MAX_PROBE_DEVICES,
                _cfg.device_binding_path.is_none(),
            ) {
                crate::resolver::GstProbeOutcome::Available { probes, errors } => {
                    // 生产路径: 把分类后的单设备探测失败原因记录为 warning, 绝不静默丢弃 (用户 §⑥).
                    for (n, e) in &errors {
                        tracing::warn!(
                            device_number = n,
                            error = %e,
                            "GStreamer 单设备名 探测失败 (已分类, 不再静默丢弃)"
                        );
                    }
                    probes
                }
                // Unavailable / Empty → 无可用绑定; materialize 走拒绝路径, 绝不盲开 device 0.
                _ => Vec::new(),
            };
            // 绑定解析: 生产/诊断都执行 (用于 manifest 校验与诊断启动), 但**不在此启动任何管线**.
            #[cfg(feature = "gstreamer-backend")]
            let bindings = match &_cfg.device_binding_path {
                // 生产 BMD 绑定权威路径: DeviceBindingManifest 显式契约.
                // 加载/结构/machine 任一失败 → 失败闭合 (拒绝 materialize, 绝不盲开 device 0, 用户 §四/§五/§六).
                Some(p) => match crate::resolver::DeviceBindingManifest::load(p) {
                    Ok(m) => {
                        // (a) 结构完整性校验 (唯一性/非空 machine_id): 失败即拒绝 (ManifestInvalid).
                        let structural = m.validate_manifest();
                        // (b) 主机身份校验: 不符 → 失败闭合 (拒绝, 非 warning, 用户 §五).
                        let machine_ok =
                            m.check_machine_identity(&crate::resolver::current_machine_id());
                        if let Err(e) = &structural {
                            tracing::error!(error = %e, "DeviceBindingManifest 结构校验失败; 生产绑定失败闭合, 拒绝 materialize");
                            std::collections::HashMap::new()
                        } else if let Err(e) = &machine_ok {
                            tracing::error!(error = %e, "DeviceBindingManifest 主机身份不符; 生产绑定失败闭合, 拒绝 materialize (非 warning)");
                            std::collections::HashMap::new()
                        } else {
                            // (c) 版本一致性软告警 (P1-2: 已接真实 runtime 版本, 非 None).
                            let sdk_v = crate::resolver::declared_bmd_sdk_version();
                            let gst_v = crate::resolver::actual_gstreamer_version();
                            let plugin_v = crate::resolver::actual_decklink_plugin_version()
                                .unwrap_or_else(|| "unknown".to_string());
                            // P1-1: 真实运行时 SDK 身份 provenance (build include + libDeckLinkAPI.so).
                            tracing::info!(detected_sdk = %crate::resolver::detected_bmd_sdk_version(), "BMD SDK runtime identity (provenance)");
                            for w in
                                m.validate_environment(Some(&sdk_v), Some(&plugin_v), Some(&gst_v))
                            {
                                tracing::warn!(warning = %w, "device-binding manifest 版本校验");
                            }
                            crate::resolver::collect_bindings_from_manifest(
                                &discovered,
                                &gst_probes,
                                &m,
                            )
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "DeviceBindingManifest 加载失败; 生产绑定失败闭合, 拒绝 materialize");
                        std::collections::HashMap::new()
                    }
                },
                // 未提供清单:
                //  - 生产模式 → 失败闭合 (拒绝 materialize, 绝不回退 legacy 盲猜, 用户 §四).
                //  - diagnostic 模式 (MEDIA_AGENT_MODE=diagnostic) → 显式回退 legacy (仅排障).
                None => match mode {
                    crate::pipeline::MaterializeMode::Diagnostic => {
                        tracing::warn!("未提供 DeviceBindingManifest; diagnostic 模式显式回退 legacy auto-resolution (仅排障, 生产禁用)");
                        crate::resolver::collect_bindings(&discovered, &gst_probes)
                    }
                    crate::pipeline::MaterializeMode::Production => {
                        tracing::error!("生产模式未提供 DeviceBindingManifest; 绑定失败闭合, 拒绝 materialize (不回退 legacy, 用户 §四)");
                        std::collections::HashMap::new()
                    }
                },
            };
            #[cfg(not(feature = "gstreamer-backend"))]
            let bindings: std::collections::HashMap<
                uuid::Uuid,
                crate::resolver::ResolvedDeviceBinding,
            > = std::collections::HashMap::new();
            #[cfg(not(feature = "gstreamer-backend"))]
            let _ = &bindings;

            // 端口注册表 (供 materialize 经 Manifest 声明 + 运行时探测推导连接类型, 不硬编码 connection=sdi):
            // 仅当提供 manifest 时构建; 诊断 auto-start 无 manifest 回退 legacy → registry=None → connection 由插件默认探测.
            #[cfg(feature = "gstreamer-backend")]
            let registry = _cfg.device_binding_path.as_ref().and_then(|p| {
                crate::resolver::DeviceBindingManifest::load(p)
                    .ok()
                    .map(|m| {
                        crate::port::PortRegistry::build(&discovered, &gst_probes, &m, &bindings)
                            .expect("端口发现与 manifest 不一致 (fail-closed 拒绝)")
                    })
            });
            // 非 gst 构建 (hardware-test): 端口注册表由真机闭环/会话路径消费, 此处不构建
            // (P0-7A 起无 gst 的物化路径已不存在, registry 留空避免 unused)。
            #[cfg(not(feature = "gstreamer-backend"))]
            let _registry: Option<crate::port::PortRegistry> = None;

            // P0-7A SESSION-RT-01/RESOURCE-RT-01 真机门禁入口 (仿 VBMF_LOOPBACK 模式):
            // 全生命周期 create→start→观察 10s→stop→close 逐步 verdict + 第二会话冲突拒绝实证。
            // 命中即 exit, 绝不进入生产 auto_start。
            #[cfg(feature = "gstreamer-backend")]
            if std::env::var("VBMF_SESSION_LIFECYCLE").is_ok() {
                let first_id = devices
                    .first()
                    .map(|d| d.device_id.to_string())
                    .unwrap_or_default();
                let manifest_path = match &_cfg.device_binding_path {
                    Some(p) => p.clone(),
                    None => {
                        eprintln!("VBMF_SESSION_LIFECYCLE 需要 DeviceBindingManifest (MEDIA_AGENT_DEVICE_BINDING)");
                        std::process::exit(2);
                    }
                };
                let manifest = match crate::resolver::DeviceBindingManifest::load(&manifest_path) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("manifest 加载失败: {e}");
                        std::process::exit(2);
                    }
                };
                if manifest.validate_manifest().is_err() {
                    eprintln!("manifest 结构校验失败");
                    std::process::exit(2);
                }
                let gst_probes = match crate::resolver::probe_gstreamer_devices(
                    crate::resolver::MAX_PROBE_DEVICES,
                    false,
                ) {
                    crate::resolver::GstProbeOutcome::Available { probes, .. } => probes,
                    _ => Vec::new(),
                };
                let bindings = crate::resolver::collect_bindings_from_manifest(
                    &discovered,
                    &gst_probes,
                    &manifest,
                );
                let registry = match crate::port::PortRegistry::build(
                    &discovered,
                    &gst_probes,
                    &manifest,
                    &bindings,
                ) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("PortRegistry 构建失败 (fail-closed): {e:?}");
                        std::process::exit(2);
                    }
                };
                let resources = crate::resource::SharedResourceRegistry::new(
                    crate::resource::ResourceRegistry::derive_from_discovery(&registry),
                );
                let ctrl: std::sync::Arc<dyn MediaBackend> =
                    crate::registry::AdapterRegistry::build_media_backend().unwrap_or_else(|e| {
                        eprintln!("adapter feature 冲突 (fail-closed): {e}");
                        std::process::exit(2);
                    });
                let mgr = std::sync::Arc::new(crate::session::SessionManager::new(
                    resources,
                    lm.clone(),
                    sup.clone(),
                    ctrl.clone(),
                    std::sync::Arc::new(devices.clone()),
                    std::sync::Arc::new(bindings.clone()),
                    Some(registry),
                    crate::pipeline::MaterializeMode::Diagnostic,
                    crate::session::SessionTuning::default(),
                    event_log.clone(),
                ));
                // P0.7C-2: Runtime Query (Pure Read) 门面 — 硬件证据冒烟。
                let _rq = crate::runtime_query::RuntimeQuery::new(std::sync::Arc::clone(&mgr));
                let intent = crate::graph_intent::GraphRuntimeIntent {
                    version: "1.0".into(),
                    devices: vec![crate::graph_intent::DeviceIntent {
                        device_id: first_id.clone(),
                        role: "CAPTURE".into(),
                        pipeline: crate::graph_intent::PipelineIntent {
                            source: crate::graph_intent::SourceIntent {
                                kind: "decklink".into(),
                                device_id: first_id.clone(),
                                port_id: None,
                            },
                            sink: crate::graph_intent::SinkIntent {
                                kind: "rtmp".into(),
                            },
                        },
                    }],
                };
                let mut ok = true;
                // bootstrap 占位租约让位: 真实会话租约 (owner=session) 接管排他性。
                let _ = lm.release(&crate::lease::DeviceLease {
                    device_id: Uuid::parse_str(&first_id).unwrap_or(Uuid::nil()),
                    owner: "bootstrap".into(),
                    acquired_at: chrono::Utc::now(),
                    ttl: std::time::Duration::from_secs(60),
                });
                // RUNTIME-STATE-RT-01 (Hardware): create 前 CanonicalRuntimeState。
                match serde_json::to_string_pretty(&mgr.runtime_state()) {
                    Ok(json) => {
                        println!("=== RUNTIME-STATE-RT-01 CanonicalRuntimeState (create 前) ===");
                        println!("{json}");
                    }
                    Err(e) => eprintln!("runtime state 序列化失败: {e}"),
                }
                println!("SESSION-RT-01 step=create ...");
                match mgr.create(intent.clone()) {
                    Ok(sid) => {
                        println!("SESSION-RT-01 step=create verdict=OK session={sid}");
                        println!("SESSION-RT-01 step=start ...");
                        match mgr.start(&sid) {
                            Ok(()) => {
                                println!("SESSION-RT-01 step=start verdict=OK (pipeline Running)");
                                println!("SESSION-RT-01 step=observe 10s ...");
                                std::thread::sleep(std::time::Duration::from_secs(10));
                                let running = mgr
                                    .status(&sid)
                                    .map(|s| s.phase == crate::session::SessionPhase::Running)
                                    .unwrap_or(false);
                                println!(
                                    "SESSION-RT-01 step=observe verdict={} (running={running})",
                                    if running { "OK" } else { "FAIL" }
                                );
                                ok &= running;
                            }
                            Err(e) => {
                                println!("SESSION-RT-01 step=start verdict=FAIL error={e}");
                                ok = false;
                            }
                        }
                        println!("SESSION-RT-01 step=stop ...");
                        match mgr.stop(&sid) {
                            Ok(()) => println!("SESSION-RT-01 step=stop verdict=OK"),
                            Err(e) => {
                                println!("SESSION-RT-01 step=stop verdict=FAIL error={e}");
                                ok = false;
                            }
                        }
                        let _ = mgr.close(&sid);
                    }
                    Err(e) => {
                        println!("SESSION-RT-01 step=create verdict=FAIL error={e}");
                        ok = false;
                    }
                }
                // RUNTIME-STATE-RT-01 (Hardware): 会话生命周期后 CanonicalRuntimeState
                // (资源回落 Available / 会话终态投影可见)。
                match serde_json::to_string_pretty(&mgr.runtime_state()) {
                    Ok(json) => {
                        println!("=== RUNTIME-STATE-RT-01 CanonicalRuntimeState (生命周期后) ===");
                        println!("{json}");
                    }
                    Err(e) => eprintln!("runtime state 序列化失败: {e}"),
                }
                // P0.7C-3 COMMAND-CONTRACT-RT-01 (Hardware): envelope 驱动段——
                // Start→observe→Stop→Release 经 Command Contract (与直接路径等价)。
                {
                    use crate::command::{
                        dispatch, CommandEnvelope, CommandId, CommandKind, CommandTarget,
                    };
                    let cmd_env = CommandEnvelope {
                        command_id: CommandId(uuid::Uuid::new_v4()),
                        kind: CommandKind::StartSession,
                        target: CommandTarget::Session {
                            intent: intent.clone(),
                        },
                        issued_at_ms: 0,
                        requested_by: "vbmf-session-lifecycle-gate".into(),
                    };
                    let out = dispatch(&mgr, &cmd_env);
                    println!(
                        "COMMAND-CONTRACT-RT-01 step=start status={:?} detail={:?}",
                        out.status, out.detail
                    );
                    let sid = mgr.list().first().map(|s| s.session_id);
                    if let Some(sid) = sid {
                        println!("COMMAND-CONTRACT-RT-01 step=observe 10s ...");
                        std::thread::sleep(std::time::Duration::from_secs(10));
                        let running = mgr
                            .status(&sid)
                            .map(|s| s.phase == crate::session::SessionPhase::Running)
                            .unwrap_or(false);
                        println!("COMMAND-CONTRACT-RT-01 step=observe running={running}");
                        let stop_env = CommandEnvelope {
                            command_id: CommandId(uuid::Uuid::new_v4()),
                            kind: CommandKind::StopSession,
                            target: CommandTarget::SessionById { session_id: sid },
                            issued_at_ms: 0,
                            requested_by: "vbmf-session-lifecycle-gate".into(),
                        };
                        let out = dispatch(&mgr, &stop_env);
                        println!(
                            "COMMAND-CONTRACT-RT-01 step=stop status={:?} detail={:?}",
                            out.status, out.detail
                        );
                        let rel_env = CommandEnvelope {
                            command_id: CommandId(uuid::Uuid::new_v4()),
                            kind: CommandKind::ReleaseSession,
                            target: CommandTarget::SessionById { session_id: sid },
                            issued_at_ms: 0,
                            requested_by: "vbmf-session-lifecycle-gate".into(),
                        };
                        let out = dispatch(&mgr, &rel_env);
                        println!(
                            "COMMAND-CONTRACT-RT-01 step=release status={:?} detail={:?}",
                            out.status, out.detail
                        );
                    }
                }
                // P0.7C-4 IDEMPOTENCY-RT-01 (Hardware): 幂等裁决段——
                // 同 envelope 重发 Replayed / 同 id 换 intent Conflict / 会话数不增。
                {
                    use crate::command::{CommandEnvelope, CommandId, CommandKind, CommandTarget};
                    use crate::idempotency::{CommandIdempotency, IdempotentDispatch};
                    let idem = CommandIdempotency::new(std::sync::Arc::clone(&mgr));
                    let cmd_env = CommandEnvelope {
                        command_id: CommandId(uuid::Uuid::new_v4()),
                        kind: CommandKind::StartSession,
                        target: CommandTarget::Session {
                            intent: intent.clone(),
                        },
                        issued_at_ms: 0,
                        requested_by: "vbmf-idempotency-gate".into(),
                    };
                    let start_out = match idem.dispatch(&cmd_env) {
                        IdempotentDispatch::Executed(o) => Some(o),
                        other => {
                            println!("IDEMPOTENCY-RT-01 step=start verdict=UNEXPECTED {other:?}");
                            None
                        }
                    };
                    if let Some(o) = &start_out {
                        println!(
                            "IDEMPOTENCY-RT-01 step=start verdict=executed status={:?} classification={:?} detail={:?} sessions={}",
                            o.status,
                            o.classification,
                            o.detail,
                            mgr.list().len()
                        );
                    }
                    // 重复投递 → Replayed (会话数不增)。
                    let sessions_before = mgr.list().len();
                    match idem.dispatch(&cmd_env) {
                        IdempotentDispatch::Replayed(o) => {
                            let replay_same = start_out.as_ref() == Some(&o);
                            println!(
                                "IDEMPOTENCY-RT-01 step=duplicate verdict=replayed status={:?} classification={:?} sessions={sessions_before} outcome_equal={replay_same}",
                                o.status, o.classification
                            );
                        }
                        other => {
                            println!(
                                "IDEMPOTENCY-RT-01 step=duplicate verdict=UNEXPECTED {other:?}"
                            );
                        }
                    }
                    // 同 command_id 换 intent → Conflict (零执行)。
                    // (改 version 而非 sink.kind: 真机 intent 的 sink 恒为 rtmp,
                    //  改成 rtmp 等于没改 → 指纹不变 → 误判 Replayed; version 是
                    //  canonical 字段且真机恒为 "1.0", 必产生指纹差。)
                    let mut conflict_env = cmd_env.clone();
                    if let CommandTarget::Session { intent } = &mut conflict_env.target {
                        intent.version = "idempotency-conflict-probe".into();
                    }
                    let sessions_before = mgr.list().len();
                    match idem.dispatch(&conflict_env) {
                        IdempotentDispatch::Conflict { .. } => {
                            println!(
                                "IDEMPOTENCY-RT-01 step=conflict verdict=conflict sessions={sessions_before}"
                            );
                        }
                        other => {
                            println!(
                                "IDEMPOTENCY-RT-01 step=conflict verdict=UNEXPECTED {other:?}"
                            );
                        }
                    }
                    // observe: 幂等段创建的会话仍在运行。
                    if let Some(sid) = mgr.list().first().map(|s| s.session_id) {
                        println!("IDEMPOTENCY-RT-01 step=observe 10s ...");
                        std::thread::sleep(std::time::Duration::from_secs(10));
                        let running = mgr
                            .status(&sid)
                            .map(|s| s.phase == crate::session::SessionPhase::Running)
                            .unwrap_or(false);
                        println!("IDEMPOTENCY-RT-01 step=observe running={running}");
                        let stop_env = CommandEnvelope {
                            command_id: CommandId(uuid::Uuid::new_v4()),
                            kind: CommandKind::StopSession,
                            target: CommandTarget::SessionById { session_id: sid },
                            issued_at_ms: 0,
                            requested_by: "vbmf-idempotency-gate".into(),
                        };
                        match idem.dispatch(&stop_env) {
                            IdempotentDispatch::Executed(o) => println!(
                                "IDEMPOTENCY-RT-01 step=stop verdict=executed status={:?} classification={:?}",
                                o.status, o.classification
                            ),
                            other => {
                                println!(
                                    "IDEMPOTENCY-RT-01 step=stop verdict=UNEXPECTED {other:?}"
                                );
                            }
                        }
                        let rel_env = CommandEnvelope {
                            command_id: CommandId(uuid::Uuid::new_v4()),
                            kind: CommandKind::ReleaseSession,
                            target: CommandTarget::SessionById { session_id: sid },
                            issued_at_ms: 0,
                            requested_by: "vbmf-idempotency-gate".into(),
                        };
                        match idem.dispatch(&rel_env) {
                            IdempotentDispatch::Executed(o) => println!(
                                "IDEMPOTENCY-RT-01 step=release verdict=executed status={:?} classification={:?}",
                                o.status, o.classification
                            ),
                            other => {
                                println!(
                                    "IDEMPOTENCY-RT-01 step=release verdict=UNEXPECTED {other:?}"
                                );
                            }
                        }
                        // P0.7C-5 ERROR-MODEL-RT-01 (Hardware): ghost 探针——
                        // 对已 release 的会话再发 Stop (新 command_id) → Failed +
                        // classification=PermanentFailure (UnknownSession 臂)。
                        let ghost_env = CommandEnvelope {
                            command_id: CommandId(uuid::Uuid::new_v4()),
                            kind: CommandKind::StopSession,
                            target: CommandTarget::SessionById { session_id: sid },
                            issued_at_ms: 0,
                            requested_by: "vbmf-error-model-gate".into(),
                        };
                        match idem.dispatch(&ghost_env) {
                            IdempotentDispatch::Executed(o) => println!(
                                "ERROR-MODEL-RT-01 step=ghost-stop status={:?} classification={:?} detail={:?}",
                                o.status, o.classification, o.detail
                            ),
                            other => {
                                println!(
                                    "ERROR-MODEL-RT-01 step=ghost-stop verdict=UNEXPECTED {other:?}"
                                );
                            }
                        }
                    }
                }
                // RESOURCE-RT-01: 第二会话争同资源必须被拒 (首会话已释放 → 先占住再争)。
                println!("RESOURCE-RT-01 step=conflict ...");
                let sid_a = mgr.create(crate::graph_intent::GraphRuntimeIntent {
                    version: "1.0".into(),
                    devices: vec![crate::graph_intent::DeviceIntent {
                        device_id: first_id.clone(),
                        role: "CAPTURE".into(),
                        pipeline: crate::graph_intent::PipelineIntent {
                            source: crate::graph_intent::SourceIntent {
                                kind: "decklink".into(),
                                device_id: first_id.clone(),
                                port_id: None,
                            },
                            sink: crate::graph_intent::SinkIntent {
                                kind: "appsink".into(),
                            },
                        },
                    }],
                });
                match sid_a {
                    Ok(a) => {
                        let conflict = mgr.create(crate::graph_intent::GraphRuntimeIntent {
                            version: "1.0".into(),
                            devices: vec![crate::graph_intent::DeviceIntent {
                                device_id: first_id.clone(),
                                role: "CAPTURE".into(),
                                pipeline: crate::graph_intent::PipelineIntent {
                                    source: crate::graph_intent::SourceIntent {
                                        kind: "decklink".into(),
                                        device_id: first_id.clone(),
                                        port_id: None,
                                    },
                                    sink: crate::graph_intent::SinkIntent {
                                        kind: "appsink".into(),
                                    },
                                },
                            }],
                        });
                        println!(
                            "RESOURCE-RT-01 step=conflict verdict={}",
                            if conflict.is_err() {
                                "OK (第二会话被拒)"
                            } else {
                                "FAIL (资源被超卖!)"
                            }
                        );
                        ok &= conflict.is_err();
                        let _ = mgr.close(&a);
                    }
                    Err(e) => {
                        println!("RESOURCE-RT-01 step=conflict verdict=FAIL error={e}");
                        ok = false;
                    }
                }
                // P0.7C-6 EVENT-PROJECTION-RT-01 (Hardware): 消费接线实证——
                // 全生命周期事件 drain → project → 只读快照 (Observation, 不写回)。
                let drained_events = event_log.drain();
                let p = crate::event_projection::project(&drained_events);
                {
                    println!(
                        "EVENT-PROJECTION-RT-01 total={} kinds={:?}",
                        p.total, p.kind_counts
                    );
                    println!(
                        "EVENT-PROJECTION-RT-01 session_states={:?} session_failures={:?} has_critical={}",
                        p.session_states, p.session_failures, p.has_critical
                    );
                    println!(
                        "EVENT-PROJECTION-RT-01 dropped_obs={} dropped_crit={}",
                        event_log.dropped_observations(),
                        event_log.dropped_criticals()
                    );
                }
                // P0.7C-7 EXTERNAL-API-RT-01 (Hardware): API Boundary Model 实证——
                // 真机 Runtime State → API 模型纯转换 + 序列化往返 (非 Web Server;
                // 零 transport / 零持久化; 禁清单 11 项)。
                {
                    use crate::api_boundary::{
                        default_idempotency_boundary, to_api_query_snapshot, ApiProjectionResponse,
                        ApiQuerySnapshot,
                    };
                    let state = _rq.get_runtime_state();
                    let snap = to_api_query_snapshot(&state);
                    let snap_json =
                        serde_json::to_string(&snap).expect("ApiQuerySnapshot 必须可序列化");
                    let _roundtrip: ApiQuerySnapshot =
                        serde_json::from_str(&snap_json).expect("ApiQuerySnapshot 必须可反序列化");
                    let api_proj: ApiProjectionResponse = (&p).into();
                    let proj_json = serde_json::to_string(&api_proj)
                        .expect("ApiProjectionResponse 必须可序列化");
                    let boundary = default_idempotency_boundary();
                    let boundary_json = serde_json::to_string(&boundary)
                        .expect("ApiIdempotencyBoundary 必须可序列化");
                    let api_ok = snap.devices.len() == state.devices.len()
                        && snap.sessions.len() == state.sessions.len()
                        && snap_json.contains("\"devices\"")
                        && proj_json.contains("\"event_projection_snapshot\"")
                        && boundary_json.contains("\"process_local\"")
                        && boundary_json.contains("\"durable_log_deferred\"")
                        && boundary_json.contains("\"restart_breaks_replay\"");
                    println!(
                        "EXTERNAL-API-RT-01 verdict={} devices={} sessions={} resources={} boundary=process_local/durable_log_deferred/restart_breaks_replay",
                        if api_ok { "OK" } else { "FAIL" },
                        snap.devices.len(),
                        snap.sessions.len(),
                        snap.resources.len()
                    );
                    println!(
                        "EXTERNAL-API-RT-01 projection_total={} snapshot_kind=event_projection_snapshot",
                        p.total
                    );
                    ok &= api_ok;
                }
                println!(
                    "=== SESSION-RT-01/RESOURCE-RT-01/EXTERNAL-API-RT-01 ALL {} ===",
                    if ok { "PASS" } else { "FAIL" }
                );
                std::process::exit(if ok { 0 } else { 2 });
            }

            // 生产启动语义 (用户 §七 P1-3): 仅 diagnostic (或 self-test) 自动从绑定创建并启动 media pipeline;
            // Production **绝不**自行取 first device 制造 GraphRuntimeIntent —— 必须等待 Control Plane
            // 显式 StartPipeline Intent. (rpc.rs 当前 No transport yet, 故 Production 在此 idle:
            // 仅校验 manifest + 提供 /health, 不自动启动任何媒体管线.)
            // P0.7C-8: api_mgr 已提升到 main body 顶层 (见 agent_state 之后), 此处仅赋值。
            let auto_start = matches!(mode, crate::pipeline::MaterializeMode::Diagnostic);
            if auto_start {
                let first_id = devices
                    .first()
                    .map(|d| d.device_id.to_string())
                    .unwrap_or_default();
                let intent = crate::graph_intent::GraphRuntimeIntent {
                    version: "1.0".into(),
                    devices: vec![crate::graph_intent::DeviceIntent {
                        device_id: first_id.clone(),
                        role: "CAPTURE".into(),
                        pipeline: crate::graph_intent::PipelineIntent {
                            source: crate::graph_intent::SourceIntent {
                                kind: "decklink".into(),
                                device_id: first_id.clone(),
                                port_id: None,
                            },
                            sink: crate::graph_intent::SinkIntent {
                                kind: "rtmp".into(),
                            },
                        },
                    }],
                };

                #[cfg(feature = "gstreamer-backend")]
                {
                    // P0-7A: 生命周期由 SessionManager 唯一拥有 (RUNTIME_SESSION_MODEL §4.1) —
                    // create = Preflight→Reserve→建档→Lease→Binding verify (失败逆序回滚零孤儿);
                    // start = materialize→instantiate→Allocate→Backend.start→Running。
                    let resources = crate::resource::SharedResourceRegistry::new(
                        registry
                            .as_ref()
                            .map(|reg| {
                                crate::resource::ResourceRegistry::derive_from_discovery(reg)
                            })
                            .unwrap_or_default(),
                    );
                    let ctrl: std::sync::Arc<dyn MediaBackend> =
                        crate::registry::AdapterRegistry::build_media_backend().unwrap_or_else(
                            |e| {
                                eprintln!("adapter feature 冲突 (fail-closed): {e}");
                                std::process::exit(2);
                            },
                        );
                    // P0.7C-8: Arc 化 (tick 线程 + transport 上下文共享; 原 mgr 被 tick 线程 move,
                    // 共享须 Arc; 既有 mgr.xxx() 调用经 Arc 透传, 零语义变化)。
                    let mgr: std::sync::Arc<crate::session::SessionManager> =
                        std::sync::Arc::new(crate::session::SessionManager::new(
                            resources,
                            lm.clone(),
                            sup.clone(),
                            ctrl.clone(),
                            std::sync::Arc::new(devices.clone()),
                            std::sync::Arc::new(bindings.clone()),
                            registry.clone(),
                            mode,
                            crate::session::SessionTuning::default(),
                            event_log.clone(),
                        ));
                    api_mgr = Some(mgr.clone());
                    let dev_uuid = Uuid::parse_str(&first_id).unwrap_or(Uuid::nil());
                    // bootstrap 占位租约让位: 真实会话租约接管排他性 (P0-7A)。
                    let _ = lm.release(&crate::lease::DeviceLease {
                        device_id: dev_uuid,
                        owner: "bootstrap".into(),
                        acquired_at: chrono::Utc::now(),
                        ttl: std::time::Duration::from_secs(60),
                    });
                    match mgr.create(intent.clone()) {
                        Ok(sid) => match mgr.start(&sid) {
                            Ok(()) => {
                                tracing::info!(gst_version = ?crate::adapters::gstreamer::gstreamer_runtime_version(), "GStreamer runtime version (evidence)");
                                tracing::info!(session = %sid, "P0-7A Session create+start 全链通过 (SessionManager owner)");
                                *agent_state.lock().unwrap() = health::AgentState::Capturing;
                                // watchdog 继续 Supervise pipeline (recover 前重验 lease 不变量保留)。
                                if let Some(h) = mgr.status(&sid).and_then(|s| s.pipeline) {
                                    spawn_ingest_watchdog(
                                        ctrl,
                                        h,
                                        dev_uuid,
                                        sup.clone(),
                                        lm.clone(),
                                        agent_state.clone(),
                                    );
                                }
                                // tick 驱动 lease 续期/预留过期 (无后台定时器, 借常驻线程节拍)。
                                std::thread::spawn(move || loop {
                                    std::thread::sleep(std::time::Duration::from_secs(5));
                                    mgr.tick();
                                });
                            }
                            Err(e) => {
                                tracing::error!(error = %e, session = %sid, "P0-7A Session start 失败 (已逆序回滚, fail-closed)");
                                *agent_state.lock().unwrap() = health::AgentState::Degraded;
                            }
                        },
                        Err(e) => {
                            tracing::error!(error = %e, "P0-7A Session create 失败 (Preflight/Reserve/Lease fail-closed, 零孤儿)");
                            *agent_state.lock().unwrap() = health::AgentState::Degraded;
                        }
                    }
                }
                #[cfg(not(feature = "gstreamer-backend"))]
                {
                    let _ = &intent; // 无后端构建: 不启动 (canonical launch 待启用 feature 'gstreamer')
                    tracing::info!(
                        "canonical 计划已物化; 真实 GStreamer launch 待启用 feature 'gstreamer'"
                    );
                }
            } else {
                // Production: manifest 已在上校验 (缺失/无效 → 失败闭合已记录), 不自动启动任何媒体管线.
                tracing::info!(
                    "production runtime ready: manifest 已校验, 等待 Control Plane 显式 StartPipeline Intent (不自动启动 media pipeline; RPC transport 待接, 见 rpc.rs)"
                );
                *agent_state.lock().unwrap() = health::AgentState::Ready;
            }
        }
    }

    // P0.7C-8: Transport 上下文 (Query/Command 持 Option: 生产路径无 mgr → 503 契约诚实;
    // events/agent_state/device_count 全路径可用)。/health 响应体经 transport::route 保持
    // 逐字段不变 (回归锚点)。
    let transport_ctx = crate::transport::TransportContext {
        events: event_log.clone(),
        agent_state: agent_state.clone(),
        device_count,
        query: api_mgr
            .as_ref()
            .map(|m| std::sync::Arc::new(crate::runtime_query::RuntimeQuery::new(m.clone()))),
        idem: api_mgr
            .as_ref()
            .map(|m| std::sync::Arc::new(crate::idempotency::CommandIdempotency::new(m.clone()))),
    };

    std::thread::spawn({
        let transport_ctx = transport_ctx.clone();
        // 管理面绑定 (用户 §二十二 P1 Security): 默认 127.0.0.1:8080 (仅本机回环), 不裸露公网;
        // 生产部署由 `MEDIA_AGENT_HEALTH_BIND` 覆盖为内网接口/经 Fastify/Nginx 反向代理 + 认证.
        move || match std::net::TcpListener::bind(&_cfg.health_bind) {
            Ok(listener) => {
                tracing::info!(bind = %_cfg.health_bind, "health+api endpoints listening (internal-only; 经反向代理/认证暴露, 见用户 §二十二)");
                for s in listener.incoming().flatten() {
                    crate::transport::serve_connection(s, &transport_ctx);
                }
            }
            Err(e) => tracing::error!(error = %e, "health bind failed"),
        }
    });

    tracing::info!("media-agent canonical runtime loaded (health :8080; ingest via GStreamer started on lease acquire)");
    // 常驻以便 health 探测 (Gate 2.4 演示); 生产由 supervisor 管理生命周期.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

/// MEDIA-RT-01 watchdog (Supervisor → PipelineController.recover 运行时接线).
///
/// 单向健康链 (回应 #9): `GStreamer Bus → PipelineHealth → AgentState → Supervisor → Health API`.
/// 周期: 真 bus 监控 (Error/EOS/StateChanged) + appsink 计数 → 推导 MEDIA-RT-01
/// A1-A4 / B1-B4 / C1-C4 → 错误时报告 Supervisor (决策引擎) → Restart → 重校 lease → recover.
/// Supervisor 仅决策, 不碰 GStreamer (硬边界); 实际重启由这里执行.
/// 调用点 (self-test / canonical) 均在 `#[cfg(feature = "bmd-provider")]` 块内, 故本函数仅在 bmd && gstreamer 时编译.
/// `ctrl` 已为 `Arc<dyn MediaBackend>` (C2c): Mock 与 GStreamer 共享同一 `PipelinePlan` 契约.
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn spawn_ingest_watchdog(
    ctrl: Arc<dyn MediaBackend>,
    handle: crate::pipeline::PipelineHandle,
    device_uuid: Uuid,
    sup: Arc<std::sync::Mutex<supervisor::Supervisor>>,
    lm: Arc<lease::InMemoryLeaseManager>,
    agent_state: Arc<std::sync::Mutex<health::AgentState>>,
) {
    std::thread::spawn(move || {
        // A1/A2 在 start 前已由 materialize (身份解析) + lm.is_valid (租约) 保证, 否则不会进 watchdog.
        let _stability_window = std::time::Duration::from_secs(10); // MEDIA-RT-01C 验收窗口
        let mut prev_video = 0u64;
        let mut prev_audio = 0u64;
        let mut tick = 0u64;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            // 真实 GStreamer bus 监控 (Error/EOS/StateChanged) —— Supervisor 闭环数据源 (#8).
            let events = ctrl.observe(&handle);
            let mut bus_events: u64 = 0;
            // 在共享 Arc 上就地更新 acceptance 子项: 只读 live 状态→推导→写回 acceptance,
            // 绝不覆盖 appsink 回调写入的 video_frame_count/audio_frame_count/PTS/video_pts_state/audio_pts_state,
            // 否则每轮 snapshot 写回会把实时计数回退, 破坏 c4(计数增长) 判定 (#4 回归).
            let (pass, has_error) = if let Some(h) = crate::pipeline_events::HEALTH_ARCS
                .lock()
                .unwrap()
                .get(&handle)
            {
                let mut g = h.lock().unwrap();
                g.acceptance.a1_identity_resolved = true;
                g.acceptance.a2_lease_acquired = true;
                g.acceptance.a4_signal_detected = g.first_frame_ok();
                g.acceptance.b1_first_video = g.video_first_pts.is_some();
                g.acceptance.b2_first_audio = g.audio_first_pts.is_some();
                g.acceptance.b3_valid_pts = g.video_first_pts.is_some();
                g.acceptance.a3_pipeline_playing = g.playing;
                // b4 由两路 PTS 三态推导 (P1-3): 仅当 video 与 audio 均 ValidMonotonic 才视为 PTS 单调通过.
                // 绝不回退到单一 bool; Unknown/NonMonotonic 任一即不通过.
                g.acceptance.b4_pts_monotonic = g.video_pts_state
                    == crate::pipeline::PtsMonotonicity::ValidMonotonic
                    && g.audio_pts_state == crate::pipeline::PtsMonotonicity::ValidMonotonic;
                g.acceptance.c1_no_unexpected_eos = g.acceptance.c_unexpected_eos == 0;
                g.acceptance.c2_no_pipeline_error = g.last_error.is_none();
                g.acceptance.c3_no_repeated_reneg = g.acceptance.c_renegotiations == 0;
                let v = g.video_frame_count;
                let a = g.audio_frame_count;
                g.acceptance.c4_counters_continue = v > prev_video && a > prev_audio;
                prev_video = v;
                prev_audio = a;
                // C 稳定性窗口计时 + 测量字段 (用户复核 §十二).
                if let Some(started) = g.started_at {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    g.acceptance.c_observed_ms = Some(((now - started).max(0) as u64) * 1000);
                }
                g.acceptance.c_video_frames = g.video_frame_count;
                g.acceptance.c_audio_frames = g.audio_frame_count;
                let before = g.bus_event_count;
                g.bus_event_count += events.len() as u64;
                // P1-4 接线证据: 首次接获任意真实 Bus 事件时打一条 info (仅一次),
                // 证明 Bus watch → channel → poll_bus 链路端到端生效 (非 stub).
                if before == 0 && !events.is_empty() {
                    let kinds: Vec<&'static str> = events
                        .iter()
                        .map(|e| match e.kind {
                            crate::pipeline_events::PipelineBusEventKind::Error => "Error",
                            crate::pipeline_events::PipelineBusEventKind::Eos => "Eos",
                            crate::pipeline_events::PipelineBusEventKind::StateChanged => {
                                "StateChanged"
                            }
                            crate::pipeline_events::PipelineBusEventKind::Warning => "Warning",
                            crate::pipeline_events::PipelineBusEventKind::ClockLost => "ClockLost",
                        })
                        .collect();
                    tracing::info!(
                        handle = %handle.0,
                        kinds = ?kinds,
                        "MEDIA-RT-01 bus watch 首次接获真实 GStreamer Bus 事件 (P1-4 接线生效)"
                    );
                }
                for e in &events {
                    match e.kind {
                        crate::pipeline_events::PipelineBusEventKind::Error => {
                            g.acceptance.c_pipeline_errors += 1;
                        }
                        crate::pipeline_events::PipelineBusEventKind::Eos => {
                            g.acceptance.c_unexpected_eos += 1;
                        }
                        // P1-4 最低策略映射 (bus_event_recovery_policy): ClockLost = degraded, 不自动重启.
                        crate::pipeline_events::PipelineBusEventKind::ClockLost => {
                            crate::pipeline::CLOCK_LOST_EVENTS
                                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            tracing::warn!(
                                handle = %handle.0,
                                severity = ?e.severity,
                                detail = %e.detail,
                                policy = crate::pipeline_events::bus_event_recovery_policy(e.kind),
                                "Bus ClockLost: 标记 degraded, 不触发重启 (完整 Clock Recovery 属 V0.3/P2)"
                            );
                        }
                        crate::pipeline_events::PipelineBusEventKind::Warning => {
                            tracing::warn!(
                                handle = %handle.0,
                                severity = ?e.severity,
                                detail = %e.detail,
                                "Bus Warning (可恢复异常, 记录不重启)"
                            );
                        }
                        crate::pipeline_events::PipelineBusEventKind::StateChanged => {
                            tracing::info!(
                                handle = %handle.0,
                                detail = %e.detail,
                                "Bus StateChanged (生命周期事件)"
                            );
                        }
                    }
                }
                bus_events = g.bus_event_count;
                (
                    g.acceptance.a_pass() && g.acceptance.b_pass() && g.acceptance.c_pass(),
                    g.last_error.is_some(),
                )
            } else {
                (false, false)
            };

            // 错误 / 总线错误 → Supervisor 决策引擎 (仅决策, 不碰 GStreamer).
            if has_error
                || events.iter().any(|e| {
                    matches!(
                        e.kind,
                        crate::pipeline_events::PipelineBusEventKind::Error
                            | crate::pipeline_events::PipelineBusEventKind::Eos
                    )
                })
            {
                match sup.lock().unwrap().report_failure(&device_uuid) {
                    Ok(supervisor::SupervisorAction::Restart) => {
                        // Lease→Pipeline: recover 前必须重校租约仍在有效期内 (MEDIA-03 排他不变量).
                        if !lm.is_valid(&device_uuid) {
                            tracing::error!(device = %device_uuid, "recover 中止: lease 失效 (排他不变量)");
                            *agent_state.lock().unwrap() = health::AgentState::ManualRequired;
                            continue;
                        }
                        let backoff = sup.lock().unwrap().backoff(&device_uuid);
                        let _ = sup.lock().unwrap().begin_restart(&device_uuid);
                        std::thread::sleep(backoff);
                        match ctrl.recover(&handle) {
                            Ok(()) => {
                                sup.lock().unwrap().report_recovered(&device_uuid).ok();
                                tracing::warn!(handle = %handle.0, "MEDIA-RT-01 watchdog: recover 成功 (Supervisor→PipelineController.recover 闭环)");
                            }
                            Err(e) => tracing::error!(error = %e, "recover 失败"),
                        }
                    }
                    Ok(supervisor::SupervisorAction::Escalate) => {
                        tracing::error!(handle = %handle.0, "MEDIA-RT-01 watchdog: Escalate (MANUAL_REQUIRED)");
                        *agent_state.lock().unwrap() = health::AgentState::ManualRequired;
                    }
                    Err(e) => tracing::error!(error = %e, "supervisor report_failure 失败"),
                }
            } else if pass {
                *agent_state.lock().unwrap() = health::AgentState::Capturing;
                tracing::info!(
                    handle = %handle.0,
                    video_frames = prev_video,
                    audio_frames = prev_audio,
                    bus_events,
                    "MEDIA-RT-01: A+B+C 全过 (canonical first-buffer 路径健康)"
                );
            } else if tick.is_multiple_of(20) {
                // 诊断: pass 未达成时打印各子项, 便于现场定位 (每 ~10s 一次, 防刷屏).
                let snap = crate::pipeline_events::read_health(&handle).unwrap_or_default();
                tracing::info!(
                    tick = tick,
                    a1 = snap.acceptance.a1_identity_resolved,
                    a2 = snap.acceptance.a2_lease_acquired,
                    a3 = snap.acceptance.a3_pipeline_playing,
                    a4 = snap.acceptance.a4_signal_detected,
                    b1 = snap.acceptance.b1_first_video,
                    b2 = snap.acceptance.b2_first_audio,
                    b3 = snap.acceptance.b3_valid_pts,
                    b4 = snap.acceptance.b4_pts_monotonic,
                    c1 = snap.acceptance.c1_no_unexpected_eos,
                    c2 = snap.acceptance.c2_no_pipeline_error,
                    c3 = snap.acceptance.c3_no_repeated_reneg,
                    c4 = snap.acceptance.c4_counters_continue,
                    cwin_ms = snap.acceptance.c_observed_ms.unwrap_or(0),
                    cwin_cfg = snap.acceptance.c_configured_window_ms,
                    vframes = snap.video_frame_count,
                    aframes = snap.audio_frame_count,
                    vpts = snap.video_first_pts.unwrap_or(0),
                    apts = snap.audio_first_pts.unwrap_or(0),
                    bus = snap.bus_event_count,
                    "MEDIA-RT-01 诊断 (未全过)"
                );
            }
            tick += 1;
        }
    });
}
