# Technology Stack & Runtime Ownership (V0.2 Reconciliation SoT)

> **文档身份**: VBMF 技术栈与运行时所有权 **Reconciliation Contract**。
> **生成**: 2026-08-25（基于基线 `a6eca1f`；第 42 轮建契约，第 43 轮复核 `c232206` 增补 GSTR-02/HW-01/RECORD-02/FRONT-02 + Gate A/B/C）。
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
| **Project Implementation Plan** | `docs/SYSTEM_AND_PROJECT_PLAN.md` | 服务器基类、基础设施、实施路线（须与三层 SoT 对齐） | 不得凌驾于 V0.2 Runtime Semantics |
| **Deployment / Dev Runtime SoT** | `docs/architecture/DEPLOYMENT_AND_DEV_RUNTIME.md` | 部署平面 / BMD 设备透传 / SSH / Hot Reload / Self-Healing / ENV Preflight | 不得违反 F2/F4/F11（Media Agent 硬件媒体生命周期归属） |
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

### 3.1 GSTR-02：Pipeline 构造归属（P1）

> **Pipeline Construction = Media Agent responsibility**。
> **Graph Compiler 只产生 `Runtime Graph Intent`（语义：节点 / 边 / 时钟约束 / latency budget），不得生成具体 GStreamer/FFmpeg 进程命令。**
> Media Agent 负责把 Intent **materialize** 为：GStreamer pipeline 实例、FFmpeg 进程、DeckLink session、restart policy。
> 禁止：Node Graph Compiler 直接拼 `gst-launch` / `ffmpeg` CLI 字符串交给某处执行（见 §4 F9）。

### 3.2 BOUNDARY-06 / RECORD-02：Runtime Artifact → Async Job Handoff（P1）

Live Recording 落盘后，与 Post-processing 的交接必须走正式契约，禁止"文件丢在某目录等 Worker 扫"：

```
Media Agent (host)
  └─ Local Recording Artifact (segment/rollover/close 由 Agent 负责)
       └─ Finalization Event (artifact path + PGM ref + clock + session)
            └─ BullMQ Post-process Job (BODY 携带 artifact 引用, 非轮询)
                 └─ Node Worker → RustFS / Asset
```

- **谁负责 segment/rollover/close**：Media Agent（跟 runtime clock / PGM / AV sync）。
- **何时写 RustFS**：Post-process Job 完成且 checksum 通过后，由 Worker 写；Agent 不直写对象存储。
- **Worker 从哪接管**：订阅 Finalization Event（BullMQ），不轮询共享目录。

### 3.3 FRONT-02：Media Controller 边界（P1，Phase 4 前冻结）

浏览器侧视频/解码**不**由 React 直接操作。建议形态：

```
packages/media-runtime/
├── hls/        (hls.js adapter)
├── whep/       (WebRTC/WHEP adapter)
├── webcodecs/  (WebCodecs frame processing)
├── video-element/
└── controller/ (统一 MediaSessionState / MediaPlaybackState 出口)
```

- React **只能消费** `MediaSessionState` / `MediaPlaybackState` 这类状态；
- React **不得直接操作** `hls.js instance` / `RTCPeerConnection` / `VideoFrame`；
- Media Controller 是 browser-side runtime abstraction（非独立后端服务），归 Frontend 平面但独立于 React render 树。

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
| F9 | **FFmpeg Invocation Guard**：Live FFmpeg 仅 Media Agent 可 spawn；File FFmpeg 仅 Async Worker 可 spawn；**Fastify/Control Plane 不得 `import child_process` 直接 spawn ffmpeg** | 规范正确但代码易违规；建议工程层加 lint / repo-check 拦截 Control Plane 的 ffmpeg spawn（见 §6 Gate A） |
| F10 | **Graph Compiler MUST NOT emit concrete media process command** | 只能产 `Runtime Graph Intent`；materialize 归 Media Agent（GSTR-02） |
| F11 | **DeckLink / Hardware device = exclusive lease owned by Media Agent** | 设备发现后建 Device Registry + Exclusive Lease + Session Owner；禁止第二进程（含诊断工具/Recording job/Operator 误改）抢占同一 device（HW-01） |
| F12 | **Media Controller (browser) MUST isolate hls.js/WHEP/WebCodecs from React render** | React 只消费状态，不直接持有媒体对象（FRONT-02） |

---

## 5. 与四套文档的对应关系

