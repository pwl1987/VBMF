# VBMF Phase 0.5 Milestones (历史阶段归档)

> **目的:** 把 `phase-0.5 / phase-0.5b / phase-0.5b.1 / phase-0.5b.2` 这些"目录分层"统一为
> **Phase 0.5 下的历史 milestone**, Git commit 仍然负责版本管理, 目录只表达 `phase / domain / role`。
>
> **Current phase status (2026-08-25):** 🟢 **Phase 0.5 = LOCK FINAL** — 0.5C RECONCILED→LOCK · 0.5D LOCK · 0.5E LOCK · 0.5F.1-.8 完成 · **0.5F.9 收口补丁 (2 P0 + 5 P1)** · **0.5F.10 Source & Runtime Safety 收口 (2 P0 + 7 P1)** · **0.5F.11 Final Consistency & Safety Closure (2 P0 + 4 P1)** · **0.5F.13 Profile Ownership & Variant Delivery** · **0.5F.14 Object Boundary & Channel Workspace** · **0.5F.15 Final Workflow Consistency & Source/Channel UX** · **0.5F.16 SoT & Acceptance Final Reconciliation** · **0.5F.17 Lock Semantics Reconciliation** · → Phase 0.6 Executable Acceptance。
>
> **本文件 = Phase 阶段状态唯一事实源 (SoT)** — README / Root README / docs/phase-0.5/README 仅展示派生状态, 不自行定义阶段状态 (阶段状态 SoT 规则由 **0.5F.11 P0-2** 焊死, 0.5F.16/0.5F.17 继承并回写). SURFACE_REGISTRY.yaml 承担 Surface 计数 SoT.

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

## 2. Phase 0.5 Milestone History

