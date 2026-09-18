# RF-NORM-01 Phase B 验证报告

- 日期：2026-09-18
- 状态：**PHASE B COMPLETE / SOFTWARE + CI + BMD HARDWARE VERIFIED；RUNTIME WARNINGS REGISTERED**
- Final commit：`cb14014`
- Authority：`docs/superpowers/specs/2026-09-18-rf-norm-01-normalize-execution.md`

## 1. Scope and boundary

RF-NORM-01 Phase B 将显式 Normalize plan 接入 GStreamer per-plane
source → normalize → exact caps → selector 链，并在 selector boundary 产生
真实 video/audio caps evidence。FrameSwitch 仍是唯一可执行切换策略；
PACKET_SWITCH 与 MASTER_SWITCH 继续 fail-closed。

本阶段没有启用 MASTER_SWITCH，也没有修改 Network Source、FFmpeg Output、
Session/Resource/Lease、Clock/Flow/Idempotency、Recording/Replay 或 24h
stability contract。

## 2. Implementation

- `721a8ea`：补齐 progressive/interlaced Normalize chain；interlaced 路径为
  deinterlace → videoconvert → videoscale → videorate → 2x progressive rate
  → interlace field-pattern=1:1 → exact interleaved target caps。
- `75e7159`：保留旧 signal probe 的 interlaced caps，避免探针丢失输入形态。
- `cb14014`：gate 接入 L2c hard gate，要求 selector boundary 观察到 exact
  video + exact audio evidence，缺任一 plane 即 fail-closed。
- Canonical target：video I420 1920x1080 25/1 interleaved；audio S16LE 2ch
  48000 Hz。

## 3. Software and CI

- Development VM：`cargo test` 269/269；`cargo fmt`、`cargo clippy --all-targets -- -D warnings`、`git diff --check` PASS。
- GitHub Actions `35393863201`：7/7 required contexts PASS。
- GStreamer feature build 由 CI 与 BMD native build 验证；开发 VM 缺少本地 GStreamer dev pkg-config，不据此伪报 feature compile。

## 4. BMD exact-commit acceptance

- Source archive SHA-256：`7999e2fe1b59493db3d02b3b54abf69f080cb74860e8ce26d72f694a0b8a99f1`。
- Native build：`bmd,gstreamer` binary build PASS；binary SHA-256：
  `c25b146df2bc8391607c4940a392c9ae20b2f00b61896c5a95bfc5451b840444`。
- Manifest MD5：`7521d17e7fd02e50eb2b0a84374a43dd`。
- Inputs：handle `46:00000000:002e4500` / device-number 0，以及
  `46:00000000:002e4400` / device-number 1；output device-number 2 未触碰。
- Gate：**11/11 PASS**，包括 L1a/b/c/d、L2a/b、L2c exact V+A、
  L3、L4、L5 failure isolation/recovery、Supervisor、Teardown。
- L2c：`video=ObservedExact audio=ObservedExact complete=true`。
- Output baseline：PID `992634` 与原命令保持不变；无 orphan/残留。

Evidence archive：
`evidence/bmd-10.30.15.10/2026-09-18-rf-norm-01-phase-b/`

## 5. Runtime warning debt

最终 gate 仍观察到非致命 GStreamer Video critical：
`gst_video_converter_* assertion`，发生在 graph setup 及一次 recover；
teardown 另有 `gst_pad_unlink: GST_PAD_IS_SRC assertion`。这些告警没有
改变 11/11 verdict、L2c exact evidence、恢复或 teardown 结果，且已如实登记；
本报告不宣称 runtime warning-free。后续可单独开 hardening packet，不能在
本包结果中隐藏或重写为失败。

24h RSS `rss_bounded` debt 仍未验证关闭。

## 6. Next boundary

RF-NORM-01 Phase B 已收口。下一步只做 Runtime Features bounded packet
selection：先复核 frozen SwitchPolicy / MASTER_SWITCH boundary、当前
switch execution 与 RH-CLOCK/RF-NORM evidence，冻结单一 packet 的 allowed /
forbidden scope 和 acceptance 后再实施。