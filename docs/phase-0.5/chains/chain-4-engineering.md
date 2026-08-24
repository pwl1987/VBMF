# 链 4：Engineering 工程（Engineer 系统工程师）

> V0.2 §10.11 链 4 锁定
> 角色：Engineer（系统工程师）
> 端到端：Graph Designer → 拖节点连边 → Save Spec → Compile (X1) → Resource Plan → Preflight (X2) → Apply with Change Set (X3) → Runtime 上线 → Health Tree (X5) → QC 持续监测 → 异常 → Incident (X4)

## 流程

```
[Engineer 登录] → [Graph Designer 08]
  ↓
[设计 / 修改 Graph]
  ├─ 拖入 Source 节点
  ├─ 拖入 Process 节点 (Switcher / Composition / Audio)
  ├─ 拖入 Output 节点
  └─ 连边 + 声明 Data Plane / Clock Domain / Edge Policy
  ↓
[Save Spec → graph_specs DRAFT]
  ↓
[Compile (X1): Validator / Insert Missing / Clock Align / Latency / Resource / Emit Runtime]
  ↓
[Compile Report 编译报告: 0 critical / 1 warning]
  ↓
[Resource Plan 资源计划: cpu=18, gpu=0, ram=12GB, nic=8Gbps]
  ↓
[Preflight (X2): Graph 类]
  ├─ Resource Vector 9-dim ✓
  ├─ Device Token (BMD_PORT) ✓
  ├─ Clock Reference 完整 ✓
  └─ 0 critical
  ↓
[Apply with Change Set (X3)]
  ↓
[Logical Atomic / Transactional Cutover 业务层原子切换]
  ↓
[Runtime 上线: media_sessions 启动]
  ↓
[Health Tree (X5): 7 Health Invariants 全部通过]
  ↓
[QC 持续监测（raw / compressed / metadata）]
  ↓
[异常: AV sync drift +12ms]
  ↓
[ALERT → Incident (X4) #1247]
  ↓
[Engineer 下钻 Health Tree → 修复]
```

## 步骤明细

| 步 | 操作 | 页面 / 引擎 | 关键检查 | 输出 |
|---|---|---|---|---|
| 1 | Engineer 登录 | — | RBAC: Engineer | 拒绝 |
| 2 | 打开 Graph Designer | 08-graph-designer | 当前 Channel Graph 加载 | — |
| 3 | 拖入 / 修改节点 | 08-graph-designer | Node Palette 11 Source + N Process + N Output | — |
| 4 | 连边 + Edge Policy | 08-graph-designer | Data Plane / Clock / Latency / Capability | 边声明 |
| 5 | Save Spec | — | graph_specs DRAFT | — |
| 6 | Compile (X1) | 自动 | Validator / Insert Missing / Clock Align / Latency / Resource | graph_revisions VALIDATED |
| 7 | Resource Plan | 自动 | 9-dim Vector + Device Token | resource_plan_json |
| 8 | Preflight (X2) | 自动 | Resource / Clock / Data Plane | preflight_report |
| 9 | Dry Run 试运行 | 08-graph-designer | 模拟运行 | dry_run_output |
| 10 | Apply Change Set (X3) | 手动 | DRAFT → VALIDATED → APPLIED | config_revisions + change_sets |
| 11 | Logical Atomic Cutover | 自动 | snapshot + prepare + commit | media_sessions 启动 |
| 12 | Runtime 上线 | 自动 | media_session_runtime 三轴 | lifecycle=RUNNING |
| 13 | Health Tree 7 Invariants | 自动 | H1-H7 | channel_health_view |
| 14 | QC 持续监测 | 自动 | raw/compressed/metadata | qc_reports |
| 15 | 异常检测 | 自动 | AV sync / freeze / black | ALERT |
| 16 | Incident (X4) | 自动 | severity + summary | incidents #1247 |
| 17 | Engineer 下钻 | 09-health-tree | 颜色 + details | 诊断 |
| 18 | 修复 | — | 修改 Graph / 调参 / 重启 | 新 Change Set |

## X1 Graph Compiler 输出

```yaml
graph_compile_report:
  status: OK
  original_nodes: 14
  auto_inserted:
    - { node: Decode, reason: "next node needs RAW_VIDEO" }
    - { node: Encode, reason: "next node needs COMPRESSED_VIDEO" }
  clock_align:
    - { source: PTP, target: SYSTEM, action: insert_converter }
  latency_estimate_ms: 1800
  resource_plan:
    cpu_threads: 18
    gpu_sessions: 0
    vram_mb: 0
    ram_mb: 12288
    ingress_mbps: 850
    egress_mbps: 850
    disk_write_mbps: 50
    pcie_rx_mb_s: 320    # scheduling estimate 调度估算
    pcie_tx_mb_s: 320    # scheduling estimate 调度估算
  warnings:
    - "redundant path: Source.A and Source.B both reach Switcher"
```

