# CI Runner Strategy（自托管 GitHub Actions Runner 基础设施契约）

> **文档身份**: VBMF 自托管 CI Runner **基础设施 SoT**（Phase 1 v1.2 冻结版）。
> **生成**: 2026-09-11，基线 master `ff2c481`（`media-agent.yml` blob `5bf1c0e`）。
> **与既有 SoT 关系**: 本文件只管「Runner 基础设施层」；部署/验收分层仍以
> [`DEPLOYMENT_AND_DEV_RUNTIME.md`](DEPLOYMENT_AND_DEV_RUNTIME.md) §9（两级验收）为准。
> 本文件**不改变任何现有 CI 门禁**。
> **裁决历史**: v1.0（外部方案）→ v1.1 → v1.2 冻结（两轮评审共 17 条收紧全部并入；不再迭代 v1.3）。

---

## 0. Phase 1 定位

> 一句话：建立一台**可重复部署、可审计、可回滚、权限边界清晰**的 self-hosted Runner，
> 并证明现有 GitHub-hosted CI 完全未受影响。

**不是把现有 CI 搬到 self-hosted。** Phase 1 完成后才有资格讨论 Phase 2（General CI Migration）。

### 0.1 Phase 1 绝对不做（红线）

- ❌ 任何 job 的 `runs-on` 改造
- ❌ 7 个 CI job 重构 / 改名 / 聚合 / 删除；branch protection required contexts 不动
- ❌ BMD 盒（10.30.15.10）装 runner（Deployment §9 红线 + NET-01 出网受限）
- ❌ Runner Group 重构 / organization 级注册
- ❌ CI 测试逻辑修改；不顺手做任何「CI 优化」

---

## 1. A Baseline（2026-09-11 只读取证，冻结）

| # | 项 | 实测值 |
|---|---|---|
| A1 | master HEAD | `ff2c4816e8dfe02b23907e12092cbec691d101b9` |
| A2 | `.github/workflows/media-agent.yml` blob SHA | `5bf1c0e321bbca9bd9ba9f80596b828f2927d0e5` |
| A3 | required contexts（7，与 job `name` 严格一致） | `rust-format` / `rust-test-matrix` / `rust-clippy` / `hardware-test-compile` / `architecture-portability` / `gstreamer-build` / `session-lifecycle` |
| A4 | branch protection | strict=true（须含最新 master）；enforce_admins=false；无 PR review 强制；无 restrictions；禁 force-push / 禁删除 |
| A5 | repo self-hosted runners | total=**0**；owner_type=**User**（个人账户仓库：无 org runner-groups API → 「默认 group 实际名称」= **N/A，无 API 面**，repo 级 `actions/runner-groups` 端点实测 **HTTP 500**（2026-09-11，3 次确定性复现）；仓库级注册构造性限定于本仓库） |
| A6 | 与 PR #31 文件交集 | PR #31（head `feat/v03-standalone-product-baseline`，14 文件 = `ROADMAP.md` + `docs/architecture/V0.3/**`）与本 Phase 文件集 **零交集** |
| — | 内网代理连通（管理机实测） | HTTP CONNECT `http://10.30.5.73:8118` → api.github.com 200（1.8s）；SOCKS5 `10.30.5.73:1080` → 200（1.2s）。runner 宿主机待 B0 预检 |

> 归因纪律：此后任何 CI 变化，按本表回答「Runner Phase 1 引入还是仓库原存」，不靠记忆判断。

---

## 2. 三层 CI 架构（目标态）

```
                         VBMF CI
                            │
             ┌──────────────┼──────────────┐
             │              │              │
          GATE-A          GATE-B        GATE-C
        Software CI       Media CI     Hardware
             │              │        (Acceptance Line)
        rust-format      hardware-    DeckLink / BMD
        rust-clippy      test-compile  真实 SDI / 设备租约
        rust-test-       gstreamer-
        matrix ...       build ...
             │              │              │
        vbmf-general    vbmf-media    不进普通 PR CI
```

