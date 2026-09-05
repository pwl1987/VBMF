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

## 24. 第十九轮终裁（基线 `cb78adc`）: A2-8 Gate Hardening（H1-H4 + P1）

### 24.1 裁决账本（全实码核验通过）

| # | 终裁 | 核验 |
|---|------|------|
| 一 | cb78adc 单笔提交 = Gate 正式入仓; bootstrap::build 唯一构造源正确 | ✅ 实锚 |
| 二 | L0 形态 Gate PASS（恰 2 Input port/2 设备; 一卡双输入拒） | ✅ 维持 |
| 三 | **P0: L1 失败仍继续 L2**（L1a/b/c 仅 record 无 fail-stop → Session 照建） | ✅ 实锚 dual_input.rs:168-216→218 |
| 四 | **P0: Gate 验 Manifest Port 但 Runtime Intent port_id=None**（Session 实际用"每设备首 Input Resource"） | ✅ 实锚 dual_input.rs:265 |
| 五 | 禁为 Gate 临时改 `derive_claims()`（会重耦合 PORT-IDENTITY-AND-RESOURCE-ADDRESSING）——Gate 侧闭合证据链 | ✅ 遵守（本轮零改 session.rs） |
| 六 | P1: L1 对应关系未锁一一映射（"Device A signal=true"散点非端口行） | ✅ H4 补强 |
| 七~十二 | L2 架构/L3 输出定义（Program Graph 媒体推进证据≠HLS/RTMP）/L4（PTS observation+switch continuity ≠ "证明两输入 PTS 已同步"）/L5（隔离非自动切源）/FailureDomain/Teardown 全 PASS 维持 | ✅ 零改动 |
| 十三 | P1: Gate verdict ≠ Production health state（部分路径直写 Degraded/Capturing） | ✅ 实锚 6 处直写; session_lifecycle 惯例=只读派生断言 |
| 十四 | 依赖图无新冲突 | ✅ 维持 |
| 十六 | **下一刀 = 仅 A2-8 Gate Hardening H1-H4, 不再扩大范围**; 完成后批准直接进真机 02-I | ✅ 本轮执行 |
| 最终 | cb78adc 不回滚、Gate 架构不推倒; "唯一阻塞=双 SDI 窗口"表述被纠正为"尚有一次必要 hardening" | ✅ 采纳 |

**关键实码发现（H3 前置问题答案, 强于"无副作用承载"预期）**:
`SourceIntent.port_id` **不是无副作用字段——已被 materialize 精确消费**
（pipeline.rs:630-666: Some→registry 按 `p.identity.port_id == Some(u)` 精确
匹配出 connector; 无匹配生产 fail-closed"拒绝静默回退 auto 探测",
Diagnostic 回退 None; None 才回退设备首输入端口; 既有测试
`materialize_resolves_explicit_port_id_in_registry` /
`materialize_rejects_explicit_port_id_missing_in_registry_production` 锁定）。
→ Gate 携带 port_id 直接闭合 **Manifest→Registry→Intent→connector 定位链**。

### 24.2 实现账（H1-H4 + P1, 2 文件 +350/−78）

