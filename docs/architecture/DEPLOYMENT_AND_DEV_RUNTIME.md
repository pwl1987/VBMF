# Deployment & Development Runtime Contract (V0.2)

> **文档身份**: VBMF 部署 / 开发运行时 **Deployment SoT**。
> **生成**: 2026-08-25（基于基线 `a6eca1f`，第 44 轮审查结论；commit `3888980` 复核后落地）。
> **状态**: **DEPLOYMENT SoT — 与 `ARCHITECTURE_V0.2.md`（Runtime SoT）+ `TECHNOLOGY_STACK_AND_RUNTIME_OWNERSHIP.md`（Ownership SoT）共同构成三层 SoT；不重开 V0.2**。
>
> **为什么存在**：Runtime Ownership 已在 Ownership SoT 锁死（Media Agent = DeckLink/GStreamer/Live FFmpeg 唯一 Owner），但「Docker 开发环境 / BMD 设备透传 / SSH 远程开发 / 热更新 / 自愈」尚未整合成正式 Deployment Contract，且 `SYSTEM_AND_PROJECT_PLAN.md` 仍存在与 Ownership 冲突的旧表述（方式 B 推荐生产、RustFS 生产可 MinIO）。本文件负责锁这五件事。

---

## 0. 三层 SoT 关系

```
ARCHITECTURE_V0.2.md          → Runtime Semantics / 进程模型 (LOCK FINAL)
TECHNOLOGY_STACK_AND_         → 技术栈 + 所有权矩阵 + Forbidden (F1-F12)
  RUNTIME_OWNERSHIP.md
DEPLOYMENT_AND_DEV_RUNTIME.md → 部署平面 + BMD Host + SSH + Hot Reload + Self-Healing  ← 本文件
SYSTEM_AND_PROJECT_PLAN.md    → 实施路线 (须与本三层 SoT 对齐, 冲突以本层为准)
```

> **冲突裁决**：部署/开发运行时任何表述若与 Ownership SoT 的 F2/F4/F11（Media Agent 拥有硬件媒体生命周期）冲突，一律以 Ownership SoT 为准。

---

## 1. Deployment Plane（全容器化，仅宿主机保留 kernel/driver/hardware）

```
HOST (Linux kernel + BMD driver + DeckLink hardware + Docker Engine + SSH)
│
├── Docker Compose ────────────────────────────────────────
│   CONTROL:   Web(Vite) · Fastify · Worker(BullMQ) ·
│              PostgreSQL · Valkey · RustFS · SRS
│   MEDIA:     Media-Agent container
│                 ├── Rust Media Agent
│                 ├── GStreamer
│                 ├── FFmpeg (live)
│                 └── DeckLink SDK  (--device /dev/blackmagic)
└─────────────────────────────────────────────────────────
```

- **Docker 化范围**：PostgreSQL / Valkey / RustFS / SRS / Fastify / Worker / Web / Media Agent **全部容器**；只有 Linux kernel、BMD Desktop Video 驱动、DeckLink 硬件、Docker Engine、SSH 属宿主机。
- **RustFS = canonical object storage**：不再保留 MinIO 分叉（INFRA-02 已修）。
- **BMD + Docker = 设备透传（方式 A）唯一模式**；方式 B（Host ffmpeg → RTMP → Container）降级为诊断/受限环境 workaround，不进 V0.2 baseline（DEPLOY-03 已修，冲突 F4/F11）。

---

## 2. BMD Host（设备透传边界）

```
BMD Host
├── Linux Kernel
├── BMD Desktop Video Driver
├── DeckLink Hardware (/dev/blackmagic/{dv0,dv1,io0})
├── Docker Engine
└── Media-Agent Container
      ├── Rust Media Agent   ← Runtime Owner
      ├── GStreamer          ← executor (owned by Agent)
      ├── FFmpeg (live)      ← owned by Agent
      └── DeckLink SDK
```

- 设备发现后 Media Agent 建 **Device Registry + Exclusive Lease + Session Owner**（F11）；容器 `--device /dev/blackmagic` 透传，字符设备不可多容器共享。
- **禁止** Host 侧 ffmpeg 直接持有 DeckLink（否则 Media Agent 失去 Owner 身份）。

