---
comet_change: a2-5-master-join
role: technical-design
canonical_spec: openspec
status: probe-stage
---

# Design Doc — a2-5-master-join（A2-5: Master Join）

> A2-5-00 已 CLOSED（五问终裁 + R-A..R-J 硬约束，全文见
> [sot-probe 报告 §8](../reports/2026-09-02-a2-5-master-join-sot-probe.md)）。
> 当前阶段：**A2-5-01 Domain Shape Probe**（16 项必查，零生产代码）。

## 0'. 五问终裁要点 + R-A..R-J（全文见 probe §8）

- **OQ-A**：Join 出判定声明，Runtime/Safety 消费定 DEGRADED/FAILOVER；
  Join 零 Recovery 方法、也禁 `valid: bool` 空洞化。
- **OQ-B**：ProgramMaster = **组合根**（三 Master + MasterJoinResult），
  禁字段复制/展平，非第四 Stage Pipeline。
- **OQ-C**：A2-5 只做 AVSync 声明面+分类输入；**DB schema ≠ Domain SoT**。
- **OQ-D**：Join 出 classification input；classify→action 归 Runtime/Safety。
- **OQ-E**：禁 `all==MASTER_JOINED` 与 `Participating→Ready` 提升；三路
  **非对称输入**真值矩阵属 A2-5-02。
- **R-A..R-J**：语义不可坍缩 / Facts≠Declaration / Declaration≠Readiness /
  Readiness≠Health / Health≠Classification / Failure≠Action / Join≠Watchdog /
  Join≠Safety / D14 不入 Join / Timecode SoT 不动。

## 1. A2-5-02 模型提案（四件事，待用户终裁后进 03——本节零代码纯设计）

> 提案基准：01 终裁（shape-probe §6）+ 16 项探针事实 + V0.2 §1.20 L155/
> §3.7/§3.8/§5 阈值表/§8.9-8.10。所有形态为提案，未裁不编码。

### 1.1 提案一：MasterJoinInput / MasterJoinResult 最小闭合模型

```text
join(input: MasterJoinInput) -> MasterJoinOutput

MasterJoinInput（组合参数，零 trait 零抽象——01 终裁禁 Master trait）
├── video: VideoMaster          （按值组合，非引用非展平）
├── audio: AudioMaster
├── metadata: MetadataMaster
├── avsync: AVSyncClassification（非 Option——Unknown 变体即"未测"，
│                                  None 与 Unknown 双表达=双 SoT）
├── video_failed: bool           （Runtime 平面注入的 failed 事实——
└── audio_failed: bool            R-G/R-H：Join 不自取 Runtime 状态）

MasterJoinOutput（三件分离——Eligibility≠Readiness≠Result）
├── eligibility: JoinEligibility   （三域各 eligibility + 合取 readiness）
├── result: MasterJoinResult       （仅 readiness 后有意义）
└── classification_input: ...      （伴随输出，见 1.4）
```

- `MasterJoinResult` 最小闭合三值（01 终裁方向批准）：
  `Acceptable / Degraded / Failed`（wire SCREAMING_SNAKE_CASE 同域惯例）。
  **`Failed` = Program Join semantic failure**（doc + 测试写死三不等式：
  ≠ Runtime health 态、≠ SupervisorAction、≠ CommandStatus::Failed）。
- **无 `Ready` 成员**（01 终裁禁；Readiness 独立层）。
- 零时间字段（D13）/零 action（D8）/零阈值（见 1.3）/零 trait。

### 1.2 提案二：Eligibility ≠ Readiness ≠ Result 三层矩阵

**Eligibility（每域独立：能否作为 Join 的有效参与者）**：

| 域 | 判定 | 依据 |
|---|---|---|
| Video | `stage == MasterJoined`（**复用 `is_program_scope_master()`**，不重定义第二判定） | V0.2 §3.7 Join=链尾，中间态不可参与 |
| Audio | 同上（复用其 `is_program_scope_master()`） | 同 |
| Metadata | `declaration ∈ {Participating, NotPresent}` | 两态均为有效声明；`Unknown`=声明未成（**非非法非 failed**——R-A） |

**Readiness（联合层，中间 decision 不入 Result）**：
`Ready ⟺ video_eligible ∧ audio_eligible ∧ metadata_eligible`。
非 Ready 的输出=NotReady（原因由三 eligibility 分量携带，不折叠成单 enum）。

**Result 矩阵（仅 readiness=Ready 时判定）**：

| 条件（按序短路） | Result | 依据 |
|---|---|---|
| C′ 矛盾快照（`join_declaration==NotPresent` ∧ ∃fact `presence==Present`） | **Failed**（fail-closed 消费规则的 Join 侧落点；classification=inconsistency） | A2-4 C′ |
| `video_failed ∧ audio_failed`（双媒体路 failed） | **Failed** | 联合体无可接受媒体基底 |
| `video_failed ∨ audio_failed`（单媒体路 failed） | **Degraded** | §1.20 L155 逐字："任一路 failed → Program Master 进入 DEGRADED 或触发 FAILOVER"（FAILOVER=Runtime 动作非 Join 状态） |
| 否则 | **Acceptable** | |

- **AVSync 不直接改 Result**（提案）：classification 作为**伴随
  classification_input 输出**（OQ-D："Join 可产生/暴露 classification input"）——
  忠于 §8.10（red 后须 failure domain 分类才知道是否节目源问题；分类不归 Join；
  PLAYER 绝不切源）。若裁"red 须降级"，则加行 `avsync==Failed → Degraded`
  （**待裁**）。
