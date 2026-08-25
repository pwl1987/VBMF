#!/usr/bin/env python3
"""Phase 0.6 Runner — UI-E2E (browser click path).

演进 (回应 GDOC-06): 不再 "结构连通即 PASS"。
- 通过 harness_common.evaluate_pass_rule 对 expected 做结构化求值。
- UI 行为不可旁路 UI: 真实阶段用 Playwright 驱动 browser click (playwright 占位),
  click 路径必须保留 (可降级渲染校验, 不得整体改回 API 测试)。
- 当前 measured=None (无真实浏览器驱动 <BROWSER_DRIVER>), 只能产出 HARNESS_READY。
- 真实点击路径经 pass_rule 判定通过 (含 TAKE 用 EFFECTIVE revision) 才输出 PASS。

用法:
  python docs/phase-0.6/runners/run_ui_e2e.py docs/phase-0.6/tests/UI-E2E-01-001.yaml
"""
import sys
import json
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
P06 = ROOT / "docs" / "phase-0.6"
sys.path.insert(0, str(P06 / "runners"))
import harness_common as H  # noqa: E402


def resolve_ref(test, subdir, key):
    ref = test.get(key)
    if not ref:
        return None
    p = P06 / subdir / f"{ref}.yaml"
    if not p.exists():
        raise SystemExit(f"[HARNESS] {key} '{ref}' not found at {p}")
    return H.load_yaml(p)


def main():
    if len(sys.argv) < 2:
        print("usage: run_ui_e2e.py <test.yaml>")
        sys.exit(2)
    test = H.load_yaml(Path(sys.argv[1]))
    fixture = resolve_ref(test, "fixtures", "fixture_id")
    env = resolve_ref(test, "env", "env_prereq_id")
    run_ts = time.strftime("%Y%m%dT%H%M%S")

    # --- 真实浏览器驱动调用点 (占位) ---
    # measured = browser_driver.playwright_click(fixture["seed"])  # <BROWSER_DRIVER>
    #         -> {context_preserved, runtime_active, output_health, take_uses_effective_revision, ...}
    measured = None  # 骨架阶段: 不臆造测量值 (Playwright 占位)

    result, detail = H.classify(test, fixture, env, measured)
    evidence = {
        "test_id": test["id"],
        "run_ts": run_ts,
        "ui_path": "browser-click (Playwright, <BROWSER_DRIVER> not yet wired)",
        "expect_take_uses_effective_revision": True,
        "runner_phase": "skeleton (UI click via <BROWSER_DRIVER> not yet wired)",
        "pass_rule": test.get("pass_rule"),
        "result": result,
        "measurements": detail,
    }
    art = P06 / "evidence" / f"{test['id']}_{run_ts}_{result.lower()}.json"
    art.write_text(json.dumps(evidence, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(evidence, indent=2, ensure_ascii=False))
    sys.exit(0 if result == "PASS" else 1)


if __name__ == "__main__":
    main()
