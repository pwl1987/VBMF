# A2-8-00 — Dual-input Switch Execution SOT Probe

> Status: `PROBE ONLY / NO CODE CHANGE`
> Authority: A2-7 CLOSED @7745968 后用户终裁（否决直接编码；批准 00 Probe；
> 12 红线 + 六问冻结）
> Date: 2026-09-03 · Change: a2-8-dual-input-switch · Base: master `7745968`
> 定位转折（终裁原文）：A2-8 不再沿 A2-7"Domain→Custody"路线堆模型，而是
> **首次进入 "Program Semantic → Execution Adapter → GStreamer Graph" 实现层**。

---

## 1. 裁决事实断言复核（§三-§七/§十三，全部实锚确认）

| 断言 | 复核 | 实锚 |
|---|---|---|
| 多输入执行已具备（Session N-input） | ✅ 属实 | `inputs: Vec<SessionInput>` + start() 全 plans 实例化（A2-7-00 已锚） |
| Session 已 N-input 但旧 API 仍 first-pipeline | ✅ 属实 | `pipeline: Option<PipelineHandle>`（首输入兼容字段，session.rs）——**未完成迁移边界非 bug** |
| Watchdog 仍单 Pipeline 视角 | ✅ 属实 | bin/media-agent.rs **L403** + gates L165：`status(&sid).and_then(\|s\| s.pipeline)` → 仅首 handle spawn——B 路无 watchdog/health 观测 |
| GStreamer 无 Switch 节点 | ✅ 属实 | 实链 `src→caps→tee→{appsink,encode}`（A2-7-01 已锚）；无 input A/B→switcher→program 拓扑 |
| switch_mode 是预留 intent 非可执行 Switch Plan | ✅ 属实 | PipelinePlan.switch_mode（L144）**单路采集计划内的 Program execution intent 预留**——无法在单 PipelinePlan 内表达 A↔B |
| 单输出承诺 | ✅ 属实 | pipeline.rs L114"L114 单会话单输出"+ L659"Alpha-1 仅首输入物化输出" |
| 双路独立输出≠切换 | ✅ 接受 | A→RTMP + B→RTMP = 双路独立输出（Alpha-1 已能）非 Program switch |
| Identity 三层正确/双语义债务/V0.3 边界 | ✅ 维持 | A2-7-03 反推结论复认 |

## 2. 六问探针（终裁 §A2-8-00 必产出）

### Q1 两个 Pipeline 能否同时真实运行？

**Mock 层已证**（A2-7-04 custody_09：双 Session 双 handle 并行 start/stop）；
**真机层**：Alpha-1 Gate A1-01..07 已实证**双 SDI 卡同会话 inputs=2**（A1
收口记录）——即同 Session 双 Pipeline 真机并行采集**已验证过**。
**残余**：双 Pipeline + **双输出段**组合未验证（当前单输出承诺下 B 路强制
纯分析）；switch 场景要求的是"双采集 + 单 Program 输出"——与 A1 验证形态
不同但基础能力在。真机确认留 01 前置 gate。

### Q2 两个 Pipeline 如何进入同一个 Program execution graph？

**三种候选形态**（GStreamer 能力实查，盒上 gst-inspect）：
- **(a) input-selector**（**在**：Long-name "Input selector"；属性
  `active-pad`/`switch-mode`/`drop-backwards`/`cache-buffers`——frame
  boundary 切换原生支持；audio 对应 output-selector+audiomixer 在）：
  单 pipeline 内 A/B 源 → input-selector → program——**要求 A/B 在同一
  GStreamer pipeline 实例内**（与当前"每设备一 PipelinePlan"模型冲突）；
- **(b) intervideosink/intervideosrc**（**在**）跨 pipeline 隧道：A/B 各自
  pipeline → inter sink → program pipeline inter src → selector——**保持
  每设备一 pipeline**（SessionInput 模型不变），切换在 program pipeline 内；
- **(c) appsink→appsrc 桥接**（Rust 层转发）：零新元素但引入用户空间拷贝
  与时钟问题——不倾向。
