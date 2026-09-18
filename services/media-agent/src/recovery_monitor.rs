//! Backend-neutral single-input failure/recovery monitor.
//!
//! This module is deliberately independent of concrete media adapters. It consumes
//! canonical backend observations, lets Supervisor decide, re-validates the lease,
//! and invokes recovery on the same PipelineHandle. SessionStopHook makes teardown
//! the owner of monitor cancellation: stop signal -> monitor join -> backend stop.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use uuid::Uuid;

use crate::contracts::backend::MediaBackend;
use crate::events::EventSource;
use crate::lease::InMemoryLeaseManager;
use crate::pipeline::PipelineHandle;
use crate::pipeline_events::{PipelineBusEvent, PipelineBusEventKind};
use crate::session::SessionStopHook;
use crate::source::{LeaseKey, NetworkSourceId};
use crate::supervisor::{fault_trigger_from_events, Supervisor, SupervisorAction};

struct StopSignal {
    stopped: Mutex<bool>,
    wake: Condvar,
}

impl StopSignal {
    fn new() -> Self {
        Self {
            stopped: Mutex::new(false),
            wake: Condvar::new(),
        }
    }

    fn request(&self) {
        *self.stopped.lock().unwrap() = true;
        self.wake.notify_all();
    }

    fn wait_or_stopped(&self, duration: Duration) -> bool {
        let guard = self.stopped.lock().unwrap();
        if *guard {
            return true;
        }
        let (guard, _) = self
            .wake
            .wait_timeout_while(guard, duration, |stopped| !*stopped)
            .unwrap();
        *guard
    }
}

/// A cancellable, joinable monitor owned by one running Session.
pub struct RecoveryMonitorHandle {
    stop: Arc<StopSignal>,
    join: Mutex<Option<JoinHandle<()>>>,
    recoveries: Arc<AtomicU64>,
    exited: Arc<AtomicBool>,
}

impl RecoveryMonitorHandle {
    /// Request cancellation and wait until the monitor has exited.
    pub fn stop_and_join(&self) {
        self.stop.request();
        let join = self.join.lock().unwrap().take();
        if let Some(join) = join {
            if join.thread().id() != thread::current().id() {
                let _ = join.join();
            }
        }
    }

    pub fn is_exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }

    pub fn recovery_count(&self) -> u64 {
        self.recoveries.load(Ordering::SeqCst)
    }
}

