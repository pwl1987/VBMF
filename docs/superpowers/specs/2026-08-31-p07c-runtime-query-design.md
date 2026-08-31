---
comet_change: p07c-runtime-query
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-31-p07c-runtime-query
status: final
---

# Design Doc — p07c-runtime-query（Phase 0.7C-2: Runtime Query Model + D6）

> open design.md D1-D8 实现级细化。锚点：PHASE_IMPLEMENTATION_MAP §3（Query Model 项）；终审 §十二-§十四（只读/snapshot/Pure Read 原则）。

## 1. `src/runtime_query.rs` — 只读门面（零 DTO）

```rust
/// Runtime Query Model — **Pure Read / Snapshot Semantics**（终审 §十四新原则）。
/// 只回答"系统现在是什么状态"; 命令动词 (start/stop/restart/allocate/release/route/
/// switch/probe/refresh/cleanup) 在类型层面不存在 (公开面 allowlist 白盒锁定)。
pub struct RuntimeQuery {
    mgr: std::sync::Arc<crate::session::SessionManager>,
}

impl RuntimeQuery {
    pub fn new(mgr: Arc<SessionManager>) -> Self;
    pub fn get_runtime_state(&self) -> CanonicalRuntimeState;          // 委托 mgr.runtime_state()
    pub fn get_device(&self, id: Uuid) -> Option<DeviceRuntimeState>;  // snapshot 内查
    pub fn get_port(&self, id: Uuid) -> Option<PortRuntimeState>;
    pub fn get_resource(&self, id: Uuid) -> Option<ResourceRuntimeState>;
    pub fn get_session(&self, id: SessionId) -> Option<SessionRuntimeState>;
    pub fn list_sessions(&self) -> Vec<SessionRuntimeState>;
    pub fn get_capabilities(&self) -> Vec<(Uuid, DeviceCapabilitiesSummary)>;
}
```
**零新 DTO**：全部返回既有 `CanonicalRuntimeState` 子项克隆（防第二套模型）。每次 get_* 取新鲜 snapshot（简单正确；缓存属后续优化——登记不必要）。

**白盒 allowlist**：`["new","get_runtime_state","get_device","get_port","get_resource","get_session","list_sessions","get_capabilities"]`——新增公开项须显式过 Pure Read 评审。

## 2. Capability projection（D6）

```rust
// runtime_state.rs:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityFlag { Unknown, Supported, Unsupported }  // 直取 CapabilityValue 三态

pub struct DeviceCapabilitiesSummary {
    pub can_input: CapabilityFlag,
    pub can_output: CapabilityFlag,
    pub input_ports: Option<u32>,   // Supported(n) → Some(n); 否则 None
    pub output_ports: Option<u32>,
}
// DeviceRuntimeState 增: pub capabilities: Option<DeviceCapabilitiesSummary>
// (assemble 从 DeviceInfo.capabilities 投影; None = 数据不在场)
```

## 3. Preflight 硬判定（D6）

BackendCapability stage 升级（PreflightInputs 已含 devices——零新输入）：
- 设备 capability 投影可知 且 `can_input == Unsupported` ⇒ **FAIL**（"设备 {u} 无输入能力 (capability=unsupported)"）；
- `Unknown` 或投影缺失 ⇒ **WARN**（absence≠evidence，不臆造——保留现 WARN 文案细化）；
- `Supported` ⇒ **Pass**。

## 4. D14/D15 契约标注（登记不实现）

- `CanonicalRuntimeState` 文档：**"snapshot, 非事务一致（D14）——devices/ports/resources/sessions 为各源独立观测的拼合快照；一致性语义（source observation time / state version）属后续"**。
- `PortMediaSemantics` 文档：**"PortId 是物理/逻辑绑定关系，不等于单一 media flow（D15）——一 Port 可对应 0/1/N flows（audio 多轨/timecode/metadata 属后续）；Vec 结构已避免过度限制"**。
- 债务表新增 D14/D15 行。

## 5. main.rs 接线（最小）

- `mgr` Arc 化（`Arc::new(SessionManager...)` → clone 进 RuntimeQuery + 原用法不变——Command Contract 后续也需共享所有权）。
- SESSION_LIFECYCLE gate：runtime_state 输出自然含 capabilities（Hardware 证据）；追加 RuntimeQuery 冒烟（get_device/get_session 各一次，打印命中/未命中）。

## 6. RUNTIME-QUERY-RT-01（三层）

| 层 | 测试 |
|----|------|
| Unit | `pure_read_public_surface_allowlist`（硬编码清单+命令动词禁入）；`get_*` 全路径（命中/None）；D6 三态（Unsupported FAIL/Unknown WARN/Supported Pass——直接构造 DeviceInfo.capabilities 注入） |
| Simulation | mock 世界：create→query 全面（device/port/resource/session/capabilities 各命中 + 幽灵 id None + list_sessions 生命周期投影） |
| Hardware | SESSION_LIFECYCLE：runtime_state JSON 含 `"capabilities"` 投影 + RuntimeQuery 冒烟输出 |

## 7. 实施顺序

runtime_state.rs（projection + D14/D15 注释）→ runtime_query.rs → preflight D6 → session.rs 无改（assemble 读 DeviceInfo.capabilities 已在字段内）→ main.rs Arc 化+挂点 → 测试 → 盒上 → 债务表/Phase Map。
