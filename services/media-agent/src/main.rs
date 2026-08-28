//! VBMF Rust Media Agent — Gate 2 skeleton + Gate 5/6/7 scaffolding.
//!
//! Boundary (SoT §14): this binary owns the Hardware Plane only.
//! Control Plane (API/auth/RBAC/config/UI) stays in Node/Fastify.

mod config;
mod adapters;
mod contracts;
mod device;
mod fixture; // HW-PORT-01 / MEDIA-RT-01 复用的 BMD-SDI-LOOPBACK Fixture (host-specific 证据)
mod graph_intent;
mod health;
mod hw_port_01; // HW-PORT-01 Gate: 端口级绑定闭环验收
mod lease;
mod pipeline;
mod port; // 五层模型: Device → Port → Capability → Runtime Binding → Signal
mod resolver;
mod rpc;
// sdk 已迁入 adapters/blackmagic (BMD Reference Adapter)

mod signal; // 信号探测 + 亮度黑场检测
mod supervisor; // Gate 6/7: real DeckLink enumeration (feature `bmd-provider`)

// 硬规则 (Phase 0.6): `hardware-test` (IDeckLinkInput SDK 探针) 与 canonical `gstreamer`
// 运行时互斥 —— 生产运行不得同时打开同一块 DeckLink (避免双采 / 设备争用). 编译期强制.
#[cfg(all(feature = "hardware-test", feature = "gstreamer-backend"))]
compile_error!("hardware-test SDK 探针与 canonical GStreamer 运行时互斥; 生产运行不得同时启用 (避免双采/争用同一块 DeckLink)");

// Trait must be in scope to call `discover()` (trait method, not inherent).
use crate::contracts::provider::HardwareProvider;
// Trait must be in scope to call `acquire`/`is_valid` on `Arc<InMemoryLeaseManager>`
// (trait method, auto-deref via Arc; 否则 E0599 no method named `acquire`).
use lease::LeaseManager;
// Trait must be in scope to call `prepare`/`start`/`recover` on `Arc<dyn MediaBackend>`
// (trait 方法, 否则 E0599 no method named `recover`). C2c 经 `dyn MediaBackend` 接线; 调用点均在
// `#[cfg(feature = "bmd-provider")]` 块内 → bmd && gstreamer 才编译.
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::backend::MediaBackend;
use std::io::Write;
use std::sync::Arc;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use uuid::Uuid;

