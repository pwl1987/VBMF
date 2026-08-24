# Phase 0.5.1 ERRATA — UI Semantics Closure 变更归档

> **Phase 0.5.1 — Stateful Operator UX Closure**
>
> 本文档归档 Phase 0.5.1 期间的 18 项 UI 语义 / 运营操作语义 / 状态语义修正。所有修改仅在 UI / Workflow 层面，**不修改 V0.2 架构**。
>
> 适用版本：VBMF V0.2 LOCK FINAL · Phase 0.5.1
> 关联文档：[`docs/phase-0.5/OPERATOR_WORKFLOW.md`](OPERATOR_WORKFLOW.md) · [`docs/phase-0.5/chains/`](chains/)

---

## 0. Phase 0.5.1 缘起

Phase 0.5 完成 9 Low-Fi Wireframes + 4 关键操作链后，从"24/7 广播机房操作员能否安全、快速、无歧义地使用"角度复审，发现 **18 个 UI/UX 语义缺口**，其中 1 项（SDI + PACKET_SWITCH 示例冲突）已经直接违反 V0.2 Runtime Semantics 锁定。

Phase 0.5.1 目标：只修 UI / Workflow / State / Interaction，不碰 V0.2 架构。

---

## 1. P0 — 必须修 / 5 项

### P0-1 · Graph Designer 08 · SDI + PACKET_SWITCH 错误示例 → 拆 Scenario

**问题：** Graph Designer 原示例 `Source.A SDI/dv0 + Source.B SDI/dv1 → PACKET_SWITCH`，但 V0.2 锁定 `SDI = RAW_VIDEO/RAW_AUDIO`，而 `PACKET_SWITCH` 要求 COMPRESSED 域 + Capability Contract 预对齐。**示例与架构直接冲突。**

**修复：**
- 新增 **Scenario 选择器**（PACKET / FRAME / MASTER / REJECT）
- DEFAULT Scenario = **FRAME**（最符合 SDI 主备真实场景）
- Scenario = PACKET 时：Source 强制 SRT/H.264-A + SRT/H.264-B
- Scenario = FRAME / MASTER 时：Source 为 SDI + Normalize + Encode 自动插入
- Inspector 顶部明示 `Scenario: FRAME_SWITCH (符合 V0.2 §3.4)`
- 新增 `[DESIGN] / [COMPILED] / [VALIDATION]` 三个 Tab
  - DESIGN：用户图
  - COMPILED：实际 Runtime Graph（含 [AUTO INSERTED] Normalize / Encode / SRS Adapter）
  - VALIDATION：Capability / Clock / Latency / Resource / Preflight

**新增 Edge Inspector：**
- Data Plane / Clock Domain / Latency Budget / Backpressure / Capability
- `[AUTO INSERTED NORMALIZE]` Reason 字段（RAW_VIDEO format mismatch 等）
- 视觉标识：`[AUTO INSERTED]` 虚线节点 / `[WARNING]` 黄 / `[REJECT]` 红

**文件：** `docs/phase-0.5/wireframes/08-graph-designer.html`

---

### P0-2 · Dashboard 01 · 缺 NOW/NEXT/CHANNEL STATUS + System State Bar + Operator Intent Layer

**问题：** 原 Dashboard 只有 PVW/PGM/Cut to/TAKE/queued，**操作员意图模型不明确**：
- CURRENT（现在播什么）= NOW
- NEXT（下一步准备）= NEXT
- TAKE（我要执行什么）= TAKE
- 三者不能混在一句话里

**修复：**
1. **System State Bar**（顶部 6 项）— 一眼看到关键态势
   - PGM / Backup / SRS / REC / Clock / Incidents
2. **三栏主区**：
   - NOW pane（左，红色边框）— 当前在播
   - NEXT pane（中，蓝色边框）— 下一步准备
   - Channel Status（右）— Lifecycle / Readiness / Health / Compiled / Effective / Hot-Standby
3. **Operator Intent Layer**（P2-1 已并入此项）— 6 步意图流：
   - NOW / NEXT / TAKE / SCHEDULED / SYSTEM DECIDES / EXECUTION RESULT
4. **Controls** — 单一主 TAKE 按钮 + 排播信息

