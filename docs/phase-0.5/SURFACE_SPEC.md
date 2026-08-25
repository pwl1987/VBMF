# VBMF UI/UX Surface Specification V0.2

> **文档定位:** V0.2 架构对象 → VBMF Console UI 表面 的完整映射
>
> **适用版本:** VBMF V0.2 LOCK FINAL + Phase 0.5A LOCK FINAL + Phase 0.5B-Closure-1 + 0.5B.1 + 0.5B.2 LOCK FINAL + 0.5C DRAFT 0.1
>
> **Phase 0.5C 锁定:** UI 顶层导航从 6 编号域 (01..06) 改为 4 业务域 (BROADCAST / MEDIA / ENGINEERING / ADMIN). 详见 [`NAVIGATION.md`](NAVIGATION.md) §1 与本节 §29.2.
>
> **Baseline Metadata (强制对齐 — 与 GitHub `master` 一致):**
>
> ```yaml
> architecture_version: V0.2.4          # 当前 LOCK FINAL
> runtime_semantics: CLOSED            # implementation_ambiguity: NONE
> review_passes: 22                    # 22 轮 review
> latest_errata: 14                    # Errata-1 ~ Errata-14
> runtime_domains_closed: 9            # 含 Clock (9 大 Runtime 域)
> health_invariants: 7                 # H1-H7 (Errata-14 锁定)
> canonical_vocabulary: LOCKED         # 见 §2.6
> never_reopen: V0.2.5                 # 不再开 V0.2.5
> phase_0_5_status: 0.5A/0.5B/0.5B.1/0.5B.2 LOCK FINAL + 0.5C DRAFT 0.1
> phase_0_5c: Information_Architecture_Closure  # 目录归并 + Object Vocabulary + Navigation 4 域
> top_navigation: 4 域 (BROADCAST / MEDIA / ENGINEERING / ADMIN)  # 0.5C 锁定
> baseline_sot: docs/architecture/ARCHITECTURE_V0.2.md
> ```
>
> **关联文档 (Phase 0.5C 归并后):**
> - [`docs/architecture/ARCHITECTURE_V0.2.md`](../architecture/ARCHITECTURE_V0.2.md) — V0.2 架构基线 (192KB / 4020 行 / 22 轮 review)
> - [`docs/phase-0.5/README.md`](../README.md) — Phase 0.5 顶层入口 (4 域导航)
> - [`docs/phase-0.5/OBJECT_VOCABULARY.md`](OBJECT_VOCABULARY.md) — 0.5C 14 对象权威定义
> - [`docs/phase-0.5/PRODUCT_OBJECT_MODEL.md`](PRODUCT_OBJECT_MODEL.md) — 0.5C 3 层组合关系
> - [`docs/phase-0.5/NAVIGATION.md`](NAVIGATION.md) — 0.5C 4 域顶层导航
> - [`docs/phase-0.5/MILESTONES.md`](MILESTONES.md) — 0.5C 历史 milestone 归档
> - [`docs/phase-0.5/ERRATA.md`](ERRATA.md) — Phase 0.5A 20 项修复归档
> - [`docs/phase-0.6/README.md`](../phase-0.6/README.md) — Executable Acceptance Specification (前置: Phase 0.5 LOCK FINAL)
> - [`docs/phase-0.5/DESIGN_SYSTEM.md`](DESIGN_SYSTEM.md) — V0.1 Design System
> - [`docs/phase-0.5/I18N_SPEC.md`](I18N_SPEC.md) — V0.1 i18n Contract

---

## 0. 文档目的

V0.2 架构已定义 ~30 个核心对象（media_assets / encoding_profiles / change_sets / incidents / users / ...），但 UI 表面目前只覆盖 9 + 1 个（Operator 播控 + State Reference）。

本规范的目标是**一次性把所有架构对象映射到 UI 表面**，定义每页的：

1. **目标 / Goal** — 这页解决什么问题
2. **信息架构 / Information Architecture** — 主要字段 / 列表 / 详情
3. **主要操作 / Primary Action** — Operator / Engineer 点进来第一件事
4. **状态模型 / State Model** — 6 状态 (Normal / Loading / Empty / Error / Warning / Critical)
5. **危险操作 / Dangerous Action** — L1 / L2 / L3 分级
6. **权限模型 / Permission Model** — R / W / A (Read / Write / Admin)
7. **关联工作流 / Related Workflow** — 哪个 Chain 涉及
8. **跳转关系 / Navigation** — 从哪些页面跳转过来
9. **架构对象映射 / Architecture Object Mapping** — 哪几张表 / 哪几个 service

---

## 1. 工作域 (Work Domains) — 计数口径统一

> **Phase 0.5C 起顶层导航为 4 业务域** (BROADCAST / MEDIA / ENGINEERING / ADMIN, 见 §29.2 与 [`NAVIGATION.md`](NAVIGATION.md))。
> 下表 6 编号域保留为**表面编号体系与历史口径**, 不再是 UI 顶层导航。

| # | 工作域 | 角色 | UI 表面数 | 来源 | 状态 (0.5C.1 回写) |
|---|---|---|---|---|---|
| 01 | **Broadcast 播控** | Operator / Director | 9 Core | 0.5A LOCK | 🟢 LOCK FINAL (operator/) |
| 02 | **Media 媒体资产** | Director / Engineer | 6 新 (M-11~16) | 0.5B 新增 | 🟢 Spec 锁定 · M-11/M-12/M-14 有 wireframe (product/) |
| 03 | **Profiles 配置** | Engineer | 7 新 (P-21~27) | 0.5B 新增 | 🟢 Spec 锁定 · P-21/P-22 有 wireframe (product/) |
| 04 | **Engineering 工程** | Engineer | 2 升级 (E-31) + 6 新 (E-32~37) | 0.5A #08 + 0.5B 新增 | 🟢 LOCK + Spec 锁定 |
| 05 | **Operations 运维** | Operator / Engineer | 1 升级 (O-41) + 4 新 (O-42~45) | 0.5A #09 + 0.5B 新增 | 🟢 LOCK + Spec 锁定 |
| 06 | **Administration 平台管理** | Admin | 5 新 (A-51~55) | 0.5B 新增 | 🟢 Spec 锁定 |
| +CD | **Channel Detail** (01 域子页) | Operator / Director | 1 (CD-01) | 0.5B Closure-1 新增 (§17) | 🟢 Spec 锁定 · wireframe 0.5D+ |
| +1 | **State Reference 状态参考** | 全员 | 1 (10-states) | 0.5A LOCK (Validation) | 🟢 LOCK FINAL |

**口径说明 (避免歧义):**
- **0.5A 锁定的 UI 表面**: 9 Core (01-09) + 1 Validation (10-states) = **10**
- **0.5B 新增 UI 表面**: M(6) + P(7) + E(6) + O(4) + A(5) = **28**
- **0.5B Closure-1 新增**: CD-01 Channel Detail (§17) = **1** (单独计数, CD 前缀不占域内序号)
- **从 0.5A 升级到 0.5B 工作域的 UI 表面**: E-31 (Graph Designer 升级到 Engineering) + O-41 (Health Tree 升级到 Operations) = **2 升级** (升级 = 重新归类, 不是新增)
- **O-44 说明**: O-44 Replay 是 0.5B 新定义的独立表面（继承 0.5A #07 的 Replay 子区语义, wireframe 已随 0.5A LOCK, 无需重画）, 计入 O(4) 新增; 与 E-31 / O-41 的"整页升级"不同
- **Phase 0.5 已锁定总计**: 10 (0.5A) + 28 (0.5B 新) + 1 (CD-01) = **39**
- **0.5D 后总计**: 39 + 5 新 (M-17/M-18/P-20/P-28/E-38) = **44** (E-37 升级与 M-14 重画不加数; 见 §29.2 计数表)
- **不要再写 "30 / ~25 / ~35" 等模糊数字**

**Surface 编号约定 (锁定):**
- 0.5A 沿用 `01-09` 编号 (不变)
- 0.5B 新增使用 `M-11`/`P-21`/`E-31`/`O-41`/`A-51` 域前缀 + 序号
- 序号在每个域内连续, 跨域不连续 (避免重排 0.5A)

---

## 2. 全局规范

### 2.1 6 状态样例 (适用于每页)

> **口径分层 (0.5C.1 回写)**: **Spec 级** — 每个表面必须有 6 状态定义（正文各表面"状态模型"小节 + §2.1.1 补全表共同构成 SoT）；
> **Wireframe 级** — 0.5D / Phase 4 出图时必须逐页呈现 6 个状态的**视觉样例**（此前宣称"缺一视为不完整"对 Spec-only 表面不可验证, 现以此为修正）。

| 状态 | 触发条件 | UI 表现 |
|---|---|---|
| **Normal 正常** | 业务无异常 | 全量数据 + 正常色码 (绿) |
| **Loading 加载中** | 首次 / 刷新 | Skeleton / Spinner + 灰底 |
| **Empty 空态** | 无数据 (新 Channel / 首次启动) | 引导 + "新建 / 导入" 主按钮 |
| **Warning 警告** | 软指标越界 (漂移 / 漂移率 / 磁盘 80%) | 黄色 + Alert Banner |
| **Error 错误** | 单次操作失败 (Encode 失败 / Profile 校验错) | 红色 + 错误信息 + 重试按钮 |
| **Critical 严重** | 业务中断 (Source 全 FAILED / Change Set 失败) | 红色脉冲 + Incident 入口 |

### 2.1.1 Spec 级状态模型补全表 (0.5C.1 — 覆盖 0.5B 正文缺状态模型的 13 个表面 + 缺 Loading/Empty 的 4 个表面)

| 表面 | Normal | Loading | Empty | Warning | Error | Critical |
|---|---|---|---|---|---|---|
| **M-15** Transcode Jobs | Job 列表实时刷新 | 表格 skeleton | 无历史 Job + "发起第一个转码" 引导 | 排队超阈值 / Worker 接近满载 | 单 Job FAILED + Retry 入口 | 全部 Worker 不可达 |
| **M-16** Versions / Renders | 版本列表 + 当前默认高亮 | skeleton | 仅原始上传 1 版 + "创建 Proxy" 引导 | 默认版本 QC WARN 角标 | 渲染失败行 + Retry | 默认版本缺失 (被删/损坏) 红条 |
| **P-23** Audio Profiles | Profile 列表 | skeleton | 0 Profile + 模板引导 | 引用中的 Profile 长期未验证 | 响度参数越界校验失败 | 删除被引用 Profile 阻断 |
| **P-24** Graphic Profiles | 模板列表 + 画布预览 | skeleton | 0 模板 + 内置模板引导 | 模板资源缺失 (字体/图片) | 模板解析失败 | On-Air 模板修改被 ChangeSet 阻断 |
| **P-25** QC Profiles | Profile 列表 + 阈值表 | skeleton | 0 Profile + "Broadcast Default" 模板引导 | 阈值偏离 EBU R128 建议值提醒 | 阈值组合非法 | 删除被引用 QC Profile 阻断 |
| **P-26** Rights Profiles | 模板列表 | skeleton | 0 模板 + 引导 | 默认模板即将到期 | 地域/平台组合冲突 | 误删默认模板阻断 |
| **P-27** Edge Policy | Policy 列表 | skeleton | 0 Policy + LIVE_EDGE_DEFAULT 模板引导 | Policy 与 Switch Mode 不匹配提示 | 参数校验失败 | 删除被引用 Edge Policy 阻断 |
| **E-34** Capability Registry | 矩阵只读展示 | skeleton | Discovery 未运行 + "重新发现" 引导 | Registry 缓存 STALE | Discovery 失败 + Retry | Registry 不可用 (Preflight 降级红条) |
| **E-37** Clock | Reference Locked (绿) | skeleton | 无外部 Reference, SYSTEM 兜底提示 | CLOCK_DEGRADED (offset 越界, 黄) | CLOCK_FAILED (Fallback 生效, 红) | 全部 Reference 失效 (红条 + Incident) |
| **O-43** Incident Timeline | 时间线滚动 | skeleton | 时间窗内无事件 (正常空态说明) | 事件密度异常提示 | 加载失败 + Retry | 在播 Channel 出现 ACTIVE 事件 (置顶红条) |
| **O-45** Benchmarks | 最新 p50/p95/p99 表 | skeleton | 无基准数据 + "运行基准" 引导 | measured 接近 target (>80%) | 基准任务失败 | measured p95 > target (HOT 场景红条) |
| **A-52** Roles | 角色列表 | skeleton | 仅内置 4 角色 (不可删提示) | 自定义角色含高危权限组合 | 并发编辑保存冲突 | 移除内置角色阻断 |
| **A-53** Permissions | 矩阵展示 | skeleton | 不适用 (内置矩阵永不为空, 显示说明) | 自定义权限覆盖提示 | 加载失败 + Retry | 不适用 |
| **A-55** System Settings | 10 区设置 | skeleton | 不适用 (各区永有默认值) | 改动未 Apply (ChangeSet 待提交) | 保存校验失败 | failover/安全类设置修改需 L3 + ChangeSet |
| **E-36** Resource | 容量仪表全绿 | 图表 skeleton | 无 Runtime 数据 (Host 离线) | 80-90% 黄 | 90-95% 红 + 释放建议 | >95% 红条 + 写入保护提示 |
| **A-51** Users | 用户列表 | skeleton | 仅初始 admin 账户提示 | 账户即将过期 / 异地登录 | 用户名冲突 / 保存失败 | 锁定自身 admin 账户阻断 |
| **A-54** Audit Logs | 日志流 | skeleton | 查询窗内无记录 + "审计已启用" 说明 | Hash Chain 校验慢告警 | 查询失败 + Retry | Hash Chain 验证失败 (审计完整性破坏, 红条) |

### 2.2 危险操作 3 层

继承 Phase 0.5A 锁定：

| 层级 | 触发 | UI 表现 |
|---|---|---|
| **L1 普通** | 切播 / 编辑 Profile / 启停录制 | 按钮直接执行 |
| **L2 重要** | TAKE / Apply Change Set / 替换 Asset | 二次确认 Modal + 3s 倒计时 |
| **L3 危险** | DELETE Asset / ROLLBACK / DISABLE Output / 强制 STOP | 必须输入 "YES" + 5s 倒计时 + 红框 |

### 2.3 权限模型 (R / W / A)

| 权限 | 含义 | UI 表现 |
|---|---|---|
| **R (Read)** | 查看 | 所有页面对所有人都至少 R |
| **W (Write)** | 编辑 | 输入框 / 按钮可交互 |
| **A (Admin)** | 管理 | 删 / 改权限 / 系统设置 |

### 2.4 4 角色 (V0.2 锁定)

| 角色 | 范围 | 默认权限 |
|---|---|---|
| **Operator 操作员** | 1A 播控 + 1B 运维只读 | R 全 + W 0.5A 9 Core + 部分 Profile |
| **Director 节目总监** | 1A 播控 + 2 Media 全部 | R 全 + W 0.5A 9 Core + M-11~16 + 部分 P-24/26 |
| **Engineer 工程师** | 1+2+3+4 全部 | R 全 + W 0.5A 9 Core + 0.5B 全部 + 不含 A 全部 |
| **Admin 管理员** | 全 | A 全部 |

### 2.5 Navigation 跳转规则

- **同工作域内**: sidebar 同组显示，左侧导航
- **跨工作域**: breadcrumb + 顶部菜单（6 大工作域切换）
- **Drawer / 子页**: 不计入主导航（如 Asset Detail 是 Media Library 的子页）
- **Modals**: 不计入导航

### 2.6 i18n 规范（继承 0.5A）

- 中文为主
- Section header 中英并列（`<span class="cn">中文</span> / ENGLISH`）
- 按钮 / 表格列 / 状态指示：双语
- 保留 Canonical Vocabulary 原文（PACKET / FRAME / MASTER / HLS / RTMP / WebRTC / SRT / H.264 / H.265 / PTP / LUFS / EBU R128 / dBTP / TS / Rust / JSON Schema / PG enum 等）

---

## 3. 角色 × 工作域 × 权限 全局矩阵

| 工作域 | Operator | Director | Engineer | Admin |
|---|---|---|---|---|
| **01 Broadcast** 播控 | R+W (1A 9 Core) | R+W (1A 9 Core) | R+W (1A 9 Core) | A |
| **02 Media** 媒体 | R | R+W (M-11~16) | R+W (M-11~16) | A |
| **03 Profiles** 配置 | R (own channel) | R + W (仅 P-24 Graphic / P-26 Rights 模板) | R+W (P-21~27) | A |
| **04 Engineering** 工程 | R (limited) | R (limited) | R+W (E-31~37) | A |
| **05 Operations** 运维 | R+W (1A 9 Core + O-42 告警确认) | R (O-41~45) | R+W (O-41~45) | A |
| **06 Administration** 管理 | — | — | R (own profile) | A |

**R+W (own channel)**: Operator 只能改自己 channel 相关的 Profile；改他人 channel 的 Profile 需要 Engineer / Admin

**A 全部**: Admin 可改所有页面的所有字段

---

# 4. 02 Media 媒体资产工作域 (Director / Engineer)

## 工作域概述

```
Media Library (M-11)  ← 入口
  ├─ Asset Detail (M-12)   ← 列表点击 (5 Tab 锁定)
  │   ├─ Overview Tab (M-12a)  ← 默认子 tab
  │   ├─ Versions Tab (M-12b)
  │   ├─ QC Tab (M-12c)
  │   ├─ Rights Tab (M-12d)
  │   └─ History Tab (M-12e)
  ├─ Upload / Ingest (M-13)  ← 新建
  └─ Transcode Center (M-14)  ← 转码入口
      ├─ Transcode Jobs (M-15)  ← 队列
      └─ Versions / Renders (M-16)  ← 产物
```

**主要角色:** Director (节目总监) / Engineer (工程师)
**关联架构对象:** `media_assets / asset_versions / asset_rights / media_jobs / media_job_attempts / uploads / upload_jobs`

---

## M-11 · Media Library 媒体库

| 维度 | 定义 |
|---|---|
| **目标** | 列出系统所有媒体资产（视频/音频/图片/字幕/Project/Recording），支持筛选、搜索、批量操作 |
| **主要操作** | 搜索 / 筛选 / 选择 / 上传 / 批量操作 (Archive/Delete/Transcode) |
| **权限** | R: 全部 · W: Director+ · A: Admin |
| **关联工作流** | Chain 3 Playout / Chain 4 Engineering |
| **跳转** | 入口: 顶部菜单"Media" · 出口: M-12 (点击 Asset) / M-13 (Upload) / M-14 (Transcode) |

### 信息架构

**顶部工具栏:** `[+ Upload]` 主按钮 (L1) · `[Import]` (L1) · `[Scan]` (L1) · `[Search]` 搜索框 · 视图切换 (网格/列表)
**左侧筛选:** 类型 (Video/Audio/Image/Subtitle/Project/Recording) · 状态 (UPLOADING/INGESTING/PROBING/READY/TRANSCODING/QC_FAILED/QC_PASSED/RIGHTS_BLOCKED/ARCHIVED/FAILED) · 标签 · 上传时间 · 拥有者

**主列表字段:** 缩略图 · 名称 · 类型 · Duration · Resolution · Codec · FPS · Audio · Loudness · QC · Rights · Versions · Status

**每行操作:** `[Open]` (L1) · `[Transcode]` (L1) · `[Create Variant]` (L1) · `[Archive]` (L2) · `[Delete]` (L3)

### 状态模型
- Normal: READY 资产 > 0
- Loading: 首次/刷新, 10 行 Skeleton (与 M-11 wireframe 一致; DS §7 允许 3-10 行)
- Empty: 0 assets + `[+ Upload Asset]` 主按钮 + "从录制导入" 副按钮
- Warning: 黄色 Banner "12 assets have QC issues, 3 have rights issues"
- Error: Probe 失败 / Hash 不匹配 + `[Retry Probe]`
- Critical: Storage > 95% 顶部红条（阈值口径全局统一: Warning ≥80% / Error ≥90% / Critical >95%, 与 E-36 一致）

---

## M-12 · Asset Detail 资产详情

| 维度 | 定义 |
|---|---|
| **目标** | 单个资产全部信息：元数据 / 预览 / 版本 / QC / Rights / 历史 |
| **主要操作** | 预览 / Transcode / Create Variant / QC / Replace / Archive / Delete |
| **权限** | R: 全部 · W: Director+ (自己) / Engineer · A: Admin |
| **跳转** | 入口: M-11 点击 · 出口: M-14 (Transcode) / M-15 (Job) |

### 信息架构

**Header:** 缩略图 + 资产名 + 状态徽章 + 关键指标条 (Duration/Resolution/FPS/Codec/Container/File Size/Hash/Created) + 主操作

**Tab 区 (5 个 — 锁定):**

| Tab | 编号 | 内容 |
|---|---|---|
| **Overview 总览** | M-12a | 默认 Tab · 摘要 + 缩略图 + 关键元数据 + Used By (引用清单) + 最近变更 |
| **Versions 版本** | M-12b | 列表 (Version名/类型/编码/分辨率/大小/时间/状态) + `[+ Create Version]` |
| **QC 质量** | M-12c | qc_profile + 检测项 (Black/Freeze/Audio/Loudness/AV Sync) 阈值与实测 + `[Re-run QC]` / `[Change QC Profile]` |
| **Rights 版权** | M-12d | 列表 (地域/平台/起始/截止/状态) + `[Block]` `[Extend]` `[Override]` `[Audit]` |
| **History 历史** | M-12e | 时间线 (谁/何时/改了什么) + 可回滚 |

**Tab 顺序锁定:** Overview → Versions → QC → Rights → History (Overview 默认显示)

