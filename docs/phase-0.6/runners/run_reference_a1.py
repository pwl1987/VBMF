#!/usr/bin/env python3
"""Phase 0.6 Runner — Reference A1 (PACKET switch) + AC-03B Override.

演进 (回应 GDOC-04/05): 不再 "文件存在即 PASS"。
- 通过 harness_common.evaluate_pass_rule 对 expected 做结构化求值。
- 当前 measured=None (无真实 runtime 调用点 <RUNTIME_RPC>), 只能产出 HARNESS_READY。
- 真正实体测量值经 pass_rule 判定通过才输出 PASS。

用法:
  python docs/phase-0.6/runners/run_reference_a1.py docs/phase-0.6/tests/AC-01-001.yaml
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
        print("usage: run_reference_a1.py <test.yaml>")
        sys.exit(2)
    test = H.load_yaml(Path(sys.argv[1]))
    fixture = resolve_ref(test, "fixtures", "fixture_id")
    env = resolve_ref(test, "env", "env_prereq_id")
    run_ts = time.strftime("%Y%m%dT%H%M%S")

    # --- 真实 runtime 调用点 (占位) ---
    # measured = runtime_rpc.switch_take(source=..., mode=...)  # <RUNTIME_RPC>
    # measured = runtime_rpc.query_health(channel=...)          # 返回 {no_black_frame: bool, p99_latency_ms: int, ...}
    measured = None  # 骨架阶段: 不臆造测量值

    result, detail = H.classify(test, fixture, env, measured)
    evidence = {
        "test_id": test["id"],
        "run_ts": run_ts,
        "fixture": test.get("fixture_id"),
        "env": test.get("env_prereq_id"),
        "runner_phase": "skeleton (measured via <RUNTIME_RPC> not yet wired)",
        "pass_rule": test.get("pass_rule"),
        "result": result,
        "measurements": detail,
    }
    art = P06 / "evidence" / f"{test['id']}_{run_ts}_{result.lower()}.json"
    art.write_text(json.dumps(evidence, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(evidence, indent=2, ensure_ascii=False))
    # HARNESS_READY / FAIL 不视为 Acceptance PASS; 仅 PASS 退出 0
    sys.exit(0 if result == "PASS" else 1)


if __name__ == "__main__":
    main()
