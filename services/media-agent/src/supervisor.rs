//! Runtime Supervisor — recovery-policy DECISION ENGINE for Gate 5 (MEDIA-03).
//!
//! ## Boundary (HARD RULE)
//! The Supervisor ONLY *decides* a recovery strategy. It never touches GStreamer,
//! FFmpeg, or the DeckLink SDK directly, and it does NOT spawn `ffmpeg` /
//! `gst-launch` / device processes itself.
//!
//! ```text
//! Supervisor ──(SupervisorAction)──▶ Pipeline / Device / Process Controller
//! ```
//!
//! The Controller performs the actual restart / re-enumerate, then reports back via
//! `report_recovered` / `report_failure`. This keeps Gate 5 out of the Media
//! Runtime execution layer.
//!
//! Pure logic, hardware-independent: fully covered by unit tests (`cargo test`).
//! `#![allow(dead_code)]`: some transitions are only exercised by integration
//! callers (Gate 5 wiring) and by `#[cfg(test)]`; removed once the watchdog loop
//! is connected.
#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// Restart policy: bounded retries with exponential backoff (crash-loop guard).
#[derive(Debug, Clone)]
pub struct RestartPolicy {
    pub max_retries: u32,
    pub base_backoff: Duration,
    pub max_backoff: Duration,
    /// Consecutive failures that trip the circuit breaker -> escalate.
    pub circuit_threshold: u32,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
            circuit_threshold: 5,
        }
    }
}

impl RestartPolicy {
    /// Exponential backoff for the given attempt (0-based), capped at `max_backoff`.
    pub fn backoff_for(&self, attempt: u32) -> Duration {
        let exp = self.base_backoff.saturating_mul(2u32.saturating_pow(attempt));
        exp.min(self.max_backoff)
    }

    /// Whether another restart attempt is within budget.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

/// Supervisor state for a single watched process/pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Unhealthy,
    Restarting,
    Recovered,
    Backoff,
    Escalated,
    ManualRequired,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SupervisorError {
    #[error("restart budget exhausted")]
    BudgetExhausted,
    #[error("device lost, cannot restart without hotplug (MEDIA-04)")]
    DeviceLost,
    #[error("unknown handle")]
    UnknownHandle,
}

/// What the caller should do in response to a reported failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorAction {
    /// Restart the process (optionally after `backoff()`).
    Restart,
    /// Escalate: human / automation intervention required (MANUAL_REQUIRED).
    Escalate,
}

#[derive(Debug, Clone)]
struct Status {
    state: ProcessState,
    attempts: u32,
    circuit_open: bool,
}

/// Watchdog / restart supervisor. Hardware-independent; drive it from health probes.
pub struct Supervisor {
    policy: RestartPolicy,
    states: HashMap<Uuid, Status>,
}

impl Supervisor {
    pub fn new(policy: RestartPolicy) -> Self {
        Self {
            policy,
            states: HashMap::new(),
        }
    }

    /// Register a handle as Running.
    pub fn register(&mut self, handle: Uuid) {
        self.states.insert(
            handle,
            Status {
                state: ProcessState::Running,
                attempts: 0,
                circuit_open: false,
            },
        );
    }

    pub fn status(&self, handle: &Uuid) -> Option<ProcessState> {
        self.states.get(handle).map(|s| s.state)
    }

    /// Health probe reported failure for `handle`.
    /// Increments the attempt counter, trips the circuit breaker at `circuit_threshold`,
    /// and decides Restart (within budget) vs Escalate (budget exhausted / circuit open).
    pub fn report_failure(&mut self, handle: &Uuid) -> Result<SupervisorAction, SupervisorError> {
        let st = self.states.get_mut(handle).ok_or(SupervisorError::UnknownHandle)?;
        st.attempts += 1;
        if st.attempts >= self.policy.circuit_threshold {
            st.circuit_open = true;
        }
        if !self.policy.should_retry(st.attempts) || st.circuit_open {
            st.state = ProcessState::ManualRequired;
            return Ok(SupervisorAction::Escalate);
        }
        st.state = ProcessState::Unhealthy;
        Ok(SupervisorAction::Restart)
    }

    /// Begin a restart; the caller performs the actual restart then calls
    /// `report_recovered` (success) or `report_failure` (failure) again.
    pub fn begin_restart(&mut self, handle: &Uuid) -> Result<(), SupervisorError> {
        let st = self.states.get_mut(handle).ok_or(SupervisorError::UnknownHandle)?;
        st.state = ProcessState::Restarting;
        Ok(())
    }

