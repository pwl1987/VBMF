# VBMF Object Vocabulary (V0.1 锁定)

> **目的:** 消除 Phase 0.5B/0.5C 反复出现的"Variant / Version / Profile"歧义, 给每个核心对象一个**唯一正式定义**。
>
> **作用范围:** V0.2 Architecture + Phase 0.5 + Phase 0.6 + Phase 1 + Phase 4 全栈统一
>
> **本阶段:** 0.5C Information Architecture Closure
>
> **状态:** 🟢 **SEMANTIC LOCKED 0.2** (0.5D.5 + 0.5F F2) — 15 核心对象 · ChangeSet 三层状态 · 4 域映射 · 状态语言统一 (DRAFT/REVIEW/SEMANTIC_LOCKED/UI_LOCKED/IMPLEMENTATION_READY/DEPRECATED)
>
> **对象计数口径 (0.5F F2 焊死):** **15 canonical product objects** = **14 diffable objects** (Configuration Diff 适用, 0.5E Part 2) + **1 revision meta-object**。Revision 是不可变快照, 自身即 diff 基线, 不做对象级 Diff — Diff 目标是 **Revision 对**, 不是对象体。15 与 14 不冲突, 差异仅在此。

---

## 0. 三条不可破坏的原则

1. **每个对象只有一个正式名字。** 在所有 wireframe / Surface Spec / Architecture / code / DB schema 中, 同一个概念必须用同一个术语。
2. **对象有 `kind` 属性。** 任何引用 / 引用计数 / 关系图, 都必须显示 `kind`, 防止"Variant"混用。
3. **Runtime execution objects 不用通用 `status` 字段 (0.5D.5 修正)。** 运行态对象 (Session / Output / Source runtime) 状态走 `lifecycle` (STOPPED/STARTING/RUNNING/STOPPING) + `readiness` (NOT_READY/READY_TO_TAKE) + `health` (HEALTHY/DEGRADED/FAILED/UNKNOWN) 三轴分离 (V0.2 §1.5); **Business objects 可定义领域状态机**: Job → `JobState` (PENDING/QUEUED/RUNNING/COMPLETED/FAILED/CANCELLED) · ChangeSet → `ChangeSetStatus`/`ReviewState`/`TransactionPhase` 三层 · Source → `SourceLifecycle` (DRAFT→TESTING→VERIFIED→ASSIGNED→ACTIVE→STANDBY→OFFLINE) · Asset → `MediaAssetStatus` · Adapter → 3-Tier `AVAILABLE/RESERVED/UNAVAILABLE`。

---

## 1. 15 个核心对象 (Phase 0.5D.1 锁定)

### 1.1 Asset 媒体资产

| 字段 | 锁定 |
|---|---|
| **正式名** | Asset |
| **复数** | assets |
| **kind 值** | `ASSET` |
| **DB 表** | `media_assets` |
| **UI 入口** | M-11 Media Library / M-12 Asset Detail |
| **唯一 ID** | `asset_id` (UUID) |
| **核心字段** | `asset_id, title, original_filename, source_url, sha256, duration_ms, container, status, created_by, created_at, owner` |
| **状态** | `MediaAssetStatus` enum: `INGESTING / READY / PROCESSING / ARCHIVED / FAILED` |
| **绝不允许混用** | ❌ 不叫 "Media" / "File" / "Clip" / "Content" |
| **典型错误** | "Library" = Asset + Asset Version + Output Variant, 必须细分 |

### 1.2 Asset Version 资产版本 (V0.2 锁定)

| 字段 | 锁定 |
|---|---|
| **正式名** | Asset Version |
| **kind 值** | `ASSET_VERSION` |
| **DB 表** | `media_asset_versions` |
| **UI 入口** | M-12 Asset Detail / Tab ② Versions |
| **唯一 ID** | `version_id` (UUID) |
| **命名约定** | 工程师自由: `Master / Proxy / Mobile / Archive / Custom` |
| **核心字段** | `version_id, asset_id, kind (Master/Proxy/Mobile/Archive/Custom), profile_ref, file_path, sha256, duration_ms, size_bytes, qc_status, created_at` |
| **绝不允许混用** | ❌ 不要叫 "Variant" — 那是 Output 域的概念 |
| **典型错误** | "Create Variant" 按钮含义模糊 → 必须分 "Create Asset Version" (M-12) / "Create Output Variant" (CD-01 Tab 6) |

### 1.3 Profile 配置档 (8 种子类, kind 必填)

> 关键: **8 种 Profile 是 8 个 kind, 共享 P-20 Profile Center 入口, 但 schema 独立**。**Packaging Profile 与 Encoding Profile 严格分离**: Encoding 只负责 codec / resolution / framerate / bitrate / GOP / rate-control / 2-pass; Packaging 只负责 container / segment / HLS·DASH / manifest / DRM; **Encoding 禁止承担 Packaging 职责** (Phase 0.6 §3.1)。

