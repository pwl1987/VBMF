# Comet Design Handoff

- Change: p06-de-runtime-event-resource
- Phase: design
- Mode: compact
- Context hash: 1753077fe9f365087a935a5065b5d6a1c2884c897d710eb385acebed4893f0e9

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p06-de-runtime-event-resource/proposal.md

- Source: docs/openspec/changes/p06-de-runtime-event-resource/proposal.md
- Lines: 1-21
- SHA256: 5290375eb9a4ca537c2f28b64afe1b51f71985352c8fe39b0566e56b324e8809

```md
# Change: Phase 0.6 C2 — RuntimeEvent / Supervisor 统一 + Resource / Preflight (0.6D+E)

## Why

- 当前 `supervisor.rs` / `health.rs` 直接使用 GStreamer `Message` / HRESULT 等 vendor 错误，违反 Canonical Error-Event 契约；事件源分散，难以做统一可观测与回放。
- 缺 Resource 模型：`materialize` 前不校验设备/端口可用性，易出现「自动 Fallback 到错误硬件」——而 ARCH 防回退门禁要求 **Policy + Capability + Preflight + 决策**，绝不静默换硬件。
- 0.6F（Mock）、0.6G（解耦门禁）、0.6H/I（多门禁）都依赖统一的 `RuntimeEvent` 与 `Resource` 抽象作为前提。

## What is Changing

- 新增 `events.rs`：`RuntimeEvent` 枚举（IdentityResolved / SourceMaterialized / SignalVerified / LoopbackVerified / LeaseGranted / ResourceAllocated / PipelineFault / HardwareFault / HealthChanged / AmbiguousIdentity），取代散落的 vendor 错误；`supervisor.rs` 成为唯一事件源。
- 重构 `supervisor.rs`：消费 Provider/Backend 上抛的归一化事件，统一发出 `RuntimeEvent`；Rust 错误类型不再携带 BMD/GStreamer specifics。
- 新增 `resource.rs`：`Resource` 模型（Capability / Capacity / Availability / Allocation / Reservation），状态机 Available→Reserved→Allocated→Releasing→Faulted，对齐 V0.2 §3.11；**Resource ≠ Device**。
- 新增 Preflight：`materialize` 前做 Resource 可用性校验 + 防自动 Fallback（失败须 Policy+Capability+Preflight+决策，绝不静默换硬件）。
- 不改 JSON-RPC 契约、V0.2 核心定义、canonical 管线语义。

## Impact

- 编译：`default` / `simulation` 单测保持通过；
- 受影响：Supervisor/Health 事件链路、`materialize` 入口（加 Preflight 闸门）；Provider/Backend 错误须映射到 `RuntimeEvent`；
- 后续铺垫：0.6F Mock（用 `RuntimeEvent` 仿真）、0.6G 解耦门禁、0.6H/I 的 Resource-01 / HW-PORT-01 / HW-IDENT-02 / MEDIA-RT-01。

```

## docs/openspec/changes/p06-de-runtime-event-resource/design.md

- Source: docs/openspec/changes/p06-de-runtime-event-resource/design.md
- Lines: 1-42
- SHA256: 73b094228c9be537c37d472168485a111ecce2201abfc3efd92b6dd23b3dfa2f

```md
---
title: "Phase 0.6 C2 (0.6D+E): RuntimeEvent / Resource — 技术设计"
change: p06-de-runtime-event-resource
change_id: p06-de-runtime-event-resource
comet_change: p06-de-runtime-event-resource
role: technical-design
spec: openspec
canonical_spec: openspec
links:
  - "[p06-de-runtime-event-resource](p06-de-runtime-event-resource)"
---

# Design: Phase 0.6 C2 — RuntimeEvent / Resource

## RuntimeEvent（草拟）

- 枚举成员覆盖生命周期：IdentityResolved / SourceMaterialized / SignalVerified / LoopbackVerified / LeaseGranted / ResourceAllocated / ResourceReservationExpired / PipelineFault / HardwareFault / HealthChanged / AmbiguousIdentity(device_handle, candidates)；
- `supervisor.rs` 订阅 Provider/Backend 归一化事件，作为唯一 `RuntimeEvent` 出口；下游（Health / RPC / 日志）只消费 `RuntimeEvent`，不再直接碰 vendor 类型。

## Resource 模型（对齐 V0.2 §3.11）

- `Resource { id, capability: CapabilityValue<…>, capacity, availability, allocation, reservation }`；状态机：Available → Reserved → Allocated → (Releasing | Faulted)；
- Resource 是对 Capability 的抽象（≠ Device/Port）；一个 Device 可暴露多个 Resource；
- 与 `DeviceCapabilities`(`port.rs`) / `PortRegistry` 衔接：Resource 由 Discovery 结果派生。

## Preflight（防自动 Fallback）

- `materialize` 入口前置 `preflight(plan, resources)`：校验目标 Resource 可用 + 无冲突预留 + 身份已 Resolve；
- 失败路径：返回 `RuntimeEvent::AmbiguousIdentity` / `ResourceUnavailable`，由上层 Policy 决策；**绝不静默回退到 device 0 或另一硬件**；
- 与 0.6B+C 的 Provider/Backend SPI 配合：Provider 暴露的 identity 作为 Preflight 输入。

## 关键约束（来自 CODEBUDDY.md）

- 多重 HIGH → `Ambiguous`（拒）；`device-number` 绝不默认 0；
- MEDIA-RT-01：`pts_monotonic` 只置 false；`PipelineHealth` Default=true；
- 不改 V0.2 的 12 Engines / Switch Mode 3 种 / Data Plane 4 Layer。

## 不做（本 change 边界）

- 不做 Mock Provider/Backend（0.6F）；
- 不做架构 lint 门禁（0.6G）；
- 不做 HW-PORT-01 真机回路（0.6H/I）。

```

## docs/openspec/changes/p06-de-runtime-event-resource/tasks.md

- Source: docs/openspec/changes/p06-de-runtime-event-resource/tasks.md
- Lines: 1-27
- SHA256: e5e1c0c116eb9d1524a7f878ddf425fa69e5bb3f0117669d067e7b80797897f7

```md
# Tasks: Phase 0.6 C2 (0.6D+E)

## 1. RuntimeEvent 模型

- [ ] 新增 `events.rs` 定义 `RuntimeEvent` 枚举（全生命周期成员）
- [ ] 定义 vendor 错误 → `RuntimeEvent` 的映射 trait / 辅助

## 2. Supervisor 归一化

- [ ] `supervisor.rs` 改为唯一 `RuntimeEvent` 出口，消费 Provider/Backend 上抛事件
- [ ] Health / RPC / 日志改为只消费 `RuntimeEvent`，移除直接 vendor 错误依赖

## 3. Resource 模型

- [ ] 新增 `resource.rs`：`Resource` + 状态机（Available→Reserved→Allocated→Releasing→Faulted），对齐 V0.2 §3.11
- [ ] Resource 由 Discovery（`DeviceCapabilities` / `PortRegistry`）派生

## 4. Preflight 闸门

- [ ] `materialize` 入口前置 `preflight(plan, resources)`：可用性 + 冲突预留 + 身份 Resolve 校验
- [ ] 失败返回 `AmbiguousIdentity` / `ResourceUnavailable`，由 Policy 决策；禁止静默回退

## 5. 验证（CI 门禁）

- [ ] `cargo clippy --all-targets -- -D warnings`（default + `bmd-provider,gstreamer-backend` 两套 feature）
- [ ] `cargo test` default + simulation 通过
- [ ] `cargo build --features bmd-provider,gstreamer-backend` 通过

```
