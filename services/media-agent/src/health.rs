//! Health — agent runtime state machine (for /health endpoint & ops UI).
//! Frozen interface per SoT §15.2. State enum only.
//!
//! P0-7D: Gate 2.1 skeleton → 完整实现 — `reduce` 纯函数把 `RuntimeEvent` 流折叠为
//! `AgentState` 派生 (事件内消费集成)。契约 (Design Doc §3):
//! - 纯函数: 同输入同输出, 消费侧只读派生, **零写回** (Observation≠Configuration 红线);
//! - `HealthFold` 是跨 tick 持久的最小折叠上下文 (禁万能 struct), `active_sessions`
//!   仅服务 Ready/Capturing 派生, 不追踪微相位;
//! - 决策权威: `HealthChanged` (Supervisor 决策事件) 直接迁移; 观测事件按优先级格
//!   `ManualRequired > Degraded > Capturing > Ready` 派生候选;
//! - 诚实登记: `Restarting`/`Backoff` 词表在册但无事件生产者 (`begin_restart`/`backoff`
//!   不发事件), 且现行散写也从不写这两态 — 本期不派生 (登记为后续 watchdog 演进项);
//!   `Starting` 为构造 bootstrap 态, 非事件派生。
#![allow(dead_code)] // 0.7D 起 reducer/AgentState 已接线 (非死骨架); 本 allow 仅覆盖冻结
                     // 8 态词表中无生产构造点的词汇完整性项 (Starting/Restarting/Backoff/
                     // Escalated — SoT §15.2 词表冻结, 不为词表完整性伪造构造点)。

use crate::events::RuntimeEvent;
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
    /// 状态优先级 (P0-7D reducer 折叠格): 高优先压低优先; 重置边 (recovered/会话归零)
    /// 例外。`Starting` 不参与格 (仅构造 bootstrap)。
    fn precedence(self) -> u8 {
        match self {
            AgentState::ManualRequired | AgentState::Escalated => 4,
            AgentState::Degraded => 3,
            AgentState::Restarting | AgentState::Backoff | AgentState::Capturing => 2,
            AgentState::Ready => 1,
            AgentState::Starting => 0,
        }
    }

    /// 从 Supervisor 的进程态映射 (Gate 5 wiring 用)。
    #[allow(dead_code)] // 冻结契约映射 (SoT §15.2); 现行接线经事件流派生, 保留供控制面/未来 watchdog 直连
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

/// Liveness/readiness payload (shape TBD with Fastify control plane, §14)。
#[allow(dead_code)] // 冻结契约类型; 消费方为 rpc.rs 冻结 SoT §14 skeleton (不在 wire 路径)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub state: AgentState,
    pub devices: usize,
    pub active_pipelines: usize,
}

/// reducer 跨 tick 持久折叠上下文 (P0-7D; 最小派生面)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthFold {
    /// 当前派生的 agent 观测态 (watchdog tick 写回 `Arc<Mutex<AgentState>>`)。
    pub agent: AgentState,
    /// 活跃会话计数 (SessionCreated +1 / Released -1); 归零且无故障 pending → Ready。
    pub active_sessions: usize,
}

impl HealthFold {
    /// 以当前实际 agent 态为初值构造 (bootstrap 语义: 构造期/乐观写入是 reducer 的
    /// 输入初值 — 499 Ready / 537·1233 Capturing 等, 见 Design Doc §3.3)。
    pub fn bootstrap(agent: AgentState) -> Self {
        Self {
            agent,
            active_sessions: 0,
        }
    }
    /// 优先级格内抬升: 仅当候选态优先级高于当前态时生效 (Degraded 压 Capturing、
    /// Capturing 抬 Ready; 重置边不经此路径)。
    fn elevate(&mut self, candidate: AgentState) {
        if candidate.precedence() > self.agent.precedence() {
            self.agent = candidate;
        }
    }
}