| 子类 | kind | DB 表 | 锁定状态 |
|---|---|---|---|
| **Encoding Profile** | `ENCODING_PROFILE` | `encoding_profiles` | 🟢 LOCK (P-21 wireframe) |
| **Audio Profile** | `AUDIO_PROFILE` | `audio_profiles` | 🟡 P1 |
| **Packaging Profile** | `PACKAGING_PROFILE` | `packaging_profiles` | 🟡 P1 (Phase 4 wireframe) |
| **Output Profile** | `OUTPUT_PROFILE` | `output_profiles` | 🟢 LOCK (P-22 wireframe) |
| **Graphic Profile** | `GRAPHIC_PROFILE` | `graphic_profiles` | 🟡 P1 |
| **QC Profile** | `QC_PROFILE` | `qc_profiles` | 🟡 P1 |
| **Rights Profile** | `RIGHTS_PROFILE` | `rights_profiles` | 🟡 P1 |
| **Edge Policy Profile** | `EDGE_POLICY_PROFILE` | `edge_policy_profiles` | 🟡 P1 (P-27) |

| 字段 | 锁定 |
|---|---|
| **绝不允许混用** | ❌ 不要叫 "Preset" / "Template" / "Config" (Preset 是 Profile 的快速创建模板, 不等于 Profile) |
| **典型错误** | "Output Variant" 跟 "Output Profile" 混用 → 严格分: Profile 是 Policy, Variant 是 Per-Channel 派生实例 |

### 1.4 Profile Bundle 通道交付包 (Phase 0.5D 新增, 0.5C 文档)

| 字段 | 锁定 |
|---|---|
| **正式名** | Profile Bundle |
| **kind 值** | `PROFILE_BUNDLE` |
| **DB 表** | `profile_bundles` (V0.4+) |
| **UI 入口** | P-28 Profile Bundle (Phase 0.5D) |
| **唯一 ID** | `bundle_id` (UUID) |
| **核心字段** | `bundle_id, name, channel_id (1:1 Instance Bundle), encoding_profile_ref, audio_profile_ref, packaging_profile_ref (Bundle Default), default_output_profile_ref (Bundle Default, 实例化带入), qc_profile_ref, rights_profile_ref, edge_policy_ref, graphic_profile_ref, notes` |
| **示例** | `CH01-News-Live` = H264-LIVE-1080P25-5M + NEWS-STEREO-R128 + HLS-CMAF-PKG + HLS-LIVE-MAIN + NEWS-QC + NEWS-DOMESTIC + LIVE-DEFAULT + NEWS-LOWERTHIRD |
| **绝不允许混用** | ❌ 不是 "Channel Profile" (那是 channel-level config), 不是 "Preset" |

### 1.5 Channel 通道

| 字段 | 锁定 |
|---|---|
| **正式名** | Channel |
| **kind 值** | `CHANNEL` |
| **DB 表** | `channels` |
| **UI 入口** | Dashboard (CD-01 Channel Detail) |
| **唯一 ID** | `channel_id` (UUID) |
| **核心字段** | `channel_id, name, profile_bundle_ref, source_refs[], output_variant_refs[], redundancy_group_id, hot_standby_level` |
| **绝不允许混用** | ❌ 不要叫 "Stream" / "Program" — Program 是 Composition 输出层, Channel 是运营单位 |

> **Channel Template ≠ Bundle (0.5D.1 正式对象):** 见 §1.15 — `Channel Template`(创建工厂, **不进运行态**) → 实例化 `Profile Bundle`(当前 Channel 配置集合) → 引用 `Profile`(可复用策略) → 派生 `Output Variant`(当前交付实例). Template 仅用于"新建 Channel"时一键带出 Bundle.

### 1.6 Source 源

| 字段 | 锁定 |
|---|---|
| **正式名** | Source |
| **kind 值** | `SOURCE` |
| **DB 表** | `sources` |
| **UI 入口** | 02 Sources (operator) / M-11 (asset source) |
| **唯一 ID** | `source_id` (UUID) |
| **核心字段** | `source_id, kind (SDI/SRT/RTMP/HLS/WebRTC/RTP/UDP/RTSP/FILE/INTERNAL/COMPOSITE), name, signal_contract, redundancy_group_id, health` |
| **绝不允许混用** | ❌ 不要叫 "Input" / "Feed" — Input 是 Process 内部术语, Source 是 Operator 可见对象 |

