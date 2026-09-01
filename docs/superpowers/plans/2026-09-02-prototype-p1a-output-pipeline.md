---
archived-with: 2026-09-02-prototype-p1a-output-pipeline
status: final
---
# 执行计划 — prototype-p1a-output-pipeline

---
base-ref: 16a8136fcdeb438ac99aa6dc359208ebb30ad1bc
comet_change: prototype-p1a-output-pipeline
design_doc: docs/superpowers/specs/2026-09-02-prototype-p1a-output-pipeline-design.md
---

> TDD 硬律：每个任务先失败测试后实现。cargo 一律经盒（tar+scp 同步 `~/media-agent-build`，`source ~/.cargo/env`）；本地 Windows 无 Rust。
> 基线：mock 221 / default 155 / simulation 155 / bmd,gstreamer 155（D14 后）。基线命令 `cargo test --features mock`。

## Task 0 基线核验

- [x] **Step 1** `git rev-parse HEAD` = `16a8136fcdeb438ac99aa6dc359208ebb30ad1bc`；工作树仅含本 change 产物
- [x] **Step 2** 盒同步基线树 + `cargo test --features mock` = 221 passed / `cargo fmt --all -- --check` = FMT_CLEAN

## Task 1 PrototypeOutputConfig（config.rs）

- [x] **Step 1 RED** 测试：默认值（无 env ⇒ kind 覆盖 None/HLS_DIR None/RTMP_URL None/6000/128000）+ env 全设解析
- [x] **Step 2 GREEN** `PrototypeOutputConfig{sink_kind_override, hls_dir, rtmp_url, video_bitrate_kbps, audio_bitrate_bps}` + `from_env()`（VBMF_OUTPUT_KIND/VBMF_OUTPUT_HLS_DIR/VBMF_OUTPUT_RTMP_URL/VBMF_OUTPUT_V_BITRATE_KBPS/VBMF_OUTPUT_A_BITRATE_BPS）；demo 层注释（不进 Runtime Contract）
- [x] **Step 3** 全绿 + fmt

## Task 2 OutputKind/OutputPlan + materialize 消费 sink.kind（pipeline.rs）

- [x] **Step 1 RED** 测试（materialize_with_output 纯变体）：
  - kind="hls" + hls_dir 设 ⇒ outputs==[OutputPlan{Hls,6000,128000,dir}]
  - kind="rtmp" + rtmp_url 设 ⇒ outputs==[OutputPlan{Rtmp,…,url}]
  - kind="rtmp" 目标缺失 ⇒ outputs 空（fail-soft）
  - kind="appsink" ⇒ outputs 空（现行为）
  - kind="bogus" ⇒ Err（fail-closed，生产/诊断一致）
  - bitrate env 覆盖透传
- [x] **Step 2 GREEN** 类型 + `materialize_with_output(..., cfg)` 全逻辑 + `materialize(...)` = from_env 委托（签名零改动）；`PipelinePlan.outputs` 加法（构造点默认空 vec）
- [x] **Step 3** 既有 pipeline 测试零回退 + fmt

## Task 3 output_launch 段构造（pipeline.rs）

> probe 终裁修订（Design Doc §3.4, 实现据此）: 编码器 = **openh264enc**（非 x264enc——本机
> x264enc 与格式 caps 约束互斥死锁且无 caps 时输出 4:4:4 浏览器不可播）; HLS = hlssink2
> **命名 request pad**（out.video/out.audio, 内部自带 mux, 无外置 mpegtsmux）。

- [x] **Step 1 RED** 测试：空 outputs ⇒ ""；HLS 串含 tee name=v/a、**分析分支逐字符**（`v. ! queue ! appsink name=videosink async=false`/`a. ! queue ! appsink name=audiosink async=false`）、openh264enc bitrate={kbps*1000}（单位 bps）、h264parse、avenc_aac bitrate=128000、aacparse、out.video/out.audio、hlssink2 playlist-location/target-duration=2/max-files=10/playlist-length=5；RTMP 串 flvmux streamable=true + rtmpsink location={url} sync=false；bitrate env 值注入
- [x] **Step 2 GREEN** `PipelinePlan::output_launch()`（构造细节 = Design Doc §3.3 串）
- [x] **Step 3** 词表快照测试（appsink/hls/rtmp 三词 + 未知拒绝）+ fmt

## Task 4 controller 串接（adapters/gstreamer/controller.rs）

- [x] **Step 1** `build_pipeline()`：plan.outputs 空 ⇒ launch=今日串（**逐字节**）；非空 ⇒ `{vsrc} ! ` + plan.output_launch() 拼接（controller 零 element 名）
- [x] **Step 2** Simulation：mock 矩阵全绿（gstreamer-backend 构建编译过：`cargo build --features bmd,gstreamer` 盒）
- [x] **Step 3** 真机 smoke（盒，属 gate 前置）：kind=hls 诊断跑 30s，断言 index.m3u8 + seg*.ts 增长（此步即 P1a-03 首证）

## Task 5 运行时可见性（session.rs + runtime_state.rs）

- [x] **Step 1 RED** `SessionRuntimeState.outputs` 投影测试（assemble 侧）；去 Copy derive 波及面编译核查
- [x] **Step 2 GREEN** `MediaSession.outputs` 加法（create 初始化空/start 从 plans 回填 kind 字符串）+ SessionRuntimeState 投影；**顶层 8 键测试不动原样绿**
- [x] **Step 3** 全绿 + fmt

## Task 6 main.rs 诊断接线

- [x] **Step 1** sink kind 来源 `VBMF_OUTPUT_KIND`（默认 "rtmp" 同今天）；无 env 路径行为不变（diff 审）
- [x] **Step 2** 盒上无 env 诊断 smoke = 既有行为（输出 diff 仅增量）

## Task 7 Hardware Gate P1a-01..06（盒上 ~/p1a_gate.sh，不入库）

- [x] **Step 1** 既有回归先行：14 步矩阵 + SESSION_LIFECYCLE(E1-E8) + VBMF_LOOPBACK + transport gate 16 探针 全 PASS（零退化）
- [x] **Step 2** P1a-01/02：真 SDI 诊断 + 分析链断言（first-frame/PTS/health 照旧）
- [x] **Step 3** P1a-03：kind=hls ≥60s；index.m3u8 存在、seg 计数持续增、ffprobe 双流（h264+aac）、期间 appsink 帧持续
- [x] **Step 4** P1a-04：ffmpeg listener 先起 → kind=rtmp ≥60s → 收流文件增长 + ffprobe v+a
- [x] **Step 5** P1a-05 汇总断言：无 EOS/ERROR、PTS 单调、分片/收流持续、appsink 不停滞
- [x] **Step 6** 全 gate 输出落 verify 报告证据

## Task 8 交付

- [x] **Step 1** 全 CI 等价矩阵（盒 14 步 + mock ≥ 221+新增）+ review gate（standard 一次全 change）
- [x] **Step 2** 单代码 commit（代码文件集）+ 产物 commit + build guard record-check
- [x] **Step 3** verify 报告 → verify guard → archive → PR → CI 七 checks 实查 → merge → 删分支
