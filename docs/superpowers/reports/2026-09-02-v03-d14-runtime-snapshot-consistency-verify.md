# Verify 报告 — V0.3-1 D14 Runtime Snapshot Consistency

- **Change**: `v03-d14-runtime-snapshot-consistency`（full workflow, skip_specs:true）
- **分支**: `comet/v03-d14-runtime-snapshot-consistency`（base `4d13265` = master）
- **代码提交**: `f52d882`（单 commit, 5 文件, +285/−26; 回滚 = revert 单提交）
- **日期**: 2026-09-02
- **Design Doc**: `docs/superpowers/specs/2026-09-01-v03-d14-runtime-snapshot-consistency-design.md`
- **执行计划**: `docs/superpowers/plans/2026-09-02-v03-d14-runtime-snapshot-consistency.md`

## 0. 结论

**D14 CLOSED。** 一致性类 = **swept, non-transactional, start-ordered**。观察信封
`SnapshotObservation{revision, lineage, observed_at_ms}` 落地，`CanonicalRuntimeState`/`ApiQuerySnapshot`
additive 两字段（旧 6 键 JSON → 0/nil 双向非破坏），`assemble()` 纯化（隐式墙钟删除），
`SessionManager` 唯一序列 owner。三层测试全 PASS，盒上矩阵零回退，硬件门禁 D14 断言真机 PASS。
**零行为变更**（除 wire 面两新键外, `/health` 五字段/生命周期/事件平面逐项未触碰——review gate 独立复核确认）。

## 1. 四栏纪律表

| # | 任务 | Contract | Implementation | Verification | Gate |
|---|------|----------|----------------|--------------|------|
| 1.1 | SnapshotObservation 结构体 | proposal §What-1 / design D5 | 已 @ f52d882 | 五套 feature 矩阵编译 exit 0 | 无 |
| 1.2 | CanonicalRuntimeState additive 2 字段 `#[serde(default)]` | EXTERNAL_API_CONTRACT §2 #126 | 已 @ f52d882 | serde 往返 + **真实旧 6 键 JSON blob**→0/nil 双测试 PASS | 无 |
| 1.3 | assemble 6 参纯化（删 now_ms） | design D5 / proposal §Why-3 | 已 @ f52d882 | 纯度测试（同 obs 两次逐字段相等）PASS; grep gate 0 命中 | 无 |
| 2.1 | SessionManager owner（AtomicU64 + lineage） | design D1/D3 | 已 @ f52d882（**Relaxed 终裁**, 见 §3 裁定注） | 起点 1 / 严格 +1 / lineage 恒同非 nil PASS | 无 |
| 2.2 | runtime_state() fetch_add 先于采集 | proposal §What-2 | 已 @ f52d882 | 矩阵编译全过 + 既有测试零回退（215→221 恰 +6） | 无 |
| 3.1 | ApiQuerySnapshot additive 投影 | EXTERNAL_API_CONTRACT §2 #126 | 已 @ f52d882 | 投影透传测试 PASS + 真机响应实查（D14 断言块） | 无 |
| 3.2 | runtime_query.rs 镜像注释关闭 | 既有注释 | 已 @ f52d882 | 两处注释 grep + 人工复核一致 | 无 |
| 4.1 | 契约注释 CLOSED 声明 | design D7/D3 | 已 @ f52d882 | 注释与 design 逐句一致（review gate 复核） | 无 |
| 4.2 | 债务账本 D14 行 CLOSED | 债务清单纪律 L6 | 已 @ f52d882 | 行含 change 名/日期/证据锚（→本报告） | 无 |
| 4.3 | 键集合断言 6→8 | design D8 | 已 @ f52d882 | 该测试于 mock 221 内 PASS | 无 |
| 5.1 | Unit 层三测试 | D14 关闭条件 | 已 @ f52d882 | mock 全绿 221 | 无 |
| 5.2 | Simulation 层（8×1000 击穿） | design D1/D6 / D9-C 先例 | 已 @ f52d882 | release 真跑 PASS（§4） | 无 |
| 5.3 | Hardware 层（D14 断言块） | D14 关闭条件 | 已（盒上脚本, 不入库） | 真机实跑 PASS（§5） | 盒上 matrix 含此项 |
| 6.1 | 盒上全矩阵零回退 | 验收口径 BOX | 已（2026-09-02 实跑） | 14 步全 exit 0（§6） | BOX |
| 6.2 | CI 七 required checks | 验收口径 CI | PR 后 gh 实查 | 见 §8 | CI |
| 6.3 | verify→archive→PR→merge→删分支 | 交付纪律 | 本报告 + 后续流程 | PR merged | RELEASE |

