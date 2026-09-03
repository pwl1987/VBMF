//! A2-8-01: GStreamer Switch Execution Adapter——Program graph 物化与真实切换。
//!
//! **topology = 实现细节（终裁冻结 #5, probe §7）**: 本文件内的 GStreamer
//! 拓扑选择可整体替换（换拓扑不经 Domain/契约变更; trait 面
//! `contracts/switch.rs` 零 GStreamer 词）。
//!
//! **v1 materialization（A2-8-01 盒上仿真验证形态）**: 单一 program
//! pipeline 内双源（is-live 测试源, 双源可区分）→ `input-selector`(video)
//! + `input-selector`(audio) → program appsink 观测出口。
//!
//! 切换 = 双平面同目标 `active-pad` 置位（video+audio 成对——方案 A;
//! input-selector 于下一缓冲到达时生效 = 帧边界对齐）。
//!
//! **inter 系跨管线隧道（真机 SDI 双输入）= A2-8-02 候选拓扑**: 需输入侧
//! intervideosink/intervideosrc 注入面（触及既有输入管线构链表面——本
//! change 不动 `pipeline.rs`/既有 `build_pipeline`, 02 前置设计点已登记
//! 于 tasks.md）。trait 接口对两种拓扑同一。
//!
//! 观测 = **Observed 平面实测**: `active-pad` 属性回读（非命令回显）+
//! program appsink 帧计数/PTS 三态（经 `HEALTH_ARCS` 注册, 与输入管线同
//! 机制）+ 组输入管线健康弧读数（零第二 registry）。

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;

use gstreamer::prelude::*;
use uuid::Uuid;

use crate::contracts::switch::{
    FrameBoundary, InputPts, ProgramObservation, SwitchExecuted, SwitchExecutionAdapter,
};
use crate::pipeline::{PipelineHandle, PipelineHealth, PtsMonotonicity, NEXT_PIPELINE_ID};
use crate::program::SwitchPolicy;
use crate::switch_execution::{ExecutionGroup, SwitchDesired, SwitchError, SwitchExecutionPlan};

/// 一张已物化的 Program graph（真实 GStreamer 元素引用 + 观测簿记）。
struct SwitchGraph {
    devices: [Uuid; 2],
    /// 组输入 (device, handle)——observe 读输入健康弧（无第二 registry）。
    input_handles: [(Uuid, PipelineHandle); 2],
    started: bool,
    av_epoch: u64,
    pipeline: gstreamer::Pipeline,
    video_selector: gstreamer::Element,
    audio_selector: gstreamer::Element,
    /// device → selector sink pad 序（video/audio 平面同命名: sink_0/sink_1）。
    pad_index: HashMap<Uuid, usize>,
}

impl SwitchGraph {
    fn pad_name(&self, device: Uuid) -> Option<String> {
        self.pad_index.get(&device).map(|i| format!("sink_{i}"))
    }

    /// 实际 active 平面读数（Observed——属性回读非命令记忆）。
    fn observed_plane(selector: &gstreamer::Element) -> Option<String> {
        let pad: Option<gstreamer::Pad> = selector.property("active-pad");
        pad.map(|p| p.name().to_string())
    }

    fn device_of_pad(&self, pad_name: &str) -> Option<Uuid> {
        let idx: usize = pad_name.rsplit('_').next()?.parse().ok()?;
        self.devices
            .iter()
            .find(|d| self.pad_index.get(d) == Some(&idx))
            .copied()
    }

    /// 置位某平面 active-pad（pad 引用自 selector 自身 static pad）。
    fn set_active(
        selector: &gstreamer::Element,
        device: Uuid,
        graph: &SwitchGraph,
    ) -> Result<(), SwitchError> {
        let pad_name = graph
            .pad_name(device)
            .ok_or_else(|| SwitchError::Backend("selector pad 映射缺失".into()))?;
        let pad = selector
            .static_pad(&pad_name)
            .ok_or_else(|| SwitchError::Backend(format!("pad {pad_name} 不可得")))?;
        // gstreamer-rs set_property 失败时 panic（属性存在于 input-selector,
        // 类型匹配由 pad 类型保证）——此处无 Err 路径。
        selector.set_property("active-pad", &pad);
        Ok(())
    }
}

