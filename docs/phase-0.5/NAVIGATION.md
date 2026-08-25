# VBMF Navigation Model (V0.1 锁定)

> **目的:** Phase 0.5C Information Architecture Closure 收口的"导航层"。
> 顶层导航用**业务域**, 不用**编号**; 数字降级为路由 / 页面 ID 内部使用。
>
> **本阶段:** 0.5C Information Architecture Closure (0.5E Cross-Domain Capabilities 已 Spec 锁)
>
> **状态:** 🟢 **LOCK FINAL** (0.5C + 0.5E)
>
> **权威源:** [`OBJECT_VOCABULARY.md`](OBJECT_VOCABULARY.md) · [`PRODUCT_OBJECT_MODEL.md`](PRODUCT_OBJECT_MODEL.md) · [`SURFACE_SPEC.md`](SURFACE_SPEC.md) · [`0.5E-CROSS_DOMAIN_CAPABILITIES.md`](0.5E-CROSS_DOMAIN_CAPABILITIES.md) (Impact Preview / Configuration Diff / Command Palette 跨域)

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

### 2.1 BROADCAST 域 (12 表面 · 0.5F 后)

| # | 表面 | 状态 | 角色 |
|---|---|---|---|
| **01-dashboard** | Dashboard 主控台 | 🟢 LOCK (0.5A) | Operator |
| **02-sources** | Sources 源管理 | 🟢 LOCK (0.5A, **0.5F 重画**: 二级 Taxonomy Local + External Network) | Operator / Engineer |
| **03-switcher** | Switcher 切播 | 🟢 LOCK (0.5A) | Operator |
| **04-composition** | Composition 图文包装 | 🟢 LOCK (0.5A) | Director |
| **05-audio** | Audio 音频 | 🟢 LOCK (0.5A) | Operator |
| **06-output** | Output 输出 | 🟢 LOCK (0.5A) | Operator |
| **07-recording** | Recording 录制 | 🟢 LOCK (0.5A) | Operator |
| **CH-01** | Channel List 通道列表 | 🟢 LOCK (0.5F, PIA V0.1 锁) | Operator / Director |
| **CD-01** | Channel Control Workspace 通道控制工作台 (Take Desk 7 块) | 🟢 LOCK (0.5F, PIA V0.1 锁) | Operator |
| **CD-01** | Channel Detail 通道详情 (8 Tab) | 🟢 LOCK (0.5F, 原 Spec 0.5B.0 升 wireframe) | All |
| **M-17** | Realtime Transcode 实时转码 | 🟢 LOCK (0.5D) | Operator / Engineer |

> **注**: 08-graph-designer / 09-health-tree 已划归 ENGINEERING 域 (PIA V0.1 §12); 10-states Validation 是全局 Validation 不属于 BROADCAST。

### 2.2 MEDIA 域 (7 表面 · 0.5D 后)

| # | 表面 | 状态 | 角色 |
|---|---|---|---|
| **M-11** | Media Library 媒体库 | 🟢 LOCK (0.5B.1) | Content Manager |
| **M-12** | Asset Detail 资产详情 (5 Tab) | 🟢 LOCK (0.5B.1) | Content Manager / Editor |
| **M-13** | Upload / Ingest 上传/收录 | 🟡 Spec (0.5B.0) | Content Manager |
| **M-14** | File Transcode 文件转码 (新名) | 🟢 LOCK (0.5D, 重画 6 步 Wizard) | Editor / Engineer |
| **M-15** | Transcode Jobs 转码任务 (M-14 母页) | 🟡 Spec (0.5B.0) | Editor / Engineer |
| **M-16** | Versions / Renders 资产版本渲染 | 🟡 Spec (0.5B.0) | Editor |
| **M-17** | Realtime Transcode 实时转码 | 🟢 LOCK (0.5D) | Operator / Engineer |
| **M-18** | Transcode Job Detail 转码任务详情 (M-15 子页升级为独立页) | 🟢 LOCK (0.5D) | Editor / Engineer |

### 2.3 ENGINEERING 域 (25 表面 · 0.5F 后)

