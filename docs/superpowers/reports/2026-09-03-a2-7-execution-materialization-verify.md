# A2-7 Verify Report — Execution Materialization（Custody 模型 + 闭环验证）

> Change: a2-7-execution-materialization · Date: 2026-09-03 · Base: master `caab630`
> 提交链：`a45a9d5`（00 Probe）→ `25acafa`（00 终裁+01 Probe）→ `e6627ca`
> （02 首刀）→ `ded0221`（02 修正）→ `22e5e6c`（02 二轮修正 identity）→
> `348fb28`（03 桥+身份反推）→ `9ecc0f1`（03 终裁落实）→ `70ffb5e`
> （04 mock lifecycle）→ 本收口。
> **定位（终裁分层）**：A2-7 交付 = **Custody 模型 + 组件间数据契约闭环
> 验证**；生产 Runtime→Custody 链完整化（failure attribution contract）
> 保留到后续 Event Contract / Failure Attribution change。

## 1. 交付物总账

| 刀 | 产物 | 状态 |
|---|---|---|
| 00 SoT/Ownership Probe | 分账（已有八项禁重造 vs 缺失五项真空白）+ 9 候选七维裁表 + **关键披露（normalize 声明被静默忽略）** + OQ-1..5 终裁（事实驱动+缺失 Deferred·否声明性推进 / 无 producer 恒 UNKNOWN / Custody attribution first / 复用双 PTS / 独立 Custody） | ✅ CLOSED |
| 01 Fact Shape Probe | **决定性发现**（normalize 零消费→NORMALIZED 再收紧 Deferred）+ 四空白 + OQ-6..9 终裁（Gap 登记/b1-b4=Ingest Observation/最小可证事实 fact absent≠false/Metadata 生产权=orchestration） | ✅ CLOSED |
| 02 Custody 首刀（两轮修正） | `custody.rs`：FailureObservation（**pipeline_id=legacy DeviceId 承载**+source+scope）+ attribute_failures（identity correlation+联合匹配）+ custody_snapshot + 4 测试；两轮复核修正：FailurePath[调用方预归因]→FailureScope[SharedPipeline 证据]→pipeline_id 关联[跨实例污染防线] | ✅ CLOSED（三轮 20 Gate） |
| 03 生产桥+身份反推 | `observations_from_events`（回声排除/nil 拒收/单来源）+ **身份 SoT 反推**（PipelineFault.pipeline=设备身份·legacy/misnamed 债务登记·不建 mapping 表） | ✅ CLOSED（三段式） |
| 04 Mock Lifecycle | `custody::lifecycle` 子模块：custody_08 六验收 + custody_09 A/B 双实例反证（零隐藏 mapping） | ✅ CLOSED（**封存名=Mock Closed-Loop Validation 非生产闭环**） |
| 05 收口 | 本报告 + 交付链 | ✅ |

## 2. 05 第一项前置确认（终裁指定）

**"真实 BMD/GStreamer 故障产生带设备归属的 canonical RuntimeEvent"——
前置条件现状 = 不具备**：DefaultRuntimeEventMapper 兜底产
`PipelineFault{pipeline: Uuid::nil()}`（events.rs L181-186），桥 fail-closed
拒收（正确行为）；MockBackend.observe() 为空。**边界**：A2-7 收尾 Custody
模型；生产链三缺口（Backend attributed 事件产生/watchdog 归属/常驻消费
者）→ 后续 Event Contract / Failure Attribution change。

## 3. 盒上全矩阵（p07_verify.sh，14 步 ALL_DONE，总 EXIT=0）

- fmt apply+check：PASS
- test×4：**default 200 / simulation 200 / mock 307 / bmd,gstreamer 200**
  ——全 0 failed（mock 基线 299→**307**，+8 恰）
- clippy -D ×4：EXIT=0 ×4；build×3：EXIT=0 ×3；remove-adapter PROOF：EXIT=0

## 4. 硬件电池

声明性 Custody/桥零执行面——硬件行为零变化；矩阵含 hardware-test build +
bmd,gstreamer 全量 test。真机 gate 无涉及面（custody.rs 无 runtime 接线）。

## 5. 架构成果

- **Execution Fact→Program Semantic Lifecycle 链的模型层闭合**：
  RuntimeEvent→桥→identity correlation→attribution→JoinInput→join→
  ProgramMaster（组件间数据契约，mock 实证）；
- 身份边界三结论：PipelineFault.pipeline=legacy 设备身份承载（V0.3 债务）/
  SessionInput=identity association SoT（禁第二 registry）/ 三身份分层
  （Device/Handle/Session）；
- normalize 声明-执行缺口 = Execution Adapter Gap 正式登记（PipelinePlan
  doc）——可见可追踪不伪装；
- 七不 Custody 红线 + 八红线（04）全守；advance 零触发（无证据不推进）；
  Metadata Unknown fail-closed；AVSync Unknown 维持。

## 6. 债务与遗留（交后续 change）

1. **生产 Runtime→Custody 链**（三缺口，见 §2）→ Event Contract / Failure
   Attribution change；
2. PipelineFault.pipeline 双语义（SourceMaterialized.pipeline=Pipeline
   identity）→ V0.3 Event Contract cleanup；
3. normalize Execution Gap（Adapter 实插元素+可观测完成点）→ A2-7+ 后续
   或独立工作项；
4. Metadata producer（Control/orchestration 语义）→ A4 线；
5. AVSync measurement/classification 通路 → 执行面工作项。
