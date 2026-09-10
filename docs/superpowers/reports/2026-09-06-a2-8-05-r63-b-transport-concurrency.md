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

## 4. B1/B2/B3 实现（已执行核对）

实际 diff = 3 文件 +89/−28 + 新测试文件（git diff --stat）：

| 文件 | 改动 | 性质 |
|---|---|---|
| transport.rs | 新 `pub fn serve_forever(listener, ctx)`（accept → read/write timeout → `ctx.clone()` → `std::thread::spawn(serve_connection)`）+ 模块头/serve_connection 并发模型自述更新 | 允许面 |
| bin/media-agent.rs | 内联 accept 环路一行化改调 `transport::serve_forever`（−9 行） | 允许面 |
| program_execution.rs | B3 最小开启（逐处登记）: ①`published: Mutex<Option<ProgramExecutionObservation>>` 字段（Inner 外）②create 发布初始快照 ③switch_program 出口发布（成败与 R63-A 恢复后）④teardown 清空（投影块诚实缺席契约保持）⑤`observe_execution` 改 try_lock 短锁+快照回退 | **核心四·B0 审计证明必须** |
| tests/control_plane_concurrency_probe.rs（新） | B-T1..T7 + B3 直测（真实 socket + `serve_forever` 本体 + SuppressFactsAdapter 真实 5s 证据窗） | 纯测试面 |

**B2 落实**：零新锁——切换串行边界=既有 `inner`（双反向并发以后到者在边界排队，窗后合法执行）; idempotency 未动（同 id InFlight 等待+replay 同表）。**核心域文件（switch_graph/switch_execution/program_timeline/contracts/switch_mock/command/idempotency/api_boundary）零触碰实证（diff 清单）**。

## 5. B4 并发测试矩阵（mock 全链 8/8·真实 socket·5s 证据窗）

| 项 | 结果 | 指标 |
|---|---|---|
| b3 直测 observe_execution | ✅ 窗内每次回退已提交快照（observed=旧源）→ 切换后新鲜（恢复落 B·epoch=1·teardown 后 None） | **worst_query=19.9µs**（5s 窗内 10 次采样） |
| B-T1 /health during switch | ✅ 3× 200 | worst=840µs |
| B-T2 /runtime during switch | ✅ 200+投影块在·窗内 observed=旧源（快照证）→切换后=B | 816µs |
| B-T3 /events during switch | ✅ 200 | 653µs |
| B-T4 同 id replay during switch | ✅ 二发等 InFlight 完成后 detail/classification/command_id 逐字节 identical·原 outcome=Failed 不洗 | first=5.01s / second=4.99s（等待证明） |
| B-T5 双反向并发 | ✅ inner 串行: 首个 5s Failed+恢复落 B → 第二个窗后合法 executed preserved·epoch 恰 +2·终态唯一 Active(a)（**恢复后连续切换=B6 语义锚**） | 两者全消费 |
| B-T6 慢读者隔离 | ✅ 部分请求挂连接期间 /health 照常 200 | **718µs（R62 同形=10.175s 全管理面冻结）** |
| B-T7 N=16 并发 /runtime | ✅ 16/16 200 | worst=1.43ms |

R63-A 恢复矩阵 9/9 零回归（同轮复跑）。**盒矩阵**: fmt CLEAN / default 229 / sim 229 / mock **411+9+8=428** / hw 268 / clippy×3（default/mock/bmd+gstreamer）退出码 0 全清。

## 6. 真机 B5/B6（bin 1326be285e0ee2aee43648c6f2f82b52·重建先行·证据盒 r63b-concurrency 14 件 md5 盒=origin）

- **B5-1** 后台 A→B（executed·av_epoch=1·preserved）+ 窗内并发 /health 644µs·/runtime 1.19ms·/events 607µs·/health 581µs（全 200）→ 回读 observed=B/seg=1/DD/continuous（R53 签名）。
- **B5-2** 后台 B→A（av_epoch=2）+ 窗内 750µs/1.02ms/672µs → 回读 observed=A/seg=2/同签名。
- **B5-3 慢读者隔离**: 部分请求挂连接 ~8s，期间 t+1s=1.08ms / t+4s=962µs / t+8s=1.01ms 全 200——**R62 实测同形 10.175s 全管理面冻结的修复实证**。
- **B5-4 replay**: 同 id 二发 = 1 executed + 1 replayed·detail_identical=True·classification_identical=True；回读 seg=3。
- **B5-5** N=8 并发 /runtime 全 200（680µs–1.04ms）。
- **B6 正常链**（A→B→Query→B→A→Query 并发全覆盖于 B5-1/2/4）+ stop_session → **teardown 完成行=1 + watchdog 停止旗退出行=1**（服务常驻=设计语义·取证后显式停止）。故障→reconcile 段真机仍无注入旋钮——mock 主证（B-T5+b3+R63-A 矩阵）+如实披露，注入通道归 R64 议。

## 7. 披露与红线核对

1. **红线核对**: 核心域文件续冻兑现（diff 仅 3 文件+测试）; std-only（无 tokio/axum/hyper/tower·仅 std::thread）; Connection: close 不变; complete_switch/force_release 未动; 五端点集合与 wire 形状零变化。
2. **残留如实**: 无连接数上限（诊断 127.0.0.1 回环前提·正式化归反向代理层）; during-switch 查询=上一次已提交事实（诚实时间戳在载荷——B-T2 的 observed=旧源即该语义的可见证明）。
3. **真机脚本工程事件（如实）**: 首版脚本裸 `wait` 把脚本自 nohup 的常驻服务当后台 job 等待→挂起（本地任务停止+盒上残留服务锚定清理）; 手工收尾命令两处笔误（SID= 前缀未剥+嵌套引号）产生一次 invalid_session_id 工件并致服务未经 stop_session 被杀——**规范证据以修复脚本后的完整重跑为准**（本报告 §6 全部数字出自该轮）; 归档脚本=修复版（B5-5 显式 PID 等待）。
4. **Mimosa**: commit 时 python_ast_unavailable ×4（工具局限·不宣称项目安全）; r63b 探针脚本 HOME 拼接"命令注入" advisory（受控盒上证据脚本·非生产码·误报语境，如实披露）。
5. **盒上 vs CI**: 本节全部数字=盒上本地验证口径（非 GitHub CI）; CI 信号另见 §交付段（gh 实测+PR checks·远端/本地分述）。
6. 用户注记回执: 您指出 GitHub 工作流查询接口未返回 616543d 记录——本仓侧 gh 实测 run 34028205578 head=616543d conclusion=success（上一轮已记）; 本轮新 commit 的 CI 另行 gh 实测并分述。
