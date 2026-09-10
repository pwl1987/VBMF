# 路线图

> V0.2 架构基线 LOCK FINAL（22 轮 review）。
> 本文档是 VBMF 项目路线图（三段式：历史架构 / 当前实施 / 未来产品）。
> **当前实施阶段的唯一事实源是 [`docs/architecture/PHASE_IMPLEMENTATION_MAP.md`](docs/architecture/PHASE_IMPLEMENTATION_MAP.md)**（本文件只保留概要链接，不复制细节，避免双源漂移）。
> **V0.3 产品化基线：[`docs/architecture/V0.3_STANDALONE_PRODUCT_BASELINE.md`](docs/architecture/V0.3_STANDALONE_PRODUCT_BASELINE.md)**。

## 状态总览

### A. Historical Architecture Roadmap（历史架构阶段 — LOCK FINAL，不再变更）

```
Phase 0    架构冻结                       ✅ V0.2 LOCK FINAL
Phase 0.5A 操作员语义与线框            ✅ LOCK FINAL（10 中英双语页面 + 4 链 + 20 项修复）
Phase 0.5B 产品 UI Surface             ✅ UX BASELINE LOCK FINAL（56 surfaces（55 wireframes + 1 Spec，SoT: SURFACE_REGISTRY.yaml）+ 5 P0 wireframe + 36 项收口）
Phase 0.5C 信息架构收口                 🟢 LOCK FINAL（目录归并 + 4 域导航 + Object Vocabulary）
Phase 0.5D P0 产品表面                  🟢 LOCK FINAL（6 新表面 + M-14 重画）
```

### B. Current Implementation Roadmap（当前实施阶段 — SoT: PHASE_IMPLEMENTATION_MAP.md）

```
Phase 0.6  Runtime Abstraction           ✅ COMPLETE（PR#1, tag phase-0.6-runtime-abstraction-baseline）
Phase 0.7A Session Runtime               ✅ COMPLETE（PR#2, 四轮 Merge Gate Hardening）
Phase 0.7B  Media Semantics              ✅ COMPLETE（Normalize/Clock/Audio/Timecode 四基础, PR#3-#6）
Phase 0.7C External Integration          ✅ COMPLETE（Runtime State → Query → Command → Idempotency → Error → Event → API → Transport）
Phase 0.7D Event Integration             ✅ COMPLETE
Phase 0.8 Federation / Multi-site         📋 P2
```

### C. V0.3 Productization Roadmap（当前产品化方向）

```
V0.3-P0  Standalone Product Baseline     ✅ BASELINE（架构/红线/不可变项已落盘）
V0.3-P1  Standalone Boot + Control API   📋 NEXT
V0.3-P2  Web Console P0                  📋 Dashboard/Sources/Sessions/Switcher/Outputs/Health/Engineering
V0.3-P3  Bug Fix + Runtime Hardening     📋 Regression/Fault Injection/24h Stability
V0.3-P4  Broadcast Professionalization   📋 Recording/Incident/Graph/Preflight/Capability/AVSync/Redundancy
V0.3-P5  Conformance                     📋 Mother Contract Mapping + Consumer Evidence
V0.3-P6  Integrated Mode                 📋 Mother Platform Integration
```

> V0.3 不要求母框架完成后才能实施。Standalone Mode 是一等产品运行形态；Integrated Mode 在母框架具备足够 Contract/Control 能力后接入。

## Phase 0 — 架构冻结 ✅

**已交付**（22 轮 review）：

- 12 Engines + 5 横向系统 + 6 横切能力 + 22 原则 + 57 决策
- V0.2 Runtime Semantics CLOSED
- implementation_ambiguity: NONE
- 9 Runtime 域 CLOSED + 3 Schema 焊死 + 2 Semantic Cleanup + 7 Health Invariants
- 文档：`docs/architecture/ARCHITECTURE_V0.2.md`（192KB / 4020 lines）

