# A2-5-00 — Master Join SoT / Contract Probe

> Status: `PROBE ONLY / NO CODE CHANGE`
> Authority: `V0.2 Architecture Baseline — LOCK FINAL`（含 V0.2.4 Errata 全部修订）+ A2-4 Boundary Contract（Design §1.5a-1.5b）
> Date: 2026-09-02 · Change: a2-5-master-join · Base: master `1779429`（A2-4 CLOSED）
> 执行纪律（用户裁定）：七刀链 00 SoT Probe → 01 Domain Shape → 02 输入/输出模型裁定
> → 03 实现 → 04 ProgramMaster 聚合+AVSync 边界 → 05 Semantic Review → 06 收口。
> 本刀只探针摸底；V0.2 契约与现有代码真冲突才停裁。

---

## 1. V0.2 Join 侧证据全景（本次新增读取）

### 1.1 联合判定唯一权威句（§1.20 L155，V0.2.3 措辞修正）

> Video / Audio / Metadata 在**处理层独立隔离**，单一路径故障不会直接破坏其他
> 路径的运行实例；但 **Master Join 处会做一致性判定**，若任何一路 **failed**，
> Program Master 会进入 `DEGRADED` 或触发 `FAILOVER`。**不是"完全独立"**，
> 是"故障域隔离 + 联合判定"。

### 1.2 §8.9 Failure Domain Matrix（V0.2.4 patch 2）——Master 是 7 故障域之一

| 故障域 | 自动动作 | 切源 | 垫片 |
|---|---|---|---|
| Source | Backup / Filler | ✅ | 备源失败后 |
| Pipeline | Restart node / Backup node | 视 Session | 必要时 |
| **Master** | **Filler / Emergency** | **✅** | **✅** |
| Output | Restart adapter / alternate | ❌ | ❌ |
| Recording / Clock / Resource | … | ❌ | ❌ |

- 关键规则：**节目源没故障时绝不能因 Output 故障切源**；此表由
  **Safety + Watchdog + Health Tree 共同执行，不新增 Engine**（L2151）。

### 1.3 §8.10 AV Sync 异常决策（V0.2.4 Cleanup-2）

- yellow（|Δ|>100ms）→ compensate + monitor；red（|Δ|>250ms）→
  **classify_failure_domain 先分类后动作**（消费 §8.9）：
  SOURCE→FAILOVER / PIPELINE→RESTART / OUTPUT→OUTPUT_RECOVERY /
  **PLAYER→NOTIFY（绝不切源）** / UNKNOWN→SAFE_DEGRADE+ALERT；
- 旧版 `av_offset>250ms → CRITICAL+切 backup` **已删除**（绝对规则禁令）。

### 1.4 §8.11 三轴状态机（Cleanup-2）——health 轴含 UNKNOWN

`lifecycle{STOPPED/STARTING/RUNNING/STOPPING} × readiness{NOT_READY/
READY_TO_TAKE} × health{HEALTHY/DEGRADED/FAILED/UNKNOWN}`——**health
UNKNOWN 是独立合法值**（"Backup 启动中=UNKNOWN"等组合表 L2229-2238）；
Channel Health SoT = Health Tree Aggregation（§3.9）。

### 1.5 §3.13/§3.8 AVSync Manager（Errata-9）

= **Measurement + Offset/Drift Correction + Failure Classification**；
**不做 Recovery Action**（§8.9 是 Recovery Policy SoT）；识别层（§3.13）
与决策层（§8.9）分离；7 OperationalFailureDomain vs DiagnosticFailureClass
（PLAYER/UNKNOWN 只 NOTIFY/SAFE_DEGRADE）。

### 1.6 Master Join 全出现点（11 处，grep 复核）

§1.20 图（L147）+ L154/155（AV Sync 测量在 Join + 联合判定）+ §3.7 三个
Join 节点（Video/Audio/Metadata Master Join, L783/796/805）+ L830（AV Sync
是 Join 属性非普通节点）+ §3.9 Health Tree 两节点（L908/909）+ L2362
（三 graph 归一图）。**V0.2 无独立 "MasterJoin" 章/词表/DB 表**——Join
语义散布于上述节。

## 2. 代码现状（引用 A2-4-04 探针，@1779429 复核未变）

Join/ProgramMaster/AVSync/FAILOVER/READY_TO_TAKE/events-metadata-kind
**全零生产代码**（J1/J2/J5/J6/J7/J9）；failed/health 全在 Runtime 平面
（AgentState + RuntimeEvent::reduce，J3）；`absence≠evidence` 先例 =
CapabilityFlag::Unknown（J4）。三 Master 齐：VideoMaster/AudioMaster
（stage 终态 + is_program_scope_master()）+ MetadataMaster（facts +
join_declaration）。

## 3. 十危险点证据锚（用户裁定必锁，逐条 SoT/代码双锚）

