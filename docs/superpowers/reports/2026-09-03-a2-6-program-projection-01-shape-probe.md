# A2-6-01 — Consumer + Projection Shape Probe

> Status: `PROBE ONLY / NO CODE CHANGE`
> Authority: A2-6-00 终裁 §7（七项必查 + 硬 Gate：禁临时创建假 ProgramMaster
> 用于投影；01 严格限定"真实消费者→Projection Shape"，不实现 Custody）
> Date: 2026-09-03 · Change: a2-6-program-projection · Base: master `2166d25`

---

## 1. 七项逐项证据

### 项 1 — ProgramMaster projection 的真实消费者盘点

现有全部 API 消费者（代码级清点）：

| 消费者 | 消费端点 | 消费字段 |
|---|---|---|
| **P1b Web Console**（transport.rs L271+ 内嵌 INDEX_HTML） | `GET /api/v1/runtime`（×2 轮询）+ `/hls/index.m3u8` 探测 | `sessions[].state/.phase`（Start/Stop 按钮）+ `outputs`（播放选择）；**零 program/master 级消费** |
| transport `/health` 线程 | 内部 `TransportContext`（events/agent_state/device_count/query） | 五字段（P1a 起逐字节冻结） |
| events/projection 端点 | RuntimeEventLog 投影 | EventProjection |
| 未来 A2-6 projection | （本 change 交付物） | — |

**结论：当前零 Program 级真实消费者**——与 `join()` 零生产调用者（00 报告
Q2）互为表里：没有 ProgramMaster 快照产生，自然没有消费者。**A2-6-01 只能
产出"消费者画像 + 形态建议"，真实消费者 = A2-7 执行链出现后的 A2-6-02 裁。**

### 项 2 — API Resource 语义（节目语义 vs 运行状态）

`api_boundary.rs` 先例决定语义归属方式：每个 `Api*` 资源的字段**仅由消费
语义驱动，不绑回内部 enum**（L12 纪律 + `ApiSession.state` 用字符串化 wire
值非 Rust enum 序列化）。探针发现现有 API 的**语义分层惯例**：
- 运行状态类（device/port/resource/session）：来自 CanonicalRuntimeState 投影；
- 物化事实类（`ApiSession.outputs: Vec<String>`——P1a"降级不虚报"）：来自
  Execution 侧事实回填。

ProgramMaster projection 的语义按同律应为**第三类：Program semantic 事实**
（三 Master 声明 + Join 结果），非运行状态、非执行事实。命名候选评估（终裁
§7-OQ-4 deferred 至本刀，仍不预设——列证据）：
- `ApiProgram`——若消费者语义是"节目"整体（含 A2-7 后的输出事实）；
- `ApiProgramMaster`——若语义严格=Domain 组合根投影；
- 现有族无 `Snapshot` 后缀先例（`ApiQuerySnapshot` 是聚合响应非单资源）——
  `ApiProgramSnapshot` 与族惯例不符概率高；`ApiChannelProgram` 依赖 Channel
  概念（A4 线未建）。
**仍交用户终裁**（探针仅给命名惯例证据）。

### 项 3 — None wire 表达

现状证据：现有 wire 面处理"缺"的两种先例——
(a) `ApiSession.outputs: Vec<String>` 空 vec = "纯分析/降级"（空数组语义）；
(b) `ApiQuerySnapshot.observation_revision` additive 字段**有意不带
serde(default)**（L167-169 注释：响应模型无旧 JSON 消费方）。
`join_result: Option<MasterJoinResult>` 的候选：`null` / 字段缺席 / 独立
wire 字符串值（如 `"NOT_FORMED"`——**禁**：与 R-A 冲突，None≠第五语义）。
探针倾向：**`null`（serde Option 内建）**——零自定义、与 04 实现的 Option
absence 语义零转换。**wire 终裁留用户。**

### 项 4 — AVSync projection

若 A2-6 暴露：形态只能是 `ApiProgramMaster`（或同语义资源）内的
**classification 透传字段**（`"avsync": "ACCEPTABLE|DEGRADED|FAILED|UNKNOWN"`），
**禁**映射为任何 status/health 字段值（§8.10 red≠节目故障——PLAYER 绝不
切源）。词表四值已锁（A2-5-03），投影层零转换直传。

