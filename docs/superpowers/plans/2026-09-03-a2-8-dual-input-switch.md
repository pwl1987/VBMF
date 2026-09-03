---
change: a2-8-dual-input-switch
design-doc: docs/superpowers/specs/2026-09-03-a2-8-dual-input-switch-design.md
base-ref: 2ce90088ba48e977cc9a5a7fd0653c1701e7a44d
---

# A2-8-01 Dual-input FRAME_SWITCH Execution Group MVP Implementation Plan

> **For agentic workers:** 本计划按任务逐项执行，任务内步骤用 checkbox 跟踪。
> 上游裁决：用户两轮终裁（probe §7 十项冻结 + T1-T12 矩阵 + 12 红线），
> 冲突时以 probe §7 为准。

**Goal:** 交付 **A2-8-01 = 最小、可验证、可监督的 Program-level FRAME_SWITCH
Execution Group**：SwitchIntent→SwitchExecutionPlan→SwitchExecutionAdapter
执行链（独立于 MediaBackend 五方法）、ExecutionGroup 边界概念、Program
graph 物化（topology=实现细节，v1 候选=inter 系）、Video/Audio 成对显式切换
（方案 A）、六路 PTS 观测点、MultiInputWatchdog（修正首句柄单视角为
execution group 四视角）。mock 层 T1-T12 全落地 + 盒上编译/仿真验证；
真机 SDI 留 A2-8-02。

**Architecture:** 纯执行面模型落在 `src/switch_execution.rs`（复用
`SessionInput`，零第二 identity）；SPI trait 落在 `src/contracts/switch.rs`
（与 backend.rs 平行，**不扩 MediaBackend**）；Mock 适配器
`src/adapters/switch_mock.rs`（确定性 PTS 流 + 成对切换 + 故障注入）；
GStreamer 适配器 `src/adapters/gstreamer/switch_graph.rs`（feature 门控，
v1 materialization=inter 系候选：input pipeline tee→intervideosink/
interaudiosink + program pipeline intervideosrc×2→input-selector(video)+
input-selector(audio)→program 出口，selector `switch-mode` 帧边界；
文档标注"topology=实现细节可替换"）；观测折叠纯函数
`execution_group_observe_fold`（watchdog.rs，mock 可测）+ hw 门控薄壳
`spawn_execution_group_watchdog`；生产接线仅改 `bin/media-agent.rs` L403
双输入分支（单输入路径与 gates L165 原样——gate 是单输入 SESSION-RT-01）。
**状态三分离**：Desired（`SwitchDesired`）≠ Execution（adapter 内 selector
态）≠ Observed（`ProgramObservation`）；switch state 绝不进
MediaSession/SessionInput（T9）。

**Tech Stack:** Rust（`services/media-agent`），uuid v1，serde；cargo feature
矩阵 default/simulation/mock/bmd-provider/gstreamer-backend；mock 测试基线
307（A2-7 收口）；盒上 cargo（10.30.15.10 SSH，`box_build.sh` 先例）；
CI 七 checks（`.github/workflows/media-agent.yml`）。

---

## 0. 输入、事实基线与纪律映射

### 0.1 输入（本计划唯一事实源）

| 输入 | 路径 |
| --- | --- |
| 终裁冻结（十项 + T1-T12 + 12 红线 + OQ 修正） | `docs/superpowers/reports/2026-09-03-a2-8-dual-input-switch-sot-probe.md` §3/§7 |
| Design Doc（gate-frozen） | `docs/superpowers/specs/2026-09-03-a2-8-dual-input-switch-design.md` |
| 任务边界（四栏纪律） | `docs/openspec/changes/a2-8-dual-input-switch/tasks.md`（本计划覆盖任务 3，不扩展） |
| 上游参照 | 同目录 `proposal.md` / `design.md`；A2-7 归档 `docs/openspec/changes/archive/2026-09-03-a2-7-execution-materialization/` |

### 0.2 已验证实码事实（base-ref = `2ce9008`；2026-09-03 Build 前逐项复核）

