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
| 7 | **Output** | HLS / RTMP / UDP 各 Variant HEALTHY | 🔴 阻断（按 Failure Domain，**不误切源**） |
| 8 | **Latency** | Budget PASS（≤ `max_startup_latency`） | 🔴 阻断 |
| 9 | **Resource** | E-36 预算满足 | 🟡 Warning（可放行） |

---

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
