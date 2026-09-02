//! A2-0 (A20-03, 用户裁定): Bootstrap — 唯一构造源（Dependency Construction）。
//!
//! **硬边界（用户 2026-09-02 裁定）: bootstrap 只构造, 不运行。**
//! 允许: Config / Provider / Discovery / EventLog+FanoutSink / LeaseManager
//! （含 bootstrap 占位租约与排他性自检——初始化态, 非媒体运行）/ Supervisor
//! 注册 / AgentState。
//! 禁止: Session::start / Pipeline::start / watchdog / recover / sleep /
//! Gate 断言 / process::exit（构造失败的 fail-closed 退出除外——无东西可运行）/ HTTP accept。
//!
//! 消费方（同源, 消灭"两套初始化语义"）:
//! - `bin/media-agent.rs`（生产 Composition Root）
//! - `bin/gates.rs`（Diagnostic Root——Gate 是 Consumer 不是 Bootstrapper）
//!
//! **不在本模块**（用户裁定: 诊断行为 ≠ 依赖构造）:
//! - SDK FFI probe（`probe_sdk`）——生产侧保留为 production diagnostic wiring
//!   （`bin/media-agent.rs` 内, 标记待后续独立变更裁决）; gate 侧在 `bin/gates.rs`。

use std::sync::Arc;

use crate::config::Config;
use crate::contracts::provider::{DiscoveredDevice, HardwareProvider};
use crate::device::DeviceInfo;
use crate::events::{RuntimeEventLog, RuntimeEventSink};
use crate::health::AgentState;
use crate::lease::{InMemoryLeaseManager, LeaseManager as _};
use crate::supervisor::Supervisor;

/// 共同构造件集合（仅两个入口都消费的对象; `provider` 在 build 内被 discover
/// 消费, 不留存——God Object 红线, 字段以实际使用为准）。
pub struct BootstrapContext {
    pub config: Config,
    pub discovered: Vec<DiscoveredDevice>,
    pub devices: Vec<DeviceInfo>,
    /// P0-7D D3 双日志: 投影（transport/gate 证据）与内消费（watchdog→reduce）。
    pub projection_log: Arc<RuntimeEventLog>,
    pub internal_log: Arc<RuntimeEventLog>,
    pub event_sink: Arc<dyn RuntimeEventSink>,
    /// bootstrap 占位租约已按设备全部持有（真实会话接管时让位——见各消费方）。
    pub lease_manager: Arc<InMemoryLeaseManager>,
    pub supervisor: Arc<std::sync::Mutex<Supervisor>>,
    pub agent_state: Arc<std::sync::Mutex<AgentState>>,
}

/// 唯一构造入口。构造失败 = fail-closed 进程退出（无媒体行为可泄漏）。
pub fn build() -> BootstrapContext {
    // Gate 2.1: load config shape from env.
    let config = Config::from_env();

    // P1-2 (用户 §二十二): RPC 绑定安全校验 — Rust 不负责 Auth, RPC 须 localhost/UDS.
    for w in config.rpc_bind_security_warnings() {
        tracing::warn!(warning = %w, "rpc_bind 安全");
    }

    // Gate 2.2: adapter 选择收口 (P0-4; mock > simulation > bmd-provider > default).
    let provider: Box<dyn HardwareProvider> =
        crate::registry::AdapterRegistry::build_provider().unwrap_or_else(|e| {
            eprintln!("adapter feature 冲突 (fail-closed): {e}");
            std::process::exit(2);
        });
    let (active_provider, active_backend) = crate::registry::active_adapters();
    tracing::info!(
        provider = active_provider,
        backend = active_backend,
        mode = std::env::var("MEDIA_AGENT_MODE")
            .as_deref()
            .unwrap_or("production"),
        "adapter selection (P0-4 运维可见)"
    );
    // P1-2: discover fail-closed — SDK/驱动失败显式拒启, 绝不与"无设备"混淆.
    let discovered = match provider.discover() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("device discovery 失败 (fail-closed): {e}");
            std::process::exit(2);
        }
    };
    let devices: Vec<DeviceInfo> = discovered.iter().map(|d| d.device.clone()).collect();
    tracing::info!(count = devices.len(), "device discovery complete");

    // P0-7D D3 (双日志分流定稿): FanoutSink 同序双写, emit 永不阻塞永不失败。
    let projection_log = Arc::new(RuntimeEventLog::new());
    let internal_log = Arc::new(RuntimeEventLog::new());
    let event_sink: Arc<dyn RuntimeEventSink> = Arc::new(crate::events::FanoutSink::new(
        projection_log.clone(),
        internal_log.clone(),
    ));

    // Gate 2.3: lease manager + bootstrap 占位租约（初始化态; 真实会话让位语义在各消费方）。
    let lease_manager = Arc::new(InMemoryLeaseManager::new());
    for d in &devices {
        match lease_manager.acquire(&d.device_id, "bootstrap", std::time::Duration::from_secs(60)) {
            Ok(l) => tracing::info!(device = %l.device_id, "lease acquired"),
            Err(e) => tracing::warn!(error = %e, "lease acquire failed"),
        }
    }
    // 排他性不变量自检: 同一设备重复 acquire 必须被拒 (防 host ffmpeg / 双采).
    if let Some(first) = devices.first() {
        match lease_manager.acquire(
            &first.device_id,
            "second-owner",
            std::time::Duration::from_secs(60),
        ) {
            Ok(_) => tracing::warn!("LEASE COLLISION — double-capture risk!"),
            Err(e) => tracing::info!(error = %e, "lease re-acquire correctly rejected"),
        }
    }

    // Gate 5: Supervisor 注册设备（只决策, 不碰 GStreamer; 持 FanoutSink）。
    let supervisor = Arc::new(std::sync::Mutex::new(Supervisor::new(
        crate::supervisor::RestartPolicy::default(),
        event_sink.clone(),
    )));
    for d in &devices {
        supervisor.lock().unwrap().register(d.device_id);
    }
    tracing::info!(watched = devices.len(), "supervisor initialized");

    let agent_state = Arc::new(std::sync::Mutex::new(AgentState::Ready));

    BootstrapContext {
        config,
        discovered,
        devices,
        projection_log,
        internal_log,
        event_sink,
        lease_manager,
        supervisor,
        agent_state,
    }
}
