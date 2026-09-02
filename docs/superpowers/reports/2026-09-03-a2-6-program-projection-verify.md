# A2-6 Verify Report — ProgramMaster Runtime Projection（API Boundary 首刀）

> Change: a2-6-program-projection · Date: 2026-09-03 · Base: master `2166d25`
> 提交链：`c762e3f`/`be7b2b8`（00 终裁+01 Probe）→ `a993a36`（02 原版）→
> `44cd65e`（02 修正：薄镜像 DTO）→ 本收口。
> 定位：**六刀链实质收口于 02**——00/01 双 Probe + 02 投影实现；03/04/05
> （Query 接线/API/Transport）真实 consumer 等 A2-7 ProgramMaster 生产
> 生命周期出现后另裁（A2-6-01 探针实证零 Program 级消费者）。

## 1. 交付物总账（六刀链）

| 刀 | 产物 | 状态 |
|---|---|---|
| 00 Ownership/SoT Probe | 八问全证据（Owner 零现状/join() 零生产调用者=真前置/Snapshot 独立边界/API Boundary 先例/冻结面）+ 禁止捷径红线 | ✅ CLOSED（OQ-1=B 角色 Custody 实现 deferred A2-7 + 双禁令；OQ-2=Deferred + Watchdog 非 writer；OQ-3=独立 snapshot + API 并列 projection；OQ-4/5 deferred to 01；事实修正 allowlist 7+new=8） |
| 01 Consumer/Shape Probe | 七项全证据（真实消费者盘点=零 Program 级/API 语义三分类/None wire 先例/AVSync 透传/并列位置/ProgramQuery 零可查物/唯一转换点）+ 硬 Gate 执行（零假 ProgramMaster） | ✅ CLOSED（OQ-6=`ApiProgramMaster`/OQ-7=null/OQ-8=零挂载只做 DTO+mapper/OQ-9=五字段暴露面） |
| 02 投影实现（含修正） | `ApiProgramMaster{video,audio,metadata,join_result:Option,avsync}` + 三薄镜像 DTO（ApiVideoMaster/ApiAudioMaster/ApiMetadataMaster，字段 1:1 canonical wire shape）+ 四 mapper 显式机械映射 + 8 测试 | ✅ 原版 REJECT（Domain 容器直曝 API）→ 修正版 APPROVED/CLOSED @44cd65e |
| 03/04/05 | Query 接线/API/Transport | ⏸️ 保持延期（A2-7 后消费者驱动） |
| 06 | 本收口 | ✅ |

## 2. 关键边界裁决记档（A2-6 全程）

- **两层权限不可混同**："Canonical types 允许 mapper 消费" ≠ "DTO 字段
  类型等于 Domain 类型"——Domain 容器直曝 REJECT 已修复（薄镜像）；
- **镜像 DTO ≠ 重新解释 Domain**：explicit mechanical mapping（语义来自
  Domain，所有权与演进边界属于 API）；
- **两类区分（终裁 §3，长期边界）**：Domain Container（三 Master）→ API
  必须镜像；Canonical Vocabulary/Leaf Value（Stage/DataPlane/MetadataType/
  Declaration/JoinResult）→ 可直接复用——叶子若开始承载 Runtime/vendor/
  execution 语义须重新判断（**措辞收紧：非"零演化风险"；LOCK FINAL 变化
  须经版本/架构变更流程**）；
- **禁止捷径**：禁从 RuntimeState/GraphRuntimeIntent 重建三 Master 再
  compose（Runtime facts 反推 Program semantics = 边界反向）；
- **硬 Gate 维持**：零假 ProgramMaster（测试底座=真实 join() 产出）；
  零挂载（无 producer 无 consumer 接线=空中楼阁）。

## 3. 盒上全矩阵（p07_verify.sh，14 步 ALL_DONE，总 EXIT=0）

- fmt apply+check：PASS
- test×4：**default 194 / simulation 194 / mock 299 / bmd,gstreamer 194**
  ——全 0 failed（mock 基线 291→**299**，+8 恰：02 原版 7 + 修正版镜像
  保真 1）
- clippy -D ×4：EXIT=0 ×4；build×3：EXIT=0 ×3；remove-adapter PROOF：EXIT=0

## 4. 硬件电池

声明性投影零执行面——硬件行为零变化；矩阵含 hardware-test build +
bmd,gstreamer 全量 test。真机 gate 无涉及面（api_boundary 声明层，无
runtime/transport 接线），不重复跑。

## 5. A2-7 输入清单（消费者驱动接线的未来依据）

- Program Runtime Custody 角色已批（实现 deferred）：receives execution
  facts → advances declarations → invokes join() → publishes snapshot；
- Watchdog 不是 ProgramMaster writer；Event Projection 不成 Join；
- 接线触发条件：A2-7 执行事实链建立 + 真实 Program consumer 出现；
- 届时按需裁：ProgramQuery vs Query Facade / ApiQuerySnapshot 并列
  projection / transport 端点 / None wire 消费语义 / inconsistency 用户
  语义（API 层定义）。

## 6. 债务与遗留

零新增债务。零挂载为**有意裁决**非遗留（A2-6-01/02 双终裁）；03/04/05
延期属六刀链设计内。