- **GSTR-01（GStreamer ownership 未闭合）** → 本节 §3/§4 F4 锁死：Media Agent owns GStreamer pipeline lifecycle。
- **TECH-02（Live vs Async FFmpeg）** → §3 双 owner；§4 F1/F3。
- **RECORD-01（Recording 灰区）** → §3 Live Recording→Agent，Post-processing→Worker。
- **TECH-04（SRS vs MediaMTX）** → §1 / §4 F8；SRS 锁定，MediaMTX 退出 baseline。
- **TECH-05（AntD Pro 漂移）** → §1 已锁 shadcn/ui（SYSTEM_AND_PROJECT_PLAN 第 245/297 行已落实）。
- **TECH-06（Plan 身份）** → §0 分层；SYSTEM_AND_PROJECT_PLAN 头部须声明 "V0.1 Planning / Reconciled with V0.2"。

### 5.1 第 43 轮复核新增落点（基于 commit `c232206` 复核，`62a7687` 为 rebase 前旧 hash）

| 类别 | 问题 | 落点 |
|---|---|---|
| GSTR-02 | Pipeline 构造归属未明文 | §3.1 + §4 F10（Agent materialize，Graph Compiler 只产 Intent） |
| TECH-02 (enforcement) | 缺工程级 Invocation Guard | §4 F9（lint / repo-check 建议，见 §6 Gate A） |
| BOUNDARY-06 / RECORD-02 | Runtime Artifact → Async Job 交接 | §3.2 Handoff Contract |
| GSTR-03 / ACC-03 | Agent restart / GStreamer crash recovery 未充分验收 | Phase 0.6 补 acceptance 项（见 §6 Gate B）；本文档 §4 F4 已锁 lifecycle owner |
| HW-01 | DeckLink 独占/租约缺失 | §4 F11（Device Registry + Exclusive Lease） |
| GRAPH-01 | Intent vs concrete runtime materialization | §3.1 / §4 F10 |
| FRONT-02 | Media Controller 边界未定义 | §3.3（packages/media-runtime 形态） |
| UX-03 | Runtime Owner 可见性 | Phase 4 Health Tree 须含 Runtime Owner 链（Agent/host/session/pipeline/recovery） |
| STREAM-03 | Gateway capability ↔ protocol 映射 | Object Model 层下一步；Output Variant = {Adapter, Capability, Endpoint, Protocol, Latency, Health} |
| ACC-03 | G-DOC Runner 真实执行 | 代码工作，不在本文档范围；见 §6 Gate B |

> 本文件是 G-DOC 前置 Gate 的 SoT 输入：任何后续技术栈变更须先过本契约与 V0.2 Architecture Change Review。

---

## 6. 下一阶段 Gate（Ownership Enforcement，非架构 Review）

> **原则**：V0.2 继续冻结；不再做第 N 轮架构 Review。下一步把本契约从"文档"转为"可执行约束 + 验收项"。

### Gate A — Ownership Enforcement（P1，工程层）
1. **FFmpeg Invocation Guard**：在 `api/` (Fastify/Control Plane) 加 lint/repo-check，禁止 `child_process.spawn("ffmpeg"...)`；Live FFmpeg 仅 `media-agent/` 可 spawn，File FFmpeg 仅 `worker/` 可 spawn（F9）。
2. **DeckLink Exclusive Lease**：Media Agent 在 Hardware Discovery 后建 Device Registry + Exclusive Lease + Session Owner；任何第二 opener（诊断工具 / Recording job / Operator 误改）被拒绝（F11）。
3. **Graph Compiler 输出断言**：CI 检查 Graph Compiler 产物不含 `gst-launch`/`ffmpeg` CLI 字符串，只含 Runtime Graph Intent（F10）。

### Gate B — G-DOC / G-RUNTIME（P0/P1）
1. **Runner 真实化**：Phase 0.6 runner 真正执行 `pass_rule`，不再仅 STRUCTURE/COVERAGE 层面（ACC-03）。
2. **GStreamer ownership runtime test**：新增 acceptance 项——
   - `Media Agent restart → GStreamer pipeline recovery → DeckLink session recover → downstream state recover`
   - `GStreamer process crash → Agent detects → restart pipeline → health DEGRADED → recovery`
3. **执行顺序**：ENV Preflight → A1 → A2 → B → FI → HA → G-UIUX。

### Gate C — Phase 1 Implementation Contract
- Runtime Artifact → Async Job Handoff 落地为消息契约（§3.2）。
- Media Controller `packages/media-runtime` 在 Phase 4 前冻结（§3.3）。
- SRS Output Variant capability 映射进入 Object Model（STREAM-03）。
