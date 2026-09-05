//! A2-8-03-01-B/C（R44 §5/§8）: internal 事件平面**单一 destructive-drain
//! 边界** + Custody 生产接线。
//!
//! 裁决方向（R44）: 「一个事实消费点完成事件取得, 然后做非破坏性 fan-out /
//! fold」——**禁** Custody 自行 drain internal_log 与既有 watchdog 消费者抢
//! 事件（否则 G-1 从"未接线"变成"多消费者抢事件"，反而制造新的数据一致性
//! 故障）。
//!
//! 边界语义:
//! - 本类型持有 internal 平面 [`RuntimeEventLog`] 的共享句柄, 是**唯一**调用
//!   `drain()` 的生产消费点（组合根经 bootstrap 构造共享单实例——BS-01;
//!   生产 watchdog 线程不再直接持有 internal log——类型级排他）;
//! - 各周期驱动器（ingest / execution group watchdog tick——watchdog 退化为
//!   周期驱动器, 不再是事件事实的唯一所有者）经 [`InternalEventIntake::consume`]
//!   取得本 tick 批次并做**本地** fold（health reduce / fault_trigger——调用方
//!   间分区语义与既有行为一致, 零变化）;
//! - 边界内对同一 drained 批次做非破坏性 fan-out: custody 生产桥
//!   （`observations_from_events`, A2-7 冻结提取规则: echo 排除 / nil 拒收 /
//!   只提取 PipelineFault）**全量恰一次累积**——任意驱动器先 drain 都不丢
//!   custody 事实（G-1 拓扑硬约束的闭合点）。
//!
//! 红线（A2-7 七不 + R44 §9/§10）:
//! - 累积**只增不改**; 不 advance 三 Master（advance 零触发维持）; 不建第二
//!   SoT（归因消费时装配——`attribute_failures`/`custody_snapshot` 语义零放宽）;
//! - 身份缺失（nil）不归因 fail-closed——identity correlation 防线不动;
//! - [`RuntimeEventLog`] 契约（P1-3 FIFO / 两级丢弃 / 丢弃计数 / fail-closed）
//!   零触碰——本模块只消费不改日志。
//!
//! 消费面状态: 03-01-D/E/F（FailureDomain 生产消费 / Supervisor 接线）待
//! 真实测试结果后另行裁决——本轮仅暴露 [`InternalEventIntake::observations`]
//! 只读访问, 不新增生产消费者。

use std::sync::Arc;

use crate::custody::{observations_from_events, CustodyObservations};
use crate::events::{RuntimeEvent, RuntimeEventLog};

/// internal 平面唯一事实消费边界（见模块文档）。线程安全经组合根
/// `Arc<Mutex<InternalEventIntake>>` 装配（各驱动器 tick 短临界区）。
#[derive(Debug)]
pub struct InternalEventIntake {
    log: Arc<RuntimeEventLog>,
    custody: CustodyObservations,
}

impl InternalEventIntake {
    /// 以 internal 平面日志句柄构造（bootstrap 唯一构造源; 共享单实例）。
    pub fn new(log: Arc<RuntimeEventLog>) -> Self {
        Self {
            log,
            custody: observations_from_events(&[]),
        }
    }

    /// **唯一 drain 入口**: 排空 internal 平面 → 边界内 custody 全量恰一次
    /// 累积（A2-7 桥提取规则）→ 返回 drained 批次供调用方本地 fold
    /// （health / fault_trigger——非破坏性: 同一批事件对 custody 与调用方
    /// 均可见, 互不抢事件）。
    pub fn consume(&mut self) -> Vec<RuntimeEvent> {
        let drained = self.log.drain();
        let batch = observations_from_events(&drained);
        self.custody.failures.extend(batch.failures);
        drained
    }

    /// custody 累积观测的只读访问（累积只增不改; 03-01-D/E/F 生产消费面
    /// 后续裁决——本轮仅暴露, 不新增消费者）。
    pub fn observations(&self) -> &CustodyObservations {
        &self.custody
    }
}

