# R64：Control Plane / Switch Recovery 综合验收（基线 faa9b8d）

## 1. 用户裁决 → 落实映射

| 用户条目 | 落实 |
|---|---|
| R64-0 基线审计（只读） | §2（HEAD/PR 事实分述 + 触碰面核对） |
| R64-1 真服务恢复矩阵 | 新 gate `src/gates/r64_control_plane.rs`（env `VBMF_A2_8_R64_CP`）：真 BMD + 真 `GStreamerSwitchAdapter::bridged()` 包注入 wrapper + 真 transport/idempotency/api_boundary，全 HTTP 闭环（§3） |
| R64-2 故障→恢复→再切换三态 | 阶段一 C1（observed=from）/C2（observed=to·R62 场景闭环）/C3（observed=None→RecoveryRequired 终态→拒收→teardown）全绿（§4） |
| R64-3 并发综合 | mock `r64_storm` 1 测试 + 真机 storm 场景（§5） |
| R64-4 快照真值 | 切换窗内轮询——observed 全程=旧已提交值、无未来时间戳（§5） |
| R64-5 六平面一致性 | 纯函数 checker + 5 单测 + 阶段一 13 静息检查点全 OK（§6） |
| C2b release 失败 | 阶段二 KNOWN-FINDING 观察段——三跑三态在案（§7，发现#1/#2） |
| R64-6 30min 基线 | `r64-probe/r64-stability-30m.sh`（§9——commit 2 补章） |
| 暴露缺陷→停→报告→修复另裁 | 三条发现登记（§8），生产代码零触碰 |

## 2. R64-0 基线审计（只读）

- 本地 `git rev-parse HEAD` = faa9b8d，worktree clean，`ls-remote` 同分支同 SHA（rev-list 双向 0）。
- `gh pr view 30`：state=OPEN、mergeable=MERGEABLE、head=comet/a2-8-dual-input-switch、base=master——CI 信号通道，未 merge。
- 触碰面核对：`git diff --stat 616543d..faa9b8d -- <R63-A 五文件>` 中**仅 program_execution.rs 变更**（+59/−18=R63-B B3 四处）；switch_execution / program_timeline / switch_mock / gstreamer/switch_graph 在 R63-A 后零触碰——R63-B 未破坏 R63-A 边界。功能面由盒矩阵重跑复证（R63-A 9/9 + R63-B 8/8 含于 mock 429）。

## 3. R64-1 载体：gate `r64_control_plane`（两阶段）

**构造**（a204_obs 世界逐字 + bin 同款控制面）：bootstrap::build 供给 → manifest/GStreamer probe/PortRegistry（2 输入口 fail-closed）→ ResourceRegistry → build_media_adapter_bundle → SessionManager(Diagnostic) → 双输入 create/start → ExecutionGroup → **真实 bridged 适配器外包 gate 私有 `FaultControlWrapper`**（11 trait 方法全委托·R53 correctness face 零触碰；旋钮=场景前置位/后清除：`switch()` 第 N 次委托前 Err / `release_cutover_fence()` Err / `timeline_execution_facts()`→None / `observe()` 置 observed=None）→ ProgramExecutionRuntime::create → register_stop_hook → RuntimeSwitchPlane + CommandIdempotency.with_switch_plane + RuntimeQuery + TransportContext 七字段 → `transport::serve_forever`（127.0.0.1:0）。**矩阵全程经真实 HTTP**（POST /api/v1/commands + GET /runtime·/health·/events/projection）。watchdog 不接线（a204 先例·矩阵确定性，如实披露）。

**epoch 记账口径（真机首跑实测钉死）**：`ExecutionGroup.switch_epoch`=begin 即消费（尝试数）；`ProgramObservation.switch_epoch`=已委托 switch 的 plan epoch（绝对值——C1 后 av=2/group=3，重试 av 直跳 4）。checker 只断言 组≥执行 与 API==内部观测。

**两阶段**：阶段一（干净世界·判据段）C0/C1/C2/STORM/C3→teardown；阶段二（新鲜世界·KNOWN-FINDING 观察段）C2b。exit：0=阶段一全绿+阶段二采集完整（findings 不 gate exit——观察≠判据，R52 纪律）；2=fail-closed。

## 4. 阶段一：真服务恢复矩阵（attempt5 规范跑·bin 31a00197·failures=0）

