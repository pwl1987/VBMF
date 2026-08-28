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

> **⚠️ C1 实际交付说明（2026-08-28 审计后）**：本节下方给出的富 `HardwareProvider` / `MediaBackend` trait
> 是 **C2 目标契约**，并非 C1 已实现内容。C1 仅落地了 "canonical 名 = 既有 trait 别名"
> （`HardwareProvider = DeviceManager` / `MediaBackend = PipelineController`，见 `contracts/*.rs`），
> 且 `MediaBackend` 已在 A 批修复为仅 `gstreamer-backend` 门控（解除对 `bmd-provider` 的耦合）。
> 真正的 trait 与 Adapter 物理抽取（迁出 `device.rs` / `pipeline.rs`）在 C2 起完成。

> **✅ C2 已实现（2026-08-28）**：`contracts/provider.rs` / `contracts/backend.rs` 已落地独立的
> `trait HardwareProvider` / `trait MediaBackend`，分别由 `device::*DeviceManager`
> （Blackmagic / Filesystem / Simulation）与 `pipeline::GStreamerPipelineController` 实现；
> `MediaBackend` 沿用 A 批约定仅 `gstreamer-backend` 门控。
> **与下方契约的偏差（已对齐审计）**：`HardwareProvider::discover()` 当前返回 `Vec<DeviceInfo>`
> （非 `Result<Vec<DeviceInfo>, ProviderError>`）——因 `main.rs` 在 C2c 之前仍按 `Vec` 消费，
> 且 vendor 错误类型统一到 `RuntimeEvent` 留 0.6D；`probe_capabilities` / `probe_connector_config`
> 已就位但返回占位空值，真实 SDK 能力/端口探针回填留 C5/C...。Adapter 物理迁出留 C6/C7。

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

- 即现有 `PipelineController` trait 语义（改名 `MediaBackend`）；`GStreamerPipelineController` 改名 `GStreamerMediaBackend` 移入 `adapters/gstreamer/controller.rs`。
- `PipelinePlan` / `PipelineHandle` / `PipelineError` / `PipelineBusEvent` 留在 `domain/pipeline.rs`（vendor-neutral，已是）。

## 4. Feature 变更（Cargo.toml）

- `bmd` → **`bmd-provider`**；`gstreamer` → **`gstreamer-backend`**。
- 同步更新所有 `#[cfg(feature = "bmd")]` → `#[cfg(feature = "bmd-provider")]`，`#[cfg(feature = "gstreamer")]` → `#[cfg(feature = "gstreamer-backend")]`（含 `main.rs` 模块声明、`decklink.rs`/`sdk.rs`/`pipeline.rs`/`resolver.rs`/`signal.rs` 内部）。
- `hardware-test` 互斥约束（`compile_error!`）同步改名。
- **兼容策略**：保留旧 `bmd`/`gstreamer` feature 名作为 alias（`bmd = ["bmd-provider"]`、`gstreamer = ["gstreamer-backend"]`），避免真机构建脚本立刻失效。C2–C5 期间再择机清理别名。

## 5. 逐文件迁移计划

1. **Cargo.toml**：feature 改名 + 别名；`[dependencies]` 中 `decklink`/`gstreamer`/`gstreamer-app`/`glib` 保持 optional。
2. **新建 `contracts/`**：`provider.rs` / `backend.rs` / `mod.rs` 定义 trait。
3. **新建 `adapters/`**：`mod.rs` + `blackmagic/`（移入 `decklink.rs`/`sdk.rs`）+ `gstreamer/`（移入 `controller.rs` = 原 `GStreamerPipelineController` 的 gstreamer-gated 部分）。
4. **`device.rs`**：`use crate::decklink::BmdDeviceIdentity` → 改从 `contracts::provider` 或 domain 内 canonical 类型；`crate::decklink::enumerate()` 调用改为经 `HardwareProvider::discover()`。`BmdDeviceIdentity` 若仅为内部别名，移入 `adapters/blackmagic` 并仅 adapter 内使用。
5. **`pipeline.rs`**：`PipelineController` → 保留为别名 `pub use contracts::backend::MediaBackend as PipelineController;`（避免 main.rs 大改）；`GStreamerPipelineController` 与 gstreamer-gated 方法体移入 `adapters/gstreamer/controller.rs`，`pipeline.rs` 仅留 trait + stub + vendor-neutral 逻辑。
6. **`resolver.rs` / `signal.rs`**：GStreamer 探活函数整体用 `#[cfg(feature = "gstreamer-backend")]` 包裹；顶层 `use gstreamer::*` 移入函数级或 cfg 块内，确保 `default`/`simulation` 无 gstreamer 引用。
7. **`main.rs`**：模块声明 `mod decklink`/`mod sdk` → `mod adapters`；调用 `crate::decklink::*` / `sdk::*` / `gstreamer::version()` 改为经 `HardwareProvider` / `MediaBackend` trait 对象（由 Registry 在 `simulation`/`bmd-provider,gstreamer-backend` 下选择）；`gstreamer::version()` 证据打印改为后端实现内。
8. **`rpc.rs:30`** `PipelineStarted(crate::pipeline::PipelineHandle)` 路径不变（PipelineHandle 仍在 domain）。

## 6. 风险与回滚

- **量极大、易回归**：核心不变量是 canonical 采集语义（`connection=optical-sdi` 非 `optical`；audiosrc 无 `connection`；`device-number` 绝不默认 0）。所有 `src_props`/`materialize` 逻辑**原样保留**，仅移动 GStreamer 实现，不改动语义。
- **default/simulation 必须保持 84 单测通过**、`cargo clippy --all-targets -- -D warnings` 两套 feature 通过、`--features bmd-provider,gstreamer-backend` 构建通过。
- 回滚：本 change 独立分支，失败直接丢弃分支，master 不受影响。

## 7. 验证（与 CI 门禁对齐）

- `cargo clippy --all-targets -- -D warnings`（default + `bmd-provider,gstreamer-backend`）通过。
- `cargo test` default + simulation 通过（基线 84）。
- `cargo build --features bmd-provider,gstreamer-backend` 通过（基线 bmd 83 单测）。
- 真机闭环（C5 验收用）：`--features bmd-provider,gstreamer-backend` + loopback 双门全绿不变。

## 8. 非目标（本 change 不碰）

- 不改 `materialize`/`src_props` 语义；不做 Mock（C3）；不做 Supervisor 改造（C2）；不做 Resource/Preflight（C2）；不做架构 lint 门禁（C4）；不进 Normalize（0.7）。
