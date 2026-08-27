# CODEBUDDY.md This file provides guidance to CodeBuddy when working with code in this repository.

## 项目概览
VBMF 是 24/7 广播机房的生产级 IP 媒体信号处理平台（Apache 2.0）。仓库目前只有**架构文档 + Reference 定义 + 一份可编译的 Rust 实现 `media-agent`**；Web 控制台（Node/Fastify）属 Phase 4，尚未落地代码。**V0.2 架构基线 LOCK FINAL**——架构级改动须走 V0.3 流程，本仓库内的 Rust 改动属 Phase 1 实现层，不要触碰 V0.2 的 12 Engines / Switch Mode 3 种 / Data Plane 4 Layer 等核心定义。

`media-agent` 是 **Hardware Plane（硬件平面）**：只负责 DeckLink/GStreamer 采集、设备枚举、信号探测、管线启动。控制面（API/auth/RBAC/配置/UI）在独立的 Node/Fastify 服务（尚未实现），二者边界见架构文档 SoT §14。Rust 不负责鉴权；JSON-RPC 必须 localhost/Unix socket，由 Fastify 反代。

## 常用命令（在 `services/media-agent/` 下执行）
- **默认构建**：`cargo build` —— 仅 default features（文件系统发现，无 SDK/无 GStreamer），CI 默认目标。
- **Lint（CI 强制门禁）**：`cargo clippy --all-targets -- -D warnings` —— clippy `-D warnings` 是 required gate，两套 features 都必须跑。
- **测试**：`cargo test` —— 默认 45 个单测通过；CI 仅跑 default + simulation。
- **单测**：`cargo test <测试名>` 或 `cargo test -- --nocapture` 看输出；定位某模块用 `cargo test <module>::`。
- **文档链接检查**：`python scripts/check_docs.py` —— 校验 docs 链接可达 + 关键数字口径（PR 前必跑）。
- **真机特征构建**：`cargo build --features bmd,gstreamer`（需 `DECKLINK_SDK_INCLUDE`、`LIBCLANG_PATH`、系统 GStreamer 1.22+ 与 decklink 插件；详见下文"BMD 真机"）。
- **诊断模式自启**：`MEDIA_AGENT_MODE=diagnostic MEDIA_AGENT_DEVICE_BINDING=<manifest.json> ./target/debug/media-agent`。
- **设备映射探针**：`VBMF_RESOLVER=1 ./target/debug/media-agent` —— 输出 SDK DeviceHandle↔GStreamer device-number 交叉证据后 exit(0)，不启动媒体。

## Feature 模型（编译期冻结语义）
`default` 最小可编译运行；`simulation` mock 设备（CI/单测，无硬件无 SDK）；`bmd` 编译真实 DeckLink FFI（bindgen 生成 `DeckLinkAPI.h` 绑定，需 SDK include + libclang）；`gstreamer` 编译真实 GStreamer 采集执行器（canonical 媒体路径）；`hardware-test` = `["bmd"]` 仅追加详细设备探针。**互斥约束**：`hardware-test` 与 `gstreamer` 不能同时启用（main.rs 顶部 `compile_error!`，避免双采/争用同一块 DeckLink）。真机 canonical 构建固定为 `--features bmd,gstreamer`。

## 高层架构（需跨多文件理解）
**1. 平面边界与模块划分。** `main.rs` 是二进制入口，模块见各 `mod`：设备发现（`device.rs`）、BMD FFI（`decklink.rs`/`sdk.rs`）、采集管线（`pipeline.rs`）、五层模型（`port.rs`）、信号探测（`signal.rs`/`fixture.rs`）、解析器（`resolver.rs`）、租约（`lease.rs`）、配置（`config.rs`）、健康（`health.rs`）、RPC 骨架（`rpc.rs`）、主管（`supervisor.rs`）、验收（`hw_port_01.rs`/`graph_intent.rs`）。Rust 与 Node 的唯一契约是 JSON-RPC（`session.apply_revision` 是控制面），SIGUSR1 等非产品契约。

**2. 五层模型（Device→Port→Capability→Runtime Binding→Signal）。** `port.rs` 是核心抽象：`PortRegistry` 聚合设备端口，`ConnectorType`（Sdi/Hdmi/DisplayPort/Optical/Analog/Unknown）决定 GStreamer `connection=` 取值。物化时按 `port_id`（Control Plane 显式声明）优先、否则回退设备首个 Input 端口推导连接器类型。`device.rs` 的 `DeviceBindingManifest` 是权威绑定来源：生产缺 manifest → 失败闭合（拒 materialize，绝不回退 legacy）；仅 `MEDIA_AGENT_MODE=diagnostic` 允许 legacy 回退；`validate_manifest()` 做 handle/device-number 唯一性 + machine_id 非空校验。

