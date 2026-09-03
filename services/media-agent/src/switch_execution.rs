//! A2-8-01: Switch Execution —— Program-level 双输入切换的**执行面**模型。
//!
//! **定位（用户两轮终裁, probe §7）**: Semantic Intent ≠ Execution Plan ≠
//! Execution Fact 的执行段。本模块是纯模型（零 GStreamer 依赖——topology
//! = 实现细节, 冻结 #5）, 描述 Execution Group 边界与 Desired 状态平面;
//! 实际 Program graph 物化/切换由 `SwitchExecutionAdapter`
//! （`contracts/switch.rs`, 与 `MediaBackend` 生命周期五方法**平行**,
//! 冻结 #2）承担。
//!
//! **可以做**: 定义 Execution Group（恰双输入 fail-closed, 复用
//! `SessionInput`——零第二 identity, 冻结 #1）/ 校验显式手动切换 Intent
//! （首版仅 FRAME_SWITCH, PACKET/MASTER fail-closed 拒收, 冻结 #6+T12）/
//! 维护 Desired 状态（ACTIVE / SWITCHING——与 Session lifecycle 状态空间
//! **绝对分离**, 禁 `Session.active_input`/`SessionInput.is_active`）。
//!
//! **不能做**: 不构建 GStreamer graph（SessionManager 亦不构图, 冻结 #3）/
//! 不执行 recovery（Supervisor = recovery only, 冻结 #4）/ 不自动 failover
//! （无任何隐式触发切换入口, 冻结 #10）/ 不改 `SwitchPolicy` 领域定义
//! （T11——本模块只**消费**该封闭词表）。
//!
//! **状态三分离（终裁 §7.4）**: Desired（本模块 `SwitchDesired`）≠
//! Execution（adapter 内 selector 实态）≠ Observed（`ProgramObservation`,
//! adapter 观测返回）。`complete_switch` 仅在 Observed 确认 target 后推进
//! Desired——Observation 驱动, 非命令回显。

use crate::pipeline::PipelineHandle;
use crate::program::SwitchPolicy;
use crate::session::{SessionId, SessionInput};
use uuid::Uuid;

/// Switch Execution 封闭错误词表（fail-closed: 无 silent 回退, 全部可观测）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SwitchError {
    #[error("execution group requires exactly two inputs, got {0}")]
    GroupNotDual(usize),
    #[error("duplicate device {0} in execution group")]
    DuplicateDevice(Uuid),
    #[error("initial active source {0} not in group")]
    InitialActiveNotInGroup(Uuid),
    #[error("switch target {0} not in execution group")]
    TargetNotInGroup(Uuid),
    #[error("switch policy {0:?} not supported in first version (FRAME_SWITCH only)")]
    UnsupportedPolicy(SwitchPolicy),
    #[error("target {0} is already the active source")]
    TargetAlreadyActive(Uuid),
    #[error("desired state is not an active source (switching in progress: {0:?})")]
    NotActiveSource(SwitchDesired),
    #[error("stale switch plan epoch {got} (expected {expected})")]
    StalePlanEpoch { got: u64, expected: u64 },
    #[error("program graph {0:?} not running")]
    GraphNotRunning(PipelineHandle),
    #[error("switch execution backend error: {0}")]
    Backend(String),
}

/// Desired 状态平面（终裁 §7.4 Domain/Intent 层: ACTIVE_A/ACTIVE_B/SWITCHING）。
///
/// 按 device_id 标识源（组内恰双输入, 输入身份=SessionInput——不新造
/// A/B 枚举, 防位置序脆弱与第二 identity）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwitchDesired {
    ActiveInput(Uuid),
    Switching { from: Uuid, to: Uuid },
}

/// 显式手动切换 Intent（终裁: 首版 Manual Switch——无 from 字段, 当前源
/// 由 Group Desired 唯一持有, 防调用方预归因第二 SoT）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SwitchIntent {
    pub target: Uuid,
    pub policy: SwitchPolicy,
}

/// 校验后的切换执行计划（Intent→Plan 单向: `ExecutionGroup::plan_switch`
/// fail-closed 产出; epoch 单调 = 已开始切换次数 + 1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SwitchExecutionPlan {
    pub from: Uuid,
    pub target: Uuid,
    pub policy: SwitchPolicy,
    pub epoch: u64,
}

/// Execution Group —— Program execution boundary（终裁 §7.3 冻结概念:
/// 哪些 Pipeline 属同一 Program execution / 当前 active source /
/// switch execution 协调 / group-level observation 装配）。
///
/// 与 Session（Create/Reserve/Instantiate/Start/Stop/Recover/Destroy
/// 生命周期）**协作不合并**: Session 保持 RUNNING 不因 ACTIVE=A→B 改变
/// （T9）。首版恰双输入（Dual-input MVP——N 输入泛化留后续裁决）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExecutionGroup {
    pub session_id: SessionId,
    /// 复用 Session 句柄表行（零新 identity 类型; 键集恰 {device_id, handle}）。
    pub inputs: Vec<SessionInput>,
    /// Desired 平面（本组唯一持有——Session/SessionInput 不存 switch state）。
    pub desired: SwitchDesired,
    /// 已开始切换计数（epoch 单调; `SwitchExecutionPlan.epoch` 来源）。
    pub switch_epoch: u64,
}