/// 纯函数: 事件切片 → 折叠状态 (P0-7D Health Reducer)。
///
/// 映射表 (Design Doc §3.2, 14 kinds 逐项):
/// - `HealthChanged{to:"manual_required"}` → ManualRequired (决策权威, 最高优先);
/// - `HealthChanged{to:"recovered"}` → Capturing (有活跃会话) / Ready (归零) — 重置边;
/// - `PipelineFault{retryable:true}` → Degraded (重启排程中, 待 recovered);
/// - `PipelineFault{retryable:false}` / `HardwareFault` → Degraded (等待 escalate 决策);
/// - `SessionFailed` / `AmbiguousIdentity` → Degraded;
/// - `ResourceReservationExpired` → Degraded (资源面运维可见降级);
/// - `SessionCreated` → active_sessions+1 (态不变);
/// - `SessionStateChanged{to:"Released"}` → active_sessions-1; 归零且无更高级 pending → Ready (重置边);
/// - `SessionStateChanged{to:"Running"}` / `SignalVerified` → Capturing 候选
///   (SignalVerified 承载 selftest 无会话路径的 Capturing 派生);
/// - `LeaseGranted`/`ResourceAllocated`/`SourceMaterialized`/`IdentityResolved`/
///   `LoopbackVerified` → 观测记账, 不改主态;
/// - 其余 `SessionStateChanged` (微相位) → 态不变; 未知 `HealthChanged.to` → 不偷改态。
pub fn reduce(state: &HealthFold, events: &[RuntimeEvent]) -> HealthFold {
    let mut fold = state.clone();
    for ev in events {
        match ev {
            RuntimeEvent::HealthChanged { to, .. } => match to.as_str() {
                "manual_required" => fold.agent = AgentState::ManualRequired,
                "recovered" => {
                    // 重置边: 恢复后按会话存在性回落。
                    fold.agent = if fold.active_sessions > 0 {
                        AgentState::Capturing
                    } else {
                        AgentState::Ready
                    };
                }
                _ => {}
            },
            RuntimeEvent::PipelineFault { .. }
            | RuntimeEvent::HardwareFault { .. }
            | RuntimeEvent::SessionFailed { .. }
            | RuntimeEvent::AmbiguousIdentity { .. }
            | RuntimeEvent::ResourceReservationExpired { .. } => {
                // 可重试=重启排程中; 不可重试/硬件/会话失败/拒识/预留过期 → Degraded
                // (统称"故障 pending", 待 recovered 或会话归零重置)。
                if AgentState::Degraded.precedence() > fold.agent.precedence() {
                    fold.agent = AgentState::Degraded;
                }
            }
            RuntimeEvent::SessionCreated { .. } => {
                fold.active_sessions += 1;
            }
            RuntimeEvent::SessionStateChanged { to, .. } => match to.as_str() {
                "Released" | "released" => {
                    fold.active_sessions = fold.active_sessions.saturating_sub(1);
                    // 重置边: 会话归零且无更高级 pending (Degraded 及以上) → Ready。
                    if fold.active_sessions == 0
                        && fold.agent.precedence() < AgentState::Degraded.precedence()
                    {
                        fold.agent = AgentState::Ready;
                    }
                }
                "Running" | "running" => fold.elevate(AgentState::Capturing),
                _ => {}
            },
            RuntimeEvent::SignalVerified { .. } => {
                // 信号已验证 = 采集健康实证 (含 selftest 无会话路径)。
                fold.elevate(AgentState::Capturing);
            }
            RuntimeEvent::LeaseGranted { .. }
            | RuntimeEvent::ResourceAllocated { .. }
            | RuntimeEvent::SourceMaterialized { .. }
            | RuntimeEvent::IdentityResolved { .. }
            | RuntimeEvent::LoopbackVerified { .. } => {
                // 观测记账: 点亮后入流即可投影, 不改主态。
            }
        }
    }
    fold
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn fold(agent: AgentState, sessions: usize) -> HealthFold {
        HealthFold {
            agent,
            active_sessions: sessions,
        }
    }

    fn ev_session(to: &str) -> RuntimeEvent {
        RuntimeEvent::SessionStateChanged {
            session_id: Uuid::new_v4(),
            from: "Starting".into(),
            to: to.into(),
        }
    }

    #[test]
    fn reduce_is_pure_deterministic() {
        let s = fold(AgentState::Ready, 1);
        let events = vec![
            RuntimeEvent::SessionCreated {
                session_id: Uuid::new_v4(),
            },
            RuntimeEvent::PipelineFault {
                pipeline: Uuid::new_v4(),
                summary: "x".into(),
                retryable: true,
            },
        ];
        assert_eq!(reduce(&s, &events), reduce(&s, &events));
        // 原状态不被偷改 (纯函数: 不可变输入)。
        assert_eq!(s, fold(AgentState::Ready, 1));
    }

    #[test]
    fn decision_events_are_authoritative() {
        // manual_required 决策 → 最高优先, 不被 Observation 偷翻转。
        let s = fold(AgentState::ManualRequired, 1);
        let out = reduce(&s, &[ev_session("Running")]);
        assert_eq!(out.agent, AgentState::ManualRequired);
        // recovered 重置边: 有活跃会话 → Capturing。
        let s2 = fold(AgentState::Degraded, 1);
        let out2 = reduce(
            &s2,
            &[RuntimeEvent::HealthChanged {
                from: "restarting".into(),
                to: "recovered".into(),
            }],
        );
        assert_eq!(out2.agent, AgentState::Capturing);
        // 未知 to 词 → 不偷改态 (防御)。
        let s3 = fold(AgentState::Capturing, 0);
        let out3 = reduce(
            &s3,
            &[RuntimeEvent::HealthChanged {
                from: "x".into(),
                to: "mystery_state".into(),
            }],
        );
        assert_eq!(out3.agent, AgentState::Capturing);
    }

    #[test]
    fn faults_derive_degraded_with_precedence() {
        let s = fold(AgentState::Capturing, 1);
        let out = reduce(
            &s,
            &[RuntimeEvent::SessionFailed {
                session_id: Uuid::new_v4(),
                reason: "rollback done".into(),
            }],
        );
        assert_eq!(out.agent, AgentState::Degraded);
        // Degraded 压 Capturing, 无重置边时不被后续 Running 偷翻转。
        let out2 = reduce(&out, &[ev_session("Running")]);
        assert_eq!(out2.agent, AgentState::Degraded);
        // HardwareFault / AmbiguousIdentity / ResourceReservationExpired / 双类 PipelineFault 同格。
        for ev in [
            RuntimeEvent::HardwareFault {
                device_id: Uuid::nil(),
                summary: "lost".into(),
            },
            RuntimeEvent::AmbiguousIdentity {
                device_id: Uuid::nil(),
                candidates: vec!["a".into()],
            },
            RuntimeEvent::ResourceReservationExpired {
                resource_id: Uuid::nil(),
            },
            RuntimeEvent::PipelineFault {
                pipeline: Uuid::nil(),
                summary: "upstream".into(),
                retryable: true,
            },
            RuntimeEvent::PipelineFault {
                pipeline: Uuid::nil(),
                summary: "upstream".into(),
                retryable: false,
            },
        ] {
            let o = reduce(&fold(AgentState::Capturing, 0), &[ev]);
            assert_eq!(o.agent, AgentState::Degraded);
        }
    }

    #[test]
    fn session_lifecycle_folds_to_ready_on_drain() {
        let sid = Uuid::new_v4();
        let s = fold(AgentState::Ready, 0);
        let out = reduce(
            &s,
            &[
                RuntimeEvent::SessionCreated { session_id: sid },
                ev_session("Running"),
                RuntimeEvent::SessionStateChanged {
                    session_id: sid,
                    from: "Running".into(),
                    to: "Released".into(),
                },
            ],
        );
        assert_eq!(out.active_sessions, 0);
        assert_eq!(out.agent, AgentState::Ready);
    }

    #[test]
    fn released_with_degraded_pending_keeps_degraded() {
        // 会话归零但故障 pending 未重置 (无 recovered) → 维持 Degraded, 不偷回 Ready。
        let s = fold(AgentState::Degraded, 1);
        let out = reduce(&s, &[ev_session("Released")]);
        assert_eq!(out.active_sessions, 0);
        assert_eq!(out.agent, AgentState::Degraded);
    }

    #[test]
    fn signal_verified_derives_capturing_selftest_path() {
        // selftest 无会话: SignalVerified (a4 翻真) 是 Capturing 的实证来源。
        let s = fold(AgentState::Ready, 0);
        let out = reduce(
            &s,
            &[RuntimeEvent::SignalVerified {
                device_id: Uuid::nil(),
                port_id: None,
            }],
        );
        assert_eq!(out.agent, AgentState::Capturing);
    }

    #[test]
    fn observation_bookkeeping_does_not_change_state() {
        let s = fold(AgentState::Degraded, 1);
        let out = reduce(
            &s,
            &[
                RuntimeEvent::IdentityResolved {
                    device_id: Uuid::nil(),
                    confidence: "high".into(),
                },
                RuntimeEvent::LoopbackVerified {
                    device_id: Uuid::nil(),
                    port_id: None,
                },
                RuntimeEvent::LeaseGranted {
                    device_id: Uuid::nil(),
                    lease_id: Uuid::nil(),
                },
                RuntimeEvent::ResourceAllocated {
                    resource_id: Uuid::nil(),
                },
                RuntimeEvent::SourceMaterialized {
                    device_id: Uuid::nil(),
                    pipeline: Uuid::nil(),
                },
            ],
        );
        assert_eq!(out.agent, AgentState::Degraded);
        assert_eq!(out.active_sessions, 1);
    }

    #[test]
    fn recovered_with_no_sessions_falls_back_to_ready() {
        let s = fold(AgentState::Degraded, 0);
        let out = reduce(
            &s,
            &[RuntimeEvent::HealthChanged {
                from: "restarting".into(),
                to: "recovered".into(),
            }],
        );
        assert_eq!(out.agent, AgentState::Ready);
    }
}
