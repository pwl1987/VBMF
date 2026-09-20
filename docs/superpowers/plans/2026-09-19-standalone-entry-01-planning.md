# STANDALONE-ENTRY-01 — Standalone Runtime Entry：Planning / Reconciliation（PLAN / RECONCILIATION ONLY）

- Date: 2026-09-19（RF-SRC-RTMP-02 closure §3.60 之后；用户 2026-09-19 明确授权）
- Packet 性质：**bounded planning/reconciliation**——只读 live-tree audit + Authority reconciliation + 冻结设计 + acceptance matrix；本 packet 不做大规模实现。
- 永久定位约束：`standalone first`；`compatible, not dependent`（对 `media-digital-*`）；`Runtime owns truth`（Web/Console 只观察与命令）。**不建设** Web Console、Fastify SoR、SDK、新 Runtime owner。

## 1. 为什么现在做

Runtime 已拥有足够大的真实能力切片（FFmpeg input/output、RTMP source listener、Normalize、Master Switch、Health/Recovery、Session/Resource/Lease/Supervisor 单 owner、Network-only production composition @ `d13f1fd`）。继续无边界扩 Network umbrella 之前，先把「独立安装、配置、启动、运行、恢复、部署」收敛成 standalone 产品面——这是 VBMF 作为 Broadcast Runtime 的第一性职责。

## 2. Live-tree audit（as-is 事实，全部带出处）

### 2.1 启动 / 组合选择（本 packet 前夜刚收口）

- `bin/media-agent.rs`：`select_startup_composition_mode()` 先于 `bootstrap::build()`；`MEDIA_AGENT_NETWORK_BINDING` → Network-only（无 discovery/manifest/DeviceLease/DeckLink/SDK probe，单 Runtime owner，零媒体自动启动）；双 binding / diagnostic / selftest 叠加 fail-closed exit(2)。
- Device 平面：`bootstrap::build()`（discovery + 占位租约 + supervisor 注册）→ bmd-provider/gstreamer 或 ffmpeg 组合根；Production 不自动启动媒体，等待显式 command（P1-3）。
- `/health`（P0.7C-8）：loopback bind（`MEDIA_AGENT_HEALTH_BIND`，默认 127.0.0.1:8080）；Production 下 query/command/idempotency/switch 面 = None ⇒ 503 契约诚实；device_count；AgentState。
- **缺口 A（硬）**：**全仓无 POSIX 信号处理**（grep 证实仅媒体 `signal.rs` SDI 信号检测）。SIGTERM 默认终止：Drop 不执行 → FFmpeg 子进程/网络 listener 孤儿化、Session 无有序停止。验收 gate 之所以干净是因为 gate 显式 stop 后 `exit(0)`，生产 kill 无此保证。

### 2.2 配置 / manifests / 机器身份

- `config.rs`：9 个 `MEDIA_AGENT_*` env + `PrototypeOutputConfig`（5 个 `VBMF_OUTPUT_*`，诊断输出）+ gate envs（gates bin 专属）。
- manifests：`DeviceBindingManifest`（device 平面，v2）与 `NetworkSourceBinding`（network 平面，0600/machine-pin/startup-only/无热加载——frozen D3）。
- **缺口 B**：`current_machine_id()` = `VBMF_MACHINE_ID` env > `HOSTNAME` env > 空串。生产机器 pin 依赖 env 是弱身份；`/etc/machine-id` 未被读取。容器/裸机混用时 HOSTNAME 语义不稳定。
- **缺口 C**：无 config 文件面（全 env）；对 standalone 运维（unit 文件 + Environment= 行）可行但缺一页权威清单（env → 语义 → 默认 → fail 条件）。

### 2.3 生命周期 / 恢复 / 状态

- Session 生命周期：SessionManager 唯一 owner（create=start journal 回滚、stop/close 逆序、RecoveryMonitor + Supervisor 决策、LeaseManager 房务 + tick 线程 5s）。
- AgentState 状态机：Starting/Ready/Capturing/Degraded/Restarting/Backoff/Escalated/ManualRequired（`health.rs`，P0-7D reducer 折叠）。
- 进程生命周期：main 末尾 `loop { sleep(3600) }` 常驻；"生产由 supervisor 管理生命周期" 只是注释意图，无 unit/监督配置存在。
- **缺口 D**：readiness 与 liveness 未区分：`Starting` 状态从未被生产路径置位（构造完直接 Ready）；无 probe 语义文档。

### 2.4 部署现状（BMD 与 ops/）

