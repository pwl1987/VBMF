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

---

## 11. 第六轮终裁：假实现禁令 + 无第二 registry + 02-A..F 顺序修正（2026-09-03 落盘）

> 证据基线: 远端 `0e0e3e1` 实码（02-B 已落地确认）。

### 11.1 实码确认（六项）

1. C1 修改点确定在 **controller.rs**（纯分析分支 L266-271 组装,
   pipeline.rs 无关）——强制 output 获 tee 否决维持;
2. Controller=GstPipeline 唯一 owner（GstInstance 直持 Pipeline;
   recover=remove→stop→rebuild→Playing）——C2 非理论问题;
3. **02-B SPI 边界正确**（attach/detach/tap_attachments; channel 不透明;
   零 Program 词汇; 簿记=recover 重放事实源）——**契约不改**;
4. Program Graph 确为独立资源（自申 handle/自建 pipeline/自持 graphs;
   Session.stop 只遍历 inputs 逆序停）;
5. **G1 必须先于 MediaTap→Program 接通解决**（否则双 owner 生命周期）;
6. 顺序修正: 02-A[Controller generic tap point + GstInstance 簿记]→
   02-C[MediaTapPort→Controller-owned pipeline 物化]→02-D[recover
   attachment replay]→02-E[Program 入 Session 生命周期]→02-F[真机桥接]
   ——**不先接 intervideosink/src**。

### 11.2 假实现禁令（关键新裁决）

`MediaTapRequest` 支持 Video/Audio/Both + 同管线多 channel ⇒ **禁**在
build_pipeline 永久预塞 `tee→intervideosink channel=<固定值>` 再把
attach 降级为"登记"——API 看似动态、物化只有一个预设 tap = **假实现**,
与 AlreadyAttached/NotAttached/tap_attachments 语义不一致。正确形态:
构造期只建**通用 tap 点**（tee）; 具体 tap branch 的**生命周期由
MediaTapPort 控制**; `GstInstance.media_taps: Vec<MediaTapAttachment>`
保存簿记。

### 11.3 无第二 registry（红线）

**禁**新建独立 `GStreamerMediaTapPort` 自持第二 Pipeline Registry——
违反 "Controller=GstPipeline owner/不建第二 identity·execution
registry"。实现路径: `impl MediaTapPort for GStreamerPipelineController`,
attachment bookkeeping 入 GstInstance（同 ownership 边界）。

### 11.4 模块影响表（裁决冻结）

| 模块 | 裁决 |
|---|---|
| contracts/media_tap.rs | 已完成·**契约不改** |
| adapters/mock.rs | 已完成·继续作契约测试基准 |
| **controller.rs** | **核心修改点·必须改** |
| pipeline.rs | **零 diff 维持** |
| switch_graph.rs | 暂不改（02-F 才接真实 A/B） |
| session.rs | 02-E 修改（**不让 Session 理解 GStreamer**——仅生命周期接线缝） |
| switch_execution.rs / contracts/switch.rs | 不改 |
| Supervisor | 不改（不进 MediaTap/Program execution） |
| Health/PTS | 02-G/H 后续·不提前冻结 Timeline Normalization |

### 11.5 状态

02-B=完成; 02-A/C=**设计裁决完成, 不得以旁路 adapter 冒充实现**——
直接进 GStreamerPipelineController/GstInstance ownership 边界;
02-D 紧随（attachment replay 入 recover）; pipeline.rs 不动;
Program Graph 不接真实输入直至 02-E 生命周期统一。A2-8 NOT CLOSED。

---

## 12. 第七轮终裁：02-A/C/D 盖章 + attach 原子性债 + 02-E 强制 Gate（2026-09-03 落盘）

> 证据基线: 远端 `af3ef70` 实码逐行核验（controller/media_tap/
> switch_execution/switch_graph/media-agent 组合根/session 调用链）。

### 12.1 盖章

- **02-A ACCEPTED**: GstInstance 直持 {pipeline,plan,bus_rx,stop_flag,
  thread,media_taps}; 纯分析形态=构造期命名 tee（tap 点）非预塞 branch;
  pipeline.rs 不污染。
- **02-C ACCEPTED WITH ONE P2 DEBT**: MediaTapPort 在 Controller
  ownership 边界内**真实图变更**（非登记式）; 无第二 registry;
  **P2 债=attach 部分失败原子性**（§12.2）。
- **02-D ACCEPTED**: recover 前读簿记→销毁→重建→新管线重放（真实
  测试验证新 Pipeline 实体元素）; 依赖 C 的 bookkeeping↔graph 一致性。

### 12.2 P2 债务（实码直接推出）: attach 部分成功污染

`attach_tap_to_instance` 逐平面物化, `media_taps.push` 在**全平面成功
后**——Both 时 video 成功 + audio 失败 ⇒ video branch 已入真实图而簿记
无此行 ⇒ **Reality≠bookkeeping**（破坏"media_taps=recover 唯一事实源";
下次 recover saved_taps 缺行, video branch 丢失）。补强（不重设计 SPI）:
`attach_transactional_failure_cleanup`——video 成功+audio 失败→video
branch 回滚[request pad release+Null/remove]→簿记零增加。

### 12.3 G1 硬结论（比文档更明确）

Session.stop() 逆序停 inputs+释放资源链——Program Graph handle 不在
Session.inputs/任何生命周期集合 ⇒ **两套生命周期实存**; H3=真实管线却
不属 Session lifecycle, 违反 "Session owns lifecycle/Backend owns real
Pipeline/Handle links"。具体风险: Session Released+Inputs stopped+
**Program Graph still alive**。

### 12.4 02-E 结构（正式冻结, MANDATORY NEXT）

