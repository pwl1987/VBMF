# Channel Type Model · 频道类型模型 (D1 类型定义)

> 配套原型: `operator/CH-02-create-channel.html`
> 状态: 0.5D 验收链 D1 迭代 (类型扩展)
> 关联: PRODUCT_OBJECT_MODEL.md (Channel 组合中心), ENCODE_MODEL_SPEC.md (REALTIME_PROFILE)

---

## 0. 背景与目标

原 D1（CH-02 创建频道向导）仅覆盖「电视频道直播」。本文件定义 **三种 Channel 类型** 的
字段结构、数据模型与接口设计，作为原型与后续运行态建模（V0.3 候选）的依据。

类型判别字段: `channel_type` (枚举, 必填)。

| 值 | 名称 | 说明 |
|---|---|---|
| `TV_LIVE` | 电视频道直播 | 视频 + 音频直播链路 (原 D1 形态) |
| `RADIO_LIVE` | 广播直播 | **仅音频**, 不含任何 video 字段 |
| `VIRTUAL_PLAYOUT` | 虚拟编排播出 | (TV 或 Radio 媒体) + 节目单 + 定时调度 |

设计约束:
- `RADIO_LIVE` **明确不建模任何 `video_*` 字段** (无分辨率/帧率/视频编码/码率/GOP/视频监视器/视频输出)。
- `VIRTUAL_PLAYOUT` 通过子判别 `media_kind ∈ {VIDEO, AUDIO}` 复用 TV 或 Radio 的 **Profile 引用与输出变体**,
  并额外叠加节目单 (Playlist) 与定时调度 (Scheduler) 层。

---

## 1. 公共基座字段 (Base — 三者共有)

| 字段 | 类型 | 约束 | 说明 |
|---|---|---|---|
| `channel_id` | string | PK, 必填 | 运营单位主键 (V0.2 §3.6) |
| `channel_name` | string | 必填 | 命名遵循 V0.2 §1.5 |
| `channel_type` | enum | 必填 | `TV_LIVE` / `RADIO_LIVE` / `VIRTUAL_PLAYOUT` (0.5F.1 修正拼写, 全仓统一) |
| `workspace_id` | string | 必填 | 工作区 / 集群 |
| `clock_ref` | E-37 | 必填 | 时钟基准 + 4 级 Fallback |
| `switch_decision` | enum | 直播类必填 | `AUTO` / `MANUAL` (虚拟类由 Scheduler 接管) |
| `hot_standby_level` | enum | 必填 | `COLD` / `WARM` / `HOT` (V0.2 Canonical, ⛔ 禁 `NONE`) |
| `redundancy_enabled` | bool | 默认 true | 不需要备机时置 `false`, **不扩充 HotStandbyLevel enum** (0.5F.1 修正) |
| `bundle_id` | ref | 必填 | Profile Bundle (8 Profile 引用) |
| `channel_configuration_status` | enum | 系统写 | `DRAFT → VALIDATED → APPLIED → RETIRED` (Channel 配置生命周期; 0.5F.4 P1-5 更名 — 不叫 `lifecycle`, 避免与 Session Runtime 三轴冲突) |
| `owner` / `created_by` | string | 必填 | 责任人 |
| `created_at` / `updated_at` | timestamp | 系统写 | |

---

## 2. TV_LIVE · 电视频道直播

含视频 + 音频。完整媒体链路。

### 2.1 字段结构
| 分组 | 字段 | 类型 | 说明 |
|---|---|---|---|
| source | `primary_source_id` | ref | 主源 (视频+音频契约, SDI/IP) |
| source | `backup_source_id` | ref | 备源 (可选) |
| encode | `encoding_profile_ref` | ref → P-21 | **引用** REALTIME_PROFILE — codec/resolution/framerate/bitrate/gop/latency 全部在 P-21, **不复制** (0.5F.1 修正) |
| audio | `audio_profile_ref` | ref → P-23 | **引用** Audio Profile — layout/loudness/delay/mapping 全部在 P-23, **不复制** (0.5F.1 修正) |
| output | `variants[]` | ref[] | 基带 SDI + 网络 HLS/RTMP/UDP-MC/WebRTC (引用 P-22 Output Profile + Destination) |
| preview | (运行时) | — | 视频监视器 (16:9) + L/R 音柱 (预览端点 E-42) |

> **0.5F.1 关键约束:** Channel **不拥有** `codec/bitrate/GOP/resolution/latency` — 全部经 `bundle_id` → P-21 REALTIME_PROFILE / P-23 Audio / P-22 Output 引用。修改 Profile 只改一处, 不产生 `Channel.codec` vs `EncodingProfile.codec` 两份真相。

