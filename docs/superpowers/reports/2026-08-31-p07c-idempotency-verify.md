# Verify Report — p07c-idempotency（Phase 0.7C-4: Idempotency Foundation）

- 日期：2026-08-31 · 验证人：开发 AI（ZCode）· 模式：full workflow
- 分支：`comet/p07c-idempotency`（base `7de969b`）· 提交：`116dd87`（实现）+ `3b36818`（盒上迭代收敛）
- 结论：**PASS**（0 CRIT / 0 IMP / 2 NOTE）
- 证据纪律：每项引用 commit / 测试名 / 命令 / 结果 / 工件（0.7A 虚报教训的永久规则）

## 1. 范围对表（终审执行令逐项）

执行令：`CommandId → Canonical Command Fingerprint → Atomic Claim → Execute once → Persist Outcome → Duplicate → Replay / Conflict`；重点=冻结"什么叫同一个命令"+ 并发裁决规则，**不是**实现幂等存储。

| 执行令项 | 落点（commit 116dd87 + 3b36818） | 证据 |
|---|---|---|
| CommandId | 0.7C-3 已有 `CommandId(pub Uuid)`（Hash）作表键 | `idempotency.rs` records: `HashMap<CommandId, Record>` |
| Canonical Fingerprint | `fingerprint()` 纯函数 = kind 判别式 + `CommandTarget` canonical serde JSON | 测试 `idem_rt_01_fingerprint_semantics`（盒上 PASS） |
| Atomic Claim | 单临界区 check-and-insert + 锁外执行 + `catch_unwind` 终态兜底 | 测试 `idem_rt_01_concurrent_duplicate_single_execution` |
| Execute once | claimant 独占执行 `command::dispatch`（0.7C-3 薄映射零改动） | 同上 + `idem_rt_01_execute_once_and_replay` |
| Persist Outcome | `RecordState::Completed(CommandOutcome)` 落表 + `notify_all` | 同上（replay 读终态） |
| Duplicate → Replay | `Replayed(原 outcome)` 逐字节相等；Failed 同样 replay | `idem_rt_01_execute_once_and_replay` + `idem_rt_01_stop_replay_not_reexecute` |
| 同 id 异 payload → Conflict | `Conflict{expected,actual}` 零执行零改写 | `idem_rt_01_payload_conflict` |

## 2. D9-A~E 逐项锁死核验（终审 §11 防假关闭）

- **D9-A command identity**：参与=kind+target；**不参与=command_id/issued_at_ms/requested_by**（design.md §1 显式冻结理由：issued_at_ms 重试会变 / requested_by 审计标签 / command_id 是查表键本身）。证据：`idem_rt_01_fingerprint_semantics` 断言元数据变化指纹不变、换 command_id 指纹不变、payload 变必不等。
- **D9-B payload conflict**：`StartSession(id=X,intent=A)` 后 `StartSession(id=X,intent=B)` → `Conflict`（绝不 replay 绝不执行）；原记录保留，此后同 A 重复仍 `Replayed`；再发 B 仍 `Conflict`。证据：`idem_rt_01_payload_conflict` 全断言。
- **D9-C atomic claim**：**初版实现把 check 与 insert 分在两个临界区 = check-then-act**，被 D9-E 击穿测试当场抓住（盒上 mock 组 `Executed=2, 期望 1`，TEST_MOCK_EXIT=101，工件 `~/p07_results.txt` 第二轮）；修复为单临界区（3b36818）后并发恰一次。Rejected 不写表不占 id（`idem_rt_01_validate_rejected_does_not_claim`）。
- **D9-D result replay**：outcome 逐字节重放（`assert_eq!` 级）；ghost stop 首次 Failed、重发 Replayed(Failed 同值)；stop Executed 后重发 Replayed(Executed) **非**对 Released 再 stop 的 Failed（重复≠重新执行）。证据：两项测试。
- **D9-E concurrent duplicate**：8 线程 barrier 同 envelope → Executed×1 + Replayed×7 + outcome 全等 + 会话数 1 + 无 Conflict/Rejected 混入。

## 3. 三层证据