- **不是** Session 理解 GStreamer / `Session{program_graph}`; 正确链:
  SessionManager→lifecycle（经**抽象 Program/Execution lifecycle port**,
  不直调 stop_program）→ExecutionGroup/Program Execution owner→
  SwitchExecutionAdapter→graph;
- 组合根"临时拥有"（group/switcher/graph/watchdog 四件散持）→02-E
  必须形成 **ProgramExecution/ExecutionGroupRuntime 生命周期对象**
  （creator=destroyer）;
- 启动序: Session Create→inputs→ExecutionGroup→graph instantiate→
  start→Running; 停止序: Session Stop→**Program Stop→Tap Detach**→
  Input Stop→Resource Release→Released;
- **四场景必证**: ①正常停止全序 ②Program 创建失败→部分清理→Input/
  lease/resource rollback ③Released 后 Program/Tap/Input 零残留
  ④Stop 失败不截断释放链（沿 session.rs 既有"stop 失败不截断"原则）。

### 12.5 模块影响表（第七轮）+ 执行序冻结

contracts/media_tap+mock/switch_execution/contracts-switch/pipeline.rs/
Supervisor/pipeline_events: 不改; controller.rs: 🟡补 attach rollback;
switch_graph: 🟡02-F 再接真实 tap; **session.rs 🔴02-E 生命周期接线必改
（不触 GStreamer）**; **media-agent.rs 🔴02-E 重构 ownership/teardown**;
watchdog.rs 🟡随 Program Runtime 调整; Timeline/PTS ❌不冻结。
执行序: **02-E[可并入 02-C rollback debt]→02-F→02-G→02-H→02-I→
Timeline decision→A2-8 CLOSE**。诚实边界维持: 205 passed≠真实 DeckLink
A/B 通过; FrameAligned≠Timeline Continuity。A2-8 NOT CLOSED。

---

## 13. 第八轮终裁：stop_hook 单槽缺陷（P0）+ 身份一致性（P1）+ 修复落盘（2026-09-03）

> 证据基线: 远端 `4825a5c` 实码; 02-C/02-D ACCEPTED 复认, 02-E 判
> IMPLEMENTED / NOT CLOSED（多 Session 生命周期错误）。

### 13.1 P0 缺陷（实码直证）

SessionManager 多 Session（HashMap<SessionId, SessionInner>）但
stop_hook=**单槽 Option**——第二 Session 注册覆盖第一（`set_stop_hook`
简单覆写）→ 停止首 Session 调到第二 Runtime 的 hook → session-id guard
使其 no-op → **首 Session 的 Program Graph/Tap 存活=G1 多 Session
复现**。Runtime 自身的"错误 id 不触发"测试恰不能救 Manager 单槽。

### 13.2 修复（本轮落地）

- session.rs: `stop_hooks: Mutex<HashMap<SessionId, Arc<dyn
  SessionStopHook>>>`——**Session-scoped 生命周期回调关联表**（Session
  生命周期回调关联, 非 Device/execution identity registry——红线不破）;
  `register_stop_hook(id, hook)`（同 Session 覆盖/他 Session 不受影响）;
  stop() 按 id 查调 + **Released 后条目移除**（E-5: 防 Runtime 引用残留）。
- program_execution.rs: create() 增 **P1 一致性校验**
  `session_id == group.session_id` fail-closed（复用 SwitchError::Backend,
  零新 identity 类型/零词表扩张——switch_execution.rs 不动）。
- media-agent.rs: `register_stop_hook(&sid, runtime)`。
- **不回滚** 00ca2dc/ProgramExecutionRuntime（本轮发现是外围关联模型
  缺陷, 非 owner 设计错误）。

### 13.3 测试（E-1..E-5 Gate 全过, mock 341）

- **E-4 多 Session 回归**: session_rt_01_stop_hooks_session_scoped_
  multi_session_regression——双 Session 各注册; A 停止恰调 A 的 hook
  [B 的 hook 零调用+B 会话 Running 完整保留]; E-5: A Released 后条目
  移除[B 条目保留], B 停止后关联表清空（零引用残留）。
- **P1**: program_exec_rt_01_session_group_identity_mismatch_rejected
  ——身份不一致 fail-closed 拒收（错误可观测）。P1 校验上线即拦截测试
  助手自身的不一致构造（dual_group 内造随机 sid）——防御有效性顺带
  实证; helper 改为显式传 sid。
- E-1/E-2/E-3 既有测试复认（create/teardown 幂等·失败清理·hook 不截断）。

### 13.4 02-F 前置（第八轮冻结）

registry 必须提供**同一 concrete GStreamerPipelineController 实例的
多 trait view**（Arc<dyn MediaBackend> + Arc<dyn MediaTapPort> 同源
对象——否则 instances 表不共享, attach 得 UnknownPipeline）;
**禁第二 registry**（GStreamerMediaTapRegistry/独立 port 持第二表=两
个 execution ownership 表）。02-E 修复后按序 02-F。

---

## 14. 第九轮终裁：E-6 close-path 边界 + 02-F 执行序 + 本轮修复落盘（2026-09-03）

> 证据基线: 远端 `ed388dc` 实码; 第八轮 P0/P1 复认 PASS。

### 14.1 E-6 CLOSE-PATH GAP（实码确认并已修）

close() 是**独立终态路径**（session.rs:852-879——接受 Released/
Terminated/ProvisioningFailed/BindingFailed/StartFailed; sessions.remove
+防御性回收, **零 stop_hooks 处理**）⇒ 异常终态→close 理论上残留
Runtime 引用。**修复**: close() 增 `stop_hooks.remove(id)`——不变量
**任何 close(id) ⇒ hook 条目不存在**（防御性兜底, 与 stop 路径的
Released 后移除双保险）; 测试 session_rt_01_close_path_clears_stop_
hook_entry[终态后残留注册→close→条目必不存在]。

