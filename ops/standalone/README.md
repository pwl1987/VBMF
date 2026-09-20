# VBMF Standalone Lane — 部署工件（STANDALONE-ENTRY-01 / SE-01B）

> **Lane 边界**: 本目录是 standalone lane（单 `media-agent` + systemd + `/etc/vbmf` + `/health`）的部署工件。
> `ops/` 既有 compose/Dockerfile/nginx 为 V0.2 全栈形态占位，**本 lane 不修改、不依赖它们**（compatible, not dependent）。
> 语义权威: [`docs/architecture/STANDALONE_MEDIA_AGENT_OPERATIONS.md`](../../docs/architecture/STANDALONE_MEDIA_AGENT_OPERATIONS.md)；
> 部署 lane 增补: Deployment SoT §17。

## 工件

| 文件 | 作用 |
|---|---|
| `install.sh` | 版本化安装：展开 exact-commit archive → 版本目录内 release 构建 → `bin/` 复制 → `install-manifest.json` → 原子切 `current` |
| `switch-current.sh` | 版本切换/回滚：digest 复核（fail-closed）后原子切 `current` |
| `vbmf-media-agent.service` | systemd 单元（Restart=on-failure + StartLimitBurst；EnvironmentFile；退出码契约见语义页 §3） |

## 目录布局（frozen S6/S7）

```
/opt/vbmf/<version>-<short-sha>/     # 版本目录（只增不改；禁止原地 mutation 升级）
  ├── services/media-agent/...        # 展开的 exact-commit 源（构建现场保留）
  ├── bin/media-agent                 # release binary（sha256 记录于 install-manifest）
  ├── bin/media-agent-gates           # acceptance root（真机验收入口）
  └── install-manifest.json           # tag / archive sha256 / binary sha256 / features / ci-run-id / 时间
/opt/vbmf/current -> <版本目录>       # 原子 symlink（tmp + mv -T）；rollback = switch-current.sh 指旧版本
/etc/vbmf/media-agent.env             # EnvironmentFile（`-` 前缀 = 缺失容忍）
/etc/vbmf/network-binding.json        # NetworkSourceBinding（0600，machine-pin；SE-01D 起默认 pin /etc/machine-id）
/var/lib/vbmf/                        # 预留（Runtime 当前无持久状态；只建立不伪造用途）
```

- **禁止** floating ref 部署；**禁止**部署无 `install-manifest.json` 的目录。
- 旧 `/opt/vbmf-dev` 树与本 lane 并存、只读保留，升级/回滚均不触碰。

## 安装 / 升级 / 回滚

```bash
# 安装（exact commit archive 在本机；脚本一律经 bash 调用）
bash ops/standalone/install.sh /tmp/vbmf-<sha>.tar.gz 0.1.0-<shortsha> <ci-run-id>

# 升级 = 对新版本 tag 再跑 install.sh（旧版本目录原样保留）

# 回滚 = digest 复核后切回旧版本
bash ops/standalone/switch-current.sh 0.1.0-<旧shortsha>

# systemd
sudo cp ops/standalone/vbmf-media-agent.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl start vbmf-media-agent        # 验收演练不 enable 常开，由运维决定
```

## 运维要点

- **readiness**：轮询 `GET /health` 至 `state ∈ {Ready, Capturing}`（HTTP 200 本身不是 ready；语义页 §2）。
- **退出码**：`0` = 优雅停止；`2` = fail-closed 启动拒绝（修 env/manifest 前重启无意义）。
- **身份**：SE-01D 起默认 `/etc/machine-id`（`VBMF_MACHINE_ID` 显式覆盖）；旧 HOSTNAME 风格 pin 的清单必须重 pin。
- **日志**：stdout/stderr → journald（`journalctl -u vbmf-media-agent`）；`RUST_LOG` 经 env file 透传。
- **硬化建议（不阻塞 lane）**：专用服务用户 + 最小组替代 `User=lytv`（BMD 设备访问按现实经用户组/udev）；本验收按 BMD 现实以 lytv 运行并如实登记。
- **Runtime owns truth**：全部工件只做 启动/停止/传 env/指向 manifests；健康与状态判断只经 `/health` 与进程退出码，绝不旁路 SessionManager/LeaseManager。
