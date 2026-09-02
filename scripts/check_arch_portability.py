#!/usr/bin/env python3
# scripts/check_arch_portability.py
#
# ARCH-PORTABILITY-01 词法门禁 (VENDOR_NEUTRALITY_RULES)。
# 定位 (p06-final-merge-hardening P0-5): 本脚本是 Architecture **Lint** (词法层防回渗);
# 结构层 **Proof** 见 check_remove_adapters.py (真实移除 adapters 后 cargo check)。
#
# 目的: 在 CI 中**自动化**守住 "domain / contracts / runtime(编排层) 不得出现厂商
# crate 直接引用" 的架构边界。与编译门禁 (cargo build --no-default-features
# --features simulation / mock, 见 media-agent.yml) 互补:
#   - 编译门禁证明 "删除 BMD/GStreamer Provider 后上层仍能编译";
#   - 本步骤在词法层防止厂商名称 (decklink / gstreamer / ffmpeg / srs / aja)
#     以 crate/子模块**路径引用** (token 后接 `::`) 重新渗入受保护层真实代码。
#
# 设计要点:
#   * 受保护层 = services/media-agent/src 下除 adapters/ 的所有 .rs
#     (adapters/ 是 vendor 实现层, 允许厂商引用)。
#   * 仅当厂商 token **后接 `::`** (真实路径引用, 如 `gstreamer::` / `decklink::`)
#     才判违规; 字段名 `gstreamer:` / 注释 / 字符串 / cfg 门控区均**不**误判。
#   * cfg 门控判定用**花括号深度** (非缩进): 多行函数签名续行在 indent 0 会令
#     缩进法误弹栈, 花括号法稳健。
#   * 必须在**保留字符串的原始行**上检测 `#[cfg(feature="gstreamer-backend")]`
#     (字符串剥离会破坏 feature 名 -> cfg 检测失效 -> 门控区误判违规)。
#   * 经收敛门面 `crate::adapters::{gstreamer,blackmagic}` 的访问允许。
#   * STRICT 模式 (ARCH_PORT_STRICT=1) 额外扫描注释/字符串中的厂商词 (whole-word),
#     用于人工严审, 不在 CI 默认启用。
#
# 退出码: 0 = 通过; 1 = 受保护层出现厂商路径引用。

import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "services" / "media-agent" / "src"

# 厂商 token (大小写不敏感)。仅当其后接 `::` (路径引用) 判违规。
VENDOR_TOKENS = ["decklink", "gstreamer", "ffmpeg", "srs", "aja"]

# 收敛门面前缀: 经此路径访问厂商实现属 AdapterRegistry 收口点设计内, 不判违规。
ALLOWED_PATH_PREFIXES = [
    "crate::adapters::gstreamer",
    "crate::adapters::blackmagic",
]

# 厂商 feature (用于 cfg 门控识别)。not(...) 不计 (那是 "非厂商路径", 应被扫描)。
VENDOR_FEATURES = {
    "gstreamer-backend", "bmd-provider", "gstreamer", "bmd",
    "decklink", "ffmpeg", "srs", "aja",
}

STRICT = os.environ.get("ARCH_PORT_STRICT") == "1"


def _strip_block_comment(raw, in_block):
    out = []
    i = 0
    n = len(raw)
    while i < n:
        if in_block:
            j = raw.find("*/", i)
            if j == -1:
                return "".join(out), True
            out.append(" ")
            i = j + 2
            in_block = False
        else:
            j = raw.find("/*", i)
            if j == -1:
                out.append(raw[i:])
                return "".join(out), False
            out.append(raw[i:j])
            i = j + 2
            in_block = True
    return "".join(out), in_block


def _strip_strings(line):
    # 原始字符串 r#"..."# / r"..."# (含多 # 闭合)
    line = re.sub(r'r#*"[^"]*"#*', '""', line)
    # 普通字符串 "..."
    line = re.sub(r'"[^"]*"', '""', line)
    # 字符字面量 '...' (如 '{')
    line = re.sub(r"'[^']*'", "''", line)
    return line


def _is_comment(line):
    return line.lstrip().startswith("//")


def _is_attribute(line):
    return line.lstrip().startswith("#")


def _has_vendor_feature(line):
    if "not(" in line:
        return False
    for m in re.finditer(r'feature\s*=\s*["\']([^"\']+)["\']', line):
        if m.group(1) in VENDOR_FEATURES:
            return True
    return False


def _scan(code, tokens, rel, lineno, errors, whole_word=False):
    lower = code.lower()
    for tok in tokens:
        if whole_word:
            pat = r"(?<![A-Za-z0-9_])" + re.escape(tok) + r"(?![A-Za-z0-9_])"
        else:
            pat = r"(?<![A-Za-z0-9_])" + re.escape(tok) + r"(?=::)"
        for m in re.finditer(pat, lower):
            errors.append((rel, lineno, m.start() + 1, tok))


