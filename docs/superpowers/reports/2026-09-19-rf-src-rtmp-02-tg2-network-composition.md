# RF-SRC-RTMP-02 TG-2 — Network-Only Production Composition + Five-Tuple Authorization Wiring

- Date: 2026-09-19
- Packet: `RF-SRC-RTMP-02-IMPLEMENTATION` Step 2 / TG-2（前置：TG-1 + reconciliation `a29da14`）
- Design authority: `docs/superpowers/plans/2026-09-19-rf-src-rtmp-02-production-rtmp-input-boundary.md`（D3/D5/D10/D11 + allowed scope）
- Scope（本轮改动）:
  - 核心接线：`config.rs`、`preflight.rs`、`session.rs`、`bootstrap.rs`、`gates/ffmpeg_recovery.rs`；
  - 组合所需 neutral factory：`adapters/mod.rs`（无 manifest 的 `build_process_backend()` 双 view 工厂，零 concrete 类型外泄）+ `registry.rs`（`build_network_process_backend` 包装）；
  - `SessionManager::new` 增加 `network_binding` 类型参数引发的 1 行机械适配：`bin/media-agent.rs`、`command.rs`、`custody.rs`、`error_model.rs`、`event_projection.rs`、`idempotency.rs`、`runtime_query.rs`、`gates/{a204_obs,dual_input,r64_control_plane,session_lifecycle}.rs`、`tests/{control_plane_concurrency_probe,switch_fault_probe}.rs`（全部仅插入 `None`，零行为变化）。

## 1. 交付内容

### D10 network-only 组合路径（`bootstrap.rs`）

- **拆掉**旧 `build_ffmpeg_network_source_composition()`（先调用 `build_ffmpeg_session_composition()` 的 D10 已冻结错误路径——依赖 DeviceBindingManifest + 经 `bootstrap::build()` 的占位 Device lease），无任何残留调用。
- 新 `FfmpegNetworkComposition` + `build_ffmpeg_network_only_composition()`：
  - `NetworkSourceBinding::load()` 生产入口（getifaddrs D5 快照 + machine pin + 0600 单 fd 防竞态，全 fail-closed）**启动加载一次**；无 reload/watch/第二 endpoint truth。
  - **构造即 D10**：不调用 provider discover（无 DeckLink SDK 触碰、无 input 0/1 / output device-number 2 动作）、不读 DeviceBindingManifest、fresh lease manager（**零 bootstrap 占位 Device lease**）、仅按 `binding.authorized_sources()` 注册 Network `rtmp-input` Resource（D6 listener 冲突已在 TG-1 manifest 加载层拒绝）。
  - 复用共同原语（同类型同语义：SessionManager 唯一生命周期 owner / InMemoryLeaseManager / Supervisor / FanoutSink 双日志），`supervisor`/`lease_manager` 以同实例 Arc 暴露供 acceptance/recovery 接线（非第二决策 owner）。
  - SessionManager = `MaterializeMode::Production` + `network_binding: Some(binding)` + **device `authorizations` map 刻意为空**（Network 授权绝不进入 Device BindingAuthorization map，D11）。
  - 确定性 seam：`build_ffmpeg_network_only_composition_with(config, binding)`（测试注入已验证 binding）。
- `adapters/mod.rs::build_process_backend()`：无 manifest 的 `FFmpegBackend::new()` 双 view（backend + process_inspector）neutral 工厂（既有 `build_process_media_backend` 的孪生形态；concrete 类型不越过 adapters 层）。

### Config startup-only 入口（`config.rs`）

- `network_binding_path: Option<String>` ← `MEDIA_AGENT_NETWORK_BINDING`；仅启动组合消费一次；缺失时生产 network 组合拒启（fail-closed，测试锚定）。

### Production 五元组授权接线（`preflight.rs` / `session.rs`）

- `PreflightInputs.network_binding: Option<&NetworkSourceBinding>`；IdentityBinding 阶段重构为 5a 硬件（Decklink 原逻辑不变）/ 5b 网络 / 5c 合并单条结论：
  - Production + RTMP + **无 binding** → **FAIL**（"Production RTMP source requires NetworkSourceBinding admission (fail-closed)"）——不再走"Network/SelfTest 自动 PASS"；
  - Production + binding → 每 RTMP source `authorize()` 五元组精确授权（source_id/protocol/canonical_ip/port/exact_path），任一拒绝 → FAIL，detail 经 `NetworkBindingError` redaction-safe Display（无 endpoint 字面量）；
  - Diagnostic + 无 binding → Pass-with-note（"loopback fixture"）——**既有 loopback 回归语义保留**（SelfTest 不变）。
