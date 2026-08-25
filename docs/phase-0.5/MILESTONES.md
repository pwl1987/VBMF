# VBMF Phase 0.5 Milestones (历史阶段归档)

> **目的:** 把 `phase-0.5 / phase-0.5b / phase-0.5b.1 / phase-0.5b.2` 这些"目录分层"统一为
> **Phase 0.5 下的历史 milestone**, Git commit 仍然负责版本管理, 目录只表达 `phase / domain / role`。
>
> **Current phase status (2026-08-25, 权威见 `README.md`):** 🟢 **Phase 0.5 = LOCK FINAL** — 0.5C RECONCILED→LOCK · 0.5D LOCK · 0.5E LOCK · 0.5F.1-.8 完成 · **0.5F.9 收口补丁 (2 P0 + 5 P1)** · → Phase 0.6 Executable Acceptance。
>
> **本文件 = 历史阶段归档 (Historical), 不承担 Current Status** — 当前状态一律以 `README.md` + `SURFACE_REGISTRY.yaml` 为准 (0.5F.5 P1-5 修正)。

---

## 0. 旧结构问题 (Phase 0.5A/B 时代)

```
docs/
├── phase-0.5/      # 0.5A Operator
└── phase-0.5b/     # 0.5B Product
    (0.5B.1 / 0.5B.2 写在 commit message 里, 目录看不出来)
```

3 个问题:
1. **目录承担版本管理职责** (与 Git 重复)
2. **0.5B 子轮 (0.5B.1 / 0.5B.2) 散在 commit message 中, 文档没有结构化记录**
3. **未来必然膨胀成** `phase-0.5c / phase-0.5-final / phase-0.5-final2 / ...`

## 1. 新结构 (Phase 0.5C 锁定)

```
docs/
├── phase-0.5/                                # 整个 Phase 0.5 一个目录
│   ├── README.md                             # Phase 0.5 顶层 README
│   ├── OBJECT_VOCABULARY.md                  # 15 个对象权威定义
│   ├── PRODUCT_OBJECT_MODEL.md               # 3 层组合关系
│   ├── NAVIGATION.md                         # 4 域导航
│   ├── DESIGN_SYSTEM.md                      # 4 State Models + 15 组件
│   ├── I18N_SPEC.md                          # zh-CN + en-US 翻译规范
│   ├── SURFACE_SPEC.md                       # 表面规范 (计数由 SURFACE_REGISTRY.yaml 派生)
│   ├── OPERATOR_WORKFLOW.md                  # 9 Core 操作流
│   ├── ERRATA.md                             # 20 项修复归档
│   ├── INDEX.md                              # Phase 0.5 总索引
│   │
│   ├── milestones/                           # 历史 milestone 归档
│   │   ├── 0.5A-OPERATOR_SEMANTICS.md        # M0.5A (LOCK FINAL)
│   │   ├── 0.5B-PRODUCT_SURFACE.md           # M0.5B (LOCK FINAL)
│   │   ├── 0.5B.1-P0_WIREFRAMES.md           # M0.5B.1 (LOCK FINAL)
│   │   ├── 0.5B.2-PRODUCT_UX_CLOSURE.md      # M0.5B.2 (LOCK FINAL)
│   │   └── 0.5C-INFO_ARCH_CLOSURE.md         # M0.5C (本轮)
│   │
│   ├── operator/                             # Operator UX 9 + 1 = 10
│   │   ├── 01-dashboard.html
│   │   ├── 02-sources.html
│   │   ├── 03-switcher.html
│   │   ├── 04-composition.html
│   │   ├── 05-audio.html
│   │   ├── 06-output.html
│   │   ├── 07-recording.html
│   │   ├── 08-graph-designer.html
│   │   ├── 09-health-tree.html
│   │   └── 10-states.html                    # 1 Validation Page
│   │
│   ├── product/                              # Product UX 5+ (M-14 重画/M-17/M-18/P-20/P-28/E-38 在 0.5D)
│   │   ├── M-11-media-library.html
│   │   ├── M-12-asset-detail.html
│   │   ├── M-14-file-transcode.html          # (待 0.5D 重画)
│   │   ├── P-21-encoding-profile.html
│   │   └── P-22-output-profile.html
│   │
│   └── chains/                               # 4 关键操作链
│       ├── chain-1-on-air.md
│       ├── chain-2-failure.md
│       ├── chain-3-playout.md
│       └── chain-4-engineering.md
```

## 2. 5 个 Milestone (历史阶段)

