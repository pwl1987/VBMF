//! Phase 0.6 C2 (0.6E): Resource 模型 + Preflight 闸门.
//!
//! `Resource` 是对 `Capability` 的抽象 (≠ Device/Port): 一个 Device 可暴露多个 Resource
//! (例如某 SDI 输入端口 = 1 个 `sdi-input` Resource)。状态机对齐 V0.2 §3.11:
//! `Available → Reserved → Allocated → (Releasing | Faulted)`。
//!
//! **Preflight (防自动 Fallback)**: `materialize` 前校验目标 Resource 可用 + 无冲突预留 +
//! 身份已 Resolve; 失败返回 `ResourceUnavailable` / `AmbiguousIdentity`, 交由上层 Policy 决策,
//! **绝不静默回退到 device 0 或另一硬件**。
//!
//! 设计约束 (对齐 port.rs HARD RULE):
//! - `device-number` 绝不默认 0; 身份歧义 (多重候选) 一律拒识。
//! - Resource 由 Discovery (`PortRegistry` / `DeviceCapabilities`) 派生, 不硬编码拓扑。
//!
//! `#![allow(dead_code)]`: 本模块是 0.6E Resource/Preflight SPI — 状态机全部迁移与
//! `resolve_identity` / `ResourceRegistry::new` 等由后续 change (0.6H/I, Control Plane) 消费;
//! 当前 binary 仅 Preflight 闸门使用其中一部分。与 supervisor.rs 同款处理, 待接线完成后可收窄。
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::port::PortRegistry;
use crate::source::{NetworkSourceId, ResourceOwner};

/// 资源状态机 (V0.2 §3.11)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceState {
    /// 空闲, 可被预留。
    Available,
    /// 已预留 (某 session 持有 reservation, 尚未物化)。
    Reserved,
    /// 已分配 (管线已物化并占用该资源)。
    Allocated,
    /// 释放中 (分配撤销, 正在归还)。
    Releasing,
    /// 故障 (需运维介入; 仅可手动恢复到 Available)。
    Faulted,
}

impl ResourceState {
    /// 合法迁移判定 (状态机白名单; 其余一律拒绝, fail-closed)。
    pub fn can_transition_to(self, to: ResourceState) -> bool {
        use ResourceState::*;
        matches!(
            (self, to),
            (Available, Reserved)
                | (Available, Faulted)
                | (Reserved, Allocated)
                | (Reserved, Available)
                | (Allocated, Releasing)
                | (Allocated, Faulted)
                | (Releasing, Available)
                | (Faulted, Available)
        )
    }
    /// 是否处于"可被 Preflight 占用"的状态。
    pub fn is_available_for_preflight(self) -> bool {
        self == ResourceState::Available
    }
}

/// 预留 (reservation): 某 session 在物化前持有的占用凭据。
///
/// RH-RES-01A: TTL truth 与 claim 同驻 Resource Registry；`expires_at_ms` 为
/// Runtime 内部生命周期数据，不扩展既有序列化/wire vocabulary。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reservation {
    /// 持有者 (session/plan) 的 canonical ID。
    pub holder: Uuid,
    /// 预留凭据 (诊断用; 非身份)。
    pub token: String,
    /// 该 claim 的绝对过期时刻（Unix epoch ms）；仅 Registry 生命周期使用。
    #[serde(skip)]
    pub(crate) expires_at_ms: Option<u64>,
}

/// 资源 (Capability 的抽象; 一个 Device 可暴露多个 Resource)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource {
    pub id: Uuid,
    /// canonical 资源名 (如 "sdi-input-<port_id>")。
    pub name: String,
    /// canonical 能力键 (如 "sdi-input" / "encode-slot")。
    pub capability: String,
    /// 可并发分配数 (端口类资源通常为 1)。
    pub capacity: u32,
    /// 所属设备 ID 的兼容投影；Network 资源不使用该字段。
    pub device_id: Uuid,
    /// 资源的权威归属域；Network 资源不得伪装成 Device。
    #[serde(default)]
    pub owner: ResourceOwner,
    pub state: ResourceState,
    /// 当前预留 (仅 `Reserved` 态存在)。
    pub reservation: Option<Reservation>,
    /// 当前分配持有者 (仅 `Allocated` 态存在; session/plan ID)。
    pub allocated_to: Option<Uuid>,
}

/// 资源状态迁移错误 (非法迁移, fail-closed)。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("illegal resource state transition {from:?} -> {to:?}")]
pub struct ResourceStateError {
    pub from: ResourceState,
    pub to: ResourceState,
}