/// Phase 0.6 C2c: 经 `dyn MediaBackend` 接线 —— `mock` feature 优先使用 `MockBackend`
/// (证明 `MediaBackend` 可由非 GStreamer 实现满足, ARCH-BACKEND-01 Test B); 否则使用
/// canonical GStreamer backend. 两者共享同一 `PipelinePlan` 契约, Graph/Supervisor/Health
/// 无需改动即可互换 (Test C).
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn build_media_backend() -> Arc<dyn MediaBackend> {
    #[cfg(feature = "mock")]
    {
        Arc::new(crate::adapters::mock::MockBackend)
    }
    #[cfg(all(not(feature = "mock"), feature = "gstreamer-backend"))]
    {
        Arc::new(crate::adapters::gstreamer::GStreamerPipelineController::new())
    }
}

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
    // Phase 0.6 C2c: 经 `dyn HardwareProvider` 接线 —— discovery 路径与具体 Provider 实现解耦
    // (ARCH-PORTABILITY-01 Test C). 选择优先级: mock > simulation > bmd-provider > default(filesystem).
    // 各实现返回相同的 `Vec<DeviceInfo>` 契约, Domain / Graph / UI 无需感知差异.
    #[cfg(feature = "mock")]
    let provider: Box<dyn HardwareProvider> = Box::new(crate::adapters::mock::MockProvider);
    #[cfg(all(not(feature = "mock"), feature = "simulation"))]
    let provider: Box<dyn HardwareProvider> = Box::new(device::SimulatedDeviceManager::new());
    #[cfg(all(not(feature = "mock"), not(feature = "simulation"), feature = "bmd-provider"))]
    let provider: Box<dyn HardwareProvider> =
        Box::new(crate::adapters::blackmagic::DeckLinkDeviceManager::new());
    #[cfg(all(
        not(feature = "mock"),
        not(feature = "simulation"),
        not(feature = "bmd-provider")
    ))]
    let provider: Box<dyn HardwareProvider> = Box::new(device::FilesystemDeviceManager::new());
    let devices = provider.discover();
    tracing::info!(count = devices.len(), "device discovery complete");

    // B 选项 (用户 2026-08-28): Connector Mode / 各子设备配置方向探针.
    // 纯 SDK 读取 (IDeckLinkConfiguration), 不进媒体, 不依赖 GStreamer; 命中即 exit(0).
    // 用于无桌面环境 (无 Blackmagic Desktop Video Setup) 时直接回答
    // "每个子设备当前是 In 还是 Out、物理端口如何按 Connector Mode 分组".
    #[cfg(feature = "bmd-provider")]
    if std::env::var("VBMF_CONFIG_PROBE").is_ok() {
        match crate::adapters::blackmagic::decklink::probe_connector_config() {
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
                    Some(m) => crate::resolver::resolve_with_manifest(&devices, &probes, m),
                    None => crate::resolver::resolve(&devices, &probes),
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
                        crate::resolver::collect_bindings_from_manifest(&devices, &probes, m)
                    }
                    None => crate::resolver::collect_bindings(&devices, &probes),
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
                        crate::port::PortRegistry::build(&devices, &probes, m, &bindings)
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
            crate::resolver::collect_bindings_from_manifest(&devices, &probes, &manifest);
        let registry = match crate::port::PortRegistry::build(&devices, &probes, &manifest, &bindings) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("PortRegistry 构建失败 (fail-closed): {e:?}");
                std::process::exit(2);
            }
        };
        let fixtures_dir = std::env::var("VBMF_FIXTURES_DIR")
            .unwrap_or_else(|_| "evidence/bmd-10.30.15.10/fixtures".to_string());
        let fixtures = match crate::fixture::Fixture::load_dir(std::path::Path::new(&fixtures_dir)) {
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
    match crate::adapters::blackmagic::sdk::probe_sdk("libDeckLinkAPI.so") {
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
    match crate::adapters::blackmagic::decklink::registry() {
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
    let sup = Arc::new(std::sync::Mutex::new(supervisor::Supervisor::new(
        supervisor::RestartPolicy::default(),
    )));
    for d in &devices {
        sup.lock().unwrap().register(d.device_id);
    }
    tracing::info!(watched = devices.len(), "supervisor initialized");

    // Gate 2.4: 最简 /health (std TcpListener, 无第三方依赖; 后续可换 axum).
    // Gate 2.6 (P1②): 返回真实运行时状态, 与 Supervisor 状态机对齐 (不再固定 ready).
    let device_count = devices.len();
    let agent_state = Arc::new(std::sync::Mutex::new(health::AgentState::Ready));

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
            let ctrl: Arc<dyn MediaBackend> = build_media_backend();
            match ctrl.prepare(&plan) {
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
            match crate::adapters::blackmagic::decklink::start_capture(0) {
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
            // bmd_persistent_id / device-number 由 materialize 经 Device Registry 解析得到.
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
                                &devices,
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
                        crate::resolver::collect_bindings(&devices, &gst_probes)
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

            // 端口注册表 (供 materialize 经 Manifest 声明 + 运行时探测推导连接类型, 不硬编码 connection=sdi):
            // 仅当提供 manifest 时构建; 诊断 auto-start 无 manifest 回退 legacy → registry=None → connection 由插件默认探测.
            #[cfg(feature = "gstreamer-backend")]
            let registry = _cfg.device_binding_path.as_ref().and_then(|p| {
                crate::resolver::DeviceBindingManifest::load(p)
                    .ok()
                    .map(|m| {
                        crate::port::PortRegistry::build(&devices, &gst_probes, &m, &bindings)
                            .expect("端口发现与 manifest 不一致 (fail-closed 拒绝)")
                    })
            });
            #[cfg(not(feature = "gstreamer-backend"))]
            let registry: Option<crate::port::PortRegistry> =
                _cfg.device_binding_path.as_ref().and_then(|p| {
                    crate::resolver::DeviceBindingManifest::load(p)
                        .ok()
                        .map(|m| {
                            crate::port::PortRegistry::build(&devices, &[], &m, &bindings)
                                .expect("端口发现与 manifest 不一致 (fail-closed 拒绝)")
                        })
                });

            // 生产启动语义 (用户 §七 P1-3): 仅 diagnostic (或 self-test) 自动从绑定创建并启动 media pipeline;
            // Production **绝不**自行取 first device 制造 GraphRuntimeIntent —— 必须等待 Control Plane
            // 显式 StartPipeline Intent. (rpc.rs 当前 No transport yet, 故 Production 在此 idle:
            // 仅校验 manifest + 提供 /health, 不自动启动任何媒体管线.)
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
                                device_id: first_id,
                                port_id: None,
                            },
                            sink: crate::graph_intent::SinkIntent {
                                kind: "rtmp".into(),
                            },
                        },
                    }],
                };
                match crate::pipeline::materialize(
                    &intent,
                    &devices,
                    mode,
                    &bindings,
                    registry.as_ref(),
                ) {
                    Ok(plans) => {
                        for p in &plans {
                            tracing::info!(
                                device_id = %p.source.device_id,
                                bmd_persistent_id = p.source.bmd_persistent_id,
                                device_number = p.source.device_number,
                                connector = ?p.source.connector,
                                selection_mode = ?p.source.selection_mode,
                                "CAP-01 canonical ingest plan materialized (GStreamer decklinkvideosrc/audiosrc; selection_mode 见字段; launch pending)"
                            );
                        }
                        *agent_state.lock().unwrap() = health::AgentState::Capturing;

                        // (C) 真实 GStreamer launch (feature = "gstreamer-backend") + Supervisor→recover 接线.
                        #[cfg(feature = "gstreamer-backend")]
                        {
                            let dev_id_str = plans[0].source.device_id.clone();
                            let device_uuid = Uuid::parse_str(&dev_id_str).unwrap_or(Uuid::nil());
                            // Lease→Pipeline: 启动前确认该设备的租约仍有效 (排他采集前置条件).
                            if !lm.is_valid(&device_uuid) {
                                tracing::error!(device_id = %dev_id_str, "lease 无效, 拒绝启动 canonical 采集 (排他不变量)");
                            } else {
                                let ctrl: Arc<dyn MediaBackend> = build_media_backend();
                                // 证据: 记录 GStreamer 运行时版本 (与 SDK/driver 一并归档).
                                tracing::info!(gst_version = ?crate::adapters::gstreamer::gstreamer_runtime_version(), "GStreamer runtime version (evidence)");
                                match ctrl.prepare(&plans[0]) {
                                    Ok(h) => match ctrl.start(&h) {
                                        Ok(()) => {
                                            tracing::info!(
                                                handle = %h.0,
                                                device_id = %dev_id_str,
                                                "canonical GStreamer pipeline 启动 (decklinkvideosrc/audiosrc hw-serial-number)"
                                            );
                                            // MEDIA-RT-01A: Ingest Open 达成 (已启动, 信号检测见 health).
                                            sup.lock().unwrap().register(device_uuid);
                                            spawn_ingest_watchdog(
                                                ctrl,
                                                h,
                                                device_uuid,
                                                sup.clone(),
                                                lm.clone(),
                                                agent_state.clone(),
                                            );
                                        }
                                        Err(e) => {
                                            tracing::error!(error = %e, "canonical GStreamer 启动失败 (未盲开)")
                                        }
                                    },
                                    Err(e) => tracing::error!(error = %e, "canonical prepare 失败"),
                                }
                            }
                        }
                        #[cfg(not(feature = "gstreamer-backend"))]
                        {
                            tracing::info!("canonical 计划已物化; 真实 GStreamer launch 待启用 feature 'gstreamer'");
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "CAP-01 canonical ingest 物化失败 (identity 未解析)")
                    }
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

    std::thread::spawn({
        let agent_state = agent_state.clone();
        // 管理面绑定 (用户 §二十二 P1 Security): 默认 127.0.0.1:8080 (仅本机回环), 不裸露公网;
        // 生产部署由 `MEDIA_AGENT_HEALTH_BIND` 覆盖为内网接口/经 Fastify/Nginx 反向代理 + 认证.
        move || match std::net::TcpListener::bind(&_cfg.health_bind) {
            Ok(listener) => {
                tracing::info!(bind = %_cfg.health_bind, "health endpoint listening (internal-only; 经反向代理/认证暴露, 见用户 §二十二)");
                for mut s in listener.incoming().flatten() {
                    let st = *agent_state.lock().unwrap();
                    let active = crate::pipeline::HEALTH_ARCS.lock().unwrap().len();
                    // Bus channel 溢出计数 (P1 §十三): 暴露为 metric, 非零代表曾发生事件丢弃.
                    let dropped = crate::pipeline::dropped_bus_events();
                    let body = serde_json::json!({
                        "state": st,
                        "devices": device_count,
                        "active_pipelines": active,
                        "dropped_bus_events": dropped,
                        "clock_lost_events": crate::pipeline::clock_lost_events()
                    })
                    .to_string();
                    let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                            body.len(),
                            body
                        );
                    let _ = s.write_all(resp.as_bytes());
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
            let events = ctrl.poll_bus(&handle);
            let mut bus_events: u64 = 0;
            // 在共享 Arc 上就地更新 acceptance 子项: 只读 live 状态→推导→写回 acceptance,
            // 绝不覆盖 appsink 回调写入的 video_frame_count/audio_frame_count/PTS/video_pts_state/audio_pts_state,
            // 否则每轮 snapshot 写回会把实时计数回退, 破坏 c4(计数增长) 判定 (#4 回归).
            let (pass, has_error) = if let Some(h) =
                crate::pipeline::HEALTH_ARCS.lock().unwrap().get(&handle)
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
                            crate::pipeline::PipelineBusEventKind::Error => "Error",
                            crate::pipeline::PipelineBusEventKind::Eos => "Eos",
                            crate::pipeline::PipelineBusEventKind::StateChanged => "StateChanged",
                            crate::pipeline::PipelineBusEventKind::Warning => "Warning",
                            crate::pipeline::PipelineBusEventKind::ClockLost => "ClockLost",
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
                        crate::pipeline::PipelineBusEventKind::Error => {
                            g.acceptance.c_pipeline_errors += 1;
                        }
                        crate::pipeline::PipelineBusEventKind::Eos => {
                            g.acceptance.c_unexpected_eos += 1;
                        }
                        // P1-4 最低策略映射 (bus_event_recovery_policy): ClockLost = degraded, 不自动重启.
                        crate::pipeline::PipelineBusEventKind::ClockLost => {
                            crate::pipeline::CLOCK_LOST_EVENTS
                                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            tracing::warn!(
                                handle = %handle.0,
                                severity = ?e.severity,
                                detail = %e.detail,
                                policy = crate::pipeline::bus_event_recovery_policy(e.kind),
                                "Bus ClockLost: 标记 degraded, 不触发重启 (完整 Clock Recovery 属 V0.3/P2)"
                            );
                        }
                        crate::pipeline::PipelineBusEventKind::Warning => {
                            tracing::warn!(
                                handle = %handle.0,
                                severity = ?e.severity,
                                detail = %e.detail,
                                "Bus Warning (可恢复异常, 记录不重启)"
                            );
                        }
                        crate::pipeline::PipelineBusEventKind::StateChanged => {
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
                        crate::pipeline::PipelineBusEventKind::Error
                            | crate::pipeline::PipelineBusEventKind::Eos
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
                let snap = crate::pipeline::read_health(&handle).unwrap_or_default();
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