| Milestone | 时间 | 状态 | 交付物 | Git commit |
|---|---|---|---|---|
| **0.5A Operator Semantics** | 2026-08 | 🟢 LOCK FINAL | 9 Core + 1 Validation + 4 Chains + 20 修复 (ERRATA) | `940b7f5` (含 0.5.1) |
| **0.5B Product Surface** | 2026-08 | 🟢 LOCK FINAL | SURFACE_SPEC + i18n + 13 P0 语义 | `0a34bb0` / `50cf5a6` |
| **0.5B.0 Surface Semantics** | 2026-08 | 🟢 LOCK FINAL | 13 P0 语义边界 + I18N_SPEC | `50cf5a6` |
| **0.5B-Closure-1** | 2026-08 | 🟢 LOCK FINAL | 10 项产品化收口 (3-Layer / 4-Tuple / 3-Tier / 9D / H1-H7 / Dependency) | `270daa3` |
| **0.5B.1 P0 Wireframes** | 2026-08 | 🟢 LOCK FINAL | 5 张 P0 wireframe (M-11/M-12/M-14/P-21/P-22) | `3ef6a30` |
| **0.5B.2 Product UX Closure** | 2026-08 | 🟢 LOCK FINAL | 8 P0 + 5 P1 + Design System + UX BASELINE LOCK FINAL | `cec7407` |
| **0.5C Info Arch Closure** | 2026-08 | 🟢 RECONCILED | 目录归并 + Object Vocabulary + Navigation + Product Object Model + Phase 0.6 语义修复 | (0.5C 已完成) |
| **0.5D P0 Product Surfaces** | 2026-08 | 🟡 IN PROGRESS | D1-D6 验收链 + 0.5D.1-.6 Semantic/Execution Closure (对象边界 / TAKE 剥离 / 状态统一 / Click-Path Audit) | `3dd00bd`/`50628a2` (待 0.5D LOCK) |
| **0.5E Global UX Layer** | 2026-08 | 🟢 SEMANTIC LOCKED | Impact Preview (E-50) / Configuration Diff (E-51) / Command Palette (E-52) wireframe 已建 · 待 Phase 4 实施 | `1a1607a`+ |
| **0.5F Final UX Reconciliation** | 2026-08 | 🟢 完成 | 状态统一 / Channel Workspace 三层 / Network INGRESS·EGRESS / Transcode 双模型 / Config vs Runtime / Global Components | `22f2245` |
| **0.5F.1 Final Consistency Sweep** | 2026-08 | 🟢 完成 | D7 TAKE 残留清理 / Channel Type 引用化 / enum 清理 / Source Wizard 统一 / ENCODE SoT / B-13 内嵌 / FINAL 判定标准 | `ed41c29` |
| **0.5F.2 Runtime/Registry/Final Gate** | 2026-08 | 🟢 完成 | Session RESERVED 三轴化 / CH-02 LOCK / Network Availability / Bundle immutable / CD-01 Revision / 5 Click-Path | `e346f9c` |
| **0.5F.3 Runtime/Active-Service/Final Gate** | 2026-08 | 🟢 完成 | Reservation↔Active Service 焊死 / M-17 Runtime-Policy 分离 / CH-02 SDI·Clock·Master·FailoverPolicy / NAVIGATION 状态派生 | `435842e` |
| **0.5F.4 Cross-Surface Consistency** | 2026-08 | 🟢 完成 | B-13 TAKE≠ChangeSet · Clock Compatibility / Video Switch 分支 / CH-02 Audio→P-23 / CD-01 PENDING / M-17 Pipeline 拆分 / lifecycle 更名 / Fixture 统一 | `8dabc86` |
| **0.5F.5 Cross-Surface Final Consistency** | 2026-08 | 🟢 完成 | Source Adapter V0.2/V0.3 统一 / B-13 Spec-HTML SoT / TAKE TARGET 术语 / compact UX / #9 Hard Block / 5 工作流验收 | `cdafe33` |
| **0.5F.6 Final Semantic & Workflow Gate** | 2026-08 | 🟢 完成 | COMPOSITE V0.2/V0.3 修正 / Clock Domain=PTP / 全屏模态→Preflight Sheet / Capability×Runtime 列 / RTMP Used By / SDI+AES67 fixture | `70af9f3` |
| **0.5F.7 Semantic Closure** | 2026-08 | 🟢 完成 | ChangeSet APPROVED canonical 澄清 / TakePreflightResult×API 对齐 / TAKE 三轴 / CH-02 Profile Bundle / E-40 条件 Schema / CD-01 切换策略 / M-17 三轴 Graph + Reservation 闭环 | `cc94542` |
| **0.5F.8 Final Semantic + UX Gate** | 2026-08 | 🟢 完成 (ACCEPTED) | P0: EXECUTION_MODEL §4 旧 READY_TO_TAKE→RUNNING 清除 / D7 ChangeSet 三轴视觉分离; P1: E-40 Network Path compact / P-21 Used By 影响入口 / CH-02 Apply 前 Summary / M-17 Reservation Explain Breakdown | (本轮) |
| **0.5F.9 Micro-Closure** | 2026-08 | 🟢 完成 | P0: E-40 统一 Source Ingest Wizard + E-42 Source Verification Bench; OBJECT_VOCAB/ENCODE 清除 REALTIME_ENCODE JobKind. P1: CD-01 Audio/Output 运行控制·恢复 / Source Freshness / Provenance 折叠 / CH-02 Expected Effective / Surface 三计数 | (本轮) |

## 3. Phase 0.5 LOCK FINAL 判定矩阵

