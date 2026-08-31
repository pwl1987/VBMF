# Comet Design Handoff

- Change: p07c-runtime-state
- Phase: design
- Mode: compact
- Context hash: 158c946e455dbbf2335bb662ae2330b563540ea84e1a0c583429c8f322c4edb1

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07c-runtime-state/proposal.md

- Source: docs/openspec/changes/p07c-runtime-state/proposal.md
- Lines: 1-33
- SHA256: 443848040152e675c943b7c9183e50147468166298c6ed11a306cc3c47e48481

```md
# Change: Phase 0.7C Foundation — p07c-runtime-state（Canonical Runtime State 接线 + D2/D4/D5 伴随清债）

## Why

0.7B Consolidation 的 Integration Audit 确认：Canonical 层与 Runtime 层是**不相交子图**（canonical 输出仅 loopback 证据打印；SessionManager 零 canonical 输入）——0.7C 前置顺序的第一步就是补这条边。终审执行令：**不做纯清债 change**——D2（Resource Resolution）/D4（Port Availability）/D5（Identity Binding）恰好是 Canonical Runtime State 建立时必须消费的三类真实运行事实，随接线一并关闭；D6（BACKEND-CAPABILITY-01）靠近 Runtime Query/Capability 层，留给第二 change。

## What Changes

- **`src/runtime_state.rs`（新，编排层）——CanonicalRuntimeState 聚合（组合，绝不展开成万能 struct）**：
  - **加严红线**（终审）：`Canonical Media Semantics（是什么）≠ Runtime State（现在运行成什么状态）`——媒体语义以 `CanonicalMediaDescriptor` **整值组合**（`PortMediaSemantics { port_id, descriptor }` 列表），Runtime 侧只存运行事实字段（bound/running/health），**绝不**把 descriptor 字段平铺进 state 结构。
  - `DeviceRuntimeState { device_id, model, identity_strength, binding: Option<BindingStatus> }`（BindingStatus = production_grade binding 的摘要：match_kind + confidence）
  - `PortRuntimeState { port_id, device_id, direction, connector }`
  - `ResourceRuntimeState { resource_id, device_id, capability, state: ResourceState }`
  - `SessionRuntimeState { session_id, state, phase, resource_claims 摘要, pipeline: Option<u64> }`
  - `CanonicalRuntimeState { devices, ports, resources, sessions, media_semantics: Vec<PortMediaSemantics>, generated_at_ms }` + serde（证据输出）。
  - `assemble(devices, &PortRegistry, &ResourceRegistry, &bindings, &[MediaSession]) -> CanonicalRuntimeState`：纯装配（组合 canonical descriptor：对每个有观测的端口 normalize）。
- **`SessionManager::runtime_state(&self) -> CanonicalRuntimeState`**：第一条**生产路径**（SessionManager 字段已持有 devices/bindings/registry——snapshot 聚合 + `self.list()`）；**这是 Canonical→Runtime 边的建立**，非 loopback 证据补丁。
- **D2（derive_claims FAIL 化 / RESOURCE-RESOLUTION-01）**：preflight Stage3 升级为三态 Resolution——intent 设备在 ResourceRegistry 中**无派生 input 资源 ⇒ FAIL**（"declared capability missing"不再 WARN）；有资源但 claim 不可满足 ⇒ FAIL（现有 preflight 复用）。
- **D4（PortAvailability 端口级精确化）**：镜像 materialize 已冻结语义（pipeline.rs:485-523）——`port_id: Some` ⇒ 精确端口必须存在且 direction 为 Input/Bidirectional（缺失 ⇒ FAIL）；`port_id: None` ⇒ 设备须有 ≥1 Input 方向端口（升级现 any-port 检查）；`registry=None ⇒ WARN`（legacy 路径不变）。
- **D5（IdentityBinding 实查 / IDENTITY-BINDING-01）**：`ResolvedDeviceBinding::is_production_grade()`（`Confidence::High && matches!(match_kind, PersistentIdExact|SerialExact|DeviceHandleExact|ManifestVerified)`）共享 helper；preflight IdentityBinding 与 session.rs create 步 5 Binding verify 同步使用（key-existence → 实查强度）。对现生产者行为保持（只产 High+exact）。
- **门禁 RUNTIME-STATE-RT-01（三层）**：Unit（D2 无资源 FAIL/D4 端口级三态/D5 强度实查/聚合组合性——state 结构零 descriptor 字段平铺断言）；Simulation（mock 世界 create→runtime_state 投影）；Hardware（VBMF_SESSION_LIFECYCLE 输出 CanonicalRuntimeState JSON——真机证据）。
- **债务表**：D2/D4/D5 标记 CLOSED（本 change）；Phase Map 0.7C 行更新。

## Capabilities

（`skip_specs: true`——SoT 为 PHASE_IMPLEMENTATION_MAP §2/§3/§4 + 终审执行令。）

## Impact

- 编译：五套 feature 不回退；runtime_state.rs 位于编排层（可引 canonical + port/resource/resolver 类型，零 vendor）。
- 受影响：新 `runtime_state.rs`；`preflight.rs`（Stage2/3/5 升级）；`resolver.rs`（+is_production_grade）；`session.rs`（+runtime_state 方法；create 步 5 改用 helper；derive_claims 不动签名）；`main.rs`（mod + SESSION_LIFECYCLE 证据挂点）；债务表 + Phase Map。
- 测试影响面：preflight test 1 更新（clean case 需注册派生资源）；session.rs 现有测试零破坏（fixture 每设备有 sdi-input 资源与 Input 端口；port_id=None 走 fallback）。
- **明确不做**：D6；REST/External API/Command Contract/Event Projection/Idempotency；Audio Routing Execution；Clock Policy；Timecode Parser；D11 Timeline。

```

