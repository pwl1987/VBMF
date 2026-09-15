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

Status: **IMPLEMENTATION COMPLETE / SOFTWARE VERIFIED; HOST EXECUTION BLOCKED**

Implementation chain: `5a9ae16857ff28b29ca6f2da3f2db5acefe4fd37` → `cf1e07d1cbd8e8f304bfaf8a3fac8fd0d6408cc7`.

- exact system pin 固定为 Rust `1.98.1`，root-only `pin-system-rust.sh` + read-only `verify-system-rust.sh` 已落地；
- `prepare-system-rust-bundle.sh` 只接受 full 40-hex exact commit SHA，从该 commit byte-exact 导出 pin/verify/collect 三脚本并生成 deterministic `SHA256SUMS`；拒绝 floating ref / short SHA / missing script / 非空目标目录；
- focused non-root / zero-network tests：`scripts/ci/test-system-rust-scripts.sh` **36/36 PASS**；ShellCheck（本轮 touched scripts）PASS；docs targeted check / `git diff --check` PASS；
- GitHub Actions：`5a9ae16…` run `34934058858` PASS；`cf1e07d…` run `34935730648` PASS；
- rejected CI-root maintenance path 的真实 evidence：run `34921727757` 同时命中 `vbmf-ci-01` / `vbmf-ci-02`，两机都在 `sudo -n true` fail，system pin step skipped，**无系统修改发生**；
- CI service account `vbmf-ci` 继续保持无通用 sudo/root；唯一合法执行平面是 Strategy §15.9 的 out-of-band host-admin runbook。

本子任务不等于 P2-C 完成：两台 runner 尚未实际执行 system pin / parity re-probe，`rust-format.runs-on` 仍未迁移。

## 4. Current Task

**P2-C — `rust-format` single-job self-hosted grayscale**

Current Task Authority:

- `docs/architecture/CI_RUNNER_STRATEGY.md` §15.2 / §15.5 / §15.6 / §15.7 / §15.8 / §15.9；
- `.github/workflows/media-agent.yml` 与只读 `.github/workflows/ci-infra-probe.yml`；
- P2-B implementation `9d6df61c…` + Actions run `34920528957`；
- P2-C host-admin tooling commits `5a9ae16…` / `cf1e07d…` + Actions runs `34934058858` / `34935730648`.

P2-C 顺序必须保持：

1. 两台 `vbmf-general` runner 对称完成**同一具体 Rust 版本**的 system-level pin（`/usr/local/rustup` + `/usr/local/cargo` + `/usr/local/bin` 可见），禁止单机漂移；
2. 重新 probe 两台，证明 `rustc/cargo/rustfmt` version/path parity；
3. 只迁 `rust-format` 一个 job；同一 job/context 使用条件 `runs-on`：fork PR = GitHub-hosted，trusted main push / same-repo PR = `vbmf-general`；
4. required context 仍名为 `rust-format`，其余 6 jobs 保持 GitHub-hosted；
5. focused validation → push main → Actions 全绿 → runner identity evidence → STATE。

2026-09-15 当前前置证据：4 次 `ci-infra-probe` 分别命中 `vbmf-ci-01` / `vbmf-ci-02`，两机均报告 `rustc: MISSING` / `cargo: MISSING`；maintenance run `34921727757` 又证明两机 `vbmf-ci` 均无非交互 sudo，且在任何 system mutation 前 fail-closed。因此 system-level pin **尚未满足**。当前 Development VM / `_shared` 没有登记 devbox host-admin 地址/凭据，且本 VM 无本地 libvirt 管理面；取得合法 out-of-band host-admin 通道后才可执行 §15.9。

## 5. Next Task

**P2-D — `rust-clippy` self-hosted grayscale**，仅在 P2-C 全绿且双 runner pin/parity 证据成立后进入。

后续队列保持：

`P2-C rust-format → P2-D rust-clippy → P2-E session-lifecycle / architecture-portability / rust-test-matrix → P2-M media migration`。

