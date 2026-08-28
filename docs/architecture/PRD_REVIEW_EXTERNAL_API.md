# PRD 评审 — VBMF External Integration & Device Interoperability API PRD（实际 170 条）

> 评审对象：`docs/VBMF External Integration & Device Interoperability API PRD.md`（External Integration Plane）
> 评审日期：2026-08-28（第二轮，严格批判版）
> 对照基线：`ARCHITECTURE_V0.2.md`（LOCK FINAL）、`IMPLEMENTATION_ADDENDUM.md`（Phase 0.6 冻结裁决）、Portability PRD 评审、`services/media-agent` 当前代码
> 符号：✅ 一致｜🟡 合理但未实现（新增平面）｜🔧 合理需补契约｜⚠️ 有问题/需裁剪/与冻结决策冲突

## 0. 事实基线（决定"是否采纳"）
- `src/rpc.rs` 第 4 行：`Rust MUST NOT implement: API gateway, auth, RBAC, config UI, WebSocket aggregation`。现有 RPC 仅 Node↔Rust 边界。
- 搜索 `/api/v1/`、`webhook`、`sse`、`Idempotency` 在 `rpc.rs` **0 命中**。
- **结论：当前完全没有 External Integration Plane**，本 PRD 绝大部分是**未实现的新增平面**。
- 但关键前提：**Control Plane（Fastify/Node）、PostgreSQL、Valkey、Nginx 在当前仓库均不存在**（CODEBUDDY.md："Web 控制台 Phase 4 尚未落地代码"）。本 PRD 大量条款是为一个**尚不存在的层**做详细设计。

## 0.5 评审立场：**本 PRD 不全盘接受**
第一轮评审只做了"边界对标"，结论偏温和（"互补、可采纳"），是**半接受错误**。第二轮严格挑刺，结论修正为：

> 本 PRD **可作为 P1/P2 规划基线**，但**必须裁剪后才能进入实施**。**不全盘接受**。

必须裁剪/外置的条款（否则与 Phase 0.6 冻结裁决直接冲突）：
1. **#140–#150 Multi-site / Agent / Federation** → **P2，外置**。Phase 0.6 冻结裁决明确"P2（DB/Queue/ObjectStore/Auth Adapter）连契约都暂缓""无消费方不建抽象"。当前没有第二个 VBMF 实例、没有跨站点消费方，属无消费方建抽象。
2. **#160–#167 Artifact / 大文件上传 / Recording-Playback** → **P2，外置**。Recording/Playback 消费方（#164/#165）本身也是 P2；在 Phase 0.6/0.7 无 Recording 功能时规划完整上传子系统（presigned URL + chunk + checksum + resume + artifact_id + URI stability）是过度设计。
3. **#157 PostgreSQL 存 Integration Registry** → **P2 存储，外置**。Integration 对象属 Control Plane，但 Domain Repository 抽象尚未建立，此时直接定 PG 是层次倒置。冻结裁决要求 P2 存储"连契约都暂缓"。

## 1. 总体结论（修正）
- 与 Portability PRD **互补不重叠**（Portability=Runtime 内层 P0；本 PRD=External 外层 P1/P2）。
- **质量有两极**：核心原则（三平面、Vendor Neutrality、Data/Control 分离、Adapter 不得篡改 Canonical、Event 不盲目引 MQ）✅ 正确；但**尾部 #138–#170 大量 P2 条款与冻结裁决冲突，且存在过度设计**。
- **采纳范围收窄**：仅 #1–#137（External API / Event / Device Integration 框架 / 安全 / 门禁）可作为 P1 规划；#138–#170 中 P2 部分必须外置。

## 2. 需补建契约（🔧，标注阶段）
| 契约 | 对应 PRD 段 | 阶段 |
|---|---|---|
| `EXTERNAL_API_CONTRACT.md` | #1–#85, #108–#134, #151–#162 | **P1（规划冻结，暂不实施）** |
| `EVENT_CONTRACT.md` | #11–#34, #78–#82, #102–#105, #131, #155 | **P1（规划冻结）** |
| `DEVICE_INTEGRATION_CONTRACT.md` | #87–#107, #154 | **P1（规划冻结）**；#140–#150 多站点部分移到 P2 文档 |

> 注：这 3 份契约已建，但属**规划冻结**，**不进入 Phase 0.6 实施**。Phase 0.6 只实施 P0/P0.5 契约（Portability 侧 4 份 + 既有 6+1）。

## 3. 问题点（⚠️ 边界 / 割裂，第一轮已列）
- **(A)** 两份 PRD 互相未引用 → Master 显式缝合。
- **(B)** RuntimeEvent→External Event 转换层缺失 → Master 补 Projection 层。
- **(C)** Event Bus 选型与 Runtime 解耦：Valkey 仅 External Webhook 队列，非 Runtime 事件源。
- **(D)** PG Integration Registry（#157）属 Control Plane，非 Domain Repository（与冻结裁决 P2 冲突，见 0.5-3）。
- **(E)** Multi-site/Agent（#140–#150）未标 P2（见 0.5-1）。
- **(F)** #136 Vendor Neutrality 改引用 `VENDOR_NEUTRALITY_RULES.md`，不重复。
- **(G)** 三平面术语对齐 Portability #98/#99：Product(External)/Diagnostics/Internal(Agent)。
- **(H)** #158/#159 Data/Control 分离 ✅，Master 显式引用。

