# RF-SRC-RTMP-02-CLOSURE-RECONCILIATION — 收口报告

- Date: 2026-09-19
- Packet: `RF-SRC-RTMP-02-CLOSURE-RECONCILIATION`（STATE §3.59；用户独立复核触发，不推倒既有实现）
- Authority: frozen plan `docs/superpowers/plans/2026-09-19-rf-src-rtmp-02-production-rtmp-input-boundary.md`（D3/D10/D11 + INV-1/2/3）+ §3.59
- Implementation commit: **`d13f1fd`**（STATE 先行降级 commit `3eff73a`；CI `35487255383` **7/7 required PASS**）
- Evidence: `evidence/bmd-10.30.15.10/2026-09-19-rf-src-rtmp-02-tg6r-closure-reacceptance/`

## 1. 修复的两个闭环缺口

### 缺口 ① production composition root（frozen D10/D11）

- 事实：`MEDIA_AGENT_NETWORK_BINDING` 与 `build_ffmpeg_network_only_composition()` 此前只有 tests / `media-agent-gates` 消费；`src/bin/media-agent.rs` 无条件 `bootstrap::build()`（Device discovery + 占位 DeviceLease）后仅构造 device 组合。TG-2 report §4 预留的"随 TG-3/TG-6 落位"承诺未兑现。
- 修复（`bootstrap.rs` + `bin/media-agent.rs`）：
  - 新增 `StartupCompositionMode::{Device, NetworkOnly}` + `select_startup_composition_mode()`——在 `bootstrap::build()` **之前**完成模式选择；`MEDIA_AGENT_NETWORK_BINDING` 是显式 Network-only 选择器；与 `MEDIA_AGENT_DEVICE_BINDING` 并存、或叠加 `MEDIA_AGENT_MODE=diagnostic` / `MEDIA_AGENT_SELFTEST` 均 fail-closed 拒启。
  - `run_network_only_runtime() -> !`：直接消费 production builder（env → `NetworkSourceBinding::load()` → `build_ffmpeg_network_only_composition()`）。不调用 Device provider discovery、不读 DeviceBindingManifest、不获取任何 `LeaseKey::Device`、不打开 DeckLink input/output、**不执行 DeckLink SDK probe**。进程内只有 network composition 的单一 SessionManager/LeaseManager/Supervisor truth（Device 平面从未构造）。
  - 零媒体自动启动保持；exposure 与 Device production 一致：/health 只带 events/agent_state/device_count(=0)，query/command/idempotency/switch 面保持 None（503 契约诚实），Control Plane 接线留给后续 packet。`FfmpegNetworkComposition` 新增 `projection_log` 只读观测面（非第二 truth）。
  - `SourceIntent::Rtmp` wire shape 未变；不自动从 manifest 生成 SourceIntent；manifest 仍只是 server-side admission truth。
- production-root focused tests（新增 9 个）：
  - mode 选择 6 项（default→Device、仅 device binding→Device、network binding→NetworkOnly{path}、双 binding 拒绝、diagnostic 拒绝、selftest 拒绝）；
  - closure 3 项：**真实 production 入口**（env→loader→composition，VBMF_MACHINE_ID + loopback manifest）构造后 devices 空 / 仅 1 个 Network Resource（device_id=nil）/ DeviceLease=0；manifest 缺失文件与非法内容均 fail-closed；
  - 既有 tg2 4 项与 rf_ff_01e 3 项零回归（后者与新增 env 敏感测试共享 `STARTUP_ENV_MUTEX` 串行）。

### 缺口 ② TG-6 gate 先碰 Device（严格 D10 证据失效）

- 事实：`src/bin/gates.rs` 无条件先 `bootstrap::build()`（上轮 Tier 1/Tier 2 日志：`device discovery complete count=3` + 3 条 `lease acquired`）。
- 修复（`bin/gates.rs` + `gates/ffmpeg_recovery.rs`）：
  - `run_network_only_gate()` 在 common bootstrap **之前** dispatch（命中 `VBMF_FFMPEG_RTMP_SOURCE` 即进入并在 gate 内 exit；未命中零副作用）。该路径不执行 Device discovery / bootstrap DeviceLease / DeviceBindingManifest / DeckLink SDK probe。
  - 旧 Device/GStreamer/其它 gates 继续走唯一 `bootstrap::build()`，A20-03 语义不变；gates.rs 头注释做最小 reconciliation：Network-only 是后来冻结的受限 composition path（同一 bootstrap.rs 内 builder），不是第二套 Runtime truth。architecture lint（含 A20-03-BS-01 检查）PASS。
  - gate 内新增 D10 机械断言（直接证据，非仅日志缺席）：startup（无 device plane / 仅 rtmp-input Resource / 恰 1 个 / DeviceLease=0）与 teardown（FFmpeg listener child 无残留 / manifest 字节级不变）各打印一条 `RF-SRC-RTMP-02 D10 … PASS` 行。
  - 附带修复：source gate 失败 marker 由误标的 `RF-FF-02` 改为 `RF-SRC-RTMP-02`。
- Mimosa 提交钩子在编辑期拦截了 `ffmpeg_recovery.rs` 的路径穿越告警；按其建议完成真实加固：gate manifest 写入拆为纯 body 生成 + temp-dir-only 文件写入（路径只含 temp_dir/pid/uuid，endpoint 只进 JSON body），读回经 canonicalize + temp-dir 前缀校验；HLS fixture 目录拒绝 `..` 组件并限定 temp dir 内。聚焦 normal 扫描 findingCount=0。（两次 commit 的完整扫描因 scanner_enobufs 未完成——如实登记，不宣称完整审计。）

