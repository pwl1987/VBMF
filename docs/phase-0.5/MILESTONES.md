# VBMF Phase 0.5 Milestones (历史阶段归档)

> **目的:** 把 `phase-0.5 / phase-0.5b / phase-0.5b.1 / phase-0.5b.2` 这些"目录分层"统一为
> **Phase 0.5 下的历史 milestone**, Git commit 仍然负责版本管理, 目录只表达 `phase / domain / role`。
>
> **本阶段:** 0.5C Information Architecture Closure
>
> **状态:** 🟡 **DRAFT 0.1** — 等待 0.5C LOCK FINAL

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
│   ├── OBJECT_VOCABULARY.md                  # 14 个对象权威定义
│   ├── PRODUCT_OBJECT_MODEL.md               # 3 层组合关系
│   ├── NAVIGATION.md                         # 4 域导航
│   ├── DESIGN_SYSTEM.md                      # 4 State Models + 15 组件
│   ├── I18N_SPEC.md                          # zh-CN + en-US 翻译规范
│   ├── SURFACE_SPEC.md                       # 44 表面完整规范
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
| **0.5C Info Arch Closure** | 2026-08 | 🟡 DRAFT 0.1 (本轮) | 目录归并 + Object Vocabulary + Navigation + Product Object Model + Phase 0.6 语义修复 | (本轮提交) |
| **0.5D P0 Product Surfaces** | (待) | ⛔ 未开始 | M-17 Realtime Transcode + E-38 Hardware + E-37 Clock 升级 + P-20 Profile Center + P-28 Profile Bundle + M-18 Job Detail + M-14 重画 | — |
| **0.5E Global UX Layer** | (待) | ⛔ 未开始 | Impact Preview 全域 / Configuration Diff / Dependency View / Command Palette / Keyboard | (0.5B.2 已部分) |

## 3. Phase 0.5 LOCK FINAL 判定矩阵

| 维度 | 状态 | 锁定条件 |
|---|---|---|
| V0.2 Architecture | 🟢 LOCK FINAL | 22 review + 7 Health Invariants |
| Phase 0.5A Operator Semantics | 🟢 LOCK FINAL | 9 Core + 1 Validation + 4 Chains + 20 修复 |
| Phase 0.5B Product Surface | 🟢 LOCK FINAL | SURFACE_SPEC + i18n + 13 P0 |
| Phase 0.5B-Closure-1 | 🟢 LOCK FINAL | 10 项产品化收口 |
| Phase 0.5B.1 P0 Wireframes | 🟢 LOCK FINAL | 5 P0 wireframes |
| Phase 0.5B.2 Product UX Closure | 🟢 LOCK FINAL | 8 P0 + 5 P1 + Design System |
| **Phase 0.5C Info Arch** | 🟡 **DRAFT** (本轮) | 目录归并 + Object Vocabulary + Navigation + Product Object Model + 0.6 语义修复 + README 统一 |
| Phase 0.5D P0 Product Surfaces | ⛔ 0.5C 后做 | M-15 + E-34 + E-36 + P-20 + P-28 + M-16 + M-14 重画 |
| Phase 0.5E Global UX Layer | ⛔ 0.5D 后做 | Impact Preview + Configuration Diff 全域 |

## 4. Phase 0.5 LOCK FINAL 最终条件

只有满足下面 6 项, 才能正式宣布 **Phase 0.5 = UX BASELINE LOCK FINAL**:

1. ⛔ **0.5C LOCK FINAL** (本轮提交)
2. ⛔ **0.5D LOCK FINAL** (M-15 + E-34 + E-36 + P-20 + P-28 + M-16 + M-14 重画)
3. ⛔ **0.5E LOCK FINAL** (Impact Preview + Configuration Diff + Command Palette)
4. ⛔ **所有 README / ROADMAP / SURFACE_SPEC / Phase 0.6 README** 状态完全同步
5. ⛔ **Object Vocabulary** + **Product Object Model** + **Navigation** 3 文档 LOCK
6. ⛔ **GitHub README** 反映 4 域 + 44 表面, 不再有 "9 Core Pages" "0.5B 只定义" 等历史残留

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
| `docs/phase-0.5/SURFACE_SPEC.md` | 4 域 + 44 表面 + M-14/M-15 拆分 | ⛔ 0.5C 提交后必改 |
| `docs/phase-0.5b/README.md` | (已删除) | ✅ 0.5C 删除 |
| `docs/phase-0.6/README.md` | `< 100ms` → `target_failover_time_ms` | ⛔ 0.5C 提交后必改 |

---

**VBMF Contributors** · VBMF Phase 0.5 Milestones V0.1 · Phase 0.5C Information Architecture Closure
