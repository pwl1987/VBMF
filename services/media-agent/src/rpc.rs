//! Node↔Rust control-plane contract skeleton (Gate 2.1, SoT §14: Node=Control, Rust=Hardware).
//!
//! NOTE (0.7C-8): the current HTTP transport boundary is [`transport`](crate::transport),
//! which serves the API Boundary Model (`api_boundary`) over the five REST endpoints. It does
//! NOT serialize this method-tagged AgentRequest/AgentResponse RPC. This file is retained as
//! the frozen SoT §14 contract record for the Node↔Rust boundary; it is not on the wire path.
//!
//! Rust MUST NOT implement: API gateway, auth, RBAC, config UI, WebSocket aggregation.
//! Those are Fastify's. Rust exposes only Hardware Plane operations below.
#![allow(dead_code)] // Gate 2.1 skeleton: frozen SoT §14 contract, not on the wire path (see transport.rs).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request from Fastify control plane → Rust agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum AgentRequest {
    DiscoverDevices,
    AcquireLease {
        device_id: Uuid,
        owner: String,
        ttl_secs: u64,
    },
    ReleaseLease {
        device_id: Uuid,
    },
    StartPipeline {
        intent: crate::graph_intent::GraphRuntimeIntent,
    },
    StopPipeline {
        handle: Uuid,
    },
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