### 项 5 — Snapshot 并列关系

终裁 §7-OQ-3 已定方向（并列 projection 非存储合并）。落点证据：并列的
**自然位置 = API 响应层**（`ApiQuerySnapshot` 侧新增并列 program 字段或
独立响应资源），**不是** `CanonicalRuntimeState` 加字段（双禁令之一）。
`ApiQuerySnapshot` L167-169 先例还给了 wire 纪律：响应模型 additive 有意
不带 serde(default)——A2-6 若扩 ApiQuerySnapshot 沿用。

### 项 6 — 是否需要专门 ProgramQuery

`RuntimeQuery` = Runtime Query Model（00 报告 Q4 已锚），7 查询 + new =
8 项 surface 全部面向 CanonicalRuntimeState 子项。探针事实：**零 Program
查询调用者 + ProgramMaster 零产生源**——现在建 `ProgramQuery`（无论独立
facade 还是并入 RuntimeQuery）都没有可查询的东西。**倾向：A2-6-02 起
projection 纯函数先行（`to_api_*` 形态），Query 接线等消费证据；"独立
ProgramQuery vs 统一 Query Facade"按 00 报告原 OQ 分析留至有真实查询需求
时裁。** A2-6-02 前零扩展（冻结维持）。

### 项 7 — 到 API Boundary 的唯一转换点

`api_boundary.rs` 模式：`to_api_*(&Internal) -> Api*` 纯函数族 = 唯一转换
点（无第二入口）。A2-6 对应物 = `to_api_program_master(&ProgramMaster) ->
Api*`（命名待终裁）——**唯一转换点约束**：转换函数放 api_boundary.rs（与
既有族同址）、零内部类型泄漏（禁 `type ApiX = ProgramMaster`）、输入是
**已存在的** ProgramMaster 引用（硬 Gate：转换器不制造、不缓存、不组装
ProgramMaster——无 owner 时不产假快照）。

## 2. 硬 Gate 执行声明（终裁 §7）

本 probe 零 .rs diff；未创建任何"假 ProgramMaster"；未建 Custody/Query/
projection 函数。01 交付物 = 本报告 + tasks 更新。

## 3. Open Questions（交用户终裁，A2-6-02 前置）

| # | 问题 | 候选 | 倾向（非裁决） |
|---|---|---|---|
| OQ-6 | API 资源命名（原 OQ-4） | ApiProgram / ApiProgramMaster / 其他 | 证据 §1-项 2：族惯例不待 Snapshot 后缀；节目语义 vs 运行状态取决于消费者画像——A2-7 前仅有声明语义，倾向 `ApiProgramMaster` 直译组合根，但**接受用户另裁** |
| OQ-7 | None wire（原 OQ-5） | null / 缺席 / 独立值 | `null`（serde 内建零转换；独立值违 R-A） |
| OQ-8 | 投影挂载点 | 扩 ApiQuerySnapshot 并列 / 独立响应资源 / 暂不挂载只留 to_api_* 函数 | **倾向 C（只留纯函数）**——零消费者证据下接线即臆测；A2-6-02 交付 to_api_* + 测试，挂载 deferred |
| OQ-9 | AVSync/None/inconsistency 在投影中的暴露面 | 全透 / 只 Result / 逐字段裁 | 倾向：Result+AVSync 透传（已锁词表零转换）、inconsistency 默认不暴露（05 终裁维持） |

## 4. No-Build Gate 复认

零 .rs diff；未建假 ProgramMaster/Custody/ProgramQuery/投影函数；
RuntimeQuery/transport/CanonicalRuntimeState 零改动。

---

## 5. 用户终裁记录（A2-6-01 → A2-6-02 Gate，2026-09-03）

> **A2-6-01 = APPROVED / CLOSED；A2-6-02 = APPROVED TO IMPLEMENT**
> （只做 API Projection implementation + tests，不接 Query/Transport——
> 真正的 consumer 等 A2-7 ProgramMaster 生产生命周期出现后再接）。

