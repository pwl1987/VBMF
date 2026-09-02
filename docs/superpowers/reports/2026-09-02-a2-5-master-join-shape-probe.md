# A2-5-01 — Master Join Domain Shape Probe（16 项必查）

> Status: `PROBE ONLY / NO CODE CHANGE`
> Authority: A2-5-00 终裁（五问 CLOSED + R-A..R-J）；16 项清单 = 终裁 §九
> Date: 2026-09-02 · Change: a2-5-master-join · Base: `1c8d745`（工作分支）
> 判定标准：真契约冲突才停；设计缺口不停。

---

## 1. 16 项必查证据（D1-D16）

### 输入侧（Join 的既有材料）

- **D1 VideoMaster 终态 API**：`VideoMasterStage`（5 值）+ `as_wire()` +
  `new()/advance_to()/advance()` + **`is_program_scope_master()`**（终态判定）；
  字段 `{stage, data_plane, composition}`。
- **D2 AudioMaster 终态 API**：同构（含 MixLayout/delay/loudness）+
  `is_program_scope_master()`。
- **D3 MetadataMaster 终态 API**：`MetadataType/DataPlane/Presence/
  JoinDeclaration/Fact/Master` + 双快照 const；**无 stage 无 is_program_scope_master**——
  三 Master **无共同 trait/接口**（三形API）→ Join 输入须**组合非接口抽象**。
- **D4 隐性 Join 类型**：零（全部命中 = stage 终态值/注释）。

### 词表与占用（三大新发现）

- **D5 AVSync 全仓分布**：零 AVSync 类型；零实现；**⚠️ `offset`/`drift` 词已被
  `clock.rs::ClockObservationState`（#147 Clock 观测域）占用**——AVSync 声明面
  词表须与 Clock 域**显式消歧**（同名不同域，A2-5-02 词表设计必查）；
  timecode.rs 决策禁词表已含 `sync/correct/drift`（Observation 域纪律）。
- **D12 四词全仓语义分布**：
  - `Unknown`：六文件（CapabilityFlag/TimecodePresence/AudioPresence/…）——多域
    合法占用，语义各归其域；
  - `NotPresent`：audio/normalize（presence 族）+ metadata——presence 词族统一；
  - **`Failed`：仅 CommandStatus/ApiErrorClass 平面**（command/error_model/
    idempotency/transport）——**AgentState 无 Failed 字面值**（Degraded/
    Escalated/ManualRequired 代替）；Join 端若用 Failed 须声明属于哪个平面；
  - **`Ready`：已被 `AgentState::Ready` 占用（健康轴）**——R-D（Readiness≠
    Health）实锚：**Join 端口禁用 Ready 词**（避免与健康轴撞名）。
- **D11 wire 占名**：零（`MASTER_JOINED` 仅 stage 终态；无 master_join wire 名）。

### Runtime/安全平面（Join 的消费与禁入边界）

- **D7 failure vocabulary**：`PipelineFault{retryable}/HardwareFault/SessionFailed`
  三事件 + critical 分类——Runtime failed 事实的全部现有词汇。
- **D8 Safety/Watchdog 类型**：**`SupervisorAction{Restart, Escalate}` 已存在**
  （supervisor.rs L120）+ `ProcessState` + `RestartPolicy` +
  `fault_trigger_from_events` 纯函数——**action 概念已有家**（R-F/R-G 实锚：
  Join 不得发明第二 action 词表）。
- **D6 健康边界**：`AgentState` 八值；无 `effective_channel_status`（Channel 层
  未建，V0.2 Errata-13 属控制面未来）。
- **D16 X4/X5/H1 接口边界**：X4 Incident Timeline 零；X5 = `health.rs::reduce`
  （AgentState 派生）；H1 Safety 零代码（SupervisorAction 属 Supervisor 域非
  Safety Engine）；**transport 五端点冻结**（/health + /api/v1/{runtime,
  commands, events/projection, idempotency/boundary}）——**Join 判定声明零既有
  消费面**（A2-6 投影阶段才接线，本 change 不动 transport）。

### 结构与投影

- **D9 组合根**：零 `struct Channel/Program`——ProgramMaster 将是**第一个**
  Program 级组合根（OQ-B 终裁：组合模型，禁字段展平）。
- **D10 API 投影**：零 program/master 投影（api_boundary 无）。
- **D13 时间/revision 类型可用性**：`SnapshotObservation{revision,lineage,
  observed_at_ms}` 为 Runtime 域（R-I 禁入）——**结论：不存在可合法用于 Join
  的时间/revision 类型，Join 维持零时间字段**。
- **D14 AVSync/sync measurement 前体**：**零 domain 类型**；PTS 观测存在于
  **watchdog.rs gate 级**（`video_first_pts/audio_first_pts/pts_state`，
  appsink 回调写入的 acceptance 观测）——AV 对齐前体数据在 gate 观测域，
  非 Program Domain；A2-7 执行面素材，A2-5 声明面不消费 gate 内部结构。
- **D15 avsync_measurements**：零实现（仅 V0.2 §5 yaml）——终裁 C"DB schema
  ≠ Domain SoT" 无违例风险。

---

## 2. 结论：零真契约冲突，准入 A2-5-02

16 项全部为设计输入，无一处 V0.2 契约与现有代码冲突（终裁"设计缺口不停"
标准下无需停裁）。三大探针新发现进入 A2-5-02 必处理清单：

1. **Clock 词占用**：AVSync 词表与 `ClockObservationState`（offset/drift）
   显式消歧；
