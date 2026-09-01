---
comet_change: prototype-p1a-output-pipeline
role: technical-design
canonical_spec: openspec
archived-with: 2026-09-02-prototype-p1a-output-pipeline
status: final
---

# Design Doc — prototype-p1a-output-pipeline（Prototype-1 P1a：输出意图→输出物化）

基线：master `16a8136`（V0.3-1 D14 已合并）。用户裁定链：P1a 编码输出管线 → P1b 最小 Web 页 → Prototype-1 真机验收；Federation 继续 BLOCKED。

## 1. 目标与非目标

**目标**：打通第一个真闭环——真 SDI → DeckLink → GStreamer → H.264/AAC 编码 → MPEG-TS/HLS 落盘 + FLV/RTMP 推流；`sink.kind` 从被忽略的描述词变成被消费的契约词。完成定义（用户原话）：**"真实 SDI 已经变成（编码后）可消费的媒体"**，而非"代码编译成功"。

**非目标**：P1b Web 页（GET / 静态面+面板，A 方案属 P1b）；Program Master/Switch/多输入；输出参数正式配置契约化；transport.rs 五端点任何改动；Federation/Control Plane；硬件编码。

## 2. 现状锚点（probe 实证 @16a8136）

- `controller.rs build_pipeline()`：launch 恒 `{vsrc} ! video/x-raw ! appsink name=videosink async=false {asrc} ! audio/x-raw ! appsink name=audiosink async=false`——纯分析，无编码/输出。
- `PipelinePlan{source, normalize, switch_mode}` 无 sink；`materialize()` 忽略 `d.pipeline.sink`。
- `GstInstance{pipeline, plan, bus_rx, stop_flag, thread}` 持久存 plan；`recover()` 从存档 `build_pipeline(&plan)` 重建——**输出进 plan 即自动随恢复重建，Supervisor 边界零突破**。
- main.rs 诊断主会话 intent 已声明 `sink.kind:"rtmp"`（今天被忽略）；E5 过期测试 intent 用 `"appsink"`。
- `SessionRuntimeState` derive 含 `Copy`（runtime_state.rs L79）；`CanonicalRuntimeState` 顶层 8 键由 D14 测试锁定。
- 盒上 GStreamer 1.28.2：`x264enc/flvmux/rtmpsink/rtmp2sink/hlssink/hlssink2/mpegtsmux/h264parse/aacparse/avenc_aac` 全 HAVE；ffmpeg 7 在位（可 `-listen 1` 作 RTMP 接收端）。属性实证：`rtmpsink.location`（String URL，面最简）；`hlssink2.{playlist-location,location,target-duration,max-files,playlist-length,send-keyframe-requests}` 全在位；`x264enc.bitrate` 单位 **kbit/s**，有 `tune/speed-preset/key-int-max`；`flvmux.streamable`。

## 3. 类型与物化设计（domain 层，pipeline.rs）

### 3.1 新类型

```rust
/// P1a: 物化后的输出计划（domain 持有; controller 纯执行）。
/// 词表封闭: Hls | Rtmp（appsink = 纯分析 = 无输出段, 不入此 enum）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputKind { Hls, Rtmp }

pub struct OutputPlan {
    pub kind: OutputKind,
    pub video_bitrate_kbps: u32,   // 默认 6000
    pub audio_bitrate_bps: u32,    // 默认 128000
    pub target: String,            // Hls: 分片目录(绝对); Rtmp: rtmp:// URL
}
```

`PipelinePlan` 加法 `pub outputs: Vec<OutputPlan>`（空 = 今日行为，逐字节等价）。**单会话单输出**：`SinkIntent{kind}` 是单字符串，materialize 至多产 1 项；多输出（HLS+RTMP 并行）留 Alpha。

### 3.2 materialize 规则（词表第一次有牙齿）

```
kind ∈ {hls, rtmp} 且对应 target env 在位  → 物化 OutputPlan
kind ∈ {hls, rtmp} 但 target env 缺失      → 无输出段（fail-soft: 部署态非契约违约, log warn）
kind == "appsink"                          → 无输出段（纯分析, 默认, 现行为）
其他未知 kind                               → PipelineError（fail-closed: 契约违约, 生产/诊断一致拒绝, 不静默回退）
```

