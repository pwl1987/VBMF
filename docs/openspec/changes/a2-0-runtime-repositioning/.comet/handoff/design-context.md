# Comet Design Handoff

- Change: a2-0-runtime-repositioning
- Phase: design
- Mode: compact
- Context hash: e0a317acc23ec28b79174b6a42d8d8ce46232046e9069b85d9c0709675db02fc

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/a2-0-runtime-repositioning/proposal.md

- Source: docs/openspec/changes/a2-0-runtime-repositioning/proposal.md
- Lines: 1-29
- SHA256: b44a662d17d1626ad2b482348dba2445e58ea0c1fd837e3a477b2bfba90beb7b

```md
# Proposal — a2-0-runtime-repositioning

## Why

用户 2026-09-02 架构级复核 + Reality Audit（`docs/superpowers/reports/2026-09-02-vbmf-reality-audit.md`）裁定：**实现重心偏移**——main.rs 105,837 字节承载组合根 + 三段真机 gate（编译进生产二进制）+ ~350 行 watchdog 循环 + 诊断代偿，四职责合一。裁定 A2-0 先于一切 Program Master 代码开工：**四层归位, 不改 V0.2, 不增媒体能力, 行为零变**。

main.rs 头注释仍自称 "Gate 2 skeleton + Gate 5/6/7 scaffolding"; Cargo.toml description 仍写 "Hardware Plane...Control Plane lives in Node/Fastify"——均与代码现实严重不符（用户逐条确认）。

## What Changes（四刀, 用户定义）

1. **main.rs → Composition Root**: 只留 config → adapter selection → dependency construction → runtime wiring → transport start → process lifetime。目标 <500 行。
2. **Gate 从生产二进制隔离（最重要一刀）**: `VBMF_SESSION_LIFECYCLE`（SESSION/RESOURCE/RUNTIME-STATE/COMMAND/EVENT E1-E8, main.rs L727-1170）与 `VBMF_LOOPBACK`（HW-PORT-01D, L269-505）+ `VBMF_REGISTRY_ONLY` 迁入**独立 gates bin**（`media-agent-gates`）; 真机 gate 入口从此不编译进生产 executable。
3. **Watchdog 独立 Runtime 模块**: `spawn_ingest_watchdog`（L~1451-1804, observe→fold→signal→event→fault→supervisor→backoff→recover 全链）→ `watchdog.rs` domain 模块（Supervisor 决策边界不动）。
4. **Program Domain 腾位**: lib 化后的模块布局为 `Channel/SwitchPolicy/Masters/MasterJoin/ProgramMaster` 预留明确位置（A2-1 起; 本 change 只腾位不实现）。

结构形态: crate **lib 化**（src/lib.rs 全模块根）+ 双 bin（`media-agent` 组合根 / `media-agent-gates` 验收入口, gates 逻辑放 lib 模块保 `crate::` 路径零漂移, bin 为薄壳）; C1 探针/MEDIA-RT-01 自测/EXTERNAL-API 证据打印 → `diagnostics.rs`（诊断 boot 行为逐字节保留）; 共享构建 → `bootstrap.rs`（main 与 gates 共用组合根构件）。头注释 + Cargo description 修正为现实。

## Non-Goals

- 任何 Switch/Program Master/Master Join 语义（A2-1..A2-8）; 任何 V0.2 变更; Control Plane 实现（A4, 只腾位）; 盒脚本功能变化（仅 gate 调用换二进制名）

## 验收场景（Gate A20-01..06）

1. **A20-01** 行为零变: 生产 `media-agent` 无 gate env 诊断 boot 日志与 f3f86ef 逐段等价（模块化搬运不语义化）
2. **A20-02** gates bin: `media-agent-gates` + `VBMF_SESSION_LIFECYCLE=1` E1-E8 ALL PASS; `VBMF_LOOPBACK=1` PASS; 生产二进制内 **gate 代码零残留**（符号级证明）
3. **A20-03** watchdog.rs 模块化后 P1a/P1b/A1 gate 全 PASS（watchdog 行为不变）
4. **A20-04** main.rs <500 行且头注释与现实一致; Cargo description 修正
5. **A20-05** mock 251 零回退 + 14 步矩阵全绿 + CI 七 checks（hardware-test-compile 自动覆盖 gates bin 编译）
6. **A20-06** 盒上回归电池: P1a 12 + P1b 11 + A1 gate（gate 入口已换新二进制处更新）

```

