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

/// 事件严重级 (P1-3 两级语义): 观测类可丢弃, 故障/拒识类不可被日志挤出。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventSeverity {
    /// 观测/进度类 (健康迁移、生命周期里程碑): 日志满时可丢弃。
    #[default]
    Observation,
    /// 关键类 (PipelineFault/HardwareFault/AmbiguousIdentity): 不可被静默挤出。
    Critical,
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
    LeaseGranted { device_id: Uuid, lease_id: Uuid },
    /// 资源已分配 (Resource: Reserved → Allocated)。
    ResourceAllocated { resource_id: Uuid },
    /// 资源预留过期 (Reservation TTL 到期, 自动回收)。
    ResourceReservationExpired { resource_id: Uuid },
    /// 管线故障 (可重试, Backend 层; Supervisor 据此决定重启/退避)。
    PipelineFault {
        pipeline: Uuid,
        /// canonical 故障摘要 (vendor 细节已在映射时消化)。
        summary: String,
        retryable: bool,
    },
    /// 硬件故障 (需运维介入; 重启无法自愈)。
    HardwareFault { device_id: Uuid, summary: String },
    /// 健康状态变化 (AgentState 迁移)。
    HealthChanged { from: String, to: String },
    /// 身份歧义 (多重 HIGH 候选 → 拒识, 交由 Policy 决策; 绝不静默择一)。
    AmbiguousIdentity {
        device_id: Uuid,
        /// 候选描述集 (canonical 字符串, 如各候选的 handle/序号)。
        candidates: Vec<String>,
    },
    // ── P0-7A Session Runtime (additive; EVENT_CONTRACT: session 为一等维度) ──
    /// 会话已创建 (Preflight+Reservation 完成后登记)。
    SessionCreated { session_id: Uuid },
    /// 会话状态迁移 (粗态或微相位; canonical 字符串)。
    SessionStateChanged {
        session_id: Uuid,
        from: String,
        to: String,
    },
    /// 会话失败 (生命周期任一步失败且已回滚; Critical — 不可被日志挤出)。
    SessionFailed { session_id: Uuid, reason: String },
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
            RuntimeEvent::SessionCreated { .. } => "session_created",
            RuntimeEvent::SessionStateChanged { .. } => "session_state_changed",
            RuntimeEvent::SessionFailed { .. } => "session_failed",
        }
    }
    /// 是否为故障类事件 (需 Supervisor/Policy 关注)。
    pub fn is_fault(&self) -> bool {
        matches!(
            self,
            RuntimeEvent::PipelineFault { .. }
                | RuntimeEvent::HardwareFault { .. }
                | RuntimeEvent::AmbiguousIdentity { .. }
                | RuntimeEvent::SessionFailed { .. }
        )
    }
    /// 事件严重级 (P1-3): 故障/拒识类 Critical (不可被日志挤出), 其余 Observation。
    pub fn severity(&self) -> EventSeverity {
        if self.is_fault() {
            EventSeverity::Critical
        } else {
            EventSeverity::Observation
        }
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
/// 容量上限防止无消费者时无限增长。**P1-3 两级丢弃策略** (与 GStreamer ERROR 不静默丢同原则):
/// - Observation (观测类) 满时可被挤出; 全 Critical 时新观测直接丢弃;
/// - Critical (故障/拒识类) 永不被观测事件挤出; Critical 强推时才挤最旧。
///
/// 丢弃计数 (`dropped_observations`/`dropped_criticals`) 暴露给 Health/运维, 丢弃不静默。
#[derive(Debug)]
pub struct RuntimeEventLog {
    cap: usize,
    inner: Mutex<VecDeque<RuntimeEvent>>,
    dropped_observations: Mutex<u64>,
    dropped_criticals: Mutex<u64>,
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
            dropped_observations: Mutex::new(0),
            dropped_criticals: Mutex::new(0),
        }
    }
    /// 追加事件 (P1-3 两级丢弃策略, 见结构体文档)。
    pub fn push(&self, event: RuntimeEvent) {
        let mut g = self.inner.lock().unwrap();
        let critical = event.severity() == EventSeverity::Critical;
        if g.len() >= self.cap {
            if critical {
                // Critical 强推: 挤最旧; 若被挤的也是 Critical 计入 dropped_criticals (容量极端, 不静默).
                if g.front().map(|e| e.severity()) == Some(EventSeverity::Critical) {
                    *self.dropped_criticals.lock().unwrap() += 1;
                }
                g.pop_front();
            } else {
                // Observation: 只能腾出同级位置; 全 Critical 时丢弃新观测并计数.
                match g
                    .iter()
                    .position(|e| e.severity() == EventSeverity::Observation)
                {
                    Some(idx) => {
                        g.remove(idx);
                    }
                    None => {
                        *self.dropped_observations.lock().unwrap() += 1;
                        return;
                    }
                }
            }
        }
        g.push_back(event);
    }
    /// 因日志满被丢弃的观测类事件计数 (Health/运维可见, 丢弃不静默)。
    pub fn dropped_observations(&self) -> u64 {
        *self.dropped_observations.lock().unwrap()
    }
    /// 因 Critical 强推而被挤出的 Critical 事件计数 (容量极端情况, 正常容量下恒 0)。
    pub fn dropped_criticals(&self) -> u64 {
        *self.dropped_criticals.lock().unwrap()
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

/// 事件出口抽象 (**D8 解耦**) — 生产者 (SessionManager / Supervisor 决策 / ingest
/// 归一化) 只依赖本 trait, 不依赖 Supervisor, 也不感知事件表实现。
///
/// 契约 (0.7C-6 probe Q4 基线, 零偷改): `emit` 永不阻塞、永不失败——
/// 满时按 `RuntimeEventLog` 两级丢弃策略处理并计数 (丢弃不静默)。
pub trait RuntimeEventSink: Send + Sync {
    fn emit(&self, ev: RuntimeEvent);
}

impl RuntimeEventSink for RuntimeEventLog {
    fn emit(&self, ev: RuntimeEvent) {
        self.push(ev);
    }
}

/// 组合根事件分流 (P0-7D D3 定稿: 双日志分流)。
///
/// 解决单日志多消费者竞争: 外送投影端点 (transport `drain→project`) 与内消费
/// (watchdog tick `drain→health::reduce`) 共享同一日志时, 破坏性 `drain` 互相掏空。
/// FanoutSink 在 emit 时**同序双写**两条独立 `RuntimeEventLog`:
/// - `projection`: 外送侧 (transport 投影端点 + gate 证据路径照旧 drain);
/// - `internal`: 内消费侧 (watchdog tick drain → reducer 派生 AgentState)。
///
/// 契约继承 `RuntimeEventSink`: 永不阻塞、永不失败。顺序保证: emit 内顺序双 push,
/// 两日志事件全序一致; 各日志独立维持 0.7C-6 四语义 (FIFO/两级丢弃/重复容忍/failure 隔离)。
pub struct FanoutSink {
    projection: std::sync::Arc<RuntimeEventLog>,
    internal: std::sync::Arc<RuntimeEventLog>,
}

impl FanoutSink {
    pub fn new(
        projection: std::sync::Arc<RuntimeEventLog>,
        internal: std::sync::Arc<RuntimeEventLog>,
    ) -> Self {
        Self {
            projection,
            internal,
        }
    }
    /// 外送投影日志 (transport `TransportContext.events` 与 gate 证据路径接此实例)。
    pub fn projection(&self) -> std::sync::Arc<RuntimeEventLog> {
        self.projection.clone()
    }
    /// 内消费日志 (watchdog tick drain → `health::reduce`)。
    pub fn internal(&self) -> std::sync::Arc<RuntimeEventLog> {
        self.internal.clone()
    }
}

impl RuntimeEventSink for FanoutSink {
    fn emit(&self, ev: RuntimeEvent) {
        self.projection.push(ev.clone());
        self.internal.push(ev);
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
        assert!(m
            .map_upstream(EventSource::Upstream, "hardware: device lost")
            .is_some());
        assert!(m
            .map_upstream(
                EventSource::Upstream,
                "ambiguous identity: 2 high candidates"
            )
            .is_some());
        assert!(m
            .map_upstream(EventSource::Upstream, "pipeline error: gst bus")
            .is_some());
        // 无故障语义的观测 → 不伪造事件。
        assert!(m
            .map_upstream(EventSource::Upstream, "all nominal")
            .is_none());
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
    fn log_critical_never_evicted_by_observations_and_drops_counted() {
        // P1-3: Critical (fault) 不被 Observation 挤出; 全 Critical 满时新观测被丢弃并计数.
        let log = RuntimeEventLog::with_capacity(2);
        log.push(RuntimeEvent::HealthChanged {
            from: "a".into(),
            to: "b".into(),
        });
        let fault = RuntimeEvent::PipelineFault {
            pipeline: Uuid::nil(),
            summary: "bus error".into(),
            retryable: true,
        };
        log.push(fault.clone());
        assert_eq!(log.len(), 2);
        // 新观测: 只能挤掉 Observation (HealthChanged), 不得挤掉 PipelineFault.
        log.push(RuntimeEvent::HealthChanged {
            from: "c".into(),
            to: "d".into(),
        });
        let drained = log.drain();
        assert_eq!(drained.len(), 2);
        assert!(drained.contains(&fault), "Critical 事件不得被观测事件挤出");
        assert!(log.dropped_observations() == 0);
        // 全 Critical 满: 新 Observation 丢弃并计数.
        log.push(RuntimeEvent::PipelineFault {
            pipeline: Uuid::nil(),
            summary: "f1".into(),
            retryable: true,
        });
        log.push(RuntimeEvent::PipelineFault {
            pipeline: Uuid::nil(),
            summary: "f2".into(),
            retryable: true,
        });
        assert_eq!(log.len(), 2);
        log.push(RuntimeEvent::HealthChanged {
            from: "e".into(),
            to: "f".into(),
        });
        assert_eq!(log.len(), 2, "全 Critical 满时观测不入队");
        assert_eq!(log.dropped_observations(), 1);
        // Critical 强推仍可进行 (挤最旧), 且被挤 Critical 计数.
        log.push(RuntimeEvent::PipelineFault {
            pipeline: Uuid::nil(),
            summary: "f3".into(),
            retryable: true,
        });
        assert_eq!(log.len(), 2);
        assert_eq!(log.dropped_criticals(), 1);
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
        // P0-7A additive kinds 同样 serde 安全。
        let sf = RuntimeEvent::SessionFailed {
            session_id: Uuid::nil(),
            reason: "start failed".into(),
        };
        let j2 = serde_json::to_string(&sf).expect("serialize");
        let b2: RuntimeEvent = serde_json::from_str(&j2).expect("deserialize");
        assert_eq!(sf, b2);
        assert_eq!(sf.kind(), "session_failed");
        assert!(sf.is_fault(), "SessionFailed 为 Critical");
    }

    // ── P0-7D FanoutSink (D3 双日志分流) 四语义扩展 ──

    fn fanout_pair(
        cap: usize,
    ) -> (
        FanoutSink,
        std::sync::Arc<RuntimeEventLog>,
        std::sync::Arc<RuntimeEventLog>,
    ) {
        let projection = std::sync::Arc::new(RuntimeEventLog::with_capacity(cap));
        let internal = std::sync::Arc::new(RuntimeEventLog::with_capacity(cap));
        let sink = FanoutSink::new(projection.clone(), internal.clone());
        (sink, projection, internal)
    }

    #[test]
    fn fanout_dual_log_same_order_and_content() {
        // 顺序一致: 同一 emit 序列在两日志逐条相等 (全序保持)。
        let (sink, projection, internal) = fanout_pair(64);
        let seq = [
            RuntimeEvent::SessionCreated {
                session_id: Uuid::nil(),
            },
            RuntimeEvent::SignalVerified {
                device_id: Uuid::nil(),
                port_id: None,
            },
            RuntimeEvent::PipelineFault {
                pipeline: Uuid::nil(),
                summary: "upstream".into(),
                retryable: true,
            },
        ];
        for ev in &seq {
            sink.emit(ev.clone());
        }
        assert_eq!(projection.drain(), internal.drain());
        // 重复容忍: 双日志各自消费语义独立, drain 后再 drain 为空。
        assert!(projection.drain().is_empty());
        assert!(internal.drain().is_empty());
    }

    #[test]
    fn fanout_drop_counters_independent_per_log() {
        // 丢弃独立: 消费节奏不同不互相影响计数 (两级丢弃语义按日志独立维持)。
        // Critical 强推语义 (P1-3): 满时挤最旧 Critical 并计 dropped_criticals。
        // cap=2; 批1 (3 条 Critical) 后 projection drain 腾空, 批2 (3 条) 再入。
        // 精确账: internal 6 条未消费 → 丢 4 留 2; projection 批间消费 → 共丢 2 留 2。
        let (sink, projection, internal) = fanout_pair(2);
        let crit = || RuntimeEvent::SessionFailed {
            session_id: Uuid::nil(),
            reason: "force critical".into(),
        };
        for _ in 0..3 {
            sink.emit(crit());
        }
        let _ = projection.drain();
        for _ in 0..3 {
            sink.emit(crit());
        }
        assert_eq!(internal.dropped_criticals(), 4);
        assert_eq!(internal.len(), 2);
        assert_eq!(projection.dropped_criticals(), 2);
        assert_eq!(projection.len(), 2);
    }
}
