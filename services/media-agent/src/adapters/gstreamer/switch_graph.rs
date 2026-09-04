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
    /// 构造期 Desired 提取的初始 active（start_program 时点亮 active 簿记）。
    initial_active: Uuid,
    /// Execution 实态簿记: 当前 active 源（start 时=initial; switch 推进）。
    /// F-05 修复: 此前缺失导致 switch() 无"切当前 active"纵深拒收（Mock
    /// 适配器有而真适配器漏——伪造 plan 可对 active 源重复执行）。
    active: Option<Uuid>,
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

/// Program graph 物化形态（topology=实现细节, probe §7 冻结 #5——两形态
/// 对 trait 接口同一, 可整体替换）。
///
/// - `Simulation`: 自持 is-live 测试源（自包含验证——无需输入管线）;
/// - `Bridged`: **inter 系跨管线桥**（A2-8-02-F-03/04）——源=
///   `intervideosrc/interaudiosrc`，channel 经 `program_execution::
///   tap_channel`（DeviceId 派生 execution bridge address, 唯一约定
///   来源）消费输入管线 MediaTap 挂出的媒体面。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SwitchMaterialization {
    #[default]
    Simulation,
    Bridged,
}

/// GStreamer Switch Execution Adapter（`Default`=Simulation; 生产桥接用
/// `bridged()`）。
#[derive(Default)]
pub struct GStreamerSwitchAdapter {
    graphs: Mutex<HashMap<PipelineHandle, SwitchGraph>>,
    mode: SwitchMaterialization,
}

impl GStreamerSwitchAdapter {
    /// 自包含仿真形态（测试源自持）。
    pub fn simulation() -> Self {
        Self::default()
    }