- `src/contracts/backend.rs:22-32`：`MediaBackend` 恰五方法
  instantiate/start/stop/recover/observe——**A2-8-01 禁改**（冻结 #2）。
- `src/session.rs:193-197`：`SessionInput { device_id: Uuid, handle:
  PipelineHandle }` 两字段；`MediaSession.inputs: Vec<SessionInput>`
  （L185 区域）；**无** `active_input`/`is_active`（全库 grep 零命中）。
  `SessionManager::status()`（L829-835）返回 `MediaSession` clone——
  ExecutionGroup 装配面已具备，**SessionManager 零改动**（冻结 #3）。
- `src/bin/media-agent.rs:403-410`：`mgr.status(&sid).and_then(|s| s.pipeline)`
  → `spawn_ingest_watchdog(...)`（`s.pipeline` = `inputs.first()` 兼容字段，
  session.rs:1285）；`src/gates/session_lifecycle.rs:165` 同款（单输入 gate，
  本 change 不改）。`VBMF_DIAG_INPUTS` env（bin L271-）已支持 N 输入诊断
  intent——双输入接线入口存在。
- `src/watchdog.rs:27-276`：`spawn_ingest_watchdog` 8 参单 handle 循环：
  `ctrl.observe(&handle)` + `pipeline_events::HEALTH_ARCS` 折叠
  a1-a4/b1-b4/c1-c4 → Supervisor（只决策）→ lease 重校 → `ctrl.recover`。
  **cfg(all(bmd-provider, gstreamer-backend)) 硬件门控**；mock 面不可直测
  ——group 观测逻辑必须抽纯函数（本计划任务 4）。
- `src/pipeline_events.rs:16-24`：`HEALTH_ARCS: LazyLock<HashMap<PipelineHandle,
  Arc<Mutex<PipelineHealth>>>>` + `read_health(handle)`——program graph 观测
  复用此机制（按 handle 注册）。
- `src/adapters/mock.rs:118-141`：`MockBackend` 确定性 stub（NEXT_PIPELINE_ID
  计数 handle）；`src/pipeline.rs:535/555` `materialize`/`materialize_with_output`；
  videotestsrc 仿真源在库（simulation feature，pipeline.rs/bin/signal.rs）。
- `src/pipeline.rs:130-147`：`PipelinePlan { source, normalize(Gap 登记不动),
  switch_mode: SwitchPolicy(默认 FrameSwitch, L228), outputs }`；单输出承诺
  L114——**PipelinePlan 零改动**（禁塞 A/B 三案）。
- `src/program/switch_policy.rs:25-77`：`SwitchPolicy` LOCK FINAL 三值 +
  `SwitchIoPlane` + parse fail-closed——**零改动**（T11）。
- `src/events.rs`：`RuntimeEvent::PipelineFault { pipeline: Uuid }`（L80，
  legacy DeviceId 双语义=V0.3 债务）——**不改 identity contract**（红线 2），
  新代码不扩大歧义：switch 观测走 Observation 平面，不新增 RuntimeEvent
  变体。
- `src/supervisor.rs:62-150`：`RestartPolicy`/`SupervisorAction`/`Supervisor`
  ——recovery only，switch 路径零调用（T10）。
- mock 测试基线 307（A2-7 收口 PR#29）；clippy 4-combo + fmt 为盒上验证先例。

### 0.3 冻结纪律 → 计划落点映射

| 冻结项 | 落点 |
| --- | --- |
| #1 ExecutionGroup=boundary，SessionInput 原样 | 任务 1（复用 SessionInput，禁新 identity 类型） |
| #2 Switch≠Backend SPI | 任务 2（contracts/switch.rs 新 trait，backend.rs 零 diff） |
| #3 SessionManager≠graph builder | 全程零 session.rs diff（任务 7 grep 门禁） |
| #4 Supervisor≠switch executor | 任务 4/6（fold 无 switch 动作；supervisor.rs 零 diff） |
| #5 topology=实现细节 | 任务 3/5（trait 面零 GStreamer 词；实现内文档标注可替换） |
| #6 FRAME first | 任务 1（plan fail-closed 拒 PACKET/MASTER） |
| #7 Video+Audio 成对（方案 A） | 任务 2/3（paired switch + 双面观测，单面切=FAIL） |
| #8 AV continuity mandatory | 任务 3/4（六路 PTS + 单调性折叠） |
| #9/#10 MASTER/failover Deferred | 任务 6（T12 反证：无自动切换触发路径） |
| Desired≠Execution≠Observed | 任务 1/3（三类型分离 + T9/T10 测试） |
| Event Debt 不修 | 任务 7（events.rs 零 diff 门禁） |
| 12 红线 + 禁塞 A/B | 任务 7（grep 门禁全套） |