- **H1 fail-stop（§三链全量）**: `record` 改模块级 fn + `finish(verdicts,
  stopped_at) -> !` 统一终裁输出; L1a/b/c/**d** 任一 FAIL → `finish("L1
  fail-stop——L2-L5 不执行")`（此前零已建资源, 无清理）; L2 Session/L2a/
  Group/Runtime 四处早退统一 finish; **L2b FAIL → 完整 Teardown 后终裁不进
  L3-L5; L3 FAIL → 同链不进 L4/L5**; L4 FAIL → 跳过 L5（既有）→ final
  FAIL; L5 FAIL → final FAIL。层间失败仍走完整 Teardown（停止链本身即
  验收点 + 资源释放）。
- **H2 L1d Port↔Resource closure**: `device_input_resource_closure(resources,
  device, manifest_port)` 纯函数——该设备恰一 Input Resource
  （`capability.ends_with("-input")`）且 ID == manifest port 规范派生;
  resource.rs 抽出 `input_resource_id_for_port`（`derive_from_discovery`
  input 臂改为同源调用——单一派生来源, 零行为变化, 防消费侧复制公式成
  第二 SoT）。多输入卡/跨端口污染/零资源三路 fail-closed + 唯一对应
  正路, 4 纯函数测试锁语义。**零改 SessionManager/derive_claims**。
  证据链闭合: Manifest Port → Registry Port → 唯一 Input Resource →
  （derive_claims 首 "-input" 唯一命中）→ Session; H3 另闭合 connector
  定位: Registry Port → Intent.port_id → materialize 精确匹配。
- **H3 intent 携带 port_id**: `SourceIntent.port_id =
  Some(已验证 manifest port UUID)`（原 None）; L2a verdict detail 注记。
- **H4 每端口一行一一对应证据**: DeviceHandle(identity_handle)/DeviceId/
  PortId/connector/ordinal/dir=Input/cap.input/cap.audio(video-推导)/dn/
  signal/prod_binding 同行打印（"=== A2-8 L1 端口证据（一一对应, H4）==="
  块）; dn_sig 单次采样共用。
- **P1 收口（§十三）**: 删除全部 6 处 agent_state 直写（Degraded×4/
  Capturing×1/终裁×1）; 参数改 `_agent_state`（签名/传位不变）;
  Gate verdict = 打印 + exit code, **不写 agent_state**（session_lifecycle
  同惯例: 状态由 reducer 从真实事件流派生）。

### 24.3 盒上验证（matrix 全绿）

fmt clean（文件回拉同步）· mock **356**·bmd+gstreamer **230**（226+4
新增 `gates::dual_input::tests::*` 全绿: 唯一对应/多输入卡拒绝/跨端口
污染/零资源）·clippy 双组合 `-D warnings` clean。

### 24.4 状态与下一步

- Gate hardening 完成——02-I 回到"**硬件窗口 = 唯一阻塞**"（用户侧双
  SDI 信号源 + 两卡占用窗口）; 届时现场备 v4 manifest（双卡各一条
  Input SDI port 声明）执行 `VBMF_A2_8_DUAL_INPUT` L0→L5+Teardown。
- 冻结维持: 不碰 derive_claims/PortIdentity v2/PTS normalization/N 输入/
  Supervisor-as-executor/`MediaBackend::recover()` SPI——皆属
  PORT-IDENTITY-AND-RESOURCE-ADDRESSING 或 A2-8-03/04/05。

## 25. 第十九轮最终裁决（基线 `fe71b7c`）: APPROVED——Gate Hardening CLOSED, 进入 02-I 真机

### 25.1 终裁要点

> **APPROVED — A2-8 Gate Hardening CLOSED。`fe71b7c` 保留并冻结为 A2-8
> 当前验收候选基线。A2-8 代码前置 CLOSED ≠ 02-I 真机验收 CLOSED——
> 仅剩实际硬件 L0→L5+Teardown 证据。下一动作不是继续重构, 而是直接
> 执行 02-I 真机双 DeckLink/双 SDI 验收**（"继续找代码问题"与"开始
> 硬件验收"正式分开; A2-8 再动代码收益低且易把 Port Identity/N×M
> 问题重新污染进当前 Gate）。

本轮独立复核（零代码）: `cb78adc→fe71b7c` 单提交恰 4 文件
（dual_input.rs/resource.rs/tasks.md/probe §24）——生产核心
session/pipeline/switch_graph/program_execution/resolver/port/bootstrap/
supervisor/MediaBackend **全部不在 diff**; 控制流实锚: L1 fail-stop
（dual_input.rs:363）·L2b teardown+finish（:550-551）·L3 teardown+finish
（:579-580）·H3 port_id=Some manifest port（:419）·agent_state 直写零
命中（P1 闭合）; derive_claims/session.rs 零触碰。

### 25.2 裁决表（H1-H4+P1 全 CLOSED, 冻结面全确认未动）

H1 L1 fail-stop CLOSED（实际控制流已变, 非测试层面"看起来正确"）·
H2 Port→Resource CLOSED（`input_resource_id_for_port` 单一派生源,
derive 与 Gate validation 同源）·H3 Intent→Port CLOSED（materialize
真实消费链: Manifest→Port→Intent→Materialize→Connector 闭环）·
H4 一一对应证据 CLOSED（cap.audio=video-推导 明确标注=证据纪律）·
Gate verdict≠health state CLOSED（观察者污染消除）·
SessionManager/derive_claims/PortIdentity v2/PTS normalization/N-input/
Supervisor executor 化/recover SPI **均未修改=正确**。

L4 语义确认: PTS observation+switch continuity evidence——**非**
"证明 A/B 两路 PTS 完全同步"（无过度声明）; L5 确认: 隔离非自动切源,
Supervisor 仍非 Switch Executor。资源状态机
Available→Reserved→Allocated→Releasing→Available 未被 Gate 越权改写
（Gate 走 validate→create→start→observe→stop）。

### 25.3 债务重新定级账本（十九轮 §11——全部不阻塞 02-I）

| 级 | 债务 | 归属 |
|---|------|------|
| P1 | derive_claims() 只取首 Input Resource | PORT-IDENTITY-AND-RESOURCE-ADDRESSING |
| P1 | PortIdentity 未含 direction（Resource 层 input/output namespace 已隔离不直接碰撞; 结构性问题仍在 PortIdentity） | PORT-IDENTITY-AND-RESOURCE-ADDRESSING |
| P1 | Component/Composite/SVideo connector folding→Analog | Port Identity/connector taxonomy change |
| P1 | audio capability=video 推导非独立 SDK 探针（Gate 已标注） | 独立（02-I 禁偷报已守） |
| P1 | Serial-only binding 无法成生产 canonical key（identity_handle=device_handle） | 独立 identity change |
| **P1** | **canonical UUID namespace 未统一（BMD/filesystem/simulation 差异）——本轮新增登记** | **独立 identity closure** |
| P2 | tap_channel() 层次归属（宜为 execution/bridge addressing primitive） | 独立（G/H-1 已注） |
| P2 | Production API 保持 503/未启用（A2-8 ≠ "已打开 Production Session API"——正确状态） | A4/后续 |

### 25.4 02-I 执行序（§16 冻结: 现场零代码改动）

```bash
VBMF_A2_8_DUAL_INPUT=1 \
MEDIA_AGENT_DEVICE_BINDING=<v4-dual-input-manifest> \
<media-agent-gates binary>   # 运维纪律: gate env 须用 media-agent-gates bin
```

v4 manifest 要求: Card A `Input/SDI/port_id A` + Card B
`Input/SDI/port_id B` 双声明（旧 hw-ident-02 触发 L0 fail-closed
=**正确行为非 Gate bug**, 十八轮 smoke 已证）。状态树（§15）:
A2-0..A2-7 全 CLOSED; A2-8: 02-A..02-H CLOSED; 02-I 子项 code
precondition/Gate automation/H1/H2/H3/H4/health-state isolation 全
CLOSED——**Real hardware OPEN**（DeckLink A/B + SDI source A/B +
L0→L5+Teardown）。

## 26. 第二十轮裁决（基线 `019f89e`）: APPROVED / FROZEN / GO——02-I 真机执行纪律冻结

### 26.1 终裁与禁令

> **APPROVED / FROZEN / GO。双基线: `fe71b7c` = A2-8 实现冻结基线;
> `019f89e` = 文档/裁决账本基线。A2-8 代码前置 CLOSED, 02-I Real
> Hardware OPEN。下一动作 = 直接执行真实双 DeckLink/双 SDI
> L0→L5+Teardown, 禁止再修改 A2-8 代码。**

禁改清单（冻结）: `derive_claims()` / PortIdentity（含 direction 入键）/
PTS normalization / N-input switch / Supervisor executor 化 /
`MediaBackend::recover()` SPI / Production API——全部 OPEN 债务, 禁
"顺手优化"。**首跑 FAIL 纪律（§11）: 先保留完整证据再按 A/B/C 分类——
A=真代码缺陷（如 Resource allocation failed / Manifest Port≠
Materialized connector）→ 新 change; B=硬件/输入条件（无 SDI 信号/错源/
卡被占）→ 修环境不改码; C=已登记架构债务（如 PortIdentity direction
collision）→ 禁为过 02-I 临时改架构。禁止为"跑绿"直接改代码。**

阶段模型（§12）: Code Closed → Real Hardware → PASS=A2-8 acceptance
close / FAIL=classify（hardware→fix env · evidence→Gate correction ·
code→new change review）。

### 26.2 §9 验收矩阵 ↔ Gate 实现逐项映射（真机证据点核对表）

| 矩阵行 | Gate 实际检查（实锚） |
|---|---|
| L0: 恰 2 Input port + 2 设备 + connector/ordinal present | 过滤 `Input && port_id.is_some()` + len==2 + 去重设备==2, 否则 exit(2) 不进 L1; **port_id Some ⟺ ordinal Known**（port.rs:248-260 "Unknown 不伪造 ID"——connector/ordinal present 由构造保证） |
| L1: Manifest Port→PortRegistry→Resource→DeviceBinding→device_number→Signal Locked 全链两卡 PASS | L1a 双卡 production_grade binding·L1b SDK 位掩码 Supported(true)·L1c 双卡 signal==Some(true)·L1d 每卡恰一 Input Resource 且 ID==port 规范派生·**H4 证据行同行印全链**（handle/port_id/conn/ordinal/dir/cap/dn/signal/prod_binding）; 任一 FAIL → finish 不进 L2（:363） |
| L2: Session create→资源分配→instantiate→双输入 start→Program runtime→Tap A/B→Bridged graph（非 PLAYING） | mgr.create（Preflight→Reserve→Lease→Binding verify）+mgr.start→l2a started_inputs==2→ExecutionGroup→ProgramExecutionRuntime::create（bridged build+start+双 tap attach）→L2b 双桥观测行 frames=Some |
| L3: video/audio 帧计数>0 + PTS progression（非 GST_STATE_PLAYING） | program_progress_since(obs1,obs2)+双 pts Some+!=NonMonotonic |
| L4: A/B pre+post × Input/Bridge/Program 三列 PTS + 切换证据保留 | sample_row 四行（pre A/pre B/post A/post B）+print_row 三列落盘; plan→begin→switch→observe→complete 全序; l4=completed∧observed==B∧av_epoch==1∧!=NonMonotonic∧pts Some; FAIL→跳 L5 |
| L5: A fail→B alive→A recover 桥复流→B fail→A alive→FailureDomain 隔离; **不得写"自动故障切换"** | 5.1/5.2/5.3/5.4 四 verdict + classify_failure_domain 真实行; Supervisor=recovery decision 注记打印（无自动 switch 声明） |
| Teardown: 正式验收项非附属 | 见 §26.3 |

### 26.3 §10 Teardown 确认清单映射（诚实分账）

**直接断言**: session_stop=is_ok（触发 hook 全链）/ program_runtime_inactive
（rt.is_active()==false——runtime teardown 序=watchdog 旗→Program Stop→
Tap Detach）/ phase==Released。
**传递保证（由 SessionManager::stop 链执行, 单测锁定, Gate 不单独打印）**:
Resource→Available（Release 步）/ Lease release。真机 Evidence Package
以全量 stdout/stderr 捕获（含 stop 链 tracing 行）佐证——**零代码改动**
（冻结纪律优先; 若二十+轮裁决要求 Gate 显式断言 Resource/Lease 终态,
属新裁决新刀, 不在本轮）。

### 26.4 v4 Manifest 生成纪律（§8）

不手工美化: 现场先跑真实盒子 Discovery（两卡 DeviceHandle/DeviceId/
PortId/connector=SDI/ordinal/direction=Input/binding/device_number 实测
落盘）→ 据实填 v4 双 Input port 声明 → `VBMF_A2_8_DUAL_INPUT=1
MEDIA_AGENT_DEVICE_BINDING=<v4> media-agent-gates` 执行, 全程零代码。

## 27. 02-I 真机首跑 Evidence Package（2026-09-04 盒上执行, 零代码改动）

### 27.1 执行序（严格按二十轮 §8/§11）

真实 Discovery 落盘（VBMF_RESOLVER + sigprobe）→ 据实生成
`~/a2-8-02i-v4.manifest.json`（两卡 Input/SDI/1 声明; SDI-IN-1 gst=1
今日实测, SDI-IN-2 gst=2 为 08-27 物理核值今日未开——resolver 诚实
fail-closed）→ `VBMF_A2_8_DUAL_INPUT=1` 两跑。证据归档
`~/a2-8-02i-evidence/`（resolver-discovery / run1-stale-bin-cb78adc /
run2-frozen-fe71b7c）。

### 27.2 今日硬件事实（Discovery, 三卡）

| 卡 | device_id | handle | 今日 gst | signal |
|---|---|---|---|---|
| SDI-IN-1 | 4fa33dcb… | 46:…002e4500 | **device 1 open OK** | **false（无信号）** |
| SDI-IN-2 | 6ede00d0… | 46:…002e4400 | **无（2-7 全 StateFailed, 复跑持续）** | 无证据 |
| MINI-MON-4K | 1afe2dcc… | 83:…1a66443b | device 0（纯输出卡） | false |

碰撞告警 e43d8f5a/f0f53b80 证据面照常落盘。

### 27.3 Run1（意外对照: 陈旧 cb78adc bin）→ Run2（fe71b7c 冻结行为）

run1 意外用上 cb78adc 时代 target/debug bin（教训: **cargo test/clippy
不刷新普通可执行档——gates 真机复跑前必须 cargo build --bin
media-agent-gates**）。其行为 = 十九轮 §3 P0 的真机活体演示: L1a/L1c
FAIL 后**继续进 L2**（仅被 SessionManager preflight IdentityBinding
Fail 兜住——纵深防御实证）。重新 build fe71b7c 后 run2:

- **H4 证据行齐全**（双卡 handle/port_id/Sdi/Known(1)/Input/cap/dn/
  signal/prod_binding 同行）;
- **L1a FAIL**（bindings=1/2 production_grade——SDI-IN-2 Unresolved）;
- L1b PASS（双卡 SDK 位掩码 input=Supported(true), audio=video-推导标注）;
- **L1c FAIL**（SDI-IN-1 signal=Some(false); SDI-IN-2 无 signal 证据）;
- **L1d PASS**（双卡 唯一InputResource+ID对应=true——H2 闭环真机成立,
  manifest port_id 与规范派生一致）;
- **H1 fail-stop 精确触发**: `FAIL (2/4 verdicts; L1 fail-stop——L2-L5
  不执行（H1）)` exit 2, **零会话创建**。

run1↔run2 同硬件同条件正反对照 = H1 hardening 的最强真机验证。

### 27.4 §11 分类裁决: **B 类 Real Hardware / Runtime Environment Preconditions——零代码改动**

> **二十一轮精度修正（probe §28.2）**: 分类正式定名 **B 类 Real
> Hardware / Runtime Environment Preconditions**——当前证据仅证明
> "该环境不满足 02-I 验收前置", **不证明、也不得写成"已定位某一具体
> 硬件故障根因"**。

1. SDI-IN-1: gst 可开（dn=1）但**无 SDI 信号接入**（08-27 rt01 时代
   signal=true, 今日 false——信号源未接/已断）——可直接归入硬件/输入
   条件;
2. SDI-IN-2: **gst 输入不可开（稳态, 复跑持续）**——仅 device 0/1 可开
   （0=Mini Monitor, 1=SDI-IN-1）; 08-27 时代 device 2=SDI-IN-2 可开,
   今日 2-7 全 StateFailed。**证据边界: 只证明"当前 Runtime Environment
   无法获得 SDI-IN-2 的可用 GStreamer binding", 不证明唯一根因**——
   候选（未定, 用户侧排查）: B1 duplex 端口方向配置 / B2 Desktop Video
   状态 / B3 驱动状态 / B4 卡被其他进程占用 / B5 设备注册状态 / B6 需
   重启盒 / B7 硬件本身异常 / B8 Runtime probe/OS 设备枚举环境异常。

Gate/Preflight 行为全部正确（fail-closed 精确）; 无 A 类（代码缺陷）
无 C 类触发。**02-I 硬件前置细化为: ① 双 SDI 信号源接入两卡输入
②SDI-IN-2 gst 输入可开性恢复**。恢复后**无需修改代码; 但必须重新以
当日 Discovery 核验 manifest 的 runtime binding（device-number=Runtime
instance address 非 Device Identity, 重枚举后编号可能变化）, 若
device_number 发生变化则据实更新 v4**, 再复跑 `VBMF_A2_8_DUAL_INPUT=1`
（完整执行序①-⑧=probe §28.3）。

## 28. 第二十一轮裁决（基线 `d0ffff9`）: APPROVED / FROZEN / GO 维持——02-I=B 类前置条件未满足（根因未证明）+ 账本三处精度修正

### 28.1 终裁

> **维持 APPROVED / FROZEN / GO。02-I 当前不是"代码失败", 而是"真机
> 前置条件未满足"。`fe71b7c` 仍为 A2-8 Implementation Freeze;
> `d0ffff9` = Real Hardware Evidence / B-class FAIL 账本提交（非代码
> 修复提交）; 02-I = OPEN; 代码 = 禁止修改——本轮禁改一行 A2-8 代码。**

用户侧独立核验（GitHub 实物）与本侧复核实一致: fe71b7c→d0ffff9
ahead 3 / behind 0, 仅 tasks.md+probe 两文件, `services/media-agent/
src/**` 零变化——实现未被真机测试偷改, §1 正式 CLOSED。H1/H2/H3 经
真实代码+本次真机行为一致验证 **CLOSED**（H1=L1 fail-stop/exit 2/
零 Session——Gate 正确拒绝了不满足验收前置的硬件系统, 非"跑失败";
H2=`input_resource_id_for_port` 单一派生源+L1d 反向闭环, 真机 PASS
强证据; H3=Manifest→PortRegistry→validated PortId→SourceIntent.
port_id=Some→materialize 精确匹配, 全链非假闭环）。Session 层/
Resource 状态机未被 A2-8 污染（SessionManager 仍为唯一创建/销毁者;
L1 fail 发生在 Session.create 与 Resource.allocate 之前=零 runtime
污染的理想失败位; Available→Reserved→Allocated→Releasing→Available
链未被越权改写）。L1a FAIL/L1b PASS/L1c FAIL/L1d PASS 组合=系统正确
分离 **Capability ≠ Runtime Binding ≠ Signal ≠ Resource** 四层
（audio=video-推导的工程事实已在证据表标注=证据纪律守住）。

### 28.2 账本三处精度修正（本轮落实, §27.4 已按此改写）

1. **B 类表述降级**: SDI-IN-2 gst 不可开暂归 **B 类 Real Hardware /
   Runtime Environment Preconditions**, 非已证明的单一硬件故障根因
   ——证据只支持"Runtime Environment 无法获得可用 GStreamer
   binding", 候选 B1..B8（含新增 B8=Runtime probe/OS 设备枚举环境
   异常）, 禁断言唯一根因;
2. **v4 manifest 复核义务**: "恢复后无需再改 manifest"→**"恢复后
   无需修改代码; 必须重新以当日 Discovery 核验 manifest 的 runtime
   binding, 若 device_number 发生变化则据实更新 v4"**（架构自身
   规定 device-number=Runtime instance address 非 Device Identity,
   重枚举后编号可能变为 2/3/其他）;
3. **时间戳审计**: d0ffff9 提交消息/文档记 2026-09-04 而仓库系统
   日期 2026-09-03 = **evidence host clock / timezone mismatch**
   （盒钟先跨日; 不影响技术裁决, 影响 Evidence Package 时间线审计）
   ——后续真机复跑证据必须同录 `date -u` / `date` / `timedatectl` /
   `git rev-parse HEAD` 四件套。

### 28.3 02-I 复跑执行序（①-⑧, 冻结）

```text
① 两路真实 SDI source 接入
② 排查 SDI-IN-2 为什么无法被 GStreamer open（B1..B8 逐一排查）
③ 修复后重新 Discovery
④ 核验当日 gst_device_number
⑤ 必要时据实刷新 v4 manifest
⑥ cargo build --features bmd,gstreamer --bin media-agent-gates
   （普通可执行档必须显式刷新——cargo test/clippy 不刷新;
   run1/run2 陈旧 bin 正反对照已实证其必要性）
⑦ L0 → L5 → Teardown
⑧ 全量 Evidence Package（含 28.2-3 时间戳四件套）
```

结果 PASS=A2-8 收口路径; FAIL=按 A/B/C 分类（A=新 change / B=修环境
不改码 / C=禁临时改架构）, 禁为跑绿改代码。

### 28.4 债务账本（C 类, 全 OPEN 不为 02-I 临时修）

derive_claims 首输入寻址 / PortIdentity direction 入键 / Analog
connector folding / audio capability 独立 SDK 证据 / Serial identity
binding / canonical UUID namespace 统一 / tap_channel 层级归属 /
Production API 503 / PTS normalization execution gap / N-input
general switch——其中 PortIdentity direction 修复若启动必须一次性
联动 PortIdentity→PortId→Manifest→PortRegistry→ResourceRegistry→
derive_claims→Session, 禁只改 UUID 公式。

## 29. 第二十二轮裁决（基线 `9d5c0d8`）: APPROVED / FROZEN / GO 维持——主线切换"02-I 真机条件恢复与证据验收"+ 环境证据包纪律

### 29.1 终裁

> **APPROVED / FROZEN / GO 维持。`9d5c0d0` 系列不需要重新打开 A2-8
> 实现; 主线自"代码审查"彻底切换到"02-I 真机条件恢复与证据验收"。
> 本轮无新代码裁决、无新架构决策需要批准。下一次有效动作 = 硬件条件
> 恢复后的 02-I 第二次真机验收。**

基线状态无冲突（本轮独立核验: 9d5c0d8 恰两份 docs, 下列文件零
触碰——services/media-agent/**·port.rs·resource.rs·session.rs·
pipeline.rs·switch_graph.rs·program_execution.rs）:

```text
fe71b7c = Implementation Freeze
d0ffff9 = 02-I 首轮真机证据
9d5c0d8 = 第二十一轮裁决修正
02-I    = OPEN
```

确认项: ①B 类定义正确（Real Hardware / Runtime Environment
Preconditions, 具体根因未证明——当前证据只到"GStreamer probe 无法
获得可用 binding 且复跑持续", 未证明 duplex/Desktop Video/驱动/占用/
OS enumeration/probe 环境/硬件本身任一; **禁根据猜测改代码**）;
②v4 manifest 当日复核=硬性 Gate（gst_device_number=Runtime address
非 Canonical Device Identity——避免"硬件已恢复但枚举顺序变化, Gate
错绑另一设备"; fail-closed 设计本应阻止此事）; ③run1/run2 新旧行为
对照=H1 CLOSED 强证据（同一真实硬件/同一失败条件: 旧 cb78adc=L1 FAIL
错误继续 L2 被 Preflight 二次闸门兜住, 新 fe71b7c=第一闸门正确
fail-stop·Session 不创建——非单测层面证明）; ④C 类十项债务确认全
不属于 02-I 阻塞, 尤其禁因"恰好两张 Duplex DeckLink"顺手修
PortIdentity 把 PortId→Resource→Manifest→Session 身份链重新打开。

### 29.2 环境证据包纪律（02-I 第二次真机验收起生效——零代码, 非新 Gate）

**证据头五件套**（复跑证据开头固定同录, 在 §28.2-3 四件套上增
`git status --short`）:

```text
date
date -u
timedatectl
git rev-parse HEAD
git status --short
```

**完整执行序**（§28.3 ①-⑧ 细化——增 build 后 HEAD 复核）:

```text
证据头五件套
→ Discovery（两卡 DeviceHandle/DeviceId/PortId/Signal 实测落盘）
→ GStreamer probe（当日 gst_device_number）
→ v4 manifest（据实生成/核验, device_number 变则更新）
→ cargo build --features bmd,gstreamer --bin media-agent-gates
→ git rev-parse HEAD（build 后复核=实际执行确为冻结版源）
→ L0 → L5 → Teardown（VBMF_A2_8_DUAL_INPUT=1）
→ 全量 Evidence Package 归档
```

最终 Evidence Package 须能回答六问: ①什么时候测的 ②哪个时区
③盒子当前跑什么 Git commit ④实际执行的是不是冻结版 gate binary
⑤当时两张卡到底是什么 Discovery 状态 ⑥最终失败/成功属于代码·环境·
硬件哪类（届时仍 FAIL 则严格按 A=代码 / B=Hardware/Runtime
Environment / C=已知架构债务 三分类裁决, 禁为通过 Gate 改代码）。
此纪律比继续增加 Gate 断言更有价值——证据可审计性优先。

## 30. 第二十三轮裁决（基线 `b20ff70`）: APPROVED / FROZEN / GO 维持——02-I 阻塞点重定义: Runtime Address / Provisioning Identity 闭环

### 30.1 终裁

> **A2-8 继续 APPROVED / FROZEN / GO。02-I 继续 OPEN——阻塞点从"第二张卡疑似不可用"
> 改为"必须重建当日的 物理身份 ↔ GStreamer runtime address 权威绑定"。
> 零代码、零 PortIdentity、零 Session/Resource 修改。
> 不批准按现场推断直接生成新 v4 manifest 后跑 L0→L5。**

### 30.2 代码级核验（本轮独立复核, 与裁决一致）

- **resolver.rs**: `resolve_with_manifest()`（:903）验证链 = Manifest 宣称
  dn → probe 可开 → 可选 `expected_hw_serial_number`/`expected_model`
  交叉校验（:939-985, 不符 fail-closed）; `is_production_grade()`（:528）
  要求 HIGH confidence 且接受 PersistentId/Serial/DeviceHandle exact/
  ManifestVerified（:535, :1022）。**语义边界（:615 注释已自知"当前硬件
  serial 恒空"）**: hw-serial=NULL + 两卡同 model=DeckLink SDI 时,
  "dn 可开 + model 相符" ≠ "dn ↔ 指定 Handle 同一硬件"——
  `ManifestVerified` = Manifest 指定 dn + probe 成功 + 可选校验通过,
  **非 Handle↔runtime 硬件同一性证明**。登记不修（canonical identity
  closure 独立 change 冻结; 禁为 02-I 塞 device-number/拓扑猜测进
  resolver）。
- **dual_input.rs**: Gate 经 `collect_bindings_from_manifest()`（:198）
  消费 Manifest 解析绑定; L1/H4 按绑定采样 dn/signal（:232-249）,
  **无写死 device-number**——首跑 "SDI-IN-2 unresolved" = Manifest→probe
  验证失败, 非 Gate 硬编码"第二张卡必须是 dn2"。Gate 不改。
- H2/Resource/Session/ProgramExecutionRuntime/SwitchGraph 链未被击穿
  （device-number 从未被当 Port identity）; 出问题的仅
  Canonical DeviceHandle→Runtime binding→gst dn 这一 runtime mapping 层。

### 30.3 定性修正（对 §29 现场报告）

1. 现场三重互证推断 gst 序今日=[SDI(1), SDI(2), Mini] = **runtime /
   physical correlation evidence, 非 canonical identity proof**——两层次
   严格分开, 禁用 runtime enumeration order 反推 DeviceHandle
   （resolver 自身冻结原则）;
2. PID 577061 ball sink（dn2, 09-02 07:38 起）= **保留为现场事实, 不判定
   为最终根因**——更合理解释: dn2 = Mini Monitor output-only slot 被
   sink 使用 → 自然无法作 video input 打开（B4"占用"降级, 与 ffmpeg
   双输入成功证据一致）;
3. 旧 v4 manifest **正式作废, 不直接复用**（裁决批准）;
4. 已证明事实: 两路 BNC 接线正确（#2/#4 均输入）; SDI-IN-1 = A 类
  （真实输入有信号 1080i25）; SDI-IN-2 = A 类（真实输入有信号 1080p25）;
  两路 BMD SDK 输入能力活着（ffmpeg 75 帧/3s × 2）——02-I 已具备进入
  最终验收的硬件基础。

### 30.4 下一步 = 身份闭环核验（Provisioning）, 非简单"刷新 manifest 重跑"

```text
当前真实 Discovery → DeviceHandle A/B
  ↔ 物理 BNC（#2 电视 / #4 4K 输出卡）↔ SDI(1)/SDI(2) 输入
  ↔ GStreamer runtime probe（dn0/dn1/…）
  → 人工 / 物理 / 官方工具交叉确认
  → 新 v4 Manifest（真正 Provisioning 意义）
  → frozen binary build → L0 → L5 → Teardown
```

禁: 猜 dn0/dn1/dn2 → 写 v4。**"dn0=SDI(1)、dn1=SDI(2)" 现在不写死
进 v4**——须先完成 DeviceHandle↔物理输入↔runtime address 权威确认,
v4 才具有 Provisioning 意义。身份闭环完成后进入 L0-L5; 届时仍 FAIL
仍按 A/B/C 三分类裁决, 禁为跑绿改码。

## 31. 第二十四轮执行（基线 `8fea7ea`）: 02-I Provisioning Identity Closure 现场执行——Step 0/1/2 证据包（零代码）

### 31.1 裁决记录（对 §30.4 的严格修正）

> 维持 APPROVED / FROZEN / GO。**"人工/物理/官方工具交叉确认"=
> Provisioning/Evidence 层必要证据, 不是 A2-8 Runtime 代码前置条件,
> 不因此重开代码 change。**11 不清单: 不改 Resolver/Manifest schema/
> PortIdentity/ResourceRegistry/SessionManager/ProgramExecution/
> SwitchGraph, 不增 Runtime 自动猜测, 不因 dn 枚举变化改码, 不把
> PID 577061 当已证明根因, 不把 runtime correlation 冒充 canonical
> proof。状态梯: BMD physical PASS·SDK enumeration PASS·FFmpeg
> acquisition PASS·GStreamer runtime map 未闭环·Manifest binding 待
> 重建（"SDI-IN-2 hardware unavailable" 正式撤销）。唯一工作项 =
> 02-I Provisioning Identity Closure。

### 31.2 Step 0 环境证据（盒 2026-09-04 14:49 CST, NTP synced）

盒 build 目录非 git checkout → 以 sha256 等价替代 git 两件套:
**68/68 .rs 文件 盒==本地 HEAD `8fea7ea`（=fe71b7c 实现冻结基线,
services/ 自冻结零改动）**, sort-normalized diff 为空。双侧清单归档
盒 `~/a2-8-02i-evidence/2026-09-04-step0-*`。

### 31.3 Step 1 当日 Discovery（`VBMF_RESOLVER=1`, cargo build 后 bin）

SDK 侧: 3 设备+lease 幂等全过（4fa33dcb/46:…2e4500·6ede00d0/
46:…2e4400·1afe2dcc Mini）。gst 侧: **dn0/dn1=PropertyMissing**
（设备可开至 Playing 但 hw-serial-number/persistent-id/model 全空
→无法建立身份）; **dn2-7=StateFailed**; legacy 全 Unresolved +
"production MUST reject" 注记。形态与 09-03/04 首跑**完全一致**
→当时 unresolvable 的 runtime 侧根源=本机身份字段常态缺失, 非新故障。

### 31.4 Step 2 内容特征差分（视觉指纹 + 杀源差分 + 复原）

**A. 视觉指纹**（gst 抓 JPEG 帧模型判读, 归档 `~/a2-8-02i-evidence/
frames/`）: **dn0=真实电视广播**（临沂经济生活频道《真心英雄13》
警务/演播场景）→BNC#2; **dn1=ball 测试图**（videotestsrc 特征）
→BNC#4。**B. 杀源差分**（kill PID 577061 ball sink→4s 后）:
进程死透; dn1 Signal lost ✓; **但 SDI(2) 仍锁定 1080p25 出帧且内容
仍=ball → BNC#4 的 ball 源独立于 PID 577061/Mini Monitor 输出——
"BNC#4←4K 卡"假设被证伪**; dn0 同窗 Signal lost（电视分钟级抖动
第三次实证）。**C. 复原**: 原命令行 nohup 重启（新 PID 992634,
14:56:19 CST）。

### 31.5 Provisioning 映射表（证据分级）

| 链 | 证据 | 等级 |
|---|---|---|
| dn0 ↔ 电视(BNC#2) ↔ SDI(1) ↔ 1080i25 | 视觉+模式+ffmpeg 按名 | **PROVEN** |
| dn1 ↔ ball(BNC#4) ↔ SDI(2) ↔ 1080p25 | 视觉+模式+杀源差分 | **PROVEN** |
| dn2 = Mini Monitor output-only（输入面恒败） | 矩阵+ffmpeg 输入清单 | **PROVEN** |
| BNC#4 ball 源 ≠ PID 577061（Mini）输出 | 杀后信号不灭 | **PROVEN**（BNC#4 线缆实际对端=现场待核） |
| 4fa33dcb→SDI(1)·6ede00d0→SDI(2) | 双侧同 IDeckLinkIterator 序（VBMF lease 序 vs ffmpeg 列表序） | **CORRELATION ONLY**——待用户裁决/照片/官方工具侧证 |

事实记录（非改码提案）: DeviceInfo.display_name（SDK GetDisplayName,
含 "(1)/(2)" 后缀）已被适配器捕获但无任何 gate 输出面打印——身份
closure change（冻结）可为未来读出点。

### 31.6 候选 v4（待裁决, 不写死）与残余风险

据上证据链候选: SDI-IN-1(4fa33dcb/46:…2e4500)→gst 0·SDI-IN-2
(6ede00d0/46:…2e4400)→gst 1。**最终成立条件=用户裁决 iterator 序
correlation 或提供照片/官方侧证**（correlation 单独不作 canonical
proof——§30.3-1 红线）。残余: ①BNC#4 独立 ball 源的物理对端设备
待现场核实（照片/线缆追踪）; ②dn2→Mini 输出线缆去向未知; ③电视
分钟级抖动=L1c 时序风险（撞窗即 B 类, 重跑不改码）。照片请求: 本
侧仅 SSH 无物理在场, 需用户侧提供; 已以四帧内容 JPEG（dn0/dn1/
双输入 postkill）作为内容侧物理证据归档。

## 32. 第二十五轮执行（基线 `56f8b8e`）: Provisioning Identity Closure 零代码达成 + 02-I 第二次验收（v5）——L1c 采样窗口发现

### 32.1 裁决记录

> 维持 APPROVED / FROZEN / GO。**否决候选 v4 直接生成**——iterator 序
> correlation 只作证据、不作 canonical identity（index 0==index 0 在
> SDK 序/驱动序/占用/过滤变化下可失效）。GO = Provisioning Identity
> Closure。Priority 1=零代码取得 handle→GetDisplayName; 无出口才建
> 窄 Provisioning Identity Probe（Evidence 工具非 Runtime 依赖）。
> BNC#4 重定义为"独立 1080p25 ball 源, 对端待现场确认"; PID 992634
> 只记为独立 SDI 输出测试进程。一旦身份链闭合即批准据实生成新
> v4→frozen build→HEAD 复核→L0→L1→…→L5→Teardown。

### 32.2 Priority 1 达成——canonical closure 零代码闭合（零 iterator 假设）

三链拼合（全部既有/当日证据）:

1. **VBMF 确定性联结**（run2 既有日志, 代码派生非顺序相关）: 碰撞
   告警（port.rs:871, 含 @display_name）: port_id `e43d8f5a`↔
   **"DeckLink SDI (1)"**、`f0f53b80`↔**"DeckLink SDI (2)"**;
   H4 行: `4fa33dcb`↔e43d8f5a、`6ede00d0`↔f0f53b80
   ⇒ **handle↔SDK 显示名**;
2. **内核驱动 canonical**（当日 dmesg/lspci）: `dv0[pci@0000:44:00.0]`、
   `dv1[pci@0000:45:00.0]`; 两 SDI handle 差异字节 44/45↔PCI bus;
   Mini 芯片序列 `1a66443b` 与 handle `83:1a66443b:00000000` 中段交叉
   命中（验证 handle 承载驱动身份）⇒ 4fa33dcb=45:00.0=dv1·
   6ede00d0=44:00.0=dv0——**SDK 显示名 (1)=PCI45=dv1: SDK 序≠dv 序
   ≠PCI 序, 实证"iterator 序非 ABI 契约"**;
3. **内容指纹**（§31）: SDI(1)=1080i25 电视=BNC#2=dn0·
   SDI(2)=1080p25 ball=BNC#4=dn1。

⇒ **`4fa33dcb/46:…2e4500 = SDI(1) = dn0 = BNC#2 = 电视`;
   `6ede00d0/46:…2e4400 = SDI(2) = dn1 = BNC#4 = ball`**。
   附实锤: 旧 v4 声称 4fa33dcb→gst 1 = **错绑**（run2 H4
   prod_binding=true 恰把 SDI-IN-1 身份绑上 SDI(2) 硬件）——作废
   裁决完全正确。

### 32.3 v5 据实生成 + 02-I 第二次验收

v5 = SDI-IN-1(2e4500)→gst 0·SDI-IN-2(2e4400)→gst 1（盒
`~/a2-8-02i-v5.manifest.json`, JSON 校验过）; §29.2 纪律全程
（证据头五件套 15:38 CST·cargo build OK·bin 源=sha 验证冻结基线）。
结果: **L0 PASS（进入 L1 链）·L1a PASS bindings=2/2
production_grade（首次双卡 ManifestVerified+HIGH）·L1b PASS·L1d
PASS 双卡（H2 闭环再证）**——**L1c FAIL 双卡 signal=Some(false)**
→ H1 fail-stop, exit 2, 零会话创建（fail-stop 行为正确）。
日志=`~/a2-8-02i-evidence/2026-09-04-02i-second-acceptance-v5.log`。

### 32.4 L1c FAIL 根因定位: Gate probe signal 采样窗口（A 类证据自动化发现, 冻结未修）

跑后同分钟复核: **ffmpeg 双输入均出帧（信号物理在场）**; gst 手动
12s 全窗: 双卡均 `signal=false→true` 翻转+caps 锁定（dn0=1080i25
电视·dn1=1080p25 ball——v5 映射内容级再验证正确）。
**代码锚点: resolver.rs:230-232 `set_state(Playing)` 后仅
`sleep(300ms)` 即读 signal（:257-259）**——decklink 输入信号检测器
锁定需 ~1-3s（实测消息序 #21 false→#38 true）, 300ms 窗口
**结构性假阴性**; dual_input L1c/H4 复用该 probe。**生产链不受
影响**（长生命周期管线·L3 用 SAMPLE_GAP 采样增量）; 属 Gate 证据
自动化缺陷 ⇒ **§11 A 类候选（新 change 范畴, 本轮零代码未动）**。
**probe 修复前 L1c 确定性 false——02-I 无法通过 L1c, 待用户裁决
最小修授权（如采样窗延长/重试窗口）**。历史一致性: 09-04 run2
L1c signal=Some(false) 同受此窗口影响（当时信号在场性另议）。

### 32.5 残余清单

① **L1c probe 采样窗口=02-I 唯一剩余代码级阻塞**（A 类候选待裁）;
② BNC#4 独立 ball 源物理对端 + dn2→Mini 输出线缆去向=现场项
（照片/线缆追踪, 用户侧）; ③ 电视分钟级抖动=L2-L5 潜在 B 类时序
（撞窗重跑不改码）; ④ 其余 C 类债务账本不变。

## §33 第二十六轮终裁执行: A2-8-C1 授权落地 + 第三/四次 02-I 验收（L1c 修复真机成立·L4 新签名确定性复现）

### 33.1 裁决接收（第二十六轮: APPROVED / FROZEN / **CHANGE REQUIRED**）

状态梯子: Architecture APPROVED · Runtime FROZEN · Provisioning Identity
CLOSED · v5 VALID · Hardware PASS · L1a/b PASS · **L1c=BLOCKED BY PROBE
DEFECT** · L1d PASS · L2-L5=NOT YET VALIDLY EXECUTED · **Change REQUIRED**。
定性更正: L1c false **非 Hardware FAIL**——probe 把 "检测器未锁定" 误投影为
"无信号" = 证据语义错误（§11 归 A 类, 开 C1）。**A2-8-C1 授权范围**: 仅
resolver.rs; 允许=观察窗/重采样/窗口内重读/锁定提前结束/超时 fail-closed/
保留错误分类与生产绑定语义; 禁止=改 Manifest/v5/identity/L1c 判定原则/H1/
Session/ProgramRuntime/SwitchGraph/Supervisor/MediaTap/recover SPI/L0-L5
状态机/独立 Gate pipeline/**顺手重构 Option\<bool\>**/其他 P1 债务。
probe contract 冻结: `PROBE_SIGNAL_WINDOW=3000ms` ·
`PROBE_SIGNAL_INTERVAL=100ms`（当前 A2-8 验收策略值非永久冻结）。
v4=**INVALID/ARCHIVED**（historical invalid provisioning artifact, 任何
Gate 默认配置禁再引用）; v5 保留不重生成。

### 33.2 C1 实现（1 文件 +86/−3, commit **1c3032b**）

- 常量 `PROBE_SIGNAL_WINDOW`/`PROBE_SIGNAL_INTERVAL`（resolver.rs:22-25,
  注释写明 1-3s 实测依据与策略值性质）;
- 观察窗自 **set_state(Playing) 起算**（:239-240 deadline 锚点, 窗口覆盖
  既有 300ms 错误上报宽限——裁决契约 "t=0 在 Playing"）;
- signal 读取: 单次快照 → `poll_signal_until_locked` 窗口化（:267-274:
  100ms 间隔重采样·锁定即返·耗尽 `Some(false)` fail-closed·find_property
  缺失仍 None）;
- 纯决策核 `poll_signal_until_locked`（:370-387, 抽样器注入, 双 profile
  可测）;
- **契约零变更**: `GStreamerDeviceProbe.signal` 仍 `Option<bool>`;
  错误分类链（OpenFailed/StateFailed/PropertyMissing）与生产绑定语义原样;
- 3 单测: `false,false→true` 窗口内锁定 PASS / 全窗 false fail-closed 且
  重采样≥2（证非单次快照）/ 首采 true 恰 1 次采样（证不烧窗）;
- **dual_input.rs 零改动**（:232-249 仍消费 resolver probe = 单一设备
  打开者, 裁决 §8/§9 天然合规）。

### 33.3 盒上矩阵（§29.2 纪律）

sha256 **68/68 盒源==HEAD 1c3032b**; bin `media-agent-gates`
（--features bmd,gstreamer）sha `f0ca5db9…`。fmt --check OK;
**mock 211→214**（同命令前后实测, +3 恰为新测试; 注: 账本历史 mock 数为
workspace 口径, 盒 crate 口径以 211→214 为准）; **bmd+gstreamer 233 全过**
（新 3 测命中日志）; clippy `--all-targets -- -D warnings` 双 profile OK。
证据: 盒 `/tmp/c1-{mock,hw}-test.log`。

### 33.4 第三次 02-I 验收（v5, 2026-09-04 15:59:24 CST）

证据头五件套+bin sha 已入 log（盒 `~/a2-8-02i-evidence/
2026-09-04-02i-c1-acceptance.log`; NTP synchronized; HEAD=1c3032b;
status clean）。结果: L0 PASS · L1a PASS（bindings=2/2 production_grade,
双 ManifestVerified+HIGH）· L1b PASS · **L1c PASS**（dn0/dn1 均
`signal=Some(true)`——**C1 窗口语义真机成立, §32 根因确认修复**）· L1d
PASS（双卡 closure）· L2a PASS（session 双输入·H3 port_id 精确消费）·
L2b PASS（双 tap frames=83）· L3 PASS（video 120→210·audio 160→280·
ValidMonotonic）· **L4 FAIL** · L5 FAIL（=H1 设计性跳过, 非独立失败）·
Teardown PASS（Program Stop→Tap Detach→Input Stop→Release 全真）。
总 **8/10 verdicts, EXIT=2, 全链完成 L0→L5+Teardown**（A2-8 历史上
首次越过 L1）。

