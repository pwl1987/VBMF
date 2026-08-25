# 变更日志

VBMF（IP Broadcast Media Fabric）的所有重要变更都记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
本项目遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/spec/v2.0.0.html)。

> **V0.2 架构基线已 LOCK FINAL**（22 轮 review）。
> 任何 V0.2.x 架构 review 已永久关闭。Phase 0.5/0.6 阶段以 Acceptance Validation 为主。

## [未发布]

### Phase 0.5F — Final Consistency & Safety Closure（0.5F.11，2026-08-25，LOCK FINAL）
- **P0-1 运行控制语义闭环**：CD-01 修 REQUIRED Output→TAKE 规则矛盾——REQUIRED 输出不健康/被禁用必须 TAKE = BLOCKED；新增 Emergency Override（L3 + Audit + Who/Why）逃生通道。
- **P0-2 Phase 状态唯一事实源（SoT）**：MILESTONES.md 声明为 Phase 阶段状态 SoT；README / Phase README / ROADMAP 派生一致；0.5F 子项编号清理（FG-01..FG-07 全 ✅）。
- **P1-1 E-40 动态分支**：Kind Selector 交互切换 5 个 Kind 表单（Network/Physical/File/Internal/Composite），只渲染选中分支，非静态展板。
- **P1-2 E-42 真实验证档**：新增 Validation Profile Selector，5 个 Kind 各含真实验证步骤（Network/Physical/File/Internal 5 层、Composite 7 层），预览独立成 panel。
- **P1-3 E-40 Composite 修正**：GraphSpec → Child Sources → Compile → Verify 流程，明确 Graph ≠ Source（Composite Source 包装的是 Graph/Route，非另一 Source）。
- **P1-4 M-17 命名统一**：全仓库 "Realtime Transcode" 残留文本统一为 "Realtime Session 实时媒体会话"（POM/PIA/ENCODE_MODEL/E-41/M-14/OBJECT_VOCABULARY/SURFACE_REGISTRY/SURFACE_SPEC/ROADMAP/0.5C closure）；RECONCILIATION 历史决策记录保留原 rename 表述。
- 校验：`scripts/check_docs.py` 链接可达性 + 数字口径一致性 **PASS**。完成后 Phase 0.5 可真正冻结并进入 Phase 0.6 Executable Acceptance。

### Phase 0.5C — Information Architecture Closure（2026-08-25，DRAFT 0.1 待审）
- 目录归并：`docs/phase-0.5b/` 并入 `docs/phase-0.5/`（git mv 保留 history；wireframes 拆为 `operator/` 10 张 + `product/` 5 张）
- 新增 `OBJECT_VOCABULARY.md`（14 对象权威定义）/ `PRODUCT_OBJECT_MODEL.md`（Profile/Bundle/Variant 3 层）/ `NAVIGATION.md` / `MILESTONES.md` + `milestones/` 历史归档
- UI 顶层导航从 6 编号域改为 4 业务域（BROADCAST / MEDIA / ENGINEERING / ADMIN）
- Phase 0.6 README 语义修复（failover 时延验收写法：target_failover_time_ms + 实测 p50/p95/p99，禁止协议式保证）
- 0.5C.1 回写与对账（本轮）：README 引擎/横向系统名单按架构 §2.1/§2.2 修正；根目录残留副本清理；`.gitignore` 排雷（`*.ts` 全局忽略会吞掉 Phase 2/4 TypeScript 源码）；product wireframe 死链修复；ROADMAP/CHANGELOG/docs README 门面同步

### Phase 0.5B — Product UI Surface（UX BASELINE LOCK FINAL）
- 0.5B.0：`SURFACE_SPEC.md`（38 编号 UI 表面 = 0.5A 10 + 新增 28；Closure-1 另增 CD-01，现总 39）+ 13 项 P0 语义收口（SP-P0-1..13）+ `I18N_SPEC.md`（zh-CN + en-US 契约 / Canonical Vocabulary / 11 enum 翻译表）
- 0.5B.1：5 张 P0 wireframe（M-11 Media Library / M-12 Asset Detail / M-14 Transcode Center / P-21 Encoding Profile / P-22 Output Profile）
- Closure-1：10 项产品化收口（Output Profile/Variant/Destination/Adapter 4 元组、DESIRED/COMPILED/EFFECTIVE 三层、Dependency Preview 等）
- 0.5B.2：8 项 P0 + 5 项 P1 UX 收口 + `DESIGN_SYSTEM.md`（token / 组件 / 状态模型 / 键盘）→ UX BASELINE LOCK FINAL
- 语义收口合计 36 项（31 P0 + 5 P1），明细见 `docs/phase-0.5/SURFACE_SPEC.md` §30 收口项附录

