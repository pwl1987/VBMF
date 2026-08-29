---
comet_change: p07-session-runtime
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-29-p07-session-runtime
status: final
---

# Design Doc — p07-session-runtime（Phase 0.7A: Session Runtime）

> open design.md D1-D11 的实现级细化。所有语义锚定冻结契约：RUNTIME_SESSION_MODEL（§2 结构/§3 状态/§4.1 ownership/§5 #145 验收）、RUNTIME_RESOURCE_MODEL（§3 状态机/§4.1-4.2 Reservation-TTL-cleanup）、RUNTIME_LIFECYCLE_SEQUENCE（§1 顺序/§2 creator=destroyer+零孤儿）、MEDIA_BACKEND_CONTRACT §1.1（P0-8）、V0.2 §1.2（Preflight 三层）/§3.11（DEVICE_EXCLUSIVITY）、Addendum §4/§8。

## 1. `src/session.rs` — 类型

```rust
pub struct SessionId(pub Uuid);                       // Copy/PartialEq/Display; 独立于硬件身份 (模型 §2)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {          // 粗态 (模型 §3; 对外投影)
    Reserved, Running, Paused, Releasing, Released,
}
impl SessionState {
    pub fn can_transition_to(self, to: Self) -> bool {   // 白名单; RELEASED→Running 恒 false (#114)
        matches!((self, to),
            (Reserved, Running) | (Reserved, Releasing) | (Reserved, Released)
            | (Running, Paused) | (Running, Releasing)
            | (Paused, Running) | (Paused, Releasing)
            | (Releasing, Released))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {          // 微相位 (Addendum §4.3; 对内引擎步进)
    Requested, Provisioning, Binding, Leased, Starting, Running, Stopping, Released,
    ProvisioningFailed, BindingFailed, StartFailed, Degraded, Recovery, Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceClaim { pub resource_id: Uuid, pub holder: Uuid,
    pub phase: ClaimPhase, /* Reserved | Allocated */ }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSession {        // 逻辑持有、物理引用; 无任何 concrete backend 对象 (模型 §2)
    pub session_id: SessionId,
    pub state: SessionState,
    pub phase: SessionPhase,
    pub graphs: crate::graph_intent::GraphRuntimeIntent,   // canonical 语义图
    pub source_ports: Vec<Uuid>,
    pub output_ports: Vec<Uuid>,
    pub resource_claims: Vec<ResourceClaim>,
    pub lease: Option<crate::lease::DeviceLease>,
    pub pipeline: Option<crate::pipeline::PipelineHandle>, // Handle 链接 Session↔Backend 对象
    pub health: SessionHealthSnapshot,                     // {last_ok, last_error, agent_state 镜像}
    pub created_at: i64,
}
```

## 2. SessionManager — 唯一 owner（模型 §4.1 逐字）

```rust
pub struct SessionManager {
    resources: crate::resource::SharedResourceRegistry,
    leases: std::sync::Arc<dyn crate::lease::LeaseManager>,
    sup: std::sync::Arc<std::sync::Mutex<crate::supervisor::Supervisor>>,
    events: std::sync::Arc<crate::events::RuntimeEventLog>,
    cfg: SessionTuning { default_lease_ttl, lease_renew_window, resilience_window_ms },
    backend: std::sync::OnceLock<std::sync::Arc<dyn crate::contracts::backend::MediaBackend>>, // 惰性工厂
    bindings: std::sync::Arc<std::sync::Mutex<HashMap<Uuid, ResolvedDeviceBinding>>>,
    devices: std::sync::Arc<Vec<crate::device::DeviceInfo>>,   // canonical 只读快照
    registry: Option<crate::port::PortRegistry>,
    sessions: std::sync::Mutex<HashMap<SessionId, SessionInner>>, // SessionInner = MediaSession + runtime 私有槽
}
```
API：`create(intent) -> Result<SessionId, SessionError>`、`start(id) -> Result<(), SessionError>`、`stop(id) -> Result<(), SessionError>`、`close(id)`（幂等）、`status(id)/list()`、`tick()`、`backend(&self)`（惰性 `AdapterRegistry::build_media_backend`/`build_provider` 组合，默认构建用 provider-mock 组合规则）。并发：`sessions` 表 Mutex 串行化生命周期变迁；资源争用由 `SharedResourceRegistry::acquire` 原子性裁决（后到者 `NotAcquirable`，RESOURCE-RT-01）。

