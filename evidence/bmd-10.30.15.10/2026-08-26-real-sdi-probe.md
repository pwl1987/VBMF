# 2026-08-26 真实 SDI 采集探测 (canonical media-agent + gst-launch)

环境: BMD `10.30.15.10` (lytv), 二进制 `media-agent-gstreamer-linux`
(`--features bmd,gstreamer`, CI run 32967629294, commit `f4970db`)。

## 运行方式
- 真实 canonical 路径: `./media-agent-gstreamer` (**未**设 `MEDIA_AGENT_SELFTEST`)。
- 对照直探: `gst-launch-1.0 decklinkvideosrc device-number=N [connection=sdi] num-buffers=30 ! fakesink`。

## 结果 1 — 真实 media-agent 运行 (日志已落盘)
文件: `evidence/bmd-10.30.15.10/real-canonical-run-2026-08-26.log`

```
device discovery complete count=3
lease acquired device=578a04d1-... / 987f93c2-... / 4b5d3e8c-...
lease re-acquire correctly rejected (排他不变量 OK)
ERROR CAP-01 canonical ingest 物化失败 (identity 未解析):
  identity resolution failed: 578a04d1-...: BMD PersistentID 未解析 (bmd_persistent_id=0)
media-agent canonical runtime loaded (health :8080; ingest via GStreamer started on lease acquire)
```

→ **三卡 `bmd_persistent_id=0` → `MaterializeMode::Production` 硬规则返回 `IdentityUnresolved`,
   pipeline 从未启动**, 因此根本没走到信号检测 / 首帧判断。

根因: `DeckLinkDeviceManager::discover()` 以 BMD `DeviceHandle` 中段作 `bmd_persistent_id`
(`parse_persistent_id(&serial).unwrap_or(0)`); 三卡 `DeviceHandle` 中段均为 `00000000` → 0。
`materialize` 生产路径 (device.rs) 对 `bmd_persistent_id==0` 直接 `IdentityUnresolved`
(Phase 0.6 锁死: 绝不 `unwrap_or(0)` 盲开 device 0)。

## 结果 2 — gst-launch 直探 SDI 输入卡
| device-number | 结果 |
|---|---|
| 0 | Mini Monitor 4K — `does not have input interface` (仅输出, 无采集输入) |
| 1 | `connection=auto` 与 `connection=sdi` 均 `Signal lost / No input source detected` (30 buffers EOS, 0 帧) |
| 2 | `Failed to set pipeline to PAUSED` (无法启动) |

## 结论
当前 BMD 三卡均**未观测到 SDI 信号锁定**; 且 canonical 生产路径因 `PersistentID=0` 身份守卫
根本无法起 pipeline。两者叠加 → 真实 SDI 首帧 (CAP-01 生产语义) 暂时**不可达**。

## 待澄清 (用户)
1. **物理链路**: SDI 缆是否接进卡的 SDI **In** BNC? 信号源是否在发? 用对连接类型
   (decklinkvideosrc `connection=sdi`)? device 2 起不来也需排查硬件/占用。
2. **身份解析**: 三卡 `PersistentID=0` 是真实硬件值, 还是 `DeviceHandle` 解析偏差
   (中段未必是 PersistentID)? 若确需以 `device-number` 兜底, 应走代码已具备的
   `MaterializeMode::Diagnostic` (显式回退 + 证据标注), 属**架构决策, 不擅自放宽 Production 守卫**。

注: 自测模式 (videotestsrc) 已验证媒体运行时链路 + 验收推导可达 "MEDIA-RT-01: A+B+C 全过",
与生产 SDI 信号无关 (见同日 media-agent 验收 Default 修复记录)。
