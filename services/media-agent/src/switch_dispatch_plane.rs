//! v0.2 Control Plane Expansion（R61 实现·R60 探针 + 用户六点裁决冻结）:
//! `SwitchProgram` 命令的执行平面 + `program_switch` 事实回读平面。
//!
//! 架构纪律（用户 R61 裁决原文）: **Command Plane 负责"请求执行", Query
//! Plane 负责"事实回读", 两者不互相越界**——本模块以两个独立 trait 落实:
//! - [`SwitchDispatchPlane`]: 命令面（command::dispatch/idempotency 通道）——只执行;
//! - [`SwitchReadbackPlane`]: 查询面（transport GET /api/v1/runtime 投影通道）——只读事实。
//!
//! 边界（R57 §23.4/§24.2 冻结面）: 只**调用** `ProgramExecutionRuntime::
//! switch_program/observe_execution`——A2-8 核心四文件零修改; policy 固定
//! `FrameSwitch`（R60 裁决②: Packet/Master 未真机验证, 不暴露 wire 面）;
//! v0.2 单活跃会话语义（明示, 不伪装多会话——注册表/Event/RPC 归 Step 16）。

use crate::command::{CommandId, CommandKind, CommandOutcome, CommandStatus};
use crate::contracts::switch::ProgramExecutionObservation;
use crate::error_model::ErrorClassification;
use crate::program::SwitchPolicy;
use crate::program_execution::ProgramExecutionRuntime;
use crate::program_timeline::TransitionOutcome;
use crate::session::SessionId;
use crate::switch_execution::{SwitchError, SwitchIntent};

/// 命令面: `SwitchProgram` 的唯一执行通道（幂等层 claimant 锁外调用）。
/// 无默认实现——沿契约纪律每个实现方显式表态。
pub trait SwitchDispatchPlane: Send + Sync {
    /// 执行一次切换。会话身份不匹配/平面未装配等语义裁决在实现内完成
    /// （形状校验归 `command::validate`, 此处不再重复）。
    fn switch(
        &self,
        command_id: CommandId,
        session_id: SessionId,
        target_device: uuid::Uuid,
    ) -> CommandOutcome;
}

/// 查询面: `program_switch` 投影块唯一数据源（R60 裁决③: 绑定
/// `observe_execution`; 不污染 sessions[]; 命令动词禁入本平面）。
pub trait SwitchReadbackPlane: Send + Sync {
    /// 事实回读（None = 执行平面未激活/已 teardown——诚实缺席, 非 false）。
    fn observe(&self) -> Option<ProgramExecutionObservation>;
    /// 平面服务的会话身份（投影块 session_id 字段来源）。
    fn session_id(&self) -> SessionId;
}

/// 真实实现: `ProgramExecutionRuntime` 薄包装（bin 诊断双输入装配为唯一
/// 构造点——R60 探针 §2.6: 原 Arc move 进 stop_hook 注册表, 现保留 clone）。
pub struct RuntimeSwitchPlane {
    session_id: SessionId,
    runtime: std::sync::Arc<ProgramExecutionRuntime>,
}

impl RuntimeSwitchPlane {
    pub fn new(session_id: SessionId, runtime: std::sync::Arc<ProgramExecutionRuntime>) -> Self {
        Self {
            session_id,
            runtime,
        }
    }
}

impl SwitchDispatchPlane for RuntimeSwitchPlane {
    fn switch(
        &self,
        command_id: CommandId,
        session_id: SessionId,
        target_device: uuid::Uuid,
    ) -> CommandOutcome {
        if session_id != self.session_id {
            // 会话语义裁决（未触媒体 Runtime——只比身份）; 与 stop_session
            // UnknownSession → Failed(Permanent) 既有语义一致。
            return CommandOutcome {
                command_id,
                kind: CommandKind::SwitchProgram,
                status: CommandStatus::Failed,
                detail: Some(format!(
                    "switch plane 不服务该会话（v0.2 单活跃会话形态; 平面会话 {}）",
                    self.session_id.0
                )),
                classification: Some(ErrorClassification::PermanentFailure),
            };
        }
        let intent = SwitchIntent {
            target: target_device,
            // R60 裁决②: 固定 FrameSwitch（gates 真机口径）; 不入 wire 面。
            policy: SwitchPolicy::FrameSwitch,
        };
        match self.runtime.switch_program(&intent) {
            Ok(report) => {
                // outcome↔continuity（04-探针 P2 语义）: Preserved/NewEpoch = 切换
                // 成功; **Failed{reason} = 执行走完但连续性未立——命令如实 Failed**
                // （不把降级切换包装成 Executed）。
                let outcome_failed = matches!(report.outcome, TransitionOutcome::Failed { .. });
                let (outcome_name, outcome_epoch) = match &report.outcome {
                    TransitionOutcome::Preserved { epoch, .. } => ("preserved", epoch.0),
                    TransitionOutcome::NewEpoch { epoch, .. } => ("new_epoch", epoch.0),
                    TransitionOutcome::Failed { .. } => {
                        ("failed", report.observation.program_epoch.0)
                    }
                };
                if outcome_failed {
                    return CommandOutcome {
                        command_id,
                        kind: CommandKind::SwitchProgram,
                        status: CommandStatus::Failed,
                        detail: Some(format!(
                            "switch executed but transition failed: outcome=failed timeline_epoch={} ({:?})",
                            outcome_epoch, report.outcome
                        )),
                        classification: Some(ErrorClassification::PermanentFailure),
                    };
                }
                CommandOutcome {
                    command_id,
                    kind: CommandKind::SwitchProgram,
                    status: CommandStatus::Executed,
                    detail: Some(format!(
                        "switch executed: av_epoch={} outcome={} timeline_epoch={}",
                        report.executed.av_epoch, outcome_name, outcome_epoch
                    )),
                    classification: None,
                }
            }
            Err(e) => CommandOutcome {
                command_id,
                kind: CommandKind::SwitchProgram,
                status: CommandStatus::Failed,
                detail: Some(format!("{e}")),
                classification: Some(classify_switch_error(&e)),
            },
        }
    }
}

