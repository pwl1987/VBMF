# C-TIMELINE-01 Implementation Probe & Impact Map（零代码）

- 日期：2026-09-04；分支 `comet/a2-8-dual-input-switch`；基线 f3158a0（Design
  Freeze 有效——用户复核通过）。
- 性质：**只读实现前探针，零代码**。按用户执行令："实现前代码拓扑探针 /
  Impact Map → 最小变更面冻结 → 再写代码"；十项落点逐项钉死，其中
  **第 4/5/6 项以真实 Rust/GStreamer API 与现有 graph 生命周期为准，
  不凭架构图猜**（已上盒 `gst-inspect` 实证 + gstreamer-0.23.7 crate
  源码实证）。
- 裁决链：Design Freeze（15 项+R1-R8）→ 本探针 → OQ-IMP-1..7 待裁 →
  最小变更面冻结 → 实现批次。
- 纪律重申：本轮**不实现**；不再讨论已冻结架构；发现与 Freeze 冲突处
  停下回裁。

## 1. As-Is 代码拓扑（实锚）

### 1.1 组合根与生命周期

- `ProgramExecutionRuntime`（program_execution.rs:209）=
  `{session_id, Mutex<Option<Inner>>}`；`Inner` = `{group, switcher,
  graph, taps, tap_port, watchdog_stop}`（:197-205）。
- `create()` :225（creator=destroyer；session/group 身份一致性
  fail-closed）；`teardown()` :317。
- **真实构造点恰两处生产面**：诊断 bin（bin/media-agent.rs:464）+
  Gate（gates/dual_input.rs:480）；其余为 feature-gated 测试
  （registry.rs:300/:376、program_execution.rs tests）。TimelineAuthority
  挂点的全部调用面=这两处+测试。

### 1.2 Plan 面（Q1 对象）

- `PipelinePlan`（pipeline.rs:134-148）：`{source, normalize: bool(:141),
  switch_mode, outputs}`；**derive Serialize/Deserialize（wire 面）**，
  serde roundtrip 测试存在（switch_mode 替换曾有"wire 值不变"兼容锚）。
- `normalize: true` 全仓 8 处：生产仅 2 处——`self_test()`(:218/:227) 与
  `materialize` 的 `plans.push`(:694/:695)；其余 6 处为测试构造
  （:1034/:1154/:1184/:1208/:1230/:1337）。消费点=零（doc :136-141
  自认）。
- 概念消歧（防误伤）：`normalize.rs` 模块=P0.7B Canonical 描述层
  （judge-only，另一概念）；`switch_policy.rs` MASTER_SWITCH 文档=
  V0.2 §1.17 格式策略词表（Format Policy 域，非本字段）。

### 1.3 切换执行链（As-Is 全序，dual_input.rs:626-644 实锚）

```text
group.plan_switch(SwitchIntent)          // Desired → Plan（switch_execution.rs）
group.begin_switch(plan)                 // Desired 状态机推进
switcher.switch(&graph, &plan)           // Execution（adapter）
  ├─ degraded fail-closed / TargetNotInGroup / UnsupportedPolicy
  ├─ StalePlanEpoch（plan.epoch != g.av_epoch+1）   // epoch 门
  ├─ TargetAlreadyActive 纵深
  ├─ set_active(video_selector) ──┐      // switch_graph.rs:500 区域
  ├─ set_active(audio_selector) ──┤ P1-1：audio 败→回滚 video；
  │        回滚再败→degraded+active=None（fail-closed）
  └─ g.active/g.av_epoch → SwitchExecuted{FrameAligned, av_epoch}
sleep(SETTLE_SECS)                       // settle（纯等待）
switcher.observe(&graph)                 // Observed（active-pad 属性回读）
group.complete_switch(observed_active)   // Observed 确认后才算完成
```

- `set_active`（switch_graph.rs:80-97）= `selector.set_property(
  "active-pad", &pad)` **纯属性写**，无任何事件发送。
- `SwitchExecutionPlan{from, target, policy, epoch}`
  （switch_execution.rs:78-83）；`SwitchExecuted{boundary, av_epoch}`
  （contracts/switch.rs:35-38）。
