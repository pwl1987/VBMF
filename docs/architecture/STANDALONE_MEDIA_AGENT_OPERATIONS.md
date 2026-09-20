# Standalone Media Agent Operations — Readiness/Liveness · 配置权威清单 · 退出码契约

> **STATUS（DOCUMENT_STATUS_MODEL）**: CONTRACT_STATUS = FROZEN（STANDALONE-ENTRY-01 冻结设计 S4/S8/S10 的文档面；S4 readiness 判据经 §3 reconciliation 收敛，见 STATE §3.63）· IMPLEMENTATION_STATUS = IMPLEMENTED（本文全部语义逐条来自 live 代码，非目标设计）· VERIFICATION_STATUS = LAB_VERIFIED（code-to-doc cross-check；shutdown/exit 语义另有 BMD runtime smoke，STATE §3.62）
> **Lane**: Standalone lane（单 `media-agent` 进程 + systemd + environment file + `/health`）。`compatible, not dependent`：不依赖 DB / Fastify / Web Console / SRS / Node 层；Runtime owns truth 不变。full-stack lane（Deployment SoT §1–§14）不受影响，`ops/` 占位不动。
> **代码权威**: 本文每条语义以标注的源文件为准；文档与代码冲突时以代码为准并回改本文。install/upgrade/rollback、systemd unit 落地属 SE-01B，本文只钉语义。

---

## 1. `/health` 契约（as-is，零变化）

`transport.rs::route`（`("GET", "/health")`）固定返回 **HTTP 200** + 五字段 JSON：

```json
{"state":"<AgentState>","devices":<usize>,"active_pipelines":<usize>,"dropped_bus_events":<u64>,"clock_lost_events":<u64>}
```

- `state` = `AgentState`（`health.rs`，serde 默认 PascalCase 序列化：`Starting`/`Ready`/`Capturing`/`Degraded`/`Restarting`/`Backoff`/`Escalated`/`ManualRequired`）。
- `devices` = TransportContext 注入的 device_count（Network-only 恒 0；Device 模式为 discovery 计数）。
- `active_pipelines` = `pipeline_events::HEALTH_ARCS` 长度；`dropped_bus_events`/`clock_lost_events` = pipeline 观测计数器。
- 监听：`Config::health_bind`，默认 `127.0.0.1:8080`，`MEDIA_AGENT_HEALTH_BIND` 覆盖；仅内网/回环，生产暴露必须经反向代理 + 认证（用户 §二十二 P1 Security）。
- **wire shape 冻结（P0.7C-8）**：SE-01C 未新增 `/ready`、`/live`，未改字段/casing。未来若需独立 readiness endpoint，必须先过 EXTERNAL_API_CONTRACT review。
- 诚实披露：health 线程 `TcpListener::bind` 失败仅 `tracing::error!` 记日志、进程继续运行（`media-agent.rs` health thread）——即 **liveness 探测失败 ≠ 进程死亡**；运维判断需结合 systemd 主进程状态。

## 2. Liveness / Readiness 语义

- **Liveness** = 进程存活（systemd 视角主进程在）且 `/health` 可建立 TCP + HTTP 应答。**收到 HTTP 200 本身不是 ready**（`/health` 无条件 200）。
- **Readiness** = 解释 `/health.state`：**`state ∈ {Ready, Capturing}` 即 ready**；其余六态一律 live-but-not-ready。
- 消费方（systemd readiness / 上层编排）正确姿势：轮询 `GET /health`，解析 `state` 字段判定；连接失败区分"进程死"与"health 未监听"两种情况（后者见 §1 披露）。

### 2.1 八态矩阵