- **GATE-C 是 Acceptance Line，不是普通 PR Line**（与 Deployment §9 一致：实机验收走
  Remote BMD 人工 SSH + pinned SHA + evidence；不建议 CI 直接驱动硬件验收）。
- Phase 1 只建 `vbmf-ci-01`（General）。`vbmf-ci-02` / `vbmf-ci-media` 复用同一 runbook。

| Runner | 标签（exact set） | 职责 |
|---|---|---|
| `vbmf-ci-01` | `self-hosted, linux, x64, vbmf, vbmf-general` | Rust format/clippy/test、architecture tests、docs/contract |
| `vbmf-ci-02`（未来） | 同上 | 并发 PR / 回归 / 长测试 |
| `vbmf-ci-media`（未来） | `self-hosted, linux, x64, vbmf, vbmf-media` | FFmpeg/GStreamer/libclang/protobuf/SDK 头编译链 |

标签命名纪律：用**能力**（`vbmf-general`），不用机器名（`server01`）。

---

## 3. Runner Identity & Label Contract（CI-RUNNER-LABEL-01）

- `config.sh --labels` 只追加**自定义**标签（如 `vbmf-general`）；`self-hosted / linux / x64`
  由注册过程产生——**只验证、不假设**。
- 验收用**集合精确相等**（排序后逐字符比较），禁止 contains 式检查：

```text
expected(vbmf-ci-01) = { self-hosted, linux, x64, vbmf, vbmf-general }
```

- 实现：`scripts/ci/verify-runner.sh`（R2/R5）。

## 4. Repository Scope Contract（CI-RUNNER-SCOPE-01）

- 只注册为 **`pwl1987/VBMF` 仓库级** runner；不做 organization-wide。
- A5 实测：owner_type=User → 无 org runner 概念，scope=本仓库（构造性保证 + API 回读双证）；
  未来若迁移到 org，必须另立 Runner Group 契约评审。
- **公共仓安全边界表述（CI-RUNNER-SEC-02）**：Phase 1 **不授予不可信 fork PR 调度
  self-hosted runner 的路径**——本阶段不修改任何 PR/push workflow 的 `runs-on`；唯一引用
  新标签的 workflow 为 `ci-infra-probe.yml`，仅 `workflow_dispatch`（fork PR 无法触发）。
  不写「天然不可达」——只声明明确建立的控制边界，不依赖 GitHub 行为推导安全。
  Phase 2 迁移前必须另行裁决双通道（fork PR 留 GitHub-hosted）。

## 5. 网络与代理契约（CI-RUNNER-NET-01）

环境事实（内网代理，2026-09-11 管理机实测，见 §1）：

```text
HTTP CONNECT 代理: http://10.30.5.73:8118   # Actions Runner 唯一可用的代理类型
SOCKS5 代理:      10.30.5.73:1080          # 仅 ssh 等工具用；Runner 不支持 SOCKS
```

规则：

1. **B0 连通性预检**：在 runner 宿主机先测直连 `https://api.github.com/`；通则 runner
   运行时**零代理**。
2. 直连不通 → 唯一允许路径：`config.sh` 时以**瞬态 env**（`http_proxy/https_proxy/no_proxy`）
   传入，由 runner 自持久化到自身配置；**禁止** systemd `Environment=`、runner `.env`、
   workflow env 注入代理（CI-RUNNER-SEC-01）。运行时只能用 8118（HTTP）；
   1080 留给 `~/.ssh/config` / per-command 工具。
3. provisioning 下载（runner tarball / rustup / 任何直连 GitHub 的 curl）可经
   `VBMF_CI_PROXY` env 按命令传入；**脚本不硬编码代理地址**。
4. `no_proxy` 至少含 `localhost,127.0.0.1,10.0.0.0/8,169.254.169.254`。
5. 探针对 job env 做代理变量取证（PRESENT/ABSENT），作为红线的运行时证据。

## 6. 目录布局（冻结）