impl Drop for RecoveryMonitorHandle {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

impl SessionStopHook for RecoveryMonitorHandle {
    fn on_session_stopping(&self, _id: &crate::session::SessionId) -> Result<(), String> {
        self.stop_and_join();
        Ok(())
    }
}

/// Spawn the single-input monitor with the production polling interval.
pub fn spawn(
    backend: Arc<dyn MediaBackend>,
    handle: PipelineHandle,
    device_id: Uuid,
    supervisor: Arc<Mutex<Supervisor>>,
    leases: Arc<InMemoryLeaseManager>,
) -> Arc<RecoveryMonitorHandle> {
    spawn_with_key(
        backend,
        handle,
        LeaseKey::Device(device_id),
        supervisor,
        leases,
        Duration::from_millis(50),
    )
}

/// Spawn the same recovery monitor for a typed NetworkSource lease.
pub fn spawn_network(
    backend: Arc<dyn MediaBackend>,
    handle: PipelineHandle,
    source_id: NetworkSourceId,
    supervisor: Arc<Mutex<Supervisor>>,
    leases: Arc<InMemoryLeaseManager>,
) -> Arc<RecoveryMonitorHandle> {
    spawn_with_key(
        backend,
        handle,
        LeaseKey::Network(source_id),
        supervisor,
        leases,
        Duration::from_millis(50),
    )
}

#[cfg(test)]
fn spawn_with_interval(
    backend: Arc<dyn MediaBackend>,
    handle: PipelineHandle,
    device_id: Uuid,
    supervisor: Arc<Mutex<Supervisor>>,
    leases: Arc<InMemoryLeaseManager>,
    interval: Duration,
) -> Arc<RecoveryMonitorHandle> {
    spawn_with_key(
        backend,
        handle,
        LeaseKey::Device(device_id),
        supervisor,
        leases,
        interval,
    )
}

fn spawn_with_key(
    backend: Arc<dyn MediaBackend>,
    handle: PipelineHandle,
    lease_key: LeaseKey,
    supervisor: Arc<Mutex<Supervisor>>,
    leases: Arc<InMemoryLeaseManager>,
    interval: Duration,
) -> Arc<RecoveryMonitorHandle> {
    let stop = Arc::new(StopSignal::new());
    let recoveries = Arc::new(AtomicU64::new(0));
    let exited = Arc::new(AtomicBool::new(false));
    let monitor = Arc::new(RecoveryMonitorHandle {
        stop: stop.clone(),
        join: Mutex::new(None),
        recoveries: recoveries.clone(),
        exited: exited.clone(),
    });
    let join = thread::Builder::new()
        .name("vbmf-recovery-monitor".into())
        .spawn(move || {
            run(
                backend, handle, lease_key, supervisor, leases, stop, recoveries, exited, interval,
            );
        })
        .expect("recovery monitor thread must start");
    *monitor.join.lock().unwrap() = Some(join);
    monitor
}
#[allow(clippy::too_many_arguments)]
fn run(
    backend: Arc<dyn MediaBackend>,
    handle: PipelineHandle,
    lease_key: LeaseKey,
    supervisor: Arc<Mutex<Supervisor>>,
    leases: Arc<InMemoryLeaseManager>,
    stop: Arc<StopSignal>,
    recoveries: Arc<AtomicU64>,
    exited: Arc<AtomicBool>,
    interval: Duration,
) {
    let device_id = match lease_key {
        LeaseKey::Device(device_id) => device_id,
        LeaseKey::Network(source_id) => source_id.0,
    };
    while !stop.wait_or_stopped(interval) {
        let events = backend.observe(&handle);
        let Some(event) = events.into_iter().find(|event| {
            event.handle == handle
                && matches!(
                    event.kind,
                    PipelineBusEventKind::Error | PipelineBusEventKind::Eos
                )
        }) else {
            continue;
        };

        let Some(observation) = canonical_failure_observation(&event) else {
            continue;
        };
        let canonical =
            supervisor
                .lock()
                .unwrap()
                .ingest(EventSource::Upstream, device_id, &observation);
        let Some(canonical) = canonical else {
            continue;
        };
        if !fault_trigger_from_events(std::slice::from_ref(&canonical), device_id) {
            continue;
        }

        let action = supervisor
            .lock()
            .unwrap()
            .report_failure(&device_id, None, None);
        match action {
            Ok(SupervisorAction::Restart) => {
                if !leases.is_key_valid(&lease_key) {
                    tracing::error!(
                        device = %device_id,
                        "recovery monitor stopped: lease invalid; manual intervention required"
                    );
                    let _ = supervisor.lock().unwrap().escalate(&device_id);
                    break;
                }
                let backoff = supervisor.lock().unwrap().backoff(&device_id);
                if supervisor
                    .lock()
                    .unwrap()
                    .begin_restart(&device_id)
                    .is_err()
                {
                    break;
                }
                if stop.wait_or_stopped(backoff) {
                    break;
                }
                match backend.recover(&handle) {
                    Ok(()) => {
                        recoveries.fetch_add(1, Ordering::SeqCst);
                        let _ = supervisor.lock().unwrap().report_recovered(&device_id);
                    }
                    Err(error) => {
                        tracing::error!(
                            device = %device_id,
                            handle = %handle.0,
                            %error,
                            "recovery monitor recover failed; escalating"
                        );
                        let _ = supervisor.lock().unwrap().escalate(&device_id);
                        break;
                    }
                }
            }
            Ok(SupervisorAction::Escalate) | Err(_) => break,
        }
    }
    exited.store(true, Ordering::SeqCst);
}

fn canonical_failure_observation(event: &PipelineBusEvent) -> Option<String> {
    match event.kind {
        PipelineBusEventKind::Error => Some(format!("pipeline error: {}", event.detail)),
        PipelineBusEventKind::Eos => {
            Some(format!("pipeline error: unexpected eos: {}", event.detail))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{RuntimeEvent, RuntimeEventLog, RuntimeEventSink};
    use crate::lease::LeaseManager;
    use crate::pipeline_events::BusSeverity;
    use crate::supervisor::{ProcessState, RestartPolicy};
    use std::sync::atomic::AtomicUsize;

    struct TestBackend {
        events: Mutex<Vec<PipelineBusEvent>>,
        recoveries: AtomicUsize,
    }

    impl TestBackend {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
                recoveries: AtomicUsize::new(0),
            }
        }

        fn fail(&self, handle: PipelineHandle) {
            self.events.lock().unwrap().push(PipelineBusEvent {
                handle,
                kind: PipelineBusEventKind::Error,
                source: "test".into(),
                timestamp: 0,
                detail: "child exited".into(),
                severity: BusSeverity::Error,
            });
        }
    }

    impl MediaBackend for TestBackend {
        fn instantiate(
            &self,
            _plan: &crate::pipeline::PipelinePlan,
        ) -> Result<PipelineHandle, crate::pipeline::PipelineError> {
            Ok(PipelineHandle(1))
        }
        fn start(&self, _handle: &PipelineHandle) -> Result<(), crate::pipeline::PipelineError> {
            Ok(())
        }
        fn stop(&self, _handle: &PipelineHandle) -> Result<(), crate::pipeline::PipelineError> {
            Ok(())
        }
        fn recover(&self, _handle: &PipelineHandle) -> Result<(), crate::pipeline::PipelineError> {
            self.recoveries.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn observe(&self, _handle: &PipelineHandle) -> Vec<PipelineBusEvent> {
            std::mem::take(&mut *self.events.lock().unwrap())
        }
    }

    fn supervisor(device: Uuid) -> (Arc<Mutex<Supervisor>>, Arc<RuntimeEventLog>) {
        let log = Arc::new(RuntimeEventLog::new());
        let policy = RestartPolicy {
            base_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(10),
            ..RestartPolicy::default()
        };
        let mut value = Supervisor::new(policy, log.clone() as Arc<dyn RuntimeEventSink>);
        value.register(device);
        (Arc::new(Mutex::new(value)), log)
    }

    fn wait_until(mut predicate: impl FnMut() -> bool) {
        for _ in 0..100 {
            if predicate() {
                return;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn error_enters_canonical_path_and_recovers_same_handle() {
        let device = Uuid::new_v4();
        let backend = Arc::new(TestBackend::new());
        let leases = Arc::new(InMemoryLeaseManager::new());
        leases
            .acquire(&device, "session", Duration::from_secs(30))
            .unwrap();
        let (supervisor, log) = supervisor(device);
        let handle = PipelineHandle(7);
        let monitor = spawn_with_interval(
            backend.clone(),
            handle,
            device,
            supervisor.clone(),
            leases,
            Duration::from_millis(2),
        );
        backend.fail(handle);
        wait_until(|| monitor.recovery_count() == 1);
        assert_eq!(monitor.recovery_count(), 1);
        assert_eq!(
            supervisor.lock().unwrap().status(&device),
            Some(ProcessState::Recovered)
        );
        assert!(log.drain().iter().any(|event| matches!(event, RuntimeEvent::PipelineFault { pipeline, .. } if *pipeline == device)));
        monitor.stop_and_join();
        assert!(monitor.is_exited());
        assert_eq!(backend.recoveries.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn invalid_lease_escalates_without_recover() {
        let device = Uuid::new_v4();
        let backend = Arc::new(TestBackend::new());
        let leases = Arc::new(InMemoryLeaseManager::new());
        let (supervisor, _log) = supervisor(device);
        let monitor = spawn_with_interval(
            backend.clone(),
            PipelineHandle(8),
            device,
            supervisor.clone(),
            leases,
            Duration::from_millis(2),
        );
        backend.fail(PipelineHandle(8));
        wait_until(|| monitor.is_exited());
        assert_eq!(
            supervisor.lock().unwrap().status(&device),
            Some(ProcessState::ManualRequired)
        );
        assert_eq!(backend.recoveries.load(Ordering::SeqCst), 0);
        monitor.stop_and_join();
    }

    #[test]
    fn stop_signal_joins_without_observing_after_stop() {
        let device = Uuid::new_v4();
        let backend = Arc::new(TestBackend::new());
        let leases = Arc::new(InMemoryLeaseManager::new());
        let (supervisor, _log) = supervisor(device);
        let monitor = spawn_with_interval(
            backend,
            PipelineHandle(9),
            device,
            supervisor,
            leases,
            Duration::from_secs(60),
        );
        monitor.stop_and_join();
        assert!(monitor.is_exited());
    }
}