## docs/openspec/changes/p07c-runtime-state/design.md

- Source: docs/openspec/changes/p07c-runtime-state/design.md
- Lines: 1-29
- SHA256: 0cb61b765ac6cfe99dffe09a2cfb4ec372e05d9b9ac00fca9b092c3f7dd6112c

```md
# Design: Phase 0.7C Foundation — p07c-runtime-state

## Context

Integration Audit 事实：Canonical/Runtime 不相交子图；SessionManager 已持有 devices/bindings/registry 字段（session.rs:233-245）可零构造改动聚合。D2/D4/D5 的最小侵入路径已探底：D2 落 preflight Stage3（不动 derive_claims 签名）；D4 镜像 materialize 冻结语义（pipeline.rs:485-523）；D5 抽共享 helper（现生产者 High-by-construction，行为保持）。

## Goals / Non-Goals

**Goals:** CanonicalRuntimeState 聚合（组合非展开）+ SessionManager::runtime_state 生产路径 + D2/D4/D5 关闭 + RUNTIME-STATE-RT-01 三层。
**Non-Goals:** D6/API/Command Contract/Event Projection/Idempotency/Audio Execution/Clock Policy/Timecode Parser/D11；不重构 derive_claims 签名；不改 SessionManager 构造函数。

## Decisions

- **D1 组合红线（终审加严）**：`media_semantics: Vec<PortMediaSemantics{port_id, descriptor: CanonicalMediaDescriptor}>` ——descriptor 整值组合；Runtime 结构（Device/Port/Resource/Session RuntimeState）只存运行事实。**测试锁定**：serde JSON 断言 descriptor 字段（width/role/presence 等）只出现在 `media_semantics[].descriptor` 命名空间内，绝不平铺到 state 顶层。
- **D2 三态 Resolution（preflight Stage3）**：per intent-device：`resources.iter().any(r.device_id==u && capability ends_with("-input"))`——Missing ⇒ FAIL("设备无派生 input 资源 (declared capability missing)")；有资源 ⇒ 现有 claim 逐项 preflight（不可满足 FAIL）；claims 空且资源存在（诊断路径）保持 WARN。registry=None 的 legacy WARN 分支保留但仅当资源表也空。
- **D4 端口级（preflight Stage2，镜像 materialize）**：`port_id:Some(pid)` → parse 失败或 `ports.find(identity.port_id==Some(u))` 无匹配 ⇒ FAIL；匹配但 direction ∉ {Input,Bidirectional} ⇒ FAIL（Capture intent 需输入端口）；`port_id:None` → 设备须有 ≥1 Input/Bidirectional 端口（升级 any-port）；registry=None ⇒ WARN。
- **D5 is_production_grade（resolver.rs）**：`pub fn is_production_grade(&self) -> bool { self.confidence == Confidence::High && matches!(self.match_kind, PersistentIdExact|SerialExact|DeviceHandleExact|ManifestVerified) }`；preflight Stage5 正分支改 `contains_key && is_production_grade()`（未达标条目列出）；session.rs:457-470 Binding verify 同步。
- **D6 聚合器**：`assemble()` 纯函数（无 IO/锁）；descriptor 装配复用 `RawInputDescription::from_port`+`normalize_input`（0.7B 资产）；binding 摘要 `BindingStatus{match_kind, confidence}` 仅 production_grade 才入 DeviceRuntimeState。
- **D7 生产路径**：`SessionManager::runtime_state()`（snapshot：devices.clone + bindings.clone + registry.clone + resources.with_inner(clone) + self.list() + leases.list_active 计数）——**不经 loopback 证据路径**；`VBMF_SESSION_LIFECYCLE` 在 create 前后各输出一次 state JSON（真机证据：资源 Reserved/Released 在 state 中的投影变化）。

## Risks / Trade-offs

- preflight test 1 更新（clean case 需派生资源）——预期内；session.rs 测试 fixture 天然满足新语义（探底确认）零破坏。
- D4 对 port_id=None 收紧为 Input-only：现有 fixture 全为 Input 端口；真机 manifest 端口也是 Input——零影响，且修复了"Output 端口混过 Capture"的漏洞。
- runtime_state() 每次全量 clone：规模为端口/会话数级（个位-十位数），无性能面；tick 高频调用不引入（仅按需/证据）。

## 实施顺序

runtime_state.rs 类型+assemble → D5 helper（resolver）→ preflight D2/D4/D5 + 测试更新 → session.rs runtime_state()+binding verify 换 helper → main.rs 挂点 → 盒上矩阵 + 真机 → 债务表/Phase Map 更新。

```

