//! Phase 0.7A: MediaSession + SessionManager — Runtime Ownership 层。
//!
//! 契约锚点 (全部 FROZEN, 实现级对齐):
//! - RUNTIME_SESSION_MODEL §2/§3: Session = canonical 运行时组合单元, **逻辑持有、物理引用**
//!   (绝不包含 concrete Pipeline/GStreamer 对象); 两级状态 (粗态 §3 + 微相位 Addendum §4.3);
//!   RELEASED→RUNNING 必须拒绝 (#114)。
//! - RUNTIME_SESSION_MODEL §4.1: **Runtime Session Manager 是 Session 唯一创建者/销毁者**;
//!   Provider/Backend/Supervisor 均不创建 Session; "Session owns lifecycle / Backend owns
//!   object / Handle links the two"。
//! - RUNTIME_LIFECYCLE_SEQUENCE §1: Intent → Preflight → Reserve → Create Session →
//!   Lease → Binding verify → Backend.instantiate → Allocate → Backend.start → Running;
//!   §2 铁律: 失败分支绝不遗留孤儿 Reservation/Lease/Allocation; **creator = destroyer**。
//! - MEDIA_BACKEND_CONTRACT §1.1 (P0-8): Backend 只消费已授权 Resource, 不得自行寻找设备。
//!
//! 并发: 多会话共享 `SharedResourceRegistry` (原子 acquire) — 后到会话对同资源 `NotAcquirable`
//! (单 agent 多会话仲裁; 无全局 Scheduler, 用户 §十九)。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use uuid::Uuid;

use crate::contracts::backend::MediaBackend;
use crate::device::DeviceInfo;
use crate::events::{RuntimeEvent, RuntimeEventSink};
use crate::graph_intent::GraphRuntimeIntent;
use crate::lease::{DeviceLease, LeaseManager};
use crate::pipeline::{MaterializeMode, PipelineHandle};
use crate::port::PortRegistry;
use crate::preflight::{PreflightInputs, PreflightReport};
use crate::resolver::ResolvedDeviceBinding;
use crate::resource::{AcquisitionRequest, SharedResourceRegistry};
use crate::supervisor::Supervisor;

/// 会话 ID (canonical; 独立于硬件身份, 模型 §2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "session-{}", self.0.simple())
    }
}

/// 粗态 (RUNTIME_SESSION_MODEL §3; 对外投影; 白名单迁移, RELEASED→Running 恒拒绝 #114)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Reserved,
    Running,
    Paused,
    Releasing,
    Released,
}

impl SessionState {
    pub fn can_transition_to(self, to: Self) -> bool {
        use SessionState::*;
        matches!(
            (self, to),
            (Reserved, Running)
                | (Reserved, Releasing)
                | (Reserved, Released)
                | (Running, Paused)
                | (Running, Releasing)
                | (Paused, Running)
                | (Paused, Releasing)
                | (Releasing, Released)
        )
    }
}

/// 微相位 (Addendum §4.3; 对内生命周期引擎步进)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    Requested,
    Provisioning,
    Binding,
    Leased,
    Starting,
    Running,
    Stopping,
    Released,
    ProvisioningFailed,
    BindingFailed,
    StartFailed,
    Degraded,
    Recovery,
    Terminated,
}

impl SessionPhase {
    /// 是否处于失败/终态 (保留供诊断, 等 close)。
    pub fn is_terminal_failure(self) -> bool {
        matches!(
            self,
            SessionPhase::ProvisioningFailed
                | SessionPhase::BindingFailed
                | SessionPhase::StartFailed
                | SessionPhase::Terminated
        )
    }
    /// 相位迁移白名单 (P0-2: 引擎所有 set_phase 必须过此守卫; 非法迁移 = 编程错误, 拒绝并 Critical 上报)。
    pub fn can_transition_to(self, to: Self) -> bool {
        use SessionPhase::*;
        matches!(
            (self, to),
            (Requested, Provisioning)
                | (Requested, ProvisioningFailed)
                | (Provisioning, Binding)
                | (Provisioning, Terminated)
                | (Provisioning, ProvisioningFailed)
                | (Binding, Leased)
                | (Binding, Starting)
                | (Binding, Terminated)
                | (Binding, BindingFailed)
                | (Leased, Starting)
                | (Leased, Stopping)
                | (Leased, Terminated)
                | (Starting, Running)
                | (Starting, StartFailed)
                | (Running, Stopping)
                | (Running, Degraded)
                | (Running, Recovery)
                | (Stopping, Released)
                | (Stopping, Terminated)
                | (Degraded, Recovery)
                | (Degraded, Starting)
                | (Degraded, Terminated)
                | (Recovery, Running)
                | (Recovery, Degraded)
                | (Recovery, Terminated)
                | (ProvisioningFailed, Terminated)
                | (BindingFailed, Terminated)
                | (StartFailed, Terminated)
        )
    }
}

/// 资源占用相位 (与 Resource.state 对齐的会话侧视图)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimPhase {
    Reserved,
    Allocated,
}

/// 会话侧资源占用凭据 (同一锁域内与 Resource.state 一致推进, 见 SessionManager)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceClaim {
    pub resource_id: Uuid,
    pub phase: ClaimPhase,
}

/// 会话健康快照 (watchdog/事件投影消费; 不含具体 vendor 字段)。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionHealthSnapshot {
    pub last_ok_at: Option<i64>,
    pub last_error: Option<String>,
}

/// MediaSession — canonical 运行时组合单元 (模型 §2; 逻辑持有、物理引用)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSession {
    pub session_id: SessionId,
    pub state: SessionState,
    pub phase: SessionPhase,
    /// canonical 语义图描述 (非 GStreamer 管线)。
    pub graphs: GraphRuntimeIntent,
    pub source_ports: Vec<Uuid>,
    pub output_ports: Vec<Uuid>,
    pub resource_claims: Vec<ResourceClaim>,
    pub leases: Vec<DeviceLease>,
    /// Backend 所拥有的 pipeline 实例句柄 (Handle 链接 Session↔对象)。
    /// Alpha-1 兼容保留 = 首输入主句柄（= `inputs.first()`）。
    pub pipeline: Option<PipelineHandle>,
    /// P1a: **物化**输出 kind 列表（start() 从 materialize 产物回填; 空 = 纯分析——
    /// 含 kind 声明输出但目标 env 缺失的 fail-soft 降级, 绝不虚报声明态）。
    pub outputs: Vec<String>,
    /// Alpha-1 (D10 激活): 多输入句柄表（每实例化管线一行, 序 = plans 序;
    /// 空 = 未 start。start() 全量回填, stop 逆序全停——creator=destroyer 延续）。
    pub inputs: Vec<SessionInput>,
    pub health: SessionHealthSnapshot,
    pub created_at: i64,
}

/// Alpha-1: 会话输入摘要（D10 每管线句柄表行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInput {
    /// canonical 设备身份（materialize 解析后所绑设备）。
    pub device_id: Uuid,
    pub handle: PipelineHandle,
}

