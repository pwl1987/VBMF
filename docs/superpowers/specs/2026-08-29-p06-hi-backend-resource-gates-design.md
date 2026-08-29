---
comet_change: p06-hi-backend-resource-gates
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-29-p06-hi-backend-resource-gates
status: final
---

# Design Doc — p06-hi-backend-resource-gates（Phase 0.6 C5 / 0.6H+I: 后端/资源/真机验收门禁组）

> 本文是 open 阶段 `design.md` 高层框架的深度技术细化：逐项给出实现设计、复用点、技术风险、测试策略与边界条件。canonical 需求以 OpenSpec change 为准，本文不重复需求。

## 0. 目标与依赖前提（已核实）

本 change 落地第二批验收门禁组（ARCH-BACKEND-01 / RESOURCE-01 / HW-PORT-01 / HW-IDENT-02 / MEDIA-RT-01），作为 CI required gate 与 0.6G（ARCH-PORTABILITY-01）并列；全绿后 BMD/GStreamer 方可降级为 Reference Adapter 并进入 Normalize(0.7)。

依赖（均已在本分支 `b9333e4` 前驱链落地，已逐一核实存在）：
- **0.6B+C（p06-bc）**：`contracts/backend.rs` 的 `trait MediaBackend: Send + Sync`；`adapters/mock.rs::MockBackend` 与 `adapters/gstreamer/controller.rs::GStreamerPipelineController` 均 `impl MediaBackend` 且 `prepare(&PipelinePlan)`。
- **0.6F（p06-f）**：`MockProvider`/`MockProviderB`/`MockBackend`（Test C Provider 侧已存在）。
- **0.6C2（p06-de）**：`resource.rs` 的 `Resource`/`ResourceState`/`ResourceRegistry`/`preflight`/`resolve_identity` + `main.rs` materialize 入口 Preflight 闸门（RESOURCE-01 直接复用）。
- **0.6G（p06-g）**：CI 门禁框架（`media-agent.yml` Gate A：fmt / 词法 lint / build / clippy / simulation+mock 编译门禁 / test default+sim）+ `scripts/check_arch_portability.py`。

## 1. ARCH-BACKEND-01（Mock 与 GStreamer 共享 CanonicalPipelinePlan，互可替换）

**实现设计**
- 架构事实（已核实）：两个 `MediaBackend` 实现都从同一 `PipelinePlan`（canonical plan）物化，`prepare(&PipelinePlan) -> Result<PipelineHandle>` 是统一入口；Domain/Graph/Session/Supervisor/Health 只依赖 `MediaBackend` trait 与 `PipelinePlan` 类型，不依赖具体 backend 类型。
- 门禁断言（Test C 延伸到 Backend 侧）：新增 `adapters/` 或 `tests/` 级测试 `backend_swap_shares_canonical_plan`：
  - 用同一 `PipelinePlan::self_test()`（canonical）分别交给 `MockBackend` 与（feature 门控下）`GStreamerPipelineController::prepare`，断言两者都返回 `Ok(PipelineHandle)` 且 plan 的 canonical 字段（source/normalize/switch_mode）未被 backend 篡改；
  - 断言 `MockBackend` 与 `GStreamerPipelineController` 都实现 `MediaBackend`（trait 对象 `Box<dyn MediaBackend>` 可互换编译）。
- 落地位置：纯 Rust 断言，**无新增 public API**；GStreamer 侧断言用 `#[cfg(feature = "gstreamer-backend")]` 门控（CI 无 GStreamer 运行时则该侧仅编译期 trait 断言，运行期断言在盒上/真机跑）。

**风险**：GStreamer `prepare` 需真实 GStreamer 运行时；CI 无运行时 → 该侧只做 trait-object 编译断言 + Mock 侧运行断言，运行期对等性由盒上/真机闭环补证。**边界**：`PipelinePlan` 是不可变 canonical 输入，backend 不得回写；断言只读比较。

## 2. RESOURCE-01（materialize 前 Preflight 闸门，绝不静默回退）

