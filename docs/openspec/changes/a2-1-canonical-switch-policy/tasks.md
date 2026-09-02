# Tasks — a2-1-canonical-switch-policy

> 四栏纪律。TDD; cargo 经盒; 基线 mock 251。

## 1. program 模块 + SwitchPolicy（TDD）

- [x] 1.1 RED: 词表快照（恰三词/serde 名逐字/parse 受纳+拒绝含大小写敏感）/ IO 平面+前置约束访问器 / serde 反序列化未知值 fail-closed `Contract: V0.2 §1.17+§313-315` | `Implementation: 已` | `Verification: Unit` | `Gate: 无`
- [x] 1.2 GREEN: `src/program/{mod,switch_policy}.rs` + lib.rs 锚转真实声明 `Contract: design D1` | `Implementation: 已` | `Verification: Unit 全绿` | `Gate: 无`

## 2. PipelinePlan 类型化

- [x] 2.1 RED+GREEN: `switch_mode: SwitchPolicy`（默认 FRAME_SWITCH 不变）; 测试字面量同步; wire 序列化值不变断言 `Contract: design D2` | `Implementation: 已` | `Verification: Unit + mock 251 零回退` | `Gate: 无`

## 3. 回归 + 交付

- [x] 3.1 全回归（矩阵/gates bin 双 gate/P1a/P1b/transport）零退化 `Contract: 验收口径` | `Implementation: 已` | `Verification: 盒上全 PASS` | `Gate: BOX`
- [x] 3.2 review + verify 报告 + 双 guard + archive + PR + CI + merge + memory `Contract: 交付纪律` | `Implementation: 已` | `Verification: PR merged` | `Gate: CI/RELEASE`