### 14.2 02-F 执行序（第九轮冻结）

02-E-E6[本轮已修]→**02-F-01** AdapterRegistry 同一 concrete controller
双 trait view→**02-F-02** Runtime 真接 tap_port→**02-F-03**
switch_graph videotestsrc→intervideosrc/interaudiosrc→**02-F-04** A/B
跨 pipeline 接通→**02-F-05** 双 plane 成对切换→02-G→02-H→02-I。

### 14.3 02-F-01 已交付（本轮）

- registry.rs 增 `MediaAdapterBundle{backend, media_tap}` +
  `build_media_adapter_bundle()`——**单次构造 concrete controller →
  双 clone 各自 coerce**（两 trait object 同源同一对象; 禁二次构造）;
  mock 分支无共享 instances 表（独立实例语义等价）;
- **同源行为证明**（盒上真实 GStreamer）:
  registry_rt_01_bundle_dual_view_same_controller——经 backend view
  实例化的 handle 经 tap view attach 成功+簿记在（**若二次构造两
  controller, instances 分裂 → UnknownPipeline——反证排除**）;
- SessionManager 仍只见 MediaBackend（Session 抽象边界不破——组合根
  持双 view）; channel 语义维持: DeviceId=canonical hardware identity,
  tap channel=execution bridge address（不提升新 identity）。

### 14.4 边界维持

watchdog 不承担 Tap ownership（Tap=Runtime 创建/销毁资源的一部分）;
watchdog 恢复仍只 ctrl.recover(故障输入 handle) 非切换; pipeline.rs
零 diff; Supervisor 不动; AVSync/Timeline 继续冻结; A2-8 NOT CLOSED。
盒上: mock 342·bmd+gstreamer 207·clippy 双组合 clean·fmt clean。

---

## 15. 第十轮终裁：E-6/F-01 CLOSED + 唯一构造路径 + F-02 执行（2026-09-03）

> 证据基线: 远端 `efc1b2a` 实码; E-6 正式 CLOSED[调用链结构补认:
> hook 仅 Running 后注册→异常终态无 hook, close 防御性 remove=双保险;
> 隐含不变量"close⇒无运行 Runtime"列为未来 Runtime 生命周期测试
> 长期红线——Runtime 若提前到 Starting 阶段注册则须显式测试]; F-01
> CLOSED[同源双 view+行为证明]; **残留发现: build_media_backend 旧
> 单 view 入口仍在+bin 仍在用——F-02 必须替换而非叠加**。

### 15.1 第十轮复核结论（全确认）

2 输入限制=ExecutionGroup/SwitchGraph MVP 限制非硬件模型限制
（port.rs N×M/Discovery 无双口硬编码——4/8/N 扩展只动 ExecutionGroup
<N>/SwitchGraph<N>/watchdog<N>, Device/Port/SessionInput/lifecycle
不改）; SimulatedDeviceManager 固定 (0..2)=🟡N 泛化阶段补 fixture;
F-03 换 inter* **禁顺手修 PTS**（bridge 与 Timeline/G-H 独立成刀）;
GitHub check-runs=0=feature 分支不触发 CI（既定惯例）——盒上证据非
CI 独立验证（如实区分）。

### 15.2 F-02 已执行（本轮交付）

- **唯一构造路径封死**: `build_media_backend()` 改为
  `Self::build_media_adapter_bundle()?.backend` 委托面——全仓库唯一
  concrete controller 构造点= bundle; 旧独立 `Arc::new(controller)`
  路径删除（恢复即制造第二 instances 表）;
- **组合根切换**: bin 主装配改 `build_media_adapter_bundle()`——
  backend→SessionManager（Session 仍只见 MediaBackend）, media_tap→
  Program 装配;
- **Runtime 真接 MediaTap**: `create(sid, group, switcher, Some
  (tap_port), tap_wirings)`——wirings 由 device_id 派生
  （`tap-{device_id}`=execution bridge address 非新 identity）; attach
  随 create 真实发生, detach 随 Session 停止链;
- **真实 GStreamer 生命周期集成测试**:
  registry_rt_01_runtime_tap_lifecycle_on_same_controller——bundle
  双 view→双真实管线→Runtime create 真挂（A/B 管线簿记各 1）→
  teardown 真摘（清空）。

### 15.3 状态

E-6/F-01/F-02 🟢; F-03[intervideosrc 双源]→F-04[跨管线接通]→F-05
[成对切换]→G→H→I 待续; A2-8 NOT CLOSED。盒上: mock 342·
bmd+gstreamer 208·clippy 双组合 clean·fmt clean。

---

## 16. 第十一轮终裁：F-02 全 CLOSED + F-03/F-04 合并执行（2026-09-03）

> 证据基线: 远端 `0df5b4a` 实码; F-02 五项全 CLOSED（唯一构造路径/
> 生产 bundle 接线/Runtime 真接/teardown/recover 兼容）+ Session 边界
> + N×M 硬件模型复核全确认; 2 输入=ExecutionGroup/SwitchGraph MVP
> 限制（非架构错误）。**F-03+F-04 合并一刀直接执行**（不再细拆）。

### 16.1 F-03/F-04 交付（本轮）

- **channel 唯一约定来源**: `program_execution::tap_channel(device_id)`
  （`tap-{device_id}`=execution bridge address, 非新 identity）+
  `TapWiring::for_input`——组合根挂 tap 与 program 桥消费两侧同源,
  禁内联重写（漂移=桥断）;
