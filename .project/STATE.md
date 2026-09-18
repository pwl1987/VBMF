# VBMF Project State

> **Dynamic Project State Authority**
>
> 本文件是 VBMF 唯一动态项目状态与跨 Chat / Work handoff 入口。
> `ROADMAP.md` 与 `docs/architecture/PHASE_IMPLEMENTATION_MAP.md` 只承担宏观/历史路线职责；冻结 Architecture / Contract / ADR 仍承担设计 Authority。
> 若本文件与 live `main`、真实代码、测试或 Runtime / hardware evidence 冲突，先做 reconciliation，不得凭旧 roadmap、旧聊天或历史分支让项目倒退。

## 1. Repository / Git Authority

- Repository: `pwl1987/VBMF`
- Canonical development branch: `main`
- Default branch: `main`
- Git Authority policy: `main` 是唯一开发 Authority；不得再创建 `feature/*`、`fix/*`、`temp/*`、`repair/*`、`experiment/*` 等并行开发分支。
- `master → main` migration: **COMPLETE**（2026-09-13，原位 rename；历史链未复制、未 rewrite、未 force-push）。
- Remote `master` ref: **ABSENT**（GitHub 旧路径兼容重定向不构成实际 ref）。
- Single-branch convergence（2026-09-15）：单人开发裁定，`main` 为唯一分支；7 个历史远端分支（`diag/c2-o3-e2-single-input` / `diag/c2-o3-e3a-selftest` / `docs/v03-architecture-reorg` / `docs/v03-p1-plan` / `docs/v03-p1-plan-2` / `feat/v03-p1-standalone-control-plane` / `feat/v03-standalone-product-baseline`）已删除，远端 heads 仅存 `main`；PR #31 随 head 分支删除自动 CLOSED。
- Branch protection: `main` protected；required contexts 保持 7 个：
  - `rust-format`
  - `rust-test-matrix`
  - `rust-clippy`
  - `hardware-test-compile`
  - `architecture-portability`
  - `gstreamer-build`
  - `session-lifecycle`
- Repository rulesets: 当前远端查询为空；classic branch protection 仍有效。
- Last non-state-init implementation / CI reconciliation anchor: `cf1e07d1cbd8e8f304bfaf8a3fac8fd0d6408cc7` (`ci: automate P2-C host-admin bundle preparation`; GitHub Actions `34935730648` PASS).
- Dynamic state baseline: **以包含本 `.project/STATE.md` 当前版本的 live `main` commit 为准**；冷启动时用 `git log -1 -- .project/STATE.md` / GitHub file history 取得，不在文件内硬编码自引用 SHA。

### Local Working Tree / Execution Environment

2026-09-15 已迁入标准 Development VM 布局并可真实观察：

- Development VM: `ubuntu2604`；development checkout = `/home/ubuntu/dev/VBMF`；
- checkout 从 GitHub canonical `main` 新建，P2-B push 后与 live `main` 对齐；
- 当前窗口可直接核查 staged / unstaged / untracked，不再使用 `NOT OBSERVABLE` 口径；
- 旧 STATE 记录的另一环境未提交 `docs/architecture/DEPLOYMENT_AND_DEV_RUNTIME.md` 修改**不在本新 checkout 中**；未取得原 checkout 证据前不得声称其已删除或已迁移。
- Development VM 只承担开发/软件验证；BMD `lytv` 承担部署、Broadcast Runtime、DeckLink/hardware verification。

Agent foundation（2026-09-15 复核）：`/home/ubuntu/dev/_shared/bin/agent-preflight.sh /home/ubuntu/dev/VBMF --smoke` 已证明 Pi provider/model smoke **PASS**、Claude Code auth/model smoke **PASS**、Claude project trust **PASS**。Claude Code 后续写任务通过 `_shared/bin/run-claude-agent.sh` 在 tmux 持久会话中执行，保留 writer lock / structured activity / acceptance state；agent 输出仍不构成项目完成 Authority。

## 2. Current Phase

**Runtime Hardening immediate gates COMPLETE；Runtime Features entry review COMPLETE；当前 RF-FF-01A READY**

Phase 2 与 STAB-O3.1/O4 均已收口；本阶段只处理进入 Runtime Features 前会扩大故障面的关键 hardening。采用“按依赖按需清偿”而非一次清空全部历史债务：已被 BMD 实证的多输入 Bus/故障观测缺陷最高优先，随后是会被新 Source/Output 生命周期放大的 D1/D3/D7；D11+D13 在 Clock/Timecode 下一触碰点前清偿，D15 在多流 Audio/Metadata 前清偿，durable idempotency 在外部持久控制面前清偿。

当前不是 Runtime / Web 新业务功能开发阶段。P2-B 安全边界冻结、P2-C 全链（双机 pin→parity→rust-format 灰度）、P2-D（clippy 组件+rust-clippy 灰度）、P2-E1/P2-E2/P2-E3（session-lifecycle / architecture-portability / rust-test-matrix 灰度 + rustdoc 暴录）、P2-M0（SDK 裁决 B）、P2-M1（vbmf-media tier 供给 + SDK host 预装 16.0.0 + hardware-test-compile 迁移 + secrets 分片删除）、P2-M2（gstreamer-build 迁 vbmf-media，D1 空过窗口关闭）均已在 `main` 完成并通过真实 GitHub Actions；7 个 required job 全部完成 self-hosted 条件灰度（5 general + 2 media），GitHub-hosted 仅余 fork 回退路径。旧 `ROADMAP.md` / `PHASE_IMPLEMENTATION_MAP.md` 中更早的 “NEXT” 不得重新成为当前任务。

VBMF 的永久产品定位保持：

- Broadcast Runtime / Fabric；
- standalone first；
- 与 `media-digital-*` compatible, not dependent；
- Runtime owns truth; Web Console observes and commands；
- 广播专业 Contract 留在 VBMF / `vbmf-sdk`；
- Health / Watchdog / Recovery 是核心能力。

## 3. Last Completed

### 3.1 P2-A — dual general-runner parity / merge

Status: **COMPLETE**

Authority / evidence: `docs/architecture/CI_RUNNER_STRATEGY.md` §15.5 + commit `db54d45c76bbd05917292131c0a4a803b2907f36`.

Recorded result:

- `vbmf-ci-01` + `vbmf-ci-02` 已同挂 `{self-hosted,Linux,X64,vbmf,vbmf-general}`；
- 6 个被测工具 parity 逐字符一致；4 项环境差异已裁决；
- 两台 R Gate PASS，dispatch 随机派发已实测；
- `vbmf-ci-02` 运行于 devbox KVM VM，与 `vbmf-ci-01` 同物理故障域，事实已登记；
- P2-C 前两台需要对称执行系统级 Rust toolchain pin，禁止只改一台。

### 3.2 Git Authority convergence

Status: **COMPLETE**

- 默认分支已由 `master` 原位 rename 为 `main`；
- HEAD 在迁移时保持 `db54d45c76bbd05917292131c0a4a803b2907f36`；
- 保护规则及 7 required checks 保留；
- 远端无实际 `master` ref；
- 历史 feature/fix/实验分支只作为 historical refs，本轮未删除。

### 3.3 Project state-system initialization

Status: **COMPLETE in the commit containing this file**

- 建立唯一动态入口 `.project/STATE.md`；
- `ROADMAP.md` 与 `PHASE_IMPLEMENTATION_MAP.md` 已明确降级为宏观/历史 Roadmap；
- 未创建 `STATUS.md` / `TODO.md` / `DECISIONS.md` / `RISKS.md` / `HANDOFF.md` 等第二套动态状态文件；
- 未推进 Runtime / Web 业务功能。

### 3.4 P2-B — CI safety / scheduling boundary

Status: **COMPLETE / SOFTWARE VERIFIED**

Implementation commit: `9d6df61c1e8164d65cc94c4a0a1f4a9febe0a1c2`.

- fork PR 安全边界已冻结：P2-C+ 仅 trusted `main` push / same-repository PR 可进入 self-hosted；fork `pull_request` 永留 GitHub-hosted，禁止 `pull_request_target` 执行 PR code；
- `media-agent.yml` push branch 收敛到 canonical `main`，并加入 workflow-level `contents: read`、concurrency/cancel-in-progress 与全部 7 jobs 的 timeout；
- 7 required job/context 名称未变化，P2-B 本身未修改任何 `runs-on`；
- Strategy 操作性 `--ref master` 已改为 `--ref main`，历史 baseline 中 `master` 仅保留为历史证据；
- GitHub Actions run `34920528957`：7/7 required jobs **PASS**。

### 3.5 P2-C host-admin Rust pin tooling / runbook hardening

Status: **IMPLEMENTATION COMPLETE / SOFTWARE VERIFIED; HOST EXECUTION IN PROGRESS**

Implementation chain: `5a9ae16857ff28b29ca6f2da3f2db5acefe4fd37` → `cf1e07d1cbd8e8f304bfaf8a3fac8fd0d6408cc7` → `4eeb16e358cb7fd811763112dbaa9d0d2b954e84` → `0f375c8b49fdb61df507d7a75020c8d7c662b359`.

- exact system pin 固定为 Rust `1.98.1`，root-only `pin-system-rust.sh` + read-only `verify-system-rust.sh` 已落地；
- `prepare-system-rust-bundle.sh` 只接受 full 40-hex exact commit SHA，从该 commit byte-exact 导出 pin/verify/collect 三脚本并生成 deterministic `SHA256SUMS`；拒绝 floating ref / short SHA / missing script / 非空目标目录；
- focused non-root / zero-network tests：`scripts/ci/test-system-rust-scripts.sh` 在 verifier RCA 修复后扩展为 **41/41 PASS**；ShellCheck（touched scripts）PASS；docs targeted check / `git diff --check` PASS；
- encrypted runner route probe (`4eeb16e…`) 仅在显式 dispatch 时输出 RSA-OAEP-SHA256 密文；4 次真实 dispatch `34947844469` / `34947851225` / `34947857864` / `34947863953` 全部 PASS，并分别命中 `vbmf-ci-01` / `vbmf-ci-02`；内部管理地址/host key 信息未写入 Git/STATE/公开日志；
- host-route evidence 已恢复并严格验证 devbox host identity：Development VM 到精确宿主 SSH 可达、runner 本机 ED25519 fingerprint 与精确目标 `ssh-keyscan` 匹配、已有 key BatchMode 登录 PASS、host-admin `sudo -n` PASS；原“无 host-admin 通道”阻塞已解除；
- `vbmf-ci-01` 已实际 system-pin Rust `1.98.1` 并完成第二次 pin 幂等执行；首次独立 verify 暴露 caller rustup 污染 RCA，`0f375c8…` 已把 V2/V3/V4 锁定到 audited `/usr/local/{rustup,cargo}` 并清除 caller `RUSTUP_TOOLCHAIN`；真实 host1 正常环境与 poisoned caller env 均 V1–V4 PASS；
- GitHub Actions：`5a9ae16…` run `34934058858` PASS；`cf1e07d…` run `34935730648` PASS；`4eeb16e…` run `34947630667` PASS；`0f375c8…` run `34949135299` PASS；
- rejected CI-root maintenance path 的真实 evidence：run `34921727757` 同时命中 `vbmf-ci-01` / `vbmf-ci-02`，两机都在 `sudo -n true` fail，system pin step skipped，**无系统修改发生**；
- CI service account `vbmf-ci` 继续保持无通用 sudo/root；唯一合法执行平面是 Strategy §15.9 的 out-of-band host-admin runbook。

