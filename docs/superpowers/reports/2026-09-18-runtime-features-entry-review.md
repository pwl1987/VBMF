# Runtime Features Entry Review

日期：2026-09-18
Baseline：`fb8bc81c737bfb668342181e62d0cf903e29c941`

## 1. Entry condition

RUNTIME-HARDEN 的立即清偿链 RH-BUS-01/02、RH-LC-01、RH-RES-01A/B 已完成。D11+D13、D15、durable idempotency 继续按已裁决的 defer-until-touch gate，不提前清空。

## 2. Live implementation reconciliation

旧 `ARCHITECTURE_V0.2.md` 的 Source Adapter 表仍把 SRT/RTMP/HLS/WebRTC/RTP/UDP/RTSP 等写为“已实现”。该状态描述不能覆盖 live code / STATE：

- live `SourcePlan` 只支持 DeckLink runtime selection 与 SelfTest；
- `GraphRuntimeIntent::SourceIntent` 仍是 device-bound `device_id/port_id` 形状；
- live adapter 目录只有 Blackmagic、GStreamer、Mock/SwitchMock，无 network-source adapter；
- 因此 Network Sources 仍是 implementation BACKLOG。旧表仅保留为 V0.2 目标/Contract 语义，不作为完成证据。

## 3. Candidate dependency review

- Network Sources：先要解决 source intent/runtime binding 的 backend/protocol-neutral 边界，不能直接塞 SRT URL 进现有 device-bound plan。
- PACKET_SWITCH：执行层显式拒绝；且需要 COMPRESSED source 域，当前 ingest 不具备。
- MASTER_SWITCH：需要 Normalize execution，且下一触碰会触发 RH-CLOCK-01（D11/D13）。
- Full Hot-Standby：依赖可用 source candidates + switch/recovery。
- Recording/Replay：需要新 I/O lifecycle 与 output/source 语义。
- Composition/Audio execution：进入多 flow 后必须先触发 RH-FLOW-01（D15）。
- SRS/Output：当前 HLS/RTMP 是 GStreamer direct output；生产 SRS ownership/target model 仍需独立 packet。
- Live FFmpeg：冻结 `MEDIA_BACKEND_CONTRACT §4` 已明确是合法 Backend 替换轴，最适合成为首个 feature slice。

## 4. FFmpeg prerequisite conflict

冻结契约要求：

> GStreamer → FFmpeg 只替换 Backend + RuntimeBinding，不改变 CanonicalPipelinePlan。

但 live `PipelinePlan::SourcePlan` 仍包含 GStreamer 专属 `device_number`、`SourceSelectionMode` 等运行时地址。直接新增 FFmpegBackend 会迫使 FFmpeg 解释 GStreamer binding，违反 frozen Backend contract。

该问题属于**实现未对齐既有 frozen Contract**，不是新 Contract 设计。因此首包应先修实现边界，不修改 wire/GraphRuntimeIntent。

## 5. Decision

**RF-FF-01A — Backend-neutral RuntimeBinding extraction（FFmpeg prerequisite）** 进入 READY。

Acceptance：

1. inventory 所有 PipelinePlan/GStreamer binding 泄漏和消费者；
2. canonical plan 与 backend-specific runtime binding 分层，禁止第二套 Runtime truth；
3. GraphRuntimeIntent / command / wire vocabulary 不变；
4. GStreamer 现有 DeckLink selection 行为逐字义保持；
5. default/mock + GStreamer regression；
6. 因真实 GStreamer input binding 被触碰，必须 BMD exact-commit 双输入 smoke；
7. CI 7/7 + STATE/GitHub sync。

Next：**RF-FF-01B — FFmpegBackend process lifecycle / SelfTest**，仅在 01A 收口后进入 READY。

## 6. Deferred gates

- RH-CLOCK-01：Clock/Timecode 或 MASTER_SWITCH/Normalize 下一触碰前执行。
- RH-FLOW-01：multi-flow Audio/Metadata 下一触碰前执行。
- RH-IDEM-01：persistent external control-plane 前执行。
