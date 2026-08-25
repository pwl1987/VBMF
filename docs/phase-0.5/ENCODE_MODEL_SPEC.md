# ENCODE_MODEL_SPEC — Encoding Profile 双语义模型（File / Realtime）

> **状态**: 🟢 **SEMANTIC LOCKED 0.1 (Phase 0.5 SoT · 0.5F.1 晋升)** — Encoding Profile Schema 唯一事实源
> **来源**: 0.5C 提案 §12-§13；对齐 `SURFACE_SPEC.md` §29.3 / §3337（"P-21 schema 分 Common / Realtime / File 3 段，0.5D 实施"）
> **约束**: 不新增 Engine（PIA §9）；Encoding Profile 仅为配置对象，与 Output Profile（P-22）、Profile Bundle（P-28）解耦
> **关联**: P-21（Encoding Profile UI）· M-14（FILE_PROFILE consumer）· M-17（REALTIME_PROFILE consumer）· OBJECT_VOCABULARY §1.11（Job/Session）
> **权威链 (0.5F.1 SoT 定义)**: **ENCODE_MODEL_SPEC = Encoding Profile Schema SoT** → P-21 = Encoding Profile UI (读写) → M-14 = FILE_PROFILE consumer → M-17 = REALTIME_PROFILE consumer。P-21/M-14/M-17 不得自建 Schema 字段。

---

## 0. 核心结论

`Encoding Profile` **必须显式区分两种业务语义**，用 `profile_type` 枚举表达，而非让用户把"实时编码"伪装成一个普通 Job：

| `profile_type` | 业务语义 | 运行时包装 | 关键差异 |
|---|---|---|---|
| `FILE_PROFILE` | 文件转码（质量/效率优先，可排队/暂停/重试） | `Job`（FILE_TRANSCODE） | 不要求实时预算、热备、故障切换 |
| `REALTIME_PROFILE` | 实时编码（持续运行、低延迟、自动恢复、热备） | `Session`（MEDIA_SESSION） | 强制 Realtime 专属属性（见 §3） |

三端 schema 结构（与 SURFACE_SPEC §3337 一致）：

```
EncodingProfile
├── Common      (两类共享: Basic/Video/Audio/Container — 见 P-21 §392-470)
├── Realtime    (仅 REALTIME_PROFILE — 见 §3)
└── File        (仅 FILE_PROFILE — 见 §4)
```

---

## 1. `profile_type` 枚举（✅ 0.5D.3 已落地于 P-21）

```yaml
EncodingProfile:
  profile_type: enum[FILE_PROFILE, REALTIME_PROFILE]   # 必填, 创建后不可变
  common: CommonSegment
  realtime: RealtimeSegment   # 仅当 profile_type == REALTIME_PROFILE 时必填
  file: FileSegment           # 仅当 profile_type == FILE_PROFILE 时必填
```

> ✅ P-21 已落地 `profile_type` 单选 (FILE_PROFILE / REALTIME_PROFILE, 创建后不可变), 按类型切换 Realtime/File 段表单 (0.5D.3). 本 Spec 即其权威定义。

---

## 2. Common 段（引用 P-21，不再重定义）

直接复用 `SURFACE_SPEC.md` P-21 §392-470 的广播级字段，作为两类共享的"编码契约"：

- **Basic**: Profile Name / Description / Category / Tags
- **Video — Codec**: Codec / Profile / Level / Pixel Format
- **Video — Format**: Resolution / FPS / Time Base / SAR / Field Order / Color Space / Color Range / Color Transfer / Color Primaries / Color Metadata
- **Video — Bitrate**: Bitrate Mode / Bitrate / VBV / HRD / Min·Max Bitrate / Quality(CRF)
- **Video — GOP**: GOP Size / Closed·Open GOP / Keyframe·IDR Policy / Reference Frames / B-Frames / Lookahead / Scene Cut
- **Audio**: Codec / Sample Rate / Channel Layout / Bit Depth / Bitrate / Loudness Reference / AV Sync Offset
- **Container**: MPEG-TS / fMP4 / MP4 / MOV / MKV / Segment / Index / Metadata / Timecode

> 注：Common 段的 `Latency Mode (Normal/Low/Ultra-Low)`（P-21 §455）在 `REALTIME_PROFILE` 下被 §3 的 `latency_class` 取代并强化，不再使用松散的 Mode 表述。

---

## 3. REALTIME_PROFILE 专属属性（本 Spec 核心，对应提案 §13）

> 这些属性**不是文件转码参数**，缺失会导致实时编码无法保证广播级 SLA。全部为 `REALTIME_PROFILE` 必填/强约束。

