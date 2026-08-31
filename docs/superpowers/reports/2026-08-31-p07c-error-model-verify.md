# Verify Report — p07c-error-model（Phase 0.7C-5: Error Model Foundation）

- 日期：2026-08-31 · 验证人：开发 AI（ZCode）· 模式：full workflow
- 分支：`comet/p07c-error-model`（base `317d99d`）· 提交：`0f58d11`（实现）+ `cc62edb`（盒上迭代收敛）
- 结论：**PASS**（0 CRIT / 0 IMP / 2 NOTE）
- 证据纪律：每项引用 commit / 测试名 / 命令 / 结果 / 工件

## 1. 范围对表（终审第一道红线 + 执行范围）

| 项 | 落点（commit 0f58d11 + cc62edb） | 证据 |
|---|---|---|
| **三平面分离红线**：CommandStatus ≠ IdempotentDispatch ≠ ErrorClassification | ErrorClassification 独立 enum；零字段单元变体（serde 断言纯标签，不嵌套其他平面）；另两平面词表快照零改动回归 | `err_model_rt_01_three_plane_separation`（盒上 PASS） |
| 封闭词表五项 | Rejected/Conflict/RetryableFailure/PermanentFailure/Unknown；不纳入 InProgress/AlreadyApplied/Duplicate（design.md §2 显式理由） | `err_model_rt_01_vocabulary_snapshot` |
| classify_session_error 封闭映射 | 九臂→五类，**match 无通配臂**（SessionError 新增变体编译失败强制评审） | `err_model_rt_01_classify_matrix_closed_mapping`（10 case） |
| 分类在错误边界产生 | dispatch 三臂 `Err(e)` 处分类（e 仍为类型态）；detail 字符串零事后解析 | `err_model_rt_01_outcome_invariant` |
| outcome 分类不变量 | Failed⇒Some(非 Rejected/Conflict)；Rejected⇒Some(Rejected)；Executed⇒None | 同上 |
| panic 兜底对齐 | idempotency.rs claimant catch_unwind → `Unknown`（不臆造） | 源码 `idempotency.rs`（构造点唯一） |
| Replay 携带归因 | Replayed 重放 outcome 含 classification——同一命令重放同一归因 | `err_model_rt_01_outcome_invariant` replay 段 + `idem_rt_01_execute_once_and_replay` 逐字节回归 |
| D9 措辞收紧（Gate §11） | 债表改为 "D9-A~E **Foundation**: CLOSED（进程内）/ External API·持久化·跨重启语义 **deferred** to External API stage" | PHASE_0_7A_POST_MERGE_DEBT.md D9 行 |

## 2. 三层证据

### Unit/Simulation（盒上，~/p07_results.txt 第二轮 + ~/p07_run_console.log）
- 命令：`bash ~/p07_verify.sh`（cd ~/media-agent-build）
- 结果：`FMT_APPLY_EXIT=0 / FMT_CHECK_EXIT=0 / TEST_DEF=0 / TEST_SIM=0 / TEST_MOCK=0 / TEST_BMD=0 / CLIPPY_{DEF,MOCK,GSONLY,BMD}_EXIT=0 / BUILD_{GSONLY,BMD,HWTEST}_EXIT=0 / PROOF_EXIT=0`
- 测试计数：**138 / 138 / 182 / 138**（mock 组 178→182，+4：`err_model_rt_01_{vocabulary_snapshot, classify_matrix_closed_mapping, outcome_invariant, three_plane_separation}`；另有 `dispatch_failure_classification` 场景并入 outcome_invariant 的 dispatch 路径断言——见 NOTE 2）

### Hardware（真机 lytv@10.30.15.10，bmd,gstreamer 构建）
- 命令：`VBMF_SESSION_LIFECYCLE=1 MEDIA_AGENT_DEVICE_BINDING=/home/lytv/loopback-manifest-v2.json timeout 240 ./target/debug/media-agent`
- 结果：**GATE_EXIT=0**（工件 `~/p07_gate_hw.log`）：
  - 成功链路分类全 None（不变量真机侧）：`start/duplicate/stop/release classification=None`
  - **`ERROR-MODEL-RT-01 step=ghost-stop status=Failed classification=Some(PermanentFailure) detail=Some("unknown session session-a513…")`**——UnknownSession 臂真机实证
  - 回归：IDEMPOTENCY-RT-01（executed/replayed outcome_equal/conflict）+ `SESSION-RT-01/RESOURCE-RT-01 ALL PASS`

### CI
- PR required checks 以 GitHub 实跑为准（§6）。

## 3. 红线核验

- **禁万能 CommandResult**：三 enum 独立；classification 是 `Option<ErrorClassification>` 嵌入字段（非合并进 status 词表）；ErrorClassification 零字段单元变体（序列化纯字符串——测试断言 `v.is_string()`）。
- **0.7C-3/0.7C-4 冻结语义零触碰**：CommandStatus 四态与 IdempotentDispatch 四出口词表快照回归 PASS；validate 纯函数、dispatch 薄映射、原子 claim、fingerprint 语义全部零改动（回归测试全绿佐证）。
- **SessionError 类型零改动**：分类是纯函数投影（`git diff` 仅新增 error_model.rs + command.rs outcome 字段/Err 分支 + idempotency.rs 兜底/构造 + main.rs gate 段）。
- **接线纪律**：分类非孤立函数库——消费点=CommandOutcome.classification（External API 直接可用）+ main.rs gate 真机输出。
- **Query/Command 分离延续**：error_model.rs 零 runtime_query 引用；allowlist `[classify_session_error]` 无 get_/list_/execute 动词。

## 4. 决策披露（design.md §3 D-1 对勘）

- **选定**：outcome 嵌入 `classification` 字段（错误边界处产生）。
- **否决**：①事后从 detail 字符串恢复分类（脆弱、违背"文档语义>实现行为"）；②只做纯函数库不接线（接线纪律=未实现）；③IdempotentDispatch 增分类出口（改 0.7C-4 词表平面）。
- 选 A 触碰 command.rs **类型定义**（加字段）而非**冻结语义**（四态封闭/零执行字段/不可执行性）——serde JSON 演进经本 change 架构评审，两平面快照测试同步更新。

## 5. 迭代披露（两轮，全部如实）

1. R1：测试夹具 `ResourceState::Released` 变体不存在（实际 Available/Reserved/Allocated/Releasing/Faulted）→ 改 Reserved→Faulted（同为状态机非法迁移语义）；TEST_MOCK_EXIT=101 + CLIPPY_MOCK 同源失败。
2. R2：全绿。

## 6. 分级

- **CRIT：0** · **IMP：0**
- NOTE 1：`PreflightFailed` 整体判 `RetryableFailure` 为保守粒度——其中 BackendCapability Unsupported 子情形本质 Permanent；细化需拆解 PreflightReport 分级判定，属演进项（design.md §4 注释已登记），本 change 不做 report 深解析。
- NOTE 2：tasks.md 原列 6 项测试实际落盘 5 个 `err_model_rt_01_*`（`dispatch_failure_classification` 的断言全部并入 `outcome_invariant` 的 dispatch 路径——Rejected/ghost/成功链/replay 四场景同函数覆盖）；无虚报（本报告如实计数 182）。