impl Resource {
    /// 构造一个 `Available` 资源。
    ///
    /// 旧 Device 资源保留 `device_id` 兼容投影；正式归属由 `owner` 表达。
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        capability: impl Into<String>,
        capacity: u32,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            capability: capability.into(),
            capacity: capacity.max(1),
            device_id: Uuid::nil(),
            owner: ResourceOwner::Device(Uuid::nil()),
            state: ResourceState::Available,
            reservation: None,
            allocated_to: None,
        }
    }

    /// 构造一个 Network 资源；它没有硬件 DeviceId。
    pub fn network(
        id: Uuid,
        name: impl Into<String>,
        capability: impl Into<String>,
        source_id: NetworkSourceId,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            capability: capability.into(),
            capacity: 1,
            device_id: Uuid::nil(),
            owner: ResourceOwner::Network(source_id),
            state: ResourceState::Available,
            reservation: None,
            allocated_to: None,
        }
    }

    /// 给 Discovery 派生的 Device 资源设置权威归属。
    pub fn set_device_owner(&mut self, device_id: Uuid) {
        self.device_id = device_id;
        self.owner = ResourceOwner::Device(device_id);
    }

    /// 执行状态迁移 (白名单校验); 非法迁移返回 `ResourceStateError`。
    pub fn transition(&mut self, to: ResourceState) -> Result<(), ResourceStateError> {
        if !self.state.can_transition_to(to) {
            return Err(ResourceStateError {
                from: self.state,
                to,
            });
        }
        self.state = to;
        Ok(())
    }
    /// Reserve: Available → Reserved；TTL 绑定到**本 claim**，由 Registry 持有 truth。
    pub fn reserve(
        &mut self,
        holder: Uuid,
        token: impl Into<String>,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<(), ResourceStateError> {
        self.transition(ResourceState::Reserved)?;
        self.reservation = Some(Reservation {
            holder,
            token: token.into(),
            expires_at_ms: Some(now_ms.saturating_add(ttl_ms)),
        });
        Ok(())
    }

    /// Renew: 仅当前 Reserved holder 可续期；错 holder/非 Reserved fail-closed 且零修改。
    pub fn renew_reservation(&mut self, holder: Uuid, now_ms: u64, ttl_ms: u64) -> bool {
        if self.state != ResourceState::Reserved {
            return false;
        }
        let Some(reservation) = self.reservation.as_mut() else {
            return false;
        };
        if reservation.holder != holder
            || reservation
                .expires_at_ms
                .is_none_or(|expires_at_ms| now_ms >= expires_at_ms)
        {
            return false;
        }
        reservation.expires_at_ms = Some(now_ms.saturating_add(ttl_ms));
        true
    }

    /// 当前 claim 是否已到 TTL；Allocated/Available 等非 Reserved 恒 false。
    pub fn reservation_due(&self, now_ms: u64) -> bool {
        self.state == ResourceState::Reserved
            && self
                .reservation
                .as_ref()
                .and_then(|r| r.expires_at_ms)
                .is_some_and(|expires_at_ms| now_ms >= expires_at_ms)
    }

    /// Abort: 仅持有者可主动撤销 Reserved claim；错 holder fail-closed。
    pub fn abort_reservation(&mut self, holder: Uuid) -> bool {
        if self.state != ResourceState::Reserved
            || self.reservation.as_ref().map(|r| r.holder) != Some(holder)
        {
            return false;
        }
        self.expire_reservation().is_ok()
    }
    /// 确认分配 (Reserved → Allocated)。
    pub fn allocate(&mut self) -> Result<(), ResourceStateError> {
        self.transition(ResourceState::Allocated)?;
        self.allocated_to = self.reservation.as_ref().map(|r| r.holder);
        self.reservation = None;
        Ok(())
    }
    /// 预留过期/取消 (Reserved → Available)。由 Supervisor/TTL 判定后调用。
    pub fn expire_reservation(&mut self) -> Result<(), ResourceStateError> {
        self.transition(ResourceState::Available)?;
        self.reservation = None;
        Ok(())
    }
    /// 释放 (Allocated → Releasing → Available 的两步之一)。
    pub fn begin_release(&mut self) -> Result<(), ResourceStateError> {
        self.transition(ResourceState::Releasing)
    }
    /// 完成释放 (Releasing → Available)。
    pub fn finish_release(&mut self) -> Result<(), ResourceStateError> {
        self.transition(ResourceState::Available)?;
        self.allocated_to = None;
        Ok(())
    }
    /// 标记故障 (→ Faulted; 仅 Available/Allocated 可进入)。
    pub fn fault(&mut self) -> Result<(), ResourceStateError> {
        self.transition(ResourceState::Faulted)
    }
    /// 运维手动恢复 (Faulted → Available)。
    pub fn recover(&mut self) -> Result<(), ResourceStateError> {
        self.transition(ResourceState::Available)?;
        self.reservation = None;
        self.allocated_to = None;
        Ok(())
    }
}