- `ops/`：compose/Dockerfile/nginx 为 **V0.2 全栈形态（9 服务）占位**——`Dockerfile.media-agent` 是注释模板（rust:1.83-alpine，无真实 build context）；依赖 Fastify/PostgreSQL/Valkey/RustFS/SRS/Web 等尚未建设的服务。
- `docs/architecture/DEPLOYMENT_AND_DEV_RUNTIME.md`（Deployment SoT，2026-08-25，基线 `a6eca1f`）：全容器化 + Nginx 反代定稿。**早于** FFmpeg backend（RF-FF-01*）、RTMP source（RF-SRC-RTMP-01/02）、Network-only 启动模式（§3.60）全部演进；其 media-agent 容器假设（GStreamer 主执行器、SRS 拥有 RTMP 面）与 live 代码已有实质漂移。
- BMD 实机：`/opt/vbmf-dev/repo` detached @ `7cc33dd` 带未提交 ops 改动（§8 risk 4，明显落后）；**无任何 VBMF 服务进程**；历次验收均走 `/tmp` exact-commit archive 手工构建。device-2 长期被历史 gst-launch 占用（PID 992634）。
- **缺口 E**：standalone 形态（单 binary + manifests + env + /health，不需要任何容器/DB/控制面）没有部署 lane、没有 install/upgrade/rollback 契约、没有 exact-commit 部署对账记录。

### 2.5 日志 / 证据 / 目录

- 日志 = `tracing_subscriber::fmt::init()` stdout（无 RUST_LOG 分级配置面、无结构化落盘）；事件双平面（projection/internal RuntimeEventLog）在内存。
- 无 VBMF 自有持久目录（/var/lib、/etc 下无内容）；evidence 目录只存在于 dev 仓库。

## 3. Authority reconciliation（不重开冻结）

1. **三层 SoT 不重开**：`ARCHITECTURE_V0.2`（Runtime）/`TECHNOLOGY_STACK_AND_RUNTIME_OWNERSHIP`（Ownership）/`DEPLOYMENT_AND_DEV_RUNTIME`（Deployment）保持 LOCK。本设计不修改任何冻结 Contract/wire/Owner。
2. **Deployment SoT 的 standalone 欠缺以"补充 lane"方式收口，不改写全栈 lane**：SoT §1 的全容器化平面描述的是 V0.2 终态产品形态；standalone lane（单 media-agent 直接运行于 BMD 宿主，systemd 监管）是该文档未覆盖的**新增部署形态**，而非对 Nginx/Fastify 终态的否定。后续实施若需在 Deployment SoT 追加一节"standalone lane"，作为独立 docs 变更走协调者 review（预计属于 SE-01B）。
3. **ops/ 占位文件不动**：compose/Dockerfile 全栈占位与 standalone lane 无依赖关系；SE 系列不修改它们（compatible, not dependent 双向适用）。
4. **`Runtime owns truth` 边界冻结**：SE 系列的一切部署工件（unit/脚本）只做"启动/停止/重启进程 + 传 env + 指向 manifests"，不读取、不复制、不缓仔 Runtime 状态；健康判断以 /health 与进程退出码为依据，绝不旁路 SessionManager/LeaseManager 形成第二 truth。
5. **BMD 部署对账以 exact-commit 为唯一口径**：沿用 §3.60 证据纪律（archive sha + binary sha + manifest digest + CI run），`/opt/vbmf-dev/repo` 旧树不动、不继承其历史（risk 4 维持），新 install 落独立版本化路径。

## 4. 冻结设计决策（S 系列）

- **S1 Standalone lane 定义**：standalone VBMF = `media-agent` 单进程 + `/etc/vbmf/`（manifests，0600，service user 属主）+ env 配置（unit `Environment=`）+ `/health` + systemd 单元；运行与恢复不依赖任何容器/DB/控制面服务。全栈 lane（ops/）与其并存互不依赖。
- **S2 启动模式单一入口**：`select_startup_composition_mode()`（§3.60 已实现）是 standalone 启动的唯一模式选择；其 fail-closed 规则（双 binding/diagnostic/selftest 拒启、非 ffmpeg-backend 拒 network-only）原样冻结。
- **S3 Graceful shutdown（缺口 A，SE-01A）**：SIGTERM/SIGINT → 显式有序停止：所有活跃 Session 逆序 stop（SessionManager 唯一 owner）→ RecoveryMonitor join → FFmpeg 子进程/GStreamer 管线回收（既有 reaper/Drop 语义在受控路径上得到执行）→ /health 下线 → exit 0。SIGHUP 不做 reload（manifests startup-only 是 frozen D3；SIGHUP = 无操作并记录，或按 unit 配置等价于 restart——实现取"无操作 + warn"，避免假装支持热更新）。第二次 SIGTERM = 立即默认终止（逃生门，运维语义）。
- **S4 Readiness / Liveness（缺口 D，SE-01C）**：liveness = 进程存活 + /health 应答；readiness = `/health` 的 `agent_state` 语义化：Network-only/Device Production 构造完成且非 `Starting`/`ManualRequired` 即 ready（`Starting` 在 S3 实现中于 main 入口真实置位）。**不改 /health wire shape**（P0.7C-8 契约保持）；如未来需要独立 readiness endpoint，必须先过 EXTERNAL_API_CONTRACT review——本 packet 明确不做。
- **S5 机器身份（缺口 B，SE-01D）**：`current_machine_id()` 收敛为 `VBMF_MACHINE_ID`（显式覆盖，测试/容器用）> `/etc/machine-id`（生产权威）> 空=拒绝用于 manifest pin（fail-closed 语义不变）。HOSTNAME fallback 移除是行为变化：需 manifests 两类 pin 的回归测试与 BMD 实机重 pin 证据；在 sub-packet 内完成，不在本 planning 内偷改。
- **S6 Install / upgrade / rollback（缺口 E，SE-01B）**：版本化目录 `/opt/vbmf/<version>-<short-sha>/`（archive 展开后 release 构建）+ `/opt/vbmf/current` symlink 切换；rollback = symlink 回指；同侧行 manifest（`install-manifest.json`：exact SHA、archive sha256、binary sha256、构建 feature 集、CI run id）。**禁止**原地 mutation 升级；**禁止**部署 floating ref。
- **S7 运行时目录 / 权限（SE-01B）**：`/etc/vbmf/`（manifests 0600，service user 属主）；`/var/lib/vbmf/`（预留：Runtime 未来持久状态；当前 Runtime 无持久状态，目录在 install 时建立并记录属主，不伪造用途）；日志 = stdout → journald（systemd 捕获，`RUST_LOG` 透传为可选分级面）；evidence 类文件不入生产目录。
- **S8 服务单元（SE-01B）**：`vbmf-media-agent.service`：`Restart=on-failure` + `StartLimitBurst` 受控退避；`ExecStart` 指向 `/opt/vbmf/current/...`；`EnvironmentFile=-/etc/vbmf/media-agent.env`；退出码契约文档化（2 = fail-closed 构造拒绝；0 = 正常停止）。不做 socket activation、不做 root 运行（设备访问按 BMD 现实由用户组/udev 决定，device-2 占用进程不受影响）。
- **S9 进程内恢复边界**：in-process 恢复维持 SessionManager/RecoveryMonitor/Supervisor 既有 owner；进程级重启归 systemd；两者不叠加第二套监督 truth。24h stability debt 不因本设计清偿。
- **S10 配置权威清单（缺口 C，SE-01C 交付文档）**：env → 语义 → 默认 → fail 条件一页清单（含 gate envs 标注"gates bin 专属，不进生产"），作为 standalone 运维入口文档；不引入 config 文件机制（env + unit 文件已满足，避免新 truth 面）。

