# Comet Design Handoff

- Change: p07c-runtime-query
- Phase: design
- Mode: compact
- Context hash: 096f2e14046432238856255933e035a63dc9bb2633554225efba9d11c52415fd

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07c-runtime-query/proposal.md

- Source: docs/openspec/changes/p07c-runtime-query/proposal.md
- Lines: 1-30
- SHA256: 73d44ecad7c78dc932392b5caca1c492de55d5f841156128640aae33dbcb5a59

```md
# Change: Phase 0.7C-2 — p07c-runtime-query（Runtime Query Model + D6 Backend Capability）

## Why

0.7C Foundation 建立了 Canonical→Runtime 聚合边（`runtime_state()` snapshot），但还没有形成 External API 之前的缺失关键层：**Runtime Query Model**（只读、snapshot-oriented 的内部 Contract）。终审裁定：不做 External API——先把查询层做稳，避免把 Rust 内部 snapshot 直接变成 External DTO 产生第二套模型。同时 **D6（BACKEND-CAPABILITY-01）是 0.7C 剩余唯一前置债务**：BackendCapability stage 当前恒 WARN 占位，应升级为 `Provider → Capability Probe → Runtime State → Query Model` 的 canonical capability projection + Preflight 硬判定。

## What Changes

- **`src/runtime_query.rs`（新，编排层只读门面）**：
  - `RuntimeQuery`（包 `Arc<SessionManager>`）——**Pure Read / Snapshot Semantics**（终审新原则）：`get_runtime_state() / get_device(id) / get_port(id) / get_resource(id) / get_session(id) / list_sessions() / get_capabilities()`——**全部只读**；公开面 allowlist 白盒锁定（零 mut/命令动词：start/stop/restart/allocate/release/route/switch/probe/refresh/cleanup 全部类型层面不存在）。
  - 查询结果类型 = 既有 `CanonicalRuntimeState` 子项的引用/克隆视图（**不造第二套 DTO**）。
- **D6（BACKEND-CAPABILITY-01）**：
  - **Capability projection**：`DeviceCapabilities`（port.rs 既有：input/output 端口数与能力）投影进 `DeviceRuntimeState.capabilities: Option<DeviceCapabilitiesSummary>`；`CapabilityReport`（SPI）→ 汇总。
  - **Preflight 硬判定**：BackendCapability stage 升级——registry 在场且设备 capability 投影可知时：Capture intent 设备**无输入能力**（input=Unsupported）⇒ **FAIL**（不再恒 WARN）；能力 Unknown ⇒ 保持 WARN（absence≠evidence，不臆造）。
  - **Mock 探针实化**：`MockProvider::probe_capabilities` 返回确定性报告（sdi-capture——现已有 items: vec!["sdi-capture"]，非空）；`SessionManager` 增 capability 投影装配（devices 的 DeviceCapabilities 进 runtime_state）。
- **D14/D15 登记（新债务，随本 PR 入债务表）**：
  - **D14 Runtime Snapshot Consistency**：`runtime_state()` 当前为各源（devices/ports/resources/sessions 分别 clone）拼合的 snapshot，非事务一致——语义定义（source observation time/state version）留 Runtime Query 后续；本 change 在类型文档显式标注"snapshot, 非事务一致"。
  - **D15 Media Flow Cardinality**：`PortId ≠ Media Stream`——一个 Port 可对应 0/1/N media flows（未来 audio 多轨/timecode/metadata）；`media_semantics: Vec<PortMediaSemantics>` 已避免过度限制，契约注释补写"PortId 是物理/逻辑绑定关系，不等于单一 media flow"。
- **门禁 RUNTIME-QUERY-RT-01（三层）**：Unit（查询只读白盒 + get_* 各返回路径 + D6 FAIL/WARN/FAIL 三态）；Simulation（mock 世界全查询面 + capability 投影）；Hardware（真机 SESSION_LIFECYCLE 输出含 capabilities 的 runtime_state + 查询面冒烟）。
- **CI**：测试并入现有矩阵。

## Capabilities

（`skip_specs: true`——SoT 为 PHASE_IMPLEMENTATION_MAP §3（Query Model 项）+ 终审裁定。）

## Impact

- 编译：五套 feature 不回退；runtime_query.rs 零 vendor 依赖。
- 受影响：新 `runtime_query.rs`；`runtime_state.rs`（DeviceRuntimeState 增 capabilities 投影 + D14/D15 契约注释）；`preflight.rs`（BackendCapability 硬判定）；`session.rs`（assemble 消费 DeviceCapabilities——DeviceInfo 已有该字段）；`main.rs`（mod + gate 挂点）；债务表 + Phase Map。
- **明确不做**：External API/REST/Fastify/Webhook/GraphQL；Command Contract（start/stop 等命令动词）；Idempotency；Event Projection；Multi-site/Global Scheduler；D6 的真实 BMD SDK 深度能力探针（用既有 DeviceCapabilities 投影，SDK 深探针属 Provider 后续演进）；D14 实现（只登记）；D15 实现（只契约注释）。

```

