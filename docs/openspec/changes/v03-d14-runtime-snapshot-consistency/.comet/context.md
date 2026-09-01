# Comet Design Handoff

- Change: v03-d14-runtime-snapshot-consistency
- Phase: design
- Mode: compact
- Context hash: 441b6065bee614d38419adb2dd6f91b260159e318d3d9f6401a5be6ea27f1af9

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/v03-d14-runtime-snapshot-consistency/proposal.md

- Source: docs/openspec/changes/v03-d14-runtime-snapshot-consistency/proposal.md
- Lines: 1-66
- SHA256: 2bef996be3047e9d835a2e9be11840133a36c83fcf0d08db912f07f3d45cfa55

```md
# Proposal — V0.3-1 D14 Runtime Snapshot Consistency

> 阶段定位：用户正式裁定（2026-09-01）"下一项 = V0.3-1 D14 Runtime Snapshot Consistency；Federation implementation 继续 BLOCKED"。
> 本 change 是 V0.3 架构演进链（V0.3-1 … V0.3-6）的第一环：先定义 **Local Runtime Observation ↓ Consistent Snapshot Contract**，Federation State Exchange（V0.3-4）才能定义 Observation Envelope。

## Why

**D14（登记债务，`PHASE_0_7A_POST_MERGE_DEBT.md` 0.7C 终审追加登记 L69）**：`runtime_state()` 是各源（devices/ports/resources/sessions）独立观测的拼合 snapshot，非事务一致；需定义 **source observation time / state version / 一致性语义**。关闭条件 = 独立 change + 三层测试（债务清单纪律，不允许"顺手修"）。

现状实码核查（Contract Probe，2026-09-01，基线 master@4d13265）：

1. `CanonicalRuntimeState` 只有**单一** `generated_at_ms: u64`（`runtime_state.rs:111`）——墙钟 `SystemTime::now()`（`now_ms()` L222-227），**非单调**（NTP 回拨/时区漂移下可倒退），且失败时 `unwrap_or(0)` 静默降级。
2. **无观察序列号**（任何 revision/sequence 语义）、**无进程谱系标识**、**无一致性声明**——消费者无法回答"这份 snapshot 比上一份新吗"、"重启后 revision 可比吗"。
3. `assemble()` 注释宣称"纯函数（无 IO/锁/全局; 同输入恒同输出）"，但体内调用 `now_ms()` 读墙钟（L185）——**纯度声明与实现矛盾**（对"时间输入"而言不纯）。
4. 每次查询（`RuntimeQuery` façade / `GET /api/v1/runtime`）都重新全量装配（`session.rs:769` 唯一生产装配点）——同一进程内连续两次读取之间**无任何可判定的新旧关系**。

**为什么现在**：D14 是唯一不依赖 Control Plane、不需要 Federation、不改变 Runtime ownership、不碰网络协议的基础能力，且是 Federation State Exchange 的直接前置——在版本语义未定义前做 Federation = 传播"没有版本语义的本地观察结果"。

## What Changes

全部为 **additive**（非破坏；EXTERNAL_API_CONTRACT §2 #126"非破坏字段增加不影响旧客户端"）：

1. **`CanonicalRuntimeState` 新增观察信封字段**（wire 与域内同步）：
   - `observation_revision: u64` —— 进程内**单调递增观察序列**：每次生产装配 +1，严格单调；**不是墙钟**，不受 NTP 影响；**重启重置为 0**（诚实谱系语义，与 D9 冻结边界 `RestartBreaksReplay` 同构）。
   - `observation_lineage: Uuid` —— **进程观察谱系**：进程启动时生成一次（UUIDv4），使 `(lineage, revision)` 构成**全序**——跨重启的 revision 不可比问题由 lineage 不同显式化，客户端可机器判定而非靠文档信任。
   - `generated_at_ms` **保留**，语义收窄为**freshness 参考**（装配时刻墙钟；用于新鲜度判断，不用于新旧排序）。
2. **`SessionManager` 成为观察序列唯一 owner**（已为唯一生产装配点，`session.rs:769`）：装配前在单一临界区递增计数器并取谱系值，作为**纯输入**传入 `assemble()`。`assemble()` 语义从"自读墙钟"改为"接收观察参数（revision/lineage/observed_at）"——**F4 纯度矛盾修复**：同输入恒同输出成为真实语义。
3. **`ApiQuerySnapshot` 同构 additive 投影**（`api_boundary.rs`）：`GET /api/v1/runtime` 响应新增同名字段；投影函数保持纯。
4. **一致性声明落契约**：`CanonicalRuntimeState` 的 D14 注释从"登记不实现"改写为关闭态声明——**一致性类 = swept non-transactional**（各源在装配点各自加锁观测，跨源无原子性保证；每源观察时刻 = 其加锁读取时刻 ≤ 装配完成时刻）；per-source observation time 的**粒度决策**（是否 per-source 时间戳）在 design 阶段裁定，默认**不加** per-source 字段（无消费方不建抽象——V0.2 纪律；未来 Federation Envelope 消费的是全局 revision）。
5. **三层测试**（D14 关闭条件）：
   - **Unit**：`assemble` 纯性（观察参数为输入）；`observation_revision` 单调递增；重启语义（新 lineage + revision 归零）；顶层键集合测试更新（additive 两字段）。
   - **Simulation**：`SessionManager` 连续两次 `runtime_state()` → revision 严格 +1、lineage 不变；并发装配单临界区无跳号/重复（击穿测试，D9-C 同构纪律）。
   - **Hardware**：真机 gate 实读 `GET /api/v1/runtime` 新增字段在场且单调（盒上 matrix 含此项）。
6. **D14 债务条目关闭**：`PHASE_0_7A_POST_MERGE_DEBT.md` D14 行标 CLOSED @ 本 change（含三层测试证据锚）。

**非目标（用户裁定边界，显式排除）**：不实现 Federation；不实现 SiteId；不实现 Membership；不实现 Authority/Fencing；不修改 `RuntimeEvent`；不修改 `DeviceId`；不实现 Control Plane；不改 `If-Match` 乐观并发语义（EXTERNAL_API_CONTRACT §4 已冻结，本 change 不消费也不修改它）；不实现 V0.2 §1.21 config-apply 的 "Runtime Revision N+1"（那是配置应用域，未实现）。

**术语区分（防三处 revision 词汇纠缠）**：
- D14 `observation_revision` = **本地观察序列**（本 change）。
- V0.2 §1.21 "Runtime Revision N+1" = **config-apply 事务切换**域（Control Plane 时代，未实现）。
- EXTERNAL_API_CONTRACT §4 `If-Match: "revision-N"` = **API 乐观并发**域（已冻结，未实现）。
三者命名自解释、互不替代；本 change 只定义第一种，后两种保持原状。

## Capabilities

（`skip_specs: true` —— SoT = `PHASE_0_7A_POST_MERGE_DEBT.md` D14 + `EXTERNAL_API_CONTRACT.md` §2/§4 + `ARCHITECTURE_V0.2.md` §1.21 + `PHASE_IMPLEMENTATION_MAP`；项目既有 17 个 change 均循此惯例，specs 目录为空。）

### New Capabilities
- `runtime-snapshot-observation`（行为契约落点，spec 级描述由 D14 债务条目 + 本 proposal/design 承担）：snapshot 观察信封（revision 单调性 / lineage 谱系 / 重启重置 / freshness 参考）+ 一致性类声明（swept non-transactional）。

### Modified Capabilities
- 无既有 spec 被修改（specs 目录为空）。

## Impact

- **代码**（`services/media-agent/src/`）：
  - `runtime_state.rs`：struct additive 两字段；`assemble` 签名接收观察参数（纯性修复）；`now_ms()` 处置（移入装配 owner 或保留为纯输入来源）；顶层键集合测试更新。
  - `session.rs`：`SessionManager` 新增观察序列 owner 字段（`AtomicU64` 或等价的单临界区封装）+ lineage 一次性生成；`runtime_state()` 装配前递增。
  - `api_boundary.rs`：`ApiQuerySnapshot` additive 两字段 + 投影透传。
  - `runtime_query.rs`：façade 的 D14 镜像注释同步更新。
  - `main.rs`：RUNTIME-STATE-RT-01 hardware gate 打印兼容（additive 字段可打印可断言）。
- **Wire**：`GET /api/v1/runtime` 响应 additive（非破坏 #126）；旧客户端忽略新字段。
- **消费方**：`RuntimeQuery` façade 语义不变（仍每次全量装配，现获得可判定的 revision 递进）；transport 投影纯函数扩展。
- **未来接口（仅预留，不实现）**：Federation Observation Envelope（V0.3-4）= `SiteId + AgentId + SnapshotRevision + ObservedAt + ProducerEpoch + Freshness + Snapshot`——其中 `SnapshotRevision`/`ObservedAt`/`Freshness` 的本地载体即本 change 的 `observation_revision`/`generated_at_ms`/单调性保证；`SiteId`/`AgentId`/`ProducerEpoch` 属 V0.3-2/V0.3-3，本 change 不引入。
- **构建矩阵**：五套 feature 组合不回退；新增字段 serde 双向（`Deserialize` 需对新字段给默认值策略还是拒绝旧 wire？→ design 阶段裁定，倾向 additive 必填 + 旧客户端本就不解析，agent 自身 wire 无持久化兼容负担）。
- **文档**：`PHASE_0_7A_POST_MERGE_DEBT.md` D14 关闭；`PHASE_IMPLEMENTATION_MAP` V0.3 段（如已建）补 V0.3-1 行——**PHASE_IMPLEMENTATION_MAP 为唯一 SoT，文档漂移 = P0**。

```

