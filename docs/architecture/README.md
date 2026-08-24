# V0.2 Architecture — Quick Reference

> 完整内容见 [`ARCHITECTURE_V0.2.md`](ARCHITECTURE_V0.2.md)（192KB / 4021 lines）。
> 本文件是**快速参考卡**，用于日常查找关键定义。

## 🚦 Status

```yaml
V0.2 Architecture Baseline:  LOCK FINAL
V0.2 Runtime Semantics:      CLOSED
implementation_ambiguity:    NONE
review_passes:               22
architecture_changes_after:  FORBIDDEN
v0_2_5:                      FORBIDDEN
```

## 🏗️ 12 Engines + 5 横向 + 6 横切

| 编号 | 名称 | 说明 |
|---|---|---|
| 1 | **Source** | 11 个 Source Adapter（SDI/SRT/RTMP/HLS/...） |
| 2 | **Switcher** | 3 Switch Mode（PACKET/FRAME/MASTER） |
| 3 | **Playout** | 节目单 / 时间线 / 插播 |
| 4 | **Composition** | 图文包装（RAW 域，Program + Variant 两级） |
| 5 | **Audio** | 混音 / 响度 / 延迟 |
| 6 | **Output** | 多路分发（SRS Adapter） |
| 7 | **Recording** | 收录 / 分段（5 min/段） |
| 8 | **Replay** | 延时 / 回放 |
| 9-12 | (V0.2 Locked 4 子项) | — |
| 横切 | **X1-X6** | Compiler / Preflight / Versioning / Incident / Health Tree / Capability |

## 📊 Data Plane 4 Layer × 7 Type

| Layer | Types |
|---|---|
| **ELEMENTARY** | COMPRESSED_VIDEO, COMPRESSED_AUDIO, RAW_VIDEO, RAW_AUDIO |
| **CONTAINER** | MULTIPLEXED (TS/MP4/...) |
| **METADATA** | METADATA (SCTE-35/KLV/Timecode/...) |
| **CONTROL** | EVENT (QC Alert/Switch Event/...) |

> ⚠️ **DECODED 是过程，不是 Data Plane 类型。**

## 🔀 Switch Mode 3

| Mode | 实现 | 决策 SoT |
|---|---|---|
| **PACKET_SWITCH** | 压缩流层直接换 | §3.4 (同一 Mandatory Capability Contract) |
| **FRAME_SWITCH** | 主备都 decode → RAW 层切 → 重新 encode | §3.4 |
| **MASTER_SWITCH** | 主备都 normalize → 统一 Master → 在 Master 切 | §3.4 |

> SwitchDecisionResult = PACKET_SWITCH / FRAME_SWITCH / MASTER_SWITCH / **REJECT**（REJECT ≠ SwitchMode）

## 🌡️ Hot-Standby Level 3 (Policy/Target)

| Level | target_failover_time_ms | use_case |
|---|---|---|
| **COLD** | 30000 | 灾备录播 |
| **WARM** | 1500 | 公共服务频道 |
| **HOT** | 100 | 新闻/直播/广告插播 |

> target 是预算，不是协议保证。实测由 `failover_benchmarks` 表（§5）记录。
> Runtime State 唯一由 §8.11 三轴状态机表达。

## 🩺 Health Tree SoT = §3.9

7 Health Invariants（V0.2.4 Errata-14 C.26 锁定）：

| ID | Condition | Channel Result |
|---|---|---|
| **H1** | no ACTIVE+FAILED | (no fire) |
| **H2** | no ACTIVE+DEGRADED | (no fire) |
| **H3** | no STANDBY+FAILED | (no fire) |
| **H4** | no STANDBY+DEGRADED | (no fire) |
| **H5** | OFFLINE+FAILED | 系统已吸收（NO_DIRECT_CHANNEL_DEGRADATION） |
| **H6** | Source RG 全部候选不可用 | **FAILED** |
| **H7** | effective_channel_status = channel_health_view | 唯一入口 |

## 📊 三轴状态机（§8.11）

```
Lifecycle:    STOPPED  →  STARTING  →  RUNNING  →  STOPPING
Readiness:    NOT_READY  ↔  READY_TO_TAKE
Health:       HEALTHY / DEGRADED / FAILED / UNKNOWN
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

## 💥 Failure Domain 9

| Domain | 7 Operational | 2 Diagnostic |
|---|---|---|
| SOURCE | ✓ | |
| PIPELINE | ✓ | |
| MASTER | ✓ | |
| OUTPUT | ✓ | |
| RECORDING | ✓ | |
| CLOCK | ✓ | |
| RESOURCE | ✓ | |
| PLAYER | | ✓（NOTIFY only） |
| UNKNOWN | | ✓（SAFE_DEGRADE） |

> PLAYER / UNKNOWN **不**触发 Failover；只 NOTIFY / SAFE_DEGRADE。

## 🚨 Switch Mode Decision Tree（§3.4 摘要）

```
1. PACKET_SWITCH eligibility
   - sources = Capability Contract equal (mandatory attrs)
   - runtime alignment OK
2. FRAME_SWITCH (decode to RAW, re-encode)
3. MASTER_SWITCH (normalize to common Master)
4. REJECT (insufficient)
```

## 🔐 Canonical Vocabulary（TS / Rust / JSON Schema / PG enum 共享）

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

## ❌ Forbidden（V0.2 已锁）

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
