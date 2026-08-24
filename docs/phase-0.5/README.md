# Phase 0.5 — 操作员工作流与 Low-Fi 线框

> **状态**：✅ 完成（V0.2 LOCK FINAL 之后）
> **范围**：1 份操作员工作流 + 9 Low-Fi 页面（中英双语） + 4 关键操作链
> **目的**：操作员级验证 — UI / 角色 / 危险操作 / 状态机的 Single Source of Truth

## 入口

| 文件 | 用途 |
|---|---|
| [`INDEX.md`](INDEX.md) | Phase 0.5 总览 + 验收 |
| [`OPERATOR_WORKFLOW.md`](OPERATOR_WORKFLOW.md) | 角色矩阵 + 三轴状态机 + 危险操作 3 层 |

## 9 Low-Fi Wireframes（中英双语）

Dark Mode First（24/7 广播机房）。任何浏览器直接打开：

| # | 页面 | 主要角色 | 文件 |
|---|---|---|---|
| 1 | Dashboard 主控台 | Operator 操作员 | [`wireframes/01-dashboard.html`](wireframes/01-dashboard.html) |
| 2 | Sources 源管理 | Engineer 工程师 | [`wireframes/02-sources.html`](wireframes/02-sources.html) |
| 3 | Switcher 切播器 | Operator 操作员 | [`wireframes/03-switcher.html`](wireframes/03-switcher.html) |
| 4 | Composition 图文包装 | Director 节目总监 | [`wireframes/04-composition.html`](wireframes/04-composition.html) |
| 5 | Audio 音频 | Operator 操作员 | [`wireframes/05-audio.html`](wireframes/05-audio.html) |
| 6 | Output 输出 | Operator 操作员 | [`wireframes/06-output.html`](wireframes/06-output.html) |
| 7 | Recording 录制 | Operator 操作员 | [`wireframes/07-recording.html`](wireframes/07-recording.html) |
| 8 | Graph Designer 图设计（NEW） | Engineer 工程师 | [`wireframes/08-graph-designer.html`](wireframes/08-graph-designer.html) |
| 9 | Health Tree 健康树（NEW） | 全员 | [`wireframes/09-health-tree.html`](wireframes/09-health-tree.html) |

## 4 关键操作链

| # | 链 | 角色 | 文件 |
|---|---|---|---|
| 1 | On-Air 播出 | Operator 操作员 | [`chains/chain-1-on-air.md`](chains/chain-1-on-air.md) |
| 2 | Failure 故障 | Operator 操作员 + System | [`chains/chain-2-failure.md`](chains/chain-2-failure.md) |
| 3 | Playout 节目单 | Director 节目总监 | [`chains/chain-3-playout.md`](chains/chain-3-playout.md) |
| 4 | Engineering 工程 | Engineer 工程师 | [`chains/chain-4-engineering.md`](chains/chain-4-engineering.md) |

## 验收

- ✅ 9 页面 HTML 可打开（**中英双语**，Dark Mode 24/7）
- ✅ 9 页面互相跳转（20 链接）
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
| 9 页面 | §10 UX 架构（7+2） |
| Change Set 变更集 | §1.21 + §5 config_revisions / change_sets |
| Incident 事件 | §5 incidents（X4 Incident Timeline） |
| Recording 录制 | §3 Recording Engine + §5 chunked recording |

## 下一步

Phase 0.6 — Reference A1/A2/B + 5 Fault Injection + 7 Health Invariants = **Executable Acceptance Specification**。