**倾向（待终裁）**：(b) inter 系——最大保留既有 Execution/identity 模型
（每设备一 handle 一 watchdog 可扩展），selector 在 program graph 端。

### Q3 FRAME_SWITCH 的最小可靠实现？

`input-selector`（video）+ `input-selector`（audio，或 audiomixer 若需
叠加而非选择）+ `switch-mode=interpolate`/active-pad 运行时切换——
GStreamer 原生 frame-boundary 机制。**PALETTE_SWITCH=Deferred（压缩域
输入不存在——canonical ingest 是 RAW）/ MASTER_SWITCH=Deferred（依赖
Normalize Gap）——终裁已预裁，复认**。

### Q4 Switch ownership 落在哪个 Execution Adapter？

终裁倾向 E/D 组合（Switch Execution Adapter → GStreamer Switch Graph），
Session 只管生命周期资源。**具体落点候选**（待 01 裁）：
- `MediaBackend` SPI 扩展 switch 方法？——**风险**：Backend SPI 是
  instantiate/start/stop/recover/observe 生命周期五方法，塞 switch 可能
  越界；
- **独立 Switch Execution Adapter trait**（消费两 handle + SwitchPolicy →
  操作 program graph 的 selector）——与 Backend 平行的执行面，更贴终裁 E。
倾向独立 trait（01 裁）。

### Q5 Multi-input watchdog 挂接？

现状缺口（§1 断言核 1）：B 路零观测。候选（终裁倾向第二种）：
- (a) 每输入一 watchdog 线程——线程语义膨胀；
- **(b) MultiInputWatchdog（单 watchdog 服务 Session execution group）**：
  `spawn_ingest_watchdog` 演进为接收 `Vec<(device_id, handle)>`——
  **Precondition Gate**（终裁定性：无双路观测 = 不能作为生产双输入完成态）。
倾向 (b)；实现边界（改 watchdog 签名 vs 新包装）留 01。

### Q6 Frame boundary + AV continuity + failure takeover 观测点？

- **Frame boundary**：input-selector `switch-mode` 属性 + active-pad 切换
  时刻（selector 自身按 running-time 对齐）；
- **AV continuity**：双路 appsink PTS 观测（已有 b1-b4 机制扩展到 program
  graph 出口）；盒上已证 showinfo `type:I` 等观测手段（probe 终裁账）；
- **Failure takeover**：**首版只做显式切换（终裁 §二十）**——自动 failover
  需 Runtime failure→Custody→classification→Policy→Switch Intent 链
  （生产链三缺口债务在，不可跳）。
- **AVSync 债务升级为 A2-8 硬前置**（终裁 §二十三）：AV continuity 是真实验收
  项——至少定义测量接入边界（双 PTS 对比已有素材，OQ-4 通路）。

## 3. 十二红线（终裁冻结，全程生效）

1 不改 V0.2 Architecture Contract · 2 不改 RuntimeEvent identity contract ·
3 不建 Handle↔Device 全局 registry · 4 不把 SwitchPolicy 变成执行器 ·
5 不把 Switcher 塞进 SessionManager · 6 不让 Supervisor 直接执行切换 ·
7 不为 A2-8 虚构 Metadata · 8 不把 Normalize 声明当 Execution Fact ·
9 不顺手解决 V0.3 Event Contract · 10 不顺手做 HLS+RTMP 多输出 ·
11 不把双输入独立运行冒充双输入切换 · 12 无真实 AV/Frame continuity
证据不宣布广播级切换完成。

另：禁 PipelinePlan 硬塞 A/B（source_a/source_b/active_source/switcher
字段——Semantic Intent≠Execution Plan≠Execution Fact 边界，§九三案全禁）。

## 4. Open Questions（交终裁，01 前置）

