# Phase 0.6 — Executable Acceptance Specification 可执行验收规范

> **状态**：📋 计划中 (前置: Phase 0.5 = UX BASELINE LOCK FINAL)
> **范围**：Reference A1/A2/B + 5 Fault Injection + 7 Health Invariants tests
> **目的**：在 V0.2 架构基线上做**真实部署 / 真实故障注入**，把 22 轮 review 锁定的 7 Health Invariants / 5 FI / 3 Switch Mode 全部转成可执行测试。

## 0. V0.2 语义对齐 (Phase 0.5C 锁定)

Phase 0.6 的 latency 验收**不写**协议式保证 (e.g. "< 100ms"), 而是:

```yaml
target:
  source: hot_standby_levels.target_failover_time_ms   # V0.2 锁定, Policy 字段

acceptance:
  measured:
    source: failover_benchmarks                        # V0.2 独立 runtime 实测
    metrics: [p50, p95, p99]
    required:
      p95_lte_target: true
```

**禁止:** 任何验收项写 `切换时延 < Xms` 形式。**正确写法** 总是 `target + measured p50/p95/p99` 形式。WARN 与 PASS 严格分离, PASS 必须实测值。

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

**架构链路 (PACKET_SWITCH: COMPRESSED → Switch → COMPRESSED，V0.2 §3.4 锁死；此路径不插入 Encode)**：
```
Source.A (COMPRESSED) ─┐
                        ├── PACKET_SWITCH ──→ SRS ──→ HLS
Source.B (COMPRESSED) ─┘
```
> Encode 仅在 `RAW → Encode → COMPRESSED` 时出现（见 Reference B / Program Master delivery boundary），**不在 PACKET_SWITCH 路径内**——否则等于把已压缩数据重新编码一次，破坏 PACKET_SWITCH 的验证目标。

**验证项**：

- [ ] Capability Contract: **Mandatory Compatibility Attributes = ALL PASS**（V0.2 §3.4 Canonical；不写固定数字，未来可继续追加 attribute）
- [ ] Runtime Alignment: **Required attributes = ALL PASS**（GOP/IDR/PTS/DTS/timebase/SPS/PPS/audio continuity）
- [ ] WARN ≠ PASS（PACKET 严格要求 PASS）
- [ ] Switch decision tree 选 PACKET_SWITCH
- [ ] `target_failover_time_ms` 来自 `hot_standby_levels`（V0.2 锁定，**非协议保证**），由 `failover_benchmarks` 独立实测 p50/p95/p99
- [ ] 反复切换 100 次，零异常
- [ ] 24h 持续运行无崩溃

### Reference A2 — 真实 SDI 主备

**架构链路 (FRAME_SWITCH = RAW → Switch → RAW；MASTER_SWITCH = RAW → Normalize → Master-level Switch → RAW；Encode 是 Program-scope Master 的 delivery boundary，位于 Switcher 之后，不在切换输入侧)**：

FRAME_SWITCH：
```
SDI-A ─→ Normalize ─→ RAW ┐
                           ├── FRAME_SWITCH ──→ RAW
SDI-B ─→ Normalize ─→ RAW ┘
                           ↓
                   Program Master
                           ↓
                         Encode
                           ↓
                           SRS
                           ↓
                           HLS
```

