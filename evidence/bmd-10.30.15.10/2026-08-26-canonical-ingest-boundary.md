# Canonical Ingest 边界纠偏（Phase 0.6 决策纪要）

- 日期：2026-08-26
- 环境：BMD `10.30.15.10`（lytv）；V0.2 LOCK FINAL / Phase 0.5 LOCK FINAL / Phase 0.6 活跃
- 性质：把已偏离冻结设计的实现拉回正轨（**非**新开架构，无需 V0.3）

## 结论（与 Phase 0.6 / V0.2 一致）

1. **唯一 canonical 媒体采集通道 = GStreamer `decklinkvideosrc` + `decklinkaudiosrc`。**
   - 链路：`DeckLink → GStreamer → RAW(Video+Audio) → Normalize → FRAME/MASTER SWITCH → Program Master RAW → Encode → SRS → RTMP/HLS/WHEP`。
   - Encode 必须位于 Switcher **之后**（delivery boundary），不得提前到切换输入侧。
   - first-frame（MEDIA-RT-01）定义为：**真实 SDI → GStreamer → RAW → first GstBuffer/GstSample**（经 appsink / pad probe / metrics 观测），**不是** `IDeckLinkInputCallback` 收到的第一帧。

2. **`IDeckLinkInput`（Rust FFI）降级为 Discovery / Capability / 诊断探针。**
   - 保留 `decklink::start_capture` 作为 Gate 6/7 的 SDK 能力/诊断探针（真机已验证可行），**不得**作为生产视频数据通道。
   - 否则会与 GStreamer 争夺设备 → 双采 / 设备争用 / 时序混乱。
   - 真机已验证的 `decklink.rs` FFI 工作（枚举、DeviceHandle、模式协商 ABI）是宝贵资产，**不废弃**，转作 Discovery/Lease/诊断层。

3. **代码改动（已落地 `services/media-agent/src`）：**
   - `pipeline.rs`：原 `PipelineController = capture lifecycle` 骨架 → 重写为 **`PipelinePlan`（Source/Video/Audio/Switch/Output）+ `PipelineController::prepare/start/stop/recover` + `materialize(GraphRuntimeIntent)`**。`PipelinePlan` 明确为 `GraphRuntimeIntent` 的**物化执行计划**，不是第二套 Graph Model。
   - `main.rs` Gate 2.6 CAP-01：拆成 (A) SDK 诊断探针（保留 `start_capture`）与 (B) canonical GStreamer 采集计划（`materialize` → `PipelineController` 拥有，launch pending）。
   - `graph_intent.rs`：`SourceIntent.device_number: u32 → Option<u32>`（canonical identity 由 `DeviceIntent.device_id` 承载）。
   - `rpc.rs`：`StartPipeline { spec: PipelineSpec } → { intent: GraphRuntimeIntent }`（控制面传 Intent，Rust 侧物化）。`PipelineSpec` 已移除。

## ⚠️ 对用户主张的事实纠正（必须记录，防回归）

用户原建议表第 3 行："Device Identity: index/handle 混用 → **Persistent ID 为 canonical identity**，且 GStreamer 官方明确支持 `persistent-id` 字段"。

> ### ⚠️ 二次纠偏（2026-08-26 晚）：上一轮我认定"GStreamer 无 persistent-id 属性"是**错的**。
> 经用**当前** GStreamer 官方文档 + Blackmagic SDK 手册交叉核对：
> - GStreamer `decklinkvideosrc` / `decklinkaudiosrc` **确有 `persistent-id` 属性**（`gint64`，对应 `BMDDeckLinkPersistentID`），**优先级高于 `device-number`**，自 **GStreamer 1.22** 起可用。
> - 因此正确设计是 **VBMF `device_id` → Device Registry → BMD `PersistentID` → GStreamer `persistent-id`**；`device-number` 仅作回退/诊断（见 `device.rs` `parse_persistent_id` + `pipeline::materialize`）。
> - **`DeviceHandle` ≠ `PersistentID`**：BMD `DeviceHandle` 格式为 `RevisionID:PersistentID:TopologicalID`（官方手册 3.17），中间段才是 PersistentID。`device.rs` 的 `serial` 字段存的就是 DeviceHandle 字符串，已由 `parse_persistent_id` 提取中段；`DeviceInfo` 同时保存 `bmd_persistent_id` 与 `bmd_device_handle` 两路身份。
> - 代码已落地：`GraphRuntimeIntent.SourceIntent` 只带 `device_id`（不泄露 GStreamer 属性）；`materialize(intent, devices)` 按 `device_id` 在 Device Registry 解析，**找不到直接 `IdentityUnresolved`，绝不 `unwrap_or(0)` 盲开**。

## 性质与治理

- 本次为"实现回归冻结设计"，非新架构；不触发 V0.3。
- 但 `IDeckLinkInput` 角色重定义、canonical-ingest 边界、identity-resolution 要求，建议提升到 **Device Registry Contract / Phase 0.6 验收条目**，作为 MEDIA-RT-01 的正式定义。
- 下一步 CAP 序列（与用户一致）：CAP-01(Device+Lease+GStreamer source) → CAP-02(GStreamer→RAW first-frame, MEDIA-RT-01) → CAP-03(Normalize) → CAP-04(FRAME/MASTER SWITCH) → CAP-05(Encode→SRS) → CAP-06(FI-08 GStreamer crash recover) → CAP-07(FI-09 Agent restart reacquire)。