本子任务不等于 P2-C 完成：`vbmf-ci-01` 已完成实际 system pin 与修复版 verifier 实机验证；current-SHA bundle/manifest 收口见 §3.6。`vbmf-ci-02` 尚未 system pin；双机 parity re-probe 与 `rust-format.runs-on` 迁移均未执行。

### 3.6 P2-C1 — `vbmf-ci-01` current-SHA 正式收口

Status: **COMPLETE / HOST VERIFIED（2026-09-15，§15.9 步骤 A+B 实际执行）**

- coordinator 确认 bundle 源 exact SHA：`b0f5df3b215c536b598aed7938079572edf87395`（时点 live canonical `main` tip；与 `0f375c8` 仅差 docs，三脚本 byte-identical）；
- `prepare-system-rust-bundle.sh` 导出三脚本 + deterministic `SHA256SUMS`：`collect 5af0bfaa04607973b97ca9a40e1cdbc277818428a49181d27ec434b47b039f1a` / `pin 46056167848a2e1a71350650650ed7fcd2073aa504474ffafe0e2e6f31d995b1` / `verify d0ae04fc424824951fabb9154da5e0bb766bdaa54d72f182d7d5fa61c03d57db`；`BUNDLE_SOURCE_SHA` 与 SHA 一致；
- host1（devbox）`sha256sum -c SHA256SUMS` 3/3 OK；
- 幂等 pin 复跑（正式 bundle 脚本）：`PINNED_RUST_VERSION=1.98.1`，exit 0，版本不变；
- 正式 verifier V1–V4：`RESULT: PASS`（rustc `1.98.1 (48a229cea 2026-09-01)`、cargo `1.98.1 (797e8a9bc 2026-08-05)`、rustfmt `1.9.0-stable`；`rustup default=1.98.1-x86_64-unknown-linux-gnu`，无 rolling stable）；
- manifest 以 `vbmf-ci` 身份刷新成功（需显式 `RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo`，见下方 RCA）：`/data/actions-runners/vbmf/manifests/vbmf-ci-01/{toolchain.yaml,toolchain.txt}` @ `2026-09-15T10:01:21Z`，rustc/cargo = `/usr/local/bin/*` 1.98.1；`clang: MISSING` 为既有宿主状态，非本 gate 项；
- 管理机只读收口：`verify-runner.sh --name vbmf-ci-01` R1–R5 + scope 全 PASS（`RESULT: PASS`），labels 仍为 exact `{self-hosted,Linux,X64,vbmf,vbmf-general}`；
- host key 采信依据：Development VM 直连 `ssh-keyscan` 与 BMD 服务器侧交叉 scan 双独立路径 fingerprint 一致后写入 `known_hosts_agents`；地址/fingerprint 依红线不写入本文件；
- `vbmf-ci` 权限零变更（无 generic sudo/root 新增）；宿主机零 git 操作；bundle 临时目录已清理；
- **运维 RCA（对 P2-C2/P2-C4 有约束力）**：rustup proxy 在 `vbmf-ci` 身份下若不显式设置 `RUSTUP_HOME`/`CARGO_HOME`，会尝试 `$HOME/.rustup` 且因 runner base 目录不可写而失败（2026-09-11 旧 manifest 同样止步于 python3 的原因）。凡以 `vbmf-ci` 身份调用 rustc/cargo/rustfmt（manifest collect、未来 self-hosted CI job）必须显式携带 `RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo`；
- 本轮同步落盘 `AGENTS.md` coordination tiering（强模型 = coordinator，Pi = 默认 executor，executor 不写调度契约）；`_shared/README.md` 对应条款先行落盘（dev-agent-kit main `93a6451`）。

### 3.7 P2-C2 — `vbmf-ci-02` 对称 system Rust pin 收口

Status: **COMPLETE / HOST VERIFIED（2026-09-15，§15.9 步骤 A+B 经 devbox 跳板实际执行）**

- coordinator 确认 bundle 源 exact SHA：`9e47721eb30355ffe23cea12592445e8f0575c6a`（时点 live canonical `main` tip）；三脚本与 `b0f5df3`（P2-C1）byte-identical，`SHA256SUMS` 完全一致（collect `5af0bfaa…` / pin `46056167…` / verify `d0ae04fc…`）；
- guest（devbox KVM `vbmf-ci-02`）host-key 严格验证：devbox 管理面 `ssh-keyscan` 所得 ED25519 fingerprint 与用户 guest 控制台读出的 `/etc/ssh/ssh_host_ed25519_key.pub` fingerprint 比对一致后写入信任；地址/fingerprint 不落盘；
- guest `sha256sum -c SHA256SUMS` 3/3 OK；
- 首次 system pin + 幂等复跑：两次 `sudo pin-system-rust.sh --version 1.98.1` 均 exit 0，`PINNED_RUST_VERSION=1.98.1`（首次运行输出含 `unchanged` 提示 `/usr/local` 下 toolchain 组件已在此前 provisioning 中就位，本论完成 default/链接收口，符合幂等语义）；
- 正式 verifier V1–V4：`RESULT: PASS`（rustc `1.98.1 (48a229cea)`、cargo `1.98.1 (797e8a9bc)`、rustfmt `1.9.0-stable`；`rustup default=1.98.1-x86_64-unknown-linux-gnu`，无 rolling stable）——与 host1 exact 对称；
- manifest 以 `vbmf-ci` 身份（显式 `RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo`，§3.6 RCA 生效）刷新：`/data/actions-runners/vbmf/manifests/vbmf-ci-02/{toolchain.yaml,toolchain.txt}` @ `2026-09-15T11:14:31Z`，rustc/cargo = `/usr/local/bin/*` 1.98.1；
- 管理机只读收口：`verify-runner.sh --name vbmf-ci-02` R1–R5 + scope 全 PASS（`RESULT: PASS`），labels exact；
- **运维 RCA（对 P2-C3+ 有约束力）**：bundle 经 devbox 中转 scp 到 guest 后可能丢失执行/读权限位，宿主机执行前必须 `chmod 755` 脚本，否则 `sudo -u vbmf-ci bash <script>` 以 126 `Permission denied` 失败；
- `vbmf-ci` 权限零变更；guest 零 git 操作；guest/devbox/VM 三方 bundle 临时目录均已清理。

### 3.8 P2-C3 — 双 runner parity re-probe + probe env 修正

Status: **COMPLETE / PROBE VERIFIED（2026-09-15）**

- 前置修正：`ci-infra-probe.yml` capability step 补显式 `RUSTUP_HOME=/usr/local/rustup`、`CARGO_HOME=/usr/local/cargo` 并 `unset RUSTUP_TOOLCHAIN`（commit `53cf25b…`，与 verify-system-rust.sh V2–V4 同调用契约；§3.6 RCA 落地——plain `--version` 在 `vbmf-ci` 下会因 `$HOME/.rustup` 不可写而报错）；只读红线/结构不变；push 后 CI run `34963352333` SUCCESS；
- dispatch 证据：两次 `workflow_dispatch`（tier=`vbmf-general`）分别命中两机——run `34963581888` → `runner.name=vbmf-ci-02`（workspace `/data/actions-runners/vbmf/runners/vbmf-ci-02/_work/…`），run `34963608876` → `runner.name=vbmf-ci-01`（workspace `…/vbmf-ci-01/_work/…`）；
- parity 双证：两份报告 `rustc: /usr/local/bin/rustc (rustc 1.98.1 (48a229cea 2026-09-01))`、`cargo: /usr/local/bin/cargo (cargo 1.98.1 (797e8a9bc 2026-08-05))`，逐字符一致，无 MISSING、无 rustup 报错；
- labels：GitHub runners API 实查两机均 online 且 exact `{self-hosted,Linux,X64,vbmf,vbmf-general}`；
- manifest：引用 §3.6（host1 @10:01:21Z）/§3.7（host2 @11:14:31Z），未重跑；
- §15.7 末条与 §15.9 收尾段的 parity 前置已满足：P2-C4 允许迁 `rust-format.runs-on`。


### 3.9 P2-C4 — `rust-format` 条件 `runs-on` self-hosted 灰度（P2-C 收口）

Status: **COMPLETE / CI VERIFIED（2026-09-15）**

- implementation commit `41400e1b36e67a36cbd9a9395d4f42efb42c76ad`：仅改 `rust-format` 单 job——条件 `runs-on` 表达式（trusted = 非 fork PR 或 push → `["self-hosted","Linux","X64","vbmf","vbmf-general"]`；fork PR → `ubuntu-latest`）；dtolnay toolchain 步骤仅 `runner.environment == 'github-hosted'` 时执行且 pin exact `1.98.1`（D3，与系统 pin 对称防 rustfmt 漂移）；self-hosted 路径零 install，format check 内显式 `RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo` + `unset RUSTUP_TOOLCHAIN`（§3.6 RCA 调用契约）；
- 结构不变量已断言（本地检查 PASS）：其余 6 job 与 origin/main 完全一致、job id/name 仍 `rust-format`、无 `pull_request_target`、`permissions: contents: read`、timeout 10m、concurrency 不变；
- GitHub Actions run `34969039593`（push `main` 触发）：**7/7 required checks PASS**；`rust-format` job 由 **`vbmf-ci-02`** 执行（labels `self-hosted,Linux,X64,vbmf,vbmf-general`），job log 显示 `RUNNER_ENV: self-hosted` 分支与 `cargo fmt --all -- --check` 在系统 pin 下通过——push 路径 runner identity + 系统 toolchain 双证据落库；
- fork 路径 live 验证（brief D4）：默认真值表+结构审查；若用户提供自有 fork 测试 PR，补充实证后在本节登记；残余风险 = fork 分支未 live 实测，将来外部 fork PR 首次出现时以 job `runner_name` evidence 立即核验；
- P2-C 整体收口：§15.2 灰度阶梯 P2-C 完成，P2-D（rust-clippy）解锁。

### 3.10 P2-D — clippy 组件收口 + `rust-clippy` self-hosted 灰度

Status: **COMPLETE / HOST + CI VERIFIED（2026-09-15）**

