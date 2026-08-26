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

## MEDIA-RT-01 严格定义（2026-08-26 晚·用户终审）

**`CI default + simulation` + `BMD --features bmd` + 真机 `GStreamer decklinkvideosrc persistent-id=<BMD PersistentID>` 拿到首个真实 GstBuffer，只是 MEDIA-RT-01 的"最低闭环"。** 单独的 first-frame 不足以证明媒体运行时健康。

### 完整接受清单（>=13 项）
1. CI default PASS
2. CI simulation PASS
3. `BMD --features bmd` PASS
4. Real DeckLink identity resolved
5. Device Lease acquired
6. Canonical GStreamer pipeline starts
7. `decklinkvideosrc` + `decklinkaudiosrc` 同时就绪
8. Real SDI signal detected
9. first real GstBuffer received
10. PTS/timestamp valid **and monotonic**
11. no synthetic/test frame
12. pipeline RUNNING for acceptance window
13. evidence 记录 exact SHA / SDK / driver / GStreamer versions

### 接受判定拆三层（不是 happy path，必须确定性观测）
- **MEDIA-RT-01A — Ingest Open**：DeckLink 解析 / Lease 获取 / GStreamer 启动 / 信号检测。
- **MEDIA-RT-01B — First Frame**：真实 video **+ audio** GstBuffer 各到一帧 + 有效 timestamp（代码 `PipelineHealth.first_frame_ok()` 校验 video+audio 首帧 + PTS 单调 + 无 error）。
- **MEDIA-RT-01C — Short Stability**：N 秒窗口内 frame_count>0 / 无意外 EOS / 无 pipeline error / 无重协商（见 `pipeline.rs::MediaRt01Acceptance`；`a && b && c` 才 `pass`）。

> **MEDIA-RT-01 = A + B + C 全过**，≠ Reference A2。A2 还需 Normalize → FRAME/MASTER_SWITCH → Program Master → Encode → SRS → AV sync → output health + 24h stability。

### 音频不后置
canonical ingest 已明确 `decklinkvideosrc + decklinkaudiosrc`。MEDIA-RT-01 至少记录 `video first buffer / audio first buffer / video PTS / audio PTS`；**最终 AV sync 在 A2/B 的 Program Master / Master Join 阶段做**，不在 MEDIA-RT-01 立即完成。

## 本轮代码改动（pipeline.rs / main.rs / device.rs）

- **字段改名（P1 命名技术债）**：`SourcePlan.persistent_id: CanonicalDeviceId` → **`device_id: CanonicalDeviceId`**。`device_id` 是 VBMF 规范身份（DeviceHandle 派生 UUID），**不是** BMD PersistentID；BMD PersistentID 由 `bmd_persistent_id: u32` 承载，并新增 `selection_mode: SourceSelectionMode`（Canonical / DiagnosticFallback）。
- **硬规则（Phase 0.6 锁死）**：`materialize(intent, devices, MaterializeMode::Production)` 下，若 `device_id` 在 Registry 找不到，或 `bmd_persistent_id == 0`（PersistentID 解析失败），直接 `IdentityUnresolved` 失败——**绝不 `unwrap_or(0)` 盲开 device 0**。只有 `MaterializeMode::Diagnostic` 显式允许 `device-number` 兜底（且须在证据标注）。`device.rs::parse_persistent_id` 注释同步纠正旧"退化为 device-number"措辞。
- **真实 GStreamer launch（feature = `gstreamer`）**：`GStreamerPipelineController` 经 `gst::parse::launch("decklinkvideosrc persistent-id=<BMD PersistentID> ! video/x-raw ! appsink name=videosink   decklinkaudiosrc persistent-id=<...> ! audio/x-raw ! appsink name=audiosink")` 启动，`appsink` 回调抓首帧 + 记 PTS + 校验单调（跨线程健康写 `HEALTH_ARCS`）；`recover` = 停后起重 launch。未启用 feature 时 trait 仍提供骨架（default/simulation/bmd 构建可编译）。`Cargo.toml` 新增 `gstreamer`/`gstreamer-app`（0.23，optional）；真机 canonical 构建 = `--features bmd,gstreamer`。
- **Lease → Pipeline 接线**：`main.rs` 启动前 `lm.is_valid(&device_uuid)` 校验租约（排他不变量前置），`recover` 前再次重校。
- **Supervisor → recover 接线（P1 运行时补齐）**：`main.rs` 把 `sup` 包 `Arc<Mutex>` 与 GStreamer 看门狗共享；看门狗周期巡检 `read_health` + bus 错误 → `sup.report_failure` → `Restart`（重校 lease 后 `ctrl.recover`）/`Escalate`。Supervisor 仅决策、不碰 GStreamer（硬边界）。
- **SDK 探针限 `hardware-test`**：真机 GStreamer 启动后，`decklink::start_capture`（IDeckLinkInput）仅 `hardware-test` 运行，避免与 canonical 路径同时打开同一块 DeckLink。
- **证据记录**：canonical 启动处 `tracing::info!(gst_version = ?gstreamer::version(), ...)` 记录 GStreamer 运行时版本；SHA 由 CI 归档（运行时不适用）。

