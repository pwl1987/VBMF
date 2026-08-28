//! Phase 0.6 C1: Concrete Adapters (BMD Provider / GStreamer Backend).
//!
//! 仅此目录下的子模块允许引用 vendor crate (`decklink` / `gstreamer`) 顶层。
//! Domain / Contract / Runtime 层不得直接 `use decklink::` / `use gstreamer::`。
pub mod blackmagic;
pub mod gstreamer;
