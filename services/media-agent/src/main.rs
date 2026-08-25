//! VBMF Rust Media Agent — Gate 2 skeleton.
//! Interface shapes frozen (SoT §15.2 / §14). No business logic yet.
//!
//! Boundary (SoT §14): this binary owns the Hardware Plane only.
//! Control Plane (API/auth/RBAC/config/UI) stays in Node/Fastify.

mod device;
mod health;
mod lease;
mod pipeline;
mod rpc;
mod supervisor;

fn main() {
    // TODO(Gate 2.1): bootstrap supervisor + device manager + RPC server.
    // No logic in skeleton commit.
    tracing_subscriber::fmt::init();
    tracing::info!("media-agent skeleton loaded; interfaces frozen, logic pending");
}