| Milestone | 时间 | 状态 | 交付物 | Git commit |
|---|---|---|---|---|
| **0.5A Operator Semantics** | 2026-08 | 🟢 LOCK FINAL | 9 Core + 1 Validation + 4 Chains + 20 修复 (ERRATA) | `940b7f5` (含 0.5.1) |
| **0.5B Product Surface** | 2026-08 | 🟢 LOCK FINAL | SURFACE_SPEC + i18n + 13 P0 语义 | `0a34bb0` / `50cf5a6` |
| **0.5B.0 Surface Semantics** | 2026-08 | 🟢 LOCK FINAL | 13 P0 语义边界 + I18N_SPEC | `50cf5a6` |
| **0.5B-Closure-1** | 2026-08 | 🟢 LOCK FINAL | 10 项产品化收口 (3-Layer / 4-Tuple / 3-Tier / 9D / H1-H7 / Dependency) | `270daa3` |
| **0.5B.1 P0 Wireframes** | 2026-08 | 🟢 LOCK FINAL | 5 张 P0 wireframe (M-11/M-12/M-14/P-21/P-22) | `3ef6a30` |
| **0.5B.2 Product UX Closure** | 2026-08 | 🟢 LOCK FINAL | 8 P0 + 5 P1 + Design System + UX BASELINE LOCK FINAL | `cec7407` |
| **0.5C Info Arch Closure** | 2026-08 | 🟢 LOCK FINAL | 目录归并 + Object Vocabulary + Navigation + Product Object Model + Phase 0.6 语义修复 | (0.5C 已完成) |
| **0.5D P0 Product Surfaces** | 2026-08 | 🟢 LOCK FINAL | D1-D6 验收链 + 0.5D.1-.6 Semantic/Execution Closure (对象边界 / TAKE 剥离 / 状态统一 / Click-Path Audit) | `3dd00bd`/`50628a2` |
| **0.5E Global UX Layer** | 2026-08 | 🟢 LOCK FINAL | Impact Preview (E-50) / Configuration Diff (E-51) / Command Palette (E-52) wireframe 已建 · 待 Phase 4 实施 | `1a1607a`+ |
| **0.5F Final UX Reconciliation** | 2026-08 | 🟢 完成 | 0.5F.1-.8 + 0.5F.9 (2P0+5P1) + 0.5F.10 (2P0+7P1) + 0.5F.11 (2P0+4P1) + 0.5F.13/0.5F.14/0.5F.15/0.5F.16/0.5F.17: 状态统一 / Channel Workspace / Network / Transcode / Config vs Runtime / Global Components / Source Wizard 动态 / E-42 per-Kind Verification / Composite GraphSpec / REQUIRED→TAKE Safety / Phase Status SoT / SoT & Acceptance Reconciliation / Lock Semantics Reconciliation | `22f2245` + 本轮 |
| **0.5F.1 Final Consistency Sweep** | 2026-08 | 🟢 完成 | D7 TAKE 残留清理 / Channel Type 引用化 / enum 清理 / Source Wizard 统一 / ENCODE SoT / B-13 内嵌 / FINAL 判定标准 | `ed41c29` |
| **0.5F.2 Runtime/Registry/Final Gate** | 2026-08 | 🟢 完成 | Session RESERVED 三轴化 / CH-02 LOCK / Network Availability / Bundle immutable / CD-01 Revision / 5 Click-Path | `e346f9c` |
| **0.5F.3 Runtime/Active-Service/Final Gate** | 2026-08 | 🟢 完成 | Reservation↔Active Service 焊死 / M-17 Runtime-Policy 分离 / CH-02 SDI·Clock·Master·FailoverPolicy / NAVIGATION 状态派生 | `435842e` |
| **0.5F.4 Cross-Surface Consistency** | 2026-08 | 🟢 完成 | B-13 TAKE≠ChangeSet · Clock Compatibility / Video Switch 分支 / CH-02 Audio→P-23 / CD-01 PENDING / M-17 Pipeline 拆分 / lifecycle 更名 / Fixture 统一 | `8dabc86` |
| **0.5F.5 Cross-Surface Final Consistency** | 2026-08 | 🟢 完成 | Source Adapter V0.2/V0.3 统一 / B-13 Spec-HTML SoT / TAKE TARGET 术语 / compact UX / #9 Hard Block / 5 工作流验收 | `cdafe33` |
| **0.5F.6 Final Semantic & Workflow Gate** | 2026-08 | 🟢 完成 | COMPOSITE V0.2/V0.3 修正 / Clock Domain=PTP / 全屏模态→Preflight Sheet / Capability×Runtime 列 / RTMP Used By / SDI+AES67 fixture | `70af9f3` |
| **0.5F.7 Semantic Closure** | 2026-08 | 🟢 完成 | ChangeSet APPROVED canonical 澄清 / TakePreflightResult×API 对齐 / TAKE 三轴 / CH-02 Profile Bundle / E-40 条件 Schema / CD-01 切换策略 / M-17 三轴 Graph + Reservation 闭环 | `cc94542` |
| **0.5F.8 Final Semantic + UX Gate** | 2026-08 | 🟢 完成 (ACCEPTED) | P0: EXECUTION_MODEL §4 旧 READY_TO_TAKE→RUNNING 清除 / D7 ChangeSet 三轴视觉分离; P1: E-40 Network Path compact / P-21 Used By 影响入口 / CH-02 Apply 前 Summary / M-17 Reservation Explain Breakdown | (本轮) |
| **0.5F.9 Micro-Closure** | 2026-08 | 🟢 完成 | P0: E-40 统一 Source Ingest Wizard + E-42 Source Verification Bench; OBJECT_VOCAB/ENCODE 清除 REALTIME_ENCODE JobKind. P1: CD-01 Audio/Output 运行控制·恢复 / Source Freshness / Provenance 折叠 / CH-02 Expected Effective / Surface 三计数 | (本轮) |
| **0.5F.10 Source & Runtime Safety Micro Closure** | 2026-08 | 🟢 完成 | P0: E-40 多 Kind Wizard 视觉 (File/Internal/Composite 分支) + E-42 per-Kind Fixture (5 Kind 验收态). P1: Composite Graph-backed / Freshness Policy / STALE ON AIR / Output Disable Impact Preview / Audio Semantics+L级 / E-42 Capability Inputs / M-17 Realtime Session | (本轮) |
| **0.5F.11 Final Consistency & Safety Closure** | 2026-08 | 🟢 LOCK FINAL | 2 P0 + 4 P1: CD-01 REQUIRED→TAKE BLOCKED + Emergency Override L3 / Phase Status SoT (MILESTONES=SoT) / E-40 动态 Kind 分支 / E-42 5-Kind Validation Profiles / E-40 Composite GraphSpec→Child→Compile / M-17 Realtime Session 命名统一 | `e9ebe6f` |
| **0.5F.13 Profile Ownership & Variant Delivery Closure** | 2026-08 | 🟢 LOCK FINAL | Packaging per-Variant / Output Profile 唯一 SoT / Bundle Change→Impact Preview / 继承链可解释性 | `b4e409f` |
| **0.5F.14 Object Boundary & Channel Workspace Closure** | 2026-08 | 🟢 LOCK FINAL | 清旧双真相 (output_profile_ref→default_output_profile_ref) / M-14 拆 FILE_TRANSCODE·REALTIME SESSION 两链 / Target Asset Version / 全局 Configuration Source Panel / Source·Channel Workspace | `967b522` |
| **0.5F.15 Final Workflow Consistency & Source/Channel UX Closure** | 2026-08 | 🟢 LOCK FINAL | JobKind 5 / SourceKind 11 / CD-01-WS·Detail / AssetVersionRole / StorageDestination / TAKE·FAILOVER / 交付实例化链 / AC-01~04 | `0c8fd0d` |
| **0.5F.16 SoT & Acceptance Final Reconciliation** | 2026-08 | 🟢 LOCK FINAL | MILESTONES/README/POM/SURFACE/NAVIGATION 状态回写 0.5F.11→0.5F.15 / Output Destination UDP Egress Schema / Storage Path Override / AC-03B Temporary Override | `0c8fd0d`+本轮 |

