# VBMF i18n Contract V0.1

> **文档定位:** VBMF Console i18n 规范, 用于 Phase 4 Web Console 实施
>
> **适用版本:** VBMF V0.2 LOCK FINAL + Phase 0.5A LOCK FINAL + Phase 0.5B Surface Spec
>
> **关联文档:**
> - [`SURFACE_SPEC.md`](SURFACE_SPEC.md) — V0.2 架构对象 → UI 表面
> - [`docs/phase-0.5/README.md`](../phase-0.5/README.md) — Phase 0.5A Operator Semantics
> - [`docs/architecture/ARCHITECTURE_V0.2.md`](../architecture/ARCHITECTURE_V0.2.md) — V0.2 架构基线 (Canonical Vocabulary SoT)

---

## 0. 目的

Phase 0.5A wireframe 现状:
- HTML 里直接 hard-code 字符串如 `HEALTHY 健康`, `RUNNING`, `FRAME`
- 没有 locale 概念, 切换语言需要改源码
- Canonical Vocabulary 与本地化字符串混杂

**i18n Contract V0.1 目标:**
- 定义 2 个 Locale (zh-CN / en-US)
- 明确 **Canonical Vocabulary** (不翻译) vs **Localizable Strings** (翻译)
- 定义 enum label 翻译表 (semantic key → zh-CN / en-US)
- 定义格式约定 (date / time / number / bitrate / latency / loudness)
- 让 Phase 4 实施时**所有** UI 字符串从 i18n key 拉取, 不写死

---

## 1. Locale 锁定

```yaml
locales:
  - code: zh-CN
    name: 简体中文
    direction: ltr
    is_default: true
  - code: en-US
    name: English (US)
    direction: ltr

fallback_chain:
  - zh-CN  # default
  - en-US  # fallback if zh-CN missing
```

**V0.1 锁定: 仅支持 zh-CN + en-US 两种 Locale。**
其他 Locale (zh-TW / ja-JP / ko-KR) 留作未来, 不在 0.5B.1 / Phase 4 实施范围。

---

## 2. Canonical Vocabulary (不翻译) — 锁定

> **V0.2 Canonical Vocabulary 是 Single Source of Truth (SST)。**
> UI 字符串**永远**使用以下原文, 不允许翻译/拼音/简写。

### 2.1 V0.2 架构核心术语 (必须保留原文)

```yaml
canonical_terms:
  # Engines
  - Source
  - Switcher
  - Playout
  - Composition
  - Audio
  - Output
  - Recording
  - Replay
  - Encoder
  - Decoder
  - Adapter
  - Worker

  # 横切能力 X1-X6
  - Graph_Compiler
  - Preflight
  - Configuration_Versioning
  - Incident_Timeline
  - Health_Tree
  - Capability_Registry

  # Switch Mode 3
  - PACKET_SWITCH
  - FRAME_SWITCH
  - MASTER_SWITCH
  - REJECT

  # Hot-Standby 3
  - COLD
  - WARM
  - HOT

  # Runtime 域 / 9
  - Lifecycle
  - Readiness
  - Health
  - Switch_Mode
  - Standby_Semantics
  - Failure_Domains
  - Health_Tree
  - Channel_Status
  - Clock

  # 三轴状态
  - RUNNING
  - STARTING
  - STOPPED
  - STOPPING
  - READY_TO_TAKE
  - NOT_READY
  - HEALTHY
  - DEGRADED
  - FAILED
  - UNKNOWN

  # Failure Domain 7
  - SOURCE
  - PIPELINE
  - MASTER
  - OUTPUT
  - RECORDING
  - CLOCK
  - RESOURCE
  # DiagnosticFailureClass 2 (separate)
  - PLAYER
  - UNKNOWN_DIAGNOSTIC

  # 协议 / 格式
  - HLS
  - RTMP
  - WebRTC
  - SRT
  - UDP
  - MPEG-TS
  - fMP4
  - MP4
  - MOV
  - MKV

  # 编解码
  - H.264
  - H.265
  - HEVC
  - VP9
  - AV1
  - AAC
  - Opus
  - MP3
  - Vorbis
  - Theora

  # 时钟
  - PTP
  - TIMECODE
  - SYSTEM
  - MONOTONIC
  - BROADCAST_GRADE
  - LOCKED
  - FALLBACK

  # 音频
  - LUFS
  - LUFS-I
  - LUFS-S
  - EBU_R128
  - dBTP
  - LRA
  - dBFS
  - AV_Sync

  # 资源 / 硬件
  - CPU
  - GPU
  - VRAM
  - RAM
  - NIC
  - PCIe
  - BMD
  - NVMe
  - ETH
  - 10GbE

  # 安全 / 操作
  - READ
  - WRITE
  - ADMIN
  - L1
  - L2
  - L3

  # 工具 / 技术
  - TS
  - Rust
  - JSON
  - JSON_Schema
  - PG
  - PostgreSQL
  - Prometheus
  - SRS
  - FFmpeg
```

