# 路线图

> V0.2 架构基线 LOCK FINAL（22 轮 review）。
> 本文档是 VBMF 项目路线图，对应 V0.2 架构基线 + 实施阶段。

## 状态总览

```
Phase 0    架构冻结                       ✅ V0.2 LOCK FINAL
Phase 0.5  操作员工作流 & 线框            ✅ 完成（9 中英双语页面 + 4 链）
Phase 0.6  Reference + Fault Injection   📋 即将启动
Phase 1    Media Core（Rust + 24h 稳定）  📋 Phase 0.6 验收后
Phase 2    后端基础                       📋
Phase 2.5  Graph Compiler / Preflight     📋
Phase 3    Auth & RBAC                    📋
Phase 3.5  UI 原型与验证                 📋
Phase 4    Web 控制台                     📋
Phase 5    Signal Fabric                  📋
Phase 5.5  Health Tree & Incident UI      📋
V0.3       架构扩展                       📋 任何架构级扩展必须开 V0.3
V0.4       广播级（PTP/SDI/HA）           📋
V0.5       WebRTC + 浏览器上行            📋
V1.0       完整 IP 播控                   📋
```

## Phase 0 — 架构冻结 ✅

**已交付**（22 轮 review）：

- 12 Engines + 5 横向系统 + 6 横切能力 + 22 原则 + 57 决策
- V0.2 Runtime Semantics CLOSED
- implementation_ambiguity: NONE
- 9 Runtime 域 CLOSED + 3 Schema 焊死 + 2 Semantic Cleanup + 7 Health Invariants
- 文档：`docs/architecture/ARCHITECTURE_V0.2.md`（192KB / 4021 lines）

## Phase 0.5 — 操作员工作流与 Low-Fi 线框 ✅

**已交付**：

- `docs/phase-0.5/INDEX.md` — Phase 0.5 总览
- `docs/phase-0.5/OPERATOR_WORKFLOW.md` — 角色矩阵 + 三轴状态机 + 危险操作 3 层
- 9 Low-Fi HTML 线框（**中英双语**，Dark Mode 24/7）：
  1. Dashboard（主控台 / PVW·PGM）
  2. Sources（源管理 / 11 types）
  3. Switcher（切播器 / 3 modes）
  4. Composition（图文包装 / RAW 域）
  5. Audio（混音 / 响度 / 延迟）
  6. Output（SRS Gateway Adapter）
  7. Recording（录制 / 事件回溯）
  8. Graph Designer（拖拽式，NEW）
  9. Health Tree（X5，NEW）
- 4 关键操作链：On-Air / Failure / Playout / Engineering

**验收**：

- 9 页面互相跳转（20 链接）
- 4 链引用 V0.2 架构（7 处 §X.Y 引用）
- Health Tree 显式呈现 7 Health Invariants + 7 HA-01..HA-07 验收用例
- Dark Mode 24/7 全部 CSS 用 `:root` dark 变量

## Phase 0.6 — Reference Implementation + Fault Injection 📋 **Next**

> 即将启动。本阶段不写架构，只做 **Executable Acceptance Specification**。

**计划交付**：

- **Reference A1**（PACKET_SWITCH 基础）：预对齐压缩源 A / B（同 codec/container/时间戳）
  - 验证：Capability Contract、GOP/IDR、PTS/DTS、timebase、SPS/PPS、audio continuity
- **Reference A2**（SDI 主备走 FRAME/MASTER）：SDI-A/B → Normalize → Encode → FRAME/MASTER → SRS → HLS
- **Reference B**（异构源 + 图文 + 多 Master）：SDI + SRT + Composition + Audio Mixer → MASTER_SWITCH → Program Master → SRS
- **5 Fault Injection**：
  - FI-01：SDI 冻结 5s → SOURCE → FAILOVER
  - FI-02：音频静音 8s → PIPELINE → RESTART
  - FI-03：Primary FFmpeg 进程崩溃 → PIPELINE → RESTART
  - FI-04：Clock Drift +5ms/min → CLOCK → FALLBACK
  - FI-05：HLS 切片失败 → OUTPUT → RESTART_ADAPTER
- **7 Health Invariants** → executable test cases
  - HA-01..HA-07 from `docs/phase-0.5/wireframes/09-health-tree.html`
- 端到端：在 10.30.15.10 服务器上跑通
- 24h stability（基础）

## Phase 1 — Media Core（Rust + 24h 稳定性）📋