    /// Restart succeeded; reset budget + circuit.
    pub fn report_recovered(&mut self, handle: &Uuid) -> Result<(), SupervisorError> {
        let st = self.states.get_mut(handle).ok_or(SupervisorError::UnknownHandle)?;
        st.state = ProcessState::Recovered;
        st.attempts = 0;
        st.circuit_open = false;
        Ok(())
    }

    /// Backoff the caller should wait before the next restart attempt for `handle`.
    pub fn backoff(&self, handle: &Uuid) -> Duration {
        let attempts = self.states.get(handle).map(|s| s.attempts).unwrap_or(0);
        self.policy.backoff_for(attempts.saturating_sub(1))
    }

    /// Force escalation (FI-08/09 manual-intervention path).
    pub fn escalate(&mut self, handle: &Uuid) -> Result<(), SupervisorError> {
        let st = self.states.get_mut(handle).ok_or(SupervisorError::UnknownHandle)?;
        st.state = ProcessState::ManualRequired;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn backoff_grows_exponentially_and_caps() {
        let p = RestartPolicy::default();
        assert_eq!(p.backoff_for(0), Duration::from_secs(1));
        assert_eq!(p.backoff_for(1), Duration::from_secs(2));
        assert_eq!(p.backoff_for(2), Duration::from_secs(4));
        assert_eq!(p.backoff_for(3), Duration::from_secs(8));
        // cap at max_backoff (60s): 2^6=64 -> capped to 60.
        assert_eq!(p.backoff_for(6), Duration::from_secs(60));
        assert_eq!(p.backoff_for(100), Duration::from_secs(60));
    }

    #[test]
    fn should_retry_respects_max() {
        let p = RestartPolicy::default(); // max_retries = 5
        assert!(p.should_retry(0));
        assert!(p.should_retry(4));
        assert!(!p.should_retry(5));
        assert!(!p.should_retry(6));
    }

    #[test]
    fn register_then_running() {
        let mut s = Supervisor::new(RestartPolicy::default());
        let h = Uuid::new_v4();
        s.register(h);
        assert_eq!(s.status(&h), Some(ProcessState::Running));
    }

    #[test]
    fn restart_until_budget_exhausted_then_escalate() {
        let mut s = Supervisor::new(RestartPolicy::default()); // max_retries=5, circuit=5
        let h = Uuid::new_v4();
        s.register(h);
        for i in 0..4 {
            let action = s.report_failure(&h).unwrap();
            assert_eq!(action, SupervisorAction::Restart, "attempt {i} should restart");
            assert_eq!(s.status(&h), Some(ProcessState::Unhealthy));
            s.begin_restart(&h).unwrap();
        }
        // 5th failure exhausts budget.
        let action = s.report_failure(&h).unwrap();
        assert_eq!(action, SupervisorAction::Escalate);
        assert_eq!(s.status(&h), Some(ProcessState::ManualRequired));
    }

    #[test]
    fn circuit_breaker_trips_before_budget() {
        let policy = RestartPolicy {
            max_retries: 10,
            base_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
            circuit_threshold: 3,
        };
        let mut s = Supervisor::new(policy);
        let h = Uuid::new_v4();
        s.register(h);
        assert_eq!(s.report_failure(&h).unwrap(), SupervisorAction::Restart);
        assert_eq!(s.report_failure(&h).unwrap(), SupervisorAction::Restart);
        // 3rd failure trips circuit -> escalate even though max_retries=10.
        assert_eq!(s.report_failure(&h).unwrap(), SupervisorAction::Escalate);
        assert_eq!(s.status(&h), Some(ProcessState::ManualRequired));
    }

    #[test]
    fn recovery_resets_budget_and_circuit() {
        let mut s = Supervisor::new(RestartPolicy::default());
        let h = Uuid::new_v4();
        s.register(h);
        let _ = s.report_failure(&h); // attempt 1
        let _ = s.report_failure(&h); // attempt 2 -> still restart
        assert_eq!(s.status(&h), Some(ProcessState::Unhealthy));
        s.begin_restart(&h).unwrap();
        s.report_recovered(&h).unwrap();
        assert_eq!(s.status(&h), Some(ProcessState::Recovered));
        // after recovery, failures count from zero again.
        let a = s.report_failure(&h).unwrap();
        assert_eq!(a, SupervisorAction::Restart);
        assert_eq!(s.status(&h), Some(ProcessState::Unhealthy));
    }

    #[test]
    fn unknown_handle_errors() {
        let mut s = Supervisor::new(RestartPolicy::default());
        let h = Uuid::new_v4();
        assert_eq!(s.report_failure(&h), Err(SupervisorError::UnknownHandle));
        assert_eq!(s.escalate(&h), Err(SupervisorError::UnknownHandle));
    }
}