P2-M 前仍须单独裁决 DeckLink SDK 注入模式；BMD 实机永走 hardware acceptance 人工线，不进入普通 PR CI。

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
- P2-C preflight probes `34920607255` / `34920611254` / `34920615247` / `34920619055`：两台 runner 均被命中，`rustc/cargo` 均 MISSING，故 system-level Rust pin 尚未 verified。
- P2-C host-admin tooling：**IMPLEMENTATION COMPLETE / SOFTWARE VERIFIED** at `5a9ae16…` / `cf1e07d…`；focused tests 36/36 PASS，GitHub Actions `34934058858` / `34935730648` PASS。
- P2-C host execution / parity / `rust-format` self-hosted grayscale：**NOT RUN / BLOCKED by missing authorized host-admin channel**；不得把 tooling PASS 记成 runner pin 或 P2-C COMPLETE。

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
2. **P2-C toolchain prerequisite — ACTIVE**：2026-09-15 双 runner probe 已确认 `rustc/cargo` 均不在 job PATH；exact Rust 1.98.1 pin/verify/bundle tooling 已 software verified，但两台 host 尚未执行。迁移 rust-format 前必须通过合法 out-of-band host-admin 通道对称完成 pin + parity re-probe。
3. **shared physical failure domain**：`vbmf-ci-01` 与 `vbmf-ci-02` 同 devbox 物理故障域，只提供维护冗余/并发/parity，不等价于物理 HA。
4. **BMD deployment divergence**：`/opt/vbmf-dev/repo` 当前 detached at `7cc33dd…` 且有未提交 ops 改动，明显落后 live main；当前无 VBMF container/service 运行。不得把该旧 deployment 的 hardware evidence 继承给当前 main。
5. **BMD device occupancy**：DeckLink driver/devnodes/plugins 当前可见，但有历史手工 `gst-launch ... decklinkvideosink device-number=2` 进程持续占用设备；任何后续 hardware acceptance 前必须先归属确认/受控释放，禁止抢占。
6. **open PR #31 / historical branch**：仍 open / unmerged，不是 canonical development Authority；未来吸收前必须 main-relative reconciliation。
7. **historical local edit provenance**：旧 STATE 记录的另一 checkout 未提交 `DEPLOYMENT_AND_DEV_RUNTIME.md` 修改未出现在当前新 checkout；原工作区未重新取得前不可判定其去留。

### Current blockers

- P2-B: **NONE / COMPLETE**.
- P2-C: **BLOCKED at host execution boundary**。两台 general runner 的 system-level Rust pin 是硬前置；当前 Development VM / `_shared` 没有可用的 devbox host-admin 地址/凭据，且 `vbmf-ci` service account 的 `sudo -n` 已真实 fail-closed。必须取得既有/授权的 out-of-band host-admin 通道并对称执行 §15.9；禁止改成 job-local rolling `stable`，也禁止给 CI runner 放开通用 sudo/root。

## 9. Verification Debt

- 24h stability：需要在 RCA / fix 真正关闭后重新跑完整 24h rung；旧 FAIL 不得被短跑或诊断 run 覆盖。
- branch rename hygiene：**P2-B CLOSED**；workflow/runbook 操作性路径已统一 `main`，required context 名称未变。
- P2-C 前：两台 general runner system-level Rust 1.98.1 pin + parity re-probe 仍未执行；当前 tooling 已就绪，host-admin execution deferred/blocking。
- Development agent：Pi/Claude Code 项目级 auth + real model smoke 已 PASS；Claude project trust PASS；后续长写任务优先用 `_shared` runner + tmux，不再登记 agent smoke debt。
- BMD：当前 main 的 Runtime/hardware verification **deferred / not required for P2-B/P2-C CI-only changes**；BMD deployment 需未来按 exact commit 重建后才能产生新 evidence。
- PR #31：若未来吸收 standalone product baseline，必须先做 main-relative scope audit/reconciliation，不恢复 feature 分支 Authority。

## 10. Cold-Start Handoff

新 Chat / Work 必须按以下最小链恢复，不重新扫描整个仓库：

1. 读取 Project Instructions；
2. 确认仓库 `pwl1987/VBMF`、默认分支和 canonical branch 都是 `main`；
3. 读取 live `main` HEAD；
4. 读取本 `.project/STATE.md`；
5. 找到“包含当前 STATE 版本的 commit”，比较 live HEAD 是否有更新；若有，只 reconcile STATE 之后的新 commits；
6. 读取 **Current Task = P2-C** 对应 Authority：`docs/architecture/CI_RUNNER_STRATEGY.md` §15.2 / §15.5 / §15.6 / §15.7 / §15.8 / §15.9；
7. 核对 P2-C tooling commits `5a9ae16…` / `cf1e07d…`、maintenance failure evidence `34921727757` 与最新 `ci-infra-probe`；
8. system-level Rust pin 未满足时，先取得授权 host-admin 通道并按 §15.9 对称 provision + parity re-probe；满足后才允许迁 `rust-format` 的 `runs-on`；
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
- **Current Task = P2-C rust-format single-job grayscale**；
- **Next Task = P2-D rust-clippy self-hosted grayscale**；
- Runtime / Web 新业务功能当前不应推进；
- 24h RSS stability 仍是明确 verification debt，不能宣称 stability verified。