| state | process live? | operationally ready? | 生产可达性（现实现） | 操作员含义 | systemd / readiness consumer 动作 |
|---|---|---|---|---|---|
| `Starting` | 是 | **否** | 是（构造期真实置位：`bootstrap::build()` / network-only 入口） | 组合根构造中（discovery/manifest/租约装配） | 等待；超时按 unit 配置判启动失败 |
| `Ready` | 是 | **是** | 是（bin 装配完成显式晋升；会话归零重置边回落） | 构造完成、零活跃会话、可受命 | ready=true；正常路由新工作 |
| `Capturing` | 是 | **是** | Device 命令服务路径可达（`SessionStateChanged→Running`/`SignalVerified`/`recovered` 派生） | 在播/在采，正常服务态 | ready=true（这是正常 on-air 态） |
| `Degraded` | 是 | **否** | Device 命令服务路径可达（`PipelineFault`/`HardwareFault`/`SessionFailed`/`AmbiguousIdentity`/`ResourceReservationExpired` 经 `spawn_ingest_watchdog` 折叠） | 存在未复位故障（恢复排程或待 escalate），存量会话可能仍在跑 | ready=false（新工作别路由进来）；告警；等 `recovered`/会话归零回落 |
| `Restarting` | 是 | **否** | **当前无生产者**（冻结词表在册；`begin_restart` 不发事件，`health.rs` 头部登记） | 预留：恢复执行中 | ready=false；等待，不需要人工 |
| `Backoff` | 是 | **否** | **当前无生产者**（同上登记） | 预留：重启退避窗口 | ready=false；等待退避窗结束 |
| `Escalated` | 是 | **否** | **当前无 /health 构造点**（Supervisor `Escalated` 经唯一出口发 `HealthChanged{to:"manual_required"}`，折叠后呈现为 `ManualRequired`） | 预留：已升级 | ready=false；按 `ManualRequired` 处理 |
| `ManualRequired` | 是 | **否** | Device 命令服务路径可达（Supervisor Escalate 唯一出口 → `HealthChanged{to:"manual_required"}`，最高优先级） | 恢复预算耗尽/不可恢复，需人工介入 | ready=false；**告警并等人工**，不要自动重试同一路径 |

词表冻结于 SoT §15.2；`Restarting`/`Backoff`/`Escalated` 的"无生产者"登记是 `health.rs` 模块头部的既有诚实记录——上表为它们预先钉死判据，未来 watchdog 演进接上生产者时**继承本表语义，无需重议**。

### 2.2 S4 reconciliation（记录，不改 wire）

冻结计划 S4 原句"构造完成且非 `Starting`/`ManualRequired` 即 ready"会把 `Degraded`/`Restarting`/`Backoff`/`Escalated` 含糊计入 ready。与现行 Runtime health/recovery 权威（`health.rs` 优先级格 `ManualRequired=Escalated > Degraded > Restarting=Backoff=Capturing > Ready`；Supervisor Escalate→人工介入语义）对齐后收敛为：**ready ⟺ `Ready` | `Capturing`**。此为判据收紧（更保守），非 wire/状态机变更；已记录于 STATE §3.63。

### 2.3 按启动路径的可达性（如实）

- **Network-only production**（`MEDIA_AGENT_NETWORK_BINDING`）：`state` 现实现仅 `Starting`→`Ready`（bin 直接管理；`projection_log` 上**尚未接线** fold 消费者，网络源故障的 Degraded 派生属后续 watchdog 演进项，与 `health.rs` 登记一致）。ready 判定不受影响：`Ready` 即 ready。
- **Device production**（gstreamer-backend 命令服务路径）：`spawn_ingest_watchdog`/`spawn_execution_group_watchdog` 以当前 `agent_state` 为初值折叠事件流并写回——`Ready`/`Capturing`/`Degraded`/`ManualRequired` 真实可达。

## 3. 退出码契约

### 3.1 `media-agent`（standalone production binary）

| 退出码 | 语义 | 真实 callsites |
|---|---|---|
| `0` | 优雅停止：SIGTERM/SIGINT → drain（SessionManager 唯一 owner 逆序停会话，RecoveryMonitor/listener 回收随 stop 链）→ 完成后退出 | `media-agent.rs` device 路径与 `run_network_only_runtime` 末尾（SE-01A） |
| `2` | fail-closed 启动/构造拒绝（env/manifest/组合非法，进程拒绝以非法形态常驻） | ① `GracefulShutdown::install` 失败（无信号保护不常驻）② startup composition 模式选择失败（`MEDIA_AGENT_NETWORK_BINDING`×`MEDIA_AGENT_DEVICE_BINDING` 互斥 / ×diagnostic / ×selftest）③ `bootstrap::build()` 内 adapter feature 冲突 ④ `bootstrap::build()` 内 discovery 失败 ⑤ selftest 路径 adapter 冲突 ⑥ FFmpeg production composition 构造失败（device 模式）⑦ adapter bundle 冲突 ⑧ network-only 于非 ffmpeg-backend 构建 ⑨ network-only composition 构造失败（含 NetworkSourceBinding 载入拒绝） |

