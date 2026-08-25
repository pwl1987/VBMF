# BMD 验收服务器 — 环境准备证据

> **G-RUNTIME 第二级（Deployment SoT §9）**：远程 BMD 服务器环境准备记录。
> 本文档是真实 Blackmagic Design（BMD）服务器上环境准备的可审计记录，符合"环境预检（ENV Preflight）+ 验收证据"要求。
> **注意：SHA / 主机标识属于验收证据，不是密件，不得涂销。**

## 0. 证据标识（EVID-01 / EVID-02 修正）

> 为防止"环境已准备"被误读为"当前 commit 已远程验收"，本证据严格分离两个 SHA：
> - **`environment_base_sha`**：本机环境所基于的仓库提交（即当时 checkout 并验证的源码状态）。
> - **`test_subject_sha`**：本证据实际证明其结论的提交。
> 两者在本文件一致；后续任何 Runtime Acceptance（FI-08/FI-09）证据须单独记录，且 `test_subject_sha` 须等于被测提交。

| 字段 | 值 |
|---|---|
| 证据类型 | Environment Prep Evidence（环境准备，**非** Runtime Acceptance） |
| `environment_base_sha` | `7cc33dde2ab3070c28087df7d0aae570c6c8df5f`（`7cc33dd`） |
| `test_subject_sha` | `7cc33dde2ab3070c28087df7d0aae570c6c8df5f`（`7cc33dd`） |
| 编写提交（document commit） | `b427dd8`（仅改写本证据文本为中文，未改变被测对象） |

⚠️ **边界声明**：本证据仅证明 `7cc33dd` 环境下的 Docker / DeckLink / Compose 配置可用。**不等同**于 `b427dd8` 或任何后续提交已通过远程验收。Runtime Acceptance 证据将独立成文件（`acceptance-*.md`）。

## 1. 目标机器

| 项目 | 值 |
|---|---|
| 主机地址 | `10.30.15.10` |
| SSH 登录用户 | `lytv`（密钥认证，sudo 免密） |
| 操作系统 | Ubuntu 26.04 LTS（代号 resolute） |
| 内核 | `7.0.0-30-generic #30-Ubuntu SMP PREEMPT_DYNAMIC` |
| 架构 | x86_64 |
| `environment_base_sha` | `7cc33dde2ab3070c28087df7d0aae570c6c8df5f`（简称 `7cc33dd`） |
| 仓库路径 | `/opt/vbmf-dev/repo`（按 §9 要求精确 checkout 该 SHA） |
| 工作目录布局 | `/opt/vbmf-dev/{repo,evidence,artifacts,logs,runtime}` |

## 2. Blackmagic DeckLink 设备探测（F11 前置条件）

**步骤**：登录后检查 `/dev/blackmagic` 设备节点。

**结果**：
```text
/dev/blackmagic:
  crw-rw-rw- 1 root root 10, 263  dv0
  crw-rw-rw- 1 root root 10, 264  dv1
  crw-rw-rw- 1 root root 10, 265  io0
```

✅ **确认存在真实 BMD 硬件**（`dv0/dv1/io0` 三个 DeckLink 设备节点）。说明这台机器是合格的 G-RUNTIME 第二级验收目标。设备透传路径 `/dev/blackmagic` 与 `media-agent` 在 compose 中的 `devices:` 映射一致。

## 3. Docker 安装（动作日志）

**步骤一：发现网络限制**
- 外发 HTTPS 访问 `get.docker.com` / `download.docker.com` / `github.com`（HTTPS）均被**出口过滤重置**（裸 TCP/443 能连通，但 TLS 握手被 reset）。
- Ubuntu 26.04 的 universe 软件源中没有 `docker.io` 包。
- 结论：官方安装脚本与官方 apt 源都不可用，必须换国内镜像。

**步骤二：改用国内镜像安装**
使用 `linuxmirrors.cn` 安装脚本（其 CDN 可达），并指定阿里云 Docker CE 镜像源：

```bash
bash <(curl -sSL https://linuxmirrors.cn/docker.sh) \
  --source mirrors.aliyun.com/docker-ce \
  --source-registry registry.cn-hangzhou.aliyuncs.com
sudo systemctl enable --now docker
sudo usermod -aG docker lytv
```

**步骤三：确认安装结果**
```text
Docker version 29.7.2（Server 与 Client 均为 29.7.2）
Docker Compose version v5.5.0
Default Runtime = runc   Driver = overlayfs   Cgroup = v2
```

✅ **Docker 引擎 + Compose 插件安装成功，守护进程已运行。**

> **MEDIA-SEC-01 备注**：当前默认运行时是 `runc`，**不是** `runsc`（gVisor）。
> compose 中 `media-agent.runtime: runsc` 已指定，但本机尚未安装 runsc。
> 方案 A（gVisor）需安装 runsc 并做 DeckLink 实测；
> 方案 B（runc + seccomp/AppArmor）是当前可用路径。
> 最终选择推迟到真实媒体运行时验收（Gate 2/3）由真机裁定。

## 4. Compose 校验（DEPLOY Gate 1 验证）

**步骤**：在 `/opt/vbmf-dev/repo` 下，先 `git checkout 7cc33dd`，并生成本地 `.env`（密钥已外置，用 `openssl rand` 生成，**未提交入库**），然后执行四套 compose 配置校验：

```bash
for p in "" "compose.dev.yml" "compose.acceptance.yml" "compose.prod.yml"; do
  docker compose -f ops/docker-compose.yml ${p:+-f ops/$p} config --quiet \
    && echo "OK: $p" || echo "FAIL: $p"
done
```

**结果**：

| 配置组合 | 结果 |
|---|---|
| 基础（`docker-compose.yml`） | ✅ OK |
| 开发（`+compose.dev.yml`） | ✅ OK |
| 验收（`+compose.acceptance.yml`） | ✅ OK |
| 生产（`+compose.prod.yml`） | ✅ OK |

✅ **四套分层 compose 配置全部通过 `docker compose config --quiet` 语法校验。**
密钥外置机制（INFRA-SEC-01）已验证：缺少 `.env` 时 compose 会中止插值并报错，不会使用硬编码密码。

## 5. 遗留事项（不阻塞环境准备）

- [ ] `media-agent` 的 Dockerfile 仍是占位文件 → `docker compose up` 会卡在 build 阶段，需 Phase 1 源码后才能构建（Gate 2）。
- [ ] `runsc`（gVisor）尚未安装 → MEDIA-SEC-01 方案 A 未验证；当前可用路径为 runc。
- [ ] 本机 GitHub HTTPS 出口被封 → 仓库同步改用从开发机 `scp`，而非 `git pull`。
- [ ] `.env` 仅存在于 BMD 本地文件系统（已被 gitignore），绝不提交入库。

## 6. 签核

| 角色 | 身份 | 日期 |
|---|---|---|
| 环境准备执行人 | AI 助手（经 SSH，用户 `lytv`） | 2026-08-25 |
| 已验证 SHA | `7cc33dd` | — |
| 下一关 | Gate 2：真实 Rust Media Agent 构建 + 设备租约（需 Phase 1 源码） | — |