| # | 问题 | 倾向（非裁决） |
|---|---|---|
| OQ-1 | Program graph 形态：inter 系跨管线隧道 vs 单 pipeline 内双源 vs appsink 桥 | **inter 系**（保留每设备一 pipeline+identity 模型） |
| OQ-2 | Switch Execution Adapter 形态：独立 trait vs Backend SPI 扩展 | **独立 trait**（Backend 五方法是生命周期语义，塞 switch 越界） |
| OQ-3 | MultiInputWatchdog：改 spawn 签名收 Vec vs 新包装层 | 单 watchdog 服务 execution group（终裁倾向 (b)）；实现边界 01 裁 |
| OQ-4 | AVSync 测量接入边界（A2-8 硬前置）：program graph 出口双 PTS vs 输入侧双 PTS | 01 设计裁 |
| OQ-5 | A/B 在 GStreamer 层的构图归属：program pipeline 归 Session 还是独立 composition 执行单元 | 与 OQ-1 联动 |

## 5. No-Build Gate

零 .rs diff；六问答案基于现有代码/盒上元素实查/既有 Gate 记录；不实现
任何 switch 执行/Domain 新对象。

## 6. 证据清单

bin/media-agent.rs L403 / gates/session_lifecycle.rs L165（单 watchdog）·
pipeline.rs L114/L144/L659（单输出/switch_mode 预留/首输入物化）·
session.rs（inputs 句柄表/pipeline 兼容字段）· 盒上 gst-inspect：
input-selector（active-pad/switch-mode/drop-backwards/cache-buffers）/
output-selector/audiomixer/intervideosink/intervideosrc/valve 全在 ·
A1 收口记录（双 SDI inputs=2 真机）· A2-7 系列归档（identity/债务）。

---

## 7. OQ-1..5 终裁 + A2-8-01 Pre-Implementation Gate 十项冻结（2026-09-03 用户两轮终裁落盘）

> 终裁链：第一轮批准 OQ-1..5 + 01 开工 → 第二轮修正：**不批准直接编码**，
> 批准进入 **A2-8-01 Pre-Implementation Gate**（先在 change 记录冻结十项再
> 开工）。**A2-8-00 正式 CLOSED**（`c3d3e23` SOT Probe / Design-only /
> No-Build Gate 定格；00 与 01 不得混为一个 change 节点）。

### 7.1 OQ 终裁（含第二轮修正边界）

| OQ | 终裁 | 第二轮修正/边界 |
|---|---|---|
| OQ-1 | ✅ A/B 各自 Pipeline → inter → Program Pipeline → selector | **inter 系 = 候选 Execution Materialization，非架构合同**——GStreamer topology（inter 系 / 单图 selector / 其他）属实现细节；换 topology 不得触及 Program Domain |
| OQ-2 | ✅ 独立 Switch Execution Adapter/组件 | 不塞 Backend 五方法（复认 contracts/backend.rs:22 生命周期语义） |
| OQ-3 | ✅ Session/ExecutionGroup 级 MultiInputWatchdog 单实例 | **ExecutionGroup 概念正式冻结**（§7.3）；watchdog 职责严格限定四观测非 God Object；喂现有 RuntimeEvent→Custody→Health 链 |
| OQ-4 | ✅ APPROVED WITH SCOPE LIMIT | 六路 PTS 观测（A/B/Program × video/audio）+ before/after switch 无 rollback/discontinuity/divergence/starvation；禁 AvSyncEngine·禁 threshold 进 MasterJoin |
| OQ-5 | ✅ Program pipeline 归 Program Execution/Switch 层 | SessionManager lifecycle only（复认 session.rs:609 仅经 backend.instantiate） |

### 7.2 Pre-Implementation Gate 十项冻结（开工前置，编码全程生效）

1. **ExecutionGroup = Program execution boundary**（inputs[]+switch+program
   output+supervision；SessionInput{device_id,handle} 原样保留）
2. **Switch Execution ≠ Backend lifecycle SPI**
3. **SessionManager ≠ GStreamer graph builder**
4. **Supervisor ≠ switch executor**（decides recovery only）
5. **GStreamer topology = implementation detail**
6. **FRAME_SWITCH first**（PACKET/MASTER 不偷渡）
7. **Video + Audio switch semantics 必须显式——终裁采方案 A：Video/Audio
   成对切换**（不是 video-only；audiomixer 放进去 ≠ Audio 已解决）
