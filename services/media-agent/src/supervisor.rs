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
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::events::{EventSource, RuntimeEvent, RuntimeEventSink};

/// `report_failure` Restart 路径发射的 `PipelineFault` 摘要 (P0-7D)。
///
/// 双重身份: 既是决策事件的 canonical 摘要, 也是 watchdog 事件驱动输入的**自回声标记**——
/// Supervisor 自己发出的重启排程事件不得再次触发 `report_failure` (否则 attempts/backoff
/// 随 tick 自激翻倍)。watchdog 侧按此常量精确排除回声。
pub const RESTART_ECHO_SUMMARY: &str = "health probe failure; restart scheduled";

/// P0-7D-1.4: 事件驱动故障触发谓词 (纯函数; watchdog 每 tick drain internal 后调用)。
///
/// 归属: `PipelineFault.pipeline == device` (Supervisor 决策句柄 = 设备维度, 见
/// `register`/`report_failure` 均以 device_id 注册) 或 `Uuid::nil()` (mapper 产的上游
/// 故障未归属); `HardwareFault.device_id` 同理。
/// **自回声排除**: summary == `RESTART_ECHO_SUMMARY` 的 PipelineFault 是本 Supervisor
/// `report_failure` 决策的回声, 不得再次触发决策 (否则 attempts/backoff 随 tick 自激翻倍)。
/// 其余 kind 不在设备决策触发面: `HealthChanged` 是决策平面自身, `SessionFailed` 是
/// 会话平面故障 (经 reducer 派生观测态, 不进设备重启决策)。
pub fn fault_trigger_from_events(events: &[RuntimeEvent], device: Uuid) -> bool {
    events.iter().any(|ev| match ev {
        RuntimeEvent::PipelineFault {
            pipeline, summary, ..
        } => {
            (*pipeline == device || *pipeline == Uuid::nil())
                && summary.as_str() != RESTART_ECHO_SUMMARY
        }
        RuntimeEvent::HardwareFault { device_id, .. } => {
            *device_id == device || *device_id == Uuid::nil()
        }
        _ => false,
    })
}

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
        let exp = self
            .base_backoff
            .saturating_mul(2u32.saturating_pow(attempt));
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
///
/// 0.6D: 恢复决策 (report_failure/report_recovered/escalate) 与上游 (Provider/Backend)
/// 归一化事件 (`ingest`) 都收敛为 canonical `RuntimeEvent`。
///
/// **0.7C-6 D8 解耦**: Supervisor 回归**纯决策引擎** — 不再持有事件表
/// (原 `events: RuntimeEventLog` 字段与 `record`/`drain_events`/`pending_events` API
/// 已删, 生产代码零调用者); 决策产生的事件经组合根注入的 `RuntimeEventSink` 发射
/// (与 SessionManager 事件同表汇聚, 单表单锁全局 FIFO)。
pub struct Supervisor {
    policy: RestartPolicy,
    states: HashMap<Uuid, Status>,
    sink: Arc<dyn RuntimeEventSink>,
}

impl Supervisor {
    pub fn new(policy: RestartPolicy, sink: Arc<dyn RuntimeEventSink>) -> Self {
        Self {
            policy,
            states: HashMap::new(),
            sink,
        }
    }