---

## 3. SSH（Dev/Ops Transport，不进入 Runtime Control Plane）

```
Developer Laptop
       │ SSH
       ▼
BMD Dev Server (SSH server)
       │
       ├── docker compose up / down / logs
       ├── 更新开发代码 (bind mount + rebuild)
       ├── 查看容器 / GStreamer / FFmpeg / DeckLink / 设备状态
       └── 重启 Media Agent (controlled restart)
```

- **SSH = 开发/运维传输层**，只用于连接 BMD 服务器、管理容器与查看状态。
- **真正的运行控制**：`React → Fastify → JSON-RPC → Media Agent`。SSH 不得作为业务 Runtime Control Protocol。

---

## 4. Hot Reload / Fast Iteration（不反复打包）

| 组件 | 机制 | 约束 |
|---|---|---|
| Web (React) | Vite dev server + source bind mount + HMR | 浏览器热更，不重打镜像 |
| Fastify | tsx / node --watch + bind mount | 进程级重启，不重打镜像 |
| Worker | node --watch | 同 Fastify |
| Rust Media Agent | cargo-watch / cargo run + bind mount | **受控重启**：代码变更 → rebuild → controlled restart，**不得无控制杀掉当前 Live Session** |

> **关键规则**：Media Agent 热更新必须走 `controlled restart`（preserve `active_source_id`、DeckLink Lease 不丢、下游 state recover），否则破坏 FI-09 / 24h stability。开发模式禁止把 Live Session 当作可随意丢弃的进程。

---

## 5. Self-Healing（四层 + 防护）

```
L1 Process Self-Heal    GStreamer crash / FFmpeg(live) crash
L2 Runtime Self-Heal    Media Agent restart / Session rebuild
L3 Service Self-Heal    Fastify / Worker / SRS / PostgreSQL / Valkey / RustFS
L4 Infrastructure       Docker restart / server recovery
```

**防护（防 restart storm）**：
- **Restart Budget**：每组件单位时间最大重启次数
- **Backoff**：指数退避
- **Retry Count / Circuit Break**：超限熔断
- **Escalation → Incident → Manual Required**：超过预算转人工

> Health Tree 须暴露 Recovery State：`NONE / RESTARTING / RECOVERED / RETRYING / BACKOFF / ESCALATED / MANUAL_REQUIRED`（见 Phase 0.6 / Phase 4 UX-04）。仅 `HEALTHY` 不够，操作员须知道系统是否正在自愈。

---

## 6. ENV Preflight（Phase 0.6 前置，须覆盖以下维度）

| 维度 | 检查项 |
|---|---|
| Docker | daemon / compose / network / volume / healthcheck |
| Containers | PostgreSQL·Valkey·RustFS·SRS·Fastify·Worker·Web·Media Agent 全部 UP |
| Infra Readiness | PostgreSQL READY / Valkey READY / RustFS READY（非仅容器 started；Fastify 须等依赖 READY，非仅 `depends_on`） |
| Remote Host | SSH 连通 / OS / kernel / BMD driver / DeckLink devices / device lease 空闲 |
| Media Runtime | GStreamer / FFmpeg / DeckLink / codec / SRS 可用 |
| Hot Reload | Vite HMR / Fastify watcher / Worker watcher / Rust dev rebuild 就绪 |
| Self-Healing | restart policy / budget / backoff / supervisor 配置存在 |

> INFRA-01：PostgreSQL "container started" ≠ "ready"；compose 须用 healthcheck + Fastify 侧 readiness wait，避免 "容器都起但 API 启动失败"。

---

## 7. 与 Ownership SoT 的 forbidden 衔接

- F2/F4/F11：Media Agent 拥有 DeckLink/GStreamer/Live FFmpeg → 部署上 BMD 必须设备透传进 Media-Agent 容器（§1/§2）。
- F9：Control Plane 不得 spawn ffmpeg → 容器内 Fastify 仍禁 `child_process.spawn("ffmpeg")`；live ffmpeg 只在 Media-Agent 容器。
- 本文件与 Ownership SoT 冲突时，以 **Ownership SoT 的硬件媒体生命周期归属** 为准。