> **Source 业务生命周期 (提案采纳, 区分于 Runtime 三轴):** `DRAFT → TESTING → VERIFIED → ASSIGNED → ACTIVE → STANDBY → OFFLINE`. 这是 Source 对象自身的业务生命周期, **不可** 与 `lifecycle` / `readiness` / `health` 三轴混用 (三轴是运行态, 生命周期是对象创建-上线-退役过程). 详见 `E-42-source-test-bench.md`.
> **Endpoint 是 Source 的子对象 (提案采纳):** Source 结构 = Adapter + **Endpoint** (mode / local_interface / local_bind / remote_address / remote_port / VLAN / DSCP / TTL / IGMP / SSM) + Contract + Runtime + QC. Endpoint **不**作为独立全局持久化实体, 除非未来需多 Source 共享同一 Network Endpoint. 对应 PIA §3 Network Layer.

### 1.7 Route 路由 (Graph + 编排)

| 字段 | 锁定 |
|---|---|
| **正式名** | Route (= Graph 编译后产物) |
| **kind 值** | `ROUTE` |
| **DB 表** | `routes` (V0.4+) + `graph_specs` (设计态) |
| **UI 入口** | 08 Graph Designer (operator) / E-32 Preflight |
| **绝不允许混用** | ❌ Graph ≠ Route, Graph 是设计时 spec, Route 是编译时产物, 运行时 Route 实例 = Session |

### 1.8 Variant (Output 域) 输出变体

| 字段 | 锁定 |
|---|---|
| **正式名** | Output Variant |
| **kind 值** | `OUTPUT_VARIANT` |
| **DB 表** | `output_variants` |
| **UI 入口** | CD-01 Channel Detail / Tab 6 Output |
| **唯一 ID** | `variant_id` (UUID) |
| **核心字段** | `variant_id, channel_id, output_profile_ref (OUTPUT_PROFILE 引用), packaging_profile_ref (PACKAGING_PROFILE 引用 · 见 1.16 覆盖链), destinations[] (Output Destination 引用), runtime_state, encoding_session_ref` |
| **命名约定** | `V-{channel}-{protocol}-{region}` 例: `V-CH01-HLS-Domestic` |
| **绝不允许混用** | ❌ **Output Variant ≠ Asset Version** (后者在 1.2) |
| **关系** | 1 Channel → N Output Variants (1:1 Profile 派生, 但 1 Profile → N Variants across channels) |
| **Packaging 归属 (0.5F.13 焊死)** | ⛔ **禁止** 把 Packaging 当成 "全 Channel 唯一实例"。`packaging_profile_ref` 是 **per-Variant** 引用: 未显式指定时继承 `Bundle.packaging_profile_ref` (Default), 显式指定时 Variant Override。最终 `EFFECTIVE_PACKAGING = Bundle Default ↓ Variant Override` (见 §1.16)。HLS / RTMP / UDP / File / WebRTC / 未来 2110 共存即靠此机制。 |
| **Output Profile 唯一 SoT (0.5F.13/0.5F.15 焊死)** | `output_profile_ref` 的**唯一权威来源 = Variant** (per-Variant 派生实例)。Bundle 仅提供 **Bundle/Instance Default** (`default_output_profile_ref`, 实例化时带入, 可被 Variant 覆盖); ⚠ 此 Default 属于 **Instance Bundle 层**, **不是** Channel Template 层。真正的 Template 级默认是 `ChannelTemplate.default_output_variants[]` (实例化后才落到 Instance Bundle 的 `default_output_profile_ref`)。禁止出现 "Bundle 的 Output Profile 与 Variant 的 Output Profile 双真相"。 |

### 1.9 Destination 输出目的地

| 字段 | 锁定 |
|---|---|
| **正式名** | Output Destination |
| **kind 值** | `DESTINATION` |
| **DB 表** | `output_destinations` |
| **UI 入口** | P-22 Output Profile / Detail / 06 Output (operator) |
| **核心字段** | `destination_id, host, port, path, protocol, auth_ref, status, health, edge_policy_ref` |
| **绝不允许混用** | ❌ 不是 "Endpoint" (那是协议术语) / "URL" (那是字符串) |

### 1.10 Adapter 适配器 (真正执行协议)

| 字段 | 锁定 |
|---|---|
| **正式名** | Output Adapter |
| **kind 值** | `ADAPTER` |
| **DB 表** | `output_adapters` |
| **UI 入口** | P-22 Output Profile / 3-Tier Protocol 状态 |
| **核心字段** | `adapter_id, kind (SRSAdapter/UDPAdapter/RTPAdapter/FileAdapter/...), version, status, health` |
| **3-Tier 状态** | `AVAILABLE / RESERVED / UNAVAILABLE` (V0.2 锁定) |
| **共享语义 (0.5D.5)** | Adapter 是**运行时执行资源, 可被多个 Destination 共享**; `adapter_ref` 归属 Destination (1 个 SRS 实例服务 N 个输出 Destination, 不误建模为 N 个 Adapter) |
| **绝不允许混用** | ❌ 不是 "Protocol" / "Encoder" / "Output" |

