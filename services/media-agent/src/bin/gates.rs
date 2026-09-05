//! VBMF Media Agent Gates — Diagnostic / Acceptance Root（A2-0 归位后形态）。
//!
//! 六个真机验收 env 的**唯一**入口（生产 media-agent bin 对这些 env 零 dispatch）:
//!   VBMF_CONFIG_PROBE / VBMF_RESOLVER / VBMF_LOOPBACK / VBMF_SESSION_LIFECYCLE /
//!   VBMF_A2_8_DUAL_INPUT（A2-8-02-I 五层 Gate, 第十八轮 §十/§十五）/
//!   VBMF_A2_8_04_OBS（A2-8-04 多场景六路观测, R52——observation only）/
//!   VBMF_REGISTRY_ONLY
//! Gate 逻辑在 lib `gates/` 模块族（逐字节迁自 main.rs, 行为零变）。
//!
//! **A20-03（用户裁定）: Gate 是 Consumer 不是 Bootstrapper**——本 bin 的全部
//! 生产依赖构造来自唯一的 `bootstrap::build()`（与生产 Composition Root 同源,
//! 消灭"两套初始化语义"）; 本文件**不再拥有任何**构造代码
//! （A20-03-BS-01 Single Bootstrap Source 静态验收锁定）。
//! SDK FFI probe 属诊断行为（非依赖构造）, 留在本入口——与 bootstrap.rs 硬边界一致。

fn main() {
    tracing_subscriber::fmt::init();

    // 唯一构造源（config/provider/discovery/双日志/lease+占位租约/supervisor/agent_state）。
    let _world = media_agent::bootstrap::build();

    // SDK FFI smoke（诊断行为; 原序保留——构造后、gate dispatch 前）。
    match media_agent::adapters::blackmagic::probe_sdk("libDeckLinkAPI.so") {
        Ok(()) => tracing::info!("SDK libDeckLinkAPI.so reachable, entry symbols present"),
        Err(e) => {
            tracing::warn!(error = %e, "SDK probe failed (expected in container w/o bind-mount)")
        }
    }

    // ── Gate dispatch（原 main 相对序; 命中即 exit, 未命中落到末尾提示） ──
    #[cfg(feature = "bmd-provider")]
    media_agent::gates::config_probe::run();

    #[cfg(feature = "gstreamer-backend")]
    media_agent::gates::resolver::run(&_world.config, &_world.discovered);

    #[cfg(feature = "gstreamer-backend")]
    media_agent::gates::loopback::run(
        &_world.config,
        &_world.discovered,
        &_world.event_sink,
        &_world.projection_log,
    );

    #[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
    media_agent::gates::session_lifecycle::run(
        &_world.config,
        &_world.devices,
        &_world.discovered,
        &_world.lease_manager,
        &_world.supervisor,
        &_world.agent_state,
        &_world.event_sink,
        &_world.projection_log,
        &_world.internal_log,
        &_world.event_intake,
    );

    // A2-8-02-I（第十八轮 §十/§十五）: 双输入五层真机 Gate（两块独立单输入卡形态）。
    #[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
    media_agent::gates::dual_input::run(
        &_world.config,
        &_world.devices,
        &_world.discovered,
        &_world.lease_manager,
        &_world.supervisor,
        &_world.agent_state,
        &_world.event_sink,
        &_world.internal_log,
    );

    // A2-8-04（R52）: 多场景六路观测 Gate——observation only（无判据面, 与
    // dual_input 五层 Gate 正交; exit=采集完整性非时间线裁决）。
    #[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
    media_agent::gates::a204_obs::run(
        &_world.config,
        &_world.devices,
        &_world.discovered,
        &_world.lease_manager,
        &_world.supervisor,
        &_world.event_sink,
    );

    // REGISTRY_ONLY 置底（原 main 中该探针在 supervisor 之后; 未命中任何 gate 时
    // 本 bin 无事可做——显式提示后退出, 绝不进入生产 runtime 循环）。
    #[cfg(feature = "hardware-test")]
    media_agent::gates::registry::run();

    eprintln!(
        "media-agent-gates: 未命中任何 gate env \
         (VBMF_CONFIG_PROBE / VBMF_RESOLVER / VBMF_LOOPBACK / VBMF_SESSION_LIFECYCLE / \
         VBMF_A2_8_DUAL_INPUT / VBMF_A2_8_04_OBS / VBMF_REGISTRY_ONLY)"
    );
    std::process::exit(2);
}
