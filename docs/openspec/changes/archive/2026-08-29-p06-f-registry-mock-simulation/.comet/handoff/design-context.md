# Comet Design Handoff

- Change: p06-f-registry-mock-simulation
- Phase: design
- Mode: compact
- Context hash: e7d0c19c1584b2961208f0755cbdbf54adba760c9c8869f0927edc555f3fb41a

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p06-f-registry-mock-simulation/proposal.md

- Source: docs/openspec/changes/p06-f-registry-mock-simulation/proposal.md
- Lines: 1-21
- SHA256: 6edc6264cdd1c6f2408cdd08d4830a87c46fcda1de4509924feefed24b708d2a

```md
# Change: Phase 0.6 C3 — Registry / Mock / Simulation (0.6F)

## Why

- 0.6B+C 已冻 SPI，但只有真实 BMD/GStreamer 适配器；CI 与单测无法在无硬件/无 SDK 环境覆盖采集/信号逻辑。
- 缺统一的 Adapter Registry：无法按配置在 Mock 与真实适配器间选择，也无法为 0.6G/0.6H/I 门禁提供可切换实现。
- 0.6D 的 `RuntimeEvent` 需要可注入的仿真源来验证 Supervisor 行为。

## What is Changing

- 新增 `providers/mock/`：实现 `HardwareProvider` 的 Mock（可注入设备清单/身份/故障），无 BMD SDK 依赖；
- 新增 `backends/mock/`：实现 `MediaBackend` 的 Mock（可注入 SourcePlan→事件流），无 GStreamer 依赖；
- 新增 `registry.rs`：`AdapterRegistry` 按配置（`simulation` feature / 环境变量 / manifest）选择 Provider/Backend 实现；
- feature `simulation` 编译 Mock 适配器并接入 Registry；`default` 仍最小可编译；
- Mock 发出的事件遵循 C2 的 `RuntimeEvent` 契约（为 0.6G/0.6H/I 门禁提供可测实现）。

## Impact

- 编译：`default` / `simulation` 单测保持通过；`--features simulation` 可跑 Mock 采集链路；
- 不受影响：真实 BMD/GStreamer 适配器（0.6B+C）、V0.2 核心定义、canonical 管线语义；
- 后续铺垫：0.6G 解耦门禁（用 Mock 验证 Domain 编译）、0.6H/I 门禁（Mock vs 真实共享 CanonicalPipelinePlan）。

```

## docs/openspec/changes/p06-f-registry-mock-simulation/design.md

- Source: docs/openspec/changes/p06-f-registry-mock-simulation/design.md
- Lines: 1-41
- SHA256: 5ac61059581238e3da256f6086767f144d165fc7c5d97782d213a21978588d2a

```md
---
title: "Phase 0.6 C3 (0.6F): Registry / Mock / Simulation — 技术设计"
change: p06-f-registry-mock-simulation
change_id: p06-f-registry-mock-simulation
comet_change: p06-f-registry-mock-simulation
role: technical-design
spec: openspec
canonical_spec: openspec
links:
  - "[p06-f-registry-mock-simulation](p06-f-registry-mock-simulation)"
---

# Design: Phase 0.6 C3 — Registry / Mock / Simulation

## Mock Provider / Backend

- `providers/mock/` 实现 `HardwareProvider`：`discover()` 返回注入的设备清单；`open()` 返回可配置 DeviceHandle；`identity()` 返回注入的 CanonicalDeviceId；可注入故障（如 AmbiguousIdentity）以验证 C2 的拒识路径。
- `backends/mock/` 实现 `MediaBackend`：`materialize(plan)` 返回注入的 Pipeline（不触碰 GStreamer）；`src_props()` 返回与真实一致的 `connection=` 片段（audio 不设 connection）；按 plan 发出 C2 的 `RuntimeEvent` 序列（IdentityResolved→SourceMaterialized→SignalVerified…）。

## AdapterRegistry

- `registry.rs::AdapterRegistry` 持有 Provider/Backend 工厂；按解析顺序：显式配置 > `simulation` feature > 默认（真实）；
- 与 0.6B+C 的 trait 配合：Registry 只返回 `HardwareProvider` / `MediaBackend` trait 对象，调用方不感知具体适配器；
- 真机构建（`bmd-provider,gstreamer-backend`）仍走真实适配器；`simulation` 走 Mock。

## feature 映射

- `simulation` 编译 `providers/mock/` + `backends/mock/` + 接入 Registry；`default` 不含 Mock（最小可编译）；
- 与现有 `simulation` feature（已有 mock 设备）合并语义，避免重复。

## 关键约束

- Mock 必须复用 C2 的 `RuntimeEvent` 契约，不得另起事件类型；
- canonical 管线语义保持不变（Mock 仅仿真，不改语义）；
- 防自动 Fallback：Registry 选择失败须走 C2 的 Preflight/Policy，绝不静默换适配器。

## 不做（本 change 边界）

- 不做架构 lint 门禁（0.6G）；
- 不做 HW-PORT-01 真机回路（0.6H/I）；
- 不做 Scheduler（仅冻结契约）。

```

## docs/openspec/changes/p06-f-registry-mock-simulation/tasks.md

- Source: docs/openspec/changes/p06-f-registry-mock-simulation/tasks.md
- Lines: 1-28
- SHA256: ed55d3ee10a268e1e1cd460f4f86317079ee379b0835e28863729db73dcaf583

```md
# Tasks: Phase 0.6 C3 (0.6F)

## 1. Mock Provider

- [ ] 新增 `providers/mock/` 实现 `HardwareProvider`（注入设备/身份/故障）
- [ ] Mock `identity()` 返回注入 CanonicalDeviceId；支持注入 AmbiguousIdentity 以验证拒识

## 2. Mock Backend

- [ ] 新增 `backends/mock/` 实现 `MediaBackend`（注入 SourcePlan→事件流）
- [ ] Mock `src_props()` 返回与真实一致的 `connection=` 片段（audio 不设 connection）
- [ ] 按 plan 发出 C2 `RuntimeEvent` 序列

## 3. AdapterRegistry

- [ ] 新增 `registry.rs::AdapterRegistry`，按配置选择 Provider/Backend 实现
- [ ] 与 0.6B+C trait 配合：只暴露 trait 对象

## 4. feature 接入

- [ ] `simulation` feature 编译 Mock 并接入 Registry；`default` 仍最小可编译
- [ ] 与现有 simulation mock 设备语义合并

## 5. 验证（CI 门禁）

- [ ] `cargo clippy --all-targets -- -D warnings`（default + simulation + `bmd-provider,gstreamer-backend`）
- [ ] `cargo test` default + simulation 通过
- [ ] `cargo build --features simulation` 通过且 Mock 链路可启动

```
