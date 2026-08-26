//! RPC — transport boundary to Node/Fastify Control Plane.
//! Frozen interface per SoT §14 (Node=Control, Rust=Hardware). No transport yet.
//!
//! Rust MUST NOT implement: API gateway, auth, RBAC, config UI, WebSocket aggregation.
//! Those are Fastify's. Rust exposes only Hardware Plane operations below.
#![allow(dead_code)] // Gate 2.1 skeleton: interfaces frozen, not yet invoked.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request from Fastify control plane → Rust agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum AgentRequest {
    DiscoverDevices,
    AcquireLease { device_id: Uuid, owner: String, ttl_secs: u64 },
    ReleaseLease { device_id: Uuid },
    StartPipeline { intent: crate::graph_intent::GraphRuntimeIntent },
    StopPipeline { handle: Uuid },
    Health,
}

/// Response Rust agent → Fastify.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum AgentResponse {
    Devices(Vec<crate::device::DeviceInfo>),
    Lease(crate::lease::DeviceLease),
    Released,
    PipelineStarted(crate::pipeline::PipelineHandle),
    Stopped,
    Health(crate::health::HealthReport),
    Error(String),
}