> **0.5F.4 P1-5 更名说明:** Channel 配置生命周期用 `channel_configuration_status` (`DRAFT → VALIDATED → APPLIED → RETIRED`); **Runtime 三轴** (lifecycle `STOPPED/STARTING/RUNNING/STOPPING` · readiness `NOT_READY/READY_TO_TAKE` · health `HEALTHY/DEGRADED/FAILED/UNKNOWN`) 属于 `media_session_runtime`, 不再混用 `lifecycle` 一词。Phase 1 不再出现 `ChannelLifecycleState` / `SessionLifecycleState` 两套 enum。

---

## 3. RADIO_LIVE · 广播直播 (仅音频)

**不含任何 `video_*` 字段**。下表为全部字段, 注意 video 组为空。

### 3.1 字段结构
| 分组 | 字段 | 类型 | 说明 |
|---|---|---|---|
| source | `primary_source_id` | ref | 主源 (音频契约: AES67 / 模拟 / 编解码流, **无 video adapter**) |
| source | `backup_source_id` | ref | 备源 (可选) |
| video | — | — | **不建模** (无 resolution/framerate/video codec/bitrate/gop/video monitor) |
| audio | `audio_profile_ref` | ref → P-23 | **引用** Audio Profile — codec/layout/sample_rate/bitrate/loudness/mapping 全部在 P-23, **不复制** (0.5F.1 修正) |
| output | `variants[]` | ref[] | **仅音频**: Icecast/Shoutcast、RTMP、SRT、UDP 组播/单播、DAB+ |
| preview | (运行时) | — | **仅 L/R 音柱, 无视频监视器** |

### 3.2 显式排除清单 (Validation 依据)
提交 `RADIO_LIVE` 时若携带以下任一字段 → `422 Unprocessable Entity`:
`video`, `video.codec`, `video.resolution`, `video.framerate`, `video.bitrate`,
`video.gop_mode`, `video.latency_mode`, 以及任何 `monitor`/`preview.video` 类配置。

---

## 4. VIRTUAL_PLAYOUT · 虚拟编排播出

= (TV 或 Radio 媒体) + 节目单 + 定时调度。

> **无独立信号源实体**: 虚拟编排播出**没有** §2/§3 的 `source`(主/备源) 字段。**节目单即信号源** —
> 节目单项 (`kind`) 可为 `ASSET`(文件素材) 或 `VIDEO_SOURCE`(视频源/Live Source); 视频源须先经 E-40/E-42 验证后引用。

### 4.1 子判别
`media_kind ∈ {VIDEO, AUDIO}` — 决定复用 §2/§3 的 **Profile 引用与输出变体** (视频/音频编码与输出)。虚拟类型本身不持有 `source` 字段。

### 4.2 节目单 (Playlist)
| 字段 | 类型 | 说明 |
|---|---|---|
| `program_list_id` | string | PK |
| `schedule_mode` | enum | `LINEAR` (固定时间表) / `LOOP` (循环) / `EVENT` (事件触发) |
| `timezone` | string | IANA tz |
| `items[]` | array | 节目项列表 (有序) |
| `items[].item_id` | string | |
| `items[].kind` | enum | `ASSET` (文件素材) / `VIDEO_SOURCE` (视频源/Live Source) |
| `items[].asset_id` | ref | ASSET 类 (kind=ASSET) |
| `items[].video_source_id` | ref | 视频源/Live Source (kind=VIDEO_SOURCE, 须已 VERIFIED) |
| `items[].title` | string | |
| `items[].start_time` | time | 绝对起播 (LINEAR) 或相对偏移 |
| `items[].duration` | duration | |
| `items[].transition` | enum | HardCut / Mix |
| `items[].audio_layout` | enum | |
| `items[].fallback_asset_id` | ref | 异常兜底素材 |

### 4.3 定时调度 (Scheduler)
| 字段 | 类型 | 说明 |
|---|---|---|
| `auto_start` | time | 每日自动开播 |
| `auto_stop` | time | 每日自动停播 |
| `days_of_week[]` | enum[] | 生效日 |
| `gap_filler_asset_id` | ref | 时间表空隙垫播素材 |
| `loop_policy` | enum | 到尾处理 |

### 4.4 运行时语义
Scheduler 按 `items[].start_time` 自动切播: `ASSET` 项播文件素材, `VIDEO_SOURCE` 项切到已验证的视频源/Live Source (窗口内抢占),
结束后回退节目单。所有切换触发下游 Output (复用 §2/§3 输出变体)。

---

## 5. 接口设计 (REST, 前缀 `/api/v1`)