**create 步进表（冻结顺序，每步失败 → rollback_to(已完成步) → SessionFailed/Terminated）**：

| # | 步 | 动作 | 失败回滚 |
|---|----|------|----------|
| 0 | Requested | 生成 SessionId，登记会话（相位 Requested） | 移除表项 |
| 1 | Preflight | `preflight::run(...)` → FAIL 即 `Err`（**零预留零回滚**，LIFECYCLE §2） | 移除表项 |
| 2 | Provisioning | `resources.acquire(claim)`（原子 reserve；claim.phase=Reserved） | release_reservations(holder) |
| 3 | Session 建档 | 相位 Provisioning→登记完整 MediaSession，粗态 Reserved，`SessionCreated` | 步2 回滚 |
| 4 | Leased | `leases.acquire(device_id, owner=session_id, default_lease_ttl)` → `LeaseGranted` | 步2 回滚 + 移除表项 |
| 5 | Binding | bindings 含目标设备且 HIGH/ManifestVerified；端口存在 → `IdentityResolved` | 步4 释放 lease + 步2 |
| 6 | （instantiate 属 start） | create 返回；相位 Binding→Leased 完成，粗态 Reserved | — |

**start 步进表**：

| # | 步 | 动作 | 失败回滚（逆序） |
|---|----|------|------------------|
| 1 | Starting | materialize(intent, devices, mode, bindings, registry) → plans[0]；`SourceMaterialized` | 无副作用（materialize 纯函数）|
| 2 | Backend.instantiate | `backend().instantiate(&plan)` → `pipeline = Some(handle)` | — |
| 3 | Allocate | `resources.allocate_for(resource_id, holder)`（claim.phase=Allocated；`ResourceAllocated`） | instantiate 后 stop(handle) |
| 4 | Backend.start | `backend().start(&handle)` → 相位 Running、粗态 Running | 步3 释放 allocation → 步2 stop → 步(create) 逆序 |
| — | 任意失败 | `SessionFailed` + 相位 Terminated（Session 保留供诊断直到 close） | 每步逆序至 create 起点 |

**stop/close**：`backend().stop(handle)`（trait 方法首次真实调用）→ `resources.release_allocation(holder)`（Allocated→Releasing→Available）→ `leases.release(lease)` → `release_reservations` 兜底 → 粗态 Releasing→Released、相位 Released→`close` 移除表项。double-stop → `Err(AlreadyStopped)`（幂等 close 除外）。**creator=destroyer：除 SessionManager 外任何模块不得触碰 resource/lease/session 表**（编译层靠可见性，运行层靠代码评审）。

## 3. `src/preflight.rs` — 分级判定（judge-only）

```rust
pub enum PreflightStage { Graph, PortAvailability, ResourceCapacity, LeaseConflict,
                          IdentityBinding, BackendCapability, Topology, Risk }
pub struct StageOutcome { pub stage, pub level: Pass|Warn|Fail, pub detail: String }
pub struct PreflightReport { pub stages: Vec<StageOutcome>, pub verdict: Verdict /* Pass|Warn|Fail */ }
pub fn run(inputs: PreflightInputs) -> PreflightReport
```
0.7A 判定级：Graph（intent 解析/设备引用存在）、PortAvailability（registry 端口存在+方向匹配）、ResourceCapacity（现有 `resource::preflight` 复用）、LeaseConflict（`leases.health()` 查他人持有）、IdentityBinding（bindings 含目标且 HIGH/ManifestVerified）。WARN 级：BackendCapability（`probe_capabilities` 报告）。Report-only 占位：Topology、Risk。verdict=Fail ⇒ create 拒绝。**无任何媒体操作**（V0.2 §1.2 与 QC 解耦）。

## 4. resource.rs / lease.rs 补全

