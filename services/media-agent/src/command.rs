//! Phase 0.7C-3: Command Contract Foundation — **请求语义, 非执行计划**。
//!
//! 设计原则（终审 2026-08-31 冻结）:
//! - `Query  = What is true now?`（runtime_query.rs, Pure Read）
//! - `Command = What do I request to change?`（本模块）
//! - **第一红线（不可执行性）**: Command 只表达"请求改变什么", 绝不携带
//!   Backend/GStreamer/FFmpeg/DeviceHandle/Pipeline 等执行细节——三重守护:
//!   ①类型层仅 canonical 类型; ②serde 反向断言; ③公开面 allowlist。
//! - validation 与 execution 分离; dispatch 是 **match 三臂薄映射**
//!   （无命令循环/插件/注册机制/命令总线——终审禁"万能 CommandExecutor"）。
//! - Query/Command 分离: 本模块零 `runtime_query` 引用（反之亦然）。
//!
//! command_id 为幂等键**占位**（携带不实现——D9 幂等语义属下一 change）。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::graph_intent::GraphRuntimeIntent;
use crate::session::{SessionId, SessionManager};

/// 命令词表 — **封闭枚举**（三命令; 新命令须过架构评审并显式更新词表快照测试）。
/// 同后缀 Session 是有意的命令域命名 (allow: 终审冻结的命令词汇表)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)] // 命令域词汇表刻意同后缀 (Start/Stop/Release Session)
pub enum CommandKind {
    StartSession,
    StopSession,
    ReleaseSession,
}

/// 命令 ID（幂等键占位: D9 幂等语义属下一 change）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId(pub Uuid);

/// 命令目标 — 仅 canonical 类型（GraphRuntimeIntent / SessionId）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum CommandTarget {
    /// StartSession 用: canonical intent（控制面语义图, 零执行字段）。
    Session { intent: GraphRuntimeIntent },
    /// StopSession/ReleaseSession 用。
    SessionById { session_id: SessionId },
}

/// 命令信封 — **零执行字段**（serde 反向断言禁 gst/pipeline/device_number 等）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub command_id: CommandId,
    pub kind: CommandKind,
    pub target: CommandTarget,
    pub issued_at_ms: u64,
    /// opaque 请求方标签（非身份模型; 认证/身份属 External API 阶段）。
    pub requested_by: String,
}

/// 命令生命周期结果（命令层语义, 非 Runtime 状态投影）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    /// 验证通过（尚未执行）。
    Accepted,
    /// 验证拒绝（未触 Runtime）。
    Rejected,
    /// 薄映射执行完成。
    Executed,
    /// 执行期错误（Runtime 侧已按既有回滚语义处理）。
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOutcome {
    pub command_id: CommandId,
    pub kind: CommandKind,
    pub status: CommandStatus,
    pub detail: Option<String>,
}

/// 验证拒绝（形状层; 绝不触 Runtime）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRejection {
    pub code: String,
    pub detail: String,
}

/// **Validation（纯函数, 与 execution 分离）**——只做信封形状校验:
/// 不读取 Runtime 状态、不依赖 runtime_query（Query/Command 分离白盒）。
pub fn validate(env: &CommandEnvelope) -> Result<(), CommandRejection> {
    if env.requested_by.trim().is_empty() {
        return Err(CommandRejection {
            code: "empty_requester".into(),
            detail: "requested_by 不得为空 (opaque 请求方标签)".into(),
        });
    }
    match (env.kind, &env.target) {
        (CommandKind::StartSession, CommandTarget::Session { intent }) => {
            if intent.devices.is_empty() {
                return Err(CommandRejection {
                    code: "empty_intent".into(),
                    detail: "StartSession 目标 intent 无设备".into(),
                });
            }
            Ok(())
        }
        (CommandKind::StartSession, _) => Err(CommandRejection {
            code: "kind_target_mismatch".into(),
            detail: "StartSession 需要 Session{intent} 目标".into(),
        }),
        (
            CommandKind::StopSession | CommandKind::ReleaseSession,
            CommandTarget::SessionById { session_id },
        ) => {
            if session_id.0 == Uuid::nil() {
                return Err(CommandRejection {
                    code: "nil_session_id".into(),
                    detail: "session_id 不得为 nil UUID".into(),
                });
            }
            Ok(())
        }
        (CommandKind::StopSession | CommandKind::ReleaseSession, _) => Err(CommandRejection {
            code: "kind_target_mismatch".into(),
            detail: "StopSession/ReleaseSession 需要 SessionById 目标".into(),
        }),
    }
}