## Phase 0.5 — Operator Semantics + Product UI Surface ✅（0.5A/0.5B/0.5C/0.5D/0.5E/0.5F LOCK FINAL）

> 详细历史记录保持不变，见 `docs/phase-0.5/`、`SURFACE_REGISTRY.yaml`、`DESIGN_SYSTEM.md`、`I18N_SPEC.md`。

## Phase 0.6 — Reference Implementation + Fault Injection ✅ **COMPLETE**

> 当前实施状态以 `docs/architecture/PHASE_IMPLEMENTATION_MAP.md` 为准。

## Phase 0.7 — Runtime / Media Semantics / External Integration / Event Integration ✅ **COMPLETE**

> 0.7 全阶段最高架构红线继续有效：Observation ≠ Configuration；Semantic Intent ≠ Execution Plan；Canonical 类型零 vendor 依赖。

## V0.3 — 架构扩展 + Standalone Productization 📋

> **V0.3 不是 V0.2 语义重写。**任何 V0.2 冻结语义变化必须走版本化契约和迁移证据。
>
> 完整基线见 `docs/architecture/V0.3_STANDALONE_PRODUCT_BASELINE.md`。

### V0.3-P0 — Baseline

- [x] Standalone Product 定位
- [x] Integrated Mode 定位
- [x] Web Console 边界
- [x] Runtime 唯一执行事实源
- [x] RED LINES / NON-NEGOTIABLES
- [x] Shared SDK/UI 准入原则
- [x] V0.2 Compatibility 原则

### V0.3-P1 — Standalone Boot + Control API

- [ ] 独立启动，不依赖母框架
- [ ] Local configuration
- [ ] Control API / Query API
- [ ] Event projection/stream
- [ ] Web Console shell

### V0.3-P2 — Web Console P0

- [ ] Dashboard
- [ ] Sources
- [ ] Sessions
- [ ] Switcher
- [ ] Outputs
- [ ] Health
- [ ] Engineering

### V0.3-P3 — Bug + Runtime Hardening

- [ ] 现有 bug 全量登记
- [ ] Regression tests
- [ ] Fault injection
- [ ] 24h stability
- [ ] Runtime Observer / Watchdog / Recovery evidence

### V0.3-P4 — Broadcast Professionalization

- [ ] Recording
- [ ] Incident Timeline
- [ ] Graph / Preflight
- [ ] Capability
- [ ] AVSync / Latency
- [ ] Redundancy / Failover

### V0.3-P5 — Conformance

- [ ] Mother Contract mapping
- [ ] Standalone Consumer Evidence
- [ ] `vbmf-sdk` 稳定性评估
- [ ] Shared SDK/UI 真实 Consumer 证据

### V0.3-P6 — Integrated Mode

- [ ] Platform Identity / Control integration
- [ ] Registry / Resolver integration
- [ ] Unified UI shell integration
- [ ] 不改变 Runtime 核心

## V0.4 / V0.5 / V1.0

参见 `docs/architecture/ARCHITECTURE_V0.2.md` 附录 B（版本演进表）。V0.3 新增候选能力仍须单独走版本化流程。

## 风险 / 注意事项

| 风险 | 缓解 |
|---|---|
| Standalone 与 Integrated 分叉 | 两种模式强制共享 Runtime/Domain/Execution |
| 为母框架提前造复杂依赖 | Standalone 优先；Integration 延后；不提前微前端/分布式化 |
| 共享 SDK/UI 过早抽象 | Contract → 真实 Consumer → Conformance → Shared Module |
| Phase 0.6/0.7 真实部署发现 V0.2 漏 | 修实现；若必须改冻结语义则开版本化 V0.3+ 流程 |
| 7 Health Invariants / Fault Injection 实测失败 | 修实现或算法，不偷偷改冻结 Schema |
| UI workaround 掩盖 Runtime bug | UI 不得绕过 Runtime；bug 必须进入 Regression + Runtime evidence |

---

**VBMF Contributors · V0.2 LOCK FINAL + V0.3 Standalone Product Baseline · Apache 2.0**
