# VBMF Navigation Model (V0.1 锁定)

> **目的:** Phase 0.5C Information Architecture Closure 收口的"导航层"。
> 顶层导航用**业务域**, 不用**编号**; 数字降级为路由 / 页面 ID 内部使用。
>
> **本阶段:** 0.5C Information Architecture Closure
>
> **状态:** 🟡 **DRAFT 0.1** — 等待 0.5C LOCK FINAL
>
> **权威源:** [`OBJECT_VOCABULARY.md`](OBJECT_VOCABULARY.md) · [`PRODUCT_OBJECT_MODEL.md`](PRODUCT_OBJECT_MODEL.md) · [`SURFACE_SPEC.md`](SURFACE_SPEC.md)

---

## 0. 旧版 (Phase 0.5B) 的问题

Phase 0.5B 用了 6 个**编号工作域**:

```
01 Broadcast
02 Media
03 Profiles
04 Engineering
05 Operations
06 Administration
```

3 个根本问题:

1. **数字含义模糊** — Operator 看到 "04" 不知道是 Engineering 还是 Outputs
2. **域之间无业务边界** — Profile 既是 Configuration 又在多个域出现
3. **数字暗示层级** — 用户会以为 01 比 02 重要, 实际上 04 Engineering 才是 Phase 1+ 工程师主战场

---

## 1. 新版 4 大顶层域 (Phase 0.5C 锁定)

```
┌─────────────────────────────────────────────────────────────┐
│  VBMF Console                                                │
│                                                              │
│  ┌──────────┬──────────┬────────────┬────────────┐          │
│  │          │          │            │            │          │
│  │ BROADCAST│  MEDIA   │ ENGINEERING│    ADMIN   │  4 域   │
│  │   直播   │   媒体   │    工程    │   管理     │          │
│  │          │          │            │            │          │
│  └──────────┴──────────┴────────────┴────────────┘          │
│                                                              │
│  (Profiles / Operations 是横切, 不进 Top Nav)                │
└─────────────────────────────────────────────────────────────┘
```

| 域 | 中文 | 主要用户 | 包含对象 |
|---|---|---|---|
| **BROADCAST** | 直播 | Operator / Director | Channel, Source, Graph, Route, Session, Variant |
| **MEDIA** | 媒体 | Content Manager / Post Production | Asset, Asset Version, Job (FILE_TRANSCODE/PROBE/QC/UPLOAD/ARCHIVE) |
| **ENGINEERING** | 工程 | Engineer / SRE | Profile (6), Profile Bundle, Change Set, Preflight, Hardware, Clock, Capability, Health, Incident, Replay, Benchmark |
| **ADMIN** | 管理 | Admin | User, Role, Permission, Audit Log, System Setting |

> ⛔ **PROFILES 不再是顶层域** — 因为 Profile 是 6 个子类, 全部进 ENGINEERING 域的 P-20 Profile Center。
>
> ⛔ **OPERATIONS 不再是顶层域** — Health Tree / Incident / Replay / Benchmark 全部进 ENGINEERING 域, 与 Profile / ChangeSet 同一层, 因为这些都是"工程运维"工作。

---

## 2. 顶层 4 域到 UI 表面的映射 (Phase 0.5D 锁定)

### 2.1 BROADCAST 域 (12 表面)

| # | 表面 | 状态 | 角色 |
|---|---|---|---|
| **01-dashboard** | Dashboard 主控台 | 🟢 LOCK (0.5A) | Operator |
| **02-sources** | Sources 源管理 | 🟢 LOCK (0.5A) | Operator / Engineer |
| **03-switcher** | Switcher 切播 | 🟢 LOCK (0.5A) | Operator |
| **04-composition** | Composition 图文包装 | 🟢 LOCK (0.5A) | Director |
| **05-audio** | Audio 音频 | 🟢 LOCK (0.5A) | Operator |
| **06-output** | Output 输出 | 🟢 LOCK (0.5A) | Operator |
| **07-recording** | Recording 录制 | 🟢 LOCK (0.5A) | Operator |
| **08-graph-designer** | Graph Designer 图设计 | 🟢 LOCK (0.5A) | Engineer |
| **09-health-tree** | Health Tree 健康树 | 🟢 LOCK (0.5A) | All |
| **10-states** | 10 States 状态总览 | 🟢 LOCK (0.5A, Validation) | All |
| **CD-01** | Channel Detail 通道详情 (8 Tab) | 🟡 Spec 锁定 (0.5B.0) | All |
| **M-15** | Realtime Transcode 实时转码 | 🔴 0.5D 新增 | Operator / Engineer |

### 2.2 MEDIA 域 (7 表面)