/// **Command → Runtime lifecycle boundary（薄映射, 非 Executor）**——
/// match 三臂各调 SessionManager 公共 API; 无循环/插件/注册/总线。
/// 验证拒绝不触 Runtime; 执行期错误由 SessionManager 既有回滚语义处理。
pub fn dispatch(mgr: &SessionManager, env: &CommandEnvelope) -> CommandOutcome {
    let outcome = |status: CommandStatus, detail: Option<String>| CommandOutcome {
        command_id: env.command_id,
        kind: env.kind,
        status,
        detail,
    };
    if let Err(rej) = validate(env) {
        return outcome(
            CommandStatus::Rejected,
            Some(format!("{}: {}", rej.code, rej.detail)),
        );
    }
    match (env.kind, &env.target) {
        (CommandKind::StartSession, CommandTarget::Session { intent }) => {
            match mgr
                .create(intent.clone())
                .and_then(|sid| mgr.start(&sid).map(|_| sid))
            {
                Ok(_) => outcome(CommandStatus::Executed, None),
                Err(e) => outcome(CommandStatus::Failed, Some(format!("{e}"))),
            }
        }
        (CommandKind::StopSession, CommandTarget::SessionById { session_id }) => {
            match mgr.stop(session_id) {
                Ok(()) => outcome(CommandStatus::Executed, None),
                Err(e) => outcome(CommandStatus::Failed, Some(format!("{e}"))),
            }
        }
        (CommandKind::ReleaseSession, CommandTarget::SessionById { session_id }) => {
            match mgr.close(session_id) {
                Ok(()) => outcome(CommandStatus::Executed, None),
                Err(e) => outcome(CommandStatus::Failed, Some(format!("{e}"))),
            }
        }
        // validate 已保证形状匹配; 此臂不可达。
        _ => outcome(
            CommandStatus::Rejected,
            Some("unreachable: validated shape".into()),
        ),
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::session::SessionState;
    use std::sync::Arc;

    /// **不可执行性红线白盒**: 公开关联函数清单——仅 validate/dispatch 二方法
    /// （+类型构造）; execute_pipeline/configure_backend 等执行动词禁入。
    const PUBLIC_SURFACE_ALLOWLIST: &[&str] = &["validate", "dispatch"];
    const BANNED_VERBS: &[&str] = &[
        "execute_pipeline",
        "configure_backend",
        "run_backend",
        "build_gst",
        "emit",
        "send",
        "publish",
        "spawn",
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
        let pid = PortIdentity::derive(
            &devices[0].device_id,
            ConnectorType::Sdi,
            PortOrdinal::Known(1),
        );
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
            Arc::new(std::sync::Mutex::new(crate::supervisor::Supervisor::new(
                crate::supervisor::RestartPolicy::default(),
            ))),
            Arc::new(crate::adapters::mock::MockBackend),
            Arc::new(devices),
            Arc::new(std::collections::HashMap::new()),
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
                intent: GraphRuntimeIntent {
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
            requested_by: "command-contract-test".into(),
        }
    }

    #[test]
    fn command_rt_01_vocabulary_snapshot() {
        // 词表快照 (封闭三命令; 新命令须过架构评审并显式更新本断言)。
        assert_eq!(
            serde_json::to_string(&CommandKind::StartSession).unwrap(),
            "\"start_session\""
        );
        assert_eq!(
            serde_json::to_string(&CommandKind::StopSession).unwrap(),
            "\"stop_session\""
        );
        assert_eq!(
            serde_json::to_string(&CommandKind::ReleaseSession).unwrap(),
            "\"release_session\""
        );
    }

    #[test]
    fn command_rt_01_non_executability_serde_and_surface() {
        // ②serde 反向断言: envelope 零执行细节字样。
        let env = start_env();
        let json = serde_json::to_string(&env).expect("serialize");
        // 注: "pipeline" 不在禁列——它是 canonical GraphRuntimeIntent 的冻结 schema
        // 键名 (devices[].pipeline: PipelineIntent), 非执行细节; 禁的是执行器/handler/
        // vendor 地址类字段值 (device_number/handle/backend 等)。
        for banned in [
            "gst",
            "device_number",
            "backend",
            "handle",
            "ffmpeg",
            "alsa",
            "kafka",
            "nats",
            "decklinkvideosrc",
            "ffmpeg args",
            "provider object",
        ] {
            assert!(
                !json.to_lowercase().contains(banned),
                "执行细节渗入命令信封: {banned}"
            );
        }
        // serde roundtrip。
        let back: CommandEnvelope = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(env.command_id, back.command_id);
        // ③公开面白盒: allowlist 恒等 + 执行动词禁入。
        assert_eq!(PUBLIC_SURFACE_ALLOWLIST, &["validate", "dispatch"]);
        for name in PUBLIC_SURFACE_ALLOWLIST {
            for verb in BANNED_VERBS {
                assert!(!name.starts_with(verb), "执行动词禁入命令面: {name}");
            }
        }
    }

    #[test]
    fn command_rt_01_validation_rejection_paths() {
        let mut env = start_env();
        // empty_requester
        env.requested_by = "  ".into();
        let r = validate(&env).unwrap_err();
        assert_eq!(r.code, "empty_requester");
        // kind_target_mismatch (Start + SessionById)
        env.requested_by = "t".into();
        env.target = CommandTarget::SessionById {
            session_id: SessionId(Uuid::new_v4()),
        };
        assert_eq!(validate(&env).unwrap_err().code, "kind_target_mismatch");
        // nil_session_id (Stop + nil)
        env.kind = CommandKind::StopSession;
        env.target = CommandTarget::SessionById {
            session_id: SessionId(Uuid::nil()),
        };
        assert_eq!(validate(&env).unwrap_err().code, "nil_session_id");
        // empty_intent (Start + 空 intent)
        let mut e2 = start_env();
        if let CommandTarget::Session { intent } = &mut e2.target {
            intent.devices.clear();
        }
        assert_eq!(validate(&e2).unwrap_err().code, "empty_intent");
        // 通过路径。
        assert!(validate(&start_env()).is_ok());
    }

    #[test]
    fn command_rt_01_simulation_full_lifecycle_and_failure_paths() {
        let mgr = world();
        // Rejected: 验证拒绝不触 Runtime (会话表空)。
        let mut bad = start_env();
        bad.requested_by = String::new();
        let out = dispatch(&mgr, &bad);
        assert_eq!(out.status, CommandStatus::Rejected);
        assert!(mgr.list().is_empty(), "Rejected 不得触 Runtime");
        // Executed: Start → 会话 Running (经 mgr 状态可见)。
        let env = start_env();
        let out = dispatch(&mgr, &env);
        assert_eq!(
            out.status,
            CommandStatus::Executed,
            "detail={:?}",
            out.detail
        );
        let sid = mgr.list()[0].session_id;
        assert_eq!(mgr.status(&sid).unwrap().state, SessionState::Running);
        // Stop → Executed + Released。
        let stop = CommandEnvelope {
            command_id: CommandId(Uuid::new_v4()),
            kind: CommandKind::StopSession,
            target: CommandTarget::SessionById { session_id: sid },
            issued_at_ms: 0,
            requested_by: "t".into(),
        };
        assert_eq!(dispatch(&mgr, &stop).status, CommandStatus::Executed);
        assert_eq!(mgr.status(&sid).unwrap().state, SessionState::Released);
        // Failed: Stop 不存在的会话。
        let ghost = CommandEnvelope {
            command_id: CommandId(Uuid::new_v4()),
            kind: CommandKind::StopSession,
            target: CommandTarget::SessionById {
                session_id: SessionId(Uuid::new_v4()),
            },
            issued_at_ms: 0,
            requested_by: "t".into(),
        };
        let out = dispatch(&mgr, &ghost);
        assert_eq!(out.status, CommandStatus::Failed);
        // Release → Executed + 会话移除。
        let release = CommandEnvelope {
            command_id: CommandId(Uuid::new_v4()),
            kind: CommandKind::ReleaseSession,
            target: CommandTarget::SessionById { session_id: sid },
            issued_at_ms: 0,
            requested_by: "t".into(),
        };
        assert_eq!(dispatch(&mgr, &release).status, CommandStatus::Executed);
        assert!(mgr.status(&sid).is_none());
    }

    #[test]
    fn command_rt_01_query_command_separation() {
        // Query/Command 分离: 本模块公开面无 Query 方法; (反向由 runtime_query
        // allowlist 无命令动词保证——0.7C-2 已锁定)。
        for name in PUBLIC_SURFACE_ALLOWLIST {
            assert!(
                !name.starts_with("get_") && !name.starts_with("list_"),
                "查询动词禁入命令面: {name}"
            );
        }
    }
}
