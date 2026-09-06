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

## 3. A1 设计落名（实现清单·已执行核对）

实际 diff = 9 文件 +853/−194（git diff --stat）：

| 文件 | 改动 | 性质 |
|---|---|---|
| switch_execution.rs | `SwitchDesired::RecoveryRequired{from,to}` + `SwitchError::RecoveryRequired(SwitchDesired)` + `reconcile_switch(observed)` + plan_switch 拒收臂 + 单测 ×2 | 域状态机本体（非核心四） |
| program_timeline.rs | `reconcile_executed_failure(observed)`（SwitchRequested/SwitchExecuted/TimelineTransition/TransitionFailed → Stable{observed}+ProgramEpoch+1+恒等段+DiscontinuityDeclared+基线清空+段史 append；None→Ok(None) 诚实停留；Stable→InvalidPhase）+ 单测 ×3 | 域状态机本体（非核心四） |
| program_execution.rs | `switch_program` 抽 `switch_program_locked` + 外层 Err 统一 `recover_after_failed_switch`（再观测经 adapter observe·与 watchdog 同通路·不取 inner 锁）+ create 拒收臂 + F1 盲 abort 移除（Observed-first 统一）+ 两处过期注释更新 | **核心四·二轮裁决开启** |
| adapters/switch_mock.rs | build_program_graph 拒收臂（强制编译点）+ **新鲜度谓词修正 ×2**（install/switch：`plan.epoch != av_epoch+1` → `<= av_epoch`）+ 纵深测试更新 | **核心四·强制+语义（见下）** |
| adapters/gstreamer/switch_graph.rs | 同上四点（拒收臂 + 谓词 ×2 + 纵深测试）——R53 correctness 面零触碰 | **核心四·强制+语义** |
| gates/a204_obs.rs | RecoveryRequired 同款"如实截断"break 臂 | 强制编译点 |
| switch_dispatch_plane.rs | classify 显式 RecoveryRequired→PermanentFailure 臂 + 快照断言 | 词表快照（架构评审动作） |
| watchdog.rs | 折叠钉子单测（RecoveryRequired→consistent=false 不动作） | **watchdog 行为零改动** |
| tests/switch_fault_probe.rs | R62 五测试更新为恢复语义 + 新矩阵 7 测试 | 纯测试面 |

**新鲜度谓词修正（R63-A 执行中发现并闭合的第二缺口，如实登记）**：
两 adapter 原按 `plan.epoch != graph.av_epoch + 1` 精确锁步校验。begin 后失败（如 F1 adapter
Err 未执行即返）会留下**合法 epoch 间隙**（组平面已消费 epoch、adapter 未执行）——恢复后重试
plan epoch > av_epoch+1 → StalePlanEpoch **永久拒绝**，R1/R7（恢复后再 A→B）不可能成立
（该缺口在 R62 前被"组闩锁根本走不到重试"掩盖，修复暴露）。修正：stale 判据改为
`plan.epoch <= av_epoch`（重放已执行世代拒收——防重放锚保留；未来 epoch 新鲜度归组平面
plan/begin 的精确 epoch 消费域）。成功路径本就 `g.av_epoch = plan.epoch`（av_epoch 语义=
"本次执行纪元"，契约 :33 一致）——干净运行下数值逐字节不变（hw 268 既有断言零改动全过即证）。
语义变化点：adapter 层"伪造未来 epoch 拒收"纵深移除（归组平面），两处纵深测试由
"伪造未来 epoch"改为"重放已执行 epoch"（mock install 位点 + gst switch 位点）。

## 4. A2 异常路径矩阵（结果·mock 全链 9/9）

| 项 | 注入 | 命令 outcome | Desired 落定 | switch_epoch | Timeline 落定 | 下一次切换 | replay |
|---|---|---|---|---|---|---|---|
| ctrl | 无 | Preserved | Active(b) | 1 | epoch0/seg1/DD | — | — |
| F0 | anchors Err | Failed(Backend) | Active(a) 零变化 | 0 | 零变化 epoch0 | ✓ Preserved | — |
| R1+R7 | adapter Err（observed=from） | Failed(Backend) | **Active(a) 回旧源** | 1（已消费） | abort 语义 epoch0 零变化 | **✓ A→B Preserved epoch2** | — |
| R2+R8 | fence 确认 Err（observed=to） | Failed(Backend) | **Active(b) 按观测落 B** | 1 | **Stable{b}+epoch1+DD+恒等重开** | **✓ B→A Preserved epoch2** | — |
| R3+R6 | 证据超时（真实 5s 常量路径） | Failed（**不伪装成功**） | Active(b) | 1 | Stable{b}+epoch1+DD | ✓ B→A Preserved | — |
| R4(+R5 类) | settle PTS 矛盾 | Failed | Active(b) | 1 | Stable{b}+epoch1+DD | ✓ B→A Preserved | — |
| degraded | observed=None | Failed | **RecoveryRequired{a,b} 终态（不猜）** | 1 | 诚实停留 epoch0 | **Permanent RecoveryRequired**（确定性两次逐字节同） | ✓ |
| 0a | ①a 稳态 PTS 回退（**R63 新登记闩锁位点**） | Failed(Backend) | Active(b)（组未动·epoch 不变） | 1 | Stable{b}+epoch1+DD（闩锁解除） | ✓ B→A Preserved | — |
| replay | F2 失败+恢复后同 command_id 二发 | 原始 Failed | （已落 B） | — | — | 新 id=B→A Executed | **Replayed 逐字节（status/detail/classification/kind 全等）** |

