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
    /// RPC 绑定地址 (UNWIRED: transport 尚未实现, 见 rpc.rs "No transport yet").
    /// 安全约束 (用户 §二十二 P1-2): Rust 不负责 API Gateway / Auth (Fastify 是控制面唯一入口),
    /// 因此 RPC 一旦启用, 必须绑定 `127.0.0.1` 或 Unix socket, **绝不** `0.0.0.0`/`::` 暴露公网.
    /// 默认已改为 `127.0.0.1:50051`; `MEDIA_AGENT_RPC_BIND` 仅可覆盖为 localhost / UDS.
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
            rpc_bind: "127.0.0.1:50051".to_string(),
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

    /// RPC 绑定安全校验 (P1-2, 用户 §二十二): 若 host 为 `0.0.0.0` / `::` (暴露公网),
    /// 返回非空告警. Rust 不负责 Auth/API Gateway, RPC 须绑定 localhost / Unix socket, 由 Fastify 反向代理.
    /// 即便当前 RPC transport 未实现, 也应在启动时校验配置, 防止未来一启用即成暴露面.
    pub fn rpc_bind_security_warnings(&self) -> Vec<String> {
        let host = self
            .rpc_bind
            .rsplit_once(':')
            .map(|(h, _)| h)
            .unwrap_or(self.rpc_bind.as_str())
            .trim_matches(['[', ']']);
        let insecure = host == "0.0.0.0" || host == "::";
        if insecure {
            vec![format!(
                "rpc_bind '{}' 将暴露公网; Rust 不负责 Auth/API Gateway, RPC 须绑定 127.0.0.1 或 Unix socket, 由 Fastify 反向代理",
                self.rpc_bind
            )]
        } else {
            Vec::new()
        }
    }
}

/// P1a: Prototype 输出配置（**demo 层, 显式不进 Runtime Contract**——参数正式契约化
/// 留产品配置模型阶段, 用户 2026-09-02 裁定）。驱动 `materialize` 的输出物化
/// （pipeline.rs `materialize_with_output`）; env 缺失 ⇒ 无输出分支（行为与 P1a 前逐字节一致）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrototypeOutputConfig {
    /// `VBMF_OUTPUT_KIND`: 覆盖诊断 intent 的 sink kind（main.rs 接线用; None = intent 原值）。
    pub sink_kind_override: Option<String>,
    /// `VBMF_OUTPUT_HLS_DIR`: HLS 分片目录（绝对路径; kind=hls 时必需）。
    pub hls_dir: Option<String>,
    /// `VBMF_OUTPUT_RTMP_URL`: RTMP 推流地址（kind=rtmp 时必需）。
    pub rtmp_url: Option<String>,
    /// `VBMF_OUTPUT_V_BITRATE_KBPS`: 视频码率（x264enc 单位 kbit/s; 默认 6000 = 6 Mbps）。
    pub video_bitrate_kbps: u32,
    /// `VBMF_OUTPUT_A_BITRATE_BPS`: 音频码率（avenc_aac 单位 bit/s; 默认 128000）。
    pub audio_bitrate_bps: u32,
}

impl Default for PrototypeOutputConfig {
    fn default() -> Self {
        Self {
            sink_kind_override: None,
            hls_dir: None,
            rtmp_url: None,
            video_bitrate_kbps: 6000,
            audio_bitrate_bps: 128_000,
        }
    }
}

impl PrototypeOutputConfig {
    /// 真实 env 读取（生产入口）。
    pub fn from_env() -> Self {
        Self::from_env_lookup(|k| std::env::var(k).ok())
    }