MASTER_SWITCH：
```
SDI-A ─→ Normalize ─┐
                     ├─→ MASTER_SWITCH
SDI-B ─→ Normalize ─┘
                     ↓
             Program Master
                     ↓
                   Encode
                     ↓
                    SRS
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
- [ ] `target_failover_time_ms` 来自 `hot_standby_levels`（V0.2 锁定，**非协议保证**），由 `failover_benchmarks` 独立实测 p50/p95/p99
- [ ] 24h 稳定

### Network Source Acceptance (UDP UNI / ASM / SSM)

UDP 网络源必须按三种模式各自独立验收（**不能**合并成单一 "Multicast Address" 字段）：

| Fixture | 必填字段 | 验收点 |
|---|---|---|
| **UDP-UNI** | Remote IP / Remote Port / Interface | 单播可达；Interface 绑定正确 |
| **UDP-ASM** | Group / Port / Interface / IGMP Join | IGMPv2 加入组；多接收者 |
| **UDP-SSM** | **Source IP** / Group / Port / Interface / IGMPv3 | **Source IP 不可丢**（SSM = (S,G)）；IGMPv3 仅收指定源 |

**验证项**：

- [ ] E-40 Source Wizard 按 Source Kind = Network 动态渲染上述三套 schema（非单一 Multicast 字段）
- [ ] SSM 校验 **Source IP 必填**，缺省拒绝（否则 ASM/SSM 混淆）
- [ ] IGMP 版本随模式切换（ASM → IGMPv2 / SSM → IGMPv3）
- [ ] 三模式各自独立 Fixture，实测可达后进入 A2/B 切换链

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
- [ ] 多路 Output Variant 故障隔离：Program Master HEALTHY + {HLS HEALTHY, RTMP HEALTHY, **WHEP DEGRADED**} → Channel = **DEGRADED**（WHEP=Required）或 **HEALTHY**（WHEP=Optional）
- [ ] Variant failure ≠ Program Master failure（Failure Domain = OUTPUT，按 variant 独立判定；Required/Optional 影响 Channel 总判定）
- [ ] Program Scope Composition 跨所有 Variant 共享（e.g. 全台 Logo 出现在每一个 Variant）
- [ ] Variant Scope Composition 仅作用于目标 Variant（e.g. 平台水印只出现在 Variant A、区域版权贴片只出现在 Variant B）
- [ ] Acceptance: 共享 Logo 在所有 Variant 一致；平台水印仅目标 Variant 出现（**禁止**把 Composition 全部提前到 Program Master）

### 5 Fault Injection 故障注入

| # | 故障 | Failure Domain | 期望恢复 | 期望 Channel Health |
|---|---|---|---|---|
| **FI-01A** | Primary SDI 冻结 5s + Backup READY | SOURCE 源 | FAILOVER → Backup ACTIVE | HEALTHY (after failover) |
| **FI-01B** | Primary SDI 冻结 5s + Backup NOT_READY | SOURCE 源 | FAILOVER attempted → FILLER | DEGRADED / SAFE (Filler；非 HEALTHY) |
| **FI-02** | 音频静音 8s（injection_point: Audio Mixer / PIPELINE）| PIPELINE 管道 | RESTART audio node | DEGRADED → HEALTHY |
| **FI-03** | Primary FFmpeg 进程崩溃 | PIPELINE 管道 | RESTART + RESUME | DEGRADED → HEALTHY |
| **FI-04** | Clock Drift +5ms/min | CLOCK 时钟 | FALLBACK to TIMECODE | DEGRADED (CLOCK_DEGRADED event) |
| **FI-05** | HLS 切片失败 | OUTPUT 输出 | RESTART_ADAPTER → alternate | DEGRADED → HEALTHY |

> **FI 注入点锁定（P1）**：每条 FI 必须写明 `injection_point: {node, domain}`，否则 Phase 0.6 实测时两人会做出不同恢复动作。当前已锁定：FI-02 = Audio Mixer / PIPELINE。以下为**不同 Failure Domain**，须各自独立 FI，不得并入 FI-02：Source embedded_audio（SOURCE）、Loudness node（PIPELINE）、Audio Master Join（MASTER）。

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
| **HA-04** | ACTIVE=DEGRADED, STANDBY=OFFLINE+FAILED | **DEGRADED** (Rule: ACTIVE DEGRADED + STANDBY(FAILED\|OFFLINE) → DEGRADED；与 HA-03 全不可用区分) |
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

### Switch Test Matrix & Negative / Recovery Fixtures

每个 Reference 必须同时具备正向与负向夹具，**不能只验证 happy path**：

| Fixture | Capability | Alignment | 期望决策 |
|---|---|---|---|
| A1-PASS | PASS | PASS | PACKET_SWITCH |
| A1-WARN | WARN | PASS | 不进 PACKET，继续 Decision Tree |
| A1-FAIL | FAIL | — | 不进 PACKET → FRAME / MASTER / REJECT |
| A1-RUNTIME-MISALIGN | PASS | GOP mismatch | PACKET invalid |
| A2-PASS | PASS (RAW) | PASS | FRAME / MASTER_SWITCH |
| B-PASS | PASS | PASS | MASTER_SWITCH + Composition |

**切换压力矩阵**（不能只测 100 次人工/脚本切换；必须组合）：

- 维度：Cold / Warm / Hot × PACKET / FRAME / MASTER
- 故障组合：Forward failover · Failback · Repeated flapping · Source loss during switch · Output loss during switch · Clock degradation during switch
- 稳定性约束：`min_hold` / `hysteresis` 生效（同一切换 100ms 内禁止重试，见关键禁忌）
- Recovery 夹具：Failover → Filler / FRAME_SWITCH → DEGRADED → HEALTHY(after failover)；Failback 须满足 `min_hold` 后才回切

**TAKE 语义锁定（Acceptance Assertion）**：TAKE = Operator Intent → TakePreflightResult → Switch Command → Media Session Runtime → `active_source_id` 更新；`effective_switch_mode` 由 Decision Tree 决定。**TAKE 不是 Configuration Apply，也不是 ChangeSet Apply。**

### Profile Responsibility Boundary (Acceptance Assertion)

P-20 Profile Center / P-21 Encoding Profile 的责任边界必须锁死，防止 Packaging 职责偷偷渗入 Encoding Profile：

- **Encoding Profile**：仅 `codec / resolution / framerate / bitrate / GOP / rate-control(CRF|CBR|VBR) / 2-pass`。
- **Packaging Profile**：`container(MP4|TS) / segment(HLS|DASH) / segment-duration / playlist / manifest / DRM`。
- 两者为**独立 Profile 类型**，共享引用但运行时对象 / 状态机 / 失败恢复 / UI 分离（与 M-14 File Transcode / M-17 Realtime Session 的 Encoding↔Packaging 分离一致）。
- **禁止**：P-21 Encoding Profile 承担 container / segment / manifest / DRM；DRM 属 Packaging / Distribution 边界。

### E2E Acceptance: Profile → Bundle → ChangeSet → Runtime → Output

完整配置生命周期必须在 Phase 0.6 以真实案例跑通一次（不是 happy path，而是 Preflight + Transactional Cutover）：

```
P-21 v3 (Encoding Profile)
  ↓ Create v4
CD-01 Channel Detail → Bundle v2 → v3 (引用新 Profile)
  ↓ Impact Preview
CH01 / CH03 / CH08 affected
  ↓ select CH01 only (不强制全量)
Bundle v2 → v3
  ↓ Preflight (Capability / Runtime Alignment ALL PASS)
ChangeSet: APPLYING → APPLIED (Logical Atomic / Transactional Cutover)
  ↓
Runtime Revision N+1 (session.apply_revision)
  ↓
Effective config changed
Output still HEALTHY (Variant 全同步: HLS / RTMP / WHEP)
```

**验收项**：

- [ ] Impact Preview 准确列出受影响 Channel（CH01 / CH03 / CH08）
- [ ] 选择性 Apply（仅 CH01）不污染 CH03 / CH08
- [ ] Preflight 失败则 ChangeSet 不进入 APPLYING（WARN ≠ PASS）
- [ ] Apply 后 Runtime Revision 单调 +1，旧 Revision 可回滚
- [ ] Apply 期间 Output 持续 HEALTHY（零黑场 / 零中断）
- [ ] 回滚路径：Rollback → 上一 Runtime Revision

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
