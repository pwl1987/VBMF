//! Phase 0.6 C1: Concrete Adapters (BMD Provider / GStreamer Backend).
//!
//! 仅此目录下的子模块允许引用 vendor crate (`decklink` / `gstreamer`) 顶层。
//! Domain / Contract / Runtime 层不得直接 `use decklink::` / `use gstreamer::`。
pub mod blackmagic;
pub mod gstreamer;
#[cfg(feature = "mock")]
pub mod mock; // C3: 纯 Rust Mock Provider/Backend (无 BMD/无 GStreamer), 解锁 ARCH-PORTABILITY-01 Mock 侧.
#[cfg(feature = "mock")]
pub mod switch_mock; // A2-8-01: Mock Switch Execution Adapter (确定性 PTS 流 + 成对切换仿真)
