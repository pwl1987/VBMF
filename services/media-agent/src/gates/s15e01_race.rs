//! S15-E01 竞争窗确定性复现 Gate（Step 15 8h cycle 908 修复闭环）。
//!
//! 裁决 2026-09-08（修复③ seqnum 暂存+回放）新增回归判据的**真机层**:
//! 强制让新世代 Segment 事件到达发生在 `executed=true` 之前——必须最终
//! 被 timeline collector 计入（outcome=preserved）, 不得永久卡
//! `AwaitSegmentEvent` / 不得 `EvidenceInsufficient` 证据超时。
//! 确定性层 = switch_graph.rs 内联 S15-E01 四锁（盒 hw 测试矩阵）。
//!
//! 注入机构 = FaultControlWrapper 旋钮 `pre_switch_segment_video`
//! （默认 off; 一次性消费）: 在 `switch()` trait 委托**前**触发
//! `GStreamerSwitchAdapter::inject_pre_executed_segment` 接缝——此刻编排
//! 已完成 arm+install（fence Armed / timeline installed / executed=false）,
//! 恰为竞争窗入口。**确定性强制, 非概率轰击**（真机自然命中 ~0.1%/switch,
//! cycle 908 首证; 红态证据即该历史档, 不在未修复 bin 上回放）。
//! 接缝与真实 EVENT 探针同一生产函数（fence capture + ⑤ 暂存）, 并以
//! 同一 seqnum 配对驱动下游确认落点——等价于事件真实穿透两探针的净效果,
//! 零媒体流扰动（appsink 不收伪 Segment）。
//!
//! exit 惯例同 gates 族: 0=判据全绿 / 2=前置失败或断言失败（fail-closed）。

// 全量 feature 门控（同 r64_control_plane/dual_input 惯例）: 默认构建零编译面。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::sync::Arc;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use std::time::Instant;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::config::Config;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::contracts::provider::DiscoveredDevice;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::device::DeviceInfo;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::events::{RuntimeEventLog, RuntimeEventSink};
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::health::AgentState;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::lease::InMemoryLeaseManager;
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use crate::supervisor::Supervisor;

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
use super::r64_control_plane::{
    api_program_switch, build_gate_world, checkpoint, chk, post_stop, post_switch, GateWorld,
    Quiescence,
};

/// 入口（bin/gates.rs 派发; env `VBMF_A2_8_S15E01_RACE` 存在即跑）。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
#[allow(clippy::too_many_arguments)]
pub fn run(
    cfg: &Config,
    devices: &[DeviceInfo],
    discovered: &[DiscoveredDevice],
    lm: &Arc<InMemoryLeaseManager>,
    _sup: &Arc<std::sync::Mutex<Supervisor>>,
    agent_state: &Arc<std::sync::Mutex<AgentState>>,
    event_sink: &Arc<dyn RuntimeEventSink>,
    projection_log: &Arc<RuntimeEventLog>,
) {
    if std::env::var("VBMF_A2_8_S15E01_RACE").is_err() {
        return;
    }
    println!("=== S15-E01 竞争窗确定性复现（强制 pre-executed Segment → 修复③回放计入）===");
    println!(
        "披露: 注入=adapter 内生产函数级接缝（fence capture+⑤暂存+下游确认\
         同 seqnum 成对）——非真实 pad 事件投递; 与 cycle 908 现场同一分裂面\
         （fence 收到/timeline 没收到）, 确定性强制非概率轰击"
    );
    println!(
        "披露: 判据面=outcome=preserved + program_epoch 保持 0（无 FailClosed/\
         NewEpoch rebase）+ segment_id 逐轮前进 + DD——先到 Segment 穿真实\
         evidence collector 计入; 红态证据=8h soak cycle 908 历史档\
         （r64-stability-8h/）, 不回放"
    );

    let mut failures: Vec<String> = Vec::new();
    let w = build_gate_world(
        cfg,
        devices,
        discovered,
        lm,
        _sup,
        agent_state,
        event_sink,
        projection_log,
    );
    scenario(&mut failures, &w);

    println!(
        "S15E01-RACE summary injections={} failures={}",
        w.wrapper.segment_injections(),
        failures.len()
    );
    if failures.is_empty() {
        println!(
            "S15E01-RACE verdict PASS（强制竞争窗×10 全部被回放计入——\
             修复③真机判据绿; exit 0）"
        );
        std::process::exit(0);
    }
    eprintln!(
        "S15E01-RACE verdict FAIL（{} 项断言失败——fail-closed exit 2）:",
        failures.len()
    );
    for f in &failures {
        eprintln!("  FAIL: {f}");
    }
    std::process::exit(2);
}