**3. Canonical 采集管线。** 唯一 canonical 路径 = `decklinkvideosrc` + `decklinkaudiosrc` → GStreamer → RAW → Normalize → FRAME/MASTER SWITCH → Encode → 分发。`pipeline.rs` 的 `SourcePlan`/`materialize`/`src_props` 拼出 launch 串：`src_props` 由推导出的 `connector` 决定 videosrc 的 `connection=` 片段；**audiosrc 不设 `connection`**（该插件无此属性，音频内嵌 SDI 流）。`IDeckLinkInput` 仅作 Discovery/诊断探针，不进入媒体流。`self_test` 在 `MEDIA_AGENT_SELFTEST=1` 下跑通即 A+B+C。

**4. Resolver（C1）身份对齐。** `resolver.rs::probe_gstreamer_devices` 用 `ElementFactory::make("decklinkvideosrc")` + `set_state(READY)` 读只读属性遍历 device-number；**绝不用 `GstDeviceMonitor`**（实机不列 DeckLink）。SDK 枚举序 ≠ GStreamer `device-number`（A0：SDK#0=SDI 但 GStreamer#0=MiniMonitor），身份优先级 **PersistentId > DeviceHandle > TopologicalId > EnumerationOnly**，`device-number` 绝不默认 0。生产多重 HIGH → `Ambiguous`（拒），Unresolved/MEDIUM → `IdentityUnresolved`，绝不盲开 device 0。

**5. 设备身份与探测。** 三设备仅暴露 `IDeckLinkProfileAttributes`，canonical 硬件身份 = **DeviceHandle**（`GetString(BMDDeckLinkDeviceHandle)`）。`decklink.rs` 的 SDK 调用（`CreateDeckLinkIteratorInstance_0004` 直返 `IDeckLinkIterator*`、`QueryInterface` 的 `riid` 须 by-value 16B GUID、`EnableVideoInput`、`DoesSupportVideoMode` vtable@3 须 7 入参）是 FFI 坑集中地。

**6. 验收模式。** `diagnostic` 模式按 feature 构建 `PortRegistry` 并传入 `materialize` 自动启动采集；`VBMF_RESOLVER=1` 仅输出映射证据后退出；`hw_port_01.rs`/`signal.rs`（`ExpectedSignal` 亮度黑场检测）支撑 HW-PORT-01 / MEDIA-RT-01 端口级绑定闭环验收。

## BMD 真机（10.30.15.10）构建/验证要点
- 真机需系统 GStreamer 1.22+ 与 decklink 插件、SDK include 软链 `DECKLINK_SDK_INCLUDE`、LLVM `LIBCLANG_PATH`。盒上 `cargo` 每次会更新 crates.io lock（不影响结果）。
- **已核验的官方常量（勿臆改）**：`decklink.rs` 的 `CANDIDATES` 六码（`HD1080i50=0x48693530`/`i5994=0x48693539`/`i6000=0x48693630`/`p50=0x48703530`/`p30=0x48703330`/`720p50=0x68703530`）、`bmdFormat8BitYUV=0x32767579`('2vuy')、`bmdNoVideoInputConversion=0x6E6F6E65`('none')——`decklink.rs` 内逐字引用官方 BMD SDK 头（`DeckLinkAPIModes.h`/`DeckLinkAPI.h`）+ 真机核验。BMD SDK 头为闭源、随 SDK 分发（位于盒上），不在仓库、公开网络亦无官方可抓取源，故以 `decklink.rs` 内联引用为权威；改这些值前先 `grep` 盒上 SDK 头。
- **GStreamer `connection` 枚举 nick（官方枚举全集，见 `gst-plugins-bad/sys/decklink/gstdecklink.cpp` 的 `gst_decklink_connection_get_type()` → `GST_TYPE_DECKLINK_CONNECTION` 的 `g_enum_register_static`）**：`{AUTO,auto}` / `{SDI,sdi}` / `{HDMI,hdmi}` / `{OPTICAL_SDI,optical-sdi}` / `{COMPONENT,component}` / `{COMPOSITE,composite}` / `{SVIDEO,svideo}`。注意 Optical 对应 BMD `bmdVideoConnectionOpticalSDI`，nick 是 **`optical-sdi`**，**绝不是 `optical`**——错值会让 launch 解析失败。该枚举官方名为 `GstDecklinkConnectionEnum`（旧文档常称 `GstDeckLinkVideoConnection`，实为一物）。
- 同步到盒：本地 `tar --exclude=target -czf media-agent-src.tar.gz .`（services/media-agent）→ scp → 盒 `tar -xzf` 到 `~/media-agent-build` 再 `cargo build --features bmd,gstreamer`。二进制下发前 `pkill -x media-agent`，再用 `setsid env LD_LIBRARY_PATH=/usr/lib .../media-agent </dev/null >/tmp/x.out 2>/tmp/x.err &` 脱离会话运行。
- 本机新文件若用 CRLF，提交前转 LF（否则 git 报 `fatal: ... CRLF`）。

## 约定
- 全程中文（注释/日志/文档正文）；标识符/命令/API/库名保留英文。
- PR 标题 `[scope] 简短描述`；改动须含 What/Why/How/测试/架构影响（当前必须为"无"）。
- 架构冻结：任何 V0.2 核心定义修改须 V0.3；Phase 0.5/0.6 是 Acceptance Validation，不修改架构。