### 33.5 L4 FAIL: 确定性签名 + 初步归类（待终裁）

- **判据锚 dual_input.rs:644-648**: PASS = completed ∧ observed==B ∧
  epoch==1 ∧ **prog pts state≠NonMonotonic** ∧ pts.is_some。本跑前四项
  **全真**（切换机制 Desired=Execution=Observed 完整成立）, 唯一失败项=
  NonMonotonic;
- **复跑 2**（rerun2.log, 同日 16:0x CST）: 同签名逐项复现（8/10·EXIT=2）
  → **确定性签名, 排除电视分钟级抖动（B 类瞬态）**;
- 数字: 两跑 pre→post prog +4.0s≈settle 窗推进; A/B in 列互差 8-10ms
  （B 落后）; **in/bridge 各列保持 ValidMonotonic, 仅 prog 列翻转**;
  alive=false 为复合字段推论（program_execution.rs:111-112:
  pts.is_some ∧ ≠NonMonotonic）非独立停流证据;
- 机制: 两路自由跑源时钟（电视 1080i25/ball 1080p25 独立发生器）在
  input-selector 衔接, Program 出口无 normalization ⇒ 切换点时钟域衔接
  被观测历史分类器判 NonMonotonic——**即 A2-8-01 第三轮已裁架构级硬事实
  （source switching≠Program Timeline continuity·Timestamp Normalization
  四方案未裁·timeline continuity=DEFERRED/FAIL-PENDING-CORRECTION）的
  真机表达**;
- **初步 §11 归类: C 类（已登记架构债务）候选**——非新代码缺陷·非硬件
  前置; 最终裁归用户。连带: L5（故障注入/recover/隔离）因 H1 规则未
  执行, 真机 L5 证据仍缺;
- 工件附录（上报不定性）: `gst_video_converter` interlace 断言两跑各恰
  9 条（均在 A=interleaved 活动期; 帧流未断, L2/L3 PASS 不受影响）;
  Bus watch MainContext warn ×1/跑; teardown `pad_unlink` ×4/跑。

### 33.6 状态梯子（本轮后）

Architecture APPROVED · Runtime FROZEN（C1=授权内唯一破冰, 已并入）·
Provisioning Identity CLOSED · v5 VALID · Hardware PASS ·
**L1a/b/c/d 全 PASS** · **L2/L3 PASS** · L4=C 类候选待裁 ·
L5=未执行（H1 跳过）· Teardown PASS。

### 33.7 残余清单

① **L4/L5 终裁待用户**（选项: 裁 Timestamp Normalization 四方案之一后
开 normalization change 再跑 / 裁 L4 判据或 H1 例外（改 Gate 表面, 需
授权）——本轮零改）; ② 现场项不变（BNC#4 独立 ball 源对端/dn2→Mini
线缆去向/照片, 用户侧）; ③ converter interlace 断言=潜在独立候选
未定性; ④ 其余 C 类债务账本不变。

## 34. 第二十六轮终裁：C1 收口 + L4 双维记账 + C-TIMELINE-01 正式登记（零代码）

> 落账：2026-09-04，分支 comet/a2-8-dual-input-switch（基线 470f1a0 之上，
> **本轮零源码改动**）。裁决来源=用户第二十六轮终裁全文；本节为接收、
> 逐条锚点复核与登记。用户侧核验边界（原话要旨）：GitHub 连接器可读
> master 0b3c73a 基线，但 1c3032b/470f1a0 分支引用当前无法直接解析——
> "C1 的 resolver.rs 新增代码本身，我不把你贴出的报告当成已独立核验源码"；
> L4 相关 dual_input.rs / program_execution.rs / switch_execution.rs /
> pipeline.rs 可直接核验且足以支撑架构裁决。

### 34.1 终裁结论（照录骨架）

- **A2-8-C1：PASS / CLOSED**；
- **A2-8-02-I：L0～L3 PASS；L4 = PASS（Switch Execution 子项）+
  FAIL-PENDING-CORRECTION（Program Timeline Continuity 子项）；L5 =
  SKIPPED BY H1（合法前置条件未满足，不计独立失败）；Teardown PASS**；
- **A2-8-02-I 整体 = FAIL-PENDING-CORRECTION**——精确语义：非 A2-8 基础
  设施失败；A2-8 已完成真实双输入切换执行闭环，但 Program Timeline
  Continuity 未实现，"切换 + 节目时间线连续"完整验收尚未闭合；
- **下一阶段：单独开 Timeline/PTS Normalization change；不修改 A2-8 的
  L4 证据原则；不修改 H1；本轮零代码。**

四不批准（红线，后续任何轮次禁偷渡）：

1. **不批准现在直接进入 Timestamp Normalization 实现**——
   `PipelinePlan.normalize` 仍是声明层字段未被 Execution Adapter 消费
   （A2-7 已登记 Adapter Gap）；为跑绿临时插入 normalize 会把
   声明/Execution/Observation 三层重新耦合，违反 Intent→Plan→Fact 冻结
   边界；
2. **不批准 H1 例外**（L4-TIMELINE FAIL 仍跑 L5）——Program Timeline
   已知异常时 L5 的 Program 级观察会被污染，无法区分 failure isolation
   与 pre-existing PTS discontinuity，违反 evidence purity；
3. **不批准把 L4 判据降为只看切换成功**——会把真实架构问题从验收系统
   抹掉；
4. **不批准在 SwitchGraph / ExecutionGroup / SwitchDesired /
   SwitchExecutionPlan 内做 Normalize**（PTS offset / timestamp
   rewriting / segment manipulation / GstPadProbe timestamp mutation
   全禁）——Switch Intent 与 Timeline Execution 禁止重新耦合。

### 34.2 终裁代码锚点复核（HEAD 470f1a0 本地实证；盒==HEAD 已于 §33.3 sha256 68/68 复核）

| 终裁引用 | 实锚 | 复核 |
| --- | --- | --- |
| L4 前四项=Switch Execution 维（completed/observed==B/epoch==1） | gates/dual_input.rs:644-648 | ✅ 逐字一致，本跑全真 |
| L4 后两项=Timeline 维（state≠NonMonotonic ∧ pts.is_some） | 同上 | ✅ 唯一失败项=NonMonotonic（§33.5） |
| TimelineSample 三列独立测量 | program_execution.rs:59-77（input/bridge/program × video/audio 各 pts+state） | ✅ 三列拆开、只观测 |
| sampled_at_ms=wall-clock 与 PTS=media-clock 分离 | program_execution.rs:60 | ✅ C2 禁拿 sampled_at_ms 修 PTS |
| program_alive=复合字段（非把 PLAYING 冒充 Timeline Healthy） | program_execution.rs:111-112 | ✅ pts.is_some ∧ ≠NonMonotonic |
| PipelinePlan.normalize 声明未被消费 | pipeline.rs:136-141（doc 自认"**未被 Execution Adapter 消费**——normalize=true/false 生成管线相同"）；全仓消费点仅测试断言 | ✅ Adapter Gap 成立 |
| ExecutionGroup 不存时间戳、不 Normalize | switch_execution.rs:93-100（恰 {session_id, inputs, desired, switch_epoch}） | ✅ 切换执行与时间戳=两责任域 |
| SwitchExecution 纯模型边界 | switch_execution.rs:4/:16-17（零 GStreamer 依赖·不构图·不 recovery） | ✅ |
| H1: L4 FAIL→L5 跳过 | gates/dual_input.rs:774 | ✅ 设计性跳过维持 |
| C1 落点（收口对象） | resolver.rs:25/:27/:240/:268/:373 + 3 单测 | ✅ 在 1c3032b，用户侧源码核验见 §34.7 |

### 34.3 第二十六轮终裁表（照录）

| 项目 | 最终裁决 |
| --- | --- |
| C1 Resolver Signal Probe | **PASS / CLOSED** |
| L0 | **PASS** |
| L1a | **PASS** |
| L1b | **PASS** |
| L1c | **PASS** |
| L1d | **PASS** |
| L2a | **PASS** |
| L2b | **PASS** |
| L3 | **PASS** |
| L4-SWITCH | **PASS** |
| L4-TIMELINE | **FAIL-PENDING-CORRECTION** |
| L4 Overall | **FAIL-PENDING-CORRECTION** |
| L5 | **SKIPPED BY H1** |
| Teardown | **PASS** |
| v5 Manifest | **VALID / RETAIN** |
| v4 Manifest | **INVALID / ARCHIVED** |
| Identity | **CLOSED** |
| Port collision issue | **当前 A2-8 不再阻塞** |
| Normalize | **仍为 Adapter Gap** |
| H1 | **保持不变** |
| Supervisor | **不改** |
| SessionManager | **不改** |
| SwitchExecution | **不改** |
| MediaBackend SPI | **不改** |

### 34.4 C-TIMELINE-01 正式登记（C 类债务，取代 §33.5 "初步 C 类候选"）

- **定义**：Program Timeline Continuity Gap——双输入各自独立 clock domain
  经 selector 汇入 Program，切换点无负责重建 Program PTS continuity 的
  执行组件；真机表达=Input/Bridge 全列 ValidMonotonic 而 Program 列
  NonMonotonic（§33.5 确定性签名复跑 2 复现）。终裁定性=Architecture /
  Execution Adapter Gap（A2-7 已登记项）被真实双输入硬件首次暴露，
  **非 C1 残留的 Resolver/硬件/切换执行缺陷**。
- **排除项（终裁明确否定）**：非 C1 Resolver bug / 非 DeckLink identity /
  非 PortRegistry / 非 SwitchAdapter partial execution / 非 Supervisor /
  非 Hardware signal instability。
- **正面确证（终裁第十三节）**：Device→Port→Resource→Manifest→Resolver
  →dn→Session→SessionInput A/B→ExecutionGroup→Tap→Bridge→
  SwitchAdapter→Observed Active→Program 全链真实走通；首次真实暴露
  Input Timeline ≠ Program Timeline——证明验收系统没有把"两个输入能
  切换"错误等同"节目时间线连续"，ExecutionGroup / Observed-Desired
  分离 / 三列 Timeline Evidence 设计经受住实机验证。
- **设计裁决十问（独立 change 开工前置，冻结前禁写 normalization 代码）**：
  ① Program timeline 的 authority 是谁；② 切换时新源 PTS 如何映射；
  ③ video/audio 是否共享 epoch；④ discontinuity 如何处理；⑤ switch
  settle 期间如何处理；⑥ PTS 是否允许 offset；⑦ wall-clock 与
  media-clock 如何分离；⑧ downstream encoder 如何看到 continuity；
  ⑨ recover 后是否重新建立 epoch；⑩ observation 如何证明 normalization
  真执行。
- **开工门（第一问）**：Program Timeline Authority 放在哪里 + A→B 切换时
  Video/Audio PTS 如何建立连续映射——未冻结前开发助手禁写 normalization
  代码（终裁原令）。
- **必须保留的架构边界（"不要顺手修"清单）**：`sampled_at_ms`（wall-clock）
  绝不能修 PTS（media-clock）；Bridge liveness=observation clock 窗口与
  PTS monotonicity=media time 两证据域分层不变（二十轮 G/H-1 已建）；
  switch_execution.rs 纯模型边界（不构建 GStreamer graph / 不执行
  recovery / 不负责 timeline）不变。

### 34.5 L4 双维记账口径（验收账面模型；Gate 代码不动）

| 子项 | 判据 | 本次真机 |
| --- | --- | --- |
| L4-SWITCH | completed ∧ observed==target ∧ epoch==1 | **PASS** |
| L4-TIMELINE | program pts exists ∧ monotonic | **FAIL-PENDING-CORRECTION** |
| L4 overall | 双维合取 | **FAIL-PENDING-CORRECTION** |
| L5 | H1 前置=L4 overall | **SKIPPED BY H1** |

注：现行 dual_input.rs 单 `bool l4` 输出 FAIL，与 L4 overall 口径**零改码
天然一致**；终裁批准的"L4 子项拆分"为验收记账模型——**代码级 Gate 表面
拆分未授权于本轮**（本轮零代码），留待后续独立授权或随 normalization
change 一并裁。

### 34.6 状态梯子（终裁后）

Architecture APPROVED · Runtime FROZEN（fe71b7c + C1 1c3032b）·
Provisioning Identity CLOSED · v5 VALID（v4=INVALID/ARCHIVED）·
Hardware PASS · L1a/b/c/d PASS · L2/L3 PASS · L4-SWITCH PASS ·
L4-TIMELINE FAIL-PENDING-CORRECTION（L4 overall=FAIL-PENDING-
CORRECTION）· L5=SKIPPED BY H1（不计独立失败）· Teardown PASS ·
**02-I 整体=FAIL-PENDING-CORRECTION（停在明确 correction point）** ·
下一阶段=独立 Timeline/PTS Normalization 设计裁决（未开工）。

### 34.7 边界披露与残余

- **C1 源码的用户侧独立核验**：用户声明 GitHub 连接器当前无法解析
  1c3032b/470f1a0 分支引用，不将本报告当作"已独立核验源码"。如实登记：
  C1 CLOSED 依据=真机验收证据（L1c PASS ×2 轮）+ 盒==HEAD sha256 68/68
  + 测试矩阵；分支已推送远端（0b3c73a→1c3032b→470f1a0→本轮落账），
  用户侧独立源码核验随时可做，本账不宣称"用户已核验源码"。
- 残余：① 下一刀=Timeline/PTS Normalization 设计裁决（独立 change；
  十问未裁禁写码；本轮仅登记未开工）；② converter interlace 断言
  （每跑恰 9 条）待裁；③ 现场项（BNC#4 对端/dn2→Mini 线缆/照片）
  用户侧不阻塞；④ 冻结债务不变（PORT-IDENTITY-AND-RESOURCE-
  ADDRESSING · canonical UUID namespace · A2-8-03/04/05）。

## 35. 第二十六轮终裁补正：维持 + 两处账面表述修正 + C1-P1 登记 + C-TIMELINE-01 CONFIRMED（零代码）

> 落账：2026-09-04（d123b45 之上，本轮零源码改动；本节为追加，
> §34 及第二十六轮全部账面保持原样）。裁决来源=用户第二十六轮终裁
> 补正全文。**用户独立核验范围升级**（原话"这次不是只看你贴出来的
> 报告"）：直接核验 470f1a0 / 1c3032b / d123b45 三 commit + 八个源
> 文件 + 两次真实 compare（fe71b7c→1c3032b、470f1a0→d123b45）。
> **§34.7 边界披露就此解除**：①470f1a0→d123b45 仅两账面文件零夹带
> 源码；②fe71b7c→1c3032b 运行时代码变更仅 resolver.rs（其余=0b3c73a
> 二十五轮账面）——本轮"零代码"成立。

### 35.1 裁决骨架：全部工程裁决维持 + 两处账面表述补正

维持：C1=PASS/CLOSED · L0-L3 PASS · L4-SWITCH PASS ·
L4-TIMELINE=FAIL-PENDING-CORRECTION · L4 Overall 同 · L5=H1
SKIPPED · 02-I=FAIL-PENDING-CORRECTION · 暂不实现 Timestamp
Normalization · 不放宽 H1 · 不改 SwitchExecution/ExecutionGroup/
Supervisor/SessionManager · 下一刀=独立 Program Timeline / PTS
Normalization 设计裁决。

- **补正一（C1 变更范围表述限定）**：C1"只改 resolver.rs"须限定为
  **运行时代码变更**——fe71b7c→1c3032b compare 同时含 tasks.md +
  probe report 账面修改（即 0b3c73a 二十五轮落账）。准确表述=
  **运行时代码变更只有 resolver.rs，架构/账面文档同步另计**。后续
  账面引用 C1 一律采用限定表述。
- **补正二（C1-P1 债务登记）**：见 §35.3。

### 35.2 用户代码级确认清单（要点照录 + 本地锚点复核全部吻合）

| 用户确认 | 本地实锚 |
| --- | --- |
| C1 语义=PLAYING 起 deadline + 300ms 错误宽限 + 轮询早退（非 sleep 3s 读一次；锁定即返不人为烧满窗口） | resolver.rs:239-243 / :240 / :268-272 / :373-387 |
| ProgramObservation SPI 本即多维证据面（active/video/audio/epoch/input_pts/program pts+state+frames），非为 L4 事后拼凑 | contracts/switch.rs:56-77 |
| ExecutionGroup 恰 {session_id, inputs, desired, switch_epoch} 零时间戳——不是 Timeline Authority；complete_switch 须真实 Observed B 才推进 | switch_execution.rs:93-100 |
| Program graph 无 timeline 层（identity/videorate/timestamp rewriting/segment offset/PTS offset/timeline mapper 全零） | adapters/gstreamer/switch_graph.rs 全文件零命中 |
| Bridged 无 capsfilter（Simulation→capsfilter · Bridged→None 透传输入管线实际媒体时间属性） | switch_graph.rs:219-231（:231 `Bridged => None`）/ :253-258 / :297-300 |
| PtsMonotonicity 判定器正确（pts<last→NonMonotonic 且 sticky）；Program PTS=真实 appsink buffer PTS 非簿记推导 | pipeline.rs:236-246 / :291-311 + switch_graph.rs:147/:241 |
| L4 账法=三维分记维持；H1 维持（L5 以整个 L4 为前置）；L5 SKIPPED=正确状态非遗漏 | dual_input.rs:644-648 / :774 |
| normalize=声明存在、执行不存在（normalize=true 非 Execution Fact） | pipeline.rs:136-141 |
| A/B 异构 1080i25↔1080p25：video format continuity 亦未定义，须进设计裁决、禁提前实现 | §33 真机证据 + 本轮新开设计探针（另文件） |

### 35.3 C1-P1 登记（独立小债务；不重开 C1 · 不阻塞 C-TIMELINE-01）

- **定义**：signal polling window 内异步 Bus Error 未二次 drain——
  probe_one_device_number() 在 300ms 错误宽限检查处恰调用一次
  `drain_bus_error`（resolver.rs:243-245；fn 定义 :149），随后进入
  ≤3s signal 轮询，轮询闭包仅采样 `el.property::<bool>("signal")`
  （:268-272）**零 bus 交互**。若设备异步 Error 在 t≈300ms 后到达
  bus，Resolver 将把真实运行时错误表现成 `signal=Some(false)` 而非
  `ProbeError::StateFailed(...)` 分类——与 ProbeError 分类契约（区分
  "卡存在打不开" vs "卡没信号"）轻微不完整。
- **定性（终裁十节）**：非当前 02-I L1c blocker（真机 PASS ×2）·
  非 C1 FAIL · 不影响 v5 身份闭环 · 与 L4 PTS 问题无关——仅登记。
- **修复面（未来授权时）**：极小=poll iteration 内可选 bus error
  check（sample → 可选 drain → sleep），**禁重新设计 Resolver**。
- **执行令**：不重开 C1；不阻塞 C-TIMELINE-01。

### 35.4 C-TIMELINE-01 = CONFIRMED（自 §34.4 "正式登记"升级）

代码级三证据（用户 compare + 本地复核）：

1. Program graph 拓构=双源 → input-selector(video) + input-selector
   (audio) → appsink（switch_graph.rs:8-12 / :218 / :276），全文件零
   identity/videorate/timestamp rewriting/segment offset/PTS offset/
   timeline mapper——无 `Program PTS = f(Source PTS, Program
   Timeline)` 组件；且**零 clock/base_time/latency 设置**（grep 全文件
   零命中——program pipeline 未声明任何时间权威）；
2. Bridged 模式消费输入管线实际媒体时间属性（capsfilter=None :231）；
3. L4 FAIL=真实 appsink buffer PTS 回退（PtsMonotonicity sticky，
   pipeline.rs:291-311），非采样算法缺陷、非簿记推导。

新增维度（终裁十三节）：**A/B 异构视频 1080i25↔1080p25**——PTS
monotonicity 之外 video format continuity 仍为未定义行为；
pass-through / Deinterlace / Caps normalize / Format conversion /
Switch boundary adaptation 五选项须进设计裁决，禁提前实现。

### 35.5 设计十问 v2（A2-8-C-TIMELINE-01 开工前置；照录终裁十六节）

① Program Timeline Authority；② A→B 切换 PTS mapping；③ Video/Audio
是否共享 epoch；④ **1080i25↔1080p25 异构输入策略（新增）**；⑤ switch
settle 时间语义；⑥ discontinuity/segment event 语义；⑦ recover 后
timeline 处理；⑧ normalization 的 Execution Fact；⑨ Observation 如何
证明"真的 normalize 了"；⑩ 不把 Normalize 塞进 ExecutionGroup /
Supervisor / MediaBackend。

反假修复红线（终裁十二节）：禁 `max(last_program_pts + duration,
incoming_pts)` 类"PTS 不回退"假闭合——NonMonotonic→ValidMonotonic
不代表 AV sync / frame duration / segment semantics / latency /
switch boundary 正确；第一问不是"选哪个 GStreamer element"
（videorate / identity sync=true 之类后置）而是 **Authority 结构
冻结**。

### 35.6 影响矩阵（终裁十四节照录）

| 模块 | 当前状态 | 下一轮是否影响 |
| --- | --- | --- |
| Resolver | C1 PASS | ❌ 不动（C1-P1 仅登记） |
| Device Registry | CLOSED | ❌ |
| PortRegistry | CLOSED for 02-I | ❌ |
| Manifest | v5 VALID | ❌ |
| ResourceRegistry | CLOSED | ❌ |
| SessionManager | PASS | ❌ |
| ExecutionGroup | PASS | ❌ |
| SwitchIntent/Plan | PASS | ❌ |
| SwitchExecutionAdapter SPI | PASS | ❌ |
| GStreamer SwitchGraph | **当前 Timeline Gap 所在边界** | ⚠️ 可能 |
| ProgramObservation | 基本够用 | ⚠️ 可能增加 execution evidence |
| TimelineSample | 设计正确 | ⚠️ 可能增加 normalization evidence |
| PipelineHealth | 当前 PTS tracker 可继续复用 | ⚠️ |
| PipelinePlan.normalize | 已存在但未消费 | **核心入口之一** |
| Supervisor | 不应承担 Normalize | ❌ |
| MediaBackend SPI | 不改 | ❌ |
| H1 | 保持 | ❌ |
| L5 | 暂不执行 | ⏸ |
| A2-8-03～05 | 不提前侵入 | ❌ |

### 35.7 最终状态机（终裁十五节照录）与执行令

```text
A2-8-02-I
├── L0 PASS          ├── L1a PASS
├── L1b PASS         ├── L1c PASS ← C1 CLOSED
├── L1d PASS         ├── L2a PASS
├── L2b PASS         ├── L3 PASS
├── L4-SWITCH PASS
├── L4-TIMELINE      └── FAIL-PENDING-CORRECTION
├── L4 OVERALL       └── FAIL-PENDING-CORRECTION
├── L5               └── SKIPPED BY H1
└── Teardown PASS
```

- **A2-8-02-I = FAIL-PENDING-CORRECTION** 且 **A2-8 Switch Execution
  基础能力 = PASS**（实际代码 + 真实硬件证据双证并立）。
- 执行令：第二十六轮代码与账面不再修改（d123b45 保持）；下一轮
  直接进入 **A2-8-C-TIMELINE-01: Program Timeline Authority & PTS
  Continuity Design**（十项冻结前禁写 normalization 实现）；C1-P1 仅
  登记不修不阻塞。
- 本轮入口动作：C-TIMELINE-01 设计 SoT 探针已开（零代码新报告
  `docs/superpowers/reports/2026-09-04-c-timeline-01-program-timeline-
  authority-design-probe.md`）。

## 36. C-TIMELINE-01 十问终裁（跨账引用，2026-09-04，零代码）

