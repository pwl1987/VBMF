//! Health — agent runtime state machine (for /health endpoint & ops UI).
//! Frozen interface per SoT §15.2. State enum only.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentState {
    Starting,
    Ready,
    Capturing,
    Error,
}

/// Liveness/readiness payload (shape TBD with Fastify control plane, §14).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub state: AgentState,
    pub devices: usize,
    pub active_pipelines: usize,
}