## 2. TDD 证据（RED → GREEN）

- **RED**（实现前, 盒上 cargo 实跑）: `error[E0422] cannot find struct SnapshotObservation` +
  `error[E0061] this function takes 5 arguments but 6 were supplied` +
  `error[E0609] no field observation_revision/observation_lineage`——失败签名全部指向"特性缺失", 非拼写错误。
- **GREEN**（Task 4 调用点同步后全 crate 编译）: `test result: ok. 218 passed; 0 failed`（215 基线 + 3 新）。
- **任务 5/6 深化测试**: `221 passed; 0 failed`（+3: assemble 纯度 / 重启语义 / 8×1000 并发）。
- **零回退账目**: 215 → 221, 恰 **+6 新测试函数**（envelope 往返 / 旧 6 键 default / 纯度 / 起点 1 / 重启 / 并发）; 既有 215 全 PASS, default/simulation/bmd 三面计数不变（155/155/155）。

## 3. 两处 design→tasks 裁定注（doc 一致性）

1. **ordering**: tasks.md 原文 `fetch_add(SeqCst)`; 实现为 **Relaxed**——plan §0.3 终裁（单原子 RMW
   任何 ordering 下不重号不空洞; 无数据经计数器发布）。review gate 独立复核认定该论证成立。
2. **重启 revision**: tasks/注释原稿"归零"; 实现为**归 1**——design 终裁"计数起点 1, 0 永久保留 = absent
   （serde default 哨兵）"。

## 4. Simulation 层 — 8×1000 并发击穿（release 真跑）

命令: `cargo test --features mock --release session_rt_01_observation_revision_8x1000_concurrency_pierce -- --nocapture`（盒上, 2026-09-02）

```
test session::tests::session_rt_01_observation_revision_8x1000_concurrency_pierce ... ok
test result: ok. 1 passed; 0 failed
```

断言内容（非计时、确定性）: Barrier 同步 8 线程 × 1000 次 `runtime_state()`; 8000 份 revision
sort+dedup 后与 `{1..=8000}` 集合相等（鸽笼原理: 任何重号必挤出一个值 → 集合不等; 任何空洞同理）;
8000 份 lineage 恒同。**该测试在 `feature = "mock"` 下编译运行（与全部 session 测试同载体, CI
`session-lifecycle` job 为其承载者）。**

## 5. Hardware 层 — 真机 D14 断言（盒上 2026-09-02）

脚本: 盒上 `~/transport_hw_gate.sh`（**按裁定不入库**——D14 加法断言块, 既有 16 探针零改动;
本报告为其唯一持久证据载体）。环境: `lytv@10.30.15.10`, `--features bmd,gstreamer` 真机
loopback（manifest `~/loopback-manifest-v2.json`）, diagnostic 模式真实 SessionManager。

```
LISTENER_UP=1
PASS: health_status_200 (200)                     PASS: health_field_state … (五字段)
PASS: boundary_200 / process_local / durable_log_deferred / restart_breaks_replay
PASS: runtime_200 (mgr active)
PASS: projection_200 / projection_snapshot_kind
PASS: commands_200
PASS: notfound_404 / method_405
D14 rev1=2 rev2=3 lin1=bfeef5e1-9df9-4020-84da-18adfe227775 lin2=bfeef5e1-…775
PASS: D14-1 revision>=1
PASS: D14-2 lineage-uuid-format
PASS: D14-3 same-lineage-strict-increment
=== TRANSPORT_HW_GATE_DONE ===
```

读数说明: rev1=2 而非 1, 因既有 [C]-3 `runtime_200` 探针先消费了 revision 1——两次相邻读取
2→3 严格 +1 且 lineage 双查一致, 语义自洽。**既有 16 探针全 PASS = TRANSPORT-RT-01 零回归**
（纪律 #9: 硬件门禁只增 D14 验收, 不改采集行为; transport.rs/main.rs 零 diff）。

## 6. BOX 层 — 14 步全矩阵（p07_verify.sh, 2026-09-02）