- **非契约值**：未捕获 Rust panic = 101（默认展开语义），属缺陷而非契约；SIGKILL/崩溃由 systemd 归类。
- **第二次终止信号**：第一个 SIGTERM/SIGINT 后 handler 已恢复默认处置——第二个信号是**运维逃生门**，走 OS 默认信号终止（systemd 记录为 signal death），**不是** exit 0。
- **SIGHUP**：仅 warn，不 reload（manifests startup-only，frozen D3）。
- systemd 对接：`2` = 配置/构造类拒绝，重试无意义直至 env/manifest 修复——unit 的 `Restart`/`StartLimitBurst` 设计须把它与运行期崩溃区分（实现属 SE-01B）。

### 3.2 `media-agent-gates`（acceptance tooling，**不属于** standalone service 契约）

`0` = gate 断言全 PASS；`1` = gate 断言 FAIL（`fail()`）；`2` = 未命中任何 gate env（usage）或公共 bootstrap fail-closed。不得与 production unit 的退出码混读；gate 进程永不进入生产 runtime 常驻。

## 4. 配置权威清单（从代码枚举）

原则：**默认列 = `Config::default()` / 代码 fallback**；"startup-only" = 仅进程启动时读取一次，改后需重启（无热加载、无 SIGHUP reload）。

### 4.1 Group A — production `MEDIA_AGENT_*`（`config.rs::Config::from_env`）

| 变量 | 消费位置 | 语义 | 默认 | production 允许 | 缺失/非法行为 | startup-only |
|---|---|---|---|---|---|---|
| `MEDIA_AGENT_NETWORK_BINDING` | `bootstrap.rs` 模式选择 + `build_ffmpeg_network_only_composition` | NetworkSourceBinding manifest 路径；显式选择 Network-only 组合根（先于一切 Device 构造） | 无 | **是（Network-only 必需）** | 缺失→Device 模式；与 DEVICE_BINDING/diagnostic/selftest 组合→exit 2；manifest 无效（权限/owner/machine-pin/大小）→exit 2 | **是**（startup-only，无 watch/reload） |
| `MEDIA_AGENT_DEVICE_BINDING` | `bootstrap.rs`/`resolver.rs` | DeviceBindingManifest 路径；Device 生产绑定唯一权威 | 无 | **是（Device 生产必需；缺失即 fail-closed，无盲猜回退）** | 生产缺失/无效→构造拒绝 exit 2；仅 `MEDIA_AGENT_MODE=diagnostic` 允许回退 legacy auto-resolver | 是 |
| `MEDIA_AGENT_HEALTH_BIND` | bin health 线程 | `/health` 监听地址 | `127.0.0.1:8080` | 是（限内网/UDS；公网暴露禁止） | 缺失→默认；bind 失败→error 日志、进程继续（§1 披露） | 是 |
| `MEDIA_AGENT_RPC_BIND` | `config.rs` → bin internal control 线程（RCE-01A 起已接线） | internal Runtime Control `/internal/v1/agent` JSON-RPC 监听（§6；须 localhost/UDS） | `127.0.0.1:50051` | 是（仅生产组合根启动该面；诊断路径不启动） | bind 失败→error 日志、进程继续（与 health bind 同语义）；`0.0.0.0`/`::` → 启动告警（P1-2 校验） | 是 |
| `MEDIA_AGENT_DEVICE_ALLOWLIST` | `config.rs` | 设备节点 allowlist（逗号分隔） | `["/dev/blackmagic"]` | 是 | 缺失→默认；空串过滤后为空列表 | 是 |
| `MEDIA_AGENT_LEASE_TTL_SECS` | `config.rs`→LeaseManager | 默认租约 TTL | 300 | 是 | 非法→回退默认（fail-soft） | 是 |
| `MEDIA_AGENT_LEASE_RENEW_SECS` | `config.rs` | 续约窗口 | 30 | 是 | 非法→回退默认 | 是 |
| `MEDIA_AGENT_HEALTH_POLL_SECS` | `config.rs`→Supervisor | 健康轮询间隔 | 5 | 是 | 非法→回退默认 | 是 |
| `MEDIA_AGENT_MAX_RECOVER_ATTEMPTS` | `config.rs`→Supervisor RestartPolicy | 恢复预算（耗尽→Escalate→`ManualRequired`） | 5 | 是 | 非法→回退默认 | 是 |
| `MEDIA_AGENT_MODE` | `bootstrap.rs`/`registry.rs` | 模式标注；`diagnostic`=允许 legacy resolver 回退与 mock 共存；`simulation`=允许 mock | 未设（记 `production`） | 谨慎（diagnostic 放宽 fail-closed 面） | 与 NETWORK_BINDING 组合→exit 2 | 是 |
| `MEDIA_AGENT_SELFTEST` | bin（gstreamer-backend） | 启动自测管线（device 面） | 未设 | 否（测试/排障） | 与 NETWORK_BINDING 组合→exit 2 | 是 |