### 状态模型
- Normal: 资产 READY, 5 tab 完整
- Loading: 5 tab Skeleton
- Empty: 新上传 "Probe in progress..." spinner
- Warning: QC 有非阻塞项 (Loudness 偏离 0.5 LUFS)
- Error: QC 失败 / Probe 失败 / 编码失败
- Critical: Rights 已过期 + 仍在 Schedule 中

---

## M-13 · Upload / Ingest 上传 / 导入

| 维度 | 定义 |
|---|---|
| **目标** | 把新资产导入系统: 本地/URL/录制/磁链 4 种来源 |
| **主要操作** | 拖拽 / URL 粘贴 / 选择来源 / 启动 |
| **权限** | R+W: Director+ · A: Admin |

### 信息架构 (4 Tab)

**Tab 1 Local File:** 拖拽区 · 多文件并发 · 实时进度
**Tab 2 URL:** HTTP/HTTPS/S3/SFTP · `[Fetch Metadata]` 先 probe
**Tab 3 Recording Import:** Channel / 时间范围 / 长度
**Tab 4 Magnet:** magnet 链接 · 下载进度

**公共字段:** Asset Name (必填) · Type · Initial QC Profile (默认 P-25) · Initial Rights Profile (默认 P-26) · Tags · Auto-Transcode After Upload

### 状态模型
- Normal: 空闲, 等待拖入
- Loading: 上传中/Probe 中 (实时进度)
- Empty: 无文件时显示 dropzone
- Warning: 上传慢 / 格式不常见 / 配额将满
- Error: 上传失败 / Probe 失败 / 格式不支持
- Critical: Storage < 5% 不可上传

---

## M-14 · Transcode Center 转码中心

| 维度 | 定义 |
|---|---|
| **目标** | 创建 / 管理 / 监控 转码任务 |
| **主要操作** | New Job / 选择 Input / Profile / 监控 / Pause / Cancel / Retry |
| **权限** | R: Operator+ · W: Director+ · A: Admin |

### 信息架构

**顶部:** `[+ New Job]` · 实时统计 (Running/Queued/Completed/Failed 今日)
**左侧:** Queue (M-15) / Versions (M-16) / Workers (跳 E-36)

**主区 New Job Modal (L1):**
- Input Asset · Encoding Profile · Output Container (TS/MP4/fMP4)
- Output Destination (Local/S3/NFS) · Worker Assignment
- Schedule (Now/Scheduled/On Event) · Priority (1-10)
- Notify On (Complete/Failed/Both)

**任务列表字段:** Job ID · Asset · Profile · Status · Progress · FPS · Speed · ETA · Worker · Actions
**右侧详情面板:** Input/Output/Profile/Worker/实时资源 + 进度条 + 实时日志 + Pause/Cancel/Retry

### 状态模型
- Normal: 有 Running 任务, 健康
- Loading: 任务列表加载
- Empty: 0 jobs + 引导 "Create your first transcode job"
- Warning: 队列堆积 > 10 / Worker CPU 持续 > 90% / ETA 超预期
- Error: 任务失败 (Log) + [Retry]
- Critical: 全部 Workers offline / Disk write 失败

---

## M-15 · Transcode Jobs 转码任务 (M-14 子页)

| 维度 | 定义 |
|---|---|
| **目标** | 完整任务列表 (含历史), 过滤 / 批量重试 / 导出 |
| **主要操作** | 过滤 / 搜索 / 批量重试 / 导出 CSV |
| **权限** | R: 全部 · W: Engineer+ (Retry) · A: Admin |

**字段:** Job ID · Asset · Profile · Status · Started · Duration · Worker · Attempts (重试次数)
**操作:** `[View Detail]` `[Retry (L1)]` `[Open Asset]` `[Copy Log URL]` · 批量 `[Retry Selected (L1)]` `[Cancel Selected (L2 — 仅 QUEUED/PENDING 可取消; RUNNING 需确认停止 Worker)]` `[Export CSV]`

---

## M-16 · Versions / Renders 版本与渲染产物

| 维度 | 定义 |
|---|---|
| **目标** | 所有 Asset 所有版本统一视图 (Master/Proxy/HLS/Mobile/Archive) |
| **主要操作** | 按 Asset 筛选 / 按 Profile 筛选 / 导出 |
| **权限** | R: 全部 · W: Engineer (Delete) · A: Admin |

**字段:** Asset · Version 名 · Profile · 编码 · 分辨率 · 大小 · Hash · 创建时间 · 状态 · 引用数
**操作:** `[Download]` `[Preview]` `[Replace]` `[Delete]` (L3) `[View References]`

### 状态模型
- Normal: 完整
- Warning: 孤岛版本 (无引用, 长期)
- Error: 文件丢失 (Hash 不匹配)
- Critical: 磁盘已满, 删除旧版

---

# 5. 03 Profiles 配置 Profile 工作域 (Engineer)

## 工作域概述

```
Profiles
  ├─ Encoding Profiles (P-21)   ← 视频编码
  ├─ Output Profiles (P-22)     ← 输出目标 (SRS HLS/RTMP/WebRTC/File)
  ├─ Audio Profiles (P-23)      ← 音频 (LUFS / True Peak)
  ├─ Graphic Profiles (P-24)    ← 图文模板 (Logo/Bug/Subtitle)
  ├─ QC Profiles (P-25)         ← 质量检测规则
  ├─ Rights Profiles (P-26)     ← 版权 (地域/平台/期限)
  └─ Edge Policy Profiles (P-27) ← 边策略 (Backpressure/Latency Budget)
```

**主要角色:** Engineer
**关联架构对象:** `encoding_profiles / output_profiles / audio_profiles / graphic_profiles / qc_profiles / rights_profiles / edge_policy_profiles / composition_templates / composition_layers`

**重要架构约束 (V0.2 锁定):**
- **Encoding Profile ≠ Output Profile** — Encoding 决定"怎么编码", Output 决定"送到哪里", 两者必须分离
- **Profile 修改走 Change Set** (X3) — 任何 Profile 修改都需 VALIDATED → APPLIED
- **Profile 是引用对象** — Channel 引用 Profile ID, 不复制内容

---

## P-21 · Encoding Profiles 编码 Profile

| 维度 | 定义 |
|---|---|
| **目标** | 集中管理所有视频/音频编码配置 (Broadcast H264 / HEVC / Web / Proxy / Archive) |
| **主要操作** | Create / Edit / Clone / Test / Delete |
| **权限** | R: 全部 · W: Engineer · A: Admin |
| **跳转** | 入口: 顶部菜单"Profiles" · 出口: M-14 (New Job 选 Profile) / P-22 (Output 引用) |

### 信息架构

**列表字段:** Profile Name · Codec · Resolution · FPS · Bitrate Mode · Bitrate · GOP · Container · Use Count (引用次数) · Last Modified

**详情 (9 区 — 广播级完整字段):**

#### Basic
- Profile Name (必填, 唯一)
- Description
- Category (Broadcast / Web / Archive / Proxy / Mobile)
- Tags

#### Video — Codec
- Codec (H.264 / H.265 / VP9 / AV1)
- Profile (Baseline / Main / High)
- Level (3.0 / 3.1 / 4.0 / 4.1 / 5.0 / 5.1 / 5.2)
- Pixel Format (yuv420p / yuv422p / yuv444p / yuv420p10le)

#### Video — Format
- Resolution (1920×1080 / 1280×720 / 3840×2160 / 自定义)
- FPS (25 / 30 / 50 / 60 / 自定义)
- Time Base (1/90000 default)
- **Pixel Aspect Ratio (SAR)** (1:1 / 4:3 / 16:9)
- **Field Order** (Progressive / Top Field First / Bottom Field First)
- **Color Space** (BT.601 / BT.709 / BT.2020)
- **Color Range** (TV / PC / JPEG Full)
- **Color Transfer** (BT.709 / SMPTE 2084 / HLG)
- **Color Primaries** (BT.709 / BT.2020)
- **Color Metadata** (HDR10 / HLG / SDR / None)

#### Video — Bitrate
- Bitrate Mode (CBR / VBR / Capped VBR)
- Bitrate (Mbps)
- **VBV Maxrate** (Mbps, for CBR)
- **VBV Buffer** (kbit, for CBR)
- **HRD** (High Profile only, for broadcast compliance)
- Min Bitrate (Mbps, for VBR)
- Max Bitrate (Mbps, for VBR/Capped VBR)
- Quality / CRF (for VBR)

#### Video — GOP
- GOP Size (12 / 25 / 50 / 100 / 250)
- **Closed GOP / Open GOP** (广播必 Closed)
- **Keyframe / IDR Policy** (every N frames / on event)
- **Reference Frames** (1-16)
- B-Frames (0 / 2 / 4)
- Lookahead (0 / 10 / 20)
- Scene Cut Detection (on/off)

#### Video — Encoding
- **Hardware Encoder** (Runtime Discovery — 见下)
- Threads (1 / 2 / 4 / 8 / auto)
- Preset (ultrafast / superfast / veryfast / faster / fast / medium / slow)
- Tune (film / animation / grain / stillimage / zerolatency)
- Latency Mode (Normal / Low Latency / Ultra-Low)

#### Audio
- Codec (AAC / Opus / MP3 / Vorbis)
- Sample Rate (44.1k / 48k / 96k)
- **Channel Layout** (Mono / Stereo / 2.1 / 5.0 / 5.1 / 7.1.4)
- **Bit Depth** (16 / 24 / 32)
- Bitrate (kbps)
- Loudness Reference (LUFS, optional, 联动 P-23)
- AV Sync Offset (ms, optional)

#### Container
- MPEG-TS / fMP4 / MP4 / MOV / MKV
- Segment Duration (for TS/fMP4)
- Index Mode (for fMP4)

#### Advanced
- Metadata Policy (Copy / Rewrite / Drop)
- Timecode Policy (Preserve / Drop / Re-stamp)
- Side Data (SEI / HDR mastering display / Content light level)

#### Validation (4 检查)
- ✓ Compatible (字段相互一致)
- ✓ Resource OK (服务器端能力足够)
- ✓ Codec supported (Runtime Discovery 检查)
- ✓ Test Encode OK (sample test 跑 5s)

#### Revision
- Version (auto-increment)
- Change Notes
- Created By / At
- Status (DRAFT / ACTIVE / DEPRECATED)

#### Hardware Encoder (Runtime Discovery — 关键边界)

**V0.2 锁定:** GPU / Encoder / BMD 能力来自 Hardware Capability Discovery (E-35)。UI 不能假定硬件存在。

UI 表现形式:

```
Hardware Encoder
[ AUTO ▼ ]

Available (Runtime Discovered):
✓ libx264      (CPU)
✓ libx265      (CPU)
✓ libvpx-vp9   (CPU)
✗ NVENC        (GPU unavailable)
✗ QSV          (unavailable)
✗ VideoToolbox (N/A · Linux)
✗ BMD H.264    (no BMD encoder port)
```

UI 行为:
- 选项**只列出** E-35 Runtime Discovery 报告的可用 encoder
- 不可用项**显式标注** (✗) + 原因 (GPU unavailable / N/A on Linux)
- Profile 保存时**强制** Preflight 验证所选 encoder 实际可用
- Encoder 不可用 → Validation FAIL (Critical) → Profile 不能 ACTIVE

**编码决策链 (X1 Compiler):**
```
Codec → Encoder → Capability → Resource Estimate
  ↓        ↓          ↓              ↓
 选      动态查     查 E-35        查 E-36
 Codec  哪个 Encoder 是否支持     资源是否够
       最合适
```

**操作:** `[+ Create Profile]` · `[Clone]` (L1) · `[Test Encode]` (L1, 跑 5s 测试) · `[Edit]` (L2) · `[Delete]` (L3)

### 状态模型
- Normal: Profile 完整 + Validation PASS
- Loading: 列表/详情加载
- Empty: 0 profiles + 引导 "Create from preset"
- Warning: Resource OK 但 Compatibility 警告 (如 x264 不支持 8K)
- Error: Validation FAIL (例如 4K HEVC + CBR 1Mbps 不可能)
- Critical: Server 端 encoder 不可用 (N/A → 整个 Profile 不可用)

---

## P-22 · Output Profiles 输出 Profile

| 维度 | 定义 |
|---|---|
| **目标** | 集中管理所有输出目标配置 (SRS HLS / RTMP / WebRTC / SRT / UDP / RTP / File) |
| **主要操作** | Create / Edit / Test Connection / Delete |
| **权限** | R: 全部 · W: Engineer · A: Admin |
| **跳转** | 入口: 顶部菜单"Profiles" · 出口: Channel 配置 / Output 监控 |

### 信息架构

**列表字段:** Profile Name · Protocol · Host:Port · Stream Key · Status · Use Count · Last Modified

**详情 — 严格按 V0.2 支持范围分层:**

#### V0.2 Supported (UI 可配置 — Backend 已实现)

| Protocol | Host / Port | Key | Transport |
|---|---|---|---|
| **HLS** (SRS) | SRS endpoint | Stream path | HTTP/HTTPS |
| **RTMP** (SRS) | SRS endpoint | Stream key | TCP |
| **WebRTC** (SRS) | SRS WHIP endpoint | Stream path | UDP/QUIC |
| **SRT** | Host:Port | Stream ID | UDP |
| **UDP MPEG-TS** | Host:Port | Multicast group | UDP |
| **RTP** (RTP over UDP) | Host:Port | SSRC / Stream ID | UDP |
| **File** (Archive) | Local / S3 / NFS | Path template | — |

#### Reserved / V0.4+ (UI 显示但标 "Reserved" — Backend 未实现)

| Protocol | 状态 | V0.2 表现 |
|---|---|---|
| SDI Master Output | V0.4 Target | Architecture Contract RESERVED · V0.2 DISABLED |
| DASH | Future | 灰显 + "DASH output is reserved for V0.4+, not configurable in V0.2" |
| DRM (Widevine / FairPlay / PlayReady) | Future | 灰显 + 提示 |

**V0.2 约束 (重要):** UI 不能让 V0.2 用户误以为 DASH/DRM/SDI 已经可配置。Reserved 协议必须显式标 "[Reserved · V0.2 Disabled]"。

#### Protocol & Destination (V0.2 supported 内的详细字段)
- Protocol (HLS / RTMP / WebRTC / SRT / UDP / RTP / File) — **V0.2 限定 7 种**（0.5B.2 P0-6 加入 RTP, 与 RTPAdapter 对齐; 详见 §20.2 3-Tier）
- Host / IP
- Port
- Stream Key / Path
- Transport (TCP / UDP / QUIC — 按 protocol 自动限定)
- SRS Gateway Endpoint (HLS/RTMP/WebRTC 必填)

#### HLS Specific (V0.2 supported)
- Segment Duration (1s / 2s / 4s / 6s)
- Playlist Window (3 / 5 / 10 segments)
- Codec (H.264 / H.265)
- Latency Mode (LL-HLS / Normal HLS)
- **DRM 字段: V0.2 灰显, 标 "Reserved"**

