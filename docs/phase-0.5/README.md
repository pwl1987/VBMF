# Phase 0.5 — UX Baseline (0.5A + 0.5B + 0.5C 统一)

> **状态 (派生自 [`MILESTONES.md`](MILESTONES.md) SoT)**: 🟢 **Phase 0.5 UX BASELINE = LOCK FINAL** (2026-08-25) — 0.5C RECONCILED→LOCK · 0.5D LOCK · 0.5E LOCK · 0.5F.1-.8 完成 · **0.5F.9 收口补丁 (2 P0 + 5 P1)** · **0.5F.10 Source & Runtime Safety 收口 (2 P0 + 7 P1)** · **0.5F.11 Final Consistency & Safety Closure (2 P0 + 4 P1)** · **0.5F.13/0.5F.14/0.5F.15/0.5F.16/0.5F.17 Lock Semantics Reconciliation** · **0.5F.18/0.5F.19 Documentation Coherence Sweep (计数 32/24 + 15 对象焊死)** · **→ Phase 0.6 Executable Acceptance**
>
> **收口口径区分 (0.5F.19 焊死)**: **Latest Semantic Milestone = 0.5F.17** (语义/锁定义终态) · **Latest Documentation Reconciliation = 0.5F.18 / 0.5F.19** (文档一致性补丁, 不改变语义)。两者不互斥: F17 封语义, F18/F19 封文档自洽。
>
> **顶层入口**: 整个 Phase 0.5 的"对外"权威 README, 之前 `phase-0.5b/README.md` 已删除
>
> **Phase 0.5 LOCK FINAL 已达成** (2026-08-25, 最新收口 0.5F.17 Lock Semantics Reconciliation): 0.5C + 0.5D + 0.5E 三 LOCK + 0.5F.1-.8 + 0.5F.9~0.5F.17 全部完成, 五条 E2E 工作流复验通过 → **Phase 0.6 Executable Acceptance** (Reference A1/A2/B + Fault Injection + 7 Health Invariants + 五条真实 E2E 验收 + AC-03B)
>
> **Phase 0.5 FINAL 判定标准 (0.5F.1 定义, 0.5F.17 焊死 LOCK SEMANTICS):**
> 1. 0.5C + 0.5D + 0.5E LOCK + 0.5F/0.5F.1 完成;
> 2. **关键 surface 全部 LOCK**: CD-01 WS/Detail · B-13 · CH-01/02/02B · M-14/M-17 · E-40/E-42 · P-20/21/22/28 · E-37/38 · E-50/51/52 · D7 (Registry 已全部升 LOCK, 0.5F.1);
> 3. **Spec-only surface 保持 SPEC** (E-41 / P-23..27 / E-31..36 / O-41..45 / A-51..55) = Semantic Contract, Phase 4 实施 — 不因"每个 Spec 都要有 HTML"而无限扩张 (0.5F.17: LOCK FINAL ≠ 100% Wireframe Complete, SPEC 不阻塞 Phase 0.6)。

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
| **0.5C** Info Arch Closure | 🟢 LOCK FINAL | 目录归并 + Object Vocabulary + Navigation 4 域 + Product Object Model + 0.6 语义修复 |
| **0.5D** P0 Product Surfaces | 🟢 LOCK FINAL | D1-D6 验收链 + 0.5D.1-.6 Semantic/Execution Closure (对象边界 / TAKE 剥离 / 状态统一 / Click-Path Audit) · **0.5D LOCK** |
| **0.5E** Global UX Layer | 🟢 LOCK FINAL | Impact Preview (E-50) / Configuration Diff (E-51) / Command Palette (E-52) — wireframe 已建, 待 Phase 4 实施 · **0.5E LOCK** |
| **0.5F** Final UX Reconciliation | 🟢 完成 | 状态统一 / Channel Workspace 三层 / Network INGRESS·EGRESS / Transcode 双模型 / Config vs Runtime / Global Components |
| **0.5F.1** Final Consistency Sweep | 🟢 完成 | D7 TAKE 残留清理 / Channel Type 引用化 / enum 清理 / Source Wizard 统一 / ENCODE SoT / B-13 内嵌 / FINAL 判定标准 |
| **0.5F.2** Runtime/Registry/Final Gate | 🟢 完成 | Session RESERVED 三轴化 / CH-02 LOCK / Network Availability / Bundle immutable / CD-01 Revision / 5 Click-Path |
| **0.5F.3** Runtime/Active-Service/Final Gate | 🟢 完成 | Reservation↔Active Service 焊死 / M-17 Runtime-Policy 分离 / CH-02 SDI·Clock·Master·FailoverPolicy / NAVIGATION 状态派生 |
| **0.5F.4** Cross-Surface Consistency | 🟢 完成 | B-13 TAKE≠ChangeSet · Clock Compatibility / Video Switch 分支 / CH-02 Audio→P-23 / CD-01 PENDING / M-17 Pipeline 拆分 / lifecycle 更名 / Fixture 统一 |
| **0.5F.5** Cross-Surface Final Consistency | 🟢 完成 | Source Adapter V0.2/V0.3 统一 / B-13 Spec-HTML SoT / TAKE TARGET 术语 / compact UX / #9 Hard Block / 5 工作流验收 |
| **0.5F.6** Final Semantic & Workflow Gate | 🟢 完成 | COMPOSITE V0.2/V0.3 修正 / Clock Domain=PTP (Quality 维度) / 全屏模态→Preflight Sheet / Capability×Runtime 列 / RTMP Used By / SDI+AES67 fixture |
| **0.5F.7** Semantic Closure | 🟢 完成 | ChangeSet APPROVED canonical 澄清 / TakePreflightResult×API 对齐 / TAKE 三轴不改 Lifecycle / CH-02 Profile Bundle / E-40 条件 Schema / CD-01 切换策略 / M-17 三轴 Graph + Reservation 闭环 |
| **0.5F.8** Final Semantic + UX Gate | 🟢 完成 (ACCEPTED) | P0: EXECUTION_MODEL §4 旧 READY_TO_TAKE→RUNNING 清除 / D7 ChangeSet 三轴视觉分离; P1: E-40 Network Path compact / P-21 Used By 影响入口 / CH-02 Apply 前 Summary / M-17 Reservation Explain Breakdown · **Phase 0.5 LOCK FINAL 达成** |
| **0.5F.9** Micro-Closure (LOCK FINAL 后补丁) | 🟢 完成 | P0: E-40 统一 Source Ingest Wizard (Kind Physical/Network/File/Internal/Composite) + E-42 Source Verification Bench per-Kind; OBJECT_VOCAB/ENCODE 清除 REALTIME_ENCODE 作为 JobKind (REALTIME_PROFILE→MEDIA_SESSION). P1: CD-01 Audio 运行控制 / Output 运行恢复 / Source VERIFIED Freshness / Provenance 折叠 / CH-02 Expected Effective / Surface 三计数口径 · **Phase 0.6 前收口** |
| **0.5F.10** Source & Runtime Safety Micro Closure | 🟢 完成 | P0: E-40 真正实现多 Kind Wizard 视觉 (File/Internal/Composite 同级分支 + Composite Graph-backed) + E-42 真正实现 per-Kind Fixture (5 Kind 验收态). P1: Composite Graph-backed / Freshness Policy Default / STALE ON AIR 行为 / Output Disable Impact Preview + Confirm / Audio Action Semantics + L 级 / E-42 Capability Inputs / M-17 UI 名 Realtime Session · **Phase 0.6 前最终收口** |

