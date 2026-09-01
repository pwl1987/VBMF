# Comet Design Handoff

- Change: prototype-p1a-output-pipeline
- Phase: design
- Mode: compact
- Context hash: 79bd821fd3934d8bac414508a0f585e1fdc307d63d325b5a020dd0d477ba8750

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/prototype-p1a-output-pipeline/proposal.md

- Source: docs/openspec/changes/prototype-p1a-output-pipeline/proposal.md
- Lines: 1-47
- SHA256: 92f01ffe56861bc0d427717b4b5696fc76a681b29c4ff6e5226f10ccbcc24e2c

```md
# Proposal — prototype-p1a-output-pipeline

## Why

VBMF 已完成 0.6→0.7D→V0.3-1 的架构与 Runtime 地基（master=16a8136），但全部媒体管线是**纯分析型**：`controller.rs` 的双 branch 恒终止于 `appsink`（原始帧进 Rust 做信号分析），`GraphRuntimeIntent.sink.kind`（如 `"rtmp"`）只是语义词，后端完全忽略；`PipelinePlan` 连 sink 字段都没有。产品第一次"可见"的链路——真 SDI 输入变成编码后的 HLS/RTMP 输出——尚不存在（用户 2026-09-02 战略裁定：Prototype-1 Vertical Slice 为近期最高优先级，Federation 平行不阻塞）。

本 change 是 Prototype-1 路线的第一段（P1a）：把**「输出意图 → 输出物化」第一次真正打通**，让 `sink.kind` 从描述词变成被消费的契约。

## What Changes

- **SinkPlan 物化**（pipeline.rs）：`PipelinePlan` 增加输出段；`materialize()` 消费 `d.pipeline.sink.kind`（`appsink`=纯分析（现行为，默认）/ `hls` / `rtmp`），结合 `PrototypeOutputConfig` 物化为具体输出计划（编码参数/mux/sink target）。输出 launch 段构造与 `src_props` 同层（domain 侧物化，controller 保持纯执行——用户边界修正：不做硬编码 demo 管线）。
- **tee 双分支管线**（controller.rs）：`src ! raw ! tee` 一路保 `appsink` 分析分支（**零退化**：MEDIA-RT-01/E1-E8 全部照旧），另一路 `queue ! videoconvert ! x264enc tune=zerolatency ! h264parse`（视频）+ `audioconvert ! avenc_aac ! aacparse`（音频）→ `mpegtsmux ! hlssink2`（HLS 落盘）和/或 `flvmux ! rtmpsink`（RTMP 推流）。
- **PrototypeOutputConfig**（config.rs）：demo 层输出配置（env 驱动：码率/音频码率/HLS 目录/RTMP URL/输出开关）。**明确不进 Runtime Contract**——正式配置契约化留产品配置模型阶段（用户裁定）。
- **Runtime 输出可见性**：`SessionRuntimeState` 加法输出摘要（经 `CanonicalRuntimeState.sessions[]` 投影；顶层 8 键契约与 D14 测试锁定不动）。
- **main.rs 诊断接线**：既有主会话已声明 `sink.kind:"rtmp"`；输出目标经 env 注入（无输出 env 时行为与今天完全一致——向后兼容）。
- **真机 Gate P1a-01..06**（盒上，脚本不入库）：真 SDI 输入 → 分析链不退化 → 真实 HLS 分片（.m3u8+.ts）→ 真实 RTMP（ffmpeg listen 接收端持续收到音视频）→ 持续运行（无 EOS/ERROR、PTS 单调、分片持续、无 backpressure 死锁）→ 既有硬件回归零退化。

### Capabilities

（skip_specs:true——契约与验收锚定 Design Doc + 既有架构文档，不建 delta spec；与前例 v03-d14-runtime-snapshot-consistency 一致。）

## Non-Goals（明确不做）

- **P1b 最小 Web 页**（GET / 静态页 + /hls/* 文件服务 + 面板）——独立下一段
- Program Master / Switch / 多输入 / 混合（Alpha 阶段）
- 输出参数正式配置契约化（bitrate/GOP/profile/segment duration 进 Runtime Contract）
- transport.rs 五端点任何改动（A 方案静态文件面属 P1b）
- Federation / Control Plane（继续 BLOCKED）
- 硬件编码（盒上无 nvh264/vah264；x264 软编 1080p25 够用）

## 关键未知项（build 前 probe 收敛）

1. `rtmpsink` vs `rtmp2sink` 实际行为差异（盒上真机 probe 决定，用户已授权）
2. `hlssink2` 属性面（location/target-duration/max-files 支持实况）
3. x264 软编 1080p25 在盒上 CPU 余量（持续运行 gate 验证）
4. `SessionRuntimeState` 现 `Copy` derive 的影响面（加 Vec 字段需去 Copy；编译器全量核查）

## 验收场景（Gate P1a-01..06，用户 2026-09-02 逐条定义）

1. **P1a-01** 真实 BMD SDI 输入
2. **P1a-02** 原有分析链不退化（first-frame / PTS monotonic / signal state / runtime health）
3. **P1a-03** 真实 SDI→H.264+AAC→MPEG-TS/HLS→实际 .m3u8 + .ts 分片
4. **P1a-04** 真实 RTMP：VBMF→FLV→RTMP→ffmpeg listen receiver 持续收到音视频（非仅 pipeline PLAYING）
5. **P1a-05** 持续运行：无 EOS/ERROR、video/audio PTS 单调、分片持续产生、RTMP 持续收流、appsink 不被编码分支阻塞、无明显 backpressure
6. **P1a-06** 既有硬件回归（E1-E8 / MEDIA-RT-01 / 14 步矩阵）零退化

完成定义：**"真实 SDI 已经变成（编码后）可消费的媒体"**，而非"代码编译成功"。

```