/// GStreamer Switch Execution Adapter（`Default` 构造）。
#[derive(Default)]
pub struct GStreamerSwitchAdapter {
    graphs: Mutex<HashMap<PipelineHandle, SwitchGraph>>,
}

fn make_element(factory: &str, name: &str) -> Result<gstreamer::Element, SwitchError> {
    gstreamer::ElementFactory::make(factory)
        .name(name)
        .build()
        .map_err(|e| SwitchError::Backend(format!("{factory} 构造失败: {e}")))
}

fn map_bool_err(e: glib::BoolError, what: &str) -> SwitchError {
    SwitchError::Backend(format!("{what}: {e}"))
}

/// 注册 program 出口 appsink 观测回调（帧计数/首帧 PTS/PTS 三态——与输入
/// 管线 `attach_video_sink` 同语义, 经 `HEALTH_ARCS` 归档）。
fn attach_program_video_sink(sink: &gstreamer_app::AppSink, handle: PipelineHandle) {
    sink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gstreamer::FlowError::Eos)?;
                let buf = sample.buffer().ok_or(gstreamer::FlowError::Error)?;
                if let Some(h) = crate::pipeline_events::HEALTH_ARCS
                    .lock()
                    .unwrap()
                    .get(&handle)
                {
                    let mut h = h.lock().unwrap();
                    h.video_frame_count += 1;
                    if let Some(pts) = buf.pts().map(|c| c.nseconds()) {
                        if h.video_first_pts.is_none() {
                            h.video_first_pts = Some(pts);
                        }
                        h.observe_video_pts(pts);
                    }
                }
                Ok(gstreamer::FlowSuccess::Ok)
            })
            .build(),
    );
}

fn attach_program_audio_sink(sink: &gstreamer_app::AppSink, handle: PipelineHandle) {
    sink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gstreamer::FlowError::Eos)?;
                let buf = sample.buffer().ok_or(gstreamer::FlowError::Error)?;
                if let Some(h) = crate::pipeline_events::HEALTH_ARCS
                    .lock()
                    .unwrap()
                    .get(&handle)
                {
                    let mut h = h.lock().unwrap();
                    h.audio_frame_count += 1;
                    if let Some(pts) = buf.pts().map(|c| c.nseconds()) {
                        if h.audio_first_pts.is_none() {
                            h.audio_first_pts = Some(pts);
                        }
                        h.observe_audio_pts(pts);
                    }
                }
                Ok(gstreamer::FlowSuccess::Ok)
            })
            .build(),
    );
}