### 0.4 验收矩阵 T1-T12 → 测试映射

| Gate | mock 测试（`*_rt_01_*` 命名延续） | 真机/盒上 |
| --- | --- | --- |
| T1 双输入同跑 | `switch_rt_01_group_two_inputs_running` | 02 |
| T2 汇入同一 Program Execution | `switch_rt_01_program_graph_consumes_group` | 01 盒上仿真 |
| T3 A→B→A 真实切换 | `switch_rt_01_explicit_switch_flips_observed_active`（adapter 态真实变化非字段翻写） | 01 盒上仿真 |
| T4 帧边界 | `switch_rt_01_switch_executes_at_frame_boundary`（`SwitchExecuted.boundary` 非 Option） | 01 盒上（selector switch-mode） |
| T5 Video/Audio 成对 | `switch_rt_01_paired_av_switch_same_epoch`（双面同 flip，单面=FAIL） | 02 |
| T6 三路 PTS 可追踪 | `switch_rt_01_six_pts_surfaces_trackable` + `switch_rt_01_program_pts_monotonic_across_switch` | 02/04 |
| T7 watchdog 不看 first() | `group_fold_rt_01_standby_b_observed_and_flagged`（B 停摆被检出） | 02 |
| T8 无跨设备污染 | `group_fold_rt_01_fault_attributed_to_own_device_only` | 03 |
| T9 lifecycle/switch 分离 | `switch_rt_01_session_state_untouched_by_switch` + SessionInput 键集锚 | 02 |
| T10 Supervisor 不执行 switch | `group_fold_rt_01_switch_success_no_recovery_action` | 03 |
| T11 SwitchPolicy 不污染 | `switch_rt_01_policy_enum_unchanged_anchor`（ACCEPTED_LIST+io_plane 回归） | — |
| T12 不偷渡 | `switch_rt_01_packet_master_fail_closed` + fold 无自动切换路径 | — |

---

## 任务 1：`src/switch_execution.rs` — 执行面模型（TDD Red→Green）

- [x] 1.1 Red：`switch_rt_01_group_requires_exactly_two_inputs`（≠2 拒）、
  `switch_rt_01_intent_target_must_be_group_member`（外源拒）、
  `switch_rt_01_packet_master_fail_closed`（T12：非 FRAME_SWITCH →
  `SwitchError::UnsupportedPolicy`，无 silent 回退）、
  `switch_rt_01_duplicate_device_rejected`。
- [x] 1.2 Green：类型 + 校验（全纯函数，serde，`#[serde(rename_all="snake_case")]`
  词表纪律；键集恰定——新类型逐字段列明防蔓延）：
  - `ExecutionGroup { session_id: SessionId, inputs: Vec<SessionInput> }`
    （`new()` fail-closed 恰 2 输入、device_id 去重；**复用 SessionInput，
    零新 identity**）。
  - `SwitchIntent { target: Uuid, policy: SwitchPolicy }`（显式手动切换；
    无 from 字段——from=group 当前 Desired，防调用方预归因双 SoT）。
  - `SwitchDesired { Active(Uuid), Switching { from, to } }`——Desired 平面。
  - `SwitchExecutionPlan { target, policy, epoch: u64 }`（`from_intent(
    &ExecutionGroup, &SwitchDesired, &SwitchIntent)` 校验：target∈inputs、
    policy==FrameSwitch、from==当前 Active；epoch 单调推进）。
  - `SwitchError` 封闭词表（UnsupportedPolicy/TargetNotInGroup/NotActiveSource/
    GraphNotRunning/Backend(String)）。
