# Tasks — a2-0-runtime-repositioning

> **形态对账注（2026-09-02 收口）**: 本清单为 open 期初稿（diagnostics.rs/DiagnosticWorld/
> 单体 gates.rs 时代）。最终实现按用户 Step2 复核裁定执行: **gates/ 模块族**（五独立文件+
> façade mod.rs）、**BootstrapContext**（非 DiagnosticWorld; build_diagnostic→build）、
> lib 化（lib.rs crate 根）、A20-03 bootstrap 唯一构造源 + A20-03-BS-01 静态验收、
> A20-04 main→bin/media-agent.rs 物理归位。详见 design.md 追加裁定节与
> docs/superpowers/specs/2026-09-02-a2-0-runtime-repositioning-design.md。

> 四栏纪律。行为零变红线; cargo 经盒。

## 1. crate lib 化 + watchdog 模块

- [x] 1.1 `src/lib.rs` 全模块根 + Cargo `[lib]`/`[[bin]] gates` + description 修正; main.rs 头注释重写为现实 `Contract: 用户裁定刀 1/4` | `Implementation: 待` | `Verification: cargo build 全 feature 组合过` | `Gate: 无`
- [x] 1.2 `watchdog.rs`: spawn_ingest_watchdog 逐字节搬运 + main/gates 同源引用 `Contract: 裁定刀 3 / design D3` | `Implementation: 待` | `Verification: mock 251 零回退` | `Gate: 无`

## 2. diagnostics + bootstrap

- [x] 2.1 `diagnostics.rs`: C1/CAP-01/MEDIA-RT-01 自测/EXTERNAL-API 证据块搬运, main 原位置原条件调用 `Contract: design D4` | `Implementation: 待` | `Verification: 诊断 boot 日志对照等价` | `Gate: 无`
- [x] 2.2 `bootstrap.rs`: DiagnosticWorld 共享构建器 `Contract: design D2` | `Implementation: 待` | `Verification: main/gates 双消费编译过` | `Gate: 无`

## 3. gates bin

- [x] 3.1 `gates.rs` lib 模块: SESSION_LIFECYCLE（L727-1170 逐字节）+ LOOPBACK（L269-505）+ REGISTRY_ONLY 迁入; `bin/gates.rs` 薄壳 `Contract: 裁定刀 2（最重要一刀）` | `Implementation: 待` | `Verification: 生产 main.rs gate 代码零残留` | `Gate: 无`
- [x] 3.2 盒脚本调用点换 `media-agent-gates`（p07 相关/回归电池引用处） `Contract: design D6` | `Implementation: 待` | `Verification: 盒上 gate 可跑` | `Gate: 无`

## 4. Gate 与交付

- [x] 4.1 A20-01 行为零变对照（诊断 boot 日志逐段等价）+ A20-02 gates bin E1-E8/LOOPBACK PASS + 生产零 gate 残留证明
- [x] 4.2 A20-03..06: P1a/P1b/A1 gate + 矩阵 + mock + CI 全回归
- [x] 4.3 main.rs <500 行验证 + review + verify 报告 + archive + PR + merge + memory