- `SessionManager` 新增 `network_binding: Option<Arc<NetworkSourceBinding>>` 类型化字段（`new()` 显式参数，非塞入 device map）；`create_inner` 步 5 在 lease/reservation 建立后**二次核验**（BindingFailed fail-closed；步 1 Preflight 为第一道闸）——授权先于任何 child spawn 路径。
- `SourceIntent::Rtmp` wire shape 不变；未触碰 pipeline.rs / FFmpeg argv / stderr / recovery（TG-3/TG-4 范围）：Production 非 loopback LAN endpoint 的 materialize 打开属 TG-3。

### gates/ffmpeg_recovery.rs 迁移

- `run_rtmp_source` 改走新 network-only 组合：gate 自写 0600/machine-pin loopback binding manifest（canonical URL 形式）→ `MEDIA_AGENT_NETWORK_BINDING` → 生产组合（真实生产准入路径）；supervisor/lease 断言改用 composition 同实例。
- **编译验证口径（如实披露）**：`bmd-provider,ffmpeg-backend` 组合在本 VM 无 DeckLink SDK、CI 亦不编译该交叉 cfg；本轮以临时将该文件 cfg 降至 `ffmpeg-backend` 完成 `cargo check` 编译验证后**原样恢复** 22 处 cfg（git diff 仅含语义改动）；最终权威 = TG-6 BMD native build。

## 2. D10 负向测试（`bootstrap::rf_src_rtmp_02_tg2_tests`，ffmpeg-backend）

- `is_device_side_effect_free`：2 个授权 source → 恰 2 个 `rtmp-input` Network Resource（device_id=nil、Available）、`devices` 空、sessions 空、**DeviceLease 集合为空**（构造前=构造后=∅ 基线差异）、**manifest 文件字节不变**（digest 不变）。
- `create_uses_five_tuple_only`：精确五元组 create → 仅 `LeaseKey::Network`（runtime lease 1、DeviceLease 0）；**不 start**（ffmpeg-backend 下真 start 会 spawn listener child——属 TG-3/BMD，不进单测）。
- `rejects_mismatch_fail_closed`：port 不匹配 → `PreflightFailed`、无 lease、其 Network Resource 保持 Available、Device lease 仍空。
- input 0/1 与 output device-number 2 零动作 = 结构性保证（组合路径无 discovery/无 device plane）+ 上述断言；硬件级非触碰证据按冻结设计属 TG-6 BMD Tier 1/Tier 2。

## 3. 软件验证（Development VM, Rust 1.98.1）

| 项 | 结果 |
|---|---|
| focused `preflight::` | **12 passed**（+3：Production 无 binding fail-closed / Diagnostic loopback 回归 / 五元组精确-失配-未知 source-非规范矩阵） |
| focused `session::`（mock） | **36 passed**（+3：Production 无 binding fail-closed / 精确五元组 create-start-stop 全周期 / endpoint 失配 fail-closed） |
| focused `bootstrap::`（ffmpeg-backend） | **7 passed**（+4：D10 零设备副作用 / 五元组 create / 失配 fail-closed / 配置缺失拒启） |
| default lib | **304/304** |
| simulation lib | **304/304** |
| mock lib + integration | **494/494** + **9/9 + 12/12** |
| ffmpeg-backend lib | **330 passed / 1 ignored**（既有 ignored 保持） |
| clippy `-D warnings` | default / mock / ffmpeg-backend / mock-tests 四档 PASS |
| `cargo fmt --check` / `check --all-targets`（default+mock） | PASS |
| architecture lint（ARCH-PORTABILITY-01 + BS-01） | PASS |
| remove-adapters proof（simulation/mock） | PROOF OK |
| `git diff --check` | PASS |

全部既有回归保留（loopback fixture/Diagnostic 语义、Session 生命周期、lease/断言标准无降低）。

## 4. 边界与遗留

- Production 非 loopback RTMP 的 materialize/listener argv/stderr/recovery 未开（TG-3/TG-4）；本轮 Production LAN endpoint 在 create 授权通过后于 start 的 loopback 校验处继续 fail-closed——不会误启 FFmpeg。
- step-5 二次核验为纵深防御（Preflight 已判）；进程内无 TOCTOU 窗口，单测覆盖以 Preflight 第一道闸为准。
- `bin/media-agent.rs` 仍为设备/诊断组合根；network-only 组合的消费入口接线（bin 模式分支）随 TG-3/TG-6 BMD 验收一并落位。
- 下一步 = TG-3：FFmpeg `-rtmp_listen` argv + stderr reader + 有限归因（adapters/ffmpeg.rs）。
