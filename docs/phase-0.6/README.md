# Phase 0.6 — Executable Acceptance Specification 可执行验收规范

> **状态**：📋 计划中
> **范围**：Reference A1/A2/B + 5 Fault Injection + 7 Health Invariants tests
> **目的**：在 V0.2 架构基线上做**真实部署 / 真实故障注入**，把 22 轮 review 锁定的 7 Health Invariants / 5 FI / 3 Switch Mode 全部转成可执行测试。

## 目标

```
V0.2 Architecture   ──┐
Phase 0.5 Workflow   ──┤
                      ├──→ Executable Acceptance Spec ──→ V0.2 Architecture ACCEPTED
Phase 0.6 References ─┘
```

## 交付物

### Reference A1 — PACKET_SWITCH 基础能力

**输入**：预对齐压缩源 A / B（同 codec / container / 时间戳 / GOP）

**目的**：验证 Capability Contract / Runtime Alignment / Mandatory Attributes 全部通过

**架构链路**：
```
Source.A (compressed) ─┐
                       ├─→ [Switcher] ─→ Encode ─→ SRS ─→ HLS
Source.B (compressed) ─┘   (PACKET)
```

**验证项**：

- [ ] Capability Contract 17+ 项 mandatory attributes 全部 PASS
- [ ] Runtime Alignment（GOP/IDR/PTS/DTS/timebase/SPS/PPS/audio continuity）PASS
- [ ] WARN ≠ PASS（PACKET 严格要求 PASS）
- [ ] Switch decision tree 选 PACKET_SWITCH
- [ ] 切换时延 < 100ms（target，不构成协议保证）
- [ ] 反复切换 100 次，零异常
- [ ] 24h 持续运行无崩溃

### Reference A2 — 真实 SDI 主备

**架构链路**：
```
SDI-A ─→ Normalize ─→ Encode ─┐
                             ├─→ [Switcher] ─→ Program Master ─→ SRS ─→ HLS
SDI-B ─→ Normalize ─→ Encode ─┘   (FRAME/MASTER)
```

**关键点**：
- 独立 FFmpeg 编码器**天然不具备 PACKET switch 所需的精确对齐**
- 真实 SDI 主备默认走 **FRAME / MASTER 切换**
- 除非已做外部对齐

**验证项**：

- [ ] SDI 输入识别（BMD DeckLink dv0 / dv1）
- [ ] Normalize 后 Master 统一（Program-scope Master）
- [ ] FRAME_SWITCH 触发条件（codec/format/color space 异源）
- [ ] MASTER_SWITCH 触发条件（异构主备 / 不同色域）
- [ ] AV sync 测量（Master Join 处）
- [ ] 切换 < 500ms（target）
- [ ] 24h 稳定

### Reference B — 异构源 + 图文 + 多 Master

**架构链路**：
```
SDI  ─┐
      ├─ Normalize ─→ MASTER_SWITCH ─┐
SRT  ─┘                                ├─ Program Master
                                      │   (Video / Audio / Metadata)
Composition ───────────────────────┤
Audio Mixer / Loudness / Delay ─────┘
                                      │
                                      ▼
                                     SRS
```

**覆盖**：
- RAW + COMPRESSED 异构
- MASTER_SWITCH 决策
- Composition（Program + Variant 双层）
- Audio Mixer / Loudness / Delay
- Program Master 三独立 graph

**验证项**：

- [ ] 异构源 Normalize 成功
- [ ] MASTER_SWITCH 选型
- [ ] Program Composition + Variant Composition 渲染（RAW 域）
- [ ] Encode = delivery boundary
- [ ] Audio 三独立 graph 同步
- [ ] 多路 Output Variant 同步

### 5 Fault Injection 故障注入

| # | 故障 | Failure Domain | 期望恢复 | 期望 Channel Health |
|---|---|---|---|---|
| **FI-01** | SDI 冻结 5s | SOURCE 源 | FRAME_SWITCH + Filler | DEGRADED → HEALTHY (after failover) |
| **FI-02** | 音频静音 8s | PIPELINE 管道 | RESTART audio node | DEGRADED → HEALTHY |
| **FI-03** | Primary FFmpeg 进程崩溃 | PIPELINE 管道 | RESTART + RESUME | DEGRADED → HEALTHY |
| **FI-04** | Clock Drift +5ms/min | CLOCK 时钟 | FALLBACK to TIMECODE | DEGRADED (CLOCK_DEGRADED event) |
| **FI-05** | HLS 切片失败 | OUTPUT 输出 | RESTART_ADAPTER → alternate | DEGRADED → HEALTHY |

