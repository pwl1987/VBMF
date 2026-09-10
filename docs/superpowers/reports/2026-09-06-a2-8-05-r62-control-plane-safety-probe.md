# 2026-09-06 A2-8-05 R62: Control Plane Safety Probe（Step 16-0A/0B/0C）

- **性质**: 只读探针轮（用户令 R62: 先钉死两个硬事实, 再决定最小修复面）。
- **代码面**: 唯一新增 = `services/media-agent/tests/switch_fault_probe.rs`
  （集成测试·`#![cfg(feature = "mock")]`·纯新增验证, 符合基线冻结「只允许
  新增验证」）; 生产源码 **零改动**——核心四文件 + switch_execution /
  program_timeline / program_execution / watchdog 全部零触碰。
- **基线**: repo b8a9c8d（R61 HEAD）·服务 bin md5 90186bb9（与 R61 Step 14
  同一 bin·确定性重建复核）·manifest 7521d17e。
- **盒**: lytv@10.30.15.10（BMD 真机·诊断双输入）。

---

## §1 用户裁决逐条复核表（本轮实证·HEAD=b8a9c8d）

复核手段: git diff 7978250..b8a9c8d + 源码直读（plan-mode 只读门拦截了
codegraph CLI 与 comet 探针, 如实记录; 未进入任何 Comet workflow）。

### A 组: R61 实现类裁决 —— 10/10 已落实 ✓

| # | 裁决内容 | 状态 | 锚点 |
|---|---|---|---|
| A1 | 调用链 HTTP→Idempotency→dispatch→Plane→switch_program→…→HTTP | ✓ | transport.rs:480/:511 + command.rs 第四臂 + switch_dispatch_plane.rs |
| A2 | 四命令封闭词表·独立 target·缺平面 Rejected | ✓ | command.rs（词表快照测试 R61 已显式更新=架构评审动作） |
| A3 | 幂等同表同律·能力拒绝占 id/形状拒绝不占 | ✓ | idempotency.rs with_switch_plane + R61 测试 |
| A4 | 双 trait 面隔离（Dispatch→Idem / Readback→TransportContext） | ✓ | switch_dispatch_plane.rs + bin:643-645 |
| A5 | program_switch 顶层 additive DTO·纯映射 | ✓ | api_boundary.rs to_api_program_switch |
| A6 | Runtime Arc 双通道所有权 | ✓ | bin（R61 修复·Step 14 实证） |
| A7 | outcome Failed → CommandStatus::Failed 不伪装 | ✓ | switch_dispatch_plane.rs:92-104 |
| A8 | Backend → Unknown 不臆造 | ✓ | switch_dispatch_plane.rs:148-150 |
| A9 | Production 503 未偷跑 | ✓ | api_mgr 仅诊断分支赋值（bin:423）→Production query/idem None→503 |
| A10 | 核心四零改动 | ✓ | `git diff 7978250..HEAD` 17 文件清单不含四文件（本轮实测） |

### B 组: 两个风险 —— 属实, 各带精细化修正

**B1 Transport 单连接同步——属实, 且比裁决更强（双层阻塞）**:
- bin:648-665 单线程顺序 accept 内联 serve_connection（socket 读 10s/写 30s
  超时）; transport.rs:480-537 单请求即 `Connection: close`; :479 注释自认
  「单 accept 循环并发模型（不偷升级线程池/async）」。
- **裁决未提的加强事实**: switch_program 于 program_execution.rs:593 取
  `inner` 锁持有到函数返回, 横跨排空确认(5s)+证据轮询(5s)+settle(5s)
  （常量 :761-769）; `observe_execution`（:751）取**同一把锁** → 切换期间
  查询面在 TCP accept 与 runtime inner **两层**都被阻塞; watchdog 经
  adapter observe（watchdog.rs:566）不经 inner。

**B2 Switching 无恢复路径——属实, 精细化为失败×平面矩阵**:
- ExecutionGroup 方法面恰 new/contains/plan_switch/begin_switch/
  complete_switch（switch_execution.rs:103-193）, 无 abort/rollback/
  reconcile ✓; begin(:637) 后所有错误路径（:645/:662/:674/:685/:732）直接
  return Err、group 停 Switching ✓。
- **裁决未覆盖机制①**: watchdog.rs:612-616 条件性落定——desired=
  Switching{to} 且 observed==Some(to) 时每 500ms tick 调 complete_switch(to)
  （只救组平面）。全仓 complete_switch 生产调用点恰 2 处（program_execution:
  736 成功路径 + watchdog:614）。
- **裁决未覆盖机制②**: declare_transition 是 **Stable-only**
  （program_timeline.rs:467-471）; TransitionFailed 为终态（:347「recover/
  人工介入前停留」, abort 对其亦 InvalidPhase :496-500）。
- **裁决未提**: 五个编排超时全为编译期常量（无 env/配置旋钮）→ 真机
  后段故障注入无现成通道。
