# R63-A：切换失败状态恢复（Switch Failure Recovery）——修复架构轮第一刀

- 轮次: R63-A（用户 R63 裁决：修复架构轮·先 A 后 B·两条 Change 分开）
- 基线: R62 交付 ee4e0a7（16-0A/16-0B 探针 + 16-0C 裁决）
- 性质: 代码轮（域状态机修复）——核心四中 program_execution 按二轮裁决开启，
  switch_mock/switch_graph 各 1 强制编译臂如实登记；contracts/switch·transport·
  api_boundary·bin·command·idempotency 零触碰。

## 1. 逐条确认表（用户 R63 九项指令 → 落实归属）

| # | 指令项 | 状态 | 归属 |
|---|---|---|---|
| 1 | R63-A0 先冻结恢复契约 | 本轮落实（本报告 §2·契约 commit 先于代码 commit） | 本轮 |
| 2 | R63-A1 最小恢复机制 | 本轮落实（落名以现有状态转移核定·§3） | 本轮 |
| 3 | R63-A2 异常路径测试 R1-R8 | 本轮落实（§4） | 本轮 |
| 4 | R63-B Transport 并发 | 未落实——A/B 分离纪律 + 用户令"先 A 再 B" | 下一轮（首步=inner 结构小审计） |
| 5 | R63-B1 查询被切换锁住 | 同上随 B 轮 | 下一轮 |
| 6 | A/B 不得揉一个 PR | 已遵守（本轮仅域状态机） | 结构性 |
| 7 | R64 全矩阵+真机故障恢复 | 未落实（前提 A+B 合入） | R63-B 后 |
| 8 | Step 15 长稳 30m→24h | 未落实（用户令=R64 后·场景升级已记） | 后续 |
| 9 | Step 17 Preview RC 最后 | 未落实（验收标准升级已记） | 链末 |

Phase-1 披露: codegraph CLI 与 comet 探针被 plan-mode 只读门拦截（Bash 全类
只读拦截），核验改用 git + 直读 + 两个只读 Explore 代理（涟漪面全仓清点 +
测试地貌/openspec/watchdog 锁结构），如实记录；未进入 Comet workflow。

## 2. 恢复契约（A0·冻结文本·2026-09-06 随计划批准即锁）

> **SoT：Observed 优先，Desired 由 reconciliation 推进。
> observation ≠ intent；absence ≠ false（observed=None 不猜 A/B）。**

1. **翻转未生效的失败**（adapter Err / fence 确认失败且再观测=from）：
   Desired 回 `Active(from)`，Timeline 回 `Stable{from}`（既有 `abort_transition`
   语义：epoch/世代不变、时间线零变化）。
2. **已执行但证据/落定失败**（evidence 超时、settle 矛盾、fence 超时且再观测=to）：
   命令 outcome **保持 Failed（不伪装成功）**；状态两平面 reconcile 落
   `Active(observed)` + Timeline `Stable{observed}` 且 **ProgramEpoch+1、新段以
   observed 恒等锚重开、pts_state=DiscontinuityDeclared、PTS 基线清空（不伪造
   连续性）**——切换发生了但证据不完整，如实记不连续。
3. **observed=None/组外**：Desired → `RecoveryRequired{from,to}`（终态，不猜）；
   下一次 switch = Permanent 拒收；恢复路径=会话级 teardown（自动恢复不在本轮
   范围）。Timeline 诚实停留 TransitionFailed。
4. **replay**：同 command_id 重放 ≡ 原始 outcome 逐字节（恢复只动状态平面，
   不改写已记录应答）。
5. **红线维持**：`complete_switch` 语义（observed==to 才落定）禁改；
   `force_release` 语义禁改；watchdog 不改（新变体落入 consistent=false 通配臂
   =如实上报不动作，加单测钉住）；R53 闩锁纪律不破坏——reconcile 是**显式恢复
   转移**（干净边界重开段基准），非"段内普通帧自动恢复"，违例历史入 immutable
   段史不洗。
6. **switch_epoch 语义**：epoch 在 begin 已消费，reconcile 不再变更（epoch=已
   开始切换计数）；Timeline ProgramEpoch 在落地 reconcile 时 +1。
7. **新词表（随契约冻结）**：`SwitchDesired::RecoveryRequired{from,to}`（第三
   终态）+ `SwitchError::RecoveryRequired(SwitchDesired)`（classify →
   PermanentFailure·快照测试显式更新=架构评审动作）。

## 3. A1 设计落名（实现清单·执行后回填核对）

- `ExecutionGroup::reconcile_switch(observed: Option<Uuid>)`——仅从 Switching
  起：Some(to)→Active(to) / Some(from)→Active(from) / None·组外→RecoveryRequired；
  非 Switching→Err(NotActiveSource)。plan_switch 对 RecoveryRequired 拒收。
- `TimelineAuthority::reconcile_executed_failure(observed: Option<Uuid>)`——从
  SwitchExecuted/TimelineTransition/TransitionFailed 起：Some(候选内)→Stable+
  epoch+1+恒等段+DiscontinuityDeclared+基线清空+段史 append；None→Ok(None)
  诚实不落地；Stable→Err(InvalidPhase)。未翻转路径复用既有 abort_transition。
- `switch_program` 抽内函数 + 外层 Err 恢复（再观测走 adapter observe·与
  watchdog 同通路·无 inner 死锁；恢复失败只 log warn 不吞原始错误；fence 守卫
  Drop 兜底序不变）。
- 失败位点全覆盖（含 R62 矩阵未列的第三闩锁位点：①a 稳态 PTS 回退→组未动而
  timeline 卡 TransitionFailed——本轮 reconcile 覆盖并加测试）。
- 强制编译臂 5 处登记: program_execution create / switch_mock build_program_graph /
  switch_graph build_program_graph / gates a204_obs 如实截断 / plan_switch。

（执行后回填: 实际 diff 清单 + 强制调用点核对 + 盒矩阵计数）

## 4. A2 异常路径矩阵（R1-R8 + degraded + 0a + replay·执行后回填）

每项验证 Desired / Observed / switch_epoch / TimelinePhase / 下一次 switch /
同 command_id replay ≡ 原始 outcome。R5 如实披露: settle 段唯一 Err 源=
on_program_pts 矛盾（adapter observe 无 Err 通道），R5 由 R4 类覆盖不单独注入。

（执行后回填: 矩阵结果表）

## 5. 真机正常路径回归（执行后回填）

故障→恢复主证=mock 全链（真机故障注入仍无旋钮·R62 已实证编译期常量·R64 再议
注入通道不本轮擅自加）；真机 leg=正常 A→B/B→A/回读/teardown 零回归证明。

（执行后回填: 冒烟结果 + bin md5）

## 6. 披露与红线核对（执行后回填）
