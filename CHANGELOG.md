# Changelog

All notable changes to VBMF (IP Broadcast Media Fabric) are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **V0.2 Architecture Baseline is LOCK FINAL** (22 review rounds).
> 任何 V0.2.x 架构 review 已永久关闭。Phase 0.5/0.6 阶段以 Acceptance Validation 为主。

## [Unreleased]

### Added
- Initial open-source release preparation
- Phase 0.5 deliverables:
  - `docs/phase-0.5/INDEX.md` — Phase 0.5 总览
  - `docs/phase-0.5/OPERATOR_WORKFLOW.md` — 操作员工作流（角色 / 流程 / 危险操作分层）
  - `docs/phase-0.5/wireframes/` — 9 Low-Fi HTML wireframes (Dark Mode 24/7)
  - `docs/phase-0.5/chains/` — 4 关键操作链（On-Air / Failure / Playout / Engineering）
- Open source files: README, LICENSE (Apache 2.0), .gitignore, CONTRIBUTING, CHANGELOG, ROADMAP, SECURITY

## [V0.2] - 2026-08-24 — Architecture Baseline LOCK FINAL

### 22 review rounds locked
- 19 轮 review（V0.2.1 → V0.2.4） + Cleanup-1/2/3 + Errata-1/2/3/4/5/6/7/8/9/10/11/12/13/14
- 详见 `docs/architecture/ARCHITECTURE_V0.2.md` 附录 C（C.1-C.26 + V0.2 终态块）

### Core architecture
- **12 Engines**: Source / Switcher / Playout / Composition / Audio / Master / Output / Recording / Replay / (5 sub)
- **5 横向系统**: Source Adapter / Stream Gateway / Direct Output / Recording Distribution / Output Distribution
- **6 横切能力 (X1-X6)**: Graph Compiler / Preflight / Configuration Versioning / Incident Timeline / Health Tree / Capability Registry
- **22 原则** (V0.2.1 16 → V0.2.2 22)
- **57 决策** (V0.2-final 40 → Cleanup-2 +3 → Errata-2 +4 → Errata-3 +4 → Errata-6 +2 → Errata-9 +4)

### Data Plane
- 4 Layer: ELEMENTARY / CONTAINER / METADATA / CONTROL
- 7 types: COMPRESSED_VIDEO / COMPRESSED_AUDIO / RAW_VIDEO / RAW_AUDIO / METADATA / EVENT / DECODED-process
- Switch Mode 3: PACKET_SWITCH / FRAME_SWITCH / MASTER_SWITCH
- Switch Decision Result 4: PACKET_SWITCH / FRAME_SWITCH / MASTER_SWITCH / REJECT
- Hot-Standby Level 3: COLD / WARM / HOT (Policy/Target, NOT runtime state)

### Runtime semantics (CLOSED)
- **Lifecycle**: STOPPED / STARTING / RUNNING / STOPPING
- **Readiness**: NOT_READY / READY_TO_TAKE
- **Health**: HEALTHY / DEGRADED / FAILED / UNKNOWN
- **EffectiveChannelStatus**: HEALTHY / DEGRADED / FAILED / STARTING / STOPPED / UNKNOWN
- **Failure Model**: OperationalFailureDomain (7) + DiagnosticFailureClass (2: PLAYER / UNKNOWN)
- **Health Tree SoT**: §3.9 Health Tree Aggregation Policy
- **7 Health Invariants**: H1-H7 (C.26 Errata-14)

### Schema (V0.2.4 Errata-11/12/13/14)
- `media_session_runtime` 表：lifecycle / readiness / health / effective_switch_mode / runtime_alignment_state
- `current_health_trees` View：DISTINCT ON channel_id 取最新 snapshot
- `channel_health_aggregation` View：7 规则真执行 ACTIVE/STANDBY/OFFLINE/Subsystem/RG
- `channel_health_view` View：真执行 effective_channel_status_policy（CASE + LEFT JOIN）
- `health_tree_nodes` Schema：subsystem + redundancy_group_id + node_role + required_node + state
- `effective_channel_status_policy`：precedence + rules（STOPPED / STARTING / FAILED / DEGRADED / HEALTHY / UNKNOWN）

### Forbidden (V0.2.4 Errata 锁定)
- ❌ New Engine（12 是最终）
- ❌ New Source Adapter 在 V0.2 阶段（RIST/Zixi/NDI 等 V0.3）
- ❌ 修改 Data Plane / Switch Mode / Hot-Standby / Program Master
- ❌ 任何 V0.2.x 架构 review（永久关闭）
- ❌ 把 `current_host_snapshot` 内容写进 Architecture
- ❌ 把 `pcie_*_mb_s` 当成实测值
- ❌ 写死资源 / GPU / 设备型号

## [V0.1] - 2026-08-XX — Web 视频编码器（pre-VBMF）

- Docker compose 骨架
- 选型：FFmpeg / SRS
- 初始 SDI → RTMP 单链路
- 详见 `docs/SYSTEM_AND_PROJECT_PLAN.md`

---

## Versioning notes

- **V0.2 LOCK FINAL**: 任何架构级扩展必须开 V0.3 流程
- **Phase 0.5**: Operator Workflow + 9 Low-Fi 线框 + 4 关键操作链（已完成）
- **Phase 0.6**: Reference A1/A2/B + 5 Fault Injection + 7 Health Invariants = Executable Acceptance Specification
- **Phase 1**: Media Agent (Rust) + Session Manager + FFmpeg Command Builder + 24h stability
- **Phase 4**: Web Console (9 页面 + 4 链验证)
