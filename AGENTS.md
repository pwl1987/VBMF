<!-- CODEGRAPH_START -->
## CodeGraph

In repositories indexed by CodeGraph (a `.codegraph/` directory exists at the repo root), reach for it BEFORE grep/find or reading files when you need to understand or locate code:

- **MCP tool** (when available): `codegraph_explore` answers most code questions in one call — the relevant symbols' verbatim source plus the call paths between them, including dynamic-dispatch hops grep can't follow. Name a file or symbol in the query to read its current line-numbered source. If it's listed but deferred, load it by name via tool search.
- **Shell** (always works): `codegraph explore "<symbol names or question>"` prints the same output.

If there is no `.codegraph/` directory, skip CodeGraph entirely — indexing is the user's decision.
<!-- CODEGRAPH_END -->

<comet-ambient-resume>
<!-- Managed by Comet. Edits inside this block may be replaced by comet init/update. -->
<!-- Contract: comet.resume_probe.v2 -->

## Comet Ambient Resume

在这个仓库中，开始处理需要改动或调查的任务前，如果可能存在活跃 Comet workflow，把当前用户请求传入只读探针：`comet resume-probe . --stdin --json`。

- 如果用户通过宿主明确调用任意 Comet Skill（例如 `@comet`、`/comet`、`@comet-native` 或 `/comet-hotfix`），显式调用优先于本恢复协议；不要运行 resume probe，直接进入被调用的 Skill。
- 如果用户通过宿主明确调用的是非 Comet 的 Skill 或斜杠命令，任务意图已由该调用明确：不要运行 resume probe，直接执行该 Skill。
- 如果你正在 Comet 流程内（包括正在等待用户回复你在流程中提出的问题），不要运行 resume probe；把这类回复（例如方案/选项选择）当作当前 change 的继续，直接按用户的选择推进。
- 只信任返回的 `workflow`、`skill` 和 `entrySource`；它们只由项目配置或无配置兼容回退决定。不得扫描或切换另一套 workflow。
- 如果 probe 返回 `auto_resume`，简短说明选中的 active change，并进入 `nextCommand` 指向的永久入口。不要把状态命令当作恢复入口直接推进。
- 如果 probe 返回 `ask_user`，只问一个简短问题并等待用户回复。
- 如果当前请求未明确调用 Comet Skill，且 probe 返回 `out_of_scope` 或 `none`，不要进入 Comet workflow。
- `out_of_scope` 或 `none` 只表示不要因为这个新请求进入 Comet workflow；它绝不表示要暂停或退出一个已在进行的 Comet 流程。
- 如果配置或状态无效且没有 `nextCommand`，停止并报告原因；不要猜测另一个 workflow。
- 不能只因为存在 active change 就把无关任务挂到该 change。Native 的未提交改动由 Native 入口检查，不由探针自动归因。
</comet-ambient-resume>


# VBMF Project Execution Contract

## Authority and state
- VBMF is a professional Broadcast Runtime / Fabric. Keep it standalone-first and compatible with `media-digital-*`, never dependent on them.
- Authority order: latest explicit user instruction → frozen Architecture/Contract/ADR → `.project/STATE.md` → GitHub live `main` → real code/tests/CI/runtime/hardware evidence → Roadmap/README/history.
- `.project/STATE.md` is the **only dynamic project-state and task-queue authority**. Do not create or maintain parallel STATUS/TODO/PROGRESS/HANDOFF/RISKS/DECISIONS files.
- Never infer completion from README, Roadmap, old chats, branch names, agent memory, or the mere existence of tests. Reconcile conflicts against live evidence first.

## Cold start / task selection
1. Use the canonical checkout `/home/ubuntu/dev/VBMF` on the Development VM when available; read `/home/ubuntu/dev/_shared/README.md` first.
2. Verify local HEAD, `origin/main`, GitHub live `main`, working tree, recent commits, and unpushed commits.
3. Read `.project/STATE.md`, then select only the **first `READY` Work Packet in its Task Queue for the Current Phase**. Do not skip to `BLOCKED` or `BACKLOG` items unless the user explicitly changes priority.
4. Read only the relevant frozen Authority, code, tests, CI/runtime evidence for that Work Packet.
5. If Git is newer than STATE, reconcile before implementation. Already COMPLETE work must not be repeated.

## Git discipline
- `main` is the sole development branch and development Authority. Do not create feature/fix/temp/repair/experiment branches.
- No force push, history rewrite, destructive reset, or silent overwrite/discard of existing work.
- Verified changes go directly to `main`; finish with local/origin/GitHub live HEAD and working-tree reconciliation.


## Runtime / environment boundary
- Runtime owns truth. Web/agent/watchdog/services observe, command, and reconcile; they must not create a second Runtime truth.
- Development VM = code, lint/static analysis, unit/focused/integration tests, build, CI/self-hosted runner, non-hardware software verification.
- BMD server = actual VBMF deployment plus DeckLink/GStreamer/FFmpeg, signal/timing/switch/transport/recovery/hardware/stability acceptance. VM software PASS never substitutes for BMD hardware PASS.
- Hardware evidence is valid only for the exact commit/binary/environment/scope that produced it. If BMD is unavailable or running a different commit, mark verification deferred.

## Agent execution
- The coordinator (ChatGPT or a user-designated orchestrator) owns Authority recovery, reconciliation, Work Packet boundaries, diff review, acceptance, Git/CI/STATE transition.
- Pi is preferred for bounded/mechanical work; Claude Code is preferred for state machines, concurrency, recovery, cross-domain changes, and difficult RCA.
- On this VM use `/home/ubuntu/dev/_shared/bin/run-pi-agent.sh` or `run-claude-agent.sh`; long Claude write tasks should run inside tmux. One write-capable agent per checkout at a time.
- Every write Work Packet must state: Task ID, relevant Authority, allowed files/scope, forbidden scope, acceptance criteria, and verification environment.
- Agent exit 0 means `EXECUTOR_DONE_NEEDS_REVIEW`, not COMPLETE. Review diff/scope/tests independently before accepting.
- Agents must not autonomously edit frozen Architecture/Contract or advance to another Task Queue item. `.project/STATE.md` may be changed only as part of an explicitly assigned/reviewed state-transition task.

## Verification / completion
- Distinguish: implementation complete, software verified, runtime smoke verified, hardware verified, stability verified, verification deferred.
- Failure-first: reproduce → evidence → RCA → minimal correct fix → focused tests → regression → CI/runtime/hardware validation as required. Do not weaken tests, Health criteria, or UI truthfulness to get green.
- Commands such as start/stop/switch/route/recover/restart must distinguish requested → accepted → executing → acknowledged → actual state → succeeded/failed/timeout/reconciled. HTTP/API success alone is not Runtime success.
- Definition of Done: Implementation + Required Verification + Evidence + `.project/STATE.md` update + commit/push `main` + required GitHub CI + final HEAD/working-tree check.
