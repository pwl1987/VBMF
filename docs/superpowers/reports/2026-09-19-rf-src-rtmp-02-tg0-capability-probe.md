# RF-SRC-RTMP-02 TG-0 Capability Probe — Verify Report

- Date: 2026-09-19
- Packet: `RF-SRC-RTMP-02-IMPLEMENTATION` Step 0 (TG-0)
- Design authority: `docs/superpowers/plans/2026-09-19-rf-src-rtmp-02-production-rtmp-input-boundary.md` @ `bf0a8e0` (local = `origin/main` = GitHub live `main`, three-way verified before probe)
- Evidence: `evidence/bmd-10.30.15.10/2026-09-19-rf-src-rtmp-02-tg0-capability-probe/` (39 files incl. `tg0-manifest.md`)
- Verdict: **TG-0 PASS**；实现包 Step 1（TG-1）解锁。

## 1. 环境核对（要求 1）

| 项 | 结果 |
|---|---|
| exact commit | `bf0a8e0`，本地/origin/GitHub live 三方一致；探针不依赖盒上 repo |
| BMD 部署现状 | 无 VBMF `media-agent` 服务运行；旧 `/opt/vbmf-dev/repo` detached `7cc33dd`（STATE 风险 #4 既有事实，未动）；`/dev/blackmagic` dv0/dv1/io0 在位 |
| BMD 网络 | eno1 `10.30.15.10/16`（非 loopback 全局地址） |
| FFmpeg | `/usr/local/bin/ffmpeg` `git-2026-08-23-1019f8f-+allcodec-20260824`；`rtmp` 在 Input/Output 协议表（与 §3.45 记录一致） |
| device-2 输出 | 历史 `gst-launch … decklinkvideosink device-number=2` PID `992634` 探针前后存活、命令未变（仅观察） |

## 2. 探针① — `-rtmp_listen` 显式非 loopback LAN 绑定：**PASS**（要求 2）

- 监听 URL `rtmp://10.30.15.10:19352/live/probe`；`ss -tln` 实证 `LISTEN 10.30.15.10:19352` —— **绑定显式 LAN 地址，非通配 0.0.0.0、非 loopback**（这是一个对 D2/D5 有利的实证：URL host 即 OS bind 地址）。
- 同机 publisher（lavfi testsrc2+sine → H.264+AAC/flv，3s）推流成功：`publisher_rc=0`、`listener_rc=0`、mpegts 输出 244776 字节、video+audio 双流 muxed。
- 说明：首轮（端口 19351）已证同样实质（60 video + 174 audio 包），但其 rc 记账被 publisher 缺 `-nostdin` 的 stdin 吞脚本问题污染，已用 `-nostdin` 干净重跑为 `p1clean`；首轮原始日志按原样保留在证据目录。生产 adapter argv 本就含 `-nostdin`，与设计一致。

## 3. 探针② — path 强制校验负向矩阵：**"path is a routing label only"**（要求 3）

监听方固定声明 `/live/probe`，publisher 侧变体（各自隔离端口 19361-19365）：

| 变体 | publisher path | publisher_rc / listener_rc | 输出 md5 |
|---|---|---|---|
| 正确 path | `/live/probe` | 0 / 0 | `86cd3c82…` |
| 错误 path | `/live/other` | 0 / 0 | `86cd3c82…` |
| 尾部 `/` | `/live/probe/` | 0 / 0 | `86cd3c82…` |
| query | `/live/probe?x=1` | 0 / 0 | `86cd3c82…` |
| 编码变体 | `/live/pro%62e` | 0 / 0 | `86cd3c82…` |

**五个变体全部被接受，输出字节完全一致（md5 相同）**：BMD FFmpeg `-rtmp_listen 1` 不校验来连方的 app/play path。按冻结设计 D4 产品裁决：**"path is a routing label only"** —— 不阻塞实现包（默认接受降级声明），但 evidence/UI/日志文案永不称 "path 认证"；若未来产品要求 path 级 publisher 隔离，须回 PLAN REQUIRED。

## 4. 探针③ — `source_id` wire 归属：**PASS**（要求 4）

exact `bf0a8e0` 上 `SourceIntent::Rtmp { source_id: NetworkSourceId, endpoint: NetworkEndpoint }`（`graph_intent.rs`）——授权使用既有外层 Source 身份，无需新增 wire 字段。

## 5. 隔离与副作用（要求 5）

- 全部 fixture 仅用隔离端口 19351/19352/19361-19365 与 lavfi 合成媒体；**无任何 DeckLink 参数**、未触碰 `/dev/blackmagic`。
- 探针前后 `ss` 快照对比：无新增监听残留；`pgrep ffmpeg` = NONE（零进程泄漏）。
- device-2 输出进程 PID `992634` before/after 不变。
- 盒上临时目录 `/tmp/tg0-rf-src-rtmp-02` 已在证据回拉后删除。
- 未修改任何 runtime 代码；未启动任何 VBMF runtime（本轮零代码，docs/evidence/STATE only）。

## 6. Deferred 项登记（要求 6，转入 STATE §9）

- RTMP 侧存在 `-timeout`（"Maximum timeout (in seconds) to wait for incoming connections… Implies -rtmp_listen 1"）→ **publisher 等待期可设上限**，实现包可评估采用（不改变 D7 Waiting 非故障语义）。
- **未观察到** RTMP post-accept 握手/空闲连接超时选项 → 既有 deferred ingress risk（半连接占用唯一 listener）**维持登记**；按冻结规则不阻塞实现包，BMD 验收报告须逐项标注。
- 二进制 mpegts 输出未入仓库（工具安全扫描拦截二进制 `*.ts` 写入）；md5/字节数作为完整性记录固化在 `tg0-manifest.md`（五变体同 md5 即"path 不校验"的核心证据，不依赖二进制本体在库）。

## 7. 结论与下一步（要求 7）

**TG-0 PASS**：探针① PASS；探针② 裁定 "path is a routing label only"（D4 默认裁决下可采纳，文案降级强制）；探针③ PASS。

下一步 = 实现包 **Step 1 / TG-1**：endpoint 规范化 parser + `NetworkSourceBinding` manifest 加载器（含 INV-3 严格解析与防竞态加载），严格按冻结设计 allowed scope 与验收矩阵推进。