- 前置实测发现：系统 pin 只含 rustfmt，host1 `cargo clippy` 报 `cargo-clippy is not installed`——P2-D 需先补组件再迁 job；
- 阶段 1（脚本，commits `fe58aab…` + `4afa771…`）：pin/verify/collect/test 四脚本加 clippy（`--component clippy`、暴露 `cargo-clippy`/`clippy-driver`、verify 新 V5 gate、manifest 增行）；**RCA：clippy 版本方案为 `0.1.<rustc-minor>`（1.98.1 → `clippy 0.1.98`），gate 必须按此推导**（首次 host1 实测 `clippy 1.98.1` 断言失败后修正）；tests **45/45 PASS**，ShellCheck（warning 级，预存 info SC2031 除外）PASS；CI runs `35025999239` / `35027349065` 全绿（前者经一次出网窗口 rerun）；
- 阶段 1（host-admin，bundle 源 exact SHA `4afa7717615fbe161f640bdcd8f8ad640374b583`，collect `9dbd189d…` / pin `6f2fdca4…` / verify `cff27669…`）：host1 直连 tar 管道、host2 经 devbox 跳板 scp 中转，双机 `sha256sum -c` 3/3 OK → root 幂等 pin 复跑 exit 0 → 扩展 verify **V1–V5 全 PASS**（V5 `clippy 0.1.98 (48a229ceae 2026-09-01)`）→ `vbmf-ci` 身份 manifest 刷新（显式系统 env）：host1 @`2026-09-15T21:47:27Z`、host2 @`21:48:28Z`，`cargo-clippy: /usr/local/bin/cargo-clippy` 双证一致；`verify-runner.sh` 双机 R1–R5+scope PASS；`vbmf-ci` 零新 sudo、宿主机零 git、临时目录四方清理；
- **运维 RCA（tar/scp 中转补充 §3.7）**：中转目录若以 umask 077 建立（`drwx------`），`vbmf-ci` 无法穿越——传输后必须 `chmod 755` 目录本身，仅 chmod 脚本不够；
- 阶段 2（workflow，commit `eca00b7…`）：`rust-clippy` 条件 `runs-on`（P2-C4 同模式）；dtolnay 步骤与 `actions/cache` 仅 `runner.environment == 'github-hosted'`（fork 路径 toolchain pin `1.98.1`）；self-hosted 零 install，两条 clippy 命令内联导出 `RUSTUP_HOME=/usr/local/rustup` + checkout 外持久 `CARGO_HOME="${GITHUB_WORKSPACE%/*}/.cargo-home"`、`CARGO_TARGET_DIR="${GITHUB_WORKSPACE%/*}/target-clippy"` + `unset RUSTUP_TOOLCHAIN`（绕开 root-only `/usr/local/cargo` 与 checkout clean；持久目录替代 actions/cache）；
- CI run `35027740186`（push `main` `eca00b7`）：首跑双 self-hosted job 同遇出网故障窗（21:51–21:56 双机 fetch 断）失败，窗口过后 rerun **7/7 required 全绿**；`rust-clippy` @**vbmf-ci-02**（labels self-hosted/Linux/X64/vbmf/vbmf-general），log 实证 env 契约与真实编译（22 crates，`Finished dev 28.91s`；mock 复用 target `10.50s`）；
- **出网稳定性观察（升级为风险项）**：2026-09-15 三次故障窗（~13:12 host1、21:32–21:40 双机、21:51–21:56 双机），均表现为 runner agent 通道正常而 git fetch 新建 TLS 连接失败；影响所有 self-hosted job 首步 checkout，当前靠 rerun 收口；P2-E1 前需裁决缓解方案（job 级 git 代理与 ci-infra-probe 的 workflow-env 红线冲突，需单独决策）。

### 3.11 P2-E1 — `session-lifecycle` 条件 `runs-on` self-hosted 灰度 + 出网观察窗

Status: **COMPLETE / CI VERIFIED（2026-09-15）**

- implementation commit `810b60c8e333a48a26055bcdaf655c6fd5b022f7`：仅改 `session-lifecycle` 单 job——条件 `runs-on` 表达式与 P2-C4/P2-D 逐字一致；dtolnay 步骤与 `actions/cache` 加 `runner.environment == 'github-hosted'` 分流，fork 路径 dtolnay 保持无 pin 参数（D2：本 job 测试断言不依赖 exact 补丁版本，不顺手改语义）；self-hosted 零 install，测试步骤内联显式 `RUSTUP_HOME=/usr/local/rustup` + `unset RUSTUP_TOOLCHAIN`（§3.6 RCA）+ checkout 外持久 `CARGO_HOME="${GITHUB_WORKSPACE%/*}/.cargo-home"`（与 clippy 共享 registry）+ **独立 `CARGO_TARGET_DIR="${GITHUB_WORKSPACE%/*}/target-session"`**（D3：clippy 与 test 构建参数不同，隔离防缓存互踩）；四条 `cargo test --features mock`（session/resource/lease/preflight）逐字不变；
- 结构不变量本地断言 PASS：其余 6 job 与 origin/main byte-identical、job id/name 仍 `session-lifecycle`、`on:` 触发键 = {push, pull_request}、timeout 20 / concurrency / permissions 不变；
- GitHub Actions run `35033319131`（push `main` `810b60c`）：首跑三 self-hosted job 全部命中当日第 4 个出网故障窗（~22:55–23:05 UTC，双机 fetch TLS 失败），两次 rerun 后 **7/7 required 全绿**；`session-lifecycle` job 由 **`vbmf-ci-01`** 执行，log 实证 `RUNNER_ENV: self-hosted` 分支 + env 契约 + 四条 mock 门禁测试 **50 passed / 0 failed**；
- 出网稳定性观察（risk 8 数据）：P2-E1 验证期 2 个故障窗——窗 #4（22:55–23:05 UTC，双机，4 次 job 失败，run `35033319131` 2 次 rerun 收口）；窗 #5（23:14–23:19 UTC，双机：ci-01×2 + ci-02×1，3 次 job 失败，STATE 闭环 push `0294450` 的 run `35034627908` 1 次 rerun 收口，收口后 7/7 全绿且 `session-lifecycle` 在 **vbmf-ci-02** 亦有 live PASS 证据）；runner agent 通道始终正常；当日累计 5 窗，数据不足以裁决缓解方案，P2-E2 继续累计。

### 3.12 P2-E2 — `architecture-portability` 条件 `runs-on` self-hosted 灰度

Status: **COMPLETE / CI VERIFIED（2026-09-16）**

- implementation commit `fe2e04a…`：仅改 `architecture-portability` 单 job——条件 `runs-on` 与既有模式逐字一致；dtolnay 与 `actions/cache` 加 github-hosted 分流（fork 路径无 pin 参数，D2）；**Lint 步骤零改动**（纯宿主 python3，双机 parity 已证）；Proof 步骤 self-hosted 分支显式 `RUSTUP_HOME=/usr/local/rustup` + `unset RUSTUP_TOOLCHAIN`（§3.6 RCA）+ checkout 外持久 `CARGO_HOME=".cargo-home"`（共享）+ **独立 `CARGO_TARGET_DIR="target-arch"`**（D3：`--no-default-features --features simulation/mock` 组合独立，防缓存互踩）；两个门禁脚本零改动，`check_remove_adapters.py` 经 subprocess 继承 step env；
- 结构不变量本地断言 PASS：其余 6 job byte-identical、job id/name 不变、`on:` 键 {push, pull_request}；
- GitHub Actions run `35061944269`（push `main` `fe2e04a`）：**7/7 required 首跑全绿（无 rerun）**；job @**vbmf-ci-01**，log 实证 Lint（宿主 python3）PASS + Proof（`RUNNER_ENV: self-hosted` 分支 + env 契约 + `remove-adapters` 两条 feature cargo check 执行）成功；
- 窗 #6 补记（自 P2-E1 验收 risks 顺延，D6）：2026-09-15 23:26–23:33 UTC，双机（ci-01×2 + ci-02×1），3 次 job 失败均 checkout fetch TLS，run `35035748405` 1 次 rerun 收口；
- 出网稳定性观察（risk 8 数据）：P2-E2 验证期 **0 故障窗**（首跑全绿）；当日（2026-09-16）暂无新窗。

### 3.13 P2-E3 — `rust-test-matrix` 条件 `runs-on` self-hosted 灰度（P2-E 收口）

Status: **COMPLETE / HOST + CI VERIFIED（2026-09-16）**

- 阶段 2 先行（workflow，commits `145669c` + `a975036`）：条件 `runs-on`（第 5 个也是最后一个 general-pool required job）；**env 经单个 prep 步骤写 `$GITHUB_ENV`**（D4：6 个 cargo 步骤零改动自动继承；caller `RUSTUP_TOOLCHAIN` fail-closed 拒绝）；持久 `.cargo-home` + 独立 `target-testmatrix`；upload-artifact 因 v4 禁 `..` 路径改为 self-hosted 专用 staging 步骤（`a975036`：拷贝二进制入 workspace 后双路径条件表达式，`35079837269` 的 `Relative pathing '.' and '..' is not allowed` RCA）；6 条 cargo 命令逐字不变；
- 阶段 1（rustdoc 暴录，commit `43b665e`）：**RCA：`--profile minimal` pin 不含 rustdoc，`cargo test` doctest 阶段 PATH 找不到 rustdoc 即失败**（首跑 `35075935539` @vbmf-ci-02 实测：232 单测全过、doctest `could not execute process rustdoc`）；双机 toolchain bin 均已有 rustdoc 二进制（历史 provisioning），仅缺 `/usr/local/bin` 暴露；四脚本扩展（`--component rust-docs` 显式化 + symlink 循环 + verify V6 presence/执行 gate + manifest）后 **48/48 tests PASS**、shellcheck 零 warning；
- host-admin（bundle 源 exact SHA `43b665eb287ace31cddf2c984068ace42eb2b6bb`，collect `089d4bcc…` / pin `3afb1644…` / verify `d9c0443e…`，用户授权 D8）：双机 checksum 3/3 → root 幂等 pin 复跑 exit 0（零工具链下载）→ verify **V1–V6 全 PASS**（V6 `rustdoc 1.98.1 (48a229cea 2026-09-01)`）→ manifest 刷新 host1 @`2026-09-16T09:38:10Z` / host2 @`09:55:59Z`（`rustdoc: /usr/local/bin/rustdoc`）；GitHub API 双机 online + exact labels；临时目录三方清理；**运维 RCA：host2 首次 pin 复跑经 devbox 跳板在 rustup 网络 self-update 检查处缓慢，管道 `timeout` 曾截断 symlink 循环且 `tail` 掩盖退出码——host-admin 必须整跑判 pin 自身 RC，不得管道截断**；
- CI：run `35079837269`（`43b665e`）rust-test-matrix 仅败于 upload `..` 路径（6 cargo 步骤已全过）；run `a975036` 对应 run **7/7 required 全绿首跑**，rust-test-matrix @**vbmf-ci-01**：prep env 契约 + 232×3 单测 + **Doc-tests 三轮执行 0 FAILED**（rustdoc 生效）+ artifact 上传成功；
- **P2-E 整体收口**：5 个 general-pool required job（rust-format/rust-clippy/session-lifecycle/architecture-portability/rust-test-matrix）全部完成条件 runs-on 灰度；hardware-test-compile/gstreamer-build 留 GitHub-hosted 待 P2-M0 裁决；
- 出网稳定性观察：risk 8 补记 2026-09-16 窗（07:04–07:05 UTC，ci-01，run `35066419711`，1 rerun）；P2-E3 验证期（`145669c`/`43b665e`/`a975036` 三 push + rerun 0 次）出网零故障（`35075935539` 失败为 rustdoc 真实缺陷非出网）。

### 3.14 P2-M0 — DeckLink SDK 注入模式裁决

Status: **COMPLETE / ADJUDICATED（2026-09-16）**

- 用户裁决：**B——media 主机预装 + 版本锁定（去 secret 化）**；A（维持 secrets
  分片注入）未采纳。裁决 + 五条理由 + 三条隐含义务 + 过渡期口径已落
  `docs/architecture/CI_RUNNER_STRATEGY.md` §15.3（docs-only commit）；
- 隐含义务（P2-M1 范围）：`vbmf-media` tier 供给 runbook（SDK pin/verify/collect
  同构 + libclang/GStreamer dev/protobuf 清单）；workflow 删 secrets 注入改宿主
  路径门控（fork 空过语义不变）；SDK secrets 分片 P2-M1 收口时删除；
- 出网稳定性观察（risk 8 补记）：2026-09-16 窗 #2（10:09–10:22 UTC，ci-01，
  run `35083349130`，arch×2 + session-lifecycle，2 rerun）、窗 #3（11:54–11:59
  UTC，ci-02，run `35091905412`，1 rerun）、**窗 #4（13:20–14:50+ UTC，双机，
  run `35101465356`，10+ 次 job 失败跨 4 轮 rerun——最长窗，新签名 codeload
  action 下载超时；由本 packet 内授权的 git 代理缓解收口，7/7 全绿）**——当日
  累计 4 窗，两日累计 10 窗；
