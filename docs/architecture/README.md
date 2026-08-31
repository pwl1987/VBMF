# 文档索引

## 📌 阶段导航（一眼分清「本次实施」）

### ✅ 实施现状（2026-08-30 对账, p07b-consolidation）— Phase 0.6 ✅ + 0.7A Session Runtime ✅ + 0.7B Media Semantics ✅（实施路线 SoT: [`PHASE_IMPLEMENTATION_MAP.md`](PHASE_IMPLEMENTATION_MAP.md)）
> 0.6 全部 P0 缺口已闭合（PR#1 merged `d1cfaa9`）；0.7A Session Runtime（PR#2 `adc6f19`，四轮 Hardening）；0.7B Canonical Media Semantics 四基础齐备（Normalize/Clock/Audio/Timecode，PR#3-#6）。下一阶段 0.7C External Integration（前置债务见 Phase Map §4）。
> 下面这些契约 **CONTRACT = FROZEN**，是实施的规范依据（P0 契约 IMPLEMENTATION 已对齐实态；Clock/Timecode/Audio 契约的 canonical 语义层已落地（状态见各契约行），完整 Runtime 化与 External 契约实施属 0.7C/0.7D）。

- 综合契约（权威载体）：[`IMPLEMENTATION_ADDENDUM.md`](IMPLEMENTATION_ADDENDUM.md) ［P0］
- 决策日志：[`ARCHITECTURE_DECISION_LOG.md`](ARCHITECTURE_DECISION_LOG.md) ［P0］
- 边界契约：[`IMPLEMENTATION_BOUNDARIES.md`](IMPLEMENTATION_BOUNDARIES.md) ［P0］
- 硬件 Provider SPI：[`HARDWARE_PROVIDER_CONTRACT.md`](HARDWARE_PROVIDER_CONTRACT.md) ［P0］
- 媒体 Backend SPI：[`MEDIA_BACKEND_CONTRACT.md`](MEDIA_BACKEND_CONTRACT.md) ［P0］
- Canonical 媒体模型：[`CANONICAL_MEDIA_MODEL.md`](CANONICAL_MEDIA_MODEL.md) ［P0］
- 资源模型：[`RUNTIME_RESOURCE_MODEL.md`](RUNTIME_RESOURCE_MODEL.md) ［P0.5］
- 可移植性矩阵：[`TECHNOLOGY_PORTABILITY_MATRIX.md`](TECHNOLOGY_PORTABILITY_MATRIX.md) ［P0］
- 厂商中立守则：[`VENDOR_NEUTRALITY_RULES.md`](VENDOR_NEUTRALITY_RULES.md) ［P0］
- Session 模型：[`RUNTIME_SESSION_MODEL.md`](RUNTIME_SESSION_MODEL.md) ［P0］
- Canonical 身份：[`CANONICAL_IDENTITY.md`](CANONICAL_IDENTITY.md) ［P0］
- 拓扑契约：[`RUNTIME_TOPOLOGY_CONTRACT.md`](RUNTIME_TOPOLOGY_CONTRACT.md) ［P0.5］
- Binding 模型：[`RUNTIME_BINDING_MODEL.md`](RUNTIME_BINDING_MODEL.md) ［P0］
- **整合 Master PRD（两份 PRD + 评审 + 缺口 + 路径 + 门禁）**：[`PHASE_0_6_MASTER_PRD.md`](PHASE_0_6_MASTER_PRD.md) ［P0 整合］
- 讨论留档：[`REVIEW_2026-08-28.md`](REVIEW_2026-08-28.md) ［P0］
- 状态统一模型：[`DOCUMENT_STATUS_MODEL.md`](DOCUMENT_STATUS_MODEL.md) ［P0 全局规则］
- 实现缺口矩阵：[`PHASE_0_6_IMPLEMENTATION_GAP_MATRIX.md`](PHASE_0_6_IMPLEMENTATION_GAP_MATRIX.md) ［P0 SoT：文档≠代码］
- 验收门禁矩阵：[`PHASE_0_6_ACCEPTANCE_MATRIX.md`](PHASE_0_6_ACCEPTANCE_MATRIX.md) ［P0］
- 运行时生命周期时序：[`RUNTIME_LIFECYCLE_SEQUENCE.md`](RUNTIME_LIFECYCLE_SEQUENCE.md) ［P0］
- V0.2↔0.6 语义对照：[`V0_2_TO_PHASE_0_6_CROSSWALK.md`](V0_2_TO_PHASE_0_6_CROSSWALK.md) ［P0］