8. **AV continuity observation is mandatory**（六路 PTS；T4 element property
   ≠ PASS——须实证 switch→B 成为 program source→output 存活）
9. **MASTER_SWITCH remains Deferred**（normalize Gap 不顺手补）
10. **automatic failover remains Deferred**

### 7.3 ExecutionGroup 职责分层（冻结）

- **Session**：Create/Reserve/Instantiate/Start/Stop/Recover/Destroy（生命周期）
- **ExecutionGroup**：哪些 Pipeline 属同一 Program execution · 当前 active
  source · switch execution · Program graph · group-level observation
- **Switch Execution**：A→B / B→A
- **Watchdog**：观察 Input A · Input B · Switch · Program Output（四面）

### 7.4 状态空间三分离（Desired ≠ Execution ≠ Observed）

Domain/Intent：ACTIVE_A / ACTIVE_B / SWITCHING；Execution：selector pad A /
pad B；Observation：actual active pad · PTS · output frames。Session RUNNING
与 Program ACTIVE=A→B 正交，**绝对不共享状态机**：禁 `Session.active_input`
与 `SessionInput.is_active`（switch state 不得污染 Session lifecycle model）。

### 7.5 A2-8-01 验收矩阵 T1-T12（替代第一轮 T1-T5）

| Gate | 必须证明 |
|---|---|
| T1 | A/B 两个真实输入同时运行 |
| T2 | A/B 汇入同一个 Program Execution（非 A→output A / B→output B） |
| T3 | A→B→A 真实执行切换（改 GStreamer execution graph active source，非 Rust 状态字段） |
| T4 | 切换发生于合法 frame boundary（element property ≠ PASS，实证 A active→switch(B)→B=program source→output 存活） |
| T5 | Video/Audio Program continuity 可观测（成对切换语义） |
| T6 | A/B/Program 三者 PTS 可追踪 |
| T7 | MultiInputWatchdog 不再只看 `first()`（ExecutionGroup 四视角） |
| T8 | RuntimeEvent/Custody 不产生跨设备污染 |
| T9 | Session lifecycle 与 switch state 分离 |
| T10 | Supervisor 不执行 switch |
| T11 | `SwitchPolicy` 未被执行逻辑污染 |
| T12 | `MASTER_SWITCH` / auto-failover 未偷渡 |

### 7.6 其他维持项与完成标准

- **Event Identity Debt 不修**：`PipelineFault.pipeline`（legacy DeviceId
  承载）双语义 = V0.3 Event Contract debt，A2-8 沿用兼容层，**新增代码不得
  扩大歧义**——否则 change 膨胀为 Switch+Event Contract+Identity+Watchdog
  四合一。
- **依赖链冻结**：SwitchPolicy(semantic declaration)→SwitchIntent→
  SwitchExecutionPlan→Switch Execution Adapter→{Input A/B Pipeline, Program
  Graph}→Program Output→Observation/PTS→RuntimeEvent→Watchdog→Health/Custody；
  SessionManager owns lifecycle only / Supervisor decides recovery only。
- **01 完成标准**：不停在"设计完成"——须至少 **真实 Execution Graph + 真实
  A/B 切换 + MultiInputWatchdog 架构落地**，之后进入 02 真机验证。
- **收口链**：01 实现→02 真机→03 failure/supervision→04 AV continuity→
  05 archive+CI+merge；**A2-8 NOT CLOSED until 05**（任一中间节点完成
  不宣布 CLOSED）。

---

## 8. 第三轮终裁：A2-8-01 APPROVED + T5 拆分裁定 + 02 重定义（2026-09-03 落盘）

> 终裁链：第一轮（OQ 批准）→ 第二轮（Pre-Implementation Gate 十项冻结）
> → 第三轮（本节; 01 实现完成后裁决）。

### 8.1 A2-8-01 = IMPLEMENTATION COMPLETE / APPROVED