## docs/openspec/changes/v03-d14-runtime-snapshot-consistency/design.md

- Source: docs/openspec/changes/v03-d14-runtime-snapshot-consistency/design.md
- Lines: 1-56
- SHA256: 0fd97aa726f47b3dcd30027cf3d197b6c9ebc0bff4f7759fd4a766144e2c22cb

```md
# Design — V0.3-1 D14 Runtime Snapshot Consistency（open 阶段高层框架）

> 深度技术设计（字段级/测试级/门禁级）在 **/comet-design 阶段** 的 Design Doc 细化；本文锁定方案选型与关键决策，build 阶段不得偏离此处选定的方案。

## Context

- `CanonicalRuntimeState`（`runtime_state.rs`）= 5 个 Vec + 单一 `generated_at_ms`（墙钟 `SystemTime`，非单调，`unwrap_or(0)`）。
- **唯一生产装配点** = `SessionManager::runtime_state()`（`session.rs:769`）；`RuntimeQuery` façade（`runtime_query.rs`）与 `GET /api/v1/runtime`（`transport.rs:209`）全部经此路径（本轮实码复核确认：façade 持 `Arc<SessionManager>`，所有 getter → `mgr.runtime_state()`）。
- `assemble()` 现签名接收 5 个源引用并自读 `now_ms()`——"纯函数"注释与墙钟副作用矛盾。
- 冻结契约锚点：EXTERNAL_API_CONTRACT §2 #126（additive 非破坏）、§4 `If-Match: "revision-N"`（乐观并发域，不消费不修改）；ARCHITECTURE_V0.2 §1.21 "Runtime Revision N+1"（config-apply 域，不消费不修改）；D9 先例（`RestartBreaksReplay` 谱系边界 + 单临界区击穿测试纪律）。
- 深度设计前必须保持的契约：`assemble` 纯性（观察参数全部外置为输入）、`SessionManager` 唯一 owner、wire additive、五套 feature 矩阵不回退。

## Goals / Non-Goals

**Goals:**
- 为 snapshot 定义可机器判定的**新旧关系**（单调 revision + 谱系 lineage）与**一致性声明**（swept non-transactional）。
- 修复 `assemble` 纯度矛盾（时间/序列全部成为显式输入）。
- 三层测试关闭 D14（Unit 纯性/单调/重启语义 + Simulation 连续读取/并发击穿 + Hardware 真机字段在场）。

**Non-Goals:**
- 不实现 Federation / SiteId / Membership / Authority / Fencing；不修改 `RuntimeEvent` / `DeviceId`；不实现 Control Plane。
- 不加 per-source observation time 字段（无消费方；swept 声明已充分）。
- 不改 `RuntimeQuery` 公开面（不新增 accessor——无消费方不建 API）；不改 `If-Match` 乐观并发与 config-apply revision 语义。

## Decisions

| # | 决策 | 选定方案 | 备选与否决理由 |
|---|------|---------|----------------|
| D1 | 观察序列 owner | `SessionManager` 内 `AtomicU64`（`fetch_add(SeqCst)` 天然唯一、无跳号、无重复、无锁）+ 构造时一次性 `Uuid::new_v4()` lineage | (a) 全局 static/OnceLock 计数器——否决：全局状态破坏可测性，且装配点唯一（owner 天然存在）；(b) 独立 `ObservationClock` 服务注入——否决：单一装配点，无第二消费方，过度抽象 |
| D2 | 单调性来源 | 进程内计数（构造即单调）；墙钟仅保留为 `generated_at_ms` freshness 参考 | (a) `Instant` 单调时钟——否决：不可序列化、不表达"第 N 次观察"；(b) 墙钟序列——否决：NTP 回拨非单调（Probe F5） |
| D3 | 重启语义 | revision 归零 + **新 lineage**；`(lineage, revision)` 全序，跨重启不可比由 lineage 不同显式化（客户端可机器判定） | (a) 持久化 revision（文件/SQLite）——否决：越界（D9 先例：durable 延后）；(b) 无 lineage 仅文档声明"进程内有效"——否决：文档不可机器校验，重启后比较是客户端陷阱 |
| D4 | 字段命名 | `observation_revision: u64` + `observation_lineage: Uuid`（serde 序列化为 canonical string） | 命名自解释且避开 V0.2 §1.21 "Runtime Revision"（config-apply 域）与 API §4 "revision-N"（乐观并发域）三处词汇纠缠；与未来 Federation Envelope 的映射：`SnapshotRevision ← observation_revision`、`ObservedAt ← generated_at_ms`、`Freshness ← 单调性保证`（映射注记留 design 阶段，字段本 change 不引入） |
| D5 | `assemble` 纯性修复 | 新签名 `assemble(..., obs: &SnapshotObservation)`，`SnapshotObservation { revision: u64, lineage: Uuid, observed_at_ms: u64 }` 为小参数 struct——同输入恒同输出成为真实语义；`now_ms()` 移出 `assemble`，由 `SessionManager::runtime_state()` 在装配前取值 | 备选：保留 `assemble` 签名 + `with_observation()` builder——否决：双构造器诱发误用，单一构造器 + 显式输入更诚实 |
| D6 | wire 兼容 | `ApiQuerySnapshot` additive 同名字段；`CanonicalRuntimeState` 新字段带 `#[serde(default)]`（revision→0, lineage→nil UUID）——双向非破坏：旧客户端不解析新字段（#126），旧序列化 blob 仍可解析 | 备选：新字段必填（无 default）——否决：`Deserialize` 派生已存在，无 default 会使旧 blob 解析失败，非真正非破坏 |
| D7 | 一致性类声明 | **swept non-transactional**：各源在装配点各自加锁观测，跨源无原子性；每源观察时刻 = 其加锁读取时刻 ≤ 装配完成时刻；声明落 `CanonicalRuntimeState` 文档注释（替换现"登记不实现"注释）+ D14 债务条目关闭 | 备选：per-source `observed_at_ms` 字段——否决（无消费方）；Federation 需要时 additive 补 |
| D8 | 键集合测试 | `runtime_state_rt_01_composition_descriptor_not_flattened` 顶层键集合断言更新为 8 键（additive 两字段）——测试随契约同 change 更新（Probe F13） | — |

