# A2-7-01 — Execution Fact Shape / Ownership Probe

> Status: `PROBE + DESIGN PROPOSAL / NO CODE CHANGE`
> Authority: A2-7-00 终裁 §8（四空白 + 核心任务：查死 SOURCE_RAW→NORMALIZED
> 真实 execution completion 语义）
> Date: 2026-09-03 · Base: 分支 `a45a9d5`

---

## 1. 核心任务查死：SOURCE_RAW → NORMALIZED（高风险点结论）

### 1.1 决定性事实：`normalize` 声明被 Materialization 静默忽略

- `PipelinePlan.normalize: bool`（pipeline.rs L135）在 **GStreamer controller
  零消费**：grep 全库唯一非测试消费点 = 零；`to_pipeline_description`/
  controller 构造均不读它；
- 实际生成管线（controller.rs L268/L270）：
  - 视频：`{video_src} ! video/x-raw ! appsink`
  - 音频：`{audio_src} ! audio/x-raw ! appsink`
  - **normalize=true 与 false 生成的 pipeline 完全相同**（无 videoconvert/
    videorate/videoscale 等 Normalize 元素；唯一 `videoconvert` 在 output
    编码分支 L1052 = delivery 侧 encode 前色彩转换，非 V0.2 Normalize 语义）；
- b1/b3（first_video_pts/valid_pts）证明的是 **`src→caps→appsink` 链首帧**
  ——由于链中**无 normalize 元素**，该首帧就是 raw 源帧，与 NORMALIZED 无关。

### 1.2 终裁表再收紧（比 A2-7-00 §8 更进一步）

| Transition | A2-7-00 终裁 | **01 实查修正** |
|---|---|---|
| SOURCE_RAW→NORMALIZED | ✅ 可实现（01 查死完成语义） | **⏸️ Deferred——Normalize 执行元素不存在**；b1/b3 = RAW ingest acceptance 非 normalize completion。事实前提 = Execution Adapter 为 normalize=true **实际插入** Normalize 元素链 + 建立可观测完成点（pad probe / normalize 后首帧）——属 A2-7-02+ Execution Adapter 侧工作 |
| 其余六步 | ⏸️ Deferred | 维持（无变化） |

### 1.3 附带发现（如实上报，不在本 change 修）

`GraphRuntimeIntent.normalize` 是 V0.2 Normalize 能力的 canonical 声明，
而 Execution Adapter 未实现该声明——**声明与执行的缺口**（normalize 字段
静默忽略）。处置待裁：A2-7-02+ 实现 normalize 元素 / 登记 execution gap
债务 / 两者。**本 change 不动 materialize**（禁止清单）。

---

## 2. 四空白 Probe

### ① Execution Fact Shape（按域拆——禁万能 struct）

现有事实素材分域盘点：

| 域 | 现有素材（实锚） | Fact 候选形态（01 提案） |
|---|---|---|
| Video | `video_first_pts/video_pts_state`（appsink 回调）+ b1/b3 | `VideoExecutionFacts { ingest_first_frame: bool, pts_valid: bool }`——**ingest 级**（非 normalize 级，§1） |
| Audio | `audio_first_pts/audio_pts_state` + b2 | `AudioExecutionFacts { ingest_first_frame: bool, pts_valid: bool }` 同构 |
| Metadata | **零** | 无 Fact 可定义 → declaration 恒 Unknown（OQ-2 终裁 fail-closed） |
| Failure | `PipelineFault{pipeline:Uuid}` / `PipelineBusEvent{handle,source,...}` / `HardwareFault{device_id}` / bus Error/Eos | `FailureFacts`（来源+身份 attribution，见②） |
| AVSync | 双路独立 PTS（video/audio first/last + state） | `AVSyncObservation`（测量候选见 OQ-4 终裁：measurement source 复用双 PTS） |

**红线重申**：以上全部是**候选形态提案**——A2-7-02 前不写任何 Fact 类型；
具体字段按 Custody 消费需求逐项裁（OQ-3 attribution 规则裁决后）。

### ② Video/Audio attribution

**正面发现**：`PipelineBusEvent{handle: PipelineHandle, source: String,
timestamp, detail, severity}`（pipeline_events.rs L102-110）已**结构化携带
element 粒度身份**——`source` = 发出元素名。即 attribution 的技术通道
（element 级）已存在；**但当前分析分支仅 src/caps/appsink 三元素**，无
可归属的中间处理节点（§1）。`PipelineFault{pipeline: Uuid}` 亦带管线身份。
→ **attribution 底座已备**：`handle ↔ SessionInput.device_id` 映射 +
`source` 字段保留 element 粒度，A2-7-02 Custody 实现时直接可用。

### ③ Metadata declaration source

全库排查：`config.rs`/`fixture.rs` **零 metadata 字段**；manifest 亦无。
→ **维持 OQ-2 终裁**：无 producer → `join_declaration=UNKNOWN` →
`Join.ready=false`（fail-closed）。producer 候选（A4 Channel / 未来
metadata declaration contract）出现前不预设。**当前唯一合法 ProgramMaster
= `join_result: None`**（三 Master 永不可能全 eligible）——这与"零挂载"
（A2-6）互为印证：整条链在 Metadata producer 落地前无法产出 READY。

### ④ Program Runtime Custody lifecycle（形态建议，不实现）

- **挂载层**：独立模块（Runtime/Orchestration 侧；OQ-5 终裁）；
- **触发挂点候选**：(a) SessionManager start 成功（Running 转移）后通知；
  (b) watchdog tick 周期采样；(c) Production StartPipeline Intent。三者
  非互斥（启动通知 + 周期刷新），**A2-7-02 裁**；
- **职责**（A2-6-00 终裁原文）：receives execution facts → advances
  declarations → invokes join() → publishes snapshot；
- **与 SessionManager 协作接口**：Custody 只**读** session/runtime facts
  （单向依赖），SessionManager 不知道 Custody 存在（禁反向接线）；
- **红线继承**：Custody 零 recovery 动作（Supervisor 域）、零 transport、
  零 transport。

## 3. Open Questions（交用户，A2-7-02 前置）

| # | 问题 | 倾向（非裁决） |
|---|---|---|
| OQ-6 | `normalize` 声明-执行缺口处置：A2-7-02+ 实现 Normalize 元素 / 登记 execution gap 债务 / 两者（先登记，实现随 A2-7-02） | 倾向两者 |
| OQ-7 | SOURCE_RAW→NORMALIZED 事实前提（Normalize 元素 + 可观测完成点）的实现归属 | A2-7-02 Execution Adapter 侧（属"补 normalize 声明的执行"非新能力） |
| OQ-8 | Fact 类型形态（§2①提案五域 vs 裁决调整）与 Custody 触发挂点（启动通知+周期采样 vs 单一） | A2-7-02 主裁 |
| OQ-9 | Metadata producer 长期归属确认（A4 Channel 为唯一正源？） | 维持 UNKNOWN 至 A4 |

## 4. No-Build Gate 复认

零 .rs diff；未定义任何 ExecutionFact 类型；未写 Custody；未碰
materialize/transport/A2-8。

## 5. 证据文件清单

pipeline.rs L135/L202/L1052（normalize 声明+output 分支 videoconvert）·
adapters/gstreamer/controller.rs L229-290/L406/L434（实际元素链 src→caps→
appsink + appsink 首帧/PTS 回调）· pipeline_events.rs L102-110（BusEvent
结构化身份 handle/source）· events.rs L43（PipelineFault 管线身份）·
config.rs/fixture.rs（metadata 零字段）· session.rs L48-64（粗态白名单）·
watchdog.rs（tick 驱动）。