## 3. Phase 0.5 LOCK FINAL 判定矩阵

| 维度 | 状态 | 锁定条件 |
|---|---|---|
| V0.2 Architecture | 🟢 LOCK FINAL | 22 review + 7 Health Invariants |
| Phase 0.5A Operator Semantics | 🟢 LOCK FINAL | 9 Core + 1 Validation + 4 Chains + 20 修复 |
| Phase 0.5B Product Surface | 🟢 LOCK FINAL | SURFACE_SPEC + i18n + 13 P0 |
| Phase 0.5B-Closure-1 | 🟢 LOCK FINAL | 10 项产品化收口 |
| Phase 0.5B.1 P0 Wireframes | 🟢 LOCK FINAL | 5 P0 wireframes |
| Phase 0.5B.2 Product UX Closure | 🟢 LOCK FINAL | 8 P0 + 5 P1 + Design System |
| **Phase 0.5C Info Arch** | 🟢 **LOCK FINAL** | 目录归并 + Object Vocabulary + Navigation + Product Object Model + 0.6 语义修复 + README 统一 |
| Phase 0.5D P0 Product Surfaces | 🟢 **LOCK FINAL** | D1-D6 + 对象边界 / TAKE 剥离 / 状态统一 / Click-Path Audit |
| Phase 0.5E Global UX Layer | 🟢 **LOCK FINAL** | Impact Preview (E-50) + Configuration Diff (E-51) + Command Palette (E-52) |
| Phase 0.5F Final UX Reconciliation | 🟢 **LOCK FINAL** | 0.5F.1-.8 + 0.5F.9/0.5F.10/0.5F.11 + 0.5F.13/0.5F.14/0.5F.15/0.5F.16/0.5F.17 多轮收口 · LOCK FINAL (最新收口 = 0.5F.17 Lock Semantics Reconciliation, 2026-08-25) |

## 4. Phase 0.5 LOCK FINAL 最终条件

满足下面 **FG-01..FG-07 共 7 项** 即 Phase 0.5 **LOCK FINAL** (已于 2026-08-25 全部达成 ✅):

- ✅ **FG-01 · 0.5C RECONCILED→LOCK** (目录归并 + Object Vocabulary + Navigation + Product Object Model + 0.6 语义修复)
- ✅ **FG-02 · 0.5D LOCK** (D1-D6 验收链 + 0.5D.1-.6 语义/执行闭环: 对象边界 / TAKE 剥离 / 状态统一 / Click-Path Audit)
- ✅ **FG-03 · 0.5E LOCK** (Impact Preview E-50 + Configuration Diff E-51 + Command Palette E-52 跨域落实)
- ✅ **FG-04 · 0.5F 全部完成** (0.5F.1-.8 + 0.5F.9/0.5F.10/0.5F.11 + 0.5F.13/0.5F.14/0.5F.15/0.5F.16/0.5F.17 多轮审查收口, 无遗留 P0/P1 UX 矛盾)
  - **状态三语义 (0.5F.3 P1-7, 0.5F.17 焊死):** **Semantic Lock** (对象模型/枚举/Profile 职责焊死, OBJECT_VOCABULARY SEMANTIC LOCKED 0.2) · **Workflow Lock** (主操作链闭环) · **Surface-Contract Lock** (56 surface 契约/边界/SoT 已登记且 status 明确, 见 `SURFACE_REGISTRY.yaml`)。⚠️ **Phase 0.5 LOCK FINAL ≠ 100% Wireframe Complete**: status=LOCK 已交付 wireframe, status=SPEC 为「语义契约锁定 + Phase 4 实施 wireframe」, SPEC 不视为漏画页面, 不阻塞 Phase 0.6 (0.5F.17 P0-2 明确)。