## Risks / Trade-offs

- **[revision 空洞（crash/重启后序列不连续）]** → 契约只承诺**单调 + 唯一**，不承诺无空洞；文档显式声明（D3）。
- **[三处 "revision" 词汇混淆（本 change / config-apply / 乐观并发）]** → D4 命名自解释 + design 阶段术语映射表；三域互不替代。
- **[wire additive 被误读为 BREAKING]** → `#[serde(default)]` 双向非破坏 + 键集合/序列化测试锁定；EXTERNAL_API_CONTRACT #126 引用入 design 阶段验收场景。
- **[并发装配竞态]** → `AtomicU64::fetch_add` 无锁原子（D1），Simulation 层 8 线程击穿测试（D9-C 同构纪律）锁定唯一性 + 连续覆盖。
- **[纯度修复改变 `assemble` 调用面]** → 调用点唯一（`session.rs:769`）+ 测试内调用同步更新；五套 feature 矩阵编译即验证。

## Migration Plan

- 单 change 单分支交付；无数据迁移（纯内存状态）；无持久化兼容负担（wire 无落盘消费方，`#[serde(default)]` 已兜底）。
- 回滚 = revert commit：字段 additive，未解析新字段的消费者零感知。
- 盒上 matrix 新增：`GET /api/v1/runtime` 字段在场 + 连续读取 revision 单调断言（Hardware 层）。