| # | 危险点 | V0.2 锚 | 代码/契约锚 |
|---|---|---|---|
| 1 | Unknown ≠ NotPresent | §8.11 health 轴 UNKNOWN 独立值；§8.10 UNKNOWN→SAFE_DEGRADE（非 FAIL） | CapabilityFlag::Unknown（absence≠evidence）+ MetadataPresence 三态分离 |
| 2 | NotPresent ≠ Failed | §1.20 L155 只有 "failed" 才 DEGRADED/FAILOVER；§8.9 反例（Output 故障不切源） | C′：NotPresent=结论性负声明，正常态 |
| 3 | facts ≠ declaration | —（V0.2 无此粒度） | A2-4 三层规则 L2 禁机械推导四条（Design §1.5a） |
| 4 | Participating+[] ≠ 明确没有 | — | 快照语义（semantic-review §7 矩阵行 1） |
| 5 | NotPresent+Present fact → fail-closed | —（V0.2 未及） | C′ Join 消费规则第 4 条（join-boundary-review §3） |
| 6 | Runtime failed 只来自 Runtime/Health/Event | §8.9"由 Safety+Watchdog+Health Tree 执行" | A2-4-04 J3：AgentState+reduce 唯一派生；Program 域零健康 |
| 7 | D14 revision 不进 Program/Join | —（D14 三 revision 消歧为 Runtime 域） | Design §1.5b A2-5 红线（用户终裁） |
| 8 | Timecode SoT = CanonicalMediaDescriptor | §3.7 L801（Timecode=Metadata 源） | normalize.rs L104；OQ-1 终裁 ownership 四行 |
| 9 | AV Sync = Master Join 职责 | §3.7 L830 + §3.8 + §2.4 L319 + 决策 #37/#56 | 零既有类型（J7）——A2-5 全新声明面 |
| 10 | MetadataMaster 禁 stage/ready/health/status/revision/timestamp | — | 16 行字段表 + G-A 不变量（A2-4 verify §2） |

## 4. Open Questions（A2-5 特有，交用户裁决）

| # | 问题 | 证据 | 倾向（非裁决） |
|---|---|---|---|
| OQ-A | **Join 输出与 §8.9 Master 故障域的关系**：Master 域=7 域之一且"Program Master 失败→切源+垫片"（L2142）；但 failed 事实只能来自 Runtime 平面（危险点 6）。Join 的 DEGRADED 判定输出是 §8.9 Master 域的**输入信号**，还是 Master 域故障另有 Runtime 派生路径？ | §8.9 L2142+L2151 vs A2-4-04 J3 | Join 出**判定声明**（Program-scope 语义事实）→ Runtime/§8.9 消费为 Master 域信号——与"Event Projection 不成 Join"同构 |
| OQ-B | **ProgramMaster 形态**：V0.2 无独立 ProgramMaster 词表（§1.20 Program Master=三者联合的称谓；§3.7 产物=各路 Program-scope Master）。ProgramMaster 是第四个 domain object、Join 的输出视图、还是组合（三 Master + Join 结果声明）？ | §1.20/§3.7 全文 | 组合而非新 God Object——A2-5-01/02 裁 |
| OQ-C | **AVSync 在 A2-5 的落地范围**：§3.8 Manager 三职能（Measurement/Correction/Classification），执行（真测量/真补偿）属 GStreamer（A2-7+）？ | §3.8 + A2-7 边界 | A2-5 只做**声明面**（观测/校正声明 + 分类输入），执行面 A2-7 |
| OQ-D | **§8.10 classify 归属**：av_sync_decision 的 yellow/red/classify 链是 Recovery 决策（消费 §8.9）——A2-5 Join 承载 classify 还是只提供 measurement/classification 输入供 Runtime 消费？ | §8.10 + Errata-9 识别/决策分离 | Join 只到 classification 输入；Recovery 动作归 §8.9 Runtime（红线同构） |
| OQ-E | **三路 Join 就绪输入的形态**：Video/Audio 有 stage 终态（is_program_scope_master），Metadata 只有 declaration——三路输入不对称；禁 `all==MASTER_JOINED` 简单检查（A2-4 终裁）。就绪判定谓词的输入集如何定义？ | 三 Master 现状 | A2-5-02 输入/输出模型裁定 |

## 5. Proposed Decisions（仅为提案，待裁决）

- PD-1：A2-5 Join = **Program Domain 内的联合判定声明对象**（纯函数/纯数据），
  不执行 Recovery、不持有 health、不产生 RuntimeEvent（消费边界对称于
  "Event Projection 不成 Join"）。
- PD-2：Join 判定输入 = 三 Master 声明 + Runtime 提供的 failed 事实（作为
  参数注入，非 Join 自取）——保持 Program/Runtime 平面单向依赖。
- PD-3：AVSync 声明面（观测快照 + 分类输入）与 Recovery（§8.9）严格分离。
- PD-4：ProgramMaster 按 OQ-B 裁决，倾向组合（零新 God Object）。

## 6. No-Build Gate（本刀禁止）

Rust 生产代码（master_join.rs/program_master.rs/avsync 类型/Join 实现）；
三 Master 任何修改；Runtime/Event/Health 任何修改；词表冻结（待 01/02）；
D14 语义引用。

## 7. 证据文件清单

ARCHITECTURE_V0.2.md：§1.20 L138-155 · §2.4 L319 · §3.7 L759-872（L830）
· §3.8/§3.13 L874-895/L1237-1283 · §3.9 L901-933 · §8.9 L2134-2176 ·
§8.10 L2178-2204 · §8.11 L2206-2248 · 决策 #29/#37/#42/#49/#53-57 ·
§5 L1480-1483（avsync_measurements 表）。
代码 @1779429：program/{switch_policy,video,audio,metadata}_master.rs ·
health.rs · runtime_state.rs L249-255 · events.rs · A2-4 归档
（Design §1.5a-1.5b + join-boundary-review 报告）。