impl Default for InternalEventIntake {
    fn default() -> Self {
        Self::new(Arc::new(RuntimeEventLog::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custody::attribute_failures;
    use crate::events::{EventSource, FanoutSink, RuntimeEventSink};
    use crate::program::{AudioMaster, MasterJoinResult, VideoMaster};
    use crate::supervisor::{fault_trigger_from_events, RestartPolicy, Supervisor};
    use uuid::Uuid;

    /// 边界即唯一 drain 实现: consume 后 log 排空; 返回批次供调用方本地
    /// fold——非破坏性（custody 与调用方看到同一批, 互不抢事件）; 非故障
    /// 事件零累积。
    #[test]
    fn intake_01_sole_drain_owner_returns_batch_to_caller() {
        let log = Arc::new(RuntimeEventLog::new());
        let mut intake = InternalEventIntake::new(log.clone());
        log.push(RuntimeEvent::SignalVerified {
            device_id: Uuid::new_v4(),
            port_id: None,
        });
        let batch = intake.consume();
        assert_eq!(batch.len(), 1);
        assert!(
            log.is_empty(),
            "consume 即唯一 drain 点（生产面不再有第二 drain 调用方）"
        );
        assert!(intake.observations().failures.is_empty());
    }

    /// 多驱动器共用同一边界（组合根共享单实例的仿真）: 事件按到达顺序被
    /// 先 tick 的驱动者取得, 但 custody 在边界内对每个 drained 批次恰一次
    /// 累积——全量零丢失、零重复（echo / nil 拒收, 非故障 kind 忽略）。
    #[test]
    fn intake_02_custody_accumulates_exactly_once_across_drivers() {
        let log = Arc::new(RuntimeEventLog::new());
        let mut intake = InternalEventIntake::new(log.clone());
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        log.push(RuntimeEvent::PipelineFault {
            pipeline: a,
            summary: "driver1 窗口故障".into(),
            retryable: true,
        });
        log.push(RuntimeEvent::HealthChanged {
            from: "x".into(),
            to: "y".into(),
        });
        assert_eq!(intake.consume().len(), 2);
        assert_eq!(intake.observations().failures.len(), 1);

        log.push(RuntimeEvent::PipelineFault {
            pipeline: b,
            summary: "driver2 窗口故障".into(),
            retryable: true,
        });
        log.push(RuntimeEvent::PipelineFault {
            pipeline: a,
            summary: crate::supervisor::RESTART_ECHO_SUMMARY.into(),
            retryable: true,
        });
        log.push(RuntimeEvent::PipelineFault {
            pipeline: Uuid::nil(),
            summary: "pipeline error: unattributed".into(),
            retryable: true,
        });
        assert_eq!(intake.consume().len(), 3);
        assert!(
            intake.consume().is_empty(),
            "drain 破坏性: 已消费批次不重复"
        );

        let failures = &intake.observations().failures;
        assert_eq!(
            failures.len(),
            2,
            "恰两次累积（echo 回声排除 + nil 未归属拒收 + 非故障 kind 忽略）"
        );
        assert!(failures.iter().any(|f| f.pipeline_id == a));
        assert!(failures.iter().any(|f| f.pipeline_id == b));
    }

    /// 03-01-A+B+C 生产链全闭环: `Supervisor::ingest`（携带 canonical 设备
    /// 身份）→ FanoutSink 双写 → internal 平面唯一 drain 边界 → custody 全量
    /// 恰一次累积 → 归因（本设备双路 failed / 他设备零污染）→ custody_snapshot
    /// FAILED; fault_trigger 身份精度（归属设备触发 / 他设备零误触 / echo 不
    /// 自激）; 投影面不受内消费破坏（D3 双日志）。
    #[test]
    fn intake_03_production_chain_identity_reaches_custody() {
        let projection = Arc::new(RuntimeEventLog::new());
        let internal = Arc::new(RuntimeEventLog::new());
        let sink: Arc<dyn RuntimeEventSink> =
            Arc::new(FanoutSink::new(projection.clone(), internal.clone()));
        let mut sup = Supervisor::new(RestartPolicy::default(), sink);
        let dev = Uuid::new_v4();
        let other = Uuid::new_v4();

        // Supervisor 决策回声（report_failure 产 RESTART_ECHO_SUMMARY）+
        // 携带身份的生产上游故障（03-01-A: 身份不再在 mapper 边界丢失）。
        sup.register(dev);
        sup.report_failure(&dev).unwrap();
        sup.ingest(
            EventSource::Upstream,
            dev,
            "pipeline error: upstream decode",
        );

        let mut intake = InternalEventIntake::new(internal.clone());
        let batch = intake.consume();
        assert_eq!(batch.len(), 2, "echo + 真实故障 双双到达内平面");
        assert!(
            fault_trigger_from_events(&batch, dev),
            "归属设备触发（echo 被谓词排除）"
        );
        assert!(
            !fault_trigger_from_events(&batch, other),
            "身份化故障不误触他设备（03-01-A 精度）"
        );

        // custody: 恰一条（echo 排除）, 携带 canonical 设备身份。
        let obs = intake.observations();
        assert_eq!(obs.failures.len(), 1);
        assert_eq!(obs.failures[0].pipeline_id, dev);

        // 归因闭环: 本设备双路 failed; 他设备零污染（identity correlation 不放宽）。
        let attributed = attribute_failures(dev, &obs.failures);
        assert!(attributed.video_failed && attributed.audio_failed);
        let other_view = attribute_failures(other, &obs.failures);
        assert!(!other_view.video_failed && !other_view.audio_failed);

        // custody_snapshot 全链: 三 Master 初始态 + 双路 failed → FAILED;
        // 他设备 None。
        let (video, audio) = (VideoMaster::new(), AudioMaster::new());
        let (_, r_self) = crate::custody::custody_snapshot(&video, &audio, dev, obs);
        assert_eq!(r_self, Some(MasterJoinResult::Failed));
        let (_, r_other) = crate::custody::custody_snapshot(&video, &audio, other, obs);
        assert_eq!(r_other, None);

        // D3 双日志: 投影面不受内消费破坏（同批事件完整保留在外送侧）。
        assert_eq!(projection.drain().len(), 2);

        // 恰一次: 已消费批次不重复入 custody。
        assert!(intake.consume().is_empty());
        assert_eq!(intake.observations().failures.len(), 1);
    }

    /// 披露锁: 调用方本地 fold 仍按消费分区（既有行为, R43 §1.2 事实非
    /// 缺陷; 全量统一 fold 属 03-01-D/E/F 后续裁决面）——驱动者只见自己
    /// tick 窗口内批次, 但 custody 全量恰一次（intake_02 已锁）。
    #[test]
    fn intake_04_driver_local_fold_partition_semantics_unchanged() {
        let log = Arc::new(RuntimeEventLog::new());
        let mut intake = InternalEventIntake::new(log.clone());
        log.push(RuntimeEvent::PipelineFault {
            pipeline: Uuid::new_v4(),
            summary: "w1".into(),
            retryable: true,
        });
        let seen_by_driver1 = intake.consume();
        log.push(RuntimeEvent::PipelineFault {
            pipeline: Uuid::new_v4(),
            summary: "w2".into(),
            retryable: true,
        });
        let seen_by_driver2 = intake.consume();
        assert_eq!(seen_by_driver1.len(), 1);
        assert_eq!(seen_by_driver2.len(), 1);
        assert_ne!(seen_by_driver1, seen_by_driver2);
        assert_eq!(
            intake.observations().failures.len(),
            2,
            "custody 不分区——全量恰一次"
        );
    }
}