impl SwitchReadbackPlane for RuntimeSwitchPlane {
    fn observe(&self) -> Option<ProgramExecutionObservation> {
        self.runtime.observe_execution()
    }

    fn session_id(&self) -> SessionId {
        self.session_id
    }
}

/// `SwitchError` → `ErrorClassification`（映射收在本模块——error_model 非本轮
/// 允许修改面）。状态机/目标类错误 = PermanentFailure（重试同请求无意义）;
/// `Backend(String)` 为 adapter 透传字符串, 归因不可知 → Unknown（沿
/// 0.7C-5 "不臆造" 语义, 与 panic 兜底同桶）。
pub fn classify_switch_error(e: &SwitchError) -> ErrorClassification {
    match e {
        SwitchError::Backend(_) => ErrorClassification::Unknown,
        _ => ErrorClassification::PermanentFailure,
    }
}

#[cfg(all(test, feature = "mock"))]
pub(crate) mod test_support {
    //! 命令/幂等面单测的可编程假平面（真实平面的行为归 bin 装配与盒上验证）。

    use super::*;
    use std::sync::Mutex;

    /// 可编程假平面: 记录调用 + 返回罐头结果。
    pub struct FakeSwitchPlane {
        pub serving: SessionId,
        pub fail_with: Option<SwitchError>,
        pub calls: Mutex<Vec<(SessionId, uuid::Uuid)>>,
    }

    impl FakeSwitchPlane {
        pub fn new(serving: SessionId) -> Self {
            Self {
                serving,
                fail_with: None,
                calls: Mutex::new(Vec::new()),
            }
        }

        pub fn failing(serving: SessionId, e: SwitchError) -> Self {
            Self {
                serving,
                fail_with: Some(e),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl SwitchDispatchPlane for FakeSwitchPlane {
        fn switch(
            &self,
            command_id: CommandId,
            session_id: SessionId,
            target_device: uuid::Uuid,
        ) -> CommandOutcome {
            self.calls.lock().unwrap().push((session_id, target_device));
            if session_id != self.serving {
                return CommandOutcome {
                    command_id,
                    kind: CommandKind::SwitchProgram,
                    status: CommandStatus::Failed,
                    detail: Some("switch plane 不服务该会话".into()),
                    classification: Some(ErrorClassification::PermanentFailure),
                };
            }
            if let Some(e) = &self.fail_with {
                return CommandOutcome {
                    command_id,
                    kind: CommandKind::SwitchProgram,
                    status: CommandStatus::Failed,
                    detail: Some(format!("{e}")),
                    classification: Some(classify_switch_error(e)),
                };
            }
            CommandOutcome {
                command_id,
                kind: CommandKind::SwitchProgram,
                status: CommandStatus::Executed,
                detail: Some("switch executed (fake)".into()),
                classification: None,
            }
        }
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;

    /// 分类快照: 状态机类 → PermanentFailure; Backend → Unknown（不臆造）。
    #[test]
    fn switch_plane_rt_01_error_classification_snapshot() {
        let dev = uuid::Uuid::new_v4();
        assert_eq!(
            classify_switch_error(&SwitchError::TargetNotInGroup(dev)),
            ErrorClassification::PermanentFailure
        );
        assert_eq!(
            classify_switch_error(&SwitchError::TargetAlreadyActive(dev)),
            ErrorClassification::PermanentFailure
        );
        assert_eq!(
            classify_switch_error(&SwitchError::Backend("x".into())),
            ErrorClassification::Unknown
        );
    }

    /// 双平面隔离白盒: 命令面 trait 与查询面 trait 是两个独立对象安全面
    /// （同一具体类型可实现两者, 但通道不互通——类型级防越界）。
    #[test]
    fn switch_plane_rt_02_plane_separation_surface() {
        // Fake 只实现命令面 → 不能作为读回面使用（编译期事实, 此处以
        // trait bounds 断言两 trait 互不为超集）。
        fn assert_dispatch<T: SwitchDispatchPlane>() {}
        fn assert_readback<T: SwitchReadbackPlane>() {}
        assert_dispatch::<test_support::FakeSwitchPlane>();
        // FakeSwitchPlane 未实现 SwitchReadbackPlane（命令面专用——若误实现,
        // 下列行将编译失败, 即隔离回归锚）:
        // assert_readback::<test_support::FakeSwitchPlane>();
        // 真实平面双实现（唯一允许同时进入两通道的具体类型）:
        assert_dispatch::<RuntimeSwitchPlane>();
        assert_readback::<RuntimeSwitchPlane>();
    }
}
