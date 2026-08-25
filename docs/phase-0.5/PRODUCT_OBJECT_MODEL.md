# VBMF Product Object Model (V0.1 锁定)

> **目的:** Phase 0.5C Information Architecture Closure 的核心交付。
> 锁定 **产品对象之间的"组合关系"**, 不再让 UI 表面各管各的孤立存在。
>
> **本阶段:** 0.5C Information Architecture Closure
>
> **状态:** 🟡 **DRAFT 0.1** — 等待 0.5C LOCK FINAL
>
> **权威源:** [`OBJECT_VOCABULARY.md`](OBJECT_VOCABULARY.md) 锁定的 14 个对象
>
> **关联:** [`SURFACE_SPEC.md`](SURFACE_SPEC.md) · [`ARCHITECTURE_V0.2.md`](../architecture/ARCHITECTURE_V0.2.md)

---

## 0. 核心论点 (为什么需要这一层)

Phase 0.5B 的 30+ UI 表面已经覆盖了每个对象的详情页 (Asset / Profile / Variant / Job / Channel / Graph / Health / User), **但**这些对象之间的**组合关系**没有产品级入口:

> 用户在 M-12 (Asset Detail) 改了一个 QC 阈值, 不应该需要去 P-25 (QC Profile) 再改一次。
> 用户在 CD-01 (Channel Detail) 改了一个 Output Profile, 不应该需要去 P-22 (Output Profile) 再改一次。

正确的产品模型是:

> **Profile = Policy (跨 Channel 共享)**
> **Bundle = Channel 实际使用的 Profile 组合**
> **Variant = Channel 实际运行的输出实例**

UI 必须显式表达这 3 层的"组合关系", 而不是每个对象一个孤立页面。

---

## 1. 三个核心组合层 (Phase 0.5C 锁定)

### 1.1 第 1 层: 6 种 Profile (Policy / 跨 Channel 共享)

| 子类 | 对象 ID 命名 | 跨多少 Channel 共享? | 由谁管理 |
|---|---|---|---|
| **Encoding Profile** | `enc_profile_id` | 共享 (但每 Channel 派生 1 个 Revision) | Engineer |
| **Audio Profile** | `audio_profile_id` | 共享 | Engineer |
| **Output Profile** | `out_profile_id` | 共享 | Engineer |
| **Graphic Profile** | `graphic_profile_id` | 共享 | Designer / Engineer |
| **QC Profile** | `qc_profile_id` | 共享 | QC Lead |
| **Rights Profile** | `rights_profile_id` | 共享 | Legal / Rights Manager |
| **Edge Policy Profile** | `edge_policy_id` | 共享 | SRE / Engineer |

**UI 入口:** **P-20 Profile Center** (Phase 0.5D 新增) — 一个总览页, 顶部 7 个 Tab 切换 7 种 Profile Registry。

> **禁止:** 出现 "Channel Profile" / "Stream Profile" 这种含糊词, 用 Bundle 表达组合。

### 1.2 第 2 层: Profile Bundle (Composition / 1 个 Channel 用 1 个 Bundle)

**关键创新:** **1 个 Channel 1 个 Bundle**, Bundle 内含 6 种 Profile 的引用 (不是副本)。

```yaml
# DB schema
profile_bundles:
  bundle_id: UUID
  name: 'CH01-News-Live'
  channel_id: CH01  # 1:1 反向引用 (1 Channel 1 Bundle)
  encoding_profile_ref: H264-LIVE-1080P25-5M@v3
  audio_profile_ref:    NEWS-STEREO-R128@v1
  output_profile_ref:   HLS-LIVE-MAIN@v2
  qc_profile_ref:       NEWS-QC@v1
  rights_profile_ref:   NEWS-DOMESTIC@v4
  edge_policy_ref:      LIVE-DEFAULT@v2
  graphic_profile_ref:  NEWS-LOWERTHIRD@v1
  created_by: 'Director Zhang'
  created_at: 2026-08-25T14:00:00+08:00
  notes: '新闻直播标准配置 / News Live Standard'
```

**UI 入口:** **P-28 Profile Bundle** (Phase 0.5D 新增) — 选 6 个 Profile 引用, 不重新配置 6 套参数。

**优势:**
- Operator 改一个 Channel = 改一个 Bundle (6 个引用一次到位)
- Engineer 改一个 Profile (例如 HEVC → H.265) = 影响所有引用该 Profile 的 Bundle, 但有 Impact Preview 看到所有受影响 Channel
- 不重复配置 (6 个 Profile 不需要在每个 Channel 重新填)

**Revision 策略:**
- Bundle 自己的 `revision_id` 表达"哪 6 个 Profile 版本组合"
- 修改 Bundle = 创建新 Revision (V0.2 §1.13 锁定)

### 1.3 第 3 层: Output Variant (Instance / 1 个 Channel N 个 Variant)