- **git 出网缓解已授权并落地（2026-09-16，用户"两项都做"裁决）**：双机
  `git config --system http.https://github.com/.proxy` 指向 devbox 8118 代理
  （host1 本机、host2 经网关；地址不入 repo）；宿主机级 git 配置经既有
  out-of-band 通道下发，**不触碰 systemd unit / runner .env / workflow env，
  probe 的 proxy-env 证据面零污染（红线兼容）**；双机以 `vbmf-ci` 身份
  `git ls-remote` 经代理实测通过；**残余面：codeload action 下载走 runner
  HttpClient，仅进程代理可治——维持 rerun 口径（用户未豁免 systemd 注入红线）**；
  Strategy §15.11 文档化随 P2-M1 落盘。

### 3.15 P2-M1 — `hardware-test-compile` 迁移 vbmf-media tier（SDK host 预装落地）

Status: **COMPLETE（2026-09-17）**

- **段 1（repo 侧，`c351edd`）**：`pin-decklink-sdk.sh` / `verify-decklink-sdk.sh` /
  `collect-toolchain.sh` SDK 段 / `test-decklink-sdk-scripts.sh`（29/29 PASS）；
  Strategy §15.10（media tier 供给 runbook，§15.3 义务 1）+ §15.11（出网 git
  代理缓解文档化，P2-M0 欠账）；probe allowlist += `vbmf-media`；
- **段 2（host 侧，用户一次授权五项）**：KVM VM `vbmf-ci-media`（8C·16G·100G，
  devbox libvirtd，cloud-init seed 复用 ci-02 模式）；系统库（libclang-21 /
  GStreamer 1.28.2 dev / protoc 3.21.12 / pkg-config）；runner 注册 labels
  exact `{self-hosted,Linux,X64,vbmf,vbmf-media}`，R1–R5 PASS；Rust 1.98.1
  pin（幂等双跑，V1–V6 全 PASS）；**DeckLink SDK 16.0.0 host pin（zip 经
  out-of-band 三段 md5 一致中继上机；42 头；V1–V4 全 PASS；manifest
  `decklink_sdk` 段落盘）**；probe（tier=vbmf-media）PASS @ vbmf-ci-media；
- **RCA（`11e9dfb`）**：必备头清单修正——SDK 16.0 的 `DeckLinkAPIDispatch`
  是 `.cpp` sample 非 header；正确核心四头 = `DeckLinkAPI.h` /
  `DeckLinkAPIConfiguration.h` / `DeckLinkAPITypes.h` / `DeckLinkAPIModes.h`
  （fail-closed 预检在动 canonical 路径前拦住，零污染）；
- **段 3（workflow，`7196a0a`）**：条件 `runs-on`（vbmf-media）；secrets
  env/unpack/`_private` cleanup 全删；self-hosted fail-closed SDK 验证步 +
  §3.6 RCA env 契约 + `target-hwtest` 外置 + staging 拷贝；github-hosted
  （fork）空过语义不变；CI run `35172539232` rerun 后 **7/7 全绿**，
  `hardware-test-compile` @ **vbmf-ci-media** 实跑（log：`DeckLink SDK:
  16.0.0 (host-preprovisioned)`；`media-agent-linux` artifact 6495056B +
  `decklink-bindings-debug` 15872B，非空过）；
- **secrets 收口**：`DECKLINK_SDK_HEADERS_1/2` + `DECKLINK_SDK_VERSION` 已从
  GitHub 删除（`gh secret list` 空）；**D1 裁决的 gstreamer-build 空过窗口
  自此开启**（其 yaml 未动，`env != ''` 门自然为假，github-hosted 上空过绿；
  P2-M2 迁 vbmf-media 后恢复真实构建）；
- 出网观察：2026-09-17 窗 #1（media VM 首跑 checkout GnuTLS 失败，
  run `35172539232` 首次尝试，rerun 收口）；缓解同日扩展到 media host
  （git system proxy → 网关 8118，模式与双 general 机一致，红线兼容）。

### 3.16 P2-M2 — `gstreamer-build` 迁移 vbmf-media tier（P2-M 全链收口）

Status: **COMPLETE（2026-09-17）**

- 单 commit `0c4bbd1`（基线 `8238f0b`）：条件 `runs-on`（vbmf-media，P2-M1 逐字
  模式）；secrets env（`DECKLINK_SDK_HEADERS_1/2`/`DECKLINK_SDK_VERSION`）+
  `_private` unpack + `if: always()` cleanup 全删；self-hosted fail-closed 门 =
  SDK 16.0.0（P2-M1 同款）**+ pkg-config `gstreamer-1.0`/
  `gstreamer-plugins-base-1.0` 门（D2 新增）**；§3.6 RCA GITHUB_ENV 契约 +
  独立 `target-gstbuild`；staging 拷贝 + upload-artifact v4；fork 路径
  checkout+dtolnay+apt 保留 github-hosted 门控（D3，语义零变化）；
- Strategy 事实同步：§15.1 `vbmf-ci-02`/`vbmf-ci-media` 行翻 ✅ 在役（D4，
  ci-02 为 P2-A 历史欠账）、§10 L218 "（secret 注入）" → "（host 预装，§15.3
  裁决 B）"；
- CI run `35177980436`（push main `0c4bbd1`，run_attempt 1）：**7/7 required
  首跑全绿，零 rerun、零出网窗**；`gstreamer-build` @ **vbmf-ci-media**
  （job `105063812333`，labels exact）；log `DeckLink SDK: 16.0.0
  (host-preprovisioned); GStreamer dev: 1.28.2`；真实构建 109 crates
  `Finished dev 53.26s`；
- **D1 空过窗口关闭**：artifact `media-agent-gstreamer-linux` 恢复非空
  （12331677B）；P2-M1 删 secrets 后的空过绿过渡态结束；
- 结构断言：其余 6 job byte-identical（基线 `8238f0b`，comment-stripped
  job-block diff）；workflow 级 triggers/permissions/concurrency/defaults 不变；
- 验证：Runtime 机械检查 5/5 passed + 独立只读 Verifier 两轮（attempt 1 的
  log-evidence 检查 grep ANSI 正则缺陷——色码位于 `Finished` 与 `` `dev` ``
  之间；attempt 2 以 sed 去 ANSI 版替换后 5/5 passed；实现零缺陷）；用户已
  接受验收结果；
- **P2-M 全链完成 / Phase 2 灰度阶梯走完**：P2-M0 裁决 B → P2-M1（tier 供给 +
  SDK 预装 + hardware-test-compile + secrets 删除）→ P2-M2（gstreamer-build）；
  self-hosted 条件 `runs-on` 已覆盖全部 7 个 required job。

### 3.17 STAB-O3.1 — C2-O3-1/E2 + C2-O3-2/E3A 证据恢复登记（RSS RCA 分叉收窄）

Status: **RECOVERED / REGISTERED（2026-09-17；分叉证据·非因果·不改判 24h FAIL）**

- **恢复叙事**：E2（单输入 2.5h·2026-09-11）与 E3A（selftest 源族 45m·2026-09-12）
  在旧环境执行于 `diag/c2-o3-e2-single-input` / `diag/c2-o3-e3a-selftest` 两分支，
  随 2026-09-15 分支收敛删除且未 merge——证据从未入 main。本轮经 dev VM→盒
  （lytv@10.30.15.10·id_pwl·本窗口打通的通道）只读拉取 165+2 件，md5 盒=origin
  161/161（唯一披露：E2 observer-console.log 的 md5s 行为终行追加前旧哈希·
  双端直核一致）；盒上 `~/media-agent-build` .rs 78/78 与 main 逐字节一致。
- **E2 裁定**（冻结判读表行1）：单输入下 reservation-family 扩展照旧 ⇒
  **双输入/双实例不是必要条件**；附属行3 事实：量子化 workload-sensitive
  （连续涓流 0.87kB/s·边界 ~1190s 迁 ~1MB vs 双输入离散 5.5MB@7100s）；
  每输入聚合速率相等（2.99 vs 2.81MB/h·差 6%）；PROGRAM_SWITCH=ABSENT 且
  零切换下照增 ⇒ program graph/切换/ExecutionGroup/make_mut 探针全部排除为
  必要条件。
- **E3A 裁定**（同 bin 干净差分）：DeckLink 源族整体移除后 45min VmRSS 恒定
  21956kB·smaps 零变化 ⇒ **DeckLink 源族配置 = 形态必要条件之一**。
- **候选空间收敛**：每输入 DeckLink 源族 ingest native 分配链 × allocator
  reservation/arena 行为；六环已过 same-reservation/magnitude/repeatability，
  allocation-path/allocator-behavior 未证——无 root cause 结论。
- **偏差披露**：E2/E3A bin=ad89c6d3 ≠ 冻结 f52b0161（盒上构建时 Cargo.lock
  重解析——main 的锁不含 glib/gstreamer 可选条目·结构性债务登记）；E2 vs
  双输入历史 run 对比带依赖补丁版本混杂（E3A 同 bin 对照干净）；f52b0161
  锁版本不可重建（历史 build.log 均缓存命中）。
- 通道事实：盒 SSH = lytv@10.30.15.10 + `~/.ssh/id_pwl`（dev VM·非默认键名）；
  设备占用复核：gst-launch 历史进程（device-number=2）仍在（pid 992634），
  诊断 run 绑定空闲设备不受阻；盒上无残留 media-agent 服务。
- 分析文件：`evidence/bmd-10.30.15.10/c2o3-analysis/c2o3-e2-e3a-recovery-analysis.txt`
  + EVIDENCE-INDEX 三行。
- **收口（2026-09-17 用户裁决"123"）**：①STAB-O3.1 判定
  **INCONCLUSIVE-at-allocation-path + 候选空间收敛登记**（验收满足
  "owner 候选收敛或明确 INCONCLUSIVE"——procfs observer 面已尽·禁入清单未动·
  allocation-path/allocator-behavior 两环在现红线内不可证）；②深挖转为
  STAB-O4 首刀 = E4 型生产代码观测点（用户已授权该面·gstreamer crate API
  内省/pool 统计属 Rust 可达面·非 GST_DEBUG env）；③Cargo.lock 结构性债务
  已修：commit `7a9f696`（盒已知良全量锁·纯超集 94⊆145·双版本共存非抬升），
  CI run `35185240143` 7/7 首跑全绿——盒上 feature 构建不再重锁。

### 3.18 STAB-O4/FIX 收口（2026-09-17·E4-1 观测完成 + NO-FIX-IN-REPO）

**STAB-O4 首刀 E4-1 = INGEST-ANATOMY 生产观测点（已执行并判读）**

- 实现：中性层 `pipeline_events::IngestAnatomy`（固定容量计数·probe 侧零逐帧
  堆分配）+ controller decklinkvideosrc/audiosrc src pad BUFFER probe + bus
  watch 计数 + bin 诊断模式 30s 采样线程（`E4_INGEST_ANATOMY` info 行·
  diagnostic-only·生产模式零 spawn 零暴露·wire 契约零改动）；commits
  `430d5e7`+`e769cb9`+`9146e1e`（CI 35189959415 7/7；盒 `cargo test e4_` 3×绿）。
  设计先于数据冻结：`c2o3-analysis/c2o4-e4-ingest-anatomy-design.txt`。
- 盒执行：CYCLES=315 DWELL=28 双输入（bin=9146e1e·.rs 78/78 md5==main·零重锁）
  + observer v2.1 同面（COMPLETE/95%/trig 2+）；runner 机械 verdict
  PASS 10/10（informational only·不入梯）。