```
/data/actions-runners/vbmf/
├── packages/            # runner 安装包 + .sha256 记录（可复用，跨 runner 实例共享）
├── runners/
│   └── vbmf-ci-01/      # 实例（config.sh 装于此；_work/ _tool/ 由 runner 自建）
└── manifests/
    └── vbmf-ci-01/      # toolchain.yaml / toolchain.txt（环境证据，零 secret）
```

manifest 不放 `_work` 内；`packages` 与 `runners` 分离（同一包可配多实例）。

## 7. systemd 生命周期契约（CI-RUNNER-SYSTEMD-01）

- 进程生命周期**唯一归属**：`actions-runner-vbmf@<name>.service` 模板
  （本仓库 `scripts/ci/actions-runner-vbmf@.service`）。
- **`provision-runner.sh` 不执行 `svc.sh install`**——避免「自建模板」与「svc.sh 生成
  service」两套生命周期语义并存。
- 模板冻结参数：`Restart=always` / `RestartSec=5` / `KillMode=process` /
  `KillSignal=SIGINT` / `TimeoutStopSec=10min` / 固定 `HOME`·`LANG`·`WorkingDirectory` /
  **无任何 proxy 环境变量**。
- 安装 service 文件前必须 `diff`，不静默覆盖已修改的配置。

## 8. Provision 契约（CI-RUNNER-PROV-01）

- **fail-closed**：目标目录已存在 `.runner` → 明确 FAIL（默认非幂等、非破坏性）。
- **`--reinstall` 防误删链**（缺一不可）：
  1. 显式 `--name`；目标目录必须在 `/data/actions-runners/vbmf/runners/<name>` 下；
  2. 本地 `.runner` 的 `agentName` 与目标名匹配（本地归属证明）；
  3. `RUNNER_TOKEN` 非空；
  4. 停止对应 systemd instance；
  5. `config.sh remove`——repo 级 registration token 本身即服务端归属/删除证明；
  6. （宿主机有 gh 且已 auth 则 API 确认 absent；否则由管理机 `verify-runner.sh` 事后确认）；
  7. 以上全过 → 才删目录并重新 provision。
  **绝不「删目录然后重装」。**
- Token 纪律：只读环境变量 `RUNNER_TOKEN`；脚本拒绝回显；绝不写入仓库/文档/日志。
  历史上在聊天中出现过的 registration token 一律视为已烧毁（约 1h 有效期）。

## 9. 验收：双 Gate + A→F

### Runner Gate（R）

| # | 检查 | 实现 |
|---|---|---|
| R1 | 注册成功（API 可见） | `verify-runner.sh` |
| R2 | identity exact（name 逐字符） | 同上 |
| R3 | status=online | 同上 |
| R4 | busy=false | 同上 |
| R5 | labels 集合精确相等 | 同上 |
| R6 | probe PASS | `ci-infra-probe.yml` |

### No-Regression Gate（N)

| # | 检查 | 实现 |
|---|---|---|
| N1 | `media-agent.yml` blob SHA 不变（vs A2） | `verify-runner.sh --no-regression` |
| N2 | 7 required contexts 不变（vs A3） | 同上 |
| N3 | 现有 CI 行为不变（无 runs-on 改动；N1 覆盖） | 同上 |
| N4 | PR required checks 全绿 | `gh pr checks` |

```text
**RUNNER READY + NO REGRESSION = PHASE-1 READY**
```

### Gate 链（A→F）

```
A Baseline（已完成，§1）
 ↓
B Runner Build（宿主机：apt 基线 + rustup + 包下载 sha256 + config + systemd enable）
 ↓
C Runner Identity（verify-runner.sh：R1-R5 + scope）
 ↓
D Infrastructure Probe（ci-infra-probe.yml：R6，只读）
 ↓
E No Regression（N1-N4）
 ↓
F Rollback Drill（双次独立部署证据，见 §12）
 ↓
VBMF CI RUNNER PHASE-1 READY
```

