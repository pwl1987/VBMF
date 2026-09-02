# A2-4-04 — Master Join Boundary Review（前置全盘代码探针 + 边界契约）

> Status: `BOUNDARY CONTRACT CLOSED（终裁 2026-09-02）—— 非 "Join 设计完成"`
> ⚠️ 语义澄清（终裁 §十）：本报告交付的是 **A2-5 输入边界契约**；真实仓库
> MasterJoin/ProgramMaster/AVSync 零生产代码 = A2-5 尚未开始的真实状态。
> A2-5 = Master Join implementation/design，与本 Boundary Contract 严格分开。
> Authority: A2-4-03 终裁 §十一（九项实查清单；不能凭 V0.2 文档直接写 Join）
> Date: 2026-09-02 · Base: master 分支 comet/a2-4-metadata-master（03 终裁落盘后）
> 纪律：无真实代码证据不设计 A2-5。

---

## 1. 九项实查证据（J1-J9）

| # | 必查项 | 实查结果 | 证据 |
|---|---|---|---|
| J1 | Master Join 类型/接口真实代码 | **零**。全部命中为注释路线图（lib.rs L18/L50）与 Masters 的 `MasterJoined` **stage 终态值**（非 Join 实现） | grep `MasterJoin/master_join` 域外零命中 |
| J2 | ProgramMaster projection/runtime model | **零**（仅 mod.rs 路线图注释） | grep `ProgramMaster` 零类型命中 |
| J3 | Video/Audio failed/health/readiness 表达位置 | **全在 Runtime 平面**：`health.rs::AgentState{..Degraded..}` + `RuntimeEvent::{PipelineFault,HardwareFault,SessionFailed}` → `reduce()` 纯函数派生；**Program Domain 零健康语义**（VideoMaster/AudioMaster/MetadataMaster 均无 health/ready 字段） | health.rs L24-137 |
| J4 | Unknown 在 Health/Runtime 体系语义 | `runtime_state.rs::CapabilityFlag::Unknown`——注释明文 "**ProbeFailed 视为 Unknown（探测失败 ≠ 不支持——absence≠evidence）**"（L249/255）；`VideoContentState::Unknown`。**仓库已有 absence≠evidence 纪律先例，与 Metadata NotPresent/Unknown 完全同构** | runtime_state.rs L46/249/255/313 |
| J5 | A2-5 提前实现 placeholder | **零**（全部为"A2-5 消费时…"前瞻注释：is_program_scope_master 消费点重审/switch_policy 接入） | video_master.rs L29/L71/L132 |
| J6 | DEGRADED/FAILOVER/READY_TO_TAKE 消费边界 | Degraded = health.rs reduce 派生（PipelineFault retryable→Degraded 等，L108-111）；**FAILOVER / READY_TO_TAKE 零实现**（hot-standby §1.18 属未来） | grep 零命中 |
| J7 | AV Sync 现有类型 | **零**（V0.2 §3.8 avsync_manager yaml 是唯一规格；DB 侧 avsync_measurements 表 V0.2 §5） | grep `avsync/av_sync/AvSync` 零命中 |
| J8 | Metadata presence 能影响哪种 Join 判定 | 推导（见 §3 判定矩阵）：`Participating/NotPresent/Unknown` 只能影响 **Metadata 路声明维度**的 Join 输入；不能直接产生健康/故障结论 | §3 |
| J9 | Join 是否把"缺 metadata"等价 failed | **Join 不存在，无现状 bug**——风险为 A2-5 设计禁令（前瞻防护，§3 红线 R-5）；当前 health.rs reduce 对 metadata 类事件零处理（events.rs 零 metadata kind，grep=0） | events.rs 计数 0 |

---

## 2. 探针核心结论：三平面分离已在代码层成立

```
Program 平面（声明）           Runtime 平面（观测/健康）         Join 平面（A2-5 未来）
VideoMaster{stage..}          AgentState + reduce()             [Master Join]
AudioMaster{stage..}          RuntimeEvent{PipelineFault..}     联合判定点:
MetadataMaster{facts..}       CapabilityFlag::Unknown           - 三路声明 消费
（零 health/ready 字段）       （absence≠evidence 先例）          - 健康输入 取自 Runtime
                                                                - AV Sync 属性 落此
```

- "Video/Audio/Metadata 任一路 **failed**"（V0.2 §1.20 L155 → DEGRADED/FAILOVER）
  的 **failed 事实只能来自 Runtime 平面**（RuntimeEvent 派生），Program 平面
  从不表达故障——这是既有代码事实，A2-5 必须沿用而非破坏；