- **switch_graph 双形态**: `SwitchMaterialization{Simulation[自持测试
  源——自包含验证保留]/Bridged[inter 系跨管线桥]}`——Bridged 源=
  `intervideosrc/interaudiosrc` 消费 tap channel; capsfilter 仅
  Simulation（Bridged 透传输入实际 caps——强制 320x240 会协商冲突）;
  生产 bin 切 `bridged()`;
- **真实跨管线 Program Media Path 实证**（盒上真实 GStreamer, 输入
  管线=videotestsrc 真实帧无 SDI 亦真跑）:
  `switch_graph_rt_01_real_bridge_cross_pipeline_media_path`——
  输入管线→tee→MediaTap[intervideosink/interaudiosink]→inter src→
  selector→program appsink 全链真实流通。

### 16.2 十项验证清单映射（十一轮 §13）

| # | 项 | 证据 |
|---|---|---|
| ① | A+B 真实帧 | 双输入管线 health 弧 video_frame_count>0 ✓真 |
| ② | channel 正确 | tap 簿记 channel==tap_channel(device_id) ✓真 |
| ③ | program 有帧 | program video/audio frames>0（跨管线桥流通）✓真 |
| ④ | A→B→A | observed_active 双向 flip ✓真 |
| ⑤ | 双平面成对 | video_active==audio_active 每次切换 ✓真 |
| ⑥ | A 断 B 仍供 | 停 A 输入管线（active=B）program 持续 ✓真 |
| ⑦ | B 断 A 仍供 | ⑥之对偶（swap 即得; F-05/Gate 补全量） |
| ⑧ | program 自身故障独立观察 | 观测维度分离=GroupObservation 三维已证（mock fold 层） |
| ⑨ | teardown 零残留 | program 停+tap 摘/停侧结构空 ✓真 |
| ⑩ | recover 重挂 | 运行中 B recover→簿记重放同 channel ✓真 |

**禁项遵守**: 零 PTS 行为修改（Timeline=G/H 独立裁决）·channel 未升
identity·Supervisor/PipelinePlan/SwitchPolicy/Session 零触碰。
盒上: mock 342·**bmd+gstreamer 209**·clippy 双组合 clean·fmt clean。
下一刀: F-05（双 plane 成对切换全量验证）→G→H→I。A2-8 NOT CLOSED。

---

## 17. 第十二轮终裁：⑥ 序错修正 + 证据补强刀（2026-09-03）

> 证据基线: 远端 `7de6fec` 实码交叉裁决——架构/实现/主链 PASS;
> **⑥ 测试逻辑错误确认**[for target in [b,a] 循环后 active=A, 直接停
> h1 停的是 active 源非 standby——证明语义完全不同]; ⑦ 未真证[对偶
> 不能由结构对称自动成立]; ⑩ 仅证簿记未证媒体恢复; 缺 Runtime→
> Bridged 一体化。**F-03/F-04=实现 CLOSED·证据 PARTIAL→本轮补严**。

### 17.1 F-04 Evidence Patch（仅测试, 零生产代码改动）

1. **⑥ 严格序修正**: 切 B→确认 observed=B→停 A（standby）→B 独立
   持续供桥+active 维持 B（`real_bridge_cross_pipeline_media_path`
   重写尾段）;
2. **⑦ 真对偶新测试**: `real_bridge_standby_b_failure_dual`——独立
   场景[stop 不可逆故不共用管线]: active=A→停 B（standby）→A 独立
   持续+成对维持;
3. **⑩ 升级媒体路径恢复**: recover 运行中 active B→簿记重放**+帧继续
   增长**[intervideosrc→selector→appsink 媒体真实重新穿越全桥——
   media-path recovery 非 bookkeeping-only];
4. **Runtime→Bridged 一体化**: `registry_rt_01_full_integration_
   brid_runtime`——bundle→SessionInput→TapWiring::for_input→
   Runtime::create[bridged]→真实媒体到达 program 出口→teardown 全摘。

### 17.2 架构债务登记（非阻塞, 不动）

`adapters/gstreamer → program_execution::tap_channel` 层级债务:
bridge address 命名规则长期应独立为 bridge-address primitive——
**F-05/G/H 后低风险搬迁, 现在不为漂亮切模块**。

### 17.3 证据等级维持

盒上记录≠CI 独立验证[GitHub status checks 空=feature 分支惯例];
⑧真桥级 FULL PASS 仍未做[结构性具备+fold 层已证——不伪闭合]。
盒上: mock 342·**bmd+gstreamer 211**[209+2]·clippy clean·fmt clean。
F-03/F-04 证据链闭合→**F-05 即刻可做**（多切换序列+三态不串+禁 PTS）。

---

## 18. 第十三轮终裁：F-03/F-04 正式 CLOSED + F-05 开工（2026-09-03）

> 证据基线: 远端 `7531e87` 实码交叉核查。评分表十六层 PASS +
> PTS🟡观察不修复 + ⑧🟡OPEN + N-input🟡MVP未实现 + CI🟡无status。

### 18.1 新登记（不修, 冻结到对应刀）

- **RECOVER_PARTIAL_DEGRADED 债务**: recover=Ok 但 tap replay 失败
  目前仅 warning——"pipeline 成功·bridge degraded"不可长期只靠日志;
  属 G/H runtime health/observation 工作, 现在不修;
- **N×M 边界冻结**: "设备模型 N×M"≠"Program Switch 已 N 输入"——
  F-05 禁顺手声称 4-way switch（MVP 边界非 bug）;
- **tap_channel 层级债务**: 继续冻结到 G/H 后搬迁。

### 18.2 F-05 范围（本轮执行）

