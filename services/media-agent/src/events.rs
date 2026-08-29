//! Phase 0.6 C2 (0.6D): RuntimeEvent — Canonical Error-Event 契约.
//!
//! 取代散落的 vendor 错误 (GStreamer `Message` / BMD HRESULT): 所有跨层事件收敛为
//! vendor-neutral 的 `RuntimeEvent`。`supervisor.rs` 是唯一事件出口 (见 supervisor 归一化),
//! 下游 (Health / RPC / 日志) 只消费 `RuntimeEvent`, 不再直接触碰 vendor 类型。
//!
//! 设计要点 (对齐 design.md / V0.2 四事件分层):
//! - 成员覆盖采集全生命周期 (身份 → 物化 → 信号 → 回环 → 租约/资源 → 故障 → 健康)。
//! - 字段一律 canonical (UUID / 字符串 / 枚举), 绝不内嵌 vendor 类型 (HRESULT / GstMessage)。
//! - 故障事件区分 `PipelineFault` (可重试, Backend 层) 与 `HardwareFault` (需运维, 硬件层)。
//! - `AmbiguousIdentity` 携带候选集, 支撑 0.6D 拒识 + Policy 决策 (多重 HIGH → 拒)。
//!
//! `#![allow(dead_code)]`: 本模块是 0.6D canonical 事件契约 (SPI) — 部分成员/辅助
//! (`kind` / `is_fault` / `is_empty` 等) 由下游 Control Plane / RPC 与后续 change 消费,
//! 当前 binary 尚未全部接线; 与 supervisor.rs 同款处理, 待接线完成后可收窄。
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use uuid::Uuid;

/// 事件源 (谁上抛该事件) — canonical 枚举, 非具体适配器类型名。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    /// 尚未归一化的上游 (Provider/Backend 原始事件)。
    #[default]
    Upstream,
    /// Supervisor 归一化后。
    Supervisor,
    /// 运维 / 控制面注入 (例如手动重置)。
    Operator,
}

/// 采集/运行时 canonical 事件 (全生命周期)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeEvent {
    /// 设备身份解析完成 (ResolvedDeviceBinding 收敛为 canonical 身份)。
    IdentityResolved {
        device_id: Uuid,
        /// 解析达到的置信描述 (canonical 字符串, 如 "high" / "manifest-verified")。
        confidence: String,
    },
    /// 采集源已按 PipelinePlan 物化 (prepare 成功)。
    SourceMaterialized {
        device_id: Uuid,
        /// pipeline 句柄 ID (canonical, 非 vendor 指针)。
        pipeline: Uuid,
    },
    /// 信号已验证 (输入 Locked / 输出模式已设)。
    SignalVerified {
        device_id: Uuid,
        port_id: Option<Uuid>,
    },
    /// 回环 (loopback) 已验证 (输出→SDI→输入 收到预期信号)。
    LoopbackVerified {
        device_id: Uuid,
        port_id: Option<Uuid>,
    },
    /// 租约已授予 (lease 分配给某 session)。
    LeaseGranted {
        device_id: Uuid,
        lease_id: Uuid,
    },
    /// 资源已分配 (Resource: Reserved → Allocated)。
    ResourceAllocated {
        resource_id: Uuid,
    },
    /// 资源预留过期 (Reservation TTL 到期, 自动回收)。
    ResourceReservationExpired {
        resource_id: Uuid,
    },
    /// 管线故障 (可重试, Backend 层; Supervisor 据此决定重启/退避)。
    PipelineFault {
        pipeline: Uuid,
        /// canonical 故障摘要 (vendor 细节已在映射时消化)。
        summary: String,
        retryable: bool,
    },
    /// 硬件故障 (需运维介入; 重启无法自愈)。
    HardwareFault {
        device_id: Uuid,
        summary: String,
    },
    /// 健康状态变化 (AgentState 迁移)。
    HealthChanged {
        from: String,
        to: String,
    },
    /// 身份歧义 (多重 HIGH 候选 → 拒识, 交由 Policy 决策; 绝不静默择一)。
    AmbiguousIdentity {
        device_id: Uuid,
        /// 候选描述集 (canonical 字符串, 如各候选的 handle/序号)。
        candidates: Vec<String>,
    },
}

impl RuntimeEvent {
    /// 事件 kind 的 canonical 字符串 (与 serde tag 一致), 供日志/RPC 过滤。
    pub fn kind(&self) -> &'static str {
        match self {
            RuntimeEvent::IdentityResolved { .. } => "identity_resolved",
            RuntimeEvent::SourceMaterialized { .. } => "source_materialized",
            RuntimeEvent::SignalVerified { .. } => "signal_verified",
            RuntimeEvent::LoopbackVerified { .. } => "loopback_verified",
            RuntimeEvent::LeaseGranted { .. } => "lease_granted",
            RuntimeEvent::ResourceAllocated { .. } => "resource_allocated",
            RuntimeEvent::ResourceReservationExpired { .. } => "resource_reservation_expired",
            RuntimeEvent::PipelineFault { .. } => "pipeline_fault",
            RuntimeEvent::HardwareFault { .. } => "hardware_fault",
            RuntimeEvent::HealthChanged { .. } => "health_changed",
            RuntimeEvent::AmbiguousIdentity { .. } => "ambiguous_identity",
        }
    }
    /// 是否为故障类事件 (需 Supervisor/Policy 关注)。
    pub fn is_fault(&self) -> bool {
        matches!(
            self,
            RuntimeEvent::PipelineFault { .. }
                | RuntimeEvent::HardwareFault { .. }
                | RuntimeEvent::AmbiguousIdentity { .. }
        )
    }
}

