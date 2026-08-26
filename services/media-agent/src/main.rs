//! VBMF Rust Media Agent — Gate 2 skeleton.
//! Interface shapes frozen (SoT §15.2 / §14). No business logic yet.
//!
//! Boundary (SoT §14): this binary owns the Hardware Plane only.
//! Control Plane (API/auth/RBAC/config/UI) stays in Node/Fastify.

mod config;
mod device;
mod health;
mod lease;
mod pipeline;
mod rpc;
mod sdk;
mod supervisor;

// Trait must be in scope to call `discover()` (trait method, not inherent).
use device::DeviceManager;
use std::io::Write;
use lease::LeaseManager;

fn main() {
    // TODO(Gate 2.1): bootstrap supervisor + device manager + RPC server.
    // No logic in skeleton commit.
    tracing_subscriber::fmt::init();

    // Gate 2.1: load config shape from env (no behavior attached yet).
    let _cfg = config::Config::from_env();

    // Gate 2.2: device discovery (filesystem probe; safe on CI / non-BMD hosts).
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
    // 排他性不变量: 同一设备重复 acquire 必须被拒 (防 host ffmpeg / 双采)。
    if let Some(first) = devices.first() {
        match lm.acquire(&first.device_id, "second-owner", std::time::Duration::from_secs(60)) {
            Ok(_) => tracing::warn!("LEASE COLLISION — double-capture risk!"),
            Err(e) => tracing::info!(error = %e, "lease re-acquire correctly rejected"),
        }
    }

    // Gate 2.5 (A): DeckLink SDK FFI smoke — 验证 libDeckLinkAPI.so 在运行环境可达。
    // 宿主机(/usr/lib 默认路径)应成功; Option B 容器若不 bind-mount 库则 warn(预期)。
    match sdk::probe_sdk_version("libDeckLinkAPI.so") {
        Ok(v) => {
            let (maj, min) = sdk::decode_version(v);
            tracing::info!(encoded = v, major = maj, minor = min, "SDK libDeckLinkAPI.so loaded");
        }
        Err(e) => tracing::warn!(error = %e, "SDK probe failed (expected in container w/o bind-mount)"),
    }

    // Gate 2.4: 最简 /health (std TcpListener, 无第三方依赖; 后续可换 axum)。
    let device_count = devices.len();
    std::thread::spawn(move || {
        match std::net::TcpListener::bind("0.0.0.0:8080") {
            Ok(listener) => {
                tracing::info!("health endpoint listening on :8080");
                for stream in listener.incoming() {
                    if let Ok(mut s) = stream {
                        let body = format!(
                            "{{\"state\":\"ready\",\"devices\":{device_count},\"active_pipelines\":0}}"
                        );
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
    });

    tracing::info!("media-agent skeleton loaded; interfaces frozen, logic pending");
    // 常驻以便 health 探测 (Gate 2.4 演示); 生产由 supervisor 管理生命周期。
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