| 行 | 命令面 | 落定（六平面检查点全 OK） |
|---|---|---|
| C0a/C0b | executed+preserved（av=1/2） | Active(A)·tl=0·R53 签名 continuous ✓ |
| **C1** observed=from（switch 委托前 Err·硬件未动·全真实） | dispatch executed+class unknown（不伪装成功） | 落 Active(from=A)；av=2 保持（g=3 已消费）；tl=0（abort 回 Stable） |
| C1-replay / C1-retry | replayed 原样逐字节 / 新 id executed preserved av 直跳 4 | 恢复后合法再切换（R7 真机版）✓ |
| **C2** observed=to（**硬件真翻转**+证据抑制→真 5.0219s 超时·R62 场景+恢复闭环） | executed+unknown，detail=证据超时 FailClosed | 落 Active(to=A)+ProgramEpoch(1)+DD+identity 段 seg6；av=5；replay 原样；A→B 再切 preserved@epoch1+re-proved continuous（R8 真机版）✓ |
| **STORM**（无注入·真切换窗 0.87s） | first preserved+replayed 原样+queued 串行 executed | 窗内四路并发查询全 200<2s；真值序列窗内 observed=B（不读未来）→A→B；终态唯一 Active(B)·av=8·tl=1；查询只读（连续 GET epoch 不变）✓ |
| **C3** observed=None（接缝注入·披露） | executed+unknown | 落 **RecoveryRequired{B,A} 终态**（状态机拒绝猜测·不自动恢复）；闩锁后 API observed=真实 B；replay 原样 |
| C3-next（新 id） | 被拒（真机形态=①a PTS 回跳 FailClosed·unknown——发现#3；不变量=拒收+零委托+状态不变 ✓） | RecoveryRequired 保持（checkpoint OK） |
| C3-stop | stop_session executed | teardown：is_active=false·observe=None·API program_switch=null·sixplane[teardown] OK ✓ |

## 5. R64-3/4 并发综合 + 快照真值

- **mock**（429 全绿含新 `r64_storm_switch_query_replay_matrix`）：5s 证据窗内 T1 切换+T2×2 查询+T4 health+T5 events 同窗全 200<1s；窗内 GET observed=旧已提交 a（不读未来）；同 id replay 等待 InFlight 后 Replayed 且 detail/classification/command_id 逐字节等；异 id 排队串行（≥3s）后 executed preserved；终态 epoch=2 唯一 Active(a)；连续 observe 不改 epoch（查询只读）。
- **真机**：storm 场景（真实切换窗）四路并发查询全 200<2s；快照真值轮询（50ms）——窗内（av=6）样本 observed 全=B、observed_at_ms 单调且无未来时间戳；切换后 av 恰进至 8、终态唯一 Active(B)。
- R63-B 六门复核：连接并发/查询不阻塞/切换串行/replay/慢读者隔离（B-T6 于 mock 429 内）/真机——全部经本盒矩阵+规范跑复证。

## 6. R64-5 六平面一致性审计

gate 内纯函数 `check_quiescent_consistency`（静息快照 ①命令 ②幂等 ③组 desired/switch_epoch（group_arc 直读——**零 API 扩张**）④程序观测 ⑤时间线 ⑥API 投影）判定：活跃态 API==内部观测（同源两条读出路径）、Authority source==adapter observed、组尝试数≥执行数；RecoveryRequired 终态不约束 observed==desired（闩锁后观测可恢复=设计语义）；Teardown 全缺席。5 个单元测试入 hw 测试腿（273 内）。阶段一 **13 个静息检查点全 OK**（init/c0/c1×2/c2×2/storm/c3×2/teardown/p2-init/p2-teardown）——无任何"无法解释的跨平面矛盾"。

## 7. 阶段二：C2b release 失败——三跑三态 KNOWN-FINDING（在案）

`release_cutover_fence` 注入 Err（与真机 5s 确认超时同传播点）。**四跑覆盖三态**（attempt1=发现跑 / attempt2=L2 / attempt3=L3 / attempt4=L2 / attempt5=L3 规范跑）：

| 分支 | 恢复观测 | 落定 | Drop 强释后 | 探针 | 出口 |
|---|---|---|---|---|---|
| **L1**（attempt1） | Some(from) | Active(from)+NewEpoch | **迟到物理翻转到 to → 静息分歧 + 死锁**：切 to 被适配器 already-active 拒、切 from 被组平面拒 | 双向 permanent | 唯一 teardown |
| **L2**（attempt2/4） | None | RecoveryRequired 终态（诚实停留） | observed=to 与终态闩锁并存 | 双向 permanent（recovery required） | teardown |
| **L3**（attempt3/5） | Some(to) | Active(to)（与物理恰好一致） | 一致 | →to permanent 拒；→from **作为合法切换成功执行**（av=2 preserved——自洽可服务） | 正常 |

