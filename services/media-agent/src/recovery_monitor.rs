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
#[cfg(test)]
use crate::pipeline_events::NETWORK_EXIT_DETAIL_PREFIX;
use crate::pipeline_events::{
    network_exit_class_from_detail, NetworkExitClass, PipelineBusEvent, PipelineBusEventKind,
};
use crate::resource::SharedResourceRegistry;
use crate::session::SessionStopHook;
use crate::source::{LeaseKey, NetworkSourceId, ResourceOwner};
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
    /// TG-4 (plan D8): "stop and close clear the budget" — the supervisor
    /// entry resets when the owning Session stops/closes this monitor. A
    /// ManualRequired reached mid-run therefore persists exactly until the
    /// operator stop, never before.
    supervisor: Arc<Mutex<Supervisor>>,
    watched: Uuid,
    /// TG-4 (INV-1): signal history input — set when a completed publisher
    /// connection reached SignalVerified; consumed by the exit attribution.
    signal_verified: Arc<AtomicBool>,
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
        // D8: the Session stop/close that tears this monitor down clears the
        // budget so an explicit operator stop + new Session may try again.
        self.supervisor.lock().unwrap().register(self.watched);
        self.signal_verified.store(false, Ordering::SeqCst);
    }

    pub fn is_exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }

    pub fn recovery_count(&self) -> u64 {
        self.recoveries.load(Ordering::SeqCst)
    }

    /// TG-4 (plan D7/D8 + INV-1): report that a completed publisher
    /// connection produced the required A/V streams and reached
    /// SignalVerified. This is the ONLY budget reset during a Session's
    /// life — listener (re)creation alone never resets it.
    pub fn report_signal_verified(&self) {
        self.signal_verified.store(true, Ordering::SeqCst);
        let mut supervisor = self.supervisor.lock().unwrap();
        if supervisor.attempts(&self.watched).is_some_and(|a| a > 0) {
            let _ = supervisor.report_recovered(&self.watched);
        }
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
        None,
    )
}

/// Spawn the same recovery monitor for a typed NetworkSource lease with the
/// RF-SRC-RTMP-02 TG-4 D7/D8 decision routing: Waiting is not a fault,
/// only INV-1-attributed `PublisherDisconnected` restarts (bounded by D8,
/// after lease AND registry-claim revalidation), `BindFailure`/`SpawnFailure`/
/// `UnknownExit` go ManualRequired immediately without consuming budget.
#[allow(clippy::too_many_arguments)]
pub fn spawn_network(
    backend: Arc<dyn MediaBackend>,
    handle: PipelineHandle,
    source_id: NetworkSourceId,
    supervisor: Arc<Mutex<Supervisor>>,
    leases: Arc<InMemoryLeaseManager>,
    resources: Arc<SharedResourceRegistry>,
) -> Arc<RecoveryMonitorHandle> {
    spawn_with_key(
        backend,
        handle,
        LeaseKey::Network(source_id),
        supervisor,
        leases,
        Duration::from_millis(50),
        Some(resources),
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
        None,
    )
}