- 判读（冻结表机判·`c2o4-e4-ingest-anatomy-analysis.txt`）：**R-A 稳态成立**
  ——604 快照 video 速率 24.987-25.033/s·sizes 单指纹·meta 恰
  2.0×GstReferenceTimestampMeta/buffer·pts_backward=0·bus 线性低量 ⇒
  重协商/meta 累积/PTS 抖动/bus 洪流四类 pad 可见异常**排除**；
  **5.5MB 批触不在 pad 可见分配流内**（时窗三点计数全稳）；observer 逐 kB
  对账 ΔRss==ΔVmData==ΔRssAnon·VmSize 平直·批触 = 两离散台阶
  +5588/+5588kB（恰一台阶/输入·rel 7136/7148）落在既有 64MiB 形态匿名预留
  ——RESERVATION-EXTENSION 第五次复现·步量与 c2o2 逐 kB 相同·每输入
  2.50MB/h 对上历史 2.81/2.99（窗效应内差异）。六环：same-reservation/
  magnitude/repeatability 再证；**allocation-path 收窄至 pad 面之下 native
  分配链（仍开放）**；allocator-behavior 不可证（红线内）；root cause 未定。
- **FIX 判定 = NO-FIX-IN-REPO（诚实缺席）**：Rust 可达面（FIX 原则 A–D 作用面：
  显式 retention/pool acquire→release 链）在全部观测面上零表征；对 native
  内部行为做 repo 侧修复 = 无证据投机。后果：修复后 2h/8h/24h ladder
  **不触发**；24h rss_bounded FAIL 原样立档；stability verified 维持未标。
  E4 观测点本身保留（diagnostic-only 诊断能力）。残留路径登记不执行：
  SDK/插件版本变更后重跑 E3A 同构差分。
- 附带发现（独立登记·RUNTIME-HARDEN 候选）：**MainContext::default() 单持有者
  缺陷**——build_pipeline Bus watch 线程 push 进程级默认 context，双输入下
  第二条管线 bus watch 静默缺席（handle2 bus_msgs_total=0 生产实证·致命事件
  通道单管线化）；修复方向 = 每实例独立 `MainContext::new()`。
- 证据：`2026-09-17-c2o4-e4-ingest-anatomy-2p5h{,-observer}/`（21+99 件·md5
  盒=origin）+ 分析文件 + EVIDENCE-INDEX 三行。

### 3.19 RUNTIME-HARDEN entry review（2026-09-17）

Status: **COMPLETE / ADJUDICATED**。

- 入场裁决：进入 RUNTIME-HARDEN；不先做低风险代码洁癖，而先关闭 §3.18 BMD 已实证的多输入 Bus 故障观测缺口。
- RCA 复核确认这是两层缺陷：① `build_pipeline` 每实例 Bus 线程使用进程级 `MainContext::default()`，第二实例 `with_thread_default` 获取失败而静默退出；② Execution Group 形态的 `spawn_execution_group_watchdog()` 只读 `HEALTH_ARCS`/Program observation，**不 drain `ctrl.observe(handle)`**，因此即使 Bus watch 恢复，多输入 Error/EOS/ClockLost 仍不会进入 canonical event→Supervisor 闭环。
- 裁决：拆成两个连续 bounded packets，禁止只改 `MainContext::new()` 后宣称故障闭环修复。`RH-BUS-01` 先恢复 per-pipeline Bus watch 隔离与生命周期；`RH-BUS-02` 再把每输入 Bus event 消费/归因/overflow fatal fallback 收敛到多输入 watchdog。
- 后续 hardening 采用 dependency gate：D1 LifecycleJournal → D3 per-claim TTL → D7 backend direct field 为 Runtime Features 通用前置；D11 Clock timeline 与 D13 timecode hardening 合并到 Clock/Timecode 下一触碰点；D15 在多 flow 能力前；durable idempotency 在持久 Control Plane 前。不得为“清债”阻塞与其无依赖的 Runtime Feature。
- Authority/Contract：**无 frozen Contract 变化**；本裁决只把已有 Runtime truth / failure-first / multi-input isolation 约束转换为执行顺序。

### 3.20 RH-BUS-01 收口（2026-09-17）

Status: **COMPLETE / SOFTWARE + BMD HARDWARE VERIFIED**。

- 实现 commit：`9b32004fa6b9d5f1d6492bd37b15070339f7b22d`。每个 GStreamer pipeline 的 Bus thread 使用私有 `glib::MainContext::new()`；Bus 通过 `Bus::create_watch(...).attach(Some(&ctx))`、stop poll 通过 `timeout_source_new(...).attach(Some(&ctx))` 显式附着同一 context。禁止退回 `add_watch` / `timeout_add_local` 的默认 context 隐式绑定。
- RCA 中间态被测试拦截：仅把 `MainContext::default()` 改成 `new()` 会让 Bus source/stop timer 仍挂默认 context，私有 MainLoop 无 source，导致 stop join 卡死；该半修复**未入库**。
- 软件验证：BMD `/tmp` 非设备 self-test 形态 `cargo fmt --all -- --check` PASS；focused `cargo test --features gstreamer rh_bus_` **2/2 PASS**；full `cargo test --features gstreamer` **268/268 PASS**。
- BMD exact-commit hardware：由 `git archive 9b32004`（archive sha256 `08f278e5…a255d`）构建 `--features bmd,gstreamer`；manifest v5 md5 `7521d17e…43dd`，输入 device-number `0/1`，现存输出 device-number `2` 未停止/未受扰。30s E4 快照：**handle1 bus_msgs_total=65，handle2=65**，两路 video/audio 均持续出帧且 `pts_backward=0`；历史同形 E4 的 handle2=0 缺陷不再复现。`stop_session`=executed，Program teardown + group-watchdog stop 均有日志；Bus/MainContext failure 行=0。
- CI：GitHub Actions run `35214894696`，commit `9b32004…`，**7/7 success**。
- 证据：`evidence/bmd-10.30.15.10/2026-09-17-rh-bus-01-per-pipeline-context/` + `docs/superpowers/reports/2026-09-17-rh-bus-01-per-pipeline-bus-context.md`。
- 边界：RH-BUS-01 只关闭“每 pipeline Bus source 可被独立分发 + lifecycle 可终止”；Execution Group 仍未 drain 每输入 `ctrl.observe(handle)`，以及 fatal sticky 仍为全局单槽——两者由 RH-BUS-02 接管，**不得把 RH-BUS-01 写成多输入故障闭环已完成**。

### 3.21 RH-BUS-02 收口（2026-09-18）

Status: **COMPLETE / SOFTWARE + BMD HARDWARE + CI VERIFIED**。

- 实现 commit：`92d60075161ef080b599ce69a89a482b3798786b`。Execution Group watchdog 对每个 `(device_id, handle)` 每 tick 独立 drain `ctrl.observe(handle)`；handle 错配 fail-closed 拒收，绝不重归因。Error/EOS 先经既有 `Supervisor::ingest` 归一为 exact-device canonical `PipelineFault`，再由 intake drain 后的 canonical RuntimeEvent 派生 Supervisor 决策候选；raw Bus 事实不形成第二条决策 truth path。ClockLost 保持 degraded/no-auto-restart 冻结策略。
- fatal overflow：进程级 `LAST_FATAL_BUS_EVENT` 单槽退出生产路径，改为 `GstInstance` per-handle `fatal_fallback`；成功 send 不写 fallback，只有 channel Full + Error/EOS 落槽，`poll_bus(handle)` drain 后原子 take，stop/recover 随实例生命周期重置。
- software：BMD `/tmp` exact source tree `cargo fmt` PASS；RH-BUS focused GStreamer **12/12 PASS**；full `cargo test --features gstreamer` **278/278 PASS**；协调者 default/mock regression PASS。确定性测试覆盖 Error/EOS exact-device canonicalization、ClockLost no-restart、mismatched-handle rejection、same-device dedup、A/B 独立候选、per-handle overflow fallback 与 successful-send no-duplicate。
- BMD exact-commit：`git archive 92d6007`，archive sha256 `1007ac91de95a861382488abfc5051f660eb96b56e981951d94faa403504536d`，manifest v5 MD5 `7521d17e7fd02e50eb2b0a84374a43dd`。A2-8 dual-input L1–L5+Teardown **10/10 PASS**：A fail→B alive/program advancing；recover A→A bridge 恢复；B fail→A alive；故障归因不跨输入；teardown 完整。
- production group-watchdog 证据：handle 1/2 均出现首次真实 Bus 消费行；同一 30s E4 snapshot `bus_msgs_total=65/65`，两路 video/audio advancing、`pts_backward=0`。`stop_session=executed`；post-stop session=`released`、两 input resource=`available`、Program=null；既有 output device-number 2 进程保持存活。
- CI：codeload 出网故障在 RH-BUS-02 收口期确定性复现，不归因代码。已采用 GitHub Runner 官方 `ACTIONS_RUNNER_ACTION_ARCHIVE_CACHE` 作为主缓解，4 个 external Actions pin immutable SHA，三台 self-hosted runner cache/probe 均 PASS；current-tree run `35248227420` @ `94bf34e659fadc8c8b58a3a252fd36b53fc177db` **7/7 success**，其中此前失败的 `rust-test-matrix@vbmf-ci-02` 已越过 Set up job 并完成真实 default/simulation/mock tests。
- 证据：`evidence/bmd-10.30.15.10/2026-09-17-rh-bus-02-multi-input-fault-isolation/`（14 件 sha256）+ `docs/superpowers/reports/2026-09-18-rh-bus-02-multi-input-bus-fault-isolation.md`。后续 `f05ec60/86669b7/94bf34e` 仅 CI infrastructure；**hardware verified 仍只归 exact Runtime commit `92d6007`**。
- Authority/Contract：无 frozen Runtime Contract 变化；仅关闭既有多输入 failure-observation / attribution 缺口。

### 3.22 RH-LC-01 收口（2026-09-18）

Status: **COMPLETE / SOFTWARE + CI VERIFIED / BMD RUNTIME SMOKE DEFERRED**。

- 实现 commit：`c72ce5dba94fe815299ddfbbe2921f8252df7e59`。`SessionManager::start()` 新增私有 `CompletedStep[]` journal 与单一 `rollback_start_journal()`，materialize / empty-plan / instantiate / partial-allocation / backend.start 失败统一进入同一 rollback owner；create/stop/Program ownership 不变。
- 冻结安全序保持：全部已实例化/已启动 handles 逆序 stop → holder allocations → leases → reservations；`AllocationAcquired` 仅在首个 claim 成功后登记，journal 不记录未完成副作用。
- failure-first：新增双输入“第二句柄 start 失败”测试，机械断言 stop 顺序 `[101,100]`、每句柄恰一次、零 lease/resource orphan、失败会话不回填 handles；既有 materialize/instantiate/instantiate-second/partial-allocation/start failure 全部继续通过。
- Development VM：Rust 1.98.1；mock lib **429/429** + integration **9/9 + 12/12**；default lib **246/246**；fmt + default/mock clippy `-D warnings` PASS；CodeGraph 已同步。
- CI：GitHub Actions `35279581145` @ `c72ce5d…` **7/7 success**；`gstreamer-build --features bmd,gstreamer`、`hardware-test-compile`、`session-lifecycle`、完整 `rust-test-matrix` 均 PASS。
- BMD：exact commit archive 已复制到 `/tmp/vbmf-rh-lc01-c72ce5d`，archive sha256 `78de0709ea7e757d34e1497361cc3fbab725e972441551a6c4a1feba050fdeb9`，manifest v5 MD5 `7521d17e7fd02e50eb2b0a84374a43dd`；旧 `/opt/vbmf-dev/repo` 未改、device-number 2 未触碰。远端 build/runtime smoke 被当前工具安全层拦截，故明确 **DEFERRED**，不得继承旧硬件 PASS。
- Authority/Contract：无 frozen Contract 变化；`PHASE_0_7A_POST_MERGE_DEBT.md` D1 标 CLOSED。验收报告：`docs/superpowers/reports/2026-09-18-rh-lc-01-lifecycle-journal.md`。

