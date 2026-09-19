# RF-SRC-RTMP-02 TG-6 BMD LAN Acceptance Manifest

- Date: 2026-09-19 (BMD local 22:13–22:43, UTC+8)
- Host: BMD box `lytv` @ 10.30.15.10 (ssh via Development VM; VM = 10.30.5.80)
- Toolchain: cargo 1.98.0 on BMD; ffmpeg git-2026-08-23-1019f8f (/usr/local/bin)

## Exact commits / artifacts

| Artifact | Commit | SHA-256 |
|---|---|---|
| Tier 1 source archive | `ee856d4` (CI 35448354450, 7/7 required PASS) | `6a0f6b5f22392539fd6fb54f5a2b53231066165c72c529d0ad20f8e8d2de009a` — INVALID (subdir archive, superseded) |
| Tier 1 source archive (used) | `ee856d4` | `7b96e2aab69af059eb3aaa312e88f499a74e0ab727471fb53155899d672b8fd1` |
| Tier 1 gates binary | built from above | `aceb3a12b7507b1cf9a195ebd7fd501730d08d56b105939d56876ecded5b0066` |
| Tier 2 source archive | `58fd33d` (CI 35448724626, 7/7 required PASS) | `b1d096c1ec7bd33023ab1610a8b70fe5c7590dc09398c8795851e06527caab59` |
| Tier 2 gates binary | built from above | `ebe1a0a6e9ffed1333e5167b72aa00f463d478bf1785bf3eb4b43fc636a0f3ee` |

Build: `DECKLINK_SDK_INCLUDE=/home/lytv/decklink-sdk-include cargo build --features bmd,ffmpeg-backend --bin media-agent-gates` (dev profile, in-tree at /tmp/vbmf-tg6-<commit>).

## Runs

### Tier 1 — same-host LAN binding + publish + attributed recovery (commit ee856d4)

- Env: `VBMF_MACHINE_ID=bmd-10-30-15-10 VBMF_FFMPEG_RTMP_SOURCE=1 VBMF_FFMPEG_RTMP_SOURCE_LAN=1 VBMF_FFMPEG_RTMP_SOURCE_URL=rtmp://10.30.15.10:19350/live/source VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR=/tmp/tg6-tier1-hls`（内部同宿 publisher）
- Gate manifest（gate 自写 0600/machine-pin/D5 本机地址）: `/tmp/vbmf-ffmpeg-recovery-network-binding-3454658-a765ad69-….json`, MD5 `29570aa7ac8a89aa2ebc7ba9227d4fc1`, 159 bytes, mtime 22:22:34 — 与运行/teardown 后一致（digest 不变）。
- Listener bind 证据: `LISTEN 0 1 10.30.15.10:19350 0.0.0.0:* users:(("ffmpeg",pid=3457218,fd=3))`（运行期 ss 采样）。
- 结果（tier1-gate.log）: source PASS（listener=lan, signal_verified=true, h264+aac）→ recovery PASS（old 3454663 → new 3454862, attributed=PublisherDisconnected, supervisor=Running）→ teardown PASS（Released/Available/lease=NONE/monitor=exited/publisher_orphan=NONE）→ `RF_SRC_RTMP_01_BMD_SOURCE_RECOVERY_PASS`。
- **Tier 1 wording**: “non-loopback listener binding verified”（同宿绑定+同宿 publish；不构成跨主机声明）。

### Tier 2 — independent network namespace third-party push (commit 58fd33d)

- Env: 同上 + `VBMF_FFMPEG_RTMP_SOURCE_EXTERNAL=1`，URL `rtmp://10.30.15.10:19351/live/tier2`，HLS `/tmp/tg6-tier2-hls`。
- Publisher: BMD 上 docker 容器 `mwader/static-ffmpeg:7.1`（默认 bridge = 独立 network namespace，容器 IP 172.17.0.2）；宿主 UFW 默认 DROP，临时加规则 `allow proto tcp from 172.16.0.0/12 to any port 19351`（仅本验收窗口，**验收后已删除**，`ufw status` 复核仅剩既有 SSH 规则）。
- Cross-origin 证据（运行期 ss 采样）: `ESTAB 0 0 10.30.15.10:19351 172.17.0.2:48788`。
- 结果（tier2-gate.log）: source PASS（listener=lan, signal_verified=true — 独立 netns 推流经生产 listener 产出已验证 A/V）→ external-leg 断连（docker stop -t 1）→ recovery PASS（old 3460834 → new 3461243, attributed=PublisherDisconnected, supervisor=Running）→ 二次外部推流 A/V 恢复 → teardown PASS → `RF_SRC_RTMP_01_BMD_SOURCE_RECOVERY_PASS`。
- Gate manifests（Tier 2 各次运行，同 URL 派生 source_id=293a9db2-…）MD5 全部一致 `cdd8d08ee290045e1647ec2405d9c5d7`（确定性）。
- **Tier 2 wording**: “cross-host third-party push verified”（独立 network namespace 对 BMD LAN 地址的真实第三方推流；frozen plan §3 允许 “independent host OR independent network namespace”）。注：Development VM（10.30.5.80）→BMD 非特权端口被网络边界阻断（TCP probe failed），故采用 netns 路径；peer 地址 172.17.0.2 为 docker bridge 源，非物理远端 IP。

## Orchestration record

- Tier 2 编排脚本副本: `tier2-orchestration.sh`（本目录；在 BMD 执行）。
- 失败重试记录（如实）: Tier 2 前两次尝试失败——(1) VM docker 推流被 UFW/网络边界阻断（无 TCP 建立）；(2) BMD 容器推流首次成功连接后，编排时序（docker stop 默认 10s 优雅期 > gate 6s 恢复窗）导致恢复窗过期；第三次（本 manifest 记录的 gate4 运行）全绿。中间诊断运行（gate2）证明了断连→新 consumer 生成路径本身正常。

## Boundary integrity

- 输出设备 device-number 2 进程 PID 992634（gst-launch）全程存活：验收前 15-07:09:51 → 验收后 15-07:46:40，未触碰。
- 验收后 BMD 零 ffmpeg 残留（`pgrep -x ffmpeg` = 0）；UFW 临时规则已删除。
- DeckLink SDK 侧仅 gate 根既有 discovery（3 设备枚举，只读）；无 input 0/1 打开、无 output 打开（network-only 组合零设备副作用 + teardown 全绿断言）。

## Deferred risks annotation (frozen plan §3, 逐项如实)

- RTMP handshake timeout（空闲/恶意半连接占用唯一 listener slot）：本轮正常运行窗口未复现占用问题；BMD ffmpeg 构建未观察到 post-accept 握手/空闲超时选项（TG-0 结论维持）——**未观察到限制，仍为 deferred risk**。
- 非授权 publisher 占用唯一 `(protocol, ip, port)` listener：本轮 Tier 2 的 netns publisher 即"任意能连者可推"的直接演示（D4：path 非认证，已知接受风险；manifest 五元组仅约束监听地址/端口/标签）。
- 高速 stderr 排空成本：D9 ring 有界（64KiB/256 行/512B）；本轮无 stderr 风暴场景，未测极限——维持登记。
- 进程退出顺序（kill/wait/reader join/socket 释放）：teardown 断言（monitor=exited、publisher_orphan=NONE、无 ffmpeg 残留）在本轮全绿；TG-3 单测已覆盖 join 顺序。
- 主机防火墙：本轮 Tier 2 曾按部署建议临时放行容器源段+单端口并已回收——防火墙仅为部署建议，不替代 VBMF 内部授权（frozen wording 维持）。
