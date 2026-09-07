# S15-E01 RCA：Video timeline evidence 自发超时（Step 15 8h rung·cycle 908）只读根因调查

- 日期：2026-09-07 ｜ 调查性质：**只读**（证据零改动·生产代码零触碰·阈值零放宽）
- 用户裁决（本轮输入）：维持 8h FAIL 原判；不启动 24h；生产代码冻结；先做只读 RCA；
  调查目的**不是**找理由改判 PASS，而是判定 FAIL 属「已设计 FailClosed 行为的可接受
  偶发边界」还是「Video evidence deadline / 调度实现的真实稳定性缺陷」。
- 事件编号：**S15-E01**（用户裁定制——比 960/960 PASS 更有价值：恢复语义在长期真实
  运行中的首次自发触发实证）。

## §1 事件事实（来自未改动证据 r64-stability-8h/）

- 时刻：2026-09-07T08:29:01Z（盒钟 +08:00 即 16:29；运行第 7h34m·第 908 次切换，
  B→A 方向）。960 次切换唯一一例；全部稳定基线运行（30m v1/v2 + 2h + 8h = 1320 次）
  唯一自发例（≈0.1%）。
- 命令响应：`status.status=executed` + `classification=unknown` + detail=
  「switch execution backend error: timeline 证据超时 FailClosed: timeline
  evidence insufficient (pending planes: [Video])」——**Audio 平面证据闭合，
  唯 Video pending**。
- 后续（设计内恢复全链）：R63-A Observed-first 恢复落定（svc.log 唯一运行期 WARN
  08:29:01.557Z）→ NewEpoch{ProgramEpoch(1)} + identity SourceSegment（source_
  start_pts=0/program_start_pts=0/offset=0）+ SegmentId(909) + DD → cycle 909-960
  共 52 次切换全部 executed+preserved·tl_ep=1 保持·零死锁零发散·teardown 干净。

## §2 工件层取证（全部正常——媒体/资源/服务面无任何停摆）

| 证据面 | 事实 | 结论 |
|---|---|---|
| 帧计数（samples.csv） | cycle 908 采样间隔 34s（正常 29s·多出 ≈5s=证据窗本身）；dv=1011/da=1350，与 34.5s 标称速率外推（≈1018/1372）吻合；基线 dv 865±6 | **视频/音频产出全程满速**，无采集停摆 |
| PTS 基线（readbacks at=） | d_at 逐周期 ≈28.8s 恒定推进，cycle 908=33.7s 与拉长间隔吻合；seg 907→909 跳号·单周期 declared_discontinuity 后即回 continuous | **PTS 推进正常**，再基准后即刻恢复 |
| 输入健康弧（watchdog 行） | 08:28:41→08:29:31 双输入 observed=true advancing=true bridge=Some(true) 连续（含事件中点 08:29:01.301） | **双输入（含目标源）PTS 活性全程在线** |
| 查询面（latencies.txt） | 事件 ±90s 窗 105 次查询 max 2.128ms·mean 1.2ms | 无调度停顿/锁竞争迹象 |
| events 面 | 全程 480 采样 critical=0·kinds 空 | 零关键事件 |
| svc.log 全量 | 8h 仅 1 条运行期 WARN（即恢复行）；另 12 条全部在启动期 01:13:33（设备探测噪音/PortId 碰撞/rtmp 未设 fail-soft/bus watch——已知形态） | 无任何前兆/近失事件 |
| 周期时延 | cycle 908 多出 ≈5s=恰一个 TIMELINE_EVIDENCE_TIMEOUT；drain 确认窗（另 5s）未叠加超时 | fence 确认**成功**（见 §4） |

## §3 源码层机制（证据链 ⑤⑥⑦ 的置位条件——只读）

- 编排循环（program_execution.rs:744-769）：每 **50ms**（TIMELINE_POLL_INTERVAL）
  轮询 `observe()` + `timeline_execution_facts()` 喂 Authority；deadline=
  **5s**（TIMELINE_EVIDENCE_TIMEOUT）；到界时 `pending_planes`（:1040）= transition
  态非 Mapped 的平面 → `EvidenceInsufficient{pending}` → FailClosed。
- Video 平面到 Mapped 需要两个顺序事实（adapters/gstreamer/switch_graph.rs）：
  - ⑤ `segment_observed=true`：**selector src pad EVENT_DOWNSTREAM Segment 事件**，
    且仅在 `t.executed==true` 时计数（:181-186）；
  - ⑥⑦ `first_mapped=Some`：其后的**带 PTS 缓冲**经 `apply_declared_mapping`
    （:238-246）。`!segment_observed → None` = 零改写透传（设计内 legacy 行为）。
- 关键次序（adapter `switch()`，:1123-1147）：**video 翻转 → audio 翻转 →
  bookkeeping（active/av_epoch）→ `t.executed=true`**。即 video pad 翻转与 executed
  置位之间存在窗口（跨 audio 翻转+两把锁+簿记；亚毫秒-毫秒级）。
