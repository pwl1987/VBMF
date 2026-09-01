---
comet_change: v03-d14-runtime-snapshot-consistency
role: technical-design
canonical_spec: openspec
archived-with: 2026-09-02-v03-d14-runtime-snapshot-consistency
status: final
---

# V0.3-1 D14 Runtime Snapshot Consistency — Design Doc（深度技术设计）

> 上游：`docs/openspec/changes/v03-d14-runtime-snapshot-consistency/{proposal,design,tasks}.md` + handoff（hash 441b6065bee614d38419adb2dd6f91b260159e318d3d9f6401a5be6ea27f1af9）。
> 本文是 open 阶段高层 design.md（D1-D8）的深度细化：观察信封定稿、装配 owner 落点、一致性语义类、测试三层落点、硬件门禁断言。
> 用户正式裁定（2026-09-01）：下一项 = V0.3-1 D14；Federation implementation 继续 BLOCKED；本 change 不实现 Federation / SiteId / Membership / Authority/Fencing，不修改 RuntimeEvent / DeviceId。

## 1. 现状事实（探针 F1-F13 + 本会话实码逐行复核）

- **F1** D14 冻结于 `PHASE_0_7A_POST_MERGE_DEBT.md` L69："runtime_state() 是各源独立观测的拼合 snapshot，非事务一致 | 需定义 source observation time / state version / 一致性语义"；L6 纪律：关闭必须走独立 change + 三层测试，禁止"顺手修"。
- **F4** `CanonicalRuntimeState::assemble()`（runtime_state.rs:117-187）注释自称"纯函数（无 IO/锁/全局; 同输入恒同输出）"，但 L185 `generated_at_ms: now_ms()` 读墙钟（`SystemTime::now()`，L222-227，可 NTP 回拨 + `unwrap_or(0)`）——**"纯"声明与实现不符**，是 D14 债务的代码根因。
- **F5/F7** 全仓 `assemble(` 调用点 = **5**：生产 1（`SessionManager::runtime_state()`，session.rs:769-779）+ 测试 4（runtime_state.rs:279/329/345/357）。生产装配唯一 owner 已验证。
- **F6** `RuntimeQuery`（runtime_query.rs:29-83）是纯 façade（持 `Arc<SessionManager>`，全部 7 个 getter 路由 `self.mgr.runtime_state()`）——每次查询都重新装配一个新快照。
- **F8** `generated_at_ms` 经 `ApiQuerySnapshot`（api_boundary.rs:140-147）上 wire（transport.rs:209 `GET /api/v1/runtime`；main.rs:1268-1276 序列化+反序列化 roundtrip selftest）。
- **F9/F10/F11** additive 非破坏允许（EXTERNAL_API_CONTRACT §2 #126）；`If-Match: "revision-N"` 已冻结属**乐观并发域**（§4 #120/#121）；V0.2 §1.21 "Runtime Revision N+1" 属 **config-apply 域**——三处 "revision" 词表必须消歧（§4.3）。
- **F12** V0.2 对查询新鲜度/staleness **零规则**——freshness 语义设计空间开放，本 change 只定义最小锚点，不发明 staleness 策略。
- **本会话新增实码事实**：
  - `CanonicalRuntimeState`（runtime_state.rs:104-112）= 5 Vec + `generated_at_ms: u64`，derive `Serialize+Deserialize+PartialEq`；**全仓无任何 `CanonicalRuntimeState` 反序列化消费点**（Deserialize 是 wire 对称面声明，非活跃路径）——`#[serde(default)]` 决策不受活跃消费约束，纯 wire 契约保证。
  - `SessionManager::new()`（session.rs:256-267）10 参数；**实码 grep 复核：调用点共 10 = 生产路径 2**（main.rs:786、main.rs:1362）**+ 测试 8**（command.rs:255, error_model.rs:137, event_projection.rs:241, idempotency.rs:251, runtime_query.rs:138, session.rs:1135/1792/1858）。**不动 `new()` 签名**（§3.1 base_port 否决的连带收益：10 调用点零改动）。
  - `SessionManager::now_ms()` 私有 associated fn 已存在（session.rs:289-294，tick 在用）——观察时间戳复用既有实现，不引入第二份墙钟读。
  - uuid crate features = `["v4","v5","serde"]`（Cargo.toml:25）——`Uuid::new_v4()` 与 serde 字符串表示**零新依赖**。
  - 硬件 gate 现态：main.rs:825-831 `serde_json::to_string_pretty(&mgr.runtime_state())` 打印（RUNTIME-STATE-RT-01）；既有 box 脚本对 `/api/v1/runtime` body 字段**零断言**——D14 硬件断言是纯加法。
  - api_boundary.rs:485-492 测试字面量构造 6 字段 `CanonicalRuntimeState`——加字段触发编译级更新（§6.1 R3，编译错误驱动零遗漏）。