### 2.2 UI 显示规则

```html
<!-- WRONG: 不允许翻译 -->
<span>帧切换</span>
<span>水平开关</span>

<!-- CORRECT: 保留原文 + 中文括号 -->
<span>FRAME_SWITCH (帧切换)</span>
<span>Frame Switch / 帧切换</span>
```

**注意:** Canonical Vocabulary 可以**与中文并列**显示, 但**不能**单独翻译成中文。

---

## 3. Localizable Strings (翻译) — 锁定 i18n Key

### 3.1 i18n Key 命名规范

```yaml
key_style:
  format: "{page}.{section}.{component}.{state}"
  example: "broadcast.dashboard.intent.now"
  separator: "."
  case: snake_case
```

**例子:**

```yaml
# Page level
broadcast.dashboard.title
broadcast.dashboard.system_state_bar.pgm

# Section level
broadcast.dashboard.intent.now
broadcast.dashboard.intent.next
broadcast.dashboard.intent.take

# Component state level
media.library.status.uploading
media.library.status.ingesting
media.library.status.ready
media.library.status.qc_failed
media.library.status.rights_blocked
```

### 3.2 翻译表 (enum labels — 必须有)

> **Critical:** V0.1 必须为**所有**出现在 UI 的 enum 提供**双语**翻译。

#### HealthState 翻译表

```yaml
status.health:
  healthy:
    zh-CN: 健康
    en-US: Healthy
  degraded:
    zh-CN: 降级
    en-US: Degraded
  failed:
    zh-CN: 失败
    en-US: Failed
  starting:
    zh-CN: 启动中
    en-US: Starting
  stopped:
    zh-CN: 已停止
    en-US: Stopped
  unknown:
    zh-CN: 未知
    en-US: Unknown
```

#### LifecycleState 翻译表

```yaml
status.lifecycle:
  running:
    zh-CN: 运行中
    en-US: Running
  starting:
    zh-CN: 启动中
    en-US: Starting
  stopped:
    zh-CN: 已停止
    en-US: Stopped
  stopping:
    zh-CN: 停止中
    en-US: Stopped
```

#### ReadinessState 翻译表

```yaml
status.readiness:
  ready_to_take:
    zh-CN: 可接管
    en-US: Ready to Take
  not_ready:
    zh-CN: 未就绪
    en-US: Not Ready
```

#### SwitchMode 翻译表 (canonical + 中文括号)

```yaml
switch_mode:
  packet:
    canonical: PACKET_SWITCH
    zh-CN: 包切换
    en-US: Packet Switch
  frame:
    canonical: FRAME_SWITCH
    zh-CN: 帧切换
    en-US: Frame Switch
  master:
    canonical: MASTER_SWITCH
    zh-CN: 主母版切换
    en-US: Master Switch
  reject:
    canonical: REJECT
    zh-CN: 拒绝
    en-US: Reject
```

#### HotStandby 翻译表

```yaml
hot_standby:
  cold:
    zh-CN: 冷备
    en-US: Cold
  warm:
    zh-CN: 温备
    en-US: Warm
  hot:
    zh-CN: 热备
    en-US: Hot
```

#### FailureDomain 翻译表

```yaml
failure_domain:
  source:
    zh-CN: 源
    en-US: Source
  pipeline:
    zh-CN: 管道
    en-US: Pipeline
  master:
    zh-CN: 主母版
    en-US: Master
  output:
    zh-CN: 输出
    en-US: Output
  recording:
    zh-CN: 录制
    en-US: Recording
  clock:
    zh-CN: 时钟
    en-US: Clock
  resource:
    zh-CN: 资源
    en-US: Resource
  player:
    zh-CN: 播放端
    en-US: Player
  unknown_diagnostic:
    zh-CN: 未知 (诊断)
    en-US: Unknown (Diagnostic)
```

#### Clock State 翻译表

```yaml
clock_state:
  locked:
    zh-CN: 已锁定
    en-US: Locked
  degraded:
    zh-CN: 降级
    en-US: Degraded
  failed:
    zh-CN: 失败
    en-US: Failed
  fallback:
    zh-CN: 降级链触发
    en-US: Fallback Triggered
```