def check_file(rel, path):
    errors = []
    try:
        with open(path, encoding="utf-8") as f:
            lines = f.readlines()
    except OSError:
        return errors

    in_block = False
    brace = 0
    gate_stack = []      # (start_brace, opened)
    pending_gate = False

    for lineno, raw in enumerate(lines, 1):
        code_raw, in_block = _strip_block_comment(raw, in_block)

        if _is_comment(code_raw):
            if STRICT:
                _scan(code_raw, VENDOR_TOKENS, rel, lineno, errors, whole_word=True)
            continue
        if _is_attribute(code_raw):
            if _has_vendor_feature(code_raw):
                pending_gate = True
            if STRICT:
                _scan(code_raw, VENDOR_TOKENS, rel, lineno, errors, whole_word=True)
            continue

        # 真实代码: 剥字符串字面量, 再去行尾 // 注释。
        code = _strip_strings(code_raw)
        code = code.split("//", 1)[0]
        if code.strip() == "":
            continue

        opens = code.count("{")
        closes = code.count("}")
        new_brace = brace + opens - closes

        if pending_gate:
            gate_stack.append((brace, False))
            pending_gate = False

        # 退出已闭合的门控区 (曾开启且当前深度 <= 起始深度)。
        while gate_stack and gate_stack[-1][1] and new_brace <= gate_stack[-1][0]:
            gate_stack.pop()

        gated = False
        if gate_stack:
            start, opened = gate_stack[-1]
            gated = (new_brace > start) if opened else (new_brace >= start)

        if gate_stack and opens > 0:
            gate_stack[-1] = (gate_stack[-1][0], True)

        brace = new_brace

        if gated:
            continue

        code_for_scan = code
        for prefix in ALLOWED_PATH_PREFIXES:
            code_for_scan = code_for_scan.replace(prefix, "CRATE_ADAPTERS_SURFACE")
        _scan(code_for_scan, VENDOR_TOKENS, rel, lineno, errors, whole_word=False)

    return errors


def main():
    if not SRC.is_dir():
        print(f"WARN: 未找到源码目录 {SRC}", file=sys.stderr)
        sys.exit(0)

    all_errors = []
    for p in sorted(SRC.rglob("*.rs")):
        rel = str(p.relative_to(SRC))
        if "adapters" in rel.split(os.sep):
            continue
        all_errors.extend(check_file(rel, p))

    # A20-03-BS-01 Single Bootstrap Source (用户 2026-09-02 裁定):
    # bin 入口（bin/gates.rs; bin 不得复制生产依赖构造——Gate 是 Consumer 不是
    # Bootstrapper）。禁止在 bin 文件出现构造调用; bootstrap.rs 与 lib 是唯一构造源。
    # main.rs（生产组合根）经 bootstrap::build() 消费, 自身也不得直接构造这些依赖
    # （自测 ctrl 构建除外——那是 runtime wiring 非 bootstrap 件, 见豁免表）。
    BS_FORBIDDEN = [
        "Config::from_env(",
        "AdapterRegistry::build_provider(",
        "FanoutSink::new(",
        "Supervisor::new(",
        "InMemoryLeaseManager::new(",
        "RuntimeEventLog::new(",
        ".discover()",
    ]
    BS_SCOPE = ["bin"] + os.sep.join(["bin"])  # bin/ 子树
    bs_errors = []
    bin_dir = SRC / "bin"
    if bin_dir.is_dir():
        for p in sorted(bin_dir.rglob("*.rs")):
            rel = str(p.relative_to(SRC))
            try:
                text = p.read_text(encoding="utf-8")
            except OSError:
                continue
            # 剥注释行（// 开头）与文档注释——只查真实代码
            for i, line in enumerate(text.splitlines(), 1):
                stripped = _strip_strings(line)
                if _is_comment(stripped):
                    continue
                for tok in BS_FORBIDDEN:
                    if tok in stripped:
                        bs_errors.append((rel, i, tok))

    if all_errors:
        print("FAIL — ARCH-PORTABILITY-01 门禁: "
              f"{len(all_errors)} 处受保护层出现厂商引用:")
        for rel, lineno, col, tok in sorted(all_errors):
            print(f"  {rel}:{lineno}:{col}: 受保护层出现厂商引用 '{tok}'")
        sys.exit(1)

    if bs_errors:
        print("FAIL — A20-03-BS-01 Single Bootstrap Source: "
              f"{len(bs_errors)} 处 bin 入口出现生产依赖构造调用 "
              "(必须经 bootstrap::build() 消费):")
        for rel, lineno, tok in sorted(bs_errors):
            print(f"  {rel}:{lineno}: bin 不得直接构造 '{tok}'")
        sys.exit(1)

    print("PASS — 受保护层 (domain/contracts/runtime) 无厂商 crate 直接引用 "
          "(ARCH-PORTABILITY-01 词法门禁) + "
          "A20-03-BS-01 Single Bootstrap Source (bin 零构造调用)")
    sys.exit(0)


if __name__ == "__main__":
    main()
