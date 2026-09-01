# Brainstorm Summary

- Change: v03-d14-runtime-snapshot-consistency
- Date: 2026-09-01
- 状态: 已定稿（design 阶段交付完成）
- 1c 确认记录: 深度设计方案已于 2026-09-01 完整呈现（9 点）；open 阶段用户已明确确认同一设计方向（"确认，继续下一阶段"，覆盖 D1-D8 全部决策与任务分解）；本会话为自主会话，1c 交互式复核未获得应答，按"不阻塞已授权阶段工作"推进 Design Doc 落盘；**是否进入 /comet-build 实现留待用户裁决**（用户裁定原文：Design → 再决定是否实现）

## 已确认事实（探针 + 本会话实码复核）

- F1-F13 契约探针（前会话）：D14 冻结（L69，关闭=独立 change+三层测试）；struct=5 Vec+generated_at_ms；assemble "纯函数" 声明与 now_ms() 墙钟读不符（F4）；SystemTime 可 NTP 回拨+unwrap_or(0)（F5）；RuntimeQuery 全 getter 重装配（F6）；SessionManager::runtime_state() 唯一生产装配点（F7）；generated_at_ms 经 ApiQuerySnapshot 上 wire（F8）；additive 允许（F9）；If-Match revision-N 已冻结属乐观并发域（F10）；V0.2 §1.21 Runtime Revision N+1 属 config-apply 域（F11）；V0.2 零查询新鲜度规则（F12）；硬件 gate+六键测试约束实现（F13）。
- 本会话实码复核（全部逐行验证）：
  - `CanonicalRuntimeState`（runtime_state.rs:104-112）：5 Vec + `generated_at_ms: u64`，derive Serialize+Deserialize+PartialEq；**全仓无任何反序列化消费点**（Deserialize 是 wire 对称面声明，非活跃消费路径）。
  - `assemble()`（L117-187）5 参数，L185 `generated_at_ms: now_ms()`；`now_ms()` 私有（L222-227）。
  - 全仓 `assemble(` 调用点 = **5**（1 生产 session.rs:773 + 4 测试 runtime_state.rs:279/329/345/357）。
  - `SessionManager::new()`（session.rs:256-267）10 参数；生产调用点 1（main.rs:786）+ 测试调用点 6（command.rs:255, error_model.rs:137, event_projection.rs:241, idempotency.rs:251, main.rs:1362, runtime_query.rs:138, session.rs:1126 helper）。
  - `SessionManager::now_ms()` 私有 associated fn 已存在（session.rs:289-294），tick 在用。
  - `ApiQuerySnapshot`（api_boundary.rs:140-147）+ `to_api_query_snapshot`（L149-164）；测试字面量构造 CanonicalRuntimeState 6 字段（api_boundary.rs:485-492）→ 加字段触发编译级更新。
  - 硬件 gate 现态：main.rs:825-831 `serde_json::to_string_pretty(&mgr.runtime_state())` 打印（RUNTIME-STATE-RT-01）；transport.rs:209 GET /api/v1/runtime；main.rs:1268-1276 ApiQuerySnapshot 序列化+反序列化 roundtrip selftest。
  - uuid crate features = ["v4","v5","serde"]（Cargo.toml:25）——v4 与 serde 已启用，零新依赖。
  - PortIdentity.port_id: Option<Uuid>（port.rs:239，v5 派生）——仅用于评估 base_port 方案，最终不采用。

## 确认的技术方案（待 1c 用户确认）

1. **观察信封三字段**：`SnapshotObservation { revision: u64, lineage: Uuid, observed_at_ms: u64 }`。
   - **base_port 方案已否决**：`base_port: Uuid`（稳定锚点）属 identity 关切，用户裁定 identity 偏差（CANONICAL_IDENTITY §7）单独登记不混入 D14；且否决后 `SessionManager::new()` 零新参数（7 调用点不动）。