/// 资源注册表 — 当前 Runtime 派生的全部 Resource (由 Discovery 构建, 不硬编码)。
#[derive(Debug, Clone, Default)]
pub struct ResourceRegistry {
    pub resources: Vec<Resource>,
}

/// 端口 → 资源命名空间（UUIDv5 over `vbmf:resource:{port_id}`）。
///
/// `derive_from_discovery` 与消费侧一致性校验（A2-8 Gate L1d, 第十九轮 §十六 H2）
/// 共用的**单一派生来源**——防消费侧复制公式漂移成第二 SoT。
fn port_resource_ns(port_id: Uuid) -> Uuid {
    Uuid::new_v5(
        &Uuid::from_u128(0x6d5f_5206_4a1c_4b9e_8f0a_1b2c_3d4e_5f60),
        format!("vbmf:resource:{port_id}").as_bytes(),
    )
}

/// 端口的 Input 资源 ID（`<connector>-input` 平面的规范派生）。
///
/// A2-8 Gate L1d 以它核验"唯一 Input Resource 的 ID == Manifest Input Port
/// 的规范派生"——闭合 Manifest Port → Registry Port → Resource → Session 证据链。
pub fn input_resource_id_for_port(port_id: Uuid) -> Uuid {
    Uuid::new_v5(&port_resource_ns(port_id), "input".as_bytes())
}

/// Stable Resource ID for a configured network source.
pub fn network_resource_id_for_source(source_id: NetworkSourceId) -> Uuid {
    Uuid::new_v5(&source_id.0, b"vbmf:resource:network-input")
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one network-source Resource in the same registry/state machine
    /// used by hardware inputs. Re-registering the same source is idempotent.
    pub fn register_network_source(&mut self, source_id: NetworkSourceId) -> Uuid {
        let resource_id = network_resource_id_for_source(source_id);
        if self.resources.iter().any(|r| r.id == resource_id) {
            return resource_id;
        }
        self.resources.push(Resource::network(
            resource_id,
            format!("rtmp-input-{source_id}"),
            "rtmp-input",
            source_id,
        ));
        resource_id
    }

    /// 按 ID 查找 (可变)。
    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut Resource> {
        self.resources.iter_mut().find(|r| &r.id == id)
    }
    /// 确认分配 (Reserved → Allocated; P0-7A 编排): 仅预留持有者本人可推进
    /// (reservation.holder 必须与 holder 一致, 防越权 allocate 他人预留)。
    /// 调用时机: Backend.instantiate 成功后 (RUNTIME_LIFECYCLE_SEQUENCE §1)。
    pub fn allocate_for(
        &mut self,
        resource_id: &Uuid,
        holder: Uuid,
    ) -> Result<(), ResourceStateError> {
        let Some(res) = self.get_mut(resource_id) else {
            return Err(ResourceStateError {
                from: ResourceState::Reserved,
                to: ResourceState::Allocated,
            });
        };
        if res.reservation.as_ref().map(|r| r.holder) != Some(holder) {
            return Err(ResourceStateError {
                from: res.state,
                to: ResourceState::Allocated,
            });
        }
        res.allocate()
    }
    /// 释放 holder 名下全部 Allocated 资源 (Allocated → Releasing → Available; stop 路径)。
    /// 返回释放数。Releasing→Available 立即完成 (无异步释放方, 与状态机白名单一致)。
    pub fn release_allocation(&mut self, holder: Uuid) -> usize {
        let mut released = 0;
        for res in self.resources.iter_mut() {
            if res.state == ResourceState::Allocated
                && res.allocated_to == Some(holder)
                && res.begin_release().is_ok()
                && res.finish_release().is_ok()
            {
                released += 1;
            }
        }
        released
    }
    /// 由 `PortRegistry` 派生: 每个具有稳定 `port_id` 的端口生成 1 个能力 Resource。
    ///
    /// 仅 `port_id = Some` 的端口可派生 (与 `PortIdentity::derive` 的"Unknown 不伪造 ID"一致);
    /// 能力键按方向映射 (Input→`<connector>-input`, Output→`<connector>-output`, 双向各一)。
    pub fn derive_from_discovery(registry: &PortRegistry) -> Self {
        let mut out = Vec::new();
        for port in &registry.ports {
            let Some(port_id) = &port.identity.port_id else {
                continue; // Unknown ordinal → 无稳定身份 → 不派生 Resource (不得伪造)。
            };
            let cap_base = format!("{:?}", port.identity.connector).to_lowercase();
            let mk = |capability: &str, name: String, id: Uuid| {
                let mut r = Resource::new(id, name, capability, 1);
                r.set_device_owner(port.device_id); // Discovery 派生 → 记录所属设备 (供 Preflight 按设备校验)
                r
            };
            if port.direction == crate::port::PortDirection::Input
                || port.direction == crate::port::PortDirection::Bidirectional
            {
                out.push(mk(
                    &format!("{cap_base}-input"),
                    format!("{cap_base}-input-{port_id}"),
                    input_resource_id_for_port(*port_id),
                ));
            }
            if port.direction == crate::port::PortDirection::Output
                || port.direction == crate::port::PortDirection::Bidirectional
            {
                out.push(mk(
                    &format!("{cap_base}-output"),
                    format!("{cap_base}-output-{port_id}"),
                    Uuid::new_v5(&port_resource_ns(*port_id), "output".as_bytes()),
                ));
            }
        }
        Self { resources: out }
    }
}