## 10. job→runner 映射与迁移风险（Phase 2/3 输入，本阶段仅记录不执行）

| Job（required context） | 目标层 | 说明 |
|---|---|---|
| rust-format / rust-clippy / rust-test-matrix / session-lifecycle / architecture-portability | `vbmf-general` | 纯 Rust+Python3 |
| hardware-test-compile / gstreamer-build | `vbmf-media` | 需 libclang / GStreamer dev / protobuf / DeckLink SDK 头（secret 注入） |

迁移风险清单（Phase 2 评审输入）：

1. **SDK 残留**：`.github/_private` 仅靠 `if: always()` 末步 `rm -rf` 清理；持久 runner 上
   被杀 job 可能遗留头文件 → 迁移时需 pre-run 清理或 workspace 隔离策略。
2. **sudo apt**：两 job 每次安装系统包 → media 机预装后可去 sudo 依赖。
3. **无 timeout-minutes**：任何 job 都没有；单 runner 上挂死 job 会无限占机 → 迁移前必加。
4. **队列串行**：7 job 现在并行跑在 GitHub-hosted；单 runner 会串行化（且 push+PR 双触发
   无 concurrency 组）→ 需 ≥2 台或 concurrency 配置。
5. **target/ 共享**：持久 workspace 下多 job 共写 `services/media-agent/target`，cache
   restore/savechurn + 未缓存 job 同目录 → 磁盘与锁竞争，需 hygiene 方案。
6. **fork PR secrets**：现 `env.X != ''` 门控已优雅跳过 SDK 步骤；行为可平移，但公共仓
   双通道必须先裁决（§4）。
7. **toolchain 漂移**：`dtolnay/rust-toolchain@stable` 是滚动 stable（已发生过 CI rustfmt
   与盒上 1.9.0 版本差事件）→ 迁移时决定 pin 策略。
8. **工具链对 job 的可见性**：B0 的 rustup 装在 root `HOME`；Phase 1 job env 为
   `HOME=runners/<name>` + 最小 PATH，probe 中 `rustc/cargo` MISSING 属**预期**而非缺陷。
   工具链对 job 的 PATH/HOME 契约（系统级 `/usr/local` vs unit PATH vs runner 用户
   rustup）在 Phase 2 迁移评审中裁决。

## 11. 只读探针红线（CI-RUNNER-PROBE-01）

`ci-infra-probe.yml` **硬红线**：

```text
不安装软件 · 不修改系统 · 不访问 secrets · 不写仓库 · 不上传 artifact · 不执行 sudo
```

仅允许输出：identity / filesystem / workspace / TMPDIR / CPU-OS-arch / 工具路径与版本
（+ job env 代理变量取证）。目的：防止 probe 将来演化成「隐藏的 provisioning workflow」。

## 12. Rollback 契约（CI-RUNNER-ROLLBACK-01）

```bash
sudo systemctl stop actions-runner-vbmf@vbmf-ci-01
cd /data/actions-runners/vbmf/runners/vbmf-ci-01
sudo -u vbmf-ci env RUNNER_TOKEN="$RUNNER_TOKEN" ./config.sh remove --token "$RUNNER_TOKEN"
unset RUNNER_TOKEN
sudo rm -rf /data/actions-runners/vbmf/runners/vbmf-ci-01
```

- token 经 env 展开，字面量不进 shell history。
- 删除是否成功以 **API 服务端 absent** 为准（管理机 `verify-runner.sh`），非本地目录删除。
- GitHub-hosted CI 全程保留：**CI 基础设施失败不阻塞 VBMF 软件开发**。

### F Gate = 双次独立部署证据

> 第一次部署成功 + 完整卸载成功（remove → API absent）+ **第二次独立部署成功**
> （重新下载/sha256 校验/config/systemd/online/labels exact/probe 复跑）。

证明的是「runbook 可重复部署」，不是「这台机器偶然配置成功」。