## 2. 软件验证（Development VM，最终代码态）

- focused：mode 6/6；closure 3/3；tg2 4/4；rf_ff_01e 3/3。
- 全量：default **324/324**；simulation **324/324**；mock **536/536**（含 integration 9/9+12/12）；ffmpeg-backend **359 passed + 1 ignored**（既有 real-binary HW 冒烟）。
- clippy `-D warnings` 三档（default/mock/ffmpeg-backend）PASS；`cargo fmt --check` PASS；`check --all-targets`（default+mock）PASS；architecture lint PASS；remove-adapters proof PASS；`git diff --check` PASS。

## 3. BMD exact-commit 重验（全部绑定 `d13f1fd`）

- archive sha256 `7f242a25…`（本地=盒上一致）；binary sha256 `3d784e2a…`；cargo 1.98.0。
- **进程级严格 D10**：4 次 gate 运行日志中 `device discovery complete` / `lease acquired` / `adapter selection` / `DeckLinkAPI` 均为 **0 行**（上轮为 count=3 + 3 租约）。
- **loopback 回归** PASS：D10 startup/teardown、source（signal_verified）、recovery（3473648→3473845，attributed=PublisherDisconnected，supervisor=Running）、teardown、marker。
- **Tier 1 LAN** PASS："non-loopback listener binding verified"——`LISTEN 10.30.15.10:19350`（ffmpeg pid 3475669，显式 LAN 非 loopback/非通配）+ 同宿 ESTAB + recovery（3474113→3474312）+ teardown；gate manifest MD5 `29570aa7…`（与上轮一致=确定性）。
- **Tier 2 independent netns** PASS："cross-host third-party push verified"——`ESTAB 10.30.15.10:19351 ← 172.17.0.2:52228`（docker bridge 独立 netns 第三方推流）+ external-leg A/V verified + 断连归因恢复（3474624→3475042）+ 二次外部推流（gate 内 recovered-A/V 验证）+ teardown。VM 直推沿用上轮"被网络边界阻断"事实，未重试。
- **D7/D8**：断连→INV-1 归因（SignalVerified 历史 + 退出分类 ⇒ PublisherDisconnected）→ 预算化重启→ 二次推流成功；`supervisor=Running`（restart-completed 不重置预算）。
- **边界完整性**：device-2 PID 992634 全程存活（15-20:47:43→15-20:51:11）；零 ffmpeg/listener/docker 残留；UFW 临时规则（172.16.0.0/12→19351）验收后删除并复核。
- 详见 `tg6r-manifest.md`（含 PEER2 采样未捕获行的如实披露与 deferred risks 沿用）。

## 4. 收口判定

- RF-SRC-RTMP-02 自 §3.59 的 RECONCILIATION REQUIRED 恢复 **COMPLETE**：production root 真实可达（Network-only 模式选择先于一切 device 构造）+ TG-6 gate 严格 D10 进程级证据成立（同一 implementation commit `d13f1fd`：CI 7/7 + 三段 BMD 验收）。
- 未触发 frozen stop condition（单 Runtime owner 保持；无第二套 Session/Resource/Lease truth）。
- 未扩大边界：`SourceIntent::Rtmp` wire 未变；无 Control Plane/API 扩展；SRT、multi-input/Program/Switch、SRS/Output、Recording/Replay、RH-FLOW/RH-IDEM、24h stability 维持原状。

## 5. Addendum（2026-09-20）：Mimosa L2 停钩复查 — gate 路径产出点收口（`06e272c`）

- 复查标记 `ffmpeg_recovery.rs:370/587` 路径穿越。评估：实质风险低（manifest 路径 = temp_dir + pid + uuid，endpoint 只进 JSON body；读回 helper 已 canonicalize+限定），但复查建议（规范化 + 校验 + 限定目录）可在**产出点**真实落地，故按修复处理而非仅申辩误报。
- 修复：`write_gate_manifest_file` 返回前 canonicalize + temp-dir 前缀校验（流向 `set_var`/production loader 的 binding path 一律为规范化限定值）；`read_gate_binding_manifest` 保留纵深再校验；HLS fixture 目录补**创建前**词法 temp-dir 限定（fail-closed 前不得在 temp 外创建目录）。`SourceIntent::Rtmp` wire、gate 流程、D10 断言零变化。
- 验证：本地 `cargo check --features ffmpeg-backend --all-targets` + fmt PASS（该模块 cfg=bmd+ffmpeg，type-check 权威 = CI media-runner no-run 编译 + BMD native build，沿用 TG-2/TG-4 口径）；BMD @ `06e272c`（archive `7ae0cd87…`·binary `38af7706…`）native `bmd,ffmpeg-backend` build PASS + loopback gate leg rc=0：D10 startup/teardown PASS、`manifest_bytes_unchanged=true`（规范化读写链路实证）、归因恢复 3481014→3481211、零设备行为行、零 ffmpeg/listener 残留、device-2 PID 992634 未触碰。日志归档于 tg6r evidence 目录 `l2r-gate.log`（md5 `7c9c92cb…`）；CI 结论与最终对齐见 STATE §3.60 增补行。