| # | 表面 | 状态 | 角色 |
|---|---|---|---|
| **P-20** | Profile Center 配置中心 | 🟢 LOCK (0.5D) | Engineer |
| **P-21** | Encoding Profile 编码配置 | 🟢 LOCK (0.5B.1) | Engineer |
| **P-22** | Output Profile 输出配置 | 🟢 LOCK (0.5B.1) | Engineer |
| **P-23** | Audio Profile 音频配置 | 🟡 Spec (0.5B.0) | Engineer |
| **P-24** | Graphic Profile 图形配置 | 🟡 Spec (0.5B.0) | Engineer |
| **P-25** | QC Profile 质量配置 | 🟡 Spec (0.5B.0) | Engineer |
| **P-26** | Rights Profile 版权配置 | 🟡 Spec (0.5B.0) | Legal / Engineer |
| **P-27** | Edge Policy Profile 边缘策略 | 🟡 Spec (0.5B.0) | SRE |
| **P-28** | Profile Bundle 通道交付包 | 🟢 LOCK (0.5D) | Engineer / Director |
| **E-31** | Graph Designer 图设计 (0.5A #08 升级) | 🟢 LOCK (0.5A, 0.5B 升 Engineering) | Engineer |
| **E-32** | Preflight Center 预检中心 | 🟡 Spec (0.5B.0) | Engineer |
| **E-33** | Change Sets 变更集 | 🟡 Spec (0.5B.0) | Engineer |
| **E-34** | Capability Registry 能力注册 | 🟡 Spec (0.5B.0) | Engineer |
| **E-35** | Device Registry 设备注册 | 🟡 Spec (0.5B.0) | Engineer |
| **E-36** | Resource / Capacity 资源/容量 | 🟡 Spec (0.5B.0) | Engineer / SRE |
| **E-37** | Clock 时钟 | 🟢 LOCK (0.5D 升级: 4 级 Fallback Chain) | Engineer |
| **E-38** | Hardware Inventory 硬件清单 | 🟢 LOCK (0.5D) | Engineer |
| **E-40** | Network Source 网络源 (UDP Unicast/Multicast + 9 External 子类 + Security 8 字段) | 🟢 LOCK (0.5F) | Engineer |
| **E-41** | Network Path Inspector 网络路径检查器 (5 Hop Kind + 8 Failure Mode) | 🟡 Spec 锁 (0.5F); wireframe 0.5G 实施 | Engineer / SRE |
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

### 2.5 总计 (0.5F 后 · 52 wireframe + 1 Spec)

| 域 | 表面数 | 已 LOCK | Spec 锁定 (待 wireframe) | 0.5D 新增 | 0.5F 新增 |
|---|---|---|---|---|---|
| BROADCAST | 12 | 12 (0.5F 后全部 LOCK) | 0 | 1 (M-17) | +3 (CH-01, CD-01 WS, CD-01 Detail 升 wireframe) |
| MEDIA | 8 | 5 (M-11/12/14/17/18) | 3 (M-13/15/16) | 2 (M-14 重画, M-18 新) | 0 |
| ENGINEERING | 26 | 8 (P-20/21/22/28 + E-31/37/38/40) | 17 | 5 (P-20, P-28, E-37, E-38, 改 1) | +1 (E-40) |
| ADMIN | 5 | 0 | 5 | 0 | 0 |
| **域合计 (wireframe)** | **51** | **25** | **25** | **8** | **+4** |
| 全局 (10-states Validation) | 1 | 1 | 0 | 0 | 0 |
| **TOTAL wireframe** | **52** | **26** | **25** | **8** | **+4** |
| E-41 Network Path (Spec only) | 1 Spec | 0 | 1 (0.5F Spec 锁, 0.5G 实施) | - | +1 Spec |
| **Phase 0.5 总计** | **53** (52 wireframe + 1 Spec) | - | - | - | - |

> **历史口径演化**: 0.5B 报 "30+ UI 表面" → 实为 38 → 0.5C 重排 40 → 0.5D 后 44 → 0.5F 后 **52 个 wireframe + 1 Spec (E-41)** = 53。
> 02-sources.html 重画不计为新增。
> ⛔ 禁止在 README / 阶段总结中再使用 39 / 44 / 47 等早期数字。

---

## 3. 跨域导航 (Cross-Domain Navigation)

虽然顶层只有 4 域, 但很多业务流跨域:

| 业务流 | 起点 | 中转 | 终点 |
|---|---|---|---|
| 升级 Encoding Profile | ENGINEERING / P-21 | Change Set (E-33) → Impact Preview | BROADCAST / CD-01 (受影响的 Channel) |
| 创建转码任务 | MEDIA / M-11 选 Asset | MEDIA / M-14 (File) | MEDIA / M-18 (Job Detail) |
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
- [ ] M-17 新增 "Realtime Transcode" / 实时转码
- [ ] Profile Center (P-20) 顶部 7 Tab (Enc/Audio/Out/Graphic/QC/Rights/Edge)
- [ ] Profile Bundle (P-28) 进入 ENGINEERING 域
- [ ] Hardware (E-38) / Clock (E-37) 进入 ENGINEERING 域
- [ ] 所有跨域跳转保留 Context
- [ ] 权限矩阵在 SURFACE_SPEC §6 与本表一致

---

**VBMF Contributors** · VBMF Navigation Model V0.1 · Phase 0.5C Information Architecture Closure
