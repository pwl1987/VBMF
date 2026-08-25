# V0.2 Architecture Baseline — IP Broadcast Media Fabric (runtime-semantics freeze)

> **项目正式定性**: IP 广播级媒体信号调度与虚拟播控平台
> **缩写**: **VBMF**
> **版本**: V0.2.4 — **Runtime Semantics Patch 2 + Cleanup-1/2/3** (Architecture Baseline LOCK FINAL)
> **生成时间**: 2026-08-24
> **基础版本**: V0.2.3 (Runtime Semantics Patch 1)
> **服务器**: 10.30.15.10 (Ubuntu 26.04)
> **部署参考快照**（非 Architecture Fact）：32 核 / 30 GB / 546 GB / 3 张 BMD DeckLink；具体型号/serial/能力由 Runtime Discovery 在 Media Agent 启动时探测（见 §3.11 current_host_snapshot）。机器增 BMD / GPU 不修改 V0.2。
> **本版状态**: **V0.2 Architecture Baseline LOCK FINAL**（22 轮 review / V0.2.3 patch 1 / V0.2.4 patch 2 / Cleanup-1/2/3 / Errata-1/2/3/4/5/6/7/8/9/10/11/12/13/14 / 一致性 PASS / 可唯一实现 / **implementation_ambiguity: NONE** / 9 大 Runtime 域 CLOSED / 3 Schema + 2 Semantic Cleanup / 7 Health Invariants）。不再开 V0.2.5。**V0.2 Runtime Semantics = CLOSED / IMPLEMENTATION AUTHORITY**。

---

## 目录