## 2. 观察信封定稿（核心数据契约）

### 2.1 `SnapshotObservation`（新结构，落 `runtime_state.rs`）

```rust
/// 观察信封 —— 一次 runtime_state() 装配的观测元数据（D14 关闭定义）。
///
/// **swept, non-transactional, start-ordered**（§4.1）:
/// - `revision`: 进程内单调递增观察序号, 起点 1（0 保留 = absent）;
/// - `lineage`: 进程观察谱系（构造时 UUIDv4 一次）, 重启换新;
/// - `observed_at_ms`: 观测时刻墙钟（毫秒; 新鲜度参考, 非一致性锚点）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotObservation {
    pub revision: u64,
    pub lineage: Uuid,
    pub observed_at_ms: u64,
}
```

- **三字段，无第四字段**。base_port（稳定 port 锚点）候选已否决：属 identity 关切（CANONICAL_IDENTITY §7 偏差按裁定单独登记），混入 D14 会模糊债务边界；且否决后 `SessionManager::new()` 零新参数。
- **模块归属**（OQ2 定稿）：与唯一消费者 `assemble` 同文件（runtime_state.rs）；session.rs 只持有它的两个标量分量（AtomicU64 + Uuid），在 `runtime_state()` 内现场组装 `SnapshotObservation` 传入。

### 2.2 `CanonicalRuntimeState` 加法（wire 契约）

```rust
pub struct CanonicalRuntimeState {
    pub devices: Vec<DeviceRuntimeState>,
    pub ports: Vec<PortRuntimeState>,
    pub resources: Vec<ResourceRuntimeState>,
    pub sessions: Vec<SessionRuntimeState>,
    pub media_semantics: Vec<PortMediaSemantics>,
    pub generated_at_ms: u64,
    /// D14: 进程内单调观察序号（起点 1; 0 = absent, 见 serde default）。
    #[serde(default)]
    pub observation_revision: u64,
    /// D14: 进程观察谱系（UUIDv4; nil = absent）。
    #[serde(default)]
    pub observation_lineage: Uuid,
}
```

- `generated_at_ms` **保留不动**（既有 wire 消费者；语义 = 新鲜度参考）。`observed_at_ms` 是装配语义锚点，当前单实现下二者取同一墙钟读——不删不改既有字段（风险 R2，§6.2）。
- **计数起点 1**（非 0）：`AtomicU64::new(1)` + `fetch_add(1, Relaxed)` → 首个快照 revision=1。0 永久保留给 serde default 的 "absent" 语义——消除"真实首快照 revision 0"与"旧 producer 缺字段 default 0"的 wire 歧义。
- **wire 映射**（为 V0.3-4 Federation 观察信封预留，不实现）：`SnapshotRevision ← observation_revision`、`ObservedAt ← generated_at_ms`、跨进程全序 ← `lineage`（随机 UUID 非时序，跨进程排序必须引入时序字段——V0.3-4 设计空间）。

### 2.3 `assemble` 签名纯化（F4 根因修复）

```rust
pub fn assemble(
    devices: &[DeviceInfo],
    registry: &PortRegistry,
    resources: &ResourceRegistry,
    bindings: &std::collections::HashMap<Uuid, ResolvedDeviceBinding>,
    sessions: &[MediaSession],
    obs: &SnapshotObservation,
) -> Self
```

- 末位新参数 `obs: &SnapshotObservation`；body 内 `generated_at_ms: now_ms()` → `generated_at_ms: obs.observed_at_ms`，新增两字段直取 `obs.revision` / `obs.lineage`。
- `now_ms()` 从 runtime_state.rs **整体删除**——同输入（含 obs）恒同输出，纯函数契约真正成立；墙钟读收敛到装配 owner（session.rs 既有 `Self::now_ms()`）。
- 4 个测试调用点 + 1 个生产调用点全部显式传 `obs`（测试传常量信封，编译错误驱动零遗漏）。

## 3. 序列 owner 与装配顺序

### 3.1 候选对勘（D1 深化）

| 候选 | 机制 | 判定 |
|------|------|------|
| A. 全局 `static AtomicU64` | 进程级单例计数 | 跨 SessionManager 实例共享序号——测试多 mgr 实例时序号互串，语义变成"进程观察序号"而非"该 manager 的观察序号"；且静态可变全局违背本项目"组合根显式持有"纪律 |
| **B. `SessionManager` 字段（选定）** | `AtomicU64` + 构造时 `Uuid::new_v4()` | 与唯一生产装配点同体（F7 验证）；序号/谱系生命周期 = manager 生命周期；`new()` 零新参数（谱系内部生成）；10 调用点零改动（§1 实码复核） |
| C. 独立 `ObservationClock` service | 单独服务注入 | 为两个标量引入第三个 owner 对象——过度设计；无消费者需要独立时钟抽象（F12） |