### 🔵 Phase 0.7（P1）— 0.7A/0.7B 已实施 ✅；0.7C/0.7D 待实施（Clock/Timecode/Audio 的 canonical 层已落地，路线见 [`PHASE_IMPLEMENTATION_MAP.md`](PHASE_IMPLEMENTATION_MAP.md)）
- 外部 API：[`EXTERNAL_API_CONTRACT.md`](EXTERNAL_API_CONTRACT.md) ［P1］
- 事件：[`EVENT_CONTRACT.md`](EVENT_CONTRACT.md) ［P1］
- 设备集成：`DEVICE_INTEGRATION_CONTRACT.md` ［P1］（多站点/agent 部分属 P2，见下）
- 前提：需先有 Control Plane（Fastify/Node），当前不存在

### 🟠 Phase 0.8（P2）暂缓 — 「连契约都暂缓」
- Multi-site / Agent / Federation / Artifact / Recording-Playback 等，对应 API PRD #138–#170。
- **不全盘接受**：见 [`PRD_REVIEW_EXTERNAL_API.md`](PRD_REVIEW_EXTERNAL_API.md) §0.5 / §3.5（I–Q），不进入 0.7 实施清单。

### 🟢 V0.2 基线（LOCK FINAL，冻结，不修改）
- [`ARCHITECTURE_V0.2.md`](ARCHITECTURE_V0.2.md)

### 📂 既有 / 参考文档（非本 Phase 0.6 规划，历史参考）
- [`MEDIA_AGENT_STATE_MACHINE.md`](MEDIA_AGENT_STATE_MACHINE.md)
- [`MEDIA_RUNTIME_SECURITY_MODEL.md`](MEDIA_RUNTIME_SECURITY_MODEL.md)
- [`TECHNOLOGY_STACK_AND_RUNTIME_OWNERSHIP.md`](TECHNOLOGY_STACK_AND_RUNTIME_OWNERSHIP.md)
- [`DEPLOYMENT_AND_DEV_RUNTIME.md`](DEPLOYMENT_AND_DEV_RUNTIME.md)

### 📝 评审 / 讨论（非实施）
- PRD 评审（Portability）：[`PRD_REVIEW_RUNTIME_ABSTRACTION.md`](PRD_REVIEW_RUNTIME_ABSTRACTION.md)
- PRD 评审（API，第二轮严格批判）：[`PRD_REVIEW_EXTERNAL_API.md`](PRD_REVIEW_EXTERNAL_API.md)

> V0.2 基线 LOCK FINAL，架构改动须 V0.3 流程拍板。Phase 0.6 文档均为实现层契约补充，不修改 V0.2 语义。

---

# V0.2 架构 — 快速参考

> 完整内容见 [`ARCHITECTURE_V0.2.md`](ARCHITECTURE_V0.2.md)（192KB / 4020 lines）。
> 本文件是**快速参考卡**，用于日常查找关键定义。

## 🚦 状态

```yaml
V0.2 架构基线:            LOCK FINAL
V0.2 Runtime Semantics:   CLOSED
implementation_ambiguity: NONE
review_passes:            22
architecture_changes_after: FORBIDDEN
v0_2_5:                   FORBIDDEN
```

## 🏗️ 12 Engines + 5 横向 + 6 横切

> 编号与名称以 §2.1 / §2.2 为唯一权威。

| 编号 | 名称 | 说明 |
|---|---|---|
| 1 | **Source 源** | 11 类输入（SDI/SRT/RTMP/HLS/FILE/INTERNAL/COMPOSITE/...） |
| 2 | **Signal Fabric** | 路由 / 矩阵 / 边策略 |
| 3 | **Normalize** | 格式归一（能力可拆） |
| 4 | **Redundancy** | 主备 / 切换（PACKET/FRAME/MASTER 模式） |
| 5 | **QC** | 信号质量监测 |
| 6 | **Playout 播控** | 虚拟播控 / 时间线 / 插播 |
| 7 | **Switcher 切播** | 按 Switch Mode 执行切换 |
| 8 | **Composition 图文** | 图文包装（Program + Variant 两级；Master Join 为其子能力，非独立 Engine） |
| 9 | **Audio 音频** | 混音 / 响度 / 延迟 / 同步 |
| 10 | **Output 输出** | 多路分发（含 SRS Gateway Adapter） |
| 11 | **Recording 录制** | 收录 / 分段（5 min/段） |
| 12 | **Replay 回放** | 延时 / 回放 |
| 横向 | **H1-H5** | Safety / Resource Scheduler / Watchdog & Incident / Audit / Subtitle（V0.1 既有） |
| 横切 | **X1-X6** | Compiler / Preflight / Versioning / Incident / Health Tree / Capability |