- ✅ **FG-05 · 文档同步** (README / MILESTONES / SURFACE_SPEC / PIA / Registry 状态完全同步, 0.5F F1)
- ✅ **FG-06 · 3 文档 SEMANTIC LOCKED** (Object Vocabulary + Product Object Model + Navigation)
- ✅ **FG-07 · 阶段状态 SoT 一致** (MILESTONES.md = SoT, README/Root/Phase README 派生, 无三套状态; GitHub README 反映 4 域 + SURFACE_REGISTRY.yaml 计数 56 (33 LOCK + 23 SPEC), 无历史残留; 最新收口 = 0.5F.17, 0.5F.16/0.5F.17 P0 已回写状态 SoT)

## 5. 进入 Phase 0.6 的条件

Phase 0.5 = UX BASELINE LOCK FINAL 之后:

```
V0.2 Architecture (LOCK FINAL)
  + Phase 0.5 UX Baseline (LOCK FINAL)
    = Phase 0.6 Executable Acceptance Spec 可以开始
```

**禁止:** 在 Phase 0.5 LOCK FINAL 之前开 Phase 0.6, 否则测试会因 UI / 文档未锁而反复改。

## 5.1 Phase 4 Implementation Surfaces 裁决 (0.5F.17 P1 治理)

> 依据 §3 FG-04 的 **Surface-Contract Lock ≠ 100% Wireframe Complete** 定义, 下列 `status=SPEC` 表面 **不阻塞 Phase 0.6**, 归类为 **Phase 4 Implementation Surfaces** (语义契约已锁定, wireframe 在 Phase 4 实施):

| Surface | 域 | 优先级 (Phase 4) | 说明 |
|---|---|---|---|
| P-23 Audio Profile | ENGINEERING | **P0** | CD-01 只做 Runtime Quick Control, P-23 承担深度 Policy (Layout/Routing/Mix/Loudness) |
| P-25 QC Profile | ENGINEERING | **P0** | 直接影响播出质量闸门 |
| P-27 Edge Policy | ENGINEERING | P1 | Output Egress 边缘策略 |
| P-24 Graphic / P-26 Rights | ENGINEERING | P1 | 播控运行时非阻断 |
| E-32 Preflight / E-33 ChangeSets / E-34 Capability / E-35 Device / E-36 Resource | ENGINEERING | P1 | 工程中枢, 由 AC-03 验收先行验证语义 |
| E-41 Network Path Inspector | ENGINEERING | P1 | 端到端路径逐层诊断, 与 E-42 Source Verify 分层 (0.5F.17 明确非"唯一 SPEC") |
| O-41 Health / O-42 Incident / O-43 Incident Timeline / O-44 Replay / O-45 Benchmarks | ENGINEERING | P1 | 运维闭环, AC-04 验收先行验证语义 |
| A-51 Users / A-52 Roles / A-53 Permissions / A-54 Audit Logs / A-55 System Settings | ADMIN | P2 | 管理后台, Phase 4 实施 |
| M-13 / M-15 / M-16 | MEDIA | P2 | 文件媒资辅助流, 由 AC-02 验收先行验证语义 |
| P-20 "By Channel" Tab | ENGINEERING | Phase 4 | SURFACE_SPEC 原标 0.5G, 现裁决: **归 Phase 4**, 不进 Phase 0.6 验收范围 (0.5F.17 P1-7) |

**裁决结论:** Phase 0.5 冻结 = 上述 Surface 的**契约/边界/SoT 已锁**, 不等于其 wireframe 已交付。Phase 0.6 以 **AC-01~04 + AC-03B** 四条可执行验收链为主, 不再以"页面覆盖率"为指标。

## 6. 文档一致性快照 (Phase 0.5C 提交后必同步)

| 文档 | 顶层声明 | 与本 Milestone 表一致? |
|---|---|---|
| `README.md` (根) | Phase 0.5 = LOCK FINAL (派生自 MILESTONES.md SoT) | ✅ 已同步 (最新收口 0.5F.17) |
| `docs/phase-0.5/README.md` | Phase 0.5 = UX BASELINE LOCK FINAL (派生) | ✅ 已同步 (最新收口 0.5F.17) |
| `docs/phase-0.5/SURFACE_SPEC.md` | 4 域 + `SURFACE_REGISTRY.yaml` + M-14/M-17 拆分 | ✅ 已同步 |
| `docs/phase-0.5b/README.md` | (已删除) | ✅ 0.5C 删除 |
| `docs/phase-0.6/README.md` | `< 100ms` → `target_failover_time_ms` | ✅ 已同步 |

---

**VBMF Contributors** · VBMF Phase 0.5 Milestones V0.1 · Phase 0.5C Information Architecture Closure