## 5. Bounded sub-packet 分解（顺序执行）

| ID | 内容 | 类型 | 关键验收（摘要） |
|---|---|---|---|
| **SE-01A** | Graceful shutdown：SIGTERM/SIGINT 有序停止（S3）+ Starting 置位 | Runtime 代码（bounded） | 单测：信号路径 stop 顺序/零孤儿（mock）；ffmpeg-backend 集成：kill -TERM 后 FFmpeg child/listener 无残留；Device 生产路径零回归；CI 7/7；BMD runtime smoke（network-only kill→clean teardown） |
| **SE-01C** | Readiness/liveness 语义 + 配置权威清单 + 启动失败/退出码文档（S4/S10/S8 文档面） | 文档 + 最小代码（若 Starting 置位未在 A 完成） | /health 契约零变化断言；文档 review；无新 wire |
| **SE-01D** | 机器身份收敛 /etc/machine-id（S5） | Runtime 代码（bounded） | 两类 manifest pin 回归；BMD 重 pin 实证；CI 7/7 |
| **SE-01B** | Standalone install/upgrade/rollback + systemd unit + BMD 部署对账执行（S6/S7/S8；Deployment SoT 增补 lane 节） | 部署工件 + host 操作（需 BMD 窗口） | BMD exact-commit 安装→network-only 起→/health ready→外部推流→SIGTERM 干净停止→symlink rollback；旧 /opt/vbmf-dev 不动；device-2 不受扰 |

Forbidden（全系列）：Web Console/Fastify/Worker/DB/SRS 建设；ops/ 全栈占位修改；/health wire 变更；manifests 热加载；第二 Runtime/监督 owner；24h stability 宣称；`/opt/vbmf-dev/repo` 破坏性同步。

## 6. Acceptance matrix（standalone 面总验收）

1. fresh install（S6）于 BMD exact commit → network-only 模式以 /etc/vbmf manifest 启动 → /health `agent_state=Ready`（readiness 语义按 S4）。
2. 外部第三方推流（independent netns 形态可）→ A/V verified → SIGTERM → 退出码 0 → 零 FFmpeg/listener/monitor 残留 → systemd 按 unit 语义重启 → 再次 ready。
3. Device 生产模式在 standalone lane 下行为零回归（既有 CI 矩阵 + 一轮 BMD device smoke）。
4. rollback：symlink 回指旧版本 → 起动 → manifest digest 与 install-manifest 一致。
5. 全程 `Runtime owns truth`：部署工件不含任何状态复制/旁路。
6. 各 Runtime sub-packet 常规门禁（focused/full matrix/clippy/fmt/arch lint/remove-adapters/diff + CI 7/7）。

## 7. 与既有债务/风险的关系

- 24h RSS stability：独立 debt，不因 standalone lane 清偿或宣称。
- BMD deployment divergence（risk 4）：SE-01B 以新版本化路径正面解决，旧树只读保留。
- runner 出网/CI 基础设施：不涉及。
- Mimosa 完整扫描欠账（§3.60 披露）：SE 系列每个 Runtime commit 后如钩子仍 scanner_enobufs，登记并另行补扫；不宣称安全审计完成。