- `absence≠evidence` 在 CapabilityFlag 已有同构纪律 → Metadata 的
  Unknown≠NotPresent 与 Runtime 体系语义兼容，Join 设计可对齐。

## 3. Join Boundary 契约草案（A2-5 的输入边界，本 review 交付物）

### Join 判定输入矩阵（什么能判 Program Master / 什么绝不能）

| 输入 | 来源平面 | Join 能判 | Join 绝不能 |
|---|---|---|---|
| VideoMaster.stage / AudioMaster.stage | Program | 各路 processing progression 是否达终态 | 用 stage 推导健康（stage≠health） |
| MetadataMaster.join_declaration | Program | Metadata 路声明（三态） | 把 Unknown 升格故障；把空 facts 推导 NotPresent |
| MetadataMaster.facts | Program | 条级证据汇总入 Program-scope metadata | 对 5b 矛盾快照按有效 NotPresent 消费（**fail-closed**，C′） |
| RuntimeEvent 派生健康（AgentState/reduce） | Runtime | "任一路 failed" 的**唯一**事实来源 | ——（Join 不自造故障） |
| AV Sync | Join 自身属性 | offset/drift 测量与分类（§3.8） | 塞进 CanonicalTimecode 或任一 Master |

### 五态混淆防护红线（A2-5 设计 guard）

```
R-1 Unknown        = 观测不足          → 不产生任何 DEGRADED/FAILOVER 依据
R-2 NotPresent     = 结论性负声明      → ≠ failed（"确认无"是正常态）
R-3 inconsistency  = 5b 矛盾快照       → fail-closed 拒绝消费（不猜哪边对）
R-4 failed         = 仅由 Runtime 平面派生（RuntimeEvent/reduce）
R-5 readiness      = Join/Projection 层概念（READY_TO_TAKE 属 §1.18，零实现）
```
五者**不得揉成一个状态字段**（A2-4-03 L3 + 决策 #42 三轴分离同律）。

### Join 消费 MetadataMaster 的具体规则（C′ 的 Join 侧落点）

1. `Participating + []` → 接受该路存在，facts 空**不阻断**（空≠failed）；
2. `NotPresent + []` → 正常"确认无"，同样参与联合判定（负声明是合法输入）；
3. `Unknown + [*]` → 不判 NotPresent 成立、不升格故障（R-1）；
4. `NotPresent + [Present fact]` → **拒绝消费该快照**（fail-closed），
   定性 "semantic inconsistency; producer/aggregator responsibility to
   resolve"（不写死 producer bug）；
5. Join 输出侧是否暴露 inconsistency 细节 → A2-5 设计裁决（本 review 不预裁）。

## 4. A2-5 开工前置条件（探针判定的 Gap 清单）

| Gap | 说明 | 阻塞性 |
|---|---|---|
| G-1 | 三 Master 已齐（A2-2/3/4）但 Join 零代码——A2-5 全新域 | 正常（顺序如此） |
| G-2 | AV Sync 零类型——A2-5 需先裁 AV Sync 声明面（§3.8 yaml→Rust） | A2-5 内裁 |
| G-3 | FAILOVER/READY_TO_TAKE 零实现——Join 的 failure policy 消费面不存在 | A2-5 只做联合判定，failover 动作仍归 Runtime/Supervisor（§8.9 同律） |
| G-4 | Program-scope Master 产物聚合（Video+Audio+Metadata→ProgramMaster）零模型 | A2-5 核心 deliverable |

## 5. No-Build Gate 复认

本轮零 .rs 结构 diff（03 终裁的 4 处注释/消息修正除外——随上一提交）;未设计
Join 实现代码；五态红线仅为 A2-5 设计 guard 交付物。

## 6. 证据文件清单

services/media-agent/src: lib.rs L18/L50（路线图注释）· health.rs L24-137
（AgentState/reduce/Degraded 派生链）· runtime_state.rs L46/249-255/313
（CapabilityFlag::Unknown absence≠evidence/VideoContentState）· events.rs
（RuntimeEvent kind 零 metadata）· program/{video,audio,metadata}_master.rs
（零 health 字段 + A2-5 前瞻注释）；ARCHITECTURE_V0.2.md §1.20 L155/§1.18/
§3.8/§5（avsync_measurements）/决策 #42。