## docs/openspec/changes/p07c-runtime-query/design.md

- Source: docs/openspec/changes/p07c-runtime-query/design.md
- Lines: 1-31
- SHA256: 40d7648d31237a0d43cce4d0ad81cdf963eef063c323c21fb1141a512d6f8e9f

```md
# Design: Phase 0.7C-2 — p07c-runtime-query

## Context

0.7C Foundation 交付 `runtime_state()` snapshot 聚合；终审批准进入 Query Model（严格只读）并关闭 D6。现有素材：`DeviceInfo.capabilities: DeviceCapabilities`（port.rs:109-117，input/output 端口数 + audio 能力，`CapabilityValue<T>` Unknown/Supported/Unsupported 三态）；SPI `probe_capabilities() -> Vec<CapabilityReport>`（Mock 已返回非空 items）；preflight BackendCapability 恒 WARN（session.rs run_preflight 传空 caps）。

## Goals / Non-Goals

**Goals:** RuntimeQuery 只读门面（Pure Read 白盒）+ D6 capability projection/preflight 硬判定 + D14/D15 登记 + RUNTIME-QUERY-RT-01 三层。
**Non-Goals:** 见 proposal 不做清单（尤其：无命令动词、无 External DTO、无 SDK 深探针、D14/D15 只登记）。

## Decisions

- **D1 RuntimeQuery 门面（零 DTO）**：`pub struct RuntimeQuery(Arc<SessionManager>)`；`get_runtime_state()`（委托 mgr.runtime_state()）+ `get_device/get_port/get_resource/get_session(id)`（在 snapshot 内查，Option 返回既有类型克隆）+ `list_sessions()` + `get_capabilities()`（Vec<(Uuid, DeviceCapabilitiesSummary)>）。**不加新查询结果类型**——返回既有 CanonicalRuntimeState 子项（防第二套模型）。
- **D2 Pure Read 白盒（终审新原则）**：公开面 allowlist = 全部 `get_/list_` 前缀；测试硬编码清单比对（防 start/stop/allocate/release/route/switch/probe/refresh/cleanup 静默进入——Preflight 副作用教训的延续）；实现层不持有可变引用（构造后仅读 Arc<SessionManager> 的只读方法 + runtime_state()）。
- **D3 Capability projection（D6）**：`DeviceCapabilitiesSummary { can_input: CapabilityFlag, can_output: CapabilityFlag, input_ports: Option<u32>, output_ports: Option<u32> }`（CapabilityFlag = Unknown/Supported/Unsupported，直取 DeviceCapabilities 三态）；`DeviceRuntimeState` 增 `capabilities: Option<DeviceCapabilitiesSummary>`（assemble 从 `DeviceInfo.capabilities` 投影；None = 数据不在场）。
- **D4 Preflight 硬判定（D6）**：BackendCapability stage——当 `inputs.devices` 的 capability 投影可知（Some）：Capture intent 设备 `can_input == Unsupported` ⇒ **FAIL**（"设备无输入能力"）；`Unknown` 或投影缺失 ⇒ 保持 WARN（absence≠evidence）。需 PreflightInputs 已含 devices（现有字段，直接读 `DeviceInfo.capabilities`——零新输入）。
- **D5 SPI 报告消费（最小）**：`RuntimeQuery::get_capabilities()` 附带 SPI `CapabilityReport` 计数摘要（source 维度）——只聚合展示，不进判定（判定用 D3 投影，同源更可信）。
- **D6 D14 契约标注**：`CanonicalRuntimeState` 文档注释显式 "snapshot, 非事务一致（D14——各源独立观测时刻拼合；一致性语义属后续）"；`generated_at_ms` 保留。
- **D7 D15 契约标注**：`PortMediaSemantics` 文档注释 "PortId 是物理/逻辑绑定关系，**不等于**单一 media flow（一 Port 可对应 0/1/N flows——audio 多轨/timecode 属后续）"。
- **D8 门禁 RUNTIME-QUERY-RT-01**：Unit（白盒 allowlist / get_* 路径含 None / D6 三态 FAIL·WARN·Pass）；Simulation（mock 全查询面 + capability 投影断言）；Hardware（SESSION_LIFECYCLE 的 runtime_state 输出含 capabilities + RuntimeQuery 冒烟）。

## Risks / Trade-offs

- DeviceCapabilities 在 mock 世界多为 Unknown/default（三态 Unknown）⇒ D4 判定在 mock 下走 WARN 分支——Simulation 测试需构造 Supported/Unsupported 的 DeviceCapabilities 注入（直接构造 DeviceInfo.capabilities）。
- preflight test1（clean case）devices 由 `device()` helper 构造（capabilities: default = 全 Unknown）⇒ 保持 WARN，verdict 仍 Warn——零破坏预期；需验证。
- RuntimeQuery 包 Arc<SessionManager>——SessionManager 现无 Arc 自投影；门面构造改为接受 `Arc<SessionManager>`（main.rs 持有 mgr 为局部值——需 Arc 化或门面持引用泛型。**取舍**：`RuntimeQuery` 持 `&SessionManager` 生命周期或改为 `RuntimeQuery::new(Arc<SessionManager>)` 要求 main.rs 的 mgr Arc 化（局部小改，值得——后续 Command Contract 也要共享所有权）。

## 实施顺序

runtime_state.rs 投影+注释 → runtime_query.rs 门面+白盒 → preflight D4 → main.rs mgr Arc 化+挂点 → 测试 → 盒上矩阵+真机 → 债务表/Phase Map。

```