2. **action 已有家**：Join 零 action 词汇（SupervisorAction 唯一）；
3. **Ready 词禁入 Join 端口**（AgentState::Ready 健康轴占用）。

## 3. A2-5-02 输入事实清单（非设计，交裁定用）

- 三 Master API **非对称**（两 stage+终态判定 / 一 declaration）且无公共
  trait——Join 输入 = 组合参数，非接口抽象；
- MasterJoinResult 须表达"联合判定声明"（OQ-A：非空洞 valid:bool，非
  Recovery），enum 形态/词表（如 Jointly Acceptable/Degraded/Failed +
  classification input + AVSync 分类）**待 02 裁定**；
- 三域输入 → eligibility 真值矩阵（OQ-E：禁 all==MASTER_JOINED / 禁
  Participating→Ready / 禁 ≠UNKNOWN 合并）**待 02 裁定**；
- Join 零时间字段（D13）、零 action（D8）、零 RuntimeEvent 生产（R-G/H）、
  零 transport 接线（D16）。

## 4. No-Build Gate 复认

本轮零 .rs diff；未动三 Master/Runtime/Event/Health/transport；R-A..R-J
完好。

## 5. 证据文件清单

services/media-agent/src: program/{video,audio,metadata}_master.rs（D1-D4）·
clock.rs L4/L23（D5 Clock 词占用）· timecode.rs L234（决策禁词）·
health.rs L28-37（AgentState 八值, D6/D12）· events.rs L80-106（D7）·
supervisor.rs L45/L98-141（D8 SupervisorAction）· api_boundary.rs（D10/D12）·
runtime_state.rs L122-126（D13）· watchdog.rs L54-67（D14 PTS gate 观测）·
transport.rs L195-246（D16 五端点）。

---

## 6. 用户终裁记录（A2-5-01 → A2-5-02 Gate，2026-09-02）

> 复核基准：16 项探针结果 + Program/Runtime/Clock/Supervisor/API/Health Tree
> 实际边界。01 三项待裁中**第 1/3 项收紧、第 2 项拆层后放行**。

### 总裁决表

| 项目 | 终裁 |
|---|---|
| MasterJoinResult | 允许建立，**必须是 Join 自身联合语义结果**；最小闭合方向 `{ACCEPTABLE, DEGRADED, FAILED}` 批准；`FAILED`=**Program Join semantic failure**（非 Runtime HealthState/非 SupervisorAction——doc+测试写死）；不承载 Health/Recovery/API Status |
| 三域 eligibility | 建立显式矩阵，但**拆成 Eligibility ≠ Readiness 两层**（Eligibility=能否作为 Join 有效参与者；Readiness=联合结果是否具备进入下一步 Program Service 条件；Result=联合语义结果——三者禁合并） |
| AVSync | 只建 Join-side classification/measurement declaration；**禁复用 Clock `offset/drift` 语义**、禁把 ClockObservation 当 AVSync、禁复制 avsync_measurements DB schema 成 domain struct（Database schema ≠ Domain object） |
| `READY` | **禁作 Join Result 成员**（Readiness 独立 decision） |
| action | Join 零 action；**禁新增 JoinAction/MasterJoinAction/FailoverAction/RecoveryAction**——SupervisorAction{Restart,Escalate} 唯一 |
| 时间/revision | Join 零 timestamp/observation_revision/generated_at/epoch |
| API | A2-5 不接 transport/API projection |
| `Master` trait | **禁新增**——join(具体组合参数)，非接口抽象（保三 Master 非对称） |

### 补充红线

- 禁快捷规则：`V==MASTER_JOINED && A==MASTER_JOINED && M==PARTICIPATING → READY`
  （PARTICIPATING 是参与声明非 Ready）；禁 `≠UNKNOWN` 合并（NOT_PRESENT 与
  PARTICIPATING 语义不同）；**NOT_PRESENT 不自动 Join Failed**（合法结论性负
  声明——最终矩阵由 Video/Audio eligibility × Metadata declaration × facts 共同决定）。
- **JoinResult::Failed 不直接推 ChannelHealth::Failed**：Health Tree 有
  ACTIVE/STANDBY/OFFLINE + required/optional 聚合（Primary FAILED+Backup 已接管
  → Channel 仍 HEALTHY）——Join Result 只是 Program Join semantic fact，
  Channel Health 由 Health Tree aggregation 决定。
- AVSync 概念隔离：Clock=时钟基准关系（offset/drift 其 SoT）；Timecode=时间
  标签；AVSync=Program-level AV temporal alignment；Master Join=把 AVSync 作
  联合判定因素——四者职责互不偷渡。`AVSyncRed → FAILOVER` = 架构错误
  （§8.10 先分类后动作）。
- AVSyncClassification 候选 `{ACCEPTABLE, DEGRADED, FAILED, UNKNOWN}`（**非
  最终批准**——02 须确认阈值归属/red 语义/UNKNOWN 行为/drift 归属/
  measurement-classification 分离）。

### A2-5-02 准入（零生产代码，必裁四件事）

1. MasterJoinInput/MasterJoinResult 最小闭合模型（六件套结构：Input{三
   Master+AVSync}→Eligibility{三域}→Readiness→AVSync classification→
   Failure classification input→Result 三值）；
2. **Eligibility ≠ Readiness 三域真值矩阵**（Video/Audio stage 轴 +
   Metadata declaration 轴——具体 stage 参与集逐项裁）；
3. AVSync classification 与 Clock observation 严格消歧；
4. **JoinResult → Runtime/Safety/Health 投影边界**（谁消费/不消费/可转换/
   禁转换；DEGRADED 不一定 Channel DEGRADED——预裁"不是"）。