- `SwitchGraph` 簿记（switch_graph.rs:38-60）：devices/input_handles/
  started/initial_active/active/av_epoch/degraded/pipeline/
  video_selector/audio_selector/pad_index。
- 驱动方=诊断消费方直驱（Gate bin 均）；**watchdog 只 observe+
  complete_switch（watchdog.rs:491/:530），从不驱动 switch**。

### 1.4 观测链（As-Is）

- program appsink 回调（switch_graph.rs:150-196）→ `buf.pts()` →
  `pipeline_events::HEALTH_ARCS`（static 注册表 :16）→ `PipelineHealth`
  （pipeline.rs:250-270：first/last pts、三态
  `observe_video_pts` :295-304 sticky、帧计数）。
- `observe()`（switch_graph.rs:526-596）：active-pad 属性回读（Observed）
  + 输入管线 PipelineHealth（input_pts 列）+ program PipelineHealth
  （prog 列）→ `ProgramObservation`（contracts/switch.rs:59-77）。
- 桥列：MediaTap tap 分流 pad BUFFER probe（controller.rs:662——
  **全 adapter 层唯一 probe 用点**）→ bridge_stats（PTS+wall-clock
  now_ms 分记，G/H-1）。
- `TimelineSample` 三列独立装配（program_execution.rs:59-77，调用方
  侧）。
- **GStreamer 高层 API 零存量**：全仓零 `send_event`、零 Segment event、
  零 clock/base_time 操作（grep 实证）——时间线机械从零起。

## 2. 盒上 GStreamer 实证（1.28.2；Q4/Q5/Q6 事实源）

### 2.1 `input-selector` 真实能力面（gst-inspect 全属性）

| 属性 | 默认 | 对本设计的意义 |
| --- | --- | --- |
| `active-pad` | — | rw、**PLAYING 态可切**（现行切换机制，不变） |
| `drop-backwards` | **false** | "Drop backwards buffers on pad switch"——**默认 false=切换后回退缓冲放行=真机 NonMonotonic 直读机制**；置 true 是**丢帧藏证**（反证据红线，禁作为"修复"） |
| `sync-streams` | **true** | "Synchronize inactive streams to the running time of the active stream or to the current clock"——inactive pad 阻塞同步行为，影响 standby 源缓冲 |
| `sync-mode` | `active-segment` | 枚举 active-segment/clock——按当前 active segment 同步 |
| `cache-buffers` / `n-pads` | — | 缓存/计数，非时间线面 |

- sink pad 属性含 **`running-time`（可读）**——每源 running time 可查。
- **关键结论：input-selector 自身不做任何时间戳改写**——转发 active
  pad 的 buffer 与其原时间戳。时间线机械必须在别处落。

### 2.2 `intervideosrc`/`interaudiosrc` 真实行为面

- `channel`（现行唯一设置项，switch_graph.rs:256-259/:298-301）；
  `do-timestamp` 默认 **false**=buffer 携带**生产管线原始时间戳**跨桥
  （这正是两独立时钟域 PTS 直通的机制根源）；
- `timeout` 默认 1s=无数据后输出黑帧（桥断流行为面，L5 相关背景）。

### 2.3 `identity` 真实能力面（方案 B 的现成 primitive 候选）

- **`single-segment`**（rw，默认 false）："Timestamp buffers and eat
  segments so as to appear as one segment"——**真实存在的
  clock-segment 原语**：吞掉入向 segment event、按单一连续 segment
  重打时间戳。与 Freeze 方案 B 语义直接对应。
- `ts-offset`（i64 ns，默认 0）+ `sync`——同步用偏移（语义限于
  identity 自身 sync，非通用 PTS 平移）。
- **诚实边界**：single-segment 的精确重打数学（基于 offset/segment
  start 的具体公式）本轮未实验验证——**实现批次须先 sim 实验锚定，
  禁凭文档描述猜**。

### 2.4 gstreamer-rs 0.23.7 crate 真实 API（盒 registry 源码实证）