**文件：** `docs/phase-0.5/wireframes/01-dashboard.html`

---

### P0-3 · Switcher 03 · TAKE 缺真实状态机 + L2 确认

**问题：** OPERATOR_WORKFLOW 锁定 `TAKE = L2`（3s 倒计时），但 Switcher HTML 只有 `<button>TAKE</button>`，**没有 5 状态机可视化**。

**修复：**
- **TAKE State Machine**（5 状态）：
  ```
  IDLE → REQUESTED → COUNTDOWN (3s) → EXECUTING → SWITCHED
  ```
- **L2 Confirm Modal**：
  ```
  ┌─────────────────────────────────┐
  │ Confirm TAKE                     │
  │                                 │
  │ NOW    Source.A                 │
  │ NEXT   Source.B                 │
  │ Mode   FRAME_SWITCH              │
  │ Target Public Service           │
  │                                 │
  │ Switching in 3...                │
  │                                 │
  │ [ CANCEL ]    [ CONFIRM ]        │
  └─────────────────────────────────┘
  ```
- **EXECUTING** 实时进度条
- **SWITCHED** 终态 + 自动回 IDLE
- 危险操作分层可视化：L1（无确认）/ L2（3s 倒计时）/ L3（YES + 5s）

**文件：** `docs/phase-0.5/wireframes/03-switcher.html`

---

### P0-4 · Composition 04 · 缺 24h Timeline 排播表

**问题：** Chain 3（Playout）要求 24h 排播表，但 Composition 页面只有图层编辑器，**Timeline / Playout 实际是另一工作域**。

**修复（不增加第 10 核心页面，双栏方案）：**
- **左栏 Timeline**（24h 排播表）：
  - NOW / NEXT 标线
  - 24h 时间轴（每小时刻度）
  - Playlist items（Asset 名 / 时长 / 类型）
  - Events（手动 TAKE / 自动切换）
- **右栏 Composition Preview**：
  - Program Composition / Variant Composition
  - Logo / Bug / Subtitle
  - RAW → Master → Variant 渲染管线
- **Director 工作域 = Timeline + Composition 同屏双栏**（符合 Chain 3）

**文件：** `docs/phase-0.5/wireframes/04-composition.html`

---

### P0-5 · Health Tree 09 · CSS 单位语法错误

**问题：** `font:13 px/1.5` / `padding:16 px` / `grid-template-columns:1 fr 1 fr`（带空格）— 非法 CSS 单位，**实际浏览器表现可能与设计稿不一致**。

**修复：**
- 全局替换 `13 px` → `13px`
- `16 px` → `16px`
- `1 fr 1 fr` → `1fr 1fr`
- 重写整页 CSS（顺便统一中英双语规范）

**文件：** `docs/phase-0.5/wireframes/09-health-tree.html`

---

## 2. P1 — 强烈建议 / 5 项

### P1-2 · Health Tree 09 · 双视图 + Aggregation Rules 视图

**问题：** 7 Health Invariants H1-H7 对工程师有价值，但对 Operator 占据视觉权重过大；节点缺可解释性（State/Reason/Detected/Duration/Impact/Auto Action/Next Action）。

**修复（重写整页）：**
1. **Operator View**（默认）：
   - Hero Card（Channel 名 + 状态 + 整体健康）
   - 9 Subsystem 卡（SOURCE / SWITCHER / COMPOSITION / AUDIO / MASTER / OUTPUT / RECORDING / CLOCK / RESOURCE）
   - 节点只显示：State / Reason / Operator Action
2. **Engineering View**：
   - 完整 Health Tree 层级
   - 节点全部字段（State / Reason / Detected / Duration / Impact / Auto Action / Current Recovery / Next Action / Operator Required）
3. **Aggregation Rules View**（§3.9 7 规则）：
   - Rule 1-7 SQL 实现
   - HA-01..HA-07 验收用例
   - Health Invariants H1-H7

**文件：** `docs/phase-0.5/wireframes/09-health-tree.html`

---

### P1-3 · Audio 05 · 缺广播级安全态势

**问题：** Audio 页面有 8 通道 fader / meter / delay，但缺 §3.13 AVSync Manager 真正接得上的"安全区"：Current LUFS / Target LUFS / True Peak / L/R Balance / Mute / Phase / AV Offset / Drift。