### 3.2 `SessionManager` 落点（session.rs）

```rust
pub struct SessionManager {
    // ... 既有 11 字段 ...
    /// D14: 观察序号起点 1（0 = absent 保留给 serde default）; 每次 runtime_state() 递增。
    observation_revision: std::sync::atomic::AtomicU64,
    /// D14: 观察谱系（构造一次 UUIDv4; 重启换新 → (lineage, revision) 进程内全序）。
    observation_lineage: uuid::Uuid,
}
```

- `new()` 内：`observation_revision: AtomicU64::new(1), observation_lineage: Uuid::new_v4()`。
- `runtime_state()` 新体（**先 increment 后采集源**）：

```rust
pub fn runtime_state(&self) -> crate::runtime_state::CanonicalRuntimeState {
    // D14: 先取序号（观测开始锚点）, 再采集各源 —— swept, start-ordered（§4.1）。
    let rev = self.observation_revision.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let resources = self.resources.with_inner(|r| r.clone());
    let registry = self.registry.clone().unwrap_or_default();
    let sessions = self.list();
    crate::runtime_state::CanonicalRuntimeState::assemble(
        &self.devices,
        &registry,
        &resources,
        &self.bindings,
        &sessions,
        &crate::runtime_state::SnapshotObservation {
            revision: rev,
            lineage: self.observation_lineage,
            observed_at_ms: Self::now_ms(),
        },
    )
}
```

- **Relaxed ordering 充分性论证**：需要的保证是"每次调用获得唯一且严格递增的序号"——`fetch_add` 的原子性自身保证（任何 ordering 下原子读改写序列不重号不空洞），无需与其他内存排序；快照各字段的新鲜度由各自源锁保证，与计数器 ordering 无关。
- **先 increment 后采集**而非相反：序号标记观测**开始**时刻（fetch_add 完成点），与 §4.1 start-ordered 定义一致；若先采集后 increment，序号将标记观测**结束**，两个并发装配的序号-源值对应关系无法陈述。

## 4. 一致性语义类（D14 关闭定义）

### 4.1 定义：swept, non-transactional, start-ordered