### 4.2 Group B — production 输出 `VBMF_OUTPUT_*`（`config.rs::PrototypeOutputConfig`；P1a demo 层，**显式不进 Runtime Contract**，物化时覆盖 sink）

| 变量 | 消费位置 | 语义 | 默认 | production 允许 | 缺失/非法行为 | startup-only |
|---|---|---|---|---|---|---|
| `VBMF_OUTPUT_KIND` | materialize 输出分支 | 覆盖 intent sink kind（`hls`/`rtmp`） | 无（不覆盖） | 是 | 缺失→intent 原值；含空白/`!`/`"`→按未设置（注入卫生） | 否（随会话物化读取） |
| `VBMF_OUTPUT_HLS_DIR` | 同上 | HLS 分片目录（kind=hls 必需，绝对路径） | 无 | 是 | 同上卫生规则 | 否 |
| `VBMF_OUTPUT_RTMP_URL` | 同上 | RTMP 推流地址（kind=rtmp 必需） | 无 | 是 | 同上 | 否 |
| `VBMF_OUTPUT_V_BITRATE_KBPS` | 同上 | 视频码率 | 6000 | 是 | 非法→默认 | 否 |
| `VBMF_OUTPUT_A_BITRATE_BPS` | 同上 | 音频码率 | 128000 | 是 | 非法→默认 | 否 |

### 4.3 Group C — standalone/system 身份与运行环境

| 变量 | 消费位置 | 语义 | 默认 | production 允许 | 缺失/非法行为 | startup-only |
|---|---|---|---|---|---|---|
| `VBMF_MACHINE_ID` | `resolver.rs::current_machine_id` | 主机身份（两类 binding manifest 的 machine-pin 校验值）；**显式覆盖** `/etc/machine-id`（测试/容器用） | 无（回落 `/etc/machine-id`） | 是（standalone 可显式设置，否则用系统权威） | 缺失/空白→`/etc/machine-id`；两级均缺→空串：NetworkSourceBinding 拒绝（`MachineIdUnresolved`）、DeviceBindingManifest 跳过 pin 校验（空串永不匹配清单 pin） | 是 |
| `/etc/machine-id`（文件，非 env） | `resolver.rs::current_machine_id_from` | 生产权威主机身份（内容 trim；纯空白视为未解析出） | 系统文件 | **是（生产默认来源）** | 缺失/空白→空串（消费点按上行列既有语义处置） | 是 |
| `LD_LIBRARY_PATH` | `resolver.rs::resolve_decklink_lib` | libDeckLinkAPI.so 候选路径（诊断解析，best-effort） | 系统 | 是 | 缺失→固定候选列表 | 否（诊断面） |
| `VBMF_ALLOW_MOCK` | `registry.rs::test_mode_allows_mock` | 显式允许 mock+真实适配共存 | 未设 | **否**（放宽适配冲突防线） | 未设+冲突→fail-closed 拒启 | 是 |