**关键创新:** **1 个 Channel N 个 Output Variant**, 每个 Variant = 1 个 Profile 引用 + N 个 Destination 引用 + 1 个 Adapter 实例。

```yaml
# DB schema
output_variants:
  variant_id: V-CH01-HLS-Domestic
  channel_id: CH01
  profile_ref: HLS-LIVE-MAIN@v2  # 引用 P-22, 不是副本
  destinations:
    - dest_id: CDN-A (primary)
    - dest_id: CDN-B (备用)
  adapter: SRSAdapter
  effective_state: # runtime
    lifecycle: RUNNING
    readiness: READY_TO_TAKE
    health: HEALTHY
  runtime_metrics: # 来自 Session
    fps: 25.0
    bitrate_actual: 4.92 Mbps
    dropped_frames: 0
    pts_drift_ms: 0.4
```

**UI 入口:** **CD-01 Channel Detail / Tab 6 Output** (Phase 0.5B.0 锁定) — 列出所有 Variant, 每个 Variant 详细打开查看。

**关键约束:**
- **Variant = 派生实例**, Profile 改动必须显式 Choose New Revision, 不能在 Variant 上直接改 (改 = 创建新 Revision 推到 Profile)

---

## 2. Channel 是"组合中心" (Channel-centric Architecture)

V0.2 已经把 Channel 作为运营单位 (V0.2 §3.6)。Phase 0.5C 进一步把 Channel 作为**产品组合中心**:

```text
                       CHANNEL
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
     SOURCE            BUNDLE            VARIANT
   (1..N, 冗余)    (1, 6 个 Profile 引用)   (1..N, 输出)
        │                 │                 │
        ↓                 ↓                 ↓
    ┌──────────┐    ┌──────────┐    ┌──────────┐
    │ Health   │    │ Used By  │    │ Session  │
    │ Tree     │    │ + Impact │    │ + Run    │
    └──────────┘    └──────────┘    └──────────┘
                          │
                          ↓
                    ┌──────────┐
                    │ REVISION │
                    │ (不可变)  │
                    └──────────┘
                          │
                          ↓
                    ┌──────────┐
                    │ CHANGE   │
                    │ SET      │
                    │ (事务性)  │
                    └──────────┘
```

**核心规则:**

1. **改 Bundle = 创建 Revision → 进入 ChangeSet → 走 Logical Atomic Apply**
2. **改 Profile = 影响所有引用该 Profile 的 Bundle → 走 ChangeSet**
3. **改 Channel = 改 Bundle 引用或 Source 引用 → 走 ChangeSet**
4. **改 Variant = 不允许 (Variant 是派生) — 改回 Profile 改**

---

## 3. 4 类常见 Workflow (Phase 0.5C 锁定产品流)

### 3.1 Workflow: 创建一个新频道

```
1. Operator 选 "Create Channel"
2. 选 Bundle (新建 or 复用现有)
3. 选 Source (1..N, 冗余配置)
4. 选 Output Variant (1..N)
5. Preflight (E-32)
6. 提交 ChangeSet (DRAFT → VALIDATED → SCHEDULED → APPLYING → APPLIED)
7. Channel 进入 STARTING → READY_TO_TAKE
```

### 3.2 Workflow: 升级一个 Encoding Profile (影响多个 Channel)

```
1. Engineer 改 Encoding Profile (创建新 Revision v4)
2. 立刻看到 Impact Preview: 4 个 Channel 引用 v3 → 将升级到 v4
3. Engineer 选 Channel 子集 (e.g. 1 个 Channel 先升, 3 个稍后)
4. 创建 ChangeSet (含 4 个 ChangeSetItem: 4 个 Bundle 升级)
5. 选 Schedule (立即 / 凌晨 02:00 / 下次维护窗口)
6. 走 Logical Atomic Apply
```

### 3.3 Workflow: 改一个 Channel 的 Output Profile

```
1. Operator 进 CD-01 Channel Detail
2. 看到 Bundle = CH01-News-Live (用 v2)
3. 改 Bundle 引用 v2 → v3
4. 立刻看到 Impact Preview: 3 个 Variant 都将用 v3
5. 创建 ChangeSet (单 item)
6. 立即应用 (不调度, 因为该 Channel 在用 v2 但 Preflight 已过)
7. ChangeSet 走 APPLYING → APPLIED
8. Variant 重新派生, Session 收到 SIGUSR1 重载 (V0.2 协议)
```

### 3.4 Workflow: 临时 Override (紧急切到备用 Profile)

```
1. Operator 进 CD-01 / Tab Bundle
2. 看到 "临时 Override" 按钮 (L2)
3. 选临时 Profile (e.g. backup HLS-LL)
4. 填写 Who/Why/Until (强制 L2 审计)
5. 立即生效 (不进 ChangeSet, 但进 Audit Log)
6. Until 到期后, 自动回滚到 Bundle 默认 Profile
```

---

## 4. UI 表面到对象的映射 (Phase 0.5D 锁定)

