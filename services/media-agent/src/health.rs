//! Health — agent runtime state machine (for /health endpoint & ops UI).
//! Frozen interface per SoT §15.2. State enum only.
#![allow(dead_code)] // Gate 2.1 skeleton: interfaces frozen, not yet invoked.

use serde::{Deserialize, Serialize};

/// Agent 运行时状态机 (Gate 2.6, P1②)。
///
/// 词汇与 `supervisor::ProcessState` / `MEDIA_AGENT_STATE_MACHINE.md` 对齐
/// (Starting / Ready / Capturing / Degraded / Restarting / Backoff / Escalated /
/// ManualRequired), 避免 /health 返回固定 `ready` 而 Supervisor 内部已是 8 态的
/// 结构性不一致。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentState {
    Starting,
    Ready,
    Capturing,
    Degraded,
    Restarting,
    Backoff,
    Escalated,
    ManualRequired,
}

impl AgentState {
    /// 从 Supervisor 的进程态映射 (Gate 5 wiring 用)。
    pub fn from_process_state(s: crate::supervisor::ProcessState) -> Self {
        use crate::supervisor::ProcessState::*;
        match s {
            Running => AgentState::Capturing,
            Unhealthy => AgentState::Degraded,
            Restarting => AgentState::Restarting,
            Recovered => AgentState::Capturing,
            Backoff => AgentState::Backoff,
            Escalated | ManualRequired => AgentState::ManualRequired,
        }
    }
}

/// Liveness/readiness payload (shape TBD with Fastify control plane, §14).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub state: AgentState,
    pub devices: usize,
    pub active_pipelines: usize,
}
