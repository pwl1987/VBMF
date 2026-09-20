# SE-01A BMD Smoke Manifest — Standalone graceful shutdown（STANDALONE-ENTRY-01 S3/S4）

- Date: 2026-09-19 (BMD local 04:46, UTC+8)
- Host: BMD box `lytv` @ 10.30.15.10；cargo 1.98.0
- Packet: SE-01A（§3.61 冻结；plan `docs/superpowers/plans/2026-09-19-standalone-entry-01-planning.md` S3/S4）

## Exact commit / artifacts

| Artifact | Commit | SHA-256 |
|---|---|---|
| Source archive | `e3e7fe4`（含 d13f1fd 全部 + eb49718 shutdown 实现 + e3e7fe4 测试泄漏修复） | `c715d3738b1c364d464a08afba2ea9770ec4701d248c053d8423c080ec25dc2e` |
| `media-agent` production binary（`--features ffmpeg-backend`，dev profile，盒上原生构建） | built from above | `82abccb39046e0764e5af6418f775f45dd58f0b47c04ece26bac0ff8ace7032c` |

Feature 说明：network-only 生产路径只需 `ffmpeg-backend`（bmd-provider 在该模式下从未构造；gates binary 不参与本 smoke）。

## 运行面（se01a-smoke.log，md5 `3e49adc99efabc6a96ce8dbda36aff5e`）

- Manifest：gate 之外首次以**真实运维形态**提供的 NetworkSourceBinding——`/tmp/vbmf-se01a-binding.json`（0600，machine pin `bmd-10-30-15-10`，endpoint `rtmp://127.0.0.1:19356/live/se01a`，md5 `52be235c…`）。
- 启动 env：`VBMF_MACHINE_ID=bmd-10-30-15-10 MEDIA_AGENT_NETWORK_BINDING=… MEDIA_AGENT_HEALTH_BIND=127.0.0.1:18080`。
- Agent PID `3478856`。

## 判定（全部 PASS）

1. **启动/D10**：进程日志中 `device discovery complete` / `lease acquired` / `adapter selection` / `DeckLinkAPI` = **0 行**；network-only 组合 ready。
2. **/health（S4 readiness）**：`{"active_pipelines":0,"clock_lost_events":0,"devices":0,"dropped_bus_events":0,"state":"Ready"}`——`state=Ready`（Starting→Ready 语义）且 `devices=0`。
3. **SIGHUP（S3 无操作）**：`kill -HUP` 后进程存活，日志出现 `SIGHUP received: manifests/config are startup-only (frozen D3); no reload performed`——捕获生效（SIGHUP 默认动作是终止）。
4. **SIGTERM（S3 有序停止）**：`kill -TERM` → **exit code 0**；日志序列 `graceful shutdown signal received; draining network sessions cause=Terminated` → `graceful shutdown: network sessions drained count=0`（零自动启动 ⇒ 零会话待 drain，与 P1-3 一致）→ `graceful shutdown complete (exit 0)`。
5. **边界**：零 ffmpeg 残留；18080/1935x 无 listener 残留；输出设备 device-number 2 PID 992634 全程存活（15-21:50:19 → 15-21:50:23）未触碰。

## 覆盖口径（如实）

- Dev VM 侧进程级 binary 测试（spawn + kill 断言）因 Mimosa 写入钩子持续拒绝 `Command::new(env!(...))` 测试形态而未入库——进程级证据由本 smoke 承担（真实 binary + 真实信号 + 真实退出码）；in-crate 真信号测试（raise/wait/singleton/HUP）+ mock drain 套件覆盖语义面。
- 会话内 drain（SIGTERM 时存在活跃 FFmpeg listener）未在真实 binary 上演示：production 零自动启动且 Control Plane 未接（P1-3），无法在无命令面的情况下创建会话；drain 语义由 mock 套件（逆序/跳过 Released/多会话）与 TG-6 gate 的 per-session stop 链共同锚定。该演示留待 SE-01B 部署 smoke（systemd + 显式命令面）或后续控制面 packet。
- 二次信号逃生门（第一个信号后恢复默认处置）为代码路径（`restore_default` 于 wait 返回前执行），未做进程级演示。