### 3.23 RH-RES-01A 收口（2026-09-18）

Status: **COMPLETE / SOFTWARE + CI VERIFIED / BMD NOT REQUIRED**。

- Runtime commit：`0718757e42146cb41bd313e227f5fc7f19f3a171`。Reservation TTL truth 从 Session `created_at_ms` 近似迁入 Resource Registry 的 per-claim lifecycle。
- `Reservation` 持有内部 `expires_at_ms`（`#[serde(skip)]`）；`acquire(req, now, ttl)` 建立独立 deadline；`renew_claims` expected-claims 同锁域 all-or-none；`expire_due` per-claim Expire；`abort_reservations_of` exact-holder Abort。
- Session 不再维护第二套 Reservation 时间真值；Binding 完成作为进度点 Renew，tick 只消费 Registry expired claims 并执行 sibling Abort、lease cleanup、Session→Terminated 收敛。
- failure-first：wrong-holder Renew 拒绝；TTL 已到但未 scan 不得 Renew 复活；multi-claim Renew 失败不得部分延长 sibling；单 claim 到期触发同 Session 其余 claim Abort；wire JSON 不暴露 TTL 内部字段。
- Development VM：Resource focused **7/7**；Session `resource_rt_01_*` **6/6**；mock **432/432 + integration 9/9 + 12/12**；default **248/248**；fmt + default/mock clippy PASS。
- CI：GitHub Actions `35293309933` @ `0718757…` **7/7 success**，含 `gstreamer-build` 与 `hardware-test-compile`。
- Hardware：本 packet 未改 DeckLink/GStreamer adapter 或媒体数据面，BMD runtime/hardware smoke **NOT REQUIRED**；不得写成 hardware verified。
- Authority/Contract：无 frozen Contract / wire vocabulary 变化；D3 标 CLOSED。报告：`docs/superpowers/reports/2026-09-18-rh-res-01a-per-claim-reservation-ttl.md`。

### 3.24 RH-RES-01B 收口（2026-09-18）

Status: **COMPLETE / SOFTWARE + CI VERIFIED / BMD NOT REQUIRED**。

- Runtime commit：`fb8bc81c737bfb668342181e62d0cf903e29c941`；`SessionManager.backend` 从 `OnceLock<Arc<dyn MediaBackend>>` 收敛为 constructor-owned `Arc<dyn MediaBackend>`。
- 全仓 audit：`set_backend()` 真实 caller = **0**；删除 late-injection helper 与“backend 未注入”运行分支，`SessionManager::new(... backend ...)` 继续是唯一注入点。
- `SessionError::BackendUnavailable` 与 error-model 分类仍保留，本 packet 不扩大词表/错误模型清理范围。
- diff 仅 `session.rs` **+5/-19**；start 用 `Arc::clone(&self.backend)`，stop 直接消费同一 constructor instance；MediaBackend/Adapter 行为不变。
- Development VM：mock **433/433 + integration 9/9 + 12/12**；default **249/249**；fmt + default/mock clippy PASS；CodeGraph 已同步。
- CI：GitHub Actions `35293860197` @ `fb8bc81…` **7/7 success**。
- Hardware：constructor representation only，未改 GStreamer/DeckLink 数据面；BMD smoke **NOT REQUIRED**。
- Authority/Contract：无 frozen Contract 变化；D7 标 CLOSED。

### 3.25 Runtime Features entry review（2026-09-18）

Status: **COMPLETE / RECONCILED；RF-FF-01A READY**。

- 旧 `ARCHITECTURE_V0.2.md` Source Adapter 表中的“11 adapters 已实现”属于历史 implementation/status 描述，不能覆盖 live STATE/code/tests。live `SourcePlan` 仅 DeckLink selection + SelfTest，adapter tree 无 network-source adapter；Network Sources 仍是 BACKLOG。
- PACKET_SWITCH 需要 compressed source domain；live execution 明确拒绝 Packet/Master。MASTER_SWITCH 依赖 Normalize，并在触碰时触发 RH-CLOCK-01。Hot-Standby 依赖可用 source/switch；Composition/Audio 会触发 RH-FLOW-01；persistent control-plane 前才触发 RH-IDEM-01。
- `MEDIA_BACKEND_CONTRACT §4` 明确 GStreamer→FFmpeg 是合法替换轴，要求只替换 Backend + RuntimeBinding、不改变 CanonicalPipelinePlan。
- live `PipelinePlan::SourcePlan` 仍含 GStreamer 专属 `device_number / SourceSelectionMode`，与 frozen Backend contract 存在实现漂移。直接做 FFmpegBackend 会迫使 FFmpeg 解释 GStreamer binding，禁止。
- 因此第一 bounded feature packet 裁决为 **RF-FF-01A — Backend-neutral RuntimeBinding extraction（FFmpeg prerequisite）**：先把 backend-specific runtime address 从 canonical plan 分层出去；GraphRuntimeIntent/wire 不变，属于实现对齐 frozen Contract，不是修改 frozen Contract。
- RF-FF-01A 会触及 GStreamer input binding，验收必须包含 BMD exact-commit 双输入 smoke。报告：`docs/superpowers/reports/2026-09-18-runtime-features-entry-review.md`。

## 4. Current Task

**RF-FF-01A — Backend-neutral RuntimeBinding extraction（FFmpeg prerequisite）**。

Scope：按 frozen `MEDIA_BACKEND_CONTRACT §4` 修复实现漂移，把 GStreamer-specific runtime selection/address 从 canonical `PipelinePlan` 分层到 backend RuntimeBinding；保持 `GraphRuntimeIntent`、command/wire vocabulary、Session/Resource ownership 不变，不在本 packet 实现 FFmpegBackend。

Required acceptance：inventory 所有 `SourcePlan.device_number / SourceSelectionMode / provider_persistent_id` 消费面；定义最小 backend-neutral plan + backend-specific binding 边界；GStreamer DeckLink selection 行为不变；default/mock/GStreamer regressions；BMD exact-commit 双输入 start/stop + identity/binding smoke；CI 7/7。

## 5. Next Task

**RF-FF-01B — FFmpegBackend process lifecycle / SelfTest**：仅在 RF-FF-01A 完成 backend-neutral RuntimeBinding 分层并通过 BMD GStreamer regression 后进入 READY。

P2 系列全部收口（§3.4–§3.16）。BMD 实机永走 hardware acceptance 人工线，不进入
普通 PR CI。

### 5.1 Task Queue / Agent Work Packets

> 本节是 `.project/STATE.md` 内的唯一动态任务队列。Agent 只能选择**当前 Phase 中第一个 `READY`** 项；`BLOCKED` / `BACKLOG` 不得越级实施。每个任务完成必须满足 Implementation + Required Verification + Evidence + STATE Update + GitHub Sync。执行者输出只代表 `DONE_NEEDS_REVIEW`，未经协调者 diff/verification 复核不得转 COMPLETE。

