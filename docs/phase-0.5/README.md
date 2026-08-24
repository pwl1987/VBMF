# Phase 0.5 — 操作员工作流与 Low-Fi 线框

> **状态**：✅ **LOCK FINAL**（含 **Phase 0.5.1** UI Semantics Closure — 12 项 + 8 项收口）
> **范围**：1 份操作员工作流 + **9 Core Operational Pages + 1 Validation State Page**（共 10 HTML artifact，中英双语） + 4 关键操作链 + ERRATA 归档
> **目的**：操作员级验证 — UI / 角色 / 危险操作 / 状态机的 Single Source of Truth

## 页面架构（Phase 0.5.1 Final 锁定）

```
9 Core Operational Pages  (正式产品工作域)
+ 1 Validation State Page (Phase 0.5 验收辅助, 不在 9 Core 计数)
= 10 HTML artifacts
```

## Phase 0.5.1 — UI Semantics Closure

Phase 0.5 完成后从"24/7 广播机房操作员能否安全、快速、无歧义地使用"角度复审，发现 **18 个 UI/UX 语义缺口**（其中 1 项违反 V0.2 锁定），落地为 **12 + 8 = 20 项修复**：

- **第一轮 (12 项)**：5 项 P0 必须修 + 5 项 P1 强烈建议 + 2 项 P2 锦上添花
- **第二轮 (8 项)**：4 项 P0 语义错误 + 3 项 P1 文档/口径 + 1 项 Health Tree 9 Subsystem 对齐

完整归档见 [`ERRATA.md`](ERRATA.md)。

## 入口

| 文件 | 用途 |
|---|---|
| [`INDEX.md`](INDEX.md) | Phase 0.5 总览 + 验收 |
| [`OPERATOR_WORKFLOW.md`](OPERATOR_WORKFLOW.md) | 角色矩阵 + 三轴状态机 + 危险操作 3 层 |
| [`ERRATA.md`](ERRATA.md) | Phase 0.5.1 变更归档（12 + 8 项 UI 语义修复） |

## 9 Core Operational Pages + 1 Validation Page（中英双语）

Dark Mode First（24/7 广播机房）。任何浏览器直接打开：

| # | 页面 | 主要角色 | 文件 | Phase 0.5.1 |
|---|---|---|---|---|
| 1 | Dashboard 主控台 | Operator 操作员 | [`wireframes/01-dashboard.html`](wireframes/01-dashboard.html) | P0-2 + P2-1 |
| 2 | Sources 源管理 | Engineer 工程师 | [`wireframes/02-sources.html`](wireframes/02-sources.html) | P1-6 Clock Reference |
| 3 | Switcher 切播器 | Operator 操作员 | [`wireframes/03-switcher.html`](wireframes/03-switcher.html) | P0-3 5 状态机 + L2 |
| 4 | Composition 图文包装 | Director 节目总监 | [`wireframes/04-composition.html`](wireframes/04-composition.html) | P0-4 Timeline+Composition |
| 5 | Audio 音频 | Operator 操作员 | [`wireframes/05-audio.html`](wireframes/05-audio.html) | P1-3 广播安全区 |
| 6 | Output 输出 | Operator 操作员 | [`wireframes/06-output.html`](wireframes/06-output.html) | P1-4 3 视图 |
| 7 | Recording 录制 | Operator 操作员 | [`wireframes/07-recording.html`](wireframes/07-recording.html) | P1-5 Incident→Replay |
| 8 | Graph Designer 图设计 | Engineer 工程师 | [`wireframes/08-graph-designer.html`](wireframes/08-graph-designer.html) | P0-1 Scenario + 3 Tab |
| 9 | Health Tree 健康树 | 全员 | [`wireframes/09-health-tree.html`](wireframes/09-health-tree.html) | P0-5 + P1-2 + 9 Subsystem |
| 10 | **10 States 状态总览** | 全员（验收辅助） | [`wireframes/10-states.html`](wireframes/10-states.html) | P2-2 新增 · **Validation Page** |

## 4 关键操作链

| # | 链 | 角色 | 文件 |
|---|---|---|---|
| 1 | On-Air 播出 | Operator 操作员 | [`chains/chain-1-on-air.md`](chains/chain-1-on-air.md) |
| 2 | Failure 故障 | Operator 操作员 + System | [`chains/chain-2-failure.md`](chains/chain-2-failure.md) |
| 3 | Playout 节目单 | Director 节目总监 | [`chains/chain-3-playout.md`](chains/chain-3-playout.md) |
| 4 | Engineering 工程 | Engineer 工程师 | [`chains/chain-4-engineering.md`](chains/chain-4-engineering.md) |

## 验收

- ✅ 9 Core Pages + 1 Validation Page = 10 HTML artifacts（**中英双语**，Dark Mode 24/7）
- ✅ 9 Core Pages 互相跳转（20+ 链接）
- ✅ 4 链覆盖 4 角色（Operator / Operator+System / Director / Engineer）
- ✅ 4 链引用 V0.2 架构（§3.4 / §3.7 / §3.9 / §8.9 / X1-X6）
- ✅ Health Tree 显式呈现 7 Health Invariants + 7 HA-01..HA-07 验收用例
- ✅ 危险操作 3 层：L1 / L2 (3s 倒计时) / L3 (输入 YES)
- ✅ **所有 UI label 中英双语**（Phase 0.5 锁定要求）

## 与 V0.2 架构的对应

| 操作员概念 | V0.2 架构位 |
|---|---|
| Channel 通道 | §3.1 Data Plane + §5 channel_* |
| PVW/PGM 预览/节目 | §3.7 Program Master（Video Join） |
| TAKE 切播 | §3.4 Switch Mode Decision Tree |
| Health 颜色 | §5 channel_health_view（C.26 Errata-14 7 规则） |
| 9 Core Pages + 1 Validation Page | §10 UX 架构（7+2+1 Validation） |
| Change Set 变更集 | §1.21 + §5 config_revisions / change_sets |
| Incident 事件 | §5 incidents（X4 Incident Timeline） |
| Recording 录制 | §3 Recording Engine + §5 chunked recording |

## 下一步

Phase 0.6 — Reference A1/A2/B + 5 Fault Injection + 7 Health Invariants = **Executable Acceptance Specification**。
