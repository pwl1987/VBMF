# Comet Design Handoff

- Change: p07d-event-integration
- Phase: design
- Mode: compact
- Context hash: 88223a118ea6594b6423fe6174dc119c960dc687de65651bc1bb30f506c6cd00

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07d-event-integration/proposal.md

- Source: docs/openspec/changes/p07d-event-integration/proposal.md
- Lines: 1-50
- SHA256: 45b59b920c0aa04e14d8be59f62bb1d3587313f7ce1dc95fae6863205964e552

```md
# p07d-event-integration — 0.7D 事件内消费集成（watchdog 演进）

## Why

RuntimeEvent 平台的生产边（SessionManager 直连 emit + Supervisor 决策事件经注入 sink，0.7C-6 D8 CLOSED）、存储边（组合根单表 `RuntimeEventLog`，两级丢弃不静默）与外送边（`GET /api/v1/events/projection` drain→project→`ApiProjectionResponse`，0.7C-8）已全部建成——但事件**只外送、不内耗**：runtime 自身的健康状态派生（AgentState）仍是 `main.rs` 七处命令式散写（Ready:499 / Capturing:537,1233 / Degraded:1253,1258 / Ready:1274 / ManualRequired:1467,1483），Supervisor 决策输入仍来自轮询快照的命令式调用（watch loop → `report_failure`），且 4 项事件词表（IdentityResolved / SignalVerified / LoopbackVerified / ResourceReservationExpired）至今**零生产站点**（实测 0 个非测试 emit）。这三项正是 0.7C-6 design §4 显式 deferred 的"watchdog 演进"，也是 Phase Map 0.7D 行（"Event Projection / Integration"）在 D8 提前关闭后真正剩余的工作面。

## What Changes

- **Health Reducer 完整实现**：`RuntimeEvent` 流 → `AgentState` 派生收敛为单一纯函数 reducer；`health.rs` 从 Gate 2.1 冻结 skeleton（`#![allow(dead_code)]` 未接线）转为完整实现；`main.rs` 七处命令式散写收敛到 reducer 派生路径。watchdog tick 语义不动（0.7C-6 冻结约束）。
- **Supervisor 事件驱动消费**：决策输入从 RuntimeEvent 流获得（消费循环在接线层，**Supervisor 决策调用面零变更**——保持 0.7C-6 design"Supervisor 回归纯决策引擎"的收口形态）。
- **4 项零生产事件点亮**：接到各自真实语义生产者（IdentityResolved→身份解析路径 / SignalVerified→信号验证路径 / LoopbackVerified→loopback 验证路径 / ResourceReservationExpired→预留过期路径）；不加新事件平面、不改事件词表（EVENT_CONTRACT TD-16 保持）。
- **红线继承**：EventProjection ≠ CanonicalRuntimeState；"不得因 Projection 改变 Runtime 行为"（EVENT_CONTRACT §2）——reducer 是消费侧只读派生，不写回 Graph/Backend 决策。
- **housekeeping（三项，与语义变更同 change，避免孤立噪声提交）**：
  1. `rpc.rs` 陈旧注释修正（0.7C-8 终审裁定并入本 change："No transport yet" → 指向 transport.rs 为当前 HTTP 边界、rpc.rs 为冻结 SoT §14 契约记录不在 wire 路径）。
  2. 清理 3 个已归档阶段的陈旧 change 目录（`p07c-error-model` / `p07c-event-projection` / `p07c-external-api`——归档件完整、tasks.md 差异仅为复选框状态，零信息损失；同时消除 comet resume-probe "multiple active changes" 误判源）。
  3. Phase Map 0.7D 行再锚定（现标签"EventSink 解耦 D8 与此同期"已过时——D8 已 @0.7C-6 关闭）。
- **门禁**：`EVENT-INTEGRATION-RT-01`（名称 design 阶段定稿）三层测试：Unit（reducer 纯函数语义）→ Simulation（Mock 全链事件驱动派生 + Supervisor 消费等价性）→ Hardware（真机生命周期事件流 → AgentState 派生实证）。

### 非目标（显式排除）

- **不重做 Projection API**（`event_projection::project` + 五字段投影 + 投影端点均已 0.7C-6/8 完成——本 change 只加内消费，不碰投影本身）。
- **不做 External Event 投递**（webhook/SSE/Valkey/签名/重试——SoT `EVENT_CONTRACT.md` §2 裁定 Projection 层归属 Control Plane/Fastify；本仓库无控制面服务）。
- **不做持久化/跨进程事件总线**（deferred，类比 D9 durable log 分阶段）。
- **不动 Transport 实现本体**（用户明令；`transport.rs` 零改动，`/health` 响应字段逐字段不变）。
- **不做 D10 多管线 / D14 快照一致性 / D15 流基数**（session/query 债，另行阶段）。
- **不改 Supervisor 决策调用面与 watchdog tick 语义**（0.7C-6 design 冻结）。