| API | 锚点 | 用途 |
| --- | --- | --- |
| `event::Segment::new(&FormattedSegment)` / `SegmentBuilder` | crate event.rs:778-787/:2493 | **Segment event 可在 Rust 构造** |
| `Pad::send_event(impl Into<Event>)` | crate pad.rs:369 | 从某 pad 发事件（downstream segment 的真实发送路径） |
| `Element::send_event` | crate element.rs:138 | 元素级发送（seek 类） |
| `PadProbeInfo::buffer_mut()` | crate pad.rs:68 | **probe 内可变写 buffer——in-probe PTS 重写可行** |
| `PadProbeType::EVENT_DOWNSTREAM` | crate pad.rs:2437（例） | segment event 可被 probe 观察/拦截 |

## 3. 十项落点逐项（现状锚+事实+候选+待裁）

### Q1 `PipelinePlan` 如何替换裸 `normalize: bool`

- 事实：生产构造点仅 2（self_test/materialize）；字段在 **wire 面**
  （serde derive+roundtrip 测试先例：switch_mode 替换时"wire 值不变"）；
  消费=零。
- 候选 A：**直接删除字段+清理 8 构造点**，不在 PipelinePlan 上放
  替代物——时间线状态全部归 TimelineAuthority Domain（Program 级），
  输入 Plan 级无时间线语义。
- 候选 B：替换为 `TimelinePolicy`（词表化声明）——风险：若 Policy
  无消费者=**再造"声明未消费"字段**（A2-7-01 Gap 原样复发）。
- **OQ-IMP-1**：删字段（A）还是 TimelinePolicy（B，须同步定义其消费者
  =TimelineAuthority 装配）？wire 兼容要求（旧 JSON 带 normalize 字段
  是否须可反序列化）一并裁。

### Q2 Runtime 构造/生命周期如何挂入 TimelineAuthority

- 事实：create() 六参数（sid/group/switcher/tap_port/tap_wirings）；
  调用面恰 bin:464+gate:480+测试；Inner 六字段。
- 候选（最小面）：TimelineAuthority 作为 **Inner 内纯状态字段**（无
  新 trait 对象、无新 Arc 层——防"大型独立 Engine"）；create() 构造
  初值（program_epoch 起点与 initial_active 对应的首个 SourceSegment）；
  teardown 清理。两个生产调用点按新参数面同步。
- **OQ-IMP-2**：Timeline 数据经何路径入 Adapter 执行——扩展现有
  `SwitchExecutionPlan`（携带 timeline payload）还是
  `SwitchExecutionAdapter` 新方法（如 `apply_timeline`/`observe_
  timeline`）？Freeze §11 留白的 SPI 签名问题即此。

### Q3 switch_graph 切换边界在哪里注入 SourceSegment

- 事实：切换边界=switch() 内 set_active 成对属性写（:500 区域）；
  epoch 门在前（plan.epoch != av_epoch+1）；Domain 侧 plan 由
  group.plan_switch/begin_switch 产出。
- 候选：TimelineAuthority 在 **plan 时刻**计算新 SourceSegment
  （锚=最近 Observed 的 continuity anchor：当前段 program PTS + 目标源
  input/bridge PTS——数据经 observe 流入 Domain，**Domain 不触
  GStreamer**，R3/R4 保纯净）；segment 随 plan 下行；adapter 在
  active-pad 生效的同一切换动作内安装映射。
- 待裁并入 OQ-IMP-2/OQ-IMP-3（数据流与执行序绑定）。

### Q4 timestamp/segment execution 落点（inter 后 selector 前 vs 后）

- 事实（§2）：selector 不改写时间戳；intersrc 透传原始终戳；
  `identity single-segment` 真实存在；probe `buffer_mut` 可变写。
- 候选执行点（全部真实可行，**选点待实验+裁决**）：
  - (i) 每源分支 inter src 之后、selector **之前**插 per-source
    时间线元素/probe——源侧归一；
  - (ii) selector **之后**（selector→queue 间）单点 per-plane——
    Program 汇流后单点，天然"Program graph 边界"（与架构图
    ProgramGraph 组件对位）；
  - (iii) selector src pad probe（buffer_mut 重写 PTS，映射函数由
    Domain 提供、adapter 闭包执行——R6 合规）；
  - (iv) `Pad::send_event` 显式 Segment event（声明性边界）。
- **OQ-IMP-3**：执行点组合选型——**须以实现批次首个 sim/mock 实验
  锚定**（identity single-segment 精确行为/selector 切换时 segment
  event 实际时序/probe 重写与 selector 帧边界对齐），本轮不裁不猜。

