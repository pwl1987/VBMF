//! VBMF Media Agent Library — Domain / Contracts / Runtime / Adapters / Gates。
//!
//! A2-0 (a2-0-runtime-repositioning, 2026-09-02): crate 根自 main.rs 迁出。
//! 本 crate 是 library-first 形态（双 bin + lib）:
//! - `media-agent` bin = Production Composition Root（config → adapter selection
//!   → dependency construction → runtime wiring → transport → process lifetime;
//!   对全部 VBMF_* gate env 零 dispatch 责任）
//! - `media-agent-gates` bin = Diagnostic / Acceptance Root（五个真机验收 env 的
//!   唯一入口; Gate 逻辑在 lib `gates/` 模块族）
//! - Gate 语义纪律: Gate = 调用 Production Runtime 做验收, 绝不自造第二套 Runtime。
//!
//! 媒体内核（Phase 0.6→0.7D→Prototype-1→Alpha-1 已实证）:
//! Session/Runtime Kernel（session/pipeline/resource/lease/health/supervisor/events）
//! + Canonical 语义（normalize/clock/audio/timecode）+ Command/Idempotency/Error 平面
//! + Transport 五端点 + 多输入编码输出（HLS/RTMP）。
//!
//! **Program Domain（A2-1 起落位, 用户裁定链 A2-1..A2-8）**: `program` 模块 =
//! 节目生产模型 Canonical 层（SwitchPolicy 已落; Masters/MasterJoin/ProgramMaster
//! 属 A2-2+）。GStreamer 仅为其 Execution Adapter——绝不反向推导。

// 硬规则 (Phase 0.6): `hardware-test` (IDeckLinkInput SDK 探针) 与 canonical `gstreamer`
// 运行时互斥 —— 生产运行不得同时打开同一块 DeckLink (避免双采 / 设备争用). 编译期强制.
#[cfg(all(feature = "hardware-test", feature = "gstreamer-backend"))]
compile_error!("hardware-test SDK 探针与 canonical GStreamer 运行时互斥; 生产运行不得同时启用 (避免双采/争用同一块 DeckLink)");

pub mod adapters;
pub mod api_boundary; // P0.7C-7: External API Foundation (API Boundary Model + Idempotency 契约; 非 Web Server)
pub mod audio; // P0.7B-2B: Canonical Audio Semantics (是什么, 非怎么处理)
pub mod bootstrap; // A2-0/A20-03: 唯一构造源（只构造不运行; 双 bin 同源消费）
pub mod clock; // P0.7B-2A: Canonical Clock Domain (只描述观测, 绝不决策; #147)
pub mod command; // P0.7C-3: Command Contract (请求语义非执行计划; 不可执行性三重守护)
pub mod config;
pub mod contracts;
pub mod custody; // A2-7-02: Program Runtime Custody (Program semantic lifecycle owner; Runtime/Orchestration 边界; 七不终裁)
pub mod device;
pub mod error_model; // P0.7C-5: Error Model (失败归因分类平面; 三平面分离)
pub mod event_projection; // P0.7C-6: Event Projection Foundation (Runtime→Event→Projection 生产边)
pub mod events; // 0.6D: RuntimeEvent canonical 事件契约 + 归一化映射 + 有界事件日志
pub mod fixture; // HW-PORT-01 / MEDIA-RT-01 复用的 BMD-SDI-LOOPBACK Fixture (host-specific 证据)
pub mod gates; // A2-0: 真机验收入口族（gates/ 模块族; 入口归 media-agent-gates bin）
pub mod graph_intent;
pub mod health;
pub mod hw_port_01; // HW-PORT-01 Gate: 端口级绑定闭环验收
pub mod idempotency; // P0.7C-4: Idempotency (D9-A~E: 同一命令语义 + 原子 claim + replay/conflict)
pub mod lease;
pub mod normalize; // P0.7B-1: Normalize Foundation — Raw → CanonicalMediaDescriptor (纯函数)
pub mod pipeline;
pub mod pipeline_events; // C7: 中性共享事件/健康类型模块 (不依赖 gstreamer crate)
pub mod port; // 五层模型: Device → Port → Capability → Runtime Binding → Signal
pub mod preflight; // P0-7A: Preflight 分级判定 (judge-only; V0.2 §1.2)
pub mod program; // A2-1: Program Domain —— 节目生产模型 Canonical 层（SwitchPolicy 首块; A2-2+ Masters/MasterJoin/ProgramMaster）
pub mod program_execution; // A2-8-02-E: Program Execution Runtime（program 执行资源生命周期唯一 owner; creator=destroyer）
pub mod program_timeline; // A2-8-C-TIMELINE-01: Program Timeline Authority（Program Execution 层媒体时间线权威; 纯 Domain 零 GStreamer）
pub mod registry;
pub mod resolver;
pub mod resource; // 0.6E: Resource 模型 + 状态机 + Preflight 闸门 (防自动 Fallback)
pub mod rpc;
pub mod runtime_query; // P0.7C-2: Runtime Query Model (Pure Read / Snapshot Semantics)
pub mod runtime_state; // 0.7C-1: Canonical Runtime State 聚合 (组合非展开; D14 观察信封)
pub mod session; // P0-7A: MediaSession + SessionManager (RUNTIME_SESSION_MODEL 唯一 owner; D10 多输入)
pub mod signal; // 信号探测 + 亮度黑场检测
pub mod supervisor; // Gate 6/7: Runtime Supervisor — 恢复决策引擎（只决策, 不碰 GStreamer）
pub mod switch_execution; // A2-8-01: Switch Execution 执行面（ExecutionGroup/Intent/Plan; 纯模型零 GStreamer, probe §7 冻结）
pub mod timecode; // P0.7B-2C: Canonical Timecode (时间标签, 非时间本体; #148)
pub mod transport; // P0.7C-8: Transport (五端点 + P1b 静态文件面; std-only)
pub mod watchdog; // A2-0: Ingest Watchdog — Runtime Health/Recovery 模块