多跳 A→B→A→B→A 每跳六点验证[plan.target→selector actual→
video/audio_active→observed_active→complete_switch→Desired]+核心
断言[成对/observed==target/desired==target/epoch+=1/帧持续]+快速
A→B→A+四类 fail-closed 真适配器级[invalid target/duplicate target/
wrong epoch/PACKET·MASTER]+**⑧真桥级区分性小验收**[program 停后
observed 归零而输入健康仍在推进——Input healthy 与 Program failed
不混淆]。禁项: PTS/Session/Supervisor/PipelinePlan/N-input 零触碰。
F-05 后 G/H 合并为观测与时间线证据大刀[三列 Input/Bridge/Program
PTS]→Timeline Normalization 裁决。

---

## 19. 第十四轮终裁：G/H 四验证面 PASS* + BridgeObservation 一等事实（2026-09-03 落盘）

> 证据基线: 远端 `3378651` 实码交叉核查（不接受自报 213 passed 为证）。
> F-05 正式 CLOSED[TargetAlreadyActive 修复=真纵深缺口非测试噱头]。

### 19.1 G/H 四验证面裁决（probe §18.2 范围兑现）

- **① Bridge primitive 一等事实: PASS**——`BridgeObservation` ≠
  `MediaTapAttachment` 分层成立[静态 attachment/recover replay fact vs
  动态 runtime observation fact]·来源=tap 分支 sink pad
  `PadProbeType::BUFFER` 真实 buffer probe（tee→tap branch→probe→
  intervideosink——非复制 Input/Program 统计·非构造期假 frames+=1）;
- **② 三列 PTS 独立测量: PASS**——TimelineSample 六 PTS 列
  [Input/Bridge/Program × video/audio]各带 PtsMonotonicity·三源独立
  join[Input=PipelineHealth·Bridge=BridgeObservation·Program=
  ProgramObservation]——非"三列两份数据"复制（mock 测试六列互异反证）;
- **③ recover 降级结构化: PASS**——BridgeHealthReport
  {pipeline_recovered·expected_channels·observed_alive_channels·
  bridge_degraded}观测查询组装·recover=Ok+expected tap 在+桥无流量
  =degraded——**MediaBackend::recover() 返回类型零改动**（§18.1
  RECOVER_PARTIAL_DEGRADED 债务在观测面兑现）;
- **④ failure-domain 区分: PASS**——classify_failure_domain{None/
  Input/Bridge/Program}单故障假设·优先序 Input>Bridge>Program·
  多故障如实报首因——**禁扩张为 multi-fault root-cause engine**。

### 19.2 结构终裁（无新冲突）

ownership 四面清白: ProgramExecutionRuntime creator=destroyer 不变·
Session 仅经 SessionStopHook 触发 teardown·bundle 三 trait view 同一
`Arc<GStreamerPipelineController>` 单次构造（instances/bridge_stats/
media_taps 不分裂）·V0.2 无偷渡（Bridge=Execution-layer observation
非 13th Engine·不碰 Master Join/ProgramMaster）。N×M 边界+tap_channel
搬迁债务维持冻结。

盒上: mock 345[342+3]·bmd+gstreamer 213[212+1: gh_three_column_
observation_evidence——三列同采六 PTS 全在场+桥 probe 帧递增+recover
后双 channel 实测流通不降级+三域分类]·clippy 双组合 clean·fmt clean。

---

## 20. 第十五轮终裁：G/H-1 两项微修 → G/H 星号解除（2026-09-03 落盘）

> 证据基线: 远端 `3378651` 实码。G/H 方向 PASS 不返工·不为 F-01..05
> 重开 review·PTS normalization 继续冻结。

### 20.1 必修① tap_channel 唯一来源收尾

registry.rs 真实 runtime tap 生命周期测试残留 `format!("tap-{a}"/
"tap-{b}")` ×2 → `tap_channel(a)/(b)`——**全仓库唯一约定来源彻底
成立**[残存 `tap-` 字面量仅两处: tap_channel 本体（program_execution
.rs:34）+ controller `tap_element_name` 元素名（detach 定位锚·非桥
地址约定）]。

### 20.2 必修② Bridge liveness「当前推进」语义（§5-8）

- 缺陷: 帧基 alive=`ever_observed_alive` 非 `currently_alive`
  [frames=10_000 断流后仍 alive=I 真机误判源·Input healthy+Bridge
  falsely healthy+Program stalled 三态错判];
- **BridgeObservation 本体不加 wall-clock**（PTS=媒体时序·wall clock=
  观察时序**严格分离**·禁塞 sampled_at 进 last_pts·禁 PTS 差值当
  liveness——保护 Timeline Normalization 证据纯净）→ 窗口判定在
  port 层: **BridgeChannelLiveness{frames=历史证据·last_observed_
  at_ms=活性证据·alive_in_window}+bridge_liveness(handle, window_ms)**;
- 落地: controller BridgeStat.last_observed_ms+bridge_clock_origin
  [probe 闭包记录观察时刻]·assemble_bridge_health 第三参改
  `&[BridgeChannelLiveness]` 以 alive_in_window 判活·mock bridge_stall
  钩子+测试锁死「b frames=10_000 但窗口外→degraded」（帧基漏报根因
  场景）+从未观测→降级+recover 失败不虚报;
- §11 program_alive 弱语义: **不改 ProgramObservation**——evidence 层
  program_progress_since/input_progress_since 采样增量分离[曾经活过≠
  当前推进];
- **MediaTapPort 契约零改动·recover 返回类型零改动·PTS 行为零触碰**。

### 20.3 状态