### Q5 GStreamer SEGMENT/EVENT 真实发送路径

- 事实（§2.4）：Rust 侧 `event::Segment::new`+`Pad::send_event` 真实
  存在；EVENT_DOWNSTREAM probe 可拦截。
- 候选路径：下游 segment 经 **src pad `send_event`**（sticky 下行）；
  风险事实：外部注入 segment 与元素自身 segment 状态可能失步——
  `identity single-segment` 正是框架内做此事的安全封装（它自己吃/发
  segment）。**发送路径选型并入 OQ-IMP-3 实验裁**。
- 补充红线：`drop-backwards=true` 是丢帧藏证非修复（§2.1），禁入
  方案。

### Q6 Video/Audio 是否分别处理 segment

- 事实（非猜测）：program graph 内 video/audio **两条独立链**
  （:217-241/:275-306，各自 selector/queue/appsink）；segment event
  per-chain 下行；成对切换=同一次 switch() 内两 selector 同 epoch。
- 结论候选：**必须分别处理**（各自 segment/mapping/时序，同一
  program_epoch）——与 Freeze §9（共享 epoch 不共享数值序列）完全
  一致。此项事实充分，实现批次直接按此设计，无实验前不确定性。

### Q7 appsink PTS 观察如何与 TimelineEvidence 对齐

- 事实：program appsink→PipelineHealth（HEALTH_ARCS）；input 列=
  输入管线 PipelineHealth；bridge 列=tap probe stats（controller 侧）；
  TimelineSample 由调用方装配。同一 buffer 全程 PTS 可在三列独立测得。
- 候选：TimelineEvidence 在 **adapter 侧**装配（映射是 adapter 执行
  的，input_pts↔mapped_program_pts↔offset 同点可得）→ 经
  `observe_timeline` 类新契约面（Freeze §8：不塞 ProgramObservation）
  读出。
- **OQ-IMP-4**：evidence 读出面=SwitchExecutionAdapter 新方法 vs
  独立 timeline 契约 trait（含 Mock 适配器同步面）。

### Q8 switch→mapping→observation 事务顺序

- 事实（As-Is §1.3）+ Freeze §10（settle 期间 PTS 必须已属新
  timeline）。
- 候选序（Domain 层冻结候选）：
  `plan_switch → TimelineAuthority 计算新 SourceSegment（用最新
  Observed 锚）→ begin_switch → adapter.switch（同一切换动作内安装
  映射+active-pad）→ SwitchExecuted（av_epoch+timeline Fact 候选）
  → settle=TimelineTransition → observe（PTS 已在新 timeline+
  TimelineEvidence）→ complete_switch`。
- **OQ-IMP-5**：adapter 内部微观序（映射安装与 active-pad 写先后、
  帧边界对齐保证）——须 sim 实验锚定（与 OQ-IMP-3 同批）。

### Q9 failure/rollback 时 Timeline 状态

- 事实：P1-1 补偿（audio 败→video 回滚→Err 状态如实=未切；回滚再败
  →degraded+active=None）。
- 候选：段模型天然优势——**segment 只增不改**：失败切换若映射已装
  但未激活=段悬空无害；部分激活（video 短暂过 B）→ 若有任何跨段
  buffer 可能流出=记 `DiscontinuityDeclared`；degraded=continuity
  不可证 → 对位 Freeze §13 **Hard Recover 语义**（timeline epoch
  处置留 A2-8-03 接口）。
- **OQ-IMP-6**：三类结局（rollback-clean / discontinuity-declared /
  degraded→hard-recover）的精确判定谓词。

### Q10 L4 判据如何升级为 Timeline Mapping Evidence

- 事实：现行五合取 dual_input.rs:644-648（唯一失败项=prog pts
  NonMonotonic）；H1 跳过 :774；Freeze §14=Gate 改动属本
  implementation change 范围。
- 候选：`prog pts state≠NonMonotonic ∧ pts.is_some()` 两项替换为
  **TimelineObservation 成立**（Freeze §8 七条的证据化谓词；
  DiscontinuityDeclared+expected transition=合法边界）；其余四项
  （completed/observed=B/epoch==1/…）不动；H1/L0-L3/L5 表面不动。