/// Preflight 结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightOutcome {
    /// 通过校验、允许物化的资源 ID 集。
    pub granted: Vec<Uuid>,
}

/// Preflight 失败原因 (fail-closed, 绝不静默回退)。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PreflightError {
    /// 目标资源不存在 (Discovery 未派生 / 身份未 Resolve)。
    #[error("resource unavailable: {0}")]
    ResourceUnavailable(String),
    /// 目标资源当前不可用 (已被占用 / 故障 / 容量耗尽)。
    #[error("resource not acquirable: {0}")]
    NotAcquirable(String),
    /// 身份歧义 (多重候选 → 拒识, 交由 Policy; 绝不静默择一)。
    #[error("ambiguous identity: {candidates:?} candidate(s)")]
    AmbiguousIdentity { candidates: Vec<String> },
}

/// 物化请求 (Preflight 输入): 需要占用的能力 + 目标资源 + 持有者。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquisitionRequest {
    /// 请求方 (session/plan) canonical ID。
    pub holder: Uuid,
    /// 目标资源 ID (由 Resolver/Plan 解析得到; 绝不默认 device 0)。
    pub resource_id: Uuid,
    /// 期望能力键 (与 Resource.capability 交叉校验, 防能力错配)。
    pub expected_capability: String,
}

/// Preflight 闸门 (防自动 Fallback)。
///
/// 在 `materialize` 前调用: 校验目标 Resource 存在 + 能力匹配 + 可用 + 无冲突预留。
/// 失败返回 `PreflightError` (由上层 Policy 决策); **绝不**静默换硬件/换资源。
pub fn preflight(
    registry: &ResourceRegistry,
    req: &AcquisitionRequest,
) -> Result<PreflightOutcome, PreflightError> {
    let Some(res) = registry.resources.iter().find(|r| r.id == req.resource_id) else {
        return Err(PreflightError::ResourceUnavailable(format!(
            "resource {} not in discovery",
            req.resource_id
        )));
    };
    // 能力交叉校验: 请求能力必须与资源能力一致 (防把 encode-slot 当 sdi-input 用)。
    if res.capability != req.expected_capability {
        return Err(PreflightError::ResourceUnavailable(format!(
            "capability mismatch: requested {} but resource is {}",
            req.expected_capability, res.capability
        )));
    }
    // 可用性: 仅 Available 可被新占用; Reserved/Allocated/Releasing/Faulted 一律拒 (绝不抢占/回退)。
    if !res.state.is_available_for_preflight() {
        return Err(PreflightError::NotAcquirable(format!(
            "resource {} is {:?}",
            res.name, res.state
        )));
    }
    // 容量校验 (端口类 capacity=1; 已可用即未超容量, 此处保留显式检查以防 capacity 语义扩展)。
    if res.capacity < 1 {
        return Err(PreflightError::NotAcquirable(format!(
            "resource {} has no capacity",
            res.name
        )));
    }
    Ok(PreflightOutcome {
        granted: vec![res.id],
    })
}

/// 身份歧义判定 (多重候选 → 拒识)。
///
/// `candidates` 为已解析到的候选身份描述; 空集 → `ResourceUnavailable` (无候选),
/// 单候选 → 通过 (返回其 ID), 多重 → `AmbiguousIdentity` (拒识, 交 Policy)。
pub fn resolve_identity(
    chosen: Option<Uuid>,
    candidates: Vec<String>,
) -> Result<Uuid, PreflightError> {
    match (chosen, candidates.len()) {
        (Some(id), 0 | 1) => Ok(id),
        (None, 1) => Err(PreflightError::ResourceUnavailable(
            "identity not resolved: single candidate present but no choice made".into(),
        )),
        _ if candidates.is_empty() => Err(PreflightError::ResourceUnavailable(
            "identity not resolved: no candidates".into(),
        )),
        _ => Err(PreflightError::AmbiguousIdentity { candidates }),
    }
}