/// vendor 错误/消息 → `RuntimeEvent` 的归一化映射。
///
/// 契约: 实现方在 Adapter/Backend 内消化 vendor 细节 (HRESULT 码 / GstMessage 类型),
/// 只向 canonical 层暴露 `RuntimeEvent`。本 trait 是 0.6D「事件源分散」收敛的落点。
pub trait RuntimeEventMapper: Send + Sync {
    /// 将一条上游 (vendor) 观测归一化为 canonical 事件; 无法归类时返回 `None` (丢弃噪声)。
    fn map_upstream(&self, source: EventSource, observation: &str) -> Option<RuntimeEvent>;
}

/// 默认映射器 — 基于 canonical 观测字符串的保守归类 (Adapter 未提供专属映射器时的兜底)。
///
/// 仅识别明确的故障/健康语义关键字, 避免误判; 未知观测返回 `None` (不伪造事件)。
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultRuntimeEventMapper;

impl RuntimeEventMapper for DefaultRuntimeEventMapper {
    fn map_upstream(&self, _source: EventSource, observation: &str) -> Option<RuntimeEvent> {
        let obs = observation.to_lowercase();
        if obs.contains("ambiguous") {
            return Some(RuntimeEvent::AmbiguousIdentity {
                device_id: Uuid::nil(),
                candidates: vec![observation.to_string()],
            });
        }
        if obs.contains("hardware") || obs.contains("device lost") || obs.contains("hotplug") {
            return Some(RuntimeEvent::HardwareFault {
                device_id: Uuid::nil(),
                summary: observation.to_string(),
            });
        }
        // 其余 (pipeline 级) 归为可重试管线故障; 由 Supervisor 决定是否重试。
        if obs.contains("fault") || obs.contains("error") || obs.contains("failed") {
            return Some(RuntimeEvent::PipelineFault {
                pipeline: Uuid::nil(),
                summary: observation.to_string(),
                retryable: true,
            });
        }
        None
    }
}

/// 有界 canonical 事件日志 — Supervisor 的唯一事件出口缓冲, 下游 (Health/RPC/日志) 轮询 `drain`。
///
/// 线程安全 (Mutex), 供跨运行时线程 (Supervisor / watchdog / RPC) 消费。
/// 容量上限防止无消费者时无限增长 (满时丢弃最旧, 保留最新 — 观测数据可丢弃, 不阻塞)。
#[derive(Debug)]
pub struct RuntimeEventLog {
    cap: usize,
    inner: Mutex<VecDeque<RuntimeEvent>>,
}

impl RuntimeEventLog {
    /// 以默认容量 (1024) 构造。
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }
    /// 以指定容量构造 (至少 1)。
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            inner: Mutex::new(VecDeque::with_capacity(cap.max(1))),
        }
    }
    /// 追加事件; 满时丢弃最旧。
    pub fn push(&self, event: RuntimeEvent) {
        let mut g = self.inner.lock().unwrap();
        if g.len() >= self.cap {
            g.pop_front();
        }
        g.push_back(event);
    }
    /// 排空 (FIFO 顺序) 当前全部事件; 供下游一次性消费。
    pub fn drain(&self) -> Vec<RuntimeEvent> {
        self.inner.lock().unwrap().drain(..).collect()
    }
    /// 当前缓冲深度 (监控用)。
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for RuntimeEventLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_kind_matches_serde_tag() {
        let e = RuntimeEvent::PipelineFault {
            pipeline: Uuid::nil(),
            summary: "bus error".into(),
            retryable: true,
        };
        assert_eq!(e.kind(), "pipeline_fault");
        assert!(e.is_fault());
        let ok = RuntimeEvent::SignalVerified {
            device_id: Uuid::nil(),
            port_id: None,
        };
        assert!(!ok.is_fault());
    }

    #[test]
    fn default_mapper_classifies_faults_and_drops_noise() {
        let m = DefaultRuntimeEventMapper;
        assert!(m.map_upstream(EventSource::Upstream, "hardware: device lost").is_some());
        assert!(m
            .map_upstream(EventSource::Upstream, "ambiguous identity: 2 high candidates")
            .is_some());
        assert!(m.map_upstream(EventSource::Upstream, "pipeline error: gst bus").is_some());
        // 无故障语义的观测 → 不伪造事件。
        assert!(m.map_upstream(EventSource::Upstream, "all nominal").is_none());
    }

    #[test]
    fn log_bounded_drops_oldest_and_drains_fifo() {
        let log = RuntimeEventLog::with_capacity(2);
        for i in 0..3 {
            log.push(RuntimeEvent::HealthChanged {
                from: "a".into(),
                to: format!("s{i}"),
            });
        }
        assert_eq!(log.len(), 2); // 最旧被丢弃
        let drained = log.drain();
        assert_eq!(drained.len(), 2);
        assert!(log.is_empty());
        // FIFO: 保留的是较新的 s1, s2。
        let to: Vec<String> = drained
            .iter()
            .map(|e| match e {
                RuntimeEvent::HealthChanged { to, .. } => to.clone(),
                _ => String::new(),
            })
            .collect();
        assert_eq!(to, vec!["s1".to_string(), "s2".to_string()]);
    }

    #[test]
    fn event_roundtrips_through_serde() {
        let e = RuntimeEvent::AmbiguousIdentity {
            device_id: Uuid::nil(),
            candidates: vec!["h1".into(), "h2".into()],
        };
        let json = serde_json::to_string(&e).expect("serialize");
        let back: RuntimeEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e, back);
    }
}
