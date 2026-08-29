# Comet Design Handoff

- Change: p06-hi-backend-resource-gates
- Phase: design
- Mode: compact
- Context hash: a3d8937a170f033b2227f967dd48a5448b378c4d1f3d6b5e111370b7247816cd

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p06-hi-backend-resource-gates/proposal.md

- Source: docs/openspec/changes/p06-hi-backend-resource-gates/proposal.md
- Lines: 1-22
- SHA256: 11346275a11c224e47f6194a89195e32941d3f08b5eeca5d7829e8e2aeb9b42e

```md
# Change: Phase 0.6 C5 — 后端/资源/真机验收门禁 (0.6H+I)

## Why

- 裁决顺序在过 ARCH-PORTABILITY-01（0.6G）后，需再过第二批门禁，才允许「降级 BMD/GStreamer 为 Reference Adapter」并进入 Normalize(0.7)。
- 缺这批门禁，无法证明：Mock 与真实后端共享同一 CanonicalPipelinePlan（ARCH-BACKEND-01）、Resource 可用性闸门（RESOURCE-01）、端口级绑定闭环（HW-PORT-01）、身份解析正确（HW-IDENT-02）、实时性契约（MEDIA-RT-01）。

## What is Changing

- 新增门禁组（CI required gate，与 0.6G 并列）：
  - ARCH-BACKEND-01：Mock vs GStreamer 实现共享同一 `CanonicalPipelinePlan`，互可替换；
  - RESOURCE-01：materialize 前 Resource 可用性闸门（复用 C2 Resource + C4 门禁框架）；
  - HW-PORT-01：端口级绑定闭环验收（loopback 验证，复用 `hw_port_01` / `signal` 亮度黑场检测）；
  - HW-IDENT-02：身份解析正确性（PersistentId > DeviceHandle > TopologicalId 优先级，多重 HIGH → Ambiguous）；
  - MEDIA-RT-01：实时性契约（`pts_monotonic` 只置 false；`PipelineHealth` Default=true；`MEDIA_AGENT_SELFTEST=1` 跑通即 A+B+C）。
- 门禁以「真机 + Mock 双轨」形式落地，复用 C3 的 Mock 适配器与 C4 的架构断言框架。

## Impact

- 编译：`default` / `simulation` / `bmd-provider,gstreamer-backend` 三套均须保持可编译且单测通过；
- 受影响：验收模式（`diagnostic` / `VBMF_RESOLVER=1` / `MEDIA_AGENT_SELFTEST=1`）的断言须对齐上述门禁；
- 后续：门禁全绿后，BMD/GStreamer 正式降级为 Reference Adapter，方可进 Normalize(0.7)。

```

## docs/openspec/changes/p06-hi-backend-resource-gates/design.md

- Source: docs/openspec/changes/p06-hi-backend-resource-gates/design.md
- Lines: 1-35
- SHA256: 49415988aa49d620f346fedbd84cdeabf47ce3a94f9d8b3d0bfcc589621a351b

```md
# Design: Phase 0.6 C5 — 门禁组

## ARCH-BACKEND-01

- Mock(`backends/mock/`) 与 GStreamer(`backends/gstreamer/`) 实现同一 `MediaBackend` trait，且都从同一 `CanonicalPipelinePlan` 物化；
- 断言：交换 Backend 实现，Domain/Graph/Session/Supervisor/Health 行为一致（Test C 延伸）。

## RESOURCE-01

- 复用 C2 `Resource` 模型 + C4 门禁框架：materialize 前 `preflight` 校验 Resource 可用；不可用则拒（绝不静默回退）。

## HW-PORT-01

- 端口级绑定闭环：`manifest` 声明端口 → 实际探测（亮度黑场 / 格式匹配 / state=locked）→ 闭环验收；
- 复用 `hw_port_01::verify` 遍历 manifest 端口，实际 rank < 声明 rank ⇒ 失败闭环。

## HW-IDENT-02

- 身份优先级 PersistentId > DeviceHandle > TopologicalId > EnumerationOnly；多重 HIGH → `Ambiguous`（拒）；`device-number` 绝不默认 0；
- 复用 `resolver.rs` 的 `set_state(READY)` 遍历 + 禁 `GstDeviceMonitor`。

## MEDIA-RT-01

- `pts_monotonic` 只置 false；`PipelineHealth` Default=true；`MEDIA_AGENT_SELFTEST=1` 跑通即 A+B+C；appsink 仅 observer。

## 关键约束（来自 CODEBUDDY.md / 真机核验）

- `connection` nick：`optical-sdi` 非 `optical`；audio audiosrc 不设 connection；
- 已核验官方常量随 FFI 在 Provider 内，改前对盒 SDK 头 grep；
- 真机闭环事实：loopback = MiniMon sink2 → Duo capture0；双门全绿基线 default+sim 84 / bmd 83。

## 不做（本 change 边界）

- 不进 Normalize(0.7)（须本门禁组全绿后才允许）；
- 不做新增硬件适配器（AJA 等 P2）。

```

## docs/openspec/changes/p06-hi-backend-resource-gates/tasks.md

- Source: docs/openspec/changes/p06-hi-backend-resource-gates/tasks.md
- Lines: 1-31
- SHA256: 3bd25b873b1cedf60597449be7df5bd0cbbaa9f5a2838283cbbc3879e07ed81c

```md
# Tasks: Phase 0.6 C5 (0.6H+I)

## 1. ARCH-BACKEND-01

- [ ] 断言 Mock 与 GStreamer 共享 `CanonicalPipelinePlan`，互可替换（Test C 延伸）

## 2. RESOURCE-01

- [ ] materialize 前 `preflight` 校验 Resource 可用；不可用则拒，绝不静默回退

## 3. HW-PORT-01

- [ ] 端口级绑定闭环（`hw_port_01` 遍历 manifest，实际 rank < 声明 ⇒ 失败）

## 4. HW-IDENT-02

- [ ] 身份优先级 PersistentId>DeviceHandle>TopologicalId；多重 HIGH→Ambiguous；device-number 绝不默认 0

## 5. MEDIA-RT-01

- [ ] `pts_monotonic` 只置 false；`PipelineHealth` Default=true；`MEDIA_AGENT_SELFTEST=1` 跑通 A+B+C

## 6. 门禁接入 CI + 真机闭环

- [ ] 门禁组列为 required gate（与 0.6G 并列）
- [ ] 真机 `cargo build --features bmd,gstreamer` + loopback 双门全绿（基线 default+sim 84 / bmd 83）

## 7. 验证

- [ ] `cargo clippy --all-targets -- -D warnings`（三套 feature）通过
- [ ] `cargo test` default + simulation 通过

```
