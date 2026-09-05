//! A2-0 (用户裁定: gates/ 模块族, 非单体): 真机验收入口族。
//!
//! 每 Gate 独立文件（坏了单独定位）; mod.rs 只做 façade。
//! 语义纪律: **Gate = 调用 Production Runtime 做验收, 绝不自造第二套 Runtime**
//! （LOOPBACK 的 gate-local 构造显式标记, 见 loopback.rs 头注）。
//! 生产 bin（media-agent）自 A20-02 起对下列 env 零 dispatch 责任——
//! 入口归 `bin/gates.rs`。
//!
//! 迁移自 main.rs 对应 env 块, 逐字节搬运（a2-0-runtime-repositioning, 行为零变）。

pub mod a204_obs;
pub mod config_probe;
pub mod dual_input;
pub mod loopback;
pub mod registry;
pub mod resolver;
pub mod session_lifecycle;