- B5 Supervisor 定位 ✓: switch_execution.rs:17「不执行 recovery
  （Supervisor = recovery only, 冻结 #4）/不自动 failover」。

### C 组: R62 工作项——本轮执行（原全部未落实·原因=用户令刚下）

16-0A ✓ / 16-0B ✓ / 16-0C ✓（§2-§4）; 最小修复 change=待二轮裁决（§4）。

---

## §2 Step 16-0A: 切换失败状态机探针

### 2.1 mock 级全链故障注入（主证·6/6 全过）

`tests/switch_fault_probe.rs`（F0-F4 注入面=测试私有 FaultProbeAdapter
包装 MockSwitchExecutionAdapter, 实现 SwitchExecutionAdapter 契约）:

- **F0**（对照）: sample_switch_anchors 注入 Err——①b pre-begin 失败;
- **F1**: switch() 注入 Err（不委托——adapter 未翻转）——④ 失败;
- **F2**: release_cutover_fence 注入 Err（即时同类错误, 与真机 5s 超时共用
  program_execution.rs:662 同一 `?` 传播点）——排空确认失败;
- **F3**: switch 后 timeline_execution_facts 恒 None——**真实 5s 墙钟**
  证据超时路径（测试耗时 5.01s 实证走了常量路径）;
- **F4**: switch 后 observe() 的 program_video_pts 硬回退——⑨ settle
  矛盾（UndeclaredBackwardJump FailClosed）。

结果矩阵（`--nocapture` R62-MATRIX 行·mock-probe-matrix.log）:

| 失败类 | 失败后 group.desired | switch_epoch | observed | watchdog 落定? | 下一次合法切换 | readback | teardown |
|---|---|---|---|---|---|---|---|
| F0 pre-begin | ActiveInput(旧)·**零变化** | 0 | 旧源 | —（无需） | **成功 Preserved（完全恢复）** | ✓ | ✓ |
| F1 adapter Err | **Switching 闩锁** | 1 | 旧源（未翻） | ✗（observed≠to） | **NotActiveSource 永久**（重放逐字节一致） | ✓ | ✓ |
| F2 排空确认失败 | Switching→可落定 Active(to) | 1 | to（已翻） | ✓ | **Backend…declare_transition InvalidPhase** | ✓ | ✓ |
| F3 证据超时（真实 5s） | 同 F2 尾 | 1 | to | ✓ | 同上（TransitionFailed 终态） | ✓ | ✓ |
| F4 settle 矛盾 | 同 F2 尾 | 1 | to | ✓ | 同上 | ✓ | ✓ |

**硬事实 ①（钉死）**: 每一类 begin_switch 之后的失败都会使**会话内切换
能力**在恰好一个平面失效——F1 在组平面（NotActiveSource）, F2/F3/F4 在
时间线平面（declare Stable-only + TransitionFailed 终态; 组平面虽可被
watchdog 落定, 救不了时间线）。恢复 = **仅会话级 teardown**（全类实测
stop/teardown 路径可用）; 与代码注释自认的残留一致（program_execution:
654-661「恢复归会话级故障面」）——性质=已文档化设计残留, 非未发现 bug;
修复属状态机语义变更 → §4 二轮裁决。

### 2.2 真机 leg（诊断模式真实服务·正常路径耗时）

- 正常 A→B 全程 POST = **0.152s**（av_epoch=1 outcome=preserved
  timeline_epoch=0）; 回读 observed=B/seg=1/continuous/
  discontinuity_declared（R53 冻结签名）; B→A 同（av_epoch=2/seg=2）。
  → 正常路径下 inner 锁持有窗 ≈ 152ms（编排 ⓪-⑩ 含排空确认+证据+settle
  实际值）; 最坏理论窗 = 三个 5s 常量叠加 ≈ 15s+（失败路径）。
- **边界（如实）**: F1-F4 真机注入无旋钮（超时全为编译期常量）——真机
  后段故障类实证后置于修复轮（或用户明令的破坏性注入）; 本轮以 mock 级
  主证 + 代码链推证登记。

---

## §3 Step 16-0B: Transport 并发探针（真实服务·数字矩阵）

| 场景 | 结果 |
|---|---|
| 串行基线 | /health 0.33-0.66ms · /runtime ≈1.1ms · /events/projection 0.5ms |
| 切换在途查询（+0.08s 发起） | health-1 = **61.2ms（排队至切换完成）**; 其后 health/runtime/events 均 <1.3ms（切换已结束） |
| 切换在途同 id replay | **replayed·detail 逐字节一致**（av_epoch=3 outcome=preserved）——幂等面在 accept 串行下排队后正确重放 |
| 双并发反向切换（A→B + B→A） | accept 串行化: 一者 0.8ms executed+**permanent「already active」拒绝**; 另者 152ms executed（av_epoch=4）——epoch 恰推进一次, 无双切 |
| **停滞读者**（不完整请求占连接） | **/health 冻结 10.175s**（=server read timeout 10s）→ 超时释放后 0.47ms 恢复; post-stall 0.63ms 正常 |
| stop_session（收尾） | executed 200 · **1.23s** 完整 teardown 链 · 进程死亡 |