| 属性 | 取值 / 语义 | 联动 |
|---|---|---|
| `latency_class` | `NORMAL` / `LOW` / `ULTRA_LOW` | 决定 encoder preset/tune（zerolatency）与 buffer 预算 |
| `realtime_safe` | `true`（强制，不可 false） | 不参与"暂停/重试"语义 |
| `rate_control` | `CBR` / `Capped VBR`（**禁 unbounded VBR**） | 实时码率预算可预测 |
| `gop_strategy` | `FIXED` / `CLOSED` / `IDR`（广播强制 `CLOSED`） | 对齐 P-21 Closed GOP 要求 |
| `frame_sync` | `STRICT` | 帧对齐切换前提（V0.2 Switch Mode） |
| `audio_sync` | `STRICT` | AV Sync 预算内（通常 < ±½ frame） |
| `encoder_warmup` | `REQUIRED` | warm-up 完成前不接入 ON AIR |
| `fallback_encoder` | 指定备选 encoder | 热备/故障切换使用 |
| `max_startup_latency` | ms（如 2000） | Preflight 校验上限 |
| `target_cpu` | 核心数预算（如 4.0） | 联动 E-36 Resource Scheduler |
| `target_gpu_sessions` | 并发 session 数 | 联动 Hardware Encoder Runtime Discovery |
| `failover_compatibility` | `PACKET_SWITCH` / `FRAME_SWITCH` / `MASTER_SWITCH`（**可多选**） | Encoding Profile 只声明兼容哪些 Switch Mode; 实际 Effective Switch Mode 由 Graph Compiler Decision Tree 决定 (V0.2) |
| `hot_standby` | `COLD` / `WARM` / `HOT` | 联动 V0.2 Hot-Standby 3 |
| `resource_reservation` | `REQUIRED` | 实时预算预留，拒绝超卖 |

### 运行时指标（EFFECTIVE 层，对应 M-17 实时面板）
`FPS` / `Speed` / `CPU` / `RAM` / `PTS Drift` / `AV Offset` / `Latency` / `Dropped Frames` / `READY_TO_TAKE`

### Validation 段新增校验（P-21 §9 / §2333）
- Latency Budget PASS（≤ `max_startup_latency`）
- Failover 兼容性与目标 Channel 的 Switch Mode 匹配
- `resource_reservation` 被 E-36 满足（否则阻断 ACTIVE）

---

## 4. FILE_PROFILE 专属属性（对应提案 §14）

| 属性 | 取值 / 语义 |
|---|---|
| `purpose` | `ARCHIVE` / `PROXY` / `WEB` / `SOCIAL`（多目标派生，见 P-28 Bundle） |
| `quality` | `CRF` / `CQ`（质量优先，允许 VBR/2-pass） |
| `speed` | `VERY_FAST` → `SLOW`（编码速度优先，非延迟优先） |
| `audio` | AAC 320k 等（继承 Common Audio，可覆盖） |
| `container` | `MP4` / `MKV`（文件友好，非 MPEG-TS 传输流） |
| `metadata` | `PRESERVE`（保留原片元数据/Timecode） |
| `output_asset_version` | `Master` / `Proxy` / `Mobile` / `Archive` / `Custom`（联动 Asset Version） |

### 队列/生命周期（对应 M-14 / M-18）
`Job` 5 状态：`PENDING` / `QUEUED` / `RUNNING` / `COMPLETED` / `FAILED`；支持暂停/重试/多 Worker（见 SURFACE_SPEC §3332）。

> FILE_PROFILE **不要求** `realtime_safe` / `hot_standby` / `failover_compatibility` / `resource_reservation`。

---

## 5. 与现有对象映射

| 本 Spec | 现有文档 / 表面 |
|---|---|
| `profile_type` 选择 | ✅ 已落地 P-21 Section 1 `profile_type` 单选 (FILE_PROFILE / REALTIME_PROFILE, 0.5D.3) |
| Common 段字段 | P-21 §392-470（9 区，广播级，已 LOCK） |
| `REALTIME_PROFILE` 运行时 | M-17 Realtime Transcode（Session 三轴 + 实时指标） |
| `FILE_PROFILE` 运行时 | M-14 File Transcode（6 步 Wizard）/ M-18 Job Detail |
| `Job` / `Session` 包装 | OBJECT_VOCABULARY §1.11/§1.12（`REALTIME_PROFILE` 由 `MEDIA_SESSION` 承载, 非 Job） |
| Bundle 引用 | P-28 Profile Bundle（1 Channel 1 Bundle，引用 7 Profile） |

---

## 6. 4-Layer 语义（对齐 PIA §6）

| Layer | Encoding Profile 示例 |
|---|---|
| DESIRED | `REALTIME_PROFILE` · H264 1080p25 · `latency_class=LOW` · `failover=FRAME` |
| COMPILED | H264 / yuv420p / Closed GOP / x264 + NVENC Runtime Discovery 实例 |
| EFFECTIVE | H264 1080p25 / 5.96 Mbps / Encoder PID 1234 / CPU 34% / `READY_TO_TAKE` |
| IMPACT | Affected: CH01 + 1 Output Variant · Risk: LOW（LOCKED） |

---

## 7. 0.5D 实施锚点

1. P-21 `profile_type` 单选 (✅ 已落地 0.5D.3) → 切换 Realtime/File 段
2. Realtime 段按 §3 落字段 + Validation 新增实时校验
3. M-17 / M-14 分别绑定 `REALTIME_PROFILE` / `FILE_PROFILE` 类型过滤
4. P-28 Bundle 允许混合引用两类 Profile（Video 用 REALTIME，Archive 派生用 FILE）

⛔ 本 Spec 仅为 Schema 增补，不改动 V0.2 任何 Engine 或 Runtime 机制。
