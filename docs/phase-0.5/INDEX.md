# Phase 0.5 — 操作员工作流与 Low-Fi 线框

> V0.2 Architecture Baseline LOCK FINAL（22 轮 review）之后的第一阶段。
> 本阶段不修改架构，只做"操作员级"验证。

## 范围（按 V0.2 §10 / §11 锁定）

1. **操作员工作流文档** — 1 份
2. **9 Low-Fi Wireframe** — HTML（**中英双语**；Dark Mode First；24/7 广播机房）
3. **4 关键操作链原型** — 文档级可走通的流程规范

## 目录

```
phase-0.5/
├── README.md                       # 本目录说明（中英标签）
├── INDEX.md                        # 本文件
├── OPERATOR_WORKFLOW.md            # 操作员工作流（角色 / 流程 / 危险操作分层 / 状态机）
├── wireframes/                     # 9 Low-Fi HTML 线框（中英双语，Dark Mode）
│   ├── 01-dashboard.html           # 主控台（Channel Overview / PVW·PGM）
│   ├── 02-sources.html             # 源管理
│   ├── 03-switcher.html            # 切播器 / Take
│   ├── 04-composition.html         # 图文包装
│   ├── 05-audio.html               # 音频混音 / 响度
│   ├── 06-output.html              # 输出（HLS/RTMP/WebRTC）
│   ├── 07-recording.html           # 录制 / 回放
│   ├── 08-graph-designer.html      # Signal Graph Designer（NEW）
│   └── 09-health-tree.html         # Health Tree（NEW）
└── chains/                         # 4 关键操作链
    ├── chain-1-on-air.md           # 链 1：播出（Operator）
    ├── chain-2-failure.md          # 链 2：故障（Operator + System）
    ├── chain-3-playout.md          # 链 3：节目单（Director）
    └── chain-4-engineering.md      # 链 4：工程（Engineer）
```

## 9 页面清单（V0.2 §10）

| # | 页面 | 主要角色 | 关键 Engines | 备注 |
|---|---|---|---|---|
| 1 | Dashboard 主控台 | Operator | Channel / Switcher / Program Master | PVW/PGM 双窗 |
| 2 | Sources 源管理 | Engineer | Source Adapter (11 types) | SDI/SRT/RTMP/HLS/... |
| 3 | Switcher 切播器 | Operator | Switcher (3 modes) / Hot-Standby | Take 按钮 |
| 4 | Composition 图文 | Director | Composition (RAW 域) | Program + Variant 两级 |
| 5 | Audio 音频 | Operator | Audio Mixer / Loudness | EBU R128 |
| 6 | Output 输出 | Operator | Output (SRS Adapter) | HLS/RTMP/WebRTC |
| 7 | Recording 录制 | Operator | Recording (5 min chunk) | + 事件回溯 |
| 8 | Graph Designer 图设计 | Engineer | X1 Graph Compiler / X2 Preflight | 拖拽式 |
| 9 | Health Tree 健康树 | 全员 | X5 Health Tree / §3.9 Aggregation | 树形 + 7 Health Invariants |

## 4 关键操作链（V0.2 §10.11）

| # | 链 | 角色 | 端到端流程 |
|---|---|---|---|
| 1 | On-Air 播出 | Operator | Dashboard → PVW/PGM → Take → Master → Output → 播放 |
| 2 | Failure 故障 | Operator + System | 故障检测 → ALERT → Switch 决策 → Auto Failover → Filler → Incident |
| 3 | Playout 节目单 | Director | Timeline → Asset → Preflight → Apply → 自动到点 → 切 |
| 4 | Engineering 工程 | Engineer | Graph Designer → Compile → Preflight → Apply → Health Tree → QC |

## 与 V0.2 的关联

- §3.9 Health Tree Aggregation（7 Health Invariants）→ wireframe 09
- §3.4 Switch Mode 决策树 → chain-2
- §3.7 Program Master 三独立 graph → wireframe 03
- §8.9 Failure Domain Matrix → chain-2
- §10.11 4 关键操作链 → chains/

## 验收标准

- [x] 9 wireframe 都有 HTML 文件可打开（中英双语，Dark Mode 24/7）
- [x] 4 操作链都从主控台出发，能走到目标终点
- [x] 所有危险操作（Take / Apply / Failover）都有二次确认
- [x] Health Tree 9 页面能展示 7 Health Invariants
- [x] OPERATOR_WORKFLOW.md 包含：角色矩阵、状态机、危险操作分层、4 链 + 9 页面映射

## 下一步

Phase 0.6：Reference A1/A2/B + 5 Fault Injection + 7 Health Invariants = Executable Acceptance Specification。