G/H 星号（current liveness window 未建立）**解除**。盒上: mock 345·
bmd+gstreamer 213·clippy 双组合 -D warnings clean·fmt clean（提交
`19326e8`）。**下一步 = 02-I 真机双 DeckLink 五层 Gate**——链
discovery→Device/Port→Pipeline→MediaTap→Bridge→Program→A/B switch→
failure isolation→recover 代码全就绪零阻塞; 唯一前置=用户侧双 SDI
信号源+采集卡占用窗口。

---

## 21. 第十六轮终裁：02-I 代码级前置三项 + IdentityStrength/日期修正（2026-09-03 落盘）

> 证据基线: 远端 `8e60497` 实码。总裁决: E/F/G/H/G-H1 保持 CLOSED 不返工;
> **02-I 修正为「OPEN 且存在代码级前置项」**——"硬件接上跑一次"≠I 完整验收;
> 不建议停下来等硬件: 先一次性完成代码前置再进真机。

### 21.1 裁决账（§一..§十七 复核定性）

- **§二 五层链未闭合到同一生产调用链——实码精确定性**: 用户引用的
  `_registry=None` 行实为 `#[cfg(not(gstreamer-backend))]` hardware-test
  分支（media-agent.rs:260）; 真实缺口=**SessionManager 仅在 diagnostic
  auto-start 分支构造（media-agent.rs:378）, Production 模式（else 分支
  仅日志）PortRegistry 零消费者**——registry 在组合根构建但生产无下游。
  结论成立, 引用锚点修正;
- **§三 Capability 证据缺口**: `PortRegistry::build()` 硬置
  `(Unknown, Unknown)`（port.rs:434）——而 `discover_ports()` 已从 SDK
  位掩码算出 DeviceCapabilities 却未被 build() 消费。I Gate 验收表
  **Capability 为独立结果**, resolver 成功≠Capability PASS;
- **§四 多同类物理端口建模不完整**: 位掩码→ordinal 恒 `Known(1)`, Duo 类
  单卡双 SDI 不可自然表达两条 BindingEntry——**I Gate 硬件形态边界显式
  声明: 两块独立单输入卡=支持; 一块多输入卡=不支持; 禁把双设备验证偷换成
  N×M Port 验证**（N×M 冻结维持）;
- **§五 IdentityStrength serial 语义瑕疵**: serial-only 设备误归
  DeviceHandle 档（device_manager.rs:65 合并判定）;
- §六 Session 生命周期/§七 ProgramExecutionRuntime ownership/§八
  MediaTap recover——结构正确保持不动;
- **§九 SwitchGraph 双平面部分执行风险**: video 成·audio 败=真实输出面
  半切而 bookkeeping 未动（observe 诚实但不可恢复）——真机 on-air 前必修;
- **§十 L5 Supervision（G/H FailureDomain）≠ A2-8-03 监督闭环**
  （fact→event→custody→supervisor→action→recover）——前者不能替代后者;
- **§十一 program_alive 红线**: 任何"当前 Program alive"决策只能用 sample
  delta/progress evidence（progress_since）, 禁直接拿 program_alive 当
  实时健康信号; ProgramObservation 不重设计;
- **§十六 文档时间漂移**: 报告 §14-§20 七处 2026-09-04 → 2026-09-03
  （机器时钟 +0800 过午夜伪影——提交与文档日期悖论修正）。

### 21.2 实现（本轮四刀, 全部落地）

1. **P0-1 生产组合根接入 PortRegistry**（media-agent.rs）: SessionManager
   构造自 diagnostic auto-start 分支**上移为两种模式共同组合根**
   （registry→ResourceRegistry→bundle→SessionManager 单次构造）; Production
   分支 mgr 常驻（tick 线程持有——lease 房务零媒体启动）等待 Control
   Plane——P1-3「生产绝不自行启动」与 0.7C-8「生产 503 契约」均不变;
2. **P0-2 Capability 真实证据**（port.rs build()）: SDK 连接位掩码=能力
   证据——真实硬件（任一掩码≠0）该方向掩码含连接器→`Supported(true)`/
   不含→`Unsupported`; 仿真（双掩码=0）无证据→Unknown 保持; connector
   未声明→Unknown; **Capability≠Direction≠Signal 不变（禁方向反推）**;
   设备级 audio 能力改端口级证据聚合; 测试×3;
3. **P1-1 SwitchGraph 双平面补偿**（switch_graph.rs switch()）: audio 败
   →video 回滚至 prev（返回原错, active/epoch 不动, 双平面一致恢复）;
   回滚再败 → **degraded=true + active=None**（显式记录分离态, 后续切换
   fail-closed 拒收）——真实平面不进入无记录半切中间态; 注入测试×2
   （裸 input-selector 缺 pad: 补偿成功回滚/不可恢复降级+后续拒收）;
4. **P1-2 IdentityStrength::Serial 独立档**（device.rs + device_manager.rs）:
   serial-only 不再误归 DeviceHandle; pipeline.rs 选卡 match 经 `_` 兜底
   臂天然覆盖（serial 无 manifest handle 解析路径→生产 IdentityUnresolved/
   诊断 fallback——保守正确, 无需改 match）。

### 21.3 02-I Gate 定义（十六轮 §十四 收纳）

- **L1 Input**: discovery→canonical DeviceId→manifest→runtime binding→
  Port→Signal Locked→video/audio PTS（Capability=独立结果）;
- **L2 Execution**: A/B 真实进入 Program Graph + 双 MediaTap 真实存在;
- **L3 Output**: Program video/audio 帧真实增长;
- **L4 Timing**: Input A/B V/A + Bridge A/B V/A + Program V/A 同采 +
  monotonicity + pre/post-switch——**只测量, 不做 timestamp normalization**;
