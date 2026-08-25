#!/usr/bin/env python3
"""Phase 0.6 shared Harness evaluation logic.

关键语义 (回应 GDOC-04/05/06):
  - Runner 不再 "文件存在即 PASS"。
  - 引入 evaluate_pass_rule(): 对 test.expected 的每个断言做结构化求值。
  - result 区分三态:
      HARNESS_READY  : 骨架连通 (fixture/env/pass_rule 解析成功), 但 measured 来自占位,
                       未经过真实 runtime 测量 -> 不构成 Acceptance PASS。
      PASS           : 真实 measured 经 pass_rule 求值全部为真。
      FAIL           : pass_rule 求值存在假值, 或骨架连通失败。
  - 当前阶段 measured=None (无真实 runtime 调用点), 只能产出 HARNESS_READY,
    严禁输出 PASS, 防止验收报告污染。
"""
import re


def load_yaml(path):
    try:
        import yaml
        return yaml.safe_load(path.read_text(encoding="utf-8"))
    except ImportError:
        raise SystemExit("PyYAML required in real env: pip install pyyaml")


def evaluate_pass_rule(test, measured):
    """对 expected 中每个 key 求值。

    measured: dict | None
      - None  => 骨架占位阶段, 无法取得真实测量值
      - dict  => 真实 runtime 返回值 (由 <RUNTIME_RPC> 注入), 形如 {key: bool/value}
    返回 (passed_bool_or_None, detail_dict)
      - None 表示无法判定 (HARNESS_READY)
    """
    expected = test.get("expected") or {}
    pass_rule = test.get("pass_rule") or ""
    if not expected:
        return None, {}
    if measured is None:
        # 占位阶段: 不臆造测量值, 返回不可判定
        return None, {k: "UNMEASURED" for k in expected}
    detail = {}
    all_ok = True
    for k, want in expected.items():
        got = measured.get(k)
        ok = (got == want)
        detail[k] = {"want": want, "got": got, "ok": ok}
        if not ok:
            all_ok = False
    # pass_rule 字符串作为辅助断言 (真实环境可进一步解析, 此处仅记录)
    return all_ok, {"_pass_rule": pass_rule, "_detail": detail}


def classify(test, fixture, env, measured):
    """产出三态 result + evidence。"""
    if not (fixture and env and test.get("pass_rule")):
        return "FAIL", {"reason": "skeleton incomplete (fixture/env/pass_rule missing)"}
    passed, detail = evaluate_pass_rule(test, measured)
    if passed is None:
        return "HARNESS_READY", detail
    return ("PASS" if passed else "FAIL"), detail
