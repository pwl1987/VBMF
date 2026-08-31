---
comet_change: p07b-consolidation
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-31-p07b-consolidation
status: final
---

# Design Doc — p07b-consolidation（Phase 0.7B Consolidation：文档对账 + Phase Map + Audit + 债务重分类）

> open design.md D1-D8 实现级细化。**零代码改动**；范围=五文档状态对账 + Phase Map 冻结 + Audit 报告 + 债务重分类。

## 1. Integration Audit 结论（已完成的代码级审计，报告落 reports/）

| 审计区 | 判定 | 关键证据 |
|---|---|---|
| Provider→Canonical 统一 | **CLEAN**（+P2） | `RawInputDescription::from_port`（normalize.rs:137）为唯一装配路径，唯一生产调用 main.rs:355（loopback 证据）；normalize 零 provider 分支；接缝是中性的 `PortRegistry::build`（port.rs:380，源自 manifest+probe+binding，非 BMD SDK 观测）。P2：Mock 无生产 canonical 路径（unification 由测试证明，符合 judge-only 设计但需在报告明示） |
| Canonical→Runtime Intent 旁路 | **CLEAN** | Canonical 类型被 runtime 代码消费为**零**（grep session/pipeline/preflight/graph_intent/rpc/resource/lease/supervisor/health/events/registry = 0 命中）。`SourcePlan.device_number`←Resolver 绑定（pipeline.rs:447←resolver.rs:527-558，冻结 "Binding=Physical→Provider→Runtime Resource" 合法路径，非旁路）；`provider_persistent_id`←binding←ProviderIdentity（SPI 证据层）；`connector`←Manifest 声明经 PortRegistry（绝不从信号推断，port.rs:413 HARD RULE）。preflight/session 零读取 signal/format 字段 |
| Canonical→Backend 直连 | **CLEAN** | 四 canonical 模块 import 仅 serde/uuid/`crate::port`/互相；controller.rs 只消费 `PipelinePlan`；`src_props` 在 pipeline.rs（编排层）而非 canonical 层——依赖方向正确 |
| 接线现状（结构性事实） | **Canonical 与 Runtime 为不相交子图** | 四个 canonical 输出仅在 main.rs:355-416（VBMF_LOOPBACK）打印后 exit(0)；SessionManager API（session.rs:251-276）无任何 canonical 输入；AudioRouteIntent 零非测试构造点。**红线平凡满足——0.7B→0.7C 之间的真正工作是"补 Canonical→Runtime/Policy 边"**（届时本次审计验证的边界（SourcePlan 仅由 Resolver 绑定+manifest 喂养）成为需守护的承载性不变量） |

## 2. PHASE_IMPLEMENTATION_MAP.md（结构）

- 阶段表：Phase / 内容 / 状态 / merge commit / baseline tag / 证据——0.6(d1cfaa9) / 0.7A(adc6f19, 四轮 Hardening) / 0.7B-1(30671b5) / 0.7B-2A(f90c6f8) / 0.7B-2B(04d6f4f) / 0.7B-2C(c574238, **Canonical Media Semantics COMPLETE 待本 Consolidation 收口后正式声明**) / 0.7C / 0.7D / 0.8。
- **0.7 全阶段最高架构红线**（终审 §十七）：任何新模块必须证明——①没有把 Observation 偷变成 Configuration；②没有把 Semantic Intent 偷变成 Execution Plan；③没有把 Canonical 类型重新绑回 Vendor。
- 0.7C 前置顺序（终审 §十六）：Canonical Runtime State → Runtime Query Model → Command Contract → Idempotency → Error Model → Event Projection → External API。
- 声明：本地图是 **Implementation Roadmap**，不改变冻结 Architecture Contract（V0.2/各 FROZEN 契约）。

## 3. 五文档对账表（锚点→动作）

| 文档 | 锚点 | 动作 |
|---|---|---|
| PHASE_0_6_MASTER_PRD.md | L86-90 契约状态行 | AUDIO_ROUTING/CLOCK_TIMECODE → `CONTRACT = FROZEN / PARTIAL`（+注记：2A/2B/2C canonical 层已落地）；三 API 契约保持 NOT_STARTED（0.7C/D） |
| 同 | L137-147 旧 0.7A-G 段 | 段首加取代注记（原文保留）："0.7 实施标签已由 PHASE_IMPLEMENTATION_MAP.md 取代（2026-08-30）；旧 0.7A-G 为规划期标签" |
| README.md | L5-7 阶段导航 | 更新为：0.6 完成（Gate PASSED）+ 0.7A/0.7B 完成（链接 Phase Map）；契约清单保留（P0 契约 IMPLEMENTATION 对齐） |
| 同 | L30 Phase 0.7 段 | "暂不实施" → 0.7A/0.7B 已实施 + Clock/Timecode/Audio canonical 层落地，实施地图指向 Phase Map |
| ROADMAP.md | 状态总览 | 三段式：Historical Architecture（0/0.5 LOCK FINAL 原文）/ Current Implementation（0.6 ✅→0.8 概览+链接 Phase Map，不复制细节）/ Future Product（Phase 1-5/V0.3-V1.0 原文）；Phase 0.6 详细段标题 📋Next→✅ 并加完成注记 |
| PHASE_0_6_IMPLEMENTATION_GAP_MATRIX.md | L15-23 等 | SPI/Session/Resource/Preflight/Registry/Lint/RuntimeEvent/Identity 行：IMPLEMENTATION 列对齐（IMPLEMENTED/PARTIAL）+ Gate 列（PASS/PARTIAL）+ 注记"状态对账 2026-08-30（p07b-consolidation）" |
| PHASE_0_6_ACCEPTANCE_MATRIX.md | L10-13, L24 | ARCH-PORTABILITY-01 FAIL→PASS（CI 门禁+remove-adapter proof 实证）；L24 结论行更新；表尾增补 0.7 门禁六行（证据=各 verify 报告路径） |

## 4. 债务重分类（PHASE_0_7A_POST_MERGE_DEBT.md 增"优先级分组"节）

- **0.7C 前必须**：D2（derive_claims FAIL 化/RESOURCE-RESOLUTION-01）、D4（PortAvailability 精确化）、D5（IdentityBinding 实查/IDENTITY-BINDING-01）、D6（BACKEND-CAPABILITY-01）——External API 的设备/资源/能力问答依赖。
- **延后**：D1（LifecycleJournal）、D3（per-claim TTL）、D7（OnceLock）、D9（create 幂等键→0.7C Command Contract 内解决）、D10（Session 内多 Pipeline）。
- **优先级上调**：D11（Clock Observation Timeline——广播时钟天然时间序列，Clock 策略阶段前置）。
- **合并考虑**：D8（EventSink 解耦）与 0.7D Event Projection 同期。
- **D13 新登记**：`CanonicalTimecode::observe_transitional` 使用 debug_assert（release 不强制）——后续随 timecode 下一触碰点 Result 化/强类型拆分（P1 semantic hardening，不单独开 change）。

## 5. 验证口径

零代码改动 → 盒上矩阵引用 master 基线（134/134/154/134，PR#6 已证）；`python scripts/check_docs.py` 跑通；PR CI 七 checks 实跑为 CI 层证明；`git diff --stat services/` 为空即"零代码改动"核验。