    /// inter 系跨管线桥形态（消费 MediaTap channel——F-03/F-04）。
    pub fn bridged() -> Self {
        Self {
            mode: SwitchMaterialization::Bridged,
            ..Self::default()
        }
    }
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
    /// 物化 program pipeline（双源 → 双 input-selector → 双 appsink）。
    /// 源形态按 `mode`: Simulation=自持测试源 / Bridged=inter 系跨管线
    /// 桥（intervideosrc/interaudiosrc 消费 MediaTap channel——
    /// F-03/F-04; channel 经 `tap_channel` 唯一约定从 DeviceId 派生）。
    /// 返回 (pipeline, video_selector, audio_selector)。
    fn build_program_pipeline(
        handle: PipelineHandle,
        mode: SwitchMaterialization,
        devices: [Uuid; 2],
        initial_active: Uuid,
    ) -> Result<(gstreamer::Pipeline, gstreamer::Element, gstreamer::Element), SwitchError> {
        gstreamer::init().map_err(|e| SwitchError::Backend(format!("gst init: {e}")))?;

        let pipeline = gstreamer::Pipeline::builder().name("program-graph").build();

        // —— video 平面: 双源 → input-selector → appsink ——
        let video_selector = make_element("input-selector", "program-video-selector")?;
        // capsfilter 仅 Simulation 形态（Bridged 透传输入管线实际 caps——
        // 强制 320x240 会与桥接媒体协商冲突）。
        let v_caps = match mode {
            SwitchMaterialization::Simulation => {
                let c = make_element("capsfilter", "program-video-caps")?;
                c.set_property(
                    "caps",
                    gstreamer::Caps::from_str("video/x-raw,width=320,height=240,framerate=25/1")
                        .expect("caps 字面量恒可解析"),
                );
                Some(c)
            }
            SwitchMaterialization::Bridged => None,
        };
        let v_queue = make_element("queue", "program-video-queue")?;
        let v_sink_el = make_element("appsink", "program-video-sink")?;
        v_sink_el.set_property("sync", false);
        v_sink_el.set_property("async", false);
        let v_appsink = v_sink_el
            .clone()
            .dynamic_cast::<gstreamer_app::AppSink>()
            .map_err(|e| SwitchError::Backend(format!("appsink cast: {e:?}")))?;
        attach_program_video_sink(&v_appsink, handle);

        let (vsrc_a, vsrc_b) = match mode {
            SwitchMaterialization::Simulation => {
                let a = make_element("videotestsrc", "program-vsrc-a")?;
                a.set_property("is-live", true);
                a.set_property_from_str("pattern", "ball");
                let b = make_element("videotestsrc", "program-vsrc-b")?;
                b.set_property("is-live", true);
                b.set_property_from_str("pattern", "smpte");
                (a, b)
            }
            SwitchMaterialization::Bridged => {
                // inter 系桥: 消费输入管线 MediaTap 挂出的 channel
                //（tap_channel=DeviceId 派生唯一约定来源）。
                let a = make_element("intervideosrc", "program-vsrc-a")?;
                a.set_property("channel", crate::program_execution::tap_channel(devices[0]));
                let b = make_element("intervideosrc", "program-vsrc-b")?;
                b.set_property("channel", crate::program_execution::tap_channel(devices[1]));
                (a, b)
            }
        };

        let mut video_els: Vec<&gstreamer::Element> =
            vec![&vsrc_a, &vsrc_b, &video_selector, &v_queue, &v_sink_el];
        if let Some(c) = &v_caps {
            video_els.push(c);
        }
        for el in video_els {
            pipeline
                .add(el)
                .map_err(|e| map_bool_err(e, "add element"))?;
        }

        // —— audio 平面: 双源 → input-selector → appsink ——
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

        let (asrc_a, asrc_b) = match mode {
            SwitchMaterialization::Simulation => {
                let a = make_element("audiotestsrc", "program-asrc-a")?;
                a.set_property("is-live", true);
                a.set_property("freq", 440f64);
                let b = make_element("audiotestsrc", "program-asrc-b")?;
                b.set_property("is-live", true);
                b.set_property("freq", 880f64);
                (a, b)
            }
            SwitchMaterialization::Bridged => {
                let a = make_element("interaudiosrc", "program-asrc-a")?;
                a.set_property("channel", crate::program_execution::tap_channel(devices[0]));
                let b = make_element("interaudiosrc", "program-asrc-b")?;
                b.set_property("channel", crate::program_execution::tap_channel(devices[1]));
                (a, b)
            }
        };

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
        match &v_caps {
            Some(c) => {
                v_queue
                    .link(c)
                    .map_err(|e| map_bool_err(e, "video 出口链"))?;
                c.link(&v_sink_el)
                    .map_err(|e| map_bool_err(e, "video 出口链"))?;
            }
            None => {
                v_queue
                    .link(&v_sink_el)
                    .map_err(|e| map_bool_err(e, "video 出口链"))?;
            }
        }
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
            Self::build_program_pipeline(handle, self.mode, devices, initial_active)?;
        let pad_index: HashMap<Uuid, usize> =
            devices.iter().enumerate().map(|(i, d)| (*d, i)).collect();
        self.graphs.lock().unwrap().insert(
            handle,
            SwitchGraph {
                devices,
                input_handles,
                started: false,
                initial_active,
                active: None,
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
        // Execution 实态初始化: 启动即 active=initial（构造期已置初始 pad）。
        g.active = Some(g.initial_active);
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
        // F-05 修复: 切当前 active 源纵深拒收（与 Mock 适配器对齐——
        // 此前真适配器缺失, 伪造 plan 可对 active 源重复执行）。
        if g.active == Some(plan.target) {
            return Err(SwitchError::TargetAlreadyActive(plan.target));
        }
        // 成对切换: video+audio 双 selector 同目标 active-pad（同 epoch——
        // 方案 A; input-selector 于下一缓冲生效 = 帧边界对齐）。
        SwitchGraph::set_active(&g.video_selector, plan.target, g)?;
        SwitchGraph::set_active(&g.audio_selector, plan.target, g)?;
        g.active = Some(plan.target);
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

    // ── A2-8-02-F-03/F-04: 真实跨管线 Program Media Path（十一轮终裁十项
    // 清单; 盒上 bmd+gstreamer 非 mock）——输入管线（videotestsrc 真实帧,
    // 无 SDI 亦真跑）→ tee → MediaTap[intervideosink/interaudiosink] →
    // inter[video/audio]src → selector → program appsink。
    // 清单映射: ①双输入真实帧 ②channel 正确 ③program 有帧 ④A→B→A
    // ⑤双平面成对 ⑥A 断 B 仍供 ⑦B 断（经⑥对偶, 断非 active 路）⑨teardown
    // 零残留 ⑩recover 重挂。⑧program 自身故障独立观察=观测维度分离已由
    // fold 测试证（mock 层 GroupObservation 三维分离）——不在真桥强注。
    // **不做任何 PTS 行为修改**（Timeline=G/H 独立裁决）。
    #[cfg(all(
        feature = "bmd-provider",
        feature = "gstreamer-backend",
        not(feature = "mock")
    ))]
    #[test]
    fn switch_graph_rt_01_real_bridge_cross_pipeline_media_path() {
        use crate::contracts::media_tap::{MediaTapRequest, TapPlanes};
        use crate::pipeline::PipelinePlan;
        use crate::program_execution::tap_channel;
        use crate::session::SessionId;

        let bundle =
            crate::registry::AdapterRegistry::build_media_adapter_bundle().expect("bundle");
        let tap = bundle.media_tap.clone().expect("tap view");

        // 两条真实输入管线（self_test=videotestsrc 源——真实帧/真实 tee）。
        let h1 = bundle
            .backend
            .instantiate(&PipelinePlan::self_test())
            .expect("输入管线 A");
        let h2 = bundle
            .backend
            .instantiate(&PipelinePlan::self_test())
            .expect("输入管线 B");
        bundle.backend.start(&h1).expect("启动 A");
        bundle.backend.start(&h2).expect("启动 B");
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        // ② channel 正确（tap_channel 唯一约定）。
        for (h, d) in [(h1, a), (h2, b)] {
            tap.attach_media_tap(
                &h,
                &MediaTapRequest {
                    channel: tap_channel(d),
                    planes: TapPlanes::Both,
                },
            )
            .expect("tap attach");
            assert_eq!(tap.tap_attachments(&h)[0].channel, tap_channel(d));
        }

        // Bridged program graph: inter[video/audio]src 消费 tap channel。
        let mut group = ExecutionGroup::new(
            SessionId(Uuid::new_v4()),
            vec![
                SessionInput {
                    device_id: a,
                    handle: h1,
                },
                SessionInput {
                    device_id: b,
                    handle: h2,
                },
            ],
            a,
        )
        .expect("组");
        let adapter = GStreamerSwitchAdapter::bridged();
        let graph = adapter.build_program_graph(&group).expect("bridged 物化");
        adapter.start_program(&graph).expect("program 启动");
        std::thread::sleep(std::time::Duration::from_millis(1500));

        // ① 双输入真实帧 + ③ program 有帧（跨管线桥真实流通）。
        let ha = crate::pipeline_events::read_health(&h1).expect("A 健康弧");
        let hb = crate::pipeline_events::read_health(&h2).expect("B 健康弧");
        assert!(ha.video_frame_count > 0, "A 真实帧");
        assert!(hb.video_frame_count > 0, "B 真实帧");
        let obs = adapter.observe(&graph);
        assert_eq!(obs.observed_active, Some(a), "初始 active=A（桥回读）");
        assert!(obs.program_video_frames > 0, "program video 帧经桥到达");
        assert!(obs.program_audio_frames > 0, "program audio 帧经桥到达");

        // ④⑤ A→B→A 真实切换 + 双平面成对（桥形态）。
        for target in [b, a] {
            let plan = group
                .plan_switch(&SwitchIntent {
                    target,
                    policy: SwitchPolicy::FrameSwitch,
                })
                .expect("计划");
            group.begin_switch(&plan).expect("begin");
            adapter.switch(&graph, &plan).expect("真实切换");
            std::thread::sleep(std::time::Duration::from_millis(600));
            let o = adapter.observe(&graph);
            assert_eq!(o.observed_active, Some(target), "桥形态切换生效");
            assert_eq!(o.video_active, o.audio_active, "双平面成对");
            assert!(o.program_video_frames > 0, "切换后 program 持续");
            assert!(group.complete_switch(target));
        }

        // ⑥ 严格 standby 隔离（第十二轮修正——原序有误: [b,a] 循环后
        // active=A, 直接停 h1 停的是 **active 源**而非 standby）:
        // 正确序 = 先切到 B 并确认 observed=B, 再停 A（standby）→
        // B 作为 active source 独立持续供桥。
        let plan_b = group
            .plan_switch(&SwitchIntent {
                target: b,
                policy: SwitchPolicy::FrameSwitch,
            })
            .expect("⑥ 切 B 计划");
        group.begin_switch(&plan_b).expect("begin");
        adapter.switch(&graph, &plan_b).expect("切到 B");
        std::thread::sleep(std::time::Duration::from_millis(600));
        assert_eq!(
            adapter.observe(&graph).observed_active,
            Some(b),
            "⑥前置: active=B"
        );
        let frames_before = adapter.observe(&graph).program_video_frames;
        let _ = bundle.backend.stop(&h1); // standby A 全停（不可逆——⑦ 对偶在独立测试）
        std::thread::sleep(std::time::Duration::from_millis(600));
        let obs6 = adapter.observe(&graph);
        assert!(
            obs6.program_video_frames > frames_before,
            "⑥ A 断（standby, active=B）program 持续——B 独立供桥"
        );
        assert_eq!(obs6.observed_active, Some(b), "⑥ active 仍=B");

        // ⑩ 媒体路径恢复（第十二轮升级——非仅簿记）: recover 运行中的
        // active B → 簿记重放 → 帧继续增长 = 媒体真实重新穿越整条桥
        //（intervideosrc/interaudiosrc → selector → program appsink）。
        let frames_pre_recover = adapter.observe(&graph).program_video_frames;
        bundle.backend.recover(&h2).expect("B recover 重建");
        assert_eq!(
            tap.tap_attachments(&h2).len(),
            1,
            "⑩a recover 后 tap 簿记重放（新管线同 channel）"
        );
        std::thread::sleep(std::time::Duration::from_millis(900));
        let obs10 = adapter.observe(&graph);
        assert!(
            obs10.program_video_frames > frames_pre_recover,
            "⑩b recover 后媒体重新穿越全桥（帧增长）——media-path recovery"
        );
        assert_eq!(obs10.observed_active, Some(b), "⑩ active 维持=B");

        // ⑨ teardown 零残留: program 停 + 运行中 tap（h2）真摘; 已停 h1
        // 实例已移除（bookkeeping 随之不可达=空——结构性零残留）。
        adapter.stop_program(&graph).expect("program 停");
        let ch2 = tap
            .tap_attachments(&h2)
            .first()
            .map(|x| x.channel.clone())
            .expect("h2 tap 在");
        tap.detach_media_tap(&h2, &ch2).expect("tap 摘除");
        assert!(tap.tap_attachments(&h2).is_empty(), "⑨ h2 零残留");
        assert!(tap.tap_attachments(&h1).is_empty(), "⑨ h1 零残留");
        let _ = bundle.backend.stop(&h2);
    }

