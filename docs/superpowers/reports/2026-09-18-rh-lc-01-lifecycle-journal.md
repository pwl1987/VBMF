# RH-LC-01 — LifecycleJournal / Reverse Rollback Engine 验收报告

日期：2026-09-18
Runtime implementation commit：`c72ce5dba94fe815299ddfbbe2921f8252df7e59`
Task：关闭 `PHASE_0_7A_POST_MERGE_DEBT.md` D1。

## 1. Authority / Scope

Authority：`.project/STATE.md` RH-LC-01、`RUNTIME_LIFECYCLE_SEQUENCE.md`、D1 债务账本、`session.rs` 真实生命周期实现与失败测试。

本 change 只收敛 `SessionManager::start()` 失败回滚编排；不改变 create/stop 语义，不改 ProgramExecutionRuntime ownership，不改 Supervisor/Resource/Lease policy，不改 frozen API/wire vocabulary。

冻结安全序保持：

1. 已实例化/已启动 pipeline handles 按创建逆序 `backend.stop`；
2. 全部 pipeline teardown 后才释放 holder allocations；
3. 再释放 leases；
4. 最后释放 reservations；
5. 会话落 `StartFailed` 并保留 failure-specific `SessionFailed` reason。

## 2. Implementation

`session.rs` 新增私有 `CompletedStep` journal：`LeasesHeld`、`Instantiated(handle)`、`AllocationAcquired`。

materialize / empty-plan / instantiate / partial-allocation / backend.start 五类失败路径统一进入 `rollback_start_journal()`；原先分散的手写 stop/release/phase/event 逻辑删除。

`AllocationAcquired` 只在首个 claim 成功分配后登记，保持“CompletedStep 只记录已完成副作用”的语义；holder 级 `release_allocation()` 覆盖后续成功子集。
## 3. Failure-first Coverage

新增 `session_rt_01_multi_device_start_second_failure_journal_rollback_reverse_once`：双输入首句柄 start 成功、第二句柄注入失败；断言 stop 顺序严格 `[101,100]`、每句柄恰一次、零 lease/resource orphan、失败会话不回填 pipeline/input handles。

既有 materialize failure、instantiate failure、instantiate-second failure、partial allocation failure、backend.start failure 继续通过，证明统一 journal 未削弱既有零孤儿语义。

## 4. Software Verification

Development VM `ubuntu2604`，Rust `1.98.1`：

- `cargo fmt --all -- --check` PASS；
- `cargo test --features mock -q`：lib **429/429**，integration **9/9 + 12/12** PASS；
- `cargo test -q`：default lib **246/246** PASS；
- `cargo clippy --features mock -- -D warnings` PASS；
- `cargo clippy -- -D warnings` PASS；
- `codegraph sync`：1 changed file / 161 nodes synced。

GitHub Actions run `35279581145` @ `c72ce5d…`：7 required jobs **7/7 success**，包含真实 `gstreamer-build --features bmd,gstreamer`、`hardware-test-compile`、`session-lifecycle` 与完整 `rust-test-matrix`。

## 5. BMD Verification

BMD 可访问；`/opt/vbmf-dev/repo` 仍 detached `7cc33dd…` 且不是本 change Authority。已将 exact `c72ce5d` `git archive` 复制到 BMD `/tmp/vbmf-rh-lc01-c72ce5d`，archive sha256=`78de0709ea7e757d34e1497361cc3fbab725e972441551a6c4a1feba050fdeb9`；manifest v5 MD5=`7521d17e7fd02e50eb2b0a84374a43dd`；现有 output device-number 2 未触碰。

后续远端 exact-commit build/runtime smoke 命令被当前工具安全层拦截，因此 **BMD runtime/hardware smoke = DEFERRED**。不得把 RH-BUS-01/02 历史硬件结果继承给 `c72ce5d`，也不得写成 hardware verified。

## 6. Verdict

RH-LC-01：**IMPLEMENTATION COMPLETE / SOFTWARE VERIFIED / CI VERIFIED / BMD RUNTIME SMOKE DEFERRED**。

Frozen Architecture / Contract：**无变化**。D1 可从债务账本关闭；下一 bounded packet 为 `RH-RES-01A`（D3 per-claim Reservation TTL），随后 `RH-RES-01B`（D7 backend OnceLock→direct field）。
