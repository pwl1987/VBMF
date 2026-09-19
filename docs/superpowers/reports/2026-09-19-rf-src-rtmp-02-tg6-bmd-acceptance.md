# RF-SRC-RTMP-02 TG-6 — BMD exact-commit LAN Acceptance（Tier 1 + Tier 2）

- Date: 2026-09-19
- Packet: `RF-SRC-RTMP-02-IMPLEMENTATION` Step 6 / TG-6（前置：TG-5 `b605284`）
- Design authority: `docs/superpowers/plans/2026-09-19-rf-src-rtmp-02-production-rtmp-input-boundary.md`（§3 evidence levels + deferred risks）
- Evidence: `evidence/bmd-10.30.15.10/2026-09-19-rf-src-rtmp-02-tg6-lan-acceptance/`（tier1/tier2 gate 日志 + tg6-manifest.md + tier2-orchestration.sh）

## 1. 声明（按冻结 evidence levels 措辞）

- **Tier 1（PASS, commit `ee856d4`, CI 35448354450 7/7）**："non-loopback listener binding verified" — 生产 network-only 组合在 BMD 以 `-rtmp_listen 1` 显式绑定 LAN 地址 `10.30.15.10:19350`（ss 采样 `LISTEN 10.30.15.10:19350`），同宿 publisher 推流产出已验证 h264+aac，断连经 INV-1 归因 `PublisherDisconnected` 预算化恢复（新 consumer PID），teardown 全绿（Released/Available/lease=NONE/monitor=exited/无孤儿）。
- **Tier 2（PASS, commit `58fd33d`, CI 35448724626 7/7）**："cross-host third-party push verified"（frozen §3 允许的 independent network namespace 形态）— BMD 上 docker 容器（独立 netns，源 172.17.0.2）对 BMD LAN 地址 `10.30.15.10:19351` 真实第三方推流（ss 采样 `ESTAB … 172.17.0.2:48788`），经生产 listener 产出已验证 A/V；断连→归因恢复→二次外部推流→teardown 全绿。Development VM（10.30.5.80）直推路径被网络边界阻断（非特权端口 TCP probe 失败，如实记录），故未采用独立物理主机形态。

## 2. 生产链路实机验证面（两个 Tier 共同覆盖）

- D2/D3/D5：canonical LAN endpoint 准入（manifest 0600 + machine-pin（VBMF_MACHINE_ID）+ D5 本机地址归属经 BMD getifaddrs）——gate 自写 manifest 加载即全链路。
- D6：bind() 权威——显式 LAN 地址绑定（非通配）。
- D7/INV-1：`signal_verified=true` → 断连 → `attributed=PublisherDisconnected` → 预算化重启；`supervisor=Running`（restart-completed，D8 预算不因重建 listener 重置）。
- D9：退出归因仅类别名；argv/manifest 为显式例外。
- D10：network-only 组合零设备副作用 + teardown（资源回 Available/lease 释放/monitor 退出/无 ffmpeg 残留）；gate manifest digest 运行前后不变（Tier 1 `29570aa7…`；Tier 2 同 URL 各次运行 `cdd8d08e…` 完全一致=确定性）。
- 边界完整性：输出 device-number 2 进程 PID 992634 全程存活（15-07:09:51 → 15-07:46:40）未触碰；验收后零 ffmpeg 残留；UFW 临时规则（容器源段+19351 单端口，仅为容器推流放行）已删除并复核。

## 3. 过程如实披露

- Tier 2 前两次尝试失败：(1) VM docker 推流无 TCP 建立（UFW default-DROP + 网络边界）；(2) BMD 容器推流成功后编排时序失误（docker stop 默认 10s 优雅期 > gate 6s 恢复观察窗）导致 gate 判"无新一代"；第三次（gate4，证据主体）全绿。诊断运行另证断连→新 consumer 路径正常。
- 首次 archive 曾从子目录打包（`6a0f6b5f…` 无效，已在 manifest 标注 superseded）；有效 archive `7b96e2aa…`（ee856d4）。
- BMD /tmp 配额一度耗尽（旧会话构建残留），清理历史构建树后重建（证据均在 git，无损失）。

## 4. Deferred risks 逐项标注（frozen plan §3）

见 `tg6-manifest.md` §Deferred risks：握手超时未观察到限制（维持登记）；唯一 listener 可被任意能连者占用（Tier 2 即演示，D4 已知接受风险）；stderr 风暴极限未测（ring 有界）；退出顺序全绿；防火墙仅为部署建议（本轮临时规则已回收）。

## 5. 软件面（TG-6 预备提交）

- `145908b`（gate LAN fixture canonical 放宽）、`fc3eada`（dead wrapper 清理）、`ee856d4`（SignalVerified 证据接线 + 归因/消息如实化）、`58fd33d`（external 第三方推流模式）——各 commit CI 7/7 required PASS；本地矩阵（default/simulation/ffmpeg-backend/mock + clippy×3 + fmt + check + arch lint + remove-adapters + diff-check）全绿。

## 6. 收口

RF-SRC-RTMP-02 实现包全部 touch-gate（TG-0…TG-6）通过：软件验证 + BMD 硬件验证（Tier 1 + Tier 2）+ 证据归档 + STATE 收口。无 frozen stop condition 触发；未扩大任何边界。