- **禁快捷规则复核**：无 `all==MASTER_JOINED`（Metadata 无 stage）；无
  `Participating→Ready`（Participating 只入 eligibility）；`NotPresent` 单独
  **不触发任何 Result 降级**（合法负声明——矩阵中无 metadata-only 降级行）。

### 1.3 提案三：AVSyncClassification 与 Clock 严格消歧

```rust
/// AVSync 联合判定输入分级（proposal——值集待终裁）。
/// 概念隔离（01 终裁 §七/§八）：
/// - Clock offset/drift（clock.rs ClockObservationState, #147）= 时钟基准关系 SoT；
/// - AVSync（本类型）= Program-level AV temporal alignment 分级；
/// - 本类型零测量字段零阈值——分级由上游（A2-7 执行面/Runtime 观测）给出。
pub enum AVSyncClassification { Acceptable, Degraded, Failed, Unknown }
```

- **消歧三不**：不复用 ClockObservationState、不复制 avsync_measurements
  DB schema（Database schema ≠ Domain object）、不带 offset_ms/drift 字段名
  （measurement 载体属 A2-7/DB 侧）。
- **阈值归属提案**：§5 表（offset 40/100/250ms P0；drift 5ms/min P1）与
  §8.10 yellow(100)/red(250) 属**观测/告警配置（AVSync Manager 执行侧）**，
  Join 声明面**不带阈值不做分级计算**——分级是输入不是 Join 计算（防 Join
  变 threshold engine）。
- 值语义提案：Acceptable=正常；Degraded=yellow 级（100-250ms，compensate
  域）；Failed=red 级（>250ms，**需 failure domain 分类**——本身不是 action）；
  Unknown=未测量（不阻断不降级——§8.10 无"未测量"动作；R-A 观测不足≠故障）。

### 1.4 提案四：JoinResult → Runtime/Safety/Health 投影边界

| Result | 谁消费 | 谁不消费（A2-5 内） | 可转换（归属） | 禁转换 |
|---|---|---|---|---|
| Acceptable | A2-6 ProgramMaster 投影（未来） | transport/API（01 终裁不接） | — | →ChannelHealth 任何直推 |
| Degraded | Runtime/Safety：作 §8.9 **Master 域输入信号之一** | Join 自身（零 action） | Runtime 经 §8.9/§8.10 决定 FAILOVER/SAFE_DEGRADE（SupervisorAction 体系） | **→Channel DEGRADED 直推**（Health Tree 聚合独立：Primary failed+Backup 接管→Channel HEALTHY；DEGRADED 不一定 Channel DEGRADED——预裁"不是"） |
| Failed | Runtime/Safety：§8.9 Master 域（Filler/Emergency 消费面） | API projection | Runtime 可升 Recovery（Supervisor 决策） | →SupervisorAction 直映射 |

- classification_input 伴随输出内容（提案）：`{avsync: AVSyncClassification,
  inconsistency: Option<…>} `——供 Runtime/Safety 分类消费；**零 action 词**。

### 1.5 A2-5-02 待终裁清单

1. Result 三值 + 矩阵四行（含双路 failed→Failed 的 V0.2 未细化补全——需裁）；
2. AVSync 不改 Result（伴随输出）vs red 降级一行——需裁；
3. AVSyncClassification 四值集 + 阈值归属（Join 零阈值）——需裁；
4. MasterJoinOutput 三件分离形态 + failed 事实参数注入形态——需裁。

## 1'. 探针结论摘要（A2-5-00）

- **联合判定唯一权威句**：§1.20 L155——三 graph 处理层隔离 + Master Join
  一致性判定；任一路 **failed** → Program Master `DEGRADED` 或 `FAILOVER`。
- **§8.9 Master 是 7 故障域之一**（Program Master 失败 → Filler/Emergency，
  切源✅垫片✅）；由 Safety+Watchdog+Health Tree 执行不新增 Engine。
- **§8.10**：AV Sync red（>250ms）先 classify_failure_domain 后动作（消费
  §8.9；PLAYER 绝不切源；UNKNOWN→SAFE_DEGRADE）；绝对规则已删。
- **§8.11 三轴**：health 轴含 UNKNOWN 独立合法值。
- **Errata-9**：AVSync Manager=Measurement+Correction+Classification，
  不做 Recovery（§8.9 是 Recovery SoT；识别/决策分离）。
- **代码现状**：Join/ProgramMaster/AVSync/FAILOVER/READY_TO_TAKE 全零
  （A2-4-04 J1-J9 @1779429 复核未变）；failed 唯一来源 Runtime 平面。

## 2. 十危险点双锚 + OQ-A..E + PD-1..4

见 probe 报告 §3-§5。十危险点全部 V0.2+代码双证据锚定；五问
（Join 输出×§8.9 / ProgramMaster 形态 / AVSync 范围 / classify 归属 /
三路不对称就绪输入）交用户裁决。

## 3. No-Build Gate

零 .rs diff；不动三 Master/Runtime/Event/Health；不冻结词表；D14 语义
禁引用；GStreamer 执行面（A2-7+）不碰。

## 4. 裁决后路线（占位，勿执行）

01 Domain Shape Probe → 02 输入/输出模型裁定 → 03 实现 → 04 ProgramMaster
聚合 + AVSync 边界 → 05 Semantic Deep Review → 06 Verification & Delivery
Closure（矩阵/guards/archive/PR/CI/merge）。