    /// 可测变体: 注入查找函数（并行测试不碰进程 env）。
    ///
    /// 目标值卫生校验（review Minor#6）: `hls_dir`/`rtmp_url` 会内插进 gst-launch 串,
    /// 含空白/`!`/`"` 的值按非法处理（→ None ⇒ fail-soft 降级, 绝不拼出可注入 launch）。
    pub fn from_env_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let d = PrototypeOutputConfig::default();
        let sane = |v: Option<String>| {
            v.filter(|s| !s.chars().any(|c| c.is_whitespace() || c == '!' || c == '"'))
        };
        let bitrate = |k: &str, fallback: u32| {
            lookup(k)
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(fallback)
        };
        Self {
            sink_kind_override: sane(lookup("VBMF_OUTPUT_KIND")),
            hls_dir: sane(lookup("VBMF_OUTPUT_HLS_DIR")),
            rtmp_url: sane(lookup("VBMF_OUTPUT_RTMP_URL")),
            video_bitrate_kbps: bitrate("VBMF_OUTPUT_V_BITRATE_KBPS", d.video_bitrate_kbps),
            audio_bitrate_bps: bitrate("VBMF_OUTPUT_A_BITRATE_BPS", d.audio_bitrate_bps),
        }
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;

    fn lookup<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| {
            pairs
                .iter()
                .find(|(key, _)| *key == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn config_rt_01_prototype_output_defaults() {
        let cfg = PrototypeOutputConfig::from_env_lookup(lookup(&[]));
        assert_eq!(cfg, PrototypeOutputConfig::default());
        assert!(cfg.sink_kind_override.is_none(), "无 env ⇒ 不覆盖 intent");
        assert!(cfg.hls_dir.is_none() && cfg.rtmp_url.is_none());
        assert_eq!(cfg.video_bitrate_kbps, 6000, "默认 6 Mbps (x264enc kbit/s)");
        assert_eq!(cfg.audio_bitrate_bps, 128_000, "默认 AAC 128 kbps");
    }

    #[test]
    fn config_rt_01_prototype_output_env_full() {
        let cfg = PrototypeOutputConfig::from_env_lookup(lookup(&[
            ("VBMF_OUTPUT_KIND", "hls"),
            ("VBMF_OUTPUT_HLS_DIR", "/var/tmp/p1a"),
            ("VBMF_OUTPUT_RTMP_URL", "rtmp://127.0.0.1:1935/live/p1a"),
            ("VBMF_OUTPUT_V_BITRATE_KBPS", "4500"),
            ("VBMF_OUTPUT_A_BITRATE_BPS", "96000"),
        ]));
        assert_eq!(cfg.sink_kind_override.as_deref(), Some("hls"));
        assert_eq!(cfg.hls_dir.as_deref(), Some("/var/tmp/p1a"));
        assert_eq!(
            cfg.rtmp_url.as_deref(),
            Some("rtmp://127.0.0.1:1935/live/p1a")
        );
        assert_eq!(cfg.video_bitrate_kbps, 4500);
        assert_eq!(cfg.audio_bitrate_bps, 96_000);
    }

    #[test]
    fn config_rt_01_prototype_output_invalid_bitrate_falls_back() {
        let cfg = PrototypeOutputConfig::from_env_lookup(lookup(&[(
            "VBMF_OUTPUT_V_BITRATE_KBPS",
            "not-a-number",
        )]));
        assert_eq!(cfg.video_bitrate_kbps, 6000, "非法数值回退默认, 绝不 panic");
    }

    #[test]
    fn config_rt_01_prototype_output_unsafe_target_rejected() {
        // launch 注入卫生: 空白/!/引号 一律按未设置处理（fail-soft 降级, 不可注入）。
        for (k, v) in [
            ("VBMF_OUTPUT_HLS_DIR", "/tmp/a b"),
            ("VBMF_OUTPUT_HLS_DIR", "/tmp/!inject"),
            ("VBMF_OUTPUT_HLS_DIR", "/tmp/\"q\""),
            ("VBMF_OUTPUT_RTMP_URL", "rtmp://x/live ! fakesink"),
            ("VBMF_OUTPUT_KIND", "h ls"),
        ] {
            let cfg = PrototypeOutputConfig::from_env_lookup(lookup(&[(k, v)]));
            let got = match k {
                "VBMF_OUTPUT_HLS_DIR" => cfg.hls_dir,
                "VBMF_OUTPUT_RTMP_URL" => cfg.rtmp_url,
                _ => cfg.sink_kind_override,
            };
            assert!(got.is_none(), "{k}={v:?} 必须被拒绝");
        }
    }
}