**修复：**
- 右侧新增 **PROGRAM AUDIO 广播安全区**：
  - LUFS-I / Target -23 LUFS / Short-term / True Peak (dBTP)
  - AV Offset / Drift (ms/min)
  - 3 个 OK 状态：Loudness OK / Peak OK / Sync OK
- 与 §3.13 AVSync Manager 真正接上
- 颜色：全绿 = 安全 / 任一黄/红 = 状态降级

**文件：** `docs/phase-0.5/wireframes/05-audio.html`

---

### P1-4 · Output 06 · 3 视图 + Output Variant + Latency Probe

**问题：** 原 Output 页面只列 URL / bitrate / clients，缺 Output Variant、Encoder Profile、PTS Continuity、Segment Latency、Playlist Age、Reconnect Count 等 §3.6 Latency Probe 维度。

**修复：**
1. **List View**（默认）：所有 Output Variant 总览
2. **HLS Detail**：Encoder / Bitrate / Segment Duration / Playlist Age / PTS Continuity / Reconnect
3. **WebRTC Detail**：ICE / DTLS / SRTP / RTT / Jitter / Packet Loss / Bitrate

**文件：** `docs/phase-0.5/wireframes/06-output.html`

---

### P1-5 · Recording 07 · Incident → Replay 自动定位工作流

**问题：** 故障后真正的工作是"按 Incident 自动定位录像窗口"（前 30s + 事故 + 后 60s），但原 Recording 页面只列 Chunk + Incident 列表，**没有"按事故定位"的工作流**。

**修复（新增完整 Replay Workspace）：**
1. **顶部 4 步工作流指示器**：
   ```
   STEP 1 Incident → STEP 2 Auto-Locate → STEP 3 Select Stream → STEP 4 Replay
   ```
2. **Replay Workspace**（核心新增）：
   - 选中 Incident 详情（ID / 类型 / 发生时间 / 处置 / 影响 / 确认人）
   - Stream 选择（Program / Source.A / Source.B / Audio / Master）
   - **Replay Window 时间轴**（-30s ~ +60s）：
     - 红色竖线 = 故障时刻
     - 蓝色窗口 = 回放范围
     - 秒刻度
   - 控制：▶ 播放 / ⏸ 暂停 / 跳到事故 / -5s / +5s
   - 导出 / 复制链接
3. **Recent Incidents 列表**：每条 Incident 都有 "→ 跳到 Replay" 链接（X4 Timeline 双向链接）

**文件：** `docs/phase-0.5/wireframes/07-recording.html`

---

### P1-6 · Sources 02 · Clock Reference 完整呈现

**问题：** 原 Sources 页面 Clock 列只显示 "PTP" / "TIMECODE" / "MONOTONIC" / "—" 4 个值，**没有 LOCKED / BROADCAST_GRADE / Fallback 链**。V0.2 §3.12 把 Clock 提升到 runtime correctness 层后，UI 信息量严重不足。

**修复（顶部新增 Clock Reference Hero 区）：**
1. **LOCKED 正常态**：
   - 大字 LOCKED + 绿色边框
   - Reference: PTP · ptp0 (eth0)
   - Quality: BROADCAST_GRADE
   - Offset / Drift / Path Delay / Mode
   - **Fallback Chain**（4 节点）：PTP (active) → TIMECODE → SYSTEM → MONOTONIC
2. **DEGRADED 降级态**（折叠在 details 内）：
   - PTP lost → TIMECODE active
   - 黄色边框 + CLOCK_DEGRADED 事件
3. **Clock Quality 列**新增到 Source 表格：BROADCAST / GOOD / FAIR / POOR / N/A 5 档
4. 新增 Clock Quality badge 颜色：绿/绿/黄/红/灰

**文件：** `docs/phase-0.5/wireframes/02-sources.html`

---

## 3. P2 — 锦上添花 / 2 项

### P2-1 · Operator Intent Layer（已并入 P0-2）

**原 P2-1** 单独提出；**实施中并入 P0-2 Dashboard 修复**，作为 Operator Intent Layer 6 步流：
- NOW / NEXT / TAKE / SCHEDULED / SYSTEM DECIDES / EXECUTION RESULT

