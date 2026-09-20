//! Node↔Rust control-plane contract skeleton (Gate 2.1, SoT §14: Node=Control, Rust=Hardware).
//!
//! NOTE (0.7C-8): the current HTTP transport boundary is [`transport`](crate::transport),
//! which serves the API Boundary Model (`api_boundary`) over the five REST endpoints. It does
//! NOT serialize this method-tagged AgentRequest/AgentResponse RPC. This file is retained as
//! the frozen SoT §14 contract record for the Node↔Rust boundary; it is not on the wire path.
//!
//! RECONCILED (RCE-01A, 2026-09-20): the internal Runtime Control surface on the wire is
//! [`internal_control`](crate::internal_control) — JSON-RPC 2.0 at `/internal/v1/agent`
//! over the CURRENT frozen session command plane (start/stop/release_session +
//! switch_program + command_id idempotency, `command.rs`/`idempotency.rs`). The verb set
//! below (lease/pipeline-handle model) predates the Phase 0.7C session command plane and is
//! retained as a HISTORICAL contract record only — superseded per the authority order
//! (Phase 0.6/0.7 contract family > V0.1 SoT §14 wording). See
//! `docs/superpowers/plans/2026-09-20-runtime-control-entry-01-planning.md` §2 R3.
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
