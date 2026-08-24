# VBMF UI/UX Surface Specification V0.2

> **文档定位:** V0.2 架构对象 → VBMF Console UI 表面 的完整映射
>
> **适用版本:** VBMF V0.2 LOCK FINAL + Phase 0.5A LOCK FINAL
>
> **关联文档:**
> - [`docs/architecture/ARCHITECTURE_V0.2.md`](../architecture/ARCHITECTURE_V0.2.md) — V0.2 架构基线
> - [`docs/phase-0.5/README.md`](../phase-0.5/README.md) — Phase 0.5A Operator Semantics (LOCK FINAL)
> - [`docs/phase-0.5/ERRATA.md`](../phase-0.5/ERRATA.md) — Phase 0.5A 20 项修复归档
> - [`docs/phase-0.6/README.md`](../phase-0.6/README.md) — Executable Acceptance Specification
> - [`docs/phase-0.5b/README.md`](README.md) — Phase 0.5B 阶段介绍

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

## 1. 6 大工作域 (Work Domains)

| # | 工作域 | 角色 | UI 表面数 | 状态 |
|---|---|---|---|---|
| 01 | **Broadcast 播控** | Operator / Director | 9 Core + 0 (Phase 0.5A) | 🟢 LOCK FINAL |
| 02 | **Media 媒体资产** | Director / Engineer | 6 (M-11~16) | 🔴 待定义 |
| 03 | **Profiles 配置** | Engineer | 7 (P-21~27) | 🔴 待定义 |
| 04 | **Engineering 工程** | Engineer | 7 (E-31~37) | 🟡 2 已 LOCK + 5 待定义 |
| 05 | **Operations 运维** | Operator / Engineer | 5 (O-41~45) | 🟡 1 已 LOCK + 4 待定义 |
| 06 | **Administration 平台管理** | Admin | 5 (A-51~55) | 🔴 待定义 |
| +1 | **State Reference 状态参考** | 全员 | 1 (10-states) | 🟢 LOCK FINAL (Validation) |

**总数:** ~40 个 UI 工作面（其中 10 LOCK FINAL，~30 待 Phase 0.5B 定义）

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
**Tab 区:**
- **M-12a Versions**: 列表 (Version名/类型/编码/分辨率/大小/时间/状态) + `[+ Create Version]`
- **M-12b QC**: qc_profile + 检测项 (Black/Freeze/Audio/Loudness/AV Sync) 阈值与实测 + `[Re-run QC]` / `[Change QC Profile]`
- **M-12c Rights**: 列表 (地域/平台/起始/截止/状态) + `[Block]` `[Extend]` `[Override]` `[Audit]`
- **M-12d History**: 时间线 (谁/何时/改了什么) + 可回滚

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

**详情 (5 区):**

#### Video
- Codec (H.264 / H.265 / VP9 / AV1)
- Profile (Baseline / Main / High)
- Level (3.0 / 3.1 / 4.0 / 4.1 / 5.0 / 5.1 / 5.2)
- Resolution (1920×1080 / 1280×720 / 3840×2160)
- FPS (25 / 30 / 50 / 60)
- Pixel Format (yuv420p / yuv422p / yuv444p)
- Bitrate Mode (CBR / VBR / Capped VBR)
- Bitrate (Mbps)
- GOP (12 / 25 / 50 / 100 / 250)
- B-Frames (0 / 2 / 4)
- Lookahead (0 / 10 / 20)
- Threads (1 / 2 / 4 / 8 / auto)
- Hardware Encoder (NVENC / QSV / VideoToolbox / x264 / x265 / libvpx)

#### Audio
- Codec (AAC / Opus / MP3 / Vorbis)
- Sample Rate (44.1k / 48k)
- Channels (1 / 2 / 6 / 8)
- Bitrate (kbps)

#### Container
- MPEG-TS / fMP4 / MP4 / MOV / MKV

#### Advanced
- Preset (ultrafast / superfast / veryfast / faster / fast / medium / slow)
- Tune (film / animation / grain / stillimage / zerolatency)
- Latency Mode (Normal / Low Latency)
- Metadata (复制 / 重写 / 移除)
- Timecode (preserve / drop / re-stamp)

#### Validation
- ✓ Compatible
- ✓ Resource OK
- ✓ Codec supported (服务器端)
- ✓ Test Encode OK (sample test)

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

**详情 (6 区):**

#### Protocol & Destination
- Protocol (HLS / RTMP / WebRTC / SRT / UDP MPEG-TS / File / DASH)
- Host / IP
- Port
- Stream Key / Path
- Transport (TCP / UDP / QUIC)
- SRS Gateway / CDN Endpoint

#### HLS Specific
- Segment Duration (1s / 2s / 4s / 6s)
- Playlist Window (3 / 5 / 10 segments)
- Codec (H.264 / H.265)
- Latency Mode (LL-HLS / Normal HLS)
- DRM (none / Widevine / FairPlay / PlayReady)

