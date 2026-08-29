# Comet Design Handoff

- Change: p06-g-arch-portability-gate
- Phase: design
- Mode: compact
- Context hash: f1750380450d7ea068bf793e83045096ac02063f2414c4f7b2f3e7e1ea63baf6

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p06-g-arch-portability-gate/proposal.md

- Source: docs/openspec/changes/p06-g-arch-portability-gate/proposal.md
- Lines: 1-19
- SHA256: 0945e822cddbfafaa28497e2dd1d07b9abbf4b327a6360c68117ca3b3fc1080a

```md
# Change: Phase 0.6 C4 — ARCH-PORTABILITY-01 解耦门禁 (0.6G)

## Why

- 裁决顺序要求「先冻契约 → 过两道门禁 → 降级 BMD/GStreamer 为 Reference Adapter」。ARCH-PORTABILITY-01 是第一道门禁：验证删去 BMD Provider 后 Domain/Graph/Session/Supervisor/Health 仍能编译。
- 当前 Test A 编译不过（P0 缺口），证明 `media-agent` 仍耦合 vendor crate；不先过此门禁，0.6B+C 的 SPI 降级无法被验证。
- 这道门禁是 0.6H/I 多门禁的前提，也对应「无消费方不建抽象」之外的硬隔离验证。

## What is Changing

- 新增架构门禁测试 `ARCH-PORTABILITY-01`：在 `--no-default-features --features simulation`（或 mock-only）下断言 `domain` / `graph` / `session` / `supervisor` / `health` 模块可独立编译，不引用 `bmd` / `gstreamer` 适配器。
- 若 C1 的 SPI 抽取已落地，此测试应 PASS；若仍耦合，则本 change 负责补完解耦（必要时回补 C1 遗漏的调用点）。
- 门禁接入 CI（与 clippy `-D warnings` 两套 feature 同列为 required gate）。

## Impact

- 编译：`default` / `simulation` / `bmd-provider,gstreamer-backend` 三套均须保持可编译；
- 受影响：若发现残留耦合点，需在本 change 内补 `use` / 依赖调整（与 C1 同范围，不引入新能力）；
- 后续铺垫：0.6H/I 的 ARCH-BACKEND-01 / RESOURCE-01 等门禁。

```

## docs/openspec/changes/p06-g-arch-portability-gate/design.md

- Source: docs/openspec/changes/p06-g-arch-portability-gate/design.md
- Lines: 1-40
- SHA256: 309ea655c725c60c524f453cb62344490c3aba1e2a7b964544237ba1e68cf3d3

```md
---
title: "Phase 0.6 C4 (0.6G): ARCH-PORTABILITY-01 解耦门禁 — 技术设计"
change: p06-g-arch-portability-gate
change_id: p06-g-arch-portability-gate
comet_change: p06-g-arch-portability-gate
role: technical-design
spec: openspec
canonical_spec: openspec
links:
  - "[p06-g-arch-portability-gate](p06-g-arch-portability-gate)"
---

# Design: Phase 0.6 C4 — ARCH-PORTABILITY-01

## 门禁定义（来自 IMPLEMENTATION_ADDENDUM）

- Test A：删 BMD Provider 后 Domain/Graph/Session/Supervisor/Health 仍能编译 —— **当前不过，是 P0 缺口**；
- Test B：Mock Provider/Backend 与 GStreamer 实现共享同一 Graph/Session/Supervisor/Health；
- Test C：换 Mock B 实现不改变 Domain/Graph/控制面 UI。

## 实现方式

- 在 `services/media-agent/tests/` 或 `ci/` 增加架构门禁断言：`cargo build --no-default-features --features simulation` 必须成功编译上述模块；
- 通过 `cfg(feature)` + trait 边界确保 Domain 层不 `use bmd` / `gstreamer` crate 顶层；
- 复用 C3 的 `simulation` Mock 适配器作为「无真实硬件」编译基线。

## 与 C1 的关系

- C1 冻 SPI；C4 验证 SPI 是否真正解耦。若 C4 仍 FAIL，说明 C1 有残留耦合（如某 `use gstreamer::…` 漏改），本 change 负责最小化补完（不扩大范围）。

## 关键约束

- 门禁只验证「可编译 / 不耦合」，不改变运行时行为；
- canonical 管线语义、V0.2 核心定义不变；
- 防自动 Fallback 语义仍由 C2 Preflight 保证。

## 不做（本 change 边界）

- 不做 ARCH-BACKEND-01 / RESOURCE-01（0.6H/I）；
- 不做 HW-PORT-01 真机回路。

```