### 1.11 Job 任务 (一次性)

| 字段 | 锁定 |
|---|---|
| **正式名** | Job |
| **kind 值** | `JOB` |
| **DB 表** | `media_jobs` + `media_job_attempts` |
| **UI 入口** | M-14 File Transcode / M-18 Job Detail（⚠ M-17 是 Session 工作区, 见 1.12, 非 Job） |
| **唯一 ID** | `job_id` (UUID) |
| **5 子类 (kind 必填)** | `FILE_TRANSCODE / PROBE / QC / UPLOAD / ARCHIVE` |
| **核心字段** | `job_id, kind, status (PENDING/QUEUED/RUNNING/COMPLETED/FAILED/CANCELLED), progress_pct, worker_ref, input_ref, output_refs[], attempts[]` |
| **生命周期** | PENDING → QUEUED → RUNNING → (COMPLETED / FAILED / CANCELLED), 不可回退 (除 RESTART) |
| **绝不允许混用** | ❌ Job ≠ Session (见 1.12) · ⚠ `REALTIME_ENCODE` 已移出 Job: 实时编码是 Session, 由 `REALTIME_PROFILE` 实例化 `MEDIA_SESSION` (ENCODE_MODEL_SPEC §0) |

### 1.12 Session 会话 (持续运行)

| 字段 | 锁定 |
|---|---|
| **正式名** | Session |
| **kind 值** | `SESSION` |
| **DB 表** | `media_sessions` |
| **UI 入口** | M-17 Realtime Session 实时媒体会话 / CD-01 Channel Detail / 01 Dashboard |
| **唯一 ID** | `session_id` (UUID) |
| **2 子类 (kind 必填)** | `MEDIA_SESSION (实时编码) / OUTPUT_SESSION (实时输出)` |
| **核心字段** | `session_id, kind, lifecycle (STOPPED/STARTING/RUNNING/STOPPING), readiness (NOT_READY/READY_TO_TAKE), health (HEALTHY/DEGRADED/FAILED/UNKNOWN), parent_ref (Channel/Variant), runtime_metrics` |
| **三轴状态** | `lifecycle + readiness + health` (V0.2 §1.5 强制) |
| **绝不允许混用** | ❌ Session ≠ Job — Session 持续, Job 一次性 |

### 1.13 Revision 修订版 (不可变快照)

| 字段 | 锁定 |
|---|---|
| **正式名** | Revision |
| **kind 值** | `REVISION` |
| **DB 表** | `config_revisions` |
| **UI 入口** | P-21 Section 10 / P-22 Section / CD-01 / Change Set Detail |
| **核心字段** | `revision_id, target_kind, target_id, snapshot_json, parent_revision_id, change_set_id, created_by, created_at` |
| **不可变** | ⛔ 一旦创建, 不可修改, 只能通过新 Revision 覆盖 |
| **前缀约定 (0.5D.3)** | 人眼可区分对象类型, 禁止统一存 `version=3`: `T-v3`(Template) / `B-v2`(Bundle) / `ENC-v7`(Encoding) / `OUT-v4`(Output) / `RS-20260825-001`(Runtime Snapshot) |
| **绝不允许混用** | ❌ 不是 "Version" (那是 Asset 域) / "Snapshot" (那是运行时) |

### 1.14 Change Set 变更集 (事务性)

| 字段 | 锁定 |
|---|---|
| **正式名** | Change Set |
| **kind 值** | `CHANGE_SET` |
| **DB 表** | `change_sets` + `change_set_items` |
| **UI 入口** | E-33 Change Sets |
| **唯一 ID** | `change_set_id` (UUID) |
| **核心字段** | `change_set_id, title, status, review_state, phase, items[] (item_kind, target_id, before_revision, after_revision, impact_summary), scheduled_at` |
| **三层状态 (0.5D.1 焊死)** | `ChangeSetStatus` (业务状态: DRAFT/VALIDATED/APPROVED/SCHEDULED/APPLIED/ROLLED_BACK/ABORTED) · `ReviewState` (审批: NOT_REQUIRED/PENDING/APPROVED/REJECTED) · `TransactionPhase` (事务: PREPARING/APPLYING/COMMITTED/ABORTED) — 三者严格分离, `phase` 只描述事务执行, 不再混当业务状态 |
| **核心语义** | **Logical Atomic Apply** (V0.2 §4) — 整批生效或整批回滚 |
| **绝不允许混用** | ❌ 不是 "Deployment" (部署是运行时) / "Workflow" (流程是审批); ⛔ `status` 不能含 APPLYING (那是 TransactionPhase) |

### 1.15 Channel Template 频道模板 (0.5D.1 正式对象)

