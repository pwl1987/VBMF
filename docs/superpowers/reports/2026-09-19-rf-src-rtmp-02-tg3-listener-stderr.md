# RF-SRC-RTMP-02 TG-3 — FFmpeg Listener argv + Bounded stderr Reader + Finite Attribution

- Date: 2026-09-19
- Packet: `RF-SRC-RTMP-02-IMPLEMENTATION` Step 3 / TG-3（前置：TG-2 `ae8a93d`）
- Design authority: `docs/superpowers/plans/2026-09-19-rf-src-rtmp-02-production-rtmp-input-boundary.md`（D2/D5/D6/D9 + INV-1 + audit appendix pipeline.rs 指令）
- Scope（本轮改动）：`adapters/ffmpeg.rs`（listener argv 显式 bind 输入、D9 stderr ring/reader 线程、reaper 排序、有限归因）+ `pipeline.rs`（按冻结审计附录指令把 Network 物化从 loopback-only 放宽到 D2 canonical；Diagnostic 保持 loopback fixture 边界）。无其它文件。

## 1. 交付内容

### Listener argv（adapters/ffmpeg.rs）

- `SourcePlan::Network` 的 argv 输入改为 **strict canonical endpoint**：`endpoint.to_canonical()` → `canonical.to_url()`（IPv6 方括号形式）→ `-rtmp_listen 1 -i <canonical-url>`。该地址上的 OS `bind()` 是最终权威（D6）；argv URL 为 D9 显式例外（绝不 log）。非 canonical（hostname/不合法类别/特权端口/非规范 path）在 argv 构造前 `PrepareFailed` 拒绝。
- 旧 `validate_loopback()` 闸门替换为 D2 canonical 校验（loopback 仍是合法类别——Diagnostic fixture 路径不受影响）。

### pipeline.rs 物化放宽（冻结审计附录指令：仅经 D2–D5 放宽）

- Network 物化分支：恒过 `to_canonical()`（非规范拼写两模式全拒）；`Diagnostic` 额外要求 loopback（fixture 边界保留）；`Production` 接受 eligible 类别（LAN 打开——准入已在 TG-2 Session 层五元组完成，此处不重复持有授权真值）。`SourcePlan::Network` 计划形态保持 wire endpoint（D11 不变）。

### D9 有界 stderr 捕获（adapters/ffmpeg.rs）

- **StderrRing**：≤64 KiB 总量、≤256 行、每行入 ring 前截断 512 字节（UTF-8 边界）；FIFO 淘汰。
- **独立 reader 线程**：network-listener 子进程 stderr = `piped()`（其余计划保持 null），spawn 后立刻起 `ffmpeg-stderr-reader` 线程持续排空（防管道死锁）；EOF（子进程退出关写端）即线程退出。
- **Reaper 排序**（stop / recover / observe-exit / Drop 四路径统一）：先 terminate+wait 子进程 → 再 join reader（EOF 保证退出，无死锁）→ ring 随实例释放。绝不遗留 child/reader。测试探针 `tg3_reader_state` 断言 join 后 reader/ring 双空。

### D9 有限归因

- `NetworkExitClass` = 冻结四类 `BindFailure | PublisherDisconnected | SpawnFailure | UnknownExit`；`classify_network_exit(status, ring)`：
  - bind 错误文本（D6 明确列举 "Address already in use" 等价物）→ `BindFailure`；
  - **INV-1 遵守**：`PublisherDisconnected` 绝不从 stderr 关键词单独归因——正向归因需结合 canonical signal 历史，属 TG-4 recovery wiring（枚举位已预留，注释锚定）；
  - 其余一律 `UnknownExit`（D7/D8：ManualRequired，绝不探索性重启）。
- 网络子进程退出事件 detail = `"network listener exit classified: <Category>"`——**仅类别名**；raw stderr 永不进入 canonical 事件/错误/Debug/health/evidence（e2e 测试断言 raw 文本缺席）。

## 2. 测试（+9：ffmpeg 6 + pipeline 3；既有全部保留）

- `rf_src_rtmp_02_tg3_listener_argv_consumes_canonical_endpoint`：LAN IPv4 + 方括号 IPv6 canonical URL 进 argv。
- `rf_src_rtmp_02_tg3_non_canonical_endpoint_rejects_before_argv`：hostname/公网/特权端口/非规范 path 四族拒绝。
- `rf_src_rtmp_02_tg3_stderr_ring_bounds`：512B 行截断（UTF-8 边界）/256 行 FIFO/64 KiB 总量。
- `rf_src_rtmp_02_tg3_exit_classification_is_finite_and_redacted`：bind→BindFailure；disconnect 噪音/空 ring→UnknownExit；Display=变体名（无 vendor 文本）。
- `rf_src_rtmp_02_tg3_reader_drains_classifies_and_reaps_without_leak`：真实子进程（stderr 重定向）→ observe 退出事件含 BindFailure 且**不含** raw 文本；reader joined + ring 释放。
- `rf_src_rtmp_02_tg3_stop_joins_reader_for_live_listener_child`：活体 listener stop 完成 kill→wait→join（缺 join 即挂起）。
- `rf_src_rtmp_02_tg3_production_materializes_canonical_lan` / `_diagnostic_fixtures_stay_loopback`（含 loopback 回归）/ `_non_canonical_wire_endpoint_rejects_in_both_modes`。

## 3. 软件验证（Development VM, Rust 1.98.1）

| 项 | 结果 |
|---|---|
| focused `rf_src_rtmp_02_tg3` | 9 passed（ffmpeg-backend 6 + default 3） |
| default lib | **307/307** |
| simulation lib | **307/307** |
| ffmpeg-backend lib | **339 passed / 1 ignored**（既有 ignored 保持） |
| mock lib + integration | **497/497** + **9/9 + 12/12** |
| clippy `-D warnings` | default / mock / ffmpeg-backend 三档 PASS |
| `cargo fmt --check` / `check --all-targets` | PASS |
| architecture lint + remove-adapters proof | PASS / PROOF OK |
| `git diff --check` | PASS（仅 `adapters/ffmpeg.rs` + `pipeline.rs`） |

## 4. 边界与遗留

- 归因四类中 `PublisherDisconnected`/`SpawnFailure` 的**构造点**属 TG-4（INV-1 需 canonical signal 历史 + Session 层映射）；TG-3 交付类别集、分类器、事件通道与 redaction 保证。
- D7 `Waiting/SignalVerified` 语义、D8 预算、recovery 路由 = TG-4；bind/handshake 超时 deferred risks 维持登记。
- BMD 实机 listener bind/stderr 行为 = TG-6 Tier 1/Tier 2（本轮 e2e 用 `/bin/sh` 真子进程验证 reader/reap 机制）。
- 下一步 = TG-4：recovery_monitor/supervisor 接 D7/D8（Waiting 非故障、SignalVerified 清零、BindFailure/UnknownExit 不计预算直接 ManualRequired、重启前 lease/claim 重验证、INV-2 代际隔离）。