- **OQ-IMP-7**：L4-TIMELINE 新谓词精确措辞（随最小变更面冻结一并裁）。

## 4. 最小变更面提案（候选清单，不实现）

| # | 文件/面 | 改动类型 | 依据 |
| --- | --- | --- | --- |
| 1 | `src/program_timeline.rs`（新） | Domain 纯模型：TimelineAuthority/ProgramEpoch/SourceSegment/TimelineMapping/Discontinuity/PtsState 四态/TimelineMapped Fact/TimelineObservation——零 GStreamer（对位 switch_execution.rs 纯度） | Freeze §1-§8、R3/R4 |
| 2 | `src/contracts/switch.rs` | SwitchExecutionAdapter 契约扩展（timeline 执行+evidence 读出面，形态待 OQ-IMP-2/4）+ Mock 同步 | Freeze §11 |
| 3 | `src/switch_execution.rs` | plan↔timeline 关联（epoch 关系 per Freeze §3）；ExecutionGroup 本体零 timeline 职责（R3） | Freeze §3 |
| 4 | `src/program_execution.rs` | Inner+create/teardown 挂 TimelineAuthority（纯状态字段）；两生产调用点同步 | Q2 |
| 5 | `src/adapters/gstreamer/switch_graph.rs` | 执行机械（identity/probe/send_event 组合待 OQ-IMP-3）+双链分别处理+evidence 装配 | Q4-Q7 |
| 6 | `src/pipeline.rs` | normalize 字段处置（删/Policy 待 OQ-IMP-1）+构造点清理 | Q1 |
| 7 | `src/gates/dual_input.rs` | L4-TIMELINE 谓词升级（仅此段；H1/L0-L3/L5 不动） | Q10、Freeze §14 |
| 8 | `src/bin/media-agent.rs` | create() 签名若变的接线同步 | Q2 |
| 9 | tests | mock 套件+feature 测试+Gate 单测 | 常规 |

**零触碰**（Freeze 不触碰清单+先前冻结）：controller.rs MediaTap 契约
（bridge probe 只读复用）、watchdog.rs、supervisor.rs、resolver.rs、
registry/session、input 管线生命周期。

## 5. OQ-IMP 待裁清单（最小变更面冻结输入）

- **OQ-IMP-1** PipelinePlan.normalize：删除（A）vs TimelinePolicy（B
  须绑定消费者）；wire 兼容要求。
- **OQ-IMP-2** Timeline 数据入 Adapter 路径：扩展 Plan vs 新 trait 方法。
- **OQ-IMP-3** 执行点组合（selector 前/后/probe/send_event/identity
  single-segment）——**须 sim 实验锚定后裁**。
- **OQ-IMP-4** TimelineEvidence 读出面：adapter 新方法 vs 独立契约。
- **OQ-IMP-5** adapter 内部微观序（映射安装 vs active-pad 时序）——
  与 OQ-IMP-3 同批实验。
- **OQ-IMP-6** 失败三结局判定谓词（rollback-clean/discontinuity-
  declared/degraded→hard-recover）。
- **OQ-IMP-7** L4-TIMELINE 新谓词精确措辞。

## 6. 执行序提案

1. 用户裁 OQ-IMP-1/2/4/6/7（纯设计项）+ 授权 3/5 的 **sim 实验刀**
   （实验=实现批次内第一刀，产出行为锚后回裁选型）；
2. Domain 层（program_timeline.rs+tests）→ 契约面 → Mock 闭环；
3. Adapter 执行面（按实验锚定的选型）→ bmd+gst feature 测试；
4. Gate L4 谓词升级 → 真机复跑（§29.2 五件套纪律）；
5. 全程 R1-R8 红线+Freeze 15 项为验收基准；偏离即停回裁。

## 7. 本轮诚实边界

- identity single-segment 的精确重打数学、selector 切换时 segment
  event 实际时序、probe 重写与帧边界对齐——**均未实验**，已显式
  列为 OQ-IMP-3/5 实验前置，不猜不预裁。
- 盒实证=只读 gst-inspect+crate 源码 grep（零媒体启动、零状态变更）。
- 本轮零代码；最小变更面（§4）为候选清单，经用户冻结后才成实现范围。