- [x] 1.3 `switch_rt_01_policy_enum_unchanged_anchor`（T11：ACCEPTED_LIST 三值
  + io_plane 映射回归锁）。
- [x] 1.4 停止条件：若发现需要改 `program/`、`session.rs`、`events.rs` →
  STOP 回报（触冻结边界）。

## 任务 2：`src/contracts/switch.rs` + `src/adapters/switch_mock.rs` — SPI 与 Mock（TDD）

- [x] 2.1 Red：T1/T2/T3/T5/T6 mock 测试（见 §0.4 映射表名）。
- [x] 2.2 Green：`SwitchExecutionAdapter` trait（**平行于 MediaBackend，
  backend.rs 零 diff**；trait 面**零 GStreamer 词**——冻结 #5）：
  `build_program_graph(&self, group: &ExecutionGroup) -> Result<PipelineHandle,
  SwitchError>`（program graph 复用 PipelineHandle 类型——它是真管线实例，
  HEALTH_ARCS/read_health 按此键控；非第二 registry）、`start_program`、
  `switch(&self, graph, &SwitchExecutionPlan) -> Result<SwitchExecuted,
  SwitchError>`、`observe(&self, graph) -> ProgramObservation`、`stop_program`。
  `SwitchExecuted { boundary: FrameBoundary, av_epoch: u64 }`（成对同 epoch
  ——T5 由类型承载）、`ProgramObservation { observed_active: Option<Uuid>,
  input_pts: Vec<InputPts>, program_video_pts: Option<u64>,
  program_audio_pts: Option<u64>, ... }`（Observed 平面；`InputPts {
  device_id, video_first_pts, audio_first_pts, video_pts_state,
  audio_pts_state }` 复用 PtsMonotonicity）。
- [x] 2.3 `MockSwitchExecutionAdapter`：确定性 PTS 流（observe tick 推进
  帧计数）、成对切换（video+audio 同 epoch flip——单面切构造不出）、
  program PTS 跨切换单调、故障注入钩子（`stall(device_id)` 供 T7/T8/T10）。
- [x] 2.4 停止条件：trait 需要感知 input-selector/inter 名称 → STOP（泄漏
  topology 到契约面）。

## 任务 3：`src/adapters/gstreamer/switch_graph.rs` — GStreamer 物化（盒上验证）

- [x] 3.1 实码复核：Read 现有 gstreamer controller 构链函数（appsink/
  HEALTH_ARCS 注册/tee 结构），确定 inter sink 挂接点。
- [x] 3.2 `GStreamerSwitchAdapter`（cfg gstreamer-backend）：v1 topology=
  **inter 系候选**（文件头注释：topology=实现细节，替换不经 Domain/API 变更）：
  input pipeline tee → intervideosink/interaudiosink（命名通道按 device）；
  program pipeline = intervideosrc×2 → input-selector(video) +
  intervideosrc(audio)×2 → input-selector(audio) → program tee →
  {appsink 观测, 出口}；selector `switch-mode` 帧边界属性；switch() 同
  epoch 置双 selector active-pad；observe() 读实际 active-pad + appsink PTS
  （Observed=实际读数非命令回显）。
- [x] 3.3 盒上验证（SSH cargo）：feature 编译矩阵含
  `--features bmd-provider,gstreamer-backend`；videotestsrc 双源仿真
  switch smoke（真 GStreamer 执行图 + 真实 active-pad 切换证据，无 SDI
  依赖）——01"真实 Execution Graph+真实切换"落地；证据记录。
- [x] 3.4 停止条件：inter 通道在真实 controller 结构不可挂接 / selector
  帧边界行为与文档不符 → STOP 上报（候选拓扑不可用≠换 Domain）。

## 任务 4：`src/watchdog.rs` — MultiInputWatchdog（纯折叠 + 薄壳）

- [x] 4.1 Red：`group_fold_rt_01_standby_b_observed_and_flagged`（T7：B 路
  停摆被检出——证明非 first()）、`group_fold_rt_01_fault_attributed_to_
  own_device_only`（T8）、`group_fold_rt_01_switch_success_no_recovery_
  action`（T10：切换成功零 Supervisor 动作）、`group_fold_rt_01_program_
  pts_monotonic_fold`（六路 PTS 折叠 + 回退检出）。