## Open Questions

（以下均不改变方案/任务分解，可在 design 阶段裁定）
1. Hardware gate 打印面是否同步增加两新字段（倾向：是——字段在场 + 单调断言，随 tasks 落项）。
2. `SnapshotObservation` 参数 struct 的模块归属（`runtime_state.rs` 内 vs 独立小模块——倾向 `runtime_state.rs`，不发明新模块）。

```

## docs/openspec/changes/v03-d14-runtime-snapshot-consistency/tasks.md

- Source: docs/openspec/changes/v03-d14-runtime-snapshot-consistency/tasks.md
- Lines: 1-38
- SHA256: 6954472d12d2026101eeb4f4848aedc1427ec002f2156767dc233dcfb44a1ed2

```md
# Tasks — V0.3-1 D14 Runtime Snapshot Consistency

> 四栏纪律：每项标注 `Contract`（引用冻结文档节号）/ `Implementation` / `Verification` / `Gate` 状态。
> 深度设计在 /comet-design 阶段 Design Doc 细化后，本清单可增补任务组（编号连续），不得重排既有编号。

## 1. 观察信封（SnapshotObservation + struct additive）

- [ ] 1.1 在 `runtime_state.rs` 新增 `SnapshotObservation { revision: u64, lineage: Uuid, observed_at_ms: u64 }` 参数 struct（模块内，不发明新模块——design D5/Open Q2） `Contract: proposal §What-1 / design D5` | `Implementation: 待` | `Verification: 五套 feature 组合编译通过（cargo check 矩阵）` | `Gate: 无`
- [ ] 1.2 `CanonicalRuntimeState` 新增 additive 字段 `observation_revision: u64` + `observation_lineage: Uuid`，均带 `#[serde(default)]`（revision→0, lineage→nil） `Contract: EXTERNAL_API_CONTRACT §2 #126（additive 非破坏）` | `Implementation: 待` | `Verification: Unit 测试——serde 往返含新字段；缺新字段的旧 JSON blob 反序列化成功` | `Gate: 无`
- [ ] 1.3 `assemble` 签名改为接收 `obs: &SnapshotObservation`，`now_ms()` 移出 `assemble`（纯度修复：同输入恒同输出） `Contract: design D5 / proposal §Why-3` | `Implementation: 待` | `Verification: Unit 测试——同 5 源 + 同 obs 两次 assemble 逐字段相等（含 generated_at_ms）` | `Gate: 无`

