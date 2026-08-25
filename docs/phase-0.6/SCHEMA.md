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
| pass_rule | ✅ | 布尔表达式, 由 runner **真实求值** (对 `expected` 字段结构化断言, 非 "文件存在即 PASS") |
| artifact_naming | ✅ | 证据文件命名模板 |
| abort_rule | ✅ | Gate 级中止条件 |

### Runner 结果三态 (回应 GDOC-04/05/06)

| 结果 | 含义 | 是否构成 Acceptance PASS |
|---|---|---|
| `HARNESS_READY` | 骨架连通 (fixture/env/pass_rule 解析成功), 但 `measured` 来自占位 `<RUNTIME_RPC>`/`<INJECTION_HOOK>`/`<BROWSER_DRIVER>`, 未经过真实测量 | ❌ 否 |
| `PASS` | 真实 `measured` 经 `evaluate_pass_rule` 对 `expected` 全部判定为真 | ✅ 是 |
| `FAIL` | `pass_rule` 求值存在假值, 或骨架连通失败 | ❌ 否 |

> 当前阶段 runner 的 `measured=None` (真实 runtime 调用点未接入), 只能产出 `HARNESS_READY`, **严禁输出 PASS**, 防止验收报告污染。

> **G-DOC-READY 门禁 (三子门禁, 回应 GDOC-02)**: 所有 `tests/*.yaml` 必须通过 `scripts/check_docs.py phase06`, 拆为:
> - **G-DOC-STRUCTURE**: fixture_id / env_prereq_id / runner 文件实际存在 (防 "写了没建")。
> - **G-DOC-COVERAGE**: 规范条目族全落地 — HA-01~07、UI-E2E-01~04、AC-03B + AC-03B-2 + AC-03B-2-6、A1/A2/B、FI-01A/B/02~07 每个 ≥1 Test Case (id 前缀结构化比对, 非子串猜测)。
> - **G-DOC-EXECUTOR**: runner 已接入 `evaluate_pass_rule` 且区分 `HARNESS_READY/PASS` 三态, 不再 "文件存在即 PASS"。
>
> 三者全绿 = G-DOC-READY = GREEN。注意: 这仅证明 **"G-DOC 规范已完整建模且骨架可连通"**, 不等于 **"Runtime 真实执行 PASS"**。