## 📊 Data Plane 4 Layer × 7 Type

| Layer 层 | Types 类型 |
|---|---|
| **ELEMENTARY 元素** | COMPRESSED_VIDEO, COMPRESSED_AUDIO, RAW_VIDEO, RAW_AUDIO |
| **CONTAINER 容器** | MULTIPLEXED (TS/MP4/...) |
| **METADATA 元数据** | METADATA (SCTE-35/KLV/Timecode/...) |
| **CONTROL 控制** | EVENT (QC Alert/Switch Event/...) |

> ⚠️ **DECODED 是过程，不是 Data Plane 类型。**

## 🔀 Switch Mode 切播模式 3 种

| Mode | 实现 | 决策 SoT |
|---|---|---|
| **PACKET_SWITCH** | 压缩流层直接换 | §3.4（同一 Mandatory Capability Contract） |
| **FRAME_SWITCH** | 主备都 decode → RAW 层切 → 重新 encode | §3.4 |
| **MASTER_SWITCH** | 主备都 normalize → 统一 Master → 在 Master 切 | §3.4 |

> SwitchDecisionResult = PACKET_SWITCH / FRAME_SWITCH / MASTER_SWITCH / **REJECT**（REJECT ≠ SwitchMode）

## 🌡️ Hot-Standby Level 3（Policy/Target）

| Level | target_failover_time_ms | use_case |
|---|---|---|
| **COLD** | 30000 | 灾备录播 |
| **WARM** | 1500 | 公共服务频道 |
| **HOT** | 100 | 新闻/直播/广告插播 |

> target 是预算，不是协议保证。实测由 `failover_benchmarks` 表（§5）记录。
> Runtime State 唯一由 §8.11 三轴状态机表达。

## 🩺 Health Tree SoT = §3.9

7 Health Invariants（V0.2.4 Errata-14 C.26 锁定）：

| ID | Condition 条件 | Channel Result 通道结果 |
|---|---|---|
| **H1** | ACTIVE + FAILED | **FAILED** |
| **H2** | ACTIVE + DEGRADED | **DEGRADED** |
| **H3** | STANDBY + FAILED | **DEGRADED** |
| **H4** | STANDBY + DEGRADED | **DEGRADED** |
| **H5** | OFFLINE+FAILED | 系统已吸收（NO_DIRECT_CHANNEL_DEGRADATION） |
| **H6** | Source RG 全部候选不可用 | **FAILED** |
| **H7** | effective_channel_status = channel_health_view | 唯一入口 |

## 📊 三轴状态机（§8.11）

```
Lifecycle 生命周期:    STOPPED  →  STARTING  →  RUNNING  →  STOPPING
Readiness 就绪:        NOT_READY  ↔  READY_TO_TAKE
Health 健康:           HEALTHY / DEGRADED / FAILED / UNKNOWN
```

> Channel 对外 status = `channel_health_view.effective_channel_status`（**禁止** UI 直接读 `media_session_runtime.health` 当 Channel Status）

## 🌐 EffectiveChannelStatus Policy

| 优先级 | 条件 | 结果 |
|---|---|---|
| 1 | lifecycle = STOPPED | STOPPED |
| 2 | lifecycle ∈ {STARTING, STOPPING} | STARTING |
| 3 | channel_health_aggregation = FAILED | FAILED |
| 4 | channel_health_aggregation = DEGRADED | DEGRADED |
| 5 | lifecycle = RUNNING + aggregation = HEALTHY | HEALTHY |
| 6 | (else) | UNKNOWN |

## 💥 Failure Domain 故障域 9 类

| 类别 | 7 Operational 运行 | 2 Diagnostic 诊断 |
|---|---|---|
| SOURCE 源 | ✓ | |
| PIPELINE 管道 | ✓ | |
| MASTER 主母版 | ✓ | |
| OUTPUT 输出 | ✓ | |
| RECORDING 录制 | ✓ | |
| CLOCK 时钟 | ✓ | |
| RESOURCE 资源 | ✓ | |
| PLAYER 播放端 | | ✓（NOTIFY only） |
| UNKNOWN 未知 | | ✓（SAFE_DEGRADE） |

> PLAYER / UNKNOWN **不**触发 Failover；只 NOTIFY / SAFE_DEGRADE。

## 🚨 Switch Mode Decision Tree 切播决策树（§3.4 摘要）