## 2. 装配 owner（SessionManager 观察序列）

- [ ] 2.1 `SessionManager` 新增观察序列 owner：`AtomicU64`（`fetch_add(SeqCst)`）+ 构造时一次性 `Uuid::new_v4()` lineage `Contract: design D1/D3` | `Implementation: 待` | `Verification: Simulation 测试——连续两次 `runtime_state()` revision 严格 +1、lineage 不变` | `Gate: 无`
- [ ] 2.2 `session.rs:769 runtime_state()` 装配前"递增 → 取值 → 构造 `SnapshotObservation` → 传入 assemble" `Contract: proposal §What-2` | `Implementation: 待` | `Verification: 五套 feature 矩阵编译 + 既有 session/runtime_state 测试零回退` | `Gate: 无`

## 3. Wire 面（ApiQuerySnapshot additive）

- [ ] 3.1 `ApiQuerySnapshot` 新增 additive 同名字段 + `to_api_query_snapshot` 投影透传（保持纯函数） `Contract: EXTERNAL_API_CONTRACT §2 #126` | `Implementation: 待` | `Verification: Unit/Simulation 测试——`GET /api/v1/runtime` 响应 JSON 含 `observation_revision`/`observation_lineage` 且值与域内一致` | `Gate: 无`
- [ ] 3.2 `runtime_query.rs` 头部 D14 镜像注释同步为关闭态（引用 D14 CLOSED + 本 change） `Contract: runtime_query.rs L13-15 既有注释` | `Implementation: 待` | `Verification: 人工复核 + grep 确认两处 D14 注释（runtime_state.rs / runtime_query.rs）语义一致` | `Gate: 无`