## 3.5 严重问题清单（不全盘接受的理由，第二轮新增）
- **(I) P1/P2 混写，无阶段标记**：#138–#170 把 Multi-site/Agent/Artifact/Recording 与 P1 的 External API 塞进同一份 PRD，未分层。读者无法区分"现在做"vs"以后做"。
- **(J) 过度设计 — Event 推送三套并存**：Webhook + SSE + Subscription（#11–#34）并列。对一个零 API 平面的系统，第一阶段只需 Webhook（#156 Valkey 队列）；SSE/Subscription 是 P2 优化，不应与 Webhook 同权。
- **(K) 过度设计 — Artifact 上传子系统无消费方**：#160–#167 是完整文件上传系统，但 Recording/Playback 消费方（#164/#165）本身 P2，当前零消费方 → 违反"无消费方不建抽象"。
- **(L) 为不存在的 Control Plane 做详细实现设计（过早）**：#151–#157 详细规定 Nginx/Fastify/Valkey/PG 的实现形态，但 Fastify/Node/PG/Valkey/Nginx **均不存在**。应显式标注"这些是 P1 才引入的新增依赖/新层"，而非当作既成事实设计。
- **(M) 鉴权悬空**：#108–#111 要求 authenticated/authorized/audited/secret 只存 reference，但"谁做 authz"的承载层（Fastify 应用层）未实现且未规划 → 安全要求定义在一个不存在的层上，悬空。
- **(N) 与 Phase 0.6 冻结裁决冲突（最高优先级）**：冻结裁决"P2 连契约都暂缓""无消费方不建抽象"，但 #140–#150（multi-site/agent）、#157（PG）、#160–#167（ObjectStore Artifact）都是 P2，PRD 却作为"本 API 平面"条款，未外置。会让实施者误以为 0.7 全做。
- **(O) 大量验收无法在当前代码验证**：#78–#82、#102–#105、#131 Event/Trigger 验收、#140–#150 多站点验收，需 Control Plane + 外部系统，当前零基础，且未标 P1/P2。
- **(P) 两份 PRD 各自定义"稳定身份"，未共享 Canonical Identity 契约**：#93/#94（device identity≠IP）、#147（agent_id 稳定）与 Portability #8/#11（device_number 不作主身份）原则一致，但 API PRD 没引用 Portability 的 Runtime 身份模型，各写一套 → 应抽 `CANONICAL_IDENTITY` 契约共享。
- **(Q) 范围过大未拆**：一份 PRD 同时涵盖 External API + Device Integration + Multi-site + Artifact/Recording 四五个不同优先级子系统，应按阶段拆成 P1（API+Event+Device Adapter 框架）与 P2（Multi-site/Agent/Artifact）。

## 4. 逐条采纳状态（修正后）
- **#1–#10 分层/原则**：✅ 一致；三平面 🟡 未实现
- **#11–#34 Query/Command/Event**：🟡 合理；但 Event 三套推送收敛为 Webhook 先（见 J）
- **#35–#54 Event API**：🟡 Webhook 合理；SSE/Subscription 标 P2
- **#55–#67 Resource API**：🟡 对齐 `RUNTIME_RESOURCE_MODEL`
- **#68–#86 Device Integration API**：🟡 合理，无 Adapter 平面
- **#85–#86 Vendor Neutrality/路径**：✅（F 引用既有规则）
- **#87–#107 Integration/Routing/Trigger/Command**：🟡 合理；Adapter 不可篡改 Canonical（#101）✅ 关键；多站点部分（#140–#150）外置 P2
- **#108–#125 Security/Observability**：🟡 合理但鉴权悬空（M）；标 P1
- **#117–#134 Pagination/Error Model**：🟡 标准，应入 `EXTERNAL_API_CONTRACT`
- **#129–#134 Acceptance**：🟡 新增 EXT-* 门禁
- **#135–#137 边界门禁**：✅ ARCH-API-BOUNDARY-01 + Vendor Neutrality + Provider Extension
- **#138–#150 Capability/Federation/Multi-site/Agent**：⚠️ **P2，外置**（N/I/E）
- **#151–#162 Gateway/Protocol/EventBus/Valkey/PG/DataPlane**：🟡 原则正确，但 Control Plane 不存在（L）；PG 标 P2（D）
- **#163–#170 Artifact/Recording/Playback/URI/Event Schema/Docs**：⚠️ **P2，外置**（K/Q）；Event Schema 稳定性（#167/#168）可保留为 P1 原则

## 5. 合理且需采纳项（修正）
1. **文档层**：3 份 API 契约已补建，但标 **P1 规划冻结**，不进 Phase 0.6。
2. **代码层**：External API/Event/Device Adapter 框架属 **P1 新增平面**，前提是先有 Control Plane（Fastify/Node）——该层当前不存在，需作为 P1 前置工作。
3. **关键衔接**：Master 补 `RuntimeEvent→External Event` Projection 层（B）。
4. **必须裁剪**：#138–#170 中 P2 条款（Multi-site/Agent/Artifact/Recording/PG Registry）外置到独立 P2 文档，不进入 0.7 实施清单。

## 6. 与 Portability PRD 的关系（Master 应显式缝合）
```
Domain (Canonical Media Model)              ← Portability P0
  ↑ Contract: HardwareProvider/MediaBackend/RuntimeEvent/Session/Binding/Resource
Runtime (Supervisor, Pipeline, Preflight)   ← Portability P0
  ↑ Control Plane (Fastify, 当前不存在, P1 才引入) ← 本 PRD 前提层
      External API / Event Projection / Integration Registry / Routing / Trigger
Adapters: BMD/AJA Provider, GStreamer/FFmpeg Backend, ONVIF/SNMP/NMOS/GPIO Device Adapter
External: Query/Command/Event/Webhook consumers
```
两份 PRD 各自覆盖一层；本 PRD 的 P2 部分（Multi-site/Agent/Artifact）不应与 P1 平面混在同一实施批次。