**理由：** Operator Intent 与 System State Bar 同属 Dashboard 顶层信息架构，**分开会让 Dashboard 出现两个不同的"上层"**，违反单一职责。

---

### P2-2 · 10 状态总览页（新增 10/10 页）

**问题：** Phase 0.5 各页面只有"正常态"，但 24/7 广播真正重要的是 **状态变化后 UI 能否正确表达**。V0.2 锁定的三轴 Runtime State 至少 10 种典型组合。

**修复（新建 `10-states.html`）：**
- 顶部：三轴说明 + ECHS 说明
- Legend：Lifecycle / Readiness / Health / → ECHS 4 行
- 10 张 State Card（2x5 网格）：
  1. **NORMAL** — RUNNING / READY_TO_TAKE / HEALTHY
  2. **STARTING** — STARTING / NOT_READY / UNKNOWN
  3. **BACKUP NOT READY** — RUNNING / NOT_READY / HEALTHY
  4. **DEGRADED** — RUNNING / READY / DEGRADED
  5. **FAILED** — RUNNING / NOT_READY / FAILED
  6. **FAILOVER (mid-switch)** — RUNNING / READY / HEALTHY · SWITCHING
  7. **RECOVERY (post-failover)** — RUNNING / NOT_READY / DEGRADED
  8. **STOPPING** — STOPPING / NOT_READY / UNKNOWN
  9. **STOPPED** — STOPPED / NOT_READY / UNKNOWN
  10. **UNKNOWN** — 三轴全 UNKNOWN

每张卡显示：编号 / 名称 / 描述 / 三轴 / Compiled+Effective / Channel Health View / 典型场景

**文件：** `docs/phase-0.5/wireframes/10-states.html`（新文件）

---

## 4. 总览

| 优先级 | 数量 | 状态 |
|---|---|---|
| P0 必须修 | 5 项 | ✅ 完成 |
| P1 强烈建议 | 5 项（P1-1 跳号） | ✅ 完成 |
| P2 锦上添花 | 2 项 | ✅ 完成 |
| **合计** | **12 项修复** | ✅ **Phase 0.5.1 First Round CLOSED** |

> **注：** 原始清单 18 个 issues 来自 4 月初 Phase 0.5 完成后第一次 UI Semantics Review；其中 5 项与已有 5 个 wireframes 改动合并（如 P1-1 合并到 P0-1 Graph Designer 的 [COMPILED] tab），最终落地为 12 项独立修改。

---

# Phase 0.5.1 Final Closure — 8 项收口

> 第二轮复审（22 轮 review 完成后深度复审）发现 4 P0 硬语义错误 + 3 P1 文档/口径 + 1 健康树对账。
> 修完后 Phase 0.5 → **LOCK FINAL**，不再做 UI 语义设计变更。

## P0 必须修（4 项）

### FC-P0-1 · Graph Designer Edge e-001 · FRAME_SWITCH 输出 Data Plane 错误

**问题：** `Switcher → 视频主节点` Edge Inspector 显示 `Data Plane = COMPRESSED_VIDEO` + `Codec = H.264 High@L4.0`，但 V0.2 §3.4 / §3.7.1 锁定 FRAME_SWITCH 在 RAW 域工作（input/output = RAW_*）。这给工程师错误暗示。

**修复：**
- Data Plane: `COMPRESSED_VIDEO` → **`RAW_VIDEO`**
- Codec: `H.264 High@L4.0` → **`N/A (RAW 域, 编码在 Master Join 之后)`**
- Res: `1920×1080 / 25` → `1920×1080 / 25 (RAW frame)`
- 目标节点明确加 `(AUTO)` 标识：`视频主节点 (AUTO)`

**文件：** `docs/phase-0.5/wireframes/08-graph-designer.html`

---

### FC-P0-2 · Health Tree Operator View · CH01 HEALTHY + SRS DEGRADED 矛盾

**问题：** Operator View 顶部 CH01 = HEALTHY，但下面 "Output SRS = DEGRADED (WebRTC drift +15ms)"。按 §3.9 Rule 2 ACTIVE+DEGRADED → channel DEGRADED，**两者不能同时成立**。