## docs/openspec/changes/prototype-p1a-output-pipeline/design.md

- Source: docs/openspec/changes/prototype-p1a-output-pipeline/design.md
- Lines: 1-58
- SHA256: 3910af3aaead38eab25bd3e5fab6341abfd3794aa7fb8f35fbdfe2ad0b0afa05

```md
# Design — prototype-p1a-output-pipeline（高层框架）

> 深度技术设计在 design 阶段 Design Doc（`docs/superpowers/specs/`）细化；本文为高层决策。

## D1 分层：输出物化在 domain，controller 纯执行（用户边界修正）

```
GraphRuntimeIntent.sink.kind ──┐
PrototypeOutputConfig (env) ───┤→ pipeline.rs materialize() → PipelinePlan{source, outputs}
                               │      (输出 launch 段构造: 与 src_props 同层)
                               ▼
controller.rs build_pipeline(): 串接 src_props + tee + 分析分支(appsink 原样) + outputs 段
```

- **不做**：把 `tee ! x264enc ! ...` 硬编码进 `build_pipeline()`（demo 捷径，用户明确否决）。
- `PipelinePlan` 是唯一计划载体：`GstInstance` 已持久存 plan，`recover()` 从存档重建 → **输出分支自动随 Supervisor 恢复重建**，不需要突破 Supervisor 边界（probe 实证 controller.rs `instances: HashMap<Handle, GstInstance{plan,..}>`）。

## D2 sink.kind 词表（本 change 起成为被消费的契约词）

| kind | 物化结果 | 语义 |
|------|----------|------|
| `appsink` | 无输出分支（现行为） | 纯分析（默认；E5 过期测试路径保持） |
| `hls` | tee 编码分支 → mpegtsmux → hlssink2 落盘 | HLS 产出 |
| `rtmp` | tee 编码分支 → flvmux → rtmpsink/rtmp2sink 推流 | RTMP 推流 |

未知 kind：生产 fail-closed（沿用 materialize 既有拒绝风格），Diagnostic 拒绝（不静默回退 appsink——词表第一次有牙齿）。

## D3 tee 双分支——分析零退化红线

```
decklinkvideosrc ! video/x-raw ! tee name=v
  v. ! queue ! appsink name=videosink async=false        ← 原样（分析）
  v. ! queue ! videoconvert ! x264enc tune=zerolatency ! h264parse ! ┐
decklinkaudiosrc ! audio/x-raw ! tee name=a                             │ mux
  a. ! queue ! appsink name=audiosink async=false        ← 原样（分析）  │
  a. ! queue ! audioconvert ! avenc_aac ! aacparse ! ───────────────────┘
```

- 分析分支 element 串**逐字符不动**（`async=false` 等细节承载真机实证语义）。
- 编码分支各配独立 `queue`（解耦背压——P1a-05 的 appsink 不阻塞断言由此结构性保证）。

## D4 PrototypeOutputConfig——demo 层，不进 Runtime Contract

env 驱动（`VBMF_OUTPUT_*`）：enable / kind 细化 / video bitrate(默认 6Mbps) / audio bitrate(128k) / HLS 目录 / RTMP URL。无输出 env ⇒ 物化为无输出分支（行为与今天逐字节一致——向后兼容承诺）。参数正式契约化显式留后续（proposal Non-Goals）。

## D5 Runtime 可见性——加法且不碰 8 键契约

`SessionRuntimeState`（sessions[] 内投影）加法输出摘要（如 `outputs: Vec<String>` 物化 kind 列表）；`CanonicalRuntimeState` **顶层不加键**（D14 八键测试锁定不动）。Copy derive 影响面由编译器全量核查后处理。

## D6 默认参数（用户裁定，probe 可微调实现细节不语义）

1080p25 / x264 `tune=zerolatency` / ~6Mbps / AAC 128k / `hlssink2`；RTMP 用 `rtmpsink` 或 `rtmp2sink` 由盒上真机 probe 决定（用户已授权 build 前 probe）。

## D7 验证与 Gate

- Unit/Simulation：物化单测（kind→输出段/未知 kind 拒绝/无 env 向后兼容）+ mock 全绿不回退
- Hardware（盒上，脚本不入库）：Gate P1a-01..06（proposal 逐条）；RTMP 接收端=盒内 ffmpeg `-listen 1`（自包含）
- 回归：14 步矩阵 + E1-E8 + MEDIA-RT-01 零退化

```