| ID | Status | Task / Scope | Depends on | Required acceptance |
|---|---|---|---|---|
| **P2-C1** | **COMPLETE（§3.6，2026-09-15）** | `vbmf-ci-01` current-SHA 正式收口：从 live `main` full SHA 生成 §15.9 bundle，host checksum，Rust 1.98.1 idempotent pin，修复版 V1–V4 verify，以 `vbmf-ci` 刷新 manifest；不得修改 runner sudo 权限 | current live main + §15.9 | exact SHA/bundle checksum；V1–V4 PASS；manifest；无 generic sudo；证据记录 |
| **P2-C2** | **COMPLETE（§3.7，2026-09-15）** | `vbmf-ci-02` 对称 system Rust 1.98.1 provision/verify/manifest；经 devbox/KVM 既有授权管理面执行，host-key 必须严格验证；以 `vbmf-ci` 调用 rust 工具时显式 `RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo`（§3.6 RCA） | P2-C1 | 与 host1 同 exact toolchain/path；二次 pin 幂等；V1–V4 PASS；manifest |
| **P2-C3** | **COMPLETE（§3.8，2026-09-15）** | 双 runner read-only parity re-probe，必须实际命中 01 与 02；验证 Rust path/version、manifest、labels parity | P2-C1/C2 | 两机 evidence；`/usr/local/bin` exact 1.98.1 parity；runner identity 明确 |
| **P2-C4** | **COMPLETE（§3.9，2026-09-15）** | 只迁现有 `rust-format` job 到条件 `runs-on`；fork PR 留 GitHub-hosted，trusted main/same-repo PR 走 `vbmf-general`；其余 6 jobs 不动 | P2-C3 | context 名仍 `rust-format`；fork boundary 不变；7 required checks PASS；实际 self-hosted runner identity evidence |
| **P2-D** | **COMPLETE（§3.10，2026-09-15）** | 阶段1：pin/verify/collect/test 四脚本加 clippy 组件 + 双机 host-admin 幂等补装收口；阶段2：`rust-clippy` 条件 `runs-on` 迁移 | P2-C COMPLETE | V1–V5 双机 PASS；manifest 双证；7 checks PASS；runner identity；STATE |
| **P2-E1** | **COMPLETE（§3.11，2026-09-15）** | `session-lifecycle` grayscale + 双 runner 稳定性观察 | P2-D | required context 不变；调度/失败恢复 evidence |
| **P2-E2** | **COMPLETE（§3.12，2026-09-16）** | `architecture-portability` grayscale | P2-E1 | fork/trusted 双通道回归；7 checks PASS |
| **P2-E3** | **COMPLETE（§3.13，2026-09-16）** | `rust-test-matrix` 最后迁 general pool（含 rustdoc 暴录二阶段） | P2-E2 | matrix 完整 PASS；queue/concurrency 无未解释失败 |
| **P2-M0** | **COMPLETE（§3.14，2026-09-16）** | DeckLink SDK 注入模式裁决；host-local 与 secret injection 二选一，禁止混用 | P2-E3 | 决策落入 CI Strategy/ADR 职责文档；安全与可重复性评审 |
| **P2-M1** | **COMPLETE（§3.15，2026-09-17）** | `hardware-test-compile` migration（仅 capability build，不冒充硬件验证） | P2-M0 | CI PASS；BMD hardware 仍独立人工 acceptance |
| **P2-M2** | **COMPLETE（§3.16，2026-09-17）** | `gstreamer-build` migration（迁 vbmf-media，关闭 D1 空过窗口） | P2-M1 | CI PASS；general/media runner 边界无漂移 |
| **STAB-O3.1** | **COMPLETE（§3.17·2026-09-17·INCONCLUSIVE-at-allocation-path + 候选空间收敛）** | 24h RSS RCA causal discrimination；禁止把相关性写成 root cause | P2-M 全链完成 | observer-only evidence；owner 候选收敛或明确 INCONCLUSIVE |
| **STAB-O4/FIX** | **COMPLETE（§3.18·2026-09-17·E4-1 观测完成 + NO-FIX-IN-REPO）** | 首刀 = E4 型 allocation-path 观测（生产观测点·用户已授权）→ 最小正确修复（FIX 原则 A–D）→ focused/full regression → 2h/8h/24h ladder | STAB-O3.1 COMPLETE | 新 24h `10/10` 前不得标 stability verified；禁止降低 +50MB gate / `malloc_trim` 掩盖 |
| **RUNTIME-HARDEN** | **ACTIVE / DECOMPOSED（§3.19）** | umbrella；不可由 Agent 直接执行 | STAB-O4 COMPLETE | 只执行下列 bounded packet；按 dependency gate 清偿 |
| **RH-BUS-01** | **COMPLETE（§3.20·2026-09-17）** | 每 pipeline 独立 GLib MainContext + Bus-watch lifecycle；双并发 pipeline Bus delivery | §3.18 production defect | focused/full PASS；双 handle Bus evidence；stop/recover 无残留；BMD exact-commit 双输入；CI 7/7 |
| **RH-BUS-02** | **COMPLETE（§3.21·2026-09-18）** | Execution Group 每输入 drain Bus；Error/EOS/ClockLost canonical 归因→Supervisor；fatal overflow fallback per-handle 化 | RH-BUS-01 | focused 12/12 + full GStreamer 278/278；BMD dual-input 10/10 + live Bus 65/65；CI `35248227420` 7/7 |
| **RH-LC-01** | **COMPLETE（§3.22·2026-09-18）** | D1 LifecycleJournal / reverse rollback engine | RH-BUS closure | `CompletedStep[]` + 单一 rollback；mock 429/429 + default 246/246 + integration 9/9+12/12；CI `35279581145` 7/7；BMD smoke deferred |
| **RH-RES-01A** | **COMPLETE（§3.23·2026-09-18）** | D3 per-claim Reservation TTL + Renew/Expire/Abort lifecycle | RH-LC-01 | Resource 7/7 + Session 6/6；mock 432/432 + 9/9 + 12/12；default 248/248；CI `35293309933` 7/7；BMD not required |
| **RH-RES-01B** | **COMPLETE（§3.24·2026-09-18）** | D7 backend `OnceLock`→direct field | RH-RES-01A | one-file +5/-19；set_backend caller=0；mock 433/433 + 9/9+12/12；default 249/249；CI `35293860197` 7/7 |
| **RH-CLOCK-01** | **BACKLOG / DEFER-UNTIL-TOUCH** | D11 Clock observation timeline + D13 timecode release-build hardening | Clock/Timecode feature entry | Locked→Lost→Recovered 事件序列；release fail-closed |
| **RH-FLOW-01** | **BACKLOG / DEFER-UNTIL-TOUCH** | D15 explicit media-flow cardinality | multi-flow Audio/Metadata entry | PortId≠flow；0/1/N flow contract/tests |
| **RH-IDEM-01** | **BACKLOG / DEFER-UNTIL-CONTROL-PLANE** | durable idempotency | persistent external command entry | restart/replay/conflict durability；Runtime remains truth |
| **RUNTIME-FEATURES** | **ACTIVE / DECOMPOSED（§3.25）** | Network Sources、PACKET/MASTER、Hot-Standby、Live FFmpeg、SRS/Output、Recording/Replay、Composition/Audio execution | immediate hardening gates complete | 只执行 bounded packet；先修 frozen Backend contract 的 RuntimeBinding 实现漂移 |
| **RF-ENTRY-01** | **COMPLETE（§3.25·2026-09-18）** | Runtime Features Authority/Reconciliation | RH-RES-01B | 旧 Source status 与 live code reconciliation；deferred hardening touch-gates；首 packet 裁决 |
| **RF-FF-01A** | **READY** | Backend-neutral RuntimeBinding extraction（FFmpeg prerequisite） | RF-ENTRY-01 | canonical plan 不含 GStreamer runtime address；GStreamer behavior regression；BMD exact dual-input smoke；CI 7/7 |
| **RF-FF-01B** | **BACKLOG** | FFmpegBackend process lifecycle / SelfTest | RF-FF-01A | MediaBackend lifecycle + canonical events；不改 plan/wire；再决定 BMD FFmpeg hardware rung |
| **STANDALONE** | **BACKLOG** | production images/compose、readiness、shutdown/restart/upgrade/rollback、current-main BMD deployment reconciliation | Runtime feature slice | standalone install/run/restore；BMD exact commit acceptance |
| **CONTROL-PLANE** | **BACKLOG** | Fastify + PostgreSQL/Drizzle + Worker/BullMQ + Auth/RBAC | Standalone/runtime APIs stable | Rust Runtime remains truth；Fastify 不拥有媒体生命周期 |
| **VBMF-SDK** | **BACKLOG** | 契约测试 + 真实消费者证据后实现 Rust/TS/Python `vbmf-sdk` | stable API consumers | 不暴露 Rust/GStreamer/FFmpeg/vendor/DB internals |
| **WEB-CONSOLE** | **BACKLOG** | React19/Vite/TS 专业 Runtime Console；真实状态→命令→ack→actual→failure→alarm→recovery→reload 一致性 | Control Plane + SDK/API stable | UI 不得本地假成功；完整 operator journey E2E |
| **MOTHER-ADAPTERS** | **BACKLOG** | `media-digital-*` compatible adapters / module integration | Standalone product works alone | 可拔除；VBMF 不依赖母平台启动/运行/恢复 |
| **FEDERATION/V0.3+** | **BACKLOG** | Multi-site、NDI/RIST/Zixi、PTP/Genlock、multi-node HA、WebRTC contribution | explicit V0.3+ authority | 必须走对应版本 Architecture/ADR，不改写 V0.2 frozen contract |

#### Work Packet minimum fields

任何交给 Pi / Claude Code / 其他 AI agent 的写任务，至少明确：`Task ID`、`Authority`、`allowed files/scope`、`forbidden scope`、`acceptance`、`verification environment`。Agent 不得自行从 BACKLOG 挑功能，不得把 executor exit 0 当 COMPLETE，不得自行改变 frozen Contract 或 `.project/STATE.md` 的 Authority 模型。任何 BACKLOG umbrella 在转 READY 前必须先拆成 bounded Work Packets。

## 6. Current Authority

优先级严格如下：

1. 用户最新明确指令 / Project Instructions；
2. 冻结 Architecture / Contract / ADR；
3. **本 `.project/STATE.md`**；
4. live canonical `main`；
5. 真实 code / tests / Runtime / hardware evidence；
6. `ROADMAP.md`、`PHASE_IMPLEMENTATION_MAP.md`、README、历史任务记录、旧聊天、Memory、历史分支 / PR。

Current Task 专项 Authority：`.project/STATE.md` §3.25/§4；`MEDIA_BACKEND_CONTRACT.md §1/§4`；live `pipeline.rs` / `resolver.rs` / GStreamer controller 的 materialize→RuntimeBinding 消费面。RF-FF-01A 只把 backend-specific runtime address 从 canonical plan 分层，不新增 FFmpegBackend、不改变 GraphRuntimeIntent/wire/Session truth。CI Strategy 仅承担 required checks / runner 执行层。

注意：该 Strategy 中形成于分支迁移前的 `master` baseline 描述属于历史证据；操作性命令中的 `--ref master` 等字面量已经因 Git Authority rename 产生迁移债务，P2-B 开工时必须先按 `main` reconciliation，不能把历史分支名重新解释成开发 Authority。

## 7. Verification Level

### Git / governance

- remote default branch `main`: **VERIFIED**
- remote `main` protected: **VERIFIED**
- 7 required contexts preserved: **VERIFIED**
- actual remote `master` ref absent: **VERIFIED**
- `main` migration preserved commit history: **VERIFIED at remote ref level**

### P2-A / P2-B infrastructure

- P2-A implementation: **COMPLETE**；runner parity / dual R Gate / dispatch evidence 已登记于 `db54d45…`。
- P2-B implementation: **COMPLETE / SOFTWARE VERIFIED** at `9d6df61c…`；GitHub Actions `34920528957` 7/7 required jobs PASS。
- P2-C historical preflight probes `34920607255` / `34920611254` / `34920615247` / `34920619055`：当时两台 runner 均被命中且 `rustc/cargo` 均 MISSING；该证据只描述 pin 前基线，当前状态以下列 host execution 证据为准。
- P2-C host-admin tooling：**IMPLEMENTATION COMPLETE / SOFTWARE VERIFIED** through `0f375c8…`；focused tests **41/41 PASS**；GitHub Actions `34934058858` / `34935730648` / `34947630667` / `34949135299` PASS。
- P2-C host route recovery：**VERIFIED**（encrypted probe + strict host-key match + BatchMode SSH + host-admin `sudo -n`）；原 missing-host-admin blocker 已解除。
- `vbmf-ci-01` system Rust 1.98.1：**HOST EXECUTED / FORMAL CURRENT-SHA CLOSURE VERIFIED（P2-C1 COMPLETE，§3.6）**——exact-SHA bundle checksum、幂等 pin 复跑、V1–V4 PASS、`vbmf-ci` manifest @2026-09-15T10:01:21Z、`verify-runner.sh` R1–R5+scope PASS；`vbmf-ci-02` system pin：**HOST EXECUTED / VERIFIED（P2-C2 COMPLETE，§3.7）**；双机 parity：**VERIFIED（P2-C3，§3.8；probe env 修正后两机逐字符 1.98.1 parity）**；`rust-format` self-hosted grayscale：**COMPLETE / CI VERIFIED（P2-C4，§3.9）**；`rust-clippy` self-hosted grayscale：**COMPLETE / HOST + CI VERIFIED（P2-D，§3.10；双机 V1–V5 含 clippy、run `35027740186` 7/7，`rust-clippy`→`vbmf-ci-02`）**；`session-lifecycle` self-hosted grayscale：**COMPLETE / CI VERIFIED（P2-E1，§3.11；run `35033319131` 7/7，`session-lifecycle`→`vbmf-ci-01`，50 passed/0 failed）**；`architecture-portability` self-hosted grayscale：**COMPLETE / CI VERIFIED（P2-E2，§3.12；run `35061944269` 7/7 首跑全绿，`architecture-portability`→`vbmf-ci-01`）**；`rust-test-matrix` self-hosted grayscale：**COMPLETE / HOST + CI VERIFIED（P2-E3，§3.13；双机 V1–V6 含 rustdoc、run 7/7 首跑全绿，`rust-test-matrix`→`vbmf-ci-01`，Doc-tests 0 failed）**；P2-M0：**ADJUDICATED（§3.14，裁决 B）**；`hardware-test-compile` media migration：**COMPLETE / CI VERIFIED（P2-M1，§3.15；run `35172539232` rerun 后 7/7，@vbmf-ci-media，SDK 16.0.0 host 预装）**；`gstreamer-build` media migration：**COMPLETE / CI VERIFIED（P2-M2，§3.16；run `35177980436` 7/7 首跑全绿，@vbmf-ci-media，artifact 12331677B，D1 窗口关闭）**。

### Software / Runtime

- RH-LC-01 Runtime implementation `c72ce5d…`：**IMPLEMENTATION COMPLETE / SOFTWARE VERIFIED / CI VERIFIED**。
- Development VM：`cargo fmt` PASS；mock lib **429/429** + integration **9/9 + 12/12**；default lib **246/246**；default/mock clippy `-D warnings` PASS。
- GitHub Actions `35279581145` @ `c72ce5d…`：**7/7 required jobs PASS**，含真实 `gstreamer-build --features bmd,gstreamer` 与 `hardware-test-compile`。
- 既有 RH-BUS hardware evidence 只证明其 exact commits；不得继承到 `c72ce5d`。
- RH-RES-01A `0718757…`：**IMPLEMENTATION COMPLETE / SOFTWARE + CI VERIFIED**；Resource 7/7、Session resource 6/6、mock 432/432 + integration 9/9+12/12、default 248/248、fmt/clippy PASS；Actions `35293309933` 7/7 PASS。
- RH-RES-01B `fb8bc81…`：**IMPLEMENTATION COMPLETE / SOFTWARE + CI VERIFIED**；mock 433/433 + integration 9/9+12/12、default 249/249、fmt/clippy PASS；Actions `35293860197` 7/7 PASS。

### Hardware

