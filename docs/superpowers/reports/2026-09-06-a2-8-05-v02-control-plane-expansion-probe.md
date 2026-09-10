# A2-8-05 v0.2 Control Plane Expansion — SoT 探针（只读代码裁决·R60）

状态: **DELIVERED（2026-09-06·零代码轮）**——用户 R60 指令: "下一轮不能
偷偷把它们塞进 v0.1。应当正式进入 v0.2 Control Plane Expansion，**先做
只读代码裁决，再决定最小实现边界**"; 推进序 = 探针 → 最小命令/状态投影
设计 → 真实代码实现 → A↔B 真机业务测试（Step 14）→ 长稳（15）→
Transport 联调（16）→ Preview RC（17）。本报告 = 只读裁决交付: 现状链
证据 + 插入点映射 + 设计提案（A/B/C 三案）+ 测试面 + 实现边界与待裁
清单。**实现待验收层对本案裁决后另启。**

基线: 29cef9c（PR #30 两次 CI 7/7 PASS）; 前置审计 =
2026-09-06-a2-8-05-normal-use-startup-entry-audit.md（八缺口表）。

---

## §1 范围与零触碰承诺

- **核心四文件（switch graph / program execution / switch 契约 /
  switch mock）= 只调用、零修改**——v0.2 的全部新代码落在命令面/
  幂等面/API 边界面/transport/bin 接线/新平面模块（§4 清单）; SHA
  锚定沿 R57 §24.1 不变。
- 只读方法清单（本探针引用·全部已存在）: `switch_program` /
  `observe_execution` / `SwitchIntent` / `ProgramExecutionObservation`
  ——零新增 Domain/SPI 面。

## §2 现状链证据（file:line·全部复核过）

### 2.1 命令面五要素（command.rs）

| 要素 | 锚点 | 现状 |
|---|---|---|
| 词表 | `command.rs:28-32` | 封闭三枚举; **:23 注释明言「新命令须过架构评审并显式更新词表快照测试」** |
| 形状校验 | `command.rs:95-133` validate | kind↔target 矩阵（StartSession↔Session{intent}; Stop/Release↔SessionById） |
| 执行映射 | `command.rs:138-201` dispatch | **薄映射只收 `&SessionManager`**——三臂各调 mgr 公共 API; 零插件面 |
| 白盒锁 | `command.rs:203+` tests | 公开关联函数清单 allowlist 硬编码测试 |
| wire 映射 | `transport.rs:112-150` map_command_request | KIND_VOCAB 封闭串 + kind/target 映射; target tag=`target_type` |

### 2.2 幂等面（idempotency.rs）

- `CommandIdempotency { mgr: Arc<SessionManager>, records, condvar }`
  （:90-94）——**包住 command::dispatch 的外包装**（validate→
  fingerprint→锁内原子 claim→锁外执行→终态落表→replay/conflict,
  :110-139）; 失败也 replay（D9-D）。**executor 依赖 = SessionManager
  单一**——switch 执行体不在其内。

### 2.3 Transport 红线（transport.rs:1-14 模块头）

- 自declared 契约: "**五端点不发明**"（GET /health·GET /api/v1/
  runtime·POST /api/v1/commands·GET /api/v1/events/projection·GET
  /api/v1/idempotency/boundary; 未知 404/方法错 405/无 mgr 503）+
  "**零触碰: api_boundary / command / idempotency / runtime_query /
  event_projection / rpc 契约零改动**"。
- ⇒ **任何 switch 命令扩面 = 显式契约修订轮**（本探针即该轮的只读
  部分）; TransportContext 已有 Option 字段先例（query/idem/hls_dir,
  :42-51）——`switch_plane: Option<...>` 沿同一模式。

### 2.4 查询面（runtime_query.rs + api_boundary.rs）

- RuntimeQuery 有 **PUBLIC_SURFACE_ALLOWLIST 硬编码测试**（
  runtime_query.rs:92-99: "新增公开项必须显式更新本清单并过 Pure Read
  评审; **命令动词禁入**"）。
