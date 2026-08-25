# Phase 0.6 Test Case Schema (G-DOC SoT)

本文件是 Phase 0.6 可执行验收的 **Test Case YAML 结构定义 (SoT)**。所有 `tests/*.yaml` 必须遵循此 schema。机器可判定字段（pass_rule / expected / retry / abort）由 `runners/` 脚本消费。

```yaml
id: AC-01-001                 # 唯一 Test ID (Gate-Reference-Seq)
title: Network Source A1 PACKET switch, primary freeze
gate: G-RUNTIME               # G-RUNTIME | G-UIUX | G-DOC
reference: A1                 # A1 | A2 | B | FI | HA | UI-E2E | AC-03B
fixture_id: F-A1-PASS         # 关联夹具 (fixtures/)
env_prereq_id: ENV-LAB-01     # 环境前置 (env/)
runner: runners/run_reference_a1.py   # 执行入口
depends_on: []                # 前置 Test ID
timeout: 300                  # 秒

preconditions:
  - "Channel CH01 exists, Program Master HEALTHY"
  - "Primary SDI SOURCE ACTIVE"

actions:
  - step: 1
    do: "Issue PACKET switch command via JSON-RPC switch.take({source:'PRIMARY'})"
  - step: 2
    do: "Freeze primary SDI input 5s (FI-01A injection via fixture)"

expected:                     # 确定性期望 (机器可判定)
  p50_latency_ms: 8
  p95_latency_ms: 12
  p99_latency_ms: 16
  active_source_id: "PRIMARY"
  no_black_frame: true
  no_pts_discontinuity: true

pass_rule: >-
  active_source_id == 'PRIMARY'
  AND no_black_frame == true
  AND no_pts_discontinuity == true
  AND p99_latency_ms <= 20

negative_case:
  id: AC-01-001-NEG
  do: "Switch to absent source id"
  expect_fail: "switch rejected with INVALID_SOURCE, no state change"

evidence:
  - "evidence/{id}_{run_ts}_switch_log.json"
  - "evidence/{id}_{run_ts}_metrics_dump.json"

artifact_naming: "{id}_{run_ts}_{pass|fail}.json"

retry_rule:
  max: 1
  backoff_s: 30

abort_rule:
  consecutive_fail: 3        # 连续 3 次 FAIL → HALT Gate
```

## 字段约束

| 字段 | 必填 | 说明 |
|---|---|---|
| id | ✅ | 全局唯一; 格式 `{GATE}-{REF}-{SEQ}` |
| gate | ✅ | G-RUNTIME / G-UIUX / G-DOC |
| fixture_id | ✅ | 必须存在于 `fixtures/` |
| env_prereq_id | ✅ | 必须存在于 `env/` |
| runner | ✅ | 必须存在于 `runners/` 且可执行 |
| pass_rule | ✅ | 布尔表达式, 由 runner 求值 |
| artifact_naming | ✅ | 证据文件命名模板 |
| abort_rule | ✅ | Gate 级中止条件 |

> **G-DOC-READY 门禁**: 所有 `tests/*.yaml` 必须通过 `scripts/check_docs.py phase06` 的引用合法性校验 (fixture_id / env_prereq_id / runner 实际存在), 且每个 FI/AC ID 在 `tests/` 中有 ≥1 对应 Test Case, 方可进入 G-RUNTIME。