ExecutionGroup 模型/Switch Execution 独立于 Backend SPI/双路并存/汇入
Program Execution/A→B→A 真实切换/active-pad 实测/成对语义类型约束/
MultiInputWatchdog 脱离 first()/PTS 观测/冻结面零 diff——**正式通过**。
流程 agent 代选（决策点未应答→direct+tdd+standard）裁定为
"Process deviation disclosed, no technical invalidation"——不返工;
**改变冻结架构边界仍必须停**，普通实现细节不停。

### 8.2 T5 拆分裁定（本轮最重要）

```text
Switch Execution        PASS
Program Output Alive    PASS
Frame Switch            PASS
PTS Observation         PASS
PTS Continuity          FAIL / NOT YET SATISFIED
```

**T5 = 观测能力 PASS / 连续时间线 NOT YET PASS**——绝不能整体写 PASS。
01 状态记录为：**FRAME_SWITCH execution PASS; Program timeline
continuity DEFERRED / FAIL-PENDING-CORRECTION**。

### 8.3 架构级硬事实（提升为 A2-8 后续设计硬事实，非普通 bug）

真实 GStreamer 实证：**source switching 与 Program Timeline continuity
是两个不同问题**——input-selector 可正确完成 A/B 切换但原生透传源
时间戳不构成 Program Timeline（A→B 与 B→A 不对称; 回切可现 <1 帧 PTS
后跳→NonMonotonic）。未来应存在 Source Timeline→Switch Execution→
**Program Timeline（monotonic PTS/discontinuity handling/source
transition/潜在 AV alignment）**→Output 的连续性层; **现在不立即做
Engine**——属 02/04 设计裁决。

### 8.4 术语修正（防实现方案提前变架构合同）

登记为 **Program Timeline Continuity / Timestamp Normalization**（四
方案未裁: A 切后 Program Timestamp Regenerator / B Program Pipeline 新
Clock-Segment Timeline / C Encoder-Output boundary normalization / D
Switch transition 新 segment-timebase）——**不冻结 "Output Timestamp
Regenerator" 表述**（01 报告原"出口再生成平面"措辞废止）。

### 8.5 inter 注入面裁决（OQ-1 再进一步）

**不批准 pipeline.rs 直接耦合**——禁 `PipelinePlan{inter_channel}` /
`build_pipeline(..., switch_channel)` 式改动（Pipeline 将感知 Program/
Switch/ExecutionGroup = Program 层污染 Pipeline 层）。正确关系：
**Program Execution 层组合 execution handles/materialization resources;
Pipeline 本身不知自己是 A、B 还是 Program 的一部分**。inter 系作为
GStreamer materialization **可继续研究**，但代码 API 不得耦合。

### 8.6 A2-8-02 重定义 = Real Dual-Input Program Execution Verification

五维验证矩阵：Input[A/B alive]·Execution[A→B→A active 推进]·Output
[Program alive]·Timing[PTS monotonic/discontinuity/AV continuity——
**Program Timeline Continuity / Timestamp Normalization 为 02 明确观察
项**]·Supervision[A fail→B 仍可观测·B fail→A 仍可观测·Supervisor
echo 不被 Custody 误计新故障（沿 A2-7 链零旁路）]。
**Program Output = 一级 Observation 对象**（Input health 与 Program
execution health 两维度分离——A/B/switch 全 healthy 而 program DEAD
必须可检出; 与 V0.2 HealthState≠EffectiveChannelStatus 一致）。
**开工唯一前置 = 02 Design Gate**：裁 materialization 注入面（按 §8.5
边界约束），先裁边界再编码。

### 8.7 最终状态表

```text
A2-8-00  CLOSED
A2-8-01  IMPLEMENTATION COMPLETE / APPROVED（T5=观测 PASS·连续性未 PASS）
A2-8-02  NEXT  = Real Dual-SDI Program Execution（02 Design Gate 先行）
A2-8-03  FAILURE / SUPERVISION
A2-8-04  PROGRAM TIMELINE / AV CONTINUITY
A2-8-05  ARCHIVE / CI / MERGE
A2-8     NOT CLOSED
```

---

## 9. 第四轮终裁：能力表拆分 + 五层验收 + MediaTap 排序 + Design Gate 开工（2026-09-03 落盘）

