---
status: ready
packet: RF-FF-02
---
# RF-FF-02 — FFmpeg 单输入 Egress / Output 生命周期

## Context

RF-FF-01A–01F 已完成 backend-neutral RuntimeBinding、FFmpeg 单输入 DeckLink input、Session composition、canonical failure/recovery monitor 与 BMD failure-first acceptance。当前 `PipelinePlan` 已有冻结的 `OutputPlan { Hls, Rtmp }`，GStreamer adapter 已消费该计划；FFmpeg adapter 仍明确拒绝非空 outputs。

本 packet 只补齐同一单输入 FFmpeg backend 的 output lifecycle，不扩展新的媒体拓扑。

## Decision

采用现有 canonical `OutputPlan`，不修改 `GraphRuntimeIntent`、wire vocabulary、Session/Resource/Lease owner 或 `MEDIA_BACKEND_CONTRACT`。FFmpeg adapter 在自己的 command builder 内消费已经物化并授权的 output plan。

## Scope

- 单 input + 至多一个 output；支持现有 `Hls` 与 `Rtmp` 两个封闭词表。
- FFmpeg command argv 由 adapter 自己构造，禁止 shell string、URL/路径注入和 silent fallback。
- HLS 目标使用已校验的本地目录；RTMP 目标使用已校验的 `rtmp://` URL。
- start/observe/recover/stop/drop 对 output child 保持同一 PipelineHandle 生命周期；recover 必须重建同一 input/output plan。
- 保留现有 appsink/纯分析行为：无 output 时命令与 RF-FF-01F 一致。

## Forbidden scope

- Network Source、第二 input、Program Switch、PACKET/MASTER、Composition/Audio、SRS ownership、Recording/Replay。
- 输出 device-number 2、DeckLink output port、GStreamer output path 的改写。
- 新增 wire 字段、修改 CanonicalPipelinePlan 语义、第二套 Session/Resource/Lease truth。
- 用短跑结果关闭 24h `rss_bounded` stability debt。

## Implementation tasks

1. 在 FFmpeg adapter 中增加 output-plan 校验与 backend-owned argv；保持 input binding fail-closed。
2. 为 HLS/RTMP argv、非法 target、多个 output、unknown/empty target、recover/stop/no-orphan 增加 unit tests。
3. 增加一个 acceptance gate：通过现有 production Session composition 建立单输入 HLS output，验证 playlist/segment 与 h264/aac 可探测，随后 SIGTERM child，验证同 handle recovery 与新 child，最后 Session teardown。
4. 保持现有 RF-FF-01F monitor 接线和 canonical event path，不新增 output-specific RuntimeEvent。

## Acceptance

- default/simulation/mock/ffmpeg-backend 测试与 clippy/fmt/diff/architecture/remove-adapters 全绿。
- FFmpeg 无 output 的既有 RF-FF-01F 行为零回归；HLS/RTMP argv 均不经过 shell。
- BMD exact commit：授权 input `46:00000000:002e4500`，HLS output 产生有效 playlist/segments，child failure 后 canonical recovery/new child 成功。
- teardown：Released、Resource Available、Lease NONE、monitor exited、FFmpeg orphan NONE。
- output device-number 2 未触碰；stale `/opt/vbmf-dev/repo` 不使用不修改；CI 7/7。

## Verification order

先 unit/feature tests → VM full matrix → architecture gates → BMD native release build → BMD HLS failure-first gate → evidence/header/SHA256SUMS → STATE/GitHub sync。

## Risks and rollback

FFmpeg 编码器/封装器或 BMD 输入格式可能暴露运行时差异；任何 output acceptance 失败都保持 output path fail-closed，不回退到 appsink 假成功。实现限制在 adapter、gate、tests 和必要的 plan/state 文档内，可按文件回滚，不改变 RF-FF-01F 已验证路径。