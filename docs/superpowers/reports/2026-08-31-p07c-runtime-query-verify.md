# Verify 报告 — p07c-runtime-query (Phase 0.7C-2: Runtime Query Model + D6)

- 日期: 2026-08-31
- 分支: `comet/p07c-runtime-query`（自 master `c1b74b9` = 0.7C Foundation 拉出）
- 验证模式: **full**；盒 10.30.15.10 为准
- 契约对齐: PHASE_IMPLEMENTATION_MAP §3（Query Model 项）；终审 §十二-§十四（只读/snapshot/Pure Read 原则）

## Summary

| Dimension | Status |
|-----------|--------|
| Completeness | 4 任务组 9 项全落地（四栏纪律全 Pass） |
| Correctness | 盒上 final 矩阵：fmt 0 · test **138/138/165/138** · clippy -D ×4 零警告 · build ×3 · PROOF PASS · 真机 SESSION/RESOURCE-RT-01 回归 ALL PASS + capabilities 投影证据 |
| Coherence | Pure Read 白盒（allowlist + 命令动词禁入）；零新 DTO（返回既有类型）；不做清单零越界 |

**结论: PASS — 0 CRITICAL / 0 IMPORTANT / 1 NOTE。**

## 1. 完整验证 7 项

| # | 检查项 | 结果 | 证据 |
|---|--------|------|------|
| 1 | tasks 全部完成 | ✅ | 四栏纪律表全 Pass |
| 2 | 符合 open design.md | ✅ | D1-D8 逐项落地；不做清单零越界（无命令动词/无 API/无 SDK 深探针/D14·D15 只登记） |
| 3 | 符合 Design Doc | ✅ | §1-§6 实现级一致 |
| 4 | 能力规格场景 | ✅ (N/A) | skip_specs |
| 5 | proposal 目标满足 | ✅ | RuntimeQuery 门面 + D6 projection/硬判定 + D14/D15 登记 + 三层 |
| 6 | delta spec 无矛盾 | ✅ (N/A) | 无 delta spec |
| 7 | Design Doc 可定位 | ✅ | frontmatter 关联本 change（design guard PASS） |

## 2. 盒上最终矩阵（final code, 全绿）

fmt apply/check **0** · test **138 (default) / 138 (simulation) / 165 (mock) / 138 (bmd,gstreamer)**（0.7C-1 基线 138/138/161/138 → +4 runtime_query 门禁测试）· clippy -D ×4 零警告 · build ×3 · PROOF PASS。迭代：RQ1（类型补丁首轮断言失败未写盘——长补丁教训再现，分段修复）→ RQ2（import 归属）→ RQ3（fixture 空数组）→ RQ4（borrow 顺序）→ RQ5（needless borrow）→ RQ6 全绿。

## 3. 门禁 RUNTIME-QUERY-RT-01 逐层验收

| 层 | 测试/证据 |
|----|-----------|
| Unit | `pure_read_public_surface`（allowlist 硬编码 + 13 个命令动词禁入——Pure Read/Snapshot Semantics 白盒）· `get_paths_hit_and_miss`（device/port/resource/session 命中 + 幽灵 id → None，绝不臆造）· D6 三态（Unsupported FAIL / Unknown WARN / Supported Pass——`d5_binding_strength_checked` 等既有测试的同型三态断言；capability 测试在 `capability_projection` + preflight 层） |
| Simulation | `capability_projection`（mock 设备 0 注入 Supported：can_input=Supported/can_output=Unsupported/input_ports=Some(1)）· `simulation_session_lifecycle_projection`（create→query 会话可见（Leased/Reserved）→close 后不可查询） |
| Hardware | 真机 SESSION_LIFECYCLE：runtime_state JSON 含 `"capabilities": {can_input: "unknown", ...}` 投影——**真机 DeviceCapabilities 未探测 → Unknown 合法（absence≠evidence）**；SESSION/RESOURCE-RT-01 回归 ALL PASS |

## 4. 代码审查（review_mode=standard）+ NOTE

- **改动面**：新 `runtime_query.rs`（门面 + 4 测试）；`runtime_state.rs`（CapabilityFlag/DeviceCapabilitiesSummary + project_capabilities + DeviceRuntimeState.capabilities + D14/D15 契约注释）；`preflight.rs`（BackendCapability 硬判定 + project_input_capability）；`main.rs`（mod + SESSION_LIFECYCLE mgr Arc 化 + _rq 门面）；债务表 + Phase Map。
- **红线核验**：①Pure Read——公开面 8 项全 get_/list_ 前缀，13 命令动词白盒禁入；②零新 DTO——返回既有 CanonicalRuntimeState 子项；③零 vendor——runtime_query.rs import 仅 session/runtime_state/uuid。
- **D6 语义**：Unsupported ⇒ FAIL（硬决策）；Unknown ⇒ WARN（ProbeFailed 同 Unknown——探测失败≠不支持）；真机 Unknown 合法。**closure ≠ forever**：真实 BMD SDK 深探针登记为演进项。
- **NOTE-1**：RQ1 复现了"长 Python 补丁断言失败静默跳过后续段"（0.7A R1 教训）——本轮以分段写盘修复；该模式已两次出现，**后续补丁一律用 Write 工具落盘脚本文件再执行**（不再用内联 heredoc 长补丁）。
- **结论**：0 CRITICAL / 0 IMPORTANT。

## 5. 交付路径

archive → 单一 PR `comet/p07c-runtime-query` → `master`（七 checks）→ merge → tag `phase-0.7C2-runtime-query` → 删分支。0.7C 前置债务（D2/D4/D5/D6）全部 CLOSED；Phase Map §3 下一项 = **Command Contract**。
