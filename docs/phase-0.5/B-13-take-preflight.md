# B-13 · Take Preflight — Spec

> **状态**: 🟡 **0.5F 增补 Spec（P0）**
> **来源**: 0.5C 提案 §23
> **定位**: CD-01 Channel Control Workspace 的 **TAKE 前置联合检查面**（替代纯二次确认弹窗）
> **关联**: CD-01 Channel Control Workspace · V0.2 Switch Mode 3 · Failure Domain Matrix · Oper 流程 C（建立实时频道）/ E（故障切换）

---

## 0. 触发

- CD-01 TAKE 按钮 → **后台实时 Preflight**（Preflight Engine, 不是每次都弹巨型 UI）。
- **两级 UX (0.5F.5 P1-2):** READY → **Compact Confirmation**（TAKE TARGET / Switch Mode / Backup / Output / Resource 摘要 + `[TAKE]`）；仅 **WARNING / FAIL / CONDITIONAL** → 展开 **B-13 Full 9-item Diagnostics**。
- 全部 PASS 才放 TAKE。TAKE = **Runtime Event** (`evt-take` → Audit / Incident Timeline), **不走 ChangeSet**（0.5F.4 P0-1）。

---

## 1. 联合检查（9 项）

| # | 检查 | PASS 标准 | 失败动作 |
|---|---|---|---|
| 1 | **TAKE TARGET (Source)** | 目标信号 LOCKED（TAKE 要切到的源, ≠ CURRENT SOURCE / FAILOVER BACKUP） | 🔴 阻断 |
| 2 | **Video / Switch Compatibility** | 按 Effective Switch Decision 分支: PACKET=capability_contract strict · FRAME=COMMON_RAW_CONTRACT required + timebase alignable + Normalize 可完成 · MASTER=normalize_to_master required | 🔴 阻断 |
| 3 | **Audio** | 音轨存在 + LUFS 在 Profile 范围内 | 🔴 阻断 |
| 4 | **Clock (Compatibility/Quality)** | reference 可用 + domain compatible + quality ≥ Profile 要求 + fallback chain 有效 + timebase ALIGNABLE（非 PTP 二元） | 🔴 阻断 |
| 5 | **Switch** | 目标 Switch Mode（FRAME/MASTER/PACKET）eligible | 🔴 阻断 |
| 6 | **FAILOVER BACKUP** | READY_TO_TAKE（Hot Standby 就绪, 备源） | 🔴 阻断 |
| 7 | **Output** | REQUIRED Variant 全 HEALTHY；OPTIONAL/AUXILIARY 仅 WARNING（按 `delivery_criticality`） | REQUIRED FAIL → 🔴 阻断；OPTIONAL/AUXILIARY → 🟡 WARNING（按 Failure Domain，**不误切源**） |
| 8 | **Latency** | Budget PASS（≤ `max_startup_latency`） | 🔴 阻断 |
| 9 | **Resource** | E-36 Resource Vector：≤80% PASS；80–100% 仅当 `resource_reservation` 已满足 → 🟡 WARN；>100% → 🔴 BLOCK | 见资源规则 |

> **Video / Clock Schema (0.5F.5 P0-2 修正, 与 B-13 HTML 同 SoT):**
> ```yaml
> video:   # 按 effective_switch_mode 分支
>   packet:  {capability_contract: strict}               # codec/profile/level 严格匹配
>   frame:   {common_raw_contract: required, timebase: alignable, normalize: required}
>   master:  {normalize_to_master: required}
> clock:
>   reference: ptp0            # 不强制 "所有 Channel 必须 PTP"
>   domain: BROADCAST
>   quality: BROADCAST_GRADE   # ≥ Profile 要求
>   fallback: [PTP, TIMECODE, SYSTEM]   # 链有效
>   timebase_alignment: ALIGNABLE
> ```

---

## 1.5 Output Criticality（输出关键度，P0-4 采纳）

- 每个 Output Variant 带 `delivery_criticality`:
  - **REQUIRED** — 直播必交付（如 HLS Domestic / UDP Multicast 主链路）→ Preflight 必须 PASS，否则阻断 TAKE。
  - **OPTIONAL** — 非关键分发（如 RTMP YouTube / Facebook）→ FAIL 仅 🟡 WARNING，不阻断 TAKE。
  - **AUXILIARY** — 归档 / 监控旁路 → FAIL 仅记录，不阻断。
- 对应 P-22 Output Profile 已含 Variant / Destination / Adapter；新增 `delivery_criticality` 字段即可，无需新引擎。

## 1.6 Resource 三档规则（P0-2 采纳，对齐 ENCODE_MODEL_SPEC）

| 占用 | 判定 | TAKE 动作 |
|---|---|---|
| ≤ 80% | PASS | 放行 |
| 80–100% | WARN | 仅当 `resource_reservation` 已满足 → 可放行；否则 BLOCK |
| > 100% | FAIL | 🔴 BLOCK（不开 TAKE） |

> 与 `REALTIME_PROFILE.resource_reservation = REQUIRED` 一致：reservation 未满足即视为资源不足，阻断 ACTIVE。

## 2. 决策与阻断

- **Hard Block（TAKE 禁用）**: 任一 #1–#8 FAIL · **#9 Resource >100%** → 显示失败项 + 原因 + 建议动作（0.5F.5 P1-3: 明确含 #9）
- **Conditional**: #9 Resource 80–100% → 仅当 `resource_reservation` 已满足才放行，否则 BLOCK
- 对齐 **Failure Domain Matrix**：Output 坏 → 提示 `Rejoin Multicast` / 修 Output，**不**误切节目源
- 全部 PASS → Operator Intent → TAKE（Runtime Event `evt-take`）→ Audit / Incident Timeline（0.5F.4 P0-1 修正: **TAKE ≠ ChangeSet**；配置变更才走 E-33 ChangeSet）

---

## 3. 与 4-Layer / Impact

- B-13 输出作为 Take 的 **Preflight Impact**（PIA §6）：Affected Channel / Variant + Risk (LOW/MEDIUM/HIGH/CRITICAL)
- 联动 Oper 流程 C（建立实时频道）与 E（故障切换）

---

## 4. 实施锚点

1. CD-01 TAKE 按钮改造：点击 → 打开 B-13 面板（替代 `confirm()`）
2. 9 项检查调用 Health Tree + Capability + Clock + Output Health 接口
3. 面板底部 `[ CANCEL ]` / `[ TAKE ]`（TAKE 禁用态明确）

⛔ 仅为 Channel Control 的交互增补，不新增 Engine；Switch Mode 语义沿用 V0.2 §2.1。
