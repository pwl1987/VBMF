//! VBMF Media Agent Gates — Diagnostic / Acceptance Root（A2-0 归位后形态）。
//!
//! 五个真机验收 env 的**唯一**入口（生产 media-agent bin 对这些 env 零 dispatch）:
//!   VBMF_CONFIG_PROBE / VBMF_RESOLVER / VBMF_LOOPBACK / VBMF_SESSION_LIFECYCLE /
//!   VBMF_REGISTRY_ONLY
//! Gate 逻辑在 lib `gates/` 模块族（逐字节迁自 main.rs, 行为零变）。
//!
//! 前导构造（config→provider→discovery→FanoutSink 双日志→lease bootstrap→
//! SDK probe→supervisor→agent_state）与生产 Composition Root 同源同序——
//! **TEMPORARY（A20-03 收口点, 用户裁定序）**: 此处为字面复制; bootstrap 抽取
//! （只构造不运行硬边界）落地后 main 与本 bin 收敛到同一 `bootstrap::build()`。
//! 顺序语义: 原 main 中 config_probe/resolver 先于 lease 构造、loopback 先于
//! supervisor——本入口统一先完成全部构造再按原相对序 dispatch（gate 命中即
//! exit, gate 自身 verdict 输出零变; 前置 INFO 行集差异如实记档于 verify 报告）。

use std::sync::Arc;

use media_agent::config;
use media_agent::contracts::provider::HardwareProvider;
use media_agent::device::DeviceInfo;
use media_agent::events;
use media_agent::lease::{self, LeaseManager};

fn main() {
    tracing_subscriber::fmt::init();

    // ── 前导构造（与生产 Composition Root 同序; TEMPORARY 复制, A20-03 收口） ──
    let _cfg = config::Config::from_env();
    for w in _cfg.rpc_bind_security_warnings() {
        tracing::warn!(warning = %w, "rpc_bind 安全");
    }
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
    let discovered = match provider.discover() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("device discovery 失败 (fail-closed): {e}");
            std::process::exit(2);
        }
    };
    let devices: Vec<DeviceInfo> = discovered.iter().map(|d| d.device.clone()).collect();
    tracing::info!(count = devices.len(), "device discovery complete");

    // 双日志分流（P0-7D D3）: loopback gate 消费共享 event_sink/projection_log。
    let projection_log = Arc::new(events::RuntimeEventLog::new());
    let internal_log = Arc::new(events::RuntimeEventLog::new());
    let event_sink: Arc<dyn events::RuntimeEventSink> = Arc::new(events::FanoutSink::new(
        projection_log.clone(),
        internal_log.clone(),
    ));

    let lm = Arc::new(lease::InMemoryLeaseManager::new());
    for d in &devices {
        match lm.acquire(&d.device_id, "bootstrap", std::time::Duration::from_secs(60)) {
            Ok(l) => tracing::info!(device = %l.device_id, "lease acquired"),
            Err(e) => tracing::warn!(error = %e, "lease acquire failed"),
        }
    }
    if let Some(first) = devices.first() {
        match lm.acquire(&first.device_id, "second-owner", std::time::Duration::from_secs(60)) {
            Ok(_) => tracing::warn!("LEASE COLLISION — double-capture risk!"),
            Err(e) => tracing::info!(error = %e, "lease re-acquire correctly rejected"),
        }
    }
    match media_agent::adapters::blackmagic::probe_sdk("libDeckLinkAPI.so") {
        Ok(()) => tracing::info!("SDK libDeckLinkAPI.so reachable, entry symbols present"),
        Err(e) => {
            tracing::warn!(error = %e, "SDK probe failed (expected in container w/o bind-mount)")
        }
    }

    let sup = Arc::new(std::sync::Mutex::new(media_agent::supervisor::Supervisor::new(
        media_agent::supervisor::RestartPolicy::default(),
        event_sink.clone(),
    )));
    for d in &devices {
        sup.lock().unwrap().register(d.device_id);
    }
    tracing::info!(watched = devices.len(), "supervisor initialized");
    let agent_state = Arc::new(std::sync::Mutex::new(media_agent::health::AgentState::Ready));

    // ── Gate dispatch（原 main 相对序; 命中即 exit, 未命中落到末尾 registry 探针） ──
    #[cfg(feature = "bmd-provider")]
    media_agent::gates::config_probe::run();

    #[cfg(feature = "gstreamer-backend")]
    media_agent::gates::resolver::run(&_cfg, &discovered);

    #[cfg(feature = "gstreamer-backend")]
    media_agent::gates::loopback::run(&_cfg, &discovered, &event_sink, &projection_log);

    #[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
    media_agent::gates::session_lifecycle::run(
        &_cfg,
        &devices,
        &discovered,
        &lm,
        &sup,
        &agent_state,
        &event_sink,
        &projection_log,
        &internal_log,
    );

    // feature 组合裁掉全部 gate 调用点时, 前导构造值防 unused（clippy -D 红线;
    // 语义零影响——绑定丢弃）。
    let _ = (
        &_cfg,
        &devices,
        &discovered,
        &lm,
        &sup,
        &agent_state,
        &event_sink,
        &projection_log,
        &internal_log,
    );

    // REGISTRY_ONLY 置底（原 main 中该探针在 supervisor 之后; 未命中任何 gate 时
    // 本 bin 无事可做——显式提示后退出, 绝不进入生产 runtime 循环）。
    #[cfg(feature = "hardware-test")]
    media_agent::gates::registry::run();

    eprintln!(
        "media-agent-gates: 未命中任何 gate env \
         (VBMF_CONFIG_PROBE / VBMF_RESOLVER / VBMF_LOOPBACK / VBMF_SESSION_LIFECYCLE / VBMF_REGISTRY_ONLY)"
    );
    std::process::exit(2);
}
