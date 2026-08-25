# Phase 0.5 — UX Baseline (0.5A + 0.5B + 0.5C 统一)

> **状态**: 🟡 **0.5C RECONCILED · 0.5D IN PROGRESS** (本轮评审后)
>
> **顶层入口**: 整个 Phase 0.5 的"对外"权威 README, 之前 `phase-0.5b/README.md` 已删除
>
> **Phase 0.5 LOCK FINAL 的前置**: 0.5C LOCK + 0.5D LOCK + 0.5E LOCK (见 [`MILESTONES.md`](MILESTONES.md))

## 0. Phase 0.5 是什么

Phase 0.5 是 **V0.2 Architecture LOCK FINAL 之后, Phase 0.6 Executable Acceptance 之前** 的"产品 UX Baseline"阶段。它的目的是:

- 把 V0.2 锁定的 12 Engines / 5 横向系统 / 6 横切能力 翻译成 **UI 表面的完整规范**
- 把所有 wireframe (Operator + Product) 收口到 **同一份 Surface Spec + Design System + i18n Contract**
- 把历史 milestone (0.5A / 0.5B / 0.5B.1 / 0.5B.2) 统一为一个 Phase, **不再**用目录分层做版本管理

## 1. 当前状态矩阵

| Milestone | 状态 | 关键交付 |
|---|---|---|
| **0.5A** Operator Semantics | 🟢 LOCK FINAL | 9 Core + 1 Validation + 4 Chains + 20 项 UI 语义修复 |
| **0.5B** Product Surface | 🟢 LOCK FINAL | SURFACE_SPEC + i18n + 13 P0 语义边界 |
| **0.5B-Closure-1** | 🟢 LOCK FINAL | 10 项产品化收口 (3-Layer / 4-Tuple / 3-Tier / 9D / H1-H7 / Dependency) |
| **0.5B.1** P0 Wireframes | 🟢 LOCK FINAL | 5 张 P0 wireframe (M-11/M-12/M-14/P-21/P-22) |
| **0.5B.2** Product UX Closure | 🟢 LOCK FINAL | 8 P0 + 5 P1 + Design System + UX BASELINE LOCK FINAL |
| **0.5C** Info Arch Closure | 🟡 RECONCILED | 目录归并 + Object Vocabulary + Navigation 4 域 + Product Object Model + 0.6 语义修复 |
| **0.5D** P0 Product Surfaces | 🟡 IN PROGRESS | D1-D6 验收链 + 0.5D.1-.6 Semantic/Execution Closure (对象边界 / TAKE 剥离 / 状态统一 / Click-Path Audit) · 待 0.5D LOCK |
| **0.5E** Global UX Layer | 🟡 SPEC | Impact Preview 全域 / Configuration Diff / Command Palette |

完整 milestone 表见 [`MILESTONES.md`](MILESTONES.md)。

## 2. 4 域顶层导航 (Phase 0.5C 锁定)

UI 顶层导航改为**业务域**, 不用**编号**:

| 域 | 主要用户 | 包含对象 |
|---|---|---|
| **BROADCAST** (直播) | Operator / Director | Channel, Source, Graph, Route, Session, Variant |
| **MEDIA** (媒体) | Content Manager / Editor | Asset, Asset Version, Job (FILE_TRANSCODE/PROBE/QC/UPLOAD/ARCHIVE) |
| **ENGINEERING** (工程) | Engineer / SRE | Profile (7), Profile Bundle, ChangeSet, Preflight, Hardware, Clock, Health, Incident |
| **ADMIN** (管理) | Admin | User, Role, Permission, Audit, System Setting |

⛔ **Profiles / Operations 不再是顶层域** — 全部进 ENGINEERING 域。

详细导航模型见 [`NAVIGATION.md`](NAVIGATION.md)。

## 3. 3 大对象组合层 (Phase 0.5C 锁定)

```
Profile (Policy / 跨 Channel 共享)
   ↓ 1:1 引用
Profile Bundle (Composition / 1 Channel 1 Bundle, 7 Profile 引用)
   ↓ 1:N 派生
Output Variant (Instance / 1 Channel N Variant, Profile + Destinations + Adapter)
```

完整对象模型见 [`PRODUCT_OBJECT_MODEL.md`](PRODUCT_OBJECT_MODEL.md)。
15 个对象权威定义见 [`OBJECT_VOCABULARY.md`](OBJECT_VOCABULARY.md)。

## 4. 目录结构 (Phase 0.5C 锁定)

