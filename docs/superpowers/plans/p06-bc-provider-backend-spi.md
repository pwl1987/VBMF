# Implementation Plan — Phase 0.6 C1 (0.6B+C): Provider/Backend SPI 抽取

> Superpowers 实现计划（classic build 阶段）。配套 OpenSpec change `p06-bc-provider-backend-spi`。
> 本计划为已冻结的实现蓝图，所有任务已确认（[x]）。实际代码改动按本计划执行。

## Context

`media-agent` 当前在 `main.rs`/`device.rs`/`resolver.rs`/`signal.rs`/`pipeline.rs` 直接依赖 `decklink`/`gstreamer` crate 顶层，违反 Phase 0.6 四层架构（Domain / Contract / Runtime / Adapter）。本 change 冻结 `HardwareProvider`（= 现有 `DeviceManager` 语义）/ `MediaBackend`（= 现有 `PipelineController` 语义）SPI，并把 BMD FFI 与 GStreamer 执行器移入 `adapters/` 子目录，使 Domain 层编译期不含 vendor crate 顶层引用，满足 ARCH-PORTABILITY-01 前置。

## 不变量（严禁回归）

- canonical 采集语义不变：`connection=optical-sdi`（非 `optical`）；audiosrc 无 `connection`；`device-number` 绝不默认 0。
- `materialize` / `src_props` 逻辑原样保留，仅移动 GStreamer 实现。
- default / simulation 84 单测、`cargo clippy --all-targets -- -D warnings`（default + `bmd-provider,gstreamer-backend`）、`--features bmd-provider,gstreamer-backend` 构建全部保持通过。

## Tasks

- [x] **T1 Cargo.toml feature 重命名**：`bmd`→`bmd-provider`、`gstreamer`→`gstreamer-backend`，保留 `bmd`/`gstreamer` 别名（`bmd = ["bmd-provider"]`、`gstreamer = ["gstreamer-backend"]`）；`hardware-test = ["bmd-provider"]`。（已改）
- [x] **T2 全局 cfg 同步**：`#[cfg(feature = "bmd")]`→`#[cfg(feature = "bmd-provider")]`、`#[cfg(feature = "gstreamer")]`→`#[cfg(feature = "gstreamer-backend")]`，覆盖 `main.rs`/`decklink.rs`/`sdk.rs`/`pipeline.rs`/`resolver.rs`/`signal.rs`/`build.rs`/`device.rs`/`rpc.rs`。
- [x] **T3 新建 `contracts/`**：`contracts/mod.rs` + `contracts/provider.rs`（`HardwareProvider` trait，等价于 `DeviceManager` 语义，返回 canonical `DeviceInfo`） + `contracts/backend.rs`（`MediaBackend` trait，等价于 `PipelineController` 语义）。`HardwareProvider`/`MediaBackend` 作为 canonical SPI 名称；保留 `DeviceManager`/`PipelineController` 为 `pub use` 别名以最小化调用方改动。
- [x] **T4 新建 `adapters/` 目录**：`adapters/mod.rs` 声明 `pub mod blackmagic; pub mod gstreamer;`，二者均 `#[cfg(feature = "...")]` 门控。
- [x] **T5 移动 BMD FFI**：`src/decklink.rs`→`src/adapters/blackmagic/decklink.rs`、`src/sdk.rs`→`src/adapters/blackmagic/sdk.rs`；`device.rs` 中的 `DeckLinkDeviceManager`（含 `use crate::decklink::BmdDeviceIdentity` 与 `crate::decklink::enumerate()` 调用）移入 `adapters/blackmagic/manager.rs`，实现 `HardwareProvider`。`device.rs` 仅保留 `DeviceManager` trait + `FilesystemDeviceManager` + `SimulatedDeviceManager`（Domain，无 vendor 引用）。
- [x] **T6 移动 GStreamer 执行器**：`pipeline.rs` 中 `GStreamerPipelineController` 的 gstreamer-gated 方法体（`build_pipeline`/`translate_bus`/`attach_video_sink`/`attach_audio_sink`/`poll_bus` 及 `HEALTH_ARCS` GstInstance 字段）移入 `src/adapters/gstreamer/controller.rs`；`pipeline.rs` 仅留 `PipelineController`/`MediaBackend` trait + vendor-neutral 逻辑 + `#[cfg(not(feature="gstreamer-backend"))]` stub。
- [x] **T7 resolver/signal 去顶层 vendor 引用**：`resolver.rs`/`signal.rs` 的顶层 `use gstreamer::prelude::*` / `use gstreamer_app::prelude::*` 移入函数级或 `#[cfg(feature = "gstreamer-backend")]` 块内，确保 default/simulation 无 gstreamer 顶层引用。
- [x] **T8 main.rs 接线**：`mod decklink`/`mod sdk` → `mod adapters`；`crate::decklink::*` / `sdk::*` / `gstreamer::version()` 调用改为经 `HardwareProvider` / `MediaBackend` trait 对象（由 Registry 在 `simulation` / `bmd-provider,gstreamer-backend` 下选择）；`gstreamer::version()` 证据打印移入 backend 实现。
- [x] **T9 验证**：
  - `cargo clippy --all-targets -- -D warnings`（default）通过；
  - `cargo test` default + simulation 通过（基线 84）；
  - `cargo build --features bmd-provider,gstreamer-backend` 通过（基线 bmd 83）。
- [x] **T10 勾选 OpenSpec `tasks.md`**：所有任务标记 `[x]`，与计划一致。
- [x] **T11 提交**：将 C1 改动提交到 `comet/p06-bc-provider-backend-spi` 分支（docs/openspec/changes 已被 .gitignore 忽略，仅提交源码改动）。

## 风险与回滚

- 独立分支，失败直接丢弃。
- 任何 clippy/test 回归立即回退对应文件改动，不整体 revert 以外不破坏主树。
- 不改 `materialize`/`src_props` 语义；不做 Mock（C3）/ Supervisor 改造（C2）/ Resource（C2）/ 门禁（C4/C5）。
