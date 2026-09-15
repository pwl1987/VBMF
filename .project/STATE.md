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

**Governance / CI Infrastructure Phase 2 — P2-C rust-format grayscale preparation**

当前不是 Runtime / Web 新业务功能开发阶段。P2-B 已在 `main` 完成并通过真实 GitHub Actions；当前进入 P2-C 前置与单 job self-hosted 灰度。旧 `ROADMAP.md` / `PHASE_IMPLEMENTATION_MAP.md` 中更早的 “NEXT” 不得重新成为当前任务。

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

## 4. Current Task

**P2-C — `rust-format` single-job self-hosted grayscale**

Current Task Authority:

- `docs/architecture/CI_RUNNER_STRATEGY.md` §15.2 / §15.5 / §15.6 / §15.7 / §15.8 / §15.9；
- `.github/workflows/media-agent.yml` 与只读 `.github/workflows/ci-infra-probe.yml`；
- P2-B implementation `9d6df61c…` + Actions run `34920528957`；
- P2-C implementation chain through `0f375c8…`，含 encrypted route probe 与 verifier authority isolation；Actions runs `34934058858` / `34935730648` / `34947630667` / `34949135299`.

P2-C 顺序必须保持：

1. 两台 `vbmf-general` runner 对称完成**同一具体 Rust 版本**的 system-level pin（`/usr/local/rustup` + `/usr/local/cargo` + `/usr/local/bin` 可见），禁止单机漂移；
2. 重新 probe 两台，证明 `rustc/cargo/rustfmt` version/path parity；
3. 只迁 `rust-format` 一个 job；同一 job/context 使用条件 `runs-on`：fork PR = GitHub-hosted，trusted main push / same-repo PR = `vbmf-general`；
4. required context 仍名为 `rust-format`，其余 6 jobs 保持 GitHub-hosted；
5. focused validation → push main → Actions 全绿 → runner identity evidence → STATE。

2026-09-15 P2-C1 收口后前置证据：早期 4 次 `ci-infra-probe` 确认两机最初均 `rustc/cargo: MISSING`，maintenance run `34921727757` 证明 CI service account 无非交互 sudo且无系统修改；随后 encrypted route probe 恢复了既有 out-of-band host-admin 管理路径并严格核验 host identity。`vbmf-ci-01` 已完成 Rust `1.98.1` system pin、幂等执行、修复版 verifier V1–V4 实机 PASS 及 current-SHA bundle/manifest 正式收口（§3.6）；**尚待** `vbmf-ci-02` 对称 pin、双机 parity re-probe。只有这些证据齐备后才允许迁 `rust-format.runs-on`。

## 5. Next Task

**P2-D — `rust-clippy` self-hosted grayscale**，仅在 P2-C 全绿且双 runner pin/parity 证据成立后进入。

后续队列保持：

`P2-C rust-format → P2-D rust-clippy → P2-E session-lifecycle / architecture-portability / rust-test-matrix → P2-M media migration`。

P2-M 前仍须单独裁决 DeckLink SDK 注入模式；BMD 实机永走 hardware acceptance 人工线，不进入普通 PR CI。

### 5.1 Task Queue / Agent Work Packets

> 本节是 `.project/STATE.md` 内的唯一动态任务队列。Agent 只能选择**当前 Phase 中第一个 `READY`** 项；`BLOCKED` / `BACKLOG` 不得越级实施。每个任务完成必须满足 Implementation + Required Verification + Evidence + STATE Update + GitHub Sync。执行者输出只代表 `DONE_NEEDS_REVIEW`，未经协调者 diff/verification 复核不得转 COMPLETE。

