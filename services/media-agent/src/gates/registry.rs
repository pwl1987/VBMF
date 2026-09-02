//! A2-0: A0 纯身份核对模式（VBMF_REGISTRY_ONLY 入口）。
//!
//! 自 main.rs 逐字节迁出。语义拆分（行为零变约束下的归属裁定）:
//! - `print_registry()`: hardware-test 构建的**无条件** registry 打印——生产 boot
//!   原位置调用（hardware-test 特性启动行为, 非 env 门控）;
//! - `run()`: **env 门控**的 VBMF_REGISTRY_ONLY 提前退出（gates bin 入口;
//!   生产 main 对该 env 零 dispatch）。

#[cfg(feature = "hardware-test")]
pub fn print_registry() {
    match crate::adapters::blackmagic::registry() {
        Ok(table) => tracing::info!("DeckLink Device Registry:\n{table}"),
        Err(e) => tracing::warn!(error = %e, "registry unavailable"),
    }
}

#[cfg(feature = "hardware-test")]
pub fn run() {
    print_registry();
    if std::env::var("VBMF_REGISTRY_ONLY").is_ok() {
        tracing::info!("VBMF_REGISTRY_ONLY 已设置: 仅输出注册表, 进程退出。");
        std::process::exit(0);
    }
}