单元测试补充：switch_execution（reconcile 三路+降级终态拒收）、program_timeline（落地
NewEpoch 恒等重开/None 诚实停留/稳态闩锁恢复+InvalidPhase+last_outcome 不洗）、
watchdog（RecoveryRequired→consistent=false 零动作）、dispatch_plane（classify 快照+新词表）。

**盒矩阵**：fmt CLEAN / default 229（基线持平）/ simulation 229（持平）/ **mock 411+9=420**
（lib 405+6 新单测；probe 6−4 旧+7 新）/ hw 268（持平·谓词修正零回归证）/ clippy×3
（default/mock/bmd+gstreamer）`-D warnings` 全清。

## 5. 真机正常路径回归（证据盒 r63a-recovery·md5 盒=origin 全等）

- hw 服务 bin 重建（build-bmd）后指纹 **f6e6303bd8ed1540c41a79522c6473cd**（R63-A 源新构建）；
  manifest 7521d17e 不变；**重建先于真机运行**（R62 工程坑纪律——cargo test 会把 debug bin
  重建为 mock 特性）。
- health=Capturing（3 pipelines/3 devices）；API A→B **executed**（av_epoch=1
  outcome=preserved timeline_epoch=0）→ 回读 observed=B/seg=1/**discontinuity_declared**/
  v+a continuous（**R53 冻结签名逐字兑现**）→ API B→A executed（av_epoch=2）→ 回读
  observed=A/seg=2/同签名。
- stop_session executed → svc.log **teardown 完成行=1 + watchdog 停止旗退出行=1**；服务常驻
  为设计语义（诊断模式 session teardown ≠ 进程退出），取证后显式停止。
- 故障→恢复主证=mock 全链（真机故障注入仍无旋钮·R62 已实证编译期常量；R64 再议注入通道，
  本轮不擅自加）。

## 6. 披露与红线核对

1. **红线核对**：contracts/switch.rs·transport.rs·api_boundary.rs·command.rs·idempotency.rs·
   bin/* 零触碰（diff 清单实证）；`complete_switch` observed==to 单律未动（switch_execution
   原文保持）；`force_release` 语义未动（CutoverFenceGuard 零 diff）；watchdog 行为零改动
   （仅折叠钉子单测）；R53 correctness 面（switch_graph 观测/派生路径）零触碰——该文件仅
   加拒收臂+谓词+测试。
2. **R5 如实披露**：settle 段唯一 Err 源=on_program_pts 矛盾（adapter observe 返回非
   Result·无 Err 注入通道）——R5 由 R4 类覆盖，不单独注入。
3. **F2 注入口径**（沿 R62）：即时同类 Err 与真机 5s 超时共用同一 `?` 传播点；F3/F4 走真实
   常量/矛盾路径（elapsed≥4s 断言保持）。
4. **新鲜度谓词语义变化**（§3 详述）：adapter 层未来-epoch 纵深拒收移除（归组平面）；
   防重放锚保留；干净运行数值不变（hw 268 零改动全过）。核心四中 switch_mock/switch_graph
   各 1 强制编译臂 + 2 谓词位 + 1 测试更新，逐处登记。
5. **服务常驻语义澄清**：首轮冒烟曾误标 "PROCESS-STILL-ALIVE (unexpected)"——诊断模式
   服务设计为常驻（R59 "进程死亡" 记录来自采集脚本显式停止）；修正脚本后重跑，teardown
   链+watchdog 退出以日志行验证，最终证据以第二轮为准。
6. **Mimosa**：commit 时 python_ast_unavailable ×4（工具局限·不宣称项目安全）；r63a 冒烟
   脚本 HOME 拼接"命令注入" advisory（受控盒上证据脚本·非生产码·误报语境，如实披露）。
7. **盒上 vs CI**：本节全部数字=盒上本地验证口径（非 GitHub CI）；CI 信号另见 PR #30。
8. plan-mode 只读门拦截 codegraph CLI 与 comet 探针（§1 已披露）；未进入 Comet workflow。
