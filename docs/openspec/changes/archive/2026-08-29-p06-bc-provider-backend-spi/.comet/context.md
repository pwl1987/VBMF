# Comet Design Handoff

- Change: p06-bc-provider-backend-spi
- Phase: design
- Mode: compact
- Context hash: 63665752566dc67fe65a9f898bc87a7d0f9cdda55ee9503fdf54cc157ce06ce0

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p06-bc-provider-backend-spi/proposal.md

- Source: docs/openspec/changes/p06-bc-provider-backend-spi/proposal.md
- Lines: 1-25
- SHA256: 0b4d5d596e22568431be9c9ac514d681e54da690c7ab38e5dbd7d786ecdf8311

```md
# Change: Phase 0.6 C1 — HardwareProvider / MediaBackend SPI 抽取 (0.6B+C)

## Why

`media-agent` 当前在 `main.rs` / `resolver.rs` / `signal.rs` / `pipeline.rs` 中**直接依赖 `decklink`（BMD FFI）与 `gstreamer` crate**，违反 V0.2 四层抽象（Domain → Contract/Runtime → Adapter）。后果：

- 无法在删去真实硬件依赖时编译 Domain/Graph/Session/Supervisor/Health —— 即 ARCH-PORTABILITY-01 门禁的前提不满足；
- 难以接入 Mock Provider 做仿真矩阵（0.6F）与统一 RuntimeEvent（0.6D）；
- vendor 错误类型（HRESULT / GStreamer Message）渗入 Supervisor，破坏 Canonical Error-Event 契约。

Phase 0.6 裁决顺序：先冻契约 → 过两道门禁 → 降级 BMD/GStreamer 为 Reference Adapter → 才进 Normalize(0.7)。C1 是“冻契约 + 建 SPI”的第一步。

## What is Changing

- 新增 `providers/` 模块：`providers/blackmagic/` 承载 BMD FFI（`decklink.rs` / `sdk.rs` 迁入），对外**仅暴露 `HardwareProvider` trait**；
- 新增 `backends/` 模块：`backends/gstreamer/` 承载 GStreamer 采集执行（`pipeline.rs` 的媒体路径迁入），对外**仅暴露 `MediaBackend` trait**；
- 定义 `HardwareProvider` / `MediaBackend` trait（Provider/Runtime 契约层）；`main` / `resolver` / `signal` / `pipeline` 改为依赖 trait 而非 crate 顶层；
- feature 重构：`bmd` → `bmd-provider`，`gstreamer` → `gstreamer-backend`（保留兼容或平滑迁移）；
- canonical 采集管线（`decklinkvideosrc`+`decklinkaudiosrc` → RAW → Normalize → FRAME/MASTER SWITCH → Encode → 分发）仍由 `GStreamerBackend` 实现，身份/探测逻辑迁入 Provider。

## Impact

- 编译：`default` / `simulation` 单测保持通过；`--features bmd-provider,gstreamer-backend` 真机构建保持可用；
- 不受影响：V0.2 核心定义、JSON-RPC 契约、Resolver 身份优先级（PersistentId > DeviceHandle > TopologicalId）、canonical 管线语义；
- 后续铺垫：0.6D RuntimeEvent/Supervisor、0.6E Resource/Preflight、0.6F Mock Provider/Backend、0.6G 架构门禁。

```

## docs/openspec/changes/p06-bc-provider-backend-spi/design.md

- Source: docs/openspec/changes/p06-bc-provider-backend-spi/design.md
- Lines: 1-117
- SHA256: 5f0cf1dfdc1af42553d853130b8d4826ece31bb4c0848b5cb7076fd2534e9cf1

[TRUNCATED]

