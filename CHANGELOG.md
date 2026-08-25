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

### Phase 0.5F — Profile Ownership & Variant Delivery Closure（0.5F.13，2026-08-25，LOCK FINAL）
- **P0 · Packaging 归属焊死**：Output Variant 增 `packaging_profile_ref`（per-Variant 引用），未指定继承 `Bundle.packaging_profile_ref`（Default），指定则 Variant Override → `EFFECTIVE_PACKAGING = Bundle Default ↓ Variant Override`。支持 HLS / RTMP / UDP / File / WebRTC / 2110 多 Variant 共存（CH01 国内 HLS+CMAF / 海外 RTMP / 归档 MP4 不再共享单 Packaging）。OBJECT_VOCABULARY §1.8 + §1.16 新增。
- **P0 · Output Profile 唯一 SoT**：`output_profile_ref` 唯一权威 = Variant；Bundle 仅持 `default_output_profile_ref` 模板默认（实例化带入、可覆盖），禁止 "Bundle + Variant" 双真相。PRODUCT_OBJECT_MODEL §3.3 守卫。
- **P1 · Bundle Change 必须进 Configuration Surface**：P-28 `▾ Change` 改为先弹 Impact Preview（Affected: Encoding Session / Output Variant ×N / Resource / Reservation / Preflight；Risk 分级）→ Create ChangeSet，禁止就地下拉替换。
- **P1 · M-14 Workflow 重排**：推荐顺序对齐 Object Model——Asset → Output Version → Encoding → Packaging → Job Policy → Test Encode → Submit；并建议 Transcode Center 统一入口内分 [FILE] / [REALTIME] 两种运行模式。
- **P1 · 继承链可视化**：新增 Profile → Bundle → Variant → Runtime 5 层派生来源链，每屏派生值可展开 Inherited / Overridden / Explicit / Compiled / Effective（OBJECT_VOCABULARY §1.16 + PRODUCT_OBJECT_MODEL §3.5 + OBJECT_NAVIGATION_MATRIX §3）。
- **P1 · Source 连续流程 / UDP 网络上下文 / CD-01 工作驾驶舱**：在 0.5F.13 复检结论中记为后续可优化项（P2），本提交未展开，待 Phase 0.6 决定。
- 校验：一致性 95% / UI-UX 语义完整度 92%；**未新增任何 Surface**（Registry SoT 维持 56）；仅四份权威文档 + 两份 wireframe 内联提示收口。结论：Phase 0.5 Product/UX Semantics 可冻结，下一提交应进入 Phase 0.6 Executable Acceptance（AC-01/AC-02/AC-03）。

### Phase 0.6 Preflight / Acceptance Spec Correction（2026-08-25，基于 e9ebe6f）
- **P0-6.0-A1**：修正 Reference A1 链路——删除 PACKET_SWITCH 路径中错误的 Encode。PACKET_SWITCH = COMPRESSED → Switch → COMPRESSED（V0.2 §3.4 锁死）；Encode 仅出现于 RAW → Encode → COMPRESSED（Program Master delivery boundary）。
- **P0-6.0-A2**：修正 Reference A2 链路——FRAME_SWITCH = RAW → Switch → RAW、MASTER_SWITCH = RAW → Normalize → Master-level Switch → RAW；Encode 从 Switcher 前移到 Program Master / delivery boundary 之后（Frame/MASTER 验证的是 RAW 域切换，不是 COMPRESSED）。
- **P0-DOC-1**：README / ROADMAP 表面计数统一为 Registry SoT——删除手写 39 / 52 / 44，改为 56 surfaces（55 wireframes + 1 Spec，SoT: SURFACE_REGISTRY.yaml）；校验脚本 `check_docs.py` 改为从 Registry 解析 SoT（不再硬编码 39），SURFACE_SPEC §1 增补当前 SoT 总数 56。
- **P1**：Phase 0.6 Capability Contract 不再写 "17+"，改为 "Mandatory Compatibility Attributes = ALL PASS"（V0.2 §3.4 Canonical）；新增 Negative / Recovery Fixtures & Switch Test Matrix（PASS/WARN/FAIL/RECOVERY × Cold/Warm/Hot × PACKET/FRAME/MASTER × Failover/Failback/flapping/source loss/output loss/clock degradation）+ TAKE 语义锁定断言（TAKE = Operator Intent → Switch Command → active_source_id，非 Config/ChangeSet Apply）。
- **P1-UX**：E-42 验证结果摘要改为按当前 Profile 动态显示（NETWORK/PHYSICAL/FILE/COMPOSITE 7/7、INTERNAL 5/5），顶部徽标与结果判定同步；CD-01 Audio Delay 直接显示 L1 危险等级。
- **P1**：MILESTONES 第 2 节历史表补 0.5F.11 行（2 P0 + 4 P1 · Git e9ebe6f · LOCK FINAL）。
- 校验：`scripts/check_docs.py` **PASS**（链接可达 + 数字口径一致，SoT 取自 SURFACE_REGISTRY.yaml）。
- 结论：Phase 0.5 = 真正 LOCK FINAL；Phase 0.6 可正式进入 Executable Acceptance / Reference Implementation。