| ID | Status | Task / Scope | Depends on | Required acceptance |
|---|---|---|---|---|
| **P2-C1** | **COMPLETE（§3.6，2026-09-15）** | `vbmf-ci-01` current-SHA 正式收口：从 live `main` full SHA 生成 §15.9 bundle，host checksum，Rust 1.98.1 idempotent pin，修复版 V1–V4 verify，以 `vbmf-ci` 刷新 manifest；不得修改 runner sudo 权限 | current live main + §15.9 | exact SHA/bundle checksum；V1–V4 PASS；manifest；无 generic sudo；证据记录 |
| **P2-C2** | **READY** | `vbmf-ci-02` 对称 system Rust 1.98.1 provision/verify/manifest；经 devbox/KVM 既有授权管理面执行，host-key 必须严格验证；以 `vbmf-ci` 调用 rust 工具时显式 `RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo`（§3.6 RCA） | P2-C1 | 与 host1 同 exact toolchain/path；二次 pin 幂等；V1–V4 PASS；manifest |
| **P2-C3** | **BLOCKED by P2-C1/C2** | 双 runner read-only parity re-probe，必须实际命中 01 与 02；验证 Rust path/version、manifest、labels parity | P2-C1/C2 | 两机 evidence；`/usr/local/bin` exact 1.98.1 parity；runner identity 明确 |
| **P2-C4** | **BLOCKED by P2-C3** | 只迁现有 `rust-format` job 到条件 `runs-on`；fork PR 留 GitHub-hosted，trusted main/same-repo PR 走 `vbmf-general`；其余 6 jobs 不动 | P2-C3 | context 名仍 `rust-format`；fork boundary 不变；7 required checks PASS；实际 self-hosted runner identity evidence |
| **P2-D** | **BLOCKED by P2-C4** | `rust-clippy` 单 job self-hosted grayscale，同 P2-B 安全边界 | P2-C COMPLETE | 7 checks PASS；runner identity；STATE |
| **P2-E1** | **BLOCKED by P2-D** | `session-lifecycle` grayscale + 双 runner 稳定性观察 | P2-D | required context 不变；调度/失败恢复 evidence |
| **P2-E2** | **BLOCKED by P2-E1** | `architecture-portability` grayscale | P2-E1 | fork/trusted 双通道回归；7 checks PASS |
| **P2-E3** | **BLOCKED by P2-E2** | `rust-test-matrix` 最后迁 general pool | P2-E2 | matrix 完整 PASS；queue/concurrency 无未解释失败 |
| **P2-M0** | **BLOCKED by P2-E3** | DeckLink SDK 注入模式裁决；host-local 与 secret injection 二选一，禁止混用 | P2-E3 | 决策落入 CI Strategy/ADR 职责文档；安全与可重复性评审 |
| **P2-M1** | **BLOCKED by P2-M0** | `hardware-test-compile` migration（仅 capability build，不冒充硬件验证） | P2-M0 | CI PASS；BMD hardware 仍独立人工 acceptance |
| **P2-M2** | **BLOCKED by P2-M1** | `gstreamer-build` migration | P2-M1 | CI PASS；general/media runner 边界无漂移 |
| **STAB-O3.1** | **BACKLOG after P2-M** | 24h RSS RCA 从 C2-O3-0 继续 causal discrimination；禁止把相关性写成 root cause | P2-M | observer-only evidence；owner 候选收敛或明确 INCONCLUSIVE |
| **STAB-O4/FIX** | **BACKLOG** | 定位 owner → 最小正确修复 → focused/full regression → 2h/8h/24h ladder | STAB-O3.1 | 新 24h `10/10` 前不得标 stability verified；禁止降低 +50MB gate / `malloc_trim` 掩盖 |
| **RUNTIME-HARDEN** | **BACKLOG** | D1 LifecycleJournal、D3 per-claim TTL、D7 backend OnceLock、D11 Clock timeline、D13 timecode hardening、D15 media-flow cardinality、durable idempotency、多输入独立 watchdog | CI + stability closure | 每项独立 change；failure-first tests；涉及硬件则 BMD exact-commit evidence |
| **RUNTIME-FEATURES** | **BACKLOG** | Network Sources（SRT/RTMP/HLS/RTSP/RTP/UDP/WebRTC/File）、PACKET/MASTER switch、完整 Hot-Standby、Live FFmpeg、SRS/Output、Recording/Replay、Composition/Audio execution | hardening entry review | 逐能力 frozen Contract 对齐；不得制造第二 Runtime truth |
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

Current Task 专项 Authority：`docs/architecture/CI_RUNNER_STRATEGY.md`。

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
- `vbmf-ci-01` system Rust 1.98.1：**HOST EXECUTED / FORMAL CURRENT-SHA CLOSURE VERIFIED（P2-C1 COMPLETE，§3.6）**——exact-SHA bundle checksum、幂等 pin 复跑、V1–V4 PASS、`vbmf-ci` manifest @2026-09-15T10:01:21Z、`verify-runner.sh` R1–R5+scope PASS；`vbmf-ci-02` system pin：**NOT RUN**；双机 parity / `rust-format` self-hosted grayscale：**NOT RUN**。不得把 host1 单机成功记成 P2-C COMPLETE。

### Software / Runtime

- 本轮没有 Runtime code change。
- 本轮未执行新的 focused Rust tests / full matrix / build；不得把历史 PASS 自动宣称为本状态初始化提交的新 PASS。
- 当前 live Runtime 代码的既有测试与硬件证据只能证明其登记时对应的 exact commit / binary / scope。

### Hardware

- 历史 DeckLink / real-machine evidence 存在于已登记提交与 evidence 中。
- **hardware verification for this state-init HEAD: NOT RUN / NOT REQUIRED for docs-only governance changes.**
- 后续涉及 DeckLink/GStreamer/FFmpeg/SRT/switch/timing/failover 的 Runtime 变更必须重新按任务范围判定 hardware verification。

### Stability

**NOT VERIFIED / verification debt remains.**

已登记 Step 15 历史：

- 2h rung PASS；
- 首次 8h 因 S15-E01 evidence bookkeeping race FAIL；
- S15-E01 fix3 后 8h rerun PASS 10/10；
- fix3 后 24h rung **FAIL 9/10**，唯一失败 `rss_bounded`：首/末 1/3 均值约 `+86.6 MB`，超过 `+50 MB` 门槛；
- B1-DIAG 将增长分类为 `ANON_GROWTH`；后续 C1/C2/O1/O2/O3-0 已继续缩小 RCA，但 main 上尚无经完整回归 + 新 24h rung 证明关闭该债务的最终结果。

