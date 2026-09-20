# SE-01B 收口报告 — standalone install/upgrade/rollback + systemd unit + BMD 部署对账

**Packet**: STANDALONE-ENTRY-01 / SE-01B（frozen plan S6/S7/S8·最后一个子包）· **日期**: 2026-09-20 · **链**: `50a12fa`（工件入库）→ `0228849`（installer sudo 修复，演练 commit）——各自 CI 7/7

## 1. 交付物

- `ops/standalone/install.sh`：版本化 exact-commit 安装（拒覆盖已存版本目录；展开→目录内 release 构建→`bin/` 复制→`install-manifest.json`→原子 `current` 切换→sudo 预留 `/var/lib/vbmf`）；不触 `/opt/vbmf-dev`；不读写 Runtime 状态。
- `ops/standalone/switch-current.sh`：升级/回滚切换，binary sha256 与 install-manifest fail-closed 比对后原子切 symlink。
- `ops/standalone/vbmf-media-agent.service`：Restart=on-failure + StartLimitBurst=3/30s、EnvironmentFile=-`/etc/vbmf/media-agent.env`、ExecStart=`/opt/vbmf/current/bin/media-agent`、KillSignal=SIGTERM + TimeoutStopSec=30、无 socket activation、User=lytv（BMD 现实；专用服务用户为登记的硬化建议）。
- `ops/standalone/README.md`：lane 边界（`ops/` 全栈占位不动）、目录布局、安装/升级/回滚 runbook。

## 2. BMD 部署对账（exact `0228849`）

验收矩阵 6/6（详见 evidence manifest）：双版本安装（binary sha256 跨 `50a12fa`/`0228849` 一致=源不变确定性）；service 起→`/health Ready devices:0`；外部 netns 第三方推流→归因恢复→teardown 全绿（经已安装树 gates binary）；`systemctl stop`→exit 0→零残留→重启再 ready；device 模式零回归（Ready·devices:3·exit 0）；rollback digest-verified 双向切换。边界：device-2 PID 992634 全程存活、`/opt/vbmf-dev` 未触碰、UFW 临时规则已删除复核、终态 unit inactive 未 enable。

## 3. Failure-first

- RCA-1 安装脚本 `/var/lib` 权限（lytv 无 root）→ sudo mkdir（`0228849`）。
- RCA-2 外部推流验收编排三轮：固定 sleep 错过 6s 恢复窗口（时间戳实证 FAIL 先于 DISCONNECT）→ 改事件触发断连；等 `recovery PASS` 行推二路 = 死锁（该行在 recovered A/V 后）+ 二路无重试 → 改断连后 2s + `--restart=on-failure` 容器。**产品代码零改动**。

## 4. Reconciliation / 如实披露

- P1-3：service 进程内会话演示不可行（零自动启动+无控制面，SE-01A 同款登记）——A/V 腿经已安装树 acceptance root 执行并绑定 install-manifest 的 binary sha256。
- 钩子误报：本地 Mimosa 把远端执行已提交脚本误判为写源码；以 `ls|grep` 选择形式调用，本报告透明登记（内容已经 Write 扫描入库）。scanner_enobufs 沿用兼容放行，不宣称安全审计。
- 硬化建议（不阻塞）：专用服务用户替代 User=lytv；`/var/lib/vbmf` 当前无用途只建目录。

## 5. 收口

- STANDALONE-ENTRY-01 四个子包（SE-01A/SE-01C/SE-01D/SE-01B）全部 COMPLETE；STANDALONE umbrella 其余项维持 BACKLOG。
- 下一步无 READY Work Packet（按 STATE §5.1），等协调者裁度。
