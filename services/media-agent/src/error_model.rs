//! Phase 0.7C-5: Error Model Foundation — **失败归因分类平面**。
//!
//! 第一道架构红线（终审 2026-08-31 冻结）:
//! `CommandStatus ≠ IdempotentDispatch ≠ ErrorClassification` — 三维度
//! （执行状态 / 幂等裁决 / 错误分类）各自独立 enum, **绝不合并成万能 CommandResult**。
//!
//! 本模块只补第三个平面:
//! - 词表封闭五项, 零字段单元变体（不嵌套任何其他平面数据）;
//! - `classify_session_error` 纯函数, **match 无通配臂** — SessionError 新增
//!   变体时编译失败, 强制逐臂架构评审（与"新命令=架构评审事件"同构）;
//! - 分类在**错误边界处**产生（dispatch 三臂的 Err(e) 分支, 错误仍为类型态）,
//!   绝不从 detail 字符串事后恢复（脆弱 + 违背"文档语义>实现行为"）。
//!
//! 红线延续: 零 vendor 依赖 / 零 runtime_query 引用（Query/Command 分离）/
//! SessionError 类型零改动（分类是纯函数投影）。

#![allow(dead_code)]

use crate::lease::LeaseError;
use crate::session::SessionError;

/// 错误分类 — 失败归因（独立分类平面）。
///
/// 调用方动作语义（每类一个可辩护的下一步）:
/// - `Rejected` — 修改请求后重发;
/// - `Conflict` — 修正 command_id 或 payload;
/// - `RetryableFailure` — 换时机重试同请求（可结合 Query 观察）;
/// - `PermanentFailure` — 不要再试（查 Query 修正认知）;
/// - `Unknown` — 不臆造, 走运维/日志（D6 三态 Unknown 先例）。
///
/// **不纳入**（防词表膨胀成万能分类, design.md §2）:
/// - `InProgress` — 0.7C-4 dispatch 是同步等待语义, 无此终态出口（属未来 async 查询面）;
/// - `AlreadyApplied` — 无对应场景（create 幂等命中已存在资源的场景不存在）;
/// - `Duplicate` — 不是错误, = `IdempotentDispatch::Replayed`（幂等平面已表达）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClassification {
    /// 形状/契约拒绝（validate 四拒绝路径）。
    Rejected,
    /// command_id 被不同 payload 占用（D9-B ID 复用/语义碰撞）。
    Conflict,
    /// 可重试失败（现实不满足但随时间变化）。
    RetryableFailure,
    /// 永久失败（时序/状态机错误、目标不存在、记账错误——重试同请求无意义）。
    PermanentFailure,
    /// 无法归因（claimant panic 兜底）。
    Unknown,
}