完整 milestone 表见 [`MILESTONES.md`](MILESTONES.md)。

## 2. 4 域顶层导航 (Phase 0.5C 锁定)

UI 顶层导航改为**业务域**, 不用**编号**:

| 域 | 主要用户 | 包含对象 |
|---|---|---|
| **BROADCAST** (直播) | Operator / Director | Channel, Source, Graph, Route, Session, Variant |
| **MEDIA** (媒体) | Content Manager / Editor | Asset, Asset Version, Job (FILE_TRANSCODE/PROBE/QC/UPLOAD/ARCHIVE) |
| **ENGINEERING** (工程) | Engineer / SRE | Profile (8), Profile Bundle, ChangeSet, Preflight, Hardware, Clock, Health, Incident |
| **ADMIN** (管理) | Admin | User, Role, Permission, Audit, System Setting |

⛔ **Profiles / Operations 不再是顶层域** — 全部进 ENGINEERING 域。

详细导航模型见 [`NAVIGATION.md`](NAVIGATION.md)。

## 3. 3 大对象组合层 (Phase 0.5C 锁定)

```
Profile (Policy / 跨 Channel 共享)
   ↓ 1:1 引用
Profile Bundle (Composition / 1 Channel 1 Bundle, 8 Profile 引用)
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
├── SURFACE_REGISTRY.yaml           ← 页面计数唯一事实源 (32 LOCK wireframe + 24 SPEC = 56, 0.5F.17 纠错; SPEC = Phase 4 Implementation Surfaces)
├── EXECUTION_MODEL.md              ← 执行链/时序唯一事实源 (0.5D.3b)
├── SURFACE_SPEC.md                 ← V0.2 架构对象 → UI 表面完整映射 (计数由 SURFACE_REGISTRY.yaml 派生)
├── DESIGN_SYSTEM.md                ← V0.2 Console Design System (0.5F.17 校正版本头; Historical V0.1)
├── I18N_SPEC.md                    ← V0.2 i18n Contract (与 OBJECT_VOCABULARY SEMANTIC 0.2 对齐; Historical V0.1)
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

## 5. 页面架构 (唯一权威: SURFACE_REGISTRY.yaml · 0.5F F1 — README 不再手写计数)

> **页面计数由 `SURFACE_REGISTRY.yaml` 唯一派生**。README 不维护任何手写数字 (51/52/53/55/56 一律废止, 只引用注册表)。
>
> ```yaml
> surface_count:
>   source: SURFACE_REGISTRY.yaml
>   derived_at: 2026-08-25 (0.5D.6 终扫 + 0.5E + 0.5F)
>   domains:  {BROADCAST: 13, MEDIA: 8, ENGINEERING: 29, ADMIN: 5}
>   domain_total: 55        # 54 wireframe + 1 Spec (E-41)
>   global_validation: 1
>   total: 56               # 32 LOCK wireframe + 24 SPEC (语义契约锁定, Phase 4 实施) + 全局 Validation, 0.5F.17 纠错
> ```

- 逐条清单: [`SURFACE_REGISTRY.yaml`](SURFACE_REGISTRY.yaml) (id / domain / kind / status / milestone)
- 展示视图: [`NAVIGATION.md`](NAVIGATION.md) §2.5 (域计数表, 与 Registry 派生值一致)
- 域路径: `operator/` (01-07 + CH-01/02/02B + CD-01 + B-13 + M-11..18 + P-20..28 + E-31..52 + O-41..45 + D7 + A-51..55 + 10-states)

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

## 9. 当前 LOCK FINAL 条件 (已于 2026-08-25 全部达成 ✅ · 阶段状态 SoT 规则由 0.5F.11 P0-2 焊死, 最新收口 0.5F.17)

- ✅ **0.5C LOCK FINAL** (4 域 IA + Object Model + 57 决策)
- ✅ **0.5D LOCK FINAL** (M-17 + E-38 + E-37 + P-20 + P-28 + M-18 wireframe + M-14 重画)
- ✅ **0.5E LOCK FINAL** (Impact Preview + Configuration Diff + Command Palette 全部跨域落实)
- ✅ **README / ROADMAP / SURFACE_SPEC / Phase 0.6 README** 状态完全同步 (派生自 MILESTONES.md SoT)
- ✅ **Object Vocabulary + Product Object Model + Navigation** 3 文档 LOCK
- ✅ **GitHub README** 反映 4 域 + `SURFACE_REGISTRY.yaml` 计数 (32 LOCK wireframe + 24 SPEC = 56, 0.5F.17 纠错), 不再有 "9 Core Pages" "44" "0.5B 只定义" 等历史残留

---

**VBMF Contributors** · Phase 0.5 UX Baseline · Phase 0.5C Information Architecture Closure