#[cfg(test)]
fn spawn_network_with_interval(
    backend: Arc<dyn MediaBackend>,
    handle: PipelineHandle,
    source_id: NetworkSourceId,
    supervisor: Arc<Mutex<Supervisor>>,
    leases: Arc<InMemoryLeaseManager>,
    resources: Arc<SharedResourceRegistry>,
    interval: Duration,
) -> Arc<RecoveryMonitorHandle> {
    spawn_with_key(
        backend,
        handle,
        LeaseKey::Network(source_id),
        supervisor,
        leases,
        interval,
        Some(resources),
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_with_key(
    backend: Arc<dyn MediaBackend>,
    handle: PipelineHandle,
    lease_key: LeaseKey,
    supervisor: Arc<Mutex<Supervisor>>,
    leases: Arc<InMemoryLeaseManager>,
    interval: Duration,
    resources: Option<Arc<SharedResourceRegistry>>,
) -> Arc<RecoveryMonitorHandle> {
    let stop = Arc::new(StopSignal::new());
    let recoveries = Arc::new(AtomicU64::new(0));
    let exited = Arc::new(AtomicBool::new(false));
    let signal_verified = Arc::new(AtomicBool::new(false));
    let watched = match lease_key {
        LeaseKey::Device(device_id) => device_id,
        LeaseKey::Network(source_id) => source_id.0,
    };
    let monitor = Arc::new(RecoveryMonitorHandle {
        stop: stop.clone(),
        join: Mutex::new(None),
        recoveries: recoveries.clone(),
        exited: exited.clone(),
        supervisor: supervisor.clone(),
        watched,
        signal_verified: signal_verified.clone(),
    });
    let join = thread::Builder::new()
        .name("vbmf-recovery-monitor".into())
        .spawn(move || match resources {
            Some(resources) => run_network(
                backend,
                handle,
                lease_key,
                supervisor,
                leases,
                resources,
                stop,
                recoveries,
                exited,
                signal_verified,
                interval,
            ),
            None => run(
                backend, handle, lease_key, supervisor, leases, stop, recoveries, exited, interval,
            ),
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

/// D8 claim revalidation: the exact `(protocol, canonical_ip, port)` registry
/// claim (the Network source's `rtmp-input` Resource) must still be held
/// (Reserved/Allocated by the owning Session) before any automatic restart.
fn network_claim_held(resources: &SharedResourceRegistry, source_id: NetworkSourceId) -> bool {
    resources.with_inner(|registry| {
        registry.resources.iter().any(|resource| {
            resource.owner == ResourceOwner::Network(source_id)
                && matches!(
                    resource.state,
                    crate::resource::ResourceState::Reserved
                        | crate::resource::ResourceState::Allocated
                )
        })
    })
}

/// RF-SRC-RTMP-02 TG-4: the D7/D8 network decision loop.
///
/// * D7 row 1: a live listening child emits NO events — the loop just idles
///   (`Running / Waiting` is not a fault, no restart, no recovery event).
/// * INV-1 final attribution: exit classification + signal history. Only
///   `PublisherDisconnected` (positively attributed: child exited, a
///   publisher had reached SignalVerified on this listener, and no bind/spawn
///   evidence exists) enters the D8 budgeted recovery cycle.
/// * `BindFailure` / `SpawnFailure` / `UnknownExit` / unparseable detail →
///   ManualRequired immediately (no budget consumption, no exploratory
///   restart).
/// * Before every automatic restart: exact lease AND exact registry claim
///   revalidation (D8); either gone → manual.
/// * INV-2: a stop during the backoff window discards the restart (the
///   generation ends with the Session stop that owns this monitor).
#[allow(clippy::too_many_arguments)]
fn run_network(
    backend: Arc<dyn MediaBackend>,
    handle: PipelineHandle,
    lease_key: LeaseKey,
    supervisor: Arc<Mutex<Supervisor>>,
    leases: Arc<InMemoryLeaseManager>,
    resources: Arc<SharedResourceRegistry>,
    stop: Arc<StopSignal>,
    recoveries: Arc<AtomicU64>,
    exited: Arc<AtomicBool>,
    signal_verified: Arc<AtomicBool>,
    interval: Duration,
) {
    let LeaseKey::Network(source_id) = lease_key else {
        return;
    };
    let device = source_id.0;
    while !stop.wait_or_stopped(interval) {
        let events = backend.observe(&handle);
        let Some(event) = events.into_iter().find(|event| {
            event.handle == handle
                && matches!(
                    event.kind,
                    PipelineBusEventKind::Error | PipelineBusEventKind::Eos
                )
        }) else {
            // No exit event: the child is alive and listening (D7 Waiting).
            continue;
        };

        // Fail-closed parse: anything not positively the frozen vocabulary
        // (including a reader failure) is UnknownExit → manual.
        let observed =
            network_exit_class_from_detail(&event.detail).unwrap_or(NetworkExitClass::UnknownExit);
        let final_class = match observed {
            NetworkExitClass::UnknownExit if signal_verified.load(Ordering::SeqCst) => {
                // INV-1 positive attribution: exited child + a publisher had
                // reached SignalVerified + no bind evidence ⇒ the publisher
                // went away.
                NetworkExitClass::PublisherDisconnected
            }
            other => other,
        };

        let action = supervisor
            .lock()
            .unwrap()
            .report_network_exit(&device, final_class);
        match action {
            Ok(SupervisorAction::Restart) => {
                if !leases.is_key_valid(&lease_key) {
                    tracing::error!(
                        device = %device,
                        "network recovery stopped: lease invalid; manual intervention required"
                    );
                    let _ = supervisor.lock().unwrap().escalate(&device);
                    break;
                }
                if !network_claim_held(&resources, source_id) {
                    tracing::error!(
                        device = %device,
                        "network recovery stopped: registry claim no longer held; manual intervention required"
                    );
                    let _ = supervisor.lock().unwrap().escalate(&device);
                    break;
                }
                let backoff = supervisor.lock().unwrap().backoff(&device);
                if supervisor.lock().unwrap().begin_restart(&device).is_err() {
                    break;
                }
                if stop.wait_or_stopped(backoff) {
                    // INV-2: the owning Session stopped during backoff —
                    // discard this restart generation entirely.
                    break;
                }
                match backend.recover(&handle) {
                    Ok(()) => {
                        recoveries.fetch_add(1, Ordering::SeqCst);
                        // D8: a recreated listener does NOT reset the budget;
                        // the new listener must earn SignalVerified again.
                        signal_verified.store(false, Ordering::SeqCst);
                        let _ = supervisor.lock().unwrap().report_restart_completed(&device);
                    }
                    Err(error) => {
                        tracing::error!(
                            device = %device,
                            handle = handle.0,
                            %error,
                            "network recovery monitor recover failed; escalating"
                        );
                        let _ = supervisor.lock().unwrap().escalate(&device);
                        break;
                    }
                }
            }
            // ManualRequired (bind/spawn/unknown/budget) persists until the
            // operator stop — never an exploratory retry from here.
            Ok(SupervisorAction::Escalate) | Err(_) => break,
        }
    }
    exited.store(true, Ordering::SeqCst);
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

    // ── RF-SRC-RTMP-02 TG-4 (plan D7/D8 + INV-1/INV-2) ──────────────────────────

    impl TestBackend {
        fn fail_classified(&self, handle: PipelineHandle, class: NetworkExitClass) {
            self.events.lock().unwrap().push(PipelineBusEvent {
                handle,
                kind: PipelineBusEventKind::Error,
                source: "test".into(),
                timestamp: 0,
                detail: format!("{NETWORK_EXIT_DETAIL_PREFIX}{class}"),
                severity: BusSeverity::Error,
            });
        }
    }

    #[allow(clippy::type_complexity)]
    fn network_fixture() -> (
        Arc<TestBackend>,
        Arc<InMemoryLeaseManager>,
        Arc<Mutex<Supervisor>>,
        Arc<crate::resource::SharedResourceRegistry>,
        NetworkSourceId,
    ) {
        let source_id = NetworkSourceId(Uuid::new_v4());
        let backend = Arc::new(TestBackend::new());
        let leases = Arc::new(InMemoryLeaseManager::new());
        leases
            .acquire_key(
                &LeaseKey::Network(source_id),
                "session",
                Duration::from_secs(30),
            )
            .unwrap();
        let (supervisor, _log) = supervisor(source_id.0);
        let mut registry = crate::resource::ResourceRegistry::new();
        registry.register_network_source(source_id);
        let resources = Arc::new(crate::resource::SharedResourceRegistry::new(registry));
        (backend, leases, supervisor, resources, source_id)
    }

    /// Mark the fixture's network claim as held by a live Session.
    fn hold_claim(
        resources: &Arc<crate::resource::SharedResourceRegistry>,
        source_id: NetworkSourceId,
    ) {
        resources.with_inner_mut(|registry| {
            for resource in registry.resources.iter_mut() {
                if resource.owner == ResourceOwner::Network(source_id) {
                    resource
                        .transition(crate::resource::ResourceState::Reserved)
                        .unwrap();
                }
            }
        });
    }

    #[test]
    fn rf_src_rtmp_02_tg4_waiting_listener_is_not_a_fault() {
        // D7 row 1: a live listening child emits no events — no restart, no
        // recovery event, supervisor stays Running.
        let (backend, _leases, supervisor, _resources, source_id) = network_fixture();
        let monitor = spawn_network_with_interval(
            backend.clone(),
            PipelineHandle(21),
            source_id,
            supervisor.clone(),
            Arc::new(InMemoryLeaseManager::new()),
            Arc::new(crate::resource::SharedResourceRegistry::new(
                crate::resource::ResourceRegistry::new(),
            )),
            Duration::from_millis(2),
        );
        thread::sleep(Duration::from_millis(50));
        assert!(!monitor.is_exited());
        assert_eq!(monitor.recovery_count(), 0);
        assert_eq!(
            supervisor.lock().unwrap().status(&source_id.0),
            Some(ProcessState::Running)
        );
        monitor.stop_and_join();
    }

    #[test]
    fn rf_src_rtmp_02_tg4_bind_failure_manual_without_budget_or_restart() {
        // D6/D8: BindFailure → ManualRequired immediately, no budget
        // consumption, no restart attempt.
        let (backend, leases, supervisor, resources, source_id) = network_fixture();
        hold_claim(&resources, source_id);
        let monitor = spawn_network_with_interval(
            backend.clone(),
            PipelineHandle(22),
            source_id,
            supervisor.clone(),
            leases.clone(),
            resources,
            Duration::from_millis(2),
        );
        backend.fail_classified(PipelineHandle(22), NetworkExitClass::BindFailure);
        wait_until(|| monitor.is_exited());
        assert_eq!(
            supervisor.lock().unwrap().status(&source_id.0),
            Some(ProcessState::ManualRequired)
        );
        assert_eq!(backend.recoveries.load(Ordering::SeqCst), 0);
        assert_eq!(monitor.recovery_count(), 0);
        assert_eq!(supervisor.lock().unwrap().attempts(&source_id.0), Some(0));
        monitor.stop_and_join();
    }

    #[test]
    fn rf_src_rtmp_02_tg4_unknown_exit_without_signal_is_manual_never_exploratory() {
        // INV-1: no positive attribution (no SignalVerified history) →
        // ManualRequired, never an exploratory restart.
        let (backend, leases, supervisor, resources, source_id) = network_fixture();
        hold_claim(&resources, source_id);
        let monitor = spawn_network_with_interval(
            backend.clone(),
            PipelineHandle(23),
            source_id,
            supervisor.clone(),
            leases.clone(),
            resources,
            Duration::from_millis(2),
        );
        backend.fail_classified(PipelineHandle(23), NetworkExitClass::UnknownExit);
        wait_until(|| monitor.is_exited());
        assert_eq!(
            supervisor.lock().unwrap().status(&source_id.0),
            Some(ProcessState::ManualRequired)
        );
        assert_eq!(backend.recoveries.load(Ordering::SeqCst), 0);
        monitor.stop_and_join();
    }

    #[test]
    fn rf_src_rtmp_02_tg4_attributed_disconnect_restarts_without_budget_reset() {
        // INV-1: exited child + SignalVerified history ⇒ PublisherDisconnected
        // → one budgeted restart (after lease+claim revalidation). The
        // recreated listener does NOT reset the budget (attempts stay ≥ 1)
        // and must earn SignalVerified again.
        let (backend, leases, supervisor, resources, source_id) = network_fixture();
        hold_claim(&resources, source_id);
        let monitor = spawn_network_with_interval(
            backend.clone(),
            PipelineHandle(24),
            source_id,
            supervisor.clone(),
            leases.clone(),
            resources,
            Duration::from_millis(2),
        );
        monitor.report_signal_verified();
        backend.fail_classified(PipelineHandle(24), NetworkExitClass::UnknownExit);
        wait_until(|| monitor.recovery_count() == 1);
        assert_eq!(backend.recoveries.load(Ordering::SeqCst), 1);
        assert_eq!(
            supervisor.lock().unwrap().status(&source_id.0),
            Some(ProcessState::Running)
        );
        assert_eq!(
            supervisor.lock().unwrap().attempts(&source_id.0),
            Some(1),
            "listener recreation must not reset the D8 budget"
        );
        // the new listener lost its SignalVerified status: a further exit
        // without a new publisher goes manual, not restart.
        backend.fail_classified(PipelineHandle(24), NetworkExitClass::UnknownExit);
        wait_until(|| monitor.is_exited());
        assert_eq!(backend.recoveries.load(Ordering::SeqCst), 1);
        assert_eq!(
            supervisor.lock().unwrap().status(&source_id.0),
            Some(ProcessState::ManualRequired)
        );
        monitor.stop_and_join();
    }

    #[test]
    fn rf_src_rtmp_02_tg4_signal_verified_is_the_only_in_session_budget_reset() {
        // D8: a completed publisher connection resets the budget.
        let (backend, leases, supervisor, resources, source_id) = network_fixture();
        hold_claim(&resources, source_id);
        let monitor = spawn_network_with_interval(
            backend,
            PipelineHandle(25),
            source_id,
            supervisor.clone(),
            leases,
            resources,
            Duration::from_millis(2),
        );
        monitor.report_signal_verified();
        monitor.report_signal_verified();
        assert_eq!(
            supervisor.lock().unwrap().attempts(&source_id.0),
            Some(0),
            "no failure yet: SignalVerified must not fabricate state"
        );
        assert_eq!(
            supervisor.lock().unwrap().status(&source_id.0),
            Some(ProcessState::Running)
        );
        monitor.stop_and_join();
    }

    #[test]
    fn rf_src_rtmp_02_tg4_lost_claim_stops_recovery_manual() {
        // D8: before every automatic restart the exact registry claim is
        // revalidated; Available (claim gone) → manual, no restart.
        let (backend, leases, supervisor, resources, source_id) = network_fixture();
        // claim intentionally left Available (no session holds it)
        let monitor = spawn_network_with_interval(
            backend.clone(),
            PipelineHandle(26),
            source_id,
            supervisor.clone(),
            leases,
            resources,
            Duration::from_millis(2),
        );
        monitor.report_signal_verified();
        backend.fail_classified(PipelineHandle(26), NetworkExitClass::UnknownExit);
        wait_until(|| monitor.is_exited());
        assert_eq!(backend.recoveries.load(Ordering::SeqCst), 0);
        assert_eq!(
            supervisor.lock().unwrap().status(&source_id.0),
            Some(ProcessState::ManualRequired)
        );
        monitor.stop_and_join();
    }

    #[test]
    fn rf_src_rtmp_02_tg4_stop_during_backoff_discards_restart_generation() {
        // INV-2: a stop during the backoff window discards the restart; the
        // session stop also clears the budget for the next Session.
        let source_id = NetworkSourceId(Uuid::new_v4());
        let backend = Arc::new(TestBackend::new());
        let leases = Arc::new(InMemoryLeaseManager::new());
        leases
            .acquire_key(
                &LeaseKey::Network(source_id),
                "session",
                Duration::from_secs(30),
            )
            .unwrap();
        let log = Arc::new(RuntimeEventLog::new());
        // long backoff so the stop lands inside the window
        let policy = RestartPolicy {
            base_backoff: Duration::from_secs(30),
            max_backoff: Duration::from_secs(30),
            ..RestartPolicy::default()
        };
        let mut value = Supervisor::new(policy, log.clone() as Arc<dyn RuntimeEventSink>);
        value.register(source_id.0);
        let supervisor = Arc::new(Mutex::new(value));
        let mut registry = crate::resource::ResourceRegistry::new();
        registry.register_network_source(source_id);
        let resources = Arc::new(crate::resource::SharedResourceRegistry::new(registry));
        hold_claim(&resources, source_id);

        let monitor = spawn_network_with_interval(
            backend.clone(),
            PipelineHandle(27),
            source_id,
            supervisor.clone(),
            leases,
            resources,
            Duration::from_millis(1),
        );
        monitor.report_signal_verified();
        backend.fail_classified(PipelineHandle(27), NetworkExitClass::UnknownExit);
        monitor.stop_and_join();
        assert_eq!(
            backend.recoveries.load(Ordering::SeqCst),
            0,
            "stopped generation must not spawn a restart"
        );
        assert_eq!(
            supervisor.lock().unwrap().attempts(&source_id.0),
            Some(0),
            "operator stop clears the D8 budget for the next Session"
        );
    }
}