## docs/openspec/changes/p06-g-arch-portability-gate/tasks.md

- Source: docs/openspec/changes/p06-g-arch-portability-gate/tasks.md
- Lines: 1-36
- SHA256: 224d074a972202da8fa7c83a5c934cb9c24f4be417b44bd761d0f4f3a7ff8144

```md
# Tasks: Phase 0.6 C4 (0.6G) — ARCH-PORTABILITY-01 解耦门禁

## 1. 门禁测试 (Test A) — 编译门禁

- [x] 新增 `cargo build --no-default-features --features simulation` 架构断言：Domain/Graph/Session/Supervisor/Health 可独立编译
- [x] 新增 `cargo build --no-default-features --features mock` 架构断言：纯 Rust Mock Provider/Backend 下上层可独立编译（解锁 Test B/C 的 Mock 侧）
- [x] 确认不引用 `bmd` / `gstreamer` crate 顶层（词法门禁 `scripts/check_arch_portability.py` 覆盖）

## 2. 补完解耦（结果：无需补完）

- [x] C6 (BMD 诊断探针收敛) + C7 (GStreamer 解耦) 已完成; 词法门禁当前 **0 违规** PASS,
      编译门禁 (`simulation` / `mock`) 当前均 **OK (0 error)** PASS → 无残留耦合点需补完
- [x] 三套 feature 均可编译: default / simulation / `bmd-provider,gstreamer-backend` (C6/C7 盒上验证)

## 3. 门禁接入 CI

- [x] `scripts/check_arch_portability.py` (ARCH-PORTABILITY-01 词法 lint) 接入 `media-agent.yml` `test` job
      —— 禁止 domain/contracts/runtime 层出现 `decklink`/`gstreamer`/`ffmpeg`/`srs`/`aja` 的 crate 路径引用
      (跳过注释/字符串/cfg 门控区; 经 `crate::adapters::{gstreamer,blackmagic}` 收敛门面访问允许)
- [x] 两个编译门禁 (`--no-default-features --features simulation` / `--features mock`) 接入 `test` job 为 required gate
- [x] Test B / Test C: Mock 侧已由 `mock` feature 的 `cargo build` + `cargo test --features mock` (87 passed) 覆盖;
      Mock vs 真实共享 Graph/Session/Supervisor/Health 已通过 `HARDWARE_PROVIDER_CONTRACT` / `MEDIA_BACKEND_CONTRACT` 定型

## 4. 验证

- [x] `cargo clippy --all-targets -- -D warnings` (default / simulation / mock + gstreamer-backend + bmd,gstreamer
      + bmd-provider,gstreamer-backend + bmd-provider,gstreamer-backend,mock + hardware-test) 全 0 error
- [x] `cargo test` default (84) + simulation (84) + mock (87) passed
- [x] ARCH-PORTABILITY-01 Test A PASS (删 BMD/GStreamer Provider 后仍可编译: `simulation` / `mock` 构建 OK)
- [x] 词法门禁反向自测: 注入未门控 `use gstreamer::prelude::*;` 被正确捕获; 字段名 `gstreamer:` 正确放过

## 5. 提交修复 (2026-08-28)

- [x] 初版 `46c9a11` 因 CRLF 归一化命令 bug (`open(p,'w').write(open(p).read())` 先截断后读空)
      将三文件以 **0 字节** 提交, 导致 CI 门禁假绿 (空脚本静默 exit 0)。
- [x] 修复: 三文件以**真实内容 + LF** 重新提交 (先读后写), 门禁实跑 PASS、YAML 校验合法。

```