### 拆分裁定

不拆分：Health Reducer 与 Supervisor 事件消费共享同一 drain 语义、耦合于同一事件平面（内消费集成是单一 capability）；4 项事件点亮是该平面生产侧的小量补全（4 个 emit 站点接线）；拆开只会制造两个薄 change，违反"不做纯清债 change"纪律。housekeeping 三项无独立交付价值，按"避免孤立文档提交"裁定随本 change 走。

## Capabilities

（`skip_specs: true`——SoT 为 0.7C-6 归档 design §4 显式 deferral 清单（"Health Reducer 完整实现；Supervisor 改事件驱动决策（消费循环属 watchdog 演进）；零生产 4 项点亮"）+ `EVENT_CONTRACT.md` §1/§2 两层事件与投影不改行为约束 + `MEDIA_AGENT_STATE_MACHINE.md` 8 态词汇。与前序全部 change 的 specs 处理一致：specs 目录为空，行为契约锚定冻结架构文档，PHASE_IMPLEMENTATION_MAP 为唯一实施 SoT。）

### New Capabilities

无（见上）。

### Modified Capabilities

无（见上）。

## Impact

- **代码**：`services/media-agent/src/health.rs`（skeleton→完整实现）、`supervisor.rs`（消费接线，调用面不变）、`events.rs`（4 项事件 emit 生产点亮——若生产者锚点在 resolver/signal/loopback/resource 模块则对应接线）、`main.rs`（散写收敛 + 消费循环接线）。
- **测试**：新增三层门禁测试（Unit/Simulation/Hardware）；全部既有门禁回归（SESSION/RESOURCE/IDEMPOTENCY/ERROR/EVENT-PROJECTION/TRANSPORT 等）不破。
- **文档**：`PHASE_IMPLEMENTATION_MAP.md`（0.7D 行再锚定 + 完成态）、债表对应登记、verify 报告。
- **CI**：沿用七 required checks（不新增 context，不降既有）。
- **依赖**：零新 crate（std + 既有 serde_json/uuid）。

```

## docs/openspec/changes/p07d-event-integration/design.md

- Source: docs/openspec/changes/p07d-event-integration/design.md
- Lines: 1-44
- SHA256: e09bc2a44e3458893ea93996fa2767dd56864be610e0c6ebeef4088595049505

```md
# p07d-event-integration — Design（高层框架）

> 深度技术设计（含映射表逐项定稿）在 design 阶段 Design Doc 细化；本文只锁方向性决策与红线。

## Context

见 `proposal.md` Why。当前事件链：生产（SessionManager emit 直连 + Supervisor 决策事件）→ 存储（组合根单表 `RuntimeEventLog`，1024 cap 两级丢弃）→ 外送（transport `GET /api/v1/events/projection` drain→project）。缺口=内消费：AgentState 派生散写在 `main.rs` 七处、Supervisor 输入来自轮询、4 项事件零生产。约束：0.7C-6 四语义（顺序 FIFO / 丢失两级丢弃 / 重复容忍 / failure 隔离）零偷改；transport 本体零改动；`/health` 字段逐字段不变；无新后台线程（沿用既有模式）。

## Goals / Non-Goals

**Goals:**
- AgentState 派生收敛为单一纯函数 reducer（事件切片 → 状态），`health.rs` 转完整实现。
- Supervisor 决策输入接入事件视图（调用面零变更，保持纯决策引擎形态）。
- 4 项零生产事件接到真实语义生产者。
- housekeeping 三项（rpc.rs 注释 / 陈旧目录清理 / Phase Map 再锚定）。

**Non-Goals:**
- 不改 Projection API/投影语义；不做 External Event 投递（归 Fastify）；不做持久化/总线；不动 Transport 本体；不做 D10/D14/D15；不改事件词表与 Supervisor 调用面。

## Decisions

