# A2-8-05 正常使用形态启动入口全盘审计（Step 13a · 零代码 · R59）

状态: **DELIVERED（2026-09-06）**——R59 用户终裁 A2-8-04 = PASS/CLOSED
（R57 §23）后 A2-8-05 开启的第一个执行面; 收口时序 = 开 PR 跑 CI ·
链末收口（R57 §24.3）; v0.1 = **零代码诊断模式冒烟包**（用户裁决）。
本报告 = Step 13a 审计交付 + v0.1 runbook + v0.2 控制面扩面提案（待
裁决）; 兼作 A2-8-05 前置审查证据面。

授权来源: R59 用户裁决两项（收口时序 / v0.1 形态）+ R57 报告 §24.4
Step 13-17 阶梯。

---

## §1 审计范围与方法

- **问题**: 从 gates 测试二进制到「正常使用形态」（启动→设备发现→
  初始化→Program/Input 配置→正常输出→A↔B 切换→状态回读→异常恢复→
  持续运行）还缺什么。
- **方法**: 全仓只读审计（可执行目标 / 启动链 / 命令面 / 状态面 /
  停止链 / 环境旋钮 / 日志去向）+ 载荷锚点人工复核（bin 诊断自启
  双输入装配段 `:445-517` / watchdog 活体行 `:627-674` / session
  停止链 `:758-817` / transport 命令词表守卫 / command 枚举——五处
  全部核实与探索结论一致）。
- **零代码纪律**: 本审计不修改任何生产源码; A2-8 核心四文件为终裁
  冻结面（R57 §23.4/§24.2）。

## §2 可执行目标盘点

| 目标 | 定义处 | 用途 |
|---|---|---|
| lib `media_agent` | `Cargo.toml:11-13` | Library-first: Domain/Contracts/Runtime/Adapters/Bootstrap/Watchdog/Gates 全在 lib |
| **bin `media-agent`** | `Cargo.toml:15-17` / `src/bin/media-agent.rs` | **生产 Composition Root**: bootstrap→诊断接线→runtime wiring→transport→进程常驻; 对全部 VBMF_* gate env **零 dispatch** |
| bin `media-agent-gates` | `Cargo.toml:19-21` / `src/bin/gates.rs` | 诊断/验收 Root: 七个 VBMF_* gate env 唯一入口, 命中即跑完退出 |

- 真机 canonical 构建 = `--features bmd,gstreamer`（`scripts/build-bmd.sh:31`）;
  **本轮盒上服务 bin md5 = `6a0fa22464f37a4e23c737e1b247e0e0`**
  （源 = 4b473b3 生产面 == e09ed97·核心四文件 SHA 盒==本地全等——
  R57 §24.1 身份链）。
- 仓库根 `media-agent`（13MB·2026-08-26）是 A2-0 归位（09-02）前的
  **过期构建产物, 勿当现役入口**。
- Feature 模型: `default` = filesystem 发现零硬件; `simulation`/`mock`
  = 纯软件; `bmd-provider`/`gstreamer-backend` = 真机（`bmd,gstreamer`
  为 build-bmd.sh 口径）; `hardware-test` 与 `gstreamer-backend` 编译期
  互斥（`src/lib.rs:23-24`）。

## §3 启动链全景（file:line）

`src/bin/media-agent.rs`（除注明外）:

1. 日志初始化 `tracing_subscriber::fmt::init()` — `:23`（stdout·
   RUST_LOG 控制级别）。
2. `bootstrap::build()`（唯一构造源·只构造不运行）— `:27` →
   `src/bootstrap.rs:50-147`: `Config::from_env()`（`src/config.rs:69-104`）
   → rpc_bind 安全校验 → adapter 选择收口（mock>simulation>bmd-provider
   >filesystem·`src/registry.rs:117-136`）→ 设备发现 discover
   （fail-closed）→ 双日志 FanoutSink → LeaseManager+占位租约+排他性
   自检 → Supervisor 注册 → AgentState=Ready。
3. DeckLink SDK FFI probe（诊断接线）— `:53-58`（libloading dlopen·
   无 SDK 仅 warn）。