### Phase 0.5A — Operator Semantics（LOCK FINAL）
- `OPERATOR_WORKFLOW.md` + 10 个双语 wireframe（`operator/`，9 Core + 1 Validation）+ 4 关键操作链（On-Air / Failure / Playout / Engineering）
- 20 项 UI 语义修复（`ERRATA.md`：12 P0 + 8 P1）

### 新增
- 初始化开源仓库 + 开源文件：README、LICENSE（Apache 2.0）、.gitignore、CONTRIBUTING、CHANGELOG、ROADMAP、SECURITY

## [V0.2] - 2026-08-24 — 架构基线 LOCK FINAL

### 22 轮 review 锁定
- 19 轮 review（V0.2.1 → V0.2.4） + Cleanup-1/2/3 + Errata-1/2/3/4/5/6/7/8/9/10/11/12/13/14
- 详见 `docs/architecture/ARCHITECTURE_V0.2.md` 附录 C（C.1-C.26 + V0.2 终态块）

### 核心架构
- **12 Engines（§2.1）**：Source / Signal Fabric / Normalize / Redundancy / QC / Playout / Switcher / Composition / Audio / Output / Recording / Replay
- **5 横向系统（§2.2，V0.1 既有）**：H1 Safety / H2 Resource Scheduler / H3 Watchdog & Incident / H4 Audit / H5 Subtitle
- **6 横切能力 (X1-X6)**：Graph Compiler / Preflight / Configuration Versioning / Incident Timeline / Health Tree / Capability Registry
- **22 原则**（V0.2.1 16 → V0.2.2 22）
- **57 决策**（V0.2-final 40 → Cleanup-2 +3 → Errata-2 +4 → Errata-3 +4 → Errata-6 +2 → Errata-9 +4）

### Data Plane
- 4 Layer：ELEMENTARY / CONTAINER / METADATA / CONTROL
- 7 类型：COMPRESSED_VIDEO / COMPRESSED_AUDIO / RAW_VIDEO / RAW_AUDIO / METADATA / EVENT / DECODED-process
- Switch Mode 3：PACKET_SWITCH / FRAME_SWITCH / MASTER_SWITCH
- Switch Decision Result 4：PACKET_SWITCH / FRAME_SWITCH / MASTER_SWITCH / REJECT
- Hot-Standby Level 3：COLD / WARM / HOT（Policy/Target，非 Runtime State）

### Runtime Semantics（CLOSED）
- **Lifecycle**：STOPPED / STARTING / RUNNING / STOPPING
- **Readiness**：NOT_READY / READY_TO_TAKE
- **Health**：HEALTHY / DEGRADED / FAILED / UNKNOWN
- **EffectiveChannelStatus**：HEALTHY / DEGRADED / FAILED / STARTING / STOPPED / UNKNOWN
- **Failure Model**：OperationalFailureDomain (7) + DiagnosticFailureClass (2: PLAYER / UNKNOWN)
- **Health Tree SoT**：§3.9 Health Tree Aggregation Policy
- **7 Health Invariants**：H1-H7（C.26 Errata-14）

### Schema（V0.2.4 Errata-11/12/13/14）
- `media_session_runtime` 表：lifecycle / readiness / health / effective_switch_mode / runtime_alignment_state
- `current_health_trees` View：DISTINCT ON channel_id 取最新 snapshot
- `channel_health_aggregation` View：7 规则真执行 ACTIVE/STANDBY/OFFLINE/Subsystem/RG
- `channel_health_view` View：真执行 effective_channel_status_policy（CASE + LEFT JOIN）
- `health_tree_nodes` Schema：subsystem + redundancy_group_id + node_role + required_node + state
- `effective_channel_status_policy`：precedence + rules（STOPPED / STARTING / FAILED / DEGRADED / HEALTHY / UNKNOWN）

### 禁止项（V0.2.4 Errata 锁定）
- ❌ 新增 Engine（12 是最终）
- ❌ V0.2 阶段新增 Source Adapter（RIST/Zixi/NDI 等 V0.3）
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

## 版本说明

- **V0.2 LOCK FINAL**：任何架构级扩展必须开 V0.3 流程
- **Phase 0.5**：0.5A Operator Semantics（LOCK FINAL）+ 0.5B Product Surface（UX BASELINE LOCK FINAL）+ 0.5C Info Arch（DRAFT）+ 0.5D P0 Product Surfaces（待开始）
- **Phase 0.6**：Reference A1/A2/B + 5 Fault Injection + 7 Health Invariants = Executable Acceptance Specification（前置: Phase 0.5 LOCK FINAL）
- **Phase 1**：Media Agent (Rust) + Session Manager + FFmpeg Command Builder + 24h 稳定性
- **Phase 4**：Web Console (4 域 × 44 UI 表面 + 4 链验证)