impl GStreamerSwitchAdapter {
    /// 物化 program pipeline（双测试源 → 双 input-selector → 双 appsink）。
    /// 返回 (pipeline, video_selector, audio_selector)。
    fn build_program_pipeline(
        handle: PipelineHandle,
        devices: [Uuid; 2],
        initial_active: Uuid,
    ) -> Result<(gstreamer::Pipeline, gstreamer::Element, gstreamer::Element), SwitchError> {
        gstreamer::init().map_err(|e| SwitchError::Backend(format!("gst init: {e}")))?;

        let pipeline = gstreamer::Pipeline::builder().name("program-graph").build();

        // —— video 平面: 双测试源（可区分 pattern）→ input-selector → appsink ——
        let video_selector = make_element("input-selector", "program-video-selector")?;
        let v_caps = make_element("capsfilter", "program-video-caps")?;
        v_caps.set_property(
            "caps",
            gstreamer::Caps::from_str("video/x-raw,width=320,height=240,framerate=25/1")
                .expect("caps 字面量恒可解析"),
        );
        let v_queue = make_element("queue", "program-video-queue")?;
        let v_sink_el = make_element("appsink", "program-video-sink")?;
        v_sink_el.set_property("sync", false);
        v_sink_el.set_property("async", false);
        let v_appsink = v_sink_el
            .clone()
            .dynamic_cast::<gstreamer_app::AppSink>()
            .map_err(|e| SwitchError::Backend(format!("appsink cast: {e:?}")))?;
        attach_program_video_sink(&v_appsink, handle);

        let vsrc_a = make_element("videotestsrc", "program-vsrc-a")?;
        vsrc_a.set_property("is-live", true);
        vsrc_a.set_property_from_str("pattern", "ball");
        let vsrc_b = make_element("videotestsrc", "program-vsrc-b")?;
        vsrc_b.set_property("is-live", true);
        vsrc_b.set_property_from_str("pattern", "smpte");

        for el in [
            &vsrc_a,
            &vsrc_b,
            &video_selector,
            &v_caps,
            &v_queue,
            &v_sink_el,
        ] {
            pipeline
                .add(el)
                .map_err(|e| map_bool_err(e, "add element"))?;
        }

        // —— audio 平面: 双 audiotestsrc（可区分频率）→ input-selector → appsink ——
        let audio_selector = make_element("input-selector", "program-audio-selector")?;
        let a_queue = make_element("queue", "program-audio-queue")?;
        let a_sink_el = make_element("appsink", "program-audio-sink")?;
        a_sink_el.set_property("sync", false);
        a_sink_el.set_property("async", false);
        let a_appsink = a_sink_el
            .clone()
            .dynamic_cast::<gstreamer_app::AppSink>()
            .map_err(|e| SwitchError::Backend(format!("appsink cast: {e:?}")))?;
        attach_program_audio_sink(&a_appsink, handle);

        let asrc_a = make_element("audiotestsrc", "program-asrc-a")?;
        asrc_a.set_property("is-live", true);
        asrc_a.set_property("freq", 440f64);
        let asrc_b = make_element("audiotestsrc", "program-asrc-b")?;
        asrc_b.set_property("is-live", true);
        asrc_b.set_property("freq", 880f64);

        for el in [&asrc_a, &asrc_b, &audio_selector, &a_queue, &a_sink_el] {
            pipeline
                .add(el)
                .map_err(|e| map_bool_err(e, "add element"))?;
        }

        // 链接: 源 → selector request pad（device 序 = pad 序, 请求序恰 sink_0/1）;
        // selector → queue/caps → appsink。
        let link_src = |src: &gstreamer::Element,
                        selector: &gstreamer::Element,
                        idx: usize|
         -> Result<(), SwitchError> {
            let pad = selector
                .request_pad_simple("sink_%u")
                .ok_or_else(|| SwitchError::Backend("selector request pad 失败".into()))?;
            debug_assert_eq!(
                pad.name(),
                format!("sink_{idx}"),
                "request pad 命名与簿记一致（新 selector 恰按请求序编号）"
            );
            src.static_pad("src")
                .ok_or_else(|| SwitchError::Backend("src pad 缺失".into()))?
                .link(&pad)
                .map_err(|e| SwitchError::Backend(format!("link selector: {e:?}")))?;
            Ok(())
        };
        link_src(&vsrc_a, &video_selector, 0)?;
        link_src(&vsrc_b, &video_selector, 1)?;
        link_src(&asrc_a, &audio_selector, 0)?;
        link_src(&asrc_b, &audio_selector, 1)?;

        video_selector
            .link(&v_queue)
            .map_err(|e| map_bool_err(e, "video 出口链"))?;
        v_queue
            .link(&v_caps)
            .map_err(|e| map_bool_err(e, "video 出口链"))?;
        v_caps
            .link(&v_sink_el)
            .map_err(|e| map_bool_err(e, "video 出口链"))?;
        audio_selector
            .link(&a_queue)
            .map_err(|e| map_bool_err(e, "audio 出口链"))?;
        a_queue
            .link(&a_sink_el)
            .map_err(|e| map_bool_err(e, "audio 出口链"))?;

        // 初始 active: 双平面同目标（成对——启动即对齐, 方案 A）。
        let pad_of = |dev: Uuid| -> String {
            let idx = devices
                .iter()
                .position(|d| *d == dev)
                .expect("initial_active 已校验 ∈ 组");
            format!("sink_{idx}")
        };
        for selector in [&video_selector, &audio_selector] {
            let name = pad_of(initial_active);
            let pad = selector
                .static_pad(&name)
                .ok_or_else(|| SwitchError::Backend(format!("initial pad {name} 缺失")))?;
            selector.set_property("active-pad", &pad);
        }

        // 健康弧注册（与输入管线同机制——program 出口观测归档）。
        crate::pipeline_events::HEALTH_ARCS.lock().unwrap().insert(
            handle,
            std::sync::Arc::new(Mutex::new(PipelineHealth::default())),
        );
        if let Some(hp) = crate::pipeline_events::HEALTH_ARCS
            .lock()
            .unwrap()
            .get(&handle)
        {
            hp.lock().unwrap().started_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
            );
        }