#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
fn scenario(failures: &mut Vec<String>, w: &GateWorld) {
    let (rt, addr, sid, a, b) = (&w.rt, w.addr, &w.sid, w.a, w.b);
    println!("S15E01-RACE world A={a} B={b} sid={}", sid.0);
    checkpoint(failures, "init", &Quiescence::Active(a), rt, addr, None);

    // 对照轮: 干净切换 A→B（旋钮 off——修复不扰动正常路径; av=1, epoch 保持 0）。
    let c0 = post_switch(addr, uuid::Uuid::new_v4(), sid, b);
    println!(
        "S15E01-RACE control A→B status={} class={:?} detail={:?}",
        c0.jstatus, c0.classification, c0.detail
    );
    chk(
        failures,
        c0.status == 200 && c0.jstatus == "executed",
        format!(
            "control 干净切换 executed: http={} {:?}",
            c0.status, c0.detail
        ),
    );
    chk(
        failures,
        c0.detail
            .as_deref()
            .is_some_and(|d| d.contains("outcome=preserved")),
        format!("control 干净切换 preserved: {:?}", c0.detail),
    );
    let (_, ps0) = api_program_switch(addr);
    chk(
        failures,
        ps0["switch_epoch"].as_u64() == Some(1)
            && ps0["timeline"]["program_epoch"].as_u64() == Some(0)
            && ps0["timeline"]["segment_id"].as_u64() == Some(1),
        format!("control 后 av=1 epoch=0（保持）seg=1: {ps0}"),
    );

    // 注入 ×10 交替: 每轮强制 "Segment 先于 executed=true 到达"。
    for i in 1u64..=10 {
        let target = if i % 2 == 1 { a } else { b };
        w.wrapper.arm_pre_switch_segment();
        let t0 = Instant::now();
        let c = post_switch(addr, uuid::Uuid::new_v4(), sid, target);
        let elapsed = t0.elapsed();
        println!(
            "S15E01-RACE race[{i}] →{target} elapsed={elapsed:?} status={} class={:?} detail={:?}",
            c.jstatus, c.classification, c.detail
        );
        chk(
            failures,
            c.status == 200,
            format!("race[{i}] http={}", c.status),
        );
        chk(
            failures,
            c.jstatus == "executed"
                && c.detail
                    .as_deref()
                    .is_some_and(|d| d.contains("outcome=preserved")),
            format!(
                "race[{i}] 注入轮必须 Preserved（先到 Segment 被 timeline collector\
                 回放计入——不得卡 AwaitSegmentEvent）: {:?}",
                c.detail
            ),
        );
        chk(
            failures,
            !c.detail
                .as_deref()
                .is_some_and(|d| d.contains("证据超时") || d.contains("EvidenceInsufficient")),
            format!("race[{i}] 不得出现证据超时形态: {:?}", c.detail),
        );
        let (_, ps) = api_program_switch(addr);
        chk(
            failures,
            ps["switch_epoch"].as_u64() == Some(i + 1),
            format!("race[{i}] av={}（每轮恰 +1）: {ps}", i + 1),
        );
        chk(
            failures,
            ps["timeline"]["program_epoch"].as_u64() == Some(0),
            format!(
                "race[{i}] program_epoch=0 保持（成功切换=同纪元连续; \
                 无 FailClosed→NewEpoch rebase=竞争窗被修复吸收的直接证明）: {ps}"
            ),
        );
        chk(
            failures,
            ps["timeline"]["segment_id"].as_u64() == Some(i + 1),
            format!("race[{i}] segment_id={}（新声明段逐轮落定）: {ps}", i + 1),
        );
        chk(
            failures,
            ps["timeline"]["discontinuity_state"].as_str() == Some("discontinuity_declared"),
            format!("race[{i}] DD 边界事实: {ps}"),
        );
    }
    chk(
        failures,
        w.wrapper.segment_injections() == 10,
        format!(
            "注入计数对账: {}（期望 10——一次性消费无遗漏）",
            w.wrapper.segment_injections()
        ),
    );
    // 终态: 十次强制竞争窗后 R53 签名 re-proved continuous（映射续流连续）。
    let (_, ps) = api_program_switch(addr);
    chk(
        failures,
        ps["timeline"]["video_continuity"].as_str() == Some("continuous"),
        format!("race 后 video_continuity=continuous: {ps}"),
    );
    checkpoint(
        failures,
        "after-races",
        &Quiescence::Active(b),
        rt,
        addr,
        None,
    );

    // 收尾: stop_session teardown（唯一兜底出口——gate exit 判据）。
    let cstop = post_stop(addr, uuid::Uuid::new_v4(), sid);
    println!(
        "S15E01-RACE stop status={} class={:?} detail={:?}",
        cstop.jstatus, cstop.classification, cstop.detail
    );
    chk(
        failures,
        cstop.jstatus == "executed",
        "stop_session executed（兜底出口可用）".into(),
    );
    std::thread::sleep(std::time::Duration::from_secs(2));
    chk(
        failures,
        !rt.is_active(),
        "teardown 后 is_active=false".into(),
    );
    checkpoint(
        failures,
        "teardown",
        &Quiescence::Teardown,
        rt,
        addr,
        Some((cstop.jstatus.clone(), cstop.classification.clone())),
    );
}