#### RTMP Specific
- URL (rtmp://host:port/app/stream)
- Backup URL (failover; 切换行为由 Edge Policy (P-27) 决定)

#### RTP Specific (0.5B.2 P0-6)
- Address (host:port) + SSRC
- Payload Type ( MPEG-TS over RTP / RAW )
- 与 UDP MPEG-TS 的区别: RTP 带 RTP 头 (序列号/时间戳), 供接收端排序与丢包检测

#### WebRTC Specific
- ICE Servers (STUN / TURN)
- Signaling URL (SRS WHIP API)
- DTLS / SRTP enabled
- Bitrate cap

#### Latency / Reliability (0.5B.2 P0-6 三拆 + Edge Policy 引用)
- **Delivery Latency Target** — 协议本身延迟 (e.g. LL-HLS 2s segment / RTMP 1-3s)
- **Channel E2E Latency Target** — 整链路端到端预算 (e.g. ≤ 200 ms)
- **Failover Latency Target** — 只读引用 `hot_standby_levels.target_failover_time_ms` (e.g. Policy: HOT → 100 ms; Target 是预算, 实测看 failover_benchmarks)
- **Edge Policy Profile** — 引用 P-27 (Retry / Reconnect / Failover 统一在 P-27 配置, P-22 仅持有引用, 不再独立配置)

#### Player Capability
- Player Hint (Safari / Chrome / Android / iOS)
- Required Codecs
- Auto Transcode on demand

#### V0.2 Architecture 约束 (重要, 影响 UI):
- **SDI Master Output** (V0.4 Target) 在 0.5A #09 Health Tree 已显式标 "DISABLED (V0.4 Target)" — P-22 UI 必须继承此约束
- **DASH/DRM** 等 Reserved 协议, UI 不能诱导用户配置, 否则 Phase 4 实现时返工

---

## 4.1 Output 三元组语义 (Output Profile / Variant / Destination) — 锁定

> **V0.2 §3.7.1 Program-scope Master 锁定: Output 不是 1 个对象, 是 3 个独立对象的链。**
>
> UI 必须明确区分这 3 个概念, 否则与 V0.2 架构脱节。

### 概念定义 (锁定)

| 概念 | 含义 | 典型实例 | 决定权 |
|---|---|---|---|
| **Output Profile (P-22)** | **如何交付** — 编码 / 协议 / 可靠性策略 / Latency 目标 | "SRS HLS 1080p25 CBR 5Mbps LL-HLS" | Engineer |
| **Output Variant** | **Program-scope Master 的某个具体派生版本** — 与 Channel 1:1 绑定 | "CH01 News HD 主 HLS 1080p" | Channel 配置时引用 |
| **Output Destination** | **实际发送位置** — Endpoint / URL / Stream Key | "https://srs.internal/live/ch01.m3u8" | Deployment |

### 三者关系 (V0.2 锁定)

```
Output Profile (P-22)
    定义 HOW (如何编码、协议、可靠性)
        ↓
    1 个 Profile 可以被 多个 Variant 引用
        ↓
Output Variant
    引用 Profile + 绑定 Channel (1:1)
    + 持有 Destination 列表
        ↓
    1 个 Variant 可以发到 多个 Destination (主备)
        ↓
Output Destination
    具体 endpoint (host:port/path)
    + 当前健康状态
```

### V0.2 关键约束

- **Output Variant 是 Channel 的子对象**, 不是 Channel 的子字段, 也不在 Channel 表里
- **Output Variant 通过 `channel_routes` 关联 Channel** (V0.2 §3.4 锁定)
- **Output Destination 是 Output Variant 的子对象** (运行时配置)
- **修改 Output Profile 不直接影响 Output Variant 的 Destination** (Destination 独立管理)
- **SDI Master Output Variant 状态固定为 DISABLED** (V0.4 Target)

### UI 实现选择 (P-22 内部结构 — 锁定)

P-22 Output Profile 页面**不**包含 Variant / Destination 配置 (避免巨型表单)。结构是:

```
P-22 Output Profiles
├── 列表 (Profile 定义)
└── 详情 (Profile Definition)
    ├── Basic
    ├── Protocol & Destination (V0.2 supported 字段, 7 种协议)
    ├── HLS / RTMP / WebRTC / SRT / UDP / RTP / File 特定字段
    ├── Latency (三拆: Delivery / Channel E2E / Failover) + Edge Policy 引用 (P-27)
    └── Player Capability

P-22 内部 子页 (新增)
├── Variants 列表 (引用此 Profile 的所有 Variant)
│   └── 跳到 Output Variant 详情 (E-31 Graph Designer Channel 配置)
└── Destinations 列表 (历史 / 模板, V0.4 启用, V0.2 显示 "Reserved")

Output Variant 管理 — 实际在:
└─ E-31 Graph Designer → Channel 配置 → Output 节点
   或
└─ Channel 详情页 → Output Variants Tab (M-12 类子页)
```

### 与 V0.2 §3.4 / §3.7.1 / §3.13 关联

- §3.4: Switch Mode 决策 (PACKET/FRAME/MASTER) — 影响 Output Variant 选择
- §3.7.1: Program-scope Master 三个独立 graph (Video/Audio/Metadata) — 每个可独立配 Output Variant
- §3.13: AVSync Manager — 监控每个 Output Variant 的 Offset/Drift

### 必须避免的 UI 反模式

- ❌ Output Profile 页面直接绑定 Endpoint URL (混淆 Profile 和 Destination)
- ❌ Output Profile 列表里直接显示 "running / disconnected" (运行态在 Output Variant)
- ❌ V0.2 UI 让用户配置 DASH/DRM (Backend 未实现, Reserved)
- ❌ Output Variant 跨 Channel 共享 (V0.2 1:1 绑定)

**操作:** `[+ Create Profile]` · `[Test Connection]` (L1) · `[View in Output]` (跳 Output 页面) · `[Edit]` (L2) · `[Delete]` (L3, 检查 Use Count)

### 状态模型
- Normal: Profile 完整 + Test Connection PASS
- Loading: 列表/详情加载
- Empty: 0 profiles + 引导
- Warning: Latency Target 与 Profile 不匹配 (HLS Normal = 1-3s, Target 100ms 不可能)
- Error: Test Connection FAIL (host 不可达 / auth 失败)
- Critical: Profile 引用数 > 0 但 Server 端 Adapter 不可用

---

## P-23 · Audio Profiles 音频 Profile

| 维度 | 定义 |
|---|---|
| **目标** | 集中管理音频处理配置 (Loudness / True Peak / Phase / AV Sync) |
| **主要操作** | Create / Edit / Test / Delete |
| **权限** | R: 全部 · W: Engineer · A: Admin |

### 信息架构

**列表字段:** Profile Name · Channel Layout · Target LUFS · True Peak · Use Count · Last Modified

**详情 (4 区):**

#### Channels
- Sample Rate (44.1k / 48k / 96k)
- Channel Layout (Stereo / 5.1 / 7.1.4)
- Bit Depth (16 / 24 / 32)

#### Loudness (EBU R128)
- Target LUFS-I (-23 / -24 / -16)
- Target Short-term (3s window)
- True Peak Max (-1 / -2 dBTP)
- Loudness Range (LRA Max)

#### Processing
- Delay (ms)
- Gain (dB)
- Limiter (on/off, threshold)
- Downmix (5.1 → Stereo 规则)
- Phase (正常 / 反相)
- AV Sync Offset correction (ms)
- Drift correction (策略)

#### Validation
- ✓ EBU R128 Compliant
- ✓ Channel count OK
- ✓ AV Sync within budget

---

## P-24 · Graphic Profiles 图文 Profile

| 维度 | 定义 |
|---|---|
| **目标** | 集中管理图文模板 (Logo / Bug / Lower Third / Subtitle / Ticker / Sponsor) |
| **主要操作** | Create / Edit / Preview / Delete |
| **权限** | R: 全部 · W: Director+ · A: Admin |

### 信息架构

**列表字段:** Template Name · Type · Canvas · Use Count · Last Modified

**详情:**

#### Canvas Preview
- 1920×1080 base canvas
- 实时预览 (含 Program Video 占位)

#### Layers
- 多个 layer (按 z-order)
- 每层: 类型 (Image / Text / Video / Live) / 位置 / 大小 / 动画 / 进出时机

#### Scope
- **Program Scope** (应用到整档节目)
- **Variant Scope** (只应用到 Variant Composition)

#### Animation
- In / Out animation
- Duration
- Easing

#### Trigger
- Time-based (相对节目开始)
- Event-based (Detect Logo / Commercial)
- Manual (Operator 手动触发)

**操作:** `[+ Create Template]` · `[Live Preview]` · `[Apply to Channel]` · `[Edit]` · `[Delete]` (L3)

---

## P-25 · QC Profiles 质量检测 Profile

| 维度 | 定义 |
|---|---|
| **目标** | 集中管理 QC 检测规则 (Black / Freeze / Loudness / AV Sync) |
| **主要操作** | Create / Edit / Test / Delete |
| **权限** | R: 全部 · W: Engineer · A: Admin |

### 信息架构

**列表字段:** Profile Name · Severity · Use Count · Last Modified

**详情 (4 区):**

#### Video QC
- Signal Loss Threshold (sec)
- Black Frame Threshold (sec)
- Freeze Frame Threshold (sec)
- FPS Deviation (%)
- Resolution Check
- PTS Continuity (ms)
- Duplicate Frame (count)
- Drop Frame (count)

#### Audio QC
- Silence Duration (sec)
- Peak (dBFS)
- L/R Balance (dB)
- Phase (正常 / 反相)
- Loudness (LUFS, target ± tolerance)

#### Network QC
- Packet Loss (%)
- Jitter (ms)
- RTT (ms)

#### AV Sync
- Offset Threshold (ms)
- Drift Threshold (ms/min)

**Severity 等级:** INFO / WARNING / ERROR / CRITICAL

---

## P-26 · Rights Profiles 版权 Profile

| 维度 | 定义 |
|---|---|
| **目标** | 集中管理版权规则 (地域 / 平台 / 期限) |
| **主要操作** | Create / Edit / Override / Audit |
| **权限** | R: 全部 · W: Director+ · A: Admin |

### 信息架构

**列表字段:** Profile Name · Territory · Platform · Valid Period · Status

**详情:**

#### Territory
- China / Global / Asia / North America / Europe / Custom

#### Platform
- HLS / RTMP / WebRTC / Archive / VOD

#### Period
- Start / End / Unlimited

#### Override
- Override per Channel
- Override per Asset
- Audit Trail (who, when, why)

#### Status
- ACTIVE / EXPIRED / BLOCKED / OVERRIDDEN

---

## P-27 · Edge Policy Profiles 边策略 Profile

| 维度 | 定义 |
|---|---|
| **目标** | 集中管理 Edge 边策略 (Backpressure / Latency Budget / Queue Depth) |
| **主要操作** | Create / Edit / Test / Delete |
| **权限** | R: 全部 · W: Engineer · A: Admin |

### 信息架构

**详情:**

#### Latency Budget
- per Edge target (ms)
- per Channel target (ms)

#### Backpressure Policy
- DROP (丢帧)
- BLOCK (阻塞生产者)
- BUFFER (累积到 max)
- DEGRADE (降级)
- SWITCH (切源)

#### Queue
- max_queue (frames)
- max_buffer (MB)

#### NEVER_SILENT
- 是否允许静帧
- 静帧超阈值动作

#### Latency Probe
- 7 Core + 2 Client E2E + 1 CDN Optional

---

# 6. 04 Engineering 工程工作域 (Engineer)

## 工作域概述

```
Engineering
  ├─ Graph Designer (E-31)         ← 0.5A #08 (LOCK FINAL)
  ├─ Preflight Center (E-32)       ← 新增
  ├─ Change Sets (E-33)            ← 新增
  ├─ Capability Registry (E-34)    ← 新增
  ├─ Device Registry (E-35)        ← 新增
  ├─ Resource / Capacity (E-36)    ← 新增
  └─ Clock (E-37)                  ← 新增
```

**主要角色:** Engineer
**关联架构对象:** `signal_contracts / node_contracts / preflight_runs / config_revisions / change_sets / change_set_items / device_registry / hardware_capability / clock_fallback_chain / failover_benchmarks / latency_probes`

---

## E-31 · Graph Designer 图设计 *(= 0.5A #08)*

**状态:** 🟢 Phase 0.5A LOCK FINAL — 见 [`operator/08-graph-designer.html`](operator/08-graph-designer.html)

---

## E-32 · Preflight Center 预检中心

| 维度 | 定义 |
|---|---|
| **目标** | 统一入口运行 Preflight (Graph / Playout / Channel), 看到所有 critical/warning |
| **主要操作** | 选 Target / Run / 看报告 / Apply 触发 Change Set |
| **权限** | R: Operator+ · W: Engineer+ (Run / Apply) · A: Admin |
| **关联工作流** | X2 Preflight 三个层级 (Static / Resource / Runtime Readiness) |

### 信息架构

**顶部:** Target 选择器 (Channel / Graph / Playout) · `[Run Preflight]` 主按钮

**报告区:**

#### Static
- ✓ Graph valid (无环)
- ✓ Contract valid (Source → Switcher → Output 数据面一致)
- ✓ Rights valid (Asset 版权匹配)
- ✓ Asset exists
- ✓ Latency within budget

#### Resource (9-Dim Resource Vector — V0.2 §3.11 锁定)
- ✓ CPU_THREADS
- ⚠ GPU_SESSIONS (选配)
- ✓ VRAM_MB
- ✓ RAM_MB
- ✓ NIC_INGRESS_MBPS
- ✓ NIC_EGRESS_MBPS
- ✓ DISK_WRITE_MBPS
- ✓ PCIE_RX_MB_S
- ✓ PCIE_TX_MB_S
- ✓ BMD_INPUT_TOKENS
- ✓ BMD_OUTPUT_TOKENS
- ✓ DEVICE_EXCLUSIVITY_OK

#### Runtime Readiness
- ✓ SRS Adapter ready
- ✓ Backup node ready
- ✓ Source online
- ✓ Recorder ready
- ✓ Filler available
- ✓ Output QC ready

**底部:**
- Summary: 0 critical / 2 warnings
- `[Apply]` (L2, 触发 Change Set)
- `[Export Report]`

### 状态模型
- Normal: 0 critical / 0 warning
- Loading: Preflight 跑中
- Empty: 未跑过 Preflight
- Warning: 有 warning
- Error: 有 critical 1+ → Apply 按钮禁用
- Critical: 多 critical 阻塞 Apply

---

## E-33 · Change Sets 变更集

| 维度 | 定义 |
|---|---|
| **目标** | 所有配置变更的统一管理 (DRAFT → VALIDATED → APPLIED → ROLLED_BACK) |
| **主要操作** | List / Create / Validate / Apply / Rollback |
| **权限** | R: 全部 · W: Engineer+ (Create/Apply) · A: Admin |
| **关联工作流** | X3 Configuration Versioning |

### 信息架构

**列表字段:** CS ID · Status · Targets · Created By · Created At · Applied At · Phase

**详情:**

#### Status 状态机 (Business Status + Execution Phase — 必须分离)

**V0.2 Schema 锁定:**
- `change_sets.status` (Business Status) — 业务层面的状态
- `change_set_events.phase` (Execution Phase) — 事务执行阶段
- **两者不能混淆**, 前端 enum 不能多出 `ChangeSetStatus.ABORTED`

```
Business Status (change_sets.status):
DRAFT → VALIDATED → APPLIED → (ROLLED_BACK)
              ↓
          (no ABORTED — 这是 Phase, 不是 Status)

Execution Phase (change_set_events.phase):
PREPARING → APPLYING → COMMITTED
         ↘ ABORTED  ← Phase 终止, 不影响 Business Status
```

**关键区别:**
- Business Status: 用户 / 系统视角的"这次配置变更是什么状态"
- Execution Phase: 事务执行视角的"Apply 操作处于哪个阶段"
- ABORTED 是 Phase 终止, **不**代表 Business Status 失败
- ABORTED 后 Business Status 回 DRAFT (可重试) 或保持 VALIDATED (待重 Apply)

#### Targets
- Channel · Graph Revision · Encoding Profile · Output Profile · Graphic Template · Audio Profile

#### Impact
- CPU +4.2 / RAM +520MB / Latency +18ms

#### Preflight
- PASS (0 critical / 2 warnings)

#### Apply 选项
- `[Immediate]`
- `[Next Event Boundary]` (节目间隙)
- `[Scheduled]` (指定时间)

#### Rollback
- Available → Previous Revision
- Snapshot 已保存

**操作:** `[Create from Current]` · `[Validate]` (L1) · `[Apply]` (L2) · `[Rollback]` (L3) · `[Diff vs Current]`

### 状态模型
- Normal: 0 active CS (全部已 Apply)
- Loading: 列表加载
- Empty: 无历史 CS
- Warning: 有 DRAFT 超过 7 天
- Error: 有 ABORTED
- Critical: 当前 APPLIED 失败需要回滚

---

## E-34 · Capability Registry 能力注册表

| 维度 | 定义 |
|---|---|
| **目标** | 列出所有 signal_contract / node_contract, 支持能力对比 |
| **主要操作** | 搜索 / 筛选 / 对比 / 新建 |
| **权限** | R: 全部 · W: Engineer · A: Admin |

### 信息架构

**列表:** Contract ID · Type (Source / Switcher / Output) · Capabilities · Used By
**详情:** Data Plane · Codec · Resolution · FPS · Clock · Latency · Backpressure
**对比:** 选 2-3 个 Source Contract 并排对比 (Capability 匹配矩阵)

---

## E-35 · Device Registry 设备注册表

| 维度 | 定义 |
|---|---|
| **目标** | 列出所有硬件设备 (BMD / GPU / NIC / NVMe / Clock), 健康状态 |
| **主要操作** | 搜索 / Refresh / Lock / Unlock |
| **权限** | R: 全部 · W: Engineer (Lock/Unlock) · A: Admin |

### 信息架构

**按 Host 分组 (样例使用 [Sample Host] / [Runtime Discovered], 不写实机器快照):**

> **重要:** Architecture ≠ Runtime Hardware Snapshot (V0.2 §3.11 锁定)。
> UI 样例必须用 `[Sample Host]` 占位或 `[Runtime Discovered]`, **不能**把 `10.30.15.10` / `32 核` / `30 GB` 等真实机器参数硬编码到 wireframe。

#### Host: [Sample Host] (Runtime Discovered)
- **CPU:** [Runtime Discovered] threads
- **RAM:** [Runtime Discovered] GB
- **GPU:** [Runtime Discovered] (None / NVIDIA / AMD)
- **BMD:**
  - [Runtime Discovered] e.g. DeckLink Duo 2
    - Port 1 IN / Port 2 IN (per Duo 2 spec)
    - Driver / Firmware
    - Current Lock
    - Temperature
    - Health
- **NIC:**
  - [Runtime Discovered] e.g. eth0 (10GbE)
  - [Runtime Discovered] e.g. eth1 (1GbE management)
- **NVMe:**
  - [Runtime Discovered] e.g. /dev/nvme0n1 (1TB · 38% used)
- **Clock:**
  - [Runtime Discovered] e.g. PTP0 · ptp0 (eth0) · LOCKED · BROADCAST_GRADE

**V0.2 Hardware Snapshot (current_host_snapshot, §3.11):**
- 真实机器 `10.30.15.10` / 32 核 / 30 GB / 3 张 BMD 是 **deployment reference**
- UI **绝不** 硬编码这些值到 wireframe 样例
- UI **通过** E-35 Runtime Discovery API 动态拉取并显示
- 任何硬件增配 / 替换 **不修改** V0.2 架构和 0.5B Surface Spec

**点击 BMD 设备详情:**
- Model
- Serial
- Driver / Firmware
- Input Ports / Output Ports (per device spec)
- Supported Modes / Formats
- Current Lock
- Temperature
- Health

**操作:** `[Refresh]` · `[Lock Device]` (L2) · `[Unlock]` (L1)

### 状态模型
- Normal: 全部设备 HEALTHY
- Loading: Refresh 中
- Warning: 某设备温度高 / 磁盘 > 80%
- Error: 某设备 OFFLINE / 驱动丢失
- Critical: BMD port 锁丢失 + 当前 channel 占用

---

## E-36 · Resource / Capacity 资源容量

| 维度 | 定义 |
|---|---|
| **目标** | 实时资源使用 + 当前分配 + 未来 Plan 容量 |
| **主要操作** | 查看 / Plan 容量 / 触发 GC |
| **权限** | R: 全部 · W: Engineer (Trigger GC) · A: Admin |

### 信息架构

**实时面板 (9-Dim Resource Vector — V0.2 §3.11):**
- CPU_THREADS        ████████░░ 78%
- GPU_SESSIONS       [—  · GPU unavailable] (动态)
- VRAM_MB            [Runtime Discovered]
- RAM_MB             ██████░░░░ 62%
- NIC_INGRESS_MBPS   ████░░░░░░ 41%
- NIC_EGRESS_MBPS    ██████░░░░ 66%
- DISK_WRITE_MBPS    ███████░░░ 72%
- PCIE_RX_MB_S       ██████░░░░ 61%
- PCIE_TX_MB_S       [Runtime Discovered]
- BMD_INPUT_TOKENS   [Runtime Discovered] / [total] e.g. 3/3
- BMD_OUTPUT_TOKENS  [Runtime Discovered] / [total] e.g. 2/3
- DEVICE_EXCLUSIVITY OK / CONFLICT

> **重要:** 所有数字必须 Runtime Discovered 动态显示, **不**硬编码到 wireframe 样例。

**当前分配 (按 Channel):**
- CH01: Decode + Encode + Composition + Recording
- CH02: ...
- CH03: Idle

**Plan 容量:**
- 当前可用
- 如果新增 CH04: ?
- 触发 GC: 释放 X GB

### 状态模型
- Normal: 资源 < 80%
- Warning: 80-90%
- Error: 90-95% (新 Channel 会被拒)
- Critical: > 95% / OOM 风险

---

## E-37 · Clock 时钟

| 维度 | 定义 |
|---|---|
| **目标** | 时钟参考管理 (PTP / TIMECODE / SYSTEM), Fallback Chain |
| **主要操作** | Lock (L1) / Set Reference (L2 — 影响全部引用 Channel, 需 Impact Preview §24.2) / Test (L1) / Fallback Trigger (L3 — 影响全部 Channel, 必须审计, 见 §25.2 与 A-54) |
| **权限** | R: 全部 · W: Engineer · A: Admin |

### 信息架构

**Current Reference:** PTP · ptp0 (eth0) · LOCKED · BROADCAST_GRADE
**Metrics:** Offset / Drift / Path Delay
**Fallback Chain:** PTP (active) → TIMECODE → SYSTEM → MONOTONIC
**历史事件 (Canonical Vocabulary — V0.2 锁定):**
- `CLOCK_DEGRADED` — 漂移 / 精度降级 (BROADCAST_GRADE → GOOD/FAIR/POOR)
- `CLOCK_FAILED` — 失锁 / 不可用 (LOCKED → LOST)
- `CLOCK_FALLBACK_TRIGGERED` — Fallback Chain 切换事件

> **注意:** `CLOCK_LOST` 不在 Canonical Vocabulary 中, 统一为 `CLOCK_FAILED`。

---

# 7. 05 Operations 运维工作域 (Operator / Engineer)

## 工作域概述

```
Operations
  ├─ Health Tree (O-41)              ← 0.5A #09 (LOCK FINAL)
  ├─ Alerts / Incident Center (O-42) ← 新增
  ├─ Incident Timeline (O-43)        ← 新增
  ├─ Replay (O-44)                   ← 0.5A #07 子页 (LOCK FINAL)
  └─ Benchmarks (O-45)               ← 新增
```

**主要角色:** Operator (read + ack) / Engineer (read + manage)
**关联架构对象:** `incidents / alert_rules / alert_events / failover_benchmarks / latency_probes / signal_pool / signal_current_state`

---

## O-41 · Health Tree 健康树 *(= 0.5A #09)*

**状态:** 🟢 Phase 0.5A LOCK FINAL — 见 [`operator/09-health-tree.html`](operator/09-health-tree.html)

---

## O-42 · Alerts / Incident Center 告警事件中心

| 维度 | 定义 |
|---|---|
| **目标** | Alert Rule 配置 + 实时 Alert Events + 升级到 Incident |
| **主要操作** | View / Ack / Silence / Create Rule |
| **权限** | R: 全部 · W: Operator+ (Ack) / Engineer+ (Rule) · A: Admin |
| **关联工作流** | X4 Incident Timeline + Chain 2 Failure |

### 信息架构

**3 层 Tab:**

#### Alert Rules
- 字段: Rule Name · Trigger Condition (e.g. SRT packet loss > 0.5% for 5s) · Severity · Escalation · **Auto Action (从 §8.9 继承, 不可覆盖)**
- 操作: `[+ Create Rule]` · `[Test]` · `[Edit]` · `[Delete]`

**Auto Action 关键约束 (V0.2 §8.9 锁定 — UI 不可覆盖):**

> V0.2 §8.9 Failure Domain Matrix 是 **Recovery Policy SoT**。
> Alert Rule 不能让 Admin 任意设置"切主备"。

正确流程 (UI 必须按此实现):

```
Alert Rule 触发
    ↓
Diagnostic Classification
    ↓
OperationalFailureDomain (SOURCE / PIPELINE / MASTER / OUTPUT / RECORDING / CLOCK / RESOURCE)
    ↓
§8.9 Recovery Policy
    ↓
Allowed Action (由 §8.9 决定, 不可由 Alert Rule 改写)
```

UI 表现:

```
Auto Action
[ Inherited from §8.9 Failure Domain Policy ▼ ]

Current Domain: SOURCE
Allowed Actions (read-only):
  ✓ FAILOVER
  ✓ NOTIFY
  ✗ RESTART_ADAPTER  (不允许, 由 §8.9 决定)
  ✗ DISABLE_OUTPUT   (不允许)
```

**禁止 UI 反模式:**
- ❌ Alert Rule 任意设置 "切主备" 复选框
- ❌ Admin 覆盖 §8.9 决策
- ❌ Alert Rule 包含 `ACTION: FAILOVER` 字符串 (应是 Diagnostic → §8.9 链路)

**DiagnosticFailureClass 单独处理 (不进 7 OperationalFailureDomain):**
- PLAYER: NOTIFY only, fail_safe=true
- UNKNOWN: SAFE_DEGRADE, alert=true
- **这两类不触发 Failover**, UI 必须显式标 "[Player-side / Diagnostic only, no failover]"

#### Active Alerts
- 列表: Alert ID · Severity · Source · Description · First Seen · Last Seen · Status
- 操作: `[Ack]` (L1) · `[Silence 1h]` (L2) · `[Open Incident]` (L2) · `[Open Health Tree]`

#### Incidents
- 列表: Incident ID · Status · Channel · Started · Duration · Acked By
- 操作: `[View Timeline]` (O-43) · `[Open Replay]` (O-44) · `[Resolve]`

### 状态模型
- Normal: 0 active alerts
- Warning: 1-5 active
- Error: 5-20 active
- Critical: > 20 active / 多 channel 同时 FAILED

---

## O-43 · Incident Timeline 事件时间线

| 维度 | 定义 |
|---|---|
| **目标** | 单个 Incident 完整时间线 + 跨 Incident 横向对比 |
| **主要操作** | View / Filter / Export / Link to Replay |
| **权限** | R: 全部 · W: Engineer (Edit Notes) · A: Admin |

### 信息架构

**时间线视图 (垂直):**

```
14:25:18 ──────────────────
14:25:21 │ Source.B 冻结 5s
14:25:21 │    QC ALERT
14:25:21 │    Health Tree: Source.Primary ACTIVE+FAILED
14:25:21 │    Aggregation Rule 1 → FAILED
14:25:26 │ Switch Decision Tree: FRAME_SWITCH
14:25:26 │    Auto Failover → Source.B STANDBY→ACTIVE
14:25:27 │    Filler 兜底 (按 §8.9 Safety Policy)
14:25:27 │ Operator ALERT 推送
14:25:30 │ Incident #1248 自动建档
14:25:30 │ Recording 继续 (Chunk 完整)
14:25:45 │ Operator Ack
14:26:00 ──────────────────
```

**操作:** `[Filter by Subsystem]` · `[Add Note]` · `[Open Replay]` · `[Export JSON]`

---

## O-44 · Replay 回放 *(= 0.5A #07 子页)*

**状态:** 🟢 0.5B 新定义的独立表面（继承 0.5A #07 Replay 子区语义; wireframe 已随 0.5A #07 LOCK, 不需要 0.5E 重画）— Incident → Replay 自动定位工作流

---

## O-45 · Benchmarks 基准测试结果

| 维度 | 定义 |
|---|---|
| **目标** | Failover Benchmarks / Latency Probes / QC Throughput 等性能基准 |
| **主要操作** | View / Export / Compare |
| **权限** | R: 全部 · W: Engineer (Re-run) · A: Admin |

### 信息架构

**3 区:**

#### Failover Benchmarks
- Channel · Switch Mode · p50 / p95 / p99 (ms) · Last Updated
- 与 target_failover_time_ms 对比 (PASS/FAIL)

#### Latency Probes
- 7 Core + 2 Client + 1 CDN
- 当前 measured vs budget

#### QC Throughput
- QC 任务数 / sec
- QC 延迟分布
- 与 SLA 对比

---

# 8. 06 Administration 平台管理工作域 (Admin)

## 工作域概述

```
Administration
  ├─ Users (A-51)               ← 用户管理
  ├─ Roles (A-52)               ← 角色管理
  ├─ Permissions (A-53)         ← 权限矩阵
  ├─ Audit Logs (A-54)          ← 审计日志
  └─ System Settings (A-55)     ← 系统设置
```

**主要角色:** Admin (唯一) · Engineer (R 自己 profile)
**关联架构对象:** `users / roles / permissions / user_roles / audit_logs / api_keys / sessions / oauth_tokens / system_settings`

---

## A-51 · Users 用户

| 维度 | 定义 |
|---|---|
| **目标** | 集中管理所有用户账号 |
| **主要操作** | Create / Edit / Disable / Reset Password / Assign Role |
| **权限** | R: Admin · W: Admin · A: Admin |

### 信息架构

**列表字段:** Username · Email · Display Name · Roles · Status (Active/Disabled/Locked) · Last Login · 2FA

**详情:**
- Basic Info (Username / Email / Display Name)
- Auth (Password / 2FA / OAuth Provider)
- Roles (多选: Operator / Director / Engineer / Admin)
- Sessions (当前活跃 session)
- API Keys (关联 A-51 的子表)
- Audit (这个 user 的所有操作历史)

**操作:** `[+ Create User]` · `[Edit]` · `[Disable]` (L2) · `[Reset Password]` (L2) · `[Assign Role]` · `[Delete]` (L3)

### 状态模型
- Normal: 全部 Active
- Warning: 某用户 90 天未登录
- Error: 某用户 Locked (失败登录 > 5)
- Critical: Admin 账号被禁用 (系统无法管理)

---

## A-52 · Roles 角色

| 维度 | 定义 |
|---|---|
| **目标** | 角色管理 (V0.2 锁 4 角色: Operator / Director / Engineer / Admin) |
| **主要操作** | View / Edit (Phase 4 后允许自定义角色) |
| **权限** | R: Admin · W: Admin (V0.2 仅 4 内置) |

**列表:** Role Name · User Count · Default Permissions
**详情:** Permissions (从 A-53 选) · Users (引用 A-51) · Description

**V0.2 约束:** 角色**不可新建**, 4 角色 LOCK FINAL; 只能调整 Permission 关联

---

## A-53 · Permissions 权限矩阵

| 维度 | 定义 |
|---|---|
| **目标** | 集中查看/编辑"角色 × 工作域 × 操作" 权限矩阵 |
| **主要操作** | View / Toggle Permission |
| **权限** | R: Admin · W: Admin |

### 信息架构

**矩阵表格:**

| 操作 | Operator | Director | Engineer | Admin |
|---|---|---|---|---|
| **01 Broadcast** | | | | |
| 切播 TAKE | R+W | R+W | R+W | A |
| FAILOVER | R | R | R+W | A |
| 启停 Channel | R | R | R+W | A |
| **02 Media** | | | | |
| Upload Asset | — | R+W | R+W | A |
| Delete Asset | — | — | R+W | A |
| Transcode Create | R | R+W | R+W | A |
| **03 Profiles** | | | | |
| Edit Profile (P-21/P-22/P-23/P-25/P-27) | — | — | R+W | A |
| Edit Profile (P-24 Graphic / P-26 Rights) | — | R+W | R+W | A |
| Delete Profile | — | — | — | A |
| **04 Engineering** | | | | |
| Edit Graph | — | R | R+W | A |
| Apply Change Set | — | — | R+W | A |
| Lock Device | — | — | R+W | A |
| **05 Operations** | | | | |
| Ack Alert | R+W | R | R+W | A |
| Resolve Incident | — | — | R+W | A |
| Re-run Benchmark | — | — | R+W | A |
| **06 Administration** | | | | |
| User CRUD | — | — | — | A |
| Role Management | — | — | — | A |
| System Settings | — | — | — | A |

**操作:** `[Toggle]` (L2) · `[Export Matrix]`

---

## A-54 · Audit Logs 审计日志

| 维度 | 定义 |
|---|---|
| **目标** | 所有危险操作的不可篡改审计 (TAKE / FAILOVER / CHANGE SET / DELETE / etc.) |
| **主要操作** | View / Filter / Export / Verify Hash |
| **权限** | R: Admin · W: — (不可写, append-only) |

### 信息架构

**列表字段:** Timestamp · User · Action · Target · IP · Result · Hash Chain

**详情:** Full Action · Before/After · 关联 Change Set / Incident

**危险操作清单 (强制审计 — 0.5C.1 回写, 纳入 0.5B.2 新增 L3 操作):**
- TAKE / FAILOVER / DISABLE OUTPUT
- CHANGE SET APPLY / ROLLBACK
- DELETE Asset / Profile / Output
- EDIT Profile / Permission
- LOCK / UNLOCK Device
- USER CRUD
- RIGHTS OVERRIDE (L3, M-12 — Who/Why/Scope/Expiry/Audit Reference 五字段)
- ALERT SILENCE / ALERT RULE 修改 (O-42)
- FALLBACK TRIGGER (E-37 Clock — 影响全部 Channel, 见 §25.2)
- BATCH 批量操作 (M-11 批量转码/归档/删除)

**Hash Chain:** 每条 log 含前一条 hash, 保证不可篡改

**操作:** `[Filter]` (时间/user/action/target) · `[Export CSV]` · `[Verify Chain]` (L2) · `[View Diff]`

### 状态模型
- Normal: 日志持续 append
- Warning: 某 user 1 小时内操作 > 50 次
- Error: Hash Chain 验证 FAIL (检测篡改)
- Critical: Audit 表被 truncate (需要从 backup 恢复)

---

## A-55 · System Settings 系统设置

| 维度 | 定义 |
|---|---|
| **目标** | 平台级设置 (Clock / Storage / Workers / Prometheus / Notifications / Retention) |
| **主要操作** | View / Edit |
| **权限** | R: Engineer (R 自己的设置) · W: Admin |
| **关联工作流** | Clock fallback / Recording retention / Alert routing |

### 信息架构 (10 区)

#### General
- System Name · Timezone · Default Locale

#### Clock
- Primary Reference (PTP0 / TIMECODE / SYSTEM)
- Fallback Chain (4 节点)
- Sync Interval

#### Storage
- Default Path · Quota · Auto-archive · Retention

#### Recording
- Chunk Duration (5 min default)
- Retention (30 days default)
- Auto-cleanup

#### Workers
- Concurrency Limit · Health Check Interval

#### Prometheus
- Endpoint · Push Interval · Metrics Filter

#### Notifications
- Webhook URL · Email SMTP · Slack Channel

#### Retention
- Asset Retention · Log Retention · Backup Retention

#### Localization
- Default Language · Date Format · Time Format

#### Operator Preferences
- Default Dashboard Channel · Theme: **Dark (V0.1 锁定 — 24/7 机房; Light 为 V0.4+ 预留, 此处只读显示, 不可切换, 见 DESIGN_SYSTEM §9.2)**

**操作:** `[Edit]` (L2) · `[Reset to Default]` (L3) · `[Export Config]`

---

# 9. Navigation Graph 跳转关系

## 9.1 顶部菜单 (6 大工作域)

```
VBMF Console
├─ 01 Broadcast    ← 0.5A 9 Core (LOCK FINAL)
├─ 02 Media        ← M-11~16
├─ 03 Profiles     ← P-21~27
├─ 04 Engineering  ← E-31~37 (含 0.5A Graph Designer)
├─ 05 Operations   ← O-41~45 (含 0.5A Health Tree)
└─ 06 Administration ← A-51~55
+ 10 States (Validation, 不在主菜单)
```

## 9.2 跨工作域跳转示例

| 源页面 | 跳转 | 目标 |
|---|---|---|
| Dashboard | Channel 缩略图点击 | Sources (02) |
| Dashboard / Output | Channel 名称点击 | **CD-01 Channel Detail (§17, 8 Tab)** |
| Sources | Source.A 配置 | Encoding Profile (P-21) |
| Switcher | TAKE 失败 | Health Tree (O-41) → Incident (O-43) |
| Output | HLS DEGRADED | Output Profile (P-22) |
| Recording | 点击 Incident | Replay (O-44) |
| Health Tree | Failed 节点 | Device Registry (E-35) |
| Composition | 拖入 Asset | Media Library (M-11) → Asset Detail (M-12) |
| Change Set | Impact 显示 | Resource / Capacity (E-36) |
| Asset Detail | Transcode 按钮 | Transcode Center (M-14) |
| Asset Detail | Used By → Channel | CD-01 Channel Detail |
| Transcode Center | Profile 选择 / Profile Diff | Encoding Profile (P-21) |
| Encoding Profile | Used By → Channel | CD-01 Channel Detail |
| Profile Editor | Codec 不可用 | Device Registry (E-35) |

## 9.3 抽屉 / 子页 (不计入主导航)

| 主页面 | 子页 |
|---|---|
| Media Library | Asset Detail (5 tab) |
| Transcode Center | Jobs / Versions |
| Asset Detail | Versions / QC / Rights / History (4 tab) |
| Dashboard / Output / Sources | CD-01 Channel Detail (8 tab, §17) |
| Health Tree | Operator / Engineering / Aggregation Rules (3 view) |
| Output | HLS Detail / WebRTC Detail (3 view) |
| Composition | Timeline / Composition (2 column) |
| Switcher | TAKE State Machine (5 状态 modal) |
| Incident | Replay Workspace |

---

# 10. 实施顺序 (P0 / P1 / P2 / Defer)

## 10.1 Phase 0.5B 内部优先级 (0.5C.1 回写实际交付状态)

| 优先级 | UI 表面 | 原因 | 状态 |
|---|---|---|---|
| 🔴 **P0 必做** | M-11 (Library) + M-12 (Detail) + M-14 (Transcode) + P-21 (Encoding) + P-22 (Output) | 这些是 V0.2 核心架构对象, 缺它们 Phase 1 实施会不断回头问"放哪里" | ✅ Spec + wireframe 均已交付 (0.5B.1) |
| 🟠 **P1 强烈建议** | CD-01 (Channel Detail §17) + M-13 (Upload) + M-15 (Jobs) + M-16 (Versions) + P-23~27 (其他 Profile) + E-32~37 (工程) | 让 Phase 1 / 4 有完整可参考 UI 表面 | ✅ Spec 已锁定; wireframe 0.5D+ (CD-01 含内) |
| 🟡 **P2 锦上添花** | O-42 / O-43 / O-45 (Operations 后续) + A-51~55 (Admin) | 后期再做也来得及, Admin 可直接用 SQL 临时方案 (O-44 Replay 例外: 已随 0.5A #07 LOCK) | 📋 wireframe 0.5E+ / Phase 4 |
| ⚪ **Defer to Phase 4** | (无) | 0.5B 不实施 wireframe, 只定义; 实施在 Phase 4 | — |

## 10.2 与其他阶段衔接

```
Phase 0.5A (LOCK FINAL)
  ↓ 9 Core + 1 Validation
Phase 0.5B (当前) — Surface Spec
  ↓ 30+ Product UI 表面定义
Phase 0.5B.1 (下一轮)
  ↓ 选 P0 5 页做 wireframe (M-11/12/14 + P-21/22)
Phase 0.6
  ↓ Executable Acceptance Spec (基于 0.5A + 0.5B)
Phase 1
  ↓ Media Core (Rust) 后端实现
Phase 4
  ↓ Web Console 前端实现, 0.5A + 0.5B 全部 wireframe 落地
```

---

# 11. 架构对象映射总表 (V0.2 → UI 表面) — 完整 Exposure Matrix

> **V0.2 架构对象** 必须明确分类到 **4 个 Exposure Level**:
>
> | Level | 含义 | UI 形式 |
> |---|---|---|
> | **DIRECT** | 用户直接 CRUD 的对象 | 独立 UI 表面 / 详情页 |
> | **INDIRECT** | UI 间接显示/引用, 但不能直接编辑 | 选择器 / Inspector / 引用视图 |
> | **SYSTEM_INTERNAL** | 系统内部状态, UI 显示 read-only, 不让用户改 | 监控面板 / Status 字段 |
> | **NON_UI** | 完全不暴露到 UI (DB / API 内部使用) | 仅 DB / API |

> **重要:** 之前规范"1:1 映射"过强, 实际不是所有对象都需要独立 UI 表面。
> 现在按 4-Level 分类, 避免 Phase 4 实施返工。

## 11.1 02 Media 工作域 对象映射

| 架构对象 | Exposure | UI 表面 | 备注 |
|---|---|---|---|
| `media_assets` | **DIRECT** | M-11, M-12, M-13 | CRUD |
| `asset_versions` | **DIRECT** | M-12b (Versions Tab), M-16 | CRUD + 列表 |
| `asset_rights` | **DIRECT** | M-12d (Rights Tab) | 引用 P-26 |
| `media_jobs` | **DIRECT** | M-14, M-15 | CRUD + 监控 |
| `media_job_attempts` | **DIRECT** | M-15 (子表) | 重试历史 |
| `uploads` | **SYSTEM_INTERNAL** | M-13 (进度显示) | UI 监控, 不直接编辑 |
| `upload_chunks` | **NON_UI** | — | DB 内部 |
| `upload_jobs` | **SYSTEM_INTERNAL** | M-13 (进度) | — |

## 11.2 03 Profiles 工作域 对象映射

| 架构对象 | Exposure | UI 表面 | 备注 |
|---|---|---|---|
| `encoding_profiles` | **DIRECT** | P-21 | CRUD |
| `output_profiles` | **DIRECT** | P-22 | CRUD · 与 Output Variant 分离 |
| `audio_profiles` | **DIRECT** | P-23 | CRUD |
| `graphic_profiles` | **DIRECT** | P-24 | CRUD · 含 composition_templates |
| `qc_profiles` | **DIRECT** | P-25 | CRUD |
| `rights_profiles` | **DIRECT** | P-26 | CRUD |
| `edge_policy_profiles` | **DIRECT** | P-27 | CRUD |
| `composition_templates` | **DIRECT** | P-24 | 与 graphic_profiles 合并表 |
| `composition_layers` | **DIRECT** | P-24 (子表) | Layer 列表 |
| `playlists` | **DIRECT** | 0.5A #04 (Timeline) | — |

## 11.3 04 Engineering 工作域 对象映射

| 架构对象 | Exposure | UI 表面 | 备注 |
|---|---|---|---|
| `graph_specs` | **DIRECT** | E-31 (Graph Designer) | 用户画图 |
| `graph_revisions` | **DIRECT** | E-33 (Change Sets) | 版本管理 |
| `graph_runtimes` | **SYSTEM_INTERNAL** | E-31 (COMPILED Tab) | 编译产物, read-only |
| `graph_runtime_nodes` | **SYSTEM_INTERNAL** | E-31 (COMPILED Tab) | 节点实例 |
| `graph_runtime_edges` | **SYSTEM_INTERNAL** | E-31 (Edge Inspector) | 边实例 |
| `signal_contracts` | **DIRECT** | E-34 (Capability Registry) | 列表 / 对比 |
| `node_contracts` | **DIRECT** | E-34 (子表) | — |
| `preflight_runs` | **DIRECT** | E-32 (Preflight Center) | 历史 + 报告 |
| `config_revisions` | **DIRECT** | E-33 (Change Sets) | — |
| `change_sets` | **DIRECT** | E-33 (Change Sets) | Business Status |
| `change_set_items` | **DIRECT** | E-33 (详情) | — |
| `change_set_events` | **DIRECT** | E-33 (Execution Phase) | 与 Business Status 分离 |
| `device_registry` | **DIRECT** | E-35 (Device Registry) | 列表 + 详情 |
| `device_locks` | **DIRECT** | E-35 (Lock 状态) | — |
| `device_health_history` | **DIRECT** | E-35 (历史) | — |
| `media_devices` | **DIRECT** | E-35 (按 Host 分组) | — |
| `hardware_capability` | **DIRECT** | E-35, E-36 | 9-Dim Resource Vector |
| `clock_fallback_chain` | **DIRECT** | E-37 (Clock) | — |
| `latency_budgets` | **INDIRECT** | P-27 (引用) | 配置在 Edge Policy Profile |
| `clock_domain_mappings` | **INDIRECT** | 0.5A #02 (Sources 时钟列) | — |

## 11.4 05 Operations 工作域 对象映射

| 架构对象 | Exposure | UI 表面 | 备注 |
|---|---|---|---|
| `health_trees` | **DIRECT** | 0.5A #09 (Health Tree) | 树形 |
| `current_health_trees` | **SYSTEM_INTERNAL** | 0.5A #09 (Engineering View) | 实时快照 |
| `health_tree_nodes` | **SYSTEM_INTERNAL** | 0.5A #09 (节点状态) | — |
| `channel_health_aggregation` | **SYSTEM_INTERNAL** | 0.5A #09 (Aggregation View) | SQL 7 规则 |
| `channel_health_view` | **DIRECT** | 0.5A #01, #09 | Channel Status 唯一入口 |
| `incidents` | **DIRECT** | O-42, O-43 | CRUD (主要系统建, 人工 Ack) |
| `alert_rules` | **DIRECT** | O-42 (Alert Rules Tab) | CRUD · Auto Action 来自 §8.9 |
| `alert_events` | **DIRECT** | O-42 (Active Alerts) | 列表 |
| `failover_benchmarks` | **DIRECT** | O-45 | 历史 + 对比 target |
| `latency_probes` | **DIRECT** | O-45, 0.5A #06 | 7 Core + 2 Client + 1 CDN |
| `signal_pool` | **NON_UI** | — | Prometheus 内部时序 |
| `signal_current_state` | **SYSTEM_INTERNAL** | 0.5A #01, #09 | read-only |
| `avsync_measurements` | **DIRECT** | 0.5A #05 (Audio 安全区), O-45 | — |

## 11.5 06 Administration 工作域 对象映射

| 架构对象 | Exposure | UI 表面 | 备注 |
|---|---|---|---|
| `users` | **DIRECT** | A-51 | CRUD |
| `roles` | **DIRECT** | A-52 | 4 内置 (V0.2 锁定) |
| `permissions` | **DIRECT** | A-53 (Permission Matrix) | 角色 × 操作 |
| `user_roles` | **DIRECT** | A-51, A-52 (多对多关联) | — |
| `audit_logs` | **DIRECT** | A-54 | append-only · hash chain |
| `api_keys` | **DIRECT** | A-51 (子表) | — |
| `sessions` | **SYSTEM_INTERNAL** | A-51 (子表, 当前活跃) | — |
| `oauth_tokens` | **SYSTEM_INTERNAL** | A-51 (子表) | — |
| `system_settings` | **DIRECT** | A-55 | 10 区 |

## 11.6 01 Broadcast / Channel 工作域 对象映射

| 架构对象 | Exposure | UI 表面 | 备注 |
|---|---|---|---|
| `channels` | **DIRECT** | 0.5A #01, #03, #06 | CRUD |
| `media_sessions` | **DIRECT** | 0.5A #01, 10-states | — |
| `media_session_attempts` | **SYSTEM_INTERNAL** | 0.5A #01 (status 细节) | — |
| `media_session_runtime` | **SYSTEM_INTERNAL** | 0.5A #01, 10-states | read-only · 三轴状态 |
| `channel_routes` | **DIRECT** | 0.5A #03, E-31 (Channel Config) | 引用 Output Variant |
| `output_variants` | **DIRECT** | E-31 (Channel Output 节点) | 与 P-22 分离 |
| `output_destinations` | **DIRECT** | E-31 (Channel Output 子页) | — |
| `switch_modes` | **INDIRECT** | 0.5A #03 (选择器) | 3 模式 enum |
| `hot_standby_levels` | **INDIRECT** | 0.5A #03 (Channel Config) | 3 级 enum · Policy/Target |
| `recordings` | **DIRECT** | 0.5A #07 | — |
| `recording_segments` | **DIRECT** | 0.5A #07 (Chunk 列表) | — |

## 11.7 NON_UI 对象 (不暴露 UI)

| 架构对象 | 用途 |
|---|---|
| `upload_chunks` | DB 内部, 用于断点续传 |
| `signal_pool` | Prometheus 时序数据 |
| `internal_audit_meta` | 内部元数据 |
| `migration_history` | DB migration 记录 |
| `feature_flags` | 内部 feature flag |

## 11.8 计数

| Exposure Level | 对象数 | UI 表面影响 |
|---|---|---|
| DIRECT | ~32 | 独立 UI 表面或详情 Tab |
| INDIRECT | ~5 | 引用 / 选择器 |
| SYSTEM_INTERNAL | ~10 | 监控面板 / read-only |
| NON_UI | ~5+ | 不暴露 |

**总计 V0.2 架构对象:** ~52 个, 分布在 11.1-11.7 章节

**0.5A + 0.5B 实施原则:**
- DIRECT 对象**必须**有 UI 入口 (可能是页面/Tab/选择器)
- INDIRECT 对象**必须**有引用入口 (选择器/Inspector)
- SYSTEM_INTERNAL 对象**不**让用户编辑, 但监控面板**应该**显示
- NON_UI 对象**完全**不暴露

## 11.9 与 V0.2 §3.11 关联

V0.2 §3.11 Resource Vector 锁定的 9-Dim:

```
CPU_THREADS / GPU_SESSIONS / VRAM_MB / RAM_MB /
NIC_INGRESS_MBPS / NIC_EGRESS_MBPS / DISK_WRITE_MBPS /
PCIE_RX_MB_S / PCIE_TX_MB_S
+ BMD_INPUT_TOKENS / BMD_OUTPUT_TOKENS / DEVICE_EXCLUSIVITY
```

E-35 / E-36 必须完整呈现这 9 维 + Device Tokens, 不能简化。

---

# 12. 与 Phase 0.5A 验收关系

Phase 0.5A 锁定的 9 Core + 1 Validation 全部在 0.5B 中**保持原状**:

- 01 Dashboard → 01 Broadcast 入口
- 02 Sources → 01 Broadcast
- 03 Switcher → 01 Broadcast
- 04 Composition → 01 Broadcast
- 05 Audio → 01 Broadcast
- 06 Output → 01 Broadcast
- 07 Recording → 01 Broadcast
- 08 Graph Designer → 04 Engineering
- 09 Health Tree → 05 Operations
- 10 States → Validation Reference

**0.5A 不变, 0.5B 是新增与扩展。**

---

# 13. 经验教训 (0.5A → 0.5B 演进)

1. **架构对象必须按 4-Level Exposure 分类** (DIRECT/INDIRECT/SYSTEM_INTERNAL/NON_UI) — 不是 1:1 映射, 也不是全无映射
2. **Output Profile / Output Variant / Output Destination 三个概念必须分离** — V0.2 §3.7.1 锁定, UI 必区分
3. **Profile 必须与 Runtime 分离** — Encoding Profile ≠ Output Profile ≠ Channel; UI 也要分
4. **P-21 字段必须达到广播级** (SAR / Field Order / Color Space / HRD / Closed GOP / Reference Frames / Audio Layout / Bit Depth) — 缺这些 Capability Contract 失效
5. **Hardware Encoder 必须是 Runtime Discovery 驱动** — UI 不能假定硬件存在, 选项来自 E-35 动态发现
6. **P-22 必须区分 V0.2 Supported vs Reserved/Future** — DASH/DRM/SDI Master 等 Reserved 不能诱导用户配置
7. **资源必须显示完整 9-Dim Resource Vector** — 不能简化为"CPU/RAM/NIC" 三项
8. **Wireframe 样例必须用 [Sample Host] / [Runtime Discovered]** — 不能硬编码 10.30.15.10 / 32 核 / 30 GB 等实机器参数
9. **E-37 Clock 事件 vocabulary 统一** — 用 CLOCK_DEGRADED / CLOCK_FAILED, 不写 CLOCK_LOST
10. **E-33 ChangeSet Business Status 与 Execution Phase 必须分离** — ABORTED 是 Phase, 不是 Status
11. **Alert Rule Auto Action 必须从 §8.9 Failure Domain Policy 继承** — UI 不可让 Admin 任意设置"切主备"
12. **危险操作必须 L1/L2/L3 分级** — 不能所有操作都同等对待
13. **6 状态样例 (Normal/Loading/Empty/Error/Warning/Critical) 必须每页都有** — 缺一视为不完整
14. **审计是 Admin 的第一公民** — 不是"以后再加"
15. **资源容量必须有 Plan 视图** — 不仅"现在用了多少", 还要"如果新增 X 会怎样"
16. **i18n 必须有正式 Contract** — 见 [`I18N_SPEC.md`](I18N_SPEC.md), 不能再用 "HEALTHY 健康" 这种 hard-coded 字符串

---

---

# 14. Phase 0.5B Closure-1 — 10 项产品化收口

> **Phase 0.5B.0 锁定 13 P0 语义边界 (commit 50cf5a6)**
>
> **Phase 0.5B Closure-1 (本节) — 在 0.5B.0 基础上做一轮产品化收口**
>
> 重点:
> - 不是再加一级页面
> - 而是把"对象关系 / Runtime vs Config / 影响面 / 解释能力 / 视觉层级"做到产品级
> - 完成后才正式冻结 0.5B, 然后进入 0.5B.1 五张 P0 wireframe

本节覆盖 10 个 Closure Item:
1. **§15** Configuration / Compiled / Effective 3-Layer Model (全局 pattern)
2. **§16** VBMF Design System (4 套状态语义分离 + 颜色系统)
3. **§17** Channel Detail (CD-01) — 新增子页
4. **§18** P-21 Profile Builder 10 sections + Preset + Why Not Usable
5. **§19** M-14 Transcode Workflow (Preview → Test → Submit) + Worker=AUTO + Result
6. **§20** P-22 Output 4-tuple (Profile/Variant/Destination/Adapter) + 3-tier Available/Reserved/Unavailable
7. **§21** E-32 Preflight 9D Required/Available/Delta/Headroom
8. **§22** O-41 Health Tree H1-H7 + Failure Absorbed + redundancy_group 视觉化
9. **§23** E-34 Capability Why Compatible / Why Not + Static vs Runtime
10. **§24** Dependency / Impact Preview 全局 pattern

---

# 15. Configuration / Compiled / Effective 3-Layer Model (全局 pattern — 锁定)

> **这是整个 UI 的核心 UX 基础设施。**
> 凡是有"配置"的页面 (Profile / Channel / Switch Mode / Output / ChangeSet), 都必须按这个 3 层模式显示。

## 15.1 为什么需要 3 层

V0.2 架构已经明确区分 Configuration (X3 config_revisions) / Compiled Runtime (graph_runtimes) / Effective Runtime (media_session_runtime)。这是不同时间点的状态:

| 层 | 含义 | 写入者 | 例子 |
|---|---|---|---|
| **DESIRED** | 用户配置 | 用户 / Engineer | Encoding Profile v3 (Broadcast HEVC 1080p25) |
| **COMPILED** | Compiler 编译产物 | X1 Graph Compiler | Profile 实例 + 自动插入的节点 (Normalize / Encode) |
| **EFFECTIVE** | 当前实际运行 | Runtime (X1 / X2 / X3) | 实际跑的 Encoder = x265, Worker = host-01 |

3 层可能**不同**:
- DESIRED = PACKET_SWITCH
- COMPILED = PACKET_SWITCH
- EFFECTIVE = FRAME_SWITCH (Runtime Alignment degraded → §3.4 自动降级)

## 15.2 全局 UI 模式 (锁定)

所有"配置型"页面右侧或顶部必须有 **Configuration Triangle** 组件:

```
┌─────────────────────────────────────┐
│  ◉ DESIRED                          │
│  Encoding Profile v3                │
│  HEVC Main10 1080p25                │
│  ─────────────────                  │
│  ⚙ COMPILED                         │
│  HEVC Main10 1080p25                │
│  Encoder: x265                      │
│  Worker: host-01                    │
│  ─────────────────                  │
│  ▶ EFFECTIVE                        │
│  HEVC Main10 1080p25                │
│  Encoder: x265                      │
│  Worker: host-01                    │
│  Uptime: 02:14:37                   │
│  Bitrate: 4.8 Mbps (target 5.0)    │
│  ─────────────────                  │
│  Δ: COMPILED == EFFECTIVE ✓         │
│  Reason: 无降级                      │
└─────────────────────────────────────┘
```

如果层之间有差异, 必须显式标 `Δ` + Reason:

```
Δ: COMPILED ≠ EFFECTIVE ⚠

Reason: Runtime Alignment degraded
→ Switch Mode 自动降级到 FRAME_SWITCH
→ §3.4 Decision Tree
→ 当前: CH01 在用 EFFECTIVE=FRAME
```

## 15.3 适用页面清单

| 页面 | DESIRED 字段 | COMPILED 字段 | EFFECTIVE 字段 |
|---|---|---|---|
| **P-21** Encoding Profile | Profile 定义 | Compile 后实例 + 自动插入的 Encoder | 当前 Worker / Encoder / 实时码率 |
| **P-22** Output Profile | Profile 定义 | Output Variant 实例 | Adapter 健康 + 实际推送 |
| **CD-01** Channel Detail | Hot-Standby / Switch Policy | Compiled Mode | Effective Mode (可能因 Runtime 降级) |
| **Switcher (0.5A #03)** | Compiled Switch Policy | Compiled Mode | Effective Mode + Δ Reason |
| **E-37** Clock | Primary Reference | Active Reference | Current Lock State + 漂移 |
| **E-33** ChangeSet | Pending Change | Apply 中状态 | 已应用结果 |

## 15.4 关键规则

- **3 层**任何时候都**同时可见** (不要折叠)
- **DESIRED 改了, 不会立即影响 COMPILED / EFFECTIVE**, 必须走 ChangeSet → Apply
- **COMPILED ≠ EFFECTIVE** 是**正常**的运行时现象, UI 不应该 hide 它, 应该**显式**标 Δ + Reason
- **Operator 不应直接改 EFFECTIVE** — EFFECTIVE 由系统推导, 不能 UI 写入

## 15.5 实施原则

- Phase 4 实施: 抽 `<ConfigurationTriangle />` 公共组件, 所有配置页右侧/顶部引用
- 0.5B.1 wireframe: 在 P-21 / M-14 / CD-01 三页首次落地
- 0.5A wireframe: Switcher 03 + Dashboard 01 已有部分 3 层 (Dashboard Channel Status panel), 后续小修

---

# 16. VBMF Design System (4 套状态语义 + 颜色系统)

> **V0.1 Design System 锁定。**
> 0.5A wireframe 当前每页自己定义组件, 不统一; 0.5B 开始统一。

## 16.1 4 组状态语义 (必须分离)

> 之前 §2.1 "6 状态样例" 是 **UI Surface State**。这只是 4 组状态之一。
> 整个 VBMF 有 **4 组不同维度的 State**, **不能混用**:
> (0.5C.1 注: [`DESIGN_SYSTEM.md` §1](DESIGN_SYSTEM.md) 将 Runtime 3 轴拆开列成 6 行 = Lifecycle / Readiness / Health — 两处口径一致: **4 组 = 6 个模型**, 只是粒度不同)

| 套 | 用途 | 枚举 | 例子 |
|---|---|---|---|
| **UI Surface State** | 页面本身的渲染状态 | NORMAL / LOADING / EMPTY / WARNING / ERROR / CRITICAL | 列表加载中, 列表为空, 列表报错 |
| **Runtime State** | 业务对象运行时 (3 轴) | Lifecycle × Readiness × Health | CH01: RUNNING / READY_TO_TAKE / HEALTHY |
| **Operational Role** | Health Tree 节点角色 | ACTIVE / STANDBY / OFFLINE | Source.A: ACTIVE; Source.B: STANDBY |
| **Effective Status** | Channel 对外唯一 status | HEALTHY / DEGRADED / FAILED / STARTING / STOPPED / UNKNOWN | channel_health_view.effective_channel_status |

**反模式:** 把 HealthState (HEALTHY/DEGRADED/FAILED) 和 Operational Role (ACTIVE/STANDBY/OFFLINE) 用同一组颜色。

## 16.2 颜色语义 (锁定)

### 16.2.1 UI Surface State 颜色 (页面级)

| 状态 | 颜色 | 用途 |
|---|---|---|
| NORMAL | (无特殊色) | 页面正常显示 |
| LOADING | neutral grey | 加载中, Skeleton |
| EMPTY | neutral grey + 引导 | 空数据 + "新建" 主按钮 |
| WARNING | amber | 部分软指标越界 |
| ERROR | red | 操作失败 |
| CRITICAL | red + pulse | 业务中断 |

### 16.2.2 Runtime Health 颜色 (HealthState)

| 状态 | 颜色 | 用途 |
|---|---|---|
| HEALTHY | green | 健康 |
| DEGRADED | amber | 降级 |
| FAILED | red | 失败 |
| UNKNOWN | gray | 未知 (心跳丢失) |
| STARTING | blue | 启动中 (Policy: lifecycle=STARTING) |
| STOPPED | neutral / outline | 已停止 |

### 16.2.3 Operational Role 颜色 (ACTIVE/STANDBY/OFFLINE) — **必须独立**

| 状态 | 颜色 | 用途 |
|---|---|---|
| ACTIVE | solid blue | 当前在用 |
| STANDBY | outline blue / dashed | 热备/温备就绪 |
| OFFLINE | outline gray | 不可用 / 离线 |

> **关键:** ACTIVE 不用绿色 (避免与 HEALTHY 混淆), OFFLINE 不用红色 (避免与 FAILED 混淆)。

### 16.2.4 Lifecycle State 颜色 (辅助)

| 状态 | 颜色 | 用途 |
|---|---|---|
| RUNNING | solid | 正常运行 |
| STARTING | blue (animated) | 启动中 |
| STOPPING | gray (animated) | 停止中 |
| STOPPED | outline gray | 已停止 |

## 16.3 核心组件清单 (V0.1 锁定 — Phase 0.5B.1 / Phase 4 实施)

> **SoT 说明 (0.5C.1)**: 组件级**权威定义**以 [`DESIGN_SYSTEM.md` §6](DESIGN_SYSTEM.md)（20 个, 含 props/variants）为准; 本表为"组件 × 适用页面"的映射视角。两清单已对账: ConfigurationTriangle / ImpactPanel / PreflightPanel / DependencyGraph / ChannelStatusCard 已补入 DS §6.16-6.20。

| 组件 | 用途 | 必备字段 |
| `StatusBadge` | 显示 Runtime HealthState | state / reason / last_changed |
| `HealthDot` | 单点 Health 状态 | state / size / tooltip |
| `RuntimeStateChip` | Lifecycle + Readiness + Health 三轴合一 | lifecycle / readiness / health / uptime |
| `CapabilityChip` | Capability 验证结果 | state (PASS/WARN/FAIL) / reasons[] |
| `ResourceGauge` | 资源使用率 | required / available / delta / headroom / unit |
| `LatencyBadge` | Latency 测量 | target / measured / p50 / p95 / p99 |
| `RevisionBadge` | 资源版本 | revision / created_at / status |
| `ProfileBadge` | Profile 引用 | profile_id / version / type |
| `DangerLevelBadge` | 危险操作等级 | level (L1/L2/L3) / action |
| `ImpactPanel` | 影响预览 (见 §24) | scope / risk / estimate |
| `PreflightPanel` | Preflight 结果 (见 §21) | static / resource / runtime |
| `DiffViewer` | 差异对比 | before / after / highlights |
| `TimelineEvent` | 事件时间线单项 | timestamp / actor / action / detail |
| `DependencyGraph` | 依赖关系图 (见 §24) | node / edge / cycle_check |
| `ConfigurationTriangle` | 3 层 Config 视图 (见 §15) | desired / compiled / effective / delta_reason |
| `ChannelStatusCard` | Channel 综合状态 | 全部 3 套状态 + key metrics |

## 16.4 视觉系统总原则

1. **Broadcast 域 (Dashboard / Switcher / Output)**: 大状态 / 大数字 / 大预览 / 高优先级告警 / 低阅读负担
2. **Engineering 域 (Graph Designer / Health Tree Engineering / Preflight)**: 高信息密度 / 数据表 / Inspector / Graph / Diff / Capability / Resource
3. **两套视觉语言**自然形成, 不强行统一

---

# 17. Channel Detail (CD-01) — 新增子页

> **当前 UI 缺口里最大的一项。**
> Channel 是 V0.2 核心对象, 但目前没有一个真正的 "Channel Detail" 页面。
> Dashboard / Switcher / Composition / Audio / Output / Recording 都是"功能入口", 没有任何一个把一个 Channel 的所有配置 + runtime 串起来。

## 17.1 定位

- **不是一级菜单**
- **是 sub-page** / drawer, 从任何提到 Channel 的地方 (Dashboard / Switcher / Health Tree) 都能跳
- **入口位置:**
  - Dashboard Channel Selector 下拉 → "Channel Detail..."
  - Switcher 顶部 Channel 标签点击
  - Health Tree 节点 Source.CH01 → "Open Channel Detail"
  - Channel Routes 列表 → 行点击

## 17.2 信息架构 (8 区)

```
CD-01 Channel Detail — CH01 News HD
─────────────────────────────────────────

[ ◉ DESIRED  ⚙ COMPILED  ▶ EFFECTIVE ]  ← ConfigurationTriangle

[Tab Bar]
Overview | Signal | Switch | Master | Composition | Output | Recording | History
```

### Tab 1: Overview 总览

- Channel Name / Description / Tags
- Owner / Created At / Last Modified
- Status (ECHS): ● HEALTHY
- Uptime / Sessions Count
- 缩略图 (Program Preview)
- Key Metrics (1 屏信息密度):
  - Primary: Source.A · 1080p50
  - Backup: Source.B · 1080p25 · READY
  - Switch Mode: COMPILED=PACKET / EFFECTIVE=FRAME (Δ Reason)
  - Output: HLS 1,247 客户端 / RTMP OK / WebRTC DEGRADED
  - Recording: REC 2h14m / 77% used
  - Clock: PTP LOCKED BROADCAST_GRADE

### Tab 2: Signal 信号源

- **Signal RG (Redundancy Group):**
  - Primary (Source.A) — node_role=ACTIVE / STANDBY / OFFLINE + Capability Contract
  - Backup (Source.B) — node_role=ACTIVE / STANDBY / OFFLINE + Capability Contract
  - (可选) Tertiary (Source.C) — node_role=OFFLINE
- **Redundancy Group ID:** RG-CH01-SOURCE (可视化)
- 切换 Hysteresis 配置
- 失败吸收状态: ABSORBED ✓ (Source.A 失败时)

### Tab 3: Switch 切播

- **DESIRED:** Hot-Standby=HOT / SwitchPolicy=PACKET_FIRST
- **COMPILED:** PACKET_SWITCH / WARM_UP 100ms / FILLER 200ms
- **EFFECTIVE:** FRAME_SWITCH (Runtime Alignment degraded)
- **Δ Reason:** §3.4 Decision Tree 推导
- **Capability:** Primary vs Backup 对比 (Why PACKET? Why now FRAME?)
- 5 次历史切换事件

### Tab 4: Program Master 主母版

- Video: ● ACTIVE (RAW domain) · 3 graph (Composite / LowerThird / Subtitle)
- Audio: ● ACTIVE · -23 LUFS / -1.3 dBTP
- Metadata: ● ACTIVE
- 三独立 graph 同步状态
- AVSync Offset / Drift

### Tab 5: Composition 图文

- **Program Scope:** 1 个 Composition (固定)
- **Variant Scope:** 0~N 个 Variants
  - 国内版 / 海外版 / 存档版
- 当前 Active Variant
- Timeline (近 24h)

### Tab 6: Output 输出

- **Output Variants** 列表:
  - V-CH01-HLS · Profile=HLS-LIVE-01 · Destination=CDN-A (●) / CDN-B (备用)
  - V-CH01-RTMP · Profile=RTMP-PUSH-01 · Destination=Origin
  - V-CH01-File · Profile=ARCHIVE-01 · Destination=NFS-01
- **每个 Variant 的 DESIRED / COMPILED / EFFECTIVE 三角:**
  - Adapter health / Latency p95 / Clients / Reconnect count
- SDI Master Output: [Reserved · V0.4] (V0.2 DISABLED)

### Tab 7: Recording 录制

- Active Recording: 2h14m · 5min chunk · 750MB / chunk
- Storage: 77% used · 30 days retention
- 最近 5 个 Incident 关联 chunk
- [Open Recording Page] 跳 0.5A #07

### Tab 8: History 历史

- Configuration Revisions (5 条)
- Change Sets (5 条, 含 status)
- Incidents (10 条)
- Revisions 之间 Diff

## 17.3 状态模型

- Normal: 全部 tab 可用
- Loading: 8 tab 各自 Skeleton
- Empty: Channel 不存在 (进入路由)
- Warning: 某 tab 有 warning (例如 1 个 Output Variant DEGRADED)
- Error: Channel 启动失败 / HealthTree FAILED
- Critical: Channel 整体 FAILED

## 17.4 权限

- R: 全部 (可见自己 channel)
- W: Operator+ (本 channel) / Engineer+ (跨 channel)
- A: Admin

## 17.5 关联工作流

- Chain 1 (On-Air): 全程
- Chain 2 (Failure): Tab 2 / Tab 3 / Tab 8
- Chain 4 (Engineering): Tab 6 / Tab 7 / Tab 8

## 17.6 跳转

- 入口: Dashboard / Switcher / Health Tree 任意 Channel 引用
- 出口: 各 tab 跳到对应 0.5A 页面 (Recording / Output / Health Tree)
- 跳到 P-21/P-22 (Profile 详情)
- 跳到 E-33 (Change Set 详情)

---

# 18. P-21 Encoding Profile Builder — 10 Sections

> **重构原 P-21:** 从"大表单"改为 **Profile Builder** 模式。
> 原因: 广播工程师不会每次手工填全部 30+ 字段, 必须有 Preset + Step-by-Step。

## 18.1 整体结构 (10 Sections — 锁定)

```
P-21 Encoding Profile Builder
──────────────────────────────────

[ Header: Profile Name + Version + Status ]

[ Tab Bar ]
  1. Overview     5. Encoder     9. Validation
  2. Video        6. Audio       10. Revision
  3. Rate Control 7. Container
  4. GOP / Frame  8. Resource
```

### Section 1: Overview
- Profile Name / Description / Category / Tags
- Quick Summary (主要参数一览)
- 最近修改 / 创建者

### Section 2: Video
- Codec / Profile / Level / Pixel Format
- Resolution / FPS / Time Base
- **SAR (Pixel Aspect Ratio)** / **Field Order**
- **Color Space / Color Range / Color Transfer / Color Primaries / Color Metadata**

### Section 3: Rate Control
- Mode (CBR / VBR / Capped VBR)
- Bitrate / VBV Maxrate / VBV Buffer / **HRD**
- Min / Max Bitrate / Quality (CRF)

### Section 4: GOP / Frame
- GOP Size / **Closed vs Open GOP** / Keyframe Policy
- **Reference Frames** / B-Frames / Lookahead
- Scene Cut Detection

### Section 5: Encoder (Runtime Discovery 驱动)
- Encoder Engine (Auto / libx264 / libx265 / NVENC / QSV / BMD H.264)
- Available 列表 (动态) + ✗ 标注不可用 + 原因
- Preset / Tune / Threads / Latency Mode

### Section 6: Audio
- Codec / Sample Rate / **Channel Layout** / **Bit Depth**
- Bitrate / Loudness Reference / AV Sync Offset

### Section 7: Container / Transport
- Container (MPEG-TS / fMP4 / MP4 / MOV / MKV)
- Segment Duration / Index Mode
- Metadata / Timecode Policy
- Side Data (SEI / HDR)

### Section 8: Resource (E-32 联动)
- 预估 CPU / RAM / VRAM / Disk / PCIe
- Resource Vector 9D (见 §21)
- BMD 端口占用 (Input/Output)

### Section 9: Validation
- Static (字段一致 / Capability)
- Resource (服务器端能力)
- Runtime (Worker / Network / Clock)
- Test Encode 结果 (5s 试跑)

### Section 10: Revision
- Version (auto-increment)
- Change Notes
- Created By / At
- Status (DRAFT / ACTIVE / DEPRECATED)
- Diff vs Previous
- Used By (Channels / Variants / Sessions)
- [Open Change Set]

## 18.2 Preset / Template (新增)

> **重要:** 广播工程师不会每次填全部 30+ 字段。

进入 Create Profile 时:

```
Create Encoding Profile
────────────────────────
Start From Preset:
  ○ Broadcast 1080p25 H.264 (标准广播)
  ○ Broadcast 1080p25 HEVC (高效)
  ○ Low Latency H.264 (低延迟)
  ○ Archive Master (归档)
  ○ Proxy 720p (代理)
  ○ Mobile H.264 (移动端)
  ● Custom (自定义)
```

选择 Preset 后自动填充 9 个 section, Engineer 微调即可。

## 18.3 "Why Not Usable" 解释能力 (新增)

> 当前 Validation 只显示 "❌ FAIL" 是不够的。

当 Profile 不可用时, 详细列出:

```
❌ NOT USABLE

Reason 1: x265 encoder unavailable
  × libx265 (N/A · not in PATH)

Reason 2: GPU unavailable
  × NVENC (GPU unavailable)

Reason 3: 10-bit encoder required
  Selected: x265 8bit
  Required: HEVC Main10 (10-bit)
  Available: HEVC Main (8-bit only)

Reason 4: Output Variant requires H.264
  Profile produces: HEVC
  Required: H.264
```

操作按钮:
- [Fix Automatically] (建议修复)
- [Explain] (详细解释)
- [Open Capability Registry] (跳 E-34)
- [Open Device] (跳 E-35)

## 18.4 实施原则

- P-21 wireframe (0.5B.1) 完整实施 10 sections + Preset + Why Not Usable
- 0.5A 旧 P-21 表单 (5 sections) 在 Phase 4 实施时迁移

---

# 19. M-14 Transcode Workflow — Preview / Test / Submit

> **重构 M-14:** 从"任务列表 + 单 New Job Modal" 改为 **3 步工作流 (Preview / Test / Submit) + Worker=AUTO**。

## 19.1 核心修改: Worker 默认 AUTO

**当前问题:** Worker Assignment 让用户主导, 实际上用户在干 Scheduler 的工作。

**修复:** M-14 默认 Worker = AUTO, 由 Resource Scheduler 决定:

```
Worker
[ AUTO ▼ ]  ← 默认

Scheduler 会基于以下选择:
  Resource available (CPU/GPU/RAM/Disk)
  Queue depth
  Capability (encoder availability)

Advanced → Manual Worker
  Worker-01
  Worker-02
  Host-10.30.15.10
```

## 19.2 5 步 New Job Workflow (替代单 Modal)

```
Step 1: Select Input
  选择 Asset (来源 M-11)
  [下拉: Asset Name / Type / Size / Duration / Rights]

Step 2: Select Profile
  选择 Encoding Profile (来源 P-21)
  [下拉: Profile Name / Codec / Resolution / Bitrate]
  [最近使用] [Starred]

Step 3: Preview Output (新)
  模拟输出:
    File:        [Estimated File Name]
    Duration:    00:30:25
    Size:        ~187 MB (estimate)
    Video:       HEVC Main 1080p25
    Audio:       AAC 2ch 192k
    Codec:       HEVC
    Resolution:  1920×1080
    Bitrate:     5.0 Mbps
    FPS:         25

Step 4: Test Encode 5s (新 — 但不是 P-21 才有)
  [▶ Run Test]
  跑 5 秒 sample, 显示:
    Actual FPS:    127.4
    Actual Speed:   5.1x realtime
    CPU:            71% (host-01)
    Memory:         1.2 GB peak
    Estimated Full: 03:08
    Estimated Size: 188 MB
    Quality Check:  PASS / WARN / FAIL

  [Re-test] [Skip Test] [Continue]

Step 5: Create Full Job
  Output Destination: Local / S3 / NFS
  Schedule: Now / Scheduled / On Event
  Priority: 1-10
  Notify: Complete / Failed / Both

  [Submit Job]
```

## 19.3 Output / Result 区 (新)

> M-14 不只是"任务管理", 转码完成后必须显示结果。

```
OUTPUT — Job T-1821 (COMPLETED)
────────────────────────────────
File        /var/vbmf/media/master/News01_HEVC_1080p25.mp4
Duration    00:30:25
Size        188.4 MB
Video       HEVC Main 1080p25 · 4.98 Mbps · 25 fps
Audio       AAC 2ch 48kHz 192 kbps
Hash        sha256:abc123...
QC          ● PASS
Rights      ● Active (valid until 2027-01-01)

[Open Asset] [Open Version] [Preview] [Use in Playout] [Create Variant]
```

## 19.4 M-14 状态模型

- Normal: 有 Running 任务
- Loading: 任务列表加载
- Empty: 0 jobs + 引导 "Create your first transcode job"
- Warning: 队列堆积 > 10 / Worker CPU 持续 > 90%
- Error: 任务失败 + [Retry]
- Critical: 全部 Workers offline / Disk write 失败

---

# 20. P-22 Output Profile / Variant / Destination / Adapter — 4-Tuple 拆分

> **4 元组必须强制分离:** Output Profile (Policy) / Output Variant (Per-Channel) / Output Destination (Endpoint) / Output Adapter (Runtime Plugin)

## 20.1 4 元组定义 (与 §4.1 一致, 强化)

```
Output Profile (P-22)        = Delivery Policy (How: 编码/协议/可靠性/Latency)
Output Variant               = Per-Channel 派生 (1 Channel 1 Variant 1:1)
Output Destination           = 实际 endpoint (host:port/path)
Output Adapter               = 真正执行协议 (SRSAdapter / FileAdapter / UDPAdapter)
```

## 20.2 3-Tier Protocol 状态 (取代 P-22 之前混在一起的列表)

```
Protocol Status
──────────────
● Available     当前实现, Backend 支持, UI 可配置
○ Reserved      未来实现, UI 显示但标 Reserved
✗ Unavailable   当前不可用, 显式原因
```

V0.2 完整 3-Tier:

| Protocol | Status | V0.2 |
|---|---|---|
| HLS | ● Available | ✓ |
| RTMP | ● Available | ✓ |
| WebRTC | ● Available | ✓ (SRS WHIP) |
| SRT | ● Available | ✓ |
| UDP MPEG-TS | ● Available | ✓ |
| RTP | ● Available | ✓ (RTP over UDP · 0.5B.2 P0-6 加入, 与 RTPAdapter 对齐) |
| File | ● Available | ✓ |
| DASH | ○ Reserved | V0.4+ |
| SDI Master Output | ○ Reserved | V0.4 Target |
| DRM (Widevine) | ○ Reserved | V0.4+ |
| DRM (FairPlay) | ○ Reserved | V0.4+ |
| DRM (PlayReady) | ○ Reserved | V0.4+ |

**Reserved 协议 UI 表现:**
```
○ DASH  [Reserved · V0.4+]  ← 灰显
  ┗━━ Disabled, 不可配置
```

## 20.3 P-22 UI 重构

```
P-22 Output Profiles
────────────────────

[ Profile List ]
HLS-LIVE-01      ● ACTIVE    Used by 3 Variants
HLS-LL-01        ● ACTIVE    Used by 2 Variants
RTMP-PUSH-01     ● ACTIVE    Used by 1 Variant
ARCHIVE-01       ● ACTIVE    Used by 1 Variant
SDI-MASTER       ○ RESERVED  V0.4 Target

[ + Create Profile ]

──────────────────────────
P-22 Profile Detail
──────────────────────────
[ ◉ DESIRED  ⚙ COMPILED  ▶ EFFECTIVE ]

[ Tab Bar ]
  1. Profile     3. Latency
  2. Protocol    4. Player Capability
```

**P-22 不再包含 Variant / Destination / Adapter 配置** (它们在别处管理)。

## 20.4 Output Variant / Destination / Adapter 在哪里管理

| 对象 | 位置 | 备注 |
|---|---|---|
| **Output Profile** | P-22 | Delivery Policy 定义 |
| **Output Variant** | CD-01 Channel Detail → Tab 6 Output | 1:1 绑定 Channel |
| **Output Destination** | CD-01 Channel Detail → Tab 6 → Variant Detail | Endpoint 列表 |
| **Output Adapter** | E-35 Device Registry (Adapter Plugins) | Runtime Plugin |

---

# 21. E-32 Preflight — 9D Resource Required/Available/Delta/Headroom

> **重构 E-32:** Resource 区从 6 项简化为 9-Dim, **每个维度必须显示 Required/Available/Delta/Headroom**。

## 21.1 9-Dim Resource Vector (V0.2 §3.11 锁定)

```
CPU_THREADS / GPU_SESSIONS / VRAM_MB / RAM_MB /
NIC_INGRESS_MBPS / NIC_EGRESS_MBPS / DISK_WRITE_MBPS /
PCIE_RX_MB_S / PCIE_TX_MB_S
+ BMD_INPUT_TOKENS / BMD_OUTPUT_TOKENS / DEVICE_EXCLUSIVITY
```

## 21.2 Resource Panel UI 模式 (锁定)

```
Resource Vector (9-Dim)
─────────────────────────
CPU_THREADS
  Required   14.5
  Available  32
  Delta      -17.5
  Headroom   17.5 threads
  Status     ✓ OK

VRAM_MB
  Required   0
  Available  0
  Status     N/A · no GPU

DISK_WRITE_MBPS
  Required   32
  Available  410
  Headroom   378 MB/s
  Status     ✓ OK

BMD_INPUT_TOKENS
  Required   1
  Available  3
  Headroom   2
  Status     ✓ OK
  Allocation  CH01 → dv0

PCIE_RX_MB_S
  Required   1200
  Available  7000
  Headroom   5800
  Status     ✓ OK

DEVICE_EXCLUSIVITY
  Required   OK
  Status     ✓ No Conflict
```

## 21.3 Preflight 3 区 (锁定)

#### Static
- ✓ Graph valid (无环)
- ✓ Contract valid
- ✓ Rights valid
- ✓ Asset exists
- ✓ Latency within budget

#### Resource (9-Dim)
- 11 个维度 Required/Available/Delta/Headroom (见 §21.2)

#### Runtime Readiness
- ✓ SRS Adapter ready
- ✓ Backup node ready
- ✓ Source online
- ✓ Recorder ready
- ✓ Filler available
- ✓ Output QC ready

## 21.4 状态模型 (Preflight)

- Normal: 0 critical / 0 warning
- Loading: Preflight 跑中
- Empty: 未跑过 Preflight
- Warning: 有 warning
- Error: 有 critical → Apply 按钮禁用
- Critical: 多 critical 阻塞 Apply

## 21.5 Preflight 与 ChangeSet 联动

Preflight 报告作为 ChangeSet 的附件, 后续 Apply 时复用:
- 同一个 ChangeSet 在 Apply 前必须 PASS Preflight
- 如果 Apply 时 Preflight FAIL → Execution Phase 进入 ABORTED, Business Status 回 DRAFT

---

# 22. O-41 Health Tree — H1-H7 + Failure Absorbed + redundancy_group

> **核心升级:** Health Tree 显式展示 V0.2 Health Invariants (H1-H7) + Failure Absorbed 视觉化 + redundancy_group 关系。

## 22.1 H1-H7 健康不变量 UI 体现

```
Health Invariants
─────────────────
H1 ✓ No ACTIVE+FAILED
H2 ✓ No ACTIVE+DEGRADED
H3 ✓ No STANDBY+FAILED
H4 ✓ No STANDBY+DEGRADED
H5 ✓ OFFLINE+FAILED absorbed
H6 ✓ Source RG not all unavailable
H7 ✓ ECHS from channel_health_view
```

每个 H 规则:
- ✓ = PASS (绿)
- ⚠ = WARN (黄) — 接近违反
- ✗ = FAIL (红) — 已违反, Channel 状态受影响

## 22.2 Failure Absorbed 视觉化 (新增)

> 当 Backup 接管 Primary 失败时, UI 必须显式标 "ABSORBED"。

当前 UI:
```
Source.Primary  ● FAILED
Source.Backup   ● ACTIVE
Channel         ● HEALTHY  (但为什么?)
```

修复后:
```
Source.Primary  ○ OFFLINE  (was ACTIVE+FAILED, system absorbed)
Source.Backup   ◉ ACTIVE   (took over)
Redundancy      ✓ ABSORBED
Channel         ● HEALTHY  (Reason: H5 absorbed, Backup healthy)
```

> **关键:** 不是显示 FAILED 的红色, 而是显示 OFFLINE 的 outline 灰色 + ABSORBED 标签。
> 这传达"系统处理了, 不需要恐慌"。

## 22.3 redundancy_group 视觉化 (新增)

```
Source RG-CH01-SOURCE
─────────────────────
● Source.A    ACTIVE  · 1080p50
○ Source.B    STANDBY · 1080p25 (Capable: 50→25 FRAME_SWITCH)
○ Source.C    OFFLINE · V0.4 reserved
```

> **关键:** 不是平铺 3 个 Source, 而是用 RG 框起来, 显示"这是一个冗余组"。

## 22.4 9 Subsystem Health Matrix

```
Subsystem      | Status    | Reason
───────────────┼───────────┼─────────────────────
1. SOURCE      | ● HEALTHY | RG-01 absorbed
2. SWITCHER    | ● HEALTHY | FRAME_SWITCH (Δ from PACKET)
3. COMPOSITION | ● HEALTHY | Program+Variant OK
4. AUDIO       | ● HEALTHY | -23 LUFS / -1.3 dBTP
5. MASTER      | ● HEALTHY | AV sync +12ms
6. OUTPUT      | ● HEALTHY | HLS 1247 / RTMP OK
7. RECORDING   | ● HEALTHY | 2h14m
8. CLOCK       | ● HEALTHY | PTP LOCKED BROADCAST
9. RESOURCE    | ● HEALTHY | CPU 38% / RAM 22%
```

## 22.5 ECHS 来源声明 (必须)

> **关键约束:** 任何 Channel Health 显示都**必须**有:
> ```
> ECHS source: channel_health_view.effective_channel_status
> Aggregation: 7 rules (Rule 1-7) applied
> Policy: lifecycle_terminal > lifecycle_transition > health_tree_aggregation > unknown
> ```

UI 顶部或底部显示这一行, 让 Operator / Engineer 知道 status 是从哪儿来的。

---

# 23. E-34 Capability Registry — Why Compatible / Why Not

> **重构 E-34:** Capability 比较必须**显式**列出"为什么 PASS / 为什么 FAIL"。

## 23.1 Capability Matrix 模式 (锁定)

```
CAPABILITY COMPARISON — PACKET_SWITCH Candidate
─────────────────────────────────────────────────
Source A (Primary)  Source B (Backup)  Match  Status
─────────────────────────────────────────────────
Codec
  H.264              H.264              ✓      MATCH
Profile
  High               High               ✓      MATCH
Level
  4.0                4.0                ✓      MATCH
Resolution
  1920×1080          1920×1080          ✓      MATCH
FPS
  25                 25                 ✓      MATCH
Bit Depth
  8                  8                  ✓      MATCH
Color Space
  BT.709             BT.709             ✓      MATCH
Color Range
  TV                 TV                 ✓      MATCH
Audio Channel Layout
  Stereo             5.1                ✗      MISMATCH
Audio Codec
  AAC                AAC                ✓      MATCH
Clock Domain
  PTP                PTP                ✓      MATCH
Data Plane
  COMPRESSED_VIDEO   COMPRESSED_VIDEO   ✓      MATCH
```

## 23.2 Why PASS / Why Not

### PASS 情况

```
✅ PACKET_SWITCH ELIGIBLE

All 12 capability fields match between Source A and Source B.
Runtime alignment: PASS (last 24h)
```

### FAIL 情况

```
❌ PACKET_SWITCH NOT ELIGIBLE

Reason 1: Audio Channel Layout mismatch
  Source A: 2.0 (Stereo)
  Source B: 5.1 (Surround)
  
Reason 2: Runtime Alignment degraded
  Last 24h: 5 frames out-of-sync > 50ms threshold

Fallback:
  → FRAME_SWITCH candidate (check below)
  → MASTER_SWITCH candidate (check below)
```

## 23.3 Switch Decision Tree 完整呈现 (锁定)

UI 必须显示 3-tier 决策链:

```
Static (Capability):
  ✓ PASS  → can try PACKET_SWITCH
  ✗ FAIL  → skip to FRAME

Runtime Alignment:
  ✓ PASS  → PACKET_SWITCH ELIGIBLE
  ✗ FAIL  → degrade to FRAME_SWITCH

Frame Alignment:
  ✓ PASS  → FRAME_SWITCH ELIGIBLE
  ✗ FAIL  → degrade to MASTER_SWITCH

Master Readiness:
  ✓ READY  → MASTER_SWITCH ELIGIBLE
  ✗ NOT READY  → REJECT
```

UI 在每个 tier 显示 ✓/✗ + reason。

## 23.4 Compare 2-3 Sources UI

```
[Compare 3 Contracts]
Source A  Source B  Source C
──────────────────────────
(每个 contract 完整字段横排)
```

点击行 → 跳 E-34 capability check。

---

# 24. Dependency / Impact Preview — 全局 pattern

> **这是 V0.5B Closure-1 第二个核心 UX 基础设施。**
> 凡是有"配置"的页面, 都必须有 "Used By" 和 "Impact Preview"。

## 24.1 Used By — Profile / Channel / Device 详情必带

```
Used By
───────
Channels:
  CH01 (● ACTIVE)
  CH03 (● ACTIVE)

Graphs:
  Graph-21 (Compiled in CH01)
  Graph-25 (Draft)

Output Variants:
  V-CH01-HLS-Domestic
  V-CH03-HLS-Overseas

Active Sessions:
  2 (last 24h)

[View Dependency Graph]
```

## 24.2 Impact Preview — 修改前必看

任何修改 (Edit Profile / Change Clock / Change Switch Policy) 之前, 必须先显示:

```
Impact Preview
─────────────
You are changing:
  Encoding Profile LIVE_HEVC_1080P
  v3 → v4

Affected Channels:
  ● CH01  (currently ACTIVE)
  ● CH03  (currently ACTIVE)
  ○ CH05  (currently STOPPED)

Estimated:
  CPU    +4.2 threads
  RAM    +520 MB
  Latency  +18 ms

Risk: MEDIUM
Reason: 2 channels on-air, will require fail-over during apply

Runtime:
  Requires Change Set
  Recommended: Schedule Apply at next event boundary

[Cancel]  [Continue to ChangeSet]
```

## 24.3 Dependency Graph (可选可视化)

> Phase 4 实施时再做。当前 0.5B 用 "Used By 列表" 即可。

未来扩展 (V0.4+): 用 Graphviz 渲染:
```
Encoding Profile v3
    ↓
Graph Spec 21 ──→ Channel CH01
                  ├─ Output Variant V-CH01-HLS
                  │    └─ Destination CDN-A
                  └─ Output Variant V-CH01-RTMP
                       └─ Destination Origin

[Cycle Check: OK]
```

## 24.4 适用页面

| 页面 | Used By | Impact Preview |
|---|---|---|
| **P-21** Encoding Profile | ✓ | ✓ |
| **P-22** Output Profile | ✓ | ✓ |
| **P-23** Audio Profile | ✓ | ✓ |
| **P-24** Graphic Profile | ✓ | ✓ |
| **P-25** QC Profile | ✓ | ✓ |
| **P-26** Rights Profile | ✓ | ✓ |
| **P-27** Edge Policy Profile | ✓ | ✓ |
| **E-35** Device | ✓ (Used By) | ✓ (Lock 影响) |
| **E-37** Clock | ✓ (Used By) | ✓ (切换影响) |
| **CD-01** Channel Detail | — | (Reverse direction) |

## 24.5 实施原则

- 0.5B.1 wireframe: P-21 + P-22 实施完整 Used By + Impact Preview
- Phase 4: 全部 Profile 页实施

---

# 25. 其他 Closure 项 (Benchmarks / Clock / Permission)

## 25.1 O-45 Benchmarks 增强

当前: 只列 p50/p95/p99。

新增字段:
- **Target (Policy):** 100 ms
- **Measured:**
  - p50: 78 ms
  - p95: 87 ms
  - p99: 94 ms
- **Trend:** ↗ / → / ↘ (vs 7 days ago)
- **Test Profile:** Profile v3 + GR-42
- **Hardware:** x265 + host-01
- **Timestamp:** 2026-08-25 14:25
- **Pass / Fail:** ✓ (within budget)

**Benchmark Run Detail (drill-down):**
- 完整测试日志
- 资源占用时间序列
- FFmpeg 命令行
- 错误 / 警告

## 25.2 E-37 Clock "Used By" 字段

```
PTP0 · ptp0 (eth0)
LOCKED · BROADCAST_GRADE

Used By:
  CH01 · ● ACTIVE
  CH02 · ● ACTIVE
  CH03 · ● ACTIVE

Clock Quality:
  BROADCAST_GRADE

Fallback:
  TIMECODE (GOOD)

Impact:
  3 channels
```

切换 Clock Reference 前必须显示此 Used By, 让 Engineer 知道影响范围。

## 25.3 A-53 Permission Context 增强

> 当前 A-53 是"角色 × 操作 × R/W/A"。**升级为"角色 × 操作 × Scope × Object × Current State × Guard"。**

```
Permission
──────────
Allowed
Scope
  CH01 only
Object
  Channel.CH01 / Profile.v3
Context
  Channel = ON-AIR
Guard
  Requires L2 confirmation (3s countdown)
```

**例:**
- Engineer 可 Apply ChangeSet, 但 Channel=ON-AIR 时必须 L2
- Operator 可 Failover, 但不能改 Switch Policy (Policy 编辑需 Engineer)

## 25.4 实施原则

- 0.5B.1: O-45 + E-37 + A-53 字段增强
- Phase 4: 完整 Permission Context 引擎

---

# 26. Phase 0.5B Closure-1 完成度自评

| 维度 | 评分 |
|---|---|
| Architecture 一致性 | 97/100 |
| Runtime Semantics | 99/100 |
| **0.5A Operator UX** | 95/100 |
| **0.5B Surface Spec 总分** | **92/100** (上一版 87) |
| Profile UI | **88/100** (上一版 78) |
| Transcode UX | **85/100** (上一版 76) |
| Output UX | **88/100** (上一版 79) |
| Engineering UX | 90/100 (上一版 86) |
| Health UX | **93/100** (上一版 89) |
| 权限 / 风险 UX | 86/100 (上一版 83) |
| 视觉系统 | **87/100** (上一版 82) |
| **综合 UI/UX** | **91/100** (上一版 84) |

---

# 27. 下一阶段路径

```
Phase 0.5B Closure-1 (本节 · 完成)
       ↓
Phase 0.5B.1 = 5 张 P0 wireframe
  M-11 Media Library
  M-12 Asset Detail
  M-14 Transcode Center (含 Preview/Test/Submit + Result)
  P-21 Encoding Profile Builder (10 sections)
  P-22 Output Profile (4-tuple 拆分)
       ↓
5 张 wireframe Review
       ↓
Phase 0.5 全部 FREEZE
       ↓
Phase 0.6 Executable Acceptance Spec
       ↓
Phase 1 Media Core (Rust)
```

**Phase 0.5B.1 是最后 5 张 wireframe。**
**0.5B.1 完成后 Phase 0.5 全部 Lock, 不再扩展 UI/UX 范围。**

---

---

# 28. Phase 0.5B.2 — Product UX/Semantic Closure (最后一轮)

> **Phase 0.5B.0 (13 P0 语义边界) + Closure-1 (10 产品化收口) + B.1 (5 P0 wireframes) 已完成。**
>
> **Phase 0.5B.2 (本节) = 最后一轮 Product UX/Semantic Closure, 处理 8 P0 + 8 横切能力 + 5 P1。**
>
> 完成后:
> **Phase 0.5 = UX BASELINE LOCK FINAL** (正式宣布, 不再迭代 UI/UX)
> ↓
> 直接进入 **Phase 0.6 Executable Acceptance Specification**

## 28.1 8 P0 项目 (本轮必须收口)

### P0-1 · Signal Contract / timebase 输入来源闭合 (架构层)

**问题:** Switch Decision 需要 time-base compatibility 输入, 但 SignalContract Schema 缺字段, 容易让工程师实现分裂。

**修复 (文档层, 不改核心架构):**

```yaml
SignalContract (V0.2):
  media:
    # 已有
    video_resolution, video_fps, video_pixel_format,
    video_field_order, video_color_space, video_color_range
    audio_sample_rate, audio_channels, audio_layout
    codec_video, codec_audio, container
    # 新增明确
    video_timebase: 1/90000      # 视频时间基
    audio_timebase: 1/48000      # 音频时间基
    frame_duration: ms            # 帧时长

  clock:
    domain: PTP | TIMECODE | SYSTEM | MONOTONIC
    reference_class: BROADCAST_GRADE | GOOD | FAIR | POOR
    # 来源: Clock Reference (E-37), 不是 SignalContract 自带

RuntimeAlignment (V0.2):
  clock_lock: LOCKED | DEGRADED | FAILED | LOST
  drift: ms/min
  timestamp_continuity: OK | GAP | DUPLICATE
```

**关键边界:** 静态能力 (SignalContract) ≠ 运行态时钟对齐 (RuntimeAlignment)。

### P0-2 · Health Snapshot freshness / stale 语义 (架构层)

**问题:** `current_health_trees` 是"最新 snapshot", 不是"最新且新鲜"。Agent 断联后, DB 仍显示 HEALTHY, **24/7 危险**。

**修复 (新增 health_freshness 概念):**

```yaml
health_freshness:
  snapshot_ts: 时间戳
  observed_at: 时间戳
  max_age_ms: 30000             # 30s 是默认, 频道可配
  stale_after_ms: 60000         # 60s 是 stale 阈值
  freshness_state:
    FRESH:     正常显示 HealthState
    STALE:     Channel → 至少 UNKNOWN (不能继续显示 HEALTHY)
    UNKNOWN:   直接 UNKNOWN
```

**聚合规则新增:**
- STALE → channel_health_aggregation = UNKNOWN
- UNKNOWN 优先于其他规则
- UI 必须显式标 "STALE · Last observed Xs ago"

### P0-3 · NIC resource per-device (token + per-interface)

**问题:** 当前 NIC 资源只看总带宽, 不知 eth0/eth1 单口, 容易 1Gbps 网口被超额分配。

**修复 (Resource Vector 扩展):**

```yaml
Resource_Vector (扩展):
  NIC_INGRESS_MBPS: 总和
  NIC_EGRESS_MBPS:  总和
  NIC_TOKENS:
    - id: eth0
      speed_mbps: 10000
      role: data
    - id: eth1
      speed_mbps: 1000
      role: management

NIC_AFFINITY:
  required: [eth0]      # Job 必须绑特定 NIC
  preferred: [eth0]
  avoid: [eth1]
```

**Preflight 必须 per-NIC 检查**, 不能只看总和。

### P0-4 · P-21 wireframe DESIRED/COMPILED/EFFECTIVE 语义修正 (UI 层)

**问题:** P-21 EFFECTIVE 列显示 "5.1x realtime" — 这是 Runtime Telemetry, 不是 Configuration State。三层混了。

**修复:**
- EFFECTIVE 列**只**显示 Configuration 实际生效状态 (如 `Revision v3 ACTIVE`)
- 旁边另开 Runtime Performance 面板:
  ```
  Runtime Performance
  5.1x realtime
  127.4 FPS
  CPU 71%
  RAM 1.2 GB
  ETA 03:08
  ```
- 3-Layer (DESIRED/COMPILED/EFFECTIVE) 与 Runtime Telemetry 严格分离

### P0-5 · M-12 Asset Version vs Output Variant 命名分离 (UI 层)

**问题:** M-12 同时存在 "Asset Version" (Master/Proxy/Mobile) 和 "Output Variant" (CH01 HLS Domestic), 两种 "Variant" 极易混淆。

**修复 (命名锁定):**
- Asset Version (M-12 Tab Versions): Master / Proxy / Mobile / Archive / Custom
- Output Variant (M-12 Tab Overview + CD-01 Tab 6): V-CH01-HLS-Domestic / V-CH03-HLS-Overseas
- Button 全部改名:
  - "Create Version" → **"Create Asset Version"**
  - "Create Variant" (在 Versions Tab) → **"Create Output Variant"** (跳到 CD-01 配置)

### P0-6 · P-22 RTP / Latency / Edge Policy 三件套 (UI + 文档)

**问题 1: RTP 缺失** — Architecture 有 RTPAdapter, P-22 协议列表没列, Capability 不对齐。

**修复:** P-22 Available Protocols 加 **RTP** (RTP over UDP, 与 UDP MPEG-TS 区分)。

**问题 2: Latency Target 3 个概念混了** — Delivery / Channel E2E / Failover 三个 latency 不是同一个数字。

**修复:** P-22 Latency 区拆为 3 行:
```
Delivery Latency Target
  2.0 s    (协议本身, e.g. LL-HLS 2s segment)

Channel E2E Latency Target
  ≤ 200 ms    (整链路端到端预算)

Failover Latency Target
  Policy: HOT
  Target: 100 ms    (hot_standby_levels.target_failover_time)
```

**问题 3: Failover / Retry 放错地方** — 这些是 Edge Policy (P-27) 的事, 不是 Output Profile 的事。

**修复:** P-22 只保留 "Edge Policy" 引用字段, 不再独立配置 Retry/Reconnect/Failover:
```
Edge Policy Profile: LIVE_EDGE_DEFAULT (ref → P-27)
```

### P0-7 · M-14 Test Encode → Mini Acceptance Test (UI 层)

**问题:** 当前 Test Encode 只测性能 (FPS/Speed/CPU/RAM/Estimated), 不验证输出是否正确。

**修复:** 改名 + 扩展字段:
```
Test Encode / Mini Acceptance Test
─────────────────────────
Video
  Resolution:    1920×1080 ✓
  Pixel Format:  yuv420p10le ✓
  Color:         BT.709 ✓
  FPS:           25.00 ✓
  PTS Continuity: OK ✓

Audio
  Codec:         AAC ✓
  Sample Rate:   48kHz ✓
  Channels:      2 ✓
  Layout:        Stereo ✓

A/V Sync
  Offset:        +12 ms ✓
  Drift:         +2.1 ms/min ✓

Mux Validity
  Container:     MP4 ✓
  Index:         OK ✓
  Duration:      00:00:05 ✓

Runtime (参考)
  FPS:           127.4
  Speed:         5.1x
  CPU:           71%
  RAM:           1.2 GB
  ETA:           03:08
```

### P0-8 · 全局 Design System + State Taxonomy (新文档)

**问题:** 五张 wireframe 各自定义 CSS / State, Phase 4 实施会各做各的。

**修复:** 创建 [`DESIGN_SYSTEM.md`](DESIGN_SYSTEM.md) 锁定:
- 4 State Models Taxonomy (UI Surface / Lifecycle / Readiness / Health / Node Role / ECHS)
- Color Tokens
- Components (Button / Badge / Status / Tabs / Table / Wizard / MetricCard / ResourceGauge / HealthNode / RuntimeState / DangerActions / Timeline / Diff)
- 6 状态样例 unified
- Keyboard shortcuts (Command Palette / G D / G M / T / F / R / Esc / Space / Ctrl+S)

## 28.2 8 横切能力 (Phase 4 实施指南, 本轮锁定)

| # | 能力 | 含义 | 实施 |
|---|---|---|---|
| 1 | **Impact Preview** | 任何修改前显示: Affected / Resource / Runtime Risk / Rollback | 0.5B.1 M-14 + P-21 已部分实现 |
| 2 | **Dependency Graph** | Profile / Asset / Output / Channel 的"谁用我/我影响谁" | 0.5B.1 M-12/P-21/P-22 Used By 已部分实现 |
| 3 | **Explain Why** | 统一 6 类解释: Why selected / Why not usable / Why degraded / Why this worker / Why FRAME not PACKET / Why output failed | 0.5B.1 P-21/P-22 已部分实现 |
| 4 | **Runtime Freshness** | Health / Discovery / Capability 都有 "Last observed / Age / Fresh · Stale" | 0.5B.2 P0-2 新增 |
| 5 | **Configuration Diff** | 所有 Revision 之间 before / after / impact | P-21 Section 10 + M-14 提交前 |
| 6 | **Compatibility Advisor** | Profile ↔ Source ↔ Worker ↔ Output ↔ Player | 0.5B.1 P-21 已部分实现, 本轮强化 |
| 7 | **Design System** | 5 张 wireframe 统一组件 | 0.5B.2 P0-8 新文档 |
| 8 | **Command Palette + Keyboard** | Ctrl+K / G D / G M / T / F / R 等 | 0.5B.2 P0-8 锁定语义, Phase 4 实施 |

## 28.3 5 P1 强化 (本轮一起做)

| # | P1 | 含义 | 实施位置 |
|---|---|---|---|
| 1 | **M-11 Saved Views** | 媒体库常用查询: 今日新闻 / 待 QC / QC Failed / Rights 7d 到期 / 可直接播出 | M-11 顶部新增 Saved Views 区 |
| 2 | **Profile Diff** | M-14 New Job Step 1 → Step 2 时显示 v3 vs v4 差异 | M-14 Wizard Step 2 |
| 3 | **M-14 Use in Playout Safety Gate** | 6 检查: QC PASS / Rights VALID / Version READY / Duration OK / Loudness OK / Format compatible | M-14 Output Result 区 |
| 4 | **M-12 Rights Override L3** | Override 改 L3 + 必须填 Who/Why/Scope/Expiry | M-12 Rights Tab |
| 5 | **Delete Safety** | Profile Delete 不只检查 Use Count, 必须检查 Used By (Channels / Variants / Active Sessions / Pending ChangeSets) | P-21/P-22 Delete 按钮 |

## 28.4 Phase 0.5 = UX BASELINE LOCK FINAL 判定

完成本轮后:

| 维度 | 当前 | LOCK FINAL 阈值 | 状态 |
|---|---|---|---|
| V0.2 Architecture | 98 | ≥ 95 | 🟢 |
| Phase 0.5A Operator Semantics | 95 | ≥ 92 | 🟢 |
| Phase 0.5B Surface Spec | 94 | ≥ 90 | 🟢 |
| Phase 0.5B.1 P0 Wireframes (5 张) | 91 | ≥ 88 | 🟢 |
| **Phase 0.5 综合 (本轮收口后)** | **93** | **≥ 90** | **🟢 LOCK FINAL** |

**Phase 0.5 = UX BASELINE LOCK FINAL** 正式宣布后:
- ❌ 不再开新页面
- ❌ 不再做 UI/UX 大改
- ✅ Phase 0.6 Executable Acceptance Spec
- ✅ Phase 1 Media Core (Rust)
- ✅ Phase 4 Web Console (按本规范实施)

## 28.5 实施文件清单 (0.5B.2 本轮; 0.5C 已归并目录)

新增 / 修改 (0.5C 归并后的现路径):
- `docs/phase-0.5/SURFACE_SPEC.md` (本节 §28)
- `docs/phase-0.5/DESIGN_SYSTEM.md` (新增, P0-8)
- `docs/phase-0.5/product/M-11-media-library.html` (P1 Saved Views)
- `docs/phase-0.5/product/M-12-asset-detail.html` (P0-5 Asset Version 命名 + P1 Rights Override L3)
- `docs/phase-0.5/product/M-14-transcode-center.html` (P0-7 Mini Acceptance Test + P1 Profile Diff + Use in Playout Safety)
- `docs/phase-0.5/product/P-21-encoding-profile.html` (P0-4 EFFECTIVE 语义 + BMD port-by-port + P1 Compatibility Advisor)
- `docs/phase-0.5/product/P-22-output-profile.html` (P0-6 RTP + Latency 拆 + Edge Policy 边界)
- `README.md` (顶层, 反映 38 surfaces / UX BASELINE LOCK FINAL)

---

# 29. Phase 0.5C — Information Architecture Closure (本节)

> **状态**: 🟡 **DRAFT 0.1** (本轮, 待用户审过)
>
> **触发**: 0.5B.2 UX BASELINE LOCK FINAL 后, 用户反向 review 25 条, 发现:
> 1. 目录结构 phase-0.5 + phase-0.5b 不应并列 (应统一目录 + milestone 归档)
> 2. Realtime Encode / File Transcode 必须现在拆 (不增加 Engine)
> 3. README / ROADMAP / SURFACE_SPEC / Phase 0.6 README 之间状态不同步
> 4. 缺少"对象组合关系"产品级入口 (Profile Bundle)
> 5. 缺少 Hardware / Clock / Realtime Transcode / Profile Center / Job Detail
> 6. 数字 (01-06) 不应进 UI 顶层导航
> 7. Phase 0.6 README 写了 `< 100ms` 错误语义 (应 `target_failover_time_ms + measured p50/p95/p99`)

## 29.1 目录归并 (commit cec7407 → 本轮)

```
OLD:
docs/phase-0.5/    (Operator, 9 + 1)
docs/phase-0.5b/   (Product, 5)

NEW (本轮):
docs/phase-0.5/
├── README.md                    (Phase 0.5 顶层入口, 4 域导航)
├── OBJECT_VOCABULARY.md         (14 对象权威定义, 0.5C 新增)
├── PRODUCT_OBJECT_MODEL.md      (3 层组合, 0.5C 新增)
├── NAVIGATION.md                (4 域, 0.5C 新增)
├── MILESTONES.md                (历史 milestone, 0.5C 新增)
├── SURFACE_SPEC.md              (从 phase-0.5b/ 移过来)
├── DESIGN_SYSTEM.md             (从 phase-0.5b/ 移过来)
├── I18N_SPEC.md                 (从 phase-0.5b/ 移过来)
├── OPERATOR_WORKFLOW.md         (原 phase-0.5/)
├── ERRATA.md                    (原 phase-0.5/)
├── INDEX.md                     (原 phase-0.5/)
├── milestones/                  (5 历史 milestone 文档, 0.5C 新增)
├── operator/                    (原 phase-0.5/wireframes/, 9 + 1)
├── product/                     (原 phase-0.5b/wireframes/, 5)
└── chains/                      (原 phase-0.5/chains/, 4)
```

⛔ `phase-0.5b/` 目录已 git rm 完毕。Git history 完整保留 (全部用 `git mv`)。

## 29.2 4 域顶层导航 (覆盖原 6 工作域)

UI 顶层导航**从数字改为业务域**:

| 域 | 中文 | 主要用户 | 包含对象 (本域新增) |
|---|---|---|---|
| **BROADCAST** | 直播 | Operator / Director | Channel · Source · Session (REALTIME) · Variant |
| **MEDIA** | 媒体 | Content Manager / Editor | Asset · Asset Version · Job (6 kinds, 见 §29.5) |
| **ENGINEERING** | 工程 | Engineer / SRE | **Profile Center (P-20 新增) · Profile Bundle (P-28 新增) · 6 Profile · Graph/Route (E-31) · ChangeSet · Preflight · Hardware (E-38 新增) · Clock (E-37 升级) · Health · Incident · Replay · Benchmark** |
| **ADMIN** | 管理 | Admin | User · Role · Permission · Audit · System Setting |

**计数表 (0.5C.1 重算 — 修正草稿与已锁定编号的撞号: E-34 已是 Capability Registry, E-36 已是 Resource/Capacity, E-37 本就是 Clock, M-15/M-16 已被占用; 新表面改用空闲编号 M-17 / M-18 / E-38):**

| 域 | 已锁定表面 | 0.5D 新增 | 域内合计 |
|---|---|---|---|
| BROADCAST | 9 (01-07 Core + 08→E-31·09→O-41 归 ENGINEERING 后剩 7 + CD-01) | +1 (**M-17 Realtime Transcode**) | 9 |
| MEDIA | 6 (M-11~16) | +1 (**M-18 Job Detail**, 由 M-15 子页升级为独立页; M-14 重画不加数) | 7 |
| ENGINEERING | 19 (E-31~37 = 7 + O-41~45 = 5 + P-21~27 = 7) | +3 (**P-20** + **P-28** + **E-38 Hardware**; E-37 Clock 为升级不加数) | 22 |
| ADMIN | 5 (A-51~55) | 0 | 5 |
| 全局 | 1 (10-states Validation, 不属任何域) | 0 | 1 |
| **TOTAL** | **40** | **+5** | **44** |

> 计数口径: 已锁定 40 = 0.5A 10 (9 Core + 1 Validation) + 0.5B 新增 28 + CD-01 (Closure-1 新增, 此前未计入)。
> 0.5D 交付 = 5 新表面 (M-17 / M-18 / P-20 / P-28 / E-38) + 1 升级 (E-37 Clock) + 1 重画 (M-14 → File Transcode)。
> 禁止再使用 "M-15 Realtime / M-16 Job Detail / E-34 Hardware / E-36 Clock" 指代 0.5D 新表面 (它们是已锁定的其他表面)。

⛔ **Profiles 不再是顶层域** (归 ENGINEERING)
⛔ **Operations 不再是顶层域** (归 ENGINEERING)

## 29.3 Realtime Encode / File Transcode 拆分 (M-14 / M-17)

V0.2 锁定的 1 个 Encode Engine, Phase 0.5C 拆为 2 个**产品语义** (不增加 Engine):

| 产品语义 | 底层 Engine | 运行时对象 | UI 表面 |
|---|---|---|---|
| **Realtime Transcode** (实时转码) | Encode Engine (REALTIME mode) | **Session (MEDIA_SESSION)** | **M-17 Realtime Transcode** (0.5D 新增) |
| **File Transcode** (文件转码) | Encode Engine (FILE mode) | **Job (FILE_TRANSCODE kind)** | **M-14 File Transcode** (0.5B.1 M-14 改名, 0.5D 重画) |

详细 2 种语义对比:

| 维度 | Realtime Transcode | File Transcode |
|---|---|---|
| 输入 | Source (SDI / SRT / RTMP) | Asset (MP4 / MOV / TS) |
| 输出 | Variant (live) → SRS | Asset Version (新 Version) |
| 状态 | Session 三轴 (lifecycle + readiness + health) | Job 5 状态 (PENDING/QUEUED/RUNNING/COMPLETED/FAILED) |
| 用户关心 | FPS / Speed / CPU / Dropped Frames / AV Offset / Latency / READY_TO_TAKE | % Progress / ETA / Output Size / Quality / CRF / Loudness |
| 失败恢复 | FRAME/MASTER failover (V0.2) | 1-pass / 2-pass / Retry / Cancel |
| UI 步骤 | (无, 持续) | 6 步 Wizard: Source / Output / Profile / QC / Schedule / Submit |

**P-21 Encoding Profile** 同时支持 2 种语义, 但 schema 分 Common / Realtime / File 3 段 (0.5D 实施)。

## 29.4 0.5D 交付表面 (5 新增 + 1 升级 + 1 重画)

| 表面 | 域 | 类型 | 关键交付 |
|---|---|---|---|
| **P-20 Profile Center** | ENGINEERING | 新增 | 7 Tab 切换 6 种 Profile Registry + Profile Bundle, 顶部 Used By 全域 |
| **P-28 Profile Bundle** | ENGINEERING | 新增 | 1 Channel 1 Bundle, 6 Profile 引用, 不重新配置 6 套参数 |
| **E-38 Hardware Inventory** | ENGINEERING | 新增 | HOST 顶层 (CPU/GPU/BMD/NIC/Storage) → Device 详情 (Capabilities/Ports/Assignment/Health/Temperature/Firmware/Driver); 与 E-35 Device Registry / E-36 Resource 互补 |
| **E-37 Clock** (升级) | ENGINEERING | 升级 | Reference (PTP/TIMECODE/SYSTEM/MONOTONIC) + Fallback Chain + Offset/Drift/Lock + Fallback history (已有 Spec §E-37, 0.5D 补 wireframe) |
| **M-17 Realtime Transcode** | BROADCAST | 新增 | 顶部 Live Encoder Runtime (RUNNING/READY/HEALTHY) + 主区 SOURCE→NORMALIZE→ENCODER→OUTPUT + 右侧实时指标 (FPS/Speed/CPU/RAM/PTS Drift/AV Offset/Latency/Dropped Frames) + Primary/Backup/Effective Mode/READY_TO_TAKE |
| **M-18 Transcode Job Detail** | MEDIA | 新增 (由 M-15 子页升级为独立页) | Job #TR-1822 (Status/Input/Profile/Worker/Pipeline 6 步/Quality VMAF PSNR SSIM/Output/Attempts) |
| **M-14 File Transcode** (重画) | MEDIA | 重画 | 6 步 New File Transcode Wizard (Source / Output / Profile / QC / Schedule / Submit), 不再"贴实时 Worker" 形式 |

## 29.5 Object Vocabulary (0.5C 新增文档)

14 个核心对象锁定 (Phase 0.5 全栈唯一权威):

1. **Asset** (媒体资产, 1:1) — M-11 / M-12
2. **Asset Version** (Master/Proxy/Mobile/Archive/Custom) — M-12 Tab ②
3. **Profile** (6 子类) — P-20 Profile Center
4. **Profile Bundle** (1 Channel 1 Bundle, 6 Profile 引用) — P-28 (0.5D)
5. **Channel** (运营单位) — CD-01
6. **Source** (11 kinds) — 02 Sources
7. **Route** (Graph 编译后) — 08 Graph Designer
8. **Output Variant** (1 Channel N Variant) — CD-01 Tab 6
9. **Output Destination** (host:port) — 06 Output
10. **Output Adapter** (SRSAdapter/UDPAdapter/RTPAdapter/FileAdapter) — P-22
11. **Job** (6 kinds: FILE_TRANSCODE / REALTIME_ENCODE / PROBE / QC / UPLOAD / ARCHIVE; REALTIME_ENCODE 由 Session 包装 — 见 §29.3 与 [`OBJECT_VOCABULARY.md` §1.11](OBJECT_VOCABULARY.md)) — M-14 / M-15 / M-18
12. **Session** (2 kinds: MEDIA_SESSION / OUTPUT_SESSION, 三轴状态) — M-17 / CD-01
13. **Revision** (不可变快照) — P-21 §10 / P-22 / CD-01
14. **Change Set** (Logical Atomic Apply) — E-33

⛔ **典型易混术语对** (强制 UI 区分):
- Asset Version vs Output Variant (M-12 + CD-01)
- Profile vs Profile Bundle (P-20 + P-28)
- Job vs Session (M-14 / M-18 + M-17)
- Revision vs Version (V0.2 强约束: Version 修改 = 新 Revision)
- Graph vs Route (08 + E-32)

## 29.6 Phase 0.6 README 语义修复

Phase 0.6 README 之前写:

```text
- [ ] 切换时延 < 100ms（target，非协议保证）  ❌ 错误
- [ ] 切换 < 500ms（target）                     ❌ 错误
```

V0.2 Architecture 锁定: 任何 latency 验收**禁止**写协议式保证。正确写法:

```text
- [ ] `target_failover_time_ms` 来自 `hot_standby_levels` (V0.2 锁定, Policy 字段)
- [ ] `failover_benchmarks` 独立 runtime 实测 p50 / p95 / p99
- [ ] PASS = measured p95 <= target
```

Phase 0.6 README §0 已加 V0.2 语义对齐段, §A1 / §A2 验证项已修正。

## 29.7 Phase 0.5 LOCK FINAL 条件 (0.5C → 0.5D → 0.5E)

完整判定见 [`MILESTONES.md` §4](MILESTONES.md#4-phase-05-lock-final-判定矩阵)。

简版:
- ⛔ **0.5C LOCK FINAL** (本轮提交后, 需用户审过)
- ⛔ **0.5D LOCK FINAL** (5 个新表面 M-17/M-18/P-20/P-28/E-38 + E-37 升级 + M-14 重画)
- ⛔ **0.5E LOCK FINAL** (Impact Preview + Configuration Diff + Command Palette 全部跨域落实)
- ⛔ **README / ROADMAP / SURFACE_SPEC / Phase 0.6 README** 状态完全同步
- ⛔ **Object Vocabulary + Product Object Model + Navigation** 3 文档 LOCK

## 29.8 实施文件清单 (本轮 + 0.5D)

**本轮 (0.5C) 已完成:**
- 目录归并 (17 R + 9 M + 1 D, git history 保留)
- `docs/phase-0.5/OBJECT_VOCABULARY.md` (16.5KB 新)
- `docs/phase-0.5/PRODUCT_OBJECT_MODEL.md` (12.2KB 新)
- `docs/phase-0.5/NAVIGATION.md` (11.6KB 新)
- `docs/phase-0.5/MILESTONES.md` (7.4KB 新)
- `docs/phase-0.5/README.md` (重写, 4 域导航)
- `README.md` (根, 修 Engine 列表 + 9 Core 残留 + 4 域导航)
- `docs/phase-0.6/README.md` (修 `< 100ms` 语义)

**0.5D 实际落地路径 (更正 — 0.5C.1 目录归并后, 全部放 `operator/`):**
- `docs/phase-0.5/operator/M-17-realtime-transcode.html` (新, BROADCAST 域)
- `docs/phase-0.5/operator/E-38-hardware-inventory.html` (新, ENGINEERING 域)
- `docs/phase-0.5/operator/E-37-clock.html` (升级, 已有 Spec 无 wireframe)
- `docs/phase-0.5/operator/P-20-profile-center.html` (新, ENGINEERING 域)
- `docs/phase-0.5/operator/P-28-profile-bundle.html` (新, ENGINEERING 域)
- `docs/phase-0.5/operator/M-18-transcode-job-detail.html` (新, 由 M-15 子页升级为独立页)
- `docs/phase-0.5/operator/M-14-file-transcode.html` (原 M-14 重画, 从 product/ 移到 operator/)

---

## 29.9 Phase 0.5F — Channel/Network UX Closure (PIA V0.1 锁后实施)

PIA V0.1 锁 12 项 (见 [`PRODUCT_INFORMATION_ARCHITECTURE.md` §13](PRODUCT_INFORMATION_ARCHITECTURE.md#13-pia-锁-12-项-总结)) 后, Phase 0.5F 分 3 批落地:

### 29.9.1 Batch 1 (commit `bda8134`) — Channel 工作台 3 张

| 表面 | 域 | 类型 | 关键交付 |
|---|---|---|---|
| **CH-01 Channel List** | BROADCAST | 新增 | 4 Channel 卡片 + mini-monitor; 顶层导航 BROADCAST 域新页; Channel 为 UI 第一对象 |
| **CD-01 Channel Control Workspace** | BROADCAST | 新增 | Take Desk 7 块 1 屏: PVW/PGM/NEXT + SOURCE + AUDIO + OUTPUT + SWITCH + HEALTH + TAKE 按钮 |
| **CD-01 Channel Detail** | BROADCAST | 新增 | 8 Tab: Overview / Switch / Audio / Output / Graph / Health / History / Config; Tab 1 Overview = CD-01 Workspace 同一模板 |

### 29.9.2 Batch 2 (commit `0511c8c`) — Network Source 模型 2 张

| 表面 | 域 | 类型 | 关键交付 |
|---|---|---|---|
| **02-sources.html** (重写) | ENGINEERING | 重画 | 双段 Source 二级 Taxonomy: Local Device Source (SDI/Internal/File) + External Network Source (9 子类: UDP Unicast/Multicast, SRT, RTMP, HLS, RTSP, WebRTC Pull, RIST, Zixi, NDI) |
| **E-40 Network Source** | ENGINEERING | 新增 | UDP Unicast + Multicast 配置面板 + Multicast Diagnostics 7 项 + Network Source Security 8 字段 (PIA §11 锁) |
| `scripts/check_docs.py` | tooling | 修复 | 兼容 `?query` 和 `#anchor`, 修 CH-01 `?ch=CH0x` 假阳性 |

### 29.9.3 Batch 3 (本轮 commit 待) — Network Path Spec 1 文档

| 文档 | 域 | 类型 | 关键交付 |
|---|---|---|---|
| **E-41 Network Path Inspector Spec** | ENGINEERING | Spec 锁 | Network Path 5 类 Node Kind + 8 类失败模式 + 4-Layer 应用 + 6 状态 + Schema 草稿; wireframe 0.5G 实施 |

### 29.9.4 0.5F 计数表 (PIA Batch 1+2+3)

| 域 | 0.5D 已锁定 | 0.5F 新增 | 0.5F 域内合计 |
|---|---|---|---|
| BROADCAST | 9 (01-07 Core + CD-01 + M-17) | +3 (**CH-01** + **CD-01 Channel Workspace** + **CD-01 Detail 升 wireframe**) | 12 |
| MEDIA | 8 (M-11~18) | 0 | 8 |
| ENGINEERING | 22 (E-31~38 + O-41~45 + P-20~28) | +1 (**E-40 Network Source**); E-41 Spec 锁 (不计为表面) | 23 |
| ADMIN | 5 (A-51~55) | 0 | 5 |
| 全局 | 1 (10-states Validation, 不属任何域) | 0 | 1 |
| **TOTAL wireframe** | **44** | **+4** | **48** |
| E-41 Network Path (Spec only) | - | +1 (0.5G 实施后 +1) | 1 Spec |

> **0.5F 后 Phase 0.5 UI 表面 = 48 个 wireframe 完成** (44 + CH-01 + CD-01 WS + CD-01 Detail 升 wireframe + E-40)
> 02-sources.html 重画不计为新增。
> E-41 Network Path 仅 Spec 锁, 0.5G 实施后总计 49。
> ⛔ 禁止在 README / 阶段总结中再使用 39 / 44 / 47 等早期数字。

### 29.9.5 0.5F 实施要点

- ⛔ **不引入第 13 个 Engine** (PIA §10.3 锁); E-41 是 Network Endpoint 内部诊断
- ✅ **Channel 为 UI 第一对象** (顶层导航 BROADCAST 域)
- ✅ **Source 6 字段** (Identity / Adapter / Endpoint / Contract / Runtime / QC) 通过 E-40 完整配置
- ✅ **Network Endpoint 统一对象** (UDP Unicast/Multicast/RTP/SRT) 通过 E-40 表达
- ✅ **双层 UI**: Operation 工作台 (CD-01 Workspace) + Engineering 深页 (E-40 / E-41)
- ✅ **4-Layer (Desired/Compiled/Effective/Impact)** 推广到 E-41 (§8)
- ✅ **Take Desk 7 块** (CD-01 Workspace)
- ✅ **CD-01 8 Tab** (CD-01 Detail)
- ✅ **Network Source Security 8 字段** 在 E-40 留位 (PIA §11 锁, V0.2 不强校验)
- ✅ **check_docs.py** 通过 (修 query 兼容)

### 29.9.6 0.5F LOCK FINAL 前必过清单 (PIA §15 同步)

- [x] PIA V0.1 12 锁 (commit `bda8134` 同期)
- [x] 5 张新 wireframe 落地 (CH-01 / CD-01 Workspace / CD-01 Detail / 02-sources 重写 / E-40)
- [x] E-41 Network Path Spec 锁 (本轮)
- [x] `check_docs.py` PASS (含 ?query 兼容)
- [ ] NAVIGATION §3 表更新 (CH-01/CD-01/E-40 加进 BROADCAST/ENGINEERING 域)
- [ ] PIA §15 验证清单全部勾选
- [ ] ROADMAP 同步 47 表面口径

### 29.9.7 0.5F 后续 (0.5G / 0.5H)

| 轮次 | 范围 | 状态 |
|---|---|---|
| **0.5G** | E-41 Network Path wireframe; P-20 加 "by Channel" Tab (PIA §15 §6 项调整) | 待启动 |
| **0.5H** | Network Source Security 8 字段实装 (PIA §11, V0.3 起步) | 待启动 |
| **0.5E** | 4-Layer + Impact Preview + Configuration Diff + Command Palette 全部跨域 | 待启动 |
| **0.5 LOCK FINAL** | 0.5A/B/C/D/E/F/G/H 全部完成, PIA/MILESTONES/NAVIGATION/README/ROADMAP 同步 | 待启动 |

---

# 30. 附录：Phase 0.5B 语义收口项总清单（36 项 = 31 P0 + 5 P1）

> **目的**: 让 "N 项语义收口" 的宣称可用本文档逐项核对（此前 README 宣称 28 项但正文无 1..28 清单, 实际合计 36 项）。
> **口径**: 收口项 = 跨表面/全局语义决策; 不含各表面内部字段级定义。

## 30.1 Phase 0.5B.0 — 13 项 P0 语义边界（commit `50cf5a6`, 标签 SP-P0-*）

| # | 标签 | 内容 | 落点 |
|---|---|---|---|
| 1 | SP-P0-1 | Baseline metadata 对齐（V0.2.4 / 22 review / Errata-14 / 7 Health Invariants） | 文档头 YAML |
| 2 | SP-P0-2 | 表面计数口径统一（0.5A 10 + 0.5B 28 = 38; Closure-1 后另加 CD-01） | §1 |
| 3 | SP-P0-3 | Architecture Object Exposure Matrix（DIRECT / INDIRECT / SYSTEM_INTERNAL / NON_UI 四级） | §11 |
| 4 | SP-P0-4 | Output Profile / Variant / Destination 三元组语义焊死（Closure-1 升格为 4 元组） | §4.1 + §20 |
| 5 | SP-P0-5 | M-12 5 Tabs 锁定（Overview / Versions / QC / Rights / History） | M-12 |
| 6 | SP-P0-6 | P-21 补广播级字段（SAR / Field Order / Color Space / HRD / Closed GOP / Ref Frames / Audio Layout / Bit Depth） | P-21 + §18 |
| 7 | SP-P0-7 | P-21 Hardware Encoder 改为 Runtime Discovery 驱动 | P-21 + §18 Section 5 |
| 8 | SP-P0-8 | P-22 V0.2 Supported 与 Reserved/Future 分开（DASH / DRM / SDI Master 入 Reserved） | P-22 + §20.2 |
| 9 | SP-P0-9 | E-32 Resource Vector 9-Dim 完整表达 | §21.1 |
| 10 | SP-P0-10 | E-35 / E-36 硬件样例改 [Sample Host] / [Runtime Discovered] | E-35 / E-36 |
| 11 | SP-P0-11 | E-37 Clock 事件 vocabulary 收紧（CLOCK_DEGRADED / CLOCK_FAILED, 去 CLOCK_LOST） | E-37 |
| 12 | SP-P0-12 | E-33 ChangeSet Business Status 与 Execution Phase 分离（ABORTED 是 Phase） | E-33 |
| 13 | SP-P0-13 | O-42 Alert Rule Auto Action 从 §8.9 Failure Domain Policy 继承（UI 不可覆盖） | O-42 |

## 30.2 Phase 0.5B Closure-1 — 10 项产品化收口（commit `270daa3`）

| # | 内容 | 落点 |
|---|---|---|
| 1 | Configuration / Compiled / Effective 3-Layer Model（全局 pattern） | §15 |
| 2 | VBMF Design System（4 套状态语义分离 + 颜色系统） | §16 + DESIGN_SYSTEM.md |
| 3 | Channel Detail（CD-01）新增 8-Tab 子页 | §17 |
| 4 | P-21 Profile Builder 10 Sections + Preset + Why Not Usable | §18 |
| 5 | M-14 Transcode Workflow（Preview → Test → Submit）+ Worker=AUTO + Result 区 | §19 |
| 6 | P-22 Output 4-Tuple（Profile/Variant/Destination/Adapter）+ 3-Tier | §20 |
| 7 | E-32 Preflight 9D Required / Available / Delta / Headroom | §21 |
| 8 | O-41 Health Tree H1-H7 + Failure Absorbed + redundancy_group 视觉化 | §22 |
| 9 | E-34 Capability Why Compatible / Why Not + Static vs Runtime | §23 |
| 10 | Dependency / Impact Preview 全局 pattern | §24 |

## 30.3 Phase 0.5B.2 — 8 项 P0（commit `cec7407`）

| # | 标签 | 内容 | 落点 |
|---|---|---|---|
| 1 | P0-1 | Signal Contract / timebase 输入来源闭合（架构层） | §28.1 |
| 2 | P0-2 | Health Snapshot freshness / stale 语义（FRESH / STALE） | §28.1 |
| 3 | P0-3 | NIC resource per-device（token + per-interface） | §28.1 |
| 4 | P0-4 | P-21 wireframe DESIRED / COMPILED / EFFECTIVE 语义修正 | §28.1 + P-21 wireframe |
| 5 | P0-5 | M-12 Asset Version vs Output Variant 命名分离 | §28.1 + M-12 wireframe |
| 6 | P0-6 | P-22 RTP 加入 + Latency 三拆 + Edge Policy 移交 P-27 | §28.1 + §5 + §20.2 + P-22 wireframe |
| 7 | P0-7 | M-14 Test Encode → Mini Acceptance Test | §28.1 + M-14 wireframe |
| 8 | P0-8 | 全局 Design System + State Taxonomy（新文档） | DESIGN_SYSTEM.md |

## 30.4 Phase 0.5B.2 — 5 项 P1（commit `cec7407`）

| # | 内容 | 落点 |
|---|---|---|
| 1 | M-11 Saved Views（7 个预置视图） | §28.3 + M-11 wireframe |
| 2 | Profile Diff（M-14 Wizard Step 2 显示 v3 vs v4） | §28.3 + M-14 wireframe |
| 3 | M-14 Use in Playout Safety Gate（6 检查） | §28.3 + M-14 wireframe |
| 4 | M-12 Rights Override 升 L3（Who / Why / Scope / Expiry / Audit Reference） | §28.3 + M-12 wireframe |
| 5 | Delete Safety（Delete 检查 Used By 多维, 不只 Use Count） | §28.3 + P-21/P-22 wireframe |

> **历史勘误**: 根 README 曾宣称 "28 项语义收口", 无从溯源（疑似 13+10+5 误算）。正确合计 = **13 + 10 + 8 + 5 = 36 项（31 P0 + 5 P1）**, 以本附录为准。
> §26 的 92/100 是 0.5B.2 之前的中期自评, §28.4 的 94/100 是 0.5B.2 完成后的终评, 两者时点不同, 不构成矛盾; LOCK 判定以 §28.4 为准。

---

**VBMF Contributors** · VBMF UI/UX Surface Specification V0.3 · Phase 0.5B Closure-1 + 0.5B.2 Product UX/Semantic Closure + 0.5C Information Architecture Closure + 0.5C.1 一致性收口 + 0.5D P0 Product Surfaces + 0.5F Channel/Network UX Closure

