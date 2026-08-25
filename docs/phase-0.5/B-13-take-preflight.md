# B-13 · Take Preflight — Spec

> **状态**: 🟡 **0.5F 增补 Spec（P0）**
> **来源**: 0.5C 提案 §23
> **定位**: CD-01 Channel Control Workspace 的 **TAKE 前置联合检查面**（替代纯二次确认弹窗）
> **关联**: CD-01 Channel Control Workspace · V0.2 Switch Mode 3 · Failure Domain Matrix · Oper 流程 C（建立实时频道）/ E（故障切换）

---

## 0. 触发

- CD-01 TAKE 按钮 → 打开 B-13 全屏/模态面板（**非简单 confirm**）
- 实时对 Source B 做 9 项联合检查，全部 PASS 才放 TAKE

---

## 1. 联合检查（9 项）

| # | 检查 | PASS 标准 | 失败动作 |
|---|---|---|---|
| 1 | **Source** | Source B LOCKED | 🔴 阻断 |
| 2 | **Video** | 分辨率/Codec 匹配 Profile | 🔴 阻断 |
| 3 | **Audio** | 音轨存在 + LUFS 在 Profile 范围内 | 🔴 阻断 |
| 4 | **Clock** | PTP LOCKED | 🔴 阻断 |
| 5 | **Switch** | 目标 Switch Mode（FRAME/MASTER/PACKET）eligible | 🔴 阻断 |
| 6 | **Backup** | READY_TO_TAKE（Hot Standby 就绪） | 🔴 阻断 |
| 7 | **Output** | REQUIRED Variant 全 HEALTHY；OPTIONAL/AUXILIARY 仅 WARNING（按 `delivery_criticality`） | REQUIRED FAIL → 🔴 阻断；OPTIONAL/AUXILIARY → 🟡 WARNING（按 Failure Domain，**不误切源**） |
| 8 | **Latency** | Budget PASS（≤ `max_startup_latency`） | 🔴 阻断 |
| 9 | **Resource** | E-36 Resource Vector：≤80% PASS；80–100% 仅当 `resource_reservation` 已满足 → 🟡 WARN；>100% → 🔴 BLOCK | 见资源规则 |

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

- 任一 #1–#8 FAIL → **TAKE 按钮禁用**，显示失败项 + 原因 + 建议动作
- 对齐 **Failure Domain Matrix**：Output 坏 → 提示 `Rejoin Multicast` / 修 Output，**不**误切节目源
- 全部 PASS → 启用 TAKE，提交 Change Set（E-33）

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