### Unit/Simulation（盒上，~/p07_results.txt 第五轮 + ~/p07_run_console.log）
- 命令：`bash ~/p07_verify.sh`（cd ~/media-agent-build, DECKLINK_SDK_INCLUDE/LIBCLANG_PATH 已 export）
- 结果：`FMT_APPLY_EXIT=0 / FMT_CHECK_EXIT=0 / TEST_DEF=0 / TEST_SIM=0 / TEST_MOCK=0 / TEST_BMD=0 / CLIPPY_{DEF,MOCK,GSONLY,BMD}_EXIT=0 / BUILD_{GSONLY,BMD,HWTEST}_EXIT=0 / PROOF_EXIT=0`
- 测试计数：**138 / 138 / 178 / 138**（default/simulation/mock/bmd,gstreamer；mock 组 +8 = 0.7C-3 基线 170 → 178，`test result: ok. 178 passed; 0 failed`）
- 新增 8 测试名：`idem_rt_01_{fingerprint_semantics, vocabulary_snapshot, non_executability_surface, validate_rejected_does_not_claim, execute_once_and_replay, stop_replay_not_reexecute, payload_conflict, concurrent_duplicate_single_execution}`

### Hardware（真机 lytv@10.30.15.10，bmd,gstreamer 构建）
- 命令：`VBMF_SESSION_LIFECYCLE=1 MEDIA_AGENT_DEVICE_BINDING=/home/lytv/loopback-manifest-v2.json timeout 240 ./target/debug/media-agent`
- 结果：**GATE_EXIT=0**（工件 `~/p07_gate_hw.log`）：
  - `IDEMPOTENCY-RT-01 step=start verdict=executed status=Executed sessions=1`
  - `step=duplicate verdict=replayed status=Executed sessions=1 outcome_equal=true`
  - `step=conflict verdict=conflict sessions=1`（零执行）
  - `step=observe running=true`（10s）→ `step=stop/release verdict=executed`
  - 回归：`COMMAND-CONTRACT-RT-01 start/stop/release status=Executed + observe running=true`；`SESSION-RT-01/RESOURCE-RT-01 ALL PASS`；RUNTIME-STATE-RT-01 两点输出

### CI
- PR required checks 以 GitHub 实跑为准（见 §6；盒上绿 ≠ CI 绿纪律）。

## 4. 红线核验

- **不可执行性延续**：`idem_rt_01_non_executability_surface` — 公开面 allowlist `[fingerprint, dispatch]` 恒等；fingerprint 串禁 vendor/执行词（gst/device_number/backend/handle/ffmpeg/alsa/kafka/nats/decklinkvideosrc）；`pipeline` 按 0.7C-3 精化口径不在禁列（canonical 冻结键名）。
- **Query/Command 分离延续**：idempotency.rs 零 `runtime_query` 引用；allowlist 无 get_/list_。
- **无万能 Executor**：幂等层是 `command::dispatch` 外包装；command.rs 0.7C-3 冻结面**零改动**（diff 仅新增 idempotency.rs + main.rs gate 段）。
- **Error Model 未被吞并**：`CommandStatus` 四态零改动（`command_rt_01_vocabulary_snapshot` 回归 PASS）；`IdempotentDispatch` 四出口词表快照锁定（`idem_rt_01_vocabulary_snapshot`）；InProgress/AlreadyApplied/Retryable 未引入（0.7C-5 范围）。

## 5. 文档对账

- PHASE_IMPLEMENTATION_MAP：0.7C-4 行 ✅ COMPLETE（tag `phase-0.7C4-idempotency`）；0.7C 行下一项 = **Error Model → Event Projection → External API**。
- PHASE_0_7A_POST_MERGE_DEBT：D9 ✅ CLOSED @ p07c-idempotency，按 D9-A~E 逐项引用测试证据；D14/D15 保持 OPEN（本 change 不触碰）；D6 演进项不变。

## 6. 迭代披露（三轮，全部如实）

1. R1：main.rs gate 段 match 臂 `CommandOutcome` vs `Option` 混型（E0308）→ `Some(o)` 修复。
2. R2：clippy 1.98 `doc_overindented_list_items`（模块头续行 11 空格 → 3 空格）+ 测试残留未用变量。
3. **R3（真 bug）**：claim 两段式临界区 = check-then-act 竞态，D9-E 击穿测试抓到并发双执行（Executed=2）→ 单临界区修复。此为终审 §9 反模式的实例印证。
4. R4：真机 conflict 探针撞值（sink.kind 改 rtmp 与真机 intent 恒等）→ 改 `intent.version` 探针。

## 分级

- **CRIT：0** · **IMP：0**
- NOTE 1：幂等记录表无容量上界/驱逐策略（gate 量级无现实风险；驱逐会使 replay 退化成重执行，故不做随意驱逐——上界策略留 External API 阶段，design.md §7 已显式声明，不新开债务编号）。
- NOTE 2：`IdempotentDispatch::Executed` 的语义是"本请求是 claimant 且已执行一次"，其内 outcome.status 可为 Failed——两平面命名在 0.7C-5 Error Model 设计时需在文档中再次显式区分（预期内，非缺陷）。
