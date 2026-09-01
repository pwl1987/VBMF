---
change: v03-d14-runtime-snapshot-consistency
design-doc: docs/superpowers/specs/2026-09-01-v03-d14-runtime-snapshot-consistency-design.md
base-ref: 4d13265bcb8e31314f08d04543255b8222724f9c
---

# V0.3-1 D14 Runtime Snapshot Consistency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 关闭 D14 债务——为 `CanonicalRuntimeState` 快照引入机器可判定的观察信封（进程内单调 `observation_revision`，起点 1 + 进程生命周期稳定 `observation_lineage` UUIDv4），把 `assemble` 修复为真正的纯函数（显式 `SnapshotObservation` 输入，删隐式墙钟读），`ApiQuerySnapshot` additive 投影两字段，并以三层测试（Unit / Concurrency Simulation / Hardware Gate）+ 债务账本 CLOSED 收尾。

**Architecture:** `SessionManager`（唯一生产装配点，session.rs:769）持有 `AtomicU64` 观察计数器（初值 1，0 永久 = absent）+ `new()` 内一次性生成的 `Uuid::new_v4()` lineage（`new()` 保持 10 参数签名 → 10 个调用点零改动，Design Doc §1 实码复核）。`runtime_state()` **先** `fetch_add` 取序号 **再** 采集五源（swept, start-ordered，Design Doc §3.2/§4.1），现场构造 `SnapshotObservation` 传入 `assemble`；`assemble` 签名追加第 6 参 `obs: &SnapshotObservation`，`generated_at_ms` 改取 `obs.observed_at_ms`，墙钟读收敛到装配 owner 既有 `Self::now_ms()`（session.rs:289，复用不新造）。Wire 面 additive 非破坏：`CanonicalRuntimeState` 新字段带 `#[serde(default)]`（旧 6 键 JSON → revision=0 / lineage=nil，EXTERNAL_API_CONTRACT §2 #126）。一致性类 = **swept non-transactional**（Design Doc §4.1）；本 change 不加 per-source timestamp / vector clock / 跨进程全序 / If-Match 接入 / Federation 语义（用户纪律 #6）。

**Tech Stack:** Rust（`services/media-agent`），serde/serde_json（derive），uuid v1（features `v4`+`v5`+`serde`，Cargo.toml:25 既有零新依赖），cargo feature 矩阵（default / simulation / mock / bmd,gstreamer / hardware-test），CI = `.github/workflows/media-agent.yml` 七个 required jobs。

---

## 0. 输入、事实基线与纪律映射

### 0.1 输入（本计划唯一事实源，冲突时以"已验证实码事实"为准——用户纪律 #1）

| 输入 | 路径 |
| --- | --- |
| Design Doc（canonical 深度设计，zh-CN） | `docs/superpowers/specs/2026-09-01-v03-d14-runtime-snapshot-consistency-design.md` |
| 任务边界（四栏纪律，本计划不得扩展） | `docs/openspec/changes/v03-d14-runtime-snapshot-consistency/tasks.md` |
| 上游契约参照 | 同目录 `proposal.md` / `design.md`（D1-D8 决策） |
| 债务账本（D14 行） | `docs/architecture/PHASE_0_7A_POST_MERGE_DEBT.md` L69 |
| CI 门禁定义 | `.github/workflows/media-agent.yml` |
| 盒上 gate 先例 | `~/transport_hw_gate.sh`（盒 10.30.15.10 本地，**未入库不提交**）；硬件测试入口按 `VBMF_*` env 模式（main.rs:114/133/274/731 先例） |

### 0.2 已验证实码事实（base-ref = `4d13265bcb8e31314f08d04543255b8222724f9c` = master 0.7D；2026-09-02 Build 前逐行复核，零漂移）

- `src/runtime_state.rs`（364 行）：`CanonicalRuntimeState`(L104-112) = **6 字段**（devices/ports/resources/sessions/media_semantics 五 Vec + `generated_at_ms: u64`），derive `Debug,Clone,PartialEq,Serialize,Deserialize`；D14 注释 L101-103（"登记不实现"）；`assemble(...)`(L117-187) 五参，L185 `generated_at_ms: now_ms()`；私有 `now_ms()`(L222-227) 读 `SystemTime`；测试模块 `#[cfg(all(test, feature = "mock"))]`(L229-364)，3 个 `#[test]`：`runtime_state_rt_01_composition_descriptor_not_flattened`(L274-310，**断言恰好 6 个顶层键** L288-299)、`runtime_state_rt_01_binding_only_production_grade`(L313，assemble 调用 L329/L345)、`runtime_state_rt_01_session_projection_and_resource_states`(L353，assemble 调用 L357)；测试助手 `world()`(L259-271) / `input_port()`(L238-257)。
- `src/session.rs`(1894 行)：`SessionManager`(L238-250) = **11 字段**；`new()`(L256-267) = **10 参数**，带 `#[allow(clippy::too_many_arguments)]`；私有 `fn now_ms()`(L289-294) 已存在（tick 在用）；`runtime_state()`(L769-780) = **唯一生产 producer**，体为 resources clone → registry clone → `self.list()` → `assemble(&self.devices,&registry,&resources,&self.bindings,&sessions)`；测试模块 L1001 起（18 个 `#[test]`），测试助手 `manager_with()`(L1121-1146) / `mock_manager()`(L1148-1150) 经 10 参 `new()` 构造。
- `src/api_boundary.rs`(573 行)：`ApiQuerySnapshot`(L140-147) = **6 字段**（含 `generated_at_ms`），derive `Debug,Clone,Serialize,Deserialize,PartialEq,Eq`；`to_api_query_snapshot`(L149-164) 唯一投影，L162 `generated_at_ms: state.generated_at_ms`；测试字面量 `CanonicalRuntimeState { ...6 字段... }`(L485-492) 在 `api_rt_01_to_api_query_models`(L484-509) 内——**全仓唯一 `CanonicalRuntimeState {` 结构体字面量**。
- `src/runtime_query.rs`(317 行)：`get_runtime_state()`(L39-41) 仅委托 `self.mgr.runtime_state()`，无结构体构造；D14 镜像注释 L13-15（"登记不实现"）。
- **调用点计数（grep 复核，排除 `.mimosa` 基线副本）**：`SessionManager::new(` = **10 处**（生产 2：main.rs:786、main.rs:1362；测试 8：command.rs:255、idempotency.rs:251、runtime_query.rs:138、session.rs:1135/1792/1858、error_model.rs:137、event_projection.rs:241）；`CanonicalRuntimeState::assemble(` = **5 处**（生产 1：session.rs:773；测试 4：runtime_state.rs:279/329/345/357）；`ApiQuerySnapshot {` 字面量 = **1 处**（api_boundary.rs:156，投影函数内）。
- Cargo features（Cargo.toml:60-72）：`default=[]`、`simulation`、`bmd-provider`、`gstreamer-backend`、别名 `bmd`/`gstreamer`、`hardware-test=["bmd-provider"]`、`mock`。CI 可跑组合 = default / simulation / mock；`bmd,gstreamer` 与 `hardware-test` 组合需 DeckLink SDK，由 CI secrets job + 盒上矩阵承担。
- 债务账本 D14 行（`docs/architecture/PHASE_0_7A_POST_MERGE_DEBT.md` L69）：`| D14 | **Runtime Snapshot Consistency**：... | 需定义 source observation time / state version / 一致性语义；已作为契约注释标注在 CanonicalRuntimeState | Runtime Query 后续 |`。CLOSED 先例格式见同文件 D8 行（L29）：`~~**D8** ...~~ ✅ **CLOSED @ <change> (<阶段>, <日期>)**: <关闭证据>`。

### 0.3 用户 10 条 Build 硬纪律 → 计划落点映射

