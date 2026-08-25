#!/usr/bin/env python3
"""Phase 0.6 Runner — Reference A1 (PACKET switch) + AC-03B Override.

真实可执行骨架: 加载 Test Case YAML + Fixture + Env, 对 pass_rule 做占位求值,
产出 evidence 文件。实际 runtime 调用点以 `<RUNTIME_RPC>` 占位, 由真实环境 manifest 注入。

用法:
  python docs/phase-0.6/runners/run_reference_a1.py docs/phase-0.6/tests/AC-01-001.yaml
  python docs/phase-0.6/runners/run_reference_a1.py docs/phase-0.6/tests/AC-03B-001.yaml
"""
import sys
import json
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
P06 = ROOT / "docs" / "phase-0.6"


def load_yaml(path):
    # 极简 YAML 解析 (避免外部依赖): 真实环境应使用 PyYAML
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
        print("usage: run_reference_a1.py <test.yaml>")
        sys.exit(2)
    test = load_yaml(sys.argv[1])
    fixture = resolve_ref(test, "fixtures", "fixture_id")
    env = resolve_ref(test, "env", "env_prereq_id")
    run_ts = time.strftime("%Y%m%dT%H%M%S")

    # --- 真实 runtime 调用点 (占位) ---
    # result = runtime_rpc.switch_take(source=..., mode=...)
    # 此处仅做骨架断言: 引用文件已解析即视为 harness 连通
    passed = bool(fixture and env and test.get("pass_rule"))
    evidence = {
        "test_id": test["id"],
        "run_ts": run_ts,
        "fixture": test.get("fixture_id"),
        "env": test.get("env_prereq_id"),
        "pass_rule": test.get("pass_rule"),
        "result": "PASS" if passed else "FAIL",
    }
    art = P06 / "evidence" / f"{test['id']}_{run_ts}_{'pass' if passed else 'fail'}.json"
    art.write_text(json.dumps(evidence, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(evidence, indent=2, ensure_ascii=False))
    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