```
1. PACKET_SWITCH eligibility
   - sources = Capability Contract equal (mandatory attrs)
   - runtime alignment OK
2. FRAME_SWITCH (decode to RAW, re-encode)
3. MASTER_SWITCH (normalize to common Master)
4. REJECT (insufficient)
```

## 🔐 Canonical Vocabulary 规范词汇（TS / Rust / JSON Schema / PG enum 共享）

```yaml
DataPlaneLayer:        ELEMENTARY | CONTAINER | METADATA | CONTROL
ElementaryDataType:    COMPRESSED_VIDEO | COMPRESSED_AUDIO | RAW_VIDEO | RAW_AUDIO
SwitchMode:            PACKET_SWITCH | FRAME_SWITCH | MASTER_SWITCH
SwitchDecisionResult:  PACKET_SWITCH | FRAME_SWITCH | MASTER_SWITCH | REJECT
HotStandbyLevel:       COLD | WARM | HOT
CapabilityCheckResult: PASS | WARN | FAIL
HealthState:           HEALTHY | DEGRADED | FAILED | UNKNOWN   # RUNTIME_NODE_HEALTH_FACT
LifecycleState:        STOPPED | STARTING | RUNNING | STOPPING
ReadinessState:        NOT_READY | READY_TO_TAKE
OperationalFailureDomain: SOURCE | PIPELINE | MASTER | OUTPUT | RECORDING | CLOCK | RESOURCE
DiagnosticFailureClass:  PLAYER | UNKNOWN
EffectiveChannelStatus: HEALTHY | DEGRADED | FAILED | STARTING | STOPPED | UNKNOWN   # CHANNEL_PRESENTATION_STATUS
RuntimeAlignmentAttr:  GOP_BOUNDARY | IDR_ALIGNMENT | TIMESTAMP_CONTINUITY | ...
ResourceDimension:     CPU_THREADS | GPU_SESSIONS | VRAM_MB | RAM_MB | INGRESS_MBPS | ...
DeviceToken:           BMD_INPUT_PORT | BMD_OUTPUT_PORT
DeviceConstraint:      DEVICE_EXCLUSIVITY
NodeRole:              ACTIVE | STANDBY | OFFLINE
Subsystem:             SOURCE | SWITCHER | COMPOSITION | AUDIO | MASTER | OUTPUT | RECORDING | CLOCK | RESOURCE

# V0.2.4 Errata-10 废除: FailureDomain (alias)
#   = historical terminology
#   = replaced by OperationalFailureDomain + DiagnosticFailureClass
```

## ❌ 禁止项（V0.2 已锁）

- ❌ 新增 Engine（12 是最终）
- ❌ V0.2 Source Adapter 之外的新协议（RIST/Zixi/NDI 等 V0.3）
- ❌ 修改 Switch Mode 3 / Data Plane 4 Layer / Hot-Standby 3
- ❌ 写绝对 target 数字（`<100ms` / `0.5-2s` / `1-3s`）
- ❌ 把 `current_host_snapshot` 写进 Architecture
- ❌ 把 `pcie_*_mb_s` 当成实测值
- ❌ `effective_switch_mode` 反写 `channel_routes.switch_mode`
- ❌ 任何 V0.2.x 架构 review（永久关闭）

## 📐 关键 Schema 引用

详见 [`ARCHITECTURE_V0.2.md §5`](ARCHITECTURE_V0.2.md)：

- `media_session_runtime` (lifecycle/readiness/health/effective_switch_mode/runtime_alignment_state)
- `health_trees` / `health_tree_nodes` (subsystem / redundancy_group_id / node_role / required_node / state)
- `current_health_trees` View (DISTINCT ON)
- `channel_health_aggregation` View (7 规则)
- `channel_health_view` View (effective_channel_status_policy)
- `hot_standby_levels` (description / target_failover_time_ms / use_case；**无** resource_factor / state)
- `failover_benchmarks` (Runtime Measurement，p50/p95/p99)
- `config_revisions` / `change_sets` (X3)
- `incidents` (X4)
- `switch_modes` (id/name/description；**无** max_failover_ms)

## 🔗 完整链接

- 完整架构基线：[`ARCHITECTURE_V0.2.md`](ARCHITECTURE_V0.2.md)
- Phase 0.5 操作员层：[`../phase-0.5/`](../phase-0.5/)
- 部署参考（V0.2 §3.11 current_host_snapshot）：32 核 / 30 GB / 546 GB / 3 张 BMD DeckLink
- 完整路线图：[`../../ROADMAP.md`](../../ROADMAP.md)