## docs/openspec/changes/p07c-runtime-query/tasks.md

- Source: docs/openspec/changes/p07c-runtime-query/tasks.md
- Lines: 1-37
- SHA256: 876182bdaaf8851a30e98bc95cb971e67eb98708260e5d90eb60dab93521632b

```md
# Tasks: Phase 0.7C-2 — p07c-runtime-query

> 纪律（§十五）：每项标注 `Contract / Implementation / Verification / Gate`。证据规范（终审 §七）：commit + test name + command + result。

## 1. Runtime Query Model（runtime_query.rs 新）

- [ ] 1.1 RuntimeQuery 只读门面（get_/list_ 全集; 零新 DTO——返回既有类型; Pure Read 白盒 allowlist）
  - Contract: 终审 §十二/§十四 (Pure Read / Snapshot Semantics) | Implementation: Not Started | Verification: Test(白盒+路径) | Gate: Pending
- [ ] 1.2 D14/D15 契约标注（snapshot 非事务一致; PortId ≠ media flow）
  - Contract: 终审 §四/§五 | Implementation: Not Started | Verification: Test(文档契约编译) | Gate: Pending

## 2. D6 Backend Capability

- [ ] 2.1 Capability projection（DeviceCapabilities → DeviceCapabilitiesSummary 进 DeviceRuntimeState）
  - Contract: BACKEND-CAPABILITY-01 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 2.2 Preflight 硬判定（can_input=Unsupported ⇒ FAIL; Unknown ⇒ WARN 不臆造）
  - Contract: BACKEND-CAPABILITY-01 (hard decision) | Implementation: Not Started | Verification: Test(三态) | Gate: Pending

## 3. 门禁 RUNTIME-QUERY-RT-01（三层）

- [ ] 3.1 Unit: 白盒 + get_* 路径 + D6 三态
  - Contract: 本 change 门禁 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 3.2 Simulation: mock 世界全查询面 + capability 投影
  - Contract: 同上 | Implementation: Not Started | Verification: Simulation | Gate: Pending
- [ ] 3.3 Hardware: SESSION_LIFECYCLE runtime_state 含 capabilities + 查询冒烟
  - Contract: 同上 | Implementation: Not Started | Verification: Hardware | Gate: Pending

## 4. 交付

- [ ] 4.1 盒上全矩阵 + CI 七 checks + 真机 SESSION/RESOURCE-RT-01 回归
  - Contract: 盒上绿≠CI绿 | Implementation: Not Started | Verification: Box+CI | Gate: Pending
- [ ] 4.2 债务表 D6 CLOSED + D14/D15 登记 + Phase Map 0.7C-2 行 → verify → archive → PR#9 → tag phase-0.7C2-runtime-query → 删分支
  - Contract: 分支纪律 | Implementation: Not Started | Verification: CI+Review | Gate: Pending

## 收口确认

- 不做: External API/REST/命令动词/Idempotency/Event Projection/SDK 深探针/D14·D15 实现。

```