- [第 0 部分：项目重新定性](#第-0-部分项目重新定性)
- [第 1 部分：22 条核心架构原则](#第-1-部分22-条核心架构原则)
- [第 2 部分：12 核心 Engine + 5 横向系统 + 6 横向能力](#第-2-部分12-核心-engine--5-横向系统--6-横向能力)
- [第 3 部分：Signal Graph v0.2-final](#第-3-部分signal-graph-v0-2-final)
- [第 4 部分：进程与角色模型（含 Graph Compiler）](#第-4-部分进程与角色模型含-graph-compiler)
- [第 5 部分：数据模型（核心表）](#第-5-部分数据模型核心表)
- [第 6 部分：关键技术决策（30+）](#第-6-部分关键技术决策30)
- [第 7 部分：媒体能力矩阵（QC 完整清单）](#第-7-部分媒体能力矩阵qc-完整清单)
- [第 8 部分：状态机 / Switch Mode / Hot-Standby / 故障切换](#第-8-部分状态机--switch-mode--hot-standby--故障切换)
- [第 9 部分：部署拓扑（Host + Docker 边界）](#第-9-部分部署拓扑host--docker-边界)
- [第 10 部分：Operator UX / UI 架构](#第-10-部分operator-ux--ui-架构)
- [第 11 部分：实现路线图（重排）](#第-11-部分实现路线图重排)
- [第 12 部分：待决策项（V0.2 锁定）](#第-12-部分待决策项v0-2-锁定)
- [第 13 部分：P2 预留](#第-13-部分p2-预留)
- [附录 A：术语表](#附录-a术语表)
- [附录 B：版本演进](#附录-b版本演进)
- [附录 C：V0.2 一致性审查 + Runtime Semantics Freeze 记录](#附录-cv0-2-一致性审查--runtime-semantics-freeze-记录)

---

## 第 0 部分：项目重新定性

（同 V0.1-V0.2 第 1 轮不变）

---

## 第 1 部分：22 条核心架构原则

> V0.2 三轮原则数：12 → 16 → **22**。本轮新增 6 条（Graph Compile / Switch Mode / Hot-Standby / Latency Probe / Program Master / Configuration Revision）。

### 1.1 - 1.12（同 V0.2 第 2 轮不变）

### 1.13 **Signal Graph 显式声明 Data Plane 类型** ⭐⭐ (重要修正)

> **🔴 V0.2.4 Cleanup-2 修正**：Data Plane **不是单层枚举**，是**两维语义**。**§3.1 是唯一定义规范**，本节只做引用与解释，不再重复定义。代码侧实现为：
>
> ```ts
> // Canonical vocabulary（UPPER_CASE，统一用于 TS / Rust / JSON Schema / PG enum）
> type DataPlaneLayer =
>   | "ELEMENTARY"
>   | "CONTAINER"
>   | "METADATA"
>   | "CONTROL";
>
> type ElementaryDataType =
>   | "COMPRESSED_VIDEO"
>   | "COMPRESSED_AUDIO"
>   | "RAW_VIDEO"
>   | "RAW_AUDIO";
>
> type DataPlane =
>   | { layer: "ELEMENTARY"; type: ElementaryDataType }
>   | { layer: "CONTAINER";   type: "MULTIPLEXED" }
>   | { layer: "METADATA";    type: "METADATA"; metadata_type: "TIMECODE" | "CAPTION" | "SCTE35" | "KLV" | "SYSTEM" }
>   | { layer: "CONTROL";     type: "EVENT" };
> ```

**为什么必须分 Elementary / Container / Metadata / Control 四层**：

| 场景 | Plane | 资源 |
|---|---|---|
| 1 路 SRT H.264 + 20 个频道 | ELEMENTARY.COMPRESSED_VIDEO 扇出 | 极低 |
| 1 路 SRT 解码后 1080p + 20 个 compositor | ELEMENTARY.RAW_VIDEO 扇出 | 爆 32 核 |
| 1 路 MPEG-TS 切流 | CONTAINER.MULTIPLEXED 扇出 | 视内部而定 |
| 1 路 SDI（已是 RAW）分发 | ELEMENTARY.RAW_VIDEO 扇出 | 视后端 |

> **🔴 重要修正（沿用 round 3）**：**SDI 是 `ELEMENTARY.RAW_VIDEO + ELEMENTARY.RAW_AUDIO`**，不是 COMPRESSED。
> SDI 是设备级原始媒体（未压缩），FFmpeg 的 DeckLink 设备文档明确把 SDI 视作 raw media 处理（YUV422 / v210 + 48kHz PCM）。

**DECODED 不是 Data Plane**，它是**处理过程**（从 ELEMENTARY.COMPRESSED_VIDEO 转换到 ELEMENTARY.RAW_VIDEO 的动作）。

**不显式分清 → Scheduler 算不准 → 系统会"看起来很便宜"但实际爆 CPU。**

> 完整规范见 **§3.1 Data Plane 完整定义**。本节不重复定义，避免与 §3.1 冲突。

### 1.14 - 1.16（V0.2 第 2 轮不变：Clock Domain / Latency Budget / Backpressure）

### 1.17 **⭐ Switch Mode 必须显式**（V0.2 第 3 轮新增）

主备切换**不是单一动作**，必须按**切换粒度**分模式：

| 模式 | 含义 | 适用 |
|---|---|---|
| `PACKET_SWITCH` | 在压缩码流层切（GOP 对齐 / SPS/PPS / 时间戳连续性） | 主备 codec+profile 完全一致 |
| `FRAME_SWITCH` | 主备都先 decode → 在 RAW_VIDEO 层切 → 重新 encode | codec 不同 / 跨格式 |
| `MASTER_SWITCH` | 主备都先 normalize → 统一输出格式 → 切 | 不同设备 / 不同色域 / 异构 |

> 即使主备都是 H.264，单纯"切压缩流"也得**保证 GOP 边界、timecode 连续、audio timestamp 对齐**。做不好 → 切换瞬间画面跳变/卡顿/无声。

### 1.18 **⭐ Hot-Standby 三级**（V0.2 第 3 轮新增）

不是"备份 ffmpeg 常驻"就完事：

| Level | Policy / Target | target_failover_time_ms（设计预算，**不构成协议保证**）|
|---|---|---|
| `COLD` | 灾备录播，成本敏感 | `30000` |
| `WARM` | 公共服务频道 | `1500` |
| `HOT` | 新闻/直播/广告插播；要求 Runtime 最终满足 `READY_TO_TAKE`（§8.11 三轴状态） | `100` |

> **🔴 V0.2.3 修正**：HOT 不是"pipe 满缓冲"就完事，必须是 `READY_TO_TAKE` 状态。**`target_failover_time_ms` 是设计预算，不构成协议保证**；实际性能必须通过 `benchmark.p50/p95/p99` 验证，由 `failover_benchmarks` 表（§5）记录。
> 广播级主备推荐 HOT。COLD 灾备录播；WARM 公共服务频道；HOT 新闻/直播/广告插播。
> **禁止**在 Baseline 写"实测 50-200ms"或"target × 0.5 ~ target × 2"等固定范围（V0.2.4 Errata-3 锁定）。

### 1.19 **⭐ Latency Probe 必须在每个阶段**（V0.2 第 3 轮新增）

延迟不能光看"端到端"，必须**分段测量**：

```yaml
probe:
  capture_ts:    # SDI 抓帧
  decode_ts:     # 软解
  switch_ts:     # 主备切换后
  compose_ts:    # 图文烧录后
  encode_ts:     # 编码后
  publish_ts:    # SRS 推送后
  player_ts:     # 客户端收到
```

**E2E latency = player_ts − source_reference_ts**。

没有分段，出了问题只能猜是"FFmpeg 还是 SRS"。

### 1.20 **⭐ Program Master = Video Master + Audio Master + Metadata Master**（V0.2 第 3 轮新增）

不要把 video / audio 当成"一条 pipeline 里的两个分支"。它们是**独立的 graph**：

```
Video Graph:     SDI → Normalize → Switch → Compose → Encode
Audio Graph:     SDI → Mixer → Loudness → Delay → Encode
Metadata Graph:  Timecode / Subtitle / SCTE-35
                 ↓
              Master Join
                 ↓
            Program Master
```

理由：
- Audio Delay = +80ms 是 Audio Graph 内部的事，不是 Video Graph 的事
- AV Sync 测量在 Master Join 节点做
- **🔴 V0.2.3 措辞修正**：Video / Audio / Metadata 在**处理层独立隔离**，单一路径故障不会直接破坏其他路径的运行实例；但 Master Join 处会做**一致性判定**，若任何一路 failed，Program Master 会进入 `DEGRADED` 或触发 `FAILOVER`。**不是"完全独立"**，是"故障域隔离 + 联合判定"。

### 1.21 **⭐ Configuration Versioning 必须可回滚 + Atomic Apply**（V0.2.3 patch 1 修订）

任何**运行时配置变更**必须经过：

```
Draft
 ↓
Validate (Preflight)
 ↓
Preview
 ↓
Apply:
   - Immediate
   - At next event boundary
   - At scheduled time
 ↓
Rollback
```

GraphSpec、ChannelConfig、OutputProfile、AudioProfile、GraphicTemplate、QCProfile **全部要版本化**。事故追溯时要知道"那时跑的是哪一版"。

**🔴 V0.2.4 改 Atomic Apply → Logical Atomic / Transactional Cutover**：

跨 PG / Media Controller / Media Agent / FFmpeg / SRS **无法实现数据库式 ACID**。改用 **Logical Atomic + Transactional Cutover** 表示业务层原子性。

```yaml
change_set_apply_pipeline:
  draft:    收集所有要改的项
  validate: 每项都过 Preflight
  prepare:  staging 区域准备（不生效）
  commit:   # ⭐ Logical Atomic (Transactional Cutover)
    1. snapshot 当前 runtime（所有目标对象状态）
    2. prepare: 所有变更在 staging 完成
    3. commit_barrier: 一次性发出激活指令，所有 Agent 同时切换
    4. 若激活失败或部分失败：
       - 立即 deactivate 新配置
       - restore 原配置（回滚）
    5. 成功 → Runtime Revision N+1
  → applied / aborted
```

术语：
- **Logical Atomic**：业务层原子性（不是 DB 事务）
- **Transactional Runtime Cutover**：所有组件同时切换
- **Deactivate N+1 / Restore N**：失败时的二阶段回滚

Apply 模式：`Immediate` / `At next event boundary` / `At scheduled time`。

### 1.22 **⭐ Preflight ≠ QC**（V0.2.3 patch 1 三层结构）

- **Preflight**：变更**前**的静态检查（Graph 合法性、资源、Loudness 兼容、Clock 域、Latency 预算、Backup 是否就位）
- **QC**：运行**中**的动态监测（黑场、冻结、静音、AV 偏移）

两者解耦：Preflight 在变更前必过；QC 在运行中持续跑。

**🔴 V0.2.3 改 Preflight 三层结构**：

```yaml
preflight:
  static:                 # 静态合法性
    graph:        [data_plane_compat, capability_contract_match, clock_domain_align]
    contract:     [node_input_contract, node_output_contract]
    rights:       [media_rights_valid, output_rights_valid]
    asset:        [asset_exists, asset_duration_match, asset_loudness_match]
    latency:      [per_edge_within_budget, per_channel_within_target]

  resource:               # 9-dim Quantitative Resource Vector + Device/Port Token Constraints
    # ==== 9 维 Quantitative Vector（§3.11）====
    cpu:              "Σ(node.cpu_threads) ≤ available.cpu"
    gpu_sessions:     "Σ(node.gpu_sessions) ≤ available.gpu_sessions"
    vram_mb:          "Σ(node.vram_mb) ≤ available.vram_mb"
    ram_mb:           "Σ(node.ram_mb) ≤ available.ram_mb"
    nic_in_mbps:      "Σ(node.ingress_mbps) ≤ available.nic_ingress_mbps"
    nic_out_mbps:     "Σ(node.egress_mbps) ≤ available.nic_egress_mbps"
    disk_write_mbps:  "Σ(node.disk_write_mbps) ≤ available.disk_write_mbps"
    pcie_rx_mb_s:     "Σ(node.pcie_rx_mb_s) ≤ available.pcie_rx_mb_s"
    pcie_tx_mb_s:     "Σ(node.pcie_tx_mb_s) ≤ available.pcie_tx_mb_s"
    # ==== Device Tokens ====
    bmd_input_ports:  "Σ(node.bmd_input_ports) ≤ Σ(available.bmd_devices[*].input_ports)"
    bmd_output_ports: "Σ(node.bmd_output_ports) ≤ Σ(available.bmd_devices[*].output_ports)"
    device_exclusivity: "同一 port 同一时刻只能分配一个 node"

  runtime_readiness:      # 运行时就绪
    srs:        "SRSAdapter HEALTHY"
    backup_hot: "HotStandby = READY_TO_TAKE"
    source_lock:"Source LOCKED"
    recorder:   "Recorder READY"
    filler:     "Filler READY"
    output_qc:  "Output QC HEALTHY"
```

结果：`PASS` / `WARN` / `FAIL` 三档。FAIL 不允许 Apply。

---

## 第 2 部分：12 核心 Engine + 5 横向系统 + 6 横向能力

> V0.2 锁定：**Engine 总数 = 12**。本轮不增不减。
> 5 横向系统 = V0.1 既有。
> 6 横向能力（X1-X6）= 本轮新增的横切关注点。

### 2.1 12 核心 Engine（同 V0.2 第 2 轮，不变）

| # | Engine | 职责 |
|---|---|---|
| 1 | Source | 多协议信号接入（SDI/Raw、SRT、RTMP、HLS、FILE、INTERNAL、COMPOSITE） |
| 2 | Signal Fabric | 路由 / 矩阵 / 边策略 |
| 3 | Normalize | 格式归一（能力可拆） |
| 4 | Redundancy | 主备 / 切换（PACKET/FRAME/MASTER 模式） |
| 5 | QC | 信号质量监测 |
| 6 | Playout | 虚拟播控 / 时间线 / 插播 |
| 7 | Switcher | 主备切换（按 Switch Mode） |
| 8 | Composition | 图文包装（On-Air 渲染），**支持 Program 级与 Variant 级分层**（§3.7.1） |
| 9 | Audio | 混音 / 响度 / 延迟 / 同步 |
| 10 | Output | 多路分发（含 SRS Gateway Adapter） |
| 11 | Recording | 收录 / 分段 |
| 12 | Replay | 延时 / 回放 |

### 2.2 5 横向系统（V0.1 既有）

| # | 系统 | 职责 |
|---|---|---|
| H1 | Safety | 播出安全：节目缺失/超时/越界 → 阻断/垫片 |
| H2 | Resource Scheduler | CPU/GPU/NIC/Disk + CPU affinity |
| H3 | Watchdog & Incident | 进程 watchdog + 黑匣子 |
| H4 | Audit | 操作不可抵赖 + 告警升级 |
| H5 | Subtitle | 字幕（烧录/独立轨） |

### 2.3 6 横向能力（V0.2 第 3 轮新增 ⭐）

> 这些**不是 Engine**，是横切所有 Engine 的能力。

| # | 能力 | 职责 |
|---|---|---|
| **X1** | **Graph Compiler / Validator** | 把 GraphSpec 编译成可执行 Runtime Graph（含自动插入缺失节点、Data Plane 校验、Clock 转换、资源预估） |
| **X2** | **Preflight** | 变更**前**的静态检查（Graph / Playout / Channel 三类） |
| **X3** | **Configuration Versioning** | 所有运行时配置可版本化、可回滚、可定时 Apply |
| **X4** | **Incident Timeline** | 自动串接时间线（QC 告警 → 切换 → 录像 → 操作员确认） |
| **X5** | **Health Tree** | 通道 → 子系统（源/Switcher/Master/SRS/HLS/录制）→ 节点的分层健康视图 |
| **X6** | **Capability Registry** | 注册"信源能提供什么、节点能处理什么、输出能播放什么"（Signal Contract / Player Capability Matrix） |

### 2.4 Engine 与 Data Plane 约束（V0.2 第 3 轮修正）

⭐ **最关键**：把 SDI 改正。

| Engine | 接受 | 产出 | 备注 |
|---|---|---|---|
| Source (SDI) | — | `RAW_VIDEO` + `RAW_AUDIO` | ⭐ 修正：SDI 是 raw |
| Source (SRT/RTMP) | — | `COMPRESSED_VIDEO` + `COMPRESSED_AUDIO` + `METADATA` | 压缩域 |
| Source (FILE) | — | `COMPRESSED_VIDEO` + `COMPRESSED_AUDIO` | 或 raw（视源文件） |
| Source (INTERNAL: BLACK/BARS) | — | `RAW_VIDEO` + `RAW_AUDIO` | 内部源，预先渲染 |
| Decode | `COMPRESSED_*` | `RAW_*` | 解码到 raw |
| Encode | `RAW_*` | `COMPRESSED_*` | 编码 |
| Normalize (video) | `RAW_VIDEO` | `RAW_VIDEO` | 缩放/隔行/色彩等 |
| Normalize (audio) | `RAW_AUDIO` | `RAW_AUDIO` | 重采样/通道映射 |
| Normalize (stream) | `COMPRESSED_*` | `COMPRESSED_*` | **3 子能力**：REMUX / BITSTREAM_ADAPT / METADATA_REWRITE；**不改 codec / profile / level / resolution / fps / bitrate / GOP**（必须走 ENCODE） |
| Switcher (PACKET) | `COMPRESSED_*` | `COMPRESSED_*` | 同源 codec 时 |
| Switcher (FRAME) | `RAW_*` | `RAW_*` | 跨 codec 时 |
| Switcher (MASTER) | `RAW_*` (post-normalize) | `RAW_*` | 异构主备 |
| Composition | `RAW_VIDEO`（primary）<br>+ `COMPRESSED_*`（auxiliary，需先 Decode）<br>+ Composition Assets / Metadata | `RAW_VIDEO` | **RAW-domain Engine**；COMPRESSED 输入必须先 Decode 到 RAW，再做 Composition；Output 仍是 RAW，由后续 Encode 转回 COMPRESSED |
| Audio Mixer | `RAW_AUDIO` | `RAW_AUDIO` | 混音 |
| Loudness | `RAW_AUDIO` | `RAW_AUDIO` | EBU R128 |
| AV Sync | `RAW_*` (in master join) | `RAW_*` (aligned) | 输出 METADATA |
| QC | `COMPRESSED_*` / `RAW_*` / `METADATA` | `METADATA` / `EVENT` | 只读 |
| Recording | `COMPRESSED_*` | `COMPRESSED_*` | 透传录制 |
| Output (SRS) | `COMPRESSED_*` | `COMPRESSED_*` (HLS/RTMP/SRT/WebRTC) | 协议转换 |
| Output (SDI) | `RAW_VIDEO` + `RAW_AUDIO` | `RAW_VIDEO` + `RAW_AUDIO` | **Architecture Contract RESERVED / V0.2 Implementation DISABLED / Target V0.4**（接口保留，实现关闭）|

**Scheduler 必须看这张表算资源**：RAW_* 永远比 COMPRESSED_* 重一个数量级。

---

## 第 3 部分：Signal Graph v0.2-final

### 3.1 Data Plane 完整定义（V0.2.3 patch 1 修订）

> **🔴 V0.2.3 修正**：`MULTIPLEXED` 不与 `RAW_VIDEO` 完全并列（维度不同），改成**二级分类结构**：
>
> - **Elementary**（基本流）：`COMPRESSED_VIDEO` / `COMPRESSED_AUDIO` / `RAW_VIDEO` / `RAW_AUDIO`
> - **Container / Transport**（容器/传输）：`MULTIPLEXED`
> - **Metadata**（元数据）：`METADATA`
> - **Control**（控制）：`EVENT`

```yaml
data_plane:
  # ===== Elementary 基本流（媒体内容本身） =====
  COMPRESSED_VIDEO:
    layer: ELEMENTARY
    description: "压缩视频"
    examples: [h264, h265, av1, vp9, mpeg2, prores_422_proxy]
    # V0.2.4 Errata-13 改：resource_cost → descriptive_resource_class（仅描述，不参与调度）
    descriptive_resource_class: LOW    # scheduling_input: false; canonical_scheduler_model: §3.11 Resource Vector
    typical_bandwidth_mbps: "0.5-50"

  COMPRESSED_AUDIO:
    layer: ELEMENTARY
    description: "压缩音频"
    examples: [aac, opus, mp3, vorbis, ac3]
    descriptive_resource_class: LOW
    typical_bandwidth_kbps: "32-320"

  RAW_VIDEO:
    layer: ELEMENTARY
    description: "未压缩视频"
    examples: [uyvy422, yuv422p10, v210, rgba, bgra]
    descriptive_resource_class: HIGH  # 解码/重绘（仅描述）
    bandwidth_formula: "width × height × fps × bytes_per_pixel"
    notes:
      UYVY422_1080p25: "≈ 829.44 Mbps ≈ 103.68 MB/s"   # 1920×1080×25×2 = 103.68 MB/s
      V210_1080p25:    "≈ 1.10592 Gbps ≈ 1.106 Gbps ≈ 138.24 MB/s"    # 1920×1080×25×21.33/8 = 138.24 MB/s
      reference: "FFmpeg DeckLink devices 文档：UYVY422=8-bit YUV, V210=10-bit YUV packed (21.33 bit/pixel)"

  RAW_AUDIO:
    layer: ELEMENTARY
    description: "未压缩音频"
    examples: [pcm_s16le, pcm_s24le, pcm_f32le, s32]
    descriptive_resource_class: MEDIUM
    bandwidth_formula: "sample_rate × channels × bit_depth / 8"
    notes:
      PCM_24bit_48kHz_8ch: "≈ 1.152 Mbps ≈ 0.144 MB/s"   # 48000×8×24/8 = 1.152 Mbps
      PCM_16bit_48kHz_2ch: "≈ 1.536 Mbps ≈ 0.192 MB/s"   # 参考

  # ===== Container / Transport 容器/传输 =====
  MULTIPLEXED:
    layer: CONTAINER
    description: "复合流（视音频+metadata 打包成单一容器/传输）"
    examples: [mpegts, mp4, fmp4, matroska, rtp_payload]
    descriptive_resource_class: VARIABLE
    contains: [COMPRESSED_VIDEO, COMPRESSED_AUDIO, METADATA]
    notes: "⚠️ V0.2.3 明确：MULTIPLEXED 是 Container 维度，与 Elementary 不同层；Graph Compiler 在 MULTIPLEXED 边要展开成 elementary 才能比对"

  # ===== Metadata 元数据 =====
  METADATA:
    layer: METADATA
    description: "同步/描述信息（与媒体帧时序绑定）"
    examples: [scte-35, klv, timecode, sei, caption, vanc, cea-708]
    descriptive_resource_class: ZERO_OR_LOW   # V0.2.3 修正：不完全是 ZERO
    metadata_types:               # V0.2.3 新增：二级分类
      - TIMECODE
      - CAPTION
      - SCTE35
      - KLV
      - SYSTEM
    notes: |
      ⚠️ V0.2.3 修正：METADATA 不能全部归 ZERO 资源。
      VANC / CEA-708 / 字幕 / SCTE-35 splice 需要在 mux/demux 或协议边界
      重新注入，处理代价与 SYSTEM 状态不同。
      资源代价 = `ZERO_OR_LOW`，用 metadata_type 区分。

  # ===== Control 控制 =====
  EVENT:
    layer: CONTROL
    description: "异步事件（不与媒体帧时序绑定）"
    examples: [qc_alert, switch_event, heartbeat, webhook]
    descriptive_resource_class: ZERO
    transport: "pub/sub, not in media pipeline"
```

# V0.2.4 Errata-13 锁定：descriptive_resource_class 语义
descriptive_resource_class:
  purpose: "仅用于 Data Plane 的粗粒度描述/文档展示（不参与调度计算）"
  scheduling_input: false                                  # 调度器禁止使用
  canonical_scheduler_model: "§3.11 9-dim Quantitative Resource Vector + Device/Port Token Constraints"
  allowed_values: [LOW, MEDIUM, HIGH, VARIABLE, ZERO, ZERO_OR_LOW]
  banned_term: "resource_cost"                            # 旧名已废除，避免 Scheduler 误用

### 3.2 Source 输出 Data Plane（V0.2 第 3 轮修正，V0.2.4 Cleanup-3 修订）

> **🔴 V0.2.4 Cleanup-3 修正**：RIST / Zixi / NDI 与决策 #11 "V0.2 抽象，V0.3 实现"对齐。

| Source 类型 | V0.2 状态 | 输出（计划） | 备注 |
|---|---|---|---|
| SDI | ✅ **已实现** | `RAW_VIDEO` + `RAW_AUDIO` | ⭐ 设备级原始 |
| **SRT** | ✅ **已实现** | `COMPRESSED_VIDEO` + `COMPRESSED_AUDIO` | 压缩域 |
| RIST | ⏳ **Adapter Placeholder** | `COMPRESSED_VIDEO` + `COMPRESSED_AUDIO` | V0.2 仅接口；V0.3 实现 |
| Zixi | ⏳ **Adapter Placeholder** | `COMPRESSED_VIDEO` + `COMPRESSED_AUDIO` | V0.2 仅接口；V0.3 实现 |
| NDI | ⏳ **Adapter Placeholder** | `RAW_*` 或 `COMPRESSED_*` | 视 NDI 配置；V0.2 仅接口，V0.3 实现 |
| RTMP | ✅ **已实现** | `COMPRESSED_VIDEO` + `COMPRESSED_AUDIO` | |
| HLS / WebRTC pull | ✅ **已实现** | `COMPRESSED_*` | |
| RTP / UDP | ✅ **已实现** | `COMPRESSED_*` | |
| RTSP | ✅ **已实现** | `COMPRESSED_*` | |
| FILE (mp4/mkv) | ✅ **已实现** | `COMPRESSED_*` | |
| FILE (raw/yuv) | ✅ **已实现** | `RAW_*` | |
| INTERNAL (BLACK/BARS/FILLER) | ✅ **已实现** | `RAW_*` | 内部预渲染 |
| COMPOSITE | ✅ **已实现** | 由子图决定 | 嵌套图 |

> **V0.2 实际可用的 Source Adapter**：SDI / SRT / RTMP / HLS / WebRTC pull / RTP / UDP / RTSP / FILE / INTERNAL / COMPOSITE（11 个）
> **V0.3 才实现的**：RIST / Zixi / NDI（3 个，**接口已留，代码未实**）

### 3.3 Normalize 是能力，不是固定节点（V0.2 第 3 轮修正）

> 错误定义：Normalize = `COMPRESSED → DECODED → COMPRESSED`（强制解码+重编）
> 正确定义：Normalize = 三个独立能力

```yaml
normalize_capabilities:
  stream_normalize:
    input:  COMPRESSED_*
    output: COMPRESSED_*
    action: "容器/传输层适配，不解码不重编码"
    sub_capabilities:
      REMUX:
        description: "改变容器/封装格式"
        examples: ["MP4→TS", "TS→fMP4"]
      BITSTREAM_ADAPT:
        description: "bitstream filter / header adaptation"
        examples: ["h264_mp4toannexb", "aac_adtstoasc"]
      METADATA_REWRITE:
        description: "仅修改元数据/标签/时间戳"
        examples: ["修改 service_name", "重写 PAT/PMT"]
    cost: VERY_LOW
    note: "修改 codec / profile / level / resolution / fps / bitrate / GOP 必须走 ENCODE，不能走 stream_normalize"

  video_normalize:
    input:  RAW_VIDEO
    output: RAW_VIDEO
    action: "scale / deinterlace / colorspace / frc / SAR"
    use_case: "1080i25 → 1080p25"
    cost: MEDIUM

  audio_normalize:
    input:  RAW_AUDIO
    output: RAW_AUDIO
    action: "resample / channel remap / sample format"
    use_case: "48kHz stereo → 48kHz 5.1"
    cost: LOW

  encode_step:
    input:  RAW_*
    output: COMPRESSED_*
    action: "transcode（codec / profile / level / resolution / fps / bitrate / GOP）"
    use_case: "SDI raw → H.264"
    cost: HIGH
```

**编译器（X1）根据边 Data Plane 自动选择能力**：
- `COMPRESSED_VIDEO` → `COMPRESSED_VIDEO`：用 `stream_normalize`（不解码）
- `RAW_VIDEO` → `RAW_VIDEO`：用 `video_normalize`（不重编）
- `RAW_VIDEO` → `COMPRESSED_VIDEO`：自动插入 `encode_step`

### 3.4 Switch Mode 三种（V0.2.3 patch 1 修订，V0.2.4 Cleanup-2 微调，V0.2.4 Errata-12 验证）

> **🔴 V0.2.4 Errata-12 验证**：本节**不含**任何绝对 target 数字（`<100ms` / `0.5-2s` / `1-3s`）；Policy / Target 唯一来源 = `hot_standby_levels.target_failover_time_ms`（关联）+ `failover_benchmarks` 验证。

| 模式 | 触发条件（**Mandatory Compatibility Attributes**） | 实现 | Policy / Target（**唯一来源** = §3.5 + `channel_routes` 关联 `hot_standby_levels.target_failover_time_ms`） | 风险 |
|---|---|---|---|---|
| `PACKET_SWITCH` | 主备 Capability Contract **所有 Mandatory Attribute 相等**（见下） | 在压缩流层直接换 | **由关联 HotStandbyLevel.target_failover_time_ms 定义**；实测由 `failover_benchmarks` 验证（§5） | 主备时间戳/GOP 不齐时画面跳变/卡顿 |
| `FRAME_SWITCH` | codec 不同 / 跨格式 / 跨 color space | 主备都先 decode → 在 RAW 层切 → 重新 encode | **由关联 HotStandbyLevel.target_failover_time_ms 定义**；实测由 `failover_benchmarks` 验证 | 编码器 warmup |
| `MASTER_SWITCH` | 主备异构 / 不同色域 / 不同源类型 | 主备都先 normalize → 统一输出格式 → 在统一 MASTER 切 | **由关联 HotStandbyLevel.target_failover_time_ms 定义**；实测由 `failover_benchmarks` 验证 | 资源高（双 pipeline） |

**🔴 V0.2.4 Cleanup-2 修正**：禁止"全部 11 项"这种**具体数字**措辞。改用**Mandatory Compatibility Attributes** 命名空间。

**🔴 V0.2.4 Errata-1 修正**：拆分为 **Capability Contract（静态）** + **Runtime Alignment（运行态）** 两层 —— 前者由 X6 Capability Registry 提供，后者由实时对齐检测提供。**PACKET_SWITCH = 两层 PASS**。

```yaml
packet_switch_eligibility:
  capability_contract:              # 静态契约（X6 Capability Registry 提供）
    mandatory:
      - codec
      - profile
      - level
      - resolution
      - fps
      - pixel_aspect_ratio
      - field_order
      - color_metadata
      - audio.codec
      - audio.sample_rate
      - audio.channel_layout
      - audio.bit_depth
      - mux_format
    optional:
      - channel_mode
      - color_transfer
      - extradata

  runtime_alignment:               # 运行态对齐（实时测量）
    required:
      - gop_boundary              # 主备 GOP 边界是否同步
      - idr_alignment              # IDR 帧是否对齐
      - timestamp_continuity       # PTS/DTS 连续性
      - pts_continuity
      - dts_continuity
      - audio_continuity           # 音频时间戳连续

# 注：V0.2.4 Errata-7 删旧 `decision: ... else: 降级到 FRAME_SWITCH 或拒绝` 块
# packet_switch_eligibility 仅提供 PACKET_SWITCH 的 eligibility inputs；
# 最终 SwitchDecisionResult 必须由下方 switch_mode_decision_tree 统一产生。
# Eligibility ≠ Decision。
```

**🔴 V0.2.4 Errata-4 锁死唯一 Switch Mode 决策树**：

PACKET → FRAME → MASTER 是**降级能力链**，**不是三个并列标签**。

```yaml
# Canonical Switch Mode Decision Tree（§3.4 唯一定义，§8.2 只能引用）
switch_mode_decision_tree:
  step_1:
    condition: "packet_switch_eligibility.capability_contract.mandatory ALL PASS
                AND packet_switch_eligibility.runtime_alignment.required ALL PASS
                AND CapabilityCheckResult = PASS（**WARN ≠ PASS**，见 Errata-9 锁定）"
    result: PACKET_SWITCH
    stop: true

  step_2:
    condition: "common_raw_contract_resolution.result == COMMON_RAW_CONTRACT
                AND time-base is alignable"
    result: FRAME_SWITCH
    stop: true

  step_3:
    condition: "sources are heterogeneous (different domain, color space, source type)
                and a unified Program-scope Master can be produced via Normalize"
    result: MASTER_SWITCH
    stop: true

  step_4:
    condition: "otherwise"
    result: REJECT
    stop: true

# Common RAW Contract Resolution（V0.2.4 Errata-9 锁死，实现规范）
common_raw_contract_resolution:
  source: X6 Capability Registry (SignalContract)
  operation:
    1. intersect supported RAW capabilities from both sources
    2. apply channel/output target constraints
    3. apply clock/timebase compatibility
    4. select canonical target contract
  result:
    COMMON_RAW_CONTRACT     # FRAME_SWITCH 条件 1 满足
    NO_COMMON_RAW_CONTRACT  # 降级到 MASTER_SWITCH 或 REJECT

# Capability Check Result 严格性（V0.2.4 Errata-9 锁死）
switch_mode_eligibility:
  mandatory_capability_contract:
    PASS: eligible
    WARN: NOT eligible       # 关键：WARN ≠ PASS
    FAIL: NOT eligible
```

> **关键规则（V0.2.4 Errata-4 + Errata-6 锁定）**：
> 1. **PACKET → FRAME → MASTER → REJECT** 是**唯一**降级链，**不是三个并列标签**。
> 2. `switch_mode_decision_tree` 是**Canonical Decision Tree**，由 Graph Compiler 自动化执行。
> 3. **禁止**"不满足 PACKET → 自动 FRAME"这种简化二选一（会忽略 MASTER 适用情况）。
> 4. **禁止**"主备异构 → 直接 MASTER"这种不经过 FRAME 步骤的捷径。
> 5. **REJECT ≠ SwitchMode**（V0.2.4 Errata-6 锁定）：
>    - `SwitchMode` 类型 = `PACKET_SWITCH | FRAME_SWITCH | MASTER_SWITCH`
>    - `SwitchDecisionResult` 类型 = `PACKET_SWITCH | FRAME_SWITCH | MASTER_SWITCH | REJECT`
>    - REJECT 是**Decision Outcome**（决策结果），不是 SwitchMode（执行模式）
>
>切换质量：详见关联 `HotStandbyLevel.target_failover_time_ms` + `failover_benchmarks`（§5）；**禁止**写 `<100ms` 等绝对句式（target 是预算，不是协议保证）。
>
> 以后增加 attribute 不需要改数字，直接追加到 `capability_contract.mandatory` 或 `runtime_alignment.required` 即可。

> **🔴 V0.2.4 Errata-2 修正**：以下旧 `packet_switch_eligibility_check` YAML **已删除**（与 `packet_switch_eligibility.capability_contract + runtime_alignment` 重复）。Canonical 定义是上方的 `packet_switch_eligibility`。

<!-- 删除：旧 packet_switch_eligibility_check YAML（被 capability_contract + runtime_alignment 取代） -->
<!-- V0.2.4 Errata-5 删除：旧 "不满足 capability_contract.mandatory 全部 OR runtime_alignment.required 任一 → 自动降级到 FRAME_SWITCH 或拒绝" -->
<!-- 改为：必须继续执行上面 switch_mode_decision_tree（FRAME → MASTER → REJECT 降级链） -->

> **🔴 V0.2.4 Errata-5 关键修正**：
> PACKET_SWITCH eligibility 失败后，**禁止**直接跳到 FRAME_SWITCH 或 REJECT；必须**继续执行**本节 `switch_mode_decision_tree`（FRAME → MASTER → REJECT 降级链）。**禁止**绕过 Canonical Decision Tree。
>
> 调用 `X6 Capability Registry` 的 `SignalContract` 比对 + 实时 alignment 检测，**不允许"猜"**。

**channel_routes.switch_mode = Compiler 解析后的 resolved value**（V0.2.4 Errata-5 + Errata-6 关键锁定）：

> **🔴 V0.2.4 Errata-6 关键边界锁定**：
> - `channel_routes.switch_mode` 语义 = **COMPILED_MODE**（Graph Compiler 解析结果）。
> - 运行时实际生效的模式 = **EFFECTIVE_RUNTIME_MODE**，由 Media Session / Runtime State 表达，**不**反写 `channel_routes`。
> - 运行时 alignment 变化可导致**降级**（PACKET → FRAME → MASTER），但不修改 COMPILED_MODE。
> - 事故追溯时 `COMPILED_MODE`（"当时配置什么"）和 `EFFECTIVE_RUNTIME_MODE`（"当时实际跑什么"）必须能分开查询。

```sql
channel_routes:                       -- COMPILED_MODE（Graph Compiler 写入）
  source_a_id
  source_b_id
  switch_mode: PACKET_SWITCH         -- 语义 = COMPILED_MODE；写入路径：GraphSpec → Compiler → switch_mode_decision_tree → 此字段
  hot_standby_level: HOT             -- 策略 / 目标
  failover_hysteresis_ms: 2000
  failback_hysteresis_ms: 5000
  min_hold_ms: 10000

media_session_runtime:               -- EFFECTIVE_RUNTIME_MODE（Runtime 写入）
  session_id
  channel_id
  effective_switch_mode: FRAME_SWITCH -- 当前实际生效模式（可能因 runtime_alignment 变化降级）
  effective_since: ...
  runtime_alignment_state: ...
  -- 规则：
  --   1. 不得绕过 §3.4 decision tree
  --   2. runtime_alignment 变化可导致降级
  --   3. effective mode 不反写 channel_routes.switch_mode
```

### 3.5 Hot-Standby Level（V0.2.3 patch 1 修订，V0.2.4 Errata-4 彻底分离 definition / measurement）

> **🔴 V0.2.4 Errata-4 + Errata-8 关键锁定**：
> 1. `hot_standby_levels` **只保留** `description` / `target_failover_time_ms` / `resource_estimation.mode` / `use_case`。
> 2. **不携带** `state` / `benchmark` / `p50_ms` / `p95_ms` / `p99_ms` 字段。
> 3. HotStandbyLevel = **Policy / Target**（配置意图）；Runtime State 唯一由 §8.11 三轴状态机表达。
> 4. **禁止**任何"实测 50-200ms"或"target × N"固定范围。架构基线只声明 `target_failover_time_ms`（设计预算，**不构成协议保证**）。

```yaml
# Architecture Definition（semantic / 静态 / V0.2.4 Errata-6 删 state 字段）
# HotStandbyLevel 只描述策略 / 目标；当前真实状态由 §8.11 三轴状态机表达
hot_standby_levels:
  COLD:
    description: "进程未启动，输出断开"
    target_failover_time_ms: 30000
    resource_estimation:
      mode: GRAPH_CALCULATED
    use_case: "灾备录播，成本敏感"

  WARM:
    description: "进程已起，输入已锁，无完整输出"
    target_failover_time_ms: 1500
    resource_estimation:
      mode: GRAPH_CALCULATED
    use_case: "公共服务频道"

  HOT:
    description: "完整 pipeline 运行，可接管（READY_TO_TAKE 由 §8.11 三轴状态机判定）"
    target_failover_time_ms: 100
    resource_estimation:
      mode: GRAPH_CALCULATED
    use_case: "新闻/直播/广告插播"
```

> **🔴 V0.2.4 Errata-6 关键修正**：**删 `state: STOPPED / STARTING / READY_TO_TAKE` 字段**。`READY_TO_TAKE` 是 §8.11 Readiness 维度，不是 Hot-Standby Level 字段。HotStandbyLevel = 策略 / 目标；Runtime State = 当前事实。两层完全分离。

**Runtime Measurement**（独立表，§5）：

```sql
failover_benchmarks                          -- Architecture Definition 不含 benchmark 字段
  id, channel_id, route_id,
  switch_mode, hot_standby_level,
  measured_at, sample_count,
  p50_ms, p95_ms, p99_ms,
  test_profile_json, runtime_revision_id
```

> **🔴 V0.2.4 修正**：`expected_range_ms` 是猜的，会被慢慢当规范。架构基线只定 `target_failover_time_ms`。
>
> **🔴 V0.2.4 Cleanup-2 修正**：`resource_factor: 0.8` 同样会被当规范值。HOT 不使用固定 resource_factor，**由 Graph Compiler 根据两个实际 Runtime Graph 的 Resource Vector 求和**。
>
> **🔴 V0.2.4 Errata-3 修正**：**禁止**写"实测 50-200ms"或"target × 0.5 ~ target × 2"这种**固定范围**。架构基线只声明 `target_failover_time_ms`（设计预算，**不构成协议保证**），实际性能必须通过 `failover_benchmarks` 验证。
>
> **🔴 V0.2.4 Errata-4 修正**：`hot_standby_levels` 内**不再有** `benchmark` / `default_estimate` / `measured_factor` 字段；这些数据全部归 `failover_benchmarks` 表。`Architecture Definition ≠ Runtime Measurement`。

<!-- V0.2.4 Errata-8 删除旧句：状态字段是 READY_TO_TAKE，不是 RUNNING。READY_TO_TAKE 是 §8.11 Readiness 维度；HotStandbyLevel 不携带 state 字段。 -->

### 3.6 Latency Probes（V0.2 第 3 轮新增）

```yaml
# 7 个核心 Stage Probe + 2 个 Client E2E Probe + 1 个可选 CDN Probe
latency_probes:
  # ===== 7 个核心 Stage Probe（媒体管道，可靠）=====
  capture_ts:        # 1. SDI 抓帧时间戳（PTS）
  decode_ts:         # 2. 软解完成
  switch_ts:         # 3. 主备切换后
  compose_ts:        # 4. 图文烧录后
  encode_ts:         # 5. 编码后
  mux_ts:            # 6. 打包后
  publish_ts:        # 7. SRS 推送后

  # ===== 2 个 Client E2E Probe（需要时钟同步，否则 approx）=====
  source_reference_ts: # 源端 wall clock（用于校准）
  player_ts:         # 客户端收到

  # ===== 1 个可选 CDN Probe =====
  cdn_ts:            # CDN 分发后（optional）

e2e_latency_ms = player_ts - source_reference_ts

per_edge_latency_budget_ms: 50    # QC 检测
per_channel_latency_target_ms: 200  # QC 检测
```

> **🔴 V0.2.4 cleanup 修正**：原 §3.6 标题/术语表写"7 个测量点"但实际列了 9-10 个。现明确为 **7 Core + 2 Client E2E + 1 Optional CDN**。术语表同步：Latency Probe = 7 核心媒体阶段 + 可选 Client E2E / CDN 测量。

**🔴 V0.2.4 加 E2E 测量模式声明**（player_ts 不可靠）：

```yaml
e2e_measurement_modes:
  STAGE_LATENCY:                  # capture → publish，基于服务器时钟，可靠
    scope: [capture, decode, switch, compose, encode, mux, publish]   # 7 核心
    reliability: HIGH

  E2E_CLIENT_LATENCY:             # publish → player，需要时钟同步
    scope: [source_reference, publish, cdn, player]
    reliability: VARIABLE
    modes:
      SYNCHRONIZED_CLOCK:   # 浏览器与服务器时钟同步（PTP/NTP 精度高）
      EMBEDDED_MEDIA_PROBE: # 媒体流嵌入时间戳，播放器回报
      APPROXIMATE:          # 用 player_ts 但不保证精度
    default: APPROXIMATE
    note: "APPROXIMATE 模式下，E2E 指标标注为 approx，仅作参考"
```

### 3.7 Program Master（V0.2 第 3 轮重定义，V0.2.4 Errata-3 关键修正）⭐⭐

> **🔴 V0.2.4 Errata-3 关键边界锁定**：
>
> ```
> Program-scope Master  = RAW-domain semantic master
> Output Variant        = delivery-domain derivative
> Encode                = delivery boundary
> ```
>
> **禁止**任何把 `Program Master` 实现为 `H.264 / AAC` 等压缩域的实现。Master 一定在 RAW 域；进入压缩域是 **Encode 节点的责任**，发生在 Output Variant 阶段。

**Video / Audio / Metadata 是三个独立 graph**：

```
Video Graph:
  Source
   ↓ (RAW_VIDEO)
  [Normalize]
   ↓
  [Switcher]
   ↓
  [Program Composition]   ← 烧录节目级 Logo/Bug/字幕
   ↓ (RAW_VIDEO)
  [Video Master Join]
   ↓
  Program-scope Master (RAW_VIDEO)

Audio Graph:
  Source
   ↓ (RAW_AUDIO)
  [Audio Mixer]
   ↓
  [Loudness]
   ↓
  [Audio Delay]   ← 这里做 +80ms 补偿
   ↓
  [Audio Master Join]
   ↓
  Program-scope Master (RAW_AUDIO)

Metadata Graph:
  Timecode
  Subtitle (SRT/ASS)
  SCTE-35
   ↓
  [Metadata Master Join]
   ↓
  Program-scope Master (METADATA)
```

**完整链路**（Program-scope Master → Output Variant）：

```
Program-scope Master (RAW 域)
   ↓
[Variant Composition]   ← Variant-specific 包装
   ↓ (RAW_VIDEO + RAW_AUDIO)
[Encode]                ← ⭐ delivery boundary（RAW → COMPRESSED）
   ↓ (COMPRESSED_VIDEO + COMPRESSED_AUDIO)
[Output Variant]
   ↓
[delivery adapter]
   ├─ V0.2 enabled:
   │     ├─ SRSAdapter
   │     ├─ FileAdapter
   │     └─ UDPAdapter
   └─ V0.4 reserved (Architecture Contract: RESERVED / V0.2 Implementation: DISABLED):
         └─ SDIAdapter (SDI Master Output, Target V0.4, 决策 #49)
```

**AV Sync 测量在 Master Join 处**。**AV Sync 不再是"普通 Process Node"**——它是 Master Join 的属性。

#### 3.7.1 Composition 作用域分层（V0.2.4 patch 2 新增，V0.2.4 Cleanup-3 + Errata-3 修订）⭐

> **🔴 V0.2.4 解决 Program Master 与 Output Variant 之间的 Logo 归属冲突**。
> **🔴 V0.2.4 Cleanup-3 修正**："干净 Master" 措辞改为 **"Program-scope Master"**（节目主母版），明确**已完成 Program Scope Composition**（节目级 Logo/字幕/版权），但**不含 Variant-specific Composition**（平台 Logo/水印/区域版权）。
> **🔴 V0.2.4 Errata-3 关键锁定**：**Encode 是 delivery boundary**——把 RAW 域 Program-scope Master 编码为 COMPRESSED 域 Output Variant 的边界节点。

Composition Engine 按作用域分两级，但**仍是同一个 Engine**：

```
[Program Composition]   ← 烧录节目级 Logo/字幕/版权（所有 Variant 共享，RAW 域）
       ↓
[Program-scope Master]  ← 节目主母版（RAW 域）
       ↓
[Variant Composition]   ← 按 Output Variant 叠加 Variant-specific 包装（RAW 域）
       ↓
[Encode]                ← ⭐ delivery boundary（RAW → COMPRESSED）
       ↓
[Output Variant]        ← COMPRESSED 域；V0.2 enabled = SRS / File / UDP；SDI Adapter V0.2 DISABLED (V0.4)
```

**术语统一**：

| 名称 | 含义 | Data Plane |
|---|---|---|
| **Program-scope Master** | 已完成 Program Composition 的母版（**含**节目级 Logo/字幕/版权，**不含**平台 Logo/水印/区域版权） | **RAW**（ELEMENTARY） |
| **Clean Master** | **不存在的概念**；Program Master 一定含 Program Scope Composition | — |
| **Variant-scope Output** | 在 Program-scope Master 之上叠加 Variant Composition + Encode | **COMPRESSED**（ELEMENTARY） |

例：

```
Program-scope Master（RAW 域）
    ├── Variant A（国内版）  → Variant Composition → Encode → HLS
    ├── Variant B（海外版）  → Variant Composition → Encode → RTMP
    └── Variant C（归档版）  → 不叠加 Variant Composition → Encode → File
```

> **禁止**：任何把 `Program Master` 实现为 `H.264 / AAC` 等压缩域的实现。**Encode 是 delivery boundary**，**不**在 Master Join 之前。
```

> "Clean Master" **从术语表中删除**。统一用 **"Program-scope Master" / 节目主母版**。

### 3.8 AVSync Manager（V0.2 第 3 轮重定义）

不是 graph node，是**横切管理器**（X 范畴）：

```yaml
avsync_manager:
  measure_offset_ms: 42        # 当前 Video PTS - Audio PTS
  drift_ms_per_min: 3.2       # 每分钟漂移
  compensate_via:
    - audio_delay_ms: 80      # 给音频加 80ms 延迟
    - video_delay_ms: 0
    - pts_rewrite: false       # 还是真延时（更稳）
  auto_correct_drift: true    # 自动追漂移
  alarm_thresholds:
    yellow_ms: 100
    red_ms: 250
  exposes_metrics:
    - metric: av_offset_ms
      to: prometheus
    - metric: av_drift_ms_per_min
      to: prometheus
```

### 3.9 Health Tree（V0.2 第 3 轮新增）

把"频道红了"细化到子系统级：

```
CH01
 ├─ Source.A (SDI-01)         ●
 ├─ Source.B (SDI-02)         ●
 ├─ Switcher                  ●
 ├─ Composition               ●
 ├─ Audio Mixer               ●
 ├─ Video Master Join         ●
 ├─ Audio Master Join         ●
 ├─ Program Master            ●
 ├─ Output.SRS                ●
 │   ├─ HLS                   ●
 │   ├─ RTMP                  ●
 │   └─ WebRTC                ●
 ├─ Output.SDI                ●
 └─ Recording                 ●
```

每个节点独立健康度，UI 树形展示。**这是 X5（Health Tree）**。

**🔴 V0.2.3 加 Health Tree Aggregation Policy**（聚合规则，否则不同页面算出不同状态），**V0.2.4 Cleanup-3 修正 optional aggregation 注释矛盾**：

```yaml
health_tree_aggregation_policy:
  # V0.2.4 Errata-12 关键边界：required_node = "当前有效节目服务路径上的必需节点"
  # 不是"所有候选主备源同时必须 HEALTHY"。
  # Primary / Backup 是 Source Subsystem 内的候选成员，Backup 接管后 Primary=FAILED 仍可 HEALTHY（已冗余）。
  # V0.2.4 Errata-12 增注：runtime 维护 node_role（active / standby / offline）；
  #   - active 节点失败 → required_node=true, state=FAILED → Channel FAILED
  #   - standby 节点失败 → required_node=false, state=FAILED → Channel DEGRADED（失去 failover 候选）
  #   - offline 节点失败 → required_node=false, state=FAILED → 系统已吸收，Channel 不变
  #   - Source 全部候选 offline → Source subsystem required_node=FAILED → Channel FAILED
  #   实施：扩展 health_tree_nodes.role（active/standby/offline）或用 details_json 跟踪
  required_node:           # 当前有效服务路径上的必需节点
    meaning: "当前有效节目服务路径上的必需节点（Active Service Path）"
    healthy:   "all required_node.healthy"
    degraded:  "any required_node.degraded"
    failed:    "any required_node.failed"

  optional_node:           # 可选（备用/未激活的候选成员 / 旁路节点；含 standby + offline）
    healthy:   "all optional.healthy"
    degraded:  "any optional.degraded"
    failed:    "any optional.failed"   # standby failed → 失去 failover → Channel DEGRADED
                                        # offline failed → 系统已吸收（见 role 字段）

  derived_channel_state:
    healthy:
      "all required_node.healthy AND all optional_node.healthy"      # V0.2.4 Cleanup-3 修正
    degraded:
      "any required_node.degraded
       OR (all required_node.healthy AND any optional_node.degraded)
       OR (all required_node.healthy AND any optional_node.failed AND is_standby(optional_node))"
                                                                    # V0.2.4 Errata-12 修正
    failed:
      "any required_node.failed"                                    # 包括 Source 全部候选 offline 情形

# V0.2.4 Errata-12 关键澄清：
# required_node = Active Service Path 上的节点（不是"主备同时必须 HEALTHY"）
# Primary / Backup 共同构成 Source Subsystem；
#   Backup HEALTHY 且接管后，Primary FAILED 不影响 Channel Health（Primary 转 offline）。
# Source 全部候选都失败 → Source subsystem 视为 required_node=FAILED → Channel FAILED。
```

例（V0.2.4 Errata-12 修正）：
- **Source.Primary=FAILED, Backup=HEALTHY 且已接管, Program Master=HEALTHY, HLS=HEALTHY**
  → **Channel=HEALTHY**（Backup 接管后转 active=required 且 healthy；Primary 转 offline=optional，系统已吸收失败）
- **Source.Primary=FAILED, Backup=FAILED, Program Master=HEALTHY, HLS=HEALTHY**
  → **Channel=FAILED**（Source 全部候选失败，Source subsystem required_node=FAILED；Master/HLS 仍 HEALTHY 不改变结果）
- Source.Primary=FAILED, Backup=FAILED, Program Master=FAILED
  → Channel=`FAILED`（Active Service Path 上多个 required 失败）
- Backup=FAILED, Source.Primary=HEALTHY, 其他都正常
  → Channel=`DEGRADED`（Primary 在岗 required+healthy，但 Backup standby+failed → 失去 failover 候选）

### 3.10 Signal Graph Compiler（X1）（V0.2 第 3 轮新增）

用户画：

```
SRT-01 → PIP → HLS
```

系统自动编译成：

```
SRT-01
  ↓ COMPRESSED_VIDEO
[Decode]   ← 自动插入（因为下一个节点要 RAW_VIDEO）
  ↓ RAW_VIDEO
[PIP]      ← 用户的 PIP
  ↓ RAW_VIDEO
[Encode]   ← 自动插入（因为下一个节点要 COMPRESSED_VIDEO）
  ↓ COMPRESSED_VIDEO
[SRS]      ← 自动插入（因为 HLS 输出走 SRS）
  ↓ COMPRESSED_VIDEO
[HLS]
```

**编译流程**：

```yaml
graph_compiler_pipeline:
  input: GraphSpec (用户画的高层图)
  steps:
    1. validate: "Data Plane 是否合法？节点 contract 是否匹配？"
    2. insert_missing_nodes: "自动插入 Decode / Encode / Normalize 节点"
    3. clock_align: "标注 Clock Domain 转换点"
    4. latency_estimate: "估算每条边延迟，验证 Latency Budget"
    5. resource_plan: "Scheduler 计算资源需求（Resource Vector）"
    6. preflight: "Preflight 检查（X2）"
    7. emit: "GraphRuntime 派生"
  output: GraphRuntime (实际可执行)
```

**🔴 V0.2.3 加 Explainable Compile Preview**：

Graph Compiler **不能"偷偷"改变用户语义**，必须把"自动插入什么 + 消耗什么"显式告诉用户：

```yaml
explainable_compile_preview:
  user_spec: "SRT → PIP → HLS"

  compiled_graph:
    - SRT (Source)
    - [AUTO INSERTED] Decode       # reason: "next node needs RAW_VIDEO"
    - PIP (Composition)
    - [AUTO INSERTED] Encode       # reason: "next node needs COMPRESSED_VIDEO"
    - [AUTO INSERTED] SRS Adapter  # reason: "HLS via SRS Gateway"
    - HLS (Output)

  estimated_resources:
    cpu_threads_delta: +4.2
    ram_mb_delta:      +520
    nic_egress_mbps_delta: +4

  estimated_latency:
    per_edge_budget_total_ms: 38
    channel_latency_target_ms: 200

  warnings: []
  errors: []
```

> UI 上以"折叠"形式展示 `[AUTO INSERTED]`，点击展开 reason。**不允许默默改语义**。

### 3.11 Resource Vector（V0.2.3 patch 1 新增）⭐

> **🔴 V0.2.3 新增**：废弃笼统的 `resource_cost: LOW/MEDIUM/HIGH`（V0.2.4 Errata-13 进一步统一改名为 `descriptive_resource_class`，`scheduling_input: false`）。每个节点声明 9 维 Resource Vector，Scheduler 才能精确算。

```yaml
# ===== Architecture Resource Model =====
# 9-dim Quantitative Resource Vector + Device/Port Token Constraints
resource_model:
  quantitative_vector:
    dimensions: 9
    fields:
      - cpu_threads
      - gpu_sessions
      - ram_mb
      - vram_mb
      - ingress_mbps
      - egress_mbps
      - disk_write_mbps
      - pcie_rx_mb_s
      - pcie_tx_mb_s
    # 注：pcie_rx_mb_s / pcie_tx_mb_s 是 SCHEDULING ESTIMATE
    #     基于 media payload（见 §3.1 公式），≠ 实测 PCIe bus utilization
    #     实际 bus utilization 由 Hardware Capability Discovery 在运行时校准

  device_tokens:                 # V0.2.4 Errata-3 拆分 token vs constraint
    - BMD_INPUT_PORT              # Token: 分配单位
    - BMD_OUTPUT_PORT             # Token: 分配单位
  device_constraints:             # Constraint: 分配策略
    - DEVICE_EXCLUSIVITY         # 同一 port 同一时刻只能分配一个 node
```

**节点示例**（Quantitative Vector）：

```yaml
decode_node:
  resource_vector:
    cpu_threads: 4
    ram_mb: 200
    ingress_mbps: 8          # 压缩域
    pcie_rx_mb_s: 0

encode_node:
  resource_vector:
    cpu_threads: 4
    ram_mb: 200
    egress_mbps: 8
    pcie_tx_mb_s: 0

sdi_capture_node:
  resource_vector:
    cpu_threads: 0.5
    ram_mb: 100
    pcie_rx_mb_s:
      mode: ESTIMATED_FROM_MEDIA_PAYLOAD
      formula: "raw_video_payload_bytes_per_second"
      inputs: [resolution, fps, pixel_format, field_order, device_mode]
      # 1080p25 UYVY422 ≈ 103.68 MB/s；1080p25 V210 ≈ 138.24 MB/s（见 §3.1）
      # 注：此为 scheduling estimate，不等于实测 PCIe bus utilization
      measurement:
        optional: true
        source: "runtime hardware telemetry"

composition_node:
  resource_vector:
    gpu_sessions: 1
    vram_mb: 600
    cpu_threads: 2
    ram_mb: 800
```

**Scheduler 公式**：

```yaml
scheduling_decision:
  available:
    # ===== 9 维 Quantitative Vector（来源：Hardware Capability Discovery）=====
    cpu:               discovered.cpu.available_threads       # 不写死
    gpu_sessions:      discovered.gpu.available_sessions
    ram_mb:            discovered.memory.available_mb
    vram_mb:           discovered.gpu.available_vram_mb
    ingress_mbps:      discovered.network.ingress_mbps
    egress_mbps:       discovered.network.egress_mbps
    disk_write_mbps:   discovered.storage.write_mbps
    pcie_rx_mb_s:      discovered.pcie.rx_mb_s
    pcie_tx_mb_s:      discovered.pcie.tx_mb_s

    # ===== Device Tokens（来源：Hardware Capability Discovery）=====
    bmd_devices:       discovered.bmd.devices   # 设备/端口级 token，非笼统 channels

  Σ all_nodes:
    # 9 维 Vector 累加
    cpu:               ≤ available.cpu
    gpu_sessions:      ≤ available.gpu_sessions
    ram_mb:            ≤ available.ram_mb
    vram_mb:           ≤ available.vram_mb
    ingress_mbps:      ≤ available.ingress_mbps
    egress_mbps:       ≤ available.egress_mbps
    disk_write_mbps:   ≤ available.disk_write_mbps
    pcie_rx_mb_s:      ≤ available.pcie_rx_mb_s
    pcie_tx_mb_s:      ≤ available.pcie_tx_mb_s
    # Device Tokens
    bmd_input_ports:   ≤ Σ(available.bmd_devices[*].input_ports)
    bmd_output_ports:  ≤ Σ(available.bmd_devices[*].output_ports)

  device_exclusivity: "同一 port 同一时刻只能分配一个 node（device token + port token）"

  → PASS: 全部满足
  → WARN: 单项超 80%
  → FAIL: 任一项超 100%
```

**当前服务器硬件快照**（非架构事实，仅本次部署参考）：

```yaml
current_host_snapshot:
  server: "10.30.15.10 (Ubuntu 26.04)"
  cpu: "32 核"
  memory: "30 GB"
  storage: "546 GB"
  gpu:
    detected: false
    available_sessions: 0
  bmd_devices:
    - model: "DeckLink Duo 2"            # 具体型号由 Runtime Discovery 确认
      input_ports: 2
      output_ports: 2
    - model: "DeckLink SDI"              # 具体型号由 Runtime Discovery 确认
      input_ports: 1
      output_ports: 1
    - model: "DeckLink Mini Monitor 4K"  # 具体型号由 Runtime Discovery 确认
      input_ports: 0
      output_ports: 1
  # 实际型号/serial/capability 必须由 Hardware Capability Discovery
  # 在 Media Agent 启动时通过 BMD API 查询；不作为架构事实。
```

> **🔴 V0.2.4 Errata-2 关键修正**：
> 1. **9-dim Resource Vector + Device/Port Token Constraints** 是正式术语，§1.22 / §3.11 / 决策 #41 全部统一。
> 2. **GPU / BMD / NIC / PCIe 都是 Runtime Discovery 产物**，Architecture 不写死具体数量或型号。
> 3. **`pcie_*_mb_s` 是 SCHEDULING ESTIMATE**（基于 media payload），**≠ 实测 PCIe bus utilization**。后者由 hardware telemetry 校准。
> 4. **当前服务器硬件是 `current_host_snapshot`**，与 Architecture 解耦。机器增加 GPU 不需要修改 V0.2。

### 3.12 Clock Reference（V0.2.3 patch 1 新增）⭐

> **🔴 V0.2.3 新增**：Clock Domain 不够，还要声明**Clock Reference Source**。

```yaml
clock_reference:
  domain: PTP               # SYSTEM / MONOTONIC / MEDIA / TIMECODE / PTP
  reference_id: ptp0        # 哪个 PTP 实例 / 哪个 source / 哪个 timecode generator
  priority: 100             # 0-255，主备优先级
  fallback_chain:           # PTP 失锁时降级
    - { domain: TIMECODE, reference_id: sdi01_timecode, priority: 50 }
    - { domain: SYSTEM, reference_id: local_ntp, priority: 10 }
```

Channel 表新增字段：

```sql
channels:
  clock_domain ENUM('SYSTEM','MONOTONIC','MEDIA','TIMECODE','PTP')
  clock_reference_id VARCHAR    -- 'ptp0' / 'sdi01_timecode' / 'local_ntp'
  clock_priority INT DEFAULT 100
  clock_fallback_chain JSONB
```

> **同一 Channel 内所有 Latency Probe / AVSync / Recording 时间戳都按此 reference 校准**。

**🔴 V0.2.4 加 clock_quality 等级**（fallback 降级 ≠ 同等级时钟）：

```yaml
clock_quality:
  PTP:        BROADCAST_GRADE   # 广播级，μs 级
  TIMECODE:   MEDIA_GRADE       # 媒体级，frame 级
  MEDIA:      SOURCE_GRADE      # 源级，取决于 source
  SYSTEM:     BEST_EFFORT       # 系统时钟，ms 级
  MONOTONIC:  LOCAL_ONLY        # 进程本地
```

发生降级时（如 PTP 丢失 → TIMECODE），系统应产生 **`CLOCK_DEGRADED` 事件**，Health Tree 标 YELLOW，**不能让系统看起来仍然 GREEN**。

降级表：

| 当前 | 降级到 | 触发事件 |
|---|---|---|
| PTP | TIMECODE | CLOCK_DEGRADED (YELLOW) |
| TIMECODE | MEDIA | CLOCK_DEGRADED (YELLOW) |
| MEDIA | SYSTEM | CLOCK_DEGRADED (YELLOW) |
| SYSTEM | MONOTONIC | CLOCK_FAILED (RED) |

### 3.13 AVSync Offset Correction vs Drift Correction（V0.2.3 patch 1 修订，V0.2.4 Errata-9 角色明确）

> **🔴 V0.2.4 Errata-9 关键修正**：**AVSync Manager 不再是"identification only"**。它的角色是 **Measurement + Offset/Drift Correction + Failure Classification**。**不**负责最终 Failure Recovery Action——恢复动作由 §8.9 Failure Domain Matrix 决定。

```yaml
avsync_manager:
  measure_offset_ms: 42        # 当前 Video PTS - Audio PTS
  measure_drift_ms_per_min: 3.2

  offset_correction:           # ⭐ V0.2.3 独立
    trigger: "abs(offset_ms) > 40ms"
    action: "apply audio_delay (one-shot)"
    example: "offset=+120ms, drift=0 → audio_delay += 120ms 一次性"

  drift_correction:            # ⭐ V0.2.3 独立
    trigger: "abs(drift_ms_per_min) > 2ms/min"
    action: "gradual compensation (per N seconds)"
    example: "offset=+40ms, drift=+4ms/min → 持续追，不一次性加"

  classify_before_action:      # V0.2.4 Errata-6 + Errata-9 关键修正：只负责识别 + 分类，**不决定**最终恢复动作
    when: "abs(offset_ms) > threshold"
    classify:
      source_pts_unstable:
        diagnostic_class: SOURCE_PTS_UNSTABLE
        operational_failure_domain: SOURCE         # 7 域之一
        # NOTE: §3.13 只做识别 + 分类；最终恢复动作由 §8.9 Failure Domain Matrix 决定
        # 禁止 §3.13 直接指定 action（避免与 §8.9 SOURCE → FAILOVER 冲突）
      pipeline_slow:
        diagnostic_class: PIPELINE_SLOW
        operational_failure_domain: PIPELINE
      output_mux_error:
        diagnostic_class: OUTPUT_MUX_ERROR
        operational_failure_domain: OUTPUT
      player_buffer_anomaly:
        diagnostic_class: PLAYER_BUFFER_ANOMALY    # 诊断分类
        operational_failure_domain: null              # PLAYER 不进入 7 OperationalFailureDomain
        # PLAYER 走 DiagnosticFailureClass；只 NOTIFY，不切源
    routing:
      "§3.13 负责 identification + classification"
      "§8.9 Failure Domain Matrix 决定最终恢复动作"
      "§8.9 是 Recovery Policy 的 Single Source of Truth"
      "PLAYER/UNKNOWN 走 DiagnosticFailureClass；不进入 7 OperationalFailureDomain"
```

> **🔴 V0.2.4 Errata-6 关键边界锁定**：
> - **§3.13 = 识别层**：只负责"这是哪一类异常"
> - **§8.9 = 决策层**：决定"对应失败域的恢复动作"（SOURCE → FAILOVER；OUTPUT → RESTART/alternate；PLAYER → NOTIFY 等）
> - **禁止** §3.13 直接指定 action（避免与 §8.9 SOURCE → FAILOVER 冲突）
> - 同一份 Baseline 给两名工程师，他们必须得出**同一份**故障恢复流程

### 3.14 Output Engine：SRS = Gateway Adapter（V0.2 第 3 轮收紧）

> ❌ 错误：SRS 是 Output Engine 的子项
> ✅ 正确：SRS 是 Output Engine 下面的 **Gateway Adapter**

```yaml
output_engine:
  direct_output_adapters:
    SDIAdapter: 
      output: RAW_VIDEO + RAW_AUDIO
    UDPAdapter:
      output: COMPRESSED_VIDEO + COMPRESSED_AUDIO
    RTPAdapter:
      output: COMPRESSED_AUDIO + COMPRESSED_VIDEO
    FileAdapter:
      output: COMPRESSED_VIDEO + COMPRESSED_AUDIO (MP4/TS)

  stream_gateway_adapters:
    SRSAdapter:
      role: "Stream Gateway (RTMP ingest, HLS/WebRTC/HTTP-FLV egress)"
      output: COMPRESSED_VIDEO + COMPRESSED_AUDIO
      protocol: SRS
    Future2110Adapter:  # 占位
      role: "SMPTE 2110"
      output: RAW_*  # 2110 是 raw over IP
```

**SRS 不承载"Output Engine 的全部职责"**，它只承载**协议网关和分发**。Output Engine 本身包含所有 Adapter 的统一管理。

---

## 第 4 部分：进程与角色模型（含 Graph Compiler）

### 4.1 角色（V0.2 第 2 轮 + 新增）

| 角色 | 位置 | 进程类型 |
|---|---|---|
| Media Agent | Host (10.30.15.10) | Rust 单文件 + JSON-RPC |
| Media Controller | Docker | Fastify，状态机引擎 |
| **Graph Compiler** ⭐ | **Docker** | **Fastify worker**（集成在 Controller） |
| API | Docker | Fastify，stateless |
| Web | Docker | Vite 静态 |
| Worker (Job) | Docker | BullMQ（转码、probe、上传） |
| Worker (Recording) | Docker 或 Host | 持续 |
| Postgres / Valkey / RustFS / SRS | Docker | 各自 |
| Prometheus + Grafana | Docker | 可选 |

### 4.2 Graph Compile 链路

```
Web 用户画图
   ↓ (HTTP POST /graph-specs/:id/compile)
API → Media Controller
   ↓
Graph Compiler:
  1. Validate  (X2 Preflight 内的 Graph Preflight)
  2. Insert Missing Nodes
  3. Clock Align
  4. Latency Estimate
  5. Resource Plan  (H2 Resource Scheduler)
  6. Emit GraphRuntime
   ↓
Media Agent 启动 Session
   ↓
Graph Runtime 上线
```

---

<a id="第-5-部分数据模型核心表"></a>

## 第 5 部分：数据模型（核心表，V0.2 第 3 轮更新）

> 上一版 30+ 张。本轮重点新增：
> - `switch_modes`, `hot_standby_levels`
> - `signal_contracts`, `node_contracts`
> - `latency_probes`
> - `health_trees`
> - `preflight_runs`, `preflight_results`
> - `config_revisions`, `change_sets`
> - `incident_timeline`
> - `avsync_measurements`
> - 三个 graph 拆开：`channel_video_graph`, `channel_audio_graph`, `channel_metadata_graph`

```sql
-- ===== 身份 / 权限 =====
users, roles, permissions, user_roles

-- ===== 设备 =====
media_devices

-- ===== 信源 / 当前状态 =====
sources
signal_current_state

-- ===== Graph: 设计 vs 运行时 =====
graph_specs
graph_revisions
graph_runtimes
  -- 🔴 V0.2.3 增字段（事故追溯需要知道"当时跑的是哪一版"）：
  graph_revision_id  UUID NOT NULL REFERENCES graph_revisions(id)   -- GraphSpec 版本
  config_revision_id  UUID REFERENCES config_revisions(id)          -- 关联的 ConfigRevision
  change_set_id       UUID REFERENCES change_sets(id)               -- 关联的 ChangeSet
  activated_at        TIMESTAMPTZ
graph_runtime_nodes
graph_runtime_edges

-- ===== Channel =====
channels
channel_routes
  -- ⭐ 新增字段：
  switch_mode ENUM('PACKET_SWITCH','FRAME_SWITCH','MASTER_SWITCH')
  hot_standby_level ENUM('COLD','WARM','HOT')
  failover_hysteresis_ms INT DEFAULT 2000
  failback_hysteresis_ms INT DEFAULT 5000
  min_hold_ms INT DEFAULT 10000

-- ⭐ 新增：Video / Audio / Metadata 三个独立 graph
channel_video_graph
channel_audio_graph
channel_metadata_graph

-- ===== Output =====
output_variants
output_destinations

-- ===== Latency / Clock =====
latency_budgets
latency_probes                   -- ⭐ V0.2.4 Errata-1: 增 source_reference + measurement_mode
  probe_point ENUM('source_reference','capture','decode','switch',
                    'compose','encode','mux','publish','cdn','player')
  measurement_mode ENUM('STAGE_LATENCY','SYNCHRONIZED_CLOCK',
                         'EMBEDDED_MEDIA_PROBE','APPROXIMATE')
  probe_ts_ms BIGINT
  edge_id, channel_id

clock_domain_mappings

-- ===== Health Tree (X5) =====
health_trees                       -- 历史快照表
  channel_id, snapshot_ts

health_tree_nodes                  -- V0.2.4 Errata-11 修：加 required_node + UNIQUE + state UPPER_CASE
                                       -- V0.2.4 Errata-13 增：node_role（ACTIVE/STANDBY/OFFLINE，§3.9 语义锁死）
                                       -- V0.2.4 Errata-14 增：subsystem + redundancy_group_id（机器可判定的"候选组"关系）
  health_tree_id UUID NOT NULL REFERENCES health_trees(id)
  node_path VARCHAR NOT NULL
  -- V0.2.4 Errata-14 增：subsystem（机器可判定节点属于哪个子系统）
  --   用于 channel_health_aggregation SQL 按子系统聚合
  subsystem ENUM('SOURCE','SWITCHER','COMPOSITION','AUDIO','MASTER','OUTPUT','RECORDING','CLOCK','RESOURCE') NOT NULL
  -- V0.2.4 Errata-14 增：redundancy_group_id（机器可判定"候选组"关系）
  --   同一 redundancy_group_id 的节点构成一个冗余组（Primary + Backup = 同一组）
  --   NULL = 单一节点（Master / HLS / 等无冗余的节点）
  --   切换 OFFLINE 时 **必须保持** 同一 redundancy_group_id（不能"丢"原组）
  redundancy_group_id UUID NULL
  -- V0.2.4 Errata-13 增：node_role（canonical 字段，§3.9 三种语义进 DB）
  --   node_role = Runtime Fact（该节点当前在 active service path 上的角色）
  --   required_node = Derived Snapshot Attribute（持久化以加速查询，源 = node_role）
  --   invariant 强制：ACTIVE → required_node=TRUE；STANDBY/OFFLINE → required_node=FALSE
  --   写入：仅 Runtime 写 node_role；required_node 由 trigger / 应用层从 node_role 派生
  --   禁止：Runtime 独立写 required_node 与 node_role 矛盾
  node_role ENUM('ACTIVE','STANDBY','OFFLINE') NOT NULL
  -- V0.2.4 Errata-14 修订：required_node 语义从"事实"降级为"derived snapshot"
  required_node BOOLEAN NOT NULL
  -- V0.2.4 Errata-11 修：state UPPER_CASE（与 Canonical Vocabulary 一致）
  state ENUM('HEALTHY','DEGRADED','FAILED','UNKNOWN') NOT NULL
  details_json JSONB

-- V0.2.4 Errata-13 增：health_tree_node_role_invariant（不可歧义）
-- 校验约束（应用层 + 定期 DBA 检查）：
--   ACTIVE   → required_node MUST BE TRUE
--   STANDBY  → required_node MUST BE FALSE
--   OFFLINE  → required_node MUST BE FALSE
-- 实施：CHECK 约束 + 应用层 validation 双层保证
health_tree_node_role_invariant:
  ACTIVE:   { required_node: TRUE  }
  STANDBY:  { required_node: FALSE }
  OFFLINE:  { required_node: FALSE }

-- V0.2.4 Errata-14 锁定：node_role = SoT, required_node = derived
node_role_authority:
  source_of_truth: node_role                # Runtime 写 node_role
  required_node:
    persisted: true
    source_of_truth: node_role              # derived from node_role
    writable_by_runtime: false              # 禁止 Runtime 独立写
    derived_from:
      ACTIVE:   true
      STANDBY:  false
      OFFLINE:  false
    enforcement: "CHECK 约束 + 应用层 trigger 双层保证"
  -- V0.2.4 Errata-11 增：snapshot 内 node_path 唯一
  UNIQUE (health_tree_id, node_path)
  -- 注：channel_id 不在这里（归属由 health_trees.channel_id 决定）

-- ===== AV Sync =====
avsync_measurements                -- ⭐ 新增
  channel_id, ts, offset_ms, drift_ms_per_min,
  audio_delay_ms, video_delay_ms, status

-- ===== Switch Mode / Hot-Standby =====
switch_modes                       -- V0.2.4 Errata-5 修正：删 max_failover_ms（target 唯一来源 = hot_standby_levels.target_failover_time_ms）
  id, name, description
  -- max_failover_ms 废除：target 唯一来源是 hot_standby_levels.target_failover_time_ms
  -- 实测唯一来源是 failover_benchmarks（§5）
  -- 形成：Switch Mode → Hot-Standby Level → target_failover_time_ms → failover_benchmarks 唯一链路

hot_standby_levels                 -- V0.2.4 Errata-2 + Errata-3 修正：resource_factor 已废弃，不入 lookup
  id, name, description, target_failover_time_ms
  -- resource_factor 废除（语义属于"运行时按 Graph 计算"而非 lookup 字段）
  -- benchmark 实测进独立表 failover_benchmarks（下方）

-- V0.2.4 Errata-3 新增：failover_benchmarks（runtime measurement）
-- 与 hot_standby_levels lookup 表完全分离
failover_benchmarks
  id
  channel_id
  route_id
  switch_mode
  hot_standby_level
  measured_at
  sample_count
  p50_ms
  p95_ms
  p99_ms
  test_profile_json       -- 测试条件（codec/resolution/fps/encoder/HW 等）
  runtime_revision_id     -- 关联 graph_runtimes
  -- 实际值进 runtime benchmark 表（p50/p95/p99 / last_measured_at）

-- ===== Signal Contract (X6) =====
signal_contracts                   -- ⭐ 新增
  id, name, video_resolution, video_fps, video_pixel_format,
  video_field_order, video_color_space, video_color_range,
  audio_sample_rate, audio_channels, audio_layout,
  codec_video, codec_audio, container

node_contracts                      -- ⭐ 新增
  node_id, input_contracts[], output_contracts[],
  capabilities_json

-- ===== Preflight (X2) =====
preflight_runs                     -- ⭐ 新增
  id, type ENUM('GRAPH','PLAYOUT','CHANNEL'),
  target_id, started_at, completed_at, result, score

preflight_results                  -- ⭐ 新增
  run_id, check_name, severity, message, details_json

-- ===== Configuration Versioning (X3) =====
config_revisions                   -- ⭐ 新增
  id, target_type (ChannelConfig|OutputProfile|AudioProfile|GraphicTemplate|QCProfile|...),
  target_id, version, definition_json, created_by, created_at,
  status ENUM('draft','active','archived')

change_sets                         -- ⭐ V0.2.4 Errata-1 明确：status=business outcome, events=transaction phase
  id, name,
  status ENUM('draft','validated','applied','rolled_back')  -- 业务结果（终态）
  scheduled_at, applied_at, change_set_items[]

-- transaction phase 不进 status，进 change_set_events：
change_set_events                  -- ⭐ V0.2.4 Errata-1 新增：执行阶段事件
  change_set_id, ts, phase ENUM('PREPARING','APPLYING','COMMITTED','ABORTED'),
  message, payload_json

change_set_items                   -- ⭐ 新增
  change_set_id, target_type, target_id, before_rev, after_rev

-- ===== Incident Timeline (X4) =====
incidents                           -- ⭐ 新增 (was safety_incidents)
  id, opened_at, closed_at, severity, summary, channel_id

incident_timeline_events            -- ⭐ 新增
  incident_id, ts, source, type, message, payload_json

-- ===== Asset / Rights / Playlist =====
media_assets, asset_versions, asset_rights
playlists, playlist_items

-- ===== Composition =====
composition_templates, composition_layers

-- ===== 录制 =====
recordings, recording_segments

-- ===== Session / Job =====
media_sessions, media_session_attempts
media_jobs, media_job_attempts

-- ⭐ V0.2.4 Errata-9 增：media_session_runtime（承载 EFFECTIVE_RUNTIME_MODE + 三轴状态）
media_session_runtime
  session_id UUID PRIMARY KEY REFERENCES media_sessions(id)
  channel_id UUID NOT NULL

  -- §8.11 三轴状态机（Runtime 写入）
  lifecycle ENUM('STOPPED','STARTING','RUNNING','STOPPING')
  readiness ENUM('NOT_READY','READY_TO_TAKE')
  health    ENUM('HEALTHY','DEGRADED','FAILED','UNKNOWN')

  -- ⭐ EFFECTIVE_RUNTIME_MODE（V0.2.4 Errata-6 锁死）
  effective_switch_mode ENUM('PACKET_SWITCH','FRAME_SWITCH','MASTER_SWITCH')
  effective_since TIMESTAMPTZ

  runtime_alignment_state JSONB   -- runtime_alignment.required 各项当前状态
  active_source_id UUID
  runtime_revision_id UUID REFERENCES graph_runtimes(id)
  updated_at TIMESTAMPTZ

  -- 规则：
  --   1. 不得绕过 §3.4 decision tree
  --   2. runtime_alignment 变化可导致降级
  --   3. effective_switch_mode 不反写 channel_routes.switch_mode
  --   4. Runtime 写入；不是 Configuration 写入

-- ⭐ V0.2.4 Errata-9 + Errata-10 + Errata-11 增：channel_health_view（derived presentation）
-- 关键：effective_channel_status 来自 §3.9 Health Tree Aggregation Policy（SoT）
--      不是直接从 media_session_runtime.health 派生
-- media_session_runtime 是 Runtime Fact；§3.9 才是 Channel Health 的 SoT
-- V0.2.4 Errata-12 显式声明 source 链：
--   runtime_fact (media_session_runtime: lifecycle/readiness/health)
--       +
--   health_tree_aggregation (channel_health_aggregation: aggregated_state)
--       ↓
--   effective_channel_status_policy (precedence + rules)
--       ↓
--   channel_health_view (CASE 真正执行 Policy)
-- UI / API 唯一读取 effective_channel_status 的入口 = channel_health_view

-- 一级：Current Health Tree（每个 Channel 取最新 snapshot_ts）
--      Health Tree History = health_trees（事实表）
--      Current Health Tree = current_health_trees（Latest View）
CREATE VIEW current_health_trees AS
SELECT DISTINCT ON (channel_id)
  id,
  channel_id,
  snapshot_ts
FROM health_trees
ORDER BY channel_id, snapshot_ts DESC;

-- 二级：Channel Health Aggregation（基于 Current Health Tree，V0.2.4 Errata-14 重写）
-- 7 规则（5 来自 Errata-13 + 2 来自 Errata-14）：
--   1. ACTIVE+FAILED                  → FAILED     (active service path broken)
--   2. ACTIVE+DEGRADED                → DEGRADED   (active service path degraded)
--   3. Source RG 全部候选不可用         → FAILED     (服务真的死了)
--   4. Source RG 没有 ACTIVE 节点       → DEGRADED   (pending takeover; 取代原"pending 错误为 HEALTHY"的 bug)
--   5. STANDBY+(DEGRADED|FAILED)       → DEGRADED   (failover capability lost)
--   6. 至少一个 ACTIVE+HEALTHY
--      且无 ACTIVE/STANDBY 在 DEGRADED/FAILED/UNKNOWN → HEALTHY
--      (V0.2.4 Errata-14: UNKNOWN 不得被吸收为 HEALTHY)
--   7. 其余                            → UNKNOWN
CREATE VIEW channel_health_aggregation AS
SELECT
  ht.channel_id,
  CASE
    -- 1. ACTIVE service path failed → FAILED
    WHEN EXISTS (
      SELECT 1 FROM health_tree_nodes h
      WHERE h.health_tree_id = ht.id
        AND h.node_role = 'ACTIVE'
        AND h.state = 'FAILED'
    ) THEN 'FAILED'

    -- 2. ACTIVE service path degraded → DEGRADED
    WHEN EXISTS (
      SELECT 1 FROM health_tree_nodes h
      WHERE h.health_tree_id = ht.id
        AND h.node_role = 'ACTIVE'
        AND h.state = 'DEGRADED'
    ) THEN 'DEGRADED'

    -- 3. Source RG 全部候选不可用 → FAILED (V0.2.4 Errata-14 关键新增)
    --    "不可用" = node_role=OFFLINE OR state IN (FAILED, UNKNOWN)
    --    修复 Errata-13 漏的 bug：Primary=OFFLINE+FAILED, Backup=OFFLINE+FAILED 不应得 HEALTHY
    WHEN EXISTS (
      SELECT 1
      FROM health_tree_nodes h
      WHERE h.health_tree_id = ht.id
        AND h.subsystem = 'SOURCE'
        AND h.redundancy_group_id IS NOT NULL
        AND NOT EXISTS (
          SELECT 1 FROM health_tree_nodes h2
          WHERE h2.health_tree_id = ht.id
            AND h2.redundancy_group_id = h.redundancy_group_id
            AND h2.subsystem = 'SOURCE'
            AND h2.node_role IN ('ACTIVE', 'STANDBY')
            AND h2.state NOT IN ('FAILED', 'UNKNOWN')
        )
    ) THEN 'FAILED'

    -- 4. Source RG 没有 ACTIVE 节点 → DEGRADED (V0.2.4 Errata-14 关键新增, pending takeover)
    --    Primary=OFFLINE+FAILED, Backup=STANDBY+HEALTHY → 接管尚未发生 → DEGRADED
    --    (HA-02 验收用例: "取决于是否已接管"，未接管时 DEGRADED)
    WHEN EXISTS (
      SELECT 1
      FROM health_tree_nodes h
      WHERE h.health_tree_id = ht.id
        AND h.subsystem = 'SOURCE'
        AND h.redundancy_group_id IS NOT NULL
        AND NOT EXISTS (
          SELECT 1 FROM health_tree_nodes h2
          WHERE h2.health_tree_id = ht.id
            AND h2.redundancy_group_id = h.redundancy_group_id
            AND h2.subsystem = 'SOURCE'
            AND h2.node_role = 'ACTIVE'
        )
    ) THEN 'DEGRADED'

    -- 5. STANDBY 候选降级/失败 → DEGRADED (failover capability lost)
    WHEN EXISTS (
      SELECT 1 FROM health_tree_nodes h
      WHERE h.health_tree_id = ht.id
        AND h.node_role = 'STANDBY'
        AND h.state IN ('DEGRADED', 'FAILED')
    ) THEN 'DEGRADED'

    -- 6. 至少一个 ACTIVE+HEALTHY 存在
    --    且无 ACTIVE/STANDBY 在 DEGRADED/FAILED/UNKNOWN
    --    → HEALTHY
    --    (V0.2.4 Errata-14 关键修复: UNKNOWN 不得被吸收为 HEALTHY)
    WHEN EXISTS (
      SELECT 1 FROM health_tree_nodes h
      WHERE h.health_tree_id = ht.id
        AND h.node_role = 'ACTIVE'
        AND h.state = 'HEALTHY'
    ) AND NOT EXISTS (
      SELECT 1 FROM health_tree_nodes h
      WHERE h.health_tree_id = ht.id
        AND h.node_role IN ('ACTIVE', 'STANDBY')
        AND h.state IN ('DEGRADED', 'FAILED', 'UNKNOWN')
    ) THEN 'HEALTHY'

    -- 7. 其余 → UNKNOWN
    ELSE 'UNKNOWN'
  END AS aggregated_state
FROM current_health_trees ht;

-- 三级：Channel Health View（effective_channel_status 真正执行 effective_channel_status_policy）
-- V0.2.4 Errata-12：用 CASE 表达 Policy；LEFT JOIN 处理 STOPPED/STARTING 无 Health Tree
-- 完整定义见下方（避免重复定义）
-- CREATE VIEW channel_health_view ... → 见下面"顶层视图"

-- 完整链路：
--   health_trees（事实）
--   → current_health_trees（Latest snapshot per channel）
--   → channel_health_aggregation（§3.9 Health Tree Aggregation Policy SoT）
--   → channel_health_view（effective_channel_status，UI / API 派生展示）

-- ⭐ V0.2.4 Errata-11 增：effective_channel_status_policy（V0.2 唯一映射规则）
-- Lifecycle 优先于 Health；terminal lifecycle 直接覆盖 Health 派生
effective_channel_status_policy:
  precedence:                          # 不是数值排序，是判定顺序
    - lifecycle_terminal               # STOPPED
    - lifecycle_transition             # STARTING / STOPPING → 展示为 STARTING
    - health_tree_aggregation          # FAILED / DEGRADED / HEALTHY
    - unknown                           # UNKNOWN

  rules:
    STOPPED:
      when: { lifecycle: STOPPED }
      result: STOPPED

    STARTING:
      when: { lifecycle_in: [STARTING, STOPPING] }
      result: STARTING                # STOPPING 也展示为 STARTING（避免 enum 膨胀）

    FAILED:
      when: { health_tree_aggregation: FAILED }
      result: FAILED

    DEGRADED:
      when: { health_tree_aggregation: DEGRADED }
      result: DEGRADED

    HEALTHY:
      when:
        lifecycle: RUNNING
        health_tree_aggregation: HEALTHY
      result: HEALTHY

    UNKNOWN:
      otherwise: UNKNOWN

  # 闭集证明：EffectiveChannelStatus = {STOPPED, STARTING, FAILED, DEGRADED, HEALTHY, UNKNOWN}
  #           每个 Channel 必落到且仅落到 1 个值
  #           不再出现"无法判定"或"超 enum"的情况

-- 顶层视图：effective_channel_status 真正执行 effective_channel_status_policy
-- V0.2.4 Errata-12 修正：用 CASE 表达 Policy；LEFT JOIN 处理 STOPPED/STARTING 无 Health Tree 的情况
-- V0.2.4 Errata-13 修正：去掉 `msr.health = 'HEALTHY'` 判定；Health Tree 是 Channel Health 唯一 SoT
--   msr.health 仅作为 Runtime Fact / UI 下钻信息，不参与 effective_channel_status 计算
-- 唯一定义点：本文档 §5 channel_health_view；C.24 + C.25 锁定为"Channel Health 派生展示 SoT"
CREATE VIEW channel_health_view AS
SELECT
  msr.channel_id,
  -- Runtime 三轴（fact，纯下钻信息，不参与 effective_channel_status 计算）
  msr.lifecycle,
  msr.readiness,
  msr.health,
  -- 真正执行 effective_channel_status_policy（V0.2.4 Errata-12 + Errata-13）
  -- SoT 链：lifecycle (Runtime Fact) → health_tree_aggregation (§3.9 SoT) → effective_channel_status
  CASE
    -- precedence 1: lifecycle_terminal
    WHEN msr.lifecycle = 'STOPPED'
      THEN 'STOPPED'

    -- precedence 2: lifecycle_transition（STOPPING 也展示为 STARTING）
    WHEN msr.lifecycle IN ('STARTING', 'STOPPING')
      THEN 'STARTING'

    -- precedence 3: health_tree_aggregation（§3.9 SoT，Node Role 语义：ACTIVE/STANDBY/OFFLINE）
    WHEN cha.aggregated_state = 'FAILED'
      THEN 'FAILED'

    WHEN cha.aggregated_state = 'DEGRADED'
      THEN 'DEGRADED'

    -- precedence 4: 仅当 lifecycle=RUNNING + health_tree_aggregation=HEALTHY 才 HEALTHY
    --   msr.health 不再参与判定（V0.2.4 Errata-13 修正）
    --   即：msr.health=DEGRADED + cha.aggregated_state=HEALTHY → effective_channel_status=HEALTHY
    --       msr.health=FAILED + cha.aggregated_state=HEALTHY  → effective_channel_status=HEALTHY
    --   msr.health 仅用于 UI 下钻 / 诊断详情
    WHEN msr.lifecycle = 'RUNNING'
     AND cha.aggregated_state = 'HEALTHY'
      THEN 'HEALTHY'

    -- 其余 UNKNOWN
    ELSE 'UNKNOWN'
  END AS effective_channel_status
FROM media_session_runtime msr
LEFT JOIN channel_health_aggregation cha
  ON cha.channel_id = msr.channel_id;
  -- LEFT JOIN 原因：STOPPED/STARTING 状态下未必有 Current Health Tree

-- 注：effective_channel_status 类型 = EffectiveChannelStatus
-- 详见 Canonical Vocabulary（V0.2.4 Errata-10 新增）

-- ✅ 完整链路：
--   health_trees（history fact）
--   → current_health_trees（latest snapshot per channel）
--   → channel_health_aggregation（§3.9 SoT）
--   → effective_channel_status_policy（precedence + rules）
--   → channel_health_view（CASE 真正执行 Policy）

-- ===== Device Lock =====
device_locks, device_health_history

-- ===== Event / Audit / Alert =====
events, audit_logs, alert_rules, alerts

-- ===== API / Webhook =====
api_keys, webhooks, webhook_deliveries

-- ===== Profile =====
encoding_profiles
output_profiles
audio_profiles
graphic_profiles
qc_profiles
rights_profiles
edge_policy_profiles
```

> **🔴 V0.2.4 时序数据存储原则**（与决策 #22 一致）：
>
> - **PostgreSQL** 存：Current State、Incident Snapshot、Preflight Result、Configuration、Audit
> - **Prometheus** 存：连续遥测（latency / av_offset / av_drift / fps / bitrate / CPU / temperature / packet loss / health）
>
> 以下表**只保存 sampled snapshot 或 incident-related 记录**，不存储每秒数据：
>
> - `latency_probes` → 按 event / incident 保存，非连续
> - `avsync_measurements` → 按告警或定期采样（如每 5 分钟）保存
> - `device_health_history` → 状态变化记录，非时间序列
>
> 连续指标（每 1-2 秒）一律通过 Prometheus 采集，不写入 PG。

> **🔴 V0.2.4 Cleanup-2 + Errata-9 实现顺序提示**（非架构问题，是迁移陷阱）：
>
> `graph_runtimes` / `media_session_runtime` 通过 FK 引用前置表。**migration 必须按以下顺序执行**（或后置 FK migration）：
>
> 1. `config_revisions`
> 2. `change_sets`
> 3. `change_set_items`
> 4. `graph_specs`
> 5. `graph_revisions`
> 6. `graph_runtimes`
> 7. `media_sessions`
> 8. `media_session_runtime`

---

<a id="第-6-部分关键技术决策30"></a>

## 第 6 部分：关键技术决策（V0.2 第 3 轮，30+ 项）

| # | 决策 | 选择 | 理由 |
|---|---|---|---|
| 1 | 名字 | VBMF | 锁定 |
| 2 | 流服务器 | SRS | 锁定（SRS **Stable Line**，当前验证基线 **6.x**；架构通过 SRSAdapter 解耦具体版本，**不绑死某个未来版本号**） |
| 3 | Agent 语言 | Rust | 锁定 |
| 4 | Composition 实现 | **设计/预览 = 浏览器 Canvas，On-Air = 媒体栈渲染** | 锁定 |
| 5 | Subtitle | FFmpeg SRT/ASS 烧录 | 锁定 |
| 6 | 多租户 | 单租户起步 | 锁定 |
| 7 | 部署 | Host 媒体 + Docker 业务 | 锁定 |
| 8 | 浏览器 | 现代 only | 锁定 |
| 9 | 版权 | Rights Engine 必备 | 锁定 |
| 10 | PTP / Genlock | V0.2 抽象 | 锁定 |
| 11 | NDI / RIST / Zixi | V0.2 抽象 | 锁定 |
| 12 | SDI Master Output | **Architecture Contract: RESERVED / V0.2 Implementation: DISABLED / Target: V0.4** | 锁定（V0.2.4 Errata-3） |
| 13 | 双机热备 | V0.2 不做 | 锁定 |
| 14 | 边缘节点 | V0.2 单 Agent | 锁定 |
| 15 | UI 风格 | Dark Mode First | 锁定 |
| 16 | UI 工作域 | Broadcast / Engineering 双域 | 锁定 |
| 17 | Data Plane | **⭐ 7 种 (COMPRESSED_VIDEO/AUDIO, RAW_VIDEO/AUDIO, MULTIPLEXED, METADATA, EVENT)** | 锁定 |
| 18 | Clock Domain | 显式声明 + 跨域必转 | 锁定 |
| 19 | Latency Budget | 每边 + 每 channel | 锁定 |
| 20 | Backpressure Policy | 每边显式 | 锁定 |
| 21 | Graph Spec / Runtime | 物理分表 | 锁定 |
| 22 | 时序数据 | PG 状态 + Prometheus 时序 | 锁定 |
| 23 | fluent-ffmpeg | 不用 | 锁定 |
| 24 | ⭐ SDI 媒体表示 | **RAW_VIDEO + RAW_AUDIO**（**不是 COMPRESSED**） | 锁定（V0.2 关键修正） |
| 25 | ⭐ DECODED | **不是 Data Plane**，是处理过程 | 锁定 |
| 26 | ⭐ Normalize 拆分 | Stream / Video / Audio / Encode 四能力 | 锁定 |
| 27 | ⭐ Switch Mode | **3 种 (PACKET/FRAME/MASTER)** | 锁定 |
| 28 | ⭐ Hot-Standby | **3 级 (COLD/WARM/HOT)** | 锁定 |
| 29 | ⭐ Program Master | **Video + Audio + Metadata 三个独立 graph** | 锁定 |
| 30 | ⭐ SRS 定位 | **Stream Gateway Adapter**（不是 Output Engine 全部） | 锁定 |
| 31 | ⭐ Graph Compiler | **X1 横切能力**，自动插入缺失节点 | 锁定 |
| 32 | ⭐ Preflight | **X2 横切能力**（变更前静态检查） | 锁定 |
| 33 | ⭐ Configuration Versioning | **X3 横切能力**，可回滚 | 锁定 |
| 34 | ⭐ Incident Timeline | **X4 横切能力**，自动串接 | 锁定 |
| 35 | ⭐ Health Tree | **X5 横切能力**，分层健康 | 锁定 |
| 36 | ⭐ Capability Registry | **X6 横切能力**（Signal Contract + Player Matrix） | 锁定 |
| 37 | ⭐ AVSync Manager | **横切管理器**（不是普通节点） | 锁定 |
| 38 | ⭐ Latency Probes | **7 Core Stage + 2 Client E2E + 1 Optional CDN**（capture/decode/switch/compose/encode/mux/publish + source_reference/player + cdn） | 锁定 |
| 39 | ⭐ Player Capability | **Output Variant → Browser Adapter** | 锁定 |
| 40 | ⭐ Engine 总数 | **12（不再加）** | 锁定 |
| 41 | ⭐ Hardware Capability Discovery | **Media Agent 启动时探测真实 GPU / BMD / NVMe / NIC**；Resource Vector 与 Preflight 以探测值为准 | 锁定（V0.2.4 Cleanup-2） |
| 42 | ⭐ 三轴状态机 | **Lifecycle（STOPPED/STARTING/RUNNING/STOPPING）+ Readiness（NOT_READY/READY_TO_TAKE）+ Health（HEALTHY/DEGRADED/FAILED/UNKNOWN）** 分离 | 锁定（V0.2.4 Cleanup-2） |
| 43 | ⭐ Data Plane 唯一定义 | **§3.1 是唯一定义规范**；§1.13 等章节只引用 + 解释，不重复定义；TypeScript 用两维 `{ layer, type }` | 锁定（V0.2.4 Cleanup-2） |
| 44 | ⭐ Resource Model 正式术语 | **9-dim Quantitative Resource Vector + Device Token + Device Constraint**（9 维数值向量 + 端口 token + 排他约束）| 锁定（V0.2.4 Errata-2，V0.2.4 Errata-3 拆分 token/constraint）|
| 45 | ⭐ Architecture vs Snapshot 分离 | **Architecture Baseline ≠ 当前服务器硬件快照**；`current_host_snapshot` 是部署参考，机器增 GPU 不修改 V0.2 | 锁定（V0.2.4 Errata-2）|
| 46 | ⭐ PCIe bandwidth 语义 | **`pcie_*_mb_s` 是 SCHEDULING ESTIMATE（基于 media payload），≠ 实测 PCIe bus utilization**；实测由 hardware telemetry 校准 | 锁定（V0.2.4 Errata-2）|
| 47 | ⭐ hot_standby_levels Schema | **删 `resource_factor` 字段**（语义属于运行时 Graph 计算，不属 lookup）；runtime benchmark 进独立 `failover_benchmarks` 表 | 锁定（V0.2.4 Errata-2 + Errata-3）|
| 48 | ⭐ Program-scope Master 压缩域边界 | **Program-scope Master = RAW 域；Encode = delivery boundary**；禁止 Master 实现为 H.264/AAC | 锁定（V0.2.4 Errata-3）|
| 49 | ⭐ SDI Master Output 实现关闭 | **Architecture Contract: RESERVED / V0.2 Implementation: DISABLED / Target: V0.4** | 锁定（V0.2.4 Errata-3）|
| 50 | ⭐ Hot-Standby 实测范围 | **禁止**"实测 50-200ms"或"target × N"等固定范围；只用 `target_failover_time_ms` + `failover_benchmarks` | 锁定（V0.2.4 Errata-3）|
| 51 | ⭐ Switch Mode 决策 Single Source of Truth | **PACKET_SWITCH eligibility 仅在 §3.4 一处定义**；§8.2 只能引用，不复制判断逻辑 | 锁定（V0.2.4 Errata-3）|
| 52 | ⭐ §3.13 识别 vs §8.9 决策分离 | **§3.13 = 识别层（只分类）；§8.9 = 决策层（决定恢复动作）**；§3.13 禁止指定 action | 锁定（V0.2.4 Errata-6）|
| 53 | ⭐ REJECT ≠ SwitchMode | **`SwitchMode` = PACKET/FRAME/MASTER；`SwitchDecisionResult` = PACKET/FRAME/MASTER/REJECT**；REJECT 是 Decision Outcome 不是 SwitchMode | 锁定（V0.2.4 Errata-6）|
| 54 | ⭐ media_session_runtime 表 | **承载 EFFECTIVE_RUNTIME_MODE + 三轴状态**（lifecycle/readiness/health/effective_switch_mode/runtime_alignment_state） | 锁定（V0.2.4 Errata-9）|
| 55 | ⭐ OperationalFailureDomain vs DiagnosticFailureClass | **`OperationalFailureDomain` = 7（SOURCE/PIPELINE/MASTER/OUTPUT/RECORDING/CLOCK/RESOURCE）；`DiagnosticFailureClass` = PLAYER/UNKNOWN**；PLAYER/UNKNOWN 不进 7 OperationalFailureDomain | 锁定（V0.2.4 Errata-9）|
| 56 | ⭐ AVSync Manager 角色扩展 | **Measurement + Offset/Drift Correction + Failure Classification**；**不**是"identification only"；Recovery Action 仍由 §8.9 决定 | 锁定（V0.2.4 Errata-9）|
| 57 | ⭐ WARN ≠ PASS（switch_mode eligibility） | **PACKET_SWITCH 要求 Mandatory Capability Contract `PASS`；`WARN` 和 `FAIL` 都不满足 PACKET eligibility**；Common RAW Contract Resolution 规则锁死 | 锁定（V0.2.4 Errata-9）|

---

<a id="第-7-部分媒体能力矩阵qc-完整清单"></a>

## 第 7 部分：媒体能力矩阵（QC 完整清单，V0.2 第 3 轮更新）

> V0.2 关键新增：Latency Probes、Drift、Health Tree、Edge Policy、Clock Domain 转换。

| 类别 | 检测项 | 阈值（默认） | 状态 | 引擎 |
|---|---|---|---|---|
| 视频 | 信号丢失 | > 1s | 🔴 P0 | Source QC |
| 视频 | 黑场 | 平均亮度 < 16/255, 持续 3s | 🔴 P0 | Source QC |
| 视频 | 冻结 | 帧差 < threshold, 持续 3s/8s | 🔴 P0 | Source QC |
| 视频 | FPS 异常 | ±5% 偏离 | 🔴 P0 | Source QC |
| 视频 | 分辨率变化 | 任意变化 | 🟠 P1 | Source QC |
| 视频 | 帧重复 | dup > 1/30s | 🟠 P1 | Source QC |
| 视频 | 丢帧 | drop > 1/30s | 🟠 P1 | Source QC |
| 视频 | PTS 异常 | discontinuity | 🟠 P1 | Source QC |
| 视频 | 解码错误 | 任何 | 🟠 P1 | Source QC |
| 视频 | 色彩空间 | 与 profile 不符 | 🟠 P1 | Normalize |
| 音频 | 信号丢失 | > 1s | 🔴 P0 | Source QC |
| 音频 | 静音 | level < -60 dBFS, 持续 5s/15s | 🔴 P0 | Source QC |
| 音频 | 削波 | true peak > -1 dBTP | 🟠 P1 | Source QC |
| 音频 | L/R 失衡 | 电平差 > 20dB | 🟠 P1 | Source QC |
| 音频 | 相位反转 | phase correlation < -0.7 | 🟠 P1 | Source QC |
| 音频 | 响度 | LUFS 偏离 profile ±2 | 🟠 P1 | Audio |
| 音频 | 采样率/位深 | 与 profile 不符 | 🟠 P1 | Audio |
| **AV Sync** ⭐ | offset | \|Δ\| > 40/100/250ms | 🔴 P0 | AVSync Manager |
| **AV Sync** ⭐ | **drift** | 每分钟漂移 > 5ms | 🟠 P1 | AVSync Manager |
| **Latency** ⭐ | 边实际延迟 > 预算 | +20% | 🟠 P1 | QC |
| **Latency** ⭐ | Channel 端到端 > target | +30% | 🟠 P1 | QC |
| **Latency** ⭐ | Probe 缺失/丢失 | > 5s 没数据 | 🟠 P1 | QC |
| **Edge policy** ⭐ | 静默丢帧（NEVER_SILENT 边） | 任何 | 🔴 P0 | QC |
| **Edge policy** ⭐ | 溢出超过 max_queue | 任何 | 🟠 P1 | QC |
| **Clock** ⭐ | 跨 Clock Domain 节点未声明转换 | 任何 | 🟠 P1 | Normalize |
| **Clock** ⭐ | Clock Drift | > 1 ppm | 🟠 P1 | Source QC |
| **Health** ⭐ | 子系统 degraded | 任何 | 🟠 P1 | Health Tree |
| **Health** ⭐ | 子系统 failed | 任何 | 🔴 P0 | Health Tree |
| 网络 | 码率 | 偏离 profile | 🟠 P1 | Source QC |
| 网络 | 丢包 | SRT > 0.5% | 🟠 P1 | Source QC |
| 网络 | jitter | > 50ms | 🟠 P1 | Source QC |
| 网络 | RTT | > 500ms | 🟠 P1 | Source QC |
| 输出 | HLS 切片 | segment duration 偏离 | 🟠 P1 | Output QC |
| 输出 | HLS 清单 | playlist 滞后 > 3s | 🟠 P1 | Output QC |
| 输出 | WebRTC | ICE 失败 / 频繁重连 | 🟠 P1 | Output QC |
| 系统 | CPU | > 80% sustained | 🟠 P1 | Watchdog |
| 系统 | 内存 | > 90% | 🟠 P1 | Watchdog |
| 系统 | 磁盘 | > 90% | 🟠 P1 | Watchdog |
| BMD | Signal | unlocked | 🔴 P0 | Device |
| BMD | Format | unexpected | 🟠 P1 | Device |
| BMD | Temperature | > 80°C | 🟠 P1 | Device |

---

## 第 8 部分：状态机 / Switch Mode / Hot-Standby / 故障切换

### 8.1 Session 状态机（同 V0.2 第 2 轮）

### 8.2 Switch Mode 决策（V0.2 第 3 轮新增，V0.2.4 Cleanup-2 + Errata-3/4 修订）

> **🔴 V0.2.4 Errata-3 + Errata-4 关键修正**：
> 1. **PACKET_SWITCH 的 Canonical Eligibility Algorithm 仅在 §3.4 `packet_switch_eligibility` + `switch_mode_decision_tree` 一处定义**。
> 2. 本节（§8.2）只描述决策流程，**不复制**判断逻辑，也不写 target 数值。
> 3. 所有具体字段、能力、运行态对齐检查均**引用 §3.4**。
> 4. **目标延迟唯一来源** = `channel_routes` 关联的 `hot_standby_levels.target_failover_time_ms` + profile，不是文字。

```
源变更 / 故障触发
  ↓
按 §3.4 `switch_mode_decision_tree` 评估（PACKET → FRAME → MASTER → REJECT 降级链）
  ↓
  ├─ PACKET_SWITCH
  │  → 使用 route / hot-standby profile 定义的 target_failover_time_ms
  │  → 实测由 failover_benchmarks（§5）记录
  │
  ├─ FRAME_SWITCH
  │  → 使用对应 profile target_failover_time_ms
  │  → 实测由 failover_benchmarks 记录
  │
  ├─ MASTER_SWITCH
  │  → 使用对应 profile target_failover_time_ms
  │  → 实测由 failover_benchmarks 记录
  │
  └─ REJECT
     → 拒绝此次配置变更并告警
```

> **全文规则（V0.2.4 Errata-3 + Errata-4 锁定）**：
> 1. **禁止**"切换延迟 = X ms"绝对句式。必须 `target + failover_benchmarks` 双标注。
> 2. **禁止**在本节（§8.2）复制 §3.4 的判断字段。所有 eligibility 检查字段均**只引用 §3.4**。
> 3. **禁止**在本节（§8.2）写 target 数值（如 `<100ms` / `0.5-2s` / `1-3s`），target 唯一来源是 profile/lookup 表。

### 8.3 Hot-Standby Level Semantics（V0.2 第 3 轮新增，V0.2.4 Errata-5 + Errata-7 + Errata-8 修订）

> **🔴 V0.2.4 Errata-7 + Errata-8 关键修正**：
> - 本节描述 **Hot-Standby Level Semantics**（**不是** State Machine，**不是** Runtime Progression）
> - HotStandbyLevel = **Policy / Target**（配置意图）
> - 真实 Runtime 状态 = **§8.11 三轴状态机**（Lifecycle / Readiness / Health）
> - **禁止**把 COLD / WARM / HOT 当成 Runtime State；**禁止**用 `match standby_level { COLD => WARM, ... }` 模式
> - 真实故障 / 启动 / 接管全部由 §8.11 三轴状态机表达

**Hot-Standby Level Semantics**（3 个并列 Policy / Target，无相互迁移）：

```
              Configuration
                    │
            hot_standby_level
                    │
       ┌────────────┼────────────┐
       ▼            ▼            ▼
     COLD         WARM          HOT
   ──────────   ──────────   ──────────
   Policy:     Policy:       Policy:
   COLD        WARM          HOT
   target:     target:       target:
   30000       1500          100
   ──────────   ──────────   ──────────
       │            │            │
       └────────────┼────────────┘
                    │
                    ▼
          要求 Runtime 最终满足
          READY_TO_TAKE
          （§8.11 三轴状态）
                    │
                    ▼
         §8.11 Runtime State
       Lifecycle / Readiness / Health
```

> **关键澄清（V0.2.4 Errata-7 + Errata-8 锁定）**：
> 1. **COLD / WARM / HOT 不是 3 个 runtime state**；是 3 个**并列**的 Policy / Target。
> 2. `WARM ≠ STARTING`。一个 WARM Standby 完全可能 `lifecycle = RUNNING, readiness = NOT_READY, health = HEALTHY`（已运行但还不能接管）。
> 3. `READY_TO_TAKE` 是 §8.11 Readiness 维度的取值，**不是** HotStandbyLevel 的 state 字段。
> 4. 真正实现时**禁止**写 `if level == WARM && started { level = HOT }` 这种状态机代码。

**`READY_TO_TAKE` predicate**（V0.2.4 Errata-5 锁死，**唯一来源** = §3.5 + §8.11）：

```yaml
ready_to_take:
  # §3.5 HOT readiness requirements
  input_locked: true
  video_healthy: true            # 黑场/冻结/丢帧 = false
  audio_healthy: true            # 静音/削波 = false
  encoder_running: true
  gop_pts_dts_stable: true
  audio_clock_stable: true
  source_qc_healthy: true
  output_ready: true             # 在 SRS 缓存内
  backup_takeover_marked: true
  # §8.11 三轴状态
  lifecycle: RUNNING
  readiness: READY_TO_TAKE
  health: HEALTHY
```

> **实现规则**（V0.2.4 Errata-5 锁定）：HOT 判定**必须**评估完整 `ready_to_take` predicate，**禁止**用 `if encoder_stable && pipe_full { standby = HOT }` 这种简化谓词。`§3.5` + `§8.11` 是唯一定义。

### 8.4 故障切换策略（同 V0.2 第 2 轮 + Switch Mode 集成，V0.2.4 Errata-5 修正）

> **🔴 V0.2.4 Errata-5 修正**：第一行 "主源异常 > failback_hysteresis_ms" 是**逻辑错误**——应改为 `failover_hysteresis_ms`（判断主源故障并切备）。`failback_hysteresis_ms` 是判断主源恢复并回切。

```
主源异常 > failover_hysteresis_ms                -- V0.2.4 Errata-5 修正
  ↓
按 §3.4 switch_mode_decision_tree
结合当前 Runtime Alignment
  ↓
得到 EFFECTIVE_RUNTIME_MODE
  ↓
更新 media_session_runtime.effective_switch_mode
  ↓
执行对应切换（PACKET / FRAME / MASTER）
  ↓
切到备源后，至少保持 min_hold_ms
  ↓
主源恢复 > failback_hysteresis_ms
  ↓
回切
```

> **字段语义**（V0.2.4 Errata-5 + Errata-7 锁定）：
> - `failover_hysteresis_ms` = 主源异常持续超过此阈值 → 触发 Decision Tree
> - `failback_hysteresis_ms` = 主源恢复持续超过此阈值 → 回切主源
> - `min_hold_ms` = 切到备源后**至少保持**此时间（防抖动）
> - **`channel_routes.switch_mode` = COMPILED_MODE / Configuration Intent**，**不**作为 Runtime 恢复的直接执行依据
> - **`media_session_runtime.effective_switch_mode` = EFFECTIVE_RUNTIME_MODE**，由 Decision Tree + 当前 Runtime Alignment 计算，是 Runtime 实际执行模式

### 8.5 FFmpeg 进程崩溃恢复（同 V0.2 第 2 轮）

### 8.6 Safety Engine（同 V0.2 第 2 轮）

### 8.7 Backpressure 异常处理（同 V0.2 第 2 轮）

### 8.8 Clock Domain 异常（同 V0.2 第 2 轮）

### 8.9 Failure Domain Matrix（V0.2.4 patch 2 新增）⭐

> 🔴 **V0.2.4 解决"Output 故障被错误地切源"问题**。Fault Injection 决策链必须按故障域处理。

| 故障域 | 典型故障 | 自动动作 | 是否切源 | 是否垫片 |
|---|---|---|---|---|
| **Source** | SDI 无信号 / 冻结 | Backup / Filler | ✅ | 备源失败后 |
| **Pipeline** | FFmpeg 崩溃 / 节点异常 | Restart node / Backup node | 视 Session | 必要时 |
| **Master** | Program Master 失败 | Filler / Emergency | ✅ | ✅ |
| **Output** | HLS 切片失败 / CDN 故障 | Restart adapter / alternate destination | ❌ | ❌ |
| **Recording** | 录制失败 | Backup disk / restart | ❌ | ❌ |
| **Clock** | PTP 丢失 | Fallback clock（CLOCK_DEGRADED）| ❌ | ❌ |
| **Resource** | CPU/内存/磁盘耗尽 | Degrade background jobs | ❌ | ❌ |

> **关键规则**：
> - 节目源没故障时，**绝不能因为 Output 故障而切源**
> - HLS output failure → Output Health=FAILED → restart HLS branch / SRS adapter → retry → fallback Output Variant → alert
> - 此表由 **Safety + Watchdog + Health Tree** 共同执行，**不新增 Engine**

**反例（被禁止）**：
```
Program Master = HEALTHY
SRS = HEALTHY
HLS = FAILED
RTMP = HEALTHY
   ↓ (❌ 错误)
切 Primary → Filler   ← 节目源没故障，不应切源
```

**正确决策**：
```
HLS output failure
   ↓
Output Health = FAILED（其他域 HEALTHY）
   ↓
Restart HLS branch / SRS adapter
   ↓
retry
   ↓
fallback Output Variant
   ↓
alert
```

### 8.10 AV Sync 异常（V0.2 第 3 轮新增，V0.2.4 Cleanup-2 修订）

> **🔴 V0.2.4 Cleanup-2 修正**：`av_offset > 250ms → 切 backup` **禁止**作为绝对规则。AV Sync 异常必须先 **Failure Domain Classification**（见 §8.9），再决定恢复动作。**PLAYER 缓存异常绝不能导致切主备节目源**。

```yaml
av_sync_decision:
  yellow_offset:           # |Δ| > 100ms
    action: compensate + monitor

  red_offset:              # |Δ| > 250ms
    action: classify_failure_domain
    classify:
      SOURCE:    { action: FAILOVER, source: §8.9 }              # 主源 PTS 不稳
      PIPELINE:  { action: RESTART, source: §8.9 }               # 编码/处理慢
      OUTPUT:    { action: OUTPUT_RECOVERY, source: §8.9 }       # mux/sink 错
      PLAYER:    { action: NOTIFY, fail_safe: true, source: §8.9, note: "PLAYER 是 DiagnosticFailureClass，不进 7 OperationalFailureDomain" }  # 浏览器缓存异常，**绝不切源**
      UNKNOWN:   { action: SAFE_DEGRADE, alert: true, source: §8.9, note: "UNKNOWN 是 DiagnosticFailureClass，等待进一步分类" }
    then:
      - 100ms ≤ |Δ| < 250ms → compensate
      - |Δ| ≥ 250ms + OperationalFailureDomain.SOURCE → FAILOVER
      - |Δ| ≥ 250ms + OperationalFailureDomain.PIPELINE → RESTART
      - |Δ| ≥ 250ms + OperationalFailureDomain.OUTPUT → OUTPUT RECOVERY
      - |Δ| ≥ 250ms + DiagnosticFailureClass.PLAYER → NOTIFY（关键：不切源）
      - |Δ| ≥ 250ms + DiagnosticFailureClass.UNKNOWN → SAFE DEGRADE + ALERT
```

> 旧版 `av_offset > 250ms → CRITICAL + 切 backup` **已删除**。这是与 §3.13 + §8.9 Failure Domain 的统一。

### 8.11 三轴状态机（V0.2.4 Cleanup-2 新增）⭐

> **🔴 V0.2.4 Cleanup-2 修正**：之前 `RUNNING / READY_TO_TAKE / HEALTHY / DEGRADED / FAILED` 混合在同一维度。**三轴分离**：

```yaml
runtime_state:
  lifecycle:              # 进程生命周期
    STOPPED
    STARTING
    RUNNING
    STOPPING

  readiness:              # 是否能接管
    NOT_READY
    READY_TO_TAKE

  health:                 # 当前健康度
    HEALTHY
    DEGRADED
    FAILED
    UNKNOWN
```

**典型组合**：

| 角色 | lifecycle | readiness | health | 含义 |
|---|---|---|---|---|
| Primary 健康 | RUNNING | READY_TO_TAKE | HEALTHY | 正常播出 |
| Primary 故障中 | RUNNING | NOT_READY | DEGRADED | 异常，降级 |
| Backup 完全就绪 | RUNNING | READY_TO_TAKE | HEALTHY | 真正 Hot Standby |
| Backup 启动中 | STARTING | NOT_READY | UNKNOWN | 还没到接管条件 |
| Backup 编码不稳 | RUNNING | NOT_READY | DEGRADED | 不应触发接管 |
| 已停止 | STOPPED | NOT_READY | UNKNOWN | Cold Standby |

**对外 Channel Status = `channel_health_view.effective_channel_status`**（V0.2.4 Errata-13 修正）：

```ts
// V0.2.4 Errata-13 修正：channel_routes.status 字段废除
// 对外 Channel Status 唯一入口 = channel_health_view.effective_channel_status
// Runtime State = media_session_runtime.{lifecycle, readiness, health}（纯下钻信息，不参与 effective_channel_status 判定）
// Health Tree Aggregation (§3.9) 才是 Channel Health 的 SoT

function isHotStandbyReady(state: RuntimeState): boolean {
  return state.lifecycle === "RUNNING"
      && state.readiness === "READY_TO_TAKE"
      && state.health === "HEALTHY";
}
```

> **关键**：Media Agent / Controller / UI 状态逻辑基于三轴表达。Health Tree 聚合用 **§3.9 Health Tree Aggregation Policy**（不是 §8.9，§8.9 是 Failure Domain Matrix）。**Channel 对外 status 唯一来源 = `channel_health_view.effective_channel_status`（SoT = §3.9 Health Tree Aggregation Policy + §5 effective_channel_status_policy）**。

---

## 第 9 部分：部署拓扑（Host + Docker 边界）

（同 V0.2 第 2 轮不变）

---

## 第 10 部分：Operator UX / UI 架构

> V0.2 第 3 轮新增：**Signal Graph Designer**（在 Engineering 域）。
> V0.2 第 3 轮新增：**Health Tree** 视图。

（同 V0.2 第 2 轮 7 个核心页面 + 新增 2 个 = 9 个核心页面）

#### ⑧ Signal Graph Designer（NEW, Engineering 域）

拖拽式设计器 + Auto Layout + 实时编译预览：
- 节点类型 Source / Process / Output
- 边声明 Data Plane / Clock Domain / Edge Policy
- 编译预览（X1 Graph Compiler）
- Preflight 报告（X2）
- 模拟 Dry Run
- Apply with Change Set

#### ⑨ Health Tree（NEW, 两个域共用）

树形健康视图（X5）：
- 通道 → 子系统 → 节点
- 颜色状态 + 异常下钻
- 历史回放
- 关联到 Incident Timeline

#### 修改：原"⑦ Recording / Replay"

- 加上 "事件回溯" 链接到 Incident Timeline

#### 修改：原"⑥ QC / Alerts"

- 加上 "Health Tree" 子面板
- 加上 "AV Sync Drift" 趋势

### 10.1-10.10 视觉/危险操作/状态分层（同 V0.2 第 2 轮不变）

### 10.11 4 条关键操作链（V0.2 第 3 轮必出原型）

**链 1：播出（Operator）**

```
打开 Dashboard → 看 PVW/PGM → 选 Channel → 看 NOW/NEXT → 
按 TAKE → 切到 PGM → Master → Output → 浏览器播放
```

**链 2：故障（Operator + System）**

```
SDI 冻结 → QC 检测 → ALERT → 决策（PACKET/FRAME/MASTER）→ 
Auto Failover → 切 backup → Filler 兜底 → Operator 收到通知 → 
Incident 自动建档 → 录像回溯
```

**链 3：节目单（Director）**

```
打开 Timeline → 拖入 Asset → Preflight (loudness / rights / duration) → 
Save Draft → Validate → Schedule Apply → 
On-Air 时自动到点 → 切 → 复位
```

**链 4：工程（Engineer）**

```
打开 Graph Designer → 拖节点连边 → Save Spec → 
Compile (X1) → Resource Plan → Preflight (X2) → 
Apply with Change Set (X3) → Runtime 上线 → Health Tree (X5) → 
QC 持续监测 → 异常 → Incident (X4)
```

> **只要这 4 条在原型里走通，V0.2 架构才算经过"操作员级"验证。**

---

## 第 11 部分：实现路线图（重排）

> V0.2 第 3 轮新增 Phase 0.6（Graph Compiler/Preflight）、Phase 5.5（Health Tree）。

### Phase 0 — Architecture Freeze ✅
### Phase 0.5 — Operator Workflow & Low-Fi Wireframe
### Phase 0.6 — Reference Implementation + Fault Injection ⭐ V0.2.4 patch 2
- [ ] **Reference A1（PACKET_SWITCH 基础能力）**：
  输入：**预对齐压缩源 A / B**（同 codec/container/时间戳）
  验证：Capability Contract、GOP/IDR、PTS/DTS、timebase、SPS/PPS、audio continuity、mux continuity
  输出：确认可在压缩流层无缝切换
- [ ] **Reference A2（真实 SDI 主备）**：
  ```
  SDI-A ─→ Normalize ─→ Encode ─┐
                                ├→ FRAME_SWITCH / MASTER_SWITCH → SRS → HLS
  SDI-B ─→ Normalize ─→ Encode ─┘
  ```
  说明：🔴 **独立 FFmpeg 编码器天然不具备 packet switch 所需的精确对齐**，真实 SDI 主备默认走 FRAME/MASTER 切换，**除非已做外部对齐**。
- [ ] **Reference B（异构源 + 图文 + 多 Master）**：
  ```
  SDI ─┐
      ├─ Normalize ─→ MASTER_SWITCH ─┐
  SRT ─┘                              ├─ Program Master
                                     │   (Video / Audio / Metadata)
  Composition ──────────────────────┤
  Audio Mixer / Loudness / Delay ───┘
                                     │
                                     ▼
                                    SRS
  ```
  覆盖：RAW + COMPRESSED、MASTER_SWITCH、Composition、Audio、Program Master 三独立 graph
- [ ] **Fault Injection Test（必跑 5 种故障，按 Failure Domain 决定恢复动作）**：
  - SDI 冻结 5s
  - 音频静音 8s
  - Primary FFmpeg 进程崩溃
  - Clock Drift（注入 +5ms/min）
  - HLS 输出切片失败
- [ ] **按 Failure Domain 验证自动恢复**（见 §8.9 Failure Domain Matrix）：
  - **Source 故障** → Failover
  - **Pipeline 故障** → Restart node
  - **Master 故障** → Filler / Emergency
  - **Output 故障** → Restart adapter / alternate destination
  - **Recording 故障** → Backup disk / restart
  - **Clock 故障** → Fallback clock
  - **Resource 故障** → Degrade background jobs
- [ ] 观察链路：QC → Health Tree → Watchdog → Safety → Recovery；各域恢复动作**不混淆**
- [ ] 整套跑通 → **V0.2 Architecture Acceptance**（不再改架构）

> 🔴 V0.2.3 patch 修正：原 `SDI → PACKET_SWITCH → HLS` 在 Data Plane 定义下不成立（SDI 是 RAW，PACKET_SWITCH 是 COMPRESSED 域，中间缺 Encode）。现在改为两个完整 Reference。

### Phase 1 — Media Core（首要，24h 稳定）
- [ ] Media Agent v0（Rust + JSON-RPC）
- [ ] Session Manager（Data Plane 标注 + Switch Mode 字段 + Hot-Standby Level）
- [ ] FFmpeg Command Builder（按 Data Plane 标签组装）
- [ ] FFmpeg `-progress pipe:1` 解析
- [ ] BMD 设备 Registry
- [ ] Clock Domain 检测
- [ ] Edge Policy 引擎
- [ ] Latency Probes（**7 Core Stage + 2 Client E2E + 1 Optional CDN**，见 §3.6）
- [ ] AVSync Manager（measure / compensate / drift）
- [ ] Switcher 3 种模式
- [ ] Hot-Standby 3 级
- [ ] Local NVMe Recording（chunked, 5 min/段）
- [ ] SRS 单实例
- [ ] 端到端：`SDI → ffmpeg → SRS → HLS`，24h 不掉

### Phase 2 — Backend Foundation
- [ ] Fastify + Drizzle + Zod
- [ ] PostgreSQL schema V0.2 第 3 轮
- [ ] Valkey + Event Bus
- [ ] BullMQ + 转码 worker
- [ ] Media Controller
- [ ] GraphSpec / GraphRuntime 数据模型

### Phase 2.5 — Graph Compiler / Preflight ⭐ 新增
- [ ] X1 Graph Compiler（Validator / Insert Missing Nodes / Clock Align / Latency Estimate / Resource Plan / Emit Runtime）
- [ ] X2 Preflight（Graph / Playout / Channel 三类）
- [ ] X3 Configuration Versioning（Draft / Validate / Preview / Apply / Rollback）
- [ ] X4 Incident Timeline（自动串接）
- [ ] X5 Health Tree（分层健康视图）
- [ ] X6 Capability Registry（Signal Contract + Player Matrix）

### Phase 3 — Auth & RBAC

### Phase 3.5 — UI Prototype & Backend Validation

### Phase 4 — Web 控制台
- [ ] 9 个核心页面（包含 Graph Designer + Health Tree）
- [ ] 4 条关键操作链验证

### Phase 5 — Signal Fabric
### Phase 5.5 — Health Tree & Incident Timeline UI ⭐ 新增

### Phase 6 — Playout Engine

### Phase 7 — 切换 / 冗余（PACKET/FRAME/MASTER + COLD/WARM/HOT）

### Phase 8 — 录制 / 归档

### Phase 9 — QC Engine

### Phase 10 — 图文包装

### Phase 11 — 资源调度

### Phase 12 — 高级播控

### Phase 13 — 可观测

### Phase 14 — HA / 灾备

### Phase 15 — 多节点

### Phase 16 — 专业级扩展

---

## 第 12 部分：待决策项（V0.2 锁定）

> **V0.2.4 Errata-9 共 57 项架构决策锁定**（V0.2 第 3 轮 40 + Cleanup-2 #41-43 + Errata-2 #44-47 + Errata-3 #48-51 + Errata-6 #52-53 + Errata-9 #54-57：media_session_runtime 表 / OperationalFailureDomain vs DiagnosticFailureClass / AVSync Manager 角色扩展 / WARN ≠ PASS）。

（合并 V0.2 第 2 轮 + 第 3 轮所有决策项）

---

## 第 13 部分：P2 预留

（同 V0.2 第 2 轮，不变）

---

<a id="附录-a术语表"></a>

## 附录 A：术语表（V0.2 第 3 轮更新，V0.2.4 Errata-8 修订）

> 新增 / 修订：Data Plane 4 Layer (ELEMENTARY/CONTAINER/METADATA/CONTROL) / Switch Mode 3 种 / Hot-Standby 3 级 (Policy/Target) / **Latency Probes 10 = 7 Core + 2 Client E2E + 1 Optional CDN** / Program-scope Master / Encode = delivery boundary / **AVSync Manager (Measurement + Offset/Drift Correction + Failure Classification，不决定 Recovery Action)** / Switcher 与 Master Switch 区分 / Video/Audio/Metadata 三独立 graph / SwitchDecisionResult 4 元素（REJECT ≠ SwitchMode） / Health Tree Aggregation Policy (SoT) / Failure Domain Matrix (SoT) / Device Token + Device Constraint / failover_benchmarks (Runtime Measurement) / **COMPILED_MODE vs EFFECTIVE_RUNTIME_MODE** / **OperationalFailureDomain (7) + DiagnosticFailureClass (PLAYER/UNKNOWN)** / media_session_runtime 表 / Common RAW Contract Resolution 规则。

（同 V0.2 第 2 轮 + 补充）

| 术语 | 定义 |
|---|---|
| **Data Plane** | 7 种数据流类型（见 §3.1） |
| **COMPRESSED_VIDEO / AUDIO** | 压缩视音频 |
| **RAW_VIDEO / AUDIO** | 未压缩视音频（SDI 是 RAW！） |
| **MULTIPLEXED** | 复合流（TS/MP4） |
| **METADATA / EVENT** | 控制/异步 |
| **DECODED** | ⚠️ 不是 Data Plane，是处理过程 |
| **PACKET_SWITCH** | **compressed stream boundary**（同 codec/container/profile/timestamp 严格对齐的压缩流层切换） |
| **FRAME_SWITCH** | **raw media frame boundary**（主备都先 decode，RAW 帧层切，再共同 encode） |
| **MASTER_SWITCH** | **synchronized program boundary**（主备都先完整 normalize 到统一 Master 格式，切的是整个 Program Master 边界） |
| **Switch Mode** | PACKET_SWITCH / FRAME_SWITCH / MASTER_SWITCH |
| **Logical Atomic / Transactional Cutover** | 跨组件的业务层原子切换（V0.2.4 命名，替代误导的"Atomic Apply"） |
| **OperationalFailureDomain** | SOURCE / PIPELINE / MASTER / OUTPUT / RECORDING / CLOCK / RESOURCE — 7 类运行期故障域（决定恢复动作） |
| **DiagnosticFailureClass** | PLAYER / UNKNOWN — 2 类诊断分类（**不**进 7 OperationalFailureDomain；只 NOTIFY / SAFE_DEGRADE，不切源） |
| **Failure Domain Matrix** | §8.9，按 OperationalFailureDomain 决定自动恢复动作（切源 / 重启 / 垫片 / OUTPUT_RECOVERY / SAFE_DEGRADE / NOTIFY）；**保留**此名称作为"策略/章节名" |
| ~~FailureDomain~~ | ❌ V0.2.4 Errata-10 + Errata-12 彻底废除 alias；TS / Rust / JSON Schema / PG enum **不再创建**同名 enum；只保留 OperationalFailureDomain + DiagnosticFailureClass |
| **Hot-Standby Level** | COLD / WARM / HOT |
| **Latency Probe** | **7 核心媒体阶段 + 2 Client E2E + 1 可选 CDN**（capture/decode/switch/compose/encode/mux/publish + source_reference/player + cdn） |
| **AVSync Manager** | 横切管理器：**Measurement + Offset/Drift Correction + Failure Classification**；不决定 Recovery Action（由 §8.9 决定）|
| **Program Master** | Video Master + Audio Master + Metadata Master |
| **Video/Audio/Metadata Graph** | 三个独立 graph（不是一条 pipeline 的两分支） |
| **Health Tree** | 通道 → 子系统 → 节点的分层健康视图 |
| **Graph Compiler (X1)** | 自动把 GraphSpec 编译成 Runtime Graph |
| **Preflight (X2)** | 变更前的静态检查（≠ QC） |
| **Configuration Versioning (X3)** | 配置可版本化、可回滚 |
| **Incident Timeline (X4)** | 自动串接告警/切换/录像/操作 |
| **Health Tree (X5)** | 分层健康视图 |
| **Capability Registry (X6)** | Signal Contract + Player Capability Matrix |
| **Stream Gateway Adapter** | SRS 定位（不是 Output Engine 全集） |
| **Direct Output Adapter** | SDI/UDP/RTP/File |
| Source / Signal / Route | 同 V0.1 |
| Channel / Session | 同 V0.1 |
| Master | 同 V0.1 |
| Playout / Timeline / Filler | 同 V0.1 |
| Composition / Loudness | 同 V0.1 |
| Switcher / Take | 同 V0.1（但区分了 Switch Mode） |
| PVW / PGM | 同 V0.1 |
| Hardness | 同 V0.1 |
| Worker / Controller / Agent | 同 V0.1 |
| Watchdog / Incident / Audit | 同 V0.1（Incident 与 Incident Timeline 区分） |
| QC / ASR | 同 V0.1 |
| PTP / Genlock / Timecode | 同 V0.1 |
| Clock Domain | SYSTEM / MONOTONIC / MEDIA / TIMECODE / PTP |
| Latency Budget | 每边 + 每 channel 的目标延迟 |

---

## 附录 B：版本演进

| 版本 | 范围 | 关键里程碑 |
|---|---|---|
| V0.1 | 视频编码 Web 平台 | 选型、Docker compose 骨架、SDI→RTMP |
| V0.2 | **IP Broadcast Media Fabric** | **12 Engines + 6 横切能力 + 22 原则 + Runtime Semantics Freeze** |
| V0.2.3 | Runtime Semantics Patch 1 | 7 硬错误 + 7 强烈建议（Resource Vector / Clock Reference / Preflight 三层 / Atomic Apply / Health Tree Aggregation / AVSync 拆分 / Explainable Compile） |
| **V0.2.4** | **Runtime Semantics Patch 2 + Cleanup-1/2/3 + Errata-1/2/3/4/5/6/7/8/9/10/11/12/13/14** | **... + 5 项 Post-Freeze Schema 收口 Errata-11 + 5 项 Health/Target/Vocabulary 最终收口 Errata-12 + 5 项 Runtime Health Schema Micro-Closure Errata-13 + 4 Schema + 2 同步 Errata-14 (Source RG + UNKNOWN) = Runtime Semantics CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE（最终，22 轮 review，9 大 Runtime 域 CLOSED + 3 Schema + 2 Semantic Cleanup + 7 Health Invariants）** |
| V0.3 | 高级播控 | Playout + 切换 + 录制备 + 字幕 + PTP |
| V0.4 | 广播级 | PTP/Genlock + SDI Master + HA |
| V0.5 | WebRTC + 浏览器上行 | 浏览器推流、互动 |
| V1.0 | 完整 IP 播控 | NDI/RIST/Zixi + 多节点 |

---

## 附录 C：V0.2 一致性审查 + Runtime Semantics Freeze 记录

> **🔴 V0.2.4 Errata-14 重要说明**：
> C.1 - C.20 为**历史审查记录**。其中的 `decisions` / `review_passes` / `patch counts` 表示**当轮历史状态**，不代表当前 Baseline 状态。
> **Current authoritative state 以文档头 + C.26 + Appendix D 为准**：
> - 12 Engine + 5 横向系统 + 6 横切能力 + 22 原则 + **57 决策**
> - **22 轮 review** + V0.2.3 patch 1 / V0.2.4 patch 2 / Cleanup-1/2/3 / Errata-1/2/3/4/5/6/7/8/9/10/11/12/13/14
> - `v0_2_runtime_semantics: CLOSED / implementation_ambiguity: NONE / 9 大 Runtime 域 CLOSED / 3 Schema + 2 Semantic Cleanup / 7 Health Invariants`
>
> grep 历史 `decisions: 43` 等数字时务必以上述为最终权威。

### C.1 第 1 轮（V0.2.1）：功能域冻结
- 12 + 5 Engines
- 16 原则
- 30+ 张表
- Operator UX 架构

### C.2 第 2 轮（V0.2.2）：Operator UX + 一致性审查
- 16 原则 → 22 原则
- 新增 Operator UX 章节
- 6 横向能力占位（X1-X6）

### C.3 第 3 轮（V0.2-final）：Runtime Semantics Freeze
- **🔴 关键修正**：SDI 是 RAW_VIDEO + RAW_AUDIO，不是 COMPRESSED
- **🔴 关键修正**：DECODED 不是 Data Plane，是处理过程
- **🔴 关键修正**：Normalize 是能力，不是固定节点
- **🔴 关键修正**：主备切换 = 3 Switch Mode × 3 Hot-Standby Level
- **🔴 关键修正**：Program Master = Video + Audio + Metadata 三独立 graph
- **🔴 关键修正**：AVSync Manager 是横切管理器，不是普通节点
- **🔴 关键修正**：Latency Probes 初始 7 Core Stage Probe；后续扩展为 **10 probe points = 7 Core + 2 Client E2E + 1 Optional CDN**（见 §3.6）
- **🔴 关键修正**：SRS = Stream Gateway Adapter（不是 Output Engine 全部）
- **新增**：6 横切能力（X1-X6）全部展开定义
- **新增**：Graph Compiler / Preflight / Versioning / Incident Timeline / Health Tree / Capability Registry
- **新增**：9 个核心 UI 页面（+2：Graph Designer + Health Tree）
- **新增**：4 条关键操作链必须经过原型验证

### C.4 跨章节一致性检查

| 检查 | 结果 |
|---|---|
| Engine 列表 vs Engines in Graph | ✅ 一致 |
| Data Plane vs Engine 关系 | ✅ 修正后一致（SDI=RAW） |
| Switch Mode vs Redundancy Engine | ✅ 一致 |
| Hot-Standby Level vs Session | ✅ 一致 |
| Latency Probes vs Latency Budget | ✅ 一致 |
| Program Master 三 graph vs Switcher | ✅ 一致 |
| Graph Compiler vs GraphSpec/Runtime | ✅ 一致 |
| Configuration Versioning vs Change Set | ✅ 一致 |
| Health Tree vs Source/Output QC | ✅ 一致（X5 vs QC 矩阵） |
| UI 9 页面 vs 12 Engines | ✅ 一一对应 |
| 4 条操作链 vs 全部 Engines + 横切能力 | ✅ 全部覆盖 |

### C.5 Phase 1 实证项（V0.2.4 Errata-8 重写，不阻塞冻结）

> **🔴 V0.2.4 Errata-8 重写**：原"自动选择算法 vs 用户显式声明"和"80% resource factor"不是开放架构决策；架构已锁。改写为**实证验证项**。

1. **Switch Mode Decision Tree 行为验证**：验证 Graph Compiler 在真实 Capability Contract / Runtime Alignment 数据上的行为（PACKET/FRAME/MASTER/REJECT 落点）— Phase 1 验证
2. **Data Plane 自动验证**：节点 contract 与边 contract 的严格/宽松匹配 — Phase 1 验证
3. **Hot-Standby Resource Footprint 验证**：验证 Graph Compiler 基于 Primary + Standby Runtime Graph Resource Vector 求和结果（**不引入固定 resource factor**；resource_factor = FORBIDDEN）— Phase 1 验证
4. **AVSync 漂移补偿算法**：固定补偿 vs 实时追踪 — Phase 1 验证
5. **Health Tree 分层粒度**：每节点都列还是按子系统聚合 — UI 验证

### C.6 结论

> **V0.2 Architecture Baseline — Runtime Semantics Freeze Round 完成。**
>
> **本轮把所有"应该"清楚但之前"模糊"的概念全部锁死**：
> - SDI 是 RAW，不是 COMPRESSED
> - DECODED 是过程，不是类型
> - Switch Mode 3 种 × Hot-Standby 3 级 = 9 种组合
> - Program Master 拆三独立 graph
> - AVSync Manager 单独成类
> - Latency Probes 初始 7 Core Stage；扩展为 **10 probe points = 7 Core + 2 Client E2E + 1 Optional CDN**（见 §3.6）
> - 6 横切能力（X1-X6）全部展开
> - 9 个核心 UI 页面 + 4 条关键操作链
>
> **V0.2 现在可以真正冻结了**。Phase 0.5 + 0.6 启动后不需要回头改架构。

---

### C.7 V0.2.3 patch 1（2026-08-24，第 4 轮 review 硬错误修正）

#### 🔴 7 个硬错误

| # | 位置 | 错误 | 修正 |
|---|---|---|---|
| 1 | §11 Phase 0.6 | `SDI → PACKET_SWITCH → HLS`（Data Plane 不兼容） | 改成 2 个完整 Reference（A=实时 SDI 主备、B=异构源）+ Fault Injection |
| 2 | §1.18 + §3.5 | `failover_time_ms: 100`（写死） | 改 `target_failover_time_ms` + `expected_range_ms`（target 是预算不是保证） |
| 3 | §3.4 | PACKET_SWITCH 仅看 codec | 加完整 11+ 项 Capability Contract 检查（video/audio/stream 三大类） |
| 4 | §3.5 | HOT=`pipe 满缓冲` | HOT=**`READY_TO_TAKE`**（8 项状态全部满足） |
| 5 | §1.20 | "一条挂了不影响另一条" | 改"处理层独立隔离 + Master Join 联合判定" |
| 6 | §3.1 | MULTIPLEXED 与 RAW_VIDEO 并列（维度错） | 改二级分类：Elementary / Container / Metadata / Control |
| 7 | §5 graph_runtimes | 无 revision 字段 | 加 `graph_revision_id` / `config_revision_id` / `change_set_id` / `activated_at` |

#### 🟠 5 个强烈建议（接）

| # | 位置 | 建议 | 落点 |
|---|---|---|---|
| 8 | §3.11 (新) | Resource Vector | 新章节 §3.11：9 维资源向量（cpu/gpu/ram/vram/nic/disk/pcie/bmd） |
| 9 | §3.12 (新) | Clock Reference | 新章节 §3.12：channel 必须声明 clock_domain + reference_id + priority + fallback |
| 10 | §1.22 | Preflight 三层 | Static（合法性）+ Resource（资源向量）+ Runtime Readiness（运行就绪） |
| 11 | §1.21 | Atomic Apply | draft→validate→prepare→commit（snapshot+rollback）；禁止半配置 |
| 12 | §3.9 | Health Tree Aggregation Policy | required/optional 节点聚合规则；Source.Primary=FAILED ⇒ Channel=DEGRADED |

#### 🟡 3 个强烈建议（不接 / 留到 Phase 0.5）

| # | 建议 | 决定 |
|---|---|---|
| 13 | Offset/Drift Correction 分离 | ✅ 改成 §3.13 AVSync Manager 拆分（也接了，列为已接） |
| 14 | Explainable Compile | ✅ 改成 §3.10 Graph Compiler 加 explainable_compile_preview（也接了，列为已接） |
| 15 | Operator Intent | ❌ 留到 Phase 0.5 Workflow 文档（与"危险操作分层 / Desired vs Actual"一起） |

> **🟢 不加新 Engine**。12 Engine + 5 横向系统 + 6 横切能力总数不变。

#### C.7 结论

V0.2.3 patch 1 完成所有 7 个硬错误修正 + 7 个强烈建议（5 个核心接 + 2 个一并接 + 1 个留到 Phase 0.5）。
**V0.2.3 现在是真正可写的代码架构**，Phase 0.5 + 0.6 启动后**不需要回头改 §3 / §5 / §11**。

---

### C.8 V0.2.4 patch 2 记录（2026-08-24，第 5 轮 review 9 项语义修正）

#### 🔴 4 个必修

| # | 位置 | 修正 |
|---|---|---|
| 1 | §3.3 | `stream_normalize` 不允许修改 codec/profile/level/resolution/fps/bitrate/GOP；改 3 子能力 REMUX / BITSTREAM_ADAPT / METADATA_REWRITE |
| 2 | §3.7.1（新） | Composition 分 Program + Variant 两级；Program Master 干净，Variant 叠加平台包装 |
| 3 | §11 Phase 0.6 | Reference A 拆 A1（PACKET 基础能力，预对齐源）+ A2（真实 SDI 主备，走 FRAME/MASTER） |
| 4 | §8.10（新） | Failure Domain Matrix；Fault Injection 按 7 故障域决定恢复动作（Output 故障不切源） |

#### 🟠 5 个强烈建议

| # | 位置 | 修正 |
|---|---|---|
| 5 | §3.5 | `expected_range_ms` 写死 → 改 `benchmark`（p50/p95/p99 实测填） |
| 6 | §3.11 | `bmd_channels: 3` → device/port token（Duo 2+2、SDI 1+1、Mini Monitor 0+1） |
| 7 | §3.6 | Latency Probe 加 `e2e_measurement_modes`（STAGE / E2E+SYNC/EMBED/APPROX） |
| 8 | §5 | 时序数据原则重申：latency_probes/avsync_measurements/device_health_history 存 snapshot，不存每秒；连续遥测走 Prometheus |
| 9 | §1.21 | `Atomic Apply` 改 `Logical Atomic / Transactional Cutover`（跨组件无法 DB 事务） |

#### 🟢 不变

- Engine 总数 12；横向系统 5；横切能力 6
- 不加新 Engine
- 12 原则 + 5 横切能力总数不变

#### C.8 结论

V0.2.4 patch 2 完成所有 4 必修 + 5 强烈建议。
**V0.2.4 现在是真正可写的代码架构**。Phase 0.5 + 0.6 启动后**不需要回头改 §1 / §3 / §5 / §8 / §11**。

---

### C.9 V0.2 冻结状态（区分 ARCHITECTURE FREEZE 与 ACCEPTANCE）

#### 当前状态

- **ARCHITECTURE FREEZE (Runtime Semantics Freeze)** ✅
  - V0.2.4 patch 2 已完成
  - 12 Engine + 5 横向系统 + 6 横切能力 + 22 原则 + 9 patch 修正
  - 架构进入"可写代码"状态

- **RELEASE / BASELINE ACCEPTANCE** ⏳
  - Phase 0.5（Operator Workflow + Low-Fi Wireframe）未完成
  - Phase 0.6（Reference A1/A2 + Reference B + Fault Injection）未完成

#### V0.2 正式 Acceptance 条件

1. ✅ V0.2.4 patch 2 完成（本文档当前状态）
2. ⏳ Phase 0.5：9 个 Low-Fi 线框 + 4 条关键操作链（链 1-4）走通
3. ⏳ Phase 0.6：Reference A1 + A2 + Reference B + 5 种 Fault Injection 全部跑通
4. ⏳ 附录 C.5 的 5 个"待 Phase 1 实证"项至少 3 个有实测数据

> **不要称为 V2.0**，保持 V0.2 FINAL 标记。"V0.2 → V2.0"是命名错误。
> Phase 0.5 + 0.6 完成后冻结为 `V0.2 FINAL`，之后是 V0.3（高级播控）。

---

### C.10 V0.2.4 Cleanup（2026-08-24，patch 2 残留修正）

V0.2.4 patch 2 之后做了一次全链路一致性检查，发现 3 个文档层小问题（不涉及运行时语义）：

| # | 位置 | 问题 | 修正 |
|---|---|---|---|
| 1 | 文档头摘要 | 写"新增 §8.10 Failure Domain Matrix"，但实际是 §8.9（AV Sync 是 §8.10） | 改"§8.9 Failure Domain Matrix" |
| 2 | §1.22 Preflight resource | 用旧的 `bmd_channels` 模型，与 §3.11 新 device/port token 不一致 | 改 `bmd_input_ports` / `bmd_output_ports` / `device_exclusivity` |
| 3 | §3.6 Latency Probes | YAML 代码块未闭合（`per_channel_latency_target_ms: 200` 后缺 ```），导致后续 e2e_measurement_modes 渲染错乱；且"7 个"与实际 9-10 个矛盾 | 补 ``` + 改"7 Core + 2 Client E2E + 1 Optional CDN" |

同时同步更新：
- 术语表 Latency Probe 行（7 Core + 2 Client + 1 CDN）

**结论**：C.10 完成后 V0.2.4 真正"自洽"。**不再开 V0.2.5**。**直接进入 Phase 0.5 / 0.6**。

---

### C.11 V0.2.4 Cleanup-2（2026-08-24，7 轮 review 9 项最终 cleanup）

V0.2.4 patch 2 + C.10 之后做了一次"是否真自洽"的全链路检查，发现 9 个残留一致性问题。**全部为文档/规则级**，不涉及架构发散。

| # | 位置 | 残留问题 | 修正 |
|---|---|---|---|
| 1 | §1.13 + §3.1 | Data Plane 两套定义（7 个平铺 vs 4 层）冲突 | §1.13 改引用 §3.1；TypeScript 用 `{ layer, type }` 两维 |
| 2 | §3.4 | "全部 11 项"措辞（实际 17+ 项） | 改 `Mandatory Compatibility Attributes` 命名空间 + optional 区分 |
| 3 | §2.4 | Normalize(stream) 仍写"只改 metadata" | 同步 §3.3 三子能力 REMUX/BITSTREAM_ADAPT/METADATA_REWRITE |
| 4 | §6 决策 #38 | "Latency Probes = 7 个" | 改 "7 Core + 2 Client E2E + 1 Optional CDN" |
| 5 | §8.2 | 切换延迟绝对值（"< 100ms"等） | 全部改 "target X + measured by benchmark" |
| 6 | §3.11 | GPU capacity 写死（4 sessions）+ 假设有 GPU | 改 runtime discovery；当前服务器无 GPU，enabled: false |
| 7 | §3.5 | HOT `resource_factor: 0.8` 写死 | 改 `mode: GRAPH_CALCULATED`，由 Graph Compiler 求和 |
| 8 | §8.10 | AV Sync > 250ms → 切 backup（绕过 Failure Domain） | 改 classify 5 类（SOURCE/PIPELINE/OUTPUT/PLAYER/UNKNOWN），PLAYER 不切源 |
| 9 | §8.11 (新) | 状态机混合 lifecycle/readiness/health 三维度 | 新增 §8.11 三轴状态机；典型 6 种组合 |

**附加 2 项**：

| # | 位置 | 内容 |
|---|---|---|
| +1 | §5 数据模型 | 表顺序注释：migration 必须先建 `config_revisions` / `change_sets` 再建 `graph_runtimes` |
| +2 | 决策 #41-43 | 新增 3 条决策（Hardware Capability Discovery / 三轴状态机 / Data Plane 唯一定义） |

**C.11 结论**：

V0.2.4 + C.10 + C.11 = **V0.2 Architecture Baseline LOCK**。

```yaml
architecture_baseline_lock:
  engines: 12
  cross_systems: 5
  cross_capabilities: 6
  principles: 22
  decisions: 43
  patches: [V0.2.3 patch 1, V0.2.4 patch 2, V0.2.4 Cleanup-1, V0.2.4 Cleanup-2]
  consistency_check: PASS
  self_contained: true
  codable: true
  next_phase: "Phase 0.5 / 0.6"
```

**不再开 V0.2.5 / V0.2.6**。**直接进入 Phase 0.5 + 0.6** → V0.2 FINAL。

---

### C.12 V0.2.4 Cleanup-3（2026-08-24，8 轮 review 8 项最终文字修正）

V0.2.4 Cleanup-2 之后，用户做了最后跨章节对账，发现 8 项残留问题（4 实质 + 3 同步 + 1 enum 大小写）。**全部为文字级**，不涉及架构发散。

| # | 位置 | 残留问题 | 修正 |
|---|---|---|---|
| 1 | §3.2 | RIST/Zixi/NDI 写"已实现" | 改 ✅/⏳ 列：V0.2 实现 11 个 Source Adapter，RIST/Zixi/NDI ⏳ Placeholder（V0.3 实） |
| 2 | §3.7.1 | "干净 Master" 与 §3.7 矛盾 | 改 **Program-scope Master / 节目主母版**；删除"Clean Master"术语；明确"不存在绝对 Clean 母版" |
| 3 | §2.4 | Composition 接 COMPRESSED 让人误以为可直接叠 H.264 | 改 **RAW-domain Engine**：primary=RAW_VIDEO；COMPRESSED 必须先 Decode；Output 仍是 RAW，由后续 Encode 转 |
| 4 | §3.9 | Health Tree Aggregation optional 注释矛盾（说"不影响 healthy 判定"但又让 Channel DEGRADED） | 明确 `optional failed → Channel DEGRADED`（不使 required 失败） |
| 5 | §1.18 | 切换延迟用 `target 5-30s / 0.5-2s / <100ms` 相对值 | 改 `target_failover_time_ms = 30000/1500/100`，与 §3.5 完全一致 |
| 6 | §11 Phase 1 | "Latency Probes（7 个测量点）" | 改 **7 Core + 2 Client E2E + 1 Optional CDN**，与 §3.6 一致 |
| 7 | §12 | 写"V0.2 第 3 轮共 40 项锁定" | 改 **V0.2.4 Cleanup-2 共 43 项**（40 + 新增 #41-43） |
| 8 | §3.1 YAML | `layer: elementary` 小写与 TypeScript `"ELEMENTARY"` 大写不统一 | 全部 UPPER_CASE（ELEMENTARY/CONTAINER/METADATA/CONTROL），与 §1.13 Canonical 同步 |

**C.12 结论**：

V0.2.4 + C.10 + C.11 + C.12 = **V0.2 Architecture Baseline LOCK（最终）**。

```yaml
architecture_baseline_lock_final:
  engines: 12                # 锁定
  cross_systems: 5           # 锁定
  cross_capabilities: 6      # 锁定
  principles: 22             # 锁定
  decisions: 43              # 锁定
  source_adapters_v0_2: 11   # 锁定（RIST/Zixi/NDI 推迟到 V0.3）
  data_plane_layers: 4       # ELEMENTARY / CONTAINER / METADATA / CONTROL（UPPER_CASE canonical）
  program_master_term: "Program-scope Master / 节目主母版"
  composition_domain: RAW    # 锁定
  switch_modes: 3            # PACKET / FRAME / MASTER
  hot_standby_levels: 3      # COLD / WARM / HOT
  failure_domains: 7         # Source / Pipeline / Master / Output / Recording / Clock / Resource
  runtime_state_axes: 3      # Lifecycle / Readiness / Health
  consistency_check: PASS
  self_contained: true
  codable: true
  review_passes: 8
  patches: [V0.2.3 patch 1, V0.2.4 patch 2, V0.2.4 Cleanup-1, V0.2.4 Cleanup-2, V0.2.4 Cleanup-3]
  next_phase: "Phase 0.5 / 0.6"
```

**不再开 V0.2.5 / V0.2.6**。禁止任何架构级修改，除非发现 V0.2 **安全/正确性**错误（必须走 V0.3 流程）。

---

### C.13 V0.2.4 Errata-1（2026-08-24，9 轮 review 10 项实现级规范修正）

V0.2.4 Cleanup-3 之后，用户按"代码实现能否只看这份 Baseline 就不产生歧义"标准重审，发现 10 项实现级规范问题。**全部为数字/Schema/单位/版本/状态机映射**，不涉及架构发散。

#### 🔴 6 项必须修

| # | 位置 | 问题 | 修正 |
|---|---|---|---|
| 1 | §3.1 RAW_VIDEO | `1080p25 uyvy422 ≈ 414 Mbps` 算错 | 改 **829.44 Mbps ≈ 103.68 MB/s**（1920×1080×25×2 = 103.68 MB/s） |
| 1 | §3.1 RAW_VIDEO | `1080p25 v210 ≈ 622 Mbps` 算错 | 改 **1.106 Gbps ≈ 138.24 MB/s**（21.33 bit/pixel）|
| 1 | §3.1 RAW_AUDIO | `48kHz 8ch 24bit ≈ 9.2 Mbps` 算错 | 改 **1.152 Mbps ≈ 0.144 MB/s**（48000×8×24/8）|
| 2 | §3.11 sdi_capture_node | `pcie_rx_mb_s: 414` 单位错（实为 103.68 MB/s） | 改 `mode: FORMULA` + formula + inputs，硬件/Compiler 实际计算 |
| 3 | §6 决策 #2 | "SRS 8.0 线" 未验证版本 | 改 **SRS Stable Line / 验证基线 6.x**（通过 SRSAdapter 解耦版本）|
| 4 | §5 latency_probes | 缺 `source_reference` + `measurement_mode` | 增 `source_reference` ENUM + `measurement_mode` ENUM（4 模式）|
| 5 | §3.4 PACKET_SWITCH | mandatory 混合 capability + runtime alignment | 拆 `capability_contract`（静态，X6 Registry）+ `runtime_alignment`（动态，实时测量）|
| 6 | §1.22 Preflight | 缺 vram / pcie_tx | 补 9 维度完整 Vector（cpu/gpu_sessions/vram/ram/nic/disk/pcie_rx/pcie_tx/bmd_input/output）|

#### 🟠 4 项建议同步

| # | 位置 | 修正 |
|---|---|---|
| 7 | §3.5 | COLD/WARM `resource_factor: 0.05/0.5` 写死经验值 → 改 `resource_estimation.mode: GRAPH_CALCULATED`（统一 HOT/WARM/COLD）|
| 8 | §5 change_sets | status 是 business outcome（终态）；新增 `change_set_events` 表（PREPARING/APPLYING/COMMITTED/ABORTED）作为 transaction phase |
| 9 | 术语表 | `Failure Domain Matrix \| §8.10` → `§8.9`（章节号 Cleanup-2 后没同步）|
| 10 | 文档头 | "9+8+8 项 patch" 数字难对账 → 改具体 patch 名（V0.2.3 patch 1 / V0.2.4 patch 2 / Cleanup-1/2/3 / Errata-1）|

#### C.13 结论

V0.2.4 + C.10 + C.11 + C.12 + C.13 = **V0.2 Architecture Baseline LOCK FINAL**。

```yaml
architecture_baseline_lock_final:
  status: LOCK_FINAL
  engines: 12
  cross_systems: 5
  cross_capabilities: 6
  principles: 22
  decisions: 43
  source_adapters_v0_2: 11
  data_plane:
    layers: [ELEMENTARY, CONTAINER, METADATA, CONTROL]
  program_master: PROGRAM_SCOPE_MASTER
  composition: RAW
  switch_modes: [PACKET_SWITCH, FRAME_SWITCH, MASTER_SWITCH]
  hot_standby: [COLD, WARM, HOT]
  failure_domains: 7
  runtime_state_axes: [LIFECYCLE, READINESS, HEALTH]

  # V0.2.4 Errata-1 新增锁定
  raw_video_bandwidth:
    UYVY422_1080p25: "829.44 Mbps (103.68 MB/s)"
    V210_1080p25:    "1.106 Gbps (138.24 MB/s)"
  raw_audio_bandwidth:
    PCM_24bit_48kHz_8ch: "1.152 Mbps (0.144 MB/s)"
  packet_switch_eligibility:
    layers: [capability_contract (X6), runtime_alignment (live)]
  preflight_resource_dimensions: 9    # cpu/gpu/vram/ram/nic_in/nic_out/disk/pcie_rx/pcie_tx
  latency_probes:
    probe_points: 10    # source_reference/capture/decode/switch/compose/encode/mux/publish/cdn/player
    measurement_modes: 4   # STAGE/SYNC/EMBED/APPROX
  srs_baseline: "Stable Line / 6.x (decoupled by SRSAdapter)"
  consistency_check: PASS
  codable: true
  review_passes: 9
  patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1]
  next_phase: "Phase 0.5 / 0.6"
```

**禁止项不变**：
- ❌ 任何架构级修改（除非 V0.2 **安全/正确性**错误 → V0.3 流程）
- ❌ 增加 Engine / 改 11 Source Adapter / 改核心定义

**Errata-1 锁定的额外可执行细节**：
- ✅ RAW 带宽公式 + 典型值（避免实现时再算）
- ✅ PCIe / SDI 走 FORMULA，不写死 414
- ✅ SRS 不绑未来版本号
- ✅ latency_probes 完整 Schema（10 probe points + 4 modes）
- ✅ PACKET_SWITCH 静态/动态拆分
- ✅ Preflight 用完整 9 维 Resource Vector
- ✅ COLD/WARM/HOT 资源消耗统一 GRAPH_CALCULATED
- ✅ ChangeSet 业务结果 vs 执行阶段分离

---

### C.14 V0.2.4 Errata-2（2026-08-24，10 轮 review 6 项模型/快照分离修正）

V0.2.4 Errata-1 之后做"反向一致性审查"：不是再找旧问题，而是看 Errata-1 修完以后有没有留下新矛盾。**6 项全部为模型 vs 快照分离**，不涉及架构发散。

#### 🔴 2 项必修

| # | 位置 | 问题 | 修正 |
|---|---|---|---|
| 1 | §3.11 | "9 维 Resource Vector" 实际是 9 维 + BMD Token | 正式锁定 **9-dim Quantitative Resource Vector + Device/Port Token Constraints**；§1.22 / §3.11 / #41 全部统一 |
| 2 | §5 hot_standby_levels | 表保留 `resource_factor` 与 §3.5 "GRAPH_CALCULATED" 冲突 | **删 `resource_factor` 字段**；runtime benchmark 进独立 measurement 表 |

#### 🟠 3 项强烈建议

| # | 位置 | 修正 |
|---|---|---|
| 3 | §3.11 pcie_rx_mb_s | `pcie_*_mb_s` 是 **SCHEDULING ESTIMATE**（基于 media payload），**≠ 实测 PCIe bus utilization**；实测由 hardware telemetry 校准 |
| 4 | §3.11 gpu | `gpu.enabled: false` 是当前服务器快照，不是架构 | 拆 **Architecture（Runtime Discovery）** vs **`current_host_snapshot`（部署参考）** |
| 5 | §3.11 bmd_devices | 3 张具体型号写成架构事实 | 改 Runtime Discovery 产 `model/serial/duplex_mode/supported_formats`；当前型号进 `current_host_snapshot` |

#### 🟡 1 项顺手统一

| # | 位置 | 修正 |
|---|---|---|
| 6 | §3.4 | 旧 `packet_switch_eligibility_check` YAML 与新 `packet_switch_eligibility.capability_contract + runtime_alignment` 重复 | **删除旧 YAML**，仅保留 canonical `packet_switch_eligibility` |

#### C.14 结论

V0.2.4 + C.10 + C.11 + C.12 + C.13 + C.14 = **V0.2 Architecture Baseline LOCK FINAL**。

```yaml
architecture_baseline_lock_final:
  status: LOCK_FINAL
  engines: 12
  cross_systems: 5
  cross_capabilities: 6
  principles: 22
  decisions: 47                        # V0.2.4 Errata-2 +4 (#44-47)

  # V0.2.4 Errata-2 新增锁定
  resource_model:
    name: "9-dim Quantitative Resource Vector + Device/Port Token Constraints"
    quantitative_dimensions: 9
    device_tokens: [bmd_input_port, bmd_output_port, device_exclusivity]
  architecture_vs_snapshot:
    rule: "Architecture ≠ current_host_snapshot"
    consequence: "机器增 GPU / BMD 不修改 V0.2"
  pcie_bandwidth_semantics:
    pcie_rx_mb_s: "SCHEDULING ESTIMATE (media payload)"
    pcie_tx_mb_s: "SCHEDULING ESTIMATE (media payload)"
    measured_pcie_bus_utilization: "runtime hardware telemetry"
  hot_standby_levels_schema:
    resource_factor: REMOVED         # 运行时按 Graph 计算，不属 lookup
    benchmark: runtime measurement table

  # 既有锁定
  data_plane:
    layers: [ELEMENTARY, CONTAINER, METADATA, CONTROL]
  program_master: PROGRAM_SCOPE_MASTER
  composition: RAW
  switch_modes: [PACKET_SWITCH, FRAME_SWITCH, MASTER_SWITCH]
  hot_standby: [COLD, WARM, HOT]
  failure_domains: 7
  runtime_state_axes: [LIFECYCLE, READINESS, HEALTH]
  raw_video_bandwidth:
    UYVY422_1080p25: "829.44 Mbps (103.68 MB/s)"
    V210_1080p25:    "1.106 Gbps (138.24 MB/s)"
  raw_audio_bandwidth:
    PCM_24bit_48kHz_8ch: "1.152 Mbps (0.144 MB/s)"
  packet_switch_eligibility:
    layers: [capability_contract (X6), runtime_alignment (live)]
  preflight_resource_dimensions: 9
  latency_probes:
    probe_points: 10
    measurement_modes: 4
  srs_baseline: "Stable Line / 6.x (decoupled by SRSAdapter)"

  consistency_check: PASS
  codable: true
  review_passes: 10
  patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1, Errata-2]
  next_phase: "Phase 0.5 / 0.6"
```

**禁止项不变** + 新增：

- ❌ 任何架构级修改（除非 V0.2 **安全/正确性**错误 → V0.3 流程）
- ❌ 把 `current_host_snapshot` 内容写进 Architecture
- ❌ 把 `pcie_*_mb_s` 当成实测值
- ❌ 在 hot_standby_levels lookup 表加回 `resource_factor`

**Errata-2 锁定的关键边界**：

```
9-dimensional Resource Vector
        +
Device/Port Token Constraints

Architecture Capability
        ≠
Runtime Hardware Snapshot

Estimated PCIe Payload
        ≠
Measured PCIe Bus Utilization

Capability Contract
        +
Runtime Alignment
        =
PACKET_SWITCH Eligibility
```

这四条一旦锁死，Phase 0.5/0.6 可以从"架构讨论"切换成 **Executable Acceptance Specification**。

---

### C.15 V0.2.4 Errata-3（2026-08-24，11 轮 review 8 项实现歧义 / 文档修正）

V0.2.4 Errata-2 之后做"反向一致性审查"（patch 1 修完是否引入新矛盾）。发现 **8 项**：#1-#4 实现歧义 + #5-#7 文档级 + #8 数字一致性。**全部不涉及架构发散**。

#### 🔴 4 项实现歧义

| # | 位置 | 问题 | 修正 |
|---|---|---|---|
| 1 | §3.7 | Program-scope Master 是 RAW 还是压缩域模糊；§3.7 顶部又写 "Compose → Encode → Master" 残留 | **锁死边界**：`Program-scope Master = RAW 域 / Encode = delivery boundary / Output Variant = delivery-domain derivative`；加完整链路图 |
| 2 | §2.4 + 决策 #12 | Output(SDI) Engine 在 §2.4 列出，决策 #12 又写"SDI Master Output V0.2 不做" | 改 **`Architecture Contract: RESERVED / V0.2 Implementation: DISABLED / Target: V0.4`**（决策 #49）|
| 3 | §3.5 + §1.18 | 残留"实测 50-200ms"或"target × 0.5 ~ target × 2"固定范围 | **删**全部固定范围；只保留 `target_failover_time_ms` + `failover_benchmarks`（决策 #50）|
| 4 | §8.2 | 只检查"全部 mandatory 相等"，与 §3.4 两层模型脱节 | 改 **§8.2 只引用 §3.4 `packet_switch_eligibility`**，不复制判断字段（决策 #51）|

#### 🟠 3 项文档级

| # | 位置 | 修正 |
|---|---|---|
| 5 | §3.11 + Canonical Vocabulary | `DEVICE_EXCLUSIVITY` 是 Constraint 不是 Token；拆 `DeviceToken`（BMD_INPUT_PORT / BMD_OUTPUT_PORT）+ `DeviceConstraint`（DEVICE_EXCLUSIVITY）|
| 6 | §5 | `failover_benchmarks` 表缺失 | 加完整表（channel_id / route_id / switch_mode / hot_standby_level / measured_at / sample_count / p50/p95/p99 / test_profile_json / runtime_revision_id）|
| 7 | §6 决策表 | 编号 41, 44, 45, 46, 47, 42, 43 不连续 | 重排 41-47 连续；Errata-3 新增 #48-51 |
| 8 | §3.1 V210 | `1.106 Gbps` 缺精确值 | 改 `1.10592 Gbps ≈ 1.106 Gbps` |

#### C.15 结论

V0.2.4 + C.10 + C.11 + C.12 + C.13 + C.14 + C.15 = **V0.2 Architecture Baseline LOCK FINAL**。

```yaml
architecture_baseline:
  status: LOCK_FINAL
  runtime_semantics: FROZEN
  implementation_authority: THIS_DOCUMENT
  engines: 12
  cross_systems: 5
  cross_capabilities: 6
  principles: 22
  decisions: 51                  # Errata-3 +4 (#48-51)
  source_adapters_v0_2: 11

  # V0.2.4 Errata-3 新增锁定
  program_master_domain:
    type: "RAW-domain semantic master"
    encode: "delivery boundary"
    output_variant: "delivery-domain derivative"
  sdi_master_output:
    architecture_contract: RESERVED
    v0_2_implementation: DISABLED
    target: V0.4
  hot_standby_fixed_range: FORBIDDEN     # 仅 target + failover_benchmarks
  switch_mode_decision: "Single Source of Truth = §3.4；§8.2 只能引用"
  resource_model:
    quantitative_vector: 9-dim
    device_tokens: [BMD_INPUT_PORT, BMD_OUTPUT_PORT]
    device_constraints: [DEVICE_EXCLUSIVITY]
  failover_benchmarks: "独立 measurement 表（§5）"

  consistency_check: PASS
  codable: true
  review_passes: 11
  patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3]
  next_review:
    phase: "0.5 / 0.6"
    purpose: "acceptance only"
  v0_2_5: FORBIDDEN
```

**禁止项**：

- ❌ 任何架构级修改（除非 V0.2 **安全/正确性**错误 → V0.3 流程）
- ❌ 把 Program Master 实现为压缩域
- ❌ 实现 SDI Master Output（V0.2 阶段）
- ❌ 写"实测 50-200ms"等固定范围
- ❌ 在 §8.2 复制 §3.4 的 PACKET_SWITCH 判断字段
- ❌ 把 DEVICE_EXCLUSIVITY 当 Token（它是 Constraint）
- ❌ 在 hot_standby_levels lookup 加回 `resource_factor`
- ❌ 把 current_host_snapshot 写进 Architecture

**下一阶段**：Phase 0.5（Operator Workflow + Low-Fi Wireframe）+ Phase 0.6（Executable Acceptance Specification：Reference A1/A2/B + 5 Fault Injection）→ **V0.2 FINAL**。

**之后**：禁止第 12 轮 Runtime Semantics review。所有架构争议以"实测行为"为准。

---

### C.16 V0.2.4 Errata-4（2026-08-24，12 轮 review 7 项最后 cleanup）

V0.2.4 Errata-3 之后做"反向一致性 + 反旧语义残留"检查。发现 7 项。**全部为实现歧义 / 文档级**，不涉及架构发散。

| # | 位置 | 修正 |
|---|---|---|
| 1 | §3.5 顶部 | 删"实测范围在 `target × 0.5 ~ target × 2`"残留；只留 `target + failover_benchmarks` 表述 |
| 2 | §3.4 | **锁死 Switch Mode 降级链** `PACKET → FRAME → MASTER → REJECT`（不是三个并列标签）；新增 `switch_mode_decision_tree` Canonical |
| 3 | §3.5 | `resource_estimation` 删 `default_estimate` / `measured_factor`，只留 `mode: GRAPH_CALCULATED` |
| 4 | §3.5 | `hot_standby_levels` 删 `benchmark` 嵌入式结构；benchmark 数据全归 `failover_benchmarks` 表 |
| 5 | §3.7.1 | Output Variant 明确 `V0.2 enabled: SRS / File / UDP`；`V0.4 reserved: SDI Adapter`（Architecture Contract RESERVED / V0.2 DISABLED）|
| 6 | 文档元数据 | 全文同步：11 reviews / 51 decisions / Errata-3 / C.15=8 项 / 22+51（22+47 残留修正）|
| 7 | §8.2 | 不再写 `<100ms / 0.5-2s / 1-3s` 文字；target 唯一来源 = `channel_routes` + `hot_standby_levels.target_failover_time_ms` |

#### C.16 结论

V0.2.4 + C.10 + C.11 + C.12 + C.13 + C.14 + C.15 + C.16 = **V0.2 Architecture Baseline LOCK FINAL**。

```yaml
architecture_baseline:
  status: LOCK_FINAL
  runtime_semantics: FROZEN
  implementation_authority: THIS_DOCUMENT
  engines: 12
  cross_systems: 5
  cross_capabilities: 6
  principles: 22
  decisions: 51
  review_passes: 12
  patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3/4]

  architecture_changes_after_this:
    forbidden: true
    exception: "safety/correctness defect only"

  next_phase:
    - Phase 0.5
    - Phase 0.6
  next_review_type:
    acceptance_only: true
    runtime_semantics_review: false

  v0_2_5: FORBIDDEN
```

**Runtime Semantics Review 正式结束**。下一阶段 Phase 0.5（Operator Workflow + Low-Fi Wireframe）+ Phase 0.6（Executable Acceptance Specification：Reference A1/A2/B + 5 Fault Injection）。所有架构争议以"实测行为"为准。

---

### C.17 V0.2.4 Errata-5（2026-08-24，13 轮 review 5 项去残留）

V0.2.4 Errata-4 之后做"代码只看 Baseline 不允许脑补"标准重审。发现 5 项。**全部为去残留 / 消除歧义**，不涉及架构发散。

| # | 位置 | 修正 |
|---|---|---|
| 1 | §3.4 | 删旧句"不满足 capability_contract.mandatory 全部 OR runtime_alignment.required 任一 → 自动降级到 FRAME_SWITCH 或拒绝"；明确 **PACKET fail 后必须继续执行 switch_mode_decision_tree 降级链**，禁止绕过 |
| 2 | §3.4 + channel_routes | **`channel_routes.switch_mode = Compiler resolved value`**（V0.2 不允许绕过 Compiler 手工写入） |
| 3 | §8.3 | 删"编码稳定 + pipe 满缓冲 → HOT"简化谓词；**HOT 必须满足 `READY_TO_TAKE` 完整 predicate**（§3.5 + §8.11）|
| 4 | §5 switch_modes | 删 `max_failover_ms` 字段；target 唯一来源 = `hot_standby_levels.target_failover_time_ms`；实测唯一来源 = `failover_benchmarks` |
| 5 | §8.4 | 修"主源异常 > failback_hysteresis_ms" 逻辑错误（应 `failover_hysteresis_ms`）；明确 failover/failback 字段语义 |
| 6 | 附录 D | 47 → 51（同步 Errata-2/3/4/5 新增决策） |

#### C.17 结论

V0.2.4 + C.10 + C.11 + C.12 + C.13 + C.14 + C.15 + C.16 + C.17 = **V0.2 Architecture Baseline LOCK FINAL**。

```yaml
architecture_baseline:
  status: LOCK_FINAL
  runtime_semantics: FROZEN
  implementation_authority: THIS_DOCUMENT
  engines: 12
  cross_systems: 5
  cross_capabilities: 6
  principles: 22
  decisions: 51
  review_passes: 13
  patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3/4/5]

  # V0.2.4 Errata-5 关键锁定
  switch_mode_decision:
    source_of_truth: §3.4 switch_mode_decision_tree
    channel_routes_switch_mode: "Compiler resolved value (V0.2 不允许手工绕过)"
    degraded_path: "PACKET fail → FRAME → MASTER → REJECT (禁止跳过)"
  hot_standby_readiness:
    hot_predicate: "READY_TO_TAKE (完整 predicate from §3.5 + §8.11)"
    forbidden: "encoder_stable && pipe_full (简化谓词)"
  target_time_source:
    target: "hot_standby_levels.target_failover_time_ms (唯一)"
    measured: "failover_benchmarks (唯一)"
    forbidden: "switch_modes.max_failover_ms (已删)"

  next_phase: [Phase 0.5, Phase 0.6]
  next_review_type:
    acceptance_only: true
    runtime_semantics_review: false
  v0_2_5: FORBIDDEN
```

**Runtime Semantics Review 正式关闭**。任何发现进入 Phase 0.5/0.6 Acceptance → 实测 → Executable Acceptance Specification，**不再开第 14 轮架构 review**。

---

### C.18 V0.2.4 Errata-6（2026-08-24，14 轮 review 4 项最终锁定）

V0.2.4 Errata-5 之后做"代码只看 Baseline 不允许脑补"标准重审。发现 4 项。**全部为消除歧义 / 严格分离**，不涉及架构发散。修完正式 V0.2 Runtime Semantics = **CLOSED / IMPLEMENTATION AUTHORITY**。

| # | 位置 | 修正 |
|---|---|---|
| 1 | §3.13 | `classify_before_action` **只识别**（source_pts_unstable / pipeline_slow / output_mux_error / player_buffer_anomaly → failure_domain）；**不**指定 action。**最终恢复动作由 §8.9 Failure Domain Matrix 决定**（§8.9 = Recovery Policy SoT）|
| 2 | §3.4 channel_routes | 严格分离 `COMPILED_MODE`（`channel_routes.switch_mode`，Compiler 写入）vs `EFFECTIVE_RUNTIME_MODE`（`media_session_runtime.effective_switch_mode`，Runtime 写入）；Runtime alignment 变化可降级但**不**反写 `channel_routes` |
| 3 | Canonical Vocabulary | 新增 **`SwitchDecisionResult` = PACKET_SWITCH \| FRAME_SWITCH \| MASTER_SWITCH \| REJECT**；明确 **REJECT ≠ SwitchMode**（REJECT 是 Decision Outcome）|
| 4 | §3.5 hot_standby_levels | **删 `state: STOPPED/STARTING/READY_TO_TAKE` 字段**；`READY_TO_TAKE` 是 §8.11 Readiness 维度，不是 HotStandbyLevel 字段。HotStandbyLevel = 策略 / 目标；Runtime State = 当前事实 |

#### C.18 结论

V0.2.4 + C.10 + ... + C.18 = **V0.2 Runtime Semantics CLOSED / IMPLEMENTATION AUTHORITY**。

```yaml
v0_2_runtime_semantics:
  status: CLOSED
  implementation_authority: THIS_DOCUMENT
  engines: 12
  cross_systems: 5
  cross_capabilities: 6
  principles: 22
  decisions: 53                       # Errata-6 +2 (#52-53)
  review_passes: 14
  patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3/4/5/6]

  # V0.2.4 Errata-6 锁定的最终边界
  configuration_vs_runtime:
    channel_routes_switch_mode: COMPILED_MODE          # Graph Compiler 写入
    media_session_effective_switch_mode: EFFECTIVE_RUNTIME_MODE  # Runtime 写入
    runtime_must_not_overwrite_compiled: true

  identification_vs_recovery:
    section_3_13: identification_only   # 只分类
    section_8_9: recovery_policy_sot   # 唯一决定恢复动作

  type_distinction:
    SwitchMode:        PACKET_SWITCH | FRAME_SWITCH | MASTER_SWITCH
    SwitchDecisionResult: PACKET_SWITCH | FRAME_SWITCH | MASTER_SWITCH | REJECT
    REJECT: decision_outcome_not_switchmode

  standby_state:
    hot_standby_level: policy_target   # 策略/目标
    three_axis_runtime_state: actual_fact   # 实际事实

  next_phase: [Phase 0.5, Phase 0.6]
  next_review_type:
    acceptance_only: true
    runtime_semantics_review: false
  v0_2_5: FORBIDDEN
```

**V0.2 Runtime Semantics Review 正式关闭 = CLOSED / IMPLEMENTATION AUTHORITY**。

后续任何问题统一进入：
- Phase 0.5（Operator Workflow + Low-Fi Wireframe）
- Phase 0.6（Reference A1/A2/B + Fault Injection + Executable Acceptance Specification）
- 实测证据 = 最终架构真相

**不再开第 15 轮架构 review**。

---

### C.19 V0.2.4 Errata-7（2026-08-24，15 轮 review 5 项旧文本/同步最终修补）

V0.2.4 Errata-6 之后做"残留旧文本"最终扫描。发现 5 项：

| # | 位置 | 修正 |
|---|---|---|
| 1 | §3.4 | **删旧 `decision: ... else: 降级到 FRAME_SWITCH 或拒绝` 块**；`packet_switch_eligibility` 只提供 PACKET 的 eligibility inputs；最终 SwitchDecisionResult 由 `switch_mode_decision_tree` 统一产生（**Eligibility ≠ Decision**）|
| 2 | §8.3 | **重写 Hot-Standby Level Progression**；明确 `HotStandbyLevel = Policy / Target`，真实 Runtime 状态由 §8.11 三轴状态机表达；**WARM ≠ STARTING**（WARM 完全可能 `lifecycle=RUNNING, readiness=NOT_READY`）|
| 3 | §8.4 | 改 **§3.4 Decision Tree + 当前 Runtime Alignment → EFFECTIVE_RUNTIME_MODE**；`channel_routes.switch_mode = COMPILED_MODE` 不作为 Runtime 恢复直接执行依据 |
| 4 | §11 Phase 0.6 | 章节引用 **§8.10 → §8.9**（Failure Domain Matrix 同步）|
| 5 | 附录 D | **51 → 53 决策**（统一 12+5+6+22+53）|

#### C.19 结论

V0.2.4 + C.10 + ... + C.19 = **V0.2 Runtime Semantics = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE**。

```yaml
v0_2_runtime_semantics:
  status: CLOSED
  implementation_authority: THIS_DOCUMENT
  consistency_check: PASS
  implementation_ambiguity: NONE
  engines: 12
  cross_systems: 5
  cross_capabilities: 6
  principles: 22
  decisions: 53
  review_passes: 15
  patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3/4/5/6/7]

  # V0.2.4 Errata-7 锁定的最终边界
  eligibility_vs_decision:
    section_3_4_packet_switch_eligibility: "only provides PACKET eligibility inputs"
    section_3_4_switch_mode_decision_tree: "produces final SwitchDecisionResult"
    rule: "Eligibility ≠ Decision"

  standby_level_progression:
    hot_standby_level: "Policy / Target (configuration intent)"
    three_axis_runtime_state: "Actual Fact (current truth)"
    warm_not_equals_starting: true

  runtime_execution:
    channel_routes_switch_mode: COMPILED_MODE         # Configuration Intent
    media_session_effective_switch_mode: EFFECTIVE_RUNTIME_MODE  # Runtime actual
    runtime_drives_execution: true                    # Runtime 驱动执行

  architecture_changes_after_this: FORBIDDEN
  next_phase: [Phase 0.5, Phase 0.6]
  next_review_type: ACCEPTANCE_ONLY
```

**V0.2 Runtime Semantics Review 正式关闭 = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE**。

不再开第 16 轮架构 review。下一步直接进入 Phase 0.5 + Phase 0.6。

---

### C.20 V0.2.4 Errata-8（2026-08-24，16 轮 review 6 项最终清理）

V0.2.4 Errata-7 之后做"工程师只看当前正文，不看历史 Errata，也不能脑补"标准重审。发现 6 项：

| # | 位置 | 修正 |
|---|---|---|
| 1 | §3.5 | 顶部注释删 `state` 字段（保留 description / target / resource_estimation.mode / use_case）；删旧句"状态字段是 READY_TO_TAKE，不是 RUNNING" |
| 2 | §8.11 | 引用修正：Health Tree 聚合用 **§3.9**（不是 §8.9，§8.9 是 Failure Domain Matrix）。两个 SoT 彻底分离：§3.9 = Health Aggregation SoT，§8.9 = Recovery Policy SoT |
| 3 | §8.3 | 标题改 **Hot-Standby Level Semantics**（不再是 Progression）；图示明确 **3 个并列 Policy / Target**（无相互迁移）；禁止 `match standby_level { COLD => WARM, ... }` 模式 |
| 4 | C.5 | 重写"自动选择算法"和"80% resource factor"为**Phase 1 实证项**（架构已锁，不是开放决策）|
| 5 | C.6 / 附录 A | "Latency Probes 7 测量点" 改 **初始 7 Core Stage；扩展为 10 = 7+2+1**（历史 + 当前明确）|
| 6 | §1.18 / §3.4 | 列名 **"切换延迟" → "Policy / Target"**；§3.4 删"target < 100ms"绝对句式，改"以关联 profile target + benchmark 验证"|

#### C.20 结论

V0.2.4 + C.10 + ... + C.20 = **V0.2 Runtime Semantics = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE**。

```yaml
v0_2_runtime_semantics:
  status: CLOSED
  implementation_authority: THIS_DOCUMENT
  consistency_check: PASS
  implementation_ambiguity: NONE
  runtime_semantics_review: CLOSED
  engines: 12
  cross_systems: 5
  cross_capabilities: 6
  principles: 22
  decisions: 53
  review_passes: 16
  patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3/4/5/6/7/8]
  architecture_changes_after_this: FORBIDDEN
  next_phase: [Phase 0.5, Phase 0.6]
  next_review_type: ACCEPTANCE_ONLY
```

**V0.2 Runtime Semantics Review 正式关闭 = CLOSED / implementation_ambiguity: NONE**。

**不再开第 17 轮架构 review**。下一步直接进入 Phase 0.5 + Phase 0.6。

---

### C.21 V0.2.4 Errata-9（2026-08-24，17 轮 review 8 项最终硬点）

V0.2.4 Errata-8 之后做"实现级 Schema / Type / 表"最终审。发现 8 项（4 必修 + 4 顺手）。**全部为实现级硬点**，不涉及架构发散。

| # | 位置 | 修正 |
|---|---|---|
| 1 | §5 数据模型 | **新增 `media_session_runtime` 表**（lifecycle/readiness/health + effective_switch_mode + runtime_alignment_state + runtime_revision_id）；新增 `channel_health_view` derived view（status 不作为独立事实字段）|
| 2 | Canonical Vocabulary + §3.13 + §8.10 | **拆 `OperationalFailureDomain` (7) + `DiagnosticFailureClass` (PLAYER/UNKNOWN)**；PLAYER/UNKNOWN 不进 7 OperationalFailureDomain，只 NOTIFY/SAFE_DEGRADE，不切源 |
| 3 | §3.13 角色描述 | **AVSync Manager = Measurement + Offset/Drift Correction + Failure Classification**（不是"identification only"）；Recovery Action 仍由 §8.9 决定 |
| 4 | §3.4 Decision Tree | **新增 `common_raw_contract_resolution` 规则**（4 步：intersect / apply target / apply clock / select canonical）；**`WARN ≠ PASS`**（PACKET_SWITCH 严格）|
| 5 | §5 migration | **8 步顺序**：config_revisions → change_sets → change_set_items → graph_specs → graph_revisions → graph_runtimes → media_sessions → media_session_runtime |
| 6 | §8.11 | `channel_routes.status` **改为 derived view**（`channel_health_view`），不存为独立字段；Runtime State 唯一来源 = §8.11 三轴 |
| 7 | 附录 C 开头 | **历史说明**：C.1-C.20 数字为当轮历史状态；Current authoritative state 以文档头 + C.20/C.21 + Appendix D 为准（53 decisions / 16-17 review）|
| 8 | 决策 #54-#57 | **新增 4 条决策**：media_session_runtime 表 / OperationalFailureDomain vs DiagnosticFailureClass / AVSync Manager 角色扩展 / WARN ≠ PASS |

#### C.21 结论

V0.2.4 + C.10 + ... + C.21 = **V0.2 Runtime Semantics = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE**。

```yaml
v0_2_runtime_semantics:
  status: CLOSED
  implementation_authority: THIS_DOCUMENT
  consistency_check: PASS
  implementation_ambiguity: NONE
  runtime_semantics_review: CLOSED
  engines: 12
  cross_systems: 5
  cross_capabilities: 6
  principles: 22
  decisions: 57                       # Errata-9 +4 (#54-57)
  review_passes: 17
  patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3/4/5/6/7/8/9]

  architecture:
    engines: 12
    cross_systems: 5
    cross_capabilities: 6
    principles: 22
    decisions: 57

  canonical_runtime:
    switch:
      compiled_mode: channel_routes.switch_mode
      effective_mode: media_session_runtime.effective_switch_mode
      warn_equals_pass: false       # PACKET strict
    standby:
      level: policy_target
      runtime: lifecycle + readiness + health
    failure:
      operational_domains: 7        # SOURCE/PIPELINE/MASTER/OUTPUT/RECORDING/CLOCK/RESOURCE
      diagnostic_classes: [PLAYER, UNKNOWN]

  consistency_check: PASS
  implementation_ambiguity: NONE
  architecture_changes_after_this: FORBIDDEN
  next_phase: [Phase 0.5, Phase 0.6]
  next_review_type: ACCEPTANCE_ONLY
```

**V0.2 Runtime Semantics Review 正式关闭 = CLOSED / implementation_ambiguity: NONE**。

**不再开第 18 轮架构 review**。下一步直接进入 Phase 0.5 + Phase 0.6。

---

### C.22 V0.2.4 Errata-10（2026-08-24，18 轮 review 5 项最后修补）

V0.2.4 Errata-9 之后做最后反向审计。发现 5 项（3 必修 + 1 强烈建议 + 1 顺手）。**全部为 Baseline 自洽修补**，不涉及架构发散。

| # | 位置 | 修正 |
|---|---|---|
| 1 | 附录 D / C.20 | 决策数 **53 → 57** 统一；review **16 → 17** 统一；移除 53/57 冲突 |
| 2 | `§5` channel_health_view | **改 JOIN Health Tree Aggregation**（`channel_health_aggregation` 中间视图）→ `effective_channel_status` 来自 **§3.9 Health Tree Aggregation Policy SoT**；不再直接从 `media_session_runtime.health` 派生 |
| 3 | Canonical Vocabulary | **新增 `EffectiveChannelStatus`** 枚举（HEALTHY/DEGRADED/FAILED/STARTING/STOPPED/UNKNOWN）；明确 ≠ HealthState ≠ LifecycleState |
| 4 | Canonical Vocabulary | **彻底废除 `FailureDomain` alias**；只保留 `OperationalFailureDomain` + `DiagnosticFailureClass`；TS/Rust/JSON Schema/PG enum 不再创建同名 enum |
| 5 | 文档头 | "硬件: 3 张 BMD" → "**部署参考快照**（非 Architecture Fact）"；与 §3.11 current_host_snapshot 统一 |

#### C.22 结论

V0.2.4 + C.10 + ... + C.22 = **V0.2 Runtime Semantics = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE**（最终）。

```yaml
v0_2_runtime_semantics:
  status: CLOSED
  implementation_authority: THIS_DOCUMENT
  consistency_check: PASS
  implementation_ambiguity: NONE

  architecture:
    engines: 12
    cross_systems: 5
    cross_capabilities: 6
    principles: 22
    decisions: 57
    review_passes: 18
    patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3/4/5/6/7/8/9/10]

  canonical_state:
    lifecycle: [STOPPED, STARTING, RUNNING, STOPPING]
    readiness: [NOT_READY, READY_TO_TAKE]
    health:    [HEALTHY, DEGRADED, FAILED, UNKNOWN]

  failure_model:
    operational_domains: 7          # SOURCE/PIPELINE/MASTER/OUTPUT/RECORDING/CLOCK/RESOURCE
    diagnostic_classes: [PLAYER, UNKNOWN]

  switch_model:
    compiled_mode: channel_routes.switch_mode
    effective_mode: media_session_runtime.effective_switch_mode

  channel_health:
    source_of_truth: "§3.9 Health Tree Aggregation Policy"
    runtime_fact_source: "media_session_runtime"
    presentation_view: "channel_health_view (effective_channel_status)"
    effective_channel_status_enum: EffectiveChannelStatus   # ≠ HealthState, ≠ LifecycleState

  architecture_changes_after_this: FORBIDDEN
  next_phase: [Phase 0.5, Phase 0.6]
  next_review_type: ACCEPTANCE_ONLY
```

**V0.2 Runtime Semantics Review 正式关闭 = CLOSED / implementation_ambiguity: NONE（最终）**。

**不再开第 19 轮架构 review**。下一步直接进入 Phase 0.5 + Phase 0.6。

---

### C.23 V0.2.4 Errata-11 Post-Freeze Schema Correction（2026-08-24，19 轮 review 5 项 Schema 收口）

V0.2.4 Errata-10 之后做"Schema 级最终审计"。发现 5 项（3 必修 + 1 enum + 1 UNIQUE）。**全部为 Schema 收口**，不涉及架构发散。

| # | 位置 | 修正 |
|---|---|---|
| 1 | `§5` `health_tree_nodes` | 加 `required_node BOOLEAN NOT NULL`（§3.9 required_node/optional_node DB 表达）；删 `channel_id`（归属由 `health_trees.channel_id` 决定）；改 `state` ENUM 用 UPPER_CASE；加 `UNIQUE (health_tree_id, node_path)` |
| 2 | `§5` Schema | **新增 `current_health_trees` View**（DISTINCT ON channel_id 取最新 snapshot）；重写 `channel_health_aggregation` SQL 引用正确字段；明确 History = health_trees / Current = current_health_trees |
| 3 | `§5` | **新增 `effective_channel_status_policy`**：Lifecycle 优先于 Health；STOPPED/STARTING/STOPPING/FAILED/DEGRADED/HEALTHY/UNKNOWN 唯一映射；闭集证明（每个 Channel 必落到且仅落到 1 个值）|
| 4 | `§5` SQL | `state` 字符串 `'failed'` / `'degraded'` → **`'FAILED'` / `'DEGRADED'`**（与 Canonical Vocabulary 同步）|
| 5 | `§5` `health_tree_nodes` | 加 `UNIQUE (health_tree_id, node_path)` 约束（snapshot 内 node_path 唯一，避免聚合污染）|

#### C.23 结论

V0.2.4 + C.10 + ... + C.23 = **V0.2 Runtime Semantics = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE（最终）**。

```yaml
v0_2_runtime_semantics:
  status: CLOSED
  implementation_authority: THIS_DOCUMENT
  consistency_check: PASS
  implementation_ambiguity: NONE

  architecture:
    engines: 12
    cross_systems: 5
    cross_capabilities: 6
    principles: 22
    decisions: 57
    review_passes: 19
    patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3/4/5/6/7/8/9/10/11]

  health_model:
    runtime_fact:
      lifecycle: [STOPPED, STARTING, RUNNING, STOPPING]
      readiness: [NOT_READY, READY_TO_TAKE]
      health:    [HEALTHY, DEGRADED, FAILED, UNKNOWN]

    health_tree_sot: "§3.9"
    history:         "health_trees (fact table)"
    current_view:    "current_health_trees (latest snapshot per channel)"
    aggregation:     "channel_health_aggregation (§3.9 SoT)"
    presentation:    "channel_health_view (effective_channel_status)"

    effective_channel_status_policy:
      STOPPED:   { when: { lifecycle: STOPPED },         result: STOPPED }
      STARTING:  { when: { lifecycle_in: [STARTING, STOPPING] }, result: STARTING }
      FAILED:    { when: { health_tree_aggregation: FAILED },    result: FAILED }
      DEGRADED:  { when: { health_tree_aggregation: DEGRADED },  result: DEGRADED }
      HEALTHY:   { when: { lifecycle: RUNNING, health_tree_aggregation: HEALTHY }, result: HEALTHY }
      UNKNOWN:   { otherwise: UNKNOWN }

  switch_model:
    compiled_mode: channel_routes.switch_mode
    effective_mode: media_session_runtime.effective_switch_mode

  failure_model:
    operational_domains: 7
    diagnostic_classes: [PLAYER, UNKNOWN]

  architecture_changes_after_this: FORBIDDEN
  next_phase: [Phase 0.5, Phase 0.6]
  next_review_type: ACCEPTANCE_ONLY
```

**V0.2 Runtime Semantics Review 正式关闭 = CLOSED / implementation_ambiguity: NONE（最终）**。

**不再开第 20 轮架构 review**。下一步直接进入 Phase 0.5 + Phase 0.6。

---

### C.24 V0.2.4 Errata-12 — Health / Target / Vocabulary Final Closure（2026-08-24，20 轮 review 5 项最终收口）

V0.2.4 Errata-11 之后做"Schema/Target/Vocabulary 最终 closure"。发现 3 红 + 1 橙 + 1 验证项。**全部为正文收口**，不涉及架构发散。

| # | 位置 | 修正 |
|---|---|---|
| 1 | `§5` `channel_health_view` | 加 source 链注释：`runtime_fact (msr) + health_tree_aggregation (cha) → effective_channel_status_policy → channel_health_view`；C.24 锁定为"Channel Health 派生展示 SoT"；CASE + LEFT JOIN 真正执行 Policy（不绕过） |
| 2 | `§3.4` 表格 | **再次确认** Policy / Target 列已无绝对 target；唯一来源 = `hot_standby_levels.target_failover_time_ms`（关联）+ `failover_benchmarks` 验证；C.24 显式声明 §3.4 不再含 `<100ms` / `0.5-2s` / `1-3s` |
| 3 | `§3.9` 例子 + 算法 | **算法 vs 例子对齐**：required_node = Active Service Path；runtime 维护 `node_role (active/standby/offline)`；example 2 `Primary+Backup FAILED` → **`Channel=FAILED`**（Source 全部候选 offline，subsystem required_node=FAILED）；offline 失败 = 系统已吸收；standby 失败 = 失去 failover → DEGRADED |
| 4 | 附录 A 术语表 | **`Failure Domain` → `OperationalFailureDomain`**（UPPER_CASE）+ 显式 `DiagnosticFailureClass` 独立条目 + **`FailureDomain` alias 标记为 ❌ 废除**；"Failure Domain Matrix" 保留为"策略/章节名" |
| 5 | `§5` `channel_health_view` 重复检查 | C.24 验证：当前文档**仅 1 处** `CREATE VIEW channel_health_view AS`（line 1643）；其余两处为"指代注释"（line 1594 注释 + line 1641 章节注释），不构成重复定义 |

#### C.24 结论

V0.2.4 + C.10 + ... + C.24 = **V0.2 Runtime Semantics = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE（最终，**不可再开架构 review**）**。

```yaml
v0_2_runtime_semantics:
  status: CLOSED
  implementation_authority: THIS_DOCUMENT
  consistency_check: PASS
  implementation_ambiguity: NONE
  runtime_semantics_review: CLOSED

  architecture:
    engines: 12
    cross_systems: 5
    cross_capabilities: 6
    principles: 22
    decisions: 57                       # Errata-9 +4 (#54-57)
    review_passes: 20
    patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3/4/5/6/7/8/9/10/11/12]

  health_model:
    runtime_fact:
      lifecycle: [STOPPED, STARTING, RUNNING, STOPPING]
      readiness: [NOT_READY, READY_TO_TAKE]
      health:    [HEALTHY, DEGRADED, FAILED, UNKNOWN]

    health_tree_sot: "§3.9 Health Tree Aggregation Policy"
    history:         "health_trees (fact table)"
    current_view:    "current_health_trees (latest snapshot per channel)"
    aggregation:     "channel_health_aggregation (§3.9 SoT)"
    presentation:    "channel_health_view (effective_channel_status)"
    presentation_source:
      runtime:     "media_session_runtime"
      aggregation: "channel_health_aggregation"
      policy:      "effective_channel_status_policy (precedence + rules)"
    active_service_path_semantic:
      required_node: "currently on the active service path"
      node_role:     [active, standby, offline]   # V0.2.4 Errata-12 runtime 维护
      offline_absorption: "系统已吸收 failed，Channel Health 不变"
      standby_loss:       "失去 failover 候选 → Channel DEGRADED"

    effective_channel_status_policy:
      STOPPED:   { when: { lifecycle: STOPPED },         result: STOPPED }
      STARTING:  { when: { lifecycle_in: [STARTING, STOPPING] }, result: STARTING }
      FAILED:    { when: { health_tree_aggregation: FAILED },    result: FAILED }
      DEGRADED:  { when: { health_tree_aggregation: DEGRADED },  result: DEGRADED }
      HEALTHY:   { when: { lifecycle: RUNNING, health_tree_aggregation: HEALTHY }, result: HEALTHY }
      UNKNOWN:   { otherwise: UNKNOWN }

  switch_model:
    compiled_mode: channel_routes.switch_mode
    effective_mode: media_session_runtime.effective_switch_mode
    target_sot:    "hot_standby_levels.target_failover_time_ms (Policy/Target, NOT 协议保证)"
    measurement:   "failover_benchmarks (Runtime Measurement, p50/p95/p99)"

  failure_model:
    operational_domains: 7          # SOURCE / PIPELINE / MASTER / OUTPUT / RECORDING / CLOCK / RESOURCE
    diagnostic_classes: [PLAYER, UNKNOWN]
    failure_domain_alias:
      FailureDomain: "❌ 废除（V0.2.4 Errata-10 + Errata-12）"
      replacement:   "OperationalFailureDomain + DiagnosticFailureClass"

  architecture_changes_after_this: FORBIDDEN
  next_phase: [Phase 0.5, Phase 0.6]
  next_review_type: ACCEPTANCE_ONLY
  v0_2_5: FORBIDDEN
  next_architecture_version_requires: V0.3 process
```

**V0.2 Runtime Semantics Review 最终关闭 = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE / 不再开 V0.2.5**。

**不再开任何 V0.2.x 架构 review**。下一步直接进入 **Phase 0.5（Operator Workflow + 9 Low-Fi Wireframe）+ Phase 0.6（Reference A1/A2/B + 5 Fault Injection）** = Executable Acceptance Specification。

---

### C.25 V0.2.4 Errata-13 — Runtime Health Schema Micro-Closure（2026-08-24，21 轮 review 5 项 Schema 焊死）

V0.2.4 Errata-12 之后做"Schema/SoT 真正可执行"微焊。发现 3 红 + 2 橙。**全部为 Schema/正文收口**，不涉及架构发散。

| # | 位置 | 修正 |
|---|---|---|
| 1 | `§5` `health_tree_nodes` Schema | **加 `node_role ENUM('ACTIVE','STANDBY','OFFLINE') NOT NULL`**（§3.9 三种语义进 DB）；**加 `health_tree_node_role_invariant`**（ACTIVE→required_node=TRUE；STANDBY/OFFLINE→required_node=FALSE；CHECK 约束 + 应用层双层保证）|
| 2 | `§5` `channel_health_aggregation` SQL | **重写 SQL** 用 `node_role` 替代 `required_node BOOLEAN` 聚合：ACTIVE+FAILED→FAILED；ACTIVE+DEGRADED→DEGRADED；STANDBY+(DEGRADED\|FAILED)→DEGRADED；OFFLINE 不参与；无异常→HEALTHY；其余→UNKNOWN |
| 3 | `§5` `channel_health_view` | **去掉 `msr.health = 'HEALTHY'` 判定**（effective_channel_status 链路彻底干净）；msr.health 仅作 UI 下钻 / 诊断详情；SoT 链 = lifecycle (Runtime) → health_tree_aggregation (§3.9) → effective_channel_status |
| 4 | `§3.1` Data Plane | `resource_cost` → `descriptive_resource_class`（scheduling_input: false；canonical_scheduler_model = §3.11 Resource Vector；banned_term: "resource_cost"）|
| 5 | `§8.11` 对外 status 残留 | `channel_routes.status` 旧表述 → **`channel_health_view.effective_channel_status`**（V0.2.4 Errata-13 锁定 Channel 对外 status 唯一入口）|

#### C.25 结论

V0.2.4 + C.10 + ... + C.25 = **V0.2 Runtime Semantics = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE（最终，**全部 9 大 Runtime 域 + 3 项 Schema 焊死**）**。

```yaml
v0_2_runtime_semantics:
  status: CLOSED
  implementation_authority: THIS_DOCUMENT
  consistency_check: PASS
  implementation_ambiguity: NONE
  runtime_semantics_review: CLOSED

  architecture:
    engines: 12
    cross_systems: 5
    cross_capabilities: 6
    principles: 22
    decisions: 57                       # Errata-9 +4 (#54-57)
    review_passes: 21
    patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3/4/5/6/7/8/9/10/11/12/13]

  # C.25 关键新增：runtime 域全 CLOSED
  runtime:
    lifecycle:              CLOSED    # STOPPED / STARTING / RUNNING / STOPPING
    readiness:              CLOSED    # NOT_READY / READY_TO_TAKE
    health:                 CLOSED    # HEALTHY / DEGRADED / FAILED / UNKNOWN
    switch_mode:            CLOSED    # PACKET / FRAME / MASTER
    standby_semantics:      CLOSED    # COLD / WARM / HOT (Policy/Target, NOT runtime state)
    failure_domains:        CLOSED    # OperationalFailureDomain (7) + DiagnosticFailureClass (2)
    health_tree:            CLOSED    # §3.9 Health Tree Aggregation Policy SoT
    channel_status:         CLOSED    # §5 effective_channel_status_policy + channel_health_view

  # C.25 关键新增：Schema 全焊死
  schema:
    health_tree_role:       CLOSED    # node_role ENUM + invariant
    aggregation_sql:        CLOSED    # channel_health_aggregation 真执行 ACTIVE/STANDBY/OFFLINE
    effective_channel_status: CLOSED  # channel_health_view 真执行 policy（去掉 msr.health 干扰）

  health_model:
    runtime_fact:
      lifecycle: [STOPPED, STARTING, RUNNING, STOPPING]
      readiness: [NOT_READY, READY_TO_TAKE]
      health:    [HEALTHY, DEGRADED, FAILED, UNKNOWN]

    health_tree_sot: "§3.9 Health Tree Aggregation Policy"
    history:         "health_trees (fact table)"
    current_view:    "current_health_trees (latest snapshot per channel)"
    aggregation:     "channel_health_aggregation (§3.9 SoT, ACTIVE/STANDBY/OFFLINE 语义)"
    presentation:    "channel_health_view (effective_channel_status, policy 真执行)"
    presentation_source:
      runtime:     "media_session_runtime (lifecycle/readiness/health 纯下钻)"
      aggregation: "channel_health_aggregation"
      policy:      "effective_channel_status_policy (precedence + rules)"
    active_service_path_semantic:
      node_role:     [ACTIVE, STANDBY, OFFLINE]   # 锁进 DB
      invariant:     "ACTIVE→required_node=TRUE; STANDBY/OFFLINE→required_node=FALSE"
      offline_absorption: "系统已吸收 failed，Channel Health 不变"
      standby_loss:       "失去 failover 候选 → Channel DEGRADED"

    effective_channel_status_policy:
      STOPPED:   { when: { lifecycle: STOPPED },         result: STOPPED }
      STARTING:  { when: { lifecycle_in: [STARTING, STOPPING] }, result: STARTING }
      FAILED:    { when: { health_tree_aggregation: FAILED },    result: FAILED }
      DEGRADED:  { when: { health_tree_aggregation: DEGRADED },  result: DEGRADED }
      HEALTHY:   { when: { lifecycle: RUNNING, health_tree_aggregation: HEALTHY }, result: HEALTHY }
      UNKNOWN:   { otherwise: UNKNOWN }

  switch_model:
    compiled_mode: channel_routes.switch_mode
    effective_mode: media_session_runtime.effective_switch_mode
    target_sot:    "hot_standby_levels.target_failover_time_ms (Policy/Target, NOT 协议保证)"
    measurement:   "failover_benchmarks (Runtime Measurement, p50/p95/p99)"

  failure_model:
    operational_domains: 7          # SOURCE / PIPELINE / MASTER / OUTPUT / RECORDING / CLOCK / RESOURCE
    diagnostic_classes: [PLAYER, UNKNOWN]
    failure_domain_alias:
      FailureDomain: "❌ 废除（V0.2.4 Errata-10 + Errata-12）"
      replacement:   "OperationalFailureDomain + DiagnosticFailureClass"

  resource_model:
    canonical: "§3.11 9-dim Quantitative Resource Vector + Device/Port Token Constraints"
    descriptive_only: "descriptive_resource_class (LOW/MEDIUM/HIGH/VARIABLE/ZERO/ZERO_OR_LOW)"
    banned_term: "resource_cost"   # V0.2.4 Errata-13 废除；防止 Scheduler 误用

  architecture_changes_after_this: FORBIDDEN
  next_phase: [Phase 0.5, Phase 0.6]
  next_review_type: ACCEPTANCE_ONLY
  v0_2_5: FORBIDDEN
  next_architecture_version_requires: V0.3 process
```

**V0.2 Runtime Semantics Review 最终关闭 = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE / 9 大 Runtime 域 CLOSED / 3 项 Schema 焊死 / 不再开 V0.2.5**。

**不再开任何 V0.2.x 架构 review**。下一步直接进入 **Phase 0.5（Operator Workflow + 9 Low-Fi Wireframe）+ Phase 0.6（Reference A1/A2/B + 5 Fault Injection）** = Executable Acceptance Specification。

---

### C.26 V0.2.4 Errata-14 — Health Tree Source RG Schema & UNKNOWN Bug Fix（2026-08-24，22 轮 review 4 项 Schema 焊死 + 2 项同步）

V0.2.4 Errata-13 之后做"Health Tree 真正可执行 Schema 收口"。发现 1 真 correctness bug + 1 邻接 bug + 2 schema 收口 + 2 文档同步。**全部为 Schema/正文收口**，不涉及架构发散。

| # | 位置 | 修正 |
|---|---|---|
| 1 | `§5` `health_tree_nodes` Schema | **加 `subsystem ENUM('SOURCE','SWITCHER','COMPOSITION','AUDIO','MASTER','OUTPUT','RECORDING','CLOCK','RESOURCE') NOT NULL`** + **`redundancy_group_id UUID NULL`**；OFFLINE 节点必须保持原 redundancy_group_id（不能"丢"原组）|
| 2 | `§5` `channel_health_aggregation` SQL | **重写 7 规则**（5 旧 + 2 新）：新增 **Rule 3**「Source RG 全部候选 OFFLINE/FAILED/UNKNOWN → FAILED」（修 Primary=OFFLINE+FAILED, Backup=OFFLINE+FAILED 错得 HEALTHY 的 correctness bug）；新增 **Rule 4**「Source RG 无 ACTIVE 节点 → DEGRADED（pending takeover）」（修 HA-02 验收用例 "未接管时 DEGRADED"）|
| 3 | `§5` `channel_health_aggregation` SQL | **Rule 6 修订**：`HEALTHY` 条件加严——必须"至少一个 ACTIVE+HEALTHY 存在"+"无 ACTIVE/STANDBY 在 DEGRADED/FAILED/**UNKNOWN**"；UNKNOWN 不得被静默吸收为 HEALTHY |
| 4 | Canonical Vocabulary | **明确 `node_role = SoT`, `required_node = derived snapshot`**（Runtime 只能写 node_role；required_node 由 trigger/应用层从 node_role 派生；CHECK 约束双层保证）；**HealthState semantic_role = RUNTIME_NODE_HEALTH_FACT**；**EffectiveChannelStatus semantic_role = CHANNEL_PRESENTATION_STATUS**；禁止 `channel.health = session.health`（V0.2.4 Errata-14 锁定）|
| 5 | 终态块同步 | "9 大 Runtime 域"实际只有 8 项；**V0.2.4 Errata-14 正式把 Clock 纳入第 9 Runtime Domain**（`clock: CLOSED`，§3.12 Clock Reference / Clock Quality / CLOCK_DEGRADED / CLOCK_FAILED） |
| 6 | 终态块同步 | "3 项 Schema 焊死"统计错误；**V0.2.4 Errata-14 明确拆分：`3 Schema + 2 Semantic Cleanup = 5 项`**（Errata-13 实际条目数；Errata-14 又增 1 correctness bug fix + 1 UNKNOWN 修复） |

#### 7 个 Health Invariants（V0.2.4 Errata-14 锁定，Phase 0.6 测试断言）

```yaml
health_invariants:
  H1: { condition: "node_role=ACTIVE AND state=FAILED",           channel_result: FAILED }
  H2: { condition: "node_role=ACTIVE AND state=DEGRADED",         channel_result: DEGRADED }
  H3: { condition: "node_role=STANDBY AND state=FAILED",          channel_result: DEGRADED }
  H4: { condition: "node_role=STANDBY AND state=DEGRADED",        channel_result: DEGRADED }
  H5: { condition: "node_role=OFFLINE AND state=FAILED",          channel_result: "NO_DIRECT_CHANNEL_DEGRADATION" }
  H6: { condition: "source redundancy group has no usable candidate (all OFFLINE/FAILED/UNKNOWN)",
       channel_result: FAILED }
  H7: { rule: "effective_channel_status MUST be read from channel_health_view",
       enforcement: "Schema/Code 双向，禁止 channel.health = session.health" }
```

#### C.26 结论

V0.2.4 + C.10 + ... + C.26 = **V0.2 Runtime Semantics = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE（最终，**全部 9 大 Runtime 域 CLOSED + 3 Schema 焊死 + 2 Semantic Cleanup + 7 Health Invariants**）**。

```yaml
v0_2_runtime_semantics:
  status: CLOSED
  implementation_authority: THIS_DOCUMENT
  consistency_check: PASS
  implementation_ambiguity: NONE
  runtime_semantics_review: CLOSED

  architecture:
    engines: 12
    cross_systems: 5
    cross_capabilities: 6
    principles: 22
    decisions: 57
    review_passes: 22
    patches: [V0.2.3 patch 1, V0.2.4 patch 2, Cleanup-1/2/3, Errata-1/2/3/4/5/6/7/8/9/10/11/12/13/14]

  # V0.2.4 Errata-14 修复：完整 9 项 Runtime 域
  runtime:
    lifecycle:              CLOSED
    readiness:              CLOSED
    health:                 CLOSED
    switch_mode:            CLOSED
    standby_semantics:      CLOSED
    failure_domains:        CLOSED
    health_tree:            CLOSED
    channel_status:         CLOSED
    clock:                  CLOSED   # V0.2.4 Errata-14 新增（§3.12 Clock Reference / Clock Quality / CLOCK_DEGRADED / CLOCK_FAILED）

  # V0.2.4 Errata-14 修复：Errata-13 真实条目拆分为 3 Schema + 2 Semantic Cleanup
  schema:                  # 3 项 Schema 焊死
    health_tree_role:       CLOSED    # node_role ENUM + invariant + node_role=SoT
    aggregation_sql:        CLOSED    # channel_health_aggregation 7 规则（真执行 ACTIVE/STANDBY/OFFLINE + Source RG all unavailable）
    effective_channel_status: CLOSED  # channel_health_view 真执行 policy（去 msr.health 干扰）
  semantic_cleanup:        # 2 项 Runtime/Text Semantic Cleanup
    descriptive_resource_class: CLOSED  # resource_cost → descriptive_resource_class（scheduling_input: false）
    channel_status_source: CLOSED      # channel_routes.status → channel_health_view.effective_channel_status

  health_model:
    runtime_fact:
      lifecycle: [STOPPED, STARTING, RUNNING, STOPPING]
      readiness: [NOT_READY, READY_TO_TAKE]
      health:    [HEALTHY, DEGRADED, FAILED, UNKNOWN]

    health_tree_sot: "§3.9 Health Tree Aggregation Policy"
    history:         "health_trees (fact table)"
    current_view:    "current_health_trees (latest snapshot per channel)"
    aggregation:     "channel_health_aggregation (§3.9 SoT, 7 规则真执行 ACTIVE/STANDBY/OFFLINE/Subsystem/RG)"
    presentation:    "channel_health_view (effective_channel_status, policy 真执行)"
    presentation_source:
      runtime:     "media_session_runtime (lifecycle/readiness/health 纯下钻)"
      aggregation: "channel_health_aggregation"
      policy:      "effective_channel_status_policy (precedence + rules)"
    active_service_path_semantic:
      node_role:     [ACTIVE, STANDBY, OFFLINE]   # 锁进 DB
      subsystem:     [SOURCE, SWITCHER, COMPOSITION, AUDIO, MASTER, OUTPUT, RECORDING, CLOCK, RESOURCE]   # V0.2.4 Errata-14 增
      redundancy_group_id: "UUID NULL; 同一 RG 内候选构成冗余组"  # V0.2.4 Errata-14 增
      invariant:     "ACTIVE→required_node=TRUE; STANDBY/OFFLINE→required_node=FALSE"
      node_role_authority: { so_t: node_role, required_node: derived_snapshot }  # V0.2.4 Errata-14
      offline_absorption: "系统已吸收 failed，Channel Health 不变"
      standby_loss:       "失去 failover 候选 → Channel DEGRADED"
      source_rg_unavailable: "Source RG 全部候选 OFFLINE/FAILED/UNKNOWN → Channel FAILED"  # V0.2.4 Errata-14

    effective_channel_status_policy:
      STOPPED:   { when: { lifecycle: STOPPED },         result: STOPPED }
      STARTING:  { when: { lifecycle_in: [STARTING, STOPPING] }, result: STARTING }
      FAILED:    { when: { health_tree_aggregation: FAILED },    result: FAILED }
      DEGRADED:  { when: { health_tree_aggregation: DEGRADED },  result: DEGRADED }
      HEALTHY:   { when: { lifecycle: RUNNING, health_tree_aggregation: HEALTHY }, result: HEALTHY }
      UNKNOWN:   { otherwise: UNKNOWN }

  health_invariants:  # V0.2.4 Errata-14 锁定 7 条
    - H1: ACTIVE+FAILED → FAILED
    - H2: ACTIVE+DEGRADED → DEGRADED
    - H3: STANDBY+FAILED → DEGRADED
    - H4: STANDBY+DEGRADED → DEGRADED
    - H5: OFFLINE+FAILED → 系统已吸收（NO_DIRECT_CHANNEL_DEGRADATION）
    - H6: Source RG 全部候选不可用 → FAILED
    - H7: effective_channel_status MUST be read from channel_health_view

  switch_model:
    compiled_mode: channel_routes.switch_mode
    effective_mode: media_session_runtime.effective_switch_mode
    target_sot:    "hot_standby_levels.target_failover_time_ms (Policy/Target, NOT 协议保证)"
    measurement:   "failover_benchmarks (Runtime Measurement, p50/p95/p99)"

  failure_model:
    operational_domains: 7          # SOURCE / PIPELINE / MASTER / OUTPUT / RECORDING / CLOCK / RESOURCE
    diagnostic_classes: [PLAYER, UNKNOWN]
    failure_domain_alias:
      FailureDomain: "❌ 废除（V0.2.4 Errata-10 + Errata-12）"
      replacement:   "OperationalFailureDomain + DiagnosticFailureClass"

  resource_model:
    canonical: "§3.11 9-dim Quantitative Resource Vector + Device/Port Token Constraints"
    descriptive_only: "descriptive_resource_class (LOW/MEDIUM/HIGH/VARIABLE/ZERO/ZERO_OR_LOW)"
    banned_term: "resource_cost"   # V0.2.4 Errata-13 废除；防止 Scheduler 误用

  architecture_changes_after_this: FORBIDDEN
  next_phase: [Phase 0.5, Phase 0.6]
  next_review_type: ACCEPTANCE_ONLY
  v0_2_5: FORBIDDEN
  next_architecture_version_requires: V0.3 process
```

**V0.2 Runtime Semantics Review 最终关闭 = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE / 9 大 Runtime 域 CLOSED / 3 Schema 焊死 + 2 Semantic Cleanup + 7 Health Invariants / 不再开 V0.2.5**。

**不再开任何 V0.2.x 架构 review**。下一步直接进入 **Phase 0.5（Operator Workflow + 9 Low-Fi Wireframe）+ Phase 0.6（Reference A1/A2/B + 5 Fault Injection + 7 Health Invariants = Executable Acceptance Specification）**。

---

**文档状态**: V0.2.4 — Runtime Semantics Freeze + Patch 2 + Cleanup-1/2/3 + Errata-1/2/3/4/5/6/7/8/9/10/11/12/13/14
**Architecture Baseline**: **LOCK FINAL** (22 轮 review / **57 项决策** / 12 Engine / 22 原则 / 6 横切能力 / 一致性 PASS / 实现可唯一 / **V0.2 Runtime Semantics = CLOSED / IMPLEMENTATION AUTHORITY / implementation_ambiguity: NONE / 9 大 Runtime 域 CLOSED / 3 Schema 焊死 + 2 Semantic Cleanup / 7 Health Invariants**)
**下次评审**: Phase 0.5（Workflow + Low-Fi Wireframe）+ Phase 0.6（Reference A1/A2/B + Fault Injection + 7 Health Invariants）完成后 → V0.2 FINAL（acceptance only，不开架构 review）
**维护人**: 风从平地起

---

## 附录 D：Architecture Baseline LOCK FINAL 状态

**Architecture Baseline LOCK FINAL** 含义：

- 12 Engine + 5 横向系统 + 6 横切能力 + 22 原则 + **57 决策**，**完全自洽 + 可唯一实现**
- 12 + 5 + 6 + 22 + 57 **不再修改**
- 任何架构级扩展必须开 V0.3
- Phase 0.5 / 0.6 是 **acceptance validation**，不修改架构
- Phase 1+ 是 **implementation**，不修改架构
- **Phase 0.5 / 0.6 产物 = Executable Acceptance Specification**

**Errata-2 锁定的核心边界**：

```
9-dimensional Resource Vector
        +
Device/Port Token Constraints

Architecture Capability
        ≠
Runtime Hardware Snapshot

Estimated PCIe Payload
        ≠
Measured PCIe Bus Utilization

Capability Contract
        +
Runtime Alignment
        =
PACKET_SWITCH Eligibility
```

**所有 Lock 阶段锁定的实现级规范**：

- ✅ RAW_VIDEO 带宽典型值（UYVY422_1080p25 = 829.44 Mbps；V210_1080p25 = 1.106 Gbps）
- ✅ RAW_AUDIO 带宽典型值（PCM_24bit_48kHz_8ch = 1.152 Mbps）
- ✅ PCIe `pcie_*_mb_s` 是 SCHEDULING ESTIMATE ≠ 实测 PCIe bus utilization
- ✅ Resource Model = 9-dim Quantitative Vector + Device/Port Token Constraints
- ✅ Architecture ≠ current_host_snapshot
- ✅ SRS 不绑未来版本号（Stable Line / 6.x + SRSAdapter 解耦）
- ✅ latency_probes Schema：10 probe points + 4 measurement modes
- ✅ PACKET_SWITCH 拆 capability_contract（X6）+ runtime_alignment（live）
- ✅ Preflight 用完整 9 维 Resource Vector
- ✅ COLD/WARM/HOT 资源消耗统一 GRAPH_CALCULATED
- ✅ hot_standby_levels 表删 `resource_factor` 字段
- ✅ ChangeSet business outcome（status）+ transaction phase（events）分离

**禁止项**：

- ❌ 任何架构级修改（除非发现 V0.2 **安全/正确性**错误，必须走 V0.3 流程）
- ❌ 增加新 Engine
- ❌ 修改 Data Plane / Switch Mode / Hot-Standby / Program Master 等核心定义
- ❌ 修改 Failure Domain Matrix
- ❌ 改 11 个已实现 Source Adapter（RIST/Zixi/NDI 等 V0.3 才加）
- ❌ 把 `current_host_snapshot` 内容写进 Architecture
- ❌ 把 `pcie_*_mb_s` 当成实测值
- ❌ 在 hot_standby_levels lookup 表加回 `resource_factor`

**允许项**：

- ✅ Phase 0.5：Operator Workflow 文档、9 个 Low-Fi 线框、4 条关键操作链原型
- ✅ Phase 0.6：Reference A1（PACKET 基础，预对齐源）/ A2（SDI 主备走 FRAME/MASTER）/ B（异构源）+ 5 Fault Injection
- ✅ Phase 0.6：Executable Acceptance Specification 形式（GraphSpec → Compile → Runtime → Fault → 实测行为）

**Canonical Vocabulary**（TS / Rust / JSON Schema / PG enum 共享）：

```
DataPlaneLayer:        ELEMENTARY | CONTAINER | METADATA | CONTROL
ElementaryDataType:    COMPRESSED_VIDEO | COMPRESSED_AUDIO | RAW_VIDEO | RAW_AUDIO
SwitchMode:            PACKET_SWITCH | FRAME_SWITCH | MASTER_SWITCH
SwitchDecisionResult:  PACKET_SWITCH | FRAME_SWITCH | MASTER_SWITCH | REJECT   # V0.2.4 Errata-6 新增：REJECT ≠ SwitchMode
HotStandbyLevel:       COLD | WARM | HOT
CapabilityCheckResult: PASS | WARN | FAIL
HealthState:           HEALTHY | DEGRADED | FAILED | UNKNOWN
                       # V0.2.4 Errata-14 semantic_role: RUNTIME_NODE_HEALTH_FACT
                       # not_channel_status: true      # 不得直接当作 Channel Status
                       # 仅作为 media_session_runtime.health / health_tree_nodes.state 的事实值
                       # Channel 状态由 channel_health_view.effective_channel_status 表达
LifecycleState:        STOPPED | STARTING | RUNNING | STOPPING
ReadinessState:        NOT_READY | READY_TO_TAKE
OperationalFailureDomain: SOURCE | PIPELINE | MASTER | OUTPUT | RECORDING | CLOCK | RESOURCE
DiagnosticFailureClass:  PLAYER | UNKNOWN            # V0.2.4 Errata-9：分类，不是操作恢复域
EffectiveChannelStatus: HEALTHY | DEGRADED | FAILED | STARTING | STOPPED | UNKNOWN
                       # V0.2.4 Errata-14 semantic_role: CHANNEL_PRESENTATION_STATUS
                       # source_of_truth: channel_health_view
                       # ≠ HealthState（节点事实）≠ LifecycleState（生命周期）
                       # 禁止前端写：channel.health = session.health（V0.2.4 Errata-14 锁定）
RuntimeAlignmentAttr:  GOP_BOUNDARY | IDR_ALIGNMENT | TIMESTAMP_CONTINUITY
                       | PTS_CONTINUITY | DTS_CONTINUITY | AUDIO_CONTINUITY
E2EMeasurementMode:    STAGE_LATENCY | SYNCHRONIZED_CLOCK
                       | EMBEDDED_MEDIA_PROBE | APPROXIMATE
ChangeSetStatus:       DRAFT | VALIDATED | APPLIED | ROLLED_BACK
ChangeSetPhase:        PREPARING | APPLYING | COMMITTED | ABORTED
ResourceDimension:     CPU_THREADS | GPU_SESSIONS | VRAM_MB | RAM_MB
                       | INGRESS_MBPS | EGRESS_MBPS | DISK_WRITE_MBPS
                       | PCIE_RX_MB_S | PCIE_TX_MB_S   # scheduling estimate
DeviceToken:           BMD_INPUT_PORT | BMD_OUTPUT_PORT
DeviceConstraint:      DEVICE_EXCLUSIVITY

# 历史（V0.2.4 Errata-10 废除）：FailureDomain
#   = historical terminology
#   = replaced by OperationalFailureDomain
#   不要在 TS / Rust / JSON Schema / PG enum 中再创建同名 enum
```