```
FMT_APPLY_EXIT=0  FMT_CHECK_EXIT=0
TEST_DEF_EXIT=0 (155)  TEST_SIM_EXIT=0 (155)  TEST_MOCK_EXIT=0 (221)  TEST_BMD_EXIT=0 (155)
CLIPPY_DEF/MOCK/GSONLY/BMD_EXIT=0 (×4)
BUILD_GSONLY/BMD/HWTEST_EXIT=0 (×3)
PROOF_EXIT=0 (remove-adapters: Domain/Contracts/Runtime 无具体适配器可编译)
```

fmt apply 后 `--check` 复跑仍 clean → 本地树与盒上格式零漂移; 提交前 CRLF 自检 5 文件全 LF。

## 7. Review gate（standard 模式, requesting-code-review 全 change 一次）

裁决: **Ready to merge — With fixes**; **0 Critical / 2 Important / 4 Minor**。

- **Important #1（账本证据锚空指针）**: D14 CLOSED 行引用的 design doc（未跟踪）与 verify 报告
  （未落档）必须随 change 提交, 否则账本（SoT）指向死路径。→ **已处置**: 本报告 + design doc +
  plan + change 产物随本 change 提交（合并前在库）。
- **Important #2（repo 根残留旧版 gate 脚本）**: pre-D14 版 `transport_hw_gate.sh` 在仓库根,
  有误提交风险。→ **已处置**: 已删除（盒上 `~/transport_hw_gate.sh` 为唯一权威版本, 含 D14 块）。
- **Minor #1（observed_at_ms 时刻语义）**: → **已折叠进 f52d882**: SnapshotObservation doc 注明
  "装配点读取, 与 revision 锚定时刻非同一时刻"。
- **Minor #2（ApiQuerySnapshot 无 serde default 的不对称）**: → **已折叠进 f52d882**: 代码注释
  记录有意决定（响应模型, 无旧 JSON 反序列化消费方）; 接受理由如注释。
- **Minor #3（design §6.1 façade 行并入直接测试）**: **接受不补**——`get_runtime_state()` 为纯
  委托（runtime_query.rs:41-43）, façade 测试近似同义反复; 计划阶段已并入 `mgr.runtime_state()`
  直接测试, 语义覆盖等价。
- **Minor #4（u64 溢出回绕）**: **接受不处理**——~1000 快照/秒 ≈ 5.85 亿年; 无现实可达性。

review 同时独立复核确认: crate 内 `CanonicalRuntimeState::assemble` 生产调用恰 1 处
（session.rs）、`ApiQuerySnapshot {` 字面量恰 1 处（投影函数）、无遗漏的第二生产者; Relaxed +
fetch_add-先行的 soundness; 8×1000 测试确定性（无计时/睡眠依赖）。

## 8. CI 层（PR 后 gh 实查——6.2 完成时回填）

七 required context（rust-format / rust-test-matrix / rust-clippy / session-lifecycle /
hardware-test-compile / architecture-portability / gstreamer-build）: **见 PR 检查记录（合并前全 green 为 Merge Gate 前置）。**

## 9. 三 "revision" 消歧（防混淆锚, 与代码注释 §4.3 同源）

| 名称 | 域 | 语义 | 互替性 |
|------|----|------|--------|
| `observation_revision`（本 change） | 观察域 | 进程内快照观察序号, 起点 1, 单调唯一, 重启归 1 + 换 lineage | 不替代下两者 |
| V0.2 §1.21 "Runtime Revision N+1" | config-apply 域 | 配置应用代次 | 不替代 |
| API §4 `If-Match: "revision-N"` | 乐观并发域 | 写冲突检测 | 不替代 |

## 10. 不变量与红线复核

- **assemble 纯度**: runtime_state.rs 无 `now_ms`/`SystemTime`（grep exit 1）; 墙钟唯一读点 =
  SessionManager::now_ms（既有私有, 不新增公共面）。
- **additive 非破坏**: 真实旧 6 键 JSON 反序列化测试 PASS; 顶层键集 6→8 由测试锁定
  （`runtime_state_rt_01_composition_descriptor_not_flattened`）。
- **`new()` 10 参签名零改动**: 10 调用点（生产 main.rs ×2 + 测试 ×8）未触碰。
- **范围纪律（用户 10 条硬纪律 #10）**: 未触碰 DeviceId / RuntimeEvent / Control Plane /
  Federation / V0.2 Runtime Semantics; diff = 5 文件, 全部在 D14 范围内。
