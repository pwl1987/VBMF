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
