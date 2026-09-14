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
- Last non-state-init implementation / CI reconciliation anchor: `db54d45c76bbd05917292131c0a4a803b2907f36` (`docs(ci): P2-A execution record — dual general runners merged (#41)`).
- Dynamic state baseline: **以包含本 `.project/STATE.md` 当前版本的 live `main` commit 为准**；冷启动时用 `git log -1 -- .project/STATE.md` / GitHub file history 取得，不在文件内硬编码自引用 SHA。

### Local Working Tree

**Local Working Tree: NOT OBSERVABLE** from the current GitHub-only execution environment.

User-reported local facts at state initialization:

- local branch 已改为 `main` 并跟踪 `origin/main`；
- preserved uncommitted change: `docs/architecture/DEPLOYMENT_AND_DEV_RUNTIME.md`.

任何后续能访问本地 checkout 的窗口，在修改前必须重新执行 `git status --short --branch` / staged / unstaged / untracked 核查；不得把远端状态等同于本地 clean，也不得覆盖该用户已明确保留的未提交改动。

## 2. Current Phase

**Governance / CI Infrastructure Phase 2 Entry — P2-B gate preparation**

当前不是 Runtime / Web 新业务功能开发阶段。本轮项目状态体系初始化完成后，项目从已完成的 P2-A 进入 P2-B；旧 `ROADMAP.md` / `PHASE_IMPLEMENTATION_MAP.md` 中更早的 “NEXT” 不得重新成为当前任务。

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

## 4. Current Task

**P2-B — CI safety / scheduling boundary**

Current Task Authority:

- `docs/architecture/CI_RUNNER_STRATEGY.md` §15.2
- `docs/architecture/CI_RUNNER_STRATEGY.md` §15.5
- 相关现状 workflow: `.github/workflows/media-agent.yml`

P2-B 必须先完成裁决，再允许 P2-C 的第一个 `runs-on` 灰度。范围只有：

1. fork PR 双通道安全边界：不可信 fork 路径继续使用 GitHub-hosted；self-hosted 不获得未经裁决的不可信代码执行路径；
2. concurrency 组策略；
3. `timeout-minutes` 补齐策略；
4. 对 branch rename 后遗留的 `master` 字面量做 reconciliation，并统一到 canonical `main`；
5. focused validation / required-check context 不漂移证明；
6. 完成后同步本文件。

**本状态初始化轮没有开始 P2-B 实现。**

禁止在 P2-B 中顺手推进 Runtime、Web Console、母架构集成或其他产品功能。

## 5. Next Task

**P2-C — `rust-format` single-job self-hosted grayscale**，仅在 P2-B 通过后进入。

前置条件：

- P2-B 已冻结并验证；
- `vbmf-ci-01` 与 `vbmf-ci-02` 对称完成系统级 Rust toolchain pin；
- 不改变 7 required context 名称；
- 先迁移最便宜的 `rust-format`，全绿后才允许继续 P2-D / P2-E。

后续队列：

`P2-B → P2-C rust-format → P2-D/P2-E clippy/session/architecture/test-matrix → P2-M media migration`。

P2-M 前必须单独裁决 DeckLink SDK 注入模式；BMD 实机仍永走 hardware acceptance 人工线，不进入普通 PR CI。

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

### P2-A infrastructure

- implementation: **COMPLETE**
- runner parity / dual R Gate / dispatch evidence: **VERIFIED by registered P2-A evidence at `db54d45…`**
- current state-init changes: docs/governance only；没有重新执行 runner probe。

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
2. **P2-C toolchain prerequisite**：两台 general runner 当前需要在迁移 rust-format 前做对称系统级 Rust pin；禁止单机漂移。
3. **shared physical failure domain**：`vbmf-ci-01` 与 `vbmf-ci-02` 同 devbox 物理故障域，只提供维护冗余/并发/parity，不等价于物理 HA。
4. **branch-rename hygiene debt**：`.github/workflows/media-agent.yml` 仍有 `push.branches: [master, main]`；`CI_RUNNER_STRATEGY.md` runbook 仍有 `--ref master`。当前不构成功能阻塞，但 P2-B 必须统一到 `main`，不得长期保留双分支语义。
5. **open PR #31 / historical branch**：`feat/v03-standalone-product-baseline` 的 PR #31 仍 open / unmerged。它不是 canonical development Authority，不得直接恢复为开发入口或自动合并；其中与 Standalone First 一致的内容未来必须先与 live `main` reconciliation。Standalone First 本身来自最新 Project Instructions，不依赖 PR #31 是否合并。
6. **local pending edit**：用户报告本地 `docs/architecture/DEPLOYMENT_AND_DEV_RUNTIME.md` 有未提交改动；任何本地后续工作必须先观察并保护该改动。

### Current blockers

- Project-state initialization: **NONE** after Git Authority migration.
- P2-B: 尚未发现 Authority/Contract 硬冲突；开工前仍需增量读取相关 workflow/tests 并做 branch-rename reconciliation。

## 9. Verification Debt

- 24h stability：需要在 RCA / fix 真正关闭后重新跑完整 24h rung；旧 FAIL 不得被短跑或诊断 run 覆盖。
- 当前 state-init HEAD：没有重新跑 software CI / hardware / stability；这是 docs-only governance change，不得借历史证据冒充新验证。
- branch rename：P2-B 时清理 `master` 操作性字面量并验证 required context 名称不变。
- P2-C 前：两台 general runner system-level Rust pin + parity 复核。
- PR #31：若未来吸收 standalone product baseline，必须先做 main-relative scope audit/reconciliation，不恢复 feature 分支 Authority。

## 10. Cold-Start Handoff

新 Chat / Work 必须按以下最小链恢复，不重新扫描整个仓库：

1. 读取 Project Instructions；
2. 确认仓库 `pwl1987/VBMF`、默认分支和 canonical branch 都是 `main`；
3. 读取 live `main` HEAD；
4. 读取本 `.project/STATE.md`；
5. 找到“包含当前 STATE 版本的 commit”，比较 live HEAD 是否有更新；若有，只 reconcile STATE 之后的新 commits；
6. 读取 **Current Task = P2-B** 对应 Authority：`docs/architecture/CI_RUNNER_STRATEGY.md` §15.2 / §15.5；
7. 读取 `.github/workflows/media-agent.yml` 与 P2-B 所需的相关 CI scripts/tests；
8. 先解决已登记的 branch-rename `master` 字面量，再形成最小 P2-B 方案；
9. 不回退到已经 COMPLETE 的 0.6 / 0.7 / A2-8 / P2-A；
10. 不从历史 feature/fix/实验 branch 恢复开发；所有验证通过的改动直接推进 `main`；
11. 完成独立任务后，同一轮更新本 STATE 的 Last Completed / Current Task / Next Task / verification / risks / debt / handoff；
12. 若当前环境不能观察用户本地 checkout，明确写 `Local Working Tree: NOT OBSERVABLE`。

### Expected cold-start conclusion at this state

在 live `main` 没有比本 STATE 更新的实质提交时，正确恢复结果应是：

- Git Authority 已完成 `master → main`；
- state system 已初始化；
- P2-A 已完成；
- **Current Task = P2-B CI safety / scheduling boundary**；
- **Next Task = P2-C rust-format single-job grayscale**；
- Runtime / Web 新业务功能当前不应推进；
- 24h RSS stability 仍是明确 verification debt，不能宣称 stability verified。