| # | 纪律 | 落点 |
| --- | --- | --- |
| 1 | 严格以当前分支实码为准，不采信未再验证数字 | §0.2 事实基线 + 任务 0.1/0.2 基线核验 |
| 2 | 实现前保留"再 Read 一次目标函数"自检点 | 每个实现任务 Step 1 强制 `Read` 目标区域并比对 |
| 3 | revision 进程内单调、首值 1；lineage 进程稳定、重启改变 | 任务 2（`AtomicU64::new(1)` + `Uuid::new_v4()`）+ 任务 5/6 测试锁定 |
| 4 | 不与 V0.2 config revision / If-Match revision 混同 | 任务 7 注释改写 + 任务 9 verify 报告术语消歧表（Design Doc §4.3） |
| 5 | `assemble` 恢复纯函数，不留隐式 SystemTime 读 | 任务 1（`now_ms()` 整体删除 + grep 门禁）+ 任务 5 纯度测试 |
| 6 | 一致性语义保持 swept / non-transactional / start-ordered；不发明 Federation 语义 | 任务 7 注释声明 + 任务 9 范围冻结核查 |
| 7 | `serde(default)` 用**实际旧 6 键 JSON 字符串**反序列化测试 | 任务 1 Step 2 `runtime_state_rt_01_legacy_six_key_json_deserializes_with_defaults` |
| 8 | 并发测试实际击穿 8 线程 × 1000，集合恰为 {1..8000}，lineage 恒同 | 任务 6（`session_rt_01_observation_revision_8x1000_concurrency_pierce`） |
| 9 | Hardware Gate 只加 D14 断言（revision≥1 / lineage UUID 格式 / 两次同 lineage 严格 +1），不改既有采集行为 | 任务 8（`transport_hw_gate.sh` 盒上脚本加法，不入库） |
| 10 | 若需改 DeviceId / RuntimeEvent / Control Plane / Federation / V0.2 Runtime Semantics → 停止回报 | 每个任务"停止条件" + §10 范围冻结 |

**唯一裁决分歧（显式登记）**：tasks.md 2.1 写 `fetch_add(SeqCst)`，Design Doc §3.2 定稿 `Ordering::Relaxed` 并给充分性论证（`fetch_add` 原子读改写在**任何** ordering 下都不重号不空洞；快照各字段新鲜度由各自源锁保证，与计数器 ordering 无关）。本计划从 Design Doc（深度设计，2026-09-01 定稿）用 **Relaxed**。

### 0.4 验收口径（Design Doc §6.1 落点表 + tasks.md 四栏）

验收顺序：**Contract → Implementation → Unit → Concurrency Simulation → Hardware Gate → 全 CI**（末尾 §4 清单）。"五套 feature 组合编译"（tasks.md 1.1/2.2 Verification 列）= CI 可跑三套（default / simulation / mock）+ 盒上两套（`bmd,gstreamer` build / `hardware-test` build，任务 8 矩阵承担）。

---

## 1. 文件结构（改动面锁定，超出即范围漂移）

| 文件 | 责任 | 本计划动作 |
| --- | --- | --- |
| `services/media-agent/src/runtime_state.rs` | Canonical→Runtime 聚合边 + 观察信封类型 | **T1**：+`SnapshotObservation`；`CanonicalRuntimeState` +2 字段（`#[serde(default)]`）；`assemble` +第 6 参 `obs`；删私有 `now_ms()`；D14 注释改关闭态（**T7**）；既有 3 测试调用点传 obs + 键集合 6→8（**T4**）；+5 个 Unit 测试（**T1/T5**） |
| `services/media-agent/src/session.rs` | 装配 owner | **T2**：`SessionManager` +2 字段 + `new()` 初始化（签名不变）+ `runtime_state()` 先递增后装配；+1 连续调用测试（**T5**）+ 1 并发击穿测试（**T6**） |
| `services/media-agent/src/api_boundary.rs` | API 投影面 | **T3**：`ApiQuerySnapshot` +2 字段 + `to_api_query_snapshot` 加法投影；测试字面量 6→8 字段 + 断言 |
| `services/media-agent/src/runtime_query.rs` | 只读 façade（**零代码改动**） | **T7**：仅 L13-15 D14 镜像注释同步关闭态 |
| `services/media-agent/src/main.rs` | 组合根 / 硬件 gate 入口 | **零改动**（RUNTIME-STATE-RT-01 打印 L825-831 自动含新字段；EXTERNAL-API-RT-01 selftest L1268-1276 roundtrip 编译级自动覆盖） |
| `services/media-agent/src/{command,idempotency,runtime_query,session,error_model,event_projection}.rs` 测试 | 10 参 `SessionManager::new(` 调用点 | **零改动**（`new()` 签名不变——Design Doc §1 复核收益） |
| `docs/architecture/PHASE_0_7A_POST_MERGE_DEBT.md` | 债务账本（SoT） | **T7**：D14 行标 CLOSED |
| `~/transport_hw_gate.sh`（盒上本地） | Hardware Gate 脚本 | **T8**：加法 D14 断言块（**不入库、不提交**） |
| verify 报告（任务 9 产出） | 交付证据 | 四栏纪律表 + D14 关闭证据 + 术语消歧 |

---

## 2. 任务（按 Design Doc §8 交付序列 + 三层测试）

> 粒度检查：每个任务变更量 < 200 行（最大任务 6 ≈ 90 行，其余 ≤ 60 行），无需再拆。
> 每个实现任务 Step 1 = 纪律 #2 自检点：**先 Read 目标区域实码**，与 §0.2 基线比对；若行号/签名漂移 > ±15 行或字段/参数数不符 → 停止回报（纪律 #1/#10），不得凭本计划文本盲改。

### Task 0: 基线核验与零漂移自检（前置门禁）

- **Files:** 无改动（只读核验）
- **Contract:** 用户纪律 #1/#2（base-ref 锁定 + 实现前实码自检）
- **Implementation:** 待
- **Verification:** 下方 4 条命令输出全部符合期望
- **Gate:** 无（失败 = 不进入 Task 1，回报重新裁决）

- [ ] **Step 1: 确认 HEAD = base-ref**

Run（仓库根）:
```bash
cd /e/code/live && git rev-parse HEAD
```
Expected: `4d13265bcb8e31314f08d04543255b8222724f9c`
若不符 → **停止**，向 coordinator 回报当前 HEAD 与基线差异。

- [ ] **Step 2: 目标函数/区域再 Read 自检（纪律 #2）**

Read 以下区域并与 §0.2 比对（字段数/参数数/行号 ±15 内）：
```bash
cd /e/code/live/services/media-agent
sed -n '99,127p;179,187p;222,227p' src/runtime_state.rs   # struct 定义 + assemble 尾部 + now_ms
sed -n '238,250p;256,267p;289,294p;766,780p' src/session.rs # struct/new/now_ms/runtime_state
sed -n '138,164p;483,495p' src/api_boundary.rs             # ApiQuerySnapshot + 投影 + 测试字面量
sed -n '13,15p;39,41p' src/runtime_query.rs                # D14 镜像注释 + get_runtime_state
```
Expected: 与 §0.2 一致（6 字段 struct、10 参 new、5 参 assemble、6 字段 ApiQuerySnapshot、L485 六字段字面量）。
若不符 → **停止**回报漂移详情。

- [ ] **Step 3: 记录改动前 mock 测试基线计数（零回退锚点，纪律 #1：不采信文档数字，以实跑为准）**

Run:
```bash
cd /e/code/live/services/media-agent && cargo test --features mock 2>&1 | grep -E "^test result" | tee /tmp/d14_baseline.txt
```
Expected: 多个 `test result: ok. X passed; 0 failed; ...`（含 lib 与 main 两个二进制）。
记录 lib 二进制 passed 数 = **BASELINE_COUNT**（Design Doc 记载为 215，**以实跑输出为准**）。Task 1–6 全部落地后要求 `≥ BASELINE_COUNT + 6`（新增测试函数恰 6 个：runtime_state.rs ×3 + session.rs ×3；另有 2 个既有测试更新非新增；核对表见 §3，实数核对见 Task 9 Step 1）。

- [ ] **Step 4: fmt 基线**

Run:
```bash
cd /e/code/live/services/media-agent && cargo fmt --all -- --check && echo FMT_CLEAN
```
Expected: `FMT_CLEAN`（exit 0）。若基线本身不干净 → 停止回报（不得顺手修无关格式）。

---

### Task 1: 观察信封 `SnapshotObservation` + `CanonicalRuntimeState` additive 两字段 + `assemble` 纯度化（tasks.md 1.1 / 1.2 / 1.3）

- **Files:**
  - Modify: `services/media-agent/src/runtime_state.rs`（struct L104-112、assemble L117-187、now_ms L222-227、测试模块 L229 起）
