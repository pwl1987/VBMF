# VBMF — IP Broadcast Media Fabric

> **V0.2 Architecture Baseline LOCK FINAL · 22 轮 review · 12 Engines + 5 横向系统 + 6 横切能力 + 22 原则 + 57 决策**
>
> **V0.2 Runtime Semantics CLOSED · implementation_ambiguity: NONE · 不再开 V0.2.5**

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![V0.2 Lock](https://img.shields.io/badge/V0.2-Architecture_LOCK_FINAL-green.svg)](docs/architecture/ARCHITECTURE_V0.2.md)
[![Runtime Semantics](https://img.shields.io/badge/Runtime_Semantics-CLOSED-green.svg)](docs/architecture/ARCHITECTURE_V0.2.md)
[![Phase 0.5](https://img.shields.io/badge/Phase-0.5-yellow.svg)](docs/phase-0.5/INDEX.md)
[![Review Passes](https://img.shields.io/badge/review_passes-22-blueviolet.svg)](docs/architecture/ARCHITECTURE_V0.2.md)

[English](#english) · [中文](#中文)

---

## 中文

VBMF（**IP Broadcast Media Fabric**）是一个面向 24/7 广播机房的生产级 IP 媒体信号处理平台。

把 **SDI 摄录 → 编码 → RTMP / HLS / WebRTC 分发**，加上 **VOD 转码** 和 **Web 控制台**，全部用 **12 个核心 Engines + 5 横向系统 + 6 横切能力** 干净组合起来。

> 主备切换支持 **3 种 Switch Mode**（PACKET / FRAME / MASTER）+ **3 级 Hot-Standby**（COLD / WARM / HOT）；
> 健康监控采用 **Health Tree + 7 条 Health Invariants**，从通道 → 子系统 → 节点三层展开。

### 🎯 适合谁

- **广播 / 流媒体工程师** 想自建 SDI 摄录 + IP 分发一体化平台
- **OTT / 直播团队** 需要 PACKET / FRAME / MASTER 三档主备切换
- **架构 / 平台团队** 想参考"如何把 12 个独立 Engine 干净拼装"的最佳实践

### ✨ 核心能力

| 维度 | 状态 | 备注 |
|---|---|---|
| **架构** | ✅ V0.2 LOCK FINAL | 22 轮 review，57 决策，完整可唯一实现 |
| **Runtime Semantics** | ✅ CLOSED | 9 大 Runtime 域 + 3 Schema + 2 Semantic Cleanup + 7 Health Invariants |
| **Operator Workflow** | ✅ Phase 0.5 | 9 Low-Fi 页面 + 4 关键操作链 |
| **Reference Implementation** | 📋 Phase 0.6 | Reference A1/A2/B + 5 Fault Injection = Executable Acceptance Specification |
| **Media Agent (Rust)** | 📋 Phase 1 | JSON-RPC + Session Manager + FFmpeg Command Builder + 24h stability |
| **Web Console** | 📋 Phase 4 | 9 页面 + 4 链验证 |

### 🏗️ 12 Engines + 5 横向系统 + 6 横切能力

```
12 Engines:
  Source (11 types) · Switcher (3 modes) · Playout · Composition · Audio · Output
  Recording · Replay · Switcher · Composition · Audio · Master Join
  + 5 横向系统: Source Adapter, Stream Gateway, Direct Output, Recording, Output Distribution
  + 6 横切能力 (X1-X6):
    X1 Graph Compiler    X2 Preflight    X3 Configuration Versioning
    X4 Incident Timeline X5 Health Tree  X6 Capability Registry
```

### 📚 文档结构

```
docs/
├── architecture/
│   └── ARCHITECTURE_V0.2.md     ← 22 轮 review LOCK FINAL 架构基线 (192KB)
├── phase-0.5/                  ← Operator Workflow + 9 Low-Fi 页面 + 4 关键操作链
│   ├── INDEX.md
│   ├── OPERATOR_WORKFLOW.md
│   ├── wireframes/              (9 HTML, Dark Mode 24/7)
│   └── chains/                  (4 链：On-Air / Failure / Playout / Engineering)
├── phase-0.6/                  ← Executable Acceptance Specification（待补）
├── assets/                      ← 图 / Diagram（待补）
└── SYSTEM_AND_PROJECT_PLAN.md  ← 初始系统 + 项目计划
```

### 🚀 快速开始

> ⚠️ V0.2 还是 **架构冻结 + Phase 0.5 完成** 阶段，**代码实现要从 Phase 1 开始**。

#### 阅读顺序（推荐）

1. **[架构基线](docs/architecture/ARCHITECTURE_V0.2.md)**（~1-2 小时通读）
2. **[Phase 0.5 Operator Workflow](docs/phase-0.5/INDEX.md)**（~30 分钟浏览 9 页面 + 4 链）
3. **[初始项目计划](docs/SYSTEM_AND_PROJECT_PLAN.md)**（理解服务器 / 编译环境基线）
4. 等待 Phase 0.6 / Phase 1 实施

#### 部署参考（V0.2 §3.11 current_host_snapshot，非 Architecture Fact）

| 资源 | 数量 | 备注 |
|---|---|---|
| CPU | 32 核 | Runtime Discovery |
| RAM | 30 GB | Runtime Discovery |
| Disk | 546 GB | Runtime Discovery |
| BMD DeckLink | 3 张 (2× SDI legacy + 1× Mini Monitor 4K) | Runtime Discovery |
| OS | Ubuntu 26.04 | Runtime Discovery |

> 任何硬件增配 / 替换 **不需要修改 V0.2 架构**。

### 🛠️ 开发

```bash
# 克隆
git clone https://github.com/<your-org>/VBMF.git
cd VBMF

# 浏览架构
cat docs/architecture/ARCHITECTURE_V0.2.md | less

# 打开 Phase 0.5 线框（任意浏览器）
start docs/phase-0.5/wireframes/01-dashboard.html
```

### 🤝 贡献

欢迎提交 PR。但请先阅读：
- [CONTRIBUTING.md](CONTRIBUTING.md)
- [架构冻结说明](docs/architecture/ARCHITECTURE_V0.2.md#final-state)
- **V0.2 已 LOCK FINAL**：任何架构级修改需开 V0.3 流程；Phase 0.5/0.6 是 Acceptance Validation，不修改架构

### 📜 许可证

[Apache 2.0](LICENSE) — 商用友好，需保留版权。

### 🙏 致谢

- **架构贡献**：22 轮 review 的所有发现 + 修复
- **SRS** ([ossrs/srs](https://github.com/ossrs/srs)) — Stream Gateway Adapter
- **FFmpeg** — Media Pipeline 引擎
- **BMD Desktop Video SDK** — SDI 摄录

---

## English

**VBMF (IP Broadcast Media Fabric)** is a production-grade IP media signal processing platform designed for 24/7 broadcast rooms.

It cleanly composes **SDI ingest → Encode → RTMP / HLS / WebRTC distribution**, plus **VOD transcoding** and **Web console**, from **12 core Engines + 5 cross-cutting systems + 6 horizontal capabilities**.

> Hot-standby switching supports **3 Switch Modes** (PACKET / FRAME / MASTER) + **3 Hot-Standby Levels** (COLD / WARM / HOT);
> Health monitoring uses **Health Tree + 7 Health Invariants** unfolded across Channel → Subsystem → Node.

### 🎯 Who is this for

- **Broadcast / streaming engineers** building their own SDI ingest + IP distribution platform
- **OTT / live teams** needing PACKET / FRAME / MASTER tier failover
- **Architects / platform teams** looking for "how to cleanly compose 12 independent Engines" best practices

### 📊 Current status

| Dimension | Status | Note |
|---|---|---|
| **Architecture** | ✅ V0.2 LOCK FINAL | 22 review rounds, 57 decisions, fully implementable |
| **Runtime Semantics** | ✅ CLOSED | 9 Runtime domains + 3 Schema + 2 Semantic Cleanup + 7 Health Invariants |
| **Operator Workflow** | ✅ Phase 0.5 | 9 Low-Fi pages + 4 critical operation chains |
| **Reference Implementation** | 📋 Phase 0.6 | Reference A1/A2/B + 5 Fault Injection = Executable Acceptance Spec |
| **Media Agent (Rust)** | 📋 Phase 1 | JSON-RPC + Session Manager + FFmpeg Command Builder + 24h stability |
| **Web Console** | 📋 Phase 4 | 9 pages + 4 chain validation |

### 📚 Documentation

```
docs/
├── architecture/ARCHITECTURE_V0.2.md  ← 22 review rounds LOCK FINAL (192KB)
├── phase-0.5/                         ← Operator Workflow + 9 Low-Fi + 4 chains
├── phase-0.6/                         ← Executable Acceptance Spec (TBD)
└── SYSTEM_AND_PROJECT_PLAN.md         ← Initial server + project plan
```

### 🚀 Quick start

> ⚠️ V0.2 is **architecture frozen + Phase 0.5 complete**. **Code implementation starts at Phase 1.**

#### Reading order (recommended)

1. **[Architecture baseline](docs/architecture/ARCHITECTURE_V0.2.md)** (~1-2 hours)
2. **[Phase 0.5 Operator Workflow](docs/phase-0.5/INDEX.md)** (~30 min, 9 pages + 4 chains)
3. **[Initial project plan](docs/SYSTEM_AND_PROJECT_PLAN.md)** (server / build env baseline)
4. Wait for Phase 0.6 / Phase 1

#### Reference deployment (V0.2 §3.11, NOT Architecture Fact)

| Resource | Quantity | Note |
|---|---|---|
| CPU | 32 cores | Runtime Discovery |
| RAM | 30 GB | Runtime Discovery |
| Disk | 546 GB | Runtime Discovery |
| BMD DeckLink | 3 (2× SDI legacy + 1× Mini Monitor 4K) | Runtime Discovery |
| OS | Ubuntu 26.04 | Runtime Discovery |

> Any hardware add/replace **does not require V0.2 architecture changes**.

### 🛠️ Development

```bash
git clone https://github.com/<your-org>/VBMF.git
cd VBMF

# Browse architecture
cat docs/architecture/ARCHITECTURE_V0.2.md | less

# Open Phase 0.5 wireframes (any browser)
open docs/phase-0.5/wireframes/01-dashboard.html
```

### 🤝 Contributing

PRs welcome. Please first read:
- [CONTRIBUTING.md](CONTRIBUTING.md)
- [Architecture freeze notice](docs/architecture/ARCHITECTURE_V0.2.md#final-state)
- **V0.2 is LOCK FINAL**: any architecture-level change requires V0.3 process; Phase 0.5/0.6 are Acceptance Validation, not architecture changes

### 📜 License

[Apache 2.0](LICENSE) — commercial-friendly, requires attribution.

### 🙏 Acknowledgments

- **Architecture contribution**: 22 review rounds of findings + fixes
- **SRS** ([ossrs/srs](https://github.com/ossrs/srs)) — Stream Gateway Adapter
- **FFmpeg** — Media Pipeline Engine
- **BMD Desktop Video SDK** — SDI ingest
