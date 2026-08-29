# Comet Design Handoff

- Change: p07-session-runtime
- Phase: design
- Mode: compact
- Context hash: d5dd8e2c6df6e0bfadd6fbb9adaa1ebde0423cc9845b1863856b68785fc9f7b6

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07-session-runtime/proposal.md

- Source: docs/openspec/changes/p07-session-runtime/proposal.md
- Lines: 1-29
- SHA256: 410b60a34db0d1dd5d4f7452eeb02869b27a68e1b82a0bb7027ff6aec6c9f43e

```md
# Change: Phase 0.7A — p07-session-runtime（Session Runtime：让 0.6 冻结架构成为真正工作的媒体运行时）

## Why

0.6 Baseline（master `d1cfaa9`，tag `phase-0.6-runtime-abstraction-baseline`）完成了架构解耦，但 Gap Matrix 中 **0.6A Session Model / 0.6E Resource Model+Preflight 的实现仍为 NOT_STARTED**：`session.rs` 不存在；`lease.rs` 无 renew；resource 编排缺 allocate/release-to-stop/TTL 触发；main.rs diagnostic 路径的内联生命周期（acquire→materialize→instantiate→start→watchdog，main.rs:583-771）无测试、无 stop 路径（`MediaBackend::stop` 全仓零调用）、7 个 RuntimeEvent 变体零发射。Pipeline/Lease/Resource/Health 各自独立运行，没有统一 owner。这些是**冻结契约（RUNTIME_SESSION_MODEL / RUNTIME_RESOURCE_MODEL / RUNTIME_LIFECYCLE_SEQUENCE，均 P0 FROZEN）的实现债务**，也是后续 Normalize/Audio/API 稳定运行的前置。

**分段编号说明**：本 change 沿用 roadmap 新分段 **0.7A = Runtime Ownership**（用户终审裁定），替代 MASTER_PRD §5 旧 0.7A-G 子阶段标签（旧 0.7A=External API 等）；PRD 文档本身不改（§二十五 禁令），替代关系记录于本 change 文档。实现语义严格对齐冻结契约，无任何新发明。

## What Changes

- **`src/session.rs`（新）**：`MediaSession`（session_id/graphs 语义描述/source+output ports/resource_claims/lease/pipeline: Option<PipelineHandle>/health 快照）——逻辑持有、物理引用，绝不含 concrete backend 对象；两级状态机（粗态 RESERVED→RUNNING→PAUSED→RELEASING→RELEASED，RELEASED→RUNNING 拒绝；微相位 Requested→Provisioning→Binding→Leased→Starting→Running→Stopping→Released + 失败态），白名单迁移。
- **`SessionManager`（唯一生命周期 owner，RUNTIME_SESSION_MODEL §4.1）**：`create/start/stop/close/status/list`；生命周期引擎实现冻结顺序 `Intent→Preflight→Reserve→Create Session→Lease→Binding verify→Backend.instantiate→Allocate→Backend.start→RUNNING`；失败精确逆序回滚（Allocate→Lease→Reservation→销毁 Session），creator=destroyer，零孤儿。
- **Resource Orchestration 补全（resource.rs）**：`allocate_for`（instantiate 成功后 Reserved→Allocated）、`release_allocation`（对接 `MediaBackend::stop`——该 trait 方法首次被真实调用）、预留 TTL 过期触发。
- **Lease（lease.rs）**：trait 增 `renew`；接线 config 既有未消费旋钮（`default_lease_ttl`/`lease_renew_window`）；过期扫描由 health tick 驱动（无后台线程）；设备丢失后恢复前强制重验 lease（既有不变量）。
- **Preflight 分级报告（新 `preflight.rs`）**：`graph→port availability→resource capacity→lease conflict→identity/binding→backend capability` 逐阶段结果 + PASS/WARN/FAIL 裁决；topology/risk 级 report-only；**只判断不执行**（V0.2 §1.2 三层 Preflight 语义）。
- **事件补线（events.rs）**：additive `SessionCreated/SessionStateChanged/SessionFailed` kinds；点亮零发射事件（LeaseGranted/ResourceAllocated/ResourceReservationExpired/IdentityResolved）；不加新事件平面（TD-16 保持）。
- **main.rs 重接线**：diagnostic auto-start 路径改走 SessionManager；Production 仍 Ready 等控制面；MEDIA-RT-01 selftest 路径不动。
- **门禁**：**SESSION-RT-01**（全生命周期 + 失败回滚 + double-start/stop 拒绝）与 **RESOURCE-RT-01**（并发争抢/容量/冲突/release/expiry/crash cleanup），各三层测试（Unit/Simulation-Mock/Hardware）；CI 新增 `session-lifecycle` required job（七 context）。

## Capabilities

（`skip_specs: true`——canonical 语义 SoT 为 `docs/architecture/` 冻结契约 RUNTIME_SESSION_MODEL / RUNTIME_RESOURCE_MODEL / RUNTIME_LIFECYCLE_SEQUENCE / MEDIA_BACKEND_CONTRACT §1.1；本 change 是其实现，非新需求。）

## Impact

- **编译**：default/simulation/mock/bmd,gstreamer/hardware-test 五套保持可编译；fmt/clippy -D 全绿（CI 七 gate 不降）。
- **受影响代码**：新 `session.rs`/`preflight.rs`；`resource.rs`/`lease.rs`/`events.rs`/`main.rs`/`supervisor.rs`（recovery 服务 Session 的接线点）/CI workflow + protection。
- **行为变化**：diagnostic auto-start 从内联代码变为 SessionManager 驱动（事件更完整、有 stop 路径）；新增 `VBMF_SESSION_LIFECYCLE=1` 真机门禁入口。
- **明确不做**：Normalize/Clock/Timecode/Audio/External API/Webhook/UI/全局 Scheduler/多站点/AJA/ONVIF/Kafka/NATS/GraphQL/第二套 Resource·Event Model/V0.2 §3.11 九维资源向量（记 0.7 债务，本期 DEVICE_EXCLUSIVITY=capacity-1 端口语义）/PAUSED 媒体面语义（状态保留，行为 0.7B）。

```