    // ⑦ 严格对偶（第十二轮新增——独立场景, 不可与⑥共用管线: stop 不可逆）:
    // active=A, standby B 全停 → A 独立持续供桥 + observed 维持=A。
    #[cfg(all(
        feature = "bmd-provider",
        feature = "gstreamer-backend",
        not(feature = "mock")
    ))]
    #[test]
    fn switch_graph_rt_01_real_bridge_standby_b_failure_dual() {
        use crate::contracts::media_tap::{MediaTapRequest, TapPlanes};
        use crate::pipeline::PipelinePlan;
        use crate::program_execution::tap_channel;
        use crate::session::SessionId;

        let bundle =
            crate::registry::AdapterRegistry::build_media_adapter_bundle().expect("bundle");
        let tap = bundle.media_tap.clone().expect("tap view");
        let h1 = bundle
            .backend
            .instantiate(&PipelinePlan::self_test())
            .expect("A");
        let h2 = bundle
            .backend
            .instantiate(&PipelinePlan::self_test())
            .expect("B");
        bundle.backend.start(&h1).expect("启动 A");
        bundle.backend.start(&h2).expect("启动 B");
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        for (h, d) in [(h1, a), (h2, b)] {
            tap.attach_media_tap(
                &h,
                &MediaTapRequest {
                    channel: tap_channel(d),
                    planes: TapPlanes::Both,
                },
            )
            .expect("tap attach");
        }
        let adapter = GStreamerSwitchAdapter::bridged();
        let graph = adapter
            .build_program_graph(
                &ExecutionGroup::new(
                    SessionId(Uuid::new_v4()),
                    vec![
                        SessionInput {
                            device_id: a,
                            handle: h1,
                        },
                        SessionInput {
                            device_id: b,
                            handle: h2,
                        },
                    ],
                    a,
                )
                .expect("组"),
            )
            .expect("bridged 物化");
        adapter.start_program(&graph).expect("启动");
        std::thread::sleep(std::time::Duration::from_millis(1200));
        assert_eq!(
            adapter.observe(&graph).observed_active,
            Some(a),
            "前置: active=A"
        );
        let frames_before = adapter.observe(&graph).program_video_frames;
        assert!(frames_before > 0, "program 帧在流");

        let _ = bundle.backend.stop(&h2); // standby B 全停
        std::thread::sleep(std::time::Duration::from_millis(600));
        let obs = adapter.observe(&graph);
        assert!(
            obs.program_video_frames > frames_before,
            "⑦ B 断（standby, active=A）program 持续——A 独立供桥"
        );
        assert_eq!(obs.observed_active, Some(a), "⑦ active 维持=A");
        assert_eq!(obs.video_active, obs.audio_active, "⑦ 双平面成对维持");

        adapter.stop_program(&graph).expect("program 停");
        let ch1 = tap
            .tap_attachments(&h1)
            .first()
            .map(|x| x.channel.clone())
            .expect("h1 tap 在");
        tap.detach_media_tap(&h1, &ch1).expect("摘除");
        assert!(tap.tap_attachments(&h1).is_empty());
        assert!(tap.tap_attachments(&h2).is_empty(), "已停 B 结构性零残留");
        let _ = bundle.backend.stop(&h1);
    }

    // ── A2-8-02-F-05: Multi-Switch / State Consistency（第十三轮 §16）——
    // 多跳 A→B→A→B→A 每跳六点验证 + 快速连切 + 四类 fail-closed 真适配器
    // 级 + ⑧真桥级区分性验收。禁: PTS/Session/Supervisor/PipelinePlan/
    // N-input（零触碰）。
    #[cfg(all(
        feature = "bmd-provider",
        feature = "gstreamer-backend",
        not(feature = "mock")
    ))]
    #[test]
    fn switch_graph_rt_01_real_bridge_multi_switch_state_consistency() {
        use crate::contracts::media_tap::{MediaTapRequest, TapPlanes};
        use crate::pipeline::PipelinePlan;
        use crate::program_execution::tap_channel;
        use crate::session::SessionId;
        use crate::switch_execution::{SwitchError, SwitchExecutionPlan};

        let bundle =
            crate::registry::AdapterRegistry::build_media_adapter_bundle().expect("bundle");
        let tap = bundle.media_tap.clone().expect("tap view");
        let h1 = bundle
            .backend
            .instantiate(&PipelinePlan::self_test())
            .expect("A");
        let h2 = bundle
            .backend
            .instantiate(&PipelinePlan::self_test())
            .expect("B");
        bundle.backend.start(&h1).expect("启动 A");
        bundle.backend.start(&h2).expect("启动 B");
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        for (h, d) in [(h1, a), (h2, b)] {
            tap.attach_media_tap(
                &h,
                &MediaTapRequest {
                    channel: tap_channel(d),
                    planes: TapPlanes::Both,
                },
            )
            .expect("tap attach");
        }
        let mut group = ExecutionGroup::new(
            SessionId(Uuid::new_v4()),
            vec![
                SessionInput {
                    device_id: a,
                    handle: h1,
                },
                SessionInput {
                    device_id: b,
                    handle: h2,
                },
            ],
            a,
        )
        .expect("组");
        let adapter = GStreamerSwitchAdapter::bridged();
        let graph = adapter.build_program_graph(&group).expect("bridged 物化");
        adapter.start_program(&graph).expect("启动");
        std::thread::sleep(std::time::Duration::from_millis(1200));

        // ── 多跳 A→B→A→B→A: 每跳六点验证[plan.target→selector→双平面→
        // observed→complete→Desired]+核心断言[成对/observed==target/
        // desired==target/epoch+=1/帧持续]。
        let mut prev_epoch = 0u64;
        let mut prev_frames = adapter.observe(&graph).program_video_frames;
        let mut prev_active = a;
        for (i, target) in [b, a, b, a].into_iter().enumerate() {
            let plan = group
                .plan_switch(&SwitchIntent {
                    target,
                    policy: SwitchPolicy::FrameSwitch,
                })
                .expect("计划");
            assert_eq!(plan.target, target, "跳{i}: plan.target 一致");
            assert_eq!(plan.epoch, prev_epoch + 1, "跳{i}: epoch 恰递增");
            group.begin_switch(&plan).expect("begin");
            assert_eq!(
                group.desired,
                SwitchDesired::Switching {
                    from: prev_active,
                    to: target
                },
                "跳{i}: Desired=Switching（三态不串之一）"
            );
            let executed = adapter.switch(&graph, &plan).expect("真实切换");
            assert_eq!(executed.av_epoch, plan.epoch, "跳{i}: 执行 epoch=计划");
            std::thread::sleep(std::time::Duration::from_millis(450));
            let obs = adapter.observe(&graph);
            assert_eq!(obs.video_active, obs.audio_active, "跳{i}: 双平面成对");
            assert_eq!(obs.observed_active, Some(target), "跳{i}: observed=target");
            assert!(
                obs.program_video_frames > prev_frames,
                "跳{i}: program 帧持续"
            );
            assert!(group.complete_switch(target), "跳{i}: Observed 落定");
            assert_eq!(
                group.desired,
                SwitchDesired::ActiveInput(target),
                "跳{i}: Desired==target（三态不串之二）"
            );
            prev_epoch = plan.epoch;
            prev_frames = obs.program_video_frames;
            prev_active = target;
        }
        assert_eq!(group.switch_epoch, 4, "四跳后 epoch==4");

        // ── 快速 A→B→A（300ms 间隔）: 状态链在快节奏下仍一致。
        for target in [b, a] {
            let plan = group
                .plan_switch(&SwitchIntent {
                    target,
                    policy: SwitchPolicy::FrameSwitch,
                })
                .expect("快切计划");
            group.begin_switch(&plan).expect("begin");
            adapter.switch(&graph, &plan).expect("快切执行");
            std::thread::sleep(std::time::Duration::from_millis(300));
            let obs = adapter.observe(&graph);
            assert_eq!(obs.video_active, obs.audio_active, "快切: 成对");
            assert_eq!(obs.observed_active, Some(target), "快切: observed=target");
            assert!(group.complete_switch(target));
        }

        // ── 四类 fail-closed（真适配器级纵深 + 组级）: 全部拒收且状态零变。
        let desired_before = group.desired;
        let epoch_before = group.switch_epoch;
        let obs_before = adapter.observe(&graph);
        // 1) invalid target（组外 Uuid）。
        let outsider = Uuid::new_v4();
        assert_eq!(
            group.plan_switch(&SwitchIntent {
                target: outsider,
                policy: SwitchPolicy::FrameSwitch,
            }),
            Err(SwitchError::TargetNotInGroup(outsider)),
            "fail-closed: 组外目标（组级）"
        );
        // 2) duplicate target（当前 active=a）。
        assert_eq!(
            group.plan_switch(&SwitchIntent {
                target: a,
                policy: SwitchPolicy::FrameSwitch,
            }),
            Err(SwitchError::TargetAlreadyActive(a)),
            "fail-closed: 切当前 active（组级）"
        );
        // 3) PACKET/MASTER。
        for policy in [SwitchPolicy::PacketSwitch, SwitchPolicy::MasterSwitch] {
            assert_eq!(
                group.plan_switch(&SwitchIntent { target: b, policy }),
                Err(SwitchError::UnsupportedPolicy(policy)),
                "fail-closed: {policy:?}（组级）"
            );
        }
        // 4) 伪造 plan 打真适配器: 组外/duplicate/错 epoch/非 FRAME——
        //    adapter 纵深重校验（不信任调用方）。
        let forged = |target: Uuid, policy: SwitchPolicy, epoch: u64| SwitchExecutionPlan {
            from: a,
            target,
            policy,
            epoch,
        };
        assert_eq!(
            adapter.switch(
                &graph,
                &forged(outsider, SwitchPolicy::FrameSwitch, epoch_before + 1)
            ),
            Err(SwitchError::TargetNotInGroup(outsider)),
            "fail-closed: 组外目标（适配器纵深）"
        );
        assert_eq!(
            adapter.switch(
                &graph,
                &forged(a, SwitchPolicy::FrameSwitch, epoch_before + 1)
            ),
            Err(SwitchError::TargetAlreadyActive(a)),
            "fail-closed: 切 active（适配器纵深）"
        );
        assert_eq!(
            adapter.switch(
                &graph,
                &forged(b, SwitchPolicy::FrameSwitch, epoch_before + 5)
            ),
            Err(SwitchError::StalePlanEpoch {
                got: epoch_before + 5,
                expected: epoch_before + 1
            }),
            "fail-closed: 错 epoch（适配器纵深）"
        );
        assert_eq!(
            adapter.switch(
                &graph,
                &forged(b, SwitchPolicy::MasterSwitch, epoch_before + 1)
            ),
            Err(SwitchError::UnsupportedPolicy(SwitchPolicy::MasterSwitch)),
            "fail-closed: MASTER（适配器纵深）"
        );
        // 状态零变（Desired/epoch/observed/帧仍推进）。
        assert_eq!(group.desired, desired_before, "拒收后 Desired 零变");
        assert_eq!(group.switch_epoch, epoch_before, "拒收后 epoch 零变");
        std::thread::sleep(std::time::Duration::from_millis(300));
        let obs_after = adapter.observe(&graph);
        assert_eq!(obs_after.observed_active, obs_before.observed_active);
        assert_eq!(obs_after.switch_epoch, obs_before.switch_epoch);
        assert!(
            obs_after.program_video_frames > obs_before.program_video_frames,
            "拒收风暴后 program 帧照常推进"
        );

        // ── ⑧ 真桥级区分性小验收（第十三轮批准并入）: 主动停 Program
        // graph → observed 归零, 而两路输入健康弧仍在推进——
        // "Input healthy"与"Program failed"不混淆。
        let (ia, ib) = (
            crate::pipeline_events::read_health(&h1)
                .unwrap()
                .video_frame_count,
            crate::pipeline_events::read_health(&h2)
                .unwrap()
                .video_frame_count,
        );
        adapter
            .stop_program(&graph)
            .expect("program 停（主动故障注入）");
        let obs_dead = adapter.observe(&graph);
        assert_eq!(obs_dead.observed_active, None, "⑧ Program 停→observed 归零");
        assert_eq!(obs_dead.program_video_frames, 0);
        std::thread::sleep(std::time::Duration::from_millis(500));
        let (ia2, ib2) = (
            crate::pipeline_events::read_health(&h1)
                .unwrap()
                .video_frame_count,
            crate::pipeline_events::read_health(&h2)
                .unwrap()
                .video_frame_count,
        );
        assert!(ia2 > ia && ib2 > ib, "⑧ 输入健康照常推进——两故障面不混淆");

        // teardown。
        for (h, _) in [(h1, a), (h2, b)] {
            let ch = tap.tap_attachments(&h).first().map(|x| x.channel.clone());
            if let Some(ch) = ch {
                tap.detach_media_tap(&h, &ch).expect("摘除");
            }
            assert!(tap.tap_attachments(&h).is_empty());
            let _ = bundle.backend.stop(&h);
        }
    }

    // ── A2-8-02-G/H: Observation & Timeline Evidence（第十四轮四验证面）——
    // ①Bridge pad probe 一等事实 ②三列 Input/Bridge/Program PTS 同时采样
    // ③recover partial degraded 结构化观测 ④failure-domain 区分。
    // **只观测/只取证/绝不修 timestamp 行为**。
    #[cfg(all(
        feature = "bmd-provider",
        feature = "gstreamer-backend",
        not(feature = "mock")
    ))]
    #[test]
    fn switch_graph_rt_01_gh_three_column_observation_evidence() {
        use crate::contracts::media_tap::{MediaTapRequest, TapPlanes};
        use crate::pipeline::PipelinePlan;
        use crate::program_execution::{
            assemble_bridge_health, assemble_timeline_sample, classify_failure_domain, tap_channel,
            FailureDomain, TimelineSample,
        };
        use crate::session::SessionId;

        let bundle =
            crate::registry::AdapterRegistry::build_media_adapter_bundle().expect("bundle");
        let tap = bundle.media_tap.clone().expect("tap view");
        let bridge_port = bundle.bridge_observation.clone().expect("bridge 观测 view");
        let h1 = bundle
            .backend
            .instantiate(&PipelinePlan::self_test())
            .expect("A");
        let h2 = bundle
            .backend
            .instantiate(&PipelinePlan::self_test())
            .expect("B");
        bundle.backend.start(&h1).expect("启动 A");
        bundle.backend.start(&h2).expect("启动 B");
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        for (h, d) in [(h1, a), (h2, b)] {
            tap.attach_media_tap(
                &h,
                &MediaTapRequest {
                    channel: tap_channel(d),
                    planes: TapPlanes::Both,
                },
            )
            .expect("tap attach");
        }
        let mut group = ExecutionGroup::new(
            SessionId(Uuid::new_v4()),
            vec![
                SessionInput {
                    device_id: a,
                    handle: h1,
                },
                SessionInput {
                    device_id: b,
                    handle: h2,
                },
            ],
            a,
        )
        .expect("组");
        let adapter = GStreamerSwitchAdapter::bridged();
        let graph = adapter.build_program_graph(&group).expect("bridged 物化");
        adapter.start_program(&graph).expect("启动");
        std::thread::sleep(std::time::Duration::from_millis(1200));

        // 采样器: 输入健康弧 + 桥 probe 行[tap_channel join] + 程序观测。
        let sample = |dev: Uuid, h: PipelineHandle| -> TimelineSample {
            let bridge_row = bridge_port
                .bridge_observations(&h)
                .into_iter()
                .find(|o| o.channel == tap_channel(dev));
            let input = crate::pipeline_events::read_health(&h);
            let program = adapter.observe(&graph);
            assemble_timeline_sample(dev, input.as_ref(), bridge_row.as_ref(), &program)
        };

        // ② 三列同时采样: 切换前采两设备——六 PTS 列全在场（独立测量）。
        let mut samples: Vec<TimelineSample> = vec![sample(a, h1), sample(b, h2)];
        let plan = group
            .plan_switch(&SwitchIntent {
                target: b,
                policy: SwitchPolicy::FrameSwitch,
            })
            .unwrap();
        group.begin_switch(&plan).unwrap();
        adapter.switch(&graph, &plan).expect("切 B");
        std::thread::sleep(std::time::Duration::from_millis(700));
        samples.push(sample(a, h1));
        samples.push(sample(b, h2));
        for (i, s) in samples.iter().enumerate() {
            assert!(s.input_video_pts.is_some(), "样{i}: 输入 video PTS 在");
            assert!(
                s.bridge_video_pts.is_some(),
                "样{i}: 桥 video PTS 在（probe 实测）"
            );
            assert!(s.program_video_pts.is_some(), "样{i}: program video PTS 在");
            assert!(s.input_audio_pts.is_some() && s.bridge_audio_pts.is_some());
        }
        // 桥列独立推进（probe 帧计数递增——真实数据经过分支, 非复制）。
        let b0 = bridge_port.bridge_observations(&h1);
        std::thread::sleep(std::time::Duration::from_millis(400));
        let b1 = bridge_port.bridge_observations(&h1);
        assert!(
            b1[0].video_frames > b0[0].video_frames,
            "桥 probe 帧计数递增（tap→inter 段真实流通）"
        );
        assert!(b1[0].video_last_pts.is_some());

        // ③ recover partial degraded 结构化观测: recover active B → 簿记
        // 重放 + probe 实测流通 → degraded=false（观测查询组装——不改
        // recover 返回类型）。
        bundle.backend.recover(&h2).expect("recover B");
        std::thread::sleep(std::time::Duration::from_millis(900));
        // G/H-1: liveness 基（观察时钟窗口判定——非帧基历史存在）。
        let liveness_all: Vec<_> = bridge_port
            .bridge_liveness(&h1, 2_000)
            .into_iter()
            .chain(bridge_port.bridge_liveness(&h2, 2_000))
            .collect();
        let report =
            assemble_bridge_health(true, vec![tap_channel(a), tap_channel(b)], &liveness_all);
        assert_eq!(
            report.observed_alive_channels.len(),
            2,
            "双 channel 窗口内实测流通（当前推进）"
        );
        assert!(!report.bridge_degraded, "健康路径不降级");

        // ④ failure-domain 区分（真桥组合观测分类）。
        let input_advancing = || {
            let f = crate::pipeline_events::read_health(&h1)
                .unwrap()
                .video_frame_count;
            std::thread::sleep(std::time::Duration::from_millis(300));
            crate::pipeline_events::read_health(&h1)
                .unwrap()
                .video_frame_count
                > f
        };
        let bridge_alive = |h: &PipelineHandle| {
            // G/H-1: liveness 窗口判定（当前推进——非历史帧存在）。
            bridge_port
                .bridge_liveness(h, 2_000)
                .iter()
                .any(|l| l.alive_in_window)
        };
        let program_advancing = || {
            let f = adapter.observe(&graph).program_video_frames;
            std::thread::sleep(std::time::Duration::from_millis(300));
            adapter.observe(&graph).program_video_frames > f
        };
        assert_eq!(
            classify_failure_domain(input_advancing(), bridge_alive(&h1), program_advancing()),
            FailureDomain::None,
            "健康=无故障域"
        );
        adapter.stop_program(&graph).expect("program 停（注入）");
        assert_eq!(
            classify_failure_domain(input_advancing(), bridge_alive(&h1), false),
            FailureDomain::Program,
            "program 独立故障可分（输入+桥仍流通）"
        );
        let ch_a = tap.tap_attachments(&h1)[0].channel.clone();
        tap.detach_media_tap(&h1, &ch_a).expect("摘 A tap（注入）");
        assert_eq!(
            classify_failure_domain(input_advancing(), bridge_alive(&h1), false),
            FailureDomain::Bridge,
            "桥独立故障可分（输入仍推进, 桥无实测）"
        );

        // teardown。
        let ch_b = tap.tap_attachments(&h2)[0].channel.clone();
        tap.detach_media_tap(&h2, &ch_b).expect("摘 B");
        assert!(tap.tap_attachments(&h1).is_empty() && tap.tap_attachments(&h2).is_empty());
        let _ = bundle.backend.stop(&h1);
        let _ = bundle.backend.stop(&h2);
    }
}