用户对设计探针 OQ-1..12 **全部裁定**（全文落账=设计探针报告 §11；
冻结设计=`2026-09-04-c-timeline-01-design-freeze.md`，15 项+八红线
R1-R8）：

- **架构方向冻结**：Program Timeline Authority + Clock-Segment
  Timeline + Source Segment Mapping——**B 为主 + A 的执行机制 +
  C（"出口再生成"正式废止）/ D 不采用**。
- Authority=Program Execution 层 TimelineAuthority（禁
  ExecutionGroup/Supervisor/MediaBackend/单 pipeline/出口 muxer；
  不做大型独立 Engine；Domain 拥有语义、Adapter 拥有执行）。
- PTS=Source Segment Offset Mapping（SourceSegment 五字段；
  `max(last+dur, incoming)` 永久禁止；wall-clock 永久禁修 PTS）。
- V/A 共享 Program Epoch 不共享数值序列；switch_epoch≠program_epoch。
- Timeline 与格式归一化解耦（当前=Switch Boundary Adaptation +
  Format Contract 显式声明不保证无缝 format continuity；格式策略=
  独立 Program Media Format Policy）。
- settle=状态语义（TimelineTransition 期间 PTS 必须已属新 timeline）。
- Discontinuity 双层表达 + PtsState 四态（+DiscontinuityDeclared；
  declared ≠ unexpected backward）。
- Recover 本轮不实现、语义冻结（Soft/Hard 两类；Supervisor 只决定
  recover 不拥有 Timeline）。
- TimelineMapped 结构化 Fact ≠ TimelineHealthy；TimelineObservation
  专门证据面（observed_at=wall clock 禁入 program_pts；"真的完成"
  七条定义；pts>prev 永远不足）。
- 删裸 bool normalize → TimelinePolicy（本轮零代码）。
- 三时钟职权切开（Timeline Authority / AVSync Manager / Channel
  Reference Clock 不得互相越权）。
- **不触碰**：已 PASS 各层 + SwitchExecution/SessionManager/Resolver/
  PortRegistry/ResourceRegistry/Supervisor——只解决 L4-TIMELINE；
  **02-I 状态严格保持 FAIL-PENDING-CORRECTION（设计≠Gate PASS）**。
- 隔离禁顺手修：C1-P1（不重开 C1）/ converter interlace /
  PORT-IDENTITY / canonical UUID namespace。
- 执行令：设计 SoT 探针阶段正式结束；**不进入实现**；下一动作=
  Design Freeze（已形成）→ 冻结后才开 implementation change。

本主线状态机不变：A2-8-02-I = FAIL-PENDING-CORRECTION（L4-SWITCH
PASS / L4-TIMELINE FAIL-PENDING-CORRECTION / L5 SKIPPED BY H1）；
A2-8 Switch Execution 基础能力 = PASS 并立。

## 37. Design Freeze 复核通过 + Implementation Impact Map 交付（跨账引用，2026-09-04，零代码）

- 用户复核 f3158a0：**Design Freeze 有效**（核心冻结与十问终裁一致，
  本轮闭合不回裁）；**正式进入 A2-8-C-TIMELINE-01 Implementation
  Change**——纪律=先实现前代码拓扑探针/Impact Map→最小变更面冻结→
  再写代码；第 4/5/6 落点项须以真实 Rust/GStreamer API 为准不凭
  架构图猜。
- 工程状态表照录冻结（C1 CLOSED·C1-P1 隔离·L0-L3/L4-SWITCH PASS·
  L4-TIMELINE FAIL-PENDING-CORRECTION·L5 SKIPPED BY H1·02-I
  FAIL-PENDING-CORRECTION·Design FROZEN·Implementation=下一阶段·
  converter interlace/PortIdentity/UUID=独立队列）。
- **Implementation Impact Map 已交付**：
  `2026-09-04-c-timeline-01-implementation-impact-map.md`（十项逐项
  实锚+盒上 GStreamer 1.28.2 gst-inspect 实证+gstreamer-0.23.7
  crate 源码实证+OQ-IMP-1..7 待裁+最小变更面候选）。关键实证：
  input-selector 自身零时间戳改写（drop-backwards=丢帧藏证禁入）；
  **identity `single-segment` 真实存在**（"eat segments, appear as
  one segment"=方案 B 现成 primitive 候选，精确行为留 sim 实验锚定）；
  crate `event::Segment::new`/`Pad::send_event`/
  `PadProbeInfo::buffer_mut` 全真实可用；全仓 GStreamer 高层 API
  零存量。
- 全文落账=设计探针 §12；主账状态机不变。

## 38. OQ-IMP-1..7 裁决 + SIM-01 实验刀完成（跨账引用，2026-09-04）

- 用户裁决 OQ-IMP-1..7：**5 ADOPT**（IMP-1 normalize→TimelinePolicy[
  SourceNative/ProgramTimelineMapped·禁含糊 bool]/IMP-2 走现有
  Plan/materialization 链[禁新 Timeline trait/Port/SPI·ProgramEpoch
  authority 永在 ProgramExecutionRuntime]/IMP-4 TimelineEvidence=
  Adapter 装配 Runtime 独立读取[禁塞 PipelineHealth·Evidence≠Authority]/
  IMP-6 失败三结局[Preserve/NewEpoch/FailClosed·禁第四种猜测成功·R2
  绝对禁区]/IMP-7 L4-TIMELINE 升级为 Timeline Mapping Evidence 七合取
  [TimelineTransitionEvidence 结构]）+ **IMP-3/IMP-5 授权 sim 实验**。
- **SIM-01 已执行**（设计探针 §13+`2026-09-04-c-timeline-01-sim-01-
  experiment.md`，9 变体 2583 行，盒 ~/ct-sim-01 sha256 归档）：
  F1 桥按接收墙钟重定基=独立时钟域只剩相位差（真机 8-10ms 同源）·
  F2 selector 自然转发 stream-start/caps/segment(B)=免费边界标记·
  **F3 identity single-segment 只吃段不修 PTS=吞段假阳性实证**·
  **F4 控制线程 send_event(Segment) 两序均被拒**·**F5 selector 后
  BUFFER probe+Domain 声明映射=完整可行[backward=0·首帧精确落
  anchor·V/A 双平面 121/121+162/162]**·F6 pre-flip 安装无竞态+
  **set_property 后立即 readback=旧值而流已切**·F7 基线复现生产
  L4 签名。
- IMP-3/IMP-5 候选结论待用户终裁→冻结最小变更面→正式实现批次。
- 实验零架构漂移：normalize/PipelineHealth/L4/SwitchGraph 正式逻辑/
  Production graph 全未触碰；实验工程不入库。
- 主账状态机不变。

## 39. IMP-3/IMP-5 终裁 + IMP-2 实现层纠偏 + Batch 1 开工（跨账引用，第三十一轮，2026-09-04）

- 用户终裁（全文=设计探针 §14）：**IMP-3 ADOPT**（selector 后 per-plane
  EVENT+BUFFER probe；identity=吞段假阳性禁承担 proof；F4 精确表述=控制
  线程外部注入 sent=false 非主注入机制，不过度扩大）·**IMP-5 ADOPT**
  （①-⑩ 微观序冻结：anchor→声明→install→active-pad→Segment(B) event→
  下一枚 B 实际 buffer→mapping→TimelineMapped→settle→Stable；**生效边界
  ="事件确认+下一 Buffer"，active-pad readback 只能辅助**）·**IMP-2
  ADOPT WITH CORRECTION**（PipelinePlan=ingest 只承载 TimelinePolicy 声明
  清理；Program Timeline 走 ProgramExecutionRuntime→TimelineAuthority→
  ProgramTimelinePlan→Adapter——build_program_pipeline 实锚不消费
  PipelinePlan）·IMP-4 契约演进=**ProgramExecutionObservation{program,
  timeline}**（observe() 单一 observation surface，Mock/GStreamer 同构）
  ·IMP-6 三结局映射=Preserve(epoch N 保持)/NewEpoch(N→N+1)/FailClosed·
  PtsMonotonicity 升级四态(+DiscontinuityDeclared·禁洗状态)·L4 最终=
  九项合取 TimelineTransition proof·recover 本 change 不碰（A2-8-03）。
- **SIM-01 足够，无需第二轮实验；正式开 A2-8-C-TIMELINE-01 最小实现
  批次**：第一批 Domain+contract+Mock→第二批 GStreamer Adapter+L4→
  真机复跑。实现纪律："TimelineAuthority 产生'应该怎样映射'的声明；
  selector downstream Event/Buffer 产生'实际上发生了什么'的证据；两者
  在 Runtime 中闭合成 TimelineMapped。"
- 本轮 Batch 1（Domain+contract+Mock）实现落账=设计探针 §15。
- 主账状态机不变（02-I 仍 FAIL-PENDING-CORRECTION，L4-TIMELINE 复跑前
  不因实现存在而改判）。

## 40. Batch 1 复核终裁 APPROVED + Batch 2 开工令（跨账引用，第三十二轮，2026-09-04）

- 用户按 f82e625 实际代码全盘复核：**Batch 1 APPROVED**（Domain/GStreamer
  分层·ExecutionGroup 零污染·observe 机械波及无隐藏语义扩散·GStreamer
  诚实缺席，四项成立；PipelinePlan 边界**正式关闭不再回头**；SwitchExecution
  调用链零污染确认——on_switch_executed 禁成第二 switch state machine）。
- **两项 Batch 2 前置直接处理**：①BLOCKER-DOC=Freeze §3 epoch 文本统一
  为 Preserve=同世代不变/NewEpoch+1（switch_epoch/segment_id/program_epoch
  三职权分离）；②BLOCKER-IMPLEMENTATION=no_evidence 消除虚假 epoch=0
  （携带当前已知 epoch，十键形状不改 Option）。
- 三非阻塞风险：P2 i64 差值算法·P1 no_evidence（=②）·P1 段历史累积不
  覆盖（Batch 2 锁测试）。
- **Batch 2 十四步顺序锁定开工**（1-2 直接处理·3-12 主实现·13 双轨回归·
  14 真机复跑仅矩阵绿后）；禁做清单照录（Authority 不入 SwitchGraph/
  set_active 不产 epoch/readback 不判生效/identity 不用/send_event 不用/
  recover·Supervisor 不碰）。全文=设计探针 §16。
- 本轮执行落账=设计探针 §17；主账状态机不变（02-I 仍
  FAIL-PENDING-CORRECTION 直至真机 Timeline Evidence PASS）。

## 41. Batch 2 落地 + 真机复跑：Timeline 层真机 Preserve 达成（跨账引用，第三十二轮收官，2026-09-04）

- Batch 2 十四步全落地（设计探针 §17, commits e86d0e8 终裁账/59aec43
  Freeze epoch 统一/3ff66ad Batch 2）；盒矩阵 fmt/default 217/mock 381/
  **bmd+gst 237（含真实 GStreamer 全链 Preserve 实证）**/clippy×2 全绿。
- **真机 02-I 复跑（步骤 14, 22:15 CST, HEAD=3ff66ad, bin 31e294f4,
  68/68 源 sha==HEAD, 证据=盒 ~/a2-8-02i-evidence/2026-09-04-2230-batch2-
  ctimeline/）**: L1a-d/L2a/L2b/L3/Teardown **8/10 PASS（EXIT=2）**——
  **L4=switch_ok true ∧ outcome=Preserved（真 DeckLink 双输入全链: 声明
  offset 118799ns 相位级·Segment(B) 观测·首枚映射缓冲过证据校验·V/A
  双平面 Continuous·无未声明回退·epoch 保持 0）**; L4 overall FAIL 单点
  =九项合取转写 `mapped>pre` 严格大于 vs 真机零隙拼接**精确相等**（冻结
  语义=非回退 ≥）——**B 类 Gate 判据转写, 未改码待裁决**; L5 H1 跳过;
  **A2-8-01 架构硬事实真机表达（prog NonMonotonic 确定性签名）消失**
  （post-switch ValidMonotonic）。全文=设计探针 §18。
- 02-I 仍 FAIL-PENDING-CORRECTION（8/10）——性质迁移=架构缺口→验收判据
  单点转写; 正式 PASS 待 B 类修正（`>`→`>=`）+复跑（L5 首次真机注入）。
- 主账状态机：02-I FAIL-PENDING-CORRECTION（8/10 verdicts）暂记。

## 42. 第三十三轮终裁：Batch 2 APPROVED + L4 B 类单字符批准 + NewEpoch rebase P1（跨账引用，第三十三轮，2026-09-05）

- **Batch 2 ✅ APPROVED**（复核 14 项关闭——三职权分立无越权/①-⑩ 顺序/
  SIM-01 一致/Authority 声明→Adapter 冻结→实际 buffer 三段闭合/F6 生效
  边界/真机 Preserve=核心问题实际解决/双面分工/消费面/Teardown-Recover
  零污染——设计探针 §19.1）。
- **L4 `>`→`>=` 正式批准**（B 类 Gate-only 单字符; 禁趁机重写其余八项）
  ——冻结语义非回退=≥, 真机零隙拼接 equal≠backward。
- **NewEpoch SourceSegment rebase 缺陷 = P1 登记**:
  `program_timeline.rs:682-688` rebase 沿用旧 plan offset 未按新
  boundary 重算——不变量
  `new_segment.offset == new_segment.program_start_pts −
  new_segment.source_start_pts`; 回归四条（Preserve/NewEpoch/
  A→B→A history/append-only）——不阻断本轮, **C-TIMELINE-01 Final
  Close 前必修**; 不混入本次小修。
- **on_mapped_buffer 先行 DiscontinuityDeclared**（616 先于连续性判定）
  = NewEpoch 修复时锁回归（"新世代合法边界"≠"backward 洗白"）; 现阶段
  不判结构性错误。
- **令**: 修正后立即真机复跑——H1 开 L5, 完整 L5 真实证据必拿（A fail→
  B alive / recover A→bridge real flow / B fail→A alive / failure-domain
  classification）; L5 全绿 → 02-I 具备正式收口评审条件。

## 43. 第三十三轮执行：L4 真机正式 PASS；L5 首跑留证=C 类 recover 契约缺口（跨账引用，2026-09-04 22:35 CST）

- 执行链: c5c7753 终裁账 / b856a04 `>`→`>=` 单字符 / d5059e2 盒 fmt 残留;
  69/69 源 sha==HEAD; 矩阵 fmt/default 217/mock 381/bmd+gst 237/clippy×2
  全绿; bin 重建 c0efdfad; v5 当日复核; 证据=盒
  ~/a2-8-02i-evidence/2026-09-04-2340-l4fix-l5run（run.log sha 4616d680）。
- **L4 Timing/switch+timeline(A→B) 首次真机 PASS**——九项合取全绿:
  Preserved（epoch 保持 0）·映射闭合 6937849283+33301642==6971150925
  逐 ns·V/A Continuous·declared==observed==SegmentId(1)·无未声明回退·
  **mapped==pre_v 再次精确相等（零隙拼接复现→`>=` 修正被证实必要且
  充分）**·post prog ≥ mapped·Authority 行 mapped=Some。
- **L5 FAIL（首次真机执行; 历史两跑均被 H1 跳过）**: L5.1 A-fail→B-alive
  **true**（隔离半边真机成立）; L5.2 根因=**stop/recover 契约结构性冲突**
  [MediaBackend::stop=终态注销（P0-2 防句柄泄漏）vs recover 第一步
  instances.get 取 plan——controller.rs:314-331 vs 220-227; stop→recover
  序列生产上必败]; Mock stop/recover 均 no-op Ok（mock.rs:129-134）+
  L5 序列仅真机 gate 执行——Mock≠GStreamer 预警在 recover 契约面成真;
  L5.3/L5.4/Teardown session_stop=false 全为级联（Teardown 本体无独立
  缺陷: program_runtime_inactive=true·phase_released=true）。
- **分类=C 类候选（gate 序列×生产契约不匹配）待裁**, 候选方向三选一
  （L5 注入面改造 / Session 层 recover-from-plan / recover 语义归属
  A2-8-03 supervision 面）; 红线: MediaBackend::recover 不改 + stop 注销
  语义=P0-2 专裁不可反转。**未改码**。
- 02-I 仍 FAIL-PENDING-CORRECTION（8/10; 失败集迁移 {L4,L5-skip}→
  {L5, Teardown-级联}）。全文=设计探针 §20。

## 44. 第三十四轮终裁：方案 1 批准——Diagnostic Runtime Fault Injection（2026-09-05）

### 44.1 裁决（照录）

- **方案 1（L5 注入面改造）✅ 正式批准**，冻结名称
  **A2-8-02-I — Diagnostic Runtime Fault Injection**。定义边界:
  注入"运行故障"非"生命周期终止"——真实执行面停流·**PipelineHandle
  与 HEALTH_ARCS 保持登记**·随后 `MediaBackend.recover(handle)`=生产
  行为（同 handle 原 plan 重建）。**被证伪的是 L5 的故障注入方式, 非
  生产恢复链。**
- **落点=GStreamerPipelineController 第四 trait view**
  （MediaBackend / MediaTapPort / BridgeObservationPort /
  **DiagnosticFaultInjection**）——保持"一次 concrete controller 多
  trait view"（F-01 同源原则）; **禁入 MediaBackend 冻结 SPI**（五方法
  面不动）; SessionManager（生命周期 owner 不知"怎么搞坏 GStreamer"）/
  Supervisor（observe→decide 不做故障制造器）均不落。
- **方案 2（Session recover-from-plan）暂不批准**——Session 只存
  SessionInput{device_id,handle} 无 plan 持久引用, 真做必牵动
  SessionInput→重 instantiate→handle 替换→Health identity→Tap
  ownership→ProgramExecutionRuntime→ExecutionGroup→Watchdog 全链
  ="用 Session 重构修一个 Diagnostic Gate 错误"。
- **方案 3（recover 推 03）不作替代**——生产 watchdog→Supervisor.
  report_failure→Restart→lease 重校→backoff→ctrl.recover 接线实存,
  推迟会把已存在能力伪装成未来功能; 03 验证策略闭环但不能替代 02-I
  注入修正。
- **定性**: recover(handle) 本体 P0/P1 无阻断; stop→recover=非法 Gate
  生命周期组合; **Teardown 本体 PASS / 当前 Gate FAIL=L5 注入级联**
  （session stop 对已注销 handle 报 UnknownPipeline=级联后果, 不单独
  开缺陷——与 P0-2"backend.stop 失败仍继续释放"设计吻合）。
- **红线七条**: ✗改 MediaBackend::recover ✗改 MediaBackend::stop
  （终态注销=P0-2 防泄漏, 改成 paused-but-registered=架构回退）
  ✗Session 替换 handle ✗Supervisor 执行注入 ✗fault injection 入
  冻结 SPI ✗recover 推成"03 才有" ✗Timeline 代码混修 L5。
- **第一版故障形态**: ✗禁模拟 Bus Error 合成事件（Observation Fact ≠
  Synthetic Event; Health 体系 frames/PTS/last_observed/liveness/Bus
  分层不可污染）——✓作用于 A branch 实际执行面使真实媒体流停止产出。
- **02-I 收口条件=13 项全 PASS**（L0/L1a-d/L2a/L2b/L3/L4/L5.1-5.4/
  Teardown）→ 届时才进入 Final Close Review。
- NewEpoch rebase P1 维持; DiscontinuityDeclared 语义 P1 维持
  （Final Close 时 Declared boundary 与 Observed backward jump 锁成
  两个独立概念）; Mock 无证明价值确认（**禁扩展 Mock 假装真实
  controller registry**——bundle mock 分支 diagnostic=None）。

### 44.2 裁决代码断言实物核验（落账前）

| 断言 | 实锚 | 结论 |
| --- | --- | --- |
| 生产恢复链 watchdog→Supervisor→recover 实存 | watchdog.rs:212-233: report_failure→Ok(Restart)→lease 重校（:214-216 "recover 中止: lease 失效"）→ctrl.recover（:228）→report_recovered; 头注 :5-11 | **证实** |
| Session 不存 materialized plan | session.rs:193-197 SessionInput{device_id, handle} 恰两字段 | **证实** |
| recover=同 handle 原 plan 重建 | controller.rs:217-299（R33 已核）: get(plan)→save taps→remove→old.stop→build(plan, same handle)→Playing→insert→replay | **证实** |
| Mock stop/recover 无 registry 语义+bridge_stall 测试钩子实存 | mock.rs:129-134 no-op Ok; :153 bridge_stalled HashSet; :228 pub fn bridge_stall | **证实** |
| bundle 三 view 同源单构造 | registry.rs:193-199 MediaAdapterBundle 三字段; :162-186 单次 Arc::new(controller) 三 clone（F-01"禁二次构造"注释） | **证实** |

### 44.3 执行序（本轮）

1. 终裁落账（本节 + 设计探针 §21 跨账 + tasks 第三十四轮）零代码 commit;
2. 实现: contracts/diagnostic.rs 新契约面（仅诊断）+ controller 第四
   view impl（真实执行面停流不注销）+ MediaAdapterBundle 第四 view
   （同源第四 clone; mock=None）+ gate L5 5.1/5.3 stop→inject +
   registry rt 测试（注入保持 handle 可 recover 契约）;
3. 盒矩阵 + bin 重建 + 69/69 sha + 真机复跑（目标 13 项全 PASS）;
4. 证据归档 + §45 复跑账 + commit/push + 记忆同步。

## 45. 第三十四轮执行：Diagnostic Fault Injection 真机——9/10；L5.2 recover 真机成立；L5.4=B 类观测窗口候选留证（2026-09-05 00:19 CST）

### 45.1 执行纪律（§29.2 全项）

- HEAD=bb1360c（374f5c0 终裁账 + bb1360c 实现）; local git status
  clean; **70/70 源 sha==HEAD**（含新 contracts/diagnostic.rs）。
- 矩阵全绿: fmt OK / default **217** / mock **381** / bmd+gst
  **240（+3 diagnostic_rt×3 全过: 结构面[注入后 instances 保持+recover
  Ok]/行为面[self_test 真元素帧冻结→recover 复流]/fail-closed[stop 后
  注入拒收]** / clippy×2 `-D` PASS。
- bin 重建 sha `7e665e3b`; 盒时钟 **2026-09-05 00:19 CST**（跨午夜,
  header 照实记录; 当日 Discovery=跑内 L1a 2/2 production_grade）;
  ball PID 992634 存活 9h22m; gst dn0/dn1 可开有信号。
- 证据=盒 `~/a2-8-02i-evidence/2026-09-05-0020-r34-diag-inject/`
  （header.txt + run.log, sha256 `83017553`）。

### 45.2 结果（EXIT=2, **9/10——历史最高**; 失败集 {L5.4} 单项）

| Verdict | 结果 |
| --- | --- |
| L1a-d / L2a / L2b / L3 | PASS |
| **L4 switch+timeline** | **PASS（连续第三次）**——Preserved·epoch 0·映射闭合 6970279646+452126==6970731772 逐 ns·V/A Continuous·offset 452126ns 相位级 |
| **L5.1 A-fail→B-alive** | **PASS**（inputA_advancing=false·bridgeB_alive=true·program_advancing=true）——注入=真实运行故障实证 |
| **L5.2 recover-A→桥复流** | **PASS（首次真机）**——recovered=true·bridgeA_alive=true·degraded=false; recover tap 簿记重放成功（run.log 00:19:51.853 handle=1）。**33 轮 C 类缺口（stop→recover 结构性必败）经方案 1 修复真机闭环** |
| **L5.3 B-fail→A-alive** | **PASS**（bridgeB_alive=false·bridgeA_alive=true） |
| L5.4 故障域不越域 | **FAIL**——A行=None（期望 Program）; B行=Input ✓ |
| **Teardown** | **PASS**——session_stop=true·program_runtime_inactive=true·phase_released=true（33 轮级联彻底消失: handle 全程在册） |

### 45.3 L5.4 单点失败根因（首跑留证——**未改任何代码**）