### 4.4 Group D — `media-agent-gates` 专属 acceptance env（**一律不得进入 production service unit**）

消费入口均为 `bin/gates.rs` dispatch 或 gate 模块；命中即运行验收并退出，进程不常驻。production unit 出现下列任一变量 = 配置错误。

| 变量 | 消费位置 | 语义 |
|---|---|---|
| `VBMF_CONFIG_PROBE` | `gates/config_probe.rs` | 配置探测 gate |
| `VBMF_RESOLVER` | `gates/resolver.rs` | 设备解析原始探测 gate |
| `VBMF_REGISTRY_ONLY` | `gates/registry.rs` | 注册表 verbose 探针 gate |
| `VBMF_LOOPBACK` / `VBMF_LOOPBACK_FRAMES` / `VBMF_FIXTURES_DIR` | `gates/loopback.rs` | BMD-SDI loopback fixture gate 及参数 |
| `VBMF_SESSION_LIFECYCLE` | `gates/session_lifecycle.rs` | 会话生命周期 gate |
| `VBMF_FFMPEG_SESSION` / `VBMF_FFMPEG_RECOVERY` / `VBMF_FFMPEG_TEST_DEVICE_HANDLE` | `gates/ffmpeg_session.rs` / `ffmpeg_recovery.rs` | FFmpeg 会话/恢复 gate 与目标设备 |
| `VBMF_FFMPEG_OUTPUT` / `VBMF_FFMPEG_OUTPUT_DIR` | `ffmpeg_recovery.rs` | FFmpeg HLS 输出验收 |
| `VBMF_FFMPEG_RTMP_OUTPUT` / `VBMF_FFMPEG_RTMP_URL` | `ffmpeg_recovery.rs` | FFmpeg RTMP 输出验收 |
| `VBMF_FFMPEG_RTMP_SOURCE` / `VBMF_FFMPEG_RTMP_SOURCE_URL` / `VBMF_FFMPEG_RTMP_SOURCE_LAN` / `VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR` / `VBMF_FFMPEG_RTMP_SOURCE_EXTERNAL` | `ffmpeg_recovery.rs`（network-only gate，先于公共 bootstrap dispatch） | RTMP 网络源验收（loopback/LAN/外部推流腿；HLS fixture 限 temp root 直接子目录） |
| `VBMF_A2_8_DUAL_INPUT` / `VBMF_A2_8_04_OBS` / `VBMF_A2_8_R64_CP` / `VBMF_A2_8_S15E01_RACE` | `gates/dual_input.rs` / `a204_obs.rs` / `r64_control_plane.rs` / `s15e01_race.rs` | A2-8 族验收 gate |
| `VBMF_FFMPEG_BINDING_MANIFEST` | `adapters/ffmpeg.rs`（`#[ignore]` 硬件测试） | 授权 BMD manifest 路径（仅 ignored 测试） |
| `TMPDIR`（隐式） | `std::env::temp_dir()`（gate fixture 路径根） | gate manifest/HLS fixture 的 canonical 根 |

### 4.5 启动模式选择规则（fail-closed 汇总）

