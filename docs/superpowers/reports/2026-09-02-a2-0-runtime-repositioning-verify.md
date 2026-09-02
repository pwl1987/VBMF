# Verify 报告 — A2-0 Runtime Repositioning（a2-0-runtime-repositioning）

- **Change**: `a2-0-runtime-repositioning`（full workflow, skip_specs:true）
- **分支**: `comet/a2-0-runtime-repositioning`（base `f3f86ef` = master）
- **代码区间**: `f3f86ef..543fc0a`（含 review 修复折入; 结构四刀 + 治理脚本 + 文档对账）
- **日期**: 2026-09-02
- **Design Doc**: `docs/superpowers/specs/2026-09-02-a2-0-runtime-repositioning-design.md`

## 0. 结论

**A2-0 四刀全部落地, 行为零漂移实证, 结构形态冻结为用户裁定终态。**
main.rs 1804 行 → bin/media-agent.rs（组合根）+ bootstrap.rs（唯一构造源）+ watchdog.rs +
gates/ 五模块族 + bin/gates.rs（诊断根）。生产 bin 对五 VBMF_* gate env **零 dispatch 责任**
（静态+动态双证）; Gate=Consumer 非 Bootstrapper; bootstrap 只构造不运行;
V0.2 语义/媒体能力零触碰（审计约束: 不借 A2-0 修 gate 行为/不重构 Runtime/不加媒体能力——全程遵守）。

## 1. 用户两轮复核裁决记录

- **Step2 复核（6406e7f 落档）**: gates/ 模块族非单体; A20-02 先搬（显式参数）→ A20-03
  bootstrap（硬边界只构造不运行）→ A20-04 lib 化终刀; LOOPBACK 自建物 = Gate-local
  diagnostic construction 显式标记; 六条验收门。
- **A2-0 GO 裁定（59abe1a 复核后）**: 十项全 PASS（Bootstrap 唯一构造源/硬边界/双 bin 同源/
  Gate Consumer/BS-01/lib-first/物理归位/adapter guard 同步/V0.2 无意外修改/行为零漂移）;
  **非阻塞观察项**: bootstrap 占位租约终应归 Runtime/Session 生命周期（A2-1+ 债务观察项,
  不重开 A2-0）; 文档一致性检查项（src/main.rs 旧描述——已修）。

## 2. 四刀与证据

| 刀 | 内容 | 证据 |
|----|------|------|
| 1 watchdog | spawn_ingest_watchdog 250 行逐字节出仓（原 cfg 门控保留） | review 逐字节 diff 确认 byte-identical |
| 2 gates/ 族 | 五 env 入口 → {config_probe,resolver,loopback,registry,session_lifecycle}.rs; mod.rs 仅 façade; session_lifecycle 9 显式参数 | 双 gate 经 gates bin 真机 PASS |
| 2c lib-first | lib.rs crate 根（A2-1 腾位锚）; Cargo [lib]+[[bin]]×2+description 重写; clippy 后果最小 allow×3 | 矩阵/四组合 clippy 全绿 |
| 3 bootstrap | build()->BootstrapContext{9 字段, provider 建后不留}; 硬边界（禁 start/watchdog/recover/sleep/gate 断言/HTTP accept）; SDK probe 不进（生产侧标记 production diagnostic wiring 零漂移）; 双 bin 同源; gates TEMPORARY 复制删除 | BS-01 静态验收 PASS; A20-01 锚 |
| 4 物理归位 | main.rs→bin/media-agent.rs（100% git rename 零内容变更）; proof 脚本 BIN_FILES 同步 | git show 确认 rename |

## 3. 回归证据（盒上, 各刀后多轮）

- **矩阵 14/14**（fmt×2 / test×4 155/155/**251**/155 / clippy×4 零警 / build×3 / proof OK
  ——proof 补丁模型扩双 bin 世界）
- **双 gate 经 media-agent-gates bin**: SESSION_LIFECYCLE `ALL PASS` / LOOPBACK `ALL PASS=true`
- **生产 bin 零 dispatch 动态实证**: 带 `VBMF_SESSION_LIFECYCLE=1` boot 生产 bin → 正常启动
  （timeout 挂起, 零 gate 输出）
- **P1a gate 12 + P1b gate 11 + transport 19/0**（编码输出/控制台/五端点零回退）
- **A20-01 黄金锚**（f3f86ef 诊断 boot 2123 行）: 行类集合一致。**可接受差异清单（终版, 含
  review 补录）**: ①tracing target 前缀 `media_agent`→`media_agent::watchdog`（模块化必然）
  ②基线含 bus 溢出风暴 1639 行=瞬态窗口 ③状态变迁计数随外部信号抖动 ④生产 bin SDK probe/
  registry print 移至 bootstrap 行之后（无共享状态, 纯日志序） ⑤gates bin 前置副作用差异
  （无 bindings/registry 预计算日志; bootstrap 占位租约已持有——GStreamer 不消费, 纯内存记账;
  SELFTEST 不再遮蔽 gate env——gates bin 只认五 VBMF_* env） ⑥**env 契约变更**: 生产 bin 不再
  响应任何 VBMF_* gate env（原 `VBMF_REGISTRY_ONLY=1 media-agent` 类运维用法须换
  `media-agent-gates`; gates bin 无 env 命中时 exit(2) 显式提示）。

## 4. Review Gate（standard, subagent 一次全 change @ dc71225）

裁决 **With fixes**; 1 Critical / 1 Important / 4 Minor:
- **Critical#1（BS-01 子串误报——HEAD 实际 FAIL, "PASS"证据过时）**: `PrototypeOutputConfig::from_env(`
  含 `Config::from_env(` 子串; A20-04 归位把含此二行的 media-agent.rs 移入扫描目录后未复跑。
  **已修复（63eeb25）**: 词边界 regex token + 负样本自检（全路径 `Config::from_env(` 抓住 /
  Prototype 变体不误中）; **加固（Important#2）**: 块注释剥离与厂商扫描同法 + bin 清单从
  Cargo.toml [[bin]] path 派生（不假设 src/bin/ 布局）。修复后 HEAD 实跑 PASS。
  **教训记档**: 静态检查类改动在文件移动类提交后必须复跑（PASS-before-rename → FAIL-after
  正是本次滑过的失效模式）; BS-01 已入 CI required job（architecture-portability）自此后每 PR 自动复跑。
- Minor#3/4/5（日志序/gates 前置副作用/env 契约变更）: **接受并全文记档于 §3 可接受差异清单**。
- Minor#6（tasks.md 未勾选+陈旧形态描述/proof 注释 main.rs）: **已修**（tasks 全勾 + 形态对账注
  + 注释修正）; handoff 重新生成同步。

## 5. 用户观察项登记（非阻塞, 转 A2-1+）

- **Lease ownership**: bootstrap 占位租约+排他自检现为初始化态（A2-0 定义）; A2-1+ 进入
  Program Domain 后应评估将其归 Runtime/Session 生命周期（用户 GO 裁定原文登记）。

## 6. CI（PR 后回填）

七 required context: **见 PR 检查记录（合并前全 green 为 Merge Gate 前置）。**
注: architecture-portability job 现含 BS-01（CI 自动复跑, Critical#1 失效模式结构性封堵）。