- `classify_failure_domain(a_input_adv=true, a_bridge_alive=true,
  prog_adv2=true)` → **None**（program_execution.rs:186-199 全健康臂;
  None≠分类器缺陷——帧真的在到达, 不能声称停滞）。
- `program_progress_since`=帧计数增长（:160-166, video OR audio 任一）。
  5.4 采样窗 [B 注入后 ~8s, ~11s]（L5_WAIT 5s + 5.3 检查 + a1/GAP3/a2
  + q1/GAP3/q2）内 program 出口帧计数**仍在增长**。
- **机制=下游集料排空（drain runway）**: B 输入管线 Paused 后,
  intersink(B)→inter→intervideosrc(B)→selector→queue→appsink 链上
  已缓冲数据继续以消费速率流动——GStreamer 默认 queue ≈200 buffers
  ≈**8s@25fps** + inter 内部缓冲, 与 8-11s 采样窗**恰好重叠**。
  5.3 的 bridgeB_alive=false（tap 面 3s 窗口）证明源侧确已冻结;
  排空=正常 GStreamer 行为, 非实现缺陷。
- **分类=B 类候选（Gate L5.4 观测窗口与真实管线排空时间物理不匹配）**。
  非 A（硬件/信号/双卡全好）; 非 C 实现缺陷（排空=正常; 分类器语义
  正确）。候选修复待裁（本轮零改动）: ①5.4 前加长排空等待
  （drain-wait 常量或采样推后至预期 runway 后, e.g. ≥12-15s）
  ②program 停滞判定改为相对注入时刻锚定 ③（不推荐）显式读取 queue
  水位=过度工程。
- 02-I 收口清单现状: **14 项中 13 PASS, 唯 L5.4 待裁**。

### 45.4 工件（隔离队列不变）

converter interlace 断言同历跑; **pad_unlink CRITICAL ×4 本跑复现**
（teardown 时刻, 33 轮跑未现/32 轮曾现——间歇性）; Bus watch
MainContext already-acquired WARN 复现于 recover(B) 新管线建立前
（00:20:10.580, 与 handle=2 tap 重放成功同秒——无功能影响）。

### 45.5 02-I 状态

仍 FAIL-PENDING-CORRECTION（9/10）——但性质再迁移: 由"注入方式结构性
错误"变为"**L5.4 单项观测窗口物理不匹配（B 类候选）**"。方案 1
（Diagnostic Runtime Fault Injection）核心目标全部真机达成: 注入=
真实运行故障·handle 全程在册·recover=生产行为·隔离/复流/Teardown
全链成立。

## 46. 第三十六轮（repo 账第三十五轮）— L5.4 终裁: 方案②「相对故障注入时刻锚定」批准（2026-09-05; 裁决轮·零代码）

**裁决输入**: 用户 2026-09-05 全盘重审（基于 GitHub 真实 HEAD 1d0d314）,
对 §45.3 三候选的终裁。

### 46.1 裁决

1. **L5.4 定性升级**: B 类确认——不是 Domain/Runtime/Adapter/GStreamer
   recover/Session/Supervisor/Timeline/Diagnostic Injection 架构 bug,
   是 Gate 对「故障发生后何时开始判断 Program 停滞」的观测时序**无显式
   建模**。5.4 问的不是「B 故障后 Program 最终有没有停」而是「恰好选中
   的一个 3s 采样窗里有没有继续收帧」——两个问题不是同一个问题。
2. **方案①（机械加长等待 12-15s）**: 🟡 可行但不采纳为正式方案——把某
   一次硬件/帧率/queue 配置的实验结果硬编码成固定 sleep, 无稳定语义;
   现有 L5_WAIT 的语义是 fault observation settling time, 不是 pipeline
   topology drain time。
3. **方案②（相对故障注入时刻锚定）**: ✅ **正式批准**。时序语义冻结:
   `Fault t0（B inject_stall 成功时刻）→ Drain Grace → q1 → 固定 GAP →
   q2 → program_progress_since → classify_failure_domain`。grace 成为
   Gate 显式观测窗口参数（`L5_PROGRAM_DRAIN_GRACE`）——实现须
   `wait_until(fault_started_at + grace)` 而非叠加 sleep, 使前置
   L5_WAIT/桥检查/a1a2 采样变化不造成采样时刻漂移; 未来换帧率/queue/
   inter/format 最多调此单一 knob, FailureDomain/ProgramObservation/
   PipelineHealth/BridgeObservation 全不动。
4. **方案③（读取 queue 水位）**: ❌ 不批准——把 L5 Gate 引入 GStreamer
   topology/property 依赖, 且会把 FailureDomain 从封闭四词表
   {None,Input,Bridge,Program} 悄悄扩成 {…,Queue,…}。
5. **classify_failure_domain 冻结**: (true,true,true)→None=all-healthy
   语义正确, 禁为 Gate 通过把 None 改成 Program。Bridge liveness
   （last_observed 观察时钟窗口）与 Program 推进（帧计数增量）两证据
   模型不得合并成"统一 health"。

### 46.2 裁决代码主张核验（vs 真实代码 1d0d314; 本轮先行义务）

| # | 裁决主张 | 实锚 | 结论 |
| --- | --- | --- | --- |
| 1 | classify 优先序 Input>Bridge>Program, (true,true,true)→None | program_execution.rs:186-199 逐字符一致 | ✅ 证实 |
| 2 | L5.4 现流=B inject→L5_WAIT→桥检查→a1→3s→a2→q1→3s→q2（q2≈t0+11s） | dual_input.rs:785-808（L5_WAIT_SECS=5 :90·SAMPLE_GAP_SECS=3 :87） | ✅ 证实 |
| 3 | program graph=selector→queue→appsink, 双 queue 无显式容量属性=默认容量语义 | switch_graph.rs:397/441 `make_element("queue",…)` 后零容量 set_property | ✅ 证实 |
| 4 | appsink sync=false+async=false（下游消费不依赖实时播放时钟） | switch_graph.rs:399-400 / 443-444 | ✅ 证实 |
| 5 | SessionManager: Program teardown（hook）先于 Input Stop; hook 失败不截断资源释放 | session.rs:782-798（Err 仅 warn「仍继续输入停止与资源释放」）·:804 Backend.stop 在后 | ✅ 证实 |

### 46.3 执行令（边界照录）

- **只改** `gates/dual_input.rs` L5.4 观测安排（fault_started_at 锚点 +
  drain-grace 等待; q1/GAP/q2 与 classify 判据零变化）。
- **禁改**: program_execution.rs / contracts/diagnostic.rs /
  controller.rs / switch_graph.rs / session.rs / backend.rs /
  program_timeline.rs / Supervisor / MediaBackend SPI。
- 后续序: 修改→fmt→default/mock/bmd+gst/clippy→gates bin rebuild→真机
  02-I→核对 14/14→NewEpoch P1 关闭（独立刀·四回归+DiscontinuityDeclared
  两概念锁）→C-TIMELINE-01 Final Close→A2-8-05 archive。
- 隔离队列维持: pad_unlink CRITICAL ×4·Bus watch MainContext
  already-acquired WARN 不因 L5.4 收口顺手修。
- grace 初值=15s 依据: 真机 2026-09-05 00:19 实测 t0+8..11s program 仍
  推进（runway 下界 >11s）→15s=下界+~4s 余量; 若复跑仍见推进=新下界
  证据回裁, 禁无裁决自行调参。

## 47. 第三十六轮执行 — 方案②落地 + 真机复跑: 9/10 复现, L5.4 runway 新下界 >18s（留证·零后续改码）

### 47.1 交付（commit 3c0b2af, 单文件 +18 行）

- `L5_PROGRAM_DRAIN_GRACE = 15s` 常量（含依据注释: R34 实测下界 t0+11s+余量）。
- 5.3 `inject_stall(&h_b)` 后 `fault_started_at = Instant::now()` 锚点。
- 5.4 q1 前 `wait_until(fault_started_at + grace)`（`saturating_duration_since`
  剩余等待——前置 L5_WAIT/桥检查/a1a2 已耗时间自动折算, 不随流水 sleep 漂移）。
- q1/GAP/q2/classify 判据零变化; §46.3 禁改九面零触碰（diff 仅 dual_input.rs）。
- 盒矩阵: fmt --check 绿 · default 217 · mock 381 · bmd+gst 240 · clippy×2 绿;
  gates bin 重建 `baf5f895`。
- sha 清单（81 文件全列·较历史 70 文件清单扩大）: **80/81 符**, 唯一
  DIFF=Cargo.lock（盒 cargo 较新 lockfile v4 消歧格式重写+少量传递依赖
  显式化; Cargo.toml==HEAD; 历史 70 文件清单从不含 lock——**非本轮引入**,
  零语义影响, 披露不阻断）。

### 47.2 真机复跑（2026-09-05 00:47 CST; 证据盒 `~/a2-8-02i-evidence/2026-09-05-0047-r35-l54-anchor`; header 五件套+bin/manifest sha; run.log sha `23a5f860`）

EXIT=2 **9/10**（失败集仍 {L5.4} 单项）:

| Verdict | 结果 |
| --- | --- |
| L1a-d / L2a / L2b / L3 | PASS（dn0/dn1 signal=true·tap 82/81·L3 120→210 ValidMonotonic） |
| **L4 switch+timeline** | **PASS（连续第四次）**——Preserve·epoch 0·offset 130924ns·src 6969781703+130924==6969912627 逐 ns·V/A Continuous·undeclared_backward_jump=None |
| L5.1 / L5.2 / L5.3 | PASS（recover(A) tap 重放成功 handle=1, 00:47:35.386） |
| L5.4 故障域不越域 | **FAIL**——A行=None（期望 Program）; B行=Input ✓ |
| Teardown | PASS（session_stop=true·inactive=true·released=true） |

### 47.3 锚定机制执行精确性（时间线闭合证明）

tracing 时间戳重建: t0(B inject)≈00:47:42.5 → q1=t0+15.0 → q2=t0+18.0 →
recover(B) tap 重放成功 handle=2（00:48:01.147）→ Teardown（00:48:04.149,
pad_unlink ×4 同刻）。**wait_until 语义按设计精确执行**（前置消耗 ~8s
自动折算为 ~7s 剩余等待）。

### 47.4 L5.4 新证据与定性（维持 B 类·零后续改码）

- prog_adv2=true 于 [t0+15, t0+18] ⇒ **排空 runway >18s**（R34 下界 >11s
  再推高）。两跑数据与"固定大积压（任意 >18s）"一致, 亦与"积压≈冻结前
  B 生产窗"的累积假设一致——本跑 B 生产窗 ≈00:47:17→t0 ≈25.5s。
- 机制实锚: inter sink（tap）在**输入管线内** tee 挂接（controller.rs:645-666
  `attach_tap_to_instance`·sync_state_with_parent）——B Paused 冻结其
  inter sink 属实（bridgeB_alive=false 旁证）; program 侧唯一余流=inter
  shm 积压（intervideosrc(B)→selector→queue→appsink·sync=false）。积压
  容量/排空速率由 inter 插件内部语义决定, 仓库代码不可见。
- **候选待裁（本轮零后续改动）**: ①grace 15s→30s（覆盖累积假设 ~26s
  生产窗+裕量; 仍是方案②框架内单 knob 调参）②①+5.4 证据行打印 q1/q2
  program 帧计数（Gate 观测性一行·不改判据; 无论 PASS/FAIL 下次精确钉
  runway）③语义升级 eventually-stalled-with-deadline（grace 后循环采样
  至观测停滞或超 deadline; 最强语义·需新裁决）。**推荐②**。
- 02-I 收口清单: 14 项中 13 PASS 维持, 唯 L5.4。

### 47.5 工件（隔离队列维持）

converter interlace 断言 ×6（历跑 9·间歇性）; pad_unlink CRITICAL ×4
（teardown 时刻复现）; MainContext already-acquired WARN ×2（两次 recover
各一·无功能影响·recover tap 重放成功 ×2 同批）。

## 48. 第三十七轮（repo 账第三十六轮）— L5.4 终裁: 方案③「有界 eventual-stall」批准（2026-09-05; 裁决轮·零代码）

### 48.1 终裁主文（对 §47.4 三候选的再裁决）

- **①grace 15s→30s = ❌ 不作为最终收口方案**。>11s/>18s 只是"本次实验
  尚未排空的下界", 不证 runway=20/25/<30s; 15→30 若 PASS 只能得出
  "本次环境 30s 够了", 不能得出"L5.4 故障域语义已被严格证明"——那会把
  Gate 降级为经验性 timeout tuning 而非 failure-domain verification。
  **禁止再做 30s（及后续 30→60）盲调**。
- **②grace+q1/q2 帧计数 print = ❌ 已不足以解决根本问题**（观测性仍留
  在"固定 grace 后单窗采样"的脆弱假设上）。
- **③有界 eventual-stall（收敛版）= ✅ 正式批准**。两轮真机证据
  （15s grace FAIL·t0+15..18 仍推进）+ inter 积压不可仓库级观测 ⇒ 已无
  证据为"固定 grace"找到可证明常数——继续调常数反不如把判定语义升级为
  "eventually stalled, bounded by deadline" 严谨。

### 48.2 批准的 L5.4 语义: 三阶段观测器

- **Phase A 确认输入故障**: inject(B) → L5_WAIT → bridgeB=false ∧
  bridgeA=true（现有 5.3 检查即 Phase A, 结构不变）。
- **Phase B 排空期**: fault t0 → minimum drain grace（wait_until(t0+grace)
  锚定——第三十六轮真机时间线闭合已证精确执行, **机制保留**; 期间禁判
  停滞——runway 排空前采样即假阴性）。
- **Phase C 停滞确认循环**: grace 后取 q1 基线, 按采样间隔循环观测
  Program 增量——有增长 ⇒ stall_rounds 归零; 无增长 ⇒ +1; **连续 N=
  L5_PROGRAM_STALL_CONFIRM_ROUNDS 个采样窗无增长 ⇒ StalledConfirmed**
  （单窗零增量可能是调度/分发/桥抖动, 禁以单窗判停）; **now ≥ t0+
  L5_PROGRAM_STALL_DEADLINE 仍未确认 ⇒ StillAdvancingAtDeadline =
  L5.4 FAIL/TIMEOUT**（明确分类结局, 终结 grace 数值调参循环）。
- **结束原因三词表（evidence 必记"最终为什么结束", 禁静默超时）**:
  `StalledConfirmed`（→ 继续用 classify_failure_domain 判 Program 域）/
  `StillAdvancingAtDeadline`（明确 FAIL·不再猜"也许再等 20 秒"）/
  `ObservationInvalid`（帧计数簿记回退等观测面异常, 本轮禁判停滞）。
- **分层不变**: classify_failure_domain 与 FailureDomain 封闭四词表
  {None,Input,Bridge,Program} 冻结——真正变化的只是 prog_advancing
  这一观测输入的产生方式（固定单窗 → 有界循环）。Bridge liveness
  （last_observed 观察时钟）与 Program 推进（帧计数增量）两证据模型
  维持分离, 禁"bridgeB 死 ⇒ program 立即停滞"推导。
- **queue 水位读取维持 ❌**（vendor/topology-specific fact 会把故障域
  体系拉出第四个执行内部子域, 破坏封闭四词表）。

### 48.3 裁决代码主张核验（六项·全实锚）

| # | 主张 | 实锚 | 结果 |
| --- | --- | --- | --- |
| 1 | 现行 Gate=B inject→t0=Instant::now()→wait_until(t0+15s)→q1→3s→q2 | dual_input.rs:793-827（inject :793/t0 :795/grace wait :820-823/q1 :824/gap :825/q2 :826） | ✓ |
| 2 | Program Graph=intervideosrc(B)→selector→queue→appsink; queue 默认容量·appsink sync/async=false | switch_graph.rs:397·399-400·441·443-444; git log 证 3ff66ad 后未变 | ✓ |
| 3 | classify 优先序 !input→Input / !bridge→Bridge / !program→Program / else None | program_execution.rs:186-200 | ✓ |
| 4 | Teardown 顺序 Program Stop→Tap Detach→Backend.stop·hook 失败不截断 | session.rs:782-798; efc1b2a 后未变 | ✓ |
| 5 | Bridge liveness=last_observed 观察时钟·frames=历史证据分层 | program_execution.rs:131-143（alive_in_window 过滤 :139-143） | ✓ |
| 6 | 诊断注入=运行态暂停·handle/instances 保持·recover 同 handle 真实重建 | controller.rs 第四 view（R34 bb1360c 落地·后未变·diagnostic_rt ×3） | ✓ |

### 48.4 执行令与边界

- **只改 `gates/dual_input.rs`**; 允许面=三常量 + 观测循环 + evidence
  输出: `L5_PROGRAM_DRAIN_GRACE`（语义改写为 minimum drain grace, **值
  维持 15s**——不因新框架调参）+ `L5_PROGRAM_STALL_CONFIRM_ROUNDS`（=3;
  裁决建议 2 或 3, 取 3 配合既有 SAMPLE_GAP_SECS=3 ⇒ 9s 确认窗）+
  `L5_PROGRAM_STALL_DEADLINE`（=60s; 取值依据=两跑下界 >11/>18s+"积压≈
  冻结前 B 生产窗 ~25.5s"假设+grace+N×GAP+余量——**是验证期限不是
  通过常数**, 到期是分类结局非静默超时）。采样间隔复用 SAMPLE_GAP_SECS
  不新增第四 knob。
- 非确认结局（StillAdvancingAtDeadline/ObservationInvalid）保守按
  "未证停滞"进 classify（prog_advancing=true ⇒ A 行=None ⇒ L5.4 自然
  FAIL）, 结束原因在证据行区分——**判据表达式零变化**。
- **禁改九面维持**: program_execution.rs / contracts/diagnostic.rs /
  controller.rs / switch_graph.rs / session.rs / backend.rs /
  program_timeline.rs / Supervisor / MediaBackend SPI（stop/recover
  零修改）。
- 后续序: 修改→fmt→矩阵（default/mock/bmd+gst/clippy×2）→gates bin
  rebuild→真机 02-I→**核对 14/14**→NewEpoch P1 关闭（独立刀）→
  C-TIMELINE-01 Final Close→A2-8-05 archive。
- 隔离队列维持不得顺手修: pad_unlink CRITICAL ×4 / MainContext
  already-acquired WARN / converter interlace 断言; NewEpoch rebase P1
  不与本轮混修。

### 48.5 02-I 状态重定级

L0/L1a-d/L2a-b/L3 PASS·L4 PASS×4·L5.1-5.3 PASS·Teardown PASS——
**"Runtime 功能未做完"已排除, 唯一剩余=L5.4 Gate 观测语义**（本裁决即
其收口刀）。14 项中 13 PASS 维持。

## 49. 第三十七轮执行 — 方案③落地 + 真机两跑: L5.4 观测器按设计给出分类结局; L4 首次真机 NewEpoch（双 C 类回裁·零后续改码）

### 49.1 交付（commit d7d4fc6, 单文件 +68/−16）

- `L5_PROGRAM_DRAIN_GRACE` 15s（语义=Phase B 最小排空·值不动）+
  `L5_PROGRAM_STALL_CONFIRM_ROUNDS`=3 + `L5_PROGRAM_STALL_DEADLINE`=60s
  + `L5ProgramStallOutcome` 三词表 enum + Phase C 循环 + evidence
  （outcome/samples/stall_rounds/首末帧计数/@t0+x.xs）。
- 判据表达式与 classify 调用面零变化; 非确认结局保守按"未证停滞"进
  分类（A 行=None ⇒ 自然 FAIL）; §48.4 禁改九面零触碰。
- 盒矩阵: fmt --check 绿·default 217·mock 381·bmd+gst 240·clippy×2 绿;
  bin release 重建 `596a8bcc`（与 R35 同 target/release 路径）; sha 80/81
  唯 DIFF=Cargo.lock（既有分歧维持·Cargo.toml 在 80 内==HEAD）。

### 49.2 真机 run 1（09-05 05:38 CST; 证据盒 `2026-09-05-0538-r36-l54-eventual-stall`; run.log sha `1f0ea619`）: 8/10 — L4 首次 NewEpoch FAIL

- L1a-d/L2a-b/L3 PASS; Teardown PASS; **L4 FAIL**: outcome=NewEpoch
  {epoch 1·video program_start 6970509011/offset 33221397·audio offset
  104795}, switch_ok=false（prog_v 11170509011 NonMonotonic）,
  v/a=DeclaredDiscontinuity/Continuous, disc=DiscontinuityDeclared,
  undeclared_backward_jump=None; L5 被 H1 跳过（级联非独立失败）。
- **触发机制实锚（代码+数值联合裁定）**: `on_mapped_buffer`
  :618-622 连续性判据=mapped ≥ last_program_pts 否则 Unproven;
  `close_transition` :658-679 双平面 Continuous 才 Preserve, 否则
  NewEpoch（epoch+1·按观测边界 rebase·:700-704 非 Continuous 平面重标
  DeclaredDiscontinuity=合法世代边界）。本跑**视频 mapped 6970509011 <
  已观测 prog 6970509012——1ns 级差距**触发 Unproven→NewEpoch; run 2
  对照 mapped 6970555975 > 6970555974（1ns 高）→ Preserve。**声明锚与
  边界前在途末帧的 ns 级竞态决定结局**（历五跑 4 Preserve+1 NewEpoch=
  间歇性, 复跑不逐项复现=非确定性签名, 与 R26 电视抖动排除法相区分）。
- NewEpoch 记账面按设计运行（DiscontinuityDeclared·无 undeclared jump·
  段历史 append）; **P1 rebase 不变量本跑数值成立**（接受边界经
  :599-605 映射校验 ⇒ offset==program_start−source_start 自动满足,
  33221397 与 104795 双平面核验）——P1 登记 维持（回归锁缺失·Final
  Close 前必修不变）。L4 九项合取（冻结）要求 Preserve+Continuous ⇒
  FAIL 为判据忠实执行。
- **分类=C 类候选**（生产 Timeline Preserve/NewEpoch 判定语义 × Gate
  冻结判据首次在真机 NewEpoch 路径相遇）。回裁三问: (a) Preserve 声明
  是否应保证 mapped ≥ last（声明锚取整/上取 last+ε 等生产语义修正）
  (b) L4 是否接受"良构 NewEpoch"（DeclaredDiscontinuity·双平面一致·
  无 undeclared jump）——Gate 判据属验收记账模型, R26 红线禁自行降
  (c) NewEpoch P1 修复刀排期。**本轮零改码**。

### 49.3 真机 run 2（09-05 05:40 CST; 证据盒 `2026-09-05-0540-r36-l54-eventual-stall-rerun2`; run.log sha `ba2f1783`）: 9/10 — L5.4 观测器首次真机执行, StillAdvancingAtDeadline

- L1a-d/L2a-b/L3/L4 全 PASS（**L4 Preserve 连续第五次**·epoch 0·offset
  210016ns·V/A Continuous）; L5.1/5.2/5.3 PASS（recover(A) handle=1
  21:41:07.341 tap 重放成功）; Teardown PASS。
- **L5.4 FAIL=诚实分类结局**: `L5.4=StillAdvancingAtDeadline samples=15
  stall_rounds=0 prog_frames v 1261->2611 a 1681->3481 @t0+60.0s`——
  15/15 窗口全速增长（v +1350=恒 30fps·a +1800）, 停滞从未发生;
  deadline 锚定精确（@t0+60.0）。观测器语义达成: 无假阳性·无盲等·
  分类结局+全程证据, "15→30→60 盲调"被终结——**不是 grace 不够, 是
  停滞在 60s 验证期限内根本不发生**。