**实现设计**
- 直接复用 p06-de 的 `resource.rs::preflight` + `main.rs` 入口闸门（已落地）。本门禁**不新增模型**，只做"闸门存在且 fail-closed"的验收断言。
- 断言测试（`resource.rs` 已有 `preflight_grants_available_matching_resource` / `preflight_rejects_missing_mismatched_or_busy`，补 materialize 级集成断言）：
  - 目标 Resource `Reserved`/`Faulted` → `preflight` 返回 `NotAcquirable`，materialize 拒绝（不盲开 device 0）；
  - 目标 Resource 不存在（Discovery 未派生）→ `ResourceUnavailable`；
  - 身份多重候选 → `AmbiguousIdentity`（拒识，交 Policy，绝不静默择一）。
- 落地位置：`resource.rs` 测试 + （可选）`main.rs` 闸门路径的 `#[cfg(test)]` 断言；**无新增 public API**。

**风险**：低（纯复用 + 测试）。**边界**：`capacity<1` 显式拒；`Releasing` 态不可被新占用（不抢占）。

## 3. HW-PORT-01（端口级绑定闭环）

**实现设计**
- 复用 `hw_port_01.rs::verify(&PortRegistry, &DeviceBindingManifest) -> HwPort01Report`（已落地，遍历 manifest 端口，实际 rank < 声明 rank ⇒ 失败闭环）+ `signal.rs::verify_fixtures`（亮度黑场 / 格式匹配 / state=locked）。
- 门禁 = 单测断言（`hw_port_01` 已有 5 个 pass/fail 测试）+ **真机闭环**：manifest 声明端口 → 实际探测（loopback）→ 闭环验收。
- 真机事实（design 已固化）：loopback = **MiniMon sink2 → Duo capture0**；双门基线 **default+sim 84 / bmd 83**。
- 落地位置：无新增 public API；真机闭环经 `box_verify_*.sh` / `loopback_run.sh` 在盒上执行（需硬件）。

**风险**：真机闭环依赖硬件 + 信号路由；CI 无硬件时只跑 Mock/单测侧，真机侧在盒上补证。**边界**：`no_signal` 不解释为非输入（fail-closed，`signal.rs` 已覆盖）。

## 4. HW-IDENT-02（身份解析正确性）

**实现设计**
- 复用 `resolver.rs` 身份优先级（已落地）：`PersistentIdExact`(HIGH) > `DeviceHandleExact`(HIGH, 当前硬件路径) > `TopologicalIdGuess`(MEDIUM) > `EnumerationOnly`；多重 HIGH → `Ambiguous`（拒）；`device-number` 绝不默认 0。
- 门禁断言（`resolver.rs` 已有大量 manifest/identity fail-closed 测试）：
  - 多重 HIGH 候选 → `Ambiguous`（拒识）；
  - 单一 HIGH → 解析成功；
  - 无候选 → `ResourceUnavailable`（不伪造）；
  - 显式 `port_id` 不在 registry → 拒（`materialize_rejects_explicit_port_id_missing_in_registry_production` 已覆盖）。
- 落地位置：`resolver.rs` 测试补全（若已有则引用）；**无新增 public API**。

**风险**：`DeviceHandle == GStreamer hw-serial-number` 关系在 resolver.rs 头部标注为"尚未最终证据"（⚠️）——这是已知未决项，本门禁按"当前硬件路径 = DeviceHandleExact"推进，最终证据由真机闭环补。**边界**：Unknown ordinal 不派生稳定 ID（`port.rs` 已覆盖）。

## 5. MEDIA-RT-01（实时性契约）

**实现设计**
- 复用 `pipeline.rs`（已落地）：
  - `pts_monotonic` 三态（`Unknown`/`Monotonic`/`NonMonotonic`，`PipelineHealth` 字段），**只置 false**（`NonMonotonic` 时 `b4_pts_monotonic=false`；`Unknown` 不压成 false）——已由 `pipeline.rs` 三态模型 + 测试覆盖；
  - `PipelineHealth: Default`（`Default=true` 语义，`impl Default` 已存在）；
  - `MEDIA_AGENT_SELFTEST=1` 跑通 `PipelinePlan::self_test()`（`selection_mode: SelfTest`）即 A+B+C（`first_video && first_audio && valid_pts && pts_monotonic`）。