#### Media Asset Status 翻译表

```yaml
media.asset_status:
  uploading:
    zh-CN: 上传中
    en-US: Uploading
  ingesting:
    zh-CN: 导入中
    en-US: Ingesting
  probing:
    zh-CN: 探测中
    en-US: Probing
  ready:
    zh-CN: 就绪
    en-US: Ready
  transcoding:
    zh-CN: 转码中
    en-US: Transcoding
  qc_failed:
    zh-CN: 质量检测失败
    en-US: QC Failed
  qc_passed:
    zh-CN: 质量检测通过
    en-US: QC Passed
  rights_blocked:
    zh-CN: 版权阻止
    en-US: Rights Blocked
  archived:
    zh-CN: 已归档
    en-US: Archived
  failed:
    zh-CN: 失败
    en-US: Failed
```

#### ChangeSet Status 翻译表

```yaml
changeset.status:
  draft:
    zh-CN: 草稿
    en-US: Draft
  validated:
    zh-CN: 已校验
    en-US: Validated
  applied:
    zh-CN: 已应用
    en-US: Applied
  rolled_back:
    zh-CN: 已回滚
    en-US: Rolled Back
```

#### ChangeSet Phase 翻译表 (与 Status 分离)

```yaml
changeset.phase:
  preparing:
    zh-CN: 准备中
    en-US: Preparing
  applying:
    zh-CN: 应用中
    en-US: Applying
  committed:
    zh-CN: 已提交
    en-US: Committed
  aborted:
    zh-CN: 已中止
    en-US: Aborted
```

#### Clock Event 翻译表 (E-37)

```yaml
clock_event:
  degraded:
    zh-CN: 时钟降级
    en-US: Clock Degraded
  failed:
    zh-CN: 时钟失败
    en-US: Clock Failed
  fallback_triggered:
    zh-CN: 降级链触发
    en-US: Fallback Triggered
```

#### Dangerous Action 翻译表

```yaml
dangerous_action:
  l1:
    zh-CN: 普通操作
    en-US: Normal Operation
  l2:
    zh-CN: 重要操作 (需二次确认)
    en-US: Important (Confirm Required)
  l3:
    zh-CN: 危险操作 (需输入确认)
    en-US: Dangerous (Type to Confirm)
```

### 3.3 UI 中应**避免**的 hard-coded 字符串 (反模式)

```typescript
// WRONG
<span>{status} 健康</span>
<button>{action} 切播</button>
<div>HOT (100ms)</div>

// CORRECT
<span>{t(`status.health.${status}`)}</span>
<button>{t(`action.take.button`)}</button>
<div>{t('hot_standby.hot')} ({t('policy.target_ms', { ms: 100 })} / {t('measured.p95_ms', { ms: 87 })})</div>
```

---

## 4. Formatting 约定 (本地化感知)

```yaml
formatting:
  date:
    zh-CN: "2026-08-25"
    en-US: "2026-08-25"
  time:
    zh-CN: "14:25:36"   # 24h
    en-US: "2:25:36 PM"  # 12h
  datetime:
    zh-CN: "2026-08-25 14:25:36 CST"
    en-US: "Aug 25, 2026 2:25:36 PM CST"
  timezone:
    default: Asia/Shanghai  # user profile setting
    overrides: per channel
  number:
    decimal_separator:
      zh-CN: "."
      en-US: "."
    thousands_separator:
      zh-CN: ","
      en-US: ","

  # 广播单位 (canonical, 不本地化)
  bitrate:
    unit: "Mbps"  # 始终 Mbps, 不变
    example: "5.0 Mbps"
  latency:
    unit: "ms"  # 始终 ms
    example: "87 ms"
  loudness:
    unit: "LUFS"  # 始终 LUFS
    example: "-23.4 LUFS"
  temperature:
    unit: "°C"  # 摄氏度
    example: "45 °C"
  size:
    unit: "MB" / "GB" / "TB"
    example: "750 MB"
  duration:
    unit: "HH:MM:SS"  # 始终绝对格式
    example: "02:14:37"
```

**关键:**
- 数字格式本地化 (千分位等)
- 广播单位**不**本地化 (Mbps / ms / LUFS / dBTP 始终用国际标准)
- 时区 per user / per channel 配置

---

## 5. Pluralization 规则