- 查询链: `RuntimeQuery::get_runtime_state → mgr.runtime_state() →
  CanonicalRuntimeState → to_api_query_snapshot → ApiQuerySnapshot`
  （api_boundary.rs:158-172）——**program switch 状态不在
  CanonicalRuntimeState 中**（ProgramExecutionRuntime 未进
  SessionManager 状态面）。
- D14 观察信封: `observation_revision/observation_lineage` additive
  wire 面（api_boundary.rs:167-171）。
- ApiSession wire DTO（api_boundary.rs:73-91: id/state/phase/outputs/
  inputs）——加法扩展点。

### 2.5 执行面签名（已存在·零修改即用）

- `ProgramExecutionRuntime::switch_program(&SwitchIntent) -> Result<
  ProgramSwitchReport, SwitchError>`（program_execution.rs:589）——
  同步全链 ⓪-⑩（fence Arm→锚→Authority 声明→install→switch→
  executed→settle→confirm+Desired 推进）; **teardown 后拒收**
  （:594-597 "runtime 未激活——切换拒收"）。
- `SwitchIntent { target: Uuid, policy: SwitchPolicy }`
  （switch_execution.rs:70-73）; **gates 真机口径 = FrameSwitch**
  （gates/dual_input.rs:705-708）; SwitchPolicy 词表 = PacketSwitch/
  FrameSwitch/MasterSwitch（program/switch_policy.rs:32-39）。
- `observe_execution() -> Option<ProgramExecutionObservation>`
  （program_execution.rs:750-757）——`ProgramObservation`（contracts/
  switch.rs:78-95: observed_active/video_active/audio_active/
  switch_epoch/input_pts[]/program v-a pts+pts_state+frames）+
  `TimelineObservation`（timeline 证据行）——**回读投影的完整数据源
  已存在**。

### 2.6 所有权链（bin/media-agent.rs）

- 诊断自启双输入分支 `:445-517`: `ProgramExecutionRuntime::create`
  → watchdog → `set_watchdog_stop` → **`:513-516 register_stop_hook
  (&sid, Arc::new(runtime))`——runtime Arc 整体 move 进 SessionManager
  的 hook 注册表, bin 不留句柄**。
- TransportContext 装配 `:605-617`: query/idem 由 `api_mgr.as_ref()
  .map(...)` 构造——**Production 保持 None → 503（0.7C-8 契约）**。
- stop_hooks 是 `Mutex<HashMap<SessionId, Arc<dyn ...>>>` 的会话级
  关联表（session.rs:782-798 消费面）——hook trait 只有
  on_session_stopping, **非通用执行句柄**（不可作为 switch 通道借用）。

## §3 缺口 → 插入点映射

| 缺口（审计 §5） | 插入点 | 动作 |
|---|---|---|
| ① switch 触发入口 | command.rs 词表+validate+dispatch; transport.rs vocab+route; idempotency.rs executor 依赖; bin :513 前保留 clone | 新命令面（§4-A） |
| ② switch 状态回读 | transport.rs GET /api/v1/runtime 组装处 + api_boundary 新 DTO | observe_execution 投影（§4-A.3） |
| ③ Production 503 | 不动（v0.2 诊断形态; rpc 接线归 Step 16） | 语义维持 |

## §4 设计提案 A（推荐·最小实现边界）

### A.1 命令面（第四命令）

- `CommandKind::SwitchProgram`（serde `switch_program`）;
  `CommandTarget::SwitchProgram { session_id: SessionId,
  target_device: Uuid }`（API 层同形变体, target_type tag）。
- validate 新臂: SwitchProgram↔SwitchProgram{...}（session 非 nil;
  target_device 必须属于该会话 inputs——**运行期成员校验归执行平面
  非形状层**, 形状层只验 UUID 形）。
- **policy 固定 FrameSwitch**（与 gates 真机口径一致·不暴露 wire
  面——备选见待裁 ②）。