签名零改动：`materialize(intent, devices, mode, bindings, registry)` 内部读 `PrototypeOutputConfig::from_env()`；新增纯变体 `pub fn materialize_with_output(..., cfg: &PrototypeOutputConfig)` 承载全部逻辑（可测性）；`materialize` = env 读取 + 委托。**env-in-materialize 是显式 demo 层缝**（正式配置模型阶段收口，Design 记录在案）。

### 3.3 输出 launch 段构造（与 src_props 同层）

`PipelinePlan::output_launch(&self) -> String`（空 outputs ⇒ `""` ⇒ controller 走今日串）。产物形如（HLS 例；**接线为盒上 probe 终裁形态**，见 §3.4 裁定）：

```
video/x-raw ! tee name=v
  v. ! queue ! appsink name=videosink async=false                      ← 分析分支逐字符不动
  v. ! queue ! videoconvert ! openh264enc bitrate={vb*1000} ! h264parse ! out.video
audio/x-raw ! tee name=a
  a. ! queue ! appsink name=audiosink async=false                      ← 分析分支逐字符不动
  a. ! queue ! audioconvert ! avenc_aac bitrate={ab} ! aacparse ! out.audio
hlssink2 name=out playlist-location={dir}/index.m3u8
        location={dir}/seg%05d.ts target-duration=2 max-files=10 playlist-length=5
```

RTMP 变体：编码支同上但接 `mux.video`/`mux.audio`，尾段=`flvmux name=mux streamable=true ! rtmpsink location="{url}" sync=false`。

### 3.4 真机 probe 裁定（2026-09-02, 自持 1080p25 环回信号交替复验）

- **编码器 = `openh264enc`**（bitrate 单位 bps, config kbps ×1000 注入）：零 caps 稳定（分片产出 3/3 对照 0/2）+ 输出 **Constrained Baseline yuv420p = 4:2:0 浏览器最大兼容**。
- **`x264enc` 否决**：本机栈与任何格式约束互斥死锁——下游 capsfilter（I420/profile）或 `option-string` 均触发 `decklinkvideosrc` 运行中重协商 → `Internal data stream error`；零 caps 时虽稳定但输出 High 4:4:4 Predictive（MSE/Safari 不解码, 浏览器不可播）。
- **`hlssink2` 命名 request pad**（`out.video`/`out.audio`）：内部自带 mux，外置 `mpegtsmux` 反而 `could not link` 失败（盒上实证）。
- **RTMP = `rtmpsink`**：属性面最简（单一 `location`）；盒内 `ffmpeg -listen 1` 接收端 E2E 实证收流（h264+aac FLV）。
- **环境事实记档**：盒上 SDI-IN-1 外部信号源分钟级抖动且格式切换（1080p25 ↔ 720x486）；gate 断言分辨率无关（帧在流 + h264/aac 即成立）；必要时自持环回发生器 `decklinkvideosink device-number=2 mode=1080p25`。

controller `build_pipeline()` 改为：`src_props(plan)` 产 `{vsrc} !`/`{asrc} !` 前缀 + 无输出 ⇒ 现串；有输出 ⇒ plan.output_launch() 提供 tee 及其后全部段（**controller 不出现 x264enc/hlssink2 等名字**——纯拼接执行）。capsfilter 位置：今日 `video/x-raw` 紧跟 src；tee 版将其并入选段（`{vsrc} ! ` + `video/x-raw ! tee name=v ...`），分析分支语义不变。

## 4. PrototypeOutputConfig（config.rs，demo 层不进 Runtime Contract）

```
VBMF_OUTPUT_KIND   : 覆盖诊断 intent sink kind（默认不设 ⇒ intent 原值; main.rs 诊断接线用）
VBMF_OUTPUT_HLS_DIR: HLS 分片目录（绝对路径; kind=hls 时必需）
VBMF_OUTPUT_RTMP_URL: rtmp://…（kind=rtmp 时必需）
VBMF_OUTPUT_V_BITRATE_KBPS : 默认 6000
VBMF_OUTPUT_A_BITRATE_BPS  : 默认 128000
```