impl ExecutionGroup {
    /// fail-closed 构造: 恰 2 输入 / device_id 去重 / 初始 active ∈ 组。
    pub fn new(
        session_id: SessionId,
        inputs: Vec<SessionInput>,
        initial_active: Uuid,
    ) -> Result<Self, SwitchError> {
        if inputs.len() != 2 {
            return Err(SwitchError::GroupNotDual(inputs.len()));
        }
        if inputs[0].device_id == inputs[1].device_id {
            return Err(SwitchError::DuplicateDevice(inputs[0].device_id));
        }
        if !inputs.iter().any(|i| i.device_id == initial_active) {
            return Err(SwitchError::InitialActiveNotInGroup(initial_active));
        }
        Ok(Self {
            session_id,
            inputs,
            desired: SwitchDesired::ActiveInput(initial_active),
            switch_epoch: 0,
        })
    }

    /// 组内是否含该设备源。
    pub fn contains(&self, device_id: Uuid) -> bool {
        self.inputs.iter().any(|i| i.device_id == device_id)
    }

    /// 校验并产出切换计划（Intent→Plan 单向装配, 全 fail-closed）:
    /// target ∈ 组 / policy == FRAME_SWITCH（首版）/ Desired 为 Active /
    /// target ≠ 当前 active。**不改变任何状态**（begin 才推进）。
    pub fn plan_switch(&self, intent: &SwitchIntent) -> Result<SwitchExecutionPlan, SwitchError> {
        if !self.contains(intent.target) {
            return Err(SwitchError::TargetNotInGroup(intent.target));
        }
        if intent.policy != SwitchPolicy::FrameSwitch {
            return Err(SwitchError::UnsupportedPolicy(intent.policy));
        }
        let from = match self.desired {
            SwitchDesired::ActiveInput(active) => active,
            switching @ SwitchDesired::Switching { .. } => {
                return Err(SwitchError::NotActiveSource(switching))
            }
        };
        if intent.target == from {
            return Err(SwitchError::TargetAlreadyActive(from));
        }
        Ok(SwitchExecutionPlan {
            from,
            target: intent.target,
            policy: intent.policy,
            epoch: self.switch_epoch + 1,
        })
    }

    /// 开始切换: Desired Active(from)→Switching{from,to}, epoch +1。
    /// 必须持有效 plan（重放同一 epoch 拒绝）。
    pub fn begin_switch(&mut self, plan: &SwitchExecutionPlan) -> Result<(), SwitchError> {
        if plan.epoch != self.switch_epoch + 1 {
            return Err(SwitchError::StalePlanEpoch {
                got: plan.epoch,
                expected: self.switch_epoch + 1,
            });
        }
        match self.desired {
            SwitchDesired::ActiveInput(active) if active == plan.from => {
                self.desired = SwitchDesired::Switching {
                    from: plan.from,
                    to: plan.target,
                };
                self.switch_epoch = plan.epoch;
                Ok(())
            }
            other => Err(SwitchError::NotActiveSource(other)),
        }
    }