## 4. 契约关闭（注释 + 债务账本 + 键集合测试）

- [ ] 4.1 `CanonicalRuntimeState` 的 D14 注释改写为关闭态声明：一致性类 = swept non-transactional（各源加锁读取时刻 ≤ 装配完成时刻，跨源无原子性）+ revision 单调/唯一/不承诺无空洞 + 重启归零新 lineage `Contract: design D7/D3 / proposal §What-4` | `Implementation: 待` | `Verification: 注释与 proposal/design 声明逐句一致（人工复核）` | `Gate: 无`
- [ ] 4.2 `PHASE_0_7A_POST_MERGE_DEBT.md` D14 行标 CLOSED @ v03-d14-runtime-snapshot-consistency（含三层测试证据锚 + 关闭日期） `Contract: 债务清单纪律 L6（独立 change + 三层测试）` | `Implementation: 待` | `Verification: 账本行含 change 名、日期、证据链接（指向 verify 报告）` | `Gate: 无`
- [ ] 4.3 `runtime_state_rt_01_composition_descriptor_not_flattened` 顶层键集合断言 6 键 → 8 键（additive 两字段） `Contract: design D8` | `Implementation: 待` | `Verification: cargo test 该测试通过` | `Gate: 无`

## 5. 三层测试（D14 关闭条件）

- [ ] 5.1 **Unit 层**：assemble 纯性（观察参数为输入）/ `observation_revision` 单调语义 / 重启语义（新 lineage + revision 归零，以构造新 owner 模拟） `Contract: D14 关闭条件（三层测试）` | `Implementation: 待` | `Verification: cargo test --features mock 全绿` | `Gate: 无`
- [ ] 5.2 **Simulation 层**：连续读取 revision 严格递增；8 线程并发 `runtime_state()` 击穿——revision 唯一 + 连续覆盖（无跳号无重复，D9-C 同构纪律） `Contract: design D1/D6 / D9-C 先例` | `Implementation: 待` | `Verification: cargo test --features mock 并发测试通过` | `Gate: 无`
- [ ] 5.3 **Hardware 层**：盒上 matrix 新增——`GET /api/v1/runtime` 两新字段在场 + 连续读取 revision 单调断言（验证脚本不入库） `Contract: D14 关闭条件（三层测试）` | `Implementation: 待` | `Verification: 盒上（lytv@10.30.15.10, bmd,gstreamer 二进制）实跑 PASS，证据入 verify 报告` | `Gate: 盒上 matrix 含此项`

## 6. 验证与交付

- [ ] 6.1 盒上全矩阵（含 fmt check + hardware-test build + 既有 mock 基线 215 不回退） `Contract: 验收口径（BOX/CI/RELEASE 三层）` | `Implementation: 待` | `Verification: 盒上输出全 PASS 截图/日志入 verify 报告` | `Gate: BOX`
- [ ] 6.2 CI 全绿（现有 checks 粒度化核验，非自报） `Contract: 验收口径（CI PASS ≠ Merge Gate PASS，独立核 CI 实跑）` | `Implementation: 待` | `Verification: gh api 实查 required checks 全 green` | `Gate: CI`
- [ ] 6.3 verify 报告（Contract/Implementation/Verification/Gate 四栏纪律表 + D14 关闭证据）→ archive → PR → merge → 删分支 `Contract: 项目交付纪律` | `Implementation: 待` | `Verification: verify 报告落 docs 归档目录；PR merged` | `Gate: RELEASE`

```
