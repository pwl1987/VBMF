//! A2-8-02-I 第三十四轮终裁: Diagnostic Runtime Fault Injection 契约面。
//!
//! **仅诊断消费**（Gate/测试 harness）——禁入 `MediaBackend` 冻结 SPI
//! （instantiate/start/stop/recover/observe 五方法面不动）; 禁 Session/
//! Supervisor 侧注入（Session=生命周期 owner 不知"怎么把 GStreamer 搞坏";
//! Supervisor=observe→decide 不做故障制造器）。
//!
//! 语义边界（终裁照录）: 注入**"运行故障"**（真实执行面停流·frames/PTS
//! 冻结·liveness 窗口自然过期）非**"生命周期终止"**——`PipelineHandle`
//! 与 `HEALTH_ARCS` 登记**保持**（`MediaBackend::stop`=终态注销 P0-2
//! 契约不动; stop→recover 为非法组合, 02-I 首跑已证结构性必败）。
//! 随后 `MediaBackend::recover(handle)` 即为生产行为: 同 handle 按原
//! materialized plan 重建 concrete pipeline。
//!
//! 红线: 第一版**禁模拟 Bus Error 合成事件**（Observation Fact ≠
//! Synthetic Event——Health 体系 frames/PTS/last_observed/liveness/Bus
//! 分层不可被合成事实污染）, 只能作用于实际执行面; Mock 不假装拥有真实
//! controller registry（bundle mock 分支 diagnostic=None）。

use crate::pipeline::PipelineHandle;

/// 诊断故障注入 view——同一 concrete `GStreamerPipelineController` 的
/// 第四 trait view（MediaBackend / MediaTapPort / BridgeObservationPort
/// 之后的诊断专属面; 同源单构造原则不变）。
pub trait DiagnosticFaultInjection: Send + Sync {
    /// 注入运行故障: 该 handle 的真实媒体流停止推进（无新 buffer 产出,
    /// 观测面 frames/PTS 冻结、liveness 窗口过期）, 而 instances/健康
    /// 登记**保持**。解除故障的唯一生产路径=
    /// `MediaBackend::recover(handle)`（同 handle 原 plan 重建）。
    fn inject_runtime_stall(&self, handle: &PipelineHandle) -> Result<(), String>;
}