> 前提声明（用户原话）: 01 新增代码未经远端逐行复核（当时未 push）——
> 验证依据=盒上执行结果+已核实代码事实。**分支已于本轮推送远端**
> （`c3d3e23..f349e23`）, 后续验证恢复 GitHub 代码级核验口径。

### 9.1 A2-8-01 = Execution implementation accepted / Timeline acceptance NOT accepted

| 能力 | 裁决 |
|---|---|
| 双输入 Execution Group / A/B 独立 Pipeline / Program-level switch | 🟢 已实现 |
| A→B→A / active-pad 实际变化 / Program output 存活 | 🟢 已验证 |
| PTS observation | 🟢 已实现 |
| **PTS monotonicity / Program Timeline continuity** | 🔴 **未通过/未完成** |
| 真双 SDI | 🟡 待 02 · 故障/监督闭环 🟡 待 03 · AV continuity 🔴 待 04 |

### 9.2 Source Time / Program Time 架构分野（关键一步）

每个输入自持 PTS/DTS/timebase/segment/discontinuity（Source Time）≠
切换后统一 Program PTS/timeline/continuity（Program Time）——自动
failover 前必须解决 B 源 PTS 与 Program timeline 不一致问题。

### 9.3 02 五层验收模型（正式冻结）

**L1 Input**[SDI A/B alive]·**L2 Execution**[Pipeline A/B+ExecutionGroup]·
**L3 Switching**[A→B·B→A]·**L4 Program Output**[单输出·continuous
buffers]·**L5 Program Timeline**[PTS monotonic·segment/discontinuity
semantics·A/V relation——**明确未通过项**]。

### 9.4 MediaTap 注入面（首选确认 + 排序）

Controller 只知"媒体输出能力"（MediaTap）不知 Program——禁
`ProgramSwitchInput` 语义/`PipelinePlan{program_id,switch_channel}`。
排序: ① Controller 通用 Media Tap 🟢首选 · ② 动态 tee 注入 🟡可研究 ·
③ appsink→appsrc Rust 桥 🔴不推荐 · ④ Program 直接开双设备 🔴禁止。

### 9.5 Audio 澄清（audiomixer ≠ audio switch）

成对切换≠audiomixer（混音可产生 Video=B/Audio=A+B 非 source switch）。
02 必须明确 audio 机制=selector-style / mute-gating / mixer-based
selection; 若只 audiomixer 无 active-source semantics → 不给 Audio
Switch PASS。（Gate 复核: 01 现实现 audio 平面=input-selector+active
语义, 非 mixer——probe"audiomixer 在"仅为元素清点, 见 design-gate §⑤。）

### 9.6 Watchdog 边界维持 + 03 重点重定义

watchdog 仍是 observe→fold→RuntimeEvent, 禁 switch/restart 决策/
failover/health policy/program state machine; Supervisor 只 decide
recovery。03 验证= A fail→B+Program 仍可观测 / B fail→A 仍可观测 /
Program output failure→A/B healthy+Program 故障正确分类 / Supervisor
recovery echo 不计为第二次物理故障（不破坏 A2-7 Custody 语义）。

### 9.7 模拟源边界（必须写入 archive）

**A2-8-01 GStreamer test PASS（videotestsrc 双源）≠ A2-8 真机 PASS**
——不能证明真 DeckLink A/B 的 format/framerate/caps/clock/PTS/
segment/audio timing 一致。

### 9.8 Design Gate 批准开工（无需再问）

只读调查六产出: ①真实 PipelineController 构图分析 ②MediaTap 注入
落点 ③Program graph 生命周期（create/owns/start/stop/destroy）④A/B
handle 关联（SessionInput, 零新 registry）⑤Audio topology ⑥Program
Timeline 归一化点（**只调查不实现**）。产出=
docs/superpowers/reports/2026-09-03-a2-8-02-design-gate.md。

---

## 10. 第五轮终裁：C1/C2 成立 + G1 升级必修 + 02 重定义为 Execution Integration（2026-09-03 落盘）

