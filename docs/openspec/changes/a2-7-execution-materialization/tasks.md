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
- [x] 4. 用户对 OQ-6..9 终裁（2026-09-03 落 01 报告 §5: OQ-6=normalize 缺口
  正式登记 Execution Adapter Gap·Custody 禁隐式吸收[Intent≠Execution Fact]/
  OQ-7=completion fact 归 Execution Adapter·**b1..b4 正式归类=Ingest 
  Observation/Acceptance Evidence**/OQ-8=最小可证事实模型·**fact absent 
  而非 fact=false**·排除万能 ExecutionFact/OQ-9=Metadata 生产权归 
  Control/Program orchestration·A4=candidate 非唯一 SoT; 全局修正: A2-7 
  不追求 ProgramMaster 一定形成——当前唯一合法=join_result:None; Custody 
  七不终裁; A2-7-02 顺序修正: 先 Fact boundary→再 Custody→最后接 join;
  A2-7-01 CLOSED）
  `Contract: 用户裁定权` | `Implementation: 已` | `Verification: 01 报告 §5` | `Gate: 无`
- [x] 5. A2-7-02 首刀（最小 Fact boundary + Custody）: `src/custody.rs` 
  新模块——`FailureObservation{source:FailureSource 封闭词表, path}` +
  `attribute_failures` 保守归因（管线粒度=设备级 video+audio 同 Handle,
  media path 标注缺失→双路 failed 保守记档, element 级演进 deferred）+
  `CustodyObservations{failures,avsync}`（零第二 SoT: 消费时装配参数包,
  与 MasterJoinInput 同律）+ `custody_snapshot` 最小闭环（consume→归因→
  **advance 零触发**[无 transition evidence, 三 Master 诚实停留初始态]→
  build JoinInput→join→compose）; 4 测试: attribution 保守/无 failure=
  None+初始态不虚推进/failure 穿透 readiness gate[单路→Degraded·双路→
  Failed·AVSync FAILED 不改]/确定性+C′ 不可达; normalize Gap 正式登记
  （PipelinePlan.normalize 字段 doc: Execution Adapter Gap·Custody 禁吸收）;
  **不碰 materialize/SessionManager/watchdog/transport**（终裁禁止清单）
  `Contract: 01 终裁 §5 Custody 七不+OQ-6..9` | `Implementation: 已` | 
  `Verification: 盒上 custody 4/4 + mock 303（299+4 恰）+ clippy 4-combo 
  PASS + fmt clean` | `Gate: 无`
- [ ] 6. A2-7-03..05（链路扩展/mock lifecycle 验证/真机前置）——待用户
  复核首刀后按 OQ-6..9 裁决推进; A2-8 双输入真机切换
  `Contract: Gate 链` | `Implementation: 待` | `Verification: 后续核` | `Gate: 后续定`