1. `MEDIA_AGENT_NETWORK_BINDING` 与 `MEDIA_AGENT_DEVICE_BINDING` **互斥**：同设 = 歧义 → exit 2。
2. `MEDIA_AGENT_NETWORK_BINDING` 存在时进程**永不**进入 Device discovery / bootstrap 占位 DeviceLease / DeckLink 路径（选择先于 `bootstrap::build()`）。
3. `MEDIA_AGENT_NETWORK_BINDING` × `MEDIA_AGENT_MODE=diagnostic` → exit 2；× `MEDIA_AGENT_SELFTEST` → exit 2。
4. NetworkSourceBinding **startup-only**：启动加载一次（0600/owner/machine-pin/D5 地址归属校验 fail-closed），无热加载、无 watch、**SIGHUP 不 reload**；变更 = 重启服务。
5. machine identity（SE-01D 已实施）：`VBMF_MACHINE_ID`（显式覆盖，trim，空白视为未提供）> `/etc/machine-id`（生产权威，trim）> 空串。空串处置按消费点既有 fail-closed 语义：NetworkSourceBinding 拒绝；DeviceBindingManifest `check_machine_identity` 跳过且永不匹配。**`HOSTNAME` 不再被读取**（fallback 已移除，行为变化——既有以 HOSTNAME 值 pin 的清单必须重 pin 或改设 `VBMF_MACHINE_ID`）。

## 5. systemd / readiness consumer 参考（语义层；unit 实现属 SE-01B）

- ExecStart 指向版本化安装路径；`EnvironmentFile=-/etc/vbmf/media-agent.env`（缺失容忍）。
- readiness = 轮询 `/health` 至 `state ∈ {Ready, Capturing}`；超时窗口按 Starting 上界设定。
- `exit 2` = 启动拒绝：不自动重试直到 env/manifest 修复（`Restart=on-failure` + `StartLimitBurst` 受控退避的判别输入）。
- 第二终止信号 = 逃生门（signal death）；常规停止预期 exit 0。

## 6. Internal Runtime Control 面（RCE-01A，2026-09-20 起）

> Authority: `docs/superpowers/plans/2026-09-20-runtime-control-entry-01-planning.md`（D1–D8/R1–R5）。
> 本节为 as-is 实现记录；`EXTERNAL_API_CONTRACT.md` / `TECHNOLOGY_STACK_AND_RUNTIME_OWNERSHIP.md` 零修改。

- **面与命名空间**: `POST /internal/v1/agent`，JSON-RPC 2.0 envelope（`jsonrpc/method/params/id`）。
  生产组合根（Device / Network-only）启动时经 `MEDIA_AGENT_RPC_BIND` 监听（默认 `127.0.0.1:50051` 回环；
  localhost/UDS 纪律，见 §4.1 行与用户 §二十二 P1-2）。诊断路径不启动本面。
- **四方法封闭词表**: `runtime.query` / `command.dispatch` / `events.projection` / `agent.health`。
  未知 method → JSON-RPC error `-32601`（附词表）；缺 method/参数形状错 → `-32602`；`jsonrpc` 版本错 → `-32600`；
  非 JSON → HTTP 400；组合根未装配 → HTTP 503（诚实契约延续）。
- **两平面语义（红线）**: `command.dispatch` 响应 = 幂等裁决（Executed/Replayed/Conflict/Rejected 四出口，
  失败经 `classification` 传达，不暴露 failed——0.7C-7 冻结）；**actual state 唯一来源 = `runtime.query`**
  （SessionPhase/state 投影）+ `agent.health`。JSON-RPC 成功 ≠ SignalVerified ≠ Runtime healthy。
  能力级诚实拒绝（如 SwitchProgram 无双输入执行平面）= 裁决 executed + `classification=rejected` + detail 说明；
  形状拒绝 = Rejected（未触 Runtime、未占 command_id）。
- **幂等**: 进程内 `CommandIdempotency`（D9-A..E；重放/conflict/并发恰一次）。跨重启 durable idempotency
  = RH-IDEM-01，维持 DEFER-UNTIL-CONTROL-PLANE（解冻点 = external Fastify command entry）。
- **prototype `/api/v1/*` 五端点**: 维持 0.7C-8/R60 冻结行为；生产 query/idem 缺席 → 503 不变。
  Product External API `/api/v1/*` 归 Fastify（CONTROL-PLANE 阶段）；两个可写控制面不并存。
- **已知 wire 缝隙（如实登记，非本面包修）**: `runtime.query` 返回的 session id 为显示形态
  `session-<hex>`；`command.dispatch` 的 `session_by_id` 目标要求 canonical UUID——消费方自行映射；
  Product API 资源形状（`/sessions/{id}` URL 形态）归 Fastify 阶段裁定。