- **D1 Reducer=纯函数**：`reduce(current, events) -> AgentState`，同输入同输出、消费侧只读、不写回 Graph/Backend/Command 路径。备选（拒绝）：每事件回调即时改态（副作用、乱序敏感、难测）；仅文档化散写（不解决缺口）。
- **D2 消费点=watchdog tick 接线层**：沿用既有 tick 驱动模式（无新线程）；reducer 在接线层被调用，`health.rs` 只持纯语义。
- **D3 单日志多消费者 drain 语义（design 阶段定稿，本 change 最关键设计点）**：内消费（reducer）与外送投影端点共享同一 `RuntimeEventLog`，而 `drain()` 是破坏性排空——两消费者直接竞争会互相掏空。候选：非破坏性读取（内消费读视图、外送 drain 不变）／单一消费点分流／内消费持游标。硬约束：transport 本体零改动 + 0.7C-6 四语义零偷改 + `EventProjection ≠ CanonicalRuntimeState`。
- **D4 Supervisor 事件驱动=输入侧演进**：决策输入从事件视图获得（如故障类事件 → `report_failure` 语义等价）；`report_failure/begin_restart/report_recovered` 调用面与决策纯度不变（0.7C-6 收口形态保持）。
- **D5 4 事件点亮锚定真实语义路径**：IdentityResolved→身份解析成功点 / SignalVerified→信号验证通过点 / LoopbackVerified→loopback 验证通过点 / ResourceReservationExpired→预留过期点；只加 emit，不加词表、不加平面。
- **D6 红线守护**：reducer 输出仅观测面（`/health` + watchdog 观测），禁止进入 Command/配置路径（Observation→Configuration 红线）；静态扫描/白名单测试守护。

## Risks / Trade-offs

- [drain 竞争破坏外送投影契约] → D3 在 design 阶段以证据定稿；TRANSPORT-RT-01 与 EVENT-PROJECTION-RT-01 全量回归不破。
- [reducer 派生态与命令式旧路径不一致（状态翻转差异）] → 等价性测试：同场景新旧路径同终态（Simulation 层逐场景断言）。
- [事件不足以派生全部 8 态（如 Backoff 需 Supervisor 决策配合）] → 映射表在 design 阶段逐态定稿；不足态显式声明来源（Supervisor 决策事件已存在）而非造新事件。
- [4 事件点亮发错位置制造噪声] → 每事件锚定语义真实触发点，Simulation 断言 `kind_counts` 增量精确。
- [housekeeping 删目录误删未归档内容] → 已核验：三目录归档件完整、差异仅复选框状态、零信息损失；删除前再跑一次 diff 确认。

## Migration Plan

全部 additive + 行为收敛（散写删除由等价性测试兜底）；单 change revert 即回滚。CI 沿用七 checks 不新增不降。

## Open Questions

- D3 具体形态（非破坏读 vs 分流 vs 游标）——design 阶段以四语义与 transport 零改动约束定稿。
- AgentState 8 态 × RuntimeEvent 词表的完备映射表——design 阶段逐态定稿并测试锁定。

```

## docs/openspec/changes/p07d-event-integration/tasks.md

- Source: docs/openspec/changes/p07d-event-integration/tasks.md
- Lines: 1-76
- SHA256: 60f0b8776cfc6c7334fef319508f75dd2d09ce96cc94f00d0aaac5487444b8ca

```md
# Tasks: Phase 0.7D — p07d-event-integration

四栏纪律：`Contract: 已有(引用冻结文档节号) / Implementation / Verification / Gate`。

## 1. Health Reducer + 消费语义（design.md D1/D2/D3）
- [ ] 1.1 D3 定稿：单日志多消费者 drain 语义（非破坏读 vs 分流 vs 游标），约束=transport 本体零改动 + 0.7C-6 四语义零偷改 + `/health` 字段不变；落 Design Doc
      Contract: 0.7C-6 design §4 deferred（Health Reducer/消费循环）+ EVENT_CONTRACT §2（投影不改 Runtime 行为）
      Implementation: docs（Design Doc 决策记录）
      Verification: Design Doc 含三候选对勘 + 约束核对表
      Gate: design 阶段 guard
- [ ] 1.2 `health.rs`：`reduce(current, events) -> AgentState` 纯函数 + AgentState 8 态 × RuntimeEvent 映射表逐态定稿（不足态显式声明来源，不造新事件）；去 `#![allow(dead_code)]`（Gate 2.1 skeleton → 完整实现）
      Contract: MEDIA_AGENT_STATE_MACHINE.md 8 态词汇 + 0.7C-6 design §4
      Implementation: health.rs
      Verification: Unit 测试（纯函数同输入同输出 + 逐态映射）
      Gate: EVENT-INTEGRATION-RT-01 Unit 层
- [ ] 1.3 `main.rs` 七处命令式散写收敛到 reducer 派生（Ready:499 / Capturing:537,1233 / Degraded:1253,1258 / Ready:1274 / ManualRequired:1467,1483）
      Contract: 0.7 红线 1（Observation≠Configuration——reducer 输出仅观测面）
      Implementation: main.rs 接线
      Verification: 新旧路径等价性测试（同场景同终态，Simulation 逐场景断言）
      Gate: EVENT-INTEGRATION-RT-01 Simulation 层
