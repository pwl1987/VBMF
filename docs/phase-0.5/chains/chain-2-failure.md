# 链 2：Failure（Operator + System 故障处理）

> V0.2 §10.11 链 2 锁定
> 角色：Operator + System（自动）
> 端到端：故障检测 → ALERT → Switch 决策 → Auto Failover → Filler 兜底 → Operator 通知 → Incident 建档 → 录像回溯

## 流程

```
[Source.A SDI 冻结 5s]
  ↓
[QC 检测: black/freeze 5s]
  ↓
[Health Tree: Source.Primary ACTIVE+FAILED]
  ↓
[Rule 1: ACTIVE+FAILED → FAILED]
  ↓
[ALERT 触发到 Operator Dashboard]
  ↓
[Switch Decision Tree §3.4]
  ├─ PACKET? No（主备 codec 不齐）
  ├─ FRAME? 检查 → 是（异源）
  └─ 执行 FRAME_SWITCH
  ↓
[Auto Failover → Source.B STANDBY → ACTIVE]
  ↓
[Filler 兜底（If 切换 > 1s）]
  ↓
[Operator 收到 INCIDENT 通知]
  ↓
[Incident #1248 自动建档]
  ↓
[录像回溯 → Chunk 14:25:00-14:30:00]
```

## 步骤明细

| 步 | 操作 | 触发 / 自动 | 关键检查 | 输出 |
|---|---|---|---|---|
| 1 | Source.A 冻结 5s | 物理故障 | — | SDI 信号异常 |
| 2 | QC black/freeze 检测 | 自动 | 5s 阈值 | QC ALERT |
| 3 | Health Tree 更新 | 自动 | Source.Primary ACTIVE+FAILED | Tree 标红 |
| 4 | Aggregation Rule 1 | 自动 | ACTIVE+FAILED → FAILED | channel_health_aggregation=FAILED |
| 5 | effective_channel_status | 自动 | Policy: FAILED | channel_health_view.effective=FAILED |
| 6 | Switch Decision Tree | 自动 | §3.4 step 1-4 | FRAME_SWITCH |
| 7 | Source.B 接管 | 自动 | Backup STANDBY→ACTIVE, Primary→OFFLINE | node_role 翻转 |
| 8 | Filler 兜底 | 自动 | 切换 > 1s → Filler | 减少观众感知 |
| 9 | ALERT 到 Operator | 自动 | Push + Dashboard 红条 | Operator 知道 |
| 10 | Incident 建档 | 自动 | X4 Incident Timeline | incidents 表 #1248 |
| 11 | 录像继续 | 自动 | Recording Engine 不中断 | Chunk 完整 |
| 12 | Operator 确认 | 手动 | Acknowledge | incident.acked_at |
| 13 | 录像回溯 | 手动 | Operator 点击 Incident → Chunk | 14:25-14:30 chunk |

## Failure Domain 决定恢复动作（§8.9）

```yaml
failure_domain_matrix:
  SOURCE:      { action: FAILOVER,            target: §3.4 decision tree }
  PIPELINE:    { action: RESTART_NODE,        target: offending node }
  MASTER:      { action: FILLER_OR_EMERGENCY, target: emergency asset }
  OUTPUT:      { action: RESTART_ADAPTER,     target: alternate destination }
  RECORDING:   { action: BACKUP_DISK,         target: alternate disk }
  CLOCK:       { action: FALLBACK_CLOCK,      target: clock_domain_mappings }
  RESOURCE:    { action: DEGRADE_BG_JOBS,     target: lower-priority workers }
  # DiagnosticFailureClass (不进 7 OperationalFailureDomain)
  PLAYER:      { action: NOTIFY,              fail_safe: true }
  UNKNOWN:     { action: SAFE_DEGRADE,        alert: true }
```

## Health Invariants 检查

- **H1** (no ACTIVE+FAILED): FAIL → 触发 ALERT
- **H5** (OFFLINE+FAILED 系统已吸收): Primary → OFFLINE+FAILED，Channel 状态由 Backup ACTIVE+HEALTHY 决定
- **H6** (Source RG all unavailable): 不应触发（因为 Backup 仍可用）
- **HA-03** 验收: Primary+Backup 都 FAILED → Channel FAILED

## Phase 0.6 5 Fault Injection 验收用例

| # | 故障 | Failure Domain | 期望恢复 | 期望 Channel |
|---|---|---|---|---|
| FI-01 | SDI 冻结 5s | SOURCE | FRAME_SWITCH + Filler | DEGRADED → HEALTHY (after failover) |
| FI-02 | 音频静音 8s | PIPELINE | RESTART audio node | DEGRADED → HEALTHY |
| FI-03 | Primary FFmpeg 进程崩溃 | PIPELINE | RESTART + RESUME | DEGRADED → HEALTHY |
| FI-04 | Clock Drift +5ms/min | CLOCK | FALLBACK to TIMECODE | DEGRADED (CLOCK_DEGRADED event) |
| FI-05 | HLS 切片失败 | OUTPUT | RESTART_ADAPTER → alternate | DEGRADED → HEALTHY |

## 关键引擎 / 横切能力映射

| 步骤 | 引擎 / 能力 |
|---|---|
| 故障检测 | §3.9 Health Tree + §3.13 AVSync Manager |
| Switch 决策 | §3.4 Decision Tree + §8.9 Failure Domain Matrix |
| 接管 | §3.4 Switch Mode + X6 Capability Registry |
| Filler | §3 Playout (timeline) |
| ALERT | X4 Incident Timeline |
| 录像 | §3 Recording + X4 关联 |
| 通知 | §3.10 X4 + Webhook |

## 关联 Wireframe

- `wireframes/01-dashboard.html`（Operator 收到 ALERT）
- `wireframes/02-sources.html`（Source 状态）
- `wireframes/03-switcher.html`（Switch Mode 变化 / 最近事件）
- `wireframes/07-recording.html`（录像 Chunk + Incident 关联）
- `wireframes/09-health-tree.html`（Tree 颜色变化）

## 关键禁忌

- ❌ **PLAYER 缓存异常绝不能切源**（DiagnosticFailureClass.PLAYER → NOTIFY only）
- ❌ **AV sync 异常必须先 Failure Domain Classification**（§8.9），不能直接切 backup
- ❌ **Master Join 失败 ≠ 切源**（可能是 PIPELINE / OUTPUT 问题）
- ❌ **同一切换不能在 100ms 内重试**（避免 flapping）