- [x] 4.2 Green：`execution_group_observe_fold(...)` 纯函数（输入=各输入
  read_health 快照 + ProgramObservation + Desired；输出=`GroupObservation
  { per_input: Vec<(Uuid, InputHealthFold)>, switch_health, program_health,
  actions: Vec<GroupAction> }`——`GroupAction` **封闭词表且不含任何
  Switch/输入切换变体**（T10/T12 类型级反证）；故障沿既有
  supervisor::report_failure 语义仅标注，不在此执行）。
- [x] 4.3 `spawn_execution_group_watchdog`（cfg bmd+gstreamer 薄壳）：
  单线程循环全部输入 handle + `adapter.observe(graph)` → fold → 既有
  RuntimeEvent/Supervisor/recover 链（**禁 for 循环 spawn 多 watchdog**——
  终裁修正方向）；输入管线 recover 沿用单管线 supervisor 策略；**零自动
  切换**。
- [x] 4.4 停止条件：fold 需要 Supervisor 决策切换 → STOP（冻结 #4）。

## 任务 5：生产接线 — `bin/media-agent.rs` L403 双输入分支

- [x] 5.1 实码复核 L395-420；双输入（inputs.len()==2 且诊断双输入模式）：
  由 `mgr.status(&sid).inputs` 装配 ExecutionGroup → adapter
  build_program_graph + start_program → spawn_execution_group_watchdog
  （组合根装配，**SessionManager 零改动**）；单输入路径逐字节保持（含
  gates L165）。
- [x] 5.2 cfg 硬件门控与现有一致；mock/default 构建零新依赖。
- [x] 5.3 停止条件：需要 SessionManager 感知 program graph → STOP（冻结 #3）。

## 任务 6：T12/T9 反证 + Session 锚

- [x] 6.1 T9 落地为 `switch_rt_01_session_input_keyset_locked`（键集恰 2
  键）+ `switch_rt_01_no_auto_failover_path` 内切换链构造级证明——"session
  untouched by switch" 由结构保证（切换链零 Session/SessionManager 引用,
  编译面成立, 故未重复 mock mgr 序列化对照测试）
- [x] 6.2 SessionInput 键集锚测试（恰 2 键 device_id/handle——防 active
  字段蔓延）+ `switch_rt_01_no_auto_failover_path`（fold/adapter 无隐式
  触发切换入口的构造级反证）。

## 任务 7：全量验证 + 门禁 + 收口

- [x] 7.1 盒上 cargo：mock 全测（基线 307 + 新增恰数记录）+ clippy 4-combo
  + fmt check + feature 编译矩阵。
- [x] 7.2 grep 门禁（diff 边界定性，非计数）：`contracts/backend.rs`/
  `session.rs`/`events.rs`/`supervisor.rs`/`program/`/`pipeline.rs`
  **零 diff**；switch 契约面（contracts/switch.rs）零 GStreamer 词。
- [x] 7.3 tasks.md 勾选任务 3（四栏补全：Verification=盒上 cargo 输出 +
  T1-T12 映射）；**不宣布 A2-8 CLOSED**（02-05 未走）；commit（消息含
  T1-T12 达成态与 mock 计数）。

---

## 验证命令（盒上先例）

```bash
# 盒上 cargo（10.30.15.10 SSH，box_build.sh 先例；具体会话按仓库既有流程）
cargo test --features mock          # mock 全量（307+新增）
cargo clippy --all-targets --features mock -- -D warnings   # 4-combo 之一
cargo fmt --check
cargo check --features bmd-provider,gstreamer-backend      # 编译矩阵
```

## 范围冻结（STOP 清单汇总）

改 V0.2/Event contract/identity/SwitchPolicy/SessionManager/Supervisor/
PipelinePlan 塞 A/B/MASTER/PACKET/auto-failover/HLS+RTMP 多输出 → 一律
STOP 回报用户，不得现场破界。