```yaml
pluralization:
  zh-CN:
    rule: single_only
    example: "1 个资产" / "5 个资产"  # 中文无复数
  en-US:
    rule: ICU_plural
    categories: [zero, one, two, few, many, other]
    example: "1 asset" / "5 assets"
```

Phase 4 实施时使用 `Intl.PluralRules` API。

---

## 6. Interpolation 规则

```yaml
interpolation:
  style: "{variable_name}"
  example: "切换到 {target} 用时 {duration} ms"
  zh-CN: "{target} 切换完成 (耗时 {duration} ms)"
  en-US: "Switched to {target} (took {duration} ms)"
```

**禁止:** 字符串拼接 `target + ' 切换完成 (耗时 ' + duration + ' ms)'`

---

## 7. 错误码翻译表

```yaml
error_codes:
  e_001_channel_not_found:
    zh-CN: 通道不存在
    en-US: Channel not found
  e_002_profile_invalid:
    zh-CN: Profile 校验失败
    en-US: Profile validation failed
  e_003_preflight_failed:
    zh-CN: Preflight 未通过
    en-US: Preflight failed
  e_004_changeset_conflict:
    zh-CN: 变更集冲突
    en-US: Change Set conflict
  e_005_device_unavailable:
    zh-CN: 设备不可用
    en-US: Device unavailable
  e_006_clock_degraded:
    zh-CN: 时钟降级
    en-US: Clock degraded
  e_007_resource_exhausted:
    zh-CN: 资源耗尽
    en-US: Resource exhausted
  e_008_rights_blocked:
    zh-CN: 版权阻止
    en-US: Rights blocked
```

---

## 8. 与 Phase 4 实施约束

**Phase 4 Web Console 必须:**
- ✅ 所有 UI 字符串从 i18n key 拉取
- ✅ 切换 Locale 不需要改源码
- ✅ Canonical Vocabulary 用 enum / constant (不本地化)
- ✅ Localizable Strings 用 i18n key (本地化)
- ✅ Format 数字 / 日期 / 时间用 Intl API
- ✅ 单位 (Mbps / ms / LUFS) 用固定常量

**Phase 4 实施后:**
- ❌ 不允许在 JSX / TSX 写 "HEALTHY 健康" 这种拼接
- ❌ 不允许在代码里写 "10 ms" 之类不带单位的 latency
- ❌ 不允许 hard-code 日期格式 "YYYY-MM-DD"
- ❌ 不允许把 Canonical Vocabulary 翻译成中文

---

## 9. 与 0.5A wireframe 的桥接

Phase 0.5A wireframe 当前是 prototype, hard-coded 字符串可以接受。
但**正式 Phase 4 实施**时, 每个字符串必须迁移到 i18n key。

**桥接策略 (Phase 0.5B.1 / 0.5B.2):**
1. Phase 0.5B.1 选 5 张 P0 wireframe 实施时, 直接用 i18n key 替换 hard-code
2. Phase 0.5B.2 全部 wireframe 迁移
3. Phase 0.5A wireframe (9 Core + 1 Validation) 在 Phase 4 实施时迁移

**i18n Key 与 0.5A wireframe 字段对照表 (待 Phase 0.5B.1 完成):**
- 01-dashboard: `broadcast.dashboard.*`
- 02-sources: `broadcast.sources.*`
- 03-switcher: `broadcast.switcher.*`
- 04-composition: `broadcast.composition.*`
- 05-audio: `broadcast.audio.*`
- 06-output: `broadcast.output.*`
- 07-recording: `broadcast.recording.*`
- 08-graph-designer: `engineering.graph_designer.*`
- 09-health-tree: `operations.health_tree.*`
- 10-states: `validation.states.*`
- M-11: `media.library.*`
- ... (后续)

---

## 10. V0.1 范围与未来扩展

**V0.1 锁定:**
- 2 Locale: zh-CN (default) / en-US
- Canonical Vocabulary (不翻译)
- 11 个 enum 翻译表 (HealthState / Lifecycle / Readiness / SwitchMode / HotStandby / FailureDomain / Clock / MediaAssetStatus / ChangeSet Status+Phase / Clock Event / DangerousAction)
- Formatting 约定
- Pluralization (zh-CN single, en-US ICU)
- Interpolation
- Error Code 翻译表

**V0.2 未来 (不阻塞 Phase 0.5B.1):**
- 增加 zh-TW / ja-JP / ko-KR
- Translation Memory / 翻译记忆库
- RTL 语言支持
- 动态翻译加载 (lazy load)

---

**VBMF Contributors** · VBMF i18n Contract V0.1 · 锁定 zh-CN + en-US
