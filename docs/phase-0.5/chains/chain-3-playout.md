# 链 3：Playout（Director 节目单排播）

> V0.2 §10.11 链 3 锁定
> 角色：Director（节目总监）
> 端到端：打开 Timeline → 拖入 Asset → Preflight → Save Draft → Validate → Schedule Apply → On-Air 时自动到点 → 切 → 复位

## 流程

```
[Director 登录] → [Timeline 页面]
  ↓
[选 Channel: CH01 News HD]
  ↓
[看 24h 排播表]
  ↓
[拖入 Asset: "广告片 30s"]
  ↓
[Preflight (X2): loudness / rights / duration / codec]
  ├─ loudness: -23 LUFS ✓
  ├─ rights: valid until 2027-01-01 ✓
  ├─ duration: 30.0s ✓
  └─ codec: H.264 1080p25 ✓
  ↓
[Save Draft → change_set DRAFT]
  ↓
[Validate (X2 Preflight 完整)]: 0 critical / 0 warning
  ↓
[Schedule Apply: 2026-08-25 15:30:00]
  ↓
[change_set VALIDATED → APPLIED]
  ↓
[15:30:00 自动到点]
  ↓
[Playout Engine 自动 TAKE → 切到 "广告片 30s"]
  ↓
[30s 后自动切回原节目]
```

## 步骤明细

| 步 | 操作 | 页面 / 引擎 | 关键检查 | 失败兜底 |
|---|---|---|---|---|
| 1 | Director 登录 | — | RBAC: Director | 拒绝 |
| 2 | 打开 Timeline | 04-composition | 当日排播表加载 | 空表 → 提示新建 |
| 3 | 拖入 Asset | 04-composition | Asset 库存在 | 灰显 |
| 4 | Preflight 实时 | 04-composition | loudness / rights / duration / codec | 实时红框提示 |
| 5 | Save Draft | — | change_set: DRAFT | 自动保存 |
| 6 | Validate | — | X2 Preflight 完整 | 报告 critical 项 |
| 7 | Schedule Apply | — | time > now + 5min | 时间校验 |
| 8 | change_set APPLIED | — | X3 Configuration Versioning | snapshot + rollback |
| 9 | 自动到点 | §3 Playout | Timeline 时间匹配 | +5s 容差 |
| 10 | 自动 TAKE | §3 Playout + §3.4 | Switch Mode 决策 | 走 switch decision tree |
| 11 | 持续 30s | §3 Playout | duration 计时 | — |
| 12 | 自动切回 | §3 Playout | next item | 同 step 10 |

## X2 Preflight 检查项（Director 拖入时实时 + Validate 时完整）

```yaml
preflight_playout:
  - loudness_check:   { target: -23 LUFS, tolerance: ±2 }
  - rights_check:     { valid_until >: now + 7 days }
  - duration_check:   { min: 5s, max: 7200s }
  - codec_check:      { video: H.264/H.265, audio: AAC/Opus }
  - resolution_check: { min: 720p, max: 4K }
  - color_space_check: { allowed: [BT.709, BT.2020] }
  - audio_channels:   { allowed: [2, 6, 8] }
```

## X3 Configuration Versioning

```yaml
change_set:
  id: CS-2026-0825-001
  target_type: Playlist
  target_id: pl-CH01-2026-08-25
  before_rev: REV-001
  after_rev: REV-002
  status: APPLIED                  # DRAFT → VALIDATED → APPLIED → ROLLED_BACK
  phase: COMMITTED                 # PREPARING → APPLYING → COMMITTED → ABORTED
  applied_at: 2026-08-25 15:25:00
  scheduled_at: 2026-08-25 15:30:00
  snapshot_id: SNAP-001            # 用于回滚
```

## 状态机

```
DRAFT → VALIDATED → APPLIED → (RUBBLED_BACK)
                ↘ ABORTED

phase (事务阶段):
PREPARING → APPLYING → COMMITTED
         ↘ ABORTED
```

## 关键引擎 / 横切能力映射

| 步骤 | 引擎 / 能力 |
|---|---|
| Timeline | §3 Playout Engine |
| Preflight | X2 Preflight (Playout 类) |
| Change Set | X3 Configuration Versioning |
| Validate | X2 Preflight (完整) |
| 自动到点 | §3 Playout + §3.4 Switch Decision |
| 自动 TAKE | §3.4 Switch Mode + §3.7 Program Master |
| 回滚 | X3 ROLLED_BACK (snapshot 恢复) |

## Phase 0.6 验收用例

- **Playout-01**: 排播 "广告 30s" → 自动到点 → 自动切回（30s ±0.5s）
- **Playout-02**: Loudness 不达标 (-19 LUFS) → Preflight 拒绝 → Director 必须修正
- **Playout-03**: Rights 已过期 → Preflight 拒绝 → 不能 Apply
- **Playout-04**: 排播 1h 节目，到点前 10s Dashboard 出现"即将切换"提醒
- **Playout-05**: 排播 Apply 后立即 Rollback → 排播表恢复 REV-001

## 关联 Wireframe

- `wireframes/04-composition.html`（Program + Variant Composition + 节目编排）
- `wireframes/01-dashboard.html`（On-Air 监控）
- `wireframes/03-switcher.html`（自动 TAKE 事件流）
- `wireframes/09-health-tree.html`（Playout 状态监控）

## 关键禁忌

- ❌ **不能直接编辑 APPLIED 的 change_set**（必须新建 Change Set）
- ❌ **Rights 过期 Asset 不能 Apply**（Preflight 拒绝）
- ❌ **Schedule Apply 时间不能 < now + 5min**（避免误切）
- ❌ **Composition 切 Variant 不能影响 Program Master**（V0.2 §3.7.1 双层独立）