- 执行: dispatch 增第四臂——**不经 SessionManager**, 经新平面:
  `SwitchDispatchPlane` trait（新模块, 无默认实现——沿契约纪律:
  真实实现 = bin 装配的 `ProgramExecutionRuntime` 薄包装; mock 实现
  供命令面测试）: `fn switch(&self, env) -> ...` 内部构造
  SwitchIntent 调 `switch_program`; **会话匹配校验**（plane 持
  SessionId; 不匹配 → Rejected/Failed 分类）。
- CommandIdempotency 增 `switch_plane: Option<Arc<dyn
  SwitchDispatchPlane>>` 字段——None（Production）→ SwitchProgram
  命令走既有 503/not_available 语义; replay/conflict/fingerprint
  语义全量沿 D9（switch 幂等键=command_id 同表）。
- 错误分类: SwitchError → ErrorClassification 映射（Retryable/
  Permanent 沿 error_model 既有分类面; teardown 后拒收=Rejected 类）。

### A.2 接线（bin·非核心四）

- 诊断双输入分支: `Arc::new(runtime)` 前 clone 一份 → 构造
  `RuntimeSwitchPlane { session_id: sid, rt: Arc<...> }` →
  TransportContext/CommandIdempotency 装配; Production 分支不装
  （None→503 契约原样）。
- **v0.2 会话语义 = 单活跃双输入会话**（诊断形态一次一会话·多会话
  映射归 B 案）——如实标注, 不冒充多会话支持。

### A.3 回读面（runtime 投影）

- `GET /api/v1/runtime` 响应增**顶层可选块** `program_switch:
  Option<ApiProgramSwitchState>`（None=无活跃执行平面·诚实缺席）:
  ```
  { session_id, desired_active, observed_active, video_active,
    audio_active, switch_epoch, program_pts_state_v/a,
    program_frames_v/a, timeline: <TimelineObservation 摘要> }
  ```
  数据源 = plane.observe_execution()（已在 §2.5）; 投影在 transport
  组装处合并（runtime_query 零改动——allowlist 不动; **纯读无命令
  动词**·Pure Read 纪律保持）。
- D14 信封: observation_revision 语义不动（additive 块不破坏既有
  字段）; wire 新块**不带 serde default**（沿 ApiQuerySnapshot 既有
  设计决定）。

### A.4 触碰文件清单（预估·实现轮验证）

| 文件 | 动作 | 核心四? |
|---|---|---|
| command.rs | 词表+validate+dispatch 第四臂+allowlist 测试更新 | 否 |
| idempotency.rs | switch_plane 字段+第四臂（fingerprint 语义不变） | 否 |
| api_boundary.rs | ApiCommandTarget 变体+ApiProgramSwitchState DTO | 否 |
| transport.rs | vocab+route+TransportContext+runtime 投影合并 | 否 |
| switch_plane.rs（新） | SwitchDispatchPlane trait+真实/mock 实现 | 否（新文件） |
| bin/media-agent.rs | clone Arc+平面装配（Production None） | 否 |
| tests | 词表快照/validate 矩阵/mock plane dispatch/路由/投影 | — |

**核心四文件零行改动**（§1 承诺; 实现轮以 git diff 复核入账）。

## §5 备选案与取舍

- **B（完整面·后置）**: SessionManager 增会话级执行平面注册表
  （session.rs 改动·N 会话 switch 标准化）+ SwitchExecuted 投影事件
  （events 面）+ Production rpc 接线——**归 Step 16 Control Plane
  联调轮**; v0.2 不做（爆炸半径大·A 案数据已够 Step 14 业务测试）。
- **C（旁路诊断端点·不推荐）**: POST /api/v1/diagnostic/switch 独立
  面——不动命令词表但**制造第二命令面**（绕开幂等/分类/词表纪律）
  ; 仅可作 Step 14 盒上临时驱动, 违反"正式进入 v0.2"的用户指令
  语义, **不推荐**。

## §6 测试面提案（实现轮验收线）

1. 词表快照测试更新（command.rs allowlist + KIND_VOCAB 串）——
   显式过架构评审的动作即测试更新本身。
2. validate 矩阵新臂（形状匹配/失配/nil 拒绝）。
3. mock plane dispatch 测试: Executed/replay/conflict/teardown 拒收
   /会话不匹配分类。