| UI 表面 | 主要对象 | 次要对象 (Used By) |
|---|---|---|
| **01 Dashboard** (operator) | Channel + Session | Variant, Job, Health Tree |
| **02 Sources** (operator) | Source | Channel, Graph |
| **03 Switcher** (operator) | Route (含 Switcher) | Channel, Source |
| **04 Composition** (operator) | Route (含 Composition) | Channel, Asset, Graphic Profile |
| **05 Audio** (operator) | Audio Profile + Session | Channel |
| **06 Output** (operator) | Variant + Destination + Adapter | Channel, Session |
| **07 Recording** (operator) | Recording Session + Job | Channel, Asset |
| **08 Graph Designer** (operator) | Graph (设计态) | Channel, Route |
| **09 Health Tree** (operator) | Health Tree + Channel | Source, Node, RG |
| **10 States** (Validation) | — | (无业务对象, 状态参考) |
| **M-11 Media Library** (product) | Asset | Asset Version, Job, QC Profile, Rights Profile |
| **M-12 Asset Detail** (product) | Asset + Asset Version | QC Profile, Rights Profile, Channel (Used By) |
| **M-14 File Transcode** (product) | Job (FILE_TRANSCODE) | Asset, Encoding Profile, Variant |
| **M-17 Realtime Transcode** (product, 0.5D) | Session (MEDIA_SESSION, 包装 REALTIME_ENCODE Job) | Channel, Source, Encoding Profile |
| **M-18 Transcode Job Detail** (product, 0.5D) | Job (任意 kind) | Worker, Profile, Asset, Variant |
| **P-20 Profile Center** (0.5D) | Profile (6 子类 Registry) | Bundle, Variant, Channel |
| **P-21 Encoding Profile** | Encoding Profile | Bundle, Variant, Channel |
| **P-22 Output Profile** | Output Profile | Bundle, Variant, Destination, Edge Policy |
| **P-28 Profile Bundle** (0.5D) | Profile Bundle | Channel, 6 Profile, ChangeSet |
| **E-38 Hardware Inventory** (0.5D) | Hardware Capability + Device | Session, Job, Profile |
| **E-37 Clock** (0.5D 升级) | Clock Reference + Fallback Chain | Session, Source, Channel |
| **O-41 Health Tree** | Health Tree + Channel | Source, Node, RG, Incident |
| **O-42 Incident Center** | Incident | Channel, Replay, Job |
| **A-51 Users** | User | Role, Permission, Audit |
| **A-54 Audit Log** | Audit Event | User, Object, Action |

---

## 5. 与 V0.2 Architecture 的对应 (V0.2 LOCK FINAL 不变)

| V0.2 概念 | Product Object Model 对应 |
|---|---|
| `media_assets` | Asset (1.1) |
| `media_asset_versions` | Asset Version (1.2) |
| `encoding_profiles` / `output_profiles` / `audio_profiles` 等 6 表 | Profile (1.3, 6 kind) |
| `profile_bundles` (V0.4 规划) | Profile Bundle (1.4, 0.5C 新增) |
| `channels` | Channel (1.5) |
| `sources` | Source (1.6) |
| `graph_specs` / `routes` | Graph (设计) + Route (运行时) (1.7) |
| `output_variants` | Output Variant (1.8) |
| `output_destinations` | Output Destination (1.9) |
| `output_adapters` | Output Adapter (1.10) |
| `media_jobs` | Job (1.11, 5 kind) |
| `media_sessions` | Session (1.12, 2 kind) |
| `config_revisions` | Revision (1.13) |
| `change_sets` | Change Set (1.14) |

⛔ V0.2 LOCK FINAL 不开 V0.2.5, 本 Model 是 Product UX 层, 不改 DB schema 字段, 只在 `profile_bundles` 这张**V0.4 规划表**上加文档化声明。

---

## 6. Phase 0.5C LOCK FINAL 验证清单

- [ ] **导航 4 域** (BROADCAST / MEDIA / PROFILES / ENGINEERING) 顶层无数字
- [ ] **6 Profile** 全部进 P-20 Profile Center, 不再各自分散
- [ ] **M-14 / M-17** 显式标 "File Transcode" / "Realtime Transcode", 不再叫 "Transcode Center"
- [ ] **Variant vs Version** 命名严格分离
- [ ] **Bundle** 进 SURFACE_SPEC §3.3 (新章节)
- [ ] **Impact Preview** 在所有 Profile / Bundle / ChangeSet 页面有按钮
- [ ] **Configuration Triangle** 文档化 4 维 (Desired / Compiled / Effective / Impact)
- [ ] **Object Vocabulary** 14 个对象在所有 wireframe 中命名一致
- [ ] **DB schema 字段名** 与 Object Vocabulary 一致
- [ ] **Phase 0.6 README** 修正 `< 100ms` → `target_failover_time_ms + measured p50/p95/p99`

---

**VBMF Contributors** · VBMF Product Object Model V0.1 · Phase 0.5C Information Architecture Closure