- **排空假设被本跑算术否定（重大）**: 时间线重建=B 输入 21:40:47 起·
  t0(B inject)≈21:41:14（recover(A)+7s settle+5.3 流程锚定）⇒ **B 预
  冻结生产窗 ≈27s**; 程序自切换(~21:40:52)起已在消费 B（选择器非活动
  pad 丢弃=reader 与 writer 同步走）⇒ shm 积压上限≈秒级; 而 t0 后
  **60s 全速推进 ⇒ 余流不可能是 B 积压排空**。唯一活源=A（a_input_adv
  =true·bridgeA=true 全程）⇒ 领先假设=**程序出口在活跃输入死亡后仍被
  另一活输入全速 feeding（L5.4 隔离前提在现拓扑真机上不成立）**; 次假
  设=inter 内部超大缓冲（与恒速 30fps wall-clock 节律不符·弱）。R34
  （>11s）/R35（>18s）的"drain runway"解释被同一定量框架追溯否定。
- **回裁四选（均需授权, 本轮零改码）**: ①观测归因探针——L5.4 期间打印
  program PTS 与 A/B 源 PTS 对齐（Gate 观测性增强·不改判据）直接定
  位余流源 ②现场 gst 检查 inter/selector 行为（独立诊断管线·需授权）
  ③deadline 加大到 >B 生产窗+裕量的判别实验（区分"晚停"vs"不停"·
  单次诊断跑）④接受"L5.4 前提在现拓扑不成立"的语义重裁（改前提或改
  判据=架构级新裁决）。**推荐①**（一次跑同时钉死归因与后续方向）。
- 分类: L5.4 观测器本身=B 类无虞（按批准语义精确执行·证据完备）; 其
  暴露的隔离前提问题=**C 类候选**（Gate 前提 × 生产拓扑行为）。

### 49.4 两跑工件（隔离队列维持）

run1: interlace ×3·pad_unlink ×4·MainContext ×0（L5 未执行）; run2:
interlace ×6·pad_unlink ×4·MainContext ×2（两次 recover 各一,
21:42:15.098 WARN "already acquired by another thread" + recover(B)
handle=2 21:42:15.116）。

### 49.5 02-I 状态（双证归档·全部回裁）

- 14 项清单: 以 run2 为准 13 PASS 唯 L5.4; 但 **L4 NewEpoch 间歇性
  （1/5 真机频次）为并列未决项**——直至 (a)(b)(c) 裁决落地, L4 存在
  相位条件性 FAIL。02-I 整体维持 FAIL-PENDING-CORRECTION（8/10+9/10
  双证）; 零后续改码; 隔离队列与 NewEpoch P1 排期不变。

## 50. 第三十七轮后即时诊断（用户拍板"截图比对"）— intervideosrc 断粮自造帧实锤: L5.4 前提失败的插件级根因（2026-09-05 06:1x CST; 零仓库代码）

### 50.1 背景与执行方式

- 用户指示以"截图比对"定余流源。执行=盒独立诊断管线 `~/vbmfp-r36`
  （gst-launch + python-gst `probe.py`·不入库·零仓库 diff·未触碰 ball
  源 PID 992634·采集卡 dn0/dn1 用后即释）。
- 方法=内容取证（截图 md5/尺寸/节奏）+ Gate 同款 `set_state(Paused)`
  注入复刻。

### 50.2 实验链与结果

| # | 实验 | 结果 |
| --- | --- | --- |
| E1 | 跨进程 writer/reader（decklink dn0/dn1→inter 通道; 读者 2fps 存图） | 读者只得 320×240 同 md5 占位帧（`ad15e287`·1827B·12s+ 不间断）→ **暴露合成行为** + inter=进程内通道实证（/dev/shm 无实体·跨进程不通） |
| E2 | 无写入器 + 强制 1080p25 caps | 12s × 24 帧全同 md5（`8fdeed7b`·1920×1080·33267B）→ **在协商 caps 上合成** |
| E3 | 进程内双链: 球源 6s 真流（num-buffers=150）后断 | 真帧 md5 逐帧变化（57745/57808B）→ 断流后 23+s 恒 md5 `84546bfe`·57327B 连续 2fps **不停** |
| E4 | **Gate 同款 set_state(Paused)**（python-gst·is-live ball 1080p25·t13 注入·t38 恢复） | 真帧→**25s pause 全程每帧 md5=`84546bfe`（与 E3 断流帧同一帧）**→恢复即回真帧（`ec1e12cf`）·recover 复流 ✓ |

### 50.3 结论（插件级 CONFIRMED）

- **intervideosrc 通道断粮时以墙上时钟在协商 caps 上无限自造恒定帧**
  ——下游帧计数无法区分真假活性。
- **L5.4 前提"活跃输入死 ⇒ program 停滞"在 inter 拓扑上结构性不可
  满足**: B 注入 Paused 后 program 图的 intervideosrc(B) 转入合成, 帧
  计数恒增（run2: 15/15 窗全速 v+1350）。
- 旁证: gate program 30fps ≠ 两真实源 25fps = 合成默认节奏候选（未
  单独定率）; **R34 ">11s"/R35 ">18s" runway 解释最终修正为合成非
  排空**; §49.3 领先假设"A 喂出口"被证伪（未据此改码）。
- 诚实信号确认: 输入侧 bridge liveness 在 B 死时正确翻 false（run2
  L5.3）——**死活信号在输入侧; program 侧帧计数在 inter 拓扑下结构性
  失真**。
- 截图: 盒 `~/vbmfp-r36/`（pa-018 真球·pa-050 合成帧·pa-078 恢复）;
  本地 `D:\SYSTEM~1\Temp\vbmfp\`（1-real-ball / 2-fabricated-paused /
  3-recovered.jpg）。

### 50.4 待裁（重塑后的三选 + 并列项）

- ①program 源机制去 inter 化（架构级——02 候选机制重开）; ②program
  活性信号换面（bridge liveness 已证诚实; 与旧裁"Bridge liveness 与
  Program 推进两证据模型不合并"构成再裁关系——"program 信号被插件
  污染"新事实下是否解禁合并=用户裁决）; ③L5.4 语义重定义。
- L4 NewEpoch 1ns 竞态间歇（run1）独立并列待裁; 合成帧的 PTS 行为
  未测（时间戳归因探针仍可选）。

## 51. 第三十八轮（repo 账第三十七轮）— 双段裁决: L5.4 重定义「故障域归因完整性」+ R36 观测器撤销（2026-09-05; 裁决轮·零代码）

### 51.1 裁决主文

- **第一段（基于 6759443 复核·后被第二段部分取代）**: L5.4 批准①归因
  探针为最高优先; **deadline 非严格有界发现**（sleep 可越 deadline 上至
  SAMPLE_GAP + stall 确认先于 deadline 检查 ⇒ 理论上 t0+61s 样本仍可
  StalledConfirmed——Final Close 前必修; 随第二段撤销观测器而 moot·
  记录在案）; L4 **Preserve-only 冻结不放宽**（❌ Preserve∨NewEpoch=
  "NewEpoch 合法"≠"L4 Timing Gate 应 PASS"两语义不混）; **P1-A 根因
  确认=连续性基准使用动态 last_program_pts**（"用比被检 buffer 更晚
  观测的 Program PTS 证明该 buffer 回退"=用未来观测值判当前边界）;
  P1-B rebase offset 不变量破坏=代码结构直证; §十一依赖图全链复核
  无 ownership 冲突（Session/Backend/SwitchAdapter/TimelineAuthority/
  Gate/Diagnostic 六面职责不动）。
- **第二段（基于 6400639 诊断·终裁）**: L5.4 四选**正式选③=重定义**。
  ①去 inter **现在不批准**（inter=带 starvation fallback 语义的媒体桥
  ≠错误架构; 未来产品要求"输入死⇒真 EOS/冻结"再开独立
  PROGRAM-BRIDGE-TRANSPORT-SEMANTICS 评审）; ②bridge_liveness 与
  program_progress 合并 **❌ 维持**（三事实分层正确——变化的只是本
  Gate 证据适用范围, 非 Observation 模型合并; 禁 bridge_dead⇒
  program_dead 与组合式伪健康值）; **③L5.4=Source-fault attribution
  integrity**: B 故障场景证明 B input 不推进 ∧ B bridge 死 ∧ A input
  推进 ∧ A bridge 活 ∧ Program 输出=**非权威证据**（不作源存活证明）
  ⇒ A 行=None ∧ B 行=Input ⇒ PASS——"真实 Input 故障不得因 Program
  graph 继续产生合成帧而被错误提升为 Program 故障"（与故障域不越域
  理念更一致）。**真 Program 域故障测试归 A2-8-03**（Input 活∧Bridge
  活∧Program 死⇒Program 的专项注入在那里设计, 禁塞进现有
  DiagnosticFaultInjection 契约面）。**删除整个 grace/deadline/
  eventual-stall 循环**（已知不适用信号不断尝试自证=技术债）。
  d7d4fc6=R36 experimental implementation 保留历史; **R37=semantic
  correction** 撤销观测器——账面链: R36 实现→真机失败→独立插件诊断
  →前提证伪→R37 语义修正, 比叠改成"看起来 PASS"干净。

### 51.2 裁决代码主张核验（五项·全实锚）

| # | 主张 | 实锚 | 结果 |
| --- | --- | --- | --- |
| 1 | Phase C deadline 非严格有界（sleep 越界+stall 先于 deadline 检查） | dual_input.rs Phase C 循环序（sleep→sample→backward→stall→deadline） | ✓（随观测器撤销 moot·记录在案） |
| 2 | ProgramObservation 已有 observed_active/input_pts/program PTS/frame counters——归因探针无需扩 SPI | contracts/switch.rs:57-73（observed_active :59·input_pts :67·program_video/audio_pts :68-69·frames :72-73） | ✓ |
| 3 | on_program_pts 持续更新 last_program_pts（=P1-A 动态基准根源） | program_timeline.rs:745-769（:768 每观测必更） | ✓ |
| 4 | L4 判据 Preserve-only（match 单臂） | dual_input.rs:685-686 `match &report.outcome { TransitionOutcome::Preserved {..} => .., }` | ✓ |
| 5 | intervideosrc 官方语义=timeout 后输出黑帧（默认 1s） | 盒 gst-inspect: `timeout: Timeout after which to start outputting black frames, Default: 1000000000` | ✓（与 §50 E1-E4 实证互证） |

### 51.3 执行令与边界

- **只改 `gates/dual_input.rs`**: ①撤销 R36 观测器（三常量
  GRACE/ROUNDS/DEADLINE + L5ProgramStallOutcome enum + 5.3 t0 锚 +
  Phase C 循环全删）; ②5.4 重写为归因完整性（B input b1/b2 采样·
  双桥活性重采样·Program 输出单窗非权威观测·classify 后判
  row_a==None ∧ row_b==Input）; ③5.3 头注释陈旧表述"program 诚实
  停滞"修正（语义已被证伪）。
- classify_failure_domain/FailureDomain 四词表**冻结零触碰**;
  5.1/5.2/5.3 语义不动; DiagnosticFaultInjection 契约不动; L4 判据
  不动。
- 后续序: 修改→fmt→矩阵（default/mock/bmd+gst/clippy×2）→gates bin
  rebuild→真机 02-I→14/14 核对。**10/10 亦不触发 Final Close**:
  C-TIMELINE-01 Final Close 暂缓（两 P1 未闭合）·A2-8-05 archive
  暂缓。
- **P1-A/P1-B=下一独立刀**（program_timeline.rs·不与本轮混 commit）:
  P1-A=第一枚映射缓冲连续性基准改**冻结 transition boundary**
  （declare 的 program_start_pts）, 之后才进运行期 last_program_pts
  单调观测——"禁用未来观测值判当前切换边界"; P1-B=rebased 恢复
  offset==program_start_pts−source_start_pts 不变量。修后第五刀=
  L4 真机复跑目标稳定 Preserve（4P+1NE→Preserve·非放宽判据放过）。

## 52. 第三十七轮执行 — R37 语义修正落地 + 真机首跑 10/10 ALL PASS（02-I 全链历史首次通过）

### 52.1 交付（commit 0d59ddb, 单文件 +37/−82）

- **撤销 R36 观测器**: 三常量（GRACE/ROUNDS/DEADLINE）+
  `L5ProgramStallOutcome` enum + 5.3 t0 锚 + Phase C 循环全删（grep
  零残留）; 5.3 陈旧注释"program 诚实停滞"修正。
- **5.4 重写为归因完整性**: B input b1/b2 采样（B 故障持续确认）+
  双桥活性重采样 + Program 输出单窗非权威观测 + classify 后
  `l5d = row_a==None ∧ row_b==Input`; 证据行如实记录 Program
  advancing 与帧计数并标注非权威。
- classify_failure_domain/FailureDomain 四词表/L4 判据/5.1-5.3/
  DiagnosticFaultInjection 契约零触碰; 盒矩阵 fmt 绿·default 217·
  mock 381·bmd+gst 240·clippy×2 绿; bin release `6e02ba57`; sha
  80/81 唯 DIFF=Cargo.lock（既有分歧维持）。

### 52.2 真机（09-05 06:38 CST; 证据盒 `2026-09-05-0638-r37-l54-attribution`; run.log sha `c1c296a6`; 全程 ~46s）

**EXIT=0 · ALL PASS（10/10）——02-I 历史上首次全链通过**:

| 层 | 结果 |
| --- | --- |
| L0/L1a-d/L2a-b/L3 | PASS（双卡 signal=true·tap 81/80·L3 120→210 ValidMonotonic） |
| L4 | PASS（**Preserve 连续第六次**·epoch 0·offset 174161ns·映射 6970673376+174161==6970847537 逐 ns·V/A Continuous） |
| L5.1/5.2/5.3 | PASS（recover 桥复流·tap 重放） |
| **L5.4（新语义首跑）** | **PASS**——`故障域归因完整=true (A行=None B行=Input; B故障归Input·A无越域归因·Program输出=非权威证据[inter合成帧语义·advancing=true v 1053->1143 a 1405->1525])`: Program 输出持续增长（合成帧）被如实记录且不再承担源存活证明——重定义语义按裁决精确执行 |
| Teardown | PASS（session_stop=true·inactive=true·released=true） |

### 52.3 02-I 状态与后续

- **14/14 达成（历史首次）**; 02-I 验收全绿。
- **Final Close 不触发**（§51.3 冻结）: C-TIMELINE-01 两 P1 未闭合
  [Final Close 暂缓]·A2-8-05 archive 暂缓。
- **下一刀=P1-A/P1-B 独立修**（program_timeline.rs·方向已批）:
  P1-A=第一枚映射缓冲连续性基准改冻结 transition boundary（根除
  1ns 相位条件性 NewEpoch——当前 6 跑 5P+1NE）; P1-B=rebase offset
  不变量恢复。修后 L4 真机复跑目标=稳定 Preserve（非放宽判据）。
- 工件（隔离队列维持）: interlace ×6·pad_unlink ×4·MainContext ×2
  （两次 recover 各一）。

## 53. 第三十九轮（repo 账第三十八轮）— P1-B 撤销确认 + P1-A 批准·实现期偏离回裁（2026-09-05; 核验+测试增强轮·生产代码零改动）

### 53.1 裁决主文

- **P1-B 正式撤销（非 bug）**: 沿调用链数学闭合——on_mapped_buffer
  :597-606 先以 `seg.map_pts(source_pts)` 校验 expected==mapped（不符
  即 fail_closed MappingMismatch）, 故进入 close_transition 的合法
  boundary 必然满足 `boundary.1−boundary.0==seg.offset` ⇒ NewEpoch
  rebased（:682-691）沿用 seg.offset 时 `offset==program_start−
  source_start` 自动成立。R33 原登记表述修正为"测试不足以证明不变量,
  非代码缺陷"。
- **P1-A 批准修复**: on_mapped_buffer 首枚连续性基准从动态
  last_program_pts 改为声明冻结边界; 回归七项; 只改
  program_timeline.rs; 修后看 L4 Preserve 稳定性（非放宽判据）。
- 全盘不动清单照录（Runtime 编排/SessionManager/Supervisor/
  MediaBackend/SPI/topology/ExecutionGroup/segment history 等十余面
  ✗ 不改）; A2-8-03=Program 域故障专项; C-TIMELINE-01 Final Close
  前置; A2-8-05 禁入。

### 53.2 裁决主张核验

| # | 主张 | 实锚 | 结果 |
| --- | --- | --- | --- |
| 1 | on_mapped_buffer 先映射校验后置 boundary | :597-606（expected≠Some(mapped)⇒fail_closed）→ :624 | ✓ |
| 2 | NewEpoch rebased 沿用 seg.offset ⇒ 不变量经 ① 自动成立 | :682-691 | ✓（**撤销成立**） |
| 3 | 现连续性基准=动态 last_program_pts | :618-622 + on_program_pts :768 持续更新 | ✓ |
| 4 | segment history append-only | 测试 :1244-1290（三段零变异）——裁决回归项 6 已存在 | ✓ |
| 5 | DiscontinuityDeclared 不洗（Preserve 路径） | 测试 :952/:966/:1315——NewEpoch 平面 pts_state 断言随 §53.4 补 | ✓ |

### 53.3 P1-A 实现期偏离发现（回裁——本轮生产代码零改动）

**字面谓词与锚设计数学不相容**:
- `sample_switch_anchors` :852-867: program_anchor=出口实测 last PTS
  **+active 分支节拍**; source_anchor=target 分支 last PTS **+target
  分支节拍**——两锚各加一个独立测量的节拍。
- **四跑实测**（R34/R35/R36r2/R37）: `program_start_pts − mapped_first
  ≡ 33,333,333ns`（恰一帧@30fps·逐跑精确）⇒ **`mapped ≥
  plan.program_start_pts` 在一切健康 Preserve 跑为假**——字面实现将把
  L4 翻成恒 NewEpoch（与"稳定 Preserve"目标相反）。
- **±1ns 竞态根源修正**: mapped = pv + (d_active − d_target) +
  (S_b − target_v), 其中 (d_active − d_target)=两独立测量节拍之差
  （±1ns 观测噪声）——**竞态非来自 last_program_pts 推进**（run1/
  run2 间 A 均未跨帧）, 而来自锚公式的双节拍噪声。
- **修复方案权衡（待裁）**: **方案 α（推荐·需扩授权至 switch_graph.rs
  一处）**=锚去节拍（program_anchor=pv·source_anchor=target_v 原样）
  ——offset 仅变 ±1ns、拼接点语义不变（仍零隙落 pv）, 且
  mapped==program_start 精确相等=裁决回归项 1 的世界;
  (S_b−target_v)∈{0,+1帧}≥0 ⇒ 竞态根除。**方案 β（Domain-only·
  不越授权）**=declare 冻结 last_program_pts 为 transition 基准（=pv）
  ·谓词 mapped ≥ 基准−slack——slack 为新魔数, 竞态被吸收非消除, 与
  回归项 1"boundary==mapped"不符。
- 依 Design Freeze"实现期偏离必回裁"本轮停手, 仅交付测试增强（§53.4）。

### 53.4 本轮交付（测试增强·program_timeline.rs 单文件·零生产代码）

- 新增 `timeline_rt_01_new_epoch_rebase_offset_invariant`: NewEpoch
  双平面断言 `offset==program_start_pts−source_start_pts`（P1-B 撤销后
  的不变量锁·防未来 rebase 改动破坏）+ NewEpoch 双平面
  pts_state==DiscontinuityDeclared（不洗白·NewEpoch 路径补位）。
- 既有覆盖维持不重复: history append-only（:1244）/Discontinuity
  Declared Preserve 路径（:952/:966）。
- 盒矩阵: fmt 绿·default 217 不变·mock 381→382（timeline 测试=
  `#[cfg(all(test, feature="mock"))]` 车道）·bmd+gst 240 不变·clippy×3
  绿（default/mock/bmd+gst）。
- 真机: 本轮无生产代码变更不复跑（P1-A 裁决后一并）。

## §54 第四十轮裁决: P1-A=方案 α「边界帧锚修正」批准——β 否决·P1-B 维持撤销（2026-09-05, 落账零代码）

### 54.1 裁决前核验（依例: 裁决内代码主张先证后录）

- ✅ `sample_switch_anchors` switch_graph.rs:852-867: 双锚各 `saturating_add`
  独立测量节拍（`program_anchor: pv+video_anchor_delta`·
  `source_anchor: target_v+target_v_delta`; audio 同构）——裁决引用的
  "未来一节拍外推"实锚无误。
- ✅ `last_delta` 全仓消费点唯一=:843 `need` 闭包（本函数）: 结构定义
  :101 + 探针写点 :503（`ns−last` 步长）——锚去节拍后该字段转为
  write-only 观察事实, 需按项目惯例（pipeline_events.rs:22 先例）
  加 `#[allow(dead_code)]`+说明, **不删字段**（裁决 §三: 留作稳定性/
  格式/帧周期证据·A2-8-03 诊断备查）。
- ✅ AnchorPair 注释原文 program_timeline.rs:61-64（"B 首帧应落位的
  Program 位置"）与实现（+节拍=下一帧预测）语义错位——裁决 §九
  注释一致性修正点名成立。
- ✅ 四跑 `program_start−mapped ≡ 33,333,333ns`（R39 盒日志已证）+
  ±1ns 竞态=双节拍测量差（R39 代数）——α 后 mapped=pv+(S_b−target_v)
  ∈{pv, pv+1帧} ≥ pv=program_start, **竞态结构性根除**。

### 54.2 裁决正文（用户原意照录要点）

- **方案 α 批准, 表述=「边界帧锚修正」**: `program_anchor=pv`·
  `source_anchor=target_v`（audio: pa/target_a）原值, 去两处
  `saturating_add(节拍)`; 修改面=switch_graph.rs 单函数+回归测试+
  必要注释（含 program_timeline.rs AnchorPair 注释语义统一——注释级
  非 production 逻辑）。健康无缝切换下 `mapped==program_start_pts`
  精确相等; `mapped<program_start` 才是真正 NewEpoch（"首枚 target
  buffer 比声明边界还早"语义变干净）。
- **方案 β 否决**: slack 魔数=在错误外推锚上人工容差——吸收非消除
  竞态, 且会吞真实 discontinuity; 违反"解决映射关系, 不是调大数字"。
- **P1-A 定义正式修订**: `sample_switch_anchors() 将已观测边界帧做
  未来一个节拍的外推, 导致 SourceSegment.program_start_pts 与实际首枚
  target buffer 不在同一个离散帧边界上`（非"last_program_pts 污染"）。
  R37 run1 的 1ns NewEpoch=错位模型上双 delta 恰好不等的表象。
- **P1-B 维持撤销**; R39 交付的 NewEpoch 不变量测试保留（加强证明
  非修 bug）。
- **不修清单**: on_mapped_buffer（`expected=seg.map_pts(source_pts)`
  +:618-622 连续性判定照旧）/ close_transition / SourceSegment::
  declare / ProgramExecutionRuntime 编排链 / ExecutionGroup /
  SessionManager/Supervisor/MediaBackend——**零改**。
- **last_delta 解耦红线**: `last_delta=observation fact`;
  `program_anchor/source_anchor=timeline declaration input`——重新
  解耦, 禁删字段禁删探针写点。
- **新回归锁**（裁决 §七例值）: active_delta=33,333,333·
  target_delta=33,333,334·pv=1,000,000,000·target_v=900,000,000 ⇒
  断言 `program_anchor==pv ∧ source_anchor==target_v`（非
  pv+delta/target_v+delta）——未来把 delta 加回锚即翻。
- **R39 七项回归处置**: 项 1-4（on_mapped_buffer 谓词改法）**被本轮
  α 取代**——α 后 `mapped∈{pv,pv+1帧}≥last_program_pts` 天然成立,
  Domain 零改即达"边界==映射帧"世界; 项 5 已交付（R39）; 项 6-7 既有
  覆盖维持（:1244-1290 append-only·:952/:966/:1315 Discontinuity
  不洗）。
- **真机七项验收重点**: ①L4 连续 Preserve ②program_start_pts==
  first mapped PTS（首证精确相等）③V/A Continuous ④无 1ns 条件性
  NewEpoch ⑤NewEpoch 测试仍过（矩阵）⑥L5.4 R37 归因语义 PASS
  ⑦Teardown PASS。