- RH-LC-01 exact `c72ce5d` 已生成并复制 BMD `/tmp` archive（sha256 `78de0709…50fdeb9`），manifest v5 MD5 `7521d17e…43dd` 已核对；现存 output device-number 2 未触碰。
- 当前远程 exact-commit build/runtime smoke 命令被工具安全层拦截，故 **RH-LC-01 BMD runtime/hardware smoke = DEFERRED**；不得写成 verified。
- RH-RES-01A 未修改 DeckLink/GStreamer adapter、Pipeline backend 或媒体数据面；**BMD runtime/hardware smoke = NOT REQUIRED**，也不得写成 hardware verified。
- RH-RES-01B 仅 constructor ownership representation；**BMD runtime/hardware smoke = NOT REQUIRED**。RF-FF-01A 将触及 GStreamer runtime binding，因此必须重新做 exact-commit BMD 双输入 smoke。
- 后续涉及 DeckLink/GStreamer/FFmpeg/SRT/switch/timing/failover 的 Runtime 变更仍必须按任务范围重新做 BMD exact-commit verification。

### Stability

**NOT VERIFIED / verification debt remains.**

已登记 Step 15 历史：

- 2h rung PASS；
- 首次 8h 因 S15-E01 evidence bookkeeping race FAIL；
- S15-E01 fix3 后 8h rerun PASS 10/10；
- fix3 后 24h rung **FAIL 9/10**，唯一失败 `rss_bounded`：首/末 1/3 均值约 `+86.6 MB`，超过 `+50 MB` 门槛；
- B1-DIAG 将增长分类为 `ANON_GROWTH`；后续 C1/C2/O1/O2/O3-0 + E2/E3A（2026-09-17
  恢复登记·§3.17）已继续缩小 RCA：双输入/program graph/切换/make_mut/selftest
  源族路径均排除为必要条件，DeckLink 源族配置为必要条件之一，候选空间收敛至
  「每输入 DeckLink 源族 ingest native 分配链 × allocator reservation/arena
  行为」；allocation-path/allocator-behavior 两环未证，root cause 未定。但 main
  上尚无经完整回归 + 新 24h rung 证明关闭该债务的最终结果。
- RH-BUS-01（2026-09-17·§3.20）：software focused 2/2 + full gstreamer 268/268 PASS；BMD exact `9b32004` 双输入 E4 同快照 handle1/2 `bus_msgs_total=65/65`，stop_session teardown 完整；CI `35214894696` 7/7 PASS。
- RH-BUS-02（2026-09-18·§3.21）：Runtime commit `92d6007`；focused 12/12 + full gstreamer 278/278 PASS；BMD dual-input 10/10 + production group-watchdog Bus `65/65` + stop/release 完整；current-tree CI `35248227420` 7/7 PASS。
- STAB-O4 E4-1（2026-09-17·§3.18）：pad 可见分配流全稳 + 批触不在 pad 面
  （两离散台阶 +5588/+5588kB·一台阶/输入·既有预留内 page-commit·第五次复现）
  ⇒ FIX 判定 NO-FIX-IN-REPO·ladder 不触发——该 FAIL 债务在现证据下无 repo 侧
  可修路径，维持立档直至外部变量（SDK/插件版本）变更后重开。

因此不得写成“24h stability verified”。

## 8. Risks / Blockers

### Active risks

1. **24h RSS stability debt**：`rss_bounded` 历史 FAIL 尚未由最终修复 + 新 24h rung 关闭。
2. **P2-C toolchain prerequisite — CLOSED**：双机 exact 1.98.1 pin + current-SHA 收口 + parity re-probe 全部完成（§3.6/§3.7/§3.8）。P2-C4 迁 `rust-format.runs-on` 前置满足；self-hosted job env 必须带显式 `RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo`（§3.6 RCA，probe 已实证不带则报错）。
3. **shared physical failure domain**：`vbmf-ci-01` 与 `vbmf-ci-02` 同 devbox 物理故障域，只提供维护冗余/并发/parity，不等价于物理 HA。
4. **BMD deployment divergence**：`/opt/vbmf-dev/repo` 当前 detached at `7cc33dd…` 且有未提交 ops 改动，明显落后 live main；当前无 VBMF container/service 运行。不得把该旧 deployment 的 hardware evidence 继承给当前 main。
5. **BMD device occupancy**：历史手工 `gst-launch ... decklinkvideosink device-number=2` 仍在运行；RH-BUS-01 已证明可在不扰动该输出的前提下只使用 manifest 输入 `0/1` 完成验收。后续若任务需要输出卡 2，仍须先归属确认/受控释放，禁止抢占。
6. **closed PR #31 / branch convergence**：PR #31 已随 2026-09-15 分支收敛关闭（head 分支已删）；它从来不是 canonical development Authority；未来若吸收 standalone product baseline，按 closed PR #31 做 main-relative scope audit/reconciliation，不恢复任何分支。
7. **historical local edit provenance**：旧 STATE 记录的另一 checkout 未提交 `DEPLOYMENT_AND_DEV_RUNTIME.md` 修改未出现在当前新 checkout；原工作区未重新取得前不可判定其去留。

8. **runner 出网抖动 / codeload — MITIGATED + VERIFIED（2026-09-18）**：历史故障窗与失败证据继续保留；RH-BUS-02 收口期再次确定性复现 `codeload.github.com` Action archive HttpClient 100s×3 timeout，证明“失败后 rerun”不足。已升级为两层缓解：git checkout/fetch 继续使用 host system git proxy；Action setup 主路径改用 GitHub Runner 官方 `ACTIONS_RUNNER_ACTION_ARCHIVE_CACHE`，workflow external Actions 全 pin immutable SHA。三台 runner maintenance + probe 均通过：media `35247466755/35247550654`，general `vbmf-ci-01` probe `35248095276`、`vbmf-ci-02` probe `35248099277`（后者 direct mode 亦 cache 4/4 PASS）；current-tree required run `35248227420` **7/7 PASS**。loopback HTTP proxy 仅保留 cache maintenance/miss fallback；继续观察但不再是 Current Task blocker。

### Current blockers

- P2-B: **NONE / COMPLETE**.
- P2-C/P2-D/P2-E/P2-M0/P2-M1/P2-M2: **NONE / COMPLETE（§3.6–§3.16）**；**Phase 2 灰度阶梯全部走完**。`vbmf-ci` service account 继续无 generic sudo/root；禁止 job-local rolling `stable` 或绕过双机 parity。
- STAB-O3.1: **NONE / COMPLETE（§3.17）**。
- STAB-O4/FIX: **NONE / COMPLETE（§3.18）**。
- RH-BUS-01: **NONE / COMPLETE（§3.20）**。
- RH-BUS-02: **NONE / COMPLETE（§3.21）**。
- RH-LC-01: **NONE / COMPLETE（§3.22）**。
- RH-RES-01A: **NONE / COMPLETE（§3.23）**。
- RH-RES-01B: **NONE / COMPLETE（§3.24）**。
- RF-ENTRY-01: **NONE / COMPLETE（§3.25）**。
- RF-FF-01A: **NONE / READY**。

## 9. Verification Debt

- 24h stability：需要在 RCA / fix 真正关闭后重新跑完整 24h rung；旧 FAIL 不得被短跑或诊断 run 覆盖。
- branch rename hygiene：**P2-B CLOSED**；workflow/runbook 操作性路径已统一 `main`，required context 名称未变。
- P2-C/P2-D/P2-E：全部收口（§3.6–§3.13）。fork `runs-on` 分支未 live 实测（无现成 fork PR），已在 §3.9 登记残余风险与核验口径。
- runner 出网稳定性：codeload 缓解已从 rerun 升级为 immutable Action archive cache，并已在 general×2 + media×1 probe 及 required CI `35248227420` 7/7 验证；残余仅为持续观察 archive cache miss/Action SHA 更新流程，不再是未实施 Verification Debt。
- Development agent：Pi/Claude Code 项目级 auth + real model smoke 已 PASS；Claude project trust PASS；后续长写任务优先用 `_shared` runner + tmux，不再登记 agent smoke debt。
- BMD：RH-BUS-01 exact `9b32004`（§3.20）与 RH-BUS-02 exact `92d6007`（§3.21）均已有独立 hardware evidence；RH-LC-01 exact `c72ce5d` archive/manifest 已准备但 runtime smoke 因当前工具通道限制 **DEFERRED**。实际 `/opt/vbmf-dev/repo` deployment 仍落后且带未提交 ops 改动，未做破坏性同步。后续 packet 不得继承前述硬件证据。
- PR #31：已随分支收敛 CLOSED；若未来吸收 standalone product baseline，必须先按 closed PR #31 做 main-relative scope audit/reconciliation，不恢复 feature 分支 Authority。

## 10. Cold-Start Handoff

新 Chat / Work 必须按以下最小链恢复，不重新扫描整个仓库：

1. 读取 Project Instructions；
2. 确认仓库 `pwl1987/VBMF`、默认分支和 canonical branch 都是 `main`；
3. 读取 live `main` HEAD；
4. 读取本 `.project/STATE.md`；
5. 找到“包含当前 STATE 版本的 commit”，比较 live HEAD 是否有更新；若有，只 reconcile STATE 之后的新 commits；
6. 读取 **Current Task = RF-FF-01A** 对应 Authority：`.project/STATE.md` §3.25/§4 + `MEDIA_BACKEND_CONTRACT.md §1/§4` + `pipeline.rs`/`resolver.rs`/GStreamer controller 的 canonical plan 与 runtime binding 消费面；
7. 核对 P2-C implementation chain through `0f375c8…`、maintenance failure `34921727757`、encrypted route probes 与最新 `media-agent CI`（P2-M2 后 `gstreamer-build` 应 @vbmf-media 且 artifact 非空）；
8. 读取 §5.1 Task Queue，只执行当前 Phase 第一个 `READY` Work Packet；立即清偿 hardening 链已完成；RF-ENTRY-01 已 reconciliation；**当前 RF-FF-01A READY**；
9. 不回退到已经 COMPLETE 的 0.6 / 0.7 / A2-8 / P2-A；
10. 不从历史 feature/fix/实验 branch 恢复开发；所有验证通过的改动直接推进 `main`；
11. 完成独立任务后，同一轮更新本 STATE 的 Last Completed / Current Task / Next Task / verification / risks / debt / handoff；
12. 当前标准 Development VM checkout 应直接观察 working tree；只有执行环境确实失去该 checkout 可见性时才使用 `Local Working Tree: NOT OBSERVABLE`。

### Expected cold-start conclusion at this state

在 live `main` 没有比本 STATE 更新的实质提交时，正确恢复结果应是：

- Git Authority 已完成 `master → main`；
- state system 已初始化；
- P2-A 已完成；
- P2-B 已完成并经 Actions 7/7 PASS；
- **Phase 2 全链完成**：P2-C–P2-E、P2-M0–P2-M2 收口（§3.6–§3.16）；7 required job 全部 self-hosted 条件灰度（5 general + 2 media），GitHub-hosted 仅余 fork 回退；
- **STAB-O3.1 已收口**（E2/E3A 恢复登记 + INCONCLUSIVE-at-allocation-path + 候选空间收敛·§3.17）；
- **STAB-O4/FIX 已收口**（§3.18：E4-1 观测完成 + NO-FIX-IN-REPO·ladder 不触发·24h FAIL 立档）；
- **RUNTIME-HARDEN immediate gates 已完成（§3.20–§3.24）；Runtime Features entry review 已完成（§3.25）；Current Task = RF-FF-01A，Next Task = RF-FF-01B**；
- Runtime / Web 新业务功能当前不应越过 §5.1 queue 推进；
- 24h RSS stability 仍是明确 verification debt，不能宣称 stability verified。