- `ResourceRegistry::allocate_for(&mut self, resource_id, holder)`：资源须 Reserved 且 `reservation.holder == holder` → `allocate()`（claim 与 state 同锁域推进由调用方保证）。
- `release_allocation(&mut self, holder)`：holder 名下 Allocated → `begin_release()+finish_release()` → Available；返回释放数。
- `expire_reservations(&mut self, cutoff)`：预留 TTL 判定入口（0.7A 由 Manager.tick 以 lease 生命周期近似；真正 per-claim TTL 记 0.7 债务）。
- `LeaseManager::renew(&self, device_id, owner, ttl) -> Result<DeviceLease, LeaseError>`（trait + InMemory 实现：NotFound/AlreadyLeased-by-other 语义）；`Config::default_lease_ttl/lease_renew_window` 接入 SessionTuning。

## 5. events.rs 补线（additive）

新增 `SessionCreated{session_id}`(Observation)、`SessionStateChanged{session_id, from: String, to: String}`(Observation)、`SessionFailed{session_id, reason: String}`(**Critical**)；`kind()`/serde tag 同步。点亮：`LeaseGranted`（create 步4）、`ResourceAllocated`（start 步3）、`ResourceReservationExpired`（tick 过期）、`IdentityResolved`（Binding 步）、`SourceMaterialized`（start 步1）。全部经 `Supervisor::record` 单出口（§8）。既有 11 变体与 serde 形状不动。

## 6. main.rs 接线

- diagnostic auto_start：删除内联 preflight_session 块（main.rs:611-683 一带）→ `let mgr = SessionManager::new(...)`（会话级资源/租约/绑定快照注入）→ `create(intent)` + `start(id)`；失败路径 = SessionFailed 事件 + Degraded（语义与现在一致）。
- `VBMF_SESSION_LIFECYCLE=1`（bmd+gstreamer，仿 VBMF_LOOPBACK 模式）：加载 manifest → 构造 Manager → create→start→轮询 health 10s→stop→close，每步打印 `SESSION-RT-01 step=... verdict=OK/FAIL`；随后第二次 create（同资源）验证 `NotAcquirable` 拒绝（RESOURCE-RT-01 真机证据）；exit(0/2)。
- watchdog：recover 前强制 `lm.is_valid`（不变量保留）；Escalate 时把会话相位置 Degraded（经 Manager 提供的回调/快照接口，不反向依赖）。
- Production：Ready 等控制面不变；selftest 路径不动。
- 进程常驻期：health 端点 tick 借用（每 5s `mgr.tick()`），驱动 lease/预留过期与快照刷新。

## 7. 测试与门禁（三层）

| 层 | SESSION-RT-01 | RESOURCE-RT-01 |
|----|---------------|----------------|
| Unit | 状态机白名单（含 RELEASED→Running 拒）、回滚表纯逻辑 | claim 相位推进/allocate_for 校验 holder |
| Simulation (mock) | MockBackend 全链 happy path；FailingBackend（instantiate/start 失败注入）→ 零孤儿断言（表空+资源 Available+lease 释放）；double-start/double-stop 拒绝；事件序列断言 | 双会话线程并发争抢同资源（后到 NotAcquirable）；release 后可重占；tick 过期回收；crash cleanup（Manager 重建 + release_reservations 兜底） |
| Hardware（盒） | `VBMF_SESSION_LIFECYCLE=1`：真机全链逐步 verdict | 同入口第二会话 NotAcquirable 实证 |

FailingBackend：session.rs `#[cfg(test)]` 内测试桩（`FailingAt::Instantiate/Start`），非生产代码。

## 8. CI

新 job `session-lifecycle`（name 即 context）：checkout + toolchain + `cargo test --features mock session::` + `resource::`（session/resource 门禁测试显式跑）；protection contexts 增至 7（六项保留）。fmt/clippy -D/五套编译不降。

## 9. 实施顺序

resource/lease 补全 → preflight.rs → session.rs（类型→状态机→Manager 引擎）→ events → main 接线 → 三层测试 → CI job → 盒上矩阵（含 fmt/hardware-test）→ 真机 SESSION-RT-01/RESOURCE-RT-01。

## 10. 债务与边界（重申）

PAUSED 媒体面语义（0.7B）；V0.2 §3.11 九维资源向量（本期 DEVICE_EXCLUSIVITY=capacity-1 端口语义，向量记账 0.7 债务）；多卡 placement/balancing（P1/P2）；per-claim 独立 TTL（近似实现，0.7 债务）。