**修复：**
1. **正常态修复**：Output SRS 改为 HEALTHY（移除 "WebRTC drift +15ms"）
2. **新增 DEGRADED 示例**（折叠在 details）：
   - CH01 = DEGRADED（黄色边框）
   - Output SRS = DEGRADED
   - 明确 Aggregation Rule 2 触发
3. **SDI 主输出 (V0.4)** 移到独立折叠区，标注 "不在 9 Subsystem 计数"

**文件：** `docs/phase-0.5/wireframes/09-health-tree.html`

---

### FC-P0-3 · 10-states FAILOVER · "SWITCHING" 非 Canonical Enum

**问题：** FAILOVER state card 写 `Effective = SWITCHING...`，但 V0.2 Canonical Vocabulary 锁死 `SwitchMode = PACKET/FRAME/MASTER`、`EffectiveChannelStatus = HEALTHY/DEGRADED/FAILED/STARTING/STOPPED/UNKNOWN`、`Lifecycle = STOPPED/STARTING/RUNNING/STOPPING`。**"SWITCHING" 不属于任何 Canonical Enum**。

**修复：**
- 删除 `Effective = SWITCHING...`
- 拆为两个 Canonical 字段：
  - `effective_switch_mode = FRAME_SWITCH`
  - `switch_execution_state = EXECUTING`
- Scenario 注明 "SWITCHING 不是 Canonical Enum (V0.2 已锁死)"

**文件：** `docs/phase-0.5/wireframes/10-states.html`

---

### FC-P0-4 · 10-states STOPPING · Channel Health View = STOPPED 违反 Policy

**问题：** STOPPING state card 写 `Channel Health View → ● STOPPED (过渡)`，但 V0.2 Errata-11/12 锁死 `effective_channel_status_policy: lifecycle_terminal > lifecycle_transition > health_tree_aggregation > unknown`，**STARTING/STOPPING 都映射为 STARTING**。

**修复：**
- Channel Health View: `STOPPED` → **`STARTING (Policy: STARTING/STOPPING → STARTING)`**
- Scenario 注明 "ECHS=STARTING 是 presentation policy 锁定, 不是 lifecycle 状态"
- 删除 "持续 2-5s" 固定范围

**文件：** `docs/phase-0.5/wireframes/10-states.html`

---

## P1 强烈建议（3 项）

### FC-P1-1 · Dashboard · 87ms 不能叫 Target

**问题：** Dashboard NEXT pane 写 "目标切换 ~87ms"，Operator Intent Layer 写 "FRAME_SWITCH (87ms 预算)"。但 V0.2 锁死：
- `target_failover_time_ms` = Policy / Target
- `failover_benchmarks` = Measured p50/p95/p99

**修复：**
- "目标切换 ~87ms" → "Target 100ms · Last measured p95 87ms (failover_benchmarks)"
- "87ms 预算" → "Target 100ms, last p95 87ms"

**文件：** `docs/phase-0.5/wireframes/01-dashboard.html`

---

### FC-P1-2 · 10-states · 删除固定实测范围

**问题：** FAILOVER state card 写 "持续 87ms ~ 2s"，NOT_READY state card 写 "接管慢 1-2s"。V0.2 反复锁死 **禁止固定实测范围**。

**修复：**
- FAILOVER: "持续 87ms ~ 2s" → "实测由 failover_benchmarks 记录（不是固定范围）"
- NOT_READY: "接管慢 1-2s" → "由 failover_benchmarks 测量, 不是固定值"
- STOPPING: "持续 2-5s" → "实测由 benchmark 测量, 不写固定范围"

**文件：** `docs/phase-0.5/wireframes/10-states.html`

---

### FC-P1-3 · 文档统一 · 9 Core Pages + 1 Validation Page = 10 artifacts

**问题：** `OPERATOR_WORKFLOW.md` / `INDEX.md` / `README.md` / `10-states.html` 在 "9 页 vs 10 页" 上有口径漂移。

**修复：** 锁定表述模型：
```
9 Core Operational Pages (01-09) = 正式产品工作域
+ 1 Validation / State Reference Page (10-states) = Phase 0.5 验收辅助
= 10 HTML artifacts
```

