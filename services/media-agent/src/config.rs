//! Media Agent runtime configuration.
//!
//! Config 定义运行时配置, 且已被 Media Agent 启动真实消费 (不是 Gate 2.1 skeleton 的占位):
//! `device_binding_path` 驱动 `DeviceBindingManifest::load()` 与 production materialization;
//! `health_bind` 驱动 `/health` 监听地址 (默认 `127.0.0.1:8080`, 见用户 §二十二 P1 Security);
//! `rpc_bind` / 租约 / Supervisor 参数驱动 RPC / Lease / 恢复策略.
//! 未接线的字段在此处显式标注 `// UNWIRED`, 不再用 "does NOT yet drive any behavior" 误导后续开发.
//! Defaults are chosen for the Option B runtime (see
//! `docs/architecture/MEDIA_RUNTIME_SECURITY_MODEL.md`).
#![allow(dead_code)] // 部分字段尚未全部消费 (标注 `// UNWIRED`); 整体已是 Runtime 消费态, 非 skeleton.

use std::time::Duration;

/// Top-level agent configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Bind address for the RPC / health surface.
    pub rpc_bind: String,
    /// DeckLink device allowlist (e.g. `/dev/blackmagic`). Empty = SDK default.
    pub device_allowlist: Vec<String>,
    /// Default lease TTL when a client does not specify one.
    pub default_lease_ttl: Duration,
    /// Lease auto-renew window before expiry.
    pub lease_renew_window: Duration,
    /// Supervisor health-check polling interval.
    pub health_poll_interval: Duration,
    /// Max restart attempts before `FAILED` (see state machine).
    pub max_recover_attempts: u32,
    /// 显式绑定清单路径 (DeviceBindingManifest JSON). 生产 BMD 绑定**唯一**权威来源;
    /// 生产模式缺失 → 失败闭合 (拒绝 materialize, 绝不回退 legacy 盲猜, 用户 §四).
    /// 仅 `MEDIA_AGENT_MODE=diagnostic` 允许缺失时回退 legacy auto-resolver (排障用).
    pub device_binding_path: Option<String>,
    /// `/health` 管理面监听地址 (默认 `127.0.0.1:8080`, 仅本机回环). 生产部署应经 Node/Fastify /
    /// Nginx 反向代理 + 认证后暴露, 绝不直接裸露公网 (用户 §二十二 P1 Security). 可由
    /// `MEDIA_AGENT_HEALTH_BIND` 覆盖为内网接口地址或 Unix socket 路径.
    pub health_bind: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rpc_bind: "0.0.0.0:50051".to_string(),
            device_allowlist: vec!["/dev/blackmagic".to_string()],
            default_lease_ttl: Duration::from_secs(300),
            lease_renew_window: Duration::from_secs(30),
            health_poll_interval: Duration::from_secs(5),
            max_recover_attempts: 5,
            device_binding_path: None,
            health_bind: "127.0.0.1:8080".to_string(),
        }
    }
}

impl Config {
    /// Build from environment with `Default` fallback for unset keys.
    ///
    /// Recognized vars:
    /// - `MEDIA_AGENT_RPC_BIND`
    /// - `MEDIA_AGENT_DEVICE_ALLOWLIST` (comma-separated)
    /// - `MEDIA_AGENT_LEASE_TTL_SECS`
    /// - `MEDIA_AGENT_LEASE_RENEW_SECS`
    /// - `MEDIA_AGENT_HEALTH_POLL_SECS`
    /// - `MEDIA_AGENT_MAX_RECOVER_ATTEMPTS`
    /// - `MEDIA_AGENT_DEVICE_BINDING` (path to DeviceBindingManifest JSON)
    /// - `MEDIA_AGENT_HEALTH_BIND` (`/health` bind address; default `127.0.0.1:8080`)
    pub fn from_env() -> Self {
        let d = Config::default();
        let get = |k: &str| std::env::var(k).ok();

        let device_allowlist = get("MEDIA_AGENT_DEVICE_ALLOWLIST")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or(d.device_allowlist);

        let parse_secs = |k: &str, fallback: Duration| {
            get(k)
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs)
                .unwrap_or(fallback)
        };

        Self {
            rpc_bind: get("MEDIA_AGENT_RPC_BIND").unwrap_or(d.rpc_bind),
            device_allowlist,
            default_lease_ttl: parse_secs("MEDIA_AGENT_LEASE_TTL_SECS", d.default_lease_ttl),
            lease_renew_window: parse_secs("MEDIA_AGENT_LEASE_RENEW_SECS", d.lease_renew_window),
            health_poll_interval: parse_secs(
                "MEDIA_AGENT_HEALTH_POLL_SECS",
                d.health_poll_interval,
            ),
            max_recover_attempts: get("MEDIA_AGENT_MAX_RECOVER_ATTEMPTS")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(d.max_recover_attempts),
            device_binding_path: get("MEDIA_AGENT_DEVICE_BINDING"),
            health_bind: get("MEDIA_AGENT_HEALTH_BIND").unwrap_or(d.health_bind),
        }
    }
}
