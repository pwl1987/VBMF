//! VBMF Media Agent — Production Composition Root（A2-0 归位后形态）。
//!
//! 只承担: config → adapter selection → dependency construction → runtime
//! wiring（诊断 auto-start / transport）→ process lifetime。
//! **对全部 VBMF_* 验收 env 零 dispatch 责任**——真机 gate 入口统一在
//! `media-agent-gates` bin（src/bin/gates.rs）; Gate 逻辑见 lib `gates/` 模块族。

use std::sync::Arc;

use media_agent::config;
use media_agent::contracts::provider::HardwareProvider;
use media_agent::device::DeviceInfo;
use media_agent::events;
use media_agent::health;
use media_agent::lease::{self, LeaseManager};
use media_agent::supervisor;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use media_agent::contracts::backend::MediaBackend;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use media_agent::watchdog::spawn_ingest_watchdog;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use uuid::Uuid;

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
    let provider: Box<dyn HardwareProvider> = media_agent::registry::AdapterRegistry::build_provider()
        .unwrap_or_else(|e| {
            eprintln!("adapter feature 冲突 (fail-closed): {e}");
            std::process::exit(2);
        });
    let (active_provider, active_backend) = media_agent::registry::active_adapters();
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



    // P0-7D D3 (双日志分流定稿): 外送投影日志 (transport 投影端点 + gate 证据路径 drain) 与
    // 内消费日志 (watchdog tick drain → health::reduce 派生 AgentState) 各自独立维持
    // 0.7C-6 四语义; 生产者统一经 FanoutSink 同序双写 (emit 永不阻塞/永不失败)。
    let projection_log = Arc::new(events::RuntimeEventLog::new());
    let internal_log = Arc::new(events::RuntimeEventLog::new());
    let event_sink: Arc<dyn events::RuntimeEventSink> = Arc::new(events::FanoutSink::new(
        projection_log.clone(),
        internal_log.clone(),
    ));

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
    match media_agent::adapters::blackmagic::probe_sdk("libDeckLinkAPI.so") {
        Ok(()) => tracing::info!("SDK libDeckLinkAPI.so reachable, entry symbols present"),
        Err(e) => {
            tracing::warn!(error = %e, "SDK probe failed (expected in container w/o bind-mount)")
        }
    }

    // Gate 2.6 (P1①): bmd feature 下 `devices` 已直接来自 `DeckLinkDeviceManager`
    // (基于 DeckLink DeviceHandle 派生的 canonical identity), 不再按索引把 SDK 枚举回填
    // filesystem 列表 (拓扑变化后 index 会漂移, 见 device.rs).

    // Gate 7 (feature `hardware-test`): verbose Device Registry (model/serial/status) for BMD.
    // A2-0: 无条件打印保留（hardware-test 特性 boot 行为）; VBMF_REGISTRY_ONLY
    // env 入口已迁 gates::registry（生产 bin 对该 env 零 dispatch 责任）。
    #[cfg(feature = "hardware-test")]
    media_agent::gates::registry::print_registry();

    // Gate 5: Supervisor seeded with device handles (watchdog state machine + budget/
    // backoff/circuit-breaker are unit-tested in supervisor.rs). 包 Arc<Mutex> 以便 watch
    // 线程与 GStreamer recover 接线共享 (Supervisor 只决策, 不碰 GStreamer).
    // 0.7C-6 D8 + 0.7D D3: 生产者 (SessionManager/Supervisor/ingest/点亮) 统一经
    // FanoutSink 同序双写 projection+internal 两日志 (上方已构造); Supervisor 持 sink。
    let sup = Arc::new(std::sync::Mutex::new(supervisor::Supervisor::new(
        supervisor::RestartPolicy::default(),
        event_sink.clone(),
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
    let mut api_mgr: Option<std::sync::Arc<media_agent::session::SessionManager>> = None;

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
            let plan = media_agent::pipeline::PipelinePlan::self_test();
            let ctrl: Arc<dyn MediaBackend> =
                media_agent::registry::AdapterRegistry::build_media_backend().unwrap_or_else(|e| {
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
                            event_sink.clone(),
                            internal_log.clone(),
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
            match media_agent::adapters::blackmagic::start_capture(0) {
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
                Ok("diagnostic") => media_agent::pipeline::MaterializeMode::Diagnostic,
                _ => media_agent::pipeline::MaterializeMode::Production,
            };
            // Resolver 绑定 (物化前置): gstreamer 构建下探测并解析; 非 gstreamer 构建为空 map.
            #[cfg(feature = "gstreamer-backend")]
            let gst_probes = match media_agent::resolver::probe_gstreamer_devices(
                media_agent::resolver::MAX_PROBE_DEVICES,
                _cfg.device_binding_path.is_none(),
            ) {
                media_agent::resolver::GstProbeOutcome::Available { probes, errors } => {
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
                Some(p) => match media_agent::resolver::DeviceBindingManifest::load(p) {
                    Ok(m) => {
                        // (a) 结构完整性校验 (唯一性/非空 machine_id): 失败即拒绝 (ManifestInvalid).
                        let structural = m.validate_manifest();
                        // (b) 主机身份校验: 不符 → 失败闭合 (拒绝, 非 warning, 用户 §五).
                        let machine_ok =
                            m.check_machine_identity(&media_agent::resolver::current_machine_id());
                        if let Err(e) = &structural {
                            tracing::error!(error = %e, "DeviceBindingManifest 结构校验失败; 生产绑定失败闭合, 拒绝 materialize");
                            std::collections::HashMap::new()
                        } else if let Err(e) = &machine_ok {
                            tracing::error!(error = %e, "DeviceBindingManifest 主机身份不符; 生产绑定失败闭合, 拒绝 materialize (非 warning)");
                            std::collections::HashMap::new()
                        } else {
                            // (c) 版本一致性软告警 (P1-2: 已接真实 runtime 版本, 非 None).
                            let sdk_v = media_agent::resolver::declared_bmd_sdk_version();
                            let gst_v = media_agent::resolver::actual_gstreamer_version();
                            let plugin_v = media_agent::resolver::actual_decklink_plugin_version()
                                .unwrap_or_else(|| "unknown".to_string());
                            // P1-1: 真实运行时 SDK 身份 provenance (build include + libDeckLinkAPI.so).
                            tracing::info!(detected_sdk = %media_agent::resolver::detected_bmd_sdk_version(), "BMD SDK runtime identity (provenance)");
                            for w in
                                m.validate_environment(Some(&sdk_v), Some(&plugin_v), Some(&gst_v))
                            {
                                tracing::warn!(warning = %w, "device-binding manifest 版本校验");
                            }
                            media_agent::resolver::collect_bindings_from_manifest(
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
                    media_agent::pipeline::MaterializeMode::Diagnostic => {
                        tracing::warn!("未提供 DeviceBindingManifest; diagnostic 模式显式回退 legacy auto-resolution (仅排障, 生产禁用)");
                        media_agent::resolver::collect_bindings(&discovered, &gst_probes)
                    }
                    media_agent::pipeline::MaterializeMode::Production => {
                        tracing::error!("生产模式未提供 DeviceBindingManifest; 绑定失败闭合, 拒绝 materialize (不回退 legacy, 用户 §四)");
                        std::collections::HashMap::new()
                    }
                },
            };
            #[cfg(not(feature = "gstreamer-backend"))]
            let bindings: std::collections::HashMap<
                uuid::Uuid,
                media_agent::resolver::ResolvedDeviceBinding,
            > = std::collections::HashMap::new();
            #[cfg(not(feature = "gstreamer-backend"))]
            let _ = &bindings;

            // 端口注册表 (供 materialize 经 Manifest 声明 + 运行时探测推导连接类型, 不硬编码 connection=sdi):
            // 仅当提供 manifest 时构建; 诊断 auto-start 无 manifest 回退 legacy → registry=None → connection 由插件默认探测.
            #[cfg(feature = "gstreamer-backend")]
            let registry = _cfg.device_binding_path.as_ref().and_then(|p| {
                media_agent::resolver::DeviceBindingManifest::load(p)
                    .ok()
                    .map(|m| {
                        media_agent::port::PortRegistry::build(&discovered, &gst_probes, &m, &bindings)
                            .expect("端口发现与 manifest 不一致 (fail-closed 拒绝)")
                    })
            });
            // 非 gst 构建 (hardware-test): 端口注册表由真机闭环/会话路径消费, 此处不构建
            // (P0-7A 起无 gst 的物化路径已不存在, registry 留空避免 unused)。
            #[cfg(not(feature = "gstreamer-backend"))]
            let _registry: Option<media_agent::port::PortRegistry> = None;


            // 生产启动语义 (用户 §七 P1-3): 仅 diagnostic (或 self-test) 自动从绑定创建并启动 media pipeline;
            // Production **绝不**自行取 first device 制造 GraphRuntimeIntent —— 必须等待 Control Plane
            // 显式 StartPipeline Intent. (rpc.rs 当前 No transport yet, 故 Production 在此 idle:
            // 仅校验 manifest + 提供 /health, 不自动启动任何媒体管线.)
            // P0.7C-8: api_mgr 已提升到 main body 顶层 (见 agent_state 之后), 此处仅赋值。
            let auto_start = matches!(mode, media_agent::pipeline::MaterializeMode::Diagnostic);
            if auto_start {
                // Alpha-1: 诊断多输入——`VBMF_DIAG_INPUTS`（默认 1 = 现行为）取**已绑定**
                // 设备前 N 个（未绑定/输出卡不含; N 超可用数 ⇒ 取全部并 warn）。
                // review Important#2: 无任何绑定设备 ⇒ 回退 devices.first()（单输入
                // 兼容冻结点——绝不让 intent 落空）; 非法 N 值 warn 后按 1。
                let diag_inputs = match std::env::var("VBMF_DIAG_INPUTS") {
                    Ok(v) => match v.parse::<usize>() {
                        Ok(n) if n >= 1 => n,
                        _ => {
                            println!("WARN: VBMF_DIAG_INPUTS={v:?} 非法（须 ≥1 整数）⇒ 按 1");
                            1
                        }
                    },
                    Err(_) => 1,
                };
                let bound: Vec<String> = devices
                    .iter()
                    .filter(|d| bindings.contains_key(&d.device_id))
                    .map(|d| d.device_id.to_string())
                    .collect();
                if diag_inputs > bound.len() && !bound.is_empty() {
                    println!(
                        "WARN: VBMF_DIAG_INPUTS={diag_inputs} 超已绑定设备数 {} ⇒ 取全部",
                        bound.len()
                    );
                }
                let mut diag_ids: Vec<String> = bound.into_iter().take(diag_inputs).collect();
                if diag_ids.is_empty() {
                    if let Some(d) = devices.first() {
                        println!(
                            "WARN: 无已绑定设备 ⇒ 诊断 intent 回退首设备 {}（单输入兼容）",
                            d.device_id
                        );
                        diag_ids.push(d.device_id.to_string());
                    }
                }
                let first_id = diag_ids.first().cloned().unwrap_or_else(|| {
                    devices
                        .first()
                        .map(|d| d.device_id.to_string())
                        .unwrap_or_default()
                });
                // P1a: 诊断主会话 sink kind —— `VBMF_OUTPUT_KIND` 覆盖（hls/rtmp gate 用）;
                // 默认 "rtmp" 与 P1a 前逐字节一致。无任何 VBMF_OUTPUT_* ⇒ materialize
                // fail-soft 降级纯分析（向后兼容承诺, Design Doc §6）。
                // 同一 cfg 供降级可见性检查（默认订阅 ERROR 级, tracing warn 不可见 →
                // 启动面显式打印, review Important#2）。
                let out_cfg = media_agent::config::PrototypeOutputConfig::from_env();
                let diag_sink_kind = out_cfg.sink_kind_override.unwrap_or_else(|| "rtmp".into());
                match diag_sink_kind.as_str() {
                    "hls" if out_cfg.hls_dir.is_none() => println!(
                        "WARN: sink.kind=hls 但 VBMF_OUTPUT_HLS_DIR 未设 ⇒ 降级纯分析 (fail-soft)"
                    ),
                    "rtmp" if out_cfg.rtmp_url.is_none() => println!(
                        "WARN: sink.kind=rtmp 但 VBMF_OUTPUT_RTMP_URL 未设 ⇒ 降级纯分析 (fail-soft)"
                    ),
                    _ => {}
                }
                let intent = media_agent::graph_intent::GraphRuntimeIntent {
                    version: "1.0".into(),
                    // Alpha-1: 多输入 intent（N=VBMF_DIAG_INPUTS; 首设备承载输出声明,
                    // 次设备 materialize 强制纯分析——单输出承诺）。
                    devices: diag_ids
                        .iter()
                        .map(|id| media_agent::graph_intent::DeviceIntent {
                            device_id: id.clone(),
                            role: "CAPTURE".into(),
                            pipeline: media_agent::graph_intent::PipelineIntent {
                                source: media_agent::graph_intent::SourceIntent {
                                    kind: "decklink".into(),
                                    device_id: id.clone(),
                                    port_id: None,
                                },
                                sink: media_agent::graph_intent::SinkIntent {
                                    kind: diag_sink_kind.clone(),
                                },
                            },
                        })
                        .collect(),
                };

                #[cfg(feature = "gstreamer-backend")]
                {
                    // P0-7A: 生命周期由 SessionManager 唯一拥有 (RUNTIME_SESSION_MODEL §4.1) —
                    // create = Preflight→Reserve→建档→Lease→Binding verify (失败逆序回滚零孤儿);
                    // start = materialize→instantiate→Allocate→Backend.start→Running。
                    let resources = media_agent::resource::SharedResourceRegistry::new(
                        registry
                            .as_ref()
                            .map(|reg| {
                                media_agent::resource::ResourceRegistry::derive_from_discovery(reg)
                            })
                            .unwrap_or_default(),
                    );
                    let ctrl: std::sync::Arc<dyn MediaBackend> =
                        media_agent::registry::AdapterRegistry::build_media_backend().unwrap_or_else(
                            |e| {
                                eprintln!("adapter feature 冲突 (fail-closed): {e}");
                                std::process::exit(2);
                            },
                        );
                    // P0.7C-8: Arc 化 (tick 线程 + transport 上下文共享; 原 mgr 被 tick 线程 move,
                    // 共享须 Arc; 既有 mgr.xxx() 调用经 Arc 透传, 零语义变化)。
                    let mgr: std::sync::Arc<media_agent::session::SessionManager> =
                        std::sync::Arc::new(media_agent::session::SessionManager::new(
                            resources,
                            lm.clone(),
                            sup.clone(),
                            ctrl.clone(),
                            std::sync::Arc::new(devices.clone()),
                            std::sync::Arc::new(bindings.clone()),
                            registry.clone(),
                            mode,
                            media_agent::session::SessionTuning::default(),
                            event_sink.clone(),
                        ));
                    api_mgr = Some(mgr.clone());
                    let dev_uuid = Uuid::parse_str(&first_id).unwrap_or(Uuid::nil());
                    // bootstrap 占位租约让位: 真实会话租约接管排他性 (P0-7A)。
                    // Alpha-1: **全部**诊断输入设备的占位租约让位（多输入; 单输入行为不变）。
                    for id in std::iter::once(&first_id).chain(diag_ids.iter().skip(1)) {
                        let _ = lm.release(&media_agent::lease::DeviceLease {
                            device_id: Uuid::parse_str(id).unwrap_or(Uuid::nil()),
                            owner: "bootstrap".into(),
                            acquired_at: chrono::Utc::now(),
                            ttl: std::time::Duration::from_secs(60),
                        });
                    }
                    match mgr.create(intent.clone()) {
                        Ok(sid) => match mgr.start(&sid) {
                            Ok(()) => {
                                tracing::info!(gst_version = ?media_agent::adapters::gstreamer::gstreamer_runtime_version(), "GStreamer runtime version (evidence)");
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
                                        event_sink.clone(),
                                        internal_log.clone(),
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
    let transport_ctx = media_agent::transport::TransportContext {
        events: projection_log.clone(),
        agent_state: agent_state.clone(),
        device_count,
        query: api_mgr
            .as_ref()
            .map(|m| std::sync::Arc::new(media_agent::runtime_query::RuntimeQuery::new(m.clone()))),
        idem: api_mgr
            .as_ref()
            .map(|m| std::sync::Arc::new(media_agent::idempotency::CommandIdempotency::new(m.clone()))),
        // P1b: 静态文件面 /hls/* 目录（A 方案; 诊断输出配置接线, 生产/未配置 None ⇒ 503）。
        hls_dir: media_agent::config::PrototypeOutputConfig::from_env().hls_dir,
    };

    std::thread::spawn({
        let transport_ctx = transport_ctx.clone();
        // 管理面绑定 (用户 §二十二 P1 Security): 默认 127.0.0.1:8080 (仅本机回环), 不裸露公网;
        // 生产部署由 `MEDIA_AGENT_HEALTH_BIND` 覆盖为内网接口/经 Fastify/Nginx 反向代理 + 认证.
        move || match std::net::TcpListener::bind(&_cfg.health_bind) {
            Ok(listener) => {
                tracing::info!(bind = %_cfg.health_bind, "health+api endpoints listening (internal-only; 经反向代理/认证暴露, 见用户 §二十二)");
                for s in listener.incoming().flatten() {
                    // P1b: socket 超时（review Important#4）——串行 accept 循环下防单个
                    // 停滞读者/空闲连接永久占住唯一 listener（原型级加固; 正式化记档 §7）。
                    let _ = s.set_read_timeout(Some(std::time::Duration::from_secs(10)));
                    let _ = s.set_write_timeout(Some(std::time::Duration::from_secs(30)));
                    media_agent::transport::serve_connection(s, &transport_ctx);
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