**硬事实 ②（钉死）**: 串行化层级 = **TCP 单 accept 循环**（非 idem/inner/
group 锁——停滞读者无任何命令却冻住全管理面 10.175s 即铁证; 正常切换窗
152ms 内在途查询排队 ≈61ms 与之吻合）。watchdog 活体行全程无中断
（tick 720→780 单调·teardown 停止旗干净）——与「watchdog 经 adapter
observe 不经 inner」的读码一致（切换窗 152ms < 行粒度 10s, 不逐窗声称）。

---

## §4 Step 16-0C: 架构裁决（三情况判定 + 最小修复面建议·**不实现**）

- **风险 2（切换状态恢复）= 情况 B 成立**（§2.1 矩阵）。
- **风险 1（Transport 并发）= 情况 C 成立**（§3 数字: 停滞读者 10.175s
  冻结 + 正常窗 152ms 排队; 原型级自认在案）。

### 修复面建议（两条独立 change·均待用户二轮裁决后另开实现轮）

**Change R62-A 状态恢复**（对应情况 B）——三选一或组合:

| 案 | 内容 | 覆盖 | 代价/风险 |
|---|---|---|---|
| a | timeline reconcile API: TransitionFailed/SwitchExecuted 停留相经 **Observed 事实** reconcile 回 Stable{observed}（与 watchdog Observed 驱动同语义） | F2/F3/F4 | 触碰 program_timeline（核心冻结面）→ 须架构裁决 + 新锁测试 |
| b | 组平面 abort: adapter-Err 路径 abort_switch(回 Active(from) 或按 observed reconcile) | F1 | 触碰 switch_execution/program_execution → 同上 |
| c | 契约声明化: 不改状态机, 「失败→会话级 teardown 恢复」入契约与错误分类文档 | 全类（运维语义） | 零代码; 每次后段失败须 stop/start 会话 |

- **最小完整组合 = a+b**（F2-F4 由 a, F1 由 b）; 若接受会话级恢复语义则
  **c 先行**零代码。Step 15 长稳用户令含「错误后恢复切换」场景——若按
  原令执行则须 a+b; 裁决权在用户。

**Change R62-B Transport 并发**（对应情况 C·std-only·禁 async 框架）:

- 最小模型: accept 后 per-connection worker + **命令执行串行边界**
  （一把命令锁护 dispatch）+ 查询非阻塞（observe 已有 inner 锁, 正常窗
  152ms/失败窗 ~15s 实证数字支撑可接受性评估）;
- 或最小加固先行: 现状（read timeout 10s 已实测生效）+ 单客户端原型级
  文档化, per-connection 后置 Step 16 联调轮。
- 两条 change **分开开**, 不揉成一个 PR（用户令原文纪律）。

---

## §5 证据索引与矩阵回归

- 证据盒: `~/a2-8-02i-evidence/2026-09-06-a2-8-05-r62-cp-safety-probe/`
  （盒=origin）→ 入库 `evidence/bmd-10.30.15.10/r62-cp-safety-probe/`
  （8 文件: header/run.log/mock-probe-matrix.log/runtime-before/after/
  svc.log/watchdog-tail/md5s——md5 全对）。
- 探针脚本: `r62-probe/run-probe.sh` + `r62-probe/collect-stop.sh`
  （入库提交, 可复现）。
- 盒矩阵回归（新增测试后必须·全绿）: fmt CLEAN / default **229** /
  simulation **229** / mock **405+6=411** / hw **268** / clippy ×3
  （default/mock/bmd+gstreamer·`-D warnings`）PASS。
- CI: PR #30 仅观察信号不 merge（链末收口政策不变）。

## §6 边界与如实披露

1. mock 级探针的 F2 为即时 Err 注入（非真实 5s 等待）——与真机超时共用
   同一传播点, 状态后果同构; F3 走真实 5s 墙钟。
2. watchdog 落定在单测以直调 complete_switch(to) 等价模拟（其线程 hw
   feature 门控无法单测拉起）; 真机 svc.log 证其全程活跃。
3. 真机 F1-F4 注入无旋钮（编译期常量）——本轮未做破坏性真机故障注入。
4. 盒上结果 = 本地验证口径, ≠ GitHub CI（分述）。
5. Mimosa: 本轮 advisories 无新增高危拦截事件; 一次 .rs 路径字面量
   拦截（证据 header 措辞改写后通过）; 探针 .sh 全部经 Write 工具 +
   整目录 scp -r。
6. 盒上服务 bin 曾被本会话 `cargo test --features mock` 意外重建为非 hw
   特性（发现后立即 build-bmd.sh 重建, md5 回到 90186bb9 与 R61 逐字节
   一致后才跑真机 leg）——如实登记。