- **L5 Supervision**: A fail→B alive·B fail→A alive·Bridge fail≠Input
  fail·Program fail≠Input fail·recover 后桥真实复流——**≠A2-8-03 完成**。

前置序 P0-1→P0-2→P1-1（本轮全落地）→真机 I（双 SDI 窗口）。

盒上: mock **348**[345+3 capability]·bmd+gstreamer **218**[213+2 双平面补偿
+3 capability]·clippy 双组合 -D warnings clean·fmt clean。测试注入口教训:
input-selector `%u` 模板请求 pad 自 0 顺序编号（忽略请求名后缀）——先按
模板名顺序请求再释放多余 pad 才能构造"仅 sink_1"形态; 裸元素 NULL 态
active-pad 属性不保证可读——stand-in 断言改走真实 selector 实读。

## 22. 第十七轮终裁: Identity final closure + PortId 碰撞防线（base f28b9bf）

### 22.1 裁决账（逐条复核, 全部对实码）

- **P0-1 PASS**（组合根已共同构造）; 精确口径收纳: **dependency composition
  ≠ 生产 Session API 打通**——`api_mgr` 仍仅 diagnostic auto-start 赋值,
  query/idem 生产 503/idle 保持（0.7C-8 冻结语义, 非 bug）;
- **P0-2 主链 PASS** + **audio capability 独立性 = P1 债务**: port.rs
  `audio_input/output` 由 video 连接器能力推导（SDI 嵌入音频工程成立但
  非独立 SDK audio 证据）——02-I 不得偷报"独立真实探针", 已登记;
- **P1-1 PASS**（degraded=真实状态, 无边界破坏）; **P1-2 PASS** +
  **serial production binding 债务**: `identity_handle()` 仍只取
  device_handle（resolver.rs:509-511）, Serial 档无 manifest 交叉键——
  语义校正非完整实现, 已登记;
- **§二 PersistentId fail-open 实锤**: pipeline.rs:575 档位只看
  identity_strength 直接 PersistentIdCanonical·:653 `binding.and_then
  (persistent_id)` 可 None·src_props `unwrap_or(0)` → `persistent-id=0`
  盲开路径确实存在——**本轮已修（22.2①）**;
- **§三 三项实锤**: `connector_from_mask` Component/Composite/SVideo 三位
  全折 Analog（:674-682）·真实发现序号恒 `Known(1)`（:707/:715）·
  `PortIdentity::derive` 键 `device_id+connector+ordinal` **不含
  direction**（:255）——in/out 同 connector 必同 port_id——**本轮已立
  防线（22.2②）**; N×M = 架构模型完成非 BMD 发现实现完成（02-I 硬件
  形态边界不变: 两块独立单输入卡）;
- G/H/G-H1 维持 CLOSED 不退回; 调用链 ownership 边界（SessionManager
  只持 SessionInput{device_id,handle}）本轮未破坏。

### 22.2 本轮两刀（十七轮 §七①②）

1. **① Identity final closure**（pipeline.rs）: `PersistentId` 档证据门
   ——binding 在且 `persistent_id=Some` 才可 PersistentIdCanonical, 否则
   `IdentityUnresolved`（生产/诊断一致: 无 binding 时 device_number 同样
   无据, 降级 device-number 仍是盲 0, 故不降级）; **src_props 改 Result**
   belt——launch 串拼装层最后一道防线, 伪造/未来生产者也无法把 None 拼成
   `persistent-id=0`; controller prepare `?` 接线; 测试×3（无证据双模式
   拒/证据齐备 persistent-id=77 正控制/belt 单测）;
2. **② PortId 碰撞防线**（port.rs）: **证据面告警 + 消费面 fail-closed**
   两层分工——`warn_duplicate_discovery_port_ids`（SDK 双口是真实物理
   事实, 模型无法区分命名=已登记缺口, 拒绝整个 build 会 brick 全部真实
   流程）; registry 装配层重复 port_id → `DiscoveryMismatch` fail-closed
   （别名 port_id 永远进不了寻址 SoT）。三个裁决案例全部 registry 层
   fail-closed 测试锁死: in/out 同 connector+ordinal（manifest 声明
   双侧）·多 Analog 位（双 Analog/1 声明）·同 connector+ordinal 重复
   声明; 正控制×2（双工掩码单侧声明 OK=盒上实测形态·不同 connector
   不碰撞）。

### 22.3 实证发现: 盒上两张 DeckLink SDI 均为双工卡（in/out 同 port_id）

初版防线（discovery 层 fail-closed）在盒上真实硬件直接击穿 build:
resolver gate（VBMF_RESOLVER + hw-ident-02 manifest）panic 于
`Input/Sdi/Known(1)@DeckLink SDI (1)` vs `Output/Sdi/Known(1)` 共享
port_id `e43d8f5a-…`; 复跑（收窄后）双卡各告警一次（`e43d8f5a-…`/
`f0f53b80-…`）, build 通过, HW-PORT-01 报告正常产出。**"如果未来处理
双向接口"不是未来——两张在装卡就是 direction 碰撞拓扑**, 这是 collision
closure（direction 入键或等效）从"登记债务"升格为 **02-I L1 硬前置**
的直接证据（L1 端口寻址在双工卡在机时必须先能区分 in/out 身份）。收窄
决策（证据面告警/消费面拒绝）为执行侧最小正确解, closure 本身待专门
change 裁决。

### 22.4 债务登记（第十七轮新增/确认）

- **collision closure**（direction 入键或等效）——已从登记债务升格为
  02-I L1 硬前置候选, 待裁决;
- **audio capability 独立性**（P1）: 现为 video 连接器推导, 02-I 不得
  偷报独立探针;