| 维度 | 状态 | 锁定条件 |
|---|---|---|
| V0.2 Architecture | 🟢 LOCK FINAL | 22 review + 7 Health Invariants |
| Phase 0.5A Operator Semantics | 🟢 LOCK FINAL | 9 Core + 1 Validation + 4 Chains + 20 修复 |
| Phase 0.5B Product Surface | 🟢 LOCK FINAL | SURFACE_SPEC + i18n + 13 P0 |
| Phase 0.5B-Closure-1 | 🟢 LOCK FINAL | 10 项产品化收口 |
| Phase 0.5B.1 P0 Wireframes | 🟢 LOCK FINAL | 5 P0 wireframes |
| Phase 0.5B.2 Product UX Closure | 🟢 LOCK FINAL | 8 P0 + 5 P1 + Design System |
| **Phase 0.5C Info Arch** | 🟢 **RECONCILED** | 目录归并 + Object Vocabulary + Navigation + Product Object Model + 0.6 语义修复 + README 统一 |
| Phase 0.5D P0 Product Surfaces | 🟡 **IN PROGRESS (0.5D.1-.6 闭环)** | D1-D6 + 对象边界 / TAKE 剥离 / 状态统一 / Click-Path Audit; 待 0.5D LOCK |
| Phase 0.5E Global UX Layer | 🟢 **SEMANTIC LOCKED** | Impact Preview (E-50) + Configuration Diff (E-51) + Command Palette (E-52) |
| Phase 0.5F Final UX Reconciliation | 🟡 **IN PROGRESS (本轮)** | 状态统一 / Channel Workspace 三层 / Network INGRESS·EGRESS / Transcode 双模型 / Config vs Runtime / Global Components |

## 4. Phase 0.5 LOCK FINAL 最终条件

只有满足下面 6 项, 才能正式宣布 **Phase 0.5 = UX BASELINE LOCK FINAL**:

1. ✅ **0.5C RECONCILED** (0.5C 已完成)
2. ⛔ **0.5D LOCK** (0.5D.1-.6 语义/执行闭环完成, 待 LOCK 声明)
3. ⛔ **0.5E LOCK** (SEMANTIC LOCKED + wireframes E-50/51/52, 待 LOCK 声明)
4. ⛔ **0.5F/0.5F.1/0.5F.2/0.5F.3 全部完成** (状态统一 / Channel Workspace / Network / Transcode / Config vs Runtime / Components / FINAL 判定标准 / Reservation↔Active Service / M-17 三轴 / Navigation 派生; 关键 surface 全部升 LOCK, Spec-only 保持 SPEC)
   - **状态三语义 (0.5F.3 P1-7):** **Surface Lock** (页面/原型已冻结, 见 `SURFACE_REGISTRY.yaml`) · **Workflow Acceptance** (点击路径验收, 见 RECONCILIATION §T) · **Milestone Lock** (阶段正式 LOCK 声明)。Registry 关键 surface 已全 LOCK ≠ Milestone 已 LOCK — Milestone Lock 需 0.5D/0.5E 正式声明。
5. ✅ **所有 README / MILESTONES / SURFACE_SPEC / PIA / Registry** 状态完全同步 (0.5F F1)
6. ✅ **Object Vocabulary** + **Product Object Model** + **Navigation** 3 文档 SEMANTIC LOCKED
7. ✅ **GitHub README** 反映 4 域 + `SURFACE_REGISTRY.yaml` 计数 (55 wireframe + 1 Spec E-41 = 56), 不再有 "9 Core Pages" "44" 等历史残留

## 5. 进入 Phase 0.6 的条件

Phase 0.5 = UX BASELINE LOCK FINAL 之后:

```
V0.2 Architecture (LOCK FINAL)
  + Phase 0.5 UX Baseline (LOCK FINAL)
    = Phase 0.6 Executable Acceptance Spec 可以开始
```

**禁止:** 在 Phase 0.5 LOCK FINAL 之前开 Phase 0.6, 否则测试会因 UI / 文档未锁而反复改。

## 6. 文档一致性快照 (Phase 0.5C 提交后必同步)

| 文档 | 顶层声明 | 与本 Milestone 表一致? |
|---|---|---|
| `README.md` (根) | "Phase 0.5 = LOCK FINAL" 前的所有 milestone 状态 | ⛔ 0.5C 提交后必改 |
| `docs/phase-0.5/README.md` | "Phase 0.5 = UX BASELINE LOCK FINAL" | ⛔ 0.5D 完成后才能写 |
| `docs/phase-0.5/SURFACE_SPEC.md` | 4 域 + `SURFACE_REGISTRY.yaml` + M-14/M-17 拆分 | ⛔ 0.5C 提交后必改 |
| `docs/phase-0.5b/README.md` | (已删除) | ✅ 0.5C 删除 |
| `docs/phase-0.6/README.md` | `< 100ms` → `target_failover_time_ms` | ⛔ 0.5C 提交后必改 |

---

**VBMF Contributors** · VBMF Phase 0.5 Milestones V0.1 · Phase 0.5C Information Architecture Closure