- 对照：fence 的 `capture_segment`（:438-457）**无条件**捕获（不查 executed）——
  同一 EVENT 探针回调里两个消费者门控不同。
- map_pts（program_timeline.rs:124）：纯 i64 加法，仅算术溢出才 None——
  **结构上不存在「PTS 越声明界被拒」路径**。

## §4 根因判定

**判定：不是「Video PTS 迟到」，而是「⑤ 旗置位门控的读-竞争窗错过了一个已到达的
事件」——数据按时在场，谓词构造使 collector 看不到它。**

推理链（每步有独立证据锚）：

1. Video 新世代 Segment 事件**确实流过 selector src pad**：fence `capture_segment`
   捕获它（arm→翻转之间无其它 Segment 源——结构性唯一·rt_02 真链登记前提
   「input-selector 翻转必推 Segment」）。
2. 它**进一步被下游按序号确认**：`release_after_drain` 要求双面 seqnum 匹配的
   video_ready/audio_ready 才返回 Ok；cycle 908 编排收到的是证据超时错误而非
   fence/drain 错误 ⇒ **drain 确认成功 ⇒ Video Segment 到达 appsink 且序号匹配**。
3. 同一探针回调中，时间线侧旗（executed 门控）**未置位**：5s 内每 50ms 轮询喂入的
   facts 中 Video `segment_observed=false`（否则满速帧流下 ⑥⑦ 会在 1-2 帧内闭合）。
4. 1+3 ⇒ 该 Segment 命中探针时 `t.executed==false` ⇒ 事件到达时刻落在
   **video 翻转 → executed=true** 窗口内（fence 收到 / timeline 没收到——同一事件
   双消费者分裂，竞争窗指纹）。
5. 为何只有 Video：video 先翻（暴露窗最长·跨 audio 翻转），audio 后翻（其 Segment
   流达时 executed 已置）——与 `pending=[Video]` 精确吻合。
6. 概率形态自洽：~1ms 级窗口 × 30fps 帧边界 ≈ 0.1%/切换量级；1320 次出现 1 次。
7. **mock 为何从未复现**：mock 的证据推进由编排轮询自身驱动（循环注释「poll
   adapter observe[驱动 Mock tick]」），事件投递天然后置于 executed——竞争结构性
   缺席；这解释了 435 项 mock 测试与 F2×10 等 gate 场景零捕获。

排除的备选（用户问题 1-7 逐项对应）：

- Video PTS 迟到/停摆 ✗（输入弧 advancing 全程 + 帧满速 + at= 推进正常）；
- PTS 越声明界被拒 ✗（map_pts 无拒绝路径）；
- 缓冲无 PTS ✗（弱排除：弧/帧流正常 + 单平面单次自愈不合 Parsimony）；
- Segment 根本没推 ✗（fence 捕获 + 下游 seqnum 确认双锚）；
- CPU/调度/IO/锁停顿 ✗（查询 ≤2.1ms·threads 恒定·RSS 平·watchdog 连续）；
- events/critical ✗（零）。

## §5 定性（供裁决·不越权改判）

- 验收结果：**8h FAIL 9/10 维持**（十谓词冻结字面执行——本 RCA 不构成改判依据）。
- 缺陷定位：**真实可指认的微观竞争**，位于证据旗门控的簿记层（⑤ 置位条件与
  pad 翻转的次序），**不在**媒体/硬件/deadline/调度层。属「deadline/调度实现上的
  真实稳定性缺陷候选」的**弱形式**：触发罕见（1/1320）、无停摆、无状态污染、
  爆炸半径被 FailClosed→R63-A 恢复→NewEpoch 诚实再基准完整吸收——恢复语义的
  真机自发实证（S15-E01 的正面价值）。
- 与 R65 残留边界（「switch() Err + 物理=to」fault-injection 项）同族不同位：
  本例 switch() 本身成功，败在⑤证据旗。

## §6 修复方向候选（**全部未实施**·另裁·生产冻结）

1. `switch()` 内把 `t.executed=true` 前移到双 selector 翻转之前（最小改动；
   语义：executed=「本次执行意图已落」——需对齐 R65-A 契约 §3.2 措辞）。
2. ⑤ 旗改为无条件闩锁（Authority 侧 declare/execute 身份校验已防陈旧 Segment——
   on_switch_executed 的 epoch 联动本就是身份闭合环）。
3. EVENT 探针把 executed=false 期到达的 Segment seqnum 暂存，executed 置位时
   回放判定（零语义变更·改动面稍大）。

## §7 状态（按用户裁决表冻结）

2h PASS 10/10 ｜ 8h FAIL 9/10（不可改 PASS）｜ S15-E01 登记为本报告 ｜ 24h 未启动 ｜ 生产代码冻结 ｜ 阈值冻结 ｜ Step 17 未进入 ｜ PR #30 OPEN。