> 证据基线升级: 以远端 `d132b6c` 实码为主证（GitHub 可读）; 本轮全部
> 断言经实码复核（含 Session.stop() 逆序全停 session.rs:726-763——
> 本轮新锚定, 此前未亲验）。

### 10.1 裁定表

| 项目 | 裁决 |
|---|---|
| C1 纯分析管线无 tee | **成立**（但修正实现建议, 见 10.2） |
| C2 recover 重建丢 tap | **成立 / MUST FIX**（attachment bookkeeping 形式, 非 recover 内裸调 attach） |
| MediaTap 独立能力/平行 SPI | 批准 |
| PipelinePlan 加 Program/A-B 字段 | 否决 |
| **强制 pipeline 带 HLS/RTMP output 获得 tee** | **否决**（内部 tap 需求≠业务 OutputPlan; 无意义编码 CPU/输出失败污染输入/生命周期耦合） |
| 动态 tee 手术 | 暂不批准为首选（排序 A>C>B） |
| inter 系真机桥接 | 批准调查, 不冻结 |
| **G1 Program Graph 未入 Session 生命周期** | **当前存在, 升级为 02 必修 Gate**（Session.stop 只停 SessionInput 句柄; stop_program 存在但零接线——Program pipeline orphan/lifecycle leak 实证） |
| Program Graph 自生成 PipelineHandle | 可作为执行资源句柄, 必须纳入生命周期治理 |
| 01 videotestsrc 双源 | 只证明 GStreamer switch execution ≠ 真机输入验证 |
| Timeline continuity | 仍未通过; **FrameAligned ≠ TimelineContinuous 继续冻结** |
| A2-8 | NOT CLOSED 维持 |

### 10.2 C1 修正裁定

方向排序 **A > C > B**: A=Generic MediaTap Capability（构造期天然
generic tap: src→tee→{canonical appsink, generic tap}; Controller 持有
真实 GstPipeline 所有权）> C=intervideo 桥 > B=运行期动态插 tee。
tap 只知"要该管线的 video/audio media output", 不知 Program/A/B/
Switch/active——Program Execution 消费两个 tap 组合 Program Graph。

### 10.3 C2 实现形式裁定

禁"recover 内裸调 attach_tap()"——GstInstance 现无 attachment state。
须落 **MediaTapAttachment 簿记**（video/audio/endpoint identity——
**execution resource attachment bookkeeping, 非新 Device Identity
Registry**）, recover 重建后按簿记重放 attach。

### 10.4 A2-8-02 重定义 = Real Dual-Input Program Execution Integration

五层（替代 §9.3 层序, 本轮为准）: **L1 Input**[DeckLink A/B 各自真实
video/audio RAW+PTS+health+bus]·**L2 Execution**[A/B 真实进入 Program
Graph]·**L3 Output**[Program video/audio output 真有 frames+PTS 非
仅 selector 状态]·**L4 Timing**[A/B/Program 三列 PTS, A→B/B→A 切换
前后 monotonic?/continuous?]·**L5 Supervision**[A fail→B alive·B
fail→A alive·Program output fail 不误判为 A fail·recovery echo 不成
第二物理 failure fact]。

三件事一个完整 Execution Integration: **MediaTap + Program Graph
Lifecycle + Recover Reattachment**。执行序: 02-A Controller/Session
生命周期接线 → 02-B Generic MediaTap contract → 02-C MediaTap
materialization → 02-D recover re-attach → 02-E Program Graph 入
Session 生命周期 → 02-F intervideo A/B 真机桥接 → 02-G Program
Output observation → 02-H Timing/PTS measurement → 02-I 真机双
DeckLink 验证。停止序: Program Stop→Tap Detach→Input B/A Stop→
Resource Release; 恢复序: Supervisor 决策→Controller 重建→tap 重挂→
Program 保持可观测。

### 10.5 红线维持

不改 PipelinePlan Program 语义·SwitchPolicy 不扩执行器·HLS/RTMP 不
当 MediaTap·02 不冻结 Timeline Normalization 方案。**02 可进入编码,
但按修正后范围执行。**