4. `bmd-provider` 块（`:82-599`）: `MEDIA_AGENT_SELFTEST=1` 自测管线
   `:87-121` → 物化模式（默认 Production·`MEDIA_AGENT_MODE=diagnostic`
   回退）`:152-155` → GStreamer 设备探测 `:157-175` → **manifest
   加载/结构/machine 校验+绑定解析（生产缺 manifest = 失败闭合）**
   `:177-232` → PortRegistry `:243-256` → 共同组合根
   （SharedResourceRegistry → `AdapterRegistry::build_media_adapter_bundle()`
   → `Arc<SessionManager>`）`:268-316`。
5. **诊断 auto-start**（`mode==diagnostic`·`:323-570`）:
   `VBMF_DIAG_INPUTS` 取前 N 个已绑定设备 `:324-359` → 输出物化配置
   `PrototypeOutputConfig::from_env()`（hls/rtmp fail-soft 降级）
   `:371-381` → `GraphRuntimeIntent` `:382-403` → 占位租约让位
   `:417-424` → `mgr.create()`+`mgr.start()` `:425-426`（materialize
   →instantiate→allocate→backend.start）→ **双输入（inputs.len()==2）
   分支 `:445-517`**: `ExecutionGroup::new` `:447` →
   `GStreamerSwitchAdapter::bridged()` `:460` → `TapWiring::for_input`
   `:465-470` → `ProgramExecutionRuntime::create` `:471` →
   `spawn_execution_group_watchdog` `:486` → `set_watchdog_stop` `:505`
   → `mgr.register_stop_hook` `:513`（创建失败=会话整体回滚·不回落
   单输入 watchdog `:438-441`）——**与 gates dual_input/a204_obs 的
   L2 接线逐行同构（同一 lib·无第二实现）**。
6. **Production 分支**（`:578-597`）: 不启动任何媒体·组合根常驻
   （tick 线程 lease 房务）·查询/命令面 503（0.7C-8 生产契约·
   `src/rpc.rs` 冻结待接线）。
7. transport/health 线程 `:619-636`（`MEDIA_AGENT_HEALTH_BIND`·默认
   `127.0.0.1:8080` 仅回环·串行 accept·socket 超时加固）。
8. 主线程常驻 `loop { sleep(3600) }` — `:640-642`（无 daemon 框架·
   无信号处理）。

## §4 运行期面盘点

| 面 | 锚点 | 内容 |
|---|---|---|
| `GET /health` | `transport.rs:195-207` | AgentState(Ready/Capturing/Degraded)/devices/active_pipelines/dropped_bus_events/clock_lost_events |
| `GET /api/v1/runtime` | `transport.rs:208-214` → `runtime_query.rs` | devices/ports/resources/sessions(phase/state/**inputs**/outputs)/capabilities + D14 观察信封——**不含 program switch 状态** |
| `POST /api/v1/commands` | `transport.rs:215-230` | **封闭词表仅 start_session/stop_session/release_session**（`transport.rs:113-127`·`command.rs:28-32`） |
| `GET /api/v1/events/projection` | `transport.rs:231-236` | RuntimeEvent 投影（环形 1024·drain 读取）——无 switch/epoch 事件 |
| `GET /api/v1/idempotency/boundary` | `transport.rs:237-240` | 幂等边界声明 |
| Web Console `GET /` + `/hls/*` | `transport.rs:271-400` | 1s 轮询 health+runtime·Start/Stop 按钮·HLS 播放 |
| watchdog 活体行 | `watchdog.rs:627-674` | 每 20 tick 一行: per-input `observed/advancing/bridge/domain` + `program_advancing`——**A2-8 状态唯一"接近外部可见"面**（epoch/Desired/Observed 的 `observe_execution` 生产无消费者） |