**修改文件：**
- `docs/phase-0.5/OPERATOR_WORKFLOW.md` — §3 标题 + 验收清单
- `docs/phase-0.5/INDEX.md` — 范围段 + 目录 + 页面清单表
- `docs/phase-0.5/README.md` — 范围段 + 页面清单表
- `docs/phase-0.5/wireframes/10-states.html` — title + header + footer 三处加 "1 Validation Page" 标识
- `README.md` (顶层) — 核心能力表 / 演进历史 / 文档结构 / 中英 Summary / Current phase / badge 全部更新

---

## 非架构对账（1 项）

### FC-DOC-1 · Health Tree Operator View 9 Subsystem 对齐

**问题：** ERRATA 规范明确 9 Subsystem（SOURCE / SWITCHER / COMPOSITION / AUDIO / MASTER / OUTPUT / RECORDING / CLOCK / RESOURCE），但实际 Operator View 只显示 7 个（缺 COMPOSITION / AUDIO / RESOURCE）。

**修复：** 采用方案 A（推荐）—— 实际补齐 9 Subsystem：
1. SOURCE
2. SWITCHER
3. COMPOSITION
4. AUDIO MIXER
5. PROGRAM MASTER
6. OUTPUT (SRS)
7. RECORDING
8. CLOCK
9. RESOURCE

+ 隐藏折叠：DEGRADED 示例 + SDI 主输出 (V0.4 目标)

**文件：** `docs/phase-0.5/wireframes/09-health-tree.html`

---

## 关联 Chain 文档（1 项）

### FC-CHAIN-1 · chain-2-failure.md · Filler 固定阈值改为 Policy

**问题：** Chain 2 流程图写 "Filler 兜底（If 切换 > 1s）"，步骤表写 "切换 > 1s → Filler"。**固定 1s 阈值**不应作为架构规范。

**修复：**
- 流程图："If 切换 > 1s" → "按 §8.9 Safety Policy"
- 步骤表："切换 > 1s → Filler" → "按 §8.9 Safety Policy (不写固定阈值)"

**文件：** `docs/phase-0.5/chains/chain-2-failure.md`

---

## Final Closure 总览

| 优先级 | 数量 | 状态 |
|---|---|---|
| FC-P0 必须修 | 4 项 | ✅ 完成 |
| FC-P1 强烈建议 | 3 项 | ✅ 完成 |
| FC-DOC 文档对账 | 1 项 | ✅ 完成 |
| FC-CHAIN 链路修正 | 1 项 | ✅ 完成 |
| **合计** | **8 项收口** | ✅ **Phase 0.5.1 Final CLOSED** |

---

## Final Closure 修改文件清单

```
docs/phase-0.5/wireframes/01-dashboard.html          (M)  FC-P1-1
docs/phase-0.5/wireframes/08-graph-designer.html     (M)  FC-P0-1
docs/phase-0.5/wireframes/09-health-tree.html        (M)  FC-P0-2 + FC-DOC-1
docs/phase-0.5/wireframes/10-states.html             (M)  FC-P0-3 + FC-P0-4 + FC-P1-2 + FC-P1-3
docs/phase-0.5/OPERATOR_WORKFLOW.md                  (M)  FC-P1-3
docs/phase-0.5/INDEX.md                              (M)  FC-P1-3
docs/phase-0.5/README.md                             (M)  FC-P1-3
docs/phase-0.5/ERRATA.md                             (M)  本段
docs/phase-0.5/chains/chain-2-failure.md             (M)  FC-CHAIN-1
README.md                                            (M)  FC-P1-3
```

---

## Phase 0.5 LOCK FINAL 判定