        Ok((pipeline, video_selector, audio_selector))
    }
}

impl SwitchExecutionAdapter for GStreamerSwitchAdapter {
    fn build_program_graph(&self, group: &ExecutionGroup) -> Result<PipelineHandle, SwitchError> {
        let initial_active = match group.desired {
            SwitchDesired::ActiveInput(active) => active,
            switching @ SwitchDesired::Switching { .. } => {
                return Err(SwitchError::NotActiveSource(switching))
            }
        };
        let devices = [group.inputs[0].device_id, group.inputs[1].device_id];
        let input_handles = [
            (group.inputs[0].device_id, group.inputs[0].handle),
            (group.inputs[1].device_id, group.inputs[1].handle),
        ];
        let handle =
            PipelineHandle(NEXT_PIPELINE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        let (pipeline, video_selector, audio_selector) =
            Self::build_program_pipeline(handle, devices, initial_active)?;
        let pad_index: HashMap<Uuid, usize> =
            devices.iter().enumerate().map(|(i, d)| (*d, i)).collect();
        self.graphs.lock().unwrap().insert(
            handle,
            SwitchGraph {
                devices,
                input_handles,
                started: false,
                av_epoch: 0,
                pipeline,
                video_selector,
                audio_selector,
                pad_index,
            },
        );
        Ok(handle)
    }

    fn start_program(&self, graph: &PipelineHandle) -> Result<(), SwitchError> {
        let mut graphs = self.graphs.lock().unwrap();
        let g = graphs
            .get_mut(graph)
            .ok_or(SwitchError::GraphNotRunning(*graph))?;
        g.pipeline
            .set_state(gstreamer::State::Playing)
            .map_err(|e| SwitchError::Backend(format!("program play: {e}")))?;
        g.started = true;
        drop(graphs);
        if let Some(hp) = crate::pipeline_events::HEALTH_ARCS
            .lock()
            .unwrap()
            .get(graph)
        {
            hp.lock().unwrap().playing = true;
        }
        Ok(())
    }

    fn switch(
        &self,
        graph: &PipelineHandle,
        plan: &SwitchExecutionPlan,
    ) -> Result<SwitchExecuted, SwitchError> {
        let mut graphs = self.graphs.lock().unwrap();
        let g = graphs
            .get_mut(graph)
            .ok_or(SwitchError::GraphNotRunning(*graph))?;
        if !g.started {
            return Err(SwitchError::GraphNotRunning(*graph));
        }
        if !g.devices.contains(&plan.target) {
            return Err(SwitchError::TargetNotInGroup(plan.target));
        }
        if plan.policy != SwitchPolicy::FrameSwitch {
            return Err(SwitchError::UnsupportedPolicy(plan.policy));
        }
        if plan.epoch != g.av_epoch + 1 {
            return Err(SwitchError::StalePlanEpoch {
                got: plan.epoch,
                expected: g.av_epoch + 1,
            });
        }
        // 成对切换: video+audio 双 selector 同目标 active-pad（同 epoch——
        // 方案 A; input-selector 于下一缓冲生效 = 帧边界对齐）。
        SwitchGraph::set_active(&g.video_selector, plan.target, g)?;
        SwitchGraph::set_active(&g.audio_selector, plan.target, g)?;
        g.av_epoch = plan.epoch;
        Ok(SwitchExecuted {
            boundary: FrameBoundary::FrameAligned,
            av_epoch: g.av_epoch,
        })
    }

    fn observe(&self, graph: &PipelineHandle) -> ProgramObservation {
        let graphs = self.graphs.lock().unwrap();
        let Some(g) = graphs.get(graph) else {
            return ProgramObservation {
                observed_active: None,
                video_active: None,
                audio_active: None,
                switch_epoch: 0,
                input_pts: Vec::new(),
                program_video_pts: None,
                program_audio_pts: None,
                program_video_pts_state: PtsMonotonicity::Unknown,
                program_audio_pts_state: PtsMonotonicity::Unknown,
                program_video_frames: 0,
                program_audio_frames: 0,
            };
        };
        let video_plane = SwitchGraph::observed_plane(&g.video_selector);
        let audio_plane = SwitchGraph::observed_plane(&g.audio_selector);
        let video_active = video_plane.as_deref().and_then(|n| g.device_of_pad(n));
        let audio_active = audio_plane.as_deref().and_then(|n| g.device_of_pad(n));
        // 联合判定: 双平面一致才构成 observed_active（分离态可观测——
        // av_paired 由消费方检出）。
        let observed_active = match (video_active, audio_active) {
            (Some(v), Some(a)) if v == a => Some(v),
            _ => None,
        };

        // 输入 PTS: 读组输入健康弧（无弧 = 无证据——absence≠evidence）。
        let input_pts = g
            .input_handles
            .iter()
            .map(|(d, h)| {
                let health = crate::pipeline_events::read_health(h);
                InputPts {
                    device_id: *d,
                    video_pts: health.as_ref().and_then(|x| x.video_last_pts),
                    audio_pts: health.as_ref().and_then(|x| x.audio_last_pts),
                    video_pts_state: health
                        .as_ref()
                        .map(|x| x.video_pts_state)
                        .unwrap_or(PtsMonotonicity::Unknown),
                    audio_pts_state: health
                        .as_ref()
                        .map(|x| x.audio_pts_state)
                        .unwrap_or(PtsMonotonicity::Unknown),
                    stalled: false,
                }
            })
            .collect();

        let ph = crate::pipeline_events::read_health(graph);
        ProgramObservation {
            observed_active: g.started.then_some(observed_active).flatten(),
            video_active: g.started.then_some(video_active).flatten(),
            audio_active: g.started.then_some(audio_active).flatten(),
            switch_epoch: g.av_epoch,
            input_pts,
            program_video_pts: ph.as_ref().and_then(|x| x.video_last_pts),
            program_audio_pts: ph.as_ref().and_then(|x| x.audio_last_pts),
            program_video_pts_state: ph
                .as_ref()
                .map(|x| x.video_pts_state)
                .unwrap_or(PtsMonotonicity::Unknown),
            program_audio_pts_state: ph
                .as_ref()
                .map(|x| x.audio_pts_state)
                .unwrap_or(PtsMonotonicity::Unknown),
            program_video_frames: ph.as_ref().map(|x| x.video_frame_count).unwrap_or(0),
            program_audio_frames: ph.as_ref().map(|x| x.audio_frame_count).unwrap_or(0),
        }
    }

    fn stop_program(&self, graph: &PipelineHandle) -> Result<(), SwitchError> {
        let mut graphs = self.graphs.lock().unwrap();
        if let Some(mut g) = graphs.remove(graph) {
            let _ = g.pipeline.set_state(gstreamer::State::Null);
            g.started = false;
        }
        crate::pipeline_events::HEALTH_ARCS
            .lock()
            .unwrap()
            .remove(graph);
        Ok(())
    }
}

// —— 真实 GStreamer 执行验证（盒上 `cargo test --features bmd-provider,gstreamer-backend`）——
// 证明"真实 Execution Graph + 真实 A/B 切换"（A2-8-01 完成标准）:
// 真元素/真数据流/真 active-pad 翻转/真 PTS——非模拟状态字段。
#[cfg(all(test, feature = "gstreamer-backend"))]
mod tests {
    use super::*;
    use crate::session::{SessionId, SessionInput};
    use crate::switch_execution::SwitchIntent;

