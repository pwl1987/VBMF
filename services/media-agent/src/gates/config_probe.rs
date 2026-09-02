//! A2-0: B 选项 Connector Mode 配置探针（VBMF_CONFIG_PROBE 入口）。
//!
//! 自 main.rs 逐字节迁出（env 命中即 exit(0), 绝不进入媒体 launch——语义不变）。
//! 纯 SDK 读取 (IDeckLinkConfiguration), 不进媒体, 不依赖 GStreamer。
//! 用于无桌面环境 (无 Blackmagic Desktop Video Setup) 时直接回答
//! "每个子设备当前是 In 还是 Out、物理端口如何按 Connector Mode 分组".

#[cfg(feature = "bmd-provider")]
pub fn run() {
    if std::env::var("VBMF_CONFIG_PROBE").is_ok() {
        match crate::adapters::blackmagic::probe_connector_config() {
            Ok(rows) => match serde_json::to_string_pretty(&rows) {
                Ok(json) => {
                    println!("=== B Connector Config Probe (IDeckLinkConfiguration) ===");
                    println!("{json}");
                }
                Err(e) => eprintln!("config probe 序列化失败: {e}"),
            },
            Err(e) => eprintln!("config probe 失败: {e}"),
        }
        std::process::exit(0);
    }
}
