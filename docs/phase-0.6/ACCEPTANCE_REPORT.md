# Phase 0.6 Acceptance Report

> **Status**: DRAFT (G-DOC in progress — Structure/Coverage/Executor modeled, Runtime NOT executed)
> **Gates**: G-DOC → G-DOC-READY → G-RUNTIME → G-UIUX
> **Rule**: 任意 Gate FAIL = Phase 0.6 NOT ACCEPTED

## 关键语义澄清 (回应 GDOC-04/05/06)

- **G-DOC-READY ≠ Runtime 真实执行 PASS**。G-DOC-READY 仅证明 "规范已完整建模 + 骨架可连通"。
- Runner 当前 `measured=None` (真实 runtime 调用点 `<RUNTIME_RPC>`/`<INJECTION_HOOK>`/`<BROWSER_DRIVER>` 未接入),
  只能产出 **`HARNESS_READY`**, **严禁输出 `PASS`**。
- `HARNESS_READY` = 骨架连通; `PASS` = 真实测量值经 `evaluate_pass_rule` 对 `expected` 全判定为真。

## Gate Results

| Gate | Result | Blocked By |
|---|---|---|
| G-DOC | ✅ | 结构/规范/引用闭环已建立 (见下三子门禁) |
| G-DOC-READY | 🟢 STRUCTURE✓ COVERAGE✓ EXECUTOR✓ | scripts/check_docs.py phase06 全绿 (注意: 仅规范建模, 非运行时 PASS) |
| G-RUNTIME | ⛔ | A1 → A2 → B → FI-01A/B/02~07 → HA-01~07 (先 Runtime 再 UI); 需接入真实 runtime |
| G-UIUX | ⛔ | UI-E2E-01~04 + TAKE revision 验证; 需 Playwright 真实浏览器 |

## G-DOC-READY 三子门禁

| 子门禁 | 校验内容 | 状态 |
|---|---|---|
| G-DOC-STRUCTURE | fixture_id / env_prereq_id / runner 文件均存在 | ✅ |
| G-DOC-COVERAGE | 规范条目族全落地 (HA-01~07 / UI-E2E-01~04 / AC-03B+AC-03B-2+AC-03B-2-6 / A1/A2/B / FI-01A/B/02~07 各 ≥1 Test Case) | ✅ |
| G-DOC-EXECUTOR | runner 接入 evaluate_pass_rule 且区分 HARNESS_READY/PASS 三态 | ✅ |

## Test Case Index (24)

| Test ID | Gate | Reference | Fixture | Env | Runner | Exec Result |
|---|---|---|---|---|---|---|
| AC-01-001 | G-RUNTIME | A1 | F-A1-PASS | ENV-LAB-01 | run_reference_a1.py | HARNESS_READY |
| A2-001 | G-RUNTIME | A2 | F-A2-SDI-FRAME | ENV-LAB-01 | run_reference_a1.py | HARNESS_READY |
| B-001 | G-RUNTIME | B | F-B-HETEROGENEOUS | ENV-LAB-01 | run_reference_a1.py | HARNESS_READY |
| AC-03B-001 | G-RUNTIME | AC-03B | F-AC03B-OVERRIDE | ENV-LAB-01 | run_reference_a1.py | HARNESS_READY |
| AC-03B-2-001 | G-RUNTIME | AC-03B | F-AC03B2-RESTART | ENV-LAB-01 | run_reference_a1.py | HARNESS_READY |
| AC-03B-2-6-001 | G-RUNTIME | AC-03B | F-AC03B2-6-CLOCK | ENV-LAB-01 | run_reference_a1.py | HARNESS_READY |
| FI-01A-001 | G-RUNTIME | FI | F-FI-01A-SDI-FREEZE | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| FI-01B-001 | G-RUNTIME | FI | F-FI-01B-NDI-LOSS | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| FI-02-001 | G-RUNTIME | FI | F-FI-02-... | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| FI-03-001 | G-RUNTIME | FI | F-FI-03-... | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| FI-04-001 | G-RUNTIME | FI | F-FI-04-... | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| FI-05-001 | G-RUNTIME | FI | F-FI-05-... | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| FI-06-001 | G-RUNTIME | FI | F-FI-06-MASTER-JOIN | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| FI-07-001 | G-RUNTIME | FI | F-FI-07-RECORDING | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| HA-01-001 | G-RUNTIME | HA | F-HA-CONTROLLER-FAIL | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| HA-02-001 | G-RUNTIME | HA | F-HA-02-BACKUP-STANDBY | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| HA-03-001 | G-RUNTIME | HA | F-HA-03-BOTH-FAILED | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| HA-04-001 | G-RUNTIME | HA | F-HA-04-ACTIVE-DEGRADED | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| HA-05-001 | G-RUNTIME | HA | F-HA-05-ACTIVE-UNKNOWN | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| HA-06-001 | G-RUNTIME | HA | F-HA-06-STANDBY-FAILED | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| HA-07-001 | G-RUNTIME | HA | F-HA-07-ABSORBED | ENV-LAB-01 | run_fi_matrix.py | HARNESS_READY |
| UI-E2E-01-001 | G-UIUX | UI-E2E | F-UI-PROFILE-FLOW | ENV-LAB-01 | run_ui_e2e.py | HARNESS_READY |
| UI-E2E-02-001 | G-UIUX | UI-E2E | F-UI-TRANSCODE-FLOW | ENV-LAB-01 | run_ui_e2e.py | HARNESS_READY |
| UI-E2E-03-001 | G-UIUX | UI-E2E | F-UI-SESSION-FLOW | ENV-LAB-01 | run_ui_e2e.py | HARNESS_READY |
| UI-E2E-04-001 | G-UIUX | UI-E2E | F-UI-NAV-CLOSURE | ENV-LAB-01 | run_ui_e2e.py | HARNESS_READY |

## Surface → E2E → Acceptance Coverage (UI-E2E-04)

| Surface ID | Covered | Evidence |
|---|---|---|
| (filled per Acceptance run) | Y/N | {test_id}_{run_ts} |

## Notable Findings

- Reference B 执行复杂度最高 (ARCH-01) — 置于 A1/A2/FI 之后。
- Pending Revision (r18) vs EFFECTIVE (r17): TAKE 必须只使用 EFFECTIVE (G-UIUX/TAKE-REVISION-001)。
- ENV-01: ENV-LAB-01 仅记录拓扑占位; G-RUNTIME 前需补 `ENV-PREFLIGHT-001` (FFmpeg/BMD/PTP/ports/codecs 自检)。