- **swept（扫掠式）**：一次装配对五个源各做一次点克隆（devices Arc 共享 / registry clone / resources lock 内 clone / sessions lock 内克隆 / bindings Arc 共享），各源取点时刻独立、互不原子。
- **non-transactional**：快照整体**不承诺**任何跨源不变量（例如"session 引用的 resource 在该快照中必然呈 Allocated"）——这是现状的诚实登记（F1 债务原文），不是新承诺的放弃。
- **start-ordered（开始全序）**：进程内任意两个快照 S_a(rev_a)、S_b(rev_b)，rev_a < rev_b ⟺ S_b 的观测**开始**（fetch_add）晚于 S_a。这是 D14 新增的唯一全序保证。
- **无 per-field 单调承诺**：S_b 的某源字段可持有相对 S_a 交织更早的值（该源克隆发生在两装配交错窗口）；正常运行中源不变，字段恒等。
- **跨进程无全序**：lineage 为随机 UUIDv4（非时序）；(lineage_A, r) 与 (lineage_B, r') 不可比较。跨进程/跨站排序属 Federation 观察信封（V0.3-4），需引入时序字段——本 change 不预建。
- **无 per-source 时间戳**：F12 已证无新鲜度消费者；每源加时间戳是 YAGNI（D7 维持）。若未来出现 staleness 消费者，加法扩展 `Vec<SourceObservation>` 不破坏本契约。

### 4.2 单调性与新鲜度

- **单调性**：同一进程同一 manager 内，revision 严格 +1 步进、无重号（AtomicU64 原子性）；**不承诺跨重启连续**（重启归 1 + 新 lineage——lineage 变化即可检测代际切换）。
- **新鲜度**：`generated_at_ms`/`observed_at_ms` 是墙钟参考（可 NTP 回拨，**不是**一致性锚点——一致性锚点是 (lineage, revision)）。staleness 判定策略 V0.2 零规则（F12），本 change 不发明。

### 4.3 三 "revision" 词表消歧（F10/F11 冻结面）

| 术语 | 域 | 语义 | 本 change 动作 |
|------|----|------|----------------|
| `observation_revision` | 观察域（D14） | 进程内单调观察序号，起点 1，重启归 1 | 新增 |
| `Runtime Revision N+1`（V0.2 §1.21） | config-apply 域 | 配置应用代际，命令侧 | 不触碰 |
| `If-Match: "revision-N"`（API §4） | 乐观并发域 | 客户端-服务端并发控制 token | 不触碰（后续若接入，token 可取 observation_revision——V0.3-4+ 设计空间，本 change 不实现） |

## 5. 非目标（裁定锁定）

不实现 Federation / SiteId / Membership / Authority / Fencing；不修改 `RuntimeEvent`（EVENT_CONTRACT 偏差 B 单独裁决）；不修改 `DeviceId`（CANONICAL_IDENTITY §7 偏差 A 单独对账）；不建 per-source 时间戳 / staleness 策略 / 跨进程全序 / `If-Match` 接入 / 持久化 revision（D3 维持：重启归零 + 谱系换新）。

## 6. 测试策略（三层 + 门禁断言）

### 6.1 落点表

| 层 | 测试 | 落点 | 断言要点 |
|----|------|------|----------|
| Unit | assemble 纯函数 | runtime_state.rs tests | 同 obs 输入恒同输出（含 generated_at_ms = obs.observed_at_ms）；无墙钟读（代码级：now_ms 已删） |
| Unit | serde 双向非破坏 | runtime_state.rs tests | 新 JSON 含 8 键 roundtrip；**旧 6 键 JSON 反序列化** → revision=0 / lineage=nil（default 生效） |
| Unit | 键集合 | runtime_state.rs:274-310 既有测试更新 | 顶层键 6 → **8**（+observation_revision, +observation_lineage） |
| Unit | 计数起点 | session.rs tests | 首快照 revision=1、次快照=2；lineage 两次调用恒同 |
| Simulation | 并发击穿 | session.rs tests（mock feature） | **8 线程 × 1000 次** `runtime_state()`（同一 manager）→ 收集 8000 个修订号，集合恰为 `{1..8000}`（无重号无空洞——单 critical section 击穿纪律）；lineage 在全部 8000 份快照中恒同（单 manager 恒定性） |
| Simulation | 连续调用 | runtime_query.rs tests | 经 façade 两次 `get_runtime_state()` → revision 严格 +1 |
| Hardware | 盒上矩阵 | 既有全矩阵（fmt + hardware-test + mock 基线 **215 零回归**）+ **新增观察断言**：`/api/v1/runtime` body 含 `observation_revision`（数值 ≥1）与 `observation_lineage`（36 字符 UUID 格式）；同进程两次调用 → lineage 相同、revision 严格 +1 |
| Selftest | wire roundtrip | main.rs:1268-1276 既有 selftest | ApiQuerySnapshot 序列化+反序列化自动覆盖新字段（零改动，编译级） |

### 6.2 风险登记

- **R1 空洞**：进程崩溃导致修订号"空洞"——契约只承诺单调+唯一，不承诺连续；击穿测试锁定存活进程内 {1..N} 恰覆盖。
- **R2 双时间字段同值**：`generated_at_ms` 与 `observed_at_ms` 当前单实现下同值——wire 保留前者（既有消费者），语义锚点用后者；不合并（合并 = 删既有字段 = 破坏 additive 方向）。
- **R3 调用点遗漏**：`assemble` 签名变更（生产 1 + 测试 4 调用点）+ 1 个 struct 字面量（api_boundary.rs:485 测试字面量，6 字段 → 8 字段）全部由**编译错误**驱动更新，零静默遗漏（实码复核：`CanonicalRuntimeState {` 字面量全仓仅此 1 处）；生产路径 `to_api_query_snapshot` 加法投影（api_boundary.rs:156）属设计内改动，非编译修复。`cargo check` 即门禁。
- **R4 lineage 随机性**：UUIDv4 理论碰撞概率可忽略（122 bit）；测试断言稳定性（同 mgr 恒同）而非具体值。

## 7. 迁移与回滚

- 单 commit 交付；无数据迁移（内存态）；旧客户端忽略新字段（additive）；旧 JSON 反序列化走 serde default（§6.1）。
- 回滚 = `git revert` 单 commit；无 wire 兼容残留（新字段消失，default 面自然退回）。
- 债务账本：`PHASE_0_7A_POST_MERGE_DEBT.md` D14 行状态 → **CLOSED**（关闭证据 = 本 Design Doc + 三层测试 + 盒上矩阵）；契约注释（runtime_state.rs:101-103 / runtime_query.rs:13-15）从"登记不实现"改写为"已关闭"状态描述。

## 8. 交付序列（对应 tasks.md 六组）

§1 观察信封（SnapshotObservation + 2 加法字段 + assemble 纯化）→ §2 装配 owner（AtomicU64 + lineage + increment 先采集后）→ §3 Wire 面（ApiQuerySnapshot 加法 + runtime_query 注释同步）→ §4 契约关闭（注释改写 + 债务账本 CLOSED + 8 键测试）→ §5 三层测试 → §6 验证交付（盒上全矩阵 → CI gh api 实查 → verify → archive → PR → merge → 删分支）。