/// 纯函数: `SessionError` → 分类。**封闭映射**（design.md §4 详表, 逐臂测试锁定）。
///
/// 分类哲学: 预检/资源/租约类"现实暂时不满足"→ Retryable（现实随时间变化）;
/// 状态机/目标存在性/记账类"调用方时序或认知错误"→ Permanent（重试同序列仍错）。
pub fn classify_session_error(err: &SessionError) -> ErrorClassification {
    match err {
        // 预检 = 现实 vs 请求的 judge-only 判定; 端口无信号/资源暂占随时间变化。
        // (注: 能力 Unsupported 子情形本质 Permanent, 但分类粒度按报告整体保守判
        //  Retryable —— 细化属演进项, 不臆造。)
        SessionError::PreflightFailed(_) => ErrorClassification::RetryableFailure,
        // 资源被他会话占用, 释放后可重试。
        SessionError::ResourceConflict(_) => ErrorClassification::RetryableFailure,
        // 资源状态机拒绝（如 release 未 Reserved）— 调用方时序错误。
        SessionError::ResourceState(_) => ErrorClassification::PermanentFailure,
        // 设备被占 / TTL 到期 → 重新 acquire 有意义; 释放不存在的租约 = 记账错误。
        SessionError::Lease(LeaseError::AlreadyLeased(_)) => ErrorClassification::RetryableFailure,
        SessionError::Lease(LeaseError::Expired) => ErrorClassification::RetryableFailure,
        SessionError::Lease(LeaseError::NotFound(_)) => ErrorClassification::PermanentFailure,
        // 目标会话不存在（ghost）; 真机 ERROR-MODEL-RT-01 探针实证项。
        SessionError::UnknownSession(_) => ErrorClassification::PermanentFailure,
        // 会话状态机白名单拒绝（如 close Released 会话）。
        SessionError::InvalidTransition(_) => ErrorClassification::PermanentFailure,
        // 管线执行期错误 — Supervisor 领地, 0.6 起有 recover/retry 语义。
        SessionError::Pipeline(_) => ErrorClassification::RetryableFailure,
        // 后端暂时不可用。
        SessionError::BackendUnavailable(_) => ErrorClassification::RetryableFailure,
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::command::{CommandEnvelope, CommandId, CommandKind, CommandStatus, CommandTarget};
    use crate::idempotency::{CommandIdempotency, IdempotentDispatch};
    use crate::session::{SessionId, SessionManager};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    /// **红线白盒**: 公开关联函数清单 — 仅 classify_session_error 一方法。
    /// 执行/查询动词禁入（本模块无 dispatch/get/list 面）。
    const PUBLIC_SURFACE_ALLOWLIST: &[&str] = &["classify_session_error"];
    const BANNED_VERBS: &[&str] = &[
        "execute_", "configure_", "run_", "build_", "emit", "send", "publish", "spawn", "get_",
        "list_",
    ];

    fn world() -> Arc<SessionManager> {
        use crate::adapters::mock::MockProvider;
        use crate::contracts::provider::HardwareProvider as _;
        use crate::port::*;
        let devices: Vec<crate::device::DeviceInfo> = MockProvider
            .discover()
            .expect("mock discover")
            .into_iter()
            .map(|d| d.device)
            .collect();
        let pid =
            PortIdentity::derive(&devices[0].device_id, ConnectorType::Sdi, PortOrdinal::Known(1));
        let registry = PortRegistry {
            ports: vec![PortInfo {
                device_id: devices[0].device_id,
                provider_binding_ref: None,
                identity: PortIdentity {
                    port_id: pid,
                    connector: ConnectorType::Sdi,
                    ordinal: PortOrdinal::Known(1),
                },
                direction: PortDirection::Input,
                capabilities: PortCapabilities::default(),
                runtime_binding: None,
                signal: SignalStatus::default(),
                content: VideoContentState::Unknown,
            }],
        };
        Arc::new(SessionManager::new(
            crate::resource::SharedResourceRegistry::new(
                crate::resource::ResourceRegistry::derive_from_discovery(&registry),
            ),
            Arc::new(crate::lease::InMemoryLeaseManager::new()),
            Arc::new(Mutex::new(crate::supervisor::Supervisor::new(
                crate::supervisor::RestartPolicy::default(),
            ))),
            Arc::new(crate::adapters::mock::MockBackend),
            Arc::new(devices),
            Arc::new(HashMap::new()),
            Some(registry),
            crate::pipeline::MaterializeMode::Diagnostic,
            crate::session::SessionTuning::default(),
        ))
    }

    fn start_env() -> CommandEnvelope {
        let mgr = world();
        let dev = mgr.runtime_state().devices[0].device_id;
        CommandEnvelope {
            command_id: CommandId(Uuid::new_v4()),
            kind: CommandKind::StartSession,
            target: CommandTarget::Session {
                intent: crate::graph_intent::GraphRuntimeIntent {
                    version: "1.0".into(),
                    devices: vec![crate::graph_intent::DeviceIntent {
                        device_id: dev.to_string(),
                        role: "CAPTURE".into(),
                        pipeline: crate::graph_intent::PipelineIntent {
                            source: crate::graph_intent::SourceIntent {
                                kind: "decklink".into(),
                                device_id: dev.to_string(),
                                port_id: None,
                            },
                            sink: crate::graph_intent::SinkIntent {
                                kind: "appsink".into(),
                            },
                        },
                    }],
                },
            },
            issued_at_ms: 0,
            requested_by: "error-model-test".into(),
        }
    }

    /// **词表快照** — 五变体封闭（serde snake_case）+ 零字段单元变体
    /// （序列化仅分类标签, 不嵌套任何其他平面数据 — 禁万能分类）。
    #[test]
    fn err_model_rt_01_vocabulary_snapshot() {
        let cases = [
            (ErrorClassification::Rejected, "\"rejected\""),
            (ErrorClassification::Conflict, "\"conflict\""),
            (
                ErrorClassification::RetryableFailure,
                "\"retryable_failure\"",
            ),
            (
                ErrorClassification::PermanentFailure,
                "\"permanent_failure\"",
            ),
            (ErrorClassification::Unknown, "\"unknown\""),
        ];
        for (value, tag) in cases {
            let json = serde_json::to_string(&value).expect("serialize");
            assert_eq!(json, tag, "词表快照失配");
            let back: ErrorClassification = serde_json::from_str(&json).expect("roundtrip");
            assert_eq!(value, back);
        }
    }

    /// **封闭映射矩阵** — design.md §4 详表 10 case 逐臂断言
    /// （match 无通配臂为编译级保证: SessionError 新增变体时本函数编译失败）。
    #[test]
    fn err_model_rt_01_classify_matrix_closed_mapping() {
        use crate::preflight::{PreflightReport, PreflightStage, StageLevel, StageOutcome, Verdict};
        use crate::resource::ResourceState;
        use crate::resource::ResourceStateError;
        let preflight_fail = PreflightReport {
            stages: vec![StageOutcome {
                stage: PreflightStage::PortAvailability,
                level: StageLevel::Fail,
                detail: "no input signal".into(),
            }],
            verdict: Verdict::Fail,
        };
        let cases: Vec<(SessionError, ErrorClassification)> = vec![
            (
                SessionError::PreflightFailed(preflight_fail),
                ErrorClassification::RetryableFailure,
            ),
            (
                SessionError::ResourceConflict("port busy".into()),
                ErrorClassification::RetryableFailure,
            ),
            (
                SessionError::ResourceState(ResourceStateError {
                    from: ResourceState::Reserved,
                    to: ResourceState::Released,
                }),
                ErrorClassification::PermanentFailure,
            ),
            (
                SessionError::Lease(LeaseError::AlreadyLeased(Uuid::nil())),
                ErrorClassification::RetryableFailure,
            ),
            (
                SessionError::Lease(LeaseError::Expired),
                ErrorClassification::RetryableFailure,
            ),
            (
                SessionError::Lease(LeaseError::NotFound(Uuid::nil())),
                ErrorClassification::PermanentFailure,
            ),
            (
                SessionError::UnknownSession(SessionId(Uuid::nil())),
                ErrorClassification::PermanentFailure,
            ),
            (
                SessionError::InvalidTransition("close Released".into()),
                ErrorClassification::PermanentFailure,
            ),
            (
                SessionError::Pipeline(crate::pipeline::PipelineError::PrepareFailed(
                    "src link fail".into(),
                )),
                ErrorClassification::RetryableFailure,
            ),
            (
                SessionError::BackendUnavailable("no backend".into()),
                ErrorClassification::RetryableFailure,
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(classify_session_error(&err), expected, "映射失配: {err}");
        }
    }

    /// **outcome 分类不变量** — Failed⇒Some(非 Rejected/Conflict) /
    /// Rejected⇒Some(Rejected) / Executed⇒None。
    #[test]
    fn err_model_rt_01_outcome_invariant() {
        let mgr = world();
        let idem = CommandIdempotency::new(Arc::clone(&mgr));
        // Executed ⇒ None。
        let out = crate::command::dispatch(&mgr, &start_env());
        assert_eq!(out.status, CommandStatus::Executed);
        assert!(
            out.classification.is_none(),
            "Executed 不得携带分类: {:?}",
            out.classification
        );
        // Failed (ghost stop, UnknownSession 臂) ⇒ Some(PermanentFailure)。
        let ghost = CommandEnvelope {
            command_id: CommandId(Uuid::new_v4()),
            kind: CommandKind::StopSession,
            target: CommandTarget::SessionById {
                session_id: SessionId(Uuid::new_v4()),
            },
            issued_at_ms: 0,
            requested_by: "t".into(),
        };
        let out = crate::command::dispatch(&mgr, &ghost);
        assert_eq!(out.status, CommandStatus::Failed);
        assert_eq!(
            out.classification,
            Some(ErrorClassification::PermanentFailure),
            "ghost stop 必须归因 Permanent (UnknownSession 臂)"
        );
        // Rejected ⇒ Some(Rejected)。
        let mut bad = start_env();
        bad.requested_by = String::new();
        let out = crate::command::dispatch(&mgr, &bad);
        assert_eq!(out.status, CommandStatus::Rejected);
        assert_eq!(out.classification, Some(ErrorClassification::Rejected));
        // 幂等重放: 同一命令重放同一归因 (D9-D 逐字节重放天然涵盖)。
        let sid = mgr.list()[0].session_id;
        let stop = CommandEnvelope {
            command_id: CommandId(Uuid::new_v4()),
            kind: CommandKind::StopSession,
            target: CommandTarget::SessionById { session_id: sid },
            issued_at_ms: 0,
            requested_by: "t".into(),
        };
        let first = idem.dispatch(&stop);
        let replay = idem.dispatch(&stop);
        match (first, replay) {
            (
                IdempotentDispatch::Executed(a),
                IdempotentDispatch::Replayed(b),
            ) => {
                assert_eq!(a, b, "重放 outcome 必须含相同 classification");
                assert_eq!(a.classification, None, "成功 stop 无分类");
            }
            other => panic!("期望 Executed+Replayed, 实得 {other:?}"),
        }
    }

    /// **三平面分离白盒** — CommandStatus 4 变体与 IdempotentDispatch 4 出口
    /// 词表零改动回归; ErrorClassification allowlist + 动词禁入。
    #[test]
    fn err_model_rt_01_three_plane_separation() {
        // 平面一: CommandStatus (0.7C-3 冻结) — 快照回归。
        assert_eq!(
            serde_json::to_string(&CommandStatus::Rejected).unwrap(),
            "\"rejected\""
        );
        assert_eq!(
            serde_json::to_string(&CommandStatus::Failed).unwrap(),
            "\"failed\""
        );
        // 平面二: IdempotentDispatch (0.7C-4 冻结) — verdict 标签回归。
        let probe = serde_json::to_value(IdempotentDispatch::Rejected(crate::command::CommandRejection {
            code: "x".into(),
            detail: "d".into(),
        }))
        .unwrap();
        assert_eq!(probe["verdict"], "rejected");
        // 平面三: ErrorClassification — allowlist 恒等 + 动词禁入。
        assert_eq!(PUBLIC_SURFACE_ALLOWLIST, &["classify_session_error"]);
        for name in PUBLIC_SURFACE_ALLOWLIST {
            for verb in BANNED_VERBS {
                assert!(!name.starts_with(verb), "禁入动词 {verb} 出现在分类面: {name}");
            }
        }
        // 零字段单元变体: 序列化是纯字符串, 无嵌套结构可携带其他平面数据。
        for c in [
            ErrorClassification::Rejected,
            ErrorClassification::Conflict,
            ErrorClassification::RetryableFailure,
            ErrorClassification::PermanentFailure,
            ErrorClassification::Unknown,
        ] {
            let v = serde_json::to_value(c).unwrap();
            assert!(v.is_string(), "分类必须是纯标签: {v}");
        }
    }
}
