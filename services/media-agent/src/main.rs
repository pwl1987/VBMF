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
mod supervisor;

// Trait must be in scope to call `discover()` (trait method, not inherent).
use device::DeviceManager;
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
    if let Some(first) = devices.first() {
        match lm.acquire(&first.device_id, "bootstrap", std::time::Duration::from_secs(60)) {
            Ok(l) => tracing::info!(device = %l.device_id, "lease acquired"),
            Err(e) => tracing::warn!(error = %e, "lease acquire failed"),
        }
    }

    tracing::info!("media-agent skeleton loaded; interfaces frozen, logic pending");
}