- 门禁断言：`pipeline.rs` 已有 `non_monotonic_is_sticky` / `pts_state_starts_unknown` / `first_frame_ok_requires_both_valid` 等测试；补 `PipelineHealth::default()` 断言 + self_test A+B+C 路径断言（Mock/盒上）。
- 落地位置：`pipeline.rs` 测试补全；**无新增 public API**。

**风险/边界**：`self_test()` 的 `device_number: 0` 是**自测哨兵**（无真实设备，videotestsrc/audiotestsrc），**不违反**"device-number 绝不默认 0"（该约束针对真实设备选择不得静默落到 DeckLink 0 号）；需在测试/注释中显式区分，避免误读为违反不变量。

## 6. 门禁接入 CI（与 0.6G 并列）

**实现设计**
- 在 `.github/workflows/media-agent.yml` 的 Gate A 组内新增本门禁组的断言步骤（复用现有 `cargo test` 目标——各门禁断言均为 Rust 单测，随 `cargo test`（default/simulation）执行）；显式加 `ARCH-BACKEND-01` / `RESOURCE-01` 等的注释标记 + （可选）独立 `cargo test --features mock` / `--features bmd,gstreamer` 步骤使各门禁可追溯。
- 真机闭环（HW-PORT-01 / MEDIA-RT-01 的硬件侧）不在纯 CI 跑（无硬件），经盒上 `box_verify_*.sh` + `loopback_run.sh` 执行，基线 default+sim 84 / bmd 83 全绿。
- 落地位置：`media-agent.yml`（CI 配置）+ 各模块 `#[cfg(test)]` 断言。

**风险**：CI 新增步骤需保持 `cargo clippy --all-targets -- -D warnings` 全绿（无新增 warning）。**边界**：新增测试须对 default/simulation/mock 三套 feature 均可编译（feature 门控正确）。

## 7. 测试策略汇总

| 门禁 | 单测（CI 可跑） | 盒上/真机 | 复用 |
|------|----------------|-----------|------|
| ARCH-BACKEND-01 | backend-swap + trait-object 断言（Mock 侧运行 + GStreamer 侧编译门控） | 盒上 GStreamer 运行期对等 | p06-bc trait / p06-f mock |
| RESOURCE-01 | preflight fail-closed + materialize 拒（3 态） | — | p06-de preflight |
| HW-PORT-01 | hw_port_01 pass/fail（已有 5 测试） | 真机 loopback（MiniMon sink2→Duo cap0） | hw_port_01 / signal |
| HW-IDENT-02 | resolver 多重 HIGH→Ambiguous / 无候选拒 / 显式 port 拒 | 真机身份核验 | resolver |
| MEDIA-RT-01 | PipelineHealth default + 三态 + self_test A+B+C | `MEDIA_AGENT_SELFTEST=1` 盒上跑通 | pipeline |
| CI 接入 | `cargo clippy -D` 0 warning + test 三套 feature | 真机基线 84/83 全绿 | p06-g 框架 |

## 8. 不做（本 change 边界）

- 不进 Normalize(0.7)（须本门禁组全绿后才允许）；
- 不新增硬件适配器（AJA 等 P2）；
- 不改 `MediaBackend`/`HardwareProvider` trait 签名（只加断言，不动 SPI 契约）；
- 不新增 public API（全部门禁为"断言现有不变量 + CI 接线 + 真机闭环"）。

## 9. 实施顺序建议

1. RESOURCE-01 断言（纯复用 p06-de，最小）→ 2. HW-IDENT-02 断言（resolver 已有，补全）→ 3. MEDIA-RT-01 断言（pipeline 补 default + self_test）→ 4. ARCH-BACKEND-01 断言（backend swap）→ 5. CI 接线（media-agent.yml）→ 6. 盒上三套 feature + 真机 loopback 基线核验。