### 54.3 披露（不扩面待裁）

- **Mock 分叉**: switch_mock.rs:297-306 `sample_switch_anchors` 同为
  `+VIDEO_PTS_STEP/+AUDIO_PTS_STEP` 外推语义——本轮裁决修改面未含
  Mock, 不动; Mock 同构面（含其注释 :271-273 与 F5/F6 同构测试的
  锚语义）是否同步去 STEP=独立待裁项, 不阻塞本轮。
- Design Freeze 文本无需修改（grep 证实 Freeze 内"锚"仅现状锚用法,
  +节拍外推非 Freeze 条文而是 Batch 2 实现层选择; 本轮=对该实现
  偏离的正式回裁, 记于本账）。
- C-TIMELINE-01 Final Close 与 A2-8-05 维持暂缓（P1-A 落地+真机后
  再裁）。

## §55 第四十轮实现+真机复跑: α 落地——`program_start==first mapped` 精确相等首次真机成立·双跑 10/10（2026-09-05）

### 55.1 实现（5d61b97, switch_graph.rs + program_timeline.rs 注释）

- `sample_switch_anchors`: 双锚 `saturating_add(节拍)` 移除——
  `program_anchor=pv`·`source_anchor=target_v`（audio: pa/target_a）已观测
  边界帧原值。五道 fail-closed 门全保留; active 门降为存在性检查
  （披露: 原 pad_index 反查仅服务节拍消费, 移除后 active 分支不再参与
  锚——错误消息原文不变）。
- `last_delta`: 字段+探针写点保留, `#[allow(dead_code)]`+裁决注释
  （观察事实≠声明输入——pipeline_events.rs:22 先例）。
- `switch_graph_rt_03_anchor_declaration_excludes_branch_cadence`
  回归锁: 纯状态构造（无 PLAYING/无线程——受控节拍不被真实缓冲覆写,
  断言确定性）; 裁决例值 active_delta=33,333,333/target_delta=
  33,333,334/pv=1,000,000,000/target_v=900,000,000 ⇒ 断言四锚=原值;
  反证: delta 若回锚 video.program_anchor=1,033,333,334 即翻。
- AnchorPair 注释统一为"已观测边界帧"语义（注释级·program_timeline.rs
  生产逻辑零改）。

### 55.2 盒矩阵 + sha

fmt 零改动 · default 217 不变 · mock 382 不变 · **bmd+gst 241（+1=rt_03
通过）** · clippy×3（default/mock/bmd+gst `-D warnings`）全绿 · **sha
80/80 盒源==本地 HEAD（5d61b97）** · gates release bin `83b9b695`。

### 55.3 真机复跑 ×2（02-I v5, EXIT=0 ×2）

- **run1**（07:18:00 CST / 23:18 UTC）: **10/10 ALL PASS**。L4
  `outcome=Preserved` epoch 0·`source_pts==source_start_pts==
  6,973,066,813`·**`mapped_program_pts==program_start_pts==
  6,973,081,228`（首次精确相等——历跑 `program_start−mapped ≡
  33,333,333ns` 消失）**·offset=14,415ns·V/A Continuous·
  undeclared_backward_jump=None·pre A prog_v==mapped（零隙拼接于 pv）。
- **run2**（07:19:13, 稳定性确认跑）: **10/10 ALL PASS again**。
  `mapped==program_start==6,969,530,558`·source==6,969,476,589·
  offset=53,969ns（帧级相位随跑变化, 等式恒立）——**连续两跑 Preserve+
  精确相等 = ±1ns 条件性 NewEpoch 结构性根除的真机实证**（修正前六跑
  5P+1NE）。
- 七项验收对照（§54.2）: ①L4 连续 Preserve ✅（双跑）②program_start==
  first mapped PTS ✅（首次·双跑精确）③V/A Continuous ✅ ④无 1ns 条件性
  NewEpoch ✅（epoch 0×2）⑤NewEpoch 测试仍过 ✅（mock 382/bmd+gst 241
  含 NewEpoch 路径+不变量锁）⑥L5.4 R37 归因语义 PASS ✅（A行=None
  B行=Input×2）⑦Teardown PASS ✅（session_stop=true×2）。
- 隔离队列照旧未触碰: PortId 碰撞 WARN×2·MainContext WARN×2·interlace
  converter CRITICAL（电视 1080i25 活动期）·teardown pad_unlink ×4。
- 证据: `~/a2-8-02i-evidence/2026-09-05-r40-anchor-fix/`（run.log
  `a67ef58a`·run2.log `be80906f`·bin `83b9b695`·v5 manifest
  `7a52b498`·时钟头=盒 09-05 07:18 CST 与仓库日期一致无 clock mismatch;
  盒非 git checkout, repo HEAD 5d61b97 以 80/80 sha 清单锚定）。

### 55.4 状态

- **P1-A CLOSED**（α 实现+回归锁+真机双证）; P1-B 维持撤销（不变量
  测试在册）。02-I=10/10 EXIT=0 第二次（R37 首次·本次为锚修正后）。
- C-TIMELINE-01 Final Close 与 A2-8-05 archive: 依三十八/三十九轮
  纪律待用户裁（P1-A 已闭合为 Final Close 的前置条件之一）。
- 披露维持: switch_mock.rs +STEP 外推分叉待独立裁（不阻塞）。

## §56 第四十一轮终裁: R40 复核 PASS·C-TIMELINE-01 Final Close 批准·Mock 锚语义分叉正式立项（2026-09-05, 落账零代码）

### 56.1 核验（裁决前实锚复核——主张全部属实）

- `sample_switch_anchors`（switch_graph.rs:802-863）: 四锚=已观测边界帧
  原值（program_anchor=pv :855·source_anchor=target_v :856·audio 同构
  :859-860）; 函数内零 `saturating_add`; fail-closed 门全保留——
  GraphNotRunning(:814-818)/!started(:817-819)/TargetNotInGroup(:820-822)/
  TargetAlreadyActive(:823-825)/pad_index(:826-829)/active 存在性门
  (:832-834, 错误消息原文不变)/program V·A PTS 缺席(:837-844)/target
  V·A PTS 缺席(:846-852)。**α=锚语义修正, 非放宽 fail-closed**。
- `last_delta` 解耦属实: 字段保留(:105, `#[allow(dead_code)]` :104)+
  探针写点(:507)——生产零读取（仅注释与 rt_03 播种）。
- AnchorPair 注释统一属实(program_timeline.rs:61-66); `git diff f51d039
  5d61b97 -- program_timeline.rs` = 恰两段 doc 注释、零生产逻辑变更。
- P1-B 维持撤销属实: program_timeline.rs 生产逻辑零改;
  `timeline_rt_01_new_epoch_rebase_offset_invariant` 在册(:1121)。
- rt_03 回归锁属实(switch_graph.rs:1167-1248): 纯状态构造（无
  PLAYING/无线程）·裁决例值 active_delta=33,333,333/target_delta=
  33,333,334/pv=1,000,000,000/target_v=900,000,000 ⇒ 四锚断言=原值;
  反证注释(:1230-1231)=1,033,333,333（正确）。
- Mock 分叉属实: switch_mock.rs:299-304 仍 `+VIDEO_PTS_STEP/
  +AUDIO_PTS_STEP` 外推——真实 adapter 已切换「已观测边界帧原值」语义。
- tasks.md:6 「A2-8 NOT CLOSED until 05」在册; 项 5/6/7 全 `[ ]` 待。
- 提交链健康: f51d039(账面 3 文件)→5d61b97(恰 switch_graph.rs+
  program_timeline.rs, +107/−26)→b823e22(账面 3 文件); HEAD=b823e22=
  origin/comet/a2-8-dual-input-switch; 工作树 clean。
- **账面勘误登记**: §55.1 反证值误写 1,033,333,33**4**——正确为
  1,033,333,333（代码注释 :1231 为准; 933,333,334 方为 source_anchor
  外推值）。纯账面笔误, 零代码, 不影响任何断言/测试。

### 56.2 终裁落账

- **R40 本轮裁决 = PASS**（α 实现与裁决一致·未借删 delta 放宽任何门）。
- **C-TIMELINE-01 / P1-A Final Close = APPROVED / CLOSED**——措辞限定:
  此为 C-TIMELINE-01 专项 Close, **≠ A2-8 CLOSED**。状态板: P1-A=
  CLOSED（边界帧锚修正 α·5d61b97·rt_03 回归锁·真机双证）/ P1-B=
  REVOKED·CLOSED-AS-NON-ISSUE（不变量测试保留）/ Evidence=PASS /
  Hardware=PASS×2 / Final Close=APPROVED。Close SoT=设计探针 §33。
- **A2-8 总体 = OPEN**: 顺序维持 C-TIMELINE-01 Close → **A2-8-03** →
  A2-8-04 → A2-8-05。A2-8-05 可进入准备阶段但不得收口（03/04 未完;
  01-04 任一完成不宣布 CLOSED——tasks.md 项 7 冻结语义）。
- C-TIMELINE-01 Close ≠ A2-8-04 完成: 六路 PTS/AV continuity 验证仍待
  （02-I 验收面 ≠ 04 专项验证面）。

### 56.3 Mock 锚语义分叉正式立项（独立裁决项·非 R40 FAIL）

- **MOCK-ANCHOR-SEMANTIC-ALIGNMENT**: switch_mock.rs:299-306
  `sample_switch_anchors` 仍 +STEP 外推（「预测下一帧」语义）vs
  GStreamer 真实 adapter「已观测边界帧原值」——AnchorPair 语义分叉,
  影响 Mock 同构基线资格。
- 三不: 不回溯 R40（修改面冻结合法）·不阻塞 C-TIMELINE-01 Final
  Close·不再挂 R40 disclosure（自本轮起独立项）。
- 待裁问题: Mock 保留独立时序语义（合成流整步进, +STEP 或为合法构造）
  还是同步为观测原值语义; 若同步, F5 同构映射流与 legacy 逐字节保持面
  是否受影响须回归。
- 修复未授权——待用户独立裁决后单刀处理。

### 56.4 状态与下一刀

- 本轮零代码: 无矩阵/真机复跑（无生产变更, 沿零代码轮惯例）。
- C-TIMELINE-01=CLOSED（设计探针 §33）。02-I 现状不变: 10/10 EXIT=0
  （锚修正后双证, R37 后第二次）。
- **下一刀 = A2-8-03 failure/supervision 验证**（watchdog 四视角观测穿
  RuntimeEvent→Custody 无跨设备污染 + Supervisor 边界 recovery-only）——
  按探针先行纪律, 开工前须其 SoT Probe/裁决授权, 本轮未启动。

## §57 第四十二轮终裁: R41=PASS（含一项登记的契约注释漂移）·A2-8-03-00 SoT Probe 授权·Mock 分叉批 Probe·Mimosa 后置（2026-09-05, 裁决落账零代码）

### 57.1 核验（裁决前实锚复核——主张全部属实）

- **契约注释漂移属实**: contracts/switch.rs:91-92 仍写「program 连续性锚
  （当前出口位置+步长）与 target 源连续性锚（target 分支位置+步长）」
  ——与 R40 后真实实现（已观测边界帧原值）不一致; trait 默认面无运行
  时执行, 属**契约文档语义残留**非 P1-A 缺陷。**扩面发现**:
  switch_mock.rs:271-273 注释同源同文（"当前出口+步长"）——漂移面实为
  两处, R40 注释统一只覆盖 program_timeline.rs AnchorPair +
  switch_graph.rs。
- 既有监督能力五件属实: ①TimelineSample 三列 Input/Bridge/Program +
  program_alive（program_execution.rs:63-79, assemble :83-118）; ②
  program_progress_since/input_progress_since（:160-174, 帧计数增量
  语义）; ③BridgeHealthReport{pipeline_recovered,expected_channels,
  observed_alive_channels,bridge_degraded}+alive_in_window 当前推进性
  （:120-154, 观察时钟窗口）; ④FailureDomain{None,Input,Bridge,Program}
  + 单故障优先序 Input>Bridge>Program（:176-200）; ⑤Supervisor 纯决策
  引擎 + SupervisorAction 封闭词表 {Restart,Escalate}（supervisor.rs
  :1-19/:119-125）。
- tasks.md A2-8-02 定义含 L5 Supervision、项 5/6/7 全 `[ ]` 待——03 非
  从零开始, 定位=收敛已有 02/G/H/I supervision 观测能力。

### 57.2 终裁落账

- **R41 = PASS with one documented semantic-drift follow-up**。
- **① A2-8-03: 批准开工, 第一步必须是 SoT Probe（仅探针零代码）**;
  不重新造 watchdog/liveness/FailureDomain（代码现实已在册）; 核心查
  Observation→RuntimeEvent→Custody→Supervisor→Recovery 唯一无旁路
  无重复归因闭环; **硬红线: Supervisor 禁 switch()/begin_switch()**
  （违反即触 A2-8 冻结「Supervisor ≠ switch executor」）。Probe 十二问
  见裁决 §十二。
- **② MOCK-ANCHOR-SEMANTIC-ALIGNMENT: 立项批准, 暂不选 A/B, 批 Probe**
  ——先答三问（消费者/F5F6 测试意图/+STEP 是否入 Authority）, 第三问
  明确后才裁同步或保留。
- **③ Mimosa: 后置, 不作为 A2-8-03 前置 Gate**——正确位置=05 后 Final
  Mimosa full audit → archive/CI/merge; 例外=03 Probe 若涉新增/高风险
  路径可做局部检查, 但不得宣称完整 audit PASS。不宣称项目安全维持。
- **④ 契约注释漂移正式登记 = CONTRACT-ANCHOR-DOC-SYNC**（两处:
  contracts/switch.rs:91-92 + switch_mock.rs:271-273）: 契约注释漂移
  非 P1-A 运行缺陷; 于下一次允许的文档/契约同步轮处理, **禁留到
  A2-8 最终归档**; 届时允许修改 contracts/switch.rs（SPI 文档与已冻结
  实现语义一致, 非扩架构面）。本轮不修（裁决定位"后续修正"）。

### 57.3 状态与下一刀

- 本轮交付: A2-8-03-00 SoT Probe（另文
  `2026-09-05-a2-8-03-00-failure-supervision-sot-probe.md`, 十二问全锚
  + 三缺口 + 红线核验）+ MOCK-ANCHOR-SEMANTIC-ALIGNMENT Probe（设计
  探针 §34, 三问全答——含确定性时钟代数: mock +STEP 恒取 {0,+1} 窗口
  的 +1 臂, Preserve 成立, P1-A 失效模式在 mock 不可构造）。
- 下一刀 = 03-01（依 03-00 Probe 缺口清单裁实现面——**待用户对 Probe
  结论裁决后授权**, 本轮零代码）。

## §58 第四十三轮（R42 复核 + 03-01 第一阶段授权）

### 58.1 终裁前核验（对 R43 裁决代码声明的逐条复核, 锚 `d981728`）

- SupervisorAction 封闭 `{Restart, Escalate}`（supervisor.rs:120）✔;
  supervisor.rs grep `switch_program|begin_switch|SwitchExecution`
  **零命中**——硬红线维持（R43 §七强 PASS）。
- `observations_from_events` 存在+**零生产调用者**（custody.rs:136
  定义; 其余命中全在 #[cfg(test)] :399-:730）——G-1=真实架构/运行时
  集成缺口非文档债（R43 §五）✔。
- `custody_snapshot`/`attribute_failures` 生产调用**双零**（grep
  复核）——「能力存在≠生产监督闭环存在」（R43 §六）✔。
- classify/TimelineSample/bridge_health 生产消费面=仅 gate
  （dual_input.rs:596/:758/:827-832）——G-2 确认 ✔。
- Mock `+STEP` 旧语义仍在（switch_mock.rs:297-306 本轮重读）✔;
  contracts/switch.rs:90-93「当前出口位置+步长」漂移仍在
  （CONTRACT-ANCHOR-DOC-SYNC 两处, 本轮不修——裁决定位后续同步轮）✔。
- **GStreamer adapter observed-boundary 原值未被回退**:
  switch_graph.rs:802-863 `sample_switch_anchors`——program 锚=出口
  实测 last PTS（pipeline_events::read_health, :836-844）, target 锚=
  分支实测 last PTS（:845-852）, 零外推; R40 α 修复+真机 10/10 双证
  维持（R43 §三「R40 的真实机器 Preserve 结论没有被 R42 破坏」核验
  成立）✔。
- `d981728`=HEAD、工作树干净、零运行时代码变更（R42 提交分类=
  Documentation/Probe/Governance only——R43 §十五）✔。

### 58.2 终裁落账（R43, 四十三轮裁决全文要点）

- **R42 = PASS**（收紧表述: 「R42 PASS — Probe/ledger round
  completed; no runtime implementation authorized or introduced.
  A2-8-03-00 is complete. A2-8-03 implementation remains OPEN」）。
  状态图维持: C-TIMELINE-01=CLOSED·A2-8-02=完成·03-00=COMPLETE·
  **03-01=未实现**·04/05=OPEN·A2-8=OPEN。
- **① 实施序正式冻结（依赖 DAG, 禁四项并列同时开工）**:
  **G-1 → G-2 → Failure Attribution → Recovery Contract → G-3 →
  G-4**（Supervisor 全程纯决策）; 03-01 不能直接从 G-1「补调用」
  开始而不先裁身份/生产消费链。
- **② 授权 A2-8-03-01 第一阶段: G-1 Identity/Custody + G-2 Runtime
  Consumption 设计/实现探针; 暂不授权 Program-domain recovery
  implementation（G-3）**——避免把 G-3 做成「看似完整、实际上没有
  可靠事实来源的 supervision 系统」。
- **③ CONTRACT-ANCHOR-DOC-SYNC 与 Mock A/B 合并同一文档/契约同步轮**
  一次性统一（Contract→GStreamer[已合规零改]→Mock→F5/F6 测试意图）,
  禁半同步中间态; 用户**倾向 B**（observed-boundary 语义镜像）但
  非现在改、非本轮——执行授权留待该同步轮。
- **④ 五误区禁令（R43 §十七）**: 禁说「Supervisor 已有⇒03 supervision
  完成」/「FailureDomain::Program 存在⇒Program recovery 已完成」/
  「Custody mapper 存在⇒事件链已闭合」/「Mock 与 GStreamer 不同⇒
  一定是 bug」/「Mimosa 未完整扫描但没发现问题⇒安全」。
- **⑤ Mimosa 后置维持**（05 后 Final full audit; 本轮零代码无矩阵/
  真机复跑——R40 runtime 证据继续为 baseline, 两类证据不混）。
- tasks 项 5 保持未勾（03-00 完成≠03 完成; 03-01/验证/证据全开放）。

### 58.3 本轮交付（零代码）

- **A2-8-03-01 Phase-1 设计/实现探针**（新文
  `2026-09-05-a2-8-03-01-g1-g2-custody-consumption-design-probe.md`）:
  03-00 之后展开新事实——**internal 平面多消费者竞争 drain**
  （ingest watchdog 每输入一个 + group watchdog 共享同一
  world.internal_log 破坏性 drain, watchdog.rs:192/:537·bin:39/:479/
  :529——「单一 drain 点」假设不成立=G-1 拓扑硬约束）; 身份丢失
  机制根因（bus Error 身份在 watchdog 已知、ingest→mapper 边界归零,
  watchdog.rs:174-181·events.rs:164-189）; custody 双零生产调用
  复核; 组 watchdog 无 MediaTapPort 依赖（G-2 接线=组合根+签名面）;
  R43 目标链逐边映射表; **OQ-G1-1..7 + OQ-G2-1..6 共十三问待裁**
  （身份语义先行: PipelineFault.pipeline 设备身份三选项; 消费拓扑
  四选项含 FanoutSink 第三平面=D3 定稿修订裁面; 发射面三选项含
  「supervisor 唯一事件出口」释法; 快照调用点+Observation SoT 双通道
  职能; 丢弃×custody 事实性; G-2 子集/挂点/输出走向[GroupAction
  T10/T12 扩词=裁面]/sim 模式面）。
- 设计探针 §35（Mock 倾向 B 登记）; tasks.md 项 5 R43 段。

### 58.4 状态与下一刀

- 02-I 现状不变（10/10 EXIT=0 双证）; C-TIMELINE-01=CLOSED。
- **下一刀 = 用户裁决 OQ-G1/OQ-G2 十三问 → 03-01 实现批次授权**
  （依冻结序 G-1 先行; 实现轮须矩阵: fmt→default→mock→bmd+gst→
  clippy→bin 盒序）。CONTRACT-ANCHOR-DOC-SYNC+Mock B 同步轮另行排期
  （R43 §十一: 一次统一, 不与 03-01 混轮）。

## 59. 第四十四轮（R44, 2026-09-05: 裁决落账 + 03-01-A/B/C 实现 + 盒矩阵全绿）

### 59.1 裁决前核验（落账前置义务）

- 基线: `git rev-parse HEAD`=2b5835c·worktree clean·`git diff --stat
  d981728..2b5835c` = 恰 4 文档（tasks.md/主账/设计探针/03-01 探针）
  +412 行零 runtime——**R43 零代码轮成立**, R44 十六行代码声明在 2b5835c
  全部延续成立（代码状态与 R43 核验时点 d981728 逐字节一致）。
- 抽验复核（本轮实测）: SupervisorAction 仍恰 {Restart, Escalate}
  （supervisor.rs:120-125·全仓 grep switch 零命中）; fault_trigger
  归属/回声谓词原样（:45-58）; ingest 旧签名无身份（:161-166 旧态）;
  mapper 三类故障恒 nil（events.rs 旧态）; FanoutSink 同序双写
  （emit :328-333）+internal 有界两级丢弃原样; custody 桥规则原样
  （custody.rs:136-158）; 组 watchdog drain 只喂 health::reduce
  （watchdog.rs:537-539 旧态）; `observations_from_events`/
  `attribute_failures`/`custody_snapshot` 生产调用者零（grep 实测）。

### 59.2 R44 终裁落账

- **R43 = PASS（Probe/架构裁决轮）**; G-1/G-2 缺口确认为真实集成缺口
  非推测。
- **① 消费拓扑裁定**: 禁 custody 挂第三 drain（多消费者抢事件否决）——
  「一个事实消费点完成事件取得, 然后非破坏性 fan-out / fold」。
- **② 实施顺序收紧**: 03-01-A→B→C→D→E→F→G; 03-02 Recovery Contract;
  03-03 Program 域监督; 03-04 Mock recover/终验（十三问不逐问重裁,
  P0 两问由本轮裁掉, P1 随实现落地, P2=G-2, G-3/G-4 暂缓）。
- **③ 授权 = 03-01-A/B/C + 矩阵 fmt→default→mock→bmd+gst→clippy→binary
  gate**; G-2 接线依真实测试结果后裁。**不碰**: G-3·Mock A/B·Mimosa·
  Timeline·Supervisor switch 边界。
- **④ 新正式红线**: G-2 禁改 ProgramExecutionRuntime 切换逻辑/禁塞
  supervision 入 switch_program（execution authority≠failure decision
  authority）; 归因禁放宽（identity absent→NO ATTRIBUTION fail-closed）;
  EventLog 契约禁绕开; watchdog 重接线为周期驱动器。
- **⑤ 五误区禁令维持**; Mimosa 后置 05 后维持（不宣称安全）。

### 59.3 03-01-A/B/C 实现（运行时代码, 9 文件）

- **A 身份契约**: `Supervisor::ingest(source, device, observation)`
  签名扩展——生产唯一调用点 watchdog.rs:177 携 `device_uuid`; mapper
  重构为单一归类 `map_with_identity` + 携身份入口
  `map_upstream_for_device`（三类故障事件身份=device canonical 身份;
  trait 面 identity-less 兜底维持 nil=未归属, custody 桥拒收语义不变）。
  **词面零变化**: RuntimeEvent/EventSource/FanoutSink/RuntimeEventLog/
  EventSeverity 零触碰。