| 字段 | 锁定 |
|---|---|
| **正式名** | Channel Template |
| **kind 值** | `CHANNEL_TEMPLATE` |
| **DB 表** | `channel_templates` (V0.4+) |
| **UI 入口** | CH-02B Channel Template Center (Phase 0.5D) |
| **唯一 ID** | `template_id` (UUID) |
| **核心字段** | `template_id, name, template_revision, channel_type (TV_LIVE/RADIO_LIVE/VIRTUAL_PLAYOUT), default_source_policy, default_bundle_ref (Profile Bundle · 8 Profile 引用), default_output_variants[] (含默认 delivery_criticality), default_qc_policy, default_clock_policy, used_by[]` |
| **Revision** | `template_revision` — 模板修订不可变 (V0.2 §1.13), 改模板 = 新 Revision |
| **关系** | `Template (工厂, 不进运行态) → instantiate → Profile Bundle Revision → Channel (DRAFT)` |
| **绝不允许混用** | ❌ 不是 "Profile" / "Preset" / "Bundle" — 模板是创建工厂对象, Bundle 是 Channel 实际配置集合 |
| **典型错误** | 把"模板默认输出 criticality 改动"当成运行态改动 → 模板默认只影响**新实例化**的 Channel, 不回灌已在播 Channel (D3 Bundle 快照不变) |

---

## 2. 4 大域对应核心对象 (Phase 0.5D Navigation 锁定)

| 域 (Top Nav) | 核心对象 |
|---|---|
| **BROADCAST** | Channel, Channel Template, Source, Route, Session (OUTPUT), Graph, Output Variant, Destination, Adapter |
| **MEDIA** | Asset, Asset Version, Job (FILE_TRANSCODE / PROBE / QC / UPLOAD / ARCHIVE), Session (MEDIA) |
| **ENGINEERING** | Profile (8 子类), Profile Bundle, Revision, Channel Template, Graph (design-time), Preflight Run, Change Set, Reservation, Hardware, Clock, Health Tree, Incident, Replay, Benchmark |
| **ADMIN** | User, Role, Permission, Audit Log, System Setting |

---

## 3. 关系图 (Conceptual ER)

```
                         ┌──────────┐
                         │  CHANNEL │
                         └─────┬────┘
              ┌───────────────┼───────────────┐
              ↓               ↓               ↓
          ┌───────┐      ┌────────┐     ┌───────────┐
          │ SOURCE│      │ PROFILE│     │  OUTPUT   │
          │ (N×)  │      │ BUNDLE │     │  VARIANT  │
          └───────┘      │ (P-28) │     │  (N×)     │
                         └───┬────┘     └─────┬─────┘
              ┌──────────────┼──────────────┐  │
              ↓              ↓              ↓  ↓
        ┌──────────┐   ┌──────────┐   ┌────────────┐
        │ ENCODING │   │  AUDIO   │   │ DESTINATION│
        │ PROFILE  │   │ PROFILE  │   │   (N×)     │
        └──────────┘   └──────────┘   └─────┬──────┘
              │              │               ↓
              └──────┬───────┘         ┌──────────┐
                     ↓                 │ ADAPTER  │
                ┌─────────┐            │ (Runtime)│
                │ SESSION │←───────────└──────────┘
                │ (实时)  │
                └────┬────┘
                     │ produces
                     ↓
                ┌─────────┐
                │  ASSET  │
                │  (1)    │
                └────┬────┘
                     │ has_many
                     ↓
              ┌──────────────┐
              │ ASSET VERSION│
              │  (N×)        │
              └──────────────┘

   ┌──────────────────────────┐
   │  JOB (一次性)            │
   │  - FILE_TRANSCODE        │  1.13
   │  - ARCHIVE               │  实时编码非 Job → MEDIA_SESSION
   │  - PROBE / QC / UPLOAD   │
   └──────────────────────────┘

   ┌──────────────────────────┐
   │  REVISION (不可变)       │   1.13
   │  - Profile / Bundle /    │
   │    Variant / Channel     │
   │    任一对象的版本快照     │
   └──────────────────────────┘

   ┌──────────────────────────┐
   │  CHANGE_SET (事务)       │   1.14
   │  - 1 个 ChangeSet 包含   │
   │    N 个 ChangeSetItem    │
   │    (每个 item = 1 Revision 变更) │
   │  - Logical Atomic Apply  │
   └──────────────────────────┘
```