| 维度 | 状态 |
|---|---|
| 信息架构 | 🟢 PASS |
| 9 Core Workflow | 🟢 PASS |
| Chain 1 (On-Air) | 🟢 PASS |
| Chain 2 (Failure) | 🟢 PASS (FC-CHAIN-1 修后) |
| Chain 3 (Playout) | 🟢 PASS |
| Chain 4 (Engineering) | 🟢 PASS (FC-P0-1 修后) |
| TAKE State Machine | 🟢 PASS |
| Operator Intent | 🟢 PASS |
| Timeline / Playout | 🟢 PASS |
| Incident → Replay | 🟢 PASS |
| Clock Reference | 🟢 PASS |
| Graph Compiler UX | 🟢 PASS (FC-P0-1 修后) |
| Health Tree | 🟢 PASS (FC-P0-2 + FC-DOC-1 修后) |
| State Catalogue | 🟢 PASS (FC-P0-3 + FC-P0-4 修后) |
| 文档一致性 | 🟢 PASS (FC-P1-3 修后) |
| **Phase 0.5 Freeze** | 🟢 **LOCK FINAL** |

**Operator UX Semantics = CLOSED**
**UI Architecture = IMPLEMENTATION AUTHORITY**
**下一阶段直接 Phase 0.6 — Executable Acceptance Specification**
**不再继续讨论 "还要不要加页面 / 加功能"**

---

## 5. 修改文件清单

```
docs/phase-0.5/wireframes/01-dashboard.html        (M)  P0-2  + P2-1
docs/phase-0.5/wireframes/02-sources.html          (M)  P1-6
docs/phase-0.5/wireframes/03-switcher.html         (M)  P0-3
docs/phase-0.5/wireframes/04-composition.html      (M)  P0-4
docs/phase-0.5/wireframes/05-audio.html            (M)  P1-3
docs/phase-0.5/wireframes/06-output.html           (M)  P1-4
docs/phase-0.5/wireframes/07-recording.html        (M)  P1-5
docs/phase-0.5/wireframes/08-graph-designer.html   (M)  P0-1
docs/phase-0.5/wireframes/09-health-tree.html      (M)  P0-5 + P1-2
docs/phase-0.5/wireframes/10-states.html           (A)  P2-2 (new)
docs/phase-0.5/ERRATA.md                           (A)  本文档
```

---

## 6. Phase 0.5.1 → Phase 0.6 衔接

> **Phase 0.5 → LOCK FINAL**（Final Closure 8 项已收口）
> **Operator UX Semantics → CLOSED**
> **UI Architecture → IMPLEMENTATION AUTHORITY**

Phase 0.5.1 完成后，UI 已具备：
- ✅ 9 Core Operational Pages + 1 Validation State Page = **10 HTML artifacts**
- ✅ 三轴 Runtime 状态（10 状态样例 · 全部使用 Canonical Enum）
- ✅ 5 状态 TAKE 状态机 + L2 确认
- ✅ Incident → Replay 工作流
- ✅ Clock Reference 完整呈现
- ✅ Operator Intent Layer
- ✅ Graph Designer [DESIGN / COMPILED / VALIDATION] 三 Tab
- ✅ Health Tree 9 Subsystem + 3 视图 + 7 Health Invariants
- ✅ 4 操作链全部能在 UI 上找到对应入口

下一阶段 **Phase 0.6 — Executable Acceptance Specification**：
- Reference A1：PACKET 基础场景
- Reference A2：SDI 主备走 FRAME/MASTER
- Reference B：异构源
- 5 Fault Injection（FI-01..FI-05）
- 7 Health Invariants（H1-H7）
- = **Executable Acceptance Specification**（每条都能跑出来）

---

## 7. 经验教训

1. **"Static Wireframe" ≠ "Operational Wireframe"** — 每个关键页面至少要有 Normal + Warning + Failed + Recovery 4 个状态样例，否则 UI 在状态变化时会"丢失信息"。
2. **架构层正确的示例在 UI 层也必须正确** — P0-1 SDI + PACKET_SWITCH 冲突就是 UI 用了与架构不一致的示例，会误导工程师。
3. **State Machine 必须画出来** — TAKE 5 状态、L2 Confirm、5+1 步操作流，光写文档不够。
4. **三轴状态必须映射到 UI** — Lifecycle + Readiness + Health + ECHS 一一对应，不要让 UI 再发明新的 status 字段。
5. **Incident→Replay 是核心工作流** — 不能只列 Chunk + Incident 列表，必须有"按事故自动定位"的 UI 闭环。

---

**VBMF Contributors** · Phase 0.5 LOCK FINAL · Phase 0.5.1 20 项 UI 语义修复（12 + 8 Final Closure）
