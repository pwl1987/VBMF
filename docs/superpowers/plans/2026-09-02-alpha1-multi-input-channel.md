# 执行计划 — alpha1-multi-input-channel

---
base-ref: d2a24fb35f6361126809b43d1cf1129cbe5a9d7d
comet_change: alpha1-multi-input-channel
design_doc: docs/superpowers/specs/2026-09-02-alpha1-multi-input-channel-design.md
---

> TDD 硬律; cargo 经盒; 基线 mock 245 / FMT_CLEAN。

## Task 0 基线 ✓（245 / FMT_CLEAN / HEAD=d2a24fb / 双卡信号 probe 在案）

## Task 1 多管线编排（session.rs, TDD）

- [x] 1.1 RED: Unit——双设备 intent ⇒ start 后 inputs=2 句柄（device 序=plans 序）; 单设备 ⇒ inputs=1 且 pipeline=Some(首) 兼容
- [x] 1.2 GREEN: `SessionInput` + `MediaSession.inputs` 加法（create 空）+ start 全量实例化/回填
- [x] 1.3 RED+GREEN: 实例化中途失败 ⇒ 已建句柄全 stop + 既有回滚链零孤儿（FailingBackend 第 2 plan 注入）
- [x] 1.4 stop/close 迭代 inputs 逆序停（单输入路径行为不变）

## Task 2 投影 + 输出策略（TDD）

- [x] 2.1 RED+GREEN: `InputRuntimeSummary` + `SessionRuntimeState.inputs` + `ApiSession.inputs`; 顶层 8 键测试原样
- [x] 2.2 RED+GREEN: materialize 多设备 ⇒ 仅首 plan 输出段（次设备纯分析 + warn）; 单设备不变

## Task 3 诊断 + 控制台

- [x] 3.1 `VBMF_DIAG_INPUTS`（默认 1; 取已绑定设备前 N; 无 env 现行为）
- [x] 3.2 控制台输入行 + 聚合色（CH 编号=页面规约）

## Task 4 Gate 与交付

- [x] 4.1 盒上 `~/a1_gate.sh`: A1-01..06（双卡真机, 信号实况自适应）
- [x] 4.2 全回归: P1a gate + P1b gate + 矩阵 + lifecycle + loopback + transport
- [x] 4.3 D10 行 CLOSED + review + verify 报告 + archive + PR + CI + merge + memory