## docs/openspec/changes/p07-session-runtime/design.md

- Source: docs/openspec/changes/p07-session-runtime/design.md
- Lines: 1-31
- SHA256: 8a95e6942cb8a1107415c2d158c97ddbdd20586be563df13c1104a046652dd70

```md
# Design: Phase 0.7A — p07-session-runtime

## Context

0.6 Baseline 上，Session/Resource/Preflight 的契约已冻结（RUNTIME_SESSION_MODEL、RUNTIME_RESOURCE_MODEL、RUNTIME_LIFECYCLE_SEQUENCE、MEDIA_BACKEND_CONTRACT §1.1、V0.2 §1.2/§3.11）而实现缺位（见 proposal Why）。现有可复用件：`SharedResourceRegistry::acquire`（原子 preflight+reserve，P1-4）、Supervisor 全套（Restart/Backoff/Escalate）、`MediaBackend::instantiate/start/stop/recover/observe`（stop 未被调用）、`LeaseManager`（无 renew）、mock 世界（MockProvider/MockBackend）。constraints：分层法则（Domain/编排层不得依赖 adapters 类型）、P0-8（Backend 只消费已授权 Resource）、creator=destroyer 铁律、RELEASED→RUNNING 拒绝。

## Goals / Non-Goals

**Goals:** MediaSession + SessionManager 唯一 owner；冻结生命周期顺序 + 精确逆序回滚真正跑通；Resource 编排闭环（Reserve/Acquire/Allocate/Release/Expire/Recover）；Lease renew；Preflight 分级判定；SESSION-RT-01/RESOURCE-RT-01 三层门禁；单 agent 多会话并发仲裁。
**Non-Goals:** 见 proposal 不做清单；另加：PAUSED 媒体面语义、九维资源向量、多卡 placement/balancing（P1/P2）。

## Decisions

- **D1 Session 结构（RUNTIME_SESSION_MODEL §2-3 + Addendum §4.1）**：`MediaSession` 仅持语义描述与句柄——`session_id: SessionId(Uuid)`（独立于硬件身份）、`graphs: GraphRuntimeIntent`（canonical 语义图）、`source_ports/output_ports: Vec<Uuid>`、`resource_claims: Vec<ResourceClaim{resource_id, holder, phase}>`、`lease: Option<DeviceLease>`、`pipeline: Option<PipelineHandle>`、`health: SessionHealthSnapshot`、`created_at`。绝不含 `gstreamer::Pipeline` 等具体对象（"逻辑持有、物理引用"）。备选"直接持 Arc<dyn MediaBackend>"被否：Backend 对象由 Manager 持有，Session 只拿 `PipelineHandle` 链接（"Session owns lifecycle / Backend owns object / Handle links the two"）。
- **D2 两级状态机（模型 §3 + Addendum §4.3）**：粗态 `SessionState`（Reserved/Running/Paused/Releasing/Released，白名单迁移，RELEASED→Running 拒绝 #114）+ 微相位 `SessionPhase`（Requested/Provisioning/Binding/Leased/Starting/Running/Stopping/Released + ProvisioningFailed/BindingFailed/StartFailed/Degraded/Recovery/Terminated）。两级分离的原因：粗态对外（映射 AgentState/Projection），微相位对内（生命周期引擎步进）。
- **D3 SessionManager 单 owner**：`SessionManager { resources: SharedResourceRegistry, leases: Arc<dyn LeaseManager>, backend: OnceLock<Arc<dyn MediaBackend>>（工厂经 AdapterRegistry）, sup, events, sessions: Mutex<HashMap<SessionId, SessionInner>> , config 旋钮 }`。API：`create(intent)->SessionId`（Preflight→Reserve→建 Session→Lease→Binding verify，失败逆序回滚至 create 前态）、`start(id)->Result`（instantiate→Allocate→start→Running；失败回滚 Allocate→Lease→Reservation→Terminated）、`stop(id)`（stop pipeline→release allocation→release lease→release reservation→Released→销毁）、`close(id)`（幂等收尾）、`status/list`。并发：会话表 Mutex + SharedResourceRegistry 原子性；多会话争用同一 Resource 由 acquire 原子性裁决（后到者 NotAcquirable）。
- **D4 生命周期引擎**：每步显式阶段记录 + `rollback_to(phase)` 表驱动精确逆序（RUNTIME_LIFECYCLE_SEQUENCE §2）；任何一步失败 → 逆序回滚 → `SessionFailed` 事件 → 相位 Terminated；Session 保留供诊断直到 `close`。
- **D5 Preflight 分级（V0.2 §1.2）**：`preflight.rs` 输出 `PreflightReport { stages: Vec<(PreflightStage, StageOutcome)>, verdict }`；0.7A 实装 stages：Graph（intent 形状/设备引用）、PortAvailability（PortRegistry 端口存在 + 方向匹配）、ResourceCapacity（现有 preflight()）、LeaseConflict（lease 已被他人持有）、IdentityBinding（bindings 含目标设备且 HIGH/ManifestVerified）、BackendCapability（probe_capabilities 报告，WARN-only）；Topology/Risk 两级 report-only 占位（WARN 不阻塞）。verdict=FAIL → create 前拒绝，零预留零回滚。
- **D6 Resource 编排补全**：`ResourceRegistry::allocate_for(resource_id, holder)`（Reserved→Allocated，校验 holder 与 reservation.holder 一致）；`release_allocation(holder)`（Allocated→Releasing→Available，供 stop 路径）；`expire_reservations(cutoff)`（供 tick 驱动）。claim 相位与 Resource.state 由 SessionManager 在同一锁域推进。
- **D7 Lease renew + TTL**：`LeaseManager::renew(device_id, owner, ttl)`；SessionManager 提供 `tick()`（由 health 端点/watchdog 周期调用）：lease 到期扫描 + 预留 TTL 过期（`ResourceReservationExpired` 事件）+ 会话健康快照刷新。无后台线程（沿用现模式）。`default_lease_ttl`/`lease_renew_window` 接入 Config。
- **D8 事件**：additive kinds（serde tag 不变）：`SessionCreated{session_id}`、`SessionStateChanged{session_id, from, to}`、`SessionFailed{session_id, reason}`（Critical）；点亮 `LeaseGranted/ResourceAllocated/ResourceReservationExpired/IdentityResolved/SourceMaterialized`。事件仍经 Supervisor::record 单出口。
- **D9 main.rs 接线**：diagnostic auto_start → `SessionManager::create(start intent)` + `start()`（原内联代码删除）；`VBMF_SESSION_LIFECYCLE=1`（bmd+gstreamer）：真机跑 create→start→观察 10s→stop→close 全链，逐步 verdict 输出（SESSION-RT-01 硬件证据入口）；Production Ready 语义不变；selftest 不动。进程常驻期 Manager.tick 挂到现有 watchdog 循环。
- **D10 崩溃恢复**：pipeline 故障仍走既有 Supervisor→recover 链；recover 前强制 `lm.is_valid`（不变量）；Session 相位 Degraded/Recovery 由 watchdog 上报；Supervisor Escalate → 会话 Degraded + ManualRequired（会话不被静默销毁，等运维）。
- **D11 测试三层**：Unit（状态机/回滚表纯逻辑）；Simulation（MockBackend + MockProvider 全链——含 `FailingBackend` 测试桩注入 instantiate/start 失败验证回滚、双会话并发（线程）争抢、expiry、crash cleanup（manager 重建后孤儿预留回收））；Hardware（盒上 VBMF_SESSION_LIFECYCLE + 双会话拒绝）。

## Risks / Trade-offs

- main.rs diagnostic 路径重构触面中等：以既有 MEDIA-RT-01/LOOPBACK/HW-IDENT 回归 + SESSION-RT-01 真机新证据兜底。
- 两级状态机有表达冗余风险：以白名单单测锁死迁移，Phase 映射表唯一。
- Backend 工厂全局单例（OnceLock）：多会话共享同一 backend 实例（MediaBackend Send+Sync，controller 内部 per-handle 状态），符合现架构；多 backend 实例化留 P1。
- `stop` 首次被真实调用：GStreamer stop 路径（Bus watch join + Null + 健康表清理）在真机生命周期门禁中首次实测。

```

