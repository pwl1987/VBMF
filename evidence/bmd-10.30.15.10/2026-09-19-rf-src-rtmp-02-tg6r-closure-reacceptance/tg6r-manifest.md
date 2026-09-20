# RF-SRC-RTMP-02 TG-6R Closure Re-acceptance Manifest（RF-SRC-RTMP-02-CLOSURE-RECONCILIATION）

- Date: 2026-09-19 (BMD local 23:3x, UTC+8)
- Host: BMD box `lytv` @ 10.30.15.10 (ssh via Development VM)
- Toolchain: cargo 1.98.0 (797e8a9bc 2026-08-05) on BMD; ffmpeg git-2026-08-23-1019f8f (/usr/local/bin)
- Packet: RF-SRC-RTMP-02-CLOSURE-RECONCILIATION（STATE §3.59；前轮缺口 = production-root network-only 可达性 + gate 先经 bootstrap 污染 D10）

## Exact commit / artifacts（全部三段 + 采样运行绑定同一 implementation SHA）

| Artifact | Commit | SHA-256 |
|---|---|---|
| Source archive | `d13f1fd`（CI 35487255383，7/7 required PASS） | `7f242a2501e3b7e8dbc8ba5d3c92cb2cf762706d75a9f2f3b319a9edc056b3d2` |
| Gates binary（`--features bmd,ffmpeg-backend`，dev profile，盒上原生构建） | built from above | `3d784e2af95b0e0b66aff24cdc57a9972bf0d4d9461b941bf28e0f6d12c67855` |

Build: `DECKLINK_SDK_INCLUDE=/home/lytv/decklink-sdk-include cargo build --features bmd,ffmpeg-backend --bin media-agent-gates`，in-tree at `/tmp/vbmf-tg6r-d13f1fd`。

## 本轮修复后的 gate 进程形态（严格 D10 前提）

`src/bin/gates.rs` 现在把 `VBMF_FFMPEG_RTMP_SOURCE` dispatch 放在 common `bootstrap::build()` **之前**；本轮全部 4 次 gate 运行的进程日志中：

- `device discovery complete`：**0 行**
- bootstrap `lease acquired`：**0 行**
- `adapter selection`（provider 构建）：**0 行**
- `DeckLinkAPI` SDK probe：**0 行**

（对照：上轮 TG-6 同一 gate 的日志为 discovery count=3 + 3 条占位租约——见 §3.59 缺口记录。）

## Runs

### Leg 1 — loopback 回归（RF-SRC-RTMP-01 gate 形态）

- Env: `VBMF_MACHINE_ID=bmd-10-30-15-10 VBMF_FFMPEG_RTMP_SOURCE=1 VBMF_FFMPEG_RTMP_SOURCE_URL=rtmp://127.0.0.1:19350/live/loopback VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR=/tmp/tg6r-loopback-hls`
- rc=0；`tg6r-loopback-gate.log`（md5 `e7b81ac347958b7572b4ef58cf12b4a3`）：
  - `RF-SRC-RTMP-02 D10 startup PASS device_discovery=0 bootstrap_device_leases=0 device_resources=0 network_resources=1 manifest_bytes=159`
  - source PASS（listener=loopback, signal_verified=true, h264+aac）
  - recovery PASS（old 3473648 → new 3473845, attributed=PublisherDisconnected, supervisor=Running）
  - teardown PASS + `RF-SRC-RTMP-02 D10 teardown PASS … manifest_bytes_unchanged=true`
  - `RF_SRC_RTMP_01_BMD_SOURCE_RECOVERY_PASS`

### Leg 2 — Tier 1 LAN（同宿绑定 + 同宿 publish）

- Env: 同上 + `VBMF_FFMPEG_RTMP_SOURCE_LAN=1`，URL `rtmp://10.30.15.10:19350/live/source`，HLS `/tmp/tg6r-tier1-hls`
- rc=0；`tg6r-tier1-gate.log`（md5 `78692eadfb77bcb0fc617bc8319cf0aa`）：D10 startup/teardown PASS、source PASS（listener=lan）、recovery PASS（3474113→3474312）、teardown 全绿
- gate manifest MD5 `29570aa7ac8a89aa2ebc7ba9227d4fc1`（159 B，与上轮 TG-6 Tier 1 完全一致=确定性；本轮另由 gate 内字节级断言证明运行前后不变）
- **Tier 1 wording**: "non-loopback listener binding verified"

### Leg 3 — Tier 2 independent network namespace 第三方推流