## 13. Runbook（B→F，宿主机执行；命令逐条贴、输出回贴管理机核对）

### B0 前置预检（root）

```bash
# 网络预检：先直连，通则全程零代理
curl -sS -m 10 -o /dev/null -w 'direct=%{http_code}\n' https://api.github.com/ || true
# 直连不通才设（仅本 shell 会话，用于下载；runner 代理见 B4）
# export VBMF_CI_PROXY=http://10.30.5.73:8118
# curl -sS -m 10 --proxy "$VBMF_CI_PROXY" -o /dev/null -w 'proxy=%{http_code}\n' https://api.github.com/

# apt 基线（内网源或直连；缺啥补啥）
sudo apt-get update
sudo apt-get install -y git curl jq python3 build-essential pkg-config libclang-dev unzip tar

# rustup stable（General CI 基线；直连不通则前置 https_proxy="$VBMF_CI_PROXY"）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$HOME/.cargo/env" && rustc --version && cargo --version
```

### B1-B3 Provision + systemd（root，仓库脚本）

```bash
git clone https://github.com/pwl1987/VBMF /tmp/vbmf-repo && cd /tmp/vbmf-repo/scripts/ci

# 生成 fresh registration token 后（GitHub → Settings → Actions → Runners → New self-hosted runner）
export RUNNER_TOKEN=<fresh-token>          # 只经 env，不进任何文件

sudo --preserve-env=RUNNER_TOKEN ./provision-runner.sh --name vbmf-ci-01 --labels vbmf,vbmf-general
#   （B0 实测：本宿主机 sudo 忽略 -E；--preserve-env=RUNNER_TOKEN 为唯一可用传递方式，token 纯经 env、不进 argv）
#   直连不通时追加: --runner-proxy http://10.30.5.73:8118   （瞬态 env → runner 自持久化）

# systemd（先 diff 再安装，不静默覆盖）
diff /etc/systemd/system/actions-runner-vbmf@.service ./actions-runner-vbmf@.service || true
sudo cp ./actions-runner-vbmf@.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now actions-runner-vbmf@vbmf-ci-01

sudo -u vbmf-ci ./collect-toolchain.sh --name vbmf-ci-01   # manifests 落盘
unset RUNNER_TOKEN
```

### C/D/E 验证（管理机，只读）

```bash
scripts/ci/verify-runner.sh --name vbmf-ci-01                       # R1-R5 + scope
gh workflow run ci-infra-probe.yml --ref master                     # R6（dispatch-only；合并后 master 才有该 workflow）
gh pr checks <PR#>                                                  # N4
scripts/ci/verify-runner.sh --name vbmf-ci-01 --no-regression       # N1/N2/N3
```

### F Rollback Drill（宿主机 + 管理机）

按 §12 执行完整卸载 → `verify-runner.sh` 确认 absent → **清除 packages 缓存（落实 F Gate「重新下载」）** → 重跑 B1-B3 + C/D（第二次独立部署）：

```bash
sudo rm -f /data/actions-runners/vbmf/packages/actions-runner-linux-x64-*.tar.gz \
           /data/actions-runners/vbmf/packages/actions-runner-linux-x64-*.tar.gz.sha256
```

---

## 14. 交付物清单（本 Phase）

| 文件 | 角色 |
|---|---|
| `docs/architecture/CI_RUNNER_STRATEGY.md` | 本文件（SoT） |
| `scripts/ci/provision-runner.sh` | fail-closed provision + 安全 `--reinstall`（CI-RUNNER-PROV-01） |
| `scripts/ci/verify-runner.sh` | R1-R5 + scope + `--no-regression`（N1/N2） |
| `scripts/ci/actions-runner-vbmf@.service` | systemd 模板（CI-RUNNER-SYSTEMD-01） |
| `scripts/ci/collect-toolchain.sh` | manifests YAML+TXT（零 secret） |
| `.github/workflows/ci-infra-probe.yml` | 只读探针（CI-RUNNER-PROBE-01） |
