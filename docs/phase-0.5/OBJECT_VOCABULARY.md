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

### 1.3 Profile 配置档 (7 种子类, kind 必填)

> 关键: **7 种 Profile 是 7 个 kind, 共享 P-20 Profile Center 入口, 但 schema 独立**。

| 子类 | kind | DB 表 | 锁定状态 |
|---|---|---|---|
| **Encoding Profile** | `ENCODING_PROFILE` | `encoding_profiles` | 🟢 LOCK (P-21 wireframe) |
| **Audio Profile** | `AUDIO_PROFILE` | `audio_profiles` | 🟡 P1 |
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
| **核心字段** | `bundle_id, name, encoding_profile_ref, audio_profile_ref, output_profile_ref, qc_profile_ref, rights_profile_ref, edge_policy_ref, graphic_profile_ref, notes` |
| **示例** | `CH01-News-Live` = H264-LIVE-1080P25-5M + NEWS-STEREO-R128 + HLS-LIVE-MAIN + NEWS-QC + NEWS-DOMESTIC + LIVE-DEFAULT + NEWS-LOWERTHIRD |
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
| **核心字段** | `variant_id, channel_id, profile_ref, destinations[] (Output Destination 引用), runtime_state, encoding_session_ref` |
| **命名约定** | `V-{channel}-{protocol}-{region}` 例: `V-CH01-HLS-Domestic` |
| **绝不允许混用** | ❌ **Output Variant ≠ Asset Version** (后者在 1.2) |
| **关系** | 1 Channel → N Output Variants (1:1 Profile 派生, 但 1 Profile → N Variants across channels) |

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
| **UI 入口** | M-14 File Transcode / M-17 Realtime Transcode / M-18 Job Detail |
| **唯一 ID** | `job_id` (UUID) |
| **6 子类 (kind 必填)** | `FILE_TRANSCODE / REALTIME_ENCODE / PROBE / QC / UPLOAD / ARCHIVE` |
| **核心字段** | `job_id, kind, status (PENDING/QUEUED/RUNNING/COMPLETED/FAILED/CANCELLED), progress_pct, worker_ref, input_ref, output_refs[], attempts[]` |
| **生命周期** | PENDING → QUEUED → RUNNING → (COMPLETED / FAILED / CANCELLED), 不可回退 (除 RESTART) |
| **绝不允许混用** | ❌ Job ≠ Session (见 1.12) |

### 1.12 Session 会话 (持续运行)

| 字段 | 锁定 |
|---|---|
| **正式名** | Session |
| **kind 值** | `SESSION` |
| **DB 表** | `media_sessions` |
| **UI 入口** | M-17 Realtime Transcode / CD-01 Channel Detail / 01 Dashboard |
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
| **核心字段** | `template_id, name, template_revision, channel_type (TV_LIVE/RADIO_LIVE/VIRTUAL_PLAYOUT), default_source_policy, default_bundle_ref (Profile Bundle · 7 Profile 引用), default_output_variants[] (含默认 delivery_criticality), default_qc_policy, default_clock_policy, used_by[]` |
| **Revision** | `template_revision` — 模板修订不可变 (V0.2 §1.13), 改模板 = 新 Revision |
| **关系** | `Template (工厂, 不进运行态) → instantiate → Profile Bundle Revision → Channel (DRAFT)` |
| **绝不允许混用** | ❌ 不是 "Profile" / "Preset" / "Bundle" — 模板是创建工厂对象, Bundle 是 Channel 实际配置集合 |
| **典型错误** | 把"模板默认输出 criticality 改动"当成运行态改动 → 模板默认只影响**新实例化**的 Channel, 不回灌已在播 Channel (D3 Bundle 快照不变) |

---

## 2. 4 大域对应核心对象 (Phase 0.5D Navigation 锁定)

| 域 (Top Nav) | 核心对象 |
|---|---|
| **BROADCAST** | Channel, Channel Template, Source, Route, Session (OUTPUT), Graph, Output Variant, Destination, Adapter |
| **MEDIA** | Asset, Asset Version, Job (FILE_TRANSCODE / REALTIME_ENCODE / PROBE / QC / UPLOAD / ARCHIVE), Session (MEDIA) |
| **ENGINEERING** | Profile (7 子类), Profile Bundle, Revision, Channel Template, Graph (design-time), Preflight Run, Change Set, Reservation, Hardware, Clock, Health Tree, Incident, Replay, Benchmark |
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
   │  - REALTIME_ENCODE       │  通常 1 个 Session 包装 1 个 Job
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

**VBMF Contributors** · VBMF Object Vocabulary V0.1 · Phase 0.5C Information Architecture Closure