    fn input(device_id: Uuid, handle: u64) -> SessionInput {
        SessionInput {
            device_id,
            handle: PipelineHandle(handle),
        }
    }

    fn group(a: Uuid, b: Uuid) -> ExecutionGroup {
        ExecutionGroup::new(
            SessionId(Uuid::new_v4()),
            vec![input(a, 900_001), input(b, 900_002)],
            a,
        )
        .expect("合法双输入组")
    }

    fn wait_frames(adapter: &GStreamerSwitchAdapter, graph: &PipelineHandle, ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
        let _ = adapter.observe(graph);
    }

    #[test]
    fn switch_graph_rt_01_real_program_graph_switch() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut g = group(a, b);
        let adapter = GStreamerSwitchAdapter::default();
        let graph = adapter
            .build_program_graph(&g)
            .expect("真实 program graph 物化");
        adapter.start_program(&graph).expect("program 启动");
        wait_frames(&adapter, &graph, 1200);

        // 真实数据流 + 初始 active=A。
        let obs = adapter.observe(&graph);
        assert_eq!(
            obs.observed_active,
            Some(a),
            "初始 active=A（pad 属性回读）"
        );
        assert!(obs.program_video_frames > 0, "真实 video 帧在流");
        assert!(obs.program_audio_frames > 0, "真实 audio 帧在流");
        let v_pts_before = obs.program_video_pts.expect("video PTS 在");

