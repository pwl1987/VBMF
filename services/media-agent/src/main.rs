//! VBMF Rust Media Agent — Gate 2 skeleton + Gate 5/6/7 scaffolding.
//!
//! Boundary (SoT §14): this binary owns the Hardware Plane only.
//! Control Plane (API/auth/RBAC/config/UI) stays in Node/Fastify.

mod config;
mod device;
mod graph_intent;
mod health;
mod lease;
mod pipeline;
mod rpc;
mod sdk;
mod supervisor;
mod decklink; // Gate 6/7: real DeckLink enumeration (feature `bmd`)

// Trait must be in scope to call `discover()` (trait method, not inherent).
use device::DeviceManager;
use std::io::Write;
use lease::LeaseManager;

fn main() {
    tracing_subscriber::fmt::init();

    // Gate 2.1: load config shape from env (no behavior attached yet).
    let _cfg = config::Config::from_env();

    // Gate 2.2: device discovery.
    // `simulation` => mock devices (CI/tests, no hardware or SDK).
    // default / bmd => filesystem probe (safe on CI / non-BMD; real on BMD).
    #[cfg(feature = "simulation")]
    let dm = device::SimulatedDeviceManager::new();
    #[cfg(all(not(feature = "simulation"), feature = "bmd"))]
    let dm = device::DeckLinkDeviceManager::new();
    #[cfg(all(not(feature = "simulation"), not(feature = "bmd")))]
    let dm = device::FilesystemDeviceManager::new();
    let devices = dm.discover();
    tracing::info!(count = devices.len(), "device discovery complete");

    // Gate 2.3: lease manager (in-memory; no hardware needed for the interface).
    let lm = lease::InMemoryLeaseManager::new();
    for d in &devices {
        match lm.acquire(&d.device_id, "bootstrap", std::time::Duration::from_secs(60)) {
            Ok(l) => tracing::info!(device = %l.device_id, "lease acquired"),
            Err(e) => tracing::warn!(error = %e, "lease acquire failed"),
        }
    }
    // 排他性不变量: 同一设备重复 acquire 必须被拒 (防 host ffmpeg / 双采).
    if let Some(first) = devices.first() {
        match lm.acquire(&first.device_id, "second-owner", std::time::Duration::from_secs(60)) {
            Ok(_) => tracing::warn!("LEASE COLLISION — double-capture risk!"),
            Err(e) => tracing::info!(error = %e, "lease re-acquire correctly rejected"),
        }
    }

    // Gate 2.5 (A): DeckLink SDK FFI smoke — 验证 libDeckLinkAPI.so 在运行环境可达.
    // 宿主机(/usr/lib 默认路径)应成功; Option B 容器若不 bind-mount 库则 warn(预期).
    match sdk::probe_sdk("libDeckLinkAPI.so") {
        Ok(()) => tracing::info!("SDK libDeckLinkAPI.so reachable, entry symbols present"),
        Err(e) => tracing::warn!(error = %e, "SDK probe failed (expected in container w/o bind-mount)"),
    }

    // Gate 2.6 (P1①): bmd feature 下 `devices` 已直接来自 `DeckLinkDeviceManager`
    // (基于 DeckLink DeviceHandle 派生的 canonical identity), 不再按索引把 SDK 枚举回填
    // filesystem 列表 (拓扑变化后 index 会漂移, 见 device.rs).

    // Gate 7 (feature `hardware-test`): verbose Device Registry (model/serial/status) for BMD.
    #[cfg(feature = "hardware-test")]
    match decklink::registry() {
        Ok(table) => tracing::info!("DeckLink Device Registry:\n{table}"),
        Err(e) => tracing::warn!(error = %e, "registry unavailable"),
    }

    // Gate 5: Supervisor seeded with device handles (watchdog state machine + budget/
    // backoff/circuit-breaker are unit-tested in supervisor.rs). The full health-driven
    // restart loop attaches in Gate 5 integration.
    let mut sup = supervisor::Supervisor::new(supervisor::RestartPolicy::default());
    for d in &devices {
        sup.register(d.device_id);
    }
    tracing::info!(watched = devices.len(), "supervisor initialized");

    // Gate 2.4: 最简 /health (std TcpListener, 无第三方依赖; 后续可换 axum).
    // Gate 2.6 (P1②): 返回真实运行时状态, 与 Supervisor 状态机对齐 (不再固定 ready).
    let device_count = devices.len();
    let agent_state = std::sync::Arc::new(std::sync::Mutex::new(health::AgentState::Ready));

    // Gate 2.6 (CAP-01): bmd feature 下开启视频采集, 验证 MEDIA-RT-01 (首帧到达 + PTS 单调)
    #[cfg(feature = "bmd")]
    let capture_stats = match decklink::start_capture(0) {
        Ok(stats) => {
            *agent_state.lock().unwrap() = health::AgentState::Capturing;
            tracing::info!("CAP-01 采集已启动 (device 0)");
            Some(stats)
        }
        Err(e) => {
            tracing::error!(error = %e, "CAP-01 start_capture(0) 失败");
            None
        }
    };

    #[cfg(feature = "bmd")]
    if let Some(stats) = capture_stats.clone() {
        std::thread::spawn(move || loop {
            let n = stats.frame_count.load(std::sync::atomic::Ordering::SeqCst);
            let ff = stats.first_frame_at.lock().unwrap().is_some();
            let mono = stats.monotonic.load(std::sync::atomic::Ordering::SeqCst);
            tracing::info!(frame_count = n, first_frame = ff, pts_monotonic = mono, "CAP-01 capture live");
            std::thread::sleep(std::time::Duration::from_secs(1));
        });
    }

    std::thread::spawn({
        let agent_state = agent_state.clone();
        move || {
            match std::net::TcpListener::bind("0.0.0.0:8080") {
                Ok(listener) => {
                    tracing::info!("health endpoint listening on :8080");
                    for stream in listener.incoming() {
                        if let Ok(mut s) = stream {
                            let st = *agent_state.lock().unwrap();
                            let body = serde_json::json!({
                                "state": st,
                                "devices": device_count,
                                "active_pipelines": 0
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
                }
                Err(e) => tracing::error!(error = %e, "health bind failed"),
            }
        }
    });

    tracing::info!("media-agent skeleton loaded; interfaces frozen, logic pending");
    // 常驻以便 health 探测 (Gate 2.4 演示); 生产由 supervisor 管理生命周期.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
