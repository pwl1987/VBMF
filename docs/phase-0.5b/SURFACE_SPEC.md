# VBMF UI/UX Surface Specification V0.2

> **文档定位:** V0.2 架构对象 → VBMF Console UI 表面 的完整映射
>
> **适用版本:** VBMF V0.2 LOCK FINAL + Phase 0.5A LOCK FINAL
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
> baseline_sot: docs/architecture/ARCHITECTURE_V0.2.md
> ```
>
> **关联文档:**
> - [`docs/architecture/ARCHITECTURE_V0.2.md`](../architecture/ARCHITECTURE_V0.2.md) — V0.2 架构基线 (192KB / 4021 行 / 22 轮 review)
> - [`docs/phase-0.5/README.md`](../phase-0.5/README.md) — Phase 0.5A Operator Semantics (LOCK FINAL)
> - [`docs/phase-0.5/ERRATA.md`](../phase-0.5/ERRATA.md) — Phase 0.5A 20 项修复归档
> - [`docs/phase-0.6/README.md`](../phase-0.6/README.md) — Executable Acceptance Specification
> - [`docs/phase-0.5b/README.md`](README.md) — Phase 0.5B 阶段介绍
> - [`docs/phase-0.5b/I18N_SPEC.md`](I18N_SPEC.md) — i18n Contract (Locale / Canonical Terms / Enum Labels)

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

## 1. 6 大工作域 (Work Domains) — 计数口径统一

| # | 工作域 | 角色 | UI 表面数 | 来源 | 状态 |
|---|---|---|---|---|---|
| 01 | **Broadcast 播控** | Operator / Director | 9 Core | 0.5A LOCK | 🟢 LOCK FINAL |
| 02 | **Media 媒体资产** | Director / Engineer | 6 新 (M-11~16) | 0.5B 新增 | 🔴 待定义 |
| 03 | **Profiles 配置** | Engineer | 7 新 (P-21~27) | 0.5B 新增 | 🔴 待定义 |
| 04 | **Engineering 工程** | Engineer | 2 升级 (E-31) + 6 新 (E-32~37) | 0.5A #08 + 0.5B 新增 | 🟡 部分 LOCK + 6 待定义 |
| 05 | **Operations 运维** | Operator / Engineer | 1 升级 (O-41) + 4 新 (O-42~45) | 0.5A #09 + 0.5B 新增 | 🟡 部分 LOCK + 4 待定义 |
| 06 | **Administration 平台管理** | Admin | 5 新 (A-51~55) | 0.5B 新增 | 🔴 待定义 |
| +1 | **State Reference 状态参考** | 全员 | 1 (10-states) | 0.5A LOCK (Validation) | 🟢 LOCK FINAL |

**口径说明 (避免歧义):**
- **0.5A 锁定的 UI 表面**: 9 Core (01-09) + 1 Validation (10-states) = **10**
- **0.5B 新增 UI 表面**: M(6) + P(7) + E(6) + O(4) + A(5) = **28**
- **从 0.5A 升级到 0.5B 工作域的 UI 表面**: E-31 (Graph Designer 升级到 Engineering) + O-41 (Health Tree 升级到 Operations) = **2 升级** (升级 = 重新归类, 不是新增)
- **0.5B 完成后的总 UI 表面**: 10 (0.5A) + 28 (0.5B 新) = **38** (含 1 Validation)
- **不要再写 "30 / ~25 / ~35" 等模糊数字**

**Surface 编号约定 (锁定):**
- 0.5A 沿用 `01-09` 编号 (不变)
- 0.5B 新增使用 `M-11`/`P-21`/`E-31`/`O-41`/`A-51` 域前缀 + 序号
- 序号在每个域内连续, 跨域不连续 (避免重排 0.5A)

---

## 2. 全局规范

### 2.1 6 状态样例 (适用于每页)

每页设计稿必须包含 6 个状态样例，否则视为不完整：

| 状态 | 触发条件 | UI 表现 |
|---|---|---|
| **Normal 正常** | 业务无异常 | 全量数据 + 正常色码 (绿) |
| **Loading 加载中** | 首次 / 刷新 | Skeleton / Spinner + 灰底 |
| **Empty 空态** | 无数据 (新 Channel / 首次启动) | 引导 + "新建 / 导入" 主按钮 |
| **Warning 警告** | 软指标越界 (漂移 / 漂移率 / 磁盘 80%) | 黄色 + Alert Banner |
| **Error 错误** | 单次操作失败 (Encode 失败 / Profile 校验错) | 红色 + 错误信息 + 重试按钮 |
| **Critical 严重** | 业务中断 (Source 全 FAILED / Change Set 失败) | 红色脉冲 + Incident 入口 |

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
| **03 Profiles** 配置 | R (own channel) | R (own channel) | R+W (P-21~27) | A |
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
  ├─ Asset Detail (M-12)   ← 列表点击
  │   ├─ Versions Tab (M-12a)   ← 子 tab
  │   ├─ QC Tab (M-12b)
  │   ├─ Rights Tab (M-12c)
  │   └─ History Tab (M-12d)
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
- Loading: 首次/刷新, 6 行 Skeleton
- Empty: 0 assets + `[+ Upload Asset]` 主按钮 + "从录制导入" 副按钮
- Warning: 黄色 Banner "12 assets have QC issues, 3 have rights issues"
- Error: Probe 失败 / Hash 不匹配 + `[Retry Probe]`
- Critical: Storage > 90% 顶部红条

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
**操作:** `[View Detail]` `[Retry]` `[Open Asset]` `[Copy Log URL]` · 批量 `[Retry Selected]` `[Cancel Selected]` `[Export CSV]`

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
| **目标** | 集中管理所有输出目标配置 (SRS HLS / RTMP / WebRTC / File / UDP) |
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
| **File** (Archive) | Local / S3 / NFS | Path template | — |

#### Reserved / V0.4+ (UI 显示但标 "Reserved" — Backend 未实现)

| Protocol | 状态 | V0.2 表现 |
|---|---|---|
| SDI Master Output | V0.4 Target | Architecture Contract RESERVED · V0.2 DISABLED |
| DASH | Future | 灰显 + "DASH output is reserved for V0.4+, not configurable in V0.2" |
| DRM (Widevine / FairPlay / PlayReady) | Future | 灰显 + 提示 |

**V0.2 约束 (重要):** UI 不能让 V0.2 用户误以为 DASH/DRM/SDI 已经可配置。Reserved 协议必须显式标 "[Reserved · V0.2 Disabled]"。

#### Protocol & Destination (V0.2 supported 内的详细字段)
- Protocol (HLS / RTMP / WebRTC / SRT / UDP / File) — **V0.2 限定 6 种**
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
- Backup URL (failover)
- Reconnect Policy (immediate / 1s / 5s)

#### WebRTC Specific
- ICE Servers (STUN / TURN)
- Signaling URL (SRS WHIP API)
- DTLS / SRTP enabled
- Bitrate cap

#### Latency / Reliability
- Latency Target (50 / 100 / 200 / 500ms)
- Reconnect Policy
- Failover Destination (URL)
- CDN Endpoint (可选)

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
    ├── Protocol & Destination (V0.2 supported 字段)
    ├── HLS / RTMP / WebRTC / SRT / UDP / File 特定字段
    ├── Latency / Reliability
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

**状态:** 🟢 Phase 0.5A LOCK FINAL — 见 [`phase-0.5/wireframes/08-graph-designer.html`](../phase-0.5/wireframes/08-graph-designer.html)

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
| **主要操作** | Lock / Set Reference / Test / Fallback Trigger |
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

**状态:** 🟢 Phase 0.5A LOCK FINAL — 见 [`phase-0.5/wireframes/09-health-tree.html`](../phase-0.5/wireframes/09-health-tree.html)

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

**状态:** 🟢 Phase 0.5A LOCK FINAL — Incident → Replay 自动定位工作流 (在 Recording 07 页面内)

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
| Edit Profile | — | — | R+W | A |
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

**危险操作清单 (强制审计):**
- TAKE / FAILOVER / DISABLE OUTPUT
- CHANGE SET APPLY / ROLLBACK
- DELETE Asset / Profile / Output
- EDIT Profile / Permission
- LOCK / UNLOCK Device
- USER CRUD

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
- Default Dashboard Channel · Theme (Dark/Light)

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
| Sources | Source.A 配置 | Encoding Profile (P-21) |
| Switcher | TAKE 失败 | Health Tree (O-41) → Incident (O-43) |
| Output | HLS DEGRADED | Output Profile (P-22) |
| Recording | 点击 Incident | Replay (O-44) |
| Health Tree | Failed 节点 | Device Registry (E-35) |
| Composition | 拖入 Asset | Media Library (M-11) → Asset Detail (M-12) |
| Change Set | Impact 显示 | Resource / Capacity (E-36) |
| Asset Detail | Transcode 按钮 | Transcode Center (M-14) |
| Profile Editor | Codec 不可用 | Device Registry (E-35) |

## 9.3 抽屉 / 子页 (不计入主导航)

| 主页面 | 子页 |
|---|---|
| Media Library | Asset Detail (5 tab) |
| Transcode Center | Jobs / Versions |
| Asset Detail | Versions / QC / Rights / History (4 tab) |
| Health Tree | Operator / Engineering / Aggregation Rules (3 view) |
| Output | HLS Detail / WebRTC Detail (3 view) |
| Composition | Timeline / Composition (2 column) |
| Switcher | TAKE State Machine (5 状态 modal) |
| Incident | Replay Workspace |

---

# 10. 实施顺序 (P0 / P1 / P2 / Defer)

## 10.1 Phase 0.5B 内部优先级

| 优先级 | UI 表面 | 原因 |
|---|---|---|
| 🔴 **P0 必做** | M-11 (Library) + M-12 (Detail) + M-14 (Transcode) + P-21 (Encoding) + P-22 (Output) | 这些是 V0.2 核心架构对象, 缺它们 Phase 1 实施会不断回头问"放哪里" |
| 🟠 **P1 强烈建议** | M-13 (Upload) + M-15 (Jobs) + M-16 (Versions) + P-23~27 (其他 Profile) + E-32~37 (工程) | 让 Phase 1 / 4 有完整可参考 UI 表面 |
| 🟡 **P2 锦上添花** | O-42~45 (Operations 后续) + A-51~55 (Admin) | 后期再做也来得及, Admin 可直接用 SQL 临时方案 |
| ⚪ **Defer to Phase 4** | (无) | 0.5B 不实施 wireframe, 只定义; 实施在 Phase 4 |

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

**VBMF Contributors** · VBMF UI/UX Surface Specification V0.2 · Phase 0.5B Product UI Surface Closure