## docs/openspec/changes/p07c-runtime-state/tasks.md

- Source: docs/openspec/changes/p07c-runtime-state/tasks.md
- Lines: 1-39
- SHA256: 290326bc8e856525ddbe76d56920921cd7311636c8349984776d3d05881912ac

```md
# Tasks: Phase 0.7C Foundation — p07c-runtime-state

> 纪律（§十五）：每项标注 `Contract / Implementation / Verification / Gate`。

## 1. CanonicalRuntimeState 聚合（runtime_state.rs 新）

- [ ] 1.1 Device/Port/Resource/Session RuntimeState + PortMediaSemantics(descriptor 整值组合) + assemble() 纯装配 + serde
  - Contract: 终审加严红线 (Canonical≠Runtime State; 组合非展开) | Implementation: Not Started | Verification: Test(组合性断言) | Gate: Pending
- [ ] 1.2 SessionManager::runtime_state() 生产路径 (第一条 Canonical→Runtime 真实边)
  - Contract: PHASE_IMPLEMENTATION_MAP §3 首项 | Implementation: Not Started | Verification: Simulation+Hardware | Gate: Pending

## 2. D2/D4/D5 关闭

- [ ] 2.1 D2: preflight Stage3 三态 Resolution (设备无派生 input 资源 ⇒ FAIL)
  - Contract: RESOURCE-RESOLUTION-01 (终审 §八) | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 2.2 D4: preflight Stage2 端口级 (port_id Some 精确匹配+方向; None ⇒ ≥1 Input 端口; registry=None WARN)
  - Contract: Port Availability Contract (镜像 materialize 冻结语义) | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 2.3 D5: ResolvedDeviceBinding::is_production_grade() + preflight Stage5/session create 步 5 实查
  - Contract: IDENTITY-BINDING-01 | Implementation: Not Started | Verification: Test | Gate: Pending

## 3. 门禁 RUNTIME-STATE-RT-01（三层）

- [ ] 3.1 Unit: D2/D4/D5 三态各 FAIL 路径 + 聚合组合性 (descriptor 不平铺)
  - Contract: 本 change 门禁 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 3.2 Simulation: mock 世界 create→runtime_state 投影 (claims/资源状态可见)
  - Contract: 同上 | Implementation: Not Started | Verification: Simulation | Gate: Pending
- [ ] 3.3 Hardware: VBMF_SESSION_LIFECYCLE 输出 CanonicalRuntimeState JSON (create 前后状态变化)
  - Contract: 同上 | Implementation: Not Started | Verification: Hardware | Gate: Pending

## 4. 交付

- [ ] 4.1 盒上全矩阵 (fmt/test×4/clippy×4/build×3/PROOF) + CI 七 checks + 真机 SESSION/RESOURCE-RT-01 回归不退
  - Contract: 盒上绿≠CI绿 | Implementation: Not Started | Verification: Box+CI | Gate: Pending
- [ ] 4.2 债务表 D2/D4/D5 → CLOSED + Phase Map 0.7C 行更新 + verify → archive → PR#8 → merge → tag phase-0.7C1-* → 删分支
  - Contract: 分支纪律 | Implementation: Not Started | Verification: CI+Review | Gate: Pending

## 收口确认

- 不做清单: D6 / REST / Command Contract / Event Projection / Idempotency / Audio Execution / Clock Policy / Timecode Parser / D11。

```