| # | 表面 | 状态 | 角色 |
|---|---|---|---|
| **M-11** | Media Library 媒体库 | 🟢 LOCK (0.5B.1) | Content Manager |
| **M-12** | Asset Detail 资产详情 (5 Tab) | 🟢 LOCK (0.5B.1) | Content Manager / Editor |
| **M-13** | Upload / Ingest 上传/收录 | 🟡 Spec (0.5B.0) | Content Manager |
| **M-14** | File Transcode 文件转码 (新名) | 🟡 Spec 锁定, wireframe 待 0.5D 重画 | Editor / Engineer |
| **M-16** | Transcode Job Detail 转码任务详情 | 🔴 0.5D 新增 | Editor / Engineer |
| **M-17** | Versions / Renders 资产版本渲染 | 🟡 Spec (0.5B.0) | Editor |
| **M-18** | Playlists / Composition Templates | 🟡 Spec (0.5B.0) | Director / Editor |

### 2.3 ENGINEERING 域 (16 表面)

| # | 表面 | 状态 | 角色 |
|---|---|---|---|
| **P-20** | Profile Center 配置中心 | 🔴 0.5D 新增 (0.5C 锁 Spec) | Engineer |
| **P-21** | Encoding Profile 编码配置 | 🟢 LOCK (0.5B.1) | Engineer |
| **P-22** | Output Profile 输出配置 | 🟢 LOCK (0.5B.1) | Engineer |
| **P-23** | Audio Profile 音频配置 | 🟡 Spec (0.5B.0) | Engineer |
| **P-24** | Graphic Profile 图形配置 | 🟡 Spec (0.5B.0) | Engineer |
| **P-25** | QC Profile 质量配置 | 🟡 Spec (0.5B.0) | Engineer |
| **P-26** | Rights Profile 版权配置 | 🟡 Spec (0.5B.0) | Legal / Engineer |
| **P-27** | Edge Policy Profile 边缘策略 | 🟡 Spec (0.5B.0) | SRE |
| **P-28** | Profile Bundle 通道交付包 | 🔴 0.5D 新增 (0.5C 锁 Spec) | Engineer / Director |
| **E-32** | Preflight Center 预检中心 | 🟡 Spec (0.5B.0) | Engineer |
| **E-33** | Change Sets 变更集 | 🟡 Spec (0.5B.0) | Engineer |
| **E-34** | Hardware Inventory 硬件清单 | 🔴 0.5D 新增 (0.5C 锁 Spec) | Engineer |
| **E-35** | Device Registry 设备注册 | 🟡 Spec (0.5B.0) | Engineer |
| **E-36** | Clock 时钟 | 🔴 0.5D 新增 (0.5C 锁 Spec) | Engineer |
| **E-37** | Capability Registry 能力注册 | 🟡 Spec (0.5B.0) | Engineer |
| **O-41** | Health Tree 实时健康树 (Operator 视图) | 🟡 Spec (0.5B.0) | SRE |
| **O-42** | Incident Center 事件中心 | 🟡 Spec (0.5B.0) | SRE |
| **O-43** | Incident Timeline 事件时间线 | 🟡 Spec (0.5B.0) | SRE |
| **O-44** | Replay 录像回溯 | 🟡 Spec (0.5B.0) | SRE |
| **O-45** | Benchmarks 基准测试 | 🟡 Spec (0.5B.0) | SRE |

### 2.4 ADMIN 域 (5 表面)

| # | 表面 | 状态 | 角色 |
|---|---|---|---|
| **A-51** | Users 用户 | 🟡 Spec (0.5B.0) | Admin |
| **A-52** | Roles 角色 | 🟡 Spec (0.5B.0) | Admin |
| **A-53** | Permissions 权限 | 🟡 Spec (0.5B.0) | Admin |
| **A-54** | Audit Logs 审计日志 | 🟡 Spec (0.5B.0) | Admin |
| **A-55** | System Settings 系统设置 | 🟡 Spec (0.5B.0) | Admin |

### 2.5 总计

| 域 | 表面数 | 已 LOCK | Spec 锁定 (待 wireframe) | 0.5D 新增 |
|---|---|---|---|---|
| BROADCAST | 12 | 10 | 1 (CD-01) | 1 (M-15) |
| MEDIA | 7 | 2 (M-11, M-12) | 3 (M-13, M-17, M-18) | 2 (M-14 重画, M-16 新增) |
| ENGINEERING | 20 | 2 (P-21, P-22) | 13 | 5 (P-20, P-28, E-34, E-36 + 改 1) |
| ADMIN | 5 | 0 | 5 | 0 |
| **TOTAL** | **44** | **14** | **22** | **8** |

> 历史: 0.5B 报 "30+ UI 表面", 实为 38 (算 9 operator + 1 validation + 28 product)。现在 Phase 0.5C 重排后, **44 表面 (含 0.5D 新增 8)**, 全部进 4 域, 数字不再主导 UI。

---

## 3. 跨域导航 (Cross-Domain Navigation)

虽然顶层只有 4 域, 但很多业务流跨域:

| 业务流 | 起点 | 中转 | 终点 |
|---|---|---|---|
| 升级 Encoding Profile | ENGINEERING / P-21 | Change Set (E-33) → Impact Preview | BROADCAST / CD-01 (受影响的 Channel) |
| 创建转码任务 | MEDIA / M-11 选 Asset | MEDIA / M-14 (File) | MEDIA / M-16 (Job Detail) |
| 查看 Channel 实时状态 | BROADCAST / 01 Dashboard | BROADCAST / CD-01 | ENGINEERING / 09 Health Tree |
| 调查录像异常 | BROADCAST / 07 Recording | ENGINEERING / O-42 Incident | ENGINEERING / O-44 Replay |
| 临时 Override | BROADCAST / CD-01 | ENGINEERING / P-28 Bundle | Audit Log (ADMIN / A-54) |

**核心规则:** 任何跨域跳转必须保留 **Context** (Channel ID / Asset ID / Job ID / ChangeSet ID), 跳过去后该对象页面顶部显示"来自 / Go Back"。

---

## 4. 权限到域的映射 (Phase 0.5C 锁定)

| 角色 | BROADCAST | MEDIA | ENGINEERING | ADMIN |
|---|---|---|---|---|
| **Operator** | R/W (切播) | R | R | — |
| **Director** | R/W (Composition, Program 编排) | R/W (Playlist, Composition) | R (Profile 视图) | — |
| **Engineer** | R/W (Source, Graph, Profile 改) | R/W (Transcode, Job) | R/W (Profile, Bundle, ChangeSet, Hardware, Clock) | — |
| **SRE** | R | R | R/W (Health, Incident, Replay, Benchmark) | R (Audit) |
| **Content Manager** | — | R/W (Asset, Version, Rights) | R (Profile 视图) | — |
| **Editor** | — | R/W (Asset, Version, Transcode) | R (Profile 视图) | — |
| **Admin** | — | — | — | R/W |

> 详细 Role × Scope × Action × Object × State 矩阵 → SURFACE_SPEC §6 与 A-53 联合锁定 (Phase 0.5D 详化)。

---

## 5. URL 命名约定 (路由 ID)

虽然 UI 顶层不显示数字, 但 URL / 路由 / 内部 page_id **可以**用编号, 但**不再**用 `01..06` 这种"域编号"。

新约定 (Phase 0.5C 锁定):

```
/broadcast/dashboard         ← 域/对象
/broadcast/sources
/broadcast/cd-01             ← Channel Detail
/media/library
/media/asset/:asset_id
/media/transcode/file
/media/transcode/realtime
/engineering/profiles        ← Profile Center
/engineering/profiles/encoding/:profile_id
/engineering/profiles/output/:profile_id
/engineering/bundle/:bundle_id
/engineering/hardware
/engineering/clock
/admin/users
```

**禁止:** `/01-dashboard` `/M-11` `/P-21` 这种字母数字混编的 URL。内部 ID (M-11, P-21) 仍可在**文档/工单/通讯**中使用, 但**不是 URL 路径**。

---

## 6. 与 V0.2 Architecture 的对应 (V0.2 LOCK FINAL 不变)

V0.2 12 Engines 不变, 本 Navigation 4 域是**产品 UX 层**对应, 不动 V0.2:

| V0.2 Engine | 4 域 (主) | 备注 |
|---|---|---|
| Source | BROADCAST | 主 |
| Signal Fabric / Switcher / Composition / Audio | BROADCAST | 全在 BROADCAST |
| Output / Playout | BROADCAST (UI) + ENGINEERING (Profile) | 双域 |
| Recording | BROADCAST (录) + MEDIA (Asset) | 双域 |
| Replay | ENGINEERING | 偏 SRE |
| QC | MEDIA (Asset 关联) + ENGINEERING (Profile) | 双域 |
| Master Join | (不可外露, Route 内部) | ⛔ 不是 Engine |

---

## 7. Phase 0.5C LOCK FINAL 验证清单

- [ ] Top Nav 显示 4 域: **BROADCAST / MEDIA / ENGINEERING / ADMIN** (中文: 直播/媒体/工程/管理)
- [ ] 每个域显示主对象图标, 不用数字
- [ ] 数字仅出现在 URL path (如 `/broadcast/cd-01/:id`) 和内部 page_id
- [ ] M-14 改名为 "File Transcode" / 文件转码
- [ ] M-15 新增 "Realtime Transcode" / 实时转码
- [ ] Profile Center (P-20) 顶部 7 Tab (Enc/Audio/Out/Graphic/QC/Rights/Edge)
- [ ] Profile Bundle (P-28) 进入 ENGINEERING 域
- [ ] Hardware (E-34) / Clock (E-36) 进入 ENGINEERING 域
- [ ] 所有跨域跳转保留 Context
- [ ] 权限矩阵在 SURFACE_SPEC §6 与本表一致

---

**VBMF Contributors** · VBMF Navigation Model V0.1 · Phase 0.5C Information Architecture Closure