### Phase 0.6 Preflight Fix（2026-08-25，基于 8fd4d94）
- **P0-WebRTC**：M-17 ADAPTERS / 客户端计数 / OUTPUT VARIANTS 修正 WebRTC 方向——WHIP = Ingress/Publish（Source/Network），WHEP = Egress/Play（Browser Delivery）。M-17 原 "SRS · WHIP (Browser Delivery/Player Adapter)" 方向错误，改为 "SRS · WHEP (Browser Play/Egress Adapter)"；P-22 Output Profile 与 SURFACE_SPEC 的 WebRTC 输出协议标注为双腿 `VBMF→SRS WHIP publish / SRS→Player WHEP play`（VBMF 侧 publish 用 WHIP 正确，仅澄清播放腿为 WHEP）。
- **P0-H1-H7**：M-17 局部检查 `HEALTH INVARIANTS (H1-H7)` 改名为 `SESSION READINESS CHECKS`（Source/Node/Clock/Switch/Output Readiness + Freshness + API-UI Consistency），去掉 H1-H7 编号，避免与 Health Tree / Aggregation SoT 的 H1-H7 冲突。
- **P1-FI-01**：拆为 FI-01A（Primary 故障 + Backup READY → FAILOVER → Backup ACTIVE → HEALTHY）与 FI-01B（Primary 故障 + Backup NOT_READY → FILLER → DEGRADED/SAFE），真正验证 READY_TO_TAKE 而非简单出 Filler。
- **P1-FI-02**：锁定 `injection_point: Audio Mixer / PIPELINE`；新增 FI 注入点锁定说明，区分 Source embedded_audio（SOURCE）/ Loudness node（PIPELINE）/ Audio Master Join（MASTER）为不同 Failure Domain。
- **P1-HA**：HA-03 保留（全不可用 → FAILED），HA-04 去重改为 `ACTIVE=DEGRADED, STANDBY=OFFLINE+FAILED → DEGRADED`，避免与 HA-03 重复。
- **P1-UI**：09-health-tree `--blue` → `--accent`（页面 :root 未定义 `--blue`，ACTIVE role 背景实际不渲染）。
- **P1-DOC**：README 目录树 operator/=全部当前 HTML Prototype、product/=历史 B 轮保留目录（M-14 已并入 operator）；MILESTONES 标题 "5 个 Milestone" → "Phase 0.5 Milestone History"。
- **P1-Reference B**：新增多路 Output Variant 故障隔离测试（Program Master HEALTHY + HLS/RTMP HEALTHY + WHEP DEGRADED → Channel DEGRADED/HEALTHY 取决于 Required/Optional）+ Variant failure ≠ Program Master failure 断言。
- **P1-Profile**：新增 Encoding/Packaging Profile 责任边界写死（Encoding 仅 codec/resolution/bitrate/GOP/rate-control；Packaging 负责 container/segment/manifest/DRM；P-21 Encoding Profile 禁止承担 Packaging 职责）。
- 校验：`scripts/check_docs.py` **PASS**。结论：Phase 0.5 维持 LOCK FINAL；Phase 0.6 进入 Executable Acceptance 前的关键语义/一致性缺口已闭环。

