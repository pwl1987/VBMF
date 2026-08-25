//! Media Agent runtime configuration.
//!
//! Gate 2.1: interface freeze only. This module defines the config shape and
//! reads it from the environment; it does NOT yet drive any behavior.
//! Defaults are chosen for the Option B runtime (see
//! `docs/architecture/MEDIA_RUNTIME_SECURITY_MODEL.md`).
#![allow(dead_code)] // Gate 2.1 skeleton: config shape frozen, fields not all consumed yet.

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
    pub fn from_env() -> Self {
        let d = Config::default();
        let get = |k: &str| std::env::var(k).ok();

        let device_allowlist = get("MEDIA_AGENT_DEVICE_ALLOWLIST")
            .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
            .unwrap_or(d.device_allowlist);

        let parse_secs = |k: &str, fallback: Duration| {
            get(k).and_then(|v| v.parse::<u64>().ok()).map(Duration::from_secs).unwrap_or(fallback)
        };

        Self {
            rpc_bind: get("MEDIA_AGENT_RPC_BIND").unwrap_or(d.rpc_bind),
            device_allowlist,
            default_lease_ttl: parse_secs("MEDIA_AGENT_LEASE_TTL_SECS", d.default_lease_ttl),
            lease_renew_window: parse_secs("MEDIA_AGENT_LEASE_RENEW_SECS", d.lease_renew_window),
            health_poll_interval: parse_secs("MEDIA_AGENT_HEALTH_POLL_SECS", d.health_poll_interval),
            max_recover_attempts: get("MEDIA_AGENT_MAX_RECOVER_ATTEMPTS")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(d.max_recover_attempts),
        }
    }
}
