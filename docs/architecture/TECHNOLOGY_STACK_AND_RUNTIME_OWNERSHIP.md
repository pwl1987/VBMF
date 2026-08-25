# Technology Stack & Runtime Ownership (V0.2 Reconciliation SoT)

> **文档身份**: VBMF 技术栈与运行时所有权 **Reconciliation Contract**。
> **生成**: 2026-08-25（基于基线 `a6eca1f`，第 41+ 轮审查结论落地）。
> **状态**: **RECONCILIATION SoT — 不重开 V0.2**。
>
> **为什么存在**: `SYSTEM_AND_PROJECT_PLAN.md`（V0.1 规划）、`ARCHITECTURE_V0.2.md`（Runtime SoT）、Phase 0.5（UX）、Phase 0.6（验收）四套文档各自描述系统，但中间缺一层 **Runtime Ownership Contract**，导致 GStreamer ingest / Live FFmpeg / Recording / MediaMTX 等边界漂移。
>
> **本文件目的**：把"谁拥有媒体进程生命周期"锁死，使四套文档成为一条连续链，而非各自为政。

---

## 0. 文档权威层级（SoT 分层）

| 层级 | 文档 | 负责什么 | 不能做什么 |
|---|---|---|---|
| **Runtime Architecture SoT** | `docs/architecture/ARCHITECTURE_V0.2.md` | 进程模型、角色、Runtime Semantics、Schema、Decision Tree | 已被 LOCK FINAL；不再开 V0.2.5 |
| **本文件 (Reconciliation SoT)** | `docs/architecture/TECHNOLOGY_STACK_AND_RUNTIME_OWNERSHIP.md` | 技术栈清单 + 跨平面所有权矩阵 + Forbidden 依赖 | 不重写 V0.2 算法/语义 |
| **Project Implementation Plan** | `docs/SYSTEM_AND_PROJECT_PLAN.md` | 服务器基线、基础设施、实施路线（须与 V0.2 对齐） | 不得凌驾于 V0.2 Runtime Semantics |
| **UX / Workflow SoT** | Phase 0.5 | Operator Workflow / Surface / Object Model | 不重新选择技术栈 |
| **Acceptance SoT** | `docs/phase-0.6/` | 可执行验收（只能验证上层定义） | 不得重新选择 Rust/Node/SRS/GStreamer |

> **冲突裁决**：任何与 `ARCHITECTURE_V0.2.md` Runtime Semantic 冲突的旧表述，一律以 V0.2 为准；本文件与 V0.2 冲突时以 V0.2 为准。

---

## 1. Technology Stack（冻结）

| 能力 | 最终选择 | 备注 |
|---|---|---|
| Frontend | React 19 + Vite + **shadcn/ui** + Tailwind | ⚠️ 已替代 Ant Design Pro（广播级控制台需自建 Design System） |
| Control Plane | Fastify + TypeScript | 状态机引擎 / API / Auth |
| Data | PostgreSQL + Drizzle | Schema 以 V0.2 §5 为准 |
| Cache / Queue | Valkey + BullMQ | BullMQ 驱动异步 Job |
| Media Runtime | **Rust Media Agent** (Host, JSON-RPC) | 拥有实时媒体生命周期 |
| Realtime Ingest | **GStreamer** | Media Agent 管理的底层执行组件（非独立服务） |
| Media Processing | **FFmpeg** | 两种 owner：Live→Agent，File→Worker |
| Hardware I/O | Blackmagic DeckLink SDK | 由 Media Agent 启动时 Runtime Discovery |
| Streaming Gateway | **SRS** ⚠️ 锁定 | RTMP ingest / HLS / WebRTC-WHEP egress |
| Object Storage | RustFS (S3 兼容) | 生产可 MinIO |
| Browser | hls.js + WHEP + WebCodecs | 播放 / 低延迟 monitor / frame-level |
| Auth / RBAC | Better Auth + CASL | |
| Testing | Vitest + Playwright + Python+YAML Harness | |

> **MediaMTX**：**不纳入 V0.2 baseline**。SRS 为 Canonical Stream Gateway；仅在 Phase 1 真实证据证明 SRS 不满足明确需求时才重新评估（届时走 V0.2 Architecture Change Review）。

---

## 2. Ownership Planes（所有权平面）

```
                         WEB CONSOLE (React 19 + Vite + shadcn/ui)
                                   │  HTTP / WS / SSE
                        ┌──────────▼──────────┐
                        │     CONTROL PLANE    │  Fastify + TS
                        │  API / Auth / RBAC   │
                        │  Graph Compiler      │  (确定性编译器, 非 Job)
                        │  Preflight / ChangeSet│
                        │  Runtime Commands    │
                        └──────────┬───────────┘
                                   │  JSON-RPC / Control
                        ┌──────────▼───────────┐
                        │      MEDIA PLANE      │  Rust Media Agent (Host)
                        │  Session / Runtime Graph│
                        │  Switch / Failover    │
                        │  Clock / Health       │
                        │  Hardware Discovery   │
                        └──────┬──────────┬─────┘
                      GStreamer│          │FFmpeg (live)
                          DeckLink SDI    Live Encode
                               └────┬─────┘
                              Program Media
                                   │
                            ┌──────▼──────┐
                            │     SRS      │  Stream Gateway Adapter
                            └──────┬──────┘
                          RTMP / HLS / WHEP

   ── 完全独立的异步平面 ──────────────────────────────────────
                ASYNC JOB PLANE (Fastify + BullMQ + Node Worker)
                   FILE_TRANSCODE / PROBE / THUMBNAIL /
                   WAVEFORM / ASSET_ANALYSIS / POST_PROCESS
                           │
                        FFmpeg (file) → RustFS
```

