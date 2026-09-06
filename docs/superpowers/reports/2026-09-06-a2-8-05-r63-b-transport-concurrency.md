# R63-B：Transport 并发与 Query 解耦（Control Plane Safety Fix 第二刀）

- 轮次: R63-B（用户 R63 裁决：B0 审计先行·B1/B2/B3 三层拆分·两 commit 分离
  架构与实现·真机并发验证后才进 R64/Step 15）
- 基线: R63-A 交付 616543d（失败恢复 + epoch 间隙闭合）
- 性质: 代码轮——transport+bin 允许面 + program_execution（B3 审计证明必须·
  最小开启）; 核心域文件（switch_graph/switch_execution/program_timeline/
  contracts/switch_mock/command/idempotency/api_boundary）续冻零触碰。

## 1. 用户 11 项指令 → 落实映射

| # | 指令项 | 归属 |
|---|---|---|
| 二 | R63-B0 inner ownership 小审计（不改代码） | 本轮 commit 1（本文档 §2-§3） |
| 三 | R63-B1 per-connection std thread | commit 2（§4） |
| 四 | R63-B2 切换串行边界（HTTP 并发≠切换并发） | commit 2（inner=现有边界·零新锁） |
| 五 | R63-B3 查询快照解耦 | commit 2（§4） |
| 六 | B-T1..B-T7 + 延迟指标 | commit 2（§5） |
| 七/八 | 真机 B5 并发 + B6 恢复后连续切换 | commit 2 证据（§6） |
| 九 | 完成门禁六 PASS → R64 | 本轮产出证据·裁决归用户 |
| 十 | 触碰边界（transport+bin 先行·program_execution 审计必须才开） | 遵守（§4 逐处登记） |
| 十一 | 两 commit 分离 B0/B1 | 本轮结构 |
| 主线 | R63-B0→B1→R64→Step15→Step17；Step 15 不启动 | 遵守 |

## 2. 并发契约（B0·冻结文本·2026-09-06 随计划批准即锁）

1. **HTTP concurrency ≠ switch concurrency**: GET /health·/runtime·/events
   可并发；同一 Program 的切换（A→B/B→A）经 `ProgramExecutionRuntime::inner`
   串行——**现有锁=串行边界，不新造全局锁**；idempotency（同 command_id
   exactly-once/replay/conflict）与 inner（Program execution 互斥）职责不同，
   不合并。
2. **std-only**: 无 tokio/axum/hyper/tower；`Connection: close` 协议模型不变
   （不偷升级协议/持久连接）。
3. **Query 快照模型**: 快照 SoT=**既有 `ProgramExecutionObservation`**
   （Clone 派生已在——contracts 零触碰）；runtime 增 **derived published
   缓存**（同型快照·非第二 Runtime State——与 Desired/Execution/Observed
   三分离一致）；`observe_execution` 改 **try_lock 短锁**——锁空闲=真读+
   刷新发布；被切换编排持有=回退最近一次**已提交事实快照**
   （observed_at_ms 在载荷=诚实时间戳；不等待、不伪 None）。发布点=
   create 初始 / switch_program 出口（成败与 R63-A 恢复后）/ 自由读；
   teardown 清空（"teardown→投影块诚实缺席"transport 契约保持）。
   锁序恒 inner→published（无反向路径）——零死锁。
4. **慢读者隔离**: per-connection std thread；10s read timeout 只约束自身
   连接（对照 R62 实测: 单 accept 模型下停滞读者冻结整个管理面 10.175s）。
5. **残留如实**: 无连接数上限（诊断模式 127.0.0.1 回环绑定前提；正式化归
   反向代理层）；during-switch 查询=上一次已提交事实（非执行中实时值）；
   adapter observe 并发安全沿 watchdog 真机先例（R62: 切换编排期 watchdog
   经同通路 tick 720→780 无中断——adapter 内部自锁）。

## 3. inner ownership 审计（B0·读码核定）

`Inner`（program_execution.rs 实构）字段分类：

| 字段 | 分类 | 依据 |
|---|---|---|
| `group: Arc<Mutex<ExecutionGroup>>` | 独立可锁 | 读 desired 只需 group 锁；watchdog :577/:614 已并发使用 |
| `timeline: TimelineAuthority` | inner 持有·纯派生读 | `snapshot()` 无副作用 |
| `switcher + graph` | adapter observe 通路 | watchdog 先例=并发安全（真机实测） |
| `taps / tap_port / watchdog_stop` | teardown 专用 | 不在查询/切换热路径 |
| `inner: Mutex<Option<Inner>>` | **切换编排串行边界（B2·保持）** | ⓪-⑩ 全链 + R63-A 恢复在其内 |

调用面核对: switch_program（长持）/observe_execution（**B3 阻塞点**·同锁）/
teardown（短持·take）/graph_handle·group_arc·switcher_arc·set_watchdog_stop
（短持·clone/写字段）; is_active 无查询面调用; /health 走 agent_state（零
inner 依赖）。**结论**: B3 无法只在 transport 层解决（阻塞点在 runtime 自身
的 observe_execution）——program_execution 按用户第十条"审计证明必须"开启，
最小四处+try_lock（逐处登记于 §4）。

## 4. B1/B2/B3 实现（执行后回填核对）

（执行后回填: 实际 diff 清单 + 逐处登记 + 行为说明）

## 5. B4 并发测试矩阵（执行后回填）

（执行后回填: B-T1..T7 + B3 直测结果表·五项延迟指标）

## 6. 真机 B5/B6（执行后回填）

（执行后回填: 并发计时矩阵 + 慢读者隔离 + B6 正常链 + bin md5）

## 7. 披露（执行后回填）