环境旋钮（生产 bin 消费）: `MEDIA_AGENT_RPC_BIND`（UNWIRED）/
`MEDIA_AGENT_DEVICE_ALLOWLIST`/`MEDIA_AGENT_LEASE_TTL_SECS`/
`MEDIA_AGENT_LEASE_RENEW_SECS`/`MEDIA_AGENT_HEALTH_POLL_SECS`/
`MEDIA_AGENT_MAX_RECOVER_ATTEMPTS`/`MEDIA_AGENT_DEVICE_BINDING`
（**生产绑定唯一权威**）/`MEDIA_AGENT_HEALTH_BIND`/`MEDIA_AGENT_MODE`
/`MEDIA_AGENT_SELFTEST`; demo 层 `VBMF_DIAG_INPUTS`/
`VBMF_OUTPUT_KIND`/`VBMF_OUTPUT_HLS_DIR`/`VBMF_OUTPUT_RTMP_URL`/
`VBMF_OUTPUT_V_BITRATE_KBPS`/`VBMF_OUTPUT_A_BITRATE_BPS`。
（七个 VBMF_* gate env 仅 gates bin 消费——服务 bin 零 dispatch。）

## §5 缺口表（八条·严重度排序）

| # | 缺口 | 现状锚点 | 对测试包的影响 |
|---|---|---|---|
| 1 | **A↔B 切换无外部触发入口**: 无 switch 命令 kind、无 stdin/信号/CLI; `switch_program` 调用方 = gates 两 gate + lib 测试 | `transport.rs:113-127` 封闭词表; `program_execution.rs:589` | 正常使用形态的"实际切换"一步无法执行——**最大单点缺口** |
| 2 | **switch 状态无回读面**: runtime 查询无 program/epoch/active 字段; `observe_execution` 生产无消费者; events 无 switch 事件 | `api_boundary.rs:74-91`; `program_execution.rs:750` | Desired/Observed/epoch 只能靠 watchdog 活体行间接读 |
| 3 | **Production 模式命令面 503**: Control Plane RPC（rpc.rs）冻结未上线 | `bin/media-agent.rs:578-597` | 正常形态实际只能跑 `MEDIA_AGENT_MODE=diagnostic` |
| 4 | **无优雅停止/信号处理**: 无 ctrlc/signal_hook 依赖; 进程级停止 = kill; 会话级停止 = stop_session 完整链 | `bin/media-agent.rs:640-642`; `session.rs:758-817` | 测试包需配启停脚本; SIGTERM 语义缺失如实登记 |
| 5 | **program 输出无物化合流**: program graph 出口 = appsink 观测面（fence+帧计数/PTS）; HLS/RTMP 物化挂输入管线; 二者未合流 | `switch_graph.rs:659-697`; `bin/media-agent.rs:384-401` | 「输出」验收口径须先定义（观测面推进 or 输入管线物化） |
| 6 | 无版本标识输出（无 --version/启动版本行; 仅 gst_version 证据行） | `bin/media-agent.rs:428` | 以打包面 IDENTITY 文件替代并如实标注 |
| 7 | 无正常形态 runbook/启停脚本（现有根目录脚本均 gate/验证脚本） | 仓库根 | v0.1 打包面新增（零生产源码改动） |
| 8 | 日志仅 stdout（RUST_LOG）·无文件/滚动 | `bin/media-agent.rs:23` | 取证自重定向到日志文件 |

## §6 v0.1 定义（零代码·用户裁决形态）

- **形态**: 诊断模式真实服务 bin——`MEDIA_AGENT_MODE=diagnostic` +
  `VBMF_DIAG_INPUTS=2` + `MEDIA_AGENT_DEVICE_BINDING=<manifest v5>` +
  `RUST_LOG=info`。
- **冒烟验收线**: 进程起+身份（IDENTITY 文件+bin md5）→ 两路 BMD
  发现 → 双输入 program graph 起流（watchdog 活体行两路 advancing）→
  `GET /health` 200 → `GET /api/v1/runtime` sessions·inputs=2 →
  `GET /api/v1/events/projection` 非空 → `POST stop_session` →
  teardown 链日志（hook: Program Stop→Tap Detach **先于** Input Stop
  → 逆序 Backend.stop → allocation/lease/reservation 归还 → Released
  ·零孤儿）→ kill 常驻进程（缺口④如实: 无信号处理·进程停止=kill）。