**共同不变量（规范跑断言全过）**：命令 Failed 不伪装成功（executed+unknown）；同 id replay 原样；按分支探针不变量；stop_session teardown 兜底出口可用。gate 语义：findings 记录（12 项）不 gate exit——观察≠判据。

## 8. 发现登记（三条·归用户裁决，未在线修——红线遵守）

1. **[P1] C2b 迟到物理翻转 → 静息分歧/死锁**：真机物理 cutover 在 `release_cutover_fence`（屏障打开）时完成而非在 `switch()`（switch_graph.rs `force_open` 只开屏障不回拨 selector）。release 失败路径的恢复再观测发生在 guard Drop 强释**之前**，而观测在屏障拆除/物理翻转未落定窗内**非确定**（三跑三态）——L1 分支落 Active(from) 后强释完成翻转，desired(from)/observed(to) 永久分歧且**任何切换命令都无法成功**（死锁），唯一出口 teardown。真实 5s 确认超时与此注入同传播点同状态后果——非注入伪影。mock F2（r63_r2 落 to）掩盖此分歧（mock switch 即翻转）。
2. **[P2] 恢复观测非确定（屏障拆除窗内）**：同一注入四跑出现 Some(from)/None/Some(to) 三态——R63-A0 "Observed 优先"契约在"观测时点物理未落定"时判据本身不稳定；L2（RecoveryRequired）分支是契约安全形态，L3 恰好一致，L1 最坏。
3. **[P2] C3 终态拒收形态被 PTS 伪影遮蔽**：C3 类失败（fence 武装→强释周期）留下时间线 PTS 基线伪影，下一次切换在 ①a 即 FailClosed（undeclared backward jump·3/3 干净世界跑复现，video/audio 平面各见过）——命令面 surfaced 分类 unknown 而非契约预期的 permanent(recovery required)。状态机本身未破坏（拒收+零委托+状态不变 ✓），属可诊断性缺陷；且该失败恢复将时间线 rebase NewEpoch(+1)（c3-after-reject 检查点 tl 1→2 在案）。

**修复方向（建议·待裁）**：恢复落定移至 guard Drop 强释**之后**（先强释定物理、再观测落定）可同时消 #1/#2；#3 归 ①a 基线在 fence 周期后的重置语义。均涉核心域文件——按红线**另裁修复轮**。

**红线核对**：生产服务代码零触碰（本轮源内=gate 新文件+mod.rs 一行+bin/gates.rs 派发与 env 清单+mock 测试文件加 1 测试）；发现即停即报未在线修；complete_switch/force_release 语义禁改；Mimosa 纪律（.sh 经 Write+scp -r；一次合并命令被拒后已按纪律拆分执行；HOME 拼接 advisory 同先例披露）；盒上≠CI 分述。

## 9. R64-6 30min 稳定基线

（commit 2 补章——脚本 `r64-probe/r64-stability-30m.sh`：60 周期×30s A↔B + 每 5 周期 replay + 2s 并发查询环 + 30s RSS/fd/threads/frames 采样 + 60s /events 稀疏采样；九项显式验收谓词。）

## 10. 盒矩阵与验证闭环

- 盒矩阵（终态代码·显式退出码）：fmt check=0；clippy default/mock/hw ×3 全 0；default 229 / sim 229 / mock **429**（411+9+9·含 r64_storm）/ hw **273**（268+5 checker 单测）；`cargo build --bin media-agent-gates --features bmd,gstreamer` exit-0（**31a0019765b4213d616e839337892dc6**）。
- 规范真机跑：gates bin 31a00197 + manifest md5 先行钉扎 → `VBMF_A2_8_R64_CP=1` → **exit 0（PASS-AND-FINDINGS）**，failures=0 findings=12。证据盒 `evidence/bmd-10.30.15.10/r64-recovery/`（9 件：header/canonical gate-run + attempt1-4 全档 + mock-storm-matrix + md5s——**md5 盒=origin 逐字节**）。
- 过程披露：attempt1（发现跑 exit2/22F）与 attempt2/3（双分支版 exit2/各 5F/3F——均为场景预期 vs 真机非确定分支的预期修正，非系统新缺陷）全档保留；clippy 曾因 doc 行首 `+` 列表标记 lint 一度红（已修）；真机净切换 ~400ms（storm 全程 0.87s），轮询 50ms。