4. transport 路由测试: POST switch_program 200/400/503（无 plane）;
   GET runtime 投影块有/无。
5. 投影等值测试: observe_execution → ApiProgramSwitchState 字段
   逐项（含 None 诚实缺席）。
6. 盒上（Step 14 预演验收线·preview v0.2 脚本）: 启动 → A→B（API
   发起）→ 回读 observed_active=B·epoch 单调 → B→A → 回读 →
   输入异常/恢复一轮 → teardown——**与 R59 v0.1 冒烟同一运行形态**。

## §7 实现边界与待裁清单

**工作量预估（A 案）**: 单轮代码单元——7 文件面（§4-A.4）+ 测试
~6 组 + 盒上验证轮; 无 Domain/契约语义变更（命令面加法扩展·经
架构评审通道）。

**待验收层裁决（六点）**:

1. **命令面归属**: A（扩 CommandKind·正式词表面）✅推荐 vs C
   （旁路诊断端点）。
2. **payload 面**: target_device 单字段 + policy 固定 FrameSwitch
   ✅推荐 vs policy 入 wire（Packet/Master 语义未在真机验证过——
   暴露即承诺）。
3. **回读块形状**: 顶层 `program_switch` 可选块 ✅推荐 vs 嵌入
   sessions[].program（顶层与"单会话 v0.2 语义"一致）。
4. **多会话**: v0.2 单活跃会话如实标注 ✅推荐 vs B 案注册表。
5. **events 面**: 不加切换事件（最小）✅推荐 vs 加 SwitchExecuted
   投影（events 词汇面变更·归 B/16 轮）。
6. **Production 语义**: 503 维持（switch_plane=None）✅推荐——
   rpc 接线归 Step 16。

## §8 红线重申

- 核心四文件零修改（R57 §23.4/§24.2 冻结; force_release 语义
  禁改）; 本探针零代码零生产文件改动。
- transport.rs 自declared 五端点纪律的修订 = 本轮显式裁决对象
  （不偷偷扩）。
- 词表快照测试 + 架构评审 = 新命令的必经通道（command.rs:23 原文
  义务）。
- 盒上/CI 口径分述; 探针结论=源码审计（本地仓库 29cef9c）。

---

登记: tasks item-7 探针段 + 主账 §89 + 记忆。交付后停——待验收层
对 §7 六点裁决; 实现轮按裁决边界另启。

---

## §9 R61 验收裁决回执 + 实现记录（2026-09-06）

### 9.1 六点裁决回执（用户 R61 终裁·逐字入册）

| # | 项目 | 裁决 | 冻结规则 |
|---|---|---|---|
| 1 | 命令面归属 | ✅ A 正式扩词表 | CommandKind::SwitchProgram 第 4 正式命令; 必须同步词表快照测试 |
| 2 | Payload | ✅ 单字段 target_device + 固定 FrameSwitch | Packet/Master 不暴露 wire; 未经真机验证的能力不进公共契约 |
| 3 | 回读 | ✅ GET /api/v1/runtime 顶层 program_switch | 数据源唯一绑定 observe_execution; 不污染 sessions[] |
| 4 | 多会话 | ✅ v0.2 单活跃会话 | 明示语义不伪装; 注册表/Event/RPC 后置 Step 16 |
| 5 | Events | ✅ 本轮不新增切换事件 | 不加 SwitchExecuted; 事件化归 Step 16 |
| 6 | Production | ✅ 继续 503 | 最小 Control Plane 接线; 不提前宣称 Production Control Plane 完成 |

- 硬边界: 允许 = command/idempotency/api_boundary/transport/新
  switch_dispatch_plane 模块/bin/media-agent/对应词表契约单测;
  **禁止 = switch graph / program execution / switch 契约 / switch mock
  （核心四·A2-8 Fence/Timeline/Epoch 冻结）**。
- 架构纪律（原文）: "Command Plane 负责'请求执行', Query Plane 负责
  '事实回读', 两者不互相越界。"
