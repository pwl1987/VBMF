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
- [x] 2. 用户对 OQ-1..5 逐项裁决（2026-09-03 终裁落 probe §8: OQ-1=事实
  驱动+缺失则 Deferred·**否掉声明性推进**[Intent≠Execution Fact]·b1/b3
  禁自动命名 NormalizeComplete / OQ-2=无 producer 恒 UNKNOWN fail-closed·
  否掉 config/manifest / OQ-3=Custody attribution first·Join bool injection
  second·禁机械等价与 FailureDomain enum / OQ-4=复用双 PTS measurement·
  分类独立·不建 Engine / OQ-5=独立 Custody·SessionManager 协作不拥有·
  Supervisor 不介入; A2-7-00 CLOSED）
  `Contract: 用户裁定权` | `Implementation: 已` | `Verification: probe §8` | `Gate: 无`
- [x] 3. A2-7-01 Fact Shape/Ownership Probe（Probe+设计先行; **核心任务
  查死 NORMALIZED**）: **决定性发现——normalize 声明被 Materialization 
  静默忽略**（PipelinePlan.normalize 在 GStreamer controller 零消费;
  实际链 src→caps→appsink 无 normalize 元素, true/false 生成管线相同;
  唯一 videoconvert 在 output 编码分支=delivery 侧）→ b1/b3=RAW ingest 
  acceptance 非 normalize completion → SOURCE_RAW→NORMALIZED 从"✅ 可
  实现"再收紧为 **Deferred**（事实前提=Adapter 实插 Normalize 元素+可观测
  完成点, 属 02+ Execution Adapter 侧）; 附带发现: normalize=声明与执行
  缺口（intent 声明 V0.2 Normalize 能力未实现）; 四空白: ① Fact Shape 
  五域候选（禁万能 struct）/② attribution 底座已备（PipelineBusEvent
  {handle,source} element 粒度 + PipelineFault{pipeline}）/③ Metadata 
  source 全库零→维持 UNKNOWN fail-closed→当前唯一合法 ProgramMaster=
  join_result:None / ④ Custody lifecycle 形态建议（独立模块+三触发挂点
  候选+单向依赖禁反向接线）; OQ-6..9 交裁; 报告=docs/superpowers/reports/
  2026-09-03-a2-7-execution-materialization-01-fact-probe.md
  `Contract: 00 终裁 §8 四空白+核心任务` | `Implementation: 已（零 .rs diff）` | 
  `Verification: normalize 零消费 grep 实锚+四空白全证据` | `Gate: 无`
- [ ] 4. 用户对 OQ-6..9 终裁（normalize 缺口处置/NORMALIZED 事实前提归属/
  Fact 形态+Custody 挂点/Metadata producer 长期归属）
  `Contract: 用户裁定权` | `Implementation: 待` | `Verification: 裁决记录` | `Gate: 无`
- [ ] 5. A2-7-02..05（Custody/链路/mock 验证/真机前置）按 OQ 裁决推进
  `Contract: Gate 链` | `Implementation: 待` | `Verification: 后续核` | `Gate: 后续定`
