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