## docs/openspec/changes/p07-session-runtime/tasks.md

- Source: docs/openspec/changes/p07-session-runtime/tasks.md
- Lines: 1-55
- SHA256: 02362cd4e1d3ad355df2a60e0269a012f02b2d33a3518c8c58bdaa60ac2a215b

```md
# Tasks: Phase 0.7A — p07-session-runtime

> 纪律（§十五）：每项标注 `Contract / Implementation / Verification / Gate`。Contract=已有（引用冻结文档节号）；Implementation=Not Started/Partial/Complete；Verification=Test/Simulation/Hardware；Gate=Pending/Pass/Blocked。

## 1. Resource Orchestration 补全

- [ ] 1.1 `allocate_for` / `release_allocation` / `expire_reservations`
  - Contract: RUNTIME_RESOURCE_MODEL §3/§4.2（Reservation≠Lease≠Allocation; Available→Reserved→Allocated→Releasing→Available）| Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 1.2 `LeaseManager::renew` + Config 旋钮接线（default_lease_ttl/lease_renew_window）
  - Contract: RUNTIME_RESOURCE_MODEL §4.1（Reservation ops 含 Renew，TTL 强制）| Implementation: Not Started | Verification: Test | Gate: Pending

## 2. Preflight 分级判定（新 preflight.rs）

- [ ] 2.1 六 stage 实装 + Topology/Risk report-only + PASS/WARN/FAIL 裁决（judge-only，FAIL 零预留）
  - Contract: V0.2 §1.2（三层 Preflight; FAIL 禁 Apply）、RUNTIME_LIFECYCLE_SEQUENCE §1（Preflight 先于 Reserve）| Implementation: Not Started | Verification: Test+Simulation | Gate: Pending

## 3. Session 模型 + SessionManager（新 session.rs）

- [ ] 3.1 `MediaSession`（语义持有/物理引用）+ 两级状态机白名单
  - Contract: RUNTIME_SESSION_MODEL §2-3（§114 RELEASED→RUNNING 拒）、Addendum §4.3 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 3.2 `SessionManager` create/start/stop/close/status/list + 冻结顺序生命周期引擎 + 精确逆序回滚
  - Contract: RUNTIME_SESSION_MODEL §4.1（唯一 owner）、RUNTIME_LIFECYCLE_SEQUENCE §1-2（顺序+creator=destroyer+零孤儿）、MEDIA_BACKEND_CONTRACT §1.1（P0-8）| Implementation: Not Started | Verification: Test+Simulation | Gate: Pending
- [ ] 3.3 事件补线（Session* additive + 点亮 LeaseGranted/ResourceAllocated/ResourceReservationExpired/IdentityResolved/SourceMaterialized）
  - Contract: EVENT_CONTRACT §2（session 一等维度）、Addendum §8 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 3.4 Manager.tick（lease 过期扫描/预留 TTL/健康快照）+ Config 接线
  - Contract: RUNTIME_RESOURCE_MODEL §4.2（TTL 强制 + crash cleanup）| Implementation: Not Started | Verification: Test | Gate: Pending

## 4. main.rs 接线

- [ ] 4.1 diagnostic auto-start 改走 SessionManager；Production Ready 不变；selftest 不动
  - Contract: RUNTIME_LIFECYCLE_SEQUENCE §1 | Implementation: Not Started | Verification: Simulation+Hardware | Gate: Pending
- [ ] 4.2 `VBMF_SESSION_LIFECYCLE=1` 真机门禁入口（全链逐步 verdict）
  - Contract: RUNTIME_LIFECYCLE_SEQUENCE §1 | Implementation: Not Started | Verification: Hardware | Gate: Pending

## 5. 门禁 SESSION-RT-01 / RESOURCE-RT-01（三层）

- [ ] 5.1 SESSION-RT-01 Unit/Simulation（全链+回滚+double-start/stop 拒绝，FailingBackend 桩）
  - Contract: RUNTIME_SESSION_MODEL §5（#145 验收场景：create/start/stop/crash/recover/release/double-start/double-stop/lease conflict/resource conflict）| Implementation: Not Started | Verification: Test+Simulation | Gate: Pending
- [ ] 5.2 RESOURCE-RT-01 Unit/Simulation（并发争抢/容量/冲突/release/expiry/crash cleanup）
  - Contract: RUNTIME_RESOURCE_MODEL §4.1-4.2、V0.2 §3.11 DEVICE_EXCLUSIVITY | Implementation: Not Started | Verification: Test+Simulation | Gate: Pending
- [ ] 5.3 两门禁真机层（盒上 lifecycle 全链 + 双会话拒绝）
  - Contract: 同上 | Implementation: Not Started | Verification: Hardware | Gate: Pending

## 6. CI + 交付

- [ ] 6.1 CI 新增 `session-lifecycle` required job（mock feature session+resource 门禁测试）+ protection 七 context
  - Contract: 用户 §十七（新增 required checks，不降六项）| Implementation: Not Started | Verification: CI | Gate: Pending
- [ ] 6.2 盒上全矩阵（含 fmt check + hardware-test build）全绿
  - Contract: 教训口径（盒上绿≠CI绿）| Implementation: Not Started | Verification: Box+CI | Gate: Pending
- [ ] 6.3 verify（full，四栏纪律表）→ archive → PR → merge → 删分支
  - Contract: 新分支纪律 | Implementation: Not Started | Verification: CI+Review | Gate: Pending

## 收口确认

- 编号说明：0.7A=Runtime Ownership（新分段，替代 MASTER_PRD §5 旧 0.7A-G 标签；PRD 不改）；本 change 完成 0.6A/0.6E 冻结契约实现债务。

```