/// 线程安全注册表句柄 + **原子占用原语** (P1-4: preflight+reserve 在同一锁内完成,
/// 消除 "A preflight → B preflight → A reserve → B reserve" 竞态窗口)。
///
/// RH-RES-01A 起，Reservation TTL/Renew/Expire/Abort 也由本 Registry 锁域持有；
/// Scheduler / Placement / 全局策略仍不在此层。本原语保证 claim 生命周期原子性，
/// 不产生第二套资源 truth。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExpiredReservation {
    pub resource_id: Uuid,
    pub holder: Uuid,
}

#[derive(Debug, Default, Clone)]
pub struct SharedResourceRegistry(std::sync::Arc<std::sync::Mutex<ResourceRegistry>>);

impl SharedResourceRegistry {
    /// 包装既有注册表 (通常为 discovery 派生结果)。
    pub fn new(inner: ResourceRegistry) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(inner)))
    }
    /// Reserve: 锁内 `preflight + reserve` 原子执行；每个 claim 必须显式带 TTL。
    pub fn acquire(
        &self,
        req: &AcquisitionRequest,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<PreflightOutcome, PreflightError> {
        let mut g = self.0.lock().unwrap();
        let out = preflight(&g, req)?;
        if let Some(res) = g.get_mut(&req.resource_id) {
            res.reserve(
                req.holder,
                format!("preflight-{}", req.holder),
                now_ms,
                ttl_ms,
            )
            .map_err(|e| {
                PreflightError::NotAcquirable(format!(
                    "resource {} reserve failed after preflight: {e:?}",
                    req.resource_id
                ))
            })?;
        }
        Ok(out)
    }

    /// Renew: 对 Session 声明的 expected claims 做同锁域全量校验；任一 claim
    /// 缺失/错 holder/已到 TTL，则零修改 fail-closed；全部通过后才统一续期。
    pub fn renew_claims(
        &self,
        holder: Uuid,
        resource_ids: &[Uuid],
        now_ms: u64,
        ttl_ms: u64,
    ) -> bool {
        let mut g = self.0.lock().unwrap();
        for resource_id in resource_ids {
            let Some(res) = g.resources.iter().find(|r| r.id == *resource_id) else {
                return false;
            };
            if res.state != ResourceState::Reserved
                || res.reservation.as_ref().map(|r| r.holder) != Some(holder)
                || res.reservation_due(now_ms)
            {
                return false;
            }
        }
        for resource_id in resource_ids {
            let res = g
                .get_mut(resource_id)
                .expect("renew validation 已确认 resource 存在");
            if !res.renew_reservation(holder, now_ms, ttl_ms) {
                return false;
            }
        }
        true
    }

    /// Abort: 主动撤销 holder 名下全部仍处 Reserved 的 claims；返回实际撤销资源 ID。
    pub fn abort_reservations_of(&self, holder: Uuid) -> Vec<Uuid> {
        let mut g = self.0.lock().unwrap();
        let mut aborted = Vec::new();
        for res in g.resources.iter_mut() {
            if res.abort_reservation(holder) {
                aborted.push(res.id);
            }
        }
        aborted
    }

    /// Expire: 只扫描**各 claim 自己**的 TTL；返回 expired claim + holder，供 Session
    /// 编排层做租约/Session 收敛。Registry 是唯一 Resource state mutation owner。
    pub(crate) fn expire_due(&self, now_ms: u64) -> Vec<ExpiredReservation> {
        let mut g = self.0.lock().unwrap();
        let mut expired = Vec::new();
        for res in g.resources.iter_mut() {
            if !res.reservation_due(now_ms) {
                continue;
            }
            let Some(holder) = res.reservation.as_ref().map(|r| r.holder) else {
                continue;
            };
            if res.expire_reservation().is_ok() {
                expired.push(ExpiredReservation {
                    resource_id: res.id,
                    holder,
                });
            }
        }
        expired
    }
    /// 确认分配 (Reserved → Allocated; Backend.instantiate 成功后调用, P0-7A 编排)。
    pub fn allocate_for(&self, resource_id: &Uuid, holder: Uuid) -> Result<(), ResourceStateError> {
        self.0.lock().unwrap().allocate_for(resource_id, holder)
    }
    /// 释放 holder 名下全部 Allocated 资源 (stop 路径); 返回释放数。
    pub fn release_allocation(&self, holder: Uuid) -> usize {
        self.0.lock().unwrap().release_allocation(holder)
    }
    /// 只读快照访问 (诊断/证据)。
    pub fn with_inner<R>(&self, f: impl for<'a> FnOnce(&'a ResourceRegistry) -> R) -> R {
        let g = self.0.lock().unwrap();
        f(&g)
    }
    /// 可变快照访问 (测试收尾/编排层状态修正; SessionManager 生命周期路径外使用须审慎)。
    pub fn with_inner_mut<R>(&self, f: impl for<'a> FnOnce(&'a mut ResourceRegistry) -> R) -> R {
        let mut g = self.0.lock().unwrap();
        f(&mut g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res() -> Resource {
        Resource::new(Uuid::nil(), "r", "sdi-input", 1)
    }

    #[test]
    fn state_machine_allows_white_listed_transitions() {
        let mut r = res();
        r.reserve(Uuid::nil(), "tok", 0, 1_000).unwrap();
        assert_eq!(r.state, ResourceState::Reserved);
        r.allocate().unwrap();
        assert_eq!(r.state, ResourceState::Allocated);
        assert_eq!(r.allocated_to, Some(Uuid::nil()));
        r.begin_release().unwrap();
        r.finish_release().unwrap();
        assert_eq!(r.state, ResourceState::Available);
    }

    #[test]
    fn state_machine_rejects_illegal_transitions() {
        let mut r = res();
        // Available 不能直接到 Allocated (必须经 Reserved)。
        assert!(matches!(
            r.transition(ResourceState::Allocated),
            Err(ResourceStateError { .. })
        ));
        // Releasing 不能到 Faulted。
        let mut r2 = res();
        r2.reserve(Uuid::nil(), "t", 0, 1_000).unwrap();
        r2.allocate().unwrap();
        r2.begin_release().unwrap();
        assert!(!r2.state.can_transition_to(ResourceState::Faulted));
    }

    #[test]
    fn fault_then_manual_recover() {
        let mut r = res();
        r.reserve(Uuid::nil(), "t", 0, 1_000).unwrap();
        r.allocate().unwrap();
        r.fault().unwrap();
        assert_eq!(r.state, ResourceState::Faulted);
        // Faulted 只能手动恢复到 Available。
        assert!(!r.state.can_transition_to(ResourceState::Reserved));
        r.recover().unwrap();
        assert_eq!(r.state, ResourceState::Available);
        assert!(r.allocated_to.is_none());
    }

    #[test]
    fn derive_from_discovery_skips_unknown_ordinal() {
        // 无稳定 port_id 的端口不派生 Resource (不得伪造身份)。
        let reg = PortRegistry::default(); // 空
        let rr = ResourceRegistry::derive_from_discovery(&reg);
        assert!(rr.resources.is_empty());
    }

    #[test]
    fn network_resource_has_typed_owner_and_shared_state_machine() {
        let source_id = NetworkSourceId(Uuid::new_v4());
        let mut rr = ResourceRegistry::new();
        let resource_id = rr.register_network_source(source_id);
        assert_eq!(rr.register_network_source(source_id), resource_id);
        let resource = rr.resources.iter().find(|r| r.id == resource_id).unwrap();
        assert_eq!(resource.owner, ResourceOwner::Network(source_id));
        assert_eq!(resource.device_id, Uuid::nil());
        assert_eq!(resource.capability, "rtmp-input");

        let req = AcquisitionRequest {
            holder: Uuid::nil(),
            resource_id,
            expected_capability: "rtmp-input".into(),
        };
        preflight(&rr, &req).expect("network resource should use normal preflight");
    }

    #[test]
    fn preflight_grants_available_matching_resource() {
        let mut rr = ResourceRegistry::new();
        let id = Uuid::new_v4();
        rr.resources.push(Resource::new(id, "r", "sdi-input", 1));
        let req = AcquisitionRequest {
            holder: Uuid::nil(),
            resource_id: id,
            expected_capability: "sdi-input".into(),
        };
        let out = preflight(&rr, &req).expect("available 应通过");
        assert_eq!(out.granted, vec![id]);
    }

    #[test]
    fn preflight_rejects_missing_mismatched_or_busy() {
        let mut rr = ResourceRegistry::new();
        let id = Uuid::new_v4();
        rr.resources.push(Resource::new(id, "r", "sdi-input", 1));
        // 不存在。
        assert!(matches!(
            preflight(
                &rr,
                &AcquisitionRequest {
                    holder: Uuid::nil(),
                    resource_id: Uuid::new_v4(),
                    expected_capability: "sdi-input".into()
                }
            ),
            Err(PreflightError::ResourceUnavailable(_))
        ));
        // 能力错配。
        assert!(matches!(
            preflight(
                &rr,
                &AcquisitionRequest {
                    holder: Uuid::nil(),
                    resource_id: id,
                    expected_capability: "hdmi-input".into()
                }
            ),
            Err(PreflightError::ResourceUnavailable(_))
        ));
        // 已被占用 (Reserved) → 不抢占。
        rr.resources[0]
            .reserve(Uuid::new_v4(), "t", 0, 1_000)
            .unwrap();
        assert!(matches!(
            preflight(
                &rr,
                &AcquisitionRequest {
                    holder: Uuid::nil(),
                    resource_id: id,
                    expected_capability: "sdi-input".into()
                }
            ),
            Err(PreflightError::NotAcquirable(_))
        ));
    }

    #[test]
    fn resource_01_faulted_resource_rejects_without_fallback() {
        // RESOURCE-01: Faulted 资源不可获取 (NotAcquirable) — materialize 闸门据此拒绝,
        // 绝不静默回退到其他资源或盲开 device 0; Releasing 态同样不可被新占用 (不抢占).
        let mut rr = ResourceRegistry::new();
        let id = Uuid::new_v4();
        rr.resources.push(Resource::new(id, "r", "sdi-input", 1));
        rr.resources[0]
            .reserve(Uuid::new_v4(), "t", 0, 1_000)
            .unwrap();
        rr.resources[0].allocate().unwrap();
        rr.resources[0].fault().unwrap();
        assert_eq!(rr.resources[0].state, ResourceState::Faulted);
        assert!(matches!(
            preflight(
                &rr,
                &AcquisitionRequest {
                    holder: Uuid::new_v4(),
                    resource_id: id,
                    expected_capability: "sdi-input".into()
                }
            ),
            Err(PreflightError::NotAcquirable(_))
        ));
        // Releasing 态不可被新占用 (不抢占在途释放).
        let mut r2 = Resource::new(Uuid::nil(), "r2", "sdi-input", 1);
        r2.reserve(Uuid::new_v4(), "t", 0, 1_000).unwrap();
        r2.allocate().unwrap();
        r2.begin_release().unwrap();
        assert_eq!(r2.state, ResourceState::Releasing);
        let mut rr2 = ResourceRegistry::new();
        rr2.resources.push(r2);
        assert!(matches!(
            preflight(
                &rr2,
                &AcquisitionRequest {
                    holder: Uuid::new_v4(),
                    resource_id: rr2.resources[0].id,
                    expected_capability: "sdi-input".into()
                }
            ),
            Err(PreflightError::NotAcquirable(_))
        ));
    }

    #[test]
    fn resolve_identity_rejects_ambiguity() {
        // 单候选 + 已选择 → 通过。
        let id = Uuid::new_v4();
        assert_eq!(resolve_identity(Some(id), vec!["c1".into()]), Ok(id));
        // 无候选 → 不可用。
        assert!(matches!(
            resolve_identity(None, vec![]),
            Err(PreflightError::ResourceUnavailable(_))
        ));
        // 多重候选 → 拒识 (绝不静默择一)。
        assert!(matches!(
            resolve_identity(Some(Uuid::nil()), vec!["c1".into(), "c2".into()]),
            Err(PreflightError::AmbiguousIdentity { .. })
        ));
    }

    #[test]
    fn resource_01_atomic_acquire_and_rollback() {
        // P1-4: acquire = preflight+reserve 原子; 成功后资源即 Reserved; 回滚释放且可重占.
        let mut rr = ResourceRegistry::new();
        let id = Uuid::new_v4();
        rr.resources.push(Resource::new(id, "r", "sdi-input", 1));
        let shared = SharedResourceRegistry::new(rr);
        let holder = Uuid::new_v4();
        let out = shared
            .acquire(
                &AcquisitionRequest {
                    holder,
                    resource_id: id,
                    expected_capability: "sdi-input".into(),
                },
                100,
                1_000,
            )
            .expect("acquire 应通过并原子占用");
        assert_eq!(out.granted, vec![id]);
        // 占用生效: 同资源再次 acquire (不同 holder) 必须失败 (无竞态窗口).
        assert!(matches!(
            shared.acquire(
                &AcquisitionRequest {
                    holder: Uuid::new_v4(),
                    resource_id: id,
                    expected_capability: "sdi-input".into(),
                },
                100,
                1_000,
            ),
            Err(PreflightError::NotAcquirable(_))
        ));
        // 回滚: 释放 holder 的 Reserved; 之后可被重新 acquire.
        assert_eq!(shared.abort_reservations_of(holder).len(), 1);
        assert!(shared
            .acquire(
                &AcquisitionRequest {
                    holder: Uuid::new_v4(),
                    resource_id: id,
                    expected_capability: "sdi-input".into(),
                },
                100,
                1_000,
            )
            .is_ok());
        // 无名下预留 → 回滚 0.
        assert_eq!(shared.abort_reservations_of(holder).len(), 0);
    }

    #[test]
    fn resource_01_allocate_for_requires_matching_holder_and_release() {
        // P0-7A 编排: 仅预留持有者可 allocate; release_allocation 释放本人 Allocated。
        let mut rr = ResourceRegistry::new();
        let id = Uuid::new_v4();
        rr.resources.push(Resource::new(id, "r", "sdi-input", 1));
        let holder = Uuid::new_v4();
        rr.resources[0].reserve(holder, "tok", 0, 1_000).unwrap();
        // 越权 allocate (非预留持有者) 拒绝。
        assert!(rr.allocate_for(&id, Uuid::new_v4()).is_err());
        assert_eq!(rr.resources[0].state, ResourceState::Reserved);
        // 持有者 allocate 成功 → Allocated → release_allocation → Available。
        rr.allocate_for(&id, holder).unwrap();
        assert_eq!(rr.resources[0].state, ResourceState::Allocated);
        assert_eq!(rr.resources[0].allocated_to, Some(holder));
        assert_eq!(rr.release_allocation(holder), 1);
        assert_eq!(rr.resources[0].state, ResourceState::Available);
        assert_eq!(rr.resources[0].allocated_to, None);
    }

    #[test]
    fn resource_01_per_claim_expire_is_independent() {
        let mut rr = ResourceRegistry::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        rr.resources.push(Resource::new(a, "ra", "sdi-input", 1));
        rr.resources.push(Resource::new(b, "rb", "sdi-input", 1));
        let holder = Uuid::new_v4();
        rr.resources[0].reserve(holder, "t1", 100, 10).unwrap();
        rr.resources[1].reserve(holder, "t2", 100, 30).unwrap();
        let shared = SharedResourceRegistry::new(rr);

        let expired = shared.expire_due(110);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].resource_id, a);
        assert_eq!(expired[0].holder, holder);
        shared.with_inner(|g| {
            assert_eq!(g.resources[0].state, ResourceState::Available);
            assert_eq!(g.resources[1].state, ResourceState::Reserved);
        });
    }

    #[test]
    fn resource_01_renew_and_abort_are_holder_scoped() {
        let mut rr = ResourceRegistry::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        rr.resources.push(Resource::new(a, "ra", "sdi-input", 1));
        rr.resources.push(Resource::new(b, "rb", "sdi-input", 1));
        let h1 = Uuid::new_v4();
        let h2 = Uuid::new_v4();
        rr.resources[0].reserve(h1, "t1", 100, 10).unwrap();
        rr.resources[1].reserve(h2, "t2", 100, 10).unwrap();
        let shared = SharedResourceRegistry::new(rr);

        assert!(
            !shared.renew_claims(h2, &[a], 105, 50),
            "错 holder 不得跨 claim Renew"
        );
        assert!(shared.renew_claims(h1, &[a], 105, 50));
        assert!(
            !shared.renew_claims(h2, &[b], 110, 50),
            "TTL 已到但尚未 scan 的 claim 不得被 Renew 复活"
        );
        let expired = shared.expire_due(110);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].resource_id, b);
        assert_eq!(expired[0].holder, h2);

        assert_eq!(shared.abort_reservations_of(h1), vec![a]);
        assert!(shared.abort_reservations_of(h1).is_empty());
        shared.with_inner(|g| {
            assert_eq!(g.resources[0].state, ResourceState::Available);
            assert_eq!(g.resources[1].state, ResourceState::Available);
        });
    }

    #[test]
    fn resource_01_renew_claims_is_all_or_none_when_one_claim_expired() {
        let mut rr = ResourceRegistry::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let holder = Uuid::new_v4();
        rr.resources.push(Resource::new(a, "ra", "sdi-input", 1));
        rr.resources.push(Resource::new(b, "rb", "sdi-input", 1));
        rr.resources[0].reserve(holder, "a", 100, 10).unwrap();
        rr.resources[1].reserve(holder, "b", 100, 30).unwrap();
        let shared = SharedResourceRegistry::new(rr);

        assert!(!shared.renew_claims(holder, &[a, b], 110, 50));
        shared.with_inner(|registry| {
            assert_eq!(
                registry.resources[0]
                    .reservation
                    .as_ref()
                    .unwrap()
                    .expires_at_ms,
                Some(110)
            );
            assert_eq!(
                registry.resources[1]
                    .reservation
                    .as_ref()
                    .unwrap()
                    .expires_at_ms,
                Some(130),
                "Renew 失败不得部分延长 sibling claim"
            );
        });
    }

    #[test]
    fn resource_01_reservation_ttl_is_runtime_internal_not_wire_shape() {
        let mut r = Resource::new(Uuid::new_v4(), "r", "sdi-input", 1);
        r.reserve(Uuid::new_v4(), "tok", 100, 50).unwrap();
        let json = serde_json::to_value(&r).expect("resource serialize");
        assert!(
            json.pointer("/reservation/expires_at_ms").is_none(),
            "RH-RES-01A 不扩展既有 wire vocabulary"
        );
    }
}
