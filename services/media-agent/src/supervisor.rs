//! Runtime Supervisor — crash/hang/lost-device detection & restart.
//! Frozen interface per SoT §15.2 (MEDIA-03). No logic yet.

use std::time::Duration;

/// Monitor process/pipeline health; restart on failure.
pub trait Supervisor {
    /// Watch a pipeline; returns when stable or gives up per RestartPolicy.
    fn monitor(&self, handle: &uuid::Uuid) -> Result<(), SupervisorError>;
    /// Force restart (used by MEDIA-03 recovery + FI-08/09).
    fn restart(&self, handle: &uuid::Uuid) -> Result<(), SupervisorError>;
}

/// Restart policy (bounded retries; avoids crash loops).
#[derive(Debug, Clone)]
pub struct RestartPolicy {
    pub max_retries: u32,
    pub backoff: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    #[error("restart budget exhausted")]
    BudgetExhausted,
    #[error("device lost, cannot restart without hotplug (MEDIA-04)")]
    DeviceLost,
}
