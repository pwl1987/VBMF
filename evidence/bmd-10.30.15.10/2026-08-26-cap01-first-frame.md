# CAP-01 视频采集首帧到达验证 (MEDIA-RT-01) — BMD 真机 2026-08-26

## 背景与目标
- Gate 2.6 进入采集闭环，验收项 **MEDIA-RT-01（首帧到达）** 须在真实 DeckLink 设备 + 真实 SDI 信号下验证。
- CAP-01 走纯 FFI（`IDeckLinkInput` 回调）采集，不依赖 GStreamer；验证载体 `CaptureStats{frame_count, first_frame_at, last_pts, pts_monotonic}`。

## 环境
- 真机：`lytv@10.30.15.10`，`~/media-agent-src/` 本地 `cargo build --features bmd` 产物（commit `15025c9`，已 push 至 origin/master）。
- SDK 运行时：`LD_LIBRARY_PATH=/usr/lib` 注入 `libDeckLinkAPI.so`（Desktop Video 16.0 驱动自带）。
- 设备：3 台（`device discovery complete count=3`，含 2×DeckLink SDI + 1×DeckLink Mini Monitor 4K，型号/序列号见 Gate 6/7 枚举证据）。

## 运行命令
```bash
# BMD 真机
bash /tmp/run_cap.sh          # pkill 旧进程 + setsid ./target/debug/media-agent > /tmp/run7.log 2>&1 < /dev/null &
# 采集 ~9s 后 pkill，日志落盘：
cp /tmp/run7.log ~/cap01-first-frame-2026-08-26.log
```
原始日志：`evidence/bmd-10.30.15.10/2026-08-26-cap01-first-frame.log`

## 关键输出（节选自上述日志）
```
INFO media_agent: device discovery complete count=3
INFO media_agent: lease acquired device=578a04d1-...
INFO media_agent: lease acquired device=987f93c2-...
INFO media_agent: lease acquired device=4b5d3e8c-...
INFO media_agent: SDK libDeckLinkAPI.so reachable, entry symbols present
[CAP-01] DoesSupportVideoMode(1080i50)=hr=0x00000000 supported=1 actual=0x48693530
[CAP-01] capture started on device 0 (mode=1080i50/8bitYUV, format-detection on)
INFO media_agent: CAP-01 capture live frame_count=0 first_frame=false pts_monotonic=true
[CAP-01] first frame arrived; hw_clock=165810169361530
INFO media_agent: CAP-01 capture live frame_count=19  first_frame=true pts_monotonic=true
INFO media_agent: CAP-01 capture live frame_count=44  first_frame=true pts_monotonic=true
INFO media_agent: CAP-01 capture live frame_count=69  first_frame=true pts_monotonic=true
INFO media_agent: CAP-01 capture live frame_count=94  first_frame=true pts_monotonic=true
INFO media_agent: CAP-01 capture live frame_count=119 first_frame=true pts_monotonic=true
INFO media_agent: CAP-01 capture live frame_count=144 first_frame=true pts_monotonic=true
INFO media_agent: CAP-01 capture live frame_count=169 first_frame=true pts_monotonic=true
INFO media_agent: CAP-01 capture live frame_count=194 first_frame=true pts_monotonic=true
```

## 数据核对
- **信号格式探测**：`DoesSupportVideoMode(1080i50)` 返回 `S_OK supported=1`，实测 `actual=0x48693530`（`bmdModeHD1080i50` 官方枚举值，非此前误用的魔法数字 `12`）。
- **首帧到达**：`[CAP-01] first frame arrived; hw_clock=165810169361530` → MEDIA-RT-01 达成。
- **帧率**：9 秒内 `frame_count` 0→194，平均 ≈ 21.5fps 净增，稳态 ~25fps，符合 1080i50 标准（采集回调按帧交付，每帧 ≈ 39.9ms）。
- **PTS 单调性**：每秒 `pts_monotonic=true`，经 `GetHardwareReferenceClock(1e9)` 校验硬件时钟严格递增，无回退。

## 判定
- **MEDIA-RT-01（首帧到达）= PASS** ✅
- 关联修复：硬编码 `mode=12`（误为 `bmdModeHD1080i6000`）→ 改为 `DoesSupportVideoMode` 动态探测 + 正确枚举 `0x48693530`；`IDeckLinkInput::DoesSupportVideoMode` vtable 补为 8 参数（含 connection/conversionMode/flags + actualMode*/supported*）。
- 关联提交：`15025c9`「feat(media-agent): CAP-01 DeckLink video capture (MEDIA-RT-01 first frame)」，已 push 至 origin/master。

## 当前 Gate 进度（2026-08-26）
- 1 skeleton / 2 discovery / 3 lease / 4 health+排他性 / 2.5 SDK FFI / 6 枚举 / **7 首帧(MEDIA-RT-01) 均 OK（BMD 实机）**。
- 待办：5 supervisor 已写代码待验证；`pipeline.rs` 接 `start_capture` + 推流链路；FI-08/09 故障注入。
