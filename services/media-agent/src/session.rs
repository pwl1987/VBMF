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
use crate::events::RuntimeEvent;
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
    pub pipeline: Option<PipelineHandle>,
    pub health: SessionHealthSnapshot,
    pub created_at: i64,
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
        }
    }

    fn emit(&self, ev: RuntimeEvent) {
        self.sup.lock().unwrap().record(ev);
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
            let missing: Vec<Uuid> = intent
                .devices
                .iter()
                .filter_map(|d| Uuid::parse_str(&d.device_id).ok())
                .filter(|u| !self.bindings.contains_key(u))
                .collect();
            if !missing.is_empty() {
                let _ = self.set_phase(session_id, SessionPhase::BindingFailed);
                return Err(SessionError::InvalidTransition(format!(
                    "目标设备缺少生产绑定: {missing:?}"
                )));
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

        // 步 1: materialize (纯函数; 失败无副作用)。
        let plans = crate::pipeline::materialize(
            &intent,
            &self.devices,
            mode,
            &self.bindings,
            self.registry.as_ref(),
        )?;
        let plan = plans
            .first()
            .ok_or_else(|| SessionError::InvalidTransition("materialize 产出空计划".into()))?;
        self.emit(RuntimeEvent::SourceMaterialized {
            device_id: intent
                .devices
                .first()
                .and_then(|d| Uuid::parse_str(&d.device_id).ok())
                .unwrap_or(Uuid::nil()),
            pipeline: Uuid::new_v5(&Uuid::nil(), format!("{:?}", plan).as_bytes()),
        });

        // 步 2: Backend.instantiate (失败 → 逆序回滚 lease+reservation, 零孤儿)。
        let handle = match backend.instantiate(plan) {
            Ok(h) => h,
            Err(e) => {
                self.rollback_lease_and_reservation(id, &holder);
                let _ = self.set_phase(id, SessionPhase::StartFailed);
                self.emit(RuntimeEvent::SessionFailed {
                    session_id: id.0,
                    reason: format!("backend.instantiate: {e}"),
                });
                return Err(e.into());
            }
        };

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
            let _ = backend.stop(&handle);
            self.rollback_to_pre_start(id, &holder);
            return Err(e.into());
        }

        // 步 4: Backend.start。失败 → 逆序: stop → release allocation → lease/reservation。
        if let Err(e) = backend.start(&handle) {
            let _ = backend.stop(&handle);
            self.resources.release_allocation(holder);
            self.rollback_lease_and_reservation(id, &holder);
            let _ = self.set_phase(id, SessionPhase::StartFailed);
            self.emit(RuntimeEvent::SessionFailed {
                session_id: id.0,
                reason: format!("backend.start: {e}"),
            });
            return Err(e.into());
        }

        // 成功: RUNNING。
        // P0-2: 粗态/相位经白名单守卫迁移 (Starting → Running)。
        self.set_phase(id, SessionPhase::Running)?;
        let mut guard = self.sessions.lock().unwrap();
        let inner = guard.get_mut(id).expect("session registered");
        inner.session.pipeline = Some(handle);
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
        let (handle, holder) = {
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
            (inner.session.pipeline, inner.holder)
        };
        self.set_phase(id, SessionPhase::Stopping)?;

        // 逆序 1: Backend.stop (有句柄才调)。
        if let Some(h) = &handle {
            self.backend()?.stop(h)?;
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
                    self.resources.expire_reservations_of(holder);
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
        let sup = Arc::new(Mutex::new(Supervisor::new(
            crate::supervisor::RestartPolicy::default(),
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
        )
    }

    fn mock_manager(devices: &[DeviceInfo], lm: Arc<dyn LmTrait>) -> SessionManager {
        manager_with(Arc::new(MockBackend), devices, lm, SessionTuning::default())
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
        Start,
    }

    struct FailingBackend {
        fail_at: FailAt,
        stop_called: AtomicBool,
        instances: AtomicU64,
    }

    impl FailingBackend {
        fn new(fail_at: FailAt) -> Arc<Self> {
            Arc::new(Self {
                fail_at,
                stop_called: AtomicBool::new(false),
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
}