- **已知限制（如实标注·不冒充）**: 不含 A↔B 切换与 switch 状态回读
  （缺口①②·归 v0.2 控制面轮）; 输出 = 观测面推进 + 输入管线物化
  （缺口⑤按现状呈现）; 无版本行（缺口⑥以 IDENTITY 替代）; 日志 =
  stdout 重定向（缺口⑧）。
- **打包面**: `preview/a2-8-05-v0.1/`（README=runbook + start.sh/
  stop.sh + env.sample + IDENTITY——零生产源码改动·新增文件如实登记）。

## §7 v0.2 控制面扩面提案（待验收层裁决·本轮零实现）

- **命题一（切换入口）**: transport 命令词表扩第四命令
  `switch_program`（CommandKind 增枚举值·目标 = SessionById·载荷
  carried target input）——或备选: 专用诊断切换端点（不扩命令词表）。
- **命题二（状态回读）**: `/api/v1/runtime` 投影 `observe_execution`
  （Desired/Observed/epoch/active input/v-a continuity 摘要）。
- **纪律约束**: `command.rs:23` 明言新命令须过**架构评审 + 词表
  快照测试显式更新**; **零触碰 A2-8 核心四文件**（R57 §23.4 红线
  ——扩面在 transport/command/api_boundary 层·不进 switch graph/
  program execution/契约/mock）; Production 503 契约（0.7C-8）与诊断
  直切的两路径语义需一并裁决; Mock/GStreamer 双实现方表态义务
  （契约 trait 无默认实现的既有纪律沿用到新命令面）。
- **实现规模预估**: 词表+映射+dispatch+快照测试+runtime 投影字段+
  双路径语义测试——单轮代码单元量级。

## §8 审计结论

- **数据面已具备**: 发现→双输入→program pipeline→切换核心→teardown
  在两个 A2-8 gate 已被同一 lib 代码验证（Final Gate PASS）; 真实
  服务 bin 诊断自启已装配同一条链——"能跑"成立。
- **缺的是控制面三件套 + 运维件**: switch 命令入口（缺口①）、
  switch 状态回读端点（缺口②）、Production Control Plane transport
  （缺口③）; 外加优雅停止（④）/输出物化口径（⑤）/版本行（⑥）/
  runbook（⑦）/日志落盘（⑧）。
- **v0.1（零代码）** 验证真实运行入口的启动/观测/停止闭环——本周
  盒上冒烟执行; **切换入口归 v0.2 控制面轮**（§7 提案待裁决）——
  与验收层 Step 16（控制面/Transport 联调在阶梯后段）排序一致。
- CI/盒上口径分述: 本报告全部结论 = 仓库源码审计 + 盒上构建/冒烟
  （本地验证口径）; GitHub CI 信号 = PR 通道（R59 开 PR·结果另行
  登记）·互不替代。

---

登记: R57 §23/§24 + 谓词文档头+§11 尾 + 04-探针 §18 + 主账 §88 +
tasks item-6 勾选/item-7 解锁注。

**同轮执行结果（2026-09-06 15:0x CST）**: v0.1 盒上冒烟两跑——run1
留证（stop_session 400 = 打包脚本 UUID 展示形缺陷·形状层拒绝未触
Runtime·与设计一致; run1 其余全过）; 修正脚本后 **run2 全绿**: health
Capturing·devices=3·active_pipelines=3·零 dropped/clock_lost·双输入
program graph 起流（两路 observed/advancing/bridge 全 true·
program_advancing=Some(true)）·events total=15（has_critical=false）·
stop_session **executed**·teardown 链日志（Program Stop→Tap Detach +
group watchdog 停止旗·观测线程退出）·进程死亡（pgrep 空）。工件 =
既有已知类（gst_pad_unlink×3/@teardown·gst_video_converter_free×1/
@startup·ERROR=0）。证据入库 `evidence/bmd-10.30.15.10/a2-8-05-v0.1-
smoke/`（md5sum -c 全过·盒=origin）; PR/CI 结果 = 主账 §88 执行
记录段。