- [ ] 1.4 Supervisor 事件驱动输入接线（故障类事件视图 → `report_failure` 语义等价）；`report_failure/begin_restart/report_recovered` 调用面与决策纯度零变更
      Contract: 0.7C-6 design §4（"Supervisor 回归纯决策引擎"保持）
      Implementation: main.rs watchdog 接线层
      Verification: 消费等价性测试（事件驱动 vs 轮询快照同决策）+ supervisor.rs 既有测试回归
      Gate: EVENT-INTEGRATION-RT-01 Simulation 层

## 2. 4 项零生产事件点亮（design.md D5）
- [ ] 2.1 定位并接线：IdentityResolved→身份解析成功点 / SignalVerified→信号验证通过点 / LoopbackVerified→loopback 验证通过点 / ResourceReservationExpired→预留过期点；只加 emit，词表/平面零改动
      Contract: EVENT_CONTRACT TD-16（词表冻结）+ 0.7C-6 design §4（"零生产 4 项点亮——登记演进"）
      Implementation: 对应生产者模块（resolver/identity、signal、loopback、resource expiry 路径）
      Verification: Simulation 断言 `kind_counts` 增量精确（无噪声事件）
      Gate: EVENT-INTEGRATION-RT-01 Simulation 层

## 3. housekeeping（三项与语义变更同 change）
- [ ] 3.1 `rpc.rs` 陈旧注释修正（工作区已就绪：指向 transport.rs 为当前 HTTP 边界、rpc.rs 为冻结 SoT §14 契约记录不在 wire 路径）
      Contract: 0.7C-8 终审裁定（rpc.rs 修正并入 0.7D）
      Implementation: rpc.rs（纯注释，0 行代码变化）
      Verification: diff 非注释行数=0
      Gate: PR review
- [ ] 3.2 删除三个陈旧 change 目录（`p07c-error-model`/`p07c-event-projection`/`p07c-external-api`；删前 diff 复核归档件完整）
      Contract: 归档生命周期闭环（archive 目录为权威记录）
      Implementation: git rm 三目录
      Verification: 归档件 diff 零差异 + resume-probe 不再误判 multiple active
      Gate: PR review
- [ ] 3.3 Phase Map 0.7D 行再锚定（去"EventSink 解耦 D8 与此同期"过时标签 → 事件内消费集成）+ 债表登记
      Contract: PHASE_IMPLEMENTATION_MAP=唯一实施 SoT（文档漂移=P0）
      Implementation: PHASE_IMPLEMENTATION_MAP.md + PHASE_0_7A_POST_MERGE_DEBT.md
      Verification: 行内容与实际工作面一致
      Gate: verify 阶段复核

## 4. 三层测试 + 真机 + 交付
- [ ] 4.1 Unit：reducer 纯函数语义（同输入同输出 / 逐态映射 / 事件不足态显式来源）
      Contract: D1/D2
      Implementation: health.rs tests
      Verification: `cargo test -p media-agent --features mock` 新增全绿
      Gate: EVENT-INTEGRATION-RT-01 Unit 层
- [ ] 4.2 Simulation：Mock 全链事件驱动派生 + Supervisor 消费等价 + 4 事件点亮精确计数 + 新旧等价性 + 0.7C-6 四语义回归（evt_proj_rt_01_* 不破）
      Contract: 0.7C-6 四语义零偷改
      Implementation: 集成测试
      Verification: 新增测试全绿 + 既有 evt_proj_rt_01_* 全绿
      Gate: EVENT-INTEGRATION-RT-01 Simulation 层
- [ ] 4.3 Hardware：盒上真机 gate（生命周期事件流 → AgentState 派生实证 + TRANSPORT-RT-01/EVENT-PROJECTION-RT-01 回归——外送投影契约不因内消费破坏）
      Contract: D3 约束（transport 零改动 + 投影端点行为不变）
      Implementation: main.rs gate 段
      Verification: 盒上 VBMF_SESSION_LIFECYCLE=1 真机跑 + 全门禁回归
      Gate: EVENT-INTEGRATION-RT-01 Hardware 层
- [ ] 4.4 盒上全矩阵（fmt apply+check + test×4 feature + clippy -D×4 + build×3 + remove-adapter PROOF）+ CI 七 required checks
      Contract: 验收三层（BOX/CI/RELEASE）
      Implementation: ~/p07_verify.sh + gh CI
      Verification: 矩阵全绿 + 7/7 success（gh api 实查）
      Gate: Merge Gate
- [ ] 4.5 verify（0 CRIT/0 IMP 目标）→ archive → PR → merge → tag `phase-0.7D-event-integration` → 删分支 → memory 更新
      Contract: 归档后修复不开新 change 走原分支纪律
      Implementation: comet verify/archive + gh pr
      Verification: verify 报告 + archive 7/7 + merge commit
      Gate: 全生命周期闭环

```