- **serial production binding**（P1）: identity_handle 只认 device_handle;
- **A2-8 Dual Input Gate 正式入口**: gates.rs 现仅 probe→resolver→
  loopback→session_lifecycle, 无 A2-8 五层专用 gate——02-I 执行首刀;
- Serial production binding/closure 前的既有冻结全部不变（PTS
  normalization·N-input·recover() 返回类型·Supervisor-as-executor·
  MASTER/PACKET/auto-failover）。

盒上: mock **356**[348+3 persistent-id+5 碰撞防线]·bmd+gstreamer **226**
[218+8 同]·clippy 双组合 -D warnings clean·fmt clean·resolver gate
真机复跑双工卡 warn×2 落盘。

## 23. 第十八轮终裁: b039e0c 复核 + A2-8 Dual Input Gate 正式入口

### 23.1 裁决账（逐条对实码）

- **PersistentId fail-closed = CLOSED**（materialize 证据门 + src_props
  belt 两层确认）; **Production Composition = CLOSED**（dependency
  composition ≠ production session API 口径保持）;
- **新遗漏实锤: `SessionManager::derive_claims()`**（session.rs:372-375）
  不消费 port_id——`find(device_id && capability.ends_with("-input"))`
  取**首个** input resource; 多端口下可能预留错误 Input。裁定=P1 /
  N×M closure debt, **非当前 02-I blocker**（双卡单输入 first==intended）;
- **collision closure 纠偏（本轮最重要）**: 不批准"closure 是 02-I 硬阻塞"
  ——§22.3 的"升格 L1 硬前置"过度扩大阻塞范围; 双工卡 Manifest 只声明
  Input → registry 投影无别名（Case A 可继续）, Manifest 双侧声明 →
  registry fail-closed（Case B 正确拒绝）。closure 批准为 **Port Identity
  架构债务**（N×M/双工/多端口正式扩展前必须闭合）;
- **PortIdentity v2 = 身份迁移 change**（direction 直塞 UUID 键会永久
  改变全部现有 PortId——影响 Manifest/Intent.port_id/PipelinePlan/
  Resource ID/持久化引用; 禁当 A2-8 小修）;
- **ResourceRegistry 补偿结构实锤**（port_id 命名 + input/output 分叉;
  Resource 状态机无需返工）; **Session 多输入 PASS**（SessionInput
  {device_id,handle} 每输入一行, 合法承载）; **gates 列表实锤**（五 env
  无 A2-8——"代码前置全清"≠"acceptance automation 已存在"）;
- **登记独立后续 change: `PORT-IDENTITY-AND-RESOURCE-ADDRESSING`**——
  direction + physical connector identity + ordinal + PortId 稳定性/
  迁移 + Manifest + PortRegistry.get() + SessionManager.derive_claims()
  + Resource addressing **一次闭合**; 禁只修 UUID 不修 derive_claims
  （否则"寻址 ID 正确、实际 Resource 错位"更隐蔽）。

### 23.2 本轮交付: VBMF_A2_8_DUAL_INPUT 正式 Gate（gates/dual_input.rs）

gates 模块族新增第六入口（bin/gates.rs 接线 + mod.rs; 生产 bin 零
dispatch 不变）。五层验收链（§十一 冻结形态）:

- **L0 形态 fail-closed**: manifest 双 Input port（含 connector/ordinal
  声明）且分属两台设备——一块多输入卡拒绝（N×M 见独立 change）;
- **L1a/b/c**: 双设备生产级 binding / Capability=SDK 位掩码证据
  （**三列分记**: audio=video 推导工程事实不报独立探针）/ 双 Signal
  Locked;
- **L2a/b**: 双输入 Session（appsink 纯分析）+ ProgramExecutionRuntime
  （Bridged switcher + 双 TapWiring + stop hook 接线）+ MediaTap 桥
  簿记可查;
- **L3**: Program video/audio 帧计数与 PTS 真实增长（非 PLAYING 态）;
- **L4**: Input A/B + Bridge A/B + Program 三列 PTS 同采 pre/post +
  A→B 切换（plan→begin→switch→observe→complete 全序）——**只测量不
  normalize**;
- **L5**: A fail→B alive（含 program 不受牵连）· recover A→桥真实复流
  （assemble_bridge_health 窗口语义）· B fail→A alive · 故障域分类
  不越域（真实观测行: 存活输入行=Program 域/停滞输入行=Input 域）·
  Supervisor=recovery decision 非 switch executor 注记; Bridge 故障
  注入验证属 A2-8-03（不伪造桥故障）;
- **Teardown**: Session stop→hook→Program Stop→Tap Detach→Input
  Stop→Release 全链 verdict。

**入口 smoke（盒上真机）**: env 命中→真实 SDK discovery→registry→形态
拒绝（hw-ident-02 无 port 声明 → ports=0 devices=0 fail-closed）——
入口真实接线实证; 02-I 执行需 **v4 manifest（双卡各一条 Input port
声明）**。

### 23.3 02-I 阻塞最终态（十八轮 §十五 收敛）

代码前置（十六轮三刀+十七轮两刀）与 **acceptance automation（本轮
Gate）全部在仓**; 唯一阻塞 = **用户侧双 SDI 信号源 + 两块采集卡可占用
窗口**（+现场 v4 manifest 双 Input port 声明）。collision closure /
derive_claims / serial binding / audio capability 独立性 =
PORT-IDENTITY-AND-RESOURCE-ADDRESSING 等独立 change, 不混入 02-I。

盒上: mock **356**·bmd+gstreamer **226**·clippy 双组合 -D warnings
clean·fmt clean·A2-8 gate 入口 smoke 落盘。