/// 会话错误。
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("preflight FAIL: {0:?}")]
    PreflightFailed(PreflightReport),
    #[error("resource conflict: {0}")]
    ResourceConflict(String),
    #[error("resource state error: {0}")]
    ResourceState(#[from] crate::resource::ResourceStateError),
    #[error("lease error: {0}")]
    Lease(#[from] crate::lease::LeaseError),
    #[error("unknown session {0}")]
    UnknownSession(SessionId),
    #[error("invalid transition: {0}")]
    InvalidTransition(String),
    #[error("pipeline error: {0}")]
    Pipeline(#[from] crate::pipeline::PipelineError),
    #[error("backend unavailable: {0}")]
    BackendUnavailable(String),
}

/// 会话运行参数 (接线 Config 既有旋钮)。
#[derive(Debug, Clone)]
pub struct SessionTuning {
    pub default_lease_ttl: Duration,
    pub lease_renew_window: Duration,
    /// Reserved 相位最大停留时长 (超时 → 预留过期 + Terminated; crash-cleanup 近似)。
    pub reservation_window_ms: u64,
}

impl Default for SessionTuning {
    fn default() -> Self {
        Self {
            default_lease_ttl: Duration::from_secs(60),
            lease_renew_window: Duration::from_secs(15),
            reservation_window_ms: 30_000,
        }
    }
}

/// 运行时私有槽 (不进 MediaSession 语义结构)。
#[derive(Debug, Clone)]
struct SessionInner {
    session: MediaSession,
    /// 资源/租约 holder (= session_id.0)。
    holder: Uuid,
    created_at_ms: u64,
}

/// Runtime Session Manager — **Session 唯一创建者/销毁者** (模型 §4.1)。
///
/// **0.7C-6 D8 解耦**: 事件经注入的 `RuntimeEventSink` 直连组合根唯一
/// `RuntimeEventLog` (单表单锁全局 FIFO), **不再穿越 Supervisor** —
/// `sup` 只剩恢复决策职责 (watchdog tick 调用面零变更)。
pub struct SessionManager {
    resources: SharedResourceRegistry,
    leases: Arc<dyn LeaseManager>,
    sup: Arc<Mutex<Supervisor>>,
    backend: OnceLock<Arc<dyn MediaBackend>>,
    devices: Arc<Vec<DeviceInfo>>,
    bindings: Arc<HashMap<Uuid, ResolvedDeviceBinding>>,
    registry: Option<PortRegistry>,
    mode: MaterializeMode,
    tuning: SessionTuning,
    sessions: Mutex<HashMap<SessionId, SessionInner>>,
    events: Arc<dyn RuntimeEventSink>,
    /// D14: 观察序号起点 1（0 = absent 保留给 serde default）; 每次 runtime_state() 递增。
    observation_revision: std::sync::atomic::AtomicU64,
    /// D14: 观察谱系（构造一次 UUIDv4; 重启换新 → (lineage, revision) 进程内全序）。
    observation_lineage: uuid::Uuid,
}

impl SessionManager {
    /// 构造 (依赖注入; Backend 由调用方按 AdapterRegistry 规则提供 — mock 世界注 MockBackend,
    /// 真机注 `AdapterRegistry::build_media_backend()` 结果)。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        resources: SharedResourceRegistry,
        leases: Arc<dyn LeaseManager>,
        sup: Arc<Mutex<Supervisor>>,
        backend: Arc<dyn MediaBackend>,
        devices: Arc<Vec<DeviceInfo>>,
        bindings: Arc<HashMap<Uuid, ResolvedDeviceBinding>>,
        registry: Option<PortRegistry>,
        mode: MaterializeMode,
        tuning: SessionTuning,
        events: Arc<dyn RuntimeEventSink>,
    ) -> Self {
        let b = OnceLock::new();
        let _ = b.set(backend);
        Self {
            resources,
            leases,
            sup,
            backend: b,
            devices,
            bindings,
            registry,
            mode,
            tuning,
            sessions: Mutex::new(HashMap::new()),
            events,
            observation_revision: std::sync::atomic::AtomicU64::new(1),
            observation_lineage: uuid::Uuid::new_v4(),
        }
    }

    fn emit(&self, ev: RuntimeEvent) {
        self.events.emit(ev);
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn backend(&self) -> Result<Arc<dyn MediaBackend>, SessionError> {
        self.backend
            .get()
            .cloned()
            .ok_or_else(|| SessionError::BackendUnavailable("backend 未注入".into()))
    }

    /// 依赖注入 Backend (main 真机路径: AdapterRegistry::build_media_backend 的结果)。
    pub fn set_backend(&self, backend: Arc<dyn MediaBackend>) {
        let _ = self.backend.set(backend);
    }

    /// 由 intent + 资源注册表推导目标资源占用请求 (每个设备 1 个 `-input` 资源;
    /// registry=None 时为空 → Preflight WARN legacy 路径)。
    fn derive_claims(&self, intent: &GraphRuntimeIntent) -> Vec<AcquisitionRequest> {
        // holder 占位 (preflight 判定阶段不写任何状态; 真实 holder = session_id 在 create 时填入)。
        let holder = Uuid::nil();
        let mut claims = Vec::new();
        self.resources.with_inner(|reg| {
            for d in &intent.devices {
                let Ok(u) = Uuid::parse_str(&d.device_id) else {
                    continue;
                };
                if let Some(res) = reg
                    .resources
                    .iter()
                    .find(|r| r.device_id == u && r.capability.ends_with("-input"))
                {
                    claims.push(AcquisitionRequest {
                        holder,
                        resource_id: res.id,
                        expected_capability: res.capability.clone(),
                    });
                }
            }
        });
        claims
    }

    /// 创建会话: Preflight → Reserve → 建档 → Lease → Binding verify (冻结顺序;
    /// 任一步失败 → 逆序回滚 → 零孤儿 → `Err`)。成功后会话处于粗态 Reserved / 相位 Leased。
    pub fn create(&self, intent: GraphRuntimeIntent) -> Result<SessionId, SessionError> {
        let session_id = SessionId(Uuid::new_v4());
        let holder = session_id.0;

        // 步 0: Requested 登记 (供诊断; 失败即移除)。
        self.sessions.lock().unwrap().insert(
            session_id,
            SessionInner {
                session: MediaSession {
                    session_id,
                    state: SessionState::Reserved,
                    phase: SessionPhase::Requested,
                    graphs: intent.clone(),
                    source_ports: Vec::new(),
                    output_ports: Vec::new(),
                    resource_claims: Vec::new(),
                    leases: Vec::new(),
                    pipeline: None,
                    outputs: Vec::new(),
                    inputs: Vec::new(),
                    health: SessionHealthSnapshot::default(),
                    created_at: Self::now_ms() as i64,
                },
                holder,
                created_at_ms: Self::now_ms(),
            },
        );

        let cleanup = |mgr: &Self, sid: SessionId, holder: Uuid| {
            // create 阶段回滚: 释放预留/全部租约 → 移除表项 (零孤儿)。
            mgr.resources.release_reservations(holder);
            if let Some(inner) = mgr.sessions.lock().unwrap().remove(&sid) {
                for l in &inner.session.leases {
                    let _ = mgr.leases.release(l);
                }
            }
        };

        let result = self.create_inner(&session_id, &holder, &intent);
        if let Err(e) = &result {
            cleanup(self, session_id, holder);
            self.emit(RuntimeEvent::SessionFailed {
                session_id: session_id.0,
                reason: format!("create: {e}"),
            });
        }
        result.map(|_| session_id)
    }

    fn create_inner(
        &self,
        session_id: &SessionId,
        holder: &Uuid,
        intent: &GraphRuntimeIntent,
    ) -> Result<(), SessionError> {
        // 步 1: Preflight (只判断; FAIL ⇒ 零预留零回滚)。
        let claims = self.derive_claims(intent);
        {
            let report = self.run_preflight(intent, &claims);
            if !report.is_ok() {
                let _ = self.set_phase(session_id, SessionPhase::ProvisioningFailed);
                return Err(SessionError::PreflightFailed(report));
            }
        }

        // 步 2: Reserve (原子 acquire; 后到会话同资源 → ResourceConflict)。
        {
            let mut reserved: Vec<AcquisitionRequest> = Vec::new();
            for claim in &claims {
                let req = AcquisitionRequest {
                    holder: *holder,
                    ..claim.clone()
                };
                self.resources.acquire(&req).map_err(|e| {
                    SessionError::ResourceConflict(format!("{}: {e}", claim.resource_id))
                })?;
                reserved.push(req);
            }
            let mut guard = self.sessions.lock().unwrap();
            let inner = guard.get_mut(session_id).expect("session registered");
            inner.session.resource_claims = reserved
                .into_iter()
                .map(|r| ResourceClaim {
                    resource_id: r.resource_id,
                    phase: ClaimPhase::Reserved,
                })
                .collect();
            inner.session.source_ports = claims.iter().map(|c| c.resource_id).collect();
        }
        self.set_phase(session_id, SessionPhase::Provisioning)?;

        // 步 3: 建档完成 (粗态 Reserved), SessionCreated。
        self.emit(RuntimeEvent::SessionCreated {
            session_id: session_id.0,
        });

        // 步 4: Lease (owner = session id 字符串)。**事务式** (P0-1):
        // 多设备逐台 acquire, 任一台失败 → 逆序释放已获取的全部租约 (绝不留部分成功孤儿)。
        let mut leases: Vec<DeviceLease> = Vec::new();
        for d in &intent.devices {
            let u = Uuid::parse_str(&d.device_id)
                .map_err(|e| SessionError::InvalidTransition(format!("device_id 解析失败: {e}")))?;
            match self
                .leases
                .acquire(&u, &session_id.to_string(), self.tuning.default_lease_ttl)
            {
                Ok(l) => leases.push(l),
                Err(e) => {
                    for l in &leases {
                        let _ = self.leases.release(l);
                    }
                    return Err(e.into());
                }
            }
        }
        {
            let mut guard = self.sessions.lock().unwrap();
            let inner = guard.get_mut(session_id).expect("session registered");
            inner.session.leases = leases.clone();
            for l in &leases {
                self.emit(RuntimeEvent::LeaseGranted {
                    device_id: l.device_id,
                    lease_id: Uuid::new_v5(&Uuid::nil(), l.owner.as_bytes()),
                });
            }
        }

        // 步 5: Binding verify (bindings 为空 = legacy/simulation 路径, 跳过; 非空则目标设备须在场)。
        // 相位序 (Addendum §4.3): Provisioning → Binding → Leased (租约已持 + 绑定已验 = 就绪可 start)。
        if !self.bindings.is_empty() {
            // D5: 实查强度 (key-existence ≠ verified)。
            let missing: Vec<Uuid> = intent
                .devices
                .iter()
                .filter_map(|d| Uuid::parse_str(&d.device_id).ok())
                .filter(|u| {
                    !self
                        .bindings
                        .get(u)
                        .is_some_and(|b| b.is_production_grade())
                })
                .collect();
            if !missing.is_empty() {
                let _ = self.set_phase(session_id, SessionPhase::BindingFailed);
                return Err(SessionError::InvalidTransition(format!(
                    "目标设备缺少生产绑定: {missing:?}"
                )));
            }
            // P0-7D: 绑定验证通过 = 身份解析收敛语义时刻, 逐设备点亮
            // IdentityResolved (词表在册, 原零生产; confidence 用绑定时达到的
            // Confidence+ResolverMatch canonical 描述)。
            for u in intent
                .devices
                .iter()
                .filter_map(|d| Uuid::parse_str(&d.device_id).ok())
            {
                let confidence = self
                    .bindings
                    .get(&u)
                    .map(|b| format!("{:?}/{:?}", b.confidence, b.match_kind).to_lowercase())
                    .unwrap_or_else(|| "unverified".to_string());
                self.events.emit(RuntimeEvent::IdentityResolved {
                    device_id: u,
                    confidence,
                });
            }
        }
        self.set_phase(session_id, SessionPhase::Binding)?;
        self.set_phase(session_id, SessionPhase::Leased)?;
        Ok(())
    }

    /// 启动会话: materialize → Backend.instantiate → Allocate → Backend.start → Running
    /// (冻结顺序; 失败精确逆序回滚: stop→release allocation→release lease→release reservation)。
    pub fn start(&self, id: &SessionId) -> Result<(), SessionError> {
        let backend = self.backend()?;
        let (intent, holder, mode) = {
            let guard = self.sessions.lock().unwrap();
            let inner = guard.get(id).ok_or(SessionError::UnknownSession(*id))?;
            if inner.session.phase != SessionPhase::Leased
                && inner.session.phase != SessionPhase::Binding
            {
                return Err(SessionError::InvalidTransition(format!(
                    "start 需要相位 Leased/Binding, 实际 {:?} (double-start 防护)",
                    inner.session.phase
                )));
            }
            if inner.session.pipeline.is_some() {
                return Err(SessionError::InvalidTransition(
                    "pipeline 已存在 (double-start 防护)".into(),
                ));
            }
            (inner.session.graphs.clone(), inner.holder, self.mode)
        };
        self.set_phase(id, SessionPhase::Starting)?;

        // 步 1: materialize (纯函数)。**P0 (Round 3)**: 此时本会话已持有 lease/reservation —
        // materialize 失败 (identity/binding/runtime address/设备消失) 必须回滚,
        // Starting 相位绝不遗留 (RUNTIME_LIFECYCLE_SEQUENCE §2 零孤儿)。
        let plans = match crate::pipeline::materialize(
            &intent,
            &self.devices,
            mode,
            &self.bindings,
            self.registry.as_ref(),
        ) {
            Ok(p) => p,
            Err(e) => {
                self.rollback_lease_and_reservation(id, &holder);
                let _ = self.set_phase(id, SessionPhase::StartFailed);
                self.emit(RuntimeEvent::SessionFailed {
                    session_id: id.0,
                    reason: format!("materialize: {e}"),
                });
                return Err(e.into());
            }
        };
        // P1a: 物化输出回填会话（运行时可见性 = **物化事实**而非声明——kind=hls/rtmp 但
        // 目标 env 缺失的 fail-soft 降级在这里自然体现为 outputs 空, 绝不虚报）。
        {
            let materialized: Vec<String> = plans
                .iter()
                .flat_map(|p| p.outputs.iter().map(|o| o.kind.as_str().to_string()))
                .collect();
            if let Some(inner) = self.sessions.lock().unwrap().get_mut(id) {
                inner.session.outputs = materialized;
            }
        }
        let plan = match plans.first() {
            Some(p) => p,
            None => {
                self.rollback_lease_and_reservation(id, &holder);
                let _ = self.set_phase(id, SessionPhase::StartFailed);
                self.emit(RuntimeEvent::SessionFailed {
                    session_id: id.0,
                    reason: "materialize 产出空计划".into(),
                });
                return Err(SessionError::InvalidTransition(
                    "materialize 产出空计划".into(),
                ));
            }
        };
        self.emit(RuntimeEvent::SourceMaterialized {
            device_id: intent
                .devices
                .first()
                .and_then(|d| Uuid::parse_str(&d.device_id).ok())
                .unwrap_or(Uuid::nil()),
            pipeline: Uuid::new_v5(&Uuid::nil(), format!("{:?}", plan).as_bytes()),
        });

        // 步 2 (Alpha-1 / D10 激活): Backend.instantiate **全部** plans（逐个; 多输入会话
        // 不再只物化首计划）。任一失败 → 已建句柄全 stop + 既有逆序回滚
        // (lease+reservation, 零孤儿延续)。
        let mut inputs: Vec<SessionInput> = Vec::with_capacity(plans.len());
        for plan in &plans {
            match backend.instantiate(plan) {
                Ok(h) => inputs.push(SessionInput {
                    device_id: Uuid::parse_str(&plan.source.device_id)
                        .unwrap_or_else(|_| Uuid::nil()),
                    handle: h,
                }),
                Err(e) => {
                    for i in inputs.iter().rev() {
                        let _ = backend.stop(&i.handle);
                    }
                    self.rollback_lease_and_reservation(id, &holder);
                    let _ = self.set_phase(id, SessionPhase::StartFailed);
                    self.emit(RuntimeEvent::SessionFailed {
                        session_id: id.0,
                        reason: format!("backend.instantiate: {e}"),
                    });
                    return Err(e.into());
                }
            }
        }
        let handle = inputs
            .first()
            .map(|i| i.handle)
            .expect("plans 非空已保证 inputs 非空");

        // 步 3: Allocate (Reserved → Allocated; 失败 → stop 句柄回滚)。
        let alloc_result: Result<(), crate::resource::ResourceStateError> = (|| {
            let claims: Vec<Uuid> = self
                .sessions
                .lock()
                .unwrap()
                .get(id)
                .map(|i| {
                    i.session
                        .resource_claims
                        .iter()
                        .map(|c| c.resource_id)
                        .collect()
                })
                .unwrap_or_default();
            for rid in claims {
                self.resources.allocate_for(&rid, holder)?;
                // claim 相位同步 (会话侧视图)。
                let mut guard = self.sessions.lock().unwrap();
                if let Some(inner) = guard.get_mut(id) {
                    for c in inner.session.resource_claims.iter_mut() {
                        if c.resource_id == rid {
                            c.phase = ClaimPhase::Allocated;
                        }
                    }
                }
                self.emit(RuntimeEvent::ResourceAllocated { resource_id: rid });
            }
            Ok(())
        })();
        if let Err(e) = alloc_result {
            // Alpha-1 (review Critical#1): 已建句柄**全部**逆序 stop——allocate 失败与
            // 实例化/启动失败同属零孤儿不变量（单停首句柄会泄漏 1..N-1 管线
            // + bus-watch GLib 线程; review 实证抓出）。
            for i in inputs.iter().rev() {
                let _ = backend.stop(&i.handle);
            }
            // P0-1: 先回收本会话**已 Allocated** 的资源 (release_reservations 只处理 Reserved,
            // 部分成功分配的资源若不在此回收即成 Allocated orphan), 再回收租约/预留。
            self.resources.release_allocation(holder);
            self.rollback_lease_and_reservation(id, &holder);
            let _ = self.set_phase(id, SessionPhase::StartFailed);
            self.emit(RuntimeEvent::SessionFailed {
                session_id: id.0,
                reason: format!("allocate: {e}"),
            });
            return Err(e.into());
        }

        // 步 4 (Alpha-1): Backend.start **全部**句柄。失败 → 逆序: 全 stop →
        // release allocation → lease/reservation（零孤儿延续）。
        for i in &inputs {
            if let Err(e) = backend.start(&i.handle) {
                for j in inputs.iter().rev() {
                    let _ = backend.stop(&j.handle);
                }
                self.resources.release_allocation(holder);
                self.rollback_lease_and_reservation(id, &holder);
                let _ = self.set_phase(id, SessionPhase::StartFailed);
                self.emit(RuntimeEvent::SessionFailed {
                    session_id: id.0,
                    reason: format!("backend.start: {e}"),
                });
                return Err(e.into());
            }
        }

        // 成功: RUNNING。
        // P0-2: 粗态/相位经白名单守卫迁移 (Starting → Running)。
        self.set_phase(id, SessionPhase::Running)?;
        let mut guard = self.sessions.lock().unwrap();
        let inner = guard.get_mut(id).expect("session registered");
        inner.session.pipeline = Some(handle);
        inner.session.inputs = inputs;
        Self::transition_state(&mut inner.session.state, SessionState::Running)?;
        inner.session.health.last_ok_at = Some(Self::now_ms() as i64);
        drop(guard);
        self.emit(RuntimeEvent::SessionStateChanged {
            session_id: id.0,
            from: "reserved".into(),
            to: "running".into(),
        });
        Ok(())
    }

    /// 停止会话: Backend.stop → release allocation → release lease → release reservation
    /// → Releasing→Released (精确逆序)。double-stop 返回 InvalidTransition (close 幂等)。
    pub fn stop(&self, id: &SessionId) -> Result<(), SessionError> {
        let (handles, holder) = {
            let guard = self.sessions.lock().unwrap();
            let inner = guard.get(id).ok_or(SessionError::UnknownSession(*id))?;
            if inner.session.phase == SessionPhase::Released
                || inner.session.phase == SessionPhase::Stopping
            {
                return Err(SessionError::InvalidTransition(format!(
                    "stop 需要活动相位, 实际 {:?} (double-stop 防护)",
                    inner.session.phase
                )));
            }
            // Alpha-1: 全句柄逆序停止（多输入句柄表; 兼容回退单 pipeline 字段）。
            let mut hs: Vec<PipelineHandle> =
                inner.session.inputs.iter().map(|i| i.handle).collect();
            if hs.is_empty() {
                if let Some(h) = inner.session.pipeline {
                    hs.push(h);
                }
            }
            (hs, inner.holder)
        };
        self.set_phase(id, SessionPhase::Stopping)?;

        // 逆序 1 (Alpha-1: 全句柄逆序): Backend.stop。**P0-2**: stop 失败只记录,
        // **绝不截断后续释放链** — Session 层资源 (allocation/lease/reservation)
        // 无论 backend 结果如何都必须归还, 否则停止失败会让整个资源生命周期卡死。
        let mut stop_error: Option<crate::pipeline::PipelineError> = None;
        for h in handles.iter().rev() {
            if let Err(e) = self.backend()?.stop(h) {
                tracing::warn!(error = %e, "backend.stop 失败; 仍继续释放 Session 层资源 (P0-2)");
                stop_error = Some(e);
            }
        }
        // 逆序 2: release allocation。
        self.resources.release_allocation(holder);
        // 逆序 3+4: lease + reservation 兜底。
        self.rollback_lease_and_reservation(id, &holder);

        // P0-2: 粗态两步经白名单守卫 (Running/Reserved → Releasing → Released)。
        {
            let mut guard = self.sessions.lock().unwrap();
            let inner = guard.get_mut(id).expect("session registered");
            inner.session.pipeline = None;
            // Alpha-1: 句柄表同步清空（released 投影诚实——无活动输入）。
            inner.session.inputs = Vec::new();
            if let Some(e) = &stop_error {
                inner.session.health.last_error = Some(format!("backend.stop: {e}"));
            }
            Self::transition_state(&mut inner.session.state, SessionState::Releasing)?;
        }
        self.set_phase(id, SessionPhase::Released)?;
        {
            let mut guard = self.sessions.lock().unwrap();
            let inner = guard.get_mut(id).expect("session registered");
            Self::transition_state(&mut inner.session.state, SessionState::Released)?;
        }
        self.emit(RuntimeEvent::SessionStateChanged {
            session_id: id.0,
            from: "running".into(),
            to: "released".into(),
        });
        // P0-2: 全部释放已完成 — stop 失败在此上报 (错误不吞, 资源不卡;
        // 会话终态 Released, 停止失败详情在 health.last_error)。
        if let Some(e) = stop_error {
            return Err(e.into());
        }
        Ok(())
    }

    /// 关闭并移除会话。**P0-3**: 仅接受终态 (Released/Terminated/失败终态) —
    /// Running/其他活动相位必须先 `stop`, 绝不允许 close 绕过 pipeline 停止
    /// 造成 runtime orphan pipeline。终态移除时兜底回收 allocation/租约/预留 (零孤儿)。
    pub fn close(&self, id: &SessionId) -> Result<(), SessionError> {
        let (phase, _snapshot) = {
            let guard = self.sessions.lock().unwrap();
            let inner = guard.get(id).ok_or(SessionError::UnknownSession(*id))?;
            (inner.session.phase, inner.clone())
        };
        match phase {
            SessionPhase::Released
            | SessionPhase::Terminated
            | SessionPhase::ProvisioningFailed
            | SessionPhase::BindingFailed
            | SessionPhase::StartFailed => {}
            _ => {
                return Err(SessionError::InvalidTransition(format!(
                    "close 仅接受 Released/Terminated/失败终态, 实际 {phase:?} (活动会话请先 stop, 防 pipeline 孤儿)"
                )))
            }
        }
        if let Some(inner) = self.sessions.lock().unwrap().remove(id) {
            // 兜底零孤儿 (终态正常已释放; 防御性回收)。
            self.resources.release_allocation(inner.holder);
            self.resources.release_reservations(inner.holder);
            for l in &inner.session.leases {
                let _ = self.leases.release(l);
            }
        }
        Ok(())
    }

    /// 会话快照。
    pub fn status(&self, id: &SessionId) -> Option<MediaSession> {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .map(|i| i.session.clone())
    }

    /// **P0.7C Foundation: 第一条 Canonical→Runtime 生产聚合边**——从 SessionManager
    /// 持有的运行事实 (devices/bindings/registry/resources/sessions) 装配
    /// `CanonicalRuntimeState`（组合 canonical descriptor, 绝不平铺; 见 runtime_state.rs）。
    pub fn runtime_state(&self) -> crate::runtime_state::CanonicalRuntimeState {
        // D14: 先取序号（观测开始锚点）, 再采集各源 —— swept, start-ordered（Design Doc §4.1）。
        // Relaxed 充分: fetch_add 原子读改写在任何 ordering 下不重号不空洞（Design Doc §3.2）。
        let rev = self
            .observation_revision
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let resources = self.resources.with_inner(|r| r.clone());
        let registry = self.registry.clone().unwrap_or_default();
        let sessions = self.list();
        crate::runtime_state::CanonicalRuntimeState::assemble(
            &self.devices,
            &registry,
            &resources,
            &self.bindings,
            &sessions,
            &crate::runtime_state::SnapshotObservation {
                revision: rev,
                lineage: self.observation_lineage,
                observed_at_ms: Self::now_ms(),
            },
        )
    }

    /// 全部会话快照。
    pub fn list(&self) -> Vec<MediaSession> {
        self.sessions
            .lock()
            .unwrap()
            .values()
            .map(|i| i.session.clone())
            .collect()
    }

    /// 周期维护 (health 端点/watchdog tick 借用驱动; 无后台线程):
    /// (a) lease 续期 (剩余 < renew_window → renew); (b) Reserved 相位超时 → 预留过期 +
    /// Terminated (crash-cleanup 近似); (c) 过期租约清扫 (leases.health())。
    pub fn tick(&self) {
        // (c) 过期租约清扫 (内部 retain)。
        let _ = self.leases.health();
        let now = Self::now_ms();
        let holders: Vec<(SessionId, Uuid, SessionPhase)> = self
            .sessions
            .lock()
            .unwrap()
            .values()
            .map(|i| (i.session.session_id, i.holder, i.session.phase))
            .collect();
        for (sid, holder, phase) in holders {
            // (a) 续期: 锁内判定窗口, 锁外 renew, 成功后回写会话副本 (快照与租约表一致)。
            if matches!(
                phase,
                SessionPhase::Leased | SessionPhase::Binding | SessionPhase::Running
            ) {
                // P0-1: 多设备会话 — 逐台租约判定续期窗口并回写会话副本。
                let targets: Vec<(Uuid, String)> = self
                    .sessions
                    .lock()
                    .unwrap()
                    .get(&sid)
                    .map(|inner| {
                        inner
                            .session
                            .leases
                            .iter()
                            .filter_map(|l| {
                                let remaining = l
                                    .acquired_at
                                    .timestamp_millis()
                                    .checked_add_unsigned(l.ttl.as_millis() as u64)
                                    .unwrap_or(i64::MAX)
                                    - Self::now_ms() as i64;
                                ((remaining as u64)
                                    < self.tuning.lease_renew_window.as_millis() as u64)
                                    .then(|| (l.device_id, l.owner.clone()))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                for (device_id, owner) in targets {
                    if let Ok(updated) =
                        self.leases
                            .renew(&device_id, &owner, self.tuning.default_lease_ttl)
                    {
                        let mut guard = self.sessions.lock().unwrap();
                        if let Some(inner) = guard.get_mut(&sid) {
                            if let Some(l) = inner
                                .session
                                .leases
                                .iter_mut()
                                .find(|l| l.device_id == device_id)
                            {
                                *l = updated;
                            }
                        }
                    }
                }
            }
            // (b) Reserved 停留超时 → 预留过期 + Terminated (不 Running 的滞留会话)。
            if phase == SessionPhase::Provisioning || phase == SessionPhase::Leased {
                let stale = self
                    .sessions
                    .lock()
                    .unwrap()
                    .get(&sid)
                    .map(|i| now - i.created_at_ms > self.tuning.reservation_window_ms)
                    .unwrap_or(false);
                if stale {
                    // P0-7D: 逐资源发射 ResourceReservationExpired (词表在册, 原零生产)。
                    for rid in self.resources.expire_reservations_of(holder) {
                        self.events
                            .emit(RuntimeEvent::ResourceReservationExpired { resource_id: rid });
                    }
                    // 全部租约一并回收 (Terminated 零孤儿; RESOURCE-RT-01 crash cleanup)。
                    let leases = self
                        .sessions
                        .lock()
                        .unwrap()
                        .get(&sid)
                        .map(|i| i.session.leases.clone())
                        .unwrap_or_default();
                    for l in &leases {
                        let _ = self.leases.release(l);
                    }
                    {
                        let mut guard = self.sessions.lock().unwrap();
                        if let Some(inner) = guard.get_mut(&sid) {
                            inner.session.leases.clear();
                        }
                    }
                    // set_phase 需重新获取 sessions 锁 — 先释放 (防自死锁)。
                    let _ = self.set_phase(&sid, SessionPhase::Terminated);
                    self.emit(RuntimeEvent::SessionFailed {
                        session_id: sid.0,
                        reason: "reserved 相位超时 (crash-cleanup)".into(),
                    });
                }
            }
        }
    }

    // ── 内部工具 ────────────────────────────────────────────────────────────────

    fn run_preflight(
        &self,
        intent: &GraphRuntimeIntent,
        claims: &[AcquisitionRequest],
    ) -> PreflightReport {
        let empty: HashMap<Uuid, ResolvedDeviceBinding> = HashMap::new();
        let bindings: &HashMap<Uuid, ResolvedDeviceBinding> = if self.bindings.is_empty() {
            &empty
        } else {
            &self.bindings
        };
        let caps: Vec<crate::contracts::provider::CapabilityReport> = Vec::new();
        // 判定输入取资源表快照 (judge-only; ResourceRegistry: Clone, 规模为端口数级)。
        let snapshot = self.resources.with_inner(|r| r.clone());
        let inputs = PreflightInputs {
            intent,
            devices: &self.devices,
            resources: &snapshot,
            claims,
            leases: self.leases.as_ref(),
            bindings,
            capabilities: &caps,
            registry: self.registry.as_ref(),
        };
        crate::preflight::run(&inputs)
    }

    /// 相位迁移 (**P0-2 强制白名单**): 非法迁移拒绝并 Critical 上报, 绝不静默写入。
    fn set_phase(&self, id: &SessionId, to: SessionPhase) -> Result<(), SessionError> {
        let mut guard = self.sessions.lock().unwrap();
        let Some(inner) = guard.get_mut(id) else {
            return Err(SessionError::UnknownSession(*id));
        };
        let from = inner.session.phase;
        if from == to {
            return Ok(());
        }
        if !from.can_transition_to(to) {
            drop(guard);
            self.emit(RuntimeEvent::SessionFailed {
                session_id: id.0,
                reason: format!("非法相位迁移 {from:?} → {to:?} (白名单拒绝)"),
            });
            return Err(SessionError::InvalidTransition(format!(
                "非法相位迁移 {from:?} → {to:?}"
            )));
        }
        inner.session.phase = to;
        drop(guard);
        self.emit(RuntimeEvent::SessionStateChanged {
            session_id: id.0,
            from: format!("{from:?}").to_lowercase(),
            to: format!("{to:?}").to_lowercase(),
        });
        Ok(())
    }

    /// 粗态迁移守卫 (P0-2): 白名单外拒绝。
    fn transition_state(state: &mut SessionState, to: SessionState) -> Result<(), SessionError> {
        let from = *state;
        if from == to {
            return Ok(());
        }
        if !from.can_transition_to(to) {
            return Err(SessionError::InvalidTransition(format!(
                "非法粗态迁移 {from:?} → {to:?}"
            )));
        }
        *state = to;
        Ok(())
    }

    /// start 失败回滚 (全部租约 + reservation; allocation 已由调用方处理)。
    fn rollback_lease_and_reservation(&self, id: &SessionId, holder: &Uuid) {
        let leases = self
            .sessions
            .lock()
            .unwrap()
            .get(id)
            .map(|i| i.session.leases.clone())
            .unwrap_or_default();
        for l in &leases {
            let _ = self.leases.release(l);
        }
        // P0-1: 会话副本同步清空 (快照与租约表一致)。
        {
            let mut guard = self.sessions.lock().unwrap();
            if let Some(inner) = guard.get_mut(id) {
                inner.session.leases.clear();
            }
        }
        self.resources.release_reservations(*holder);
    }

    /// create 前段失败回滚 (由 create 的 cleanup 闭包统一处理)。
    fn rollback_to_pre_start(&self, id: &SessionId, holder: &Uuid) {
        self.rollback_lease_and_reservation(id, holder);
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::adapters::mock::{MockBackend, MockProvider, MockProviderB};
    use crate::events::{EventSeverity, RuntimeEvent};
    use crate::port::{
        PortDirection, PortIdentity, PortInfo, PortOrdinal, PortRegistry, SignalStatus,
        VideoContentState,
    };
    use crate::resource::ResourceRegistry;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

    type InMemoryLm = crate::lease::InMemoryLeaseManager;
    use crate::contracts::provider::HardwareProvider;
    use crate::lease::LeaseManager as LmTrait;

    // ── 测试世界构造 ────────────────────────────────────────────────────────────

    fn port_registry_for(dev: &DeviceInfo) -> PortRegistry {
        let pid = PortIdentity::derive(
            &dev.device_id,
            crate::port::ConnectorType::Sdi,
            PortOrdinal::Known(1),
        );
        PortRegistry {
            ports: vec![PortInfo {
                device_id: dev.device_id,
                provider_binding_ref: None,
                identity: PortIdentity {
                    port_id: pid,
                    connector: crate::port::ConnectorType::Sdi,
                    ordinal: PortOrdinal::Known(1),
                },
                direction: PortDirection::Input,
                capabilities: crate::port::PortCapabilities::default(),
                runtime_binding: None,
                signal: SignalStatus::default(),
                content: VideoContentState::Unknown,
            }],
        }
    }

    fn mock_devices() -> Vec<DeviceInfo> {
        MockProvider
            .discover()
            .expect("mock discover")
            .into_iter()
            .map(|d| d.device)
            .collect()
    }

    fn intent_for(dev: &DeviceInfo) -> GraphRuntimeIntent {
        GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![crate::graph_intent::DeviceIntent {
                device_id: dev.device_id.to_string(),
                role: "CAPTURE".into(),
                pipeline: crate::graph_intent::PipelineIntent {
                    source: crate::graph_intent::SourceIntent {
                        kind: "decklink".into(),
                        device_id: dev.device_id.to_string(),
                        port_id: None,
                    },
                    sink: crate::graph_intent::SinkIntent {
                        kind: "appsink".into(),
                    },
                },
            }],
        }
    }

    fn port_registry_for_devices(devs: &[DeviceInfo]) -> PortRegistry {
        let mut ports = Vec::new();
        for dev in devs {
            let pid = PortIdentity::derive(
                &dev.device_id,
                crate::port::ConnectorType::Sdi,
                PortOrdinal::Known(1),
            );
            ports.push(PortInfo {
                device_id: dev.device_id,
                provider_binding_ref: None,
                identity: PortIdentity {
                    port_id: pid,
                    connector: crate::port::ConnectorType::Sdi,
                    ordinal: PortOrdinal::Known(1),
                },
                direction: PortDirection::Input,
                capabilities: crate::port::PortCapabilities::default(),
                runtime_binding: None,
                signal: SignalStatus::default(),
                content: VideoContentState::Unknown,
            });
        }
        PortRegistry { ports }
    }

    fn intent_multi(devs: &[DeviceInfo]) -> GraphRuntimeIntent {
        GraphRuntimeIntent {
            version: "1.0".into(),
            devices: devs
                .iter()
                .map(|dev| crate::graph_intent::DeviceIntent {
                    device_id: dev.device_id.to_string(),
                    role: "CAPTURE".into(),
                    pipeline: crate::graph_intent::PipelineIntent {
                        source: crate::graph_intent::SourceIntent {
                            kind: "decklink".into(),
                            device_id: dev.device_id.to_string(),
                            port_id: None,
                        },
                        sink: crate::graph_intent::SinkIntent {
                            kind: "appsink".into(),
                        },
                    },
                })
                .collect(),
        }
    }

    fn manager_with(
        backend: Arc<dyn MediaBackend>,
        devices: &[DeviceInfo],
        lm: Arc<dyn LmTrait>,
        tuning: SessionTuning,
    ) -> SessionManager {
        let resources = SharedResourceRegistry::new(ResourceRegistry::derive_from_discovery(
            &port_registry_for_devices(devices),
        ));
        let event_log = Arc::new(crate::events::RuntimeEventLog::new());
        let sup = Arc::new(Mutex::new(Supervisor::new(
            crate::supervisor::RestartPolicy::default(),
            event_log.clone(),
        )));
        SessionManager::new(
            resources,
            lm,
            sup,
            backend,
            Arc::new(devices.to_vec()),
            Arc::new(HashMap::new()),
            None,
            MaterializeMode::Diagnostic,
            tuning,
            event_log,
        )
    }

    fn mock_manager(devices: &[DeviceInfo], lm: Arc<dyn LmTrait>) -> SessionManager {
        manager_with(Arc::new(MockBackend), devices, lm, SessionTuning::default())
    }

    // ── Alpha-1: 多输入多管线（D10 激活） ────────────────────────────────────────

    fn two_devices() -> Vec<DeviceInfo> {
        MockProviderB
            .discover()
            .expect("mock-b discover")
            .into_iter()
            .map(|d| d.device)
            .collect()
    }

    fn intent_for_all(devices: &[DeviceInfo]) -> GraphRuntimeIntent {
        GraphRuntimeIntent {
            version: "1.0".into(),
            devices: devices
                .iter()
                .map(|d| crate::graph_intent::DeviceIntent {
                    device_id: d.device_id.to_string(),
                    role: "CAPTURE".into(),
                    pipeline: crate::graph_intent::PipelineIntent {
                        source: crate::graph_intent::SourceIntent {
                            kind: "decklink".into(),
                            device_id: d.device_id.to_string(),
                            port_id: None,
                        },
                        sink: crate::graph_intent::SinkIntent {
                            kind: "appsink".into(),
                        },
                    },
                })
                .collect(),
        }
    }

    #[test]
    fn session_rt_01_multi_device_session_instantiates_all_inputs() {
        let devices = two_devices();
        assert_eq!(devices.len(), 2);
        let lm = Arc::new(InMemoryLm::new());
        let mgr = mock_manager(&devices, lm);
        let sid = mgr.create(intent_for_all(&devices)).expect("create");
        mgr.start(&sid).expect("start 双输入");
        let s = mgr.status(&sid).expect("status");
        assert_eq!(s.inputs.len(), 2, "D10: 全部 plan 实例化（不再只取 first）");
        assert_eq!(
            s.inputs[0].device_id, devices[0].device_id,
            "输入序 = plans 序"
        );
        assert_eq!(s.inputs[1].device_id, devices[1].device_id);
        assert_eq!(
            s.pipeline,
            s.inputs.first().map(|i| i.handle),
            "pipeline = 首输入主句柄（兼容保留）"
        );
    }

    #[test]
    fn session_rt_01_multi_device_instantiate_failure_midway_zero_orphans() {
        let devices = two_devices();
        let lm = Arc::new(InMemoryLm::new());
        let fb = FailingBackend::new(FailAt::InstantiateSecond);
        let mgr = manager_with(fb.clone(), &devices, lm.clone(), SessionTuning::default());
        let sid = mgr.create(intent_for_all(&devices)).expect("create");
        let err = mgr.start(&sid).expect_err("第二个实例化注入失败必须传播");
        assert!(err.to_string().contains("injected-second"), "{err:?}");
        assert_eq!(
            fb.instances.load(Ordering::SeqCst),
            2,
            "两次实例化尝试（第二次注入失败; instances 计尝试）"
        );
        assert_eq!(
            fb.stops.load(Ordering::SeqCst),
            1,
            "已建首句柄必须 stop（零孤儿管线占卡）"
        );
        assert_zero_orphans(&mgr, &lm);
        assert_eq!(
            mgr.status(&sid).map(|s| s.phase),
            Some(SessionPhase::StartFailed)
        );
    }

    #[test]
    fn session_rt_01_multi_device_stop_stops_all_handles() {
        let devices = two_devices();
        let lm = Arc::new(InMemoryLm::new());
        let fb = FailingBackend::new(FailAt::Never);
        let mgr = manager_with(fb.clone(), &devices, lm.clone(), SessionTuning::default());
        let sid = mgr.create(intent_for_all(&devices)).expect("create");
        mgr.start(&sid).expect("start");
        assert_eq!(fb.instances.load(Ordering::SeqCst), 2);
        mgr.stop(&sid).expect("stop");
        assert_eq!(
            fb.stops.load(Ordering::SeqCst),
            2,
            "全部句柄逆序停止（creator=destroyer 延续）"
        );
        assert_zero_orphans(&mgr, &lm);
    }

    #[test]
    fn session_rt_01_multi_device_allocate_failure_stops_all_handles() {
        // review Critical#1 回归锁: allocate 失败时已建句柄必须**全部** stop
        // （注入 = 设备 2 资源 reservation 被外部接管 ⇒ allocate_for holder 不匹配）。
        let devices = two_devices();
        let lm = Arc::new(InMemoryLm::new());
        let fb = FailingBackend::new(FailAt::Never);
        let mgr = manager_with(fb.clone(), &devices, lm.clone(), SessionTuning::default());
        let sid = mgr.create(intent_for_all(&devices)).expect("create");
        mgr.resources.with_inner_mut(|r| {
            for res in r.resources.iter_mut() {
                if res.device_id == devices[1].device_id {
                    if let Some(rv) = res.reservation.as_mut() {
                        rv.holder = Uuid::new_v4();
                    }
                }
            }
        });
        let err = mgr
            .start(&sid)
            .expect_err("allocate 第二设备注入失败必须传播");
        assert!(
            matches!(err, SessionError::ResourceState(_)),
            "allocate 失败变体: {err:?}"
        );
        assert_eq!(
            fb.stops.load(Ordering::SeqCst),
            2,
            "已建**全部**句柄 stop（含首句柄外的 1..N-1——review Critical#1）"
        );
        assert!(
            lm.health().is_empty(),
            "租约全还（注入污染的 Reserved 资源不属本路径泄漏）"
        );
        assert_eq!(
            mgr.status(&sid).map(|s| s.phase),
            Some(SessionPhase::StartFailed)
        );
        // 清理注入（测试卫生: 被接管 reservation 复位避免跨断言污染）。
        mgr.resources.with_inner_mut(|r| {
            for res in r.resources.iter_mut() {
                if res.device_id == devices[1].device_id {
                    let _ = res.expire_reservation();
                }
            }
        });
    }

    #[test]
    fn session_rt_01_observation_revision_starts_at_1_and_increments() {
        let devices = mock_devices();
        let lm = Arc::new(InMemoryLm::new());
        let mgr = mock_manager(&devices, lm);
        let first = mgr.runtime_state();
        assert_eq!(
            first.observation_revision, 1,
            "首快照 revision 必为 1, 非 0"
        );
        let second = mgr.runtime_state();
        assert_eq!(second.observation_revision, 2, "连续调用严格 +1");
        assert_eq!(
            first.observation_lineage, second.observation_lineage,
            "同一 manager lineage 恒同"
        );
        assert_ne!(
            first.observation_lineage,
            Uuid::nil(),
            "lineage 为真实 UUIDv4, 非 nil"
        );
    }

    #[test]
    fn session_rt_01_restart_semantics_new_lineage_revision_back_to_1() {
        let devices = mock_devices();
        let lm = Arc::new(InMemoryLm::new());
        let gen1 = mock_manager(&devices, lm.clone());
        let s1 = gen1.runtime_state();
        assert_eq!(s1.observation_revision, 1);
        let gen2 = mock_manager(&devices, lm); // "重启" = 新 SessionManager
        let s2 = gen2.runtime_state();
        assert_eq!(
            s2.observation_revision, 1,
            "重启后 revision 归 1 (不承诺跨重启连续)"
        );
        assert_ne!(
            s1.observation_lineage, s2.observation_lineage,
            "重启必换新 lineage"
        );
    }

    #[test]
    fn session_rt_01_observation_revision_8x1000_concurrency_pierce() {
        let devices = mock_devices();
        let lm = Arc::new(InMemoryLm::new());
        let mgr = Arc::new(mock_manager(&devices, lm));
        let workers = 8;
        let per_worker = 1000usize;
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(workers));
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let m = Arc::clone(&mgr);
            let b = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                b.wait();
                (0..per_worker)
                    .map(|_| m.runtime_state())
                    .map(|s| (s.observation_revision, s.observation_lineage))
                    .collect::<Vec<_>>()
            }));
        }
        let mut all = Vec::with_capacity(workers * per_worker);
        for h in handles {
            all.extend(h.join().expect("worker 线程不得 panic"));
        }
        assert_eq!(all.len(), 8000, "8×1000 份快照全量收集");
        let mut revs: Vec<u64> = all.iter().map(|(r, _)| *r).collect();
        revs.sort_unstable();
        revs.dedup();
        let expect: Vec<u64> = (1..=8000).collect();
        assert_eq!(
            revs, expect,
            "revision 集合恰为 {{1..8000}}: 无重号 (唯一) 无空洞 (连续覆盖)"
        );
        let lineage0 = all[0].1;
        assert!(
            all.iter().all(|(_, l)| *l == lineage0),
            "8000 份快照 lineage 恒同 (单 manager)"
        );
    }

    fn assert_zero_orphans(mgr: &SessionManager, lm: &InMemoryLm) {
        // SESSION-RT-01 / RESOURCE-RT-01 零孤儿不变量: 失败会话的资源全部回 Available, 无残留租约。
        mgr.resources.with_inner(|reg| {
            for r in &reg.resources {
                assert_eq!(
                    r.state,
                    crate::resource::ResourceState::Available,
                    "资源未回滚: {}",
                    r.name
                );
            }
        });
        assert!(lm.health().is_empty(), "存在残留租约");
    }

    // ── FailingBackend 测试桩 (仅测试世界; 注入失败验证回滚零孤儿) ──────────────
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FailAt {
        Instantiate,
        /// Alpha-1: 首个实例化成功后注入失败（多输入中途回滚测试）。
        InstantiateSecond,
        Start,
        Stop,
        /// Alpha-1: 从不失败（多句柄 stop 计数测试载体）。
        Never,
    }

    struct FailingBackend {
        fail_at: FailAt,
        stop_called: AtomicBool,
        /// Alpha-1: stop 调用计数（多句柄全停断言）。
        stops: std::sync::atomic::AtomicU64,
        instances: AtomicU64,
    }

    impl FailingBackend {
        fn new(fail_at: FailAt) -> Arc<Self> {
            Arc::new(Self {
                fail_at,
                stop_called: AtomicBool::new(false),
                stops: std::sync::atomic::AtomicU64::new(0),
                instances: AtomicU64::new(0),
            })
        }
    }

    impl MediaBackend for FailingBackend {
        fn instantiate(
            &self,
            _plan: &crate::pipeline::PipelinePlan,
        ) -> Result<PipelineHandle, crate::pipeline::PipelineError> {
            if self.fail_at == FailAt::Instantiate {
                return Err(crate::pipeline::PipelineError::PrepareFailed(
                    "injected".into(),
                ));
            }
            let n = self.instances.fetch_add(1, Ordering::SeqCst);
            if self.fail_at == FailAt::InstantiateSecond && n >= 1 {
                return Err(crate::pipeline::PipelineError::PrepareFailed(
                    "injected-second".into(),
                ));
            }
            Ok(PipelineHandle(100 + n))
        }
        fn start(&self, _handle: &PipelineHandle) -> Result<(), crate::pipeline::PipelineError> {
            if self.fail_at == FailAt::Start {
                return Err(crate::pipeline::PipelineError::StartFailed(
                    "injected".into(),
                ));
            }
            Ok(())
        }
        fn stop(&self, _handle: &PipelineHandle) -> Result<(), crate::pipeline::PipelineError> {
            self.stop_called.store(true, Ordering::SeqCst);
            self.stops.fetch_add(1, Ordering::SeqCst);
            if self.fail_at == FailAt::Stop {
                return Err(crate::pipeline::PipelineError::StopFailed(
                    "injected stop failure".into(),
                ));
            }
            Ok(())
        }
        fn recover(&self, _handle: &PipelineHandle) -> Result<(), crate::pipeline::PipelineError> {
            Ok(())
        }
        fn observe(
            &self,
            _handle: &PipelineHandle,
        ) -> Vec<crate::pipeline_events::PipelineBusEvent> {
            Vec::new()
        }
    }

    // ── SESSION-RT-01 ───────────────────────────────────────────────────────────

    #[test]
    fn session_rt_01_state_machine_rejects_released_to_running() {
        // 纯状态机层 (#114): RELEASED→Running 恒拒绝。
        assert!(!SessionState::Released.can_transition_to(SessionState::Running));
        assert!(SessionState::Reserved.can_transition_to(SessionState::Running));
        assert!(SessionState::Running.can_transition_to(SessionState::Releasing));
        assert!(SessionState::Releasing.can_transition_to(SessionState::Released));
    }

    #[test]
    fn session_rt_01_full_lifecycle_create_start_running_stop_release() {
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let mgr = mock_manager(&devices, lm.clone());
        let sid = mgr.create(intent_for(&devices[0])).expect("create 应通过");
        let s = mgr.status(&sid).expect("会话已登记");
        assert_eq!(s.state, SessionState::Reserved);
        assert_eq!(s.phase, SessionPhase::Leased);
        assert!(!s.resource_claims.is_empty(), "mock 端口应派生资源 claim");
        assert_eq!(s.resource_claims[0].phase, ClaimPhase::Reserved);

        mgr.start(&sid).expect("start 应成功");
        let s = mgr.status(&sid).expect("会话存在");
        assert_eq!(s.state, SessionState::Running);
        assert_eq!(s.phase, SessionPhase::Running);
        assert!(s.pipeline.is_some());
        assert_eq!(s.resource_claims[0].phase, ClaimPhase::Allocated);

        // double-start 防护。
        assert!(matches!(
            mgr.start(&sid),
            Err(SessionError::InvalidTransition(_))
        ));

        mgr.stop(&sid).expect("stop 应成功");
        let s = mgr.status(&sid).expect("会话保留至 close");
        assert_eq!(s.state, SessionState::Released);
        assert_eq!(s.phase, SessionPhase::Released);
        // double-stop 防护。
        assert!(matches!(
            mgr.stop(&sid),
            Err(SessionError::InvalidTransition(_))
        ));
        assert_zero_orphans(&mgr, &lm);

        mgr.close(&sid).expect("终态 close 应成功");
        assert!(mgr.status(&sid).is_none(), "close 后移除");
    }

    #[test]
    fn session_rt_01_rollback_on_instantiate_failure_leaves_zero_orphans() {
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let backend = FailingBackend::new(FailAt::Instantiate);
        let mgr = manager_with(backend, &devices, lm.clone(), SessionTuning::default());
        let sid = mgr.create(intent_for(&devices[0])).expect("create 应通过");
        let err = mgr.start(&sid).expect_err("instantiate 注入失败应传播");
        assert!(matches!(err, SessionError::Pipeline(_)));
        assert_zero_orphans(&mgr, &lm);
        let s = mgr.status(&sid).expect("失败会话保留供诊断");
        assert_eq!(s.phase, SessionPhase::StartFailed);
    }

    #[test]
    fn session_rt_01_rollback_on_start_failure_stops_handle_and_releases_all() {
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let backend = FailingBackend::new(FailAt::Start);
        let mgr = manager_with(
            backend.clone(),
            &devices,
            lm.clone(),
            SessionTuning::default(),
        );
        let sid = mgr.create(intent_for(&devices[0])).expect("create 应通过");
        assert!(mgr.start(&sid).is_err());
        // start 失败 → 已 instantiate 的句柄必须被 stop (逆序回滚)。
        assert!(
            backend.stop_called.load(Ordering::SeqCst),
            "instantiate 出的句柄必须 stop"
        );
        assert_zero_orphans(&mgr, &lm);
    }

    // ── RESOURCE-RT-01 ──────────────────────────────────────────────────────────

    #[test]
    fn resource_rt_01_second_session_conflict_rejected_first_unaffected() {
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let mgr = mock_manager(&devices, lm.clone());
        let s1 = mgr
            .create(intent_for(&devices[0]))
            .expect("第一会话 create 应通过");
        // 第二会话争同一设备: Preflight LeaseConflict / 资源占用 fail-closed 拒绝。
        assert!(
            mgr.create(intent_for(&devices[0])).is_err(),
            "同资源第二会话必须被拒"
        );
        // 第一会话不受影响, 可正常 start (此时资源 Allocated 属合法运行态, 不套零孤儿断言)。
        mgr.start(&s1).expect("第一会话 start 不受拒绝影响");
        assert_eq!(mgr.status(&s1).unwrap().state, SessionState::Running);
        mgr.stop(&s1).unwrap();
        mgr.close(&s1).expect("终态 close 应成功");

        // 释放后新会话可重占 (release → re-acquire)。
        let s2 = mgr.create(intent_for(&devices[0])).expect("释放后应可重占");
        mgr.start(&s2).expect("重占后 start 应成功");
        mgr.stop(&s2).unwrap();
        mgr.close(&s2).expect("终态 close 应成功");
        assert_zero_orphans(&mgr, &lm);
    }

    #[test]
    fn resource_rt_01_tick_expires_stale_reserved_session() {
        // crash-cleanup 近似: Reserved 滞留超过窗口 → 预留过期 + Terminated。
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let tuning = SessionTuning {
            reservation_window_ms: 0,
            ..SessionTuning::default()
        };
        let mgr = manager_with(Arc::new(MockBackend), &devices, lm.clone(), tuning);
        let sid = mgr.create(intent_for(&devices[0])).expect("create 应通过");
        // 窗口=0: 需保证 tick 时 created_at 已成过去 (毫秒精度竞态防护)。
        std::thread::sleep(std::time::Duration::from_millis(10));
        mgr.tick();
        let s = mgr.status(&sid).expect("会话保留");
        assert_eq!(s.phase, SessionPhase::Terminated);
        assert_zero_orphans(&mgr, &lm);
        // 过期后资源可被新会话占用。
        let s2 = mgr
            .create(intent_for(&devices[0]))
            .expect("过期释放后应可重占");
        assert_ne!(s2, sid);
    }

    #[test]
    fn resource_rt_01_renew_window_tick_extends_lease() {
        // Lease renew: 巨大 renew 窗口 → tick 立即续期 (刷新 acquired_at)。
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let tuning = SessionTuning {
            default_lease_ttl: Duration::from_secs(120),
            lease_renew_window: Duration::from_secs(1_000_000),
            reservation_window_ms: 30_000,
        };
        let mgr = manager_with(Arc::new(MockBackend), &devices, lm, tuning);
        let sid = mgr.create(intent_for(&devices[0])).expect("create 应通过");
        let before = mgr
            .status(&sid)
            .unwrap()
            .leases
            .first()
            .expect("lease 已持有")
            .clone();
        assert_eq!(before.ttl, Duration::from_secs(120));
        std::thread::sleep(std::time::Duration::from_millis(20));
        mgr.tick();
        let after = mgr
            .status(&sid)
            .unwrap()
            .leases
            .first()
            .expect("lease 仍在")
            .clone();
        assert!(
            after.acquired_at > before.acquired_at,
            "tick 应刷新 acquired_at (续期)"
        );
    }

    #[test]
    fn session_failed_event_is_critical() {
        // 事件语义: SessionFailed = Critical (不可被日志挤出)。
        let ev = RuntimeEvent::SessionFailed {
            session_id: Uuid::nil(),
            reason: "t".into(),
        };
        assert_eq!(ev.severity(), EventSeverity::Critical);
    }

    // ── Merge Gate Hardening (P0-1/P0-2/P0-3): 多设备/异常/close 边界 ──────────────

    #[test]
    fn session_rt_01_multi_device_acquire_release() {
        // P0-1: 多设备会话 — 每台设备一份租约; stop 必须释放全部 (零孤儿)。
        let devices: Vec<DeviceInfo> = MockProviderB
            .discover()
            .expect("mock discover")
            .into_iter()
            .map(|d| d.device)
            .collect();
        assert_eq!(devices.len(), 2, "MockProviderB 应含两台设备");
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let mgr = mock_manager(&devices, lm.clone());
        let sid = mgr
            .create(intent_multi(&devices))
            .expect("多设备 create 应通过");
        let s = mgr.status(&sid).expect("会话已登记");
        assert_eq!(s.leases.len(), 2, "两台设备两份租约");
        assert_eq!(s.resource_claims.len(), 2);
        assert_eq!(lm.health().len(), 2, "两台设备租约均在 LeaseManager");
        mgr.start(&sid)
            .expect("多设备 start 应成功 (0.7A 单管线取首计划)");
        mgr.stop(&sid).expect("stop 应释放全部租约");
        assert!(lm.health().is_empty(), "P0-1: stop 后两台租约必须全部释放");
        mgr.close(&sid).expect("终态 close 应成功");
        assert_zero_orphans(&mgr, &lm);
    }

    /// 测试替身: 第 N 次 acquire 注入失败 (确定性驱动 P0-1 事务回滚路径;
    /// 直接预占会撞 Preflight 的 LeaseConflict, 到不了租约步)。
    struct CountingFailLeaseManager {
        inner: InMemoryLm,
        calls: AtomicU32,
        fail_on_call: u32,
    }

    impl crate::lease::LeaseManager for CountingFailLeaseManager {
        fn acquire(
            &self,
            device_id: &Uuid,
            owner: &str,
            ttl: std::time::Duration,
        ) -> Result<crate::lease::DeviceLease, crate::lease::LeaseError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            if n == self.fail_on_call {
                return Err(crate::lease::LeaseError::AlreadyLeased(*device_id));
            }
            self.inner.acquire(device_id, owner, ttl)
        }
        fn release(
            &self,
            lease: &crate::lease::DeviceLease,
        ) -> Result<(), crate::lease::LeaseError> {
            self.inner.release(lease)
        }
        fn health(&self) -> Vec<crate::lease::DeviceLease> {
            self.inner.health()
        }
        fn list_active(&self) -> Vec<crate::lease::DeviceLease> {
            self.inner.list_active()
        }
        fn renew(
            &self,
            device_id: &Uuid,
            owner: &str,
            ttl: std::time::Duration,
        ) -> Result<crate::lease::DeviceLease, crate::lease::LeaseError> {
            self.inner.renew(device_id, owner, ttl)
        }
    }

    #[test]
    fn session_rt_01_partial_lease_failure_rolls_back_acquired_leases() {
        // P0-1: 事务式租约获取 — 第二台失败 → 第一台已获取的租约必须逆序释放 (零孤儿)。
        let devices: Vec<DeviceInfo> = MockProviderB
            .discover()
            .expect("mock discover")
            .into_iter()
            .map(|d| d.device)
            .collect();
        let lm: Arc<CountingFailLeaseManager> = Arc::new(CountingFailLeaseManager {
            inner: InMemoryLm::new(),
            calls: std::sync::atomic::AtomicU32::new(0),
            fail_on_call: 2,
        });
        let mgr = manager_with(
            Arc::new(MockBackend),
            &devices,
            lm.clone(),
            SessionTuning::default(),
        );
        let err = mgr
            .create(intent_multi(&devices))
            .expect_err("第二台租约注入失败应传播");
        assert!(matches!(err, SessionError::Lease(_)), "实际错误变体: {err}");
        // 事务回滚: 第一台已获取的租约必须已释放 — 租约表空 (零孤儿)。
        assert!(
            lm.list_active().is_empty(),
            "部分成功的第一台租约必须已回滚"
        );
        // create() 外壳 cleanup: 预留已释放 + 会话表已移除。
        mgr.resources.with_inner(|reg| {
            for r in &reg.resources {
                assert_eq!(r.state, crate::resource::ResourceState::Available);
            }
        });
        assert!(mgr.list().is_empty(), "失败会话应由 create 外壳清理移除");
    }

    #[test]
    fn session_rt_01_close_running_rejected_no_pipeline_orphan() {
        // P0-3: Running 直接 close 必须拒绝 (pipeline 仍被会话持有, 防 orphan pipeline)。
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let mgr = mock_manager(&devices, lm.clone());
        let sid = mgr.create(intent_for(&devices[0])).expect("create 应通过");
        mgr.start(&sid).expect("start 应成功");
        assert!(matches!(
            mgr.close(&sid),
            Err(SessionError::InvalidTransition(_))
        ));
        let s = mgr.status(&sid).expect("会话未被移除");
        assert_eq!(s.state, SessionState::Running);
        assert!(s.pipeline.is_some(), "pipeline 仍被会话持有 (未成孤儿)");
        // 正确路径: stop → Released → close 成功。
        mgr.stop(&sid).unwrap();
        mgr.close(&sid).expect("终态 close 应成功");
        assert!(mgr.status(&sid).is_none());
        assert_zero_orphans(&mgr, &lm);
    }

    #[test]
    fn session_rt_01_illegal_phase_transition_rejected_via_manager_api() {
        // P0-2: 经 SessionManager 公共 API 驱动非法迁移必须被拒 (非仅 enum helper)。
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let tuning = SessionTuning {
            reservation_window_ms: 0,
            ..SessionTuning::default()
        };
        let mgr = manager_with(Arc::new(MockBackend), &devices, lm, tuning);
        let sid = mgr.create(intent_for(&devices[0])).expect("create 应通过");
        std::thread::sleep(std::time::Duration::from_millis(10));
        mgr.tick(); // Reserved 滞留 → Terminated (白名单迁移)
        assert_eq!(
            mgr.status(&sid).unwrap().phase,
            SessionPhase::Terminated,
            "滞留会话应被 crash-cleanup 终结"
        );
        // Terminated → Starting 非法 → start 拒绝 (API 层)。
        assert!(matches!(
            mgr.start(&sid),
            Err(SessionError::InvalidTransition(_))
        ));
        // 白名单恒拒绝 (#114 与终态不可复活)。
        assert!(!SessionPhase::Released.can_transition_to(SessionPhase::Running));
        assert!(!SessionPhase::Terminated.can_transition_to(SessionPhase::Running));
    }

    #[test]
    fn resource_rt_01_partial_allocation_failure_releases_all_claims() {
        // P0-1 (Round 2): 多资源 — A allocate ✅ B allocate ❌ → A 必须回 Available
        // (**不能成为 Allocated orphan**), 租约全释放, 句柄 stop, 会话进入 StartFailed。
        let devices: Vec<DeviceInfo> = MockProviderB
            .discover()
            .expect("mock discover")
            .into_iter()
            .map(|d| d.device)
            .collect();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let mgr = mock_manager(&devices, lm.clone());
        let sid = mgr
            .create(intent_multi(&devices))
            .expect("多资源 create 应通过");
        let claims: Vec<Uuid> = mgr
            .status(&sid)
            .unwrap()
            .resource_claims
            .iter()
            .map(|c| c.resource_id)
            .collect();
        assert_eq!(claims.len(), 2);
        // 注入: B 资源在 start 前被他人抢占为 Allocated (模拟 create→start 窗口内的并发竞争)。
        let other = Uuid::new_v4();
        mgr.resources.with_inner_mut(|reg| {
            let b = reg
                .resources
                .iter_mut()
                .find(|r| r.id == claims[1])
                .expect("B 资源存在");
            b.state = crate::resource::ResourceState::Allocated;
            b.allocated_to = Some(other);
            b.reservation = None;
        });
        let err = mgr.start(&sid).expect_err("B allocate 注入失败应传播");
        assert!(matches!(err, SessionError::ResourceState(_)));
        // P0-1 核心断言: A (已 Allocated) 必须被回滚到 Available。
        mgr.resources.with_inner(|reg| {
            let a = reg
                .resources
                .iter()
                .find(|r| r.id == claims[0])
                .expect("A 存在");
            assert_eq!(
                a.state,
                crate::resource::ResourceState::Available,
                "部分分配失败的 A 必须回滚 (Allocated orphan = 违反 creator=destroyer)"
            );
            let b = reg
                .resources
                .iter()
                .find(|r| r.id == claims[1])
                .expect("B 存在");
            assert_eq!(
                b.state,
                crate::resource::ResourceState::Allocated,
                "他人持有不变"
            );
            assert_eq!(b.allocated_to, Some(other));
        });
        // B 为他人持有 — 测试自身收尾 (非会话职责)。
        mgr.resources.with_inner_mut(|reg| {
            let b = reg
                .resources
                .iter_mut()
                .find(|r| r.id == claims[1])
                .unwrap();
            b.begin_release().unwrap();
            b.finish_release().unwrap();
        });
        let s = mgr.status(&sid).unwrap();
        assert_eq!(s.phase, SessionPhase::StartFailed);
        assert!(s.pipeline.is_none(), "失败回滚后句柄已 stop");
        assert_zero_orphans(&mgr, &lm);
    }

    #[test]
    fn session_rt_01_stop_failure_still_releases_everything() {
        // P0-2: Backend.stop 失败 → 不得截断释放链 — allocation/lease/reservation 全部归还,
        // 会话终态 Released + last_error 记录, 错误在完全释放后上报。
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let mgr = manager_with(
            FailingBackend::new(FailAt::Stop),
            &devices,
            lm.clone(),
            SessionTuning::default(),
        );
        let sid = mgr.create(intent_for(&devices[0])).expect("create 应通过");
        mgr.start(&sid)
            .expect("start 应成功 (Stop 注入不影响 start)");
        let err = mgr.stop(&sid).expect_err("stop 注入失败应上报");
        assert!(matches!(err, SessionError::Pipeline(_)));
        let s = mgr.status(&sid).expect("会话保留 (Released)");
        assert_eq!(s.state, SessionState::Released);
        assert_eq!(s.phase, SessionPhase::Released);
        assert!(s.pipeline.is_none());
        assert!(s.health.last_error.is_some(), "stop 失败记入 health");
        assert_zero_orphans(&mgr, &lm);
    }

    #[test]
    fn resource_rt_01_stop_failure_multi_resource_still_releases() {
        // P0-2 × 多资源: stop 失败时多资源会话仍必须全部归还 (生命周期失败矩阵)。
        let devices: Vec<DeviceInfo> = MockProviderB
            .discover()
            .expect("mock discover")
            .into_iter()
            .map(|d| d.device)
            .collect();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let mgr = manager_with(
            FailingBackend::new(FailAt::Stop),
            &devices,
            lm.clone(),
            SessionTuning::default(),
        );
        let sid = mgr
            .create(intent_multi(&devices))
            .expect("多资源 create 应通过");
        mgr.start(&sid).expect("多资源 start 应成功");
        assert!(mgr.stop(&sid).is_err(), "stop 注入失败应上报");
        let s = mgr.status(&sid).expect("会话保留");
        assert_eq!(s.state, SessionState::Released);
        assert!(s.leases.is_empty(), "stop 失败后租约必须已归还");
        mgr.resources.with_inner(|reg| {
            for r in &reg.resources {
                assert_eq!(
                    r.state,
                    crate::resource::ResourceState::Available,
                    "多资源全部归还"
                );
            }
        });
        assert_zero_orphans(&mgr, &lm);
    }

    #[test]
    fn session_rt_01_materialize_failure_rolls_back_everything() {
        // Round 3 P0: materialize 失败 (intent 引用未注册设备 → IdentityUnresolved) →
        // Starting 相位不得遗留 lease/reservation — 全部回滚 + StartFailed (会话保留供诊断)。
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let mgr = mock_manager(&devices, lm.clone());
        let sid = mgr.create(intent_for(&devices[0])).expect("create 应通过");
        assert!(!lm.list_active().is_empty(), "前置: create 已持有租约");
        // 人为把会话图换成引用未注册设备的 intent (绕过 create 的 Preflight,
        // 直接驱动 start 的 materialize 失败分支 — 真实失败可能: identity/binding/设备消失)。
        let mut ghost = devices[0].clone();
        ghost.device_id = Uuid::new_v4();
        mgr.sessions
            .lock()
            .unwrap()
            .get_mut(&sid)
            .unwrap()
            .session
            .graphs = intent_for(&ghost);
        let err = mgr.start(&sid).expect_err("materialize 失败应传播");
        assert!(matches!(err, SessionError::Pipeline(_)));
        // 零孤儿断言: Starting 相位不得遗留 lease/reservation。
        assert!(lm.list_active().is_empty(), "lease 必须已回滚");
        mgr.resources.with_inner(|reg| {
            for r in &reg.resources {
                assert_eq!(
                    r.state,
                    crate::resource::ResourceState::Available,
                    "reservation 必须已回滚"
                );
            }
        });
        let s = mgr.status(&sid).expect("失败会话保留供诊断");
        assert_eq!(s.phase, SessionPhase::StartFailed);
        assert!(s.pipeline.is_none());
        assert!(s.leases.is_empty());
        mgr.close(&sid).expect("失败终态 close 应成功");
    }

    /// P0-7D-4.2 (Simulation): D3 全链接线 + 新旧路径终态等价 —
    /// 真实 SessionManager 事件流 (非手工事件) 经 FanoutSink 双日志:
    /// internal drain → reduce 折叠出的观测态等价于原命令式写点语义
    /// (start 后 Capturing = 原 main.rs:1233/1488; 全释放后 Ready = 原 1274);
    /// projection drain → kind_counts 精确计数 (IdentityResolved 点亮 = 绑定验证成功点)。
    #[test]
    fn evt_int_rt_01_real_lifecycle_events_drive_agent_state_via_fanout() {
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let resources = SharedResourceRegistry::new(ResourceRegistry::derive_from_discovery(
            &port_registry_for_devices(&devices),
        ));
        let projection = Arc::new(crate::events::RuntimeEventLog::new());
        let internal = Arc::new(crate::events::RuntimeEventLog::new());
        let sink: Arc<dyn crate::events::RuntimeEventSink> = Arc::new(
            crate::events::FanoutSink::new(projection.clone(), internal.clone()),
        );
        let sup = Arc::new(Mutex::new(Supervisor::new(
            crate::supervisor::RestartPolicy::default(),
            sink.clone(),
        )));
        // 生产级绑定 (D5: High + DeviceHandleExact) — 使 binding verify 步点亮 IdentityResolved。
        let bindings: HashMap<Uuid, crate::resolver::ResolvedDeviceBinding> = devices
            .iter()
            .map(|d| {
                (
                    d.device_id,
                    crate::resolver::ResolvedDeviceBinding {
                        device_number: 0,
                        hw_serial_number: None,
                        persistent_id: None,
                        confidence: crate::resolver::Confidence::High,
                        match_kind: crate::resolver::ResolverMatch::DeviceHandleExact,
                    },
                )
            })
            .collect();
        let mgr = SessionManager::new(
            resources,
            lm.clone(),
            sup,
            Arc::new(MockBackend),
            Arc::new(devices.clone()),
            Arc::new(bindings),
            None,
            MaterializeMode::Diagnostic,
            SessionTuning::default(),
            sink,
        );
        let sid = mgr.create(intent_for(&devices[0])).expect("create 应通过");
        mgr.start(&sid).expect("start 应成功");

        // 事件内消费: 真实事件流折叠 — start 后 Capturing + 活跃会话 1 (等价旧命令式)。
        let drained = internal.drain();
        let fold = crate::health::reduce(
            &crate::health::HealthFold::bootstrap(crate::health::AgentState::Ready),
            &drained,
        );
        assert_eq!(
            fold.agent,
            crate::health::AgentState::Capturing,
            "start 后应等价旧 1233/1488 命令式 Capturing"
        );
        assert_eq!(fold.active_sessions, 1);

        // 外送侧独立可观测: kind_counts 精确计数 (identity_resolved = 意图设备数 1;
        // 无故障类事件 — has_critical=false)。
        let proj = crate::event_projection::project(&projection.drain());
        assert_eq!(proj.kind_counts.get("identity_resolved"), Some(&1));
        assert_eq!(proj.kind_counts.get("session_created"), Some(&1));
        assert!(!proj.has_critical);

        // 释放: 折叠态回到 Ready (等价旧 1274 命令式), 活跃会话归零。
        mgr.stop(&sid).expect("stop 应成功");
        let fold2 = crate::health::reduce(&fold, &internal.drain());
        assert_eq!(fold2.active_sessions, 0);
        assert_eq!(
            fold2.agent,
            crate::health::AgentState::Ready,
            "全释放后应等价旧 1274 命令式 Ready"
        );
        mgr.close(&sid).expect("close 应成功");
        assert!(mgr.status(&sid).is_none());
    }

    /// P0-7D-4.2 (Simulation): ResourceReservationExpired 点亮 —
    /// 预留过期 (tick 驱动) 精确计数 1 + reducer 集成 (资源面运维可见降级 → Degraded)。
    #[test]
    fn evt_int_rt_01_reservation_expiry_emits_event_and_derives_degraded() {
        let devices = mock_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let event_log = Arc::new(crate::events::RuntimeEventLog::new());
        let resources = SharedResourceRegistry::new(ResourceRegistry::derive_from_discovery(
            &port_registry_for_devices(&devices),
        ));
        let sup = Arc::new(Mutex::new(Supervisor::new(
            crate::supervisor::RestartPolicy::default(),
            event_log.clone(),
        )));
        let tuning = SessionTuning {
            reservation_window_ms: 0,
            ..SessionTuning::default()
        };
        let mgr = SessionManager::new(
            resources,
            lm.clone(),
            sup,
            Arc::new(MockBackend),
            Arc::new(devices.clone()),
            Arc::new(HashMap::new()),
            None,
            MaterializeMode::Diagnostic,
            tuning,
            event_log.clone(),
        );
        let sid = mgr.create(intent_for(&devices[0])).expect("create 应通过");
        std::thread::sleep(std::time::Duration::from_millis(10));
        mgr.tick();
        assert_eq!(
            mgr.status(&sid).expect("会话保留").phase,
            SessionPhase::Terminated
        );

        let drained = event_log.drain();
        let proj = crate::event_projection::project(&drained);
        // 单设备意图 × 1 端口 = 1 份 claim → 恰 1 条过期事件 (无噪声)。
        assert_eq!(
            proj.kind_counts.get("resource_reservation_expired"),
            Some(&1)
        );
        // reducer 集成: 资源面降级可观测 (Degraded), 活跃会话未被偷释放 (仍 1,
        // Terminated 非 Released — 会话平面释放语义不由资源过期伪造)。
        let fold = crate::health::reduce(
            &crate::health::HealthFold::bootstrap(crate::health::AgentState::Ready),
            &drained,
        );
        assert_eq!(fold.agent, crate::health::AgentState::Degraded);
        assert_eq!(fold.active_sessions, 1);
    }
}
