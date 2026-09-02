# Design — a2-0-runtime-repositioning（高层框架）

## D1 crate 形态: lib + 双 bin（路径零漂移策略）

```
src/lib.rs          # 全模块根（既有模块 + 新 watchdog/gates/diagnostics/bootstrap）
src/bin/media-agent.rs  # media-agent 生产 bin = Composition Root（A20-04 物理归位终态; <500 为 review target 非硬指标——用户裁定）
src/bin/gates.rs    # media-agent-gates 薄壳（env 分发 → lib::gates::*）
```

- gates 逻辑放 **lib 模块** `src/gates.rs`（`pub fn session_lifecycle_gate(ctx)` / `loopback_gate(ctx)`）——保持 `crate::` 内部路径, bin 只做 env 分发; 生产 bin **不调用** gates 模块（链接器可裁; 符号在场但生产路径零引用, 由 A20-02 语义证明"生产运行零 gate 行为"）。
- CI hardware-test-compile `cargo build --features hardware-test` 自动编译双 bin（gates 编译过 = CI 顺带验证）。

## D2 bootstrap.rs（组合根构件共享）

`pub struct DiagnosticWorld { cfg, mode, devices, bindings, registry, lm, sup, event_logs, mgr, ctrl, agent_state, ... }` + `pub fn build_diagnostic() -> DiagnosticWorld`——main 与 gates 共用（复制构建 = 漂移源, 单一构建器 = 组合根唯一）。生产路径构建差异（manifest 校验/无 auto-start）保留在 main 内分支。

## D3 watchdog.rs（逐字节搬运）

`spawn_ingest_watchdog(...)` 函数签名不变整体搬入; Supervisor 边界零触碰（它本来就干净——只决策）。main 与 gates（SESSION-RT-01 内嵌 watchdog 调用）同源引用。

## D4 diagnostics.rs（诊断证据面, 行为保留）

C1 探针块（L113-268, cfg bmd-provider）/ CAP-01 探针 / MEDIA-RT-01 自测 / EXTERNAL-API-RT-01 证据打印 → 独立模块, main 诊断 boot **按原位置原条件调用**（输出逐行不变——这些在诊断 boot 总会出现, 不迁 gates bin, 否则行为变）。

## D5 Program Domain 腾位（不实现）

lib.rs 模块声明区预留注释锚: `// A2-1+: program (Channel/SwitchPolicy/Masters/MasterJoin/ProgramMaster)`——位置声明, 零类型。

## D6 冻结/风险

- **行为零变红线**: 生产路径输出/时序/相位逐段等价（A20-01 对照跑）; gates bin 仅入口换名（gate 内部逻辑逐字节搬运）。
- 风险: 大块搬运漏改路径 → 全部保 `crate::`（lib 模块内自然成立）; bin 内引用经 `media_agent::`。
- 盒 gate 脚本（不入库）调用点换 `media-agent-gates`（SESSION_LIFECYCLE/LOOPBACK/REGISTRY_ONLY 三 env）。

## 用户 2026-09-02 Step 2 复核裁定（追加, 以此为准）

1. **gates/ 模块族非单体**: `gates/{mod,config_probe,resolver,loopback,registry,session_lifecycle}.rs`; mod.rs 只做 façade `run()`。绝不形成"第二个 main.rs"。
2. **顺序重排**: A20-02 Gate Extraction（先搬, 显式参数即可, 13 参数不怕——验证"搬出去不改变行为"）→ A20-03 Bootstrap Extraction（收口共享依赖; **硬边界: bootstrap 只构造不运行**——禁 session/pipeline start、watchdog、recover、sleep、gate 断言、process::exit、HTTP accept）→ A20-04 lib 化终刀（lib.rs crate 根; **main.rs → src/bin/media-agent.rs**; bin/gates.rs; Cargo description/头注释/旧 skeleton 注释统一在此刀修, 不在搬运步产生无关 diff）。
3. **Gate 语义纪律**: Gate = 调用 Production Runtime 做验收, **不自造第二套 Runtime**; LOOPBACK 现存自建 registry/event sink = **"Gate-local diagnostic construction, A2-0 不合并 production runtime"**（Design 显式标记, 防误认为正式初始化路径）。
4. **A20-02 六条验收**: ①生产默认路径零 Gate 执行 ②生产二进制不再 dispatch 五 env（dispatch responsibility 归 gates）③`cargo run --bin gates` 可进入 ④watchdog.rs 零漂移（Step 1 状态不动）⑤mock 251 全绿 ⑥硬件编译矩阵 default/simulation/mock/bmd-provider/bmd+gstreamer。
5. 最终形态: lib(Domain/Contracts/Runtime/Adapters/Bootstrap/Watchdog) + media-agent(Production Root) + gates(Diagnostic Root)。