## 下一步（用户终审定序，不做 FI-08/09）
CI default/simulation → BMD `bmd` → 真机枚举 → Lease → GStreamer canonical ingest → first GstBuffer → **MEDIA-RT-01 PASS** → Normalize → FRAME/MASTER_SWITCH → Program Master → Encode → SRS → A2 → FI-08 → FI-09。当前最大遗留技术债：PipelineController 真实 launch（本轮已开工）、Supervisor/Lease 接线（本轮已补运行时）、Pipeline health 监控（本轮已建最小槽）、Normalize/Switch 执行层、GraphRuntimeIntent↔PipelinePlan 完整物化契约。均与 V0.2 相容，无需重开架构。

## 同步澄清 + 本轮 refinement（`5966777` 之后）
- **基线同步已确认**: `git ls-remote git@github.com:pwl1987/VBMF.git master` = `596677744ba9b65cdd7c7e268329c53926063d11` (即 `5966777`)。GitHub `master` HEAD **就是** `5966777`; raw.githubusercontent.com 显示旧骨架属 CDN 陈旧缓存, 非基线错位。公开 `master` 的 `pipeline.rs` 已无 "Gate 2.1 skeleton / 未真正 launch" 字样 (仅保留过 `PipelineError::NotImplemented` 枚举变体, 本轮已删除)。
- **用户终审反馈 → 已在本轮落地的实现级修正 (P0/P1 子集, 其余为 BMD 验证项)**:
  - **#8/#9 Supervisor 运行时闭环**: 新增 `GStreamerPipelineController::poll_bus` 真实监控 `pipeline.bus()` 的 `Error/EOS/StateChanged`, 写回 `PipelineHealth.last_error` + `acceptance.a3/c1/c2`; 看门狗形成单向链 `GStreamer Bus → PipelineHealth → AgentState → Supervisor → Health API`。
  - **#4 MEDIA-RT-01 收紧**: `MediaRt01Acceptance` 拆 **A1-A4 / B1-B4 / C1-C4** (A1 身份解析 / A2 租约 / A3 PLAYING / A4 信号检测; B1 首视频 / B2 首音频 / B3 有效 PTS / B4 单调; C1 无 EOS / C2 无 error / C3 无重协商 / C4 计数增长), 防"PLAYING 但无信号"误判 A PASS。
  - **#13 硬互斥**: `hardware-test`(IDeckLinkInput SDK 探针) 与 `gstreamer`(canonical 运行时) 在 **编译期** 互斥 (`compile_error!`); SDK 探针实际门控为 `#[cfg(all(feature="hardware-test", not(feature="gstreamer")))]`, 杜绝同卡双采。
  - **#10**: `lm.is_valid` 在 `start` 与 `recover` 前均校验 (MEDIA-03 排他不变量)。
  - **#6/#7 已满足**: `SourcePlan` 最终收敛为 `{device_id, bmd_persistent_id, device_number}`; `materialize` 只读 `DeviceInfo`(含 VBMF UUID / BMD PersistentID / DeviceHandle / Serial / Model) 作桥梁, **不**二次枚举 BMD。
- **仍为 BMD 验证项 (非代码可解, 用户既定"三件套"工作流)**: 真实 `GStreamer launch` 经 CI/BMD Linux 编译 (#2 P0); 真机首 video/audio GstBuffer + PTS + 短稳定 (#3); `gstreamer = 0.23` / `gstreamer-app = 0.23` / appsink builder 具体 API 以 BMD 编译器为准; Normalize/Switch/Encode/SRS/FI-08/09 顺序不变。本机 Windows 无法编译 `bmd,gstreamer` (缺 protoc/clang/SDK/系统 GStreamer), 故 gstreamer feature 代码须真机 `--features bmd,gstreamer` 验证。
