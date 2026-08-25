# Deployment & Development Runtime Contract (V0.2)

> **文档身份**: VBMF 部署 / 开发运行时 **Deployment SoT**。
> **生成**: 2026-08-25（基于基线 `a6eca1f`，第 44 轮审查结论；commit `3888980` 复核后落地）。
> **状态**: **DEPLOYMENT SoT — 与 `ARCHITECTURE_V0.2.md`（Runtime SoT）+ `TECHNOLOGY_STACK_AND_RUNTIME_OWNERSHIP.md`（Ownership SoT）共同构成三层 SoT；不重开 V0.2**。
>
> **为什么存在**：Runtime Ownership 已在 Ownership SoT 锁死（Media Agent = DeckLink/GStreamer/Live FFmpeg 唯一 Owner），但「Docker 开发环境 / BMD 设备透传 / SSH 远程开发 / 热更新 / 自愈 / Nginx 反代 / 远程实机验收」尚未整合成正式 Deployment Contract，且 `SYSTEM_AND_PROJECT_PLAN.md` 仍存在与 Ownership 冲突的旧表述。本文件负责锁这些事。
> **反向代理已定稿**：**Nginx** 为 V0.2 唯一对外 HTTP/HTTPS 反代（Plan §2.7 #8 已确认）；Caddy 不纳入。精细 proxy policy 见 §8。

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

---

## 8. Nginx Reverse Proxy Contract（NGINX-01 / API-02 / API-03 / OPS-01）

> **Nginx = 唯一对外 HTTP/HTTPS 入口**（V0.2 广播场景需精细 proxy policy；若保留 Caddy 须满足同等策略）。Fastify **不当作媒体流代理**；SRS 承担媒体协议平面。

### 路由边界
```
Internet/LAN ──HTTPS──▶ Nginx
 ├── /            → Web (React, Vite build)
 ├── /api/*       → Fastify (Public API)
 ├── /ops/*       → Fastify Ops API (stricter auth/audit/rate-limit/IP-allowlist/VPN-LAN)
 ├── /admin/*     → Fastify Admin (最强隔离)
 ├── /ws/*        → WebSocket (Fastify)
 ├── /events/*    → SSE (Fastify)
 └── /health/*    → Health/Readiness (liveness + preflight)
```

### Control Plane vs Media Plane
- **Nginx 负责 Control Plane**：`/api /ws /events /ops /admin` + 可选 HTTP media ingress/egress routing。
- **SRS 负责 Media Protocol Plane**：`RTMP / SRT / HLS / WHEP` 由 SRS 直接对外（或经 Nginx 域名层路由，但 **Nginx 不承担 SRS 的媒体协议转换**）。
- **禁止**：让 Fastify 变成媒体流代理（HLS 大量 segment 不得与 API 同 proxy policy）。

### TLS / Headers / 长连接
- TLS termination + HSTS；透传 `X-Forwarded-For/Proto/Host` 给 Fastify（Fastify 须信任 proxy）。
- WebSocket：`proxy_http_version 1.1` + `Upgrade` + `Connection "upgrade"`。
- SSE：`proxy_buffering off` + 长 `proxy_read_timeout` + keepalive（否则 Job Progress/Runtime State 偶发 30s 不更新）。

### 大文件上传（API-02, tus + Fastify multipart）
- `client_max_body_size 0`（或 ≥10G）；`proxy_request_buffering off`；`proxy_read_timeout` / `proxy_send_timeout` 调大（2GB/10GB 视频不得被 Nginx 截断/超时）。

### Ops 隔离（OPS-01）
- `/ops`、`/admin` 不得与普通 `/api` 同暴露面：更强 auth + audit + rate limit + IP allowlist / VPN / LAN policy。

---

## 9. Remote BMD Acceptance（G-RUNTIME 前置，必须实机）

> **Phase 0.6 G-RUNTIME 不能在 GitHub CI 完成**。GitHub Actions runner 无真实 BMD 卡/DeckLink SDK/SDI 链路/设备租约。

### 两级验收
```
Level 1 (GitHub CI):  lint / schema / Graph Compiler assertion / ownership guard /
                      unit / frontend E2E(mock) / fixture validation / runner self-test
Level 2 (Remote BMD): SSH → pinned SHA → Docker Compose → ENV Preflight →
                      Device Lease → A2(real SDI) → FI-08 → FI-09 → HA → Evidence
```

