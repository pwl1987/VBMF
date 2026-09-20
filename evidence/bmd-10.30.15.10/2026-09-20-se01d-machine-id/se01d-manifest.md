# SE-01D BMD 重 pin 实证 — machine identity 收敛 @ `6932d5e`（2026-09-20）

**Packet**: STANDALONE-ENTRY-01 / SE-01D（frozen plan S5：`VBMF_MACHINE_ID` > `/etc/machine-id` > 空；HOSTNAME fallback 移除）

## 环境

- exact commit `6932d5e389658adbe2b02cc8a5961fa0a55958f5`（CI run `35492879328` **7/7 required PASS**）
- git archive sha256 `4c79b0d59cafe3bdfadb563ca0db318cbb3f4941afbe817efd2418ba7073c840`（本地=盒上一致）
- binary sha256：`media-agent` = `491942be3c4a14dce32973f23d6a0272c06663fee5a28b3c30f7a87418921ba6`；`media-agent-gates` = `30661ab3012e34f45dae93b4d6d593d5d87aa6cefbca1fd1ad1890db1a507925`
- native build：`DECKLINK_SDK_INCLUDE=/home/lytv/decklink-sdk-include cargo build --features bmd,ffmpeg-backend --bins` PASS（24.56s）
- BMD `/etc/machine-id` = `709ceacda35847a6b61350397eef8a39`（本轮唯一身份来源——**全部 leg 均 `env -u VBMF_MACHINE_ID`**）

## Leg 1 — Network 类重 pin：loopback gate（无 env）

- Env：`VBMF_FFMPEG_RTMP_SOURCE=1 VBMF_FFMPEG_RTMP_SOURCE_URL=rtmp://127.0.0.1:19350/live/loopback VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR=/tmp/se01d-loopback-hls`（无 `VBMF_MACHINE_ID`）
- rc=0；`se01d-gate.log`（md5 `a267d48a879203c23bc866db8bf70df4`）：
  - `RF-SRC-RTMP-02 D10 startup PASS device_discovery=0 bootstrap_device_leases=0 device_resources=0 network_resources=1 manifest_bytes=176`
  - source PASS（listener=loopback, signal_verified=true, h264+aac）→ recovery PASS（3487404→3487601, attributed=PublisherDisconnected）→ teardown PASS → `D10 teardown PASS … manifest_bytes_unchanged=true` → 最终 marker
- **重 pin 证明**：gate 写出的 manifest `…-3487403-643827b2….json` 内容 `"machine_id":"709ceacda35847a6b61350397eef8a39"`（= `/etc/machine-id`），且被生产 loader 载入（manifest_bytes 159→176 = 身份值变长的确定性差异）

## Leg 2 — Network 类负 pin（production binary）

- `/tmp/se01d-net-neg.json`（0600，machine_id=`wrong-host`）+ `MEDIA_AGENT_NETWORK_BINDING`，无 env
- **exit 2** + `network-only production composition failed closed: network binding manifest rejected (fail-closed): network binding manifest machine_id mismatch`
- `se01d-net-neg.log`（md5 `ad415a4d59215a5e072ac7862ce0fc3d`）

## Leg 3 — Network 类正 pin（production binary，/etc-pinned）

- `/tmp/se01d-net-pos.json`（0600，machine_id=`709cec…`），无 env → 启动成功
- `/health` = `{"active_pipelines":0,"clock_lost_events":0,"devices":0,"dropped_bus_events":0,"state":"Ready"}`
- SIGTERM → `draining network sessions` → `drained count=0` → **exit 0**
- `se01d-net-pos.log`（md5 `be0f735d665cc91d63c1e1ed21e44289`）

## Leg 4 — Device 类负 pin（production binary）

- v5 manifest（`~/a2-8-02i-v5.manifest.json`）machine_id 改为 `wrong-host` + `MEDIA_AGENT_DEVICE_BINDING`，无 env
- **exit 2** + `FFmpeg production composition failed closed: ManifestEnvironmentMismatch: 清单 machine_id='wrong-host' 与当前主机 '709ceacda35847a6b61350397eef8a39' 不符 (误投/串机器, 生产拒绝)`
- 错误信息中"当前主机"值即 `/etc/machine-id`——运行时身份来源的直接证据
- `se01d-dev-neg.log`（md5 `990e2367e8e0bddeecff28007b193f23`）

## Leg 5 — Device 类正 pin（production binary，/etc-pinned）

- v5 manifest machine_id 改为 `709cec…`，无 env → 全启动成功：`device discovery complete` ×1（count=3，只读枚举，零媒体自动启动）
- `/health` = `{"state":"Ready","devices":3,…}`
- SIGTERM → `draining sessions` → `graceful shutdown complete` → **exit 0**
- `se01d-dev-pos.log`（md5 `359e998398c5f6c947f924c14b086fb5`）

## 边界完整性

- 输出设备 device-number 2 进程 PID 992634 全程存活（15-23:00:23 → 15-23:01:07，命令未变）
- 验收后零 ffmpeg 残留（`pgrep -x ffmpeg` = 0）；8080/1935x 无 listener 残留
- 临时 manifest 文件均在 `/tmp`（`se01d-*.json`，0600/network 类），不触碰既有 `~/a2-8-02i-v5.manifest.json`

## 如实披露

- 旧清单迁移影响已实证：既有 pin 到 `10.30.15.10`（旧 HOSTNAME 风格值）的清单在无 env 时将被拒（Leg 4 错误形态相同）；运维注记已写入 `STANDALONE_MEDIA_AGENT_OPERATIONS.md` §4.5 规则 5。
- resolver 回归测试（6 项 `se01d_*`）在 Dev VM 四档全矩阵通过（resolver 无 cfg 门，不需 BMD 重跑）。
- Dev VM 侧 Mimosa 提交钩子 scanner_enobufs（兼容策略放行）；不宣称完整项目安全审计。