| Method | Path | 说明 | 关键校验 |
|---|---|---|---|
| `POST` | `/channels` | 创建频道 | `channel_type` 必填; `RADIO_LIVE` 带 `video_*` → 422; `VIRTUAL_PLAYOUT` 缺 `playlist`/`schedule` → 422 |
| `GET` | `/channels/{id}` | 读取频道 (含 type) | |
| `POST` | `/channels/{id}/sources` | 指派主/备源 | 需 Source 已 VERIFIED (E-42) |
| `POST` | `/channels/{id}/playlist` | 创建节目单 (**仅 Virtual**) | 非 Virtual → 409 |
| `POST` | `/channels/{id}/playlist/items` | 追加节目项 | |
| `POST` | `/channels/{id}/schedule` | 设置定时调度 (**仅 Virtual**) | |
| `POST` | `/channels/{id}/take` | 开播 · `TakePreflightResult` READY/CONDITIONAL→`200` allow(+warning) · BLOCKED→`409` (0.5F.7 P0-2) | |

> **P0-2 (0.5F.7) — `TakePreflightResult` 与 API 对齐:** B-13 闭集 = `READY` / `CONDITIONAL` / `BLOCKED` (0.5D.3 焊死)。两层分离: **`readiness` (Runtime 三轴) = `NOT_READY` / `READY_TO_TAKE`** vs **`TakePreflightResult` = `READY` / `CONDITIONAL` / `BLOCKED`**。API 不再写"必须全 PASS"——`READY` 与 `CONDITIONAL` 均放行, 仅 `BLOCKED`(含 #9 Resource>100%) 返回 `409`。`CONDITIONAL` = 仅 WARNING + Reservation 满足 + REQUIRED 全 PASS。

### 5.1 请求示例
```json
POST /api/v1/channels
{
  "channel_type": "RADIO_LIVE",
  "channel_name": "CH-RADIO-02",
  "workspace_id": "MAIN-HALL-A",
  "clock_ref": "PTP Primary",
  "audio": { "layout": "STEREO", "codec": "AAC", "sample_rate": "48kHz",
             "bitrate": 128, "loudness_lufs": -23 },
  "output": { "variants": [ { "proto": "RTMP", "required": true }, { "proto": "SRT", "optional": true }, { "proto": "UDP_MC", "optional": true }, { "proto": "ICECAST", "required": true, "maturity": "V0.3_RESERVED" }, { "proto": "DAB_PLUS", "aux": true, "maturity": "V0.3_RESERVED" } ] }
}
# 若 body 含 "video": {...} → 422 (RADIO_LIVE 禁止 video 字段)
```

```json
POST /api/v1/channels
{
  "channel_type": "VIRTUAL_PLAYOUT",
  "media_kind": "VIDEO",
  "playlist": {
    "schedule_mode": "LINEAR", "timezone": "Asia/Shanghai",
    "items": [
      { "kind": "ASSET", "asset_id": "ASSET-MORNING", "start_time": "06:00",
        "duration": "30m", "transition": "HardCut", "fallback_asset_id": "ASSET-FILLER" },
      { "kind": "VIDEO_SOURCE", "video_source_id": "LIVE-BREAK", "start_time": "06:45", "duration": "10m" }
    ]
  },
  "schedule": { "auto_start": "06:00", "auto_stop": "24:00", "days_of_week": ["MON".."SUN"],
                "gap_filler_asset_id": "ASSET-FILLER" }
}
# 缺 playlist 或 schedule → 422
```

---

## 6. 与 CH-02 原型映射

| 步骤 | TV_LIVE | RADIO_LIVE | VIRTUAL_PLAYOUT |
|---|---|---|---|
| ① 模板&基础 | ✓ | ✓ | ✓ |
| ② 信号源 | 视频+音频源 + 双视频预览 | 音频源 + 仅音频预览 | — 向导中无此选项卡 (节目单即信号源) |
| ③ 节目单&调度 | — (跳过) | — (跳过) | ✓ 节目单 + Scheduler |
| ④ 编码&音频 | 视频+音频 | **仅音频** (无视频 profile) | 按 `media_kind` |
| ⑤ 输出 | 基带+网络视频 | **仅音频输出** | 按 `media_kind` |
| ⑥ 资源预览 | ✓ | ✓ | ✓ |
| ⑦ 预检&提交 | ChangeSet 含 type | ChangeSet 含 type | ChangeSet 含 type + 节目单摘要 |

> 橙色块为内联「设计缺口」: PLAYLIST/SCHEDULE/SCHEDULER_JOB 运行态对象尚未在 POM 建模,
> 切源时序与 Output 联动需独立定义 (归入 D5/D6)。