### Remote Acceptance Workspace（避免随便 clone）
```
/opt/vbmf-dev/
├── repo/          # git checkout <exact-SHA>
├── evidence/      # 实机验收产物 (FI-08/09, A2, HA 实测)
├── artifacts/
├── logs/
└── runtime/
```
- **验收绑定 Git SHA**：`git checkout <exact-SHA>`，禁止"差不多最新"。
- **Acceptance Manifest**（每次实机记录）：`commit / host / os / kernel / bmd_driver / decklink_sdk / ffmpeg / gstreamer / srs / docker / compose / test_run_id`，使 FI-08 PASS 可追溯到具体硬件/版本。
- 不建议让 GitHub CI 经 SSH 直接跑全部硬件验收（避免真实硬件成 CI 脆弱外部依赖）。

---

## 10. CI 三项硬门禁（Gate A 工程化，纯 CI 不需硬件）

1. **Fastify FFmpeg spawn Guard**：静态禁止 `apps/api`、`packages/control-plane` 出现 `spawn("ffmpeg")/exec("ffmpeg")/execFile("ffmpeg")`；允许 `apps/worker`、`services/media-agent` 按职责使用（F9）。
2. **Graph Compiler Output Assertion**：CI 校验 Compiler 产物 = `Runtime Graph Intent` schema，且 **不含** `gst-launch`/`ffmpeg` 具体进程命令（F10）。
3. **Device Lease contract test**：自动化测 Acquire/Renew/Release/Crash Recovery/Stale Lease/Double Acquire/Agent Restart/Concurrent Diagnostic Attempt（F11；实机部分留 Level 2）。

---

## 11. Environment Profile（DEV / ACCEPTANCE / PRODUCTION，FLOW-REMOTE-01）

| Profile | 特征 | 禁止 |
|---|---|---|
| **DEV** | 热更新 / debug logging / controlled restart / BMD 可选 mock | 不得当生产用 |
| **ACCEPTANCE** | pinned commit / 真实 BMD+DeckLink+SRS / evidence capture | 不得当生产 |
| **PRODUCTION** | immutable image / controlled rollout / **no HMR** / stricter self-healing / Nginx TLS | 不得 HMR / 不得调试热更 |

> Rust Media Agent 有 Live Session，**DEV hot reload 严禁误用到 PRODUCTION**（controlled restart 仅 DEV/ACCEPTANCE）。

### Controlled Rust Dev Restart（非裸 cargo-watch）
```
Source Change → Detect → Build → Validate binary → Quiesce/Prepare →
Controlled Restart → Re-acquire Device Lease → Rebuild GStreamer →
Rebuild Live FFmpeg → Restore Graph → Verify active_source_id → HEALTHY
```
> 禁止 `cargo-watch kill → build → run` 直接杀 Live Session（会丢 DeckLink lease / PGM black）。

---

## 12. Ops Visibility（UX-OPS-01，Phase 4 落）

Operator UI 除 Channel Health 外，须下钻 Runtime Owner 链：
```
Channel → Runtime → Media Agent(Host/Container/Session/Recovery)
                  → DeckLink(Device/Lease/Signal)
                  → GStreamer(Pipeline/State)
                  → FFmpeg(Process/Encode)
                  → SRS(Output)
```
且 Health Tree 暴露 Recovery State（`NONE/RESTARTING/RECOVERED/RETRYING/BACKOFF/ESCALATED/MANUAL_REQUIRED`），使 FI-08 自动恢复对现场可见（不仅 HEALTHY）。

---

## 13. Image Version Baseline（DEPLOY-BASELINE，可复现部署真源）

> **原则**：生产部署须可复现、可审计。禁止 `latest`（RustFS 尤甚）；一律锁具体 minor/patch。
> 版本基线以本章为准，`ops/docker-compose.yml` 须与下表一致。升级须走变更评审，不得悄悄 bump。

| 组件 | 基线版本 | 说明 |
|---|---|---|
| PostgreSQL | `17` (alpine) | 当前 LTS 线；非 16/18（18 已过新但非必要追） |
| Valkey | `8` (alpine) | 替代 Redis；8 当前稳定线 |
| RustFS | 锁具体发布版本（如 `2026.x.x`） | **禁止 `latest`**；RustFS 快速演进，须 pin 发布 tag |
| SRS | `6` | SRS 6 稳定线（非 5） |
| Node.js (Fastify/Worker/Web) | `24` (alpine) | 当前 Active LTS（2026-04 起 LTS） |
| Rust (Media Agent) | 锁具体 `1.8x` | 滚动版须 pin minor，不得 `rust:latest` |
| Nginx | `1.27` (alpine) 或 distro stable | 反代契约见 §8 |
| Docker Compose | v2 (`compose` spec) | gVisor 用 `runsc` runtime |

**约束**：
- `docker-compose.yml` 中所有 `image:` 必须显式带版本，CI 门禁应校验"无 `latest` / 无未 pin 版本"（DEPLOY-BASELINE-01）。
- 升级任一组件须更新本表 + compose + 在 CHANGELOG 记录 + Remote Acceptance 复测（§9）。

