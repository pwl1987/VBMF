//! 共享媒体事件/健康类型与全局健康表 (C7: 从 `pipeline.rs` / `adapters/gstreamer/controller.rs` 抽出的中性模块)。
//!
//! 这些类型只依赖 `std` 与 `crate::pipeline` 的领域类型, **不**依赖 vendor `gstreamer` crate,
//! 因此在 default / simulation / mock 等无 gstreamer 构建下也必须编译. 消费方
//! (main.rs / contracts/backend.rs / adapters/mock.rs) 直接 `use crate::pipeline_events::*`,
//! `pipeline.rs` 仅引用自身用到的 `PipelineBusEvent`, 不再经 `crate::pipeline` 重导出共享项.

use crate::pipeline::{PipelineHandle, PipelineHealth};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;

/// 运行时健康共享状态 (GStreamer 回调/bus 监控/监控线程共享).
pub(crate) static HEALTH_ARCS: LazyLock<Mutex<HashMap<PipelineHandle, Arc<Mutex<PipelineHealth>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 读取管线健康快照 (监控 API 用). 在部分 feature 组合下无调用点 (main 的 health endpoint
/// 经 cfg 门控), 故允许 dead_code; 与迁移前 `controller.rs` 模块级 `#![allow(dead_code)]` 一致.
#[allow(dead_code)]
pub fn read_health(handle: &PipelineHandle) -> Option<PipelineHealth> {
    HEALTH_ARCS
        .lock()
        .unwrap()
        .get(handle)
        .map(|h| h.lock().unwrap().clone())
}

/// GStreamer Bus 事件严重度 (喂 Supervisor 决策时判优先级).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BusSeverity {
    /// 致命: pipeline error / 解码失败 → Supervisor 必响应.
    Error,
    /// 告警: Warning / ClockLost 等可恢复异常.
    Warning,
    /// 信息: StateChanged / Eos / AsyncDone 等正常生命周期事件.
    Info,
}

/// GStreamer Bus 事件类型 (P1-4: 覆盖 Error/EOS/StateChanged/Warning/ClockLost, 真实接线到 Supervisor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineBusEventKind {
    Error,
    Eos,
    StateChanged,
    Warning,
    /// ClockLost 等 AV 同步相关 (PIPELINE-AV 后续消费; 当前仅记录).
    ClockLost,
}

/// Bus 事件 → Supervisor 恢复策略 (P1-4 最低策略映射, 用户复核 §十二):
/// - `Error` / `Eos`     : 致命 → 触发 Supervisor `report_failure` (重启/升级).
/// - `ClockLost`         : 降级 (degraded), **不**自动重启 (完整 Clock Recovery 属 V0.3/P2); 仅计数 + 健康降级.
/// - `Warning`           : 告警, 记录 + 日志, 不重启.
/// - `StateChanged`      : 信息, 仅生命周期日志.
// 在部分 feature 组合下无调用点 (main 的 bus 监控经 cfg 门控), 允许 dead_code.
#[allow(dead_code)]
pub fn bus_event_recovery_policy(kind: PipelineBusEventKind) -> &'static str {
    match kind {
        PipelineBusEventKind::Error | PipelineBusEventKind::Eos => "restart",
        PipelineBusEventKind::ClockLost => "degraded",
        PipelineBusEventKind::Warning => "warn",
        PipelineBusEventKind::StateChanged => "info",
    }
}

/// GStreamer Bus 事件 (监控线程消费, 喂 Supervisor 决策).
///
/// P1-4 改造 (用户复核 §七): 之前只有 `Error(String)` 等薄枚举, 多 pipeline 后无法诊断
/// "哪一路出的错". 现结构化携带 `handle`(哪条管线) / `source`(哪个 element 发出) /
/// `timestamp`(观测墙钟 ms) / `detail`(错误串/状态转移) / `severity`. 事件经专门 GLib
/// MainContext 线程的 Bus watch 投递进 bounded mpsc channel, `poll_bus` 非阻塞 drain.
/// C2: MediaBackend SPI 实现（复用既有 `PipelineController` + 固有 `poll_bus`）。
/// 物理迁移到中性模块 `pipeline_events` (C7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineBusEvent {
    pub handle: PipelineHandle,
    pub kind: PipelineBusEventKind,
    pub source: String,
    pub timestamp: i64,
    pub detail: String,
    pub severity: BusSeverity,
}