- Env: 同 Tier 1 + `VBMF_FFMPEG_RTMP_SOURCE_EXTERNAL=1`，URL `rtmp://10.30.15.10:19351/live/tier2`，HLS `/tmp/tg6r-tier2-hls`
- UFW 临时规则（验收窗口内）: `allow proto tcp from 172.16.0.0/12 to any port 19351 comment tg6r-temp`——**验收后已删除并复核**（status 仅剩既有 SSH 规则）；该规则仅为容器推流放行的部署建议，不构成 VBMF authorization（frozen 口径）
- Publisher: docker `mwader/static-ffmpeg:7.1`（默认 bridge = 独立 netns，容器 IP 172.17.0.2）
- Cross-origin 证据（运行期 ss 采样）: `ESTAB 0 0 10.30.15.10:19351 172.17.0.2:52228`
- rc=0；`tg6r-tier2-gate.log`（md5 `93979db69537a921fdcc8211781d10b5`）：D10 startup/teardown PASS、source PASS（listener=lan, signal_verified=true——外部推流经生产 listener 产出已验证 A/V）、`external-leg A/V verified`、断连（docker stop -t 1）→ recovery PASS（3474624→3475042, attributed=PublisherDisconnected, supervisor=Running）→ 二次外部推流（gate 内 recovered-A/V 验证通过）→ teardown 全绿
- **Tier 2 wording**: "cross-host third-party push verified"（independent network namespace 形态，frozen plan §3 允许；Development VM 直推仍被网络边界阻断——沿用上轮事实，本轮未重试 VM 直推）

### 采样运行 — D6 listener 显式绑定直接证据

- 专门运行（Tier 1 端口，rc=0，`tg6r-listen-sample.log` md5 `d9c0916f8dc7cce4f646f553874c891e`）：
  - `LISTEN 0 1 10.30.15.10:19350 0.0.0.0:* users:(("ffmpeg",pid=3475669,fd=3))`——显式非 loopback LAN 绑定（非通配）
  - `ESTAB 10.30.15.10:55804 → 10.30.15.10:19350`（同宿 publisher 连接）
  - 该运行本身完整通过（D10 teardown PASS + 最终 marker）

## D10 断言面（本轮新增的 gate 内机械证据，4 次运行全部出现）

- startup：`device_discovery=0 bootstrap_device_leases=0 device_resources=0 network_resources=1`（composition runtime_state 直接断言，非仅日志缺席）
- teardown：`ffmpeg_child=none listener=released-with-child stderr_reader=joined-by-reaper monitor=exited network_lease=none network_resource=available manifest_bytes_unchanged=true`（字节级比较，含 load/start/recover/stop 全程）

## Boundary integrity

- 输出设备 device-number 2 进程 PID 992634 全程存活：before `15-20:47:43` → after `15-20:51:11`，命令未变，未触碰
- 验收后零 ffmpeg 残留（`pgrep -x ffmpeg` = 0）；1935x 无 listener 残留；docker 容器已清理；UFW 仅剩既有 SSH 规则
- input 0/1：本轮 gate 进程从未执行 DeckLink discovery/open（0 行证据 + gate 结构性前置 dispatch），无打开/状态变化

## Deferred risks（frozen plan §3，逐项沿用上轮登记）

- RTMP 握手/空闲连接超时：BMD ffmpeg 仍未观察到 post-accept 超时选项——**未观察到限制，维持登记**
- 非授权 publisher 占用唯一 listener：Tier 2 即演示（D4 已知接受，path 非认证）
- 高速 stderr 风暴：D9 ring 有界，本轮未测极限——维持登记
- 主机防火墙仅为部署建议（本轮临时规则已回收）

## 如实披露

- 上轮 TG-6（ee856d4/58fd33d）的两个 gate revision 进程先执行 bootstrap discovery/占位租约，其"严格 D10"结论作废（§3.59）；其 listener/A/V/归因恢复/teardown 事实保留。本轮 4 次运行全部在新 implementation commit `d13f1fd` 上完成（Tier 1 与 Tier 2 绑定同一 SHA）。
- Tier 2 的 PEER2 ss 采样时间点在 TCP 建立前，未捕获行；二次外部推流的 A/V 由 gate 内 `wait_for_hls`（recovered RTMP source A/V）验证并作为 recovery PASS 前置，未以 ss 行作为证据。
- 开发 VM 侧 Mimosa 提交钩子在两次 commit 时未完成完整扫描（scanner_enobufs，兼容策略放行）；对本次触碰文件曾单独执行 normal 聚焦扫描（findingCount=0）。本 evidence 不宣称完整项目安全审计。
