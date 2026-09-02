# Tasks — a2-7-execution-materialization

> 四栏纪律。Gate 链（用户裁定冻结）：00 SoT/Ownership Probe → 01 Fact 
> Shape/Ownership → 02 Custody → 03 Execution→Master→Join→Snapshot → 
> 04 Mock/Simulation 验证 → 05 真机前置 → A2-8 双输入真机切换。

- [x] 1. A2-7-00 SoT/Ownership Probe: **分账**（已有八项实锚禁重造:
  materialize L529-549/MediaBackend SPI 五方法/Session 冻结链 L11+L530/
  watchdog b1-b3+a4 闩锁/Bus 词表/PipelineHandle+Health/outputs 投影/
  Production 等 Intent; 缺失五项=Execution Fact boundary/Custody/三 
  Master writer/Metadata producer/JoinInput 装配全零）+ 9 候选七维裁表 +
  六问 A-F 证据（**关键披露: SWITCHED/PROGRAM_COMPOSED 执行事实不存在**
  ——管线无独立 Switcher/Composition 节点）+ 十项禁止清单 + OQ-1..5 交裁;
  报告=docs/superpowers/reports/2026-09-03-a2-7-execution-materialization-
  sot-probe.md
  `Contract: A2-6 终裁 Gate 链+分账要求+十项禁止` | `Implementation: 已` | 
  `Verification: 分账两侧全实锚·零 .rs diff` | `Gate: 无`
- [ ] 2. 用户对 OQ-1..5 逐项裁决（01 输入; 含 SWITCHED/COMPOSED 推进方式:
  deferred vs 声明性推进; Metadata producer 归属）
  `Contract: 用户裁定权` | `Implementation: 待` | `Verification: 裁决记录` | `Gate: 无`
- [ ] 3. A2-7-01..05 按 OQ 裁决推进（Fact Shape/Ownership→Custody→链路→
  mock 验证→真机前置）
  `Contract: Gate 链` | `Implementation: 待` | `Verification: 后续核` | `Gate: 后续定`