```md
---
title: "Phase 0.6 C1 (0.6B+C): Provider/Backend SPI 抽取 — 技术设计"
change: p06-bc-provider-backend-spi
change_id: p06-bc-provider-backend-spi
comet_change: p06-bc-provider-backend-spi
role: technical-design
spec: openspec
canonical_spec: openspec
links:
  - "[p06-bc-provider-backend-spi](p06-bc-provider-backend-spi)"
---

# Design Doc — Phase 0.6 C1 (0.6B+C): Provider/Backend SPI 抽取

> 本设计文档对应 OpenSpec change [`p06-bc-provider-backend-spi`](../../../changes/p06-bc-provider-backend-spi)（canonical spec = OpenSpec）。
> Superpowers 深度设计文档。配套 `proposal.md` / `tasks.md`（open 阶段产物）。本文件是 design 阶段交付物，聚焦**可执行边界**与**逐文件迁移方案**。

## 1. 现状事实（已核验，非臆测）

- `pipeline.rs`（1279 行）**已经**把 GStreamer 相关代码用 `#[cfg(feature = "gstreamer")]` 隔离：
  - vendor-neutral：`SourcePlan` / `PipelinePlan` / `materialize` / `src_props` / `PipelineHealth` / `PtsMonotonicity` / `MediaRt01Acceptance` / `PipelineError` / `PipelineController` trait / `PipelineHandle` / `BusSeverity` / `PipelineBusEventKind` / `PipelineBusEvent` / `bus_event_recovery_policy` / `read_health`。
  - gstreamer-gated：`GStreamerPipelineController` 的 `build_pipeline` / `translate_bus` / `attach_video_sink` / `attach_audio_sink` / `poll_bus`，以及 `HEALTH_ARCS` 的 GstInstance 字段。
  - `#[cfg(not(feature = "gstreamer"))]` 为 `prepare/start/recover` 提供 stub。
- `decklink.rs` / `sdk.rs` 是 BMD FFI 层（`main.rs` 经 `#[cfg(feature = "bmd")]` 在模块级引用；模块内部 FFI 调用用 cfg 隔离）。`device.rs:20` 顶层 `use crate::decklink::BmdDeviceIdentity;` 与 `device.rs:182` `crate::decklink::enumerate()` 是**未 cfg 隔离**的跨层依赖（C1 需处理）。
- `resolver.rs` / `signal.rs` 大量 `use gstreamer::*`（probe / 测试 pipeline），均在某 feature 路径下被调用，但模块内部部分 `gstreamer::` 引用需确认 cfg 边界。
- `main.rs`（891 行）直接调用 `crate::decklink::*`（probe_connector_config / registry / start_capture）、`sdk::probe_sdk`、`crate::pipeline::*`、`gstreamer::version()`。

## 2. 目标架构（四层 → 模块目录）

```
src/
  domain/            (Canonical Media Runtime, 无 vendor top-level 引用)
    mod.rs  port.rs  device.rs  resolver.rs  signal.rs  lease.rs  config.rs
    health.rs  supervisor.rs  rpc.rs  graph_intent.rs  hw_port_01.rs
    pipeline.rs        ← canonical ingest 模型 + Materialize 逻辑(保留, vendor-neutral)
  contracts/         (SPI = Runtime Contracts)
    mod.rs  provider.rs  backend.rs
  runtime/           (Runtime Orchestration)
    mod.rs  session.rs  binding.rs  preflight.rs  scheduler.rs(冻结)
  adapters/
    mod.rs
    blackmagic/      ← 仅此目录引用 decklink/sdk + bmd-provider feature
      mod.rs  decklink.rs  sdk.rs
    gstreamer/       ← 仅此目录引用 gstreamer + gstreamer-backend feature
      mod.rs  controller.rs
```

> **最低可行边界（本 change 锁定）**：① `adapters/blackmagic/` 独占 `bmd`/`decklink`/`sdk` 引用；② `adapters/gstreamer/` 独占 `gstreamer`/`gstreamer_app`/`glib` 引用；③ `domain/`+`contracts/`+`runtime/` 编译期不含 vendor crate 顶层引用（`default` / `simulation` 构建验证）。
> `resolver.rs`/`signal.rs` 的 GStreamer 探活逻辑本质依赖 GStreamer，采用「保留在 domain 但用 `#[cfg(feature = "gstreamer-backend")]` 包裹」最小化移动面（其产出 `ResolvedDeviceBinding`/`SignalStatus` 属 Domain 语义）。