2. **CanonicalRuntimeState 加 2 字段**：`observation_revision: u64` + `observation_lineage: Uuid`，均 `#[serde(default)]`（revision→0=absent 保留语义，lineage→Uuid::nil）。additive，EXTERNAL_API_CONTRACT §2 #126。
3. **计数起点 1**：`AtomicU64::new(1)` + `fetch_add(1, Relaxed)` → 首个快照 revision=1，0 永久保留给 "absent"（消除与 serde default 0 的歧义）。
4. **assemble 签名纯化**：`assemble(devices, registry, resources, bindings, sessions, obs: &SnapshotObservation)`；`now_ms()` 从 runtime_state.rs 移除，`generated_at_ms`/`observed_at_ms` 由调用方传入 → "纯函数" 契约（F4）真正成立。
5. **序列 owner = SessionManager**（唯一生产装配点 F7 验证）：新字段 `observation_revision: AtomicU64`（初值 1）+ `observation_lineage: Uuid`（`new()` 中 `Uuid::new_v4()` 一次）。`runtime_state()` = 先 increment 后采集源（fetch_add → resources/registry/sessions 克隆 → assemble(obs)）。Relaxed ordering（原子递增自身保证序列，无需与其他内存排序）。
6. **一致性语义类（D14 关闭定义）**：**swept, non-transactional, start-ordered**——
   - 进程内 (lineage, revision) 全序；高 revision = 观测**开始**更晚（fetch_add 序）；
   - 无 per-field 单调承诺：各源字段克隆在各自锁下取点，高 revision 个别字段可持交织旧值；
   - 跨进程无全序（lineage 随机 UUID 非时序）；跨进程排序 = Federation 观察信封（V0.3-4，非目标）。
   - 不加 per-source 时间戳（F12 无新鲜度消费者，YAGNI）。
7. **命名消歧（三 revision 词表）**：D14 `observation_revision`（观察域）≠ V0.2 §1.21 `Runtime Revision N+1`（config-apply 域）≠ API `If-Match: revision-N`（乐观并发域）。wire 映射：`SnapshotRevision ← observation_revision`、`ObservedAt ← generated_at_ms`（新鲜度参考保留）。
8. **OQ1 解决（硬件 gate）**：硬件矩阵新增断言——wire JSON 含 `observation_revision`（数值 ≥1）+ `observation_lineage`（36 字符 UUID 格式）；同进程两次 /api/v1/runtime 调用 → lineage 相同、revision 严格 +1。现有断言零改动（纯加法）。
9. **OQ2 解决（模块归属）**：`SnapshotObservation` 放 `runtime_state.rs`（与唯一消费者 assemble 同文件）。
10. **测试策略（三层）**：
    - Unit：assemble 纯函数（同 obs 输入恒同输出、无墙钟读）；revision 单调 +1；serde default 双向 roundtrip（含缺字段旧 JSON 反序列化）；8 键集合测试（6→8）。
    - Simulation：连续两次 runtime_state() revision +1；8 线程并发 1000 次调用 → 修订号集合恰为 {1..1000}（无重号无空洞）；lineage 全进程唯一。
    - Hardware：盒上（lytv@10.30.15.10，bmd,gstreamer 二进制）矩阵 = 既有全矩阵（fmt+hardware-test+mock 基线 215 零回归）+ 新增观察字段断言（OQ1）。

## 关键取舍与风险

- **取舍**：base_port 否决（identity 关切不混入 D14）/ 计数起点 1（消 0 歧义）/ Relaxed ordering（序列保证来自原子性，无额外排序需求）/ 无 per-source 时间戳（YAGNI）。
- **风险 R1**：修订号空洞（进程崩溃）——契约不承诺连续，只承诺单调+唯一；击穿测试锁定 {1..N} 恰覆盖。
- **风险 R2**：`generated_at_ms` 与 `observed_at_ms` 当前同值（单装配点）——wire 保留 generated_at_ms（既有消费者），内部语义锚点用 observed_at_ms；不删不改既有字段。
- **风险 R3**：api_boundary.rs:485 测试字面量 + 4 个 runtime_state 测试调用点须同步更新——编译错误驱动，零遗漏。
- **回滚**：单 commit，revert 即回滚；无数据迁移。

## Spec Patch

无（skip_specs: true；SoT = D14 债务登记 + EXTERNAL_API_CONTRACT §2/§4 + V0.2 §1.21 + PHASE_IMPLEMENTATION_MAP；本 change 不新增 delta spec）
