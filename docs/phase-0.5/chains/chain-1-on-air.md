# 链 1：On-Air（Operator 日常播出）

> V0.2 §10.11 链 1 锁定
> 角色：Operator（导播 / 值班员）
> 端到端：选 Channel → 看 PVW/PGM → 选 NEXT → TAKE → 切到 PGM → Master → Output → 浏览器播放

## 流程

```
[Operator 登录] → [Dashboard 01]
  ↓
[选 Channel: CH01 News HD]
  ↓
[看 PVW 预览 / PGM 直播]
  ↓
[选 NEXT 节目：Source.B Backup]
  ↓
[按 TAKE 按钮] ← L2 重要操作（3s 倒计时 + 二次确认）
  ↓
[切到 PGM：PACKET_SWITCH（87ms）]
  ↓
[Program-scope Master 接收新源（RAW 域）]
  ↓
[Variant Composition 叠加平台 Logo]
  ↓
[Encode → COMPRESSED]
  ↓
[Output.SRS.HLS → CDN → 浏览器播放]
```

## 步骤明细

| 步 | 操作 | 页面 | 关键检查 | 失败兜底 |
|---|---|---|---|---|
| 1 | 登录（op 角色） | — | RBAC 校验 | 拒绝 + ALERT |
| 2 | 选 Channel | 01-dashboard | Channel 必须 RUNNING | 切到 Status 红/黄 |
| 3 | 看 PVW | 01-dashboard | PVW 解码正常 | PVW 异常 → ALERT |
| 4 | 选 NEXT 节目 | 01-dashboard | NEXT 节目 source HEALTHY | 灰显，禁止选 |
| 5 | **按 TAKE** | 01-dashboard | L2 二次确认（3s 倒计时） | 取消 |
| 6 | 切到 PGM | 03-switcher | Switch Mode 决策（PACKET/FRAME/MASTER） | 自动降级链 §3.4 |
| 7 | Program Master 接收 | 03-switcher | Video/Audio/Metadata 三独立 graph 同步 | 任一失败 → DEGRADED |
| 8 | Variant Composition | 04-composition | Variant Profile 加载 | 用 default variant |
| 9 | Encode | 06-output | 编码器 warm-up | Retry 3 次 |
| 10 | HLS 分发 | 06-output | SRS Push OK | Restart Adapter |
| 11 | 浏览器播放 | — | HLS .m3u8 200 | 客户端重试 |

## Health Invariants 检查

- H1 (no ACTIVE+FAILED): 必须 PASS
- H5 (OFFLINE+FAILED 系统已吸收): 切源后旧 Primary OFFLINE 时必 PASS
- H6 (Source RG all unavailable): 不应触发
- H7 (effective_channel_status = channel_health_view): 必 PASS

## 验收用例（Phase 0.6 复用）

- **OnAir-01**: 正常切源（CH01 PVW→PGM），切后 PGM 显示新源，Channel=HEALTHY
- **OnAir-02**: 切源过程中 AV sync 漂移 < 80ms，3s 内恢复
- **OnAir-03**: 切源后 Channel Health 保持 HEALTHY（H5 OFFLINE 吸收）
- **OnAir-04**: 同一 Channel 连续 TAKE 3 次，第三次仍正常
- **OnAir-05**: Variant Composition 切换无画面中断

## 关键引擎 / 横切能力映射

| 步骤 | 引擎 / 能力 |
|---|---|
| TAKE | §3.4 Switch Mode Decision Tree + §1.21 Atomic Apply |
| Switch 决策 | X6 Capability Registry + runtime_alignment |
| Master 同步 | §3.7 Program Master 三独立 graph |
| Encode | §3.7.1 Encode = delivery boundary |
| Output | §3 Output + SRSAdapter (decoupled) |
| 监控 | X5 Health Tree + X4 Incident Timeline |

## 不在本链范围

- 故障切换（见 chain-2-failure）
- 节目单排播（见 chain-3-playout）
- 图设计 / 调参（见 chain-4-engineering）

## 关联 Wireframe

- `wireframes/01-dashboard.html`（主入口）
- `wireframes/03-switcher.html`（TAKE 动作 / 模式展示）
- `wireframes/04-composition.html`（Variant Composition）
- `wireframes/06-output.html`（最终分发）
- `wireframes/09-health-tree.html`（链路监控）