> ⛔ **Adapter ≠ P-22 (0.5D.3 修正):** ER 中 `ADAPTER (Runtime)` — Output Adapter 来自 **Runtime / Capability Registry (E-34) / Device Registry (E-35/E-38)**，不是 Output Profile (P-22)。四层边界: **Output Profile (P-22) → Output Variant → Destination → Output Adapter** (V0.2 §3.10 Adapter 3-Tier AVAILABLE/RESERVED/UNAVAILABLE)。**Adapter 可共享 (0.5D.5):** `adapter_ref` 归属 Destination — Variant 不持有 variant 级 adapter 字段; 1 个 Adapter 实例可被同 Variant 或多个 Variant 的多个 Destination 引用。

---

## 4. 易混淆的 6 组术语 (Phase 0.5C 必记)

| 组 | A | B | 怎么区分 |
|---|---|---|---|
| **V/V** | **Asset Version** (M-12) | **Output Variant** (CD-01) | 域: Asset vs Output; 字段完全不同; UI 必标 "Asset Version" / "Output Variant" |
| **P/B** | **Profile** (Policy) | **Profile Bundle** (Composition) | Profile 是单一类型配置; Bundle 是多 Profile 组合 |
| **P/C** | **Profile** (Channel 用) | **Channel Profile** (deprecated) | 1.5 之后废除 "Channel Profile" 概念, 统一 Profile Bundle 表达 |
| **J/S** | **Job** (一次性) | **Session** (持续) | Job 有 start/end, Session 有 lifecycle 三轴 |
| **R/V** | **Revision** (不可变快照) | **Version** (用户可改) | Revision 不可改, Version 可改 (但 V0.2 强制 Version 修改 = 新 Revision) |
| **G/R** | **Graph** (design-time spec) | **Route** (compiled runtime) | Graph = JSON 描述, Route = 实际运行实例 |

---

## 5. 与 V0.2 Architecture 的对应

V0.2 §1-§9 已经锁定了 12 Engines + 5 横向系统 + 6 横切能力。本 Vocabulary 与之对应:

| V0.2 Engine | 对应本 Vocabulary 核心对象 |
|---|---|
| Source | Source (1.6) |
| Signal Fabric | Route (1.7) + Session (1.12) |
| Normalize | Route 内部 (不直接外露) |
| Switcher | Route 内部 + Session (1.12) |
| Composition | Route 内部 + Session (1.12) |
| Audio | Route 内部 |
| Output | Output Variant (1.8) + Destination (1.9) + Adapter (1.10) + Session |
| Playout | Session (1.12) |
| Recording | Session + Job (FILE_TRANSCODE archive kind) |
| Replay | Job (probe kind) |
| QC | Job (QC kind) + Profile (QC_PROFILE) |
| Master Join | Route 内部 (不可外露为独立 Engine) |

⛔ **Master Join 不是 Engine** — README 误列已修复, 本 Vocabulary 是权威源。

---

## 6. 验证清单 (Phase 0.5C LOCK FINAL 前必过)

- [ ] 所有 wireframe 中的按钮 / 表头 / 状态名 与本 Vocabulary 完全一致
- [ ] SURFACE_SPEC §1 §2 §3 引用本 Vocabulary 作为权威源
- [ ] DB schema migration 0.5C 完成后, 所有表名与本 Vocabulary `DB 表` 字段一致
- [ ] Phase 1 Rust code enum 与本 Vocabulary kind 字符串一致
- [ ] Phase 4 Web Console i18n 翻译表 与本 Vocabulary 字段名一致 (翻译只翻译 label, key 不翻译)

---

## 1.16 Profile → Bundle → Variant → Runtime 继承链 (0.5F.13 焊死)

广播系统配置存在 5 层派生。每层在 UI 必须一眼可解释 (继承/覆盖/快照语义), 禁止 "运行态正确但来源不可追溯"。

### 继承层级 (自上而下覆盖)

```text
Global Profile (ENC/AUD/PKG/OUT/GFX/QC/RIGHTS/EDGE)
      ↓ 引用
Profile Bundle (Channel Template)
      ↓ 实例化
Channel Configuration
      ↓ per-Variant 覆盖
Output Variant (Override)
      ↓ 编译
Compiled Runtime
      ↓ 运行
Effective Runtime
```

### 每层必须标注的来源态 (5 态)

| 态 | 含义 | UI 呈现 |
|---|---|---|
| `Inherited` | 直接继承上层, 本层未改 | 灰色 / "继承自 Bundle" |
| `Overridden` | 本层显式覆盖上层 | 橙色 / "Variant 覆盖" |
| `Explicit` | 本层原始定义 (如 Global Profile 自身) | 默认 |
| `Compiled` | 编译层合并结果 | "编译结果" |
| `Effective` | 最终运行态 | 加粗 / 主显示 |

### Packaging 归属示例 (CH01 三 Variant — 0.5F.13 核心)

