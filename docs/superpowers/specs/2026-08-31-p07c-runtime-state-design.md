---
comet_change: p07c-runtime-state
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-31-p07c-runtime-state
status: final
---

# Design Doc — p07c-runtime-state（Phase 0.7C Foundation: Canonical Runtime State）

> open design.md D1-D7 实现级细化。锚点：PHASE_IMPLEMENTATION_MAP §2（三红线）/§3（0.7C 前置首项）；终审执行令（含加严红线：禁止万能 struct）。

## 1. `src/runtime_state.rs` — 聚合类型（编排层；可引 canonical/port/resource/resolver 类型，零 vendor）

```rust
/// 设备运行态 (运行事实; 媒体语义不在此 — 组合见 media_semantics)。
pub struct DeviceRuntimeState {
    pub device_id: Uuid,
    pub model: String,
    pub identity_strength: IdentityStrength,
    pub binding: Option<BindingStatus>,   // 仅 production_grade 绑定入列
}
pub struct BindingStatus { pub match_kind: ResolverMatch, pub confidence: Confidence }

pub struct PortRuntimeState { pub port_id: Uuid, pub device_id: Uuid,
    pub direction: PortDirection, pub connector: ConnectorType }

pub struct ResourceRuntimeState { pub resource_id: Uuid, pub device_id: Uuid,
    pub capability: String, pub state: crate::resource::ResourceState }

pub struct SessionRuntimeState { pub session_id: SessionId, pub state: SessionState,
    pub phase: SessionPhase, pub claims: usize, pub pipeline: Option<u64> }

/// 媒体语义组合 (终审加严红线): descriptor 整值组合, 绝不平铺字段。
pub struct PortMediaSemantics { pub port_id: Uuid,
    pub descriptor: crate::normalize::CanonicalMediaDescriptor }

pub struct CanonicalRuntimeState {
    pub devices: Vec<DeviceRuntimeState>,
    pub ports: Vec<PortRuntimeState>,
    pub resources: Vec<ResourceRuntimeState>,
    pub sessions: Vec<SessionRuntimeState>,
    pub media_semantics: Vec<PortMediaSemantics>,
    pub generated_at_ms: u64,
}
```

`assemble(devices: &[DeviceInfo], registry: &PortRegistry, resources: &ResourceRegistry, bindings: &HashMap<Uuid, ResolvedDeviceBinding>, sessions: &[MediaSession]) -> CanonicalRuntimeState`：纯函数；media_semantics 对每个 `identity.port_id=Some` 的端口走 `RawInputDescription::from_port` + `normalize_input`（复用 0.7B 资产）；零 IO/锁/全局。

**组合性测试锁定**：serde JSON 中 width/frame_rate/role/presence 等 descriptor 字段只出现在 `media_semantics[].descriptor` 命名空间，顶层 state 键集合 == {devices, ports, resources, sessions, media_semantics, generated_at_ms}。

## 2. `SessionManager::runtime_state()`（D7 生产路径）

```rust
pub fn runtime_state(&self) -> CanonicalRuntimeState {
    let resources = self.resources.with_inner(|r| r.clone());
    let registry = self.registry.clone().unwrap_or_default();
    let sessions = self.list();
    CanonicalRuntimeState::assemble(&self.devices, &registry, &resources, &self.bindings, &sessions)
}
```
第一条 Canonical→Runtime 真实边（SessionManager 主动聚合，非 loopback 证据补丁）。PortRegistry: Clone + Default 已确认（port.rs:336）。

## 3. D2/D4/D5（preflight 升级 + 共享 helper）

**D2（Stage3 三态）**：per intent-device `resources.iter().any(|r| r.device_id == u && r.capability.ends_with("-input"))`：
- Missing ⇒ **FAIL** `"设备 {u} 无派生 input 资源 (declared capability missing)"`；
- 有资源 ⇒ 现有 claims 逐项 `resource::preflight`（NotAcquirable/Unavailable ⇒ FAIL）；
- 资源表整体为空（registry=None legacy）⇒ 保持 WARN。

**D4（Stage2 端口级，镜像 pipeline.rs:485-523）**：
- `port_id: Some(pid)`：parse 失败或无 `identity.port_id == Some(u)` 匹配 ⇒ FAIL；匹配但 direction ∉ {Input, Bidirectional} ⇒ FAIL；
- `port_id: None`：设备须有 ≥1 Input/Bidirectional 端口（升级现 any-port）；
- `registry=None` ⇒ WARN（不变）。

**D5（is_production_grade）**（resolver.rs）：
```rust
impl ResolvedDeviceBinding {
    pub fn is_production_grade(&self) -> bool {
        self.confidence == Confidence::High
            && matches!(self.match_kind, ResolverMatch::PersistentIdExact
                | ResolverMatch::SerialExact | ResolverMatch::DeviceHandleExact
                | ResolverMatch::ManifestVerified)
    }
}
```
preflight Stage5 正分支：`bindings.get(u).is_some_and(|b| b.is_production_grade())`（未达标列出详情）；session.rs:457-470 create 步 5 同步换用。现生产者（collect_bindings/collect_bindings_from_manifest）High-by-construction ⇒ 行为保持。

## 4. RUNTIME-STATE-RT-01（三层）

| 层 | 测试 |
|----|------|
| Unit | D2：无派生资源设备 ⇒ ResourceCapacity FAIL；D4：port_id 指向 Output 端口 ⇒ FAIL / 精确匹配 Input ⇒ PASS / None+仅 Output 端口 ⇒ FAIL / registry=None ⇒ WARN；D5：Medium 置信或 TopologicalIdGuess 条目 ⇒ FAIL（构造非 production_grade binding 注入）；聚合组合性（descriptor 不平铺） |
| Simulation | mock 世界：create 前 runtime_state()（资源 Available）→ create 后（Reserved + session 投影）→ stop 后（Available 回落） |
| Hardware | VBMF_SESSION_LIFECYCLE：create 前后各输出 CanonicalRuntimeState JSON（资源状态/会话投影变化 = 真机证据） |

## 5. 实施顺序

runtime_state.rs → D5 helper → preflight D2/D4/D5（+test1 更新）→ session.rs runtime_state()+verify 换 helper → main.rs 挂点（SESSION_LIFECYCLE 两点输出）→ 盒上矩阵 → 真机 → 债务表/Phase Map。
