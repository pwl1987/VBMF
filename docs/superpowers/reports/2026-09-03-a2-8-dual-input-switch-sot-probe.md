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