    /// 消费上游 (Provider/Backend) 上抛的 vendor 观测, 归一化为 `RuntimeEvent` (经默认映射器)。
    ///
    /// **03-01-A（R44 §3/§9）**: `device` = 该观测的 canonical 设备身份——
    /// watch 点持 device_uuid, 生产路径身份不再在 mapper 边界丢失
    /// （`PipelineFault.pipeline` = 设备 canonical 身份, 与 `register`/
    /// `report_failure` 决策句柄同源; nil 仅在调用方确无设备上下文时出现
    /// = 未归属, custody 生产桥拒收——fail-closed 不放宽）。
    /// Adapter 可提供专属 `RuntimeEventMapper` 消化 vendor 细节; 此处兜底使用默认映射器。
    pub fn ingest(&self, source: EventSource, device: Uuid, observation: &str) {
        let mapper = crate::events::DefaultRuntimeEventMapper;
        if let Some(ev) = mapper.map_upstream_for_device(source, device, observation) {
            self.sink.emit(ev);
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
    ///
    /// 0.6D: 决策同时发射 canonical `RuntimeEvent` (Restart → `PipelineFault{retryable}`,
    /// Escalate → `HealthChanged{.. manual_required}`), 经唯一出口写入 `events`。
    pub fn report_failure(&mut self, handle: &Uuid) -> Result<SupervisorAction, SupervisorError> {
        let action = {
            let st = self
                .states
                .get_mut(handle)
                .ok_or(SupervisorError::UnknownHandle)?;
            st.attempts += 1;
            if st.attempts >= self.policy.circuit_threshold {
                st.circuit_open = true;
            }
            if !self.policy.should_retry(st.attempts) || st.circuit_open {
                st.state = ProcessState::ManualRequired;
                SupervisorAction::Escalate
            } else {
                st.state = ProcessState::Unhealthy;
                SupervisorAction::Restart
            }
        };
        match action {
            SupervisorAction::Restart => {
                self.sink.emit(RuntimeEvent::PipelineFault {
                    pipeline: *handle,
                    summary: RESTART_ECHO_SUMMARY.into(),
                    retryable: true,
                });
            }
            SupervisorAction::Escalate => {
                self.sink.emit(RuntimeEvent::HealthChanged {
                    from: "unhealthy".into(),
                    to: "manual_required".into(),
                });
            }
        }
        Ok(action)
    }

    /// Begin a restart; the caller performs the actual restart then calls
    /// `report_recovered` (success) or `report_failure` (failure) again.
    pub fn begin_restart(&mut self, handle: &Uuid) -> Result<(), SupervisorError> {
        let st = self
            .states
            .get_mut(handle)
            .ok_or(SupervisorError::UnknownHandle)?;
        st.state = ProcessState::Restarting;
        Ok(())
    }

    /// Restart succeeded; reset budget + circuit.
    ///
    /// 0.6D: 发射 `HealthChanged{.. recovered}` 事件。
    pub fn report_recovered(&mut self, handle: &Uuid) -> Result<(), SupervisorError> {
        {
            let st = self
                .states
                .get_mut(handle)
                .ok_or(SupervisorError::UnknownHandle)?;
            st.state = ProcessState::Recovered;
            st.attempts = 0;
            st.circuit_open = false;
        }
        self.sink.emit(RuntimeEvent::HealthChanged {
            from: "restarting".into(),
            to: "recovered".into(),
        });
        Ok(())
    }

    /// Backoff the caller should wait before the next restart attempt for `handle`.
    pub fn backoff(&self, handle: &Uuid) -> Duration {
        let attempts = self.states.get(handle).map(|s| s.attempts).unwrap_or(0);
        self.policy.backoff_for(attempts.saturating_sub(1))
    }

    /// Force escalation (FI-08/09 manual-intervention path).
    ///
    /// 0.6D: 发射 `HealthChanged{.. manual_required}` 事件。
    pub fn escalate(&mut self, handle: &Uuid) -> Result<(), SupervisorError> {
        {
            let st = self
                .states
                .get_mut(handle)
                .ok_or(SupervisorError::UnknownHandle)?;
            st.state = ProcessState::ManualRequired;
        }
        self.sink.emit(RuntimeEvent::HealthChanged {
            from: "running".into(),
            to: "manual_required".into(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::RuntimeEventLog;
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

    /// D8 解耦后测试夹具: Supervisor + 注入的组合根 log (drain 经 log 而非 supervisor)。
    fn sup_with_log(policy: RestartPolicy) -> (Supervisor, Arc<RuntimeEventLog>) {
        let log = Arc::new(RuntimeEventLog::new());
        (
            Supervisor::new(policy, log.clone() as Arc<dyn RuntimeEventSink>),
            log,
        )
    }

    #[test]
    fn register_then_running() {
        let (mut s, _log) = sup_with_log(RestartPolicy::default());
        let h = Uuid::new_v4();
        s.register(h);
        assert_eq!(s.status(&h), Some(ProcessState::Running));
    }

    #[test]
    fn restart_until_budget_exhausted_then_escalate() {
        let (mut s, _log) = sup_with_log(RestartPolicy::default()); // max_retries=5, circuit=5
        let h = Uuid::new_v4();
        s.register(h);
        for i in 0..4 {
            let action = s.report_failure(&h).unwrap();
            assert_eq!(
                action,
                SupervisorAction::Restart,
                "attempt {i} should restart"
            );
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
        let (mut s, _log) = sup_with_log(policy);
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
        let (mut s, _log) = sup_with_log(RestartPolicy::default());
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
        let (mut s, _log) = sup_with_log(RestartPolicy::default());
        let h = Uuid::new_v4();
        assert_eq!(s.report_failure(&h), Err(SupervisorError::UnknownHandle));
        assert_eq!(s.escalate(&h), Err(SupervisorError::UnknownHandle));
    }

    #[test]
    fn report_failure_emits_retryable_pipeline_fault() {
        let (mut s, log) = sup_with_log(RestartPolicy::default());
        let h = Uuid::new_v4();
        s.register(h);
        s.report_failure(&h).unwrap();
        let ev = log.drain();
        assert_eq!(ev.len(), 1);
        assert_eq!(
            ev[0],
            RuntimeEvent::PipelineFault {
                pipeline: h,
                summary: "health probe failure; restart scheduled".into(),
                retryable: true,
            }
        );
        assert!(ev[0].is_fault());
        // drain 后清空。
        assert!(log.drain().is_empty());
    }

    #[test]
    fn escalate_emits_health_changed_to_manual_required() {
        let (mut s, log) = sup_with_log(RestartPolicy::default());
        let h = Uuid::new_v4();
        s.register(h);
        s.escalate(&h).unwrap();
        let ev = log.drain();
        assert_eq!(
            ev,
            vec![RuntimeEvent::HealthChanged {
                from: "running".into(),
                to: "manual_required".into(),
            }]
        );
    }

    #[test]
    fn ingest_normalizes_upstream_observation_via_mapper() {
        let (s, log) = sup_with_log(RestartPolicy::default());
        let dev = Uuid::new_v4();
        s.ingest(
            EventSource::Upstream,
            dev,
            "hardware: device lost (no hotplug)",
        );
        let ev = log.drain();
        assert_eq!(ev.len(), 1);
        assert!(
            matches!(&ev[0], RuntimeEvent::HardwareFault { device_id, .. } if *device_id == dev),
            "03-01-A: HardwareFault 携带 canonical 设备身份"
        );
        // 03-01-A: 管线级故障携带设备身份（非 nil——custody 可归因）。
        s.ingest(EventSource::Upstream, dev, "pipeline error: gst bus");
        let ev2 = log.drain();
        assert!(matches!(&ev2[0], RuntimeEvent::PipelineFault { pipeline, .. } if *pipeline == dev));
        // 无故障语义的观测不产生事件 (不伪造)。
        s.ingest(EventSource::Upstream, dev, "all nominal");
        assert!(log.drain().is_empty());
    }

    /// 03-01-A: 身份化故障只触发归属设备的 fault_trigger——他设备零误触;
    /// nil（未归属）保守全匹配维持既有语义（identity-less 兜底路径）。
    #[test]
    fn evt_int_rt_02_identity_carried_fault_triggers_only_owning_device() {
        let dev = Uuid::new_v4();
        let other = Uuid::new_v4();
        let carried = [RuntimeEvent::PipelineFault {
            pipeline: dev,
            summary: "decode error".into(),
            retryable: true,
        }];
        assert!(fault_trigger_from_events(&carried, dev));
        assert!(
            !fault_trigger_from_events(&carried, other),
            "身份化故障不误触他设备"
        );
        let unattributed = [RuntimeEvent::PipelineFault {
            pipeline: Uuid::nil(),
            summary: "pipeline error: unattributed".into(),
            retryable: true,
        }];
        assert!(
            fault_trigger_from_events(&unattributed, other),
            "nil 未归属保守匹配维持（既有语义零变化）"
        );
    }

    /// P0-7D-4.2: 事件驱动故障触发谓词 — 错误路径全覆盖
    /// (自回声不自激 / 上游触发 / 归属判定 / 平面分离)。
    #[test]
    fn evt_int_rt_01_fault_trigger_echo_never_retriggers() {
        let dev = Uuid::new_v4();
        // (1) 自回声不触发 — 防 attempts/backoff 随 tick 自激翻倍 (核心错误路径)。
        let echo = [RuntimeEvent::PipelineFault {
            pipeline: dev,
            summary: RESTART_ECHO_SUMMARY.into(),
            retryable: true,
        }];
        assert!(!fault_trigger_from_events(&echo, dev));
        // (2) 上游 PipelineFault (同设备, 非 echo) 触发。
        let upstream = [RuntimeEvent::PipelineFault {
            pipeline: dev,
            summary: "decode error".into(),
            retryable: true,
        }];
        assert!(fault_trigger_from_events(&upstream, dev));
        // (3) nil 归属 (mapper 未归属) 触发; 异设备不触发。
        let nil_attr = [RuntimeEvent::PipelineFault {
            pipeline: Uuid::nil(),
            summary: "pipeline error: bus".into(),
            retryable: true,
        }];
        assert!(fault_trigger_from_events(&nil_attr, dev));
        assert!(!fault_trigger_from_events(
            &[RuntimeEvent::PipelineFault {
                pipeline: Uuid::new_v4(),
                summary: "x".into(),
                retryable: true,
            }],
            dev
        ));
        // (4) HardwareFault 同设备/nil 触发; 异设备不触发。
        assert!(fault_trigger_from_events(
            &[RuntimeEvent::HardwareFault {
                device_id: dev,
                summary: "lost".into(),
            }],
            dev
        ));
        assert!(fault_trigger_from_events(
            &[RuntimeEvent::HardwareFault {
                device_id: Uuid::nil(),
                summary: "lost".into(),
            }],
            dev
        ));
        assert!(!fault_trigger_from_events(
            &[RuntimeEvent::HardwareFault {
                device_id: Uuid::new_v4(),
                summary: "lost".into(),
            }],
            dev
        ));
        // (5) 平面分离: HealthChanged (决策平面) 与 SessionFailed (会话平面)
        //     不在设备决策触发面。
        assert!(!fault_trigger_from_events(
            &[RuntimeEvent::HealthChanged {
                from: "unhealthy".into(),
                to: "manual_required".into(),
            }],
            dev
        ));
        assert!(!fault_trigger_from_events(
            &[RuntimeEvent::SessionFailed {
                session_id: Uuid::new_v4(),
                reason: "r".into(),
            }],
            dev
        ));
    }
}