## 3. Trait 定义（契约冻结）

### `contracts/provider.rs` — `HardwareProvider`

```rust
pub trait HardwareProvider: Send + Sync {
    /// 枚举硬件并解析为 canonical DeviceInfo（BMD 身份细节在 Adapter 内消化，不外泄）。
    fn discover(&self) -> Result<Vec<DeviceInfo>, ProviderError>;
    /// SDK 能力探针（仅 Reference Adapter 实现；返回 canonical 能力报告）。
    fn probe_capabilities(&self) -> Vec<CapabilityReport>;
    /// 连接配置探针（diagnostic / 端口闭环）。
    fn probe_connector_config(&self) -> ConnectorConfig;
}
```

- `DeviceInfo` / `CapabilityReport` / `ConnectorConfig` 均为 Domain 类型（`device.rs` / `port.rs`）。
- BMD 实现：`adapters/blackmagic/decklink.rs` 的 `discover` 内部调用原 `enumerate()` + 身份映射；`sdk.rs` 的 `probe_sdk` 映射为 `probe_capabilities`。
- 不引入 `device-number` 作为业务主身份（C1 起继续消除，全量在 C5）。

### `contracts/backend.rs` — `MediaBackend`

```rust
pub trait MediaBackend: Send + Sync {
    fn prepare(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError>;
    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    fn poll_bus(&self, handle: &PipelineHandle) -> Vec<PipelineBusEvent>;
}
```


```

Full source: docs/openspec/changes/p06-bc-provider-backend-spi/design.md

## docs/openspec/changes/p06-bc-provider-backend-spi/tasks.md

- Source: docs/openspec/changes/p06-bc-provider-backend-spi/tasks.md
- Lines: 1-32
- SHA256: f827f12f069885c3a70b4ca63abffd189719379c92fb0e260832b266ccb74a16

```md
# Tasks: Phase 0.6 C1 (0.6B+C)

## 1. 建立 Provider/Backend trait 契约

- [ ] 在 `providers/mod.rs` 定义 `HardwareProvider` trait（discover / open / identity + FFI 边界）
- [ ] 在 `backends/mod.rs` 定义 `MediaBackend` trait（materialize / src_props + GStreamer 边界）
- [ ] trait 不含 vendor 错误类型（统一到 RuntimeEvent，0.6D 落地）

## 2. 迁移 BMD FFI 到 providers/blackmagic

- [ ] 将 `decklink.rs` / `sdk.rs` 迁入 `providers/blackmagic/`，仅经 `HardwareProvider` 暴露
- [ ] `IDeckLinkInput` 限定 Discovery/诊断用途，不进入媒体流
- [ ] feature `bmd` → `bmd-provider`（保留兼容别名或文档说明迁移）

## 3. 迁移 GStreamer 执行到 backends/gstreamer

- [ ] 将 `pipeline.rs` 媒体路径迁入 `backends/gstreamer/`，仅经 `MediaBackend` 暴露
- [ ] 保留 canonical 管线语义（decklinkvideosrc + decklinkaudiosrc → RAW → Normalize → FRAME/MASTER SWITCH → Encode → 分发）
- [ ] `src_props` 按 connector 拼 `connection=`；audio audiosrc 不设 connection
- [ ] feature `gstreamer` → `gstreamer-backend`

## 4. 解耦调用方

- [ ] `main` / `resolver` / `signal` / `pipeline` 改为依赖 trait，移除对 `decklink` / `gstreamer` crate 顶层的直接依赖
- [ ] Resolver 身份优先级与 `set_state(READY)` 遍历逻辑迁入 Provider 内并保持

## 5. 验证（CI 门禁）

- [ ] `cargo clippy --all-targets -- -D warnings`（default + `bmd-provider,gstreamer-backend` 两套 feature 都必须跑）
- [ ] `cargo test` default + simulation 保持通过
- [ ] `cargo build --features bmd-provider,gstreamer-backend` 编译通过
- [ ] 真机 `cargo build --features bmd,gstreamer`（兼容名）仍可用或给出迁移说明

```