因此不得写成“24h stability verified”。

## 8. Risks / Blockers

### Active risks

1. **24h RSS stability debt**：`rss_bounded` 历史 FAIL 尚未由最终修复 + 新 24h rung 关闭。
2. **P2-C toolchain prerequisite — ACTIVE / HOST1 CLOSED**：host-admin 通道已恢复并严格验证；`vbmf-ci-01` 已完成 system pin + current-SHA 正式收口（§3.6）。仍须 `vbmf-ci-02` 对称 pin + 双机 parity re-probe，完成前禁止迁 `rust-format`。`vbmf-ci` 调用 rust 工具需显式 `RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo`（§3.6 RCA），P2-C4 self-hosted job env 必须据此设计。
3. **shared physical failure domain**：`vbmf-ci-01` 与 `vbmf-ci-02` 同 devbox 物理故障域，只提供维护冗余/并发/parity，不等价于物理 HA。
4. **BMD deployment divergence**：`/opt/vbmf-dev/repo` 当前 detached at `7cc33dd…` 且有未提交 ops 改动，明显落后 live main；当前无 VBMF container/service 运行。不得把该旧 deployment 的 hardware evidence 继承给当前 main。
5. **BMD device occupancy**：DeckLink driver/devnodes/plugins 当前可见，但有历史手工 `gst-launch ... decklinkvideosink device-number=2` 进程持续占用设备；任何后续 hardware acceptance 前必须先归属确认/受控释放，禁止抢占。
6. **closed PR #31 / branch convergence**：PR #31 已随 2026-09-15 分支收敛关闭（head 分支已删）；它从来不是 canonical development Authority；未来若吸收 standalone product baseline，按 closed PR #31 做 main-relative scope audit/reconciliation，不恢复任何分支。
7. **historical local edit provenance**：旧 STATE 记录的另一 checkout 未提交 `DEPLOYMENT_AND_DEV_RUNTIME.md` 修改未出现在当前新 checkout；原工作区未重新取得前不可判定其去留。

### Current blockers

- P2-B: **NONE / COMPLETE**.
- P2-C: **NO EXTERNAL BLOCKER; EXECUTION IN PROGRESS**。合法 out-of-band host-admin 通道已经恢复；当前剩余是 P2-C1→P2-C4 的顺序执行工作。`vbmf-ci` service account 继续无 generic sudo/root；禁止 job-local rolling `stable` 或绕过双机 parity。

## 9. Verification Debt

- 24h stability：需要在 RCA / fix 真正关闭后重新跑完整 24h rung；旧 FAIL 不得被短跑或诊断 run 覆盖。
- branch rename hygiene：**P2-B CLOSED**；workflow/runbook 操作性路径已统一 `main`，required context 名称未变。
- P2-C：host1 已完成 current-SHA bundle/manifest 正式收口（§3.6）；仍欠 host2 对称 pin、双机 parity re-probe、`rust-format` grayscale。
- Development agent：Pi/Claude Code 项目级 auth + real model smoke 已 PASS；Claude project trust PASS；后续长写任务优先用 `_shared` runner + tmux，不再登记 agent smoke debt。
- BMD：当前 main 的 Runtime/hardware verification **deferred / not required for P2-B/P2-C CI-only changes**；BMD deployment 需未来按 exact commit 重建后才能产生新 evidence。
- PR #31：已随分支收敛 CLOSED；若未来吸收 standalone product baseline，必须先按 closed PR #31 做 main-relative scope audit/reconciliation，不恢复 feature 分支 Authority。

## 10. Cold-Start Handoff

新 Chat / Work 必须按以下最小链恢复，不重新扫描整个仓库：

1. 读取 Project Instructions；
2. 确认仓库 `pwl1987/VBMF`、默认分支和 canonical branch 都是 `main`；
3. 读取 live `main` HEAD；
4. 读取本 `.project/STATE.md`；
5. 找到“包含当前 STATE 版本的 commit”，比较 live HEAD 是否有更新；若有，只 reconcile STATE 之后的新 commits；
6. 读取 **Current Task = P2-C** 对应 Authority：`docs/architecture/CI_RUNNER_STRATEGY.md` §15.2 / §15.5 / §15.6 / §15.7 / §15.8 / §15.9；
7. 核对 P2-C implementation chain through `0f375c8…`、maintenance failure `34921727757`、encrypted route probes 与最新 `media-agent CI`；
8. 读取 §5.1 Task Queue，只执行当前 Phase 第一个 `READY` Work Packet；P2-C1 已 COMPLETE（§3.6），当前从 **P2-C2** 开始，按 P2-C2→C3→C4 顺序，不重新寻找 host-admin 通道；
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
- **Current Task = P2-C rust-format single-job grayscale**；P2-C1 host1 current-SHA closure 已完成（§3.6）；当前第一个 `READY` Work Packet = **P2-C2 host2 对称 pin**；
- **Next Task = P2-D rust-clippy self-hosted grayscale**（仅 P2-C COMPLETE 后）；
- Runtime / Web 新业务功能当前不应越过 §5.1 queue 推进；
- 24h RSS stability 仍是明确 verification debt，不能宣称 stability verified。