- **Contract:** proposal §What-1/§Why-3 | design D5/D6 | EXTERNAL_API_CONTRACT §2 #126（additive 非破坏）| Design Doc §2.1/§2.2/§2.3
- **Implementation:** 待
- **Verification:** 五套 feature 组合编译通过（Task 9 矩阵）；Unit 测试——serde 往返含新字段、**旧 6 键 JSON 实际字符串**反序列化 default 生效、同输入两次 assemble 逐字段相等（Design Doc §6.1）
- **Gate:** 无
- **停止条件（纪律 #10）:** 若发现 `assemble`/`CanonicalRuntimeState` 存在 §0.2 之外的生产调用点/构造点 → 停止回报。

- [ ] **Step 1: 自检点** — Read `src/runtime_state.rs` L99-127、L179-187、L222-227（纪律 #2，对照 §0.2）。

- [ ] **Step 2: 先写失败测试（TDD）** — 在 `src/runtime_state.rs` 测试模块末尾（`runtime_state_rt_01_session_projection_and_resource_states` 之后、模块闭括号前）追加：

```rust
    /// D14 测试助手: 空世界 + 常量观察信封（纯度/serde 测试不依赖 mock 设备）。
    fn empty_world() -> (Vec<DeviceInfo>, PortRegistry, ResourceRegistry) {
        let registry = PortRegistry { ports: vec![] };
        (Vec::new(), registry, ResourceRegistry::derive_from_discovery(&registry))
    }

    /// D14 (tasks 1.2/1.3): 新 8 键 JSON 序列化 roundtrip —— 观察信封字段在场且保真。
    #[test]
    fn runtime_state_rt_01_observation_envelope_serde_roundtrip() {
        let (devices, registry, resources) = empty_world();
        let obs = SnapshotObservation {
            revision: 7,
            lineage: Uuid::new_v4(),
            observed_at_ms: 1_700_000_000_123,
        };
        let state =
            CanonicalRuntimeState::assemble(&devices, &registry, &resources, &HashMap::new(), &[], &obs);
        assert_eq!(state.observation_revision, 7, "revision 必来自 obs 输入");
        assert_eq!(state.observation_lineage, obs.lineage);
        assert_eq!(state.generated_at_ms, obs.observed_at_ms, "时间戳必来自 obs, 非墙钟");
        let json = serde_json::to_string(&state).expect("serialize 8 字段");
        let back: CanonicalRuntimeState = serde_json::from_str(&json).expect("roundtrip");
        assert_eq!(back.observation_revision, 7);
        assert_eq!(back.observation_lineage, obs.lineage);
        assert_eq!(back.generated_at_ms, obs.observed_at_ms);
    }

    /// D14 (tasks 1.2, 纪律 #7): **实际旧 6 键 JSON 字符串**（base-ref 6 字段形状, 非新结构
    /// roundtrip 替身）反序列化 —— #[serde(default)] 双向非破坏: revision→0, lineage→nil。
    #[test]
    fn runtime_state_rt_01_legacy_six_key_json_deserializes_with_defaults() {
        let legacy = r#"{
            "devices": [],
            "ports": [],
            "resources": [],
            "sessions": [],
            "media_semantics": [],
            "generated_at_ms": 1700000000000
        }"#;
        let state: CanonicalRuntimeState =
            serde_json::from_str(legacy).expect("旧 6 键 JSON 必须可反序列化 (additive)");
        assert_eq!(state.observation_revision, 0, "缺字段 default = 0 (=absent)");
        assert_eq!(state.observation_lineage, Uuid::nil(), "缺字段 default = nil UUID");
        assert_eq!(state.generated_at_ms, 1_700_000_000_000);
    }
```

- [ ] **Step 3: 运行确认失败（编译错 = 预期红）**

Run:
```bash
cd /e/code/live/services/media-agent && cargo test --features mock runtime_state_rt_01_observation_envelope_serde_roundtrip 2>&1 | tail -5
```
Expected: 编译失败（`SnapshotObservation` 未定义 / `assemble` 参数不足 / 字段不存在）。

- [ ] **Step 4: 最小实现** — 四处编辑（顺序执行）：

(4a) struct 前插入 `SnapshotObservation`（`PortMediaSemantics` 定义之后、`CanonicalRuntimeState` 之前，约 L98）：
```rust
/// D14 观察信封 —— 一次 `runtime_state()` 装配的观测元数据（Design Doc §2.1）。
///
/// **swept, non-transactional, start-ordered**:
/// - `revision`: 进程内单调递增观察序号, 起点 1（0 保留 = absent, 见 serde default）;
/// - `lineage`: 进程观察谱系（owner 构造时 UUIDv4 一次）, 重启换新;
/// - `observed_at_ms`: 观测时刻墙钟（毫秒; 新鲜度参考, **非**一致性锚点——锚点是 (lineage, revision)）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotObservation {
    pub revision: u64,
    pub lineage: Uuid,
    pub observed_at_ms: u64,
}
```

(4b) `CanonicalRuntimeState` 追加两字段（`generated_at_ms` **保留不动**，Design Doc §2.2）：
```rust
    pub generated_at_ms: u64,
    /// D14: 进程内单调观察序号（起点 1; 0 = absent, 见 serde default）。
    #[serde(default)]
    pub observation_revision: u64,
    /// D14: 进程观察谱系（UUIDv4; nil = absent）。
    #[serde(default)]
    pub observation_lineage: Uuid,
```

(4c) `assemble` 签名追加第 6 参 + 尾部三字段改取 obs：
```rust
    pub fn assemble(
        devices: &[DeviceInfo],
        registry: &PortRegistry,
        resources: &ResourceRegistry,
        bindings: &std::collections::HashMap<Uuid, ResolvedDeviceBinding>,
        sessions: &[MediaSession],
        obs: &SnapshotObservation,
    ) -> Self {
```
```rust
        Self {
            devices: devs,
            ports,
            resources: res,
            sessions: sess,
            media_semantics: media,
            generated_at_ms: obs.observed_at_ms,
            observation_revision: obs.revision,
            observation_lineage: obs.lineage,
        }
```
（`assemble` 上方 doc 注释 L115 同步为：`/// 纯装配（无 IO/锁/全局; 同输入——含 obs——恒同输出, D14 关闭）。媒体语义对每个有稳定 port_id 的端口复用 0.7B 资产（\`RawInputDescription::from_port\` + \`normalize_input\`）。`）

(4d) **整体删除** 私有 `now_ms()`（L222-227 整函数）——纪律 #5：`assemble` 不得留隐式 SystemTime 读；墙钟读唯一收敛点 = session.rs `Self::now_ms()`。

- [ ] **Step 5: 运行新测试转绿**

Run:
```bash
cd /e/code/live/services/media-agent && cargo test --features mock runtime_state 2>&1 | tail -15
```
Expected: 新 2 测试 pass；既有 3 测试中 **2 个因 `assemble` 参数不足编译失败**（调用点未传 obs——预期红，Task 4 修复）；若编译错涉及 §0.2 之外文件 → 停止回报（调用点计数漂移）。

- [ ] **Step 6: 纯度 grep 门禁**

Run:
```bash
cd /e/code/live/services/media-agent && grep -n "now_ms\|SystemTime" src/runtime_state.rs; echo "grep-exit=$?"
```
Expected: 无任何输出行，`grep-exit=1`（`now_ms`/`SystemTime` 已从该文件根除）。

---

### Task 2: 装配 owner —— `SessionManager` 观察序列（tasks.md 2.1 / 2.2）

- **Files:**
  - Modify: `services/media-agent/src/session.rs`（struct L238-250、`new()` L256-267、`runtime_state()` L769-780）
- **Contract:** design D1/D3 | proposal §What-2 | Design Doc §3.1/§3.2
- **Implementation:** 待
- **Verification:** 五套 feature 矩阵编译 + 既有 session/runtime_state 测试零回退（tasks.md 2.2）；连续两次 `runtime_state()` revision 严格 +1、lineage 不变（本任务 Step 2 TDD 测试 `session_rt_01_observation_revision_starts_at_1_and_increments`）+ 8×1000 并发击穿（Task 6）
- **Gate:** 无
- **停止条件:** `SessionManager` 字段数 ≠ 11 或 `new()` 参数数 ≠ 10（对照 §0.2）→ 停止回报。