- **B 单一 drain 边界**: 新 `event_intake.rs`（+277 行含测试）——
  `InternalEventIntake{log, custody}` 持 internal log 句柄, `consume()`
  =**唯一生产 drain 实现**（生产 internal 平面 drain 全仓普查: 仅
  event_intake.rs:60 一处; transport.rs:232=projection 面 D3 既定;
  gates/session_lifecycle.rs:564=gate 诊断 E7 残留断言——披露维持）;
  生产 watchdog 线程不再直接持 internal log（spawn 参数 `internal_log`
  →`intake`, 类型级排他）; bootstrap 构造共享单实例（BS-01, 字段
  `event_intake`）; bin 三处 spawn + gates bin/session_lifecycle 接线
  同步。
- **C custody 生产接线**: `consume()` 边界内对每 drained 批次调
  `observations_from_events`（A2-7 冻结桥规则原样）**全量恰一次累积**
  （任意驱动器先 drain 都不丢 custody 事实——G-1 拓扑硬约束闭合）;
  `observations()` 只读暴露; 零新增生产消费者·零 advance·快照调用点
  不加（OQ-G1-5 留 D/E/F）。
- **测试 +6（全绿）**: event_intake ×4（唯一 drain/跨驱动器恰一次累积/
  生产链身份→custody→归因 FAILED 全闭环·fault_trigger 精度·投影面
  D3 不破坏/本地 fold 分区语义披露锁）+ events mapper 携身份 vs
  identity-less ×1 + supervisor 身份化故障只触归属设备 ×1（nil 保守
  匹配维持）; supervisor 既有 ingest 测试升级身份断言。
- **行为变化披露（两处, 均为授权方向内的语义修正）**: ①生产上游故障
  事件由恒 nil 改为携带真实设备身份——custody 归因面可达, fault_trigger
  由"nil 误触所有设备"收敛为"只触归属设备"（nil 未归属路径的保守全匹配
  维持既有语义零变化）; ②组/ingest watchdog 本地 fold 仍按消费分区
  （既有行为零变化——R43 §1.2 事实; 全量统一 fold 属 D/E/F 裁面,
  intake_04 测试锁死该披露）。

### 59.4 盒矩阵（终态实测, 盒源 9/9 sha8 与本地一致）

- fmt --check 绿·**default 223**[217+6]·**mock 388**[382+6, §31 起
  baseline 382——主账 :2918 记录]·**bmd+gst 247**[241+6]·clippy ×2
  （default+bmd,gst 均 -D warnings exit 0）·bin: media-agent
  `d38af05f`·media-agent-gates `e73281d5`。
- 修复过程: fmt 两轮（长断言换行, 盒 cargo fmt 应用后回传本地=格式 SoT）;
  clippy 修 supervisor.rs 未用 trait import（ingest 改走 inherent 方法）。
- **状态**: 03-01 A/B/C 落地——custody「双零生产调用」闭合其一（事件
  事实流已入 custody 累积面）; 归因/快照生产消费仍零（D/E/F 待授权）。
- **下一刀 = R44 §5**: 依真实测试结果裁决 G-2 runtime consumption 具体
  接线（03-01-D/E/F）→G tests+真机; CONTRACT-ANCHOR-DOC-SYNC+Mock B
  同步轮另行排期不混轮。

### 60.1 落账前独立核验（b6b9a3f, 用户 R44 复核逐条实文对照）

- git: HEAD=b6b9a3f==远端, working tree clean; a787974 9 文件 +410/−27
  与账面一致。用户 16 行复核表逐条实文确认: custody.rs:61-68 注释原文
  确证 `PipelineFault.pipeline` 当前承载 device identity（legacy 双语义
  =V0.3 Event Contract 债, 本轮不动字段——用户裁定正确）; bootstrap.rs:36
  `internal_log` 仍 pub——**"类型级排他"表述降级为"组合根接线级唯一 drain
  ownership"成立**（代码注释 bootstrap.rs:39/event_intake.rs:12 已同步
  纠偏措辞; 强类型封锁留后续治理轮, 不为形式重构已过 Gate 接线）;
  Supervisor 无 switch 入口维持。
- **E 前提纠偏（本轮最重要事实披露）**: 用户 §11-E 前提"仓库不存在
  FailureDomain runtime contract"与实文**不符**——`program_execution.rs:179`
  已有 `pub enum FailureDomain{None,Input,Bridge,Program}` +
  `classify_failure_domain`(:186, 三列进度观测 input/bridge/program,
  单故障优先序 Input>Bridge>Program, gh_rt_01 矩阵测试); 消费现状=
  dual_input.rs L5d(:827-838) gate-only——**恰是 03-00 探针 G-2 缺口
  原文**("分类器三列观测 gate-only 无 runtime 常驻消费");
  master_join.rs:112/api_boundary.rs:406 早已预留"红后 Runtime
  classify_failure_domain"消费面(§8.10)。依用户自身红线（"必须从现有
  真实 evidence contract 向前推"）**禁新造第二同名类型**
  （PipelineFault.pipeline 同名双语义教训）→ E 刀=把现有分类器生产化。
  custody `FailureScope::SharedPipeline`（事件身份证据）与 FailureDomain
  （进度证据）为**两族互补证据, 禁融合**。

### 60.2 G-2-00 契约/组合预检 + D/E/F 落地（a787974 后续实现提交）

- **预检**: report_failure 生产调用者恰 2（watchdog.rs:217 ingest tick /
  :549 group tick; event_projection.rs:277+intake_03 均测试代码）——签名
  扩展波及面有界; 桥 liveness=`BridgeObservationPort` trait 方法
  （controller.rs:814 实现）, 组 watchdog 今日无此依赖=OQ-G2-2 实锚;
  **装配点现成**: `MediaAdapterBundle.bridge_observation` 第三 trait view
  （registry.rs:206, A2-8-02-G/H 同源 controller）——bin composition
  元组 :290-293 原样丢弃该 view, 扩 4 元透传零新构造。
- **D（custody 归因生产消费）**: `watchdog::assemble_decision_input`
  纯函数装配点——ingest tick 同临界区 consume+归因（attribute_failures
  首个生产调用者; 空 custody 证据→None=absence≠evidence; 证据在场身份
  不匹配→零归因结果≠无证据, identity correlation 零污染）。
- **E（FailureDomain 生产消费）**: 组 watchdog tick 三列生产喂入——
  ①input 列=fold per_input advancing ②桥列=bundle bridge_observation view
  `bridge_liveness(handle, FAILURE_DOMAIN_LIVENESS_WINDOW_MS=3000)`
  （与 gate L5 LIVENESS_WINDOW_MS 同值同义, 常量落 program_execution.rs
  ——gates→runtime 依赖禁反转）按 tap_channel 取本设备行 ③program 列=
  program_progress_since 两采样帧计数（首采样前不分类）。
- **F（Supervisor 决策输入面）**: `report_failure(+domain, +attributed)`
  ——按值携带零 Custody 所有权（用户拓扑: Policy input→Supervisor;
  **Custody→Supervisor→switch 禁式不可构造维持**）; Status 逐决策**替换**
  记录（Some/None 均如实——absence≠evidence 不累积）;
  `last_decision_domain/last_decision_attribution` 只读访问器;
  **决策判定逻辑零变化**: attempts/circuit/Restart/Escalate 词表预算全
  冻结, 本轮无分支消费——域→恢复策略选择=03-02 Recovery Contract 消费面。

### 60.3 披露（五项）

1. **组 tick 桥列缺席→不分类 ≠ gate L5d 缺席→false**: gate 在 L2b（tap
   在场已验）前提下 `is_some_and(alive)=false` 记账; 运行时无此前提, 按
   media_tap.rs:109 `absence≠evidence` 契约不分类（None=无分类证据）。
   喂入口径差异如实记档, 分类器本身零改动。
2. **F 无分支消费**: 决策输入本轮只记录不改变判定（用户 §11-F 授权语义
   =接收决策输入; 分支消费属 03-02）——测试锁"有证据与无证据同判"。
3. **组 watchdog tick 接线真机活体证据缺**: `spawn_execution_group_watchdog`
   唯一 spawn 点=生产 bin（bin:479）; 本轮活体=编译级（bmd+gst 全绿）+
   同一分类器 gate 侧 L5d 真机复核通过; 组 tick 活体执行留 A2-8-04 生产
   bin 验证轮。
4. **hw 门控闭包作用域 bug 盒上抓到**: bridge_alive 闭包链 `p` 越域
   （E0425）——该段 cfg(bmd+gst) 专属, default/mock 不编译（矩阵分层
   价值实证）; 盒上修复复跑全绿。
5. **基线校准**: R40 真机 dual_input 已 10/10（L5.4 经 R36/R37 闭环;
   记忆线"9/10 L5.4 FAIL"过期作废）→ 本轮门槛=10/10。

### 60.4 盒矩阵 + 真机（终态实测, 盒源 7/7 sha8 与本地一致）

- fmt 绿·**default 224**[223+1]·**mock 390**[388+2]·**bmd+gst 248**
  [247+1]·clippy ×2（default+bmd,gst 均 -D warnings exit 0）·双 bin
  构建成功; 变更 7 文件 sha8 全对（38d05f68/40e3fa87/0eeb4e93/4ff9e9e9/
  061f2b9b/cd631c0e/24e07bb5）。
- **真机（证据盒 ~/a2-8-02i-evidence/2026-09-05-r45-g2-decision-input/,
  盒钟 UTC 02:02=CST 10:02 无失配, v5 manifest sha 7a52b498 复用）**:
  ①VBMF_SESSION_LIFECYCLE **ALL PASS EXIT=0**——ingest watchdog 新决策
  输入接线真机活体（custody 归因逐 tick 生产计算+E7 internal residue
  既有语义维持）; ②VBMF_A2_8_DUAL_INPUT **ALL PASS 10/10 EXIT=0**——
  L0→L5+Teardown 全链零回归（L1a 2/2 production_grade·L1c 双信号·L2a
  port_id 精确·L2b 双 tap 82 帧·L3 120→210·L4 Preserved{epoch 0,
  offset 278599ns, V/A Continuous, DiscontinuityDeclared}·L5 归因完整
  "A行=None B行=Input"·Teardown 停止链）。
- **状态**: G-2 stage-1（D/E/F）落地——custody 归因+FailureDomain 生产
  消费+Supervisor 决策输入面三缺口闭合; **G-2 PASS 不自宣**（真机已跑
  但按纪律待用户复核; 组 tick 活体见披露 3）。
- **下一刀（待裁）**: 03-02 Recovery Contract（决策输入记录面的消费——
  域→恢复策略选择; R43/R44 冻结序下一环）; CONTRACT-ANCHOR-DOC-SYNC+
  Mock B 同步轮仍另行排期不混轮。

## §61 第四十六轮（R45 复核裁决 + G-2-G 真机活体 + 03-02 设计提案）

### 61.1 R45 复核裁决登记（用户独立实文核验后, 落账前已对分支头复核）

- **R45=PASS 限定为 G-2 Stage-1 PASS, G-2 不关闭**; 状态表: 03-01-A..F
  COMPLETE·G-2-00 COMPLETE·G-2-G PARTIAL·G-2 Final OPEN·03-02 NOT
  STARTED·A2-8-04 OPEN。
- **开发线纪律**: 一切后续复核/提交以 `comet/a2-8-dual-input-switch`@
  ff864d2 为准（已核: 本地 HEAD=远端=ff864d2; **master=7745968 旧头,
  禁混线**）。
- 用户确认要点: E 前提纠偏被采信（FailureDomain 既有复用正确, 未新造
  第二同名类型）; 单故障优先序分类器语义锁死重申（禁多故障多维归因）;
  group custody batch 为 group-wide + 逐 action device-scoped attribution
  双防线（identity correlation）边界在 03-02 必须沿用; tasks.md 单元不纯
  =engineering hygiene 非架构缺陷, 保留披露不重写历史。

### 61.2 R46 执行: 组 watchdog 真机活体（G-2-G Final 证据）

- **活体观测行使能披露**: 组 watchdog 健康路径原为静默（仅 spawn/异常
  日志）——增加两处**仅诊断输出、零决策逻辑**观测行（watchdog.rs: 周期
  活体观测行每 20 tick≈10s + 决策输入指纹行于故障动作路径; 分类经同一
  `assemble_decision_input` 纯函数, 结果不入任何状态——决策输入仍只在
  故障动作路径装配）。矩阵全绿后开跑。
- **真机活体跑**: 生产 `media-agent` bin（MEDIA_AGENT_MODE=diagnostic +
  v5 manifest sha 7a52b498 + VBMF_DIAG_INPUTS=2; VBMF_OUTPUT_* 全缺省 ⇒
  fail-soft 纯分析零外推流; 盒钟无失配; 无 stray 进程）9.5min（timeout
  SIGTERM 终止=预期, 无优雅停路径）。
- **活体证据（强阳性）**: "Execution Group 就绪... MultiInputWatchdog
  四观测面启动"（graph_handle=3, initial_active=4fa33dcb）; **线程连续
  tick 0→1120（57 条活体观测行）**; 双设备三列实时 observed=true/
  advancing=true/bridge=Some(true) + program_advancing Some(true);
  **分类器真机活体: tick 0 domain=None（首采样无证据诚实缺席）→ tick≥20
  domain=Some(None)（三列齐备全健康臂——`FailureDomain::None` 变体在
  生产线程真机产出）**。用户 §13 缺口"group watchdog 真机 ❌"的线程/
  三列/分类器三面已闭合。
- **活体缺口（如实）**: 窗口内零自然故障（TV 未抖动; ball 源 992634
  勿杀; 生产注入面=gate-only R35 红线禁入）→ 故障动作路径决策输入活体
  指纹=0（custody_evidence 恒 0, 无 ReportInputFailure）。该路径现有
  证据=纯函数测试+gate L5d 真实故障注入分类真机复核+本轮线程/三列/
  分类器活体。**OQ-R4 待裁**: 证据组合是否足以关闭 G-2-G, 还是要求
  自然故障长窗复跑。
- 证据盒: ~/a2-8-02i-evidence/2026-09-05-r46-g2g-group-watchdog-live/
  （header 五件套 + production-run.log; bin media-agent ab361801）。
- 矩阵（观测行使能后复跑）: fmt 绿·default 224·mock 390·bmd+gst 248·
  clippy×2 绿·双 bin 构建（计数零变化——纯诊断输出无新测试）。

### 61.3 03-02 Recovery Contract 设计冻结提案（零实现, 新探针文档）

- 交付 `2026-09-05-a2-8-03-02-recovery-contract-design-probe.md`:
  As-Is 实锚（决策/执行/证据/冻结四面）+ 五面契约提案（F-1 domain→
  strategy: **提案不新造 Strategy 词表**; F-2 attribution→target 双路→
  own handle 冻结; F-3 RestartPolicy 零变化; F-4 fail-closed None→现状;
  F-5 **消费点=执行域**（watchdog 读 last_decision_* ——Supervisor 判定
  /词表零变化, R44 §7 红线一致））+ **OQ-R1..R5 待裁**。
- 关键提案默认: **OQ-R1 全维持现状（03-02 记账收口零代码候选）**——
  Bridge/Program 域无执行面恢复能力, 禁凭空造; 若裁执行分支, 最小影响
  面=watchdog Restart 分支读 last_decision_domain 分支（§6 预估, 未授权）。

### 61.4 状态与下一刀

- G-2 Stage-1=PASS（维持）; **G-2-G Final=待用户对 OQ-R4 裁定**; 03-02=
  设计提案已交付待冻结（OQ-R1..R5）; A2-8-04 OPEN（组 tick 活体已并入
  本轮证据, 真实故障路径活体与 Timeline/AV continuity 专项仍待）。
- 提交: 观测行代码（watchdog.rs 单文件）+账单元（本 §+03-01 §11+03-02
  新文档+tasks R46 段）分单元提交推送。

## §62 第四十七轮（R46 复核裁决 + OQ-R4 关闭 + G-2 Final CLOSE + 03-02 命名纠偏; 零运行时代码）

### 62.1 R46 复核裁决登记（用户三层复核: 裁决原文→GitHub 提交/分支→证据链语义）

- **R46=PASS — G-2-G LIVE EVIDENCE**; 12 行逐条复核: 1-6/8-9/11-12 全
  PASS; 7 有条件通过（LIVE Gate PASS / Production Failure E2E 未触发）;
  10 = DESIGN DELIVERED / FREEZE NOT YET COMPLETE。
- 分支与提交核验属实: comet/a2-8-dual-input-switch=f5eedcb, master=
  7745968 旧头, 无混线; a8b87b1（纯代码, 观测行）/f5eedcb（账 4 文件）
  单元分离实际修正; R45 d6c6a45 卷入维持披露不重写。
- 观测代码边界确认（用户 §四: 非基本符合而是**架构边界正确**）: 诊断
  观测→assemble_decision_input→仅 logging, 零状态零决策路径; 故障动作
  路径保持独立——未污染 G-2 边界。

### 62.2 OQ-R4 正式裁决（用户 §六/§七）: **组合证据关闭, 不要求自然故障长窗**

- 裁决理由: 长窗复跑=把"验证软件链路"变成"等待电视信号自然故障"——
  概率性证据非确定性软件证据; 现有三层证据已覆盖链路/分类器/真实故障
  分类本身: Layer1 生产线程真实活体（tick 0→1120·57 行·双设备三列）+
  Layer2 生产线程真实分类器（同 assemble_decision_input, 非测试 harness,
  FailureDomain::None 真机产出）+ Layer3 真实故障分类（gate L5d 真机
  注入, 故障域归因完整=true）。
- **Gate 分层模型（正式记账）**: G-2-G-LIVE=PASS / G-2-G-CLASSIFY=PASS /
  G-2-G-FAULT=NOT OBSERVED（不阻塞——缺的是"production watchdog+真实
  故障同时发生"而非链路/分类器正确性）/ G-2-G-E2E（recovery action）=
  属后续 03-02 Recovery 与 A2-8-04 范围。**LIVE 与 E2E 两 Gate 禁混**。

### 62.3 G-2 Final=CLOSED（R47）; 03-01-G=COMPLETE

- 用户状态表收敛: 03-01-A..G 全 COMPLETE; G-2-00/G-2 Stage-1/G-2-G LIVE
  COMPLETE; G-2-G FAULT-ACTION E2E NOT OBSERVED（OQ-R4 已裁=接受, 不
  阻塞）; **G-2 Final=CLOSED**。R40..R46 全 PASS 链保持。
- 边界面保持: Supervisor RECOVERY ONLY / Switch boundary / Timeline /
  Mock A/B DEFERRED / Mimosa DEFERRED。

### 62.4 03-02 命名纠偏（用户 §八, 立即修正）

- "设计冻结提案"表述不严谨——OQ-R1..R5 未裁决前**不是 Frozen Contract**;
  准确名称=**设计探针/冻结提案（Design Probe / Freeze Proposal）**,
  状态=DESIGN DELIVERED / FREEZE NOT YET COMPLETE。03-02 文档标题/状态
  行已就地修正+§7 修正记录; 本账及下游账自本节起统一用词。
- hygiene（用户 §九, defer）: "五面 F-1..F-6"计数修正为**六面**——
  不单独制造提交, 于 03-02 正式冻结时顺手统一。

### 62.5 下一刀（用户 §十三路线图, 纪律重申）

- **禁先写 Recovery 代码**——先 OQ-R1..R5 用户裁决 → Recovery Contract
  Freeze → 最小实现（若裁执行分支）→ matrix+真机 → A2-8-04。
- 本轮=零运行时代码零矩阵（纯账面）; 提交=账单元（主账 §62+03-01 §12+
  03-02 文档纠偏+tasks R47 段）; 基线=f5eedcb..（本节提交后新头）。

## §63 第四十八轮（R47 复核裁决登记 + 状态语言规则永久锁 + OQ-R1..R5 冻结包; 零运行时代码）

### 63.1 R47 复核裁决登记（用户四层复核: 用户报告→GitHub 实际提交→当前分支状态→语义一致性）

- **总体: 🟢 R47 PASS——"实际落地正确, 且没有发现运行时代码越界"; 结论比
  R47 报告再收紧一点**。
- 13 项逐条: 分支纪律 / R46 状态继承 / 观测边界 / OQ-R4 / Gate 分层 /
  G-2 Final / 03-01-A..G / 03-02 命名 / Recovery 禁先实现 / 提交单元
  纯度 / Mimosa DEFERRED 全 ✅; F-1..F-6 六面计数 🟡 DEFER（维持, 随
  03-02 正式冻结顺手修）; **memory sync 🟡=报告自证通过但 GitHub 不可
  独立验证**（外部执行环境动作, 不构成 R47 阻塞——按"自证项"登记接受）。
- 本轮独立核验（R48 执行前置, 实文/git 为准）: local=remote=6f5735e,
  master=7745968, 树净; 6f5735e=4 文件 +108/−3 ledger-only（tasks/主账/
  03-01/03-02）, commit message 载明基线 f5eedcb/7745968+零运行时代码
  声明; 03-02 doc 实文=纠偏后标题+状态行+§7 修正记录（会话恢复缓存中的
  旧标题为陈旧快照, 实文无问题）。

### 63.2 状态语言规则（用户 §十三, **永久锁死**）

- **可以说**: G-2 Final=CLOSED（语义=**G-2 自身的 consumption/evidence/
  attribution/decision-input 闭环**）; G-2-G-LIVE=PASS /
  G-2-G-CLASSIFY=PASS / G-2-G-FAULT=NOT OBSERVED（不阻塞）。
- **不能说**: 'G-2-G E2E=PASS' / 'Recovery E2E=COMPLETE' /
  '03-02 Recovery Contract=FROZEN'（直至 OQ-R1..R5 裁决冻结）。
- 边界根源: R47 已将 E2E 从 G-2 Final 验收范围剥离——**G-2 Final=
  CLOSED 与 G-2-G-E2E=未实现/未验证 并存不矛盾**; 后续账本永久保持此
  分离, **禁把 E2E 写回 G-2 Final**; 四层 Gate LIVE/CLASSIFY/FAULT/E2E
  分层边界永久保持。
- 观测边界升级表述（用户 §三）: 诊断观测面**没有改变原有决策拓扑**
  （非仅"代码没出问题"）——此表述入账为口径基准。

### 63.3 §十五 指令接收: 活体复跑终止 + 唯一待裁=OQ-R1..R5

- **不再跑任何 R46/R47 活体——该轮已彻底结束**。
- 唯一真正待裁=03-02 OQ-R1..R5（最关键=OQ-R1）; **用户维持推荐:
  OQ-R1=全维持现状**——FailureDomain 继续作 evidence/attribution,
  不驱动新 Recovery Strategy 分支; 03-02 可成为**零运行时代码 Contract
  close-out**, 直进 A2-8-04 Program Timeline/AV continuity; 比贸然增加
  Input→recover/Bridge→…/Program→… 更稳, 符合已冻结 Supervisor
  Recovery-only 边界。
- **裁决完成前继续保持零 Recovery runtime code**; 下一轮=OQ-R1..R5
  逐项最终裁决（冻结包已落 03-02 doc §8, 待裁版）。

### 63.4 本轮执行（零运行时代码零矩阵）

- 主账 §63 + 03-02 doc §8（R48 确认+OQ 冻结包待裁版）+ tasks R48 段;
  单一账单元提交; **03-01 探针本轮不新增**（R48 无 03-01 域新裁定——
  G-2 Final 及四层 Gate 模型已录 §12, 状态语言规则属全局口径记本账+
  03-02 doc; 止于 §12——如实披露, 避免重复记账噪音）。
- 无代码变更⇒无矩阵需求; 无真机动作（§十五: 活体复跑终止）。
- 基线: comet/a2-8-dual-input-switch@6f5735e, master=7745968（本节提交
  后新头）。
