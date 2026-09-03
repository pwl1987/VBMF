//! Phase 0.6 C1: Runtime Contracts / SPI (HardwareProvider + MediaBackend).
//! 冻结的 canonical SPI 名称；后续 Reference Adapter (BMD / AJA / Mock) 实现同一组 trait.
pub mod backend;
pub mod provider;
pub mod switch; // A2-8-01: Switch Execution Adapter SPI (与 MediaBackend 平行; 契约面零 GStreamer 词)
