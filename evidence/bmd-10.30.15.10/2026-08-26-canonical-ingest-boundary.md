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

## BMD 真机验证 (commit ee26557, 2026-08-26)
- **CI**: `Rust media-agent build + test` (default+simulation) 绿; 新增 `build-gstreamer` job 经 GitHub secret 注入 DeckLink SDK 头 + 装 GStreamer dev, `cargo build --features bmd,gstreamer` 绿, 产物 `media-agent-gstreamer-linux` 上传; `ffi-contract` (hardware-test) 绿。临时编译错误已修: HEALTH_ARCS 改 LazyLock / PipelineHandle 加 Hash / main.rs 补 `use pipeline::PipelineController` / `AgentState::Failed`→`ManualRequired` / 漏 `use lease::LeaseManager`(E0599 acquire)。
- **下发**: 本机 Windows 不能编 → `gh run download` 产物 → `scp` 到 BMD `~/media-agent-gstreamer` (ee26557)。
- **BMD 运行结果 (MEDIA_AGENT_MODE=diagnostic)**: device discovery=3 / lease×3 / SDK libDeckLinkAPI.so 可达 / supervisor watched=3 / **GStreamer runtime version (evidence) gst_version=(1,28,2,0)** / **canonical GStreamer pipeline 启动 (decklinkvideosrc+decklinkaudiosrc, diagnostic 走 device-number=0) PLAYING, 无 bus error** / health :8080 监听。
- **MEDIA-RT-01 进度**: **A1(identity, diagnostic device-number)/A2(lease)/A3(PLAYING) 已达**; **A4(信号)/B(首帧)/C(稳定) 被硬件门控** —— `gst-launch-1.0 decklinkvideosrc device-number={0,1,2}` 三块卡均 **0 buffer**, 即 BMD 当前**无任何 DeckLink 设备接入 SDI 信号**。无信号 → 无首帧, 符合硬规则 (不伪造首帧)。
- **关键发现 (P1 #11 来源)**: 该 DeckLink SDI 经 `IDeckLinkProfileAttributes::GetString(DeviceHandle)` 取到的 serial 为 `n/a`, 且示例 `46:00000000:002e4500` 中 PersistentID 字段本身为 0 → `bmd_persistent_id=0` → Production 物化按硬规则 `IdentityUnresolved` 正确拒绝。即此硬件经 DeviceHandle 拿不到非零 BMD PersistentID。已加 `MEDIA_AGENT_MODE=diagnostic` 显式回退 device-number (非静默) 用于验证; 真 persistent-id 提取需正确的 SDK 调用 (待办)。
- **操作坑 (必记)**: 启动命令里写 `pkill -f media-agent-gstreamer` 会因该字符串也出现在 SSH 自身命令行而**自杀式匹配并杀掉自己的 shell**, 导致二进制根本没启动、SSH 输出全空。正确做法: 不 pkill, 或 `pkill -x`/按 PID; 后台启动用 `nohup ... </dev/null & disown` 或 `setsid`。
- **下一步 (待用户决策)**: ① 接上 SDI 信号源后重跑 → 真正 MEDIA-RT-01 PASS; 或 ② 加 `videotestsrc` 自测模式演示 首帧→PTS→MEDIA-RT-01 全过 (验证媒体运行时链路, DeckLink 信号仍待接); 或 ③ 先记录证据, 等信号再跑。Normalize/Switch/Encode/SRS/FI-08/09 顺序不变。

## BMD 真机验证（二次上机, 本回合提交, 2026-08-26）

> **【现场订正 — 推翻本纪要第 29-34 行"二次纠偏"与第 98 行"无信号"结论】** 以下为 BMD `10.30.15.10` 真机实测事实, 优先级高于文档假设。

- **用户主张复核 ("应该有 SDI 信号") → 基本成立**: device 0 (`dv0`, 两块 SDI 之一) **确有 SDI 信号**, 但信号在 `Signal lost` ↔ `Input source detected` 之间**闪动 (不稳定)**; device 1 (`dv1`) 无信号; device 2 (`io0` = Mini Monitor 4K **输出卡**) decklinkvideosrc 打不开 (`Failed to set pipeline to PAUSED`), 符合预期。
- **推翻上一轮"三块卡均 0 buffer ⇒ 无信号"**: 0 buffer 真因是 (a) **信号闪动**导致 GStreamer 无法稳定锁定出流 (decklink 元素自身打印 `Signal lost` WARNING, 可靠); (b) 上一轮计数法 `grep -c chain` 在 GStreamer 1.28 失效 (fakesink silent=false 不打印 chain), 且彼时可能无稳定信号窗口。
- **GStreamer 选卡属性现场实测 (推翻第 29-34 行)**: 本机 `gst-inspect-1.0 decklinkvideosrc` (GStreamer **1.28.2**) **没有 `persistent-id` 属性**; 选卡属性只有 `device-number` (Integer) 与 **`hw-serial-number`** (String, 硬件 ID, 可读写)。故 `pipeline.rs::src_props` 的 Canonical 分支已由 `persistent-id=` 改为 `hw-serial-number=`, 相关注释/日志同步订正。**架构护栏**: 不同 GStreamer 构建属性名可能不同 (`persistent-id` vs `hw-serial-number`), canonical 物化选卡属性应以目标机 `gst-inspect` 实测为准, 不可假设。
- **媒体运行时链路本身健康 (证伪"运行时坏")**: `gst-launch-1.0 videotestsrc num-buffers=30 ! videoconvert ! fakesink` → 30 buffer / 19ms 跑完 EOS; 完整自测描述 `videotestsrc is-live=true ! videoconvert ! video/x-raw ! appsink + audiotestsrc is-live=true ! audio/x-raw ! appsink` 亦 PLAYING。证明 GStreamer 运行时 / appsink 闭环 / PTS 机制均正常, 卡点只在 DeckLink 信号稳定性。
- **新增 MEDIA-RT-01 自测模式 (B)**: `main.rs` 支持 `MEDIA_AGENT_SELFTEST=1` → 物化 `PipelinePlan::self_test()` (videotestsrc/audiotestsrc) → `GStreamerPipelineController` launch → **复用既有 `spawn_ingest_watchdog`** 推导 A1-A4/B1-B4/C1-C4; 自测源稳定出帧 → `pass()` 达成即打印 `MEDIA-RT-01: A+B+C 全过`。此模式不依赖 DeckLink 信号, 用于证明采集/健康/验收闭环在运行时层面正确 (DeckLink 信号仍待接)。
- **下一步**: ① rebuild (`--features bmd,gstreamer`) via CI → `gh run download` → `scp` 到 BMD → `MEDIA_AGENT_SELFTEST=1 ./media-agent-gstreamer` 拿**自测 MEDIA-RT-01 PASS**; ② 真实 A 需用户在 device 0 接 **稳定** SDI 信号 (当前闪动, GStreamer 无法锁定), 再 `MEDIA_AGENT_MODE=diagnostic` 重跑拿真首帧。Normalize/Switch/Encode/SRS/FI-08/09 顺序不变。

## C1 Resolver 落地 + 身份/选卡语义订正 (2026-08-26 晚, 提交 `8efd8ae`)

> 本节能与上方第 29-34 行 / 第 109 行并存: 上方是历史现场记录, 本節是经 **A0 逐字 `gst-inspect` 证据** (见 `2026-08-26-a0-identity-verification.md` §"GStreamer 侧核实") 复核后的**权威结论**。

### 1. A0 → C 决策 (已锁)
- A0 = DONE。`PersistentID` 在三台设备均 `GetInt=0x80000003` (不支持) → 正式选 **C**: DeviceHandle canonical 身份 + GStreamer `device-number` materialization (经 `hw-serial-number` 解析)。
- **DeviceHandle 语义订正 (关键)**: 它是"**当前主机的 best-available identity**", **不是**跨机器/重启永久稳定身份 (后者仅 `PersistentID` 提供)。Blackmagic 官方层级: `PersistentID → TopologicalID → DeviceHandle → Enumeration`。

### 2. DOC-01 语义订正: PersistentID = preferred, NOT mandatory
- ❌ 旧表述 ("PersistentID mandatory / 解析失败即 IdentityUnresolved"): 过于绝对, 与官方层级不符。
- ✅ 新表述: **PersistentID 是首选身份 (官方最高优先级), 但非强制**; 当硬件不支持 PersistentID 时, canonical 身份**降级为 DeviceHandle** (经 Resolver 物化), 不得判 "代码失败"。
- 代码落地 (`materialize`): Production 守卫放宽为 `PersistentID 可用` **或** `DeviceHandle 经 Resolver 解析到 device-number` → 允许; 二者皆无才 `IdentityUnresolved` (绝不 `unwrap_or(0)` 盲开)。

### 3. SoT — Resolver 边界 (DeviceRegistry → Resolver → PipelinePlan)
- `DeviceRegistry` (SDK, `device.rs`): 产出 **硬件身份** = DeviceHandle (`serial`), 派生确定性 `device_id` (UUIDv5)。本机 `identity_strength = DeviceHandle`。
- `Resolver` (`resolver.rs`, 新增 C1): 运行时探测 GStreamer `hw-serial-number` (per `device-number`), 与 SDK DeviceHandle / TopologicalID 精确匹配 → 输出 `ResolvedDeviceBinding { device_number, hw_serial_number, confidence }`。匹配优先级: PersistentID 精确 → Serial 精确 → DeviceHandle 精确 → TopologicalID 猜测(MEDIUM) → **Unresolved** (生产拒绝, 永不回退 device-number=0)。
- `PipelinePlan` (`pipeline.rs`): **只消费 Resolver 解析后的 `device-number`**, 绝不 (在生产/已解析时) 直接用 SDK 枚举序号 (SDK index ≠ GStreamer device-number, A0 实测)。`src_props`: Canonical 用 `persistent-id=<pid>` (持久身份可用时官方首选); 本机实走 `DiagnosticFallback` 用 `device-number=<resolved> connection=sdi`。
- 运行: `VBMF_RESOLVER=1 ./media-agent` 仅输出 C1 证据 JSON (DeviceHandle ↔ GStreamer device-number 完整映射), 不启动 pipeline。

### 4. EVID-01: PersistentID unsupported = 硬件能力, 非代码失败
- 见 `2026-08-26-a0-identity-verification.md` 判定栏 EVID-01 标注。`0x80000003` = BMD 属性不支持 (官方 FAQ: PersistentID 非所有 DeckLink 均支持)。**不得**在文档/日志写 "实现错误 / 代码失败"。

### 5. Gate 状态落档
| Gate | 状态 | 说明 |
|------|------|------|
| `HW-IDENT-02` (Device Identity Resolution) | **OPEN** | Resolver 已实现 (`8efd8ae`); 待盒上 `VBMF_RESOLVER=1` 输出 C1 证据, 确认 DeviceHandle↔hw-serial-number 匹配键。 |
| `MEDIA-RT-01` (First Frame / Ingest) | **BLOCKED** | 受两因素阻塞: ① 本机 SDI 信号闪动 (device 0), GStreamer 无法稳定锁定; ② 身份未最终解析 (待 C1 证据)。自测模式 `MEDIA_AGENT_SELFTEST=1` 可证运行时闭环, 但真首帧需稳定 SDI 信号。 |

### 6. 证据矛盾复核 (第 109 行 vs A0 §GStreamer 侧核实)
- 第 109 行称 "GStreamer 1.28.2 没有 `persistent-id` 属性"; A0 证据 (§"GStreamer 侧核实", 第 58-61 行) 逐字引用 `gst-inspect` 列出 **`persistent-id` 确为属性** (higher priority than device-number), 仅本机值恒 0 (`device 0 does not have persistent id. Value set to 0`) → 不可用。
- **权威源 = A0 逐字 `gst-inspect`**: `persistent-id` 是属性, 但本硬件值=0 → 选卡不可用 → canonical 选卡落到 `hw-serial-number` / Resolver 解析的 `device-number`。
- **行动项**: 盒上重跑 `gst-inspect-1.0 decklinkvideosrc` 以最终判定 (两证据文件矛盾须消); 无论结论, C1 Resolver 设计对两者均鲁棒 (运行时探测 `hw-serial-number`, 以解析后 `device-number` 为稳定选卡键)。`src_props` Canonical 分支维持 `persistent-id=<pid>` (持久身份可用时官方首选); 若读者据第 109 行改回 `hw-serial-number=`, 本机亦无影响 (实走 DiagnosticFallback)。

## P0/P1 代码收口 (2026-08-26 晚, 提交 `4169a73`)

> 用户复核 `master` (含 C1 `8efd8ae` + P2 文档 `0d6fa4a`) 后, 给出 "现在必须修" 清单 (§二-§十九)。本轮已落地, 宏观设计仍符合冻结 V0.2 / Phase 0.5 / Phase 0.6。

### 已修 (按用户编号)
1. **P0 安全边界 — Filesystem 伪 PersistentID** (`device.rs`): `bmd_persistent_id` 由 `Some(ph)` (节点名 hash) 改为 `None`; 强度维持 `Enumeration`。旧实现会让 `materialize` 把合成 hash 当真实 PersistentID 越权选卡 (§十八)。
2. **materialize 严格按 `identity_strength`** (`pipeline.rs`): 不再只看 `bmd_persistent_id.is_some()`; 状态机 `PersistentId+Some→PersistentIdCanonical` / `DeviceHandle+resolved→DeviceHandleResolved` / `TopologicalId+resolved→Diagnostic(生产拒绝)` / `_→生产IdentityUnresolved`。合成身份 (Enumeration) 在生产路径恒拒绝, 绝不 `unwrap_or(0)` 盲开 device 0 (§二/§三/§十九)。
3. **SourceSelectionMode 无歧义** (`pipeline.rs`): `Canonical` 改名 `PersistentIdCanonical` + 新增 `DeviceHandleResolved` (当前硬件正式生产路径)。生产运行态不再伪装成 "诊断 fallback" (§五)。
4. **Resolver 多重 HIGH → Ambiguous** (`resolver.rs`): 同 SDK 设备命中 ≥2 个 HIGH 候选 → `ResolverMatch::Ambiguous` → 生产拒绝, 绝不猜设备 (§七)。
5. **PTS 真正单调** (`pipeline.rs`): 由 "首帧对比" 改为逐帧 `last_pts`, video+audio 均查; 任一流回退即 `pts_monotonic=false` (§十一)。
6. **audio PTS 单调**: 旧实现缺音频单调检查, 已补 (§十一)。
7. **C 稳定性测量窗口** (`pipeline.rs`+`main.rs`): `MediaRt01Acceptance` 增 `c_observed_ms`/`c_configured_window_ms`(默认10s)/`c_video_frames`/`c_audio_frames`/`c_unexpected_eos`/`c_pipeline_errors`/`c_renegotiations`; `c_pass` 改为 `窗口达标 ∧ 无致命 error ∧ 计数增长 ∧ PTS 单调` (§十二)。
- **结构性 (§十六)**: 新增 `DeviceIdentitySource { RealBmd / FilesystemSynthetic / Simulation }`, 三 manager 分别标注, 防 synthetic UUID 与真实 BMD UUID 混淆。
- **注释订正 (§十/§十三)**: `src_props` 文档明确 `hw-serial-number` 是 **GStreamer 侧** 硬件 ID 探测属性 (非 BMD PersistentID 别名); appsink 当前仅作 MEDIA-RT-01 首帧/PTS 探针, 非最终生产媒体出口。
- **编译依赖修正**: `ResolverMatch`/`Confidence` 补 `Deserialize` (`ResolverEvidence`/`ResolvedDeviceBinding` 反序列化所需, 原缺会导致编译失败)。

### 暂缓 (用户明确 "不要现在修")
Normalize 完整实现 / FRAME_SWITCH / MASTER_SWITCH / Audio Master Join / FFmpeg encoder / SRS Output / FI-08 / FI-09 / 24h HA — 全部留到 MEDIA-RT-01 之后。

### 编译验证状态
⚠️ **本机 (Windows) 无 cargo, 上述改动未经本地编译**; 必须在 BMD 盒 `cargo build --features bmd,gstreamer` 或 CI 编译验证后方可上机。gated (`#[cfg(feature="gstreamer")]`) 代码段 (launch/attach_*/src_props) 未经本机 rust-analyzer 检查。

### 下一步 (用户 §二十/§二十一)
1. 先跑 **C1** `VBMF_RESOLVER=1 ./media-agent` 拿证据: `BMD DeviceHandle ↔ GStreamer hw-serial-number ↔ device-number` 映射。
2. 若获 `DeviceHandleExact`/HIGH 单值无歧义 → `HW-IDENT-02 = PASS` → 立即进 MEDIA-RT-01 A/B/C。
3. 若 `Unresolved`/`Ambiguous` → 停在 Resolver, **不准**用 device-number 猜。

## Gate 状态 (正式, 据用户 §22, 2026-08-26 晚)

| Gate | 状态 | 说明 |
|------|------|------|
| Architecture V0.2 | ✅ LOCK FINAL | 未重开 |
| Phase 0.5 | ✅ LOCK FINAL | 维持 |
| Runtime Ownership | ✅ | Media Agent owns media runtime lifecycle |
| Docker / BMD Runtime | ✅ | |
| MEDIA-SEC-01 | ✅ Option B / runc | |
| Gate 5 Policy | ✅ | |
| Gate 6 BMD FFI | ✅ | 真机枚举通过 |
| HW-IDENT-01 | ✅ | PersistentID unsupported confirmed (A0) |
| **HW-IDENT-02 Resolver** | 🟡 **OPEN — next** | Resolver 已实现 + P0/P1 收口 `4169a73`; 待盒上 `VBMF_RESOLVER=1` 证据 |
| **MEDIA-RT-01 A** | 🔴 BLOCKED | 受 SDI 信号闪动 + 身份待最终解析 |
| MEDIA-RT-01 B | 🔴 BLOCKED | 同上 |
| MEDIA-RT-01 C | 🔴 BLOCKED | 同上 (稳定性窗口已实现, 待真信号) |
| Normalize | ⏳ 未实现 | 仅 intent, 非生产执行 |
| Switch | ⏳ intent | FRAME_SWITCH 仅物化值 |
| Encode | ⏳ | |
| SRS | ⏳ | |
| FI-08 | 🛔 | 暂缓 |
| FI-09 | 🛔 | 暂缓 |

## C1 真实 GStreamer 探测实现 (2026-08-26, `b52e2b6`)

- `probe_gstreamer_devices` 由空占位改为真实 `gst::DeviceMonitor` 枚举 (`feature=gstreamer`):
  读 `device-number`(guint=u32)/`hw-serial-number`/`persistent-id`/`model`; 同 `device-number`
  的 video/audio 重复只记一次 (防 Resolver 误判 `Ambiguous`); `default`/`simulation` 构建留空占位.
- C1 模式 (`VBMF_RESOLVER=1`) 现同时打印: ① 原始 GStreamer 枚举 ② Resolver 证据 ③ 解析绑定.
  现场直接比对 **SDK `bmd_device_handle` ↔ GStreamer `hw-serial-number`** (C 设计待证关系, 用户 §二十/§二十一).
- **盒上运行** (BMD 盒, `--features bmd,gstreamer`):
  `VBMF_RESOLVER=1 ./media-agent`
  - 若某设备 `match_kind=DeviceHandleExact` 且单值无歧义 → `HW-IDENT-02=PASS` → 进 MEDIA-RT-01.
  - 若 `hw-serial-number` 与 `bmd_device_handle` 不一致 (`match_kind=Unresolved`/`TopologicalIdGuess`) →
    **停 Resolver**, 据原始枚举重新判定映射键 (DeviceHandle vs topo 末段 vs serial), 不得猜 device-number.
- ⚠️ 本机 (Windows) 无 cargo/GStreamer, 此 gated 代码未经本地编译; 待盒上 build 验证.