`PrototypeOutputConfig::from_env()` 纯解析+默认值；**参数正式契约化（GOP/profile/segment-duration 等）显式留产品配置模型阶段**（用户裁定）。

## 5. 运行时可见性（加法，8 键不动）

- `MediaSession` 加法 `outputs: Vec<String>`（物化 kind 列表；默认空）——start() 从 plans 回填；构造点（create）初始化空。
- `SessionRuntimeState` 加法 `outputs: Vec<String>` 投影；**去 `Copy` derive**（Vec 非 Copy；该结构每次 assemble 重新投影，Copy 非承重——编译器全量核查）。
- `CanonicalRuntimeState` 顶层**不加键**（D14 八键测试原样绿）。

## 6. main.rs 诊断接线

- intent sink kind 来源：`VBMF_OUTPUT_KIND` 覆盖（默认 `"rtmp"` 同今天）。
- 无任何 `VBMF_OUTPUT_*` ⇒ 行为与今天**逐字节一致**（向后兼容承诺；既有 E1-E8/transport gate 全部照旧可跑）。

## 7. 测试策略（三层）

- **Unit**：materialize 三态（物化/缺 env 降级/未知 kind 拒绝）；output_launch 串生成（含默认参数注入）；config 默认值；SessionRuntimeState 投影。基线 mock 221 零回退。
- **Simulation**：mock 计划驱动（MockBackend 不触 gst）；既有 session 全链测试零回退。
- **Hardware（盒上，脚本 `~/p1a_gate.sh` 不入库）**——Gate 映射：
  - P1a-01 真实 BMD SDI 输入（MEDIA_AGENT_DEVICE_BINDING=loopback-manifest-v2.json, 同 E1-E8 既有真机输入）
  - P1a-02 分析链不退化：gate 输出含 first-frame/PTS 单调/health（复用既有 gate 断言源）
  - P1a-03 HLS：跑 kind=hls，断言 `{dir}/index.m3u8` 存在且 `seg*.ts` 持续增多 + `ffprobe` 验流（h264+aac、duration 增长）
  - P1a-04 RTMP：先起 `ffmpeg -listen 1 -i rtmp://127.0.0.1:1935/live/p1a -c copy -f mpegts recv.ts`（或 null mux），再跑 kind=rtmp，断言接收文件持续增长 + ffprobe 双流
  - P1a-05 持续运行 ≥60s：无 EOS/ERROR（log 断言）、video/audio PTS 单调（gate 输出断言既有源）、分片持续产生、RTMP 持续收流、appsink 分析不因编码分支停滞（期间帧计数持续增长）
  - P1a-06 既有硬件回归：SESSION_LIFECYCLE gate（E1-E8）+ VBMF_LOOPBACK + transport gate 16 探针 + 14 步矩阵全 exit 0

## 8. 风险与缓解

| 风险 | 缓解 |
|------|------|
| x264 软编 CPU 不足 → 掉帧/背压 | speed-preset=veryfast + zerolatency；P1a-05 持续断言；queue 解耦 |
| 编码分支反压拖慢采集 | 编码支独立 queue；appsink 支 async=false 原样；P1a-05 appsink 帧持续断言 |
| rtmpsink 连不上阻塞 PLAYING | ffmpeg listener **先起**再启动 agent（gate 顺序固化）；PLAYING 后收流断言 |
| SessionRuntimeState 去 Copy 波及面 | 编译器全量核查（结构性风险低：每次重投影） |
| env-in-materialize 隐藏输入 | 显式 demo 层缝记录 §3.2；正式配置模型阶段收口 |
| recover 丢输出 | plan 持久（probe 实证），recover 全量重建 |

## 9. 契约冻结点

- sink.kind 词表本 change 起为**被消费契约词**：appsink/hls/rtmp；未知值拒绝。词表快照测试锁定。
- 分析分支 launch 段**逐字符不动**（红线，测试锁定）。
- `CanonicalRuntimeState` 顶层键集 8 键不动（D14 锁定延续）。
- transport.rs / 五端点 / Federation 零触碰。
