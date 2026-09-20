# SE-01A — Standalone graceful shutdown 收口报告（STANDALONE-ENTRY-01 S3/S4）

- Date: 2026-09-19
- Packet: SE-01A（§3.61 冻结子包 1/4）
- Authority: `docs/superpowers/plans/2026-09-19-standalone-entry-01-planning.md`（S3/S4 + §5 + §6）
- Implementation chain: `eb49718`（shutdown 模块 + 双 runtime 接线 + Starting 置位）→ `e3e7fe4`（测试 env 泄漏 RCA 修复）。BMD smoke @ `e3e7fe4`。
- Evidence: `evidence/bmd-10.30.15.10/2026-09-19-se01a-graceful-shutdown/`

## 1. 实现

- `src/shutdown.rs`（新模块，lib 一等公民）：
  - `GracefulShutdown`：self-pipe 信号处理——handler 仅 `write`（async-signal-safe），等待线程阻塞 `read`（读端阻塞化，无空转）；安装 singleton fail-closed，**失败路径回滚写端原子**（RCA：二次 install 失败若不回滚，handler 将写向已关闭 fd，等价于关闭整个 shutdown 通道——该缺陷在开发中被测试挂起暴露并修复）；第一个终止信号返回前恢复 SIGTERM/SIGINT 默认处置（第二次信号 = 运维逃生门）；SIGHUP 捕获但 warn 无操作（manifests startup-only，frozen D3；SIGHUP 默认动作是终止，必须显式捕获）。
  - `stop_all_sessions`：经 SessionManager 唯一 owner 的有序 drain——**按 `created_at` 降序**（RCA：`runtime_state()` 投影来自 HashMap，无创建序；首版按投影逆序在并行测试中随机翻转，已改为时间戳排序 + session_id 确定性 tie-break）；只停持运行态会话（pipeline 已物化/Running/Starting/Degraded）；stop 失败记录并继续。
- `bin/media-agent.rs`：`GracefulShutdown::install()` 先于模式选择（安装失败 = fail-closed exit 2）；两个 runtime（Device/Network-only）都以 `shutdown.wait()` 常驻，收到终止信号后经 `stop_all_sessions` drain 再 exit 0——**SIGTERM 不再孤儿化 FFmpeg listener 子进程**；device 路径 drain 诊断 api_mgr 会话。
- `bootstrap::build()` 初始 `AgentState::Starting`（S4）；组合根装配完成置 Ready（device production 装配后 / network-only 组合后）；诊断路径保持会话启动后置 Capturing。

## 2. 测试与 RCA（failure-first）

- **RCA-1（hang）**：二次 install 失败路径未回滚 swap → handler 写已关闭 fd → wait 永久阻塞。修复：回滚 + 读端阻塞化。
- **RCA-2（flaky ordering）**：drain 顺序依赖 HashMap 投影序 → 并行测试随机失败。修复：created_at 降序 + tie-break。
- **RCA-3（CI failure @ 9067989，真实缺陷）**：`rejects_diagnostic_mode` 测试设置 `MEDIA_AGENT_MODE=diagnostic` 无退出清理，CI 调度序下泄漏到 closure production-entry 测试（模式选择翻转 fail-closed 拒绝），panic 中毒 `STARTUP_ENV_MUTEX` 后 4 个测试级联 PoisonError（5 failed/354 passed）。修复（`e3e7fe4`）：`StartupEnvGuard`（Drop 清理，panic 亦生效）+ `lock_startup_env()`（抗中毒获取）。修复后 ffmpeg-backend lib 套件连续 9 次全绿。
- 覆盖：信号字节决策表；真信号生命周期（raise/install 单例/HUP 无操作/TERM 终止/Drop 重装——单一串行测试，pipe 是进程级 singleton）；mock drain（空/跳过 Released/逆序多会话，ms 隔离创建时间）。
- 覆盖缺口（如实）：进程级 binary 测试未入库（Mimosa 写入钩子拒绝 `Command::new(env!(...))` 测试形态，三轮重构后放弃，改由 BMD smoke 承担）；真实 binary 上"SIGTERM 时存在活跃会话"的 drain 演示不可行（production 零自动启动 + 控制面未接，P1-3），语义由 mock 套件 + TG-6 gate stop 链锚定，留待 SE-01B。

## 3. 软件验证（Development VM，e3e7fe4 最终态）

default **326/326**；simulation **326/326**；mock **541/541**；ffmpeg-backend **361 passed + 1 ignored**；clippy×3 `-D warnings`；fmt；architecture lint；remove-adapters；diff-check 全 PASS。
CI：`eb49718` run `35489769503` 7/7 success（泄漏未在该调度复现）；`9067989` run `35487902795` rust-test-matrix FAIL（上述 RCA-3，由 e3e7fe4 修复——历史证据保留）；`e3e7fe4` run `35489924796`（见 STATE 收口记录最终结论）。

## 4. BMD exact-commit smoke（e3e7fe4；archive `c715d373…`，binary `82abccb3…`）

真实 production binary（`--features ffmpeg-backend`）+ 真实运维形态 manifest（0600/machine-pin）：
- 启动零设备行为（discovery/lease/adapter/SDK probe = 0 行）；
- `/health` = `state:Ready, devices:0`（S4 readiness 落地）；
- SIGHUP → 存活 + frozen-D3 warn 行；
- SIGTERM → **exit 0** + `draining network sessions` → `drained count=0` → `graceful shutdown complete`；
- 零 ffmpeg/listener 残留；device-2 PID 992634 未触碰。
详见 `se01a-manifest.md`。

## 5. 收口判定

SE-01A 验收（plan §5 行 1）满足：mock stop 顺序/零孤儿单测 ✓；ffmpeg-backend kill-TERM 语义（经真实 binary smoke 的 exit-0 路径 + gate 既有 stop 链）✓；Device 生产零回归（全矩阵 + CI）✓；CI 7/7 @ e3e7fe4（见 STATE）；BMD network-only kill→clean teardown smoke ✓。无 frozen stop condition；未扩大边界（/health wire 未变、无新 owner、manifests 仍 startup-only）。下一子包 = SE-01C。