- [ ] **Step 1: 自检点** — Read `src/session.rs` L238-250、L256-267、L289-294、L766-780（纪律 #2）。

- [ ] **Step 2: 先写失败测试（TDD）** — `src/session.rs` 测试模块末尾（闭括号前）追加：

```rust
    /// D14 (tasks 2.1, Design Doc §6.1): 观察序列语义 —— 首快照 revision=1（0 保留 absent）、
    /// 连续调用严格 +1、lineage 同一 manager 恒同。
    #[test]
    fn session_rt_01_observation_revision_starts_at_1_and_increments() {
        let devices = mock_devices();
        let lm = Arc::new(InMemoryLm::new());
        let mgr = mock_manager(&devices, lm);
        let first = mgr.runtime_state();
        assert_eq!(first.observation_revision, 1, "首快照 revision 必为 1, 非 0");
        let second = mgr.runtime_state();
        assert_eq!(second.observation_revision, 2, "连续调用严格 +1");
        assert_eq!(
            first.observation_lineage, second.observation_lineage,
            "同一 manager lineage 恒同"
        );
        assert_ne!(first.observation_lineage, Uuid::nil(), "lineage 为真实 UUIDv4, 非 nil");
    }
```

Run 确认失败（红）:
```bash
cd /e/code/live/services/media-agent && cargo test --features mock session_rt_01_observation_revision_starts_at_1 2>&1 | tail -5
```
Expected: 编译失败（`observation_revision`/`observation_lineage` 字段不存在——预期红，Step 3 实现后转绿）。

- [ ] **Step 3: 最小实现** — 三处编辑：

(3a) `SessionManager` 追加两字段（`events` 之后）：
```rust
    sessions: Mutex<HashMap<SessionId, SessionInner>>,
    events: Arc<dyn RuntimeEventSink>,
    /// D14: 观察序号起点 1（0 = absent 保留给 serde default）; 每次 runtime_state() 递增。
    observation_revision: std::sync::atomic::AtomicU64,
    /// D14: 观察谱系（构造一次 UUIDv4; 重启换新 → (lineage, revision) 进程内全序）。
    observation_lineage: uuid::Uuid,
```

(3b) `new()` 初始化（**签名保持 10 参数不变**，10 个调用点零改动）：
```rust
            sessions: Mutex::new(HashMap::new()),
            events,
            observation_revision: std::sync::atomic::AtomicU64::new(1),
            observation_lineage: uuid::Uuid::new_v4(),
        }
```

(3c) `runtime_state()` 新体（**先 increment 后采集源**，start-ordered，Design Doc §3.2）：
```rust
    pub fn runtime_state(&self) -> crate::runtime_state::CanonicalRuntimeState {
        // D14: 先取序号（观测开始锚点）, 再采集各源 —— swept, start-ordered（Design Doc §4.1）。
        // Relaxed 充分: fetch_add 原子读改写在任何 ordering 下不重号不空洞（Design Doc §3.2）。
        let rev = self
            .observation_revision
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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

- [ ] **Step 4: TDD 转绿 + 编译门禁**

Run:
```bash
cd /e/code/live/services/media-agent && cargo test --features mock session_rt_01_observation_revision_starts_at_1 2>&1 | tail -6
```
Expected: `test result: ok. 1 passed`（Step 2 的失败测试转绿——起点 1 / 严格 +1 / lineage 恒同全部由实码验证）。

Run:
```bash
cd /e/code/live/services/media-agent && cargo check --features mock 2>&1 | tail -8
```
Expected: `Finished`（生产路径唯一 `assemble` 调用点已更新；`cargo check` 不含测试目标，runtime_state.rs 测试的编译错待 Task 4 修复属预期）。

---

### Task 3: Wire 面 —— `ApiQuerySnapshot` additive 投影（tasks.md 3.1）

- **Files:**
  - Modify: `services/media-agent/src/api_boundary.rs`（`ApiQuerySnapshot` L140-147、`to_api_query_snapshot` L149-164、测试字面量 L485-492）
- **Contract:** EXTERNAL_API_CONTRACT §2 #126 | design D4/D6 | Design Doc §2.2（wire 映射）
- **Implementation:** 待
- **Verification:** Unit/Simulation——`GET /api/v1/runtime` 响应 JSON 含 `observation_revision`/`observation_lineage` 且值与域内一致（tasks.md 3.1；盒上实证 = Task 8）
- **Gate:** 无
- **停止条件:** `ApiQuerySnapshot {` 字面量出现 §0.2 之外的第二处 → 停止回报。

- [ ] **Step 1: 自检点** — Read `src/api_boundary.rs` L138-164、L483-509（纪律 #2）。

- [ ] **Step 2: 最小实现** — 三处编辑：

(2a) `ApiQuerySnapshot` 追加同名字段（`generated_at_ms` 保留不动）：
```rust
pub struct ApiQuerySnapshot {
    pub devices: Vec<ApiDevice>,
    pub ports: Vec<ApiPort>,
    pub resources: Vec<ApiResource>,
    pub sessions: Vec<ApiSession>,
    pub capabilities: Vec<(String, ApiCapability)>,
    pub generated_at_ms: u64,
    /// D14: 观察信封 additive 投影（wire 同名字段, 非破坏）。
    pub observation_revision: u64,
    pub observation_lineage: uuid::Uuid,
}
```
（注意：struct 现有 derive 为 `Debug, Clone, Serialize, Deserialize, PartialEq, Eq`——`Uuid: Eq` 成立，无需改 derive。）

(2b) `to_api_query_snapshot` 投影加法复制（保持纯函数）：
```rust
        generated_at_ms: state.generated_at_ms,
        observation_revision: state.observation_revision,
        observation_lineage: state.observation_lineage,
    }
```

(2c) 测试字面量 6 → 8 字段（编译级零遗漏，Design Doc §6.2 R3）+ 断言：
```rust
        let state = CanonicalRuntimeState {
            devices: vec![],
            ports: vec![],
            resources: vec![],
            sessions: vec![],
            media_semantics: vec![],
            generated_at_ms: 1_700_000_000_000,
            observation_revision: 42,
            observation_lineage: uuid::Uuid::new_v4(),
        };
        let snap = to_api_query_snapshot(&state);
        assert_eq!(snap.generated_at_ms, 1_700_000_000_000);
        assert_eq!(snap.observation_revision, 42, "投影透传 revision");
        assert_eq!(snap.observation_lineage, state.observation_lineage, "投影透传 lineage");