---

## 3. Runtime Ownership Matrix（核心契约）

| 能力 | React | Fastify (Control) | BullMQ Worker | Rust Media Agent | GStreamer | FFmpeg | SRS |
|---|---|---|---|---|---|---|---|
| UI | ✅ | | | | | | |
| CRUD / Auth / Config | | ✅ | | | | | |
| Graph Compile | | ✅ | | | | | |
| Preflight | | ✅ | | 辅助 Runtime preflight | | | |
| File Transcode | | 调度 | ✅ | | | ✅ (file) | |
| Asset Probe | | 调度 | ✅ | | | ✅ (file) | |
| **SDI Ingest** | | | ❌ | **✅ owner** | **✅ executor** | | |
| Live Decode | | | ❌ | **✅ owner** | | ✅ (live) | |
| Live Encode | | | ❌ | **✅ owner** | | ✅ (live) | |
| Live Switch | | command | ❌ | **✅** | | | |
| Failover | | command | ❌ | **✅** | | | |
| **Live Recording** | | command | ❌ | **✅** | | ✅ | |
| Output Routing | | command | ❌ | **✅** | | | ✅ adapter |
| SRS Gateway | | configure | | control | | | **✅** |
| Health (runtime truth) | UI | aggregation/API | | **✅** | telemetry | telemetry | telemetry |
| Runtime Session | | command | ❌ | **✅ owner** | subordinate | subordinate | subordinate |

> **GStreamer 定位**：Media Pipeline Execution Technology，**不是独立业务服务**；由 Rust Media Agent 拥有其 pipeline 生命周期（启动 / 监督 / 停止 / 故障重启）。
> **FFmpeg 双 owner**：File/offline → BullMQ Node Worker；Live/realtime → Rust Media Agent。二者**不得混用同一入口**（见 Phase 0.6 UI-E2E-03 `FILE_TRANSCODE ≠ REALTIME_ENCODE`）。
> **Recording 双分**：Live Recording（跟 runtime clock / PGM / AV sync / session）→ Media Agent；Post-processing（transcode/trim/proxy/checksum/archive）→ Node Worker。

---

## 4. Forbidden Dependencies（硬约束）

```
CONTROL PLANE NEVER OWNS MEDIA PROCESS LIFECYCLE
MEDIA PLANE OWNS MEDIA PROCESS LIFECYCLE
```

| # | 规则 | 违反后果 |
|---|---|---|
| F1 | **BullMQ Worker MUST NOT own live Media Session** | TAKE 实时播控被降级成 Job，破坏 `TAKE → Preflight → Switch → active_source_id` 模型 |
| F2 | **Fastify MUST NOT directly own DeckLink device** | Control Plane 侵入 Media Plane |
| F3 | **Fastify MUST NOT directly supervise live FFmpeg** | 实时编码生命周期失控 |
| F4 | **GStreamer MUST be owned by Media Runtime (Agent)** | 独立系统服务破坏 Runtime Ownership 架构 |
| F5 | **SRS MUST NOT own Switch / Failover decisions** | SRS = Gateway Adapter，非 Output Engine 全体 |
| F6 | **React MUST NOT own media process lifecycle** | `<video>`/MSE/WHEP/WebCodecs 经独立 Media Controller，不触发 React render 承担解码 |
| F7 | **Graph Compiler MUST NOT run as BullMQ Job** | 它是 Control Plane 确定性编译器，非异步媒体任务 |
| F8 | **MediaMTX MUST NOT be in V0.2 baseline** | 双 Gateway 增加运维/故障域/测试矩阵 |

---

## 5. 与四套文档的对应关系

- **GSTR-01（GStreamer ownership 未闭合）** → 本节 §3/§4 F4 锁死：Media Agent owns GStreamer pipeline lifecycle。
- **TECH-02（Live vs Async FFmpeg）** → §3 双 owner；§4 F1/F3。
- **RECORD-01（Recording 灰区）** → §3 Live Recording→Agent，Post-processing→Worker。
- **TECH-04（SRS vs MediaMTX）** → §1 / §4 F8；SRS 锁定，MediaMTX 退出 baseline。
- **TECH-05（AntD Pro 漂移）** → §1 已锁 shadcn/ui（SYSTEM_AND_PROJECT_PLAN 第 245/297 行已落实）。
- **TECH-06（Plan 身份）** → §0 分层；SYSTEM_AND_PROJECT_PLAN 头部须声明 "V0.1 Planning / Reconciled with V0.2"。

> 本文件是 G-DOC 前置 Gate 的 SoT 输入：任何后续技术栈变更须先过本契约与 V0.2 Architecture Change Review。