**关键禁忌**：

- ❌ PLAYER 播放端缓存异常绝不能切源
- ❌ AV sync 异常必须先 Failure Domain Classification
- ❌ Master Join 失败 ≠ 切源
- ❌ 同一切换不能在 100ms 内重试

### 7 Health Invariants → Executable Tests

来自 `docs/phase-0.5/wireframes/09-health-tree.html`：

| # | Health Tree 状态 | 期望 Channel |
|---|---|---|
| **HA-01** | Primary=OFFLINE+FAILED, Backup=ACTIVE+HEALTHY | **HEALTHY** (Rule 5) |
| **HA-02** | Primary=OFFLINE+FAILED, Backup=STANDBY+HEALTHY | **DEGRADED** (Rule 4: pending takeover) |
| **HA-03** | Primary=OFFLINE+FAILED, Backup=OFFLINE+FAILED | **FAILED** (Rule 3: Source RG all unavailable) |
| **HA-04** | Primary=OFFLINE+FAILED, Backup=OFFLINE+FAILED (no ACTIVE/STANDBY) | **FAILED** (Rule 3) |
| **HA-05** | ACTIVE=UNKNOWN, STANDBY=HEALTHY | **UNKNOWN** (Rule 6 拒收，fall to UNKNOWN) |
| **HA-06** | ACTIVE=HEALTHY, STANDBY=FAILED | **DEGRADED** (Rule 5: STANDBY+(DEGRADED\|FAILED)) |
| **HA-07** | ACTIVE=HEALTHY, OFFLINE+FAILED | **HEALTHY** (H5: 系统已吸收) |

### Failure Domain Matrix 故障域验证

```yaml
SOURCE 源:      { action: FAILOVER,            target: §3.4 }
PIPELINE 管道:    { action: RESTART_NODE,        target: offending node }
MASTER 主母版:    { action: FILLER_OR_EMERGENCY, target: emergency asset }
OUTPUT 输出:      { action: RESTART_ADAPTER,     target: alternate destination }
RECORDING 录制:   { action: BACKUP_DISK,         target: alternate disk }
CLOCK 时钟:       { action: FALLBACK_CLOCK,      target: clock_domain_mappings }
RESOURCE 资源:    { action: DEGRADE_BG_JOBS,     target: lower-priority workers }
PLAYER 播放端:     { action: NOTIFY,              fail_safe: true }    # DiagnosticFailureClass
UNKNOWN 未知:     { action: SAFE_DEGRADE,        alert: true }        # DiagnosticFailureClass
```

## 部署环境

- 服务器：10.30.15.10（Ubuntu 26.04，32 核 / 30 GB / 546 GB / 3 张 BMD DeckLink）
- BMD 驱动：Desktop Video 16.2a1（已安装）
- FFmpeg：git-2026-08-23 + allcodec（已编译）
- ffmpeg 路径：`/usr/local/bin/ffmpeg`
- 9 编解码库：`/usr/local/{bin,lib,include}`（x264 / x265 / libvpx / lame / fdk-aac / opus / vorbis / theora / speex）

## 启动时间表

- **Phase 0.6 启动**：V0.2 架构冻结后即可
- **Reference A1**：1 周（PACKET 基础，无硬件依赖）
- **Reference A2**：1 周（需 BMD 硬件）
- **Reference B**：2 周（复杂链路）
- **5 Fault Injection**：1 周
- **24h 稳定性**：1 周

## 验收产出

- 24h 稳定运行的 Reference A1/A2/B
- 5 Fault Injection 全部通过
- 7 Health Invariants test 全部通过
- 报告：`docs/phase-0.6/ACCEPTANCE_REPORT.md`
- 视频 / 截图证据：`docs/phase-0.6/evidence/`

## 不在本阶段范围

- V0.2 架构修改（FORBIDDEN）
- 实际生产部署
- 多节点 HA（V0.4）
- WebRTC / RIST / Zixi / NDI（V0.3/V0.5）