```text
Bundle.packaging_profile_ref = PKG-DEFAULT (HLS+CMAF 模板级默认)

V-CH01-HLS-Domestic
   packaging_profile_ref = <继承 PKG-DEFAULT>
   EFFECTIVE_PACKAGING   = HLS + CMAF + Manifest + Segment

V-CH01-RTMP-Overseas
   packaging_profile_ref = PKG-RTMP (Variant Override)
   EFFECTIVE_PACKAGING   = RTMP (无 HLS manifest)

V-CH01-Archive
   packaging_profile_ref = PKG-MP4 (Variant Override)
   EFFECTIVE_PACKAGING   = MP4 单文件
```

### Bitrate 可解释性示例

```text
Bitrate
   Profile:   5 Mbps   (Explicit)
   Bundle:    inherited
   Variant:   8 Mbps   Override
   Compiled:  8 Mbps
   Effective: 8 Mbps
```

> **守卫 (0.5F.13)**: 任何 Surface 显示 Profile 派生值, 必须同时可展开 "Inherited / Overridden / Explicit / Compiled / Effective" 来源链。禁止只显示最终值而无来源。

### 全局组件: Configuration Source Panel (0.5F.14 P1-6 焊死)

上述 5 态来源链必须收敛为**单一可复用 UI 组件**, 而不是各 Surface 各自实现。组件契约:

```text
Configuration Source Panel (点击任意派生值展开)
┌─────────────────────────────────────────┐
│ Bitrate = 8 Mbps            (Effective) │
├─────────────────────────────────────────┤
│ Profile   ENC-LIVE-v5      5 Mbps        │
│ Bundle    inherited                      │
│ Variant   8 Mbps  Override               │
│ Compiled  8 Mbps                         │
│ Runtime   8 Mbps                         │
└─────────────────────────────────────────┘
```

**强制出现位置 (SoT = 本 Vocabulary §1.16)**:
`P-21` (Encoding Profile) · `P-22` (Output Profile) · `P-28` (Profile Bundle) · `CD-01` (Channel Workspace) · `M-14` (File Transcode) · `M-17` (Realtime Session)

**约束**: 同一组件、同一 5 态渲染、同一展开交互; Phase 4 实现时禁止在 6 个 Surface 各写一套来源逻辑。

### 1.17 Source Workspace 统一入口 (0.5F.14 P2-9 焊死)

Source 已定义 **11 类 kind** + Endpoint 子对象 (0.5F.15 P0-2 校正: 原文误写"12 类", 实际枚举为 11, 与 V0.2 的 11 Source Adapter 一致)。UX 必须做成**连续 Wizard**, 而非分散页面 (E-40 Network Source / E-42 Source Test Bench 已存在, 仅 UI 收口):

```text
Create Source
  Source Type
    Physical : SDI
    Network  : SRT / RTMP / RTP / UDP / RTSP / HLS / WebRTC
    File     : FILE
    Internal : BLACK / BARS / FILLER
  ↓ (选 UDP)
    Unicast / Multicast ASM / Multicast SSM
    + Interface / VLAN / Local Bind / Group / Source IP / Port / IGMP / DSCP / TTL
  ↓
  TEST → LOCK → VERIFY → ASSIGN → ACTIVE / STANDBY
```

> ⛔ **0.5F.15 P0-2 · SourceKind = 11 (非 12)**: `SDI / SRT / RTMP / HLS / WebRTC / RTP / UDP / RTSP / FILE / INTERNAL / COMPOSITE`。未来协议 (V0.3: RIST / Zixi / NDI) 通过独立 **Source Adapter Capability Registry** 注册能力, **不**修改 `SourceKind` 枚举——`SourceKind` 数量与 `Source Adapter` 数量概念上不必相等。
> 不产生新 Surface; 复用 02 Sources + E-40 + E-42。状态流转即 Source 业务生命周期 (§1.6)。

### 1.18 两条对象链: FILE_TRANSCODE vs REALTIME SESSION (0.5F.14 P1-3 焊死)

两者**共享 Packaging Profile Registry**, 但**不共享 Variant 对象**:

```text
FILE_TRANSCODE (M-14)
  Asset → Asset Version → FILE_PROFILE → Packaging Profile → Job Policy → Job → New Asset Version
  产物 = 资产版本 (资产域)

REALTIME SESSION (M-17)
  Channel → Output Variant → Output Profile → Packaging Profile → Realtime Session → SRS
  产物 = 直播输出变体 (输出交付域)

共享: Packaging Profile Registry
禁止: Asset Version 与 Output Variant 混用同一上下文
```

### 1.19 Channel Workspace: 上下文驾驶舱 (0.5F.14 P2-10 焊死)

CD-01 作为统一操作上下文 (驾驶舱), 深度配置仍为独立页面 (Drawer/Inspector):