- [ ] Media Agent v0（Rust + JSON-RPC）
- [ ] Session Manager（Data Plane 标注 + Switch Mode + Hot-Standby）
- [ ] FFmpeg Command Builder（**不用 fluent-ffmpeg** — 锁定）
- [ ] FFmpeg `-progress pipe:1` 解析
- [ ] BMD 设备 Registry（Media Agent 启动时探测）
- [ ] Clock Domain 检测
- [ ] Edge Policy 引擎
- [ ] Latency Probes（7 Core + 2 Client E2E + 1 Optional CDN）
- [ ] AVSync Manager（measure / compensate / drift）
- [ ] Switcher 3 modes
- [ ] Hot-Standby 3 levels
- [ ] Local NVMe Recording（5 min/段）
- [ ] SRS 单实例
- [ ] 端到端：`SDI → ffmpeg → SRS → HLS`，24h 不掉

## Phase 2 — 后端基础 📋

- [ ] Fastify + Drizzle + Zod
- [ ] PostgreSQL schema V0.2.4 final
- [ ] Valkey + Event Bus
- [ ] BullMQ + 转码 worker
- [ ] Media Controller
- [ ] GraphSpec / GraphRuntime 数据模型

## Phase 2.5 — Graph Compiler / Preflight 📋

X1-X6 横切能力的实施：

- [ ] X1 Graph Compiler（Validator / Insert Missing / Clock Align / Latency Estimate / Resource Plan / Emit Runtime）
- [ ] X2 Preflight（Graph / Playout / Channel 三类）
- [ ] X3 Configuration Versioning（Draft / Validate / Preview / Apply / Rollback）
- [ ] X4 Incident Timeline（自动串接）
- [ ] X5 Health Tree（7 规则 + 7 Invariants）
- [ ] X6 Capability Registry（Signal Contract + Player Matrix）

## Phase 3 — Auth & RBAC 📋

- [ ] 用户 / 角色 / 权限
- [ ] 4 角色：Operator / Director / Engineer / Admin
- [ ] RBAC 与 V0.2 Operator Workflow 对齐

## Phase 3.5 — UI 原型与验证 📋

- [ ] 9 页面 + 4 链端到端验证
- [ ] Reference A1/A2/B 真实可跑
- [ ] 5 Fault Injection 全部覆盖

## Phase 4 — Web 控制台 📋

- [ ] 9 核心页面（Phase 0.5 wireframe → 真实现）
- [ ] 4 关键操作链验证
- [ ] Dark Mode First 24/7
- [ ] i18n 准备（中英双语）

## Phase 5 — Signal Fabric 📋

- [ ] 多 Channel 管理
- [ ] Output Variant 多路分发
- [ ] Adaptive Bitrate

## Phase 5.5 — Health Tree & Incident Timeline UI 📋

- [ ] Health Tree 可视化
- [ ] Incident Timeline 串联
- [ ] 录像回溯
- [ ] 7 Health Invariants 实时展示

## V0.3 — 架构扩展 📋

> 任何 V0.2 之后想加的功能，必须开 V0.3 流程。

候选 V0.3 特性（仅占位，不在 V0.2 范围）：

- NDI / RIST / Zixi Source Adapter
- SDI Master Output（V0.2 已 RESERVED）
- PTP / Genlock 完整支持
- 多节点 HA
- WebRTC 上行 / 互动

## V0.4 / V0.5 / V1.0

参见 `docs/architecture/ARCHITECTURE_V0.2.md` 附录 B（版本演进表）。

## 风险 / 注意事项

| 风险 | 缓解 |
|---|---|
| Phase 0.6 真实部署发现 V0.2 漏 | 开 V0.3 流程，不私自改 V0.2 |
| 7 Health Invariants 实测未通过 | 调 SQL 实现或算法，不动 Schema |
| 5 Fault Injection 恢复动作不符合 §8.9 | 修复实现，§8.9 是 SoT |
| Playwright 浏览器 E2E 不稳定 | 用直接 JSON-RPC 测试，浏览器只测渲染 |

## 与开源社区的协作

| 阶段 | 社区协作 |
|---|---|
| Phase 0.5 | 公开 9 wireframe + 4 链（中英双语），欢迎 UI / UX 反馈 |
| Phase 0.6 | 公开 Reference A1/A2/B + 5 Fault Injection 配置，欢迎调参与建议 |
| Phase 1+ | 接受 Rust / TypeScript 贡献 |

---

**VBMF Contributors** · V0.2 LOCK FINAL · Apache 2.0