    /// Observed 确认推进: 仅当 Desired==Switching{to} 且 observed==to 时
    /// 落定 Active(to), 返回 true; 否则零推进返回 false（Observation
    /// 驱动——observed 回显非 to 即不落定, 诚实停留 Switching）。
    pub fn complete_switch(&mut self, observed_active: Uuid) -> bool {
        if let SwitchDesired::Switching { to, .. } = self.desired {
            if observed_active == to {
                self.desired = SwitchDesired::ActiveInput(to);
                return true;
            }
        }
        false
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;

    fn input(device_id: Uuid, handle: u64) -> SessionInput {
        SessionInput {
            device_id,
            handle: PipelineHandle(handle),
        }
    }

    fn dual_group() -> (Uuid, Uuid, ExecutionGroup) {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let g = ExecutionGroup::new(SessionId(Uuid::new_v4()), vec![input(a, 1), input(b, 2)], a)
            .expect("合法双输入组应构造成功");
        (a, b, g)
    }

    #[test]
    fn switch_rt_01_group_requires_exactly_two_inputs() {
        let single = Uuid::new_v4();
        assert!(matches!(
            ExecutionGroup::new(SessionId(Uuid::new_v4()), vec![input(single, 1)], single),
            Err(SwitchError::GroupNotDual(1))
        ));
        let triple = (0..3)
            .map(|i| input(Uuid::new_v4(), i as u64 + 1))
            .collect();
        assert!(matches!(
            ExecutionGroup::new(SessionId(Uuid::new_v4()), triple, Uuid::nil()),
            Err(SwitchError::GroupNotDual(3))
        ));
    }

    #[test]
    fn switch_rt_01_duplicate_device_rejected() {
        let d = Uuid::new_v4();
        let err = ExecutionGroup::new(SessionId(Uuid::new_v4()), vec![input(d, 1), input(d, 2)], d)
            .expect_err("重复设备应拒收");
        assert_eq!(err, SwitchError::DuplicateDevice(d));
    }

    #[test]
    fn switch_rt_01_intent_target_must_be_group_member() {
        let (a, _b, g) = dual_group();
        let outsider = Uuid::new_v4();
        let err = g
            .plan_switch(&SwitchIntent {
                target: outsider,
                policy: SwitchPolicy::FrameSwitch,
            })
            .expect_err("组外目标应拒收");
        assert_eq!(err, SwitchError::TargetNotInGroup(outsider));
        // 组内成员（且非 active）应通过校验——对照锚。
        g.plan_switch(&SwitchIntent {
            target: group_peer(&g, a),
            policy: SwitchPolicy::FrameSwitch,
        })
        .expect("组内合法目标应产出计划");
    }

    #[test]
    fn switch_rt_01_packet_master_fail_closed() {
        let (a, _b, g) = dual_group();
        let peer = group_peer(&g, a);
        for policy in [SwitchPolicy::PacketSwitch, SwitchPolicy::MasterSwitch] {
            let err = g
                .plan_switch(&SwitchIntent {
                    target: peer,
                    policy,
                })
                .expect_err("首版仅 FRAME_SWITCH, 其余 fail-closed 拒收");
            assert_eq!(err, SwitchError::UnsupportedPolicy(policy));
        }
    }

    #[test]
    fn switch_rt_01_switch_to_active_source_rejected() {
        let (a, _b, g) = dual_group();
        let err = g
            .plan_switch(&SwitchIntent {
                target: a,
                policy: SwitchPolicy::FrameSwitch,
            })
            .expect_err("切换到当前 active 源应拒收");
        assert_eq!(err, SwitchError::TargetAlreadyActive(a));
    }

    #[test]
    fn switch_rt_01_desired_progression_active_switching_active() {
        let (a, b, mut g) = dual_group();
        assert_eq!(g.desired, SwitchDesired::ActiveInput(a));
        assert_eq!(g.switch_epoch, 0);

        let plan = g
            .plan_switch(&SwitchIntent {
                target: b,
                policy: SwitchPolicy::FrameSwitch,
            })
            .expect("合法计划");
        assert_eq!(plan.from, a);
        assert_eq!(plan.target, b);
        assert_eq!(plan.epoch, 1);

        // Observed 未确认前 Desired 停留 Switching（不因命令回显落定）。
        g.begin_switch(&plan).expect("begin 应成功");
        assert_eq!(g.desired, SwitchDesired::Switching { from: a, to: b });
        assert_eq!(g.switch_epoch, 1);
        // 同一 plan 重放拒收（epoch 已消费——stale fail-closed）。
        assert_eq!(
            g.begin_switch(&plan),
            Err(SwitchError::StalePlanEpoch {
                got: 1,
                expected: 2
            })
        );
        assert!(!g.complete_switch(a), "observed=旧源不得落定");
        assert_eq!(g.desired, SwitchDesired::Switching { from: a, to: b });

        // 切换中再 plan 应拒（Desired 非 Active 平面）。
        assert!(matches!(
            g.plan_switch(&SwitchIntent {
                target: a,
                policy: SwitchPolicy::FrameSwitch
            }),
            Err(SwitchError::NotActiveSource(_))
        ));

        assert!(g.complete_switch(b), "observed=to 应落定");
        assert_eq!(g.desired, SwitchDesired::ActiveInput(b));
        assert_eq!(g.switch_epoch, 1);
    }

    #[test]
    fn switch_rt_01_policy_enum_unchanged_anchor() {
        // T11: SwitchPolicy 封闭词表 + IO 平面映射回归锁（执行逻辑零污染）。
        assert_eq!(
            crate::program::ACCEPTED_LIST,
            &["PACKET_SWITCH", "FRAME_SWITCH", "MASTER_SWITCH"]
        );
        assert_eq!(
            SwitchPolicy::PacketSwitch.io_plane(),
            crate::program::SwitchIoPlane::CompressedToCompressed
        );
        assert_eq!(
            SwitchPolicy::FrameSwitch.io_plane(),
            crate::program::SwitchIoPlane::RawToRaw
        );
        assert_eq!(
            SwitchPolicy::MasterSwitch.io_plane(),
            crate::program::SwitchIoPlane::NormalizedRawToRaw
        );
        assert!(SwitchPolicy::parse("FRAME_SWITCH").is_ok());
        assert!(SwitchPolicy::parse("QUANTUM_SWITCH").is_err());
    }

    /// 组内另一成员（对照锚 helper）。
    fn group_peer(g: &ExecutionGroup, not_this: Uuid) -> Uuid {
        g.inputs
            .iter()
            .find(|i| i.device_id != not_this)
            .map(|i| i.device_id)
            .expect("双输入组必有对端")
    }
}