```
docs/phase-0.5/
├── README.md                       ← 本文件 (Phase 0.5 顶层入口)
├── OBJECT_VOCABULARY.md            ← 15 个对象权威定义
├── PRODUCT_OBJECT_MODEL.md         ← 3 层组合关系
├── NAVIGATION.md                   ← 4 域顶层导航
├── MILESTONES.md                   ← 历史 milestone 归档
├── SURFACE_REGISTRY.yaml           ← 页面计数唯一事实源 (52 wireframe + 1 Spec)
├── EXECUTION_MODEL.md              ← 执行链/时序唯一事实源 (0.5D.3b)
├── SURFACE_SPEC.md                 ← V0.2 架构对象 → UI 表面完整映射 (计数由 SURFACE_REGISTRY.yaml 派生)
├── DESIGN_SYSTEM.md                ← V0.1 Design System
├── I18N_SPEC.md                    ← V0.1 i18n Contract
├── OPERATOR_WORKFLOW.md            ← 9 Core 操作流
├── ERRATA.md                       ← Phase 0.5A 20 项修复归档
├── INDEX.md                        ← Phase 0.5A 总索引
│
├── milestones/                     (5 历史 milestone 文档)
├── operator/                       (9 Core + 1 Validation HTML · 中英双语 · Dark Mode 24/7)
├── product/                        (5 P0 wireframes: M-11 / M-12 / M-14 / P-21 / P-22)
└── chains/                         (4 链: On-Air / Failure / Playout / Engineering)
```

⛔ **`phase-0.5b/` 目录已删除** — 之前叫 `phase-0.5b` 是因为它晚于 `phase-0.5` 创建, 但 Git commit 才是版本管理, 目录应该表达 `phase / domain / role`, 不应承担版本职责。

## 5. 页面架构 (0.5D.1 起 · 由 SURFACE_REGISTRY.yaml 派生)

| 域 | wireframe | Spec-only | 路径 |
|---|---|---|---|
| **BROADCAST** | 13 | 0 | `operator/` (01-07 + CH-01/02/02B + CD-01 + B-13) |
| **MEDIA** | 8 | 0 | `operator/` (M-11..M-18, 含 M-17 Realtime Encode) |
| **ENGINEERING** | 25 | 1 (E-41) | `operator/` (P-20..28 + E-31..42 + O-41..45 + D7) |
| **ADMIN** | 5 | 0 | `operator/` (A-51..55) |
| **域合计** | **51** | **1** | — |
| 全局 (10-states Validation) | 1 | 0 | `operator/10-states.html` |
| **TOTAL** | **52** | **1** | — |
| **Phase 0.5 总计** | **53** (52 wireframe + 1 Spec) | | |

## 6. 与 V0.2 / Phase 0.6 / Phase 1 / Phase 4 关系

```
V0.2 Architecture (LOCK FINAL)
   ↓ 翻译为产品对象
Phase 0.5 UX Baseline (本目录, LOCK FINAL 条件见 MILESTONES.md)
   ↓ 验收
Phase 0.6 Executable Acceptance Specification
   ↓ 实施
Phase 1 Media Agent (Rust) + Phase 4 Web Console
```

**禁止:** 在 Phase 0.5 LOCK FINAL 之前开 Phase 0.6, 否则测试会因 UI / 文档未锁而反复改。

## 7. i18n 约定 (Phase 0.5.1 锁定)

- **Markdown 文档**: 中文为主
- **wireframe**: 中英双语 (中文为主, Canonical Vocabulary 保留英文: PACKET / FRAME / MASTER / HLS / RTMP / WebRTC / SRT / H.264 / H.265 / PTP / LUFS / EBU R128 / dBTP)
- **enum 翻译表**: 见 [`I18N_SPEC.md`](I18N_SPEC.md) 11 个 enum 的 zh-CN / en-US 双向映射
- **格式化**: 日期 / 时间 / 数字 / 码率 / 延迟 / 响度 / 复数 / 插值 全部锁定

## 8. 8 横切能力 (0.5B.2 + 0.5C 锁定)

1. **Impact Preview** — 任何修改前显示 Affected / Resource / Runtime Risk / Rollback
2. **Dependency Graph** — "谁用我 / 我影响谁" 全局视图
3. **Explain Why** — 6 类解释 (Why selected / Why not usable / Why degraded / Why this worker / Why FRAME not PACKET / Why output failed)
4. **Runtime Freshness** — Health / Discovery / Capability 的 Last observed / Age / Fresh·Stale
5. **Configuration Diff** — 所有 Revision 之间 before/after/impact
6. **Compatibility Advisor** — Profile ↔ Source ↔ Worker ↔ Output ↔ Player
7. **Design System** — 5 张 wireframe 统一组件 (DESIGN_SYSTEM.md)
8. **Command Palette + Keyboard** — Ctrl+K / G D / G M / T / F / R 等

## 9. 当前 LOCK FINAL 条件 (0.5C → 0.5D → 0.5E)

- ⛔ **0.5C LOCK FINAL** (本轮提交后, 需用户审过)
- ⛔ **0.5D LOCK FINAL** (M-17 + E-38 + E-37 + P-20 + P-28 + M-18 wireframe + M-14 重画)
- ⛔ **0.5E LOCK FINAL** (Impact Preview + Configuration Diff + Command Palette 全部跨域落实)
- ⛔ **README / ROADMAP / SURFACE_SPEC / Phase 0.6 README** 状态完全同步
- ⛔ **Object Vocabulary + Product Object Model + Navigation** 3 文档 LOCK
- ⛔ **GitHub README** 反映 4 域 + `SURFACE_REGISTRY.yaml` 计数 (52 wireframe + 1 Spec), 不再有 "9 Core Pages" "44" "0.5B 只定义" 等历史残留

---

**VBMF Contributors** · Phase 0.5 UX Baseline · Phase 0.5C Information Architecture Closure