### Phase 0.6 Consistency Gate（2026-08-25，基于 909d88e）
- **P0-WHEP-Scope**：Phase 0.6 `不在本阶段范围` 与 Reference B 已验收 WHEP 矛盾。改为：RIST / Zixi / NDI 完整功能开发不在 0.6；**WHEP 作为 Output Variant / Browser Playback 的 Acceptance 验证 = IN SCOPE**（实现深度：SRS Adapter 路径校验）。「WebRTC 全功能开发」≠「WHEP 输出路径 Acceptance」。
- **P1-EngCount**：NAVIGATION ENGINEERING 域 `26 表面` → `29 表面 · 数量由 SURFACE_REGISTRY.yaml 派生`（与 Registry SoT / NAVIGATION 域汇总表一致）；并新增 `check_docs.py` 规则 `check_nav_domain_counts`：NAVIGATION ENGINEERING 计数必须与 Registry 一致，禁止手写漂移。
- **P1-CD01-ID**：NAVIGATION 表面表 `CD-01` → `CD-01-WS` / `CD-01-Detail`（与 Registry 已拆分的双 surface 对齐），UI 显示名保持 Channel Control Workspace / Channel Detail，ID 不再压缩。
- **P1-POM-Status**：`PRODUCT_OBJECT_MODEL.md` 状态 `SEMANTIC LOCKED 0.1` → 文档级 `LOCK FINAL` + `Semantic Schema Version: 0.1 (PIA Schema)`，标题同步 `(V0.1 · LOCK FINAL)`，避免 "0.1 还可能重设计" 的误读。
- **P1-UI-E2E-01**：Phase 0.6 新增 `UI-E2E-01`（Profile Revision → Selective Apply → Runtime Verification），要求经**真实 UI 点击**走通 P-21 → P-28 → CD-01 → E-50 → D7 → Apply → M-17 → 06 Output，上下文全程保留，与 §E2E 系统级验证互为佐证。
- **P1-ObjectMatrix**：新增 `docs/phase-0.5/OBJECT_NAVIGATION_MATRIX.md`——核心对象跳转矩阵（查看→修改→运行态→影响→返回闭环）+ 硬规则（跳转携带对象上下文、禁止泛化首页、surface ID 与 Registry 一致），NAVIGATION 已加引用。属 Phase 4 实现约束文档（本门禁唯一新增文档，非 UI 页面）。
- 校验：`scripts/check_docs.py` **PASS**（含新增 NAVIGATION/Registry 计数一致性规则）。结论：Phase 0.5 维持 LOCK FINAL；本门禁为进入 Phase 0.6 Executable Acceptance 前的最后一组一致性收口，未做新增 UI 页面（仅 OBJECT_NAVIGATION_MATRIX.md 为授权例外）。

### Phase 0.6 Final Preflight Cleanup（2026-08-25，基于 b5c47fa）
- **P0-Contract**：`PRODUCT_OBJECT_MODEL.md` 删除 `SIGUSR1 = V0.2 协议` 的错误产品契约表述，改为正式 Runtime Apply Contract `Session Apply Runtime Revision`（JSON-RPC `session.apply_revision`, V0.2 §3.x Runtime Contract）；SIGUSR1 仅标为 Implementation Detail，禁止写入产品/运行时契约（防止进程信号偷偷变成架构事实）。
- **P1-Status**：统一 0.5C 状态文本 DRAFT/RECONCILED → LOCK FINAL——Root README Evolution 表 `🟡 DRAFT 0.1` → `🟢 LOCK FINAL`；MILESTONES `🟢 RECONCILED` → `🟢 LOCK FINAL`；NAVIGATION `🟡 RECONCILED` → `🟢 LOCK FINAL`（保留 `Historical: RECONCILED`），章节标题 `Phase 0.5C RECONCILED 验证清单` → `LOCK FINAL 验证清单`。
- **P1-M17-OutputLink**：M-17 `Backup Output (CDN Failover)` 区块改名 `Output Runtime Link`，表内容改为 `output_variant / status / primary / backup / retry_policy(defined in 06-output) / runtime_link [Open Output] → 06-output / D7`，明确 `Owner: Output Session`，消除 "Encoder 管 Output" 的视觉误导（Encoder ≠ Output 语义化）。
- **P1-RefB-Composition**：Reference B 新增 Program/Variant Composition 双层验收——Program Scope 跨 Variant 共享（全台 Logo）、Variant Scope 仅目标 Variant（平台水印/区域贴片）、Acceptance 禁止把 Composition 全提前到 Program Master。
- **P1-Network-UDP**：新增 Network Source Acceptance (UDP UNI/ASM/SSM)——三模式独立 schema 与必填字段（SSM 的 Source IP 不可丢）、IGMP 版本随模式切换 (ASM→IGMPv2 / SSM→IGMPv3)、E-40 动态渲染。
- **P1-E2E**：新增 E2E Acceptance: Profile→Bundle→ChangeSet→Runtime→Output 完整配置生命周期（Impact Preview 选择性 Apply / Preflight WARN≠PASS / Runtime Revision 单调+1 可回滚 / Apply 期间 Output HEALTHY）。
- 校验：`scripts/check_docs.py` **PASS**。结论：Phase 0.5 维持 LOCK FINAL；本清理为进入 Phase 0.6 Executable Acceptance 前的最后一组一致性/契约收口，不做任何新增页面或架构改动。

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