#### RTMP Specific
- URL (rtmp://host:port/app/stream)
- Backup URL (failover)
- Reconnect Policy (immediate / 1s / 5s)

#### WebRTC Specific
- ICE Servers (STUN / TURN)
- Signaling URL
- DTLS / SRTP enabled
- Bitrate cap

#### Latency / Reliability
- Latency Target (50 / 100 / 200 / 500ms)
- Reconnect Policy
- Failover Destination (URL)
- CDN (Cloudflare / Akamai / Aliyun)

#### Player Capability
- Player Hint (Safari / Chrome / Android / iOS)
- Required Codecs
- Auto Transcode on demand

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

#### Resource
- ✓ CPU available
- ⚠ GPU (选配, 提示)
- ✓ RAM available
- ✓ NIC bandwidth
- ✓ PCIe bandwidth
- ✓ BMD 设备 available

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

#### Status 状态机
```
DRAFT → VALIDATED → APPLIED → (ROLLED_BACK)
              ↓
          ABORTED
```

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

**按 Host 分组:**

#### Host: 10.30.15.10
- **CPU:** 32 threads
- **RAM:** 30 GB
- **GPU:** None
- **BMD:**
  - DeckLink Mini Monitor 4K (serial xxx)
    - Port 1 IN / Port 2 IN / Port 3 OUT
    - Driver / Firmware
    - Current Lock
    - Temperature
    - Health
- **NIC:**
  - eth0 (10GbE)
  - eth1 (1GbE management)
- **NVMe:**
  - /dev/nvme0n1 (1TB · 38% used)
- **Clock:**
  - PTP0 · ptp0 (eth0) · LOCKED · BROADCAST_GRADE

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

**实时面板:**
- CPU ████████░░ 78%
- RAM ██████░░░░ 62%
- NIC IN ████░░░░░░ 41%
- NIC OUT ██████░░░░ 66%
- Disk Write ███████░░░ 72%
- PCIe RX ██████░░░░ 61%
- BMD Input 3/3
- BMD Output 2/3

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
**历史事件:** CLOCK_DEGRADED / CLOCK_LOST / FALLBACK_TRIGGERED

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
- 字段: Rule Name · Trigger Condition (e.g. SRT packet loss > 0.5% for 5s) · Severity · Escalation · Auto Action
- 操作: `[+ Create Rule]` · `[Test]` · `[Edit]` · `[Delete]`

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

# 11. 架构对象映射总表 (V0.2 → UI 表面)

| 架构对象 (V0.2) | UI 表面 |
|---|---|
| `media_assets` | M-11, M-12, M-13 |
| `asset_versions` | M-12a, M-16 |
| `asset_rights` | M-12c, P-26 |
| `media_jobs` | M-14, M-15 |
| `media_job_attempts` | M-15 |
| `uploads` | M-13 |
| `encoding_profiles` | P-21, M-14 (选择器) |
| `output_profiles` | P-22, 0.5A #06 (选择器) |
| `audio_profiles` | P-23, 0.5A #05 (选择器) |
| `graphic_profiles` | P-24, 0.5A #04 (选择器) |
| `qc_profiles` | P-25, M-12b, 0.5A #09 (引用) |
| `rights_profiles` | P-26, M-12c |
| `edge_policy_profiles` | P-27, 0.5A #08 (Edge Inspector 引用) |
| `playlists` | 0.5A #04 (Timeline) |
| `composition_templates` | P-24, 0.5A #04 |
| `composition_layers` | 0.5A #04, P-24 |
| `signal_contracts` | E-34 |
| `node_contracts` | E-34 |
| `preflight_runs` | E-32 |
| `config_revisions` | E-33 |
| `change_sets` | E-33 |
| `change_set_items` | E-33 |
| `channels` | 0.5A #01, 0.5A #03, 0.5A #06 |
| `media_session_runtime` | 0.5A #01 (Status), 10-states |
| `incidents` | O-42, O-43 |
| `alert_rules` | O-42 |
| `alert_events` | O-42 |
| `failover_benchmarks` | O-45 |
| `latency_probes` | O-45, 0.5A #06 |
| `signal_pool` | (Prometheus 内部, 暂不暴露 UI) |
| `signal_current_state` | 0.5A #01, 0.5A #09 |
| `health_tree_nodes` | 0.5A #09 |
| `channel_health_aggregation` | 0.5A #09 |
| `channel_health_view` | 0.5A #01, 0.5A #09 |
| `users` | A-51 |
| `roles` | A-52 |
| `permissions` | A-53 |
| `user_roles` | A-51, A-52 |
| `audit_logs` | A-54 |
| `api_keys` | A-51 (子表) |
| `sessions` | A-51 (子表) |
| `oauth_tokens` | A-51 (子表) |
| `system_settings` | A-55 |
| `device_registry` | E-35 |
| `hardware_capability` | E-35, E-36 |
| `clock_fallback_chain` | E-37, 0.5A #02 |

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

1. **架构对象必须 1:1 映射到 UI 表面** — 否则 Phase 1 / 4 实施时会不断回头问"放哪里"
2. **Profile 必须与 Runtime 分离** — Encoding Profile ≠ Output Profile ≠ Channel; UI 也要分
3. **危险操作必须 L1/L2/L3 分级** — 不能所有操作都同等对待
4. **审计是 Admin 的第一公民** — 不是"以后再加"
5. **资源容量必须有 Plan 视图** — 不仅"现在用了多少", 还要"如果新增 X 会怎样"
6. **6 状态样例 (Normal/Loading/Empty/Error/Warning/Critical) 必须每页都有** — 缺一视为不完整

---

**VBMF Contributors** · VBMF UI/UX Surface Specification V0.2 · Phase 0.5B Product UI Surface Closure