- Step 14 解锁条件: 实现过自身契约门禁后, 真实服务进程内
  API→runtime→switch→observed readback 完整闭环——**不再接受"gates
  能切换"作为替代**。

### 9.2 实现记录（单轮代码·工作树=7978250 基线）

- **新模块 `switch_dispatch_plane`**: 双 trait 类型级隔离
  （`SwitchDispatchPlane` 命令面 / `SwitchReadbackPlane` 查询面——
  白盒测试锁 Fake 只进命令面）; `RuntimeSwitchPlane` 真实实现
  （switch_program 薄包装·policy 固定 FrameSwitch·会话不匹配→
  Failed/Permanent）; `classify_switch_error`（状态机类→Permanent·
  Backend→Unknown 不臆造）; **outcome=Failed{reason} → 命令如实
  Failed**（不把降级切换包装成 Executed——04-探针 P2 语义）。
- **command.rs**: 词表四命令（快照测试显式更新=架构评审动作）·
  validate 第四臂（nil 双字段）·dispatch 签名增
  `switch_plane: Option<&dyn …>`（平面缺席→Rejected
  "switch_plane_unavailable"）。
- **idempotency.rs**: `switch_plane` 字段 + `with_switch_plane`
  builder; replay/conflict 与会话命令同表同律（能力拒绝过形状→
  占 id·outcome=Rejected·重放同果——与形状拒绝[不占 id]分层）。
- **api_boundary.rs**: target 变体 + `ApiProgramSwitchState`/
  `ApiTimelineEvidence` DTO + `to_api_program_switch`（enum_tag
  snake_case 统一·诚实缺席透传）; ApiQuerySnapshot 增
  `program_switch`（canonical 恒 None·transport 组装处合并）。
- **transport.rs**: 模块头显式契约修订注记（非静默扩面）·vocab
  四词·map 双 UUID 解析·`apply_program_switch` 合并（平面缺席/
  teardown→块缺席）·query None 仍 503（平面不越权）。
- **bin**: 双输入分支 Arc 化 runtime（clone 先于 move 进 hook
  注册表——探针 §2.6 所有权修复）·同一 Arc 分进命令/查询两通道·
  Production/单输入/非 gst 构建 None。
- **强制调用点（签名变更机械传导·零语义改动·如实登记超出允许
  清单的 2 文件）**: error_model.rs 测试 ×3 + gates/
  session_lifecycle.rs ×3（补 None 参）; lib.rs ×1（新模块声明行）。
- **核心四零 diff 实证**: `git diff --stat` 四文件 = 0。

### 9.3 验证记录（盒 10.30.15.10·2026-09-06）

- **盒矩阵全绿**: fmt ✓ / default 229 / simulation 229 / mock 405
  （397+8 新）/ **hw 268**（266+2）/ clippy ×3（default/mock/
  bmd,gstreamer·-D warnings）。
- **新 bin**: media-agent md5 `90186bb9`（gates `a93df2b3`——lib
  变更后重构建·预期）。
- **Step 14 闭环全过（真实服务进程内·非 gates）**: 启动→双输入
  起流（watchdog 双路 advancing）→ **program_switch 投影块首次
  兑现**（observed=A·epoch=0·valid_monotonic·帧推进）→ API
  A→B（`executed·av_epoch=1 outcome=preserved timeline_epoch=0`）
  → 回读 observed=B/seg=1/continuous/discontinuity_declared（R53
  冻结签名）→ API B→A（av_epoch=2 preserved）→ 回读 observed=A/
  seg=2 → 错误路径（目标不在组/已是当前源→permanent 分类·
  TargetNotInGroup/TargetAlreadyActive 透传）→ 幂等重放
  （replayed·原 detail 逐字节）→ 冲突（conflict）→ stop_session
  executed → teardown 链（Program Stop→Tap Detach + watchdog
  停止旗）→ 进程死亡。证据 =
  `evidence/bmd-10.30.15.10/r61-v02-step14/`（盒=origin·md5 全过）。
- 登记链: 主账 §90 + tasks item-7 R61 段 + EVIDENCE-INDEX + 记忆。
