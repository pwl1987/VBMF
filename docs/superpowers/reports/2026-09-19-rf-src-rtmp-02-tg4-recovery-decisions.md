# RF-SRC-RTMP-02 TG-4 — D7/D8 Recovery Decisions Through Existing Supervisor/Monitor

- Date: 2026-09-19
- Packet: `RF-SRC-RTMP-02-IMPLEMENTATION` Step 4 / TG-4（前置：TG-3 `131aa52`）
- Design authority: `docs/superpowers/plans/2026-09-19-rf-src-rtmp-02-production-rtmp-input-boundary.md`（D6/D7/D8 + INV-1/INV-2）
- Scope（本轮改动）：`pipeline_events.rs`（中性共享层：`NetworkExitClass` + 冻结 detail 解析器）、`supervisor.rs`（typed 网络退出决策 + 预算观测 + 重启完成态）、`recovery_monitor.rs`（网络 D7/D8 决策环）、`adapters/ffmpeg.rs`（改用共享枚举，行为零变化）、`bootstrap.rs`（组合暴露同实例 resources 供 claim 重验证）、`gates/ffmpeg_recovery.rs`（spawn_network 新签名 + resources 参数）。SessionManager 所有权不变（零绕过、零第二 owner）。

## 1. 交付内容

### 中性归因词汇（pipeline_events.rs）

- `NetworkExitClass`（冻结四类）移入中性共享层；`NETWORK_EXIT_DETAIL_PREFIX` + `network_exit_class_from_detail()` 解析 FFmpeg adapter 的退出 detail（单一生产者 + 单一解析器，冻结格式）；**不可解析 → None → 恢复侧一律按 UnknownExit 处理**（INV-1 fail-closed）。

### Supervisor typed 决策（D7/D8）

- `report_network_exit(handle, class)`：`BindFailure`/`SpawnFailure`/`UnknownExit` → **Escalate（ManualRequired）立即、不消费预算、不重试**（D6/D7/D8）；`PublisherDisconnected`（唯一正向归因恢复类）→ 既有预算化 `report_failure` 路径（max 5、退避 1s·2^n cap 60s——`RestartPolicy::default()` 与 D8 数学一致）。
- `report_restart_completed(handle)`：自动重启完成回 Running **但不重置预算**（D8"listener 建立本身不重置"）；`report_recovered` 语义收窄为 **SignalVerified 级唯一会话内预算重置**（文档锚定）。
- `attempts(handle)` 只读观测（测试锚定"仅 SignalVerified 重置"不变量）。

### RecoveryMonitor 网络决策环（D7 判定表逐行）

- **Waiting 非故障**（D7 行 1）：活体监听子进程零事件 → 循环空转，无重启、无 recovery 事件（测试锚定）。
- **INV-1 终局归因**：退出分类 + signal 历史——子进程退出 + 该 listener 曾达 SignalVerified + 无 bind/spawn 证据 ⇒ `PublisherDisconnected`（预算化恢复）；否则 UnknownExit → ManualRequired（绝不探索性重启）。detail 不可解析同样 fail-closed。
- **D8 双重验证**：每次自动重启前重验 exact lease（`is_key_valid`）**与** exact `(protocol, ip, port)` registry claim（`rtmp-input` Resource 仍 Reserved/Allocated）——任一失效 → ManualRequired 不重启。
- **D8 预算重置语义**：`report_signal_verified()`（handle 公共 API）= 唯一会话内重置（attempts>0 时经 `report_recovered`）；重启后 signal 标志清除（新 listener 必须重新挣得 SignalVerified——测试锚定第二次未验证退出转 manual）；`stop_and_join`（Session stop/close 钩子入口）重置 supervisor 条目 = "stop/close 清除预算"；ManualRequired 在 operator stop 前持久（monitor 退出不清除）。
- **INV-2 代际隔离**：backoff 窗口内 stop → 丢弃本代重启（测试锚定 0 recoveries）；monitor 生命周期由 SessionStopHook/Drop join（既有机制）。
- 设备域 `run()` 语义零变化（legacy report_recovered-on-recover 保留，其 gate 依赖）。

### 接线

- `spawn_network` 增加 `Arc<SharedResourceRegistry>`（claim 重验证输入）；组合以同实例暴露 `resources`（非第二真值）；gate 调用点更新。
- `bmd-provider,ffmpeg-backend` 交叉 cfg 无 CI 编译覆盖——以临时 cfg 降级完成编译验证后原样恢复（同 TG-2 口径）；最终权威 = TG-6 BMD native build。

## 2. 测试（+10；既有全部保留）

- pipeline_events：detail roundtrip 四类 + 5 种垃圾输入 fail-closed → None。
- supervisor：manual 三类不消费预算（attempts 恒 0）/ PublisherDisconnected 预算化 + restart-completed 不重置 + report_recovered 重置。
- recovery_monitor（真实线程 + TestBackend）：
  1. Waiting 非故障（无事件 → Running/0 recovery）；
  2. BindFailure → ManualRequired、attempts 0、零重启；
  3. UnknownExit 无 signal → ManualRequired 零重启（INV-1 反探索性）；
  4. 已验证断连 → 一次预算化重启（attempts 保持 1）+ 新 listener 失去 SignalVerified（第二次退出转 manual）；
  5. SignalVerified 不伪造状态（无故障时不动预算/状态）；
  6. claim 丢失（Available）→ 不重启转 manual；
  7. backoff 中 stop → 丢弃重启代（0 recoveries）+ stop 清预算。

## 3. 软件验证（Development VM, Rust 1.98.1）

| 项 | 结果 |
|---|---|
| focused `rf_src_rtmp_02_tg4` | **10 passed** |
| default lib | **317/317** |
| simulation lib | **317/317** |
| ffmpeg-backend lib | **349 passed / 1 ignored** |
| mock lib + integration | **507/507** + **9/9 + 12/12** |
| clippy `-D warnings` | default / mock / ffmpeg-backend 三档 PASS |
| `cargo fmt --check` / `check --all-targets` | PASS |
| architecture lint + remove-adapters proof | PASS / PROOF OK |
| `git diff --check` | PASS（6 文件，全部在 TG-4 允许面内） |

## 4. 边界与遗留

- `report_signal_verified()` 的**生产调用方**（网络信号探测面）不在本轮：signal.rs 现为 DeckLink 探测；网络源 SignalVerified 生产接线随 TG-6 BMD 验收的消费入口落位（gate 可先以既有 signal 证据调用）。TG-4 交付 API + 全语义测试。
- BMD 实机断连/退避/手动路径 = TG-6 Tier 1/Tier 2 exact-commit 验收；deferred risks（握手超时/唯一 listener 占用等）维持 §9 登记。
- 下一步 = TG-5：redaction helper + 统一负向套件（PipelinePlan 序列化、session.rs Debug-hash 路径、全 canonical 面无 endpoint 字面量断言）+ D10 负向复核。