| 项 | 终裁 | 关键约束 |
|---|---|---|
| OQ-6 命名 | **`ApiProgramMaster`** | 禁 `ApiProgram`（过早吞未来 Program 语义——A2-7 完整消费者出现再定义）/`ApiChannelProgram`/`ApiProgramSnapshot` |
| OQ-7 None wire | **JSON `null`** | `"join_result": null` = 尚未形成；禁 UNKNOWN/NOT_READY/FAILED/DEGRADED 字符串化；**serde(default) 仍禁**（缺省语义≠放宽 API Contract） |
| OQ-8 挂载 | **只实现 Projection DTO + pure mapper，零挂载** | ProgramMaster→to_api_program_master()→ApiProgramMaster 到此为止；RuntimeQuery/ApiQuerySnapshot/新端点全不做——没有 producer 也没有 consumer，挂载=创造无生命周期来源的 API（空中楼阁） |
| OQ-9 暴露面 | `ApiProgramMaster{video, audio, metadata, join_result, avsync}` | **whole-value 整体投影禁 flatten**（顶层禁 video_stage/audio_stage/metadata_xxx 平铺——API 不重新解释 Domain）；**avsync=Join classification input projection**（禁 health/status/program_status 化，AVSync=FAILED 禁推 ProgramMaster=FAILED）；**inconsistency 不暴露**（内部分类输入非用户语义）；**MasterJoinOutput 禁直接投影**（eligibility/classification_input 是 Join 运算过程输出非 API Resource；禁 alias） |

### A2-6-02 十二测试 Gate（PMAPI-01..12）

01 五键存在 / 02 Some 正确序列化 / 03 None→null / 04 None 禁语义化 /
05 AVSync 四值零转换 / 06 inconsistency 不入 API / 07 whole-value 禁
flatten / 08 DTO 非 alias / 09 mapper 不创建 ProgramMaster / 10 mapper
不触 RuntimeState/SessionManager/RuntimeQuery/EventLog / 11 零新端点 /
12 零 serde(default)。

---

## 6. A2-6-02 复核终裁（CHANGES REQUIRED，2026-09-03）

> 02 整体设计 APPROVED（命名/五字段/None→null/avsync 参数化/零挂载/
> whole-value 全保留）；**唯一返工点 = Domain 容器类型直接作 API DTO 字段
> → REJECT**。

### 否决理由（两层权限不可混同）

api_boundary 冻结规则是两层：①"Canonical types 属于 mapper **允许消费的
输入来源**"；②"API Resource Model **独立定义**，禁内部 Rust DTO 直接作
API DTO"。由①不能推出②——`ApiProgramMaster.video: VideoMaster` 已形成
"API DTO 直接持有 Domain Object"结构。且 VideoMaster/AudioMaster 是 A2-2/
A2-3 的 **Domain 真相对象**（含 data_plane/composition/NonZeroU16/f32 等
Domain 表达）非 API Resource。**最大风险**：Domain struct 字段一变 → API
wire 自动变——与独立 API Contract 直接冲突。

### 架构认识纠正（终裁原文记档）

**镜像 DTO ≠ 重新解释 Domain。** 真正危险的是"API Developer 重新命名/
删减/创造另一套语义"；需要的是"Domain canonical fact → explicit mechanical
mapping → API representation of the same fact"（语义来自 Domain，所有权与
演进边界属于 API）——`to_api_device/port/session` 现行代码正是如此
（ApiSession 并未把 SessionRuntimeState 当字段暴露）。**不创造"Canonical
类型可直接暴露"例外，延续仓库既有纪律。**

### 修复范围（仅 API nested DTO + mapper，其他零扩张）

```text
ProgramMaster ── to_api_program_master() ── ApiProgramMaster
  ├── VideoMaster            ──►  ├── ApiVideoMaster
  ├── AudioMaster            ──►  ├── ApiAudioMaster
  └── MetadataMaster         ──►  └── ApiMetadataMaster
```

薄镜像 DTO 字段严格 1:1 对应 canonical wire shape；mapper 只做显式复制/
转换。**不碰**：ApiProgramQuery/RuntimeQuery/ApiQuerySnapshot/Transport/
Custody/producer/AVSync measurement DTO/failure reason DTO/Channel·Program
identity（零挂载裁决仍有效）。A2-7 不提前跳转；修复通过 PMAPI-01..12 后
**直接进 A2-6-06 收口**。

### 附注

a993a36 为 feature 分支提交，按先例不触发 CI（GitHub combined status 空
= 正常；CI 在 PR 时跑）——盒上验证与 GitHub CI 不等同的口径接受。