```text
CD-01-WS (Channel Workspace · 驾驶舱)
  Source · Switch · Health · PVW · PGM · NEXT
  Audio (LUFS/AV Sync/Drift) · Output (HLS/RTMP/UDP) 同上下文协作
  点击 Audio → P-23 / Switch → 03 / Output → 06 (深页)

CD-01-Detail (Channel Detail / Inspector · 深页)
  Variant / Destination / Source Endpoint / Audio Profile / Output Profile 深度配置
```

> ⛔ **0.5F.15 P1-2 · CD-01 命名约定 (全库统一)**: 文档中任何 "CD-01" 简写**必须**解析为 `CD-01-WS` 或 `CD-01-Detail` 之一, 不得含糊:
> - `CD-01-WS` = Channel Workspace (实时操作驾驶舱: Source/Switch/PVW/PGM/NEXT/Audio/Output/Health 同上下文)
> - `CD-01-Detail` = Channel Detail / Inspector (深度配置: Variant/Destination/Source Endpoint/Audio Profile/Output Profile)
> - 全库已落地的两份 wireframe 即 `CD-01-channel-workspace.html` (WS) 与 `CD-01-channel-detail.html` (Detail), 不能压成单页。
> 不新增 Surface; 驾驶舱 + 深页结构。

### 1.20 Asset Version Role 与 Encoding Profile Preset 分离 (0.5F.15 P1-3 焊死)

M-14 的 `Master / Proxy / Mobile / Archive / Custom` 是 **Asset Version Role**, **不是** Encoding Profile Preset。两者必须分层:

```yaml
AssetVersionRole:   # 资产版本的角色/交付规格类别
  MASTER
  PROXY
  MOBILE
  ARCHIVE
  CUSTOM

# 派生链 (0.5F.15):
Asset Version Role
   ↓ 决定目标规格类别
Encoding Profile (FILE_PROFILE)   # 具体编码参数 (P-21)
   ↓
Packaging Profile                  # 封装 (P-20)
   ↓
Job Policy                         # 运行时策略
```

> ⛔ 禁止把 `AssetVersionRole` 当 `Encoding Profile` 用 (如 "Proxy = ENC-v22" 是 Role→Profile 的绑定, 不是 Role=Profile)。M-14 Step2 选的是 Role, Step3 才选 FILE_PROFILE。

### 1.21 Storage Destination 对象化 (0.5F.15 P1-4 焊死)

File Transcode 的"保存位置"必须从路径字符串升级为对象:

```yaml
StorageDestination:
  id
  name: Local NVMe / NAS-01 / RustFS / NFS-Archive / S3-Compatible
  path_template: /mnt/storage/news/{date}/{version}/
  retention
  access_policy
  capacity
  write_speed
  availability
```

> M-14 的 `Path Template` / `Storage` 字段应绑定到 `StorageDestination` 引用, 而非裸字符串。未来 Archive/Backup/Publish/Distribution 复用同一对象。

### 1.22 Network Source = 配置 + 实时信号监控工作台 (0.5F.15 P1-5 焊死)

Source Workspace (§1.17) 的 Network Source 选 `UDP / Multicast / 239.10.10.20:1234` 后, **必须**立即呈现实时信号监控 (与 Source Monitor 强关联, 非独立页面):

```text
LINK     NIC eno2 UP · VLAN 120 OK · IGMP JOINED OK
SIGNAL   Packets 3.2M · Bitrate 12.4Mbps · Jitter 0.8ms · Loss 0
FORMAT   1080i25 · UYVY422 · 48kHz/8ch
QC       Video HEALTHY · Audio HEALTHY · PTS LOCKED · Clock PTP
```

> 配置 Wizard 与 Signal Monitor 在 Source Workspace 同上下文协作 (类比 CD-01 Audio/Switch/Output 同上下文)。

### 1.23 TAKE 与 AUTO FAILOVER 视觉层区分 (0.5F.15 P1-6 焊死)

CD-01-WS 上 `TAKE` (Operator Intent) 与 `FAILOVER` (Failure Domain 自动/辅助) 必须视觉区分, 不能同等级按钮:

```text
PRIMARY   ● ACTIVE
BACKUP    ● READY_TO_TAKE
[ TAKE BACKUP ]         ← Operator Intent (人工)
[ AUTO FAILOVER ARM ]   ← Automation policy (自动)

⚠ SOURCE FAILURE 时:
Automatic Failover · FRAME_SWITCH
Reason: Primary SDI signal lost 2.1s
[ Take Now ]  [ Inspect ]
```

> ⛔ `TAKE ≠ FAILOVER ≠ ChangeSet` (EXECUTION_MODEL §0/§4). Operator 意图与系统故障切换在 UI 上必须可分辨。

---

**VBMF Contributors** · VBMF Object Vocabulary V0.1 · Phase 0.5C Information Architecture Closure + 0.5F.13/0.5F.14/0.5F.15 Object Boundary & Workflow Closure
