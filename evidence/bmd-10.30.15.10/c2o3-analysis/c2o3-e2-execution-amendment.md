# C2-O3-1 / E2 执行勘误与冻结边界

日期：2026-09-11  
性质：**O3-0 已注册设计的可执行性勘误；不改变 RCA 问题、判读表、生产契约或 Step15 verdict。**

## 1. 审计发现

O3-0 登记要求 E2 使用 `VBMF_DIAG_INPUTS=1`，同时写有“runner 不变 / 十谓词不变”。对当前 master 的实际代码与 runner 逐行核对后，发现该表述无法按字面直接执行：

1. `r64-probe/r64-stability-long.sh` 在服务启动前写死 `export VBMF_DIAG_INPUTS=2`；外部传入 `VBMF_DIAG_INPUTS=1` 会被覆盖。
2. 该 runner 随后强依赖 `/api/v1/runtime.program_switch.observed_active`；单输入模式下生产组合根不会构建 Program Graph，因此 `program_switch` 正确缺席，原 runner 会在发现阶段退出。
3. 原 runner 主循环与三个谓词要求真实 `switch_program` 执行；单输入无 `switch_plane` 时，强行制造 PASS 将伪造语义。

因此，不能通过“只改一行输入数”或“让单输入自己切自己”来执行 E2。

## 2. 代码事实

当前生产组合根已经原生支持 E2 所需拓扑，不需要修改 Rust：

- `VBMF_DIAG_INPUTS` 默认值为 1；
- `started_inputs.len() == 2` 时才创建 `ExecutionGroup` / `ProgramExecutionRuntime`；
- 非双输入时保持原单输入 ingest watchdog 路径；
- 因而 `VBMF_DIAG_INPUTS=1` 时应满足：会话仅 1 个 input，`program_switch` 缺席，Program Graph 不构建。

## 3. 执行修正

冻结以下执行形态：

- **生产 Rust：零改动。**
- **原 `r64-stability-long.sh`：零改动。**
- **原 `reanalyze-long.py` 与 Step15 十谓词/+50MB 阈值：零改动。**
- 新增 `r64-probe/c2o3-e2-single-input.sh`，仅作为 O3/E2 diagnostic harness。
- harness 使用同一 build 脚本、同一 manifest、同一 `media-agent` binary 构建路径，显式 `MEDIA_AGENT_MODE=diagnostic` + `VBMF_DIAG_INPUTS=1`。
- 启动后必须 fail-closed 断言：`sessions == 1`、`inputs == 1`、`program_switch == absent`；否则本次 E2 = `INCONCLUSIVE` / abort，不进入观察窗。
- 复用已授权 `c2o2-observer.sh` 的 procfs 数据面与采样频率；不增加线程采样强度，不新增 allocator tracing / GStreamer debug / perf / eBPF / LD_PRELOAD 等禁入面。
- 固定 `CYCLES=315`、`DWELL=28s`、observer `NOMINAL_S=9200`，覆盖已知首事件约 7140–7200s 窗口；不因结果“不漂亮”延长。
- 保留 `/runtime` 与 `/health` 2s 交替查询负载，用于管理面存活/并发查询背景；**不发送伪 switch 命令**，因为单输入无执行平面，发送被拒命令只会引入新的控制面分配混杂因素。

## 4. E2 输出语义

E2 **不产生 Step15 PASS/FAIL**。原 24h `rss_bounded FAIL` 保持不变。

只允许以下结果：

- `SINGLE_INPUT_EVENT_OBSERVED`：单输入仍观察到目标匿名增长事件。允许结论：双输入不是必要条件，候选向 common ingest / native allocation path 收敛。
- `SINGLE_INPUT_EVENT_NOT_OBSERVED`：完整观察窗内未观察到目标事件。允许结论：双输入条件作为必要条件候选升权；**不得写成“双输入导致泄漏”**。
- `INCONCLUSIVE`：拓扑不等价、observer 非 COMPLETE、观察窗不足或运行异常。不得用异常运行做 RCA 证据。

无论哪个分支，E2 都只是候选空间分叉证据；后续候选仍必须通过 O3 六环硬门：

`candidate → allocation path → allocator behavior → same reservation/arena → magnitude → repeatability`

## 5. STOP / 红线

- 本勘误不授权 O4/FIX。
- 不改 V0.2 冻结架构，不改 SessionManager / Switch SPI / ProgramExecutionRuntime / GStreamer adapter。
- 不重跑 24h，不改 +50MB 阈值，不改旧证据，不覆盖旧 runner。
- E2 真机执行完成并完成离线判读后，再决定 O3 下一刀；不得从 E2 直接跳到生产 FIX。