        // A→B 真实切换（selector active-pad 实态翻转）。
        let plan = g
            .plan_switch(&SwitchIntent {
                target: b,
                policy: SwitchPolicy::FrameSwitch,
            })
            .expect("计划");
        g.begin_switch(&plan).expect("begin");
        let executed = adapter.switch(&graph, &plan).expect("真实切换执行");
        assert_eq!(executed.boundary, FrameBoundary::FrameAligned);
        assert_eq!(executed.av_epoch, 1);
        wait_frames(&adapter, &graph, 600);

        let after = adapter.observe(&graph);
        assert_eq!(after.observed_active, Some(b), "active-pad 实际翻到 B");
        assert_eq!(after.video_active, Some(b));
        assert_eq!(after.audio_active, Some(b), "audio 平面同切（成对）");
        assert!(
            after.program_video_frames > obs.program_video_frames,
            "切换后出口持续产出"
        );
        assert!(
            after.program_video_pts.expect("PTS 在") >= v_pts_before,
            "切换前后 program PTS 不回退"
        );
        assert!(g.complete_switch(b), "Observed=B 落定 Desired");
        assert_eq!(g.desired, SwitchDesired::ActiveInput(b));

        adapter.stop_program(&graph).expect("停止");
    }

    #[test]
    fn switch_graph_rt_01_real_ba_roundtrip_paired() {
        // T3 完整形: A→B→A 双向 + 全程成对 + 出口存活。
        //
        // **边界事实登记（T5 实证, 终裁预警验证）**: v1 selector 原生透传
        // 源时间戳——回切（B→A）边界可能出现 <1 帧的 PTS 后跳, 三态机
        // 如实检出 NonMonotonic（sticky）= 观测系统按设计工作（回退
        // 检出即 T5 rollback detection 的真实 GStreamer 证明）。
        // 广播级连续时间线 = 出口再生成平面（02 编码出口/再戳记, A2-8-04
        // 验收项）——不在 01 伪装（单测试只断言期内单调 + 边界检出）。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut g = group(a, b);
        let adapter = GStreamerSwitchAdapter::default();
        let graph = adapter.build_program_graph(&g).expect("物化");
        adapter.start_program(&graph).expect("启动");
        wait_frames(&adapter, &graph, 1000);

        // 期内（A active, 未切换）: PTS 单调确定性成立。
        let pre = adapter.observe(&graph);
        assert_eq!(pre.observed_active, Some(a));
        assert_eq!(
            pre.program_video_pts_state,
            PtsMonotonicity::ValidMonotonic,
            "期内单调"
        );

        let mut last_frames = pre.program_video_frames;
        for target in [b, a] {
            let plan = g
                .plan_switch(&SwitchIntent {
                    target,
                    policy: SwitchPolicy::FrameSwitch,
                })
                .expect("计划");
            g.begin_switch(&plan).expect("begin");
            adapter.switch(&graph, &plan).expect("执行");
            wait_frames(&adapter, &graph, 600);
            let obs = adapter.observe(&graph);
            assert_eq!(obs.observed_active, Some(target), "双向切换均生效");
            assert_eq!(obs.video_active, obs.audio_active, "全程成对");
            assert!(g.complete_switch(target));
            assert!(obs.program_video_frames > last_frames, "出口全程持续产出");
            last_frames = obs.program_video_frames;
            assert!(obs.program_video_pts.is_some(), "PTS 全程可观测");
            // 边界后跳检出 = 观测能力（状态非 Unknown 即三态机在工作;
            // 后跳发生时恰为 NonMonotonic——回退不掩盖）。
            assert_ne!(obs.program_video_pts_state, PtsMonotonicity::Unknown);
        }
        assert_eq!(g.desired, SwitchDesired::ActiveInput(a));
        adapter.stop_program(&graph).expect("停止");
    }
}