## docs/openspec/changes/a2-0-runtime-repositioning/design.md

- Source: docs/openspec/changes/a2-0-runtime-repositioning/design.md
- Lines: 1-34
- SHA256: 6df57b4004bfdc5311798bcbe876f1a234ef9e7831675f842b5299a4ba0012ec

```md
# Design — a2-0-runtime-repositioning（高层框架）

## D1 crate 形态: lib + 双 bin（路径零漂移策略）

```
src/lib.rs          # 全模块根（既有模块 + 新 watchdog/gates/diagnostics/bootstrap）
src/main.rs         # media-agent 生产 bin = Composition Root（<500 行目标）
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

```

## docs/openspec/changes/a2-0-runtime-repositioning/tasks.md

- Source: docs/openspec/changes/a2-0-runtime-repositioning/tasks.md
- Lines: 1-24
- SHA256: 95996ac1f3912b13102c01353c6eb8c93b636999c8ea33dcf2ec57c68e86de82

```md
# Tasks — a2-0-runtime-repositioning

> 四栏纪律。行为零变红线; cargo 经盒。

## 1. crate lib 化 + watchdog 模块

- [ ] 1.1 `src/lib.rs` 全模块根 + Cargo `[lib]`/`[[bin]] gates` + description 修正; main.rs 头注释重写为现实 `Contract: 用户裁定刀 1/4` | `Implementation: 待` | `Verification: cargo build 全 feature 组合过` | `Gate: 无`
- [ ] 1.2 `watchdog.rs`: spawn_ingest_watchdog 逐字节搬运 + main/gates 同源引用 `Contract: 裁定刀 3 / design D3` | `Implementation: 待` | `Verification: mock 251 零回退` | `Gate: 无`

## 2. diagnostics + bootstrap

- [ ] 2.1 `diagnostics.rs`: C1/CAP-01/MEDIA-RT-01 自测/EXTERNAL-API 证据块搬运, main 原位置原条件调用 `Contract: design D4` | `Implementation: 待` | `Verification: 诊断 boot 日志对照等价` | `Gate: 无`
- [ ] 2.2 `bootstrap.rs`: DiagnosticWorld 共享构建器 `Contract: design D2` | `Implementation: 待` | `Verification: main/gates 双消费编译过` | `Gate: 无`

## 3. gates bin

- [ ] 3.1 `gates.rs` lib 模块: SESSION_LIFECYCLE（L727-1170 逐字节）+ LOOPBACK（L269-505）+ REGISTRY_ONLY 迁入; `bin/gates.rs` 薄壳 `Contract: 裁定刀 2（最重要一刀）` | `Implementation: 待` | `Verification: 生产 main.rs gate 代码零残留` | `Gate: 无`
- [ ] 3.2 盒脚本调用点换 `media-agent-gates`（p07 相关/回归电池引用处） `Contract: design D6` | `Implementation: 待` | `Verification: 盒上 gate 可跑` | `Gate: 无`

## 4. Gate 与交付

- [ ] 4.1 A20-01 行为零变对照（诊断 boot 日志逐段等价）+ A20-02 gates bin E1-E8/LOOPBACK PASS + 生产零 gate 残留证明
- [ ] 4.2 A20-03..06: P1a/P1b/A1 gate + 矩阵 + mock + CI 全回归
- [ ] 4.3 main.rs <500 行验证 + review + verify 报告 + archive + PR + merge + memory

```