## docs/openspec/changes/prototype-p1a-output-pipeline/tasks.md

- Source: docs/openspec/changes/prototype-p1a-output-pipeline/tasks.md
- Lines: 1-28
- SHA256: 2f94f9d6ff0ea2baf64ed53a5cfbcb08a0cb5e41d9b2b2e527d0d99e1c61e3be

```md
# Tasks — prototype-p1a-output-pipeline

> 四栏纪律：每项标注 `Contract` / `Implementation` / `Verification` / `Gate` 状态。

## 1. 输出物化（domain 层）

- [ ] 1.1 `pipeline.rs`：`SinkPlan`/输出段类型 + `materialize()` 消费 `sink.kind`（appsink/hls/rtmp 词表；未知 kind 拒绝） + `PrototypeOutputConfig` 注入点（无输出 env ⇒ 无输出分支, 向后兼容） `Contract: design D1/D2/D4` | `Implementation: 待` | `Verification: Unit——kind→物化/未知拒绝/无 env 兼容三测试` | `Gate: 无`
- [ ] 1.2 `config.rs`：`PrototypeOutputConfig`（env `VBMF_OUTPUT_*`, 默认 1080p25/x264 zerolatency 6Mbps/AAC 128k）——显式不进 Runtime Contract `Contract: design D4 / 用户裁定` | `Implementation: 待` | `Verification: Unit——默认值/env 解析` | `Gate: 无`

## 2. 管线执行（adapter 层, 纯执行）

- [ ] 2.1 `controller.rs`：build_pipeline 串接 `src_props + tee + 分析分支(逐字符不动) + outputs 段`（输出段由 plan 提供, controller 不硬编码 element 名） `Contract: design D1/D3` | `Implementation: 待` | `Verification: Simulation——mock 计划驱动; 真机属 Gate` | `Gate: 无`
- [ ] 2.2 盒上 build 前 probe：`rtmpsink` vs `rtmp2sink` 行为 + `hlssink2` 属性面（location/target-duration/max-files）实证并冻结选择 `Contract: design D6（用户授权 probe 决定）` | `Implementation: 待` | `Verification: probe 输出记入 Design Doc` | `Gate: 无`

## 3. Runtime 可见性 + 接线

- [ ] 3.1 `runtime_state.rs`：`SessionRuntimeState` 加法输出摘要; 顶层 8 键契约与 D14 测试不动 `Contract: design D5` | `Implementation: 待` | `Verification: Unit——sessions[] 投影含输出摘要; 8 键测试原样绿` | `Gate: 无`
- [ ] 3.2 `main.rs`：诊断主会话输出目标 env 注入（无 env 行为不变）; E5 路径保持 appsink `Contract: proposal / probe 实证` | `Implementation: 待` | `Verification: 既有诊断输出 diff 仅增量` | `Gate: 无`

## 4. 三层测试

- [ ] 4.1 Unit/Simulation 全绿（基线 mock 221 零回退 + 新增） `Contract: D14 后基线` | `Implementation: 待` | `Verification: cargo test --features mock (盒)` | `Gate: 无`
- [ ] 4.2 Hardware Gate P1a-01..06（盒上真机; 脚本不入库; RTMP 接收端=ffmpeg -listen） `Contract: proposal 验收场景` | `Implementation: 待` | `Verification: 盒上实跑全 PASS, 输出入 verify 报告` | `Gate: P1a-01..06`
- [ ] 4.3 既有硬件回归零退化（E1-E8 / MEDIA-RT-01 / 14 步矩阵） `Contract: 验收口径` | `Implementation: 待` | `Verification: 盒上矩阵全 exit 0` | `Gate: BOX`

## 5. 交付

- [ ] 5.1 CI 七 required checks 实查全绿 → verify 报告 → archive → PR → merge → 删分支 `Contract: 项目交付纪律` | `Implementation: 待` | `Verification: gh pr checks 实查; PR merged` | `Gate: CI/RELEASE`

```
