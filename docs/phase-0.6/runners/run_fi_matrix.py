#!/usr/bin/env python3
"""Phase 0.6 Runner — FI Matrix (FI-01A/B/02~07).

加载 FI Test Case + Fixture, 对 injection_point / expected_recovery 做骨架校验,
产出 evidence。真实注入点以 `<INJECTION_HOOK>` 占位。

用法:
  python docs/phase-0.6/runners/run_fi_matrix.py docs/phase-0.6/tests/FI-01A-001.yaml
"""
import sys
import json
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
P06 = ROOT / "docs" / "phase-0.6"


def load_yaml(path):
    try:
        import yaml
        return yaml.safe_load(Path(path).read_text(encoding="utf-8"))
    except ImportError:
        raise SystemExit("PyYAML required in real env: pip install pyyaml")


def resolve_ref(test, subdir, key):
    ref = test.get(key)
    if not ref:
        return None
    p = P06 / subdir / f"{ref}.yaml"
    if not p.exists():
        raise SystemExit(f"[HARNESS] {key} '{ref}' not found at {p}")
    return load_yaml(p)


def main():
    if len(sys.argv) < 2:
        print("usage: run_fi_matrix.py <test.yaml>")
        sys.exit(2)
    test = load_yaml(sys.argv[1])
    fixture = resolve_ref(test, "fixtures", "fixture_id")
    env = resolve_ref(test, "env", "env_prereq_id")
    run_ts = time.strftime("%Y%m%dT%H%M%S")

    # 骨架校验: FI 必须有 injection_point 定义
    inj = fixture.get("injection_point") if fixture else None
    passed = bool(inj and env and test.get("pass_rule"))
    evidence = {
        "test_id": test["id"],
        "run_ts": run_ts,
        "injection_point": inj,
        "expected_recovery": fixture.get("expected_recovery") if fixture else None,
        "result": "PASS" if passed else "FAIL",
    }
    art = P06 / "evidence" / f"{test['id']}_{run_ts}_{'pass' if passed else 'fail'}.json"
    art.write_text(json.dumps(evidence, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(evidence, indent=2, ensure_ascii=False))
    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