```
（`api_rt_01_to_api_query_models` 内其余断言不动。）

- [ ] **Step 3: 运行该测试**

Run:
```bash
cd /e/code/live/services/media-agent && cargo test --features mock api_rt_01_to_api_query_models 2>&1 | tail -8
```
Expected: `test result: ok. 1 passed`。

- [ ] **Step 4: 既有 selftest 编译级覆盖确认（零改动）**

`src/main.rs` L1268-1276 EXTERNAL-API-RT-01 selftest 对 `ApiQuerySnapshot` 做 `to_string` + `from_str` roundtrip——新字段自动经 serde 覆盖，**不改 main.rs**。
Run:
```bash
cd /e/code/live/services/media-agent && grep -n "ApiQuerySnapshot" src/main.rs
```
Expected: 命中 selftest 使用点（import + roundtrip），无结构体字面量构造。

---

### Task 4: 既有测试调用点同步 —— `assemble` 五处调用 + 键集合 6→8（tasks.md 4.3 + R3 编译驱动）

- **Files:**
  - Modify: `services/media-agent/src/runtime_state.rs`（测试 L279/L329/L345/L357 四个 `assemble` 调用 + L288-299 键集合断言）
  - Modify: `services/media-agent/src/session.rs`（`runtime_state()` 已在 Task 2 更新——本任务不重复）
- **Contract:** design D8 | Design Doc §2.3/§6.1（键集合 6→8）
- **Implementation:** 待
- **Verification:** `cargo test --features mock` runtime_state 全绿；`CanonicalRuntimeState {` 字面量全仓仍仅 1 处（api_boundary.rs，已 8 字段）
- **Gate:** 无

- [ ] **Step 1: 自检点** — grep 确认待改调用点恰为 4 处：

Run:
```bash
cd /e/code/live/services/media-agent && grep -n "CanonicalRuntimeState::assemble(" src/
```
Expected: 恰 5 行命中 = runtime_state.rs 4 处测试（L279/329/345/357 附近）+ session.rs 1 处（Task 2 后已 6 参）。若数量 ≠ 5 或出现新文件 → 停止回报。

- [ ] **Step 2: 修复 4 个测试调用点** — 在测试模块顶部（`world()` 助手旁）加常量信封助手，四处调用末尾追加 `&TEST_OBS`：

```rust
    /// D14 测试常量观察信封（测试传常量, 编译错误驱动零遗漏; observed_at_ms 固定值锁定纯度）。
    const TEST_OBS: SnapshotObservation = SnapshotObservation {
        revision: 1,
        lineage: uuid::Uuid::from_u128(0),
        observed_at_ms: 0,
    };
```
四处调用（L279 / L329 / L345 / L357）统一改为：
```rust
        let state =
            CanonicalRuntimeState::assemble(
                &devices,
                &registry,
                &resources,
                &HashMap::new(),
                &[],
                &TEST_OBS,
            );
```
（L329/L345 两处 `&bindings` 实参保持不变，仅追加第 6 参。）

- [ ] **Step 3: 键集合断言 6 → 8**（`runtime_state_rt_01_composition_descriptor_not_flattened`，L288-299）：

```rust
        assert_eq!(top_keys, {
            let mut expect = vec![
                "devices",
                "ports",
                "resources",
                "sessions",
                "media_semantics",
                "generated_at_ms",
                "observation_revision",
                "observation_lineage",
            ];
            expect.sort_unstable();
            expect
        });
```
（注释 L276 同步：`// 绝不平铺到 state 顶层 (顶层键集合 == 八个固定键, D14 additive 两字段)。`）

- [ ] **Step 4: 运行 runtime_state 全组测试**

Run:
```bash
cd /e/code/live/services/media-agent && cargo test --features mock runtime_state 2>&1 | tail -12
```
Expected: `test result: ok. 5 passed; 0 failed`（既有 3 + Task 1 新增 2）。

- [ ] **Step 5: 字面量唯一性门禁**

Run:
```bash
cd /e/code/live/services/media-agent && grep -rn "CanonicalRuntimeState {" src/ | grep -v "pub struct\|let state: \|: CanonicalRuntimeState"
```
Expected: 恰 1 行命中（api_boundary.rs 测试字面量，已 8 字段）。

---

### Task 5: Unit 层收尾 —— assemble 纯度 / 重启语义（计数起点 + 连续调用已在 Task 2 落地；tasks.md 5.1 + Design Doc §6.1 Unit 行）

- **Files:**
  - Modify: `services/media-agent/src/session.rs`（测试模块，+1 测试）
  - Modify: `services/media-agent/src/runtime_state.rs`（测试模块，+1 纯度测试）
- **Contract:** D14 关闭条件（三层测试）| Design Doc §6.1（Unit：assemble 纯函数 / serde 双向 / 键集合 / 计数起点 / 重启语义）
- **Implementation:** 待
- **Verification:** `cargo test --features mock` 全绿且 passed 数 ≥ BASELINE_COUNT + 5（Task 0 锚点；截至本任务新增测试函数 = Task 1 ×2 + Task 2 ×1 + 本任务 ×2 = 5，第 6 个并发测试在 Task 6，总账见 §3 核对表）
- **Gate:** 无

- [ ] **Step 1: 自检点** — Read Task 1/4 已落地的测试区域，确认 `TEST_OBS`/`empty_world()` 助手在场。

- [ ] **Step 2: 追加 2 个测试**（连续调用测试 `session_rt_01_observation_revision_starts_at_1_and_increments` 已在 Task 2 Step 2 以 TDD 落地并转绿，此处不重复）

(2a) `src/runtime_state.rs` 测试模块追加（assemble 纯度，Design Doc §6.1 "Unit | assemble 纯函数"行）：
```rust
    /// D14 (tasks 1.3, Design Doc §6.1): assemble 纯性 —— 同 5 源 + 同 obs 两次装配
    /// 逐字段相等（含 generated_at_ms = obs.observed_at_ms）; 不同 obs → 信封字段不同。
    #[test]
    fn runtime_state_rt_01_assemble_pure_same_obs_same_output() {
        let (devices, registry, resources) = empty_world();
        let bindings = std::collections::HashMap::new();
        let sessions: &[MediaSession] = &[];
        let obs = SnapshotObservation {
            revision: 3,
            lineage: Uuid::new_v4(),
            observed_at_ms: 1_700_000_000_456,
        };
        let a = CanonicalRuntimeState::assemble(&devices, &registry, &resources, &bindings, sessions, &obs);
        let b = CanonicalRuntimeState::assemble(&devices, &registry, &resources, &bindings, sessions, &obs);
        assert_eq!(a, b, "同输入恒同输出 (PartialEq 逐字段)");
        assert_eq!(a.generated_at_ms, obs.observed_at_ms, "无隐式墙钟读");
        let other = SnapshotObservation { revision: 4, ..obs };
        let c = CanonicalRuntimeState::assemble(&devices, &registry, &resources, &bindings, sessions, &other);
        assert_ne!(a.observation_revision, c.observation_revision, "obs 是显式输入");
    }
```

(2b) `src/session.rs` 测试模块追加（重启语义 = 新 owner → 新 lineage + revision 归 1，以构造新 manager 模拟，tasks.md 5.1）：
```rust
    /// D14 (tasks 5.1, Design Doc §4.2): 重启语义 —— 新构造的 owner（模拟进程重启）
    /// revision 回 1 且 lineage 换新; (lineage, revision) 跨代不可比由 lineage 显式化。
    #[test]
    fn session_rt_01_restart_semantics_new_lineage_revision_back_to_1() {
        let devices = mock_devices();
        let lm = Arc::new(InMemoryLm::new());
        let gen1 = mock_manager(&devices, lm.clone());
        let s1 = gen1.runtime_state();
        assert_eq!(s1.observation_revision, 1);
        let gen2 = mock_manager(&devices, lm); // "重启" = 新 SessionManager
        let s2 = gen2.runtime_state();
        assert_eq!(s2.observation_revision, 1, "重启后 revision 归 1 (不承诺跨重启连续)");
        assert_ne!(s1.observation_lineage, s2.observation_lineage, "重启必换新 lineage");
    }
```
（import 自检：runtime_state.rs 测试模块 `use super::*` 已覆盖 `SnapshotObservation`/`Uuid`/`CanonicalRuntimeState`/`empty_world`；session.rs 测试模块 `use super::*` 已覆盖 `Uuid`，`InMemoryLm`/`mock_manager`/`mock_devices` 为模块内既有助手。若编译报缺 import，仅补对应 `use` 行，不得改生产面。）

- [ ] **Step 3: 运行 Unit 层全组**

Run:
```bash
cd /e/code/live/services/media-agent && cargo test --features mock 2>&1 | grep -E "^test result|running" | tail -8
```
Expected: 全 `ok`，0 failed；lib passed 数 ≥ BASELINE_COUNT + 5（截至此时新增测试函数 = Task 1 ×2 + Task 2 ×1 + 本任务 ×2 = 5；**以 Task 0 实跑基线 + `git diff --stat` 实数核对**，禁止目测；第 6 个新增测试在 Task 6 落地后补齐总账）。

---

### Task 6: Concurrency Simulation —— 8 线程 × 1000 并发击穿（tasks.md 5.2 + 用户纪律 #8）

- **Files:**
  - Modify: `services/media-agent/src/session.rs`（测试模块，+1 测试）
- **Contract:** design D1/D6 | D9-C 先例（单临界区击穿纪律，债务账本 L30）| Design Doc §6.1 "Simulation | 并发击穿"行
- **Implementation:** 待
- **Verification:** `cargo test --features mock session_rt_01_observation_revision_8x1000_concurrency_pierce` 通过；8000 个 revision 集合**恰为 {1..8000}**（无重号无空洞），8000 份快照 lineage 恒同（纪律 #8 逐字要求）
- **Gate:** 无

- [ ] **Step 1: 自检点** — Read `src/session.rs` `runtime_state()`（Task 2 后形态），确认 `fetch_add` 在场且无第二计数点。

- [ ] **Step 2: 追加并发击穿测试**（`src/session.rs` 测试模块末尾）：

```rust
    /// D14 (tasks 5.2, D9-C 同构, 用户纪律 #8): 8 线程 × 1000 次并发 runtime_state() 击穿 ——
    /// 8000 个 revision 集合恰为 {1..8000}（无重号、无空洞, 单临界区原子性实证）,
    /// 且 8000 份快照 lineage 恒同（单 manager 恒定性）。
    #[test]
    fn session_rt_01_observation_revision_8x1000_concurrency_pierce() {
        let devices = mock_devices();
        let lm = Arc::new(InMemoryLm::new());
        let mgr = Arc::new(mock_manager(&devices, lm));
        let workers = 8;
        let per_worker = 1000usize;
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(workers));
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let m = Arc::clone(&mgr);
            let b = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                b.wait(); // 同刻出发, 最大化装配交错窗口
                (0..per_worker)
                    .map(|_| m.runtime_state())
                    .map(|s| (s.observation_revision, s.observation_lineage))
                    .collect::<Vec<_>>()
            }));
        }
        let mut all = Vec::with_capacity(workers * per_worker);
        for h in handles {
            all.extend(h.join().expect("worker 线程不得 panic"));
        }
        assert_eq!(all.len(), 8000, "8×1000 份快照全量收集");
        let mut revs: Vec<u64> = all.iter().map(|(r, _)| *r).collect();
        revs.sort_unstable();
        revs.dedup();
        let expect: Vec<u64> = (1..=8000).collect();
        assert_eq!(
            revs, expect,
            "revision 集合恰为 {{1..8000}}: 无重号 (唯一) 无空洞 (连续覆盖)"
        );
        let lineage0 = all[0].1;
        assert!(
            all.iter().all(|(_, l)| *l == lineage0),
            "8000 份快照 lineage 恒同 (单 manager)"
        );
    }
```

- [ ] **Step 3: 实跑击穿（不得只编译不跑）**

Run:
```bash
cd /e/code/live/services/media-agent && cargo test --features mock --release session_rt_01_observation_revision_8x1000_concurrency_pierce -- --nocapture 2>&1 | tail -8
```
Expected: `test result: ok. 1 passed`（`--release` 提高交错压力；若 debug 亦可复跑一次 `cargo test --features mock session_rt_01_observation_revision_8x1000_concurrency_pierce`）。
**失败处理**：若出现重号/空洞 → 说明计数路径存在非原子读改写，属 P0，停止回报（不得放宽断言）。

---

### Task 7: 契约关闭 —— 注释改写 + 债务账本 CLOSED（tasks.md 3.2 / 4.1 / 4.2）

- **Files:**
  - Modify: `services/media-agent/src/runtime_state.rs`（L101-103 D14 注释）
  - Modify: `services/media-agent/src/runtime_query.rs`（L13-15 镜像注释；**零代码改动**）
  - Modify: `docs/architecture/PHASE_0_7A_POST_MERGE_DEBT.md`（L69 D14 行）
- **Contract:** design D7/D3 | proposal §What-4 | 债务清单纪律 L6（独立 change + 三层测试）| Design Doc §4.1/§4.3/§7
- **Implementation:** 待
- **Verification:** 人工/grep 复核两处 D14 注释语义一致且与 proposal/design 声明逐句吻合（tasks.md 4.1/3.2）；账本行含 change 名、日期、证据锚（tasks.md 4.2）
- **Gate:** 无

- [ ] **Step 1: 自检点** — Read `src/runtime_state.rs` L99-112、`src/runtime_query.rs` L13-15（纪律 #2；确认 Task 1 后注释仍在原位）。

- [ ] **Step 2: `runtime_state.rs` D14 注释改写为关闭态**（替换 L101-103 三行，Design Doc §4.1 声明逐句落注）：

```rust
/// **D14 契约（CLOSED @ v03-d14-runtime-snapshot-consistency, 2026-09-02）**:
/// 一致性类 = **swept non-transactional** —— 各源（devices/ports/resources/sessions）
/// 在装配点各自加锁/共享点克隆, 各源观察时刻独立（= 其读取时刻 ≤ 装配完成时刻）,
/// 跨源无原子性; **start-ordered**: 进程内 revision 严格单调 +1、唯一, rev_a < rev_b
/// ⟺ S_b 观测开始（fetch_add）晚于 S_a; 无 per-field 单调承诺、无跨进程全序
/// （lineage 为随机 UUIDv4, 非时序）。revision 起点 1（0 = absent, serde default）,
/// 重启归 1 + 换新 lineage; 不承诺跨重启连续。`generated_at_ms` 保留为新鲜度参考
/// （墙钟, 可 NTP 回拨, 非一致性锚点——锚点 = (lineage, revision)）。
/// 三 "revision" 消歧: 本 `observation_revision` = 观察域; V0.2 §1.21 "Runtime
/// Revision N+1" = config-apply 域; API §4 `If-Match: "revision-N"` = 乐观并发域
/// —— 互不替代（Design Doc §4.3）。
```

- [ ] **Step 3: `runtime_query.rs` L13-15 镜像注释同步关闭态**（与 Step 2 语义一致，façade 视角）：

```rust
//! D14 契约（CLOSED @ v03-d14-runtime-snapshot-consistency, 2026-09-02）:
//! `get_runtime_state()` 返回 **swept non-transactional** snapshot —— 各源在装配点
//! 各自加锁观测, 跨源无原子性; 新旧关系由 (observation_lineage, observation_revision)
//! 机器判定（revision 起点 1 单调唯一, 重启归 1 + 换新 lineage; 详见
//! runtime_state.rs 结构注释与 PHASE_0_7A_POST_MERGE_DEBT.md D14）。
```
（**本文件不得有任何非注释改动**——纪律 #7"runtime_query.rs 零改动"指零代码/行为改动。）

- [ ] **Step 4: 债务账本 D14 行标 CLOSED**（`docs/architecture/PHASE_0_7A_POST_MERGE_DEBT.md` L69，格式对齐 D8 行 L29 先例）：

将 L69 整行替换为：
```markdown
| D14 | ~~**Runtime Snapshot Consistency**：`runtime_state()` 是各源独立观测的拼合 snapshot，非事务一致~~ ✅ **CLOSED @ v03-d14-runtime-snapshot-consistency (V0.3-1, 2026-09-02)**: 一致性类 = **swept non-transactional / start-ordered**（Design Doc §4.1）; 观察信封 `SnapshotObservation{revision,lineage,observed_at_ms}` + `CanonicalRuntimeState` additive 两字段（`#[serde(default)]` 旧 JSON→0/nil 双向非破坏）+ `assemble` 纯化（删隐式 `now_ms()`）+ `SessionManager` 唯一 owner（`AtomicU64` 起点 1 / `Uuid::new_v4()` 进程谱系, `new()` 10 参签名零改动）; 三层测试证据: Unit（8 键 serde / 旧 6 键 JSON default / assemble 纯度 / 起点 1 / 重启语义）+ Simulation（连续严格 +1 / **8 线程×1000 击穿, 8000 个 revision 集合恰为 {1..8000} 无重号无空洞, lineage 恒同**）+ Hardware（盒上 `transport_hw_gate.sh` D14 断言块: revision≥1 / lineage 36 字符 UUID / 两次调用同 lineage 严格 +1）; 证据锚: verify 报告 `docs/superpowers/reports/2026-09-02-v03-d14-runtime-snapshot-consistency-verify.md`（本 change 交付时落档） | ~~需定义 source observation time / state version / 一致性语义~~（已定义） | ~~Runtime Query 后续~~（已收口） |
```
（**日期/证据锚在 Task 9 verify 报告落档后复核一次**；若 verify 报告路径变动，本行同步修正——PHASE_IMPLEMENTATION_MAP/账本为唯一 SoT，漂移 = P0。）

- [ ] **Step 5: 注释一致性门禁**

Run:
```bash
cd /e/code/live/services/media-agent && grep -n "D14 契约" src/runtime_state.rs src/runtime_query.rs && cargo check --features mock 2>&1 | tail -3
```
Expected: 两处命中（均含 `CLOSED @ v03-d14-runtime-snapshot-consistency`）+ `Finished`。

---

### Task 8: Hardware Gate —— 盒上矩阵 + D14 断言（tasks.md 5.3 / 6.1 + 用户纪律 #9）

- **Files:**
  - 盒上本地脚本 `~/transport_hw_gate.sh`（lytv@10.30.15.10）：**加法** D14 断言块，**不入库不提交**
  - 仓库内：**零文件改动**（main.rs RUNTIME-STATE-RT-01 / selftest 自动覆盖，Task 3 Step 4 已确认）
- **Contract:** D14 关闭条件（三层测试）| 验收口径（BOX/CI/RELEASE 三层）| Design Doc §6.1 Hardware 行
- **Implementation:** 待
- **Verification:** 盒上（lytv@10.30.15.10, `--features bmd,gstreamer` 二进制）实跑 PASS，证据（输出摘录）入 verify 报告（tasks.md 5.3/6.1）
- **Gate:** BOX（盒上 matrix 含 D14 项）
- **停止条件:** D14 断言块只允许**追加**探针行；改动既有探针/采集逻辑 = 违反纪律 #9 → 停止回报。

- [ ] **Step 1: 盒上构建 + 全矩阵前置**（盒上执行；本地不可达时由持有盒权限者执行并把输出贴回 verify 报告）：

```bash
ssh lytv@10.30.15.10 'cd ~/media-agent && git pull --ff-only && cargo fmt --all && cargo build --release --features bmd,gstreamer 2>&1 | tail -3'
```
Expected: `Finished` 无 error。（盒上仓库同步方式沿用既有 p07_verify.sh 流程，2026-09-01-p07c8-transport-verify.md §3 先例。）

- [ ] **Step 2: 既有回归矩阵先跑（零回退确认）**

按既有 `~/p07_verify.sh` 14 步矩阵执行（fmt ×2 / default test / sim+mock test / bmd,gstreamer test / clippy -D ×4 / build ×3 / remove-adapters proof），全部 exit 0 后方可进入 Step 3。
Evidence: 每步 exit code 记录入 verify 报告。

- [ ] **Step 3: `~/transport_hw_gate.sh` 追加 D14 断言块**（脚本不入库——改动仅存于盒；断言全文同步写入 verify 报告）：

在既有探针段（`GET /api/v1/runtime 200 (mgr active)` 之后）追加：
```bash
# ── D14 Runtime Snapshot Consistency (V0.3-1) 加法断言 —— 不改既有探针 ─────────
# 纪律: 两次 GET 必须相邻执行, 其间不得插入任何其他 /api/v1/runtime 或
# VBMF_* gate 探针（保证同一进程内无其他 runtime_state() 调用交错）。
R1=$(curl -s http://127.0.0.1:8080/api/v1/runtime)
R2=$(curl -s http://127.0.0.1:8080/api/v1/runtime)
REV1=$(echo "$R1" | jq -r '.observation_revision')
REV2=$(echo "$R2" | jq -r '.observation_revision')
LIN1=$(echo "$R1" | jq -r '.observation_lineage')
LIN2=$(echo "$R2" | jq -r '.observation_lineage')
echo "D14 rev1=$REV1 rev2=$REV2 lin1=$LIN1 lin2=$LIN2"
# 断言 1: 两字段在场且类型正确 (revision 数值 ≥1; lineage 36 字符 UUID 格式)
[ "$REV1" -ge 1 ] && [ "$REV2" -ge 1 ] && echo "D14-1 revision>=1 PASS" || { echo "D14-1 FAIL"; exit 1; }
[[ "$LIN1" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$ ]] && echo "D14-2 lineage-uuid-format PASS" || { echo "D14-2 FAIL"; exit 1; }
# 断言 2: 同进程两次调用 → lineage 恒同 + revision 严格 +1
[ "$LIN1" = "$LIN2" ] && [ "$REV2" -eq $((REV1 + 1)) ] && echo "D14-3 same-lineage-strict-increment PASS" || { echo "D14-3 FAIL"; exit 1; }
```
Run（盒上）: `bash ~/transport_hw_gate.sh`
Expected: 既有 16/16 探针 PASS + `D14-1/2/3` 三行 PASS，脚本 exit 0。
（`jq` 为盒上既有工具，transport 先例已用；若盒无 jq → 改用 `python3 -c` 等价解析，断言语义不变。）

- [ ] **Step 4: 既有硬件采集行为零变更确认**

Run（盒上）: 重跑 `VBMF_SESSION_LIFECYCLE=1` 回归门禁（SESSION-RT-01 + IDEMPOTENCY-RT-01 + ERROR-MODEL-RT-01 + RESOURCE-RT-01 + EXTERNAL-API-RT-01，2026-09-01-p07c8-transport-verify.md §4 回归门禁段先例）。
Expected: ALL PASS（RUNTIME-STATE-RT-01 打印 JSON 现含 8 键——additive，非行为变更）。

---

### Task 9: 全 CI + verify 报告 + 交付（tasks.md 6.1 / 6.2 / 6.3）

- **Files:**
  - Create: `docs/superpowers/reports/2026-09-02-v03-d14-runtime-snapshot-consistency-verify.md`（verify 报告）
  - 分支/PR 操作（git，按项目交付纪律）
- **Contract:** 验收口径（BOX/CI/RELEASE 三层; CI PASS ≠ Merge Gate PASS, 独立核 CI 实跑）| 项目交付纪律
- **Implementation:** 待
- **Verification:** `gh api` 实查 required checks 全 green（非自报）; verify 报告落 docs; PR merged; 分支删除
- **Gate:** CI + RELEASE

- [ ] **Step 1: 本地全量 CI 等价矩阵（提交前最后门禁）**

Run:
```bash
cd /e/code/live/services/media-agent
cargo fmt --all -- --check && echo FMT_OK
cargo test 2>&1 | grep -E "^test result" 
cargo build --no-default-features --features simulation
cargo test --features simulation 2>&1 | grep -E "^test result"
cargo test --features mock 2>&1 | grep -E "^test result"
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features mock -- -D warnings
```
Expected: `FMT_OK` + 全部 `test result: ok`（0 failed）+ clippy 0 warning。
**零回退核对（纪律 #1）**：mock lib passed 数 ≥ BASELINE_COUNT + 6（新增测试函数恰 6 个：runtime_state.rs ×3 —— roundtrip / legacy-6-key / assemble 纯度；session.rs ×3 —— 起点 1 / 重启语义 / 8×1000 并发；另有 2 个既有测试更新 —— 键集合 6→8 / api 字面量 6→8，非新增；**以 `git diff --stat` + `cargo test --features mock -- --list` 实数核对**，禁止目测）。

- [ ] **Step 2: 提交（单 commit 交付，Design Doc §7）**

Run（仓库根；分支名沿用项目惯例，如 `v03-d14-runtime-snapshot-consistency`）:
```bash
cd /e/code/live
git add services/media-agent/src/runtime_state.rs services/media-agent/src/session.rs \
        services/media-agent/src/api_boundary.rs services/media-agent/src/runtime_query.rs \
        docs/architecture/PHASE_0_7A_POST_MERGE_DEBT.md
git status --short   # 复核: 恰 5 个文件, 无 main.rs / 无其他 src 文件 / 无脚本入库
git commit -m "V0.3-1 D14: runtime snapshot observation envelope (swept non-transactional)

- SnapshotObservation{revision,lineage,observed_at_ms} + CanonicalRuntimeState
  additive 2 fields (#[serde(default)]: legacy 6-key JSON -> 0/nil)
- assemble() pure: explicit obs input, implicit now_ms() removed
- SessionManager = sole owner: AtomicU64 (start 1) + Uuid::new_v4() lineage,
  new() 10-param signature unchanged (10 call sites untouched)
- ApiQuerySnapshot additive projection; runtime_query.rs comment closed
- D14 debt ledger CLOSED; 3-tier tests (unit / 8x1000 concurrency / hw gate)"
```
Expected: `git status --short` 恰列 5 文件；commit 成功。

- [ ] **Step 3: 开 PR + CI 实查（非自报，tasks.md 6.2）**

```bash
gh pr create --base master --title "V0.3-1 D14 Runtime Snapshot Consistency" --body "$(cat <<'EOF'
D14 关闭: 观察信封 (observation_revision 起点 1 / observation_lineage UUIDv4) + assemble 纯化 + additive wire。
三层测试: Unit 5 / Simulation 8x1000 击穿 / Hardware 盒上 D14 断言块 (transport_hw_gate.sh, 未入库)。
Design Doc: docs/superpowers/specs/2026-09-01-v03-d14-runtime-snapshot-consistency-design.md
EOF
)"
gh pr checks <PR_NUMBER>    # 逐 job 实查
```
Expected: 7 个 required jobs 全绿——`rust-format` / `rust-test-matrix` / `rust-clippy` / `session-lifecycle` / `hardware-test-compile` / `architecture-portability` / `gstreamer-build`（`.github/workflows/media-agent.yml` L10-11 冻结清单）。**任一红 → 停止排查，不得自报绿。**

- [ ] **Step 4: verify 报告落档**（`docs/superpowers/reports/2026-09-02-v03-d14-runtime-snapshot-consistency-verify.md`）——必含：
  1. 四栏纪律表（Contract/Implementation/Verification/Gate × tasks.md 全 14 项逐条）；
  2. 三层测试证据：Unit 测试名 + 通过输出；8×1000 击穿输出（`{1..8000}` 断言行）；盒上 D14-1/2/3 输出 + 14 步矩阵 exit code；
  3. **术语消歧表**（纪律 #4，Design Doc §4.3 逐字）：observation_revision（观察域, 本 change 新增）/ V0.2 §1.21 "Runtime Revision N+1"（config-apply 域, 不触碰）/ `If-Match: "revision-N"`（乐观并发域, 不触碰）；
  4. 范围冻结核查（纪律 #10）：DeviceId / RuntimeEvent / Control Plane / Federation / V0.2 Runtime Semantics 零触碰声明 + `git diff --stat` 佐证；
  5. 新增测试函数清单（6 新增 + 2 既有更新，文件/名/层）+ 零回退锚点（BASELINE_COUNT 实跑值 vs 交付值）。

- [ ] **Step 5: archive → merge → 删分支**

```bash
cd /e/code/live
# openspec archive（项目惯例: openspec-verify-change 通过后方可 archive）
gh pr merge <PR_NUMBER> --merge --delete-branch
```
Expected: PR merged + 远端分支删除；`docs/openspec/changes/v03-d14-runtime-snapshot-consistency/` 按项目 archive 惯例迁移。

---

## 3. 新增测试函数清单（6 新增 + 2 既有更新，三层落点核对表）

| # | 测试函数 | 文件/模块 | 层 | 对应 tasks.md |
| --- | --- | --- | --- | --- |
| 1 | `runtime_state_rt_01_observation_envelope_serde_roundtrip` | runtime_state.rs `tests` (mock) | Unit | 1.2 / 1.3 |
| 2 | `runtime_state_rt_01_legacy_six_key_json_deserializes_with_defaults` | runtime_state.rs `tests` (mock) | Unit | 1.2（纪律 #7） |
| 3 | `runtime_state_rt_01_assemble_pure_same_obs_same_output` | runtime_state.rs `tests` (mock) | Unit | 1.3 / 5.1 |
| 4 | `session_rt_01_observation_revision_starts_at_1_and_increments` | session.rs `tests` (mock) | Unit/Simulation（连续调用） | 2.1 / 5.1 / 5.2 |
| 5 | `session_rt_01_restart_semantics_new_lineage_revision_back_to_1` | session.rs `tests` (mock) | Unit（重启语义） | 5.1 |
| 6 | `session_rt_01_observation_revision_8x1000_concurrency_pierce` | session.rs `tests` (mock) | Concurrency Simulation | 5.2（纪律 #8） |
| 7 | `api_rt_01_to_api_query_models`（既有, 6→8 字段更新 + 2 断言） | api_boundary.rs `tests` | Unit（wire 投影） | 3.1 |
| 8 | `runtime_state_rt_01_composition_descriptor_not_flattened`（既有, 6→8 键更新） | runtime_state.rs `tests` (mock) | Unit（键集合） | 4.3 |
| — | `transport_hw_gate.sh` D14-1/2/3 断言块（盒上, 非 Rust 测试） | 盒本地（不入库） | Hardware Gate | 5.3（纪律 #9） |
| — | main.rs selftest roundtrip（零改动, 编译级覆盖） | main.rs L1268-1276 | Selftest | Design Doc §6.1 Selftest 行 |

（新增 = 6 个新测试函数 + 2 个既有测试更新；Task 9 Step 1 的 "≥ BASELINE + 6" 以本表新函数数为准。）

---

## 4. 验收顺序（用户指定，逐项打勾方可进入下一项）

1. **Contract** —— 基线核验通过（Task 0：HEAD=base-ref、§0.2 零漂移、BASELINE_COUNT 锚定、FMT_CLEAN）；三 "revision" 消歧表定稿（Design Doc §4.3，Task 9 Step 4）。
2. **Implementation** —— Task 1→4 顺序落地：观察信封 + additive 字段 + assemble 纯度（now_ms 根除 grep 门禁）→ 装配 owner（fetch_add 先于采集, Relaxed 裁决在案）→ wire 加法投影 → 5 处 assemble 调用点 + 8 键测试；`cargo check --features mock` 全绿；10 处 `SessionManager::new(` 调用点零改动（grep 复核）。
3. **Unit** —— Task 5：`cargo test --features mock` 全绿，含旧 6 键 JSON 实际字符串 default 反序列化（纪律 #7）、assemble 纯度、起点 1、重启语义；passed 数 ≥ BASELINE + 5（截至本项新增 = Task 1 ×2 + Task 2 ×1 + Task 5 ×2；第 6 个在下一项 Concurrency 落地）。
4. **Concurrency Simulation** —— Task 6：8 线程 × 1000 实跑击穿，8000 个 revision 集合恰 {1..8000} 无重号无空洞、lineage 恒同（纪律 #8），`--release` 与 debug 双跑。
5. **Hardware Gate** —— Task 8：盒上 14 步矩阵全 exit 0 + D14-1/2/3 断言 PASS（revision≥1 / lineage 36 字符 UUID / 两次同 lineage 严格 +1）+ VBMF_SESSION_LIFECYCLE 回归 ALL PASS（纪律 #9 零行为变更）。
6. **全 CI** —— Task 9：本地七命令矩阵全绿 → 单 commit → PR → `gh pr checks` 7 jobs 全绿（实查非自报）→ verify 报告落档 → archive → merge → 删分支。

**任何一步失败**：停止推进，回报失败输出与最小复现；不得放宽断言、不得扩大改动面（纪律 #10）。

---

## 5. 风险登记（Design Doc §6.2 承接）

| 风险 | 处置（本计划内已落） |
| --- | --- |
| R1 revision 空洞（崩溃/重启） | 契约只承诺单调+唯一不承诺连续（Task 7 注释逐句）；击穿测试锁定存活进程内 {1..8000} 恰覆盖（Task 6） |
| R2 双时间字段同值 | `generated_at_ms` 保留不动（既有 wire 消费者），`observed_at_ms` 为语义锚点；不合并（Task 1 4b 注释） |
| R3 调用点遗漏 | `assemble` 5 调用点 + 1 字面量全部编译错误驱动（Task 4 Step 1/5 grep 门禁双保险）；`cargo check` 即门禁 |
| R4 lineage 随机性 | UUIDv4 碰撞概率可忽略；测试断言稳定性（同 mgr 恒同）而非具体值（Task 5/6） |
| 基线计数漂移 | 不采信文档数字 215，Task 0 实跑锚定 BASELINE_COUNT（纪律 #1） |
| tasks.md 2.1 SeqCst vs Design Doc Relaxed | 裁决在案（§0.3）：从 Design Doc Relaxed + 充分性论证；执行者不得"顺手"改 SeqCst（改了也不算错——原子性保证与 ordering 无关——但须回报登记） |

---

## 6. 范围冻结（非目标，超出即停止回报——纪律 #10）

不实现：Federation / SiteId / Membership / Authority / Fencing；不修改：`RuntimeEvent`（EVENT_CONTRACT 偏差 B 单独裁决）/ `DeviceId`（CANONICAL_IDENTITY §7 偏差 A 单独对账）/ `If-Match` 乐观并发（EXTERNAL_API_CONTRACT §4 冻结）/ V0.2 §1.21 config-apply "Runtime Revision N+1"；不建：per-source 时间戳 / staleness 策略 / 跨进程全序 / revision 持久化（D3：重启归 1 + 谱系换新）/ `RuntimeQuery` 新 accessor。`transport_hw_gate.sh` 不入库。