## X2 Preflight 3 类

```yaml
preflight:
  graph 图设计:    # 静态 Graph 检查
    - cycle_check 环检测
    - data_plane_compatibility 数据面兼容
    - clock_domain_resolvable 时钟域可解
    - resource_vector_within_capacity 资源向量在容量内
    - device_token_available 设备 token 可用
  playout 节目单:  # Playout 节目单检查
    - loudness 响度
    - rights 版权
    - duration 时长
    - codec 编码
  channel 通道:    # Channel 通道检查
    - capability_contract_pass
    - runtime_alignment_pass
    - hot_standby_level_consistent
    - health_tree_invariants_pass
```

## X3 Configuration Versioning 变更集

```yaml
change_set:
  id: CS-2026-0824-009
  target_type: GraphSpec
  target_id: gs-CH01
  before_rev: REV-007
  after_rev: REV-008
  status: APPLIED
  phase: COMMITTED
  snapshot_id: SNAP-007
  applied_at: 2026-08-24 14:25:00
  applied_by: eng-zhang
  preflight_report_id: PFR-2026-0824-009
  dry_run_output_id: DRY-2026-0824-009
```

## Logical Atomic / Transactional Cutover 业务层原子切换

```
snapshot 快照(rev-007)
  ↓
prepare 准备(rev-008)
  ├─ compile_runtime
  ├─ allocate_resources
  └─ preflight_check
  ↓
  ├─ success → commit 提交 (rev-008 active)
  └─ failure → rollback 回滚 (rev-007 restored)
```

**🔴 V0.2 关键边界**：禁止 DB 事务层面"原子切换"——这是业务层原子，不是数据库原子。

## 关键引擎 / 横切能力映射

| 步骤 | 引擎 / 能力 |
|---|---|
| Graph Designer 图设计 | §3.10 Graph Compiler (X1) |
| 节点库 | §3 全部 12 Engines |
| 边声明 | §1.16 Data Plane Label / §1.18 Clock Domain |
| Compile 编译 | X1 Graph Compiler |
| Preflight 预检 | X2 Preflight |
| Apply 应用 | X3 Configuration Versioning |
| Atomic Cutover 业务层原子 | X3 (snapshot+prepare+commit) |
| Runtime 运行时 | §3 全部 Engines 上线 |
| Health Tree 健康树 | X5 Health Tree |
| QC 质量检测 | §3 QC Engine |
| Incident 事件 | X4 Incident Timeline |

## Phase 0.6 验收用例

- **Eng-01**：修改 Graph（加新 Output）→ Compile OK → Preflight 0 critical → Apply 成功
- **Eng-02**：Graph 含环 → Compile 拒绝 → 不能 Apply
- **Eng-03**：Resource 不足 → Preflight 拒绝 → 提示扩资源或减 Session
- **Eng-04**：Clock Domain 不可解析 → Preflight 拒绝
- **Eng-05**：Apply 后立即 Rollback → Graph 恢复到 REV-007（< 5s）
- **Eng-06**：Runtime 启动后 7 Health Invariants 全部 PASS
- **Eng-07**：QC 检测到异常 → 自动 Incident 建档 → Engineer 收到通知

## 关联 Wireframe

- `wireframes/08-graph-designer.html`（主入口）
- `wireframes/02-sources.html`（Source 状态）
- `wireframes/09-health-tree.html`（运行时监控）
- `wireframes/07-recording.html`（录像回溯）

## 关键禁忌

- ❌ **禁止直接修改 APPLIED 的 Graph**（必须新建 Change Set）
- ❌ **禁止运行时修改 Source Adapter 类型**（必须新 Apply）
- ❌ **禁止 Dry Run 失败的 Graph Apply**
- ❌ **禁止 Logical Atomic Cutover 与普通 Apply 混用**（前者有 snapshot+rollback，后者无）
- ❌ **禁止 Runtime 修改的 `media_session_runtime.effective_switch_mode` 反写 `channel_routes.switch_mode`**（V0.2 §3.4 关键边界）
- ❌ **禁止把 `current_host_snapshot` 内容写进 Architecture**（V0.2 §3.11 关键边界）
- ❌ **禁止把 `pcie_*_mb_s` 当成实测值**（仅 scheduling estimate 调度估算）
