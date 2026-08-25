#!/usr/bin/env python3
"""VBMF 文档一致性校验（Phase 0.5C.1 引入 · 防"只追加、不回写"再发）

检查项:
  1. Markdown 相对链接目标存在（本地运行: python scripts/check_docs.py）
  2. docs/ 下 HTML wireframe 的 href 目标存在
  3. 关键数字口径（表面计数 / 收口计数 / 引擎与横向系统名单）

用法:
  python scripts/check_docs.py          # 全部检查
  python scripts/check_docs.py links    # 只查链接

退出码: 0 = 通过, 1 = 有错误
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKIP_DIRS = {".git", ".zcode", ".codebuddy", "node_modules", ".history"}
# GitHub 站内约定路径（本仓库Issues/Discussions/Security），本地不存在属正常
GITHUB_ONLY = ("../../issues", "../../discussions", "../../pulls", "../../security", "../../wiki")


def iter_files(pattern):
    for p in ROOT.rglob(pattern):
        if SKIP_DIRS & set(p.parts):
            continue
        yield p


def check_md_links():
    errs = []
    for md in iter_files("*.md"):
        text = md.read_text(encoding="utf-8")
        for m in re.finditer(r"\]\(([^)\s]+)\)", text):
            target = m.group(1)
            if re.match(r"^[a-zA-Z][a-zA-Z0-9+.-]*://", target) or target.startswith("mailto:"):
                continue
            if target.startswith(GITHUB_ONLY):
                continue
            path = target.split("#")[0]
            if not path:
                continue
            if not (md.parent / path).resolve().exists():
                errs.append(f"[MD-LINK] {md.relative_to(ROOT)} -> {target}")
    return errs


def check_html_links():
    errs = []
    for html in (ROOT / "docs").rglob("*.html"):
        if SKIP_DIRS & set(html.parts):
            continue
        text = html.read_text(encoding="utf-8")
        for m in re.finditer(r'href="([^"]+)"', text):
            target = m.group(1)
            if target.startswith(("http://", "https://", "mailto:", "javascript:")):
                continue
            # 剥离 query string (?ch=CH02) 和 anchor (#section)
            target_no_qs = target.split("?", 1)[0].split("#", 1)[0]
            if not target_no_qs:
                continue
            if not (html.parent / target_no_qs).resolve().exists():
                errs.append(f"[HTML-LINK] {html.relative_to(ROOT)} -> {target}")
        return errs


def load_sot():
    """从 SURFACE_REGISTRY.yaml 解析 Surface Count SoT（唯一事实源）"""
    reg = (ROOT / "docs" / "phase-0.5" / "SURFACE_REGISTRY.yaml")
    total = wf = None
    if reg.exists():
        t = reg.read_text(encoding="utf-8")
        m = re.search(r"TOTAL\s*(\d+)", t)
        if m:
            total = m.group(1)
        m2 = re.search(r"TOTAL\s*\d+\s*\(\s*(\d+)\s+wireframe", t)
        if m2:
            wf = m2.group(1)
    return total, wf


def check_numbers():
    errs = []
    total, wf = load_sot()
    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    expectations = [
        (f"README 含 SoT 总数 {total}", bool(total and total in readme)),
        (f"README 含 SoT wireframe 数 {wf}", bool(wf and f"{wf} wireframe" in readme)),
        ("收口计数 36（31 P0 + 5 P1）", "36 项语义收口（31 P0 + 5 P1）" in readme),
        ("README 引擎名单含 Redundancy", "Redundancy" in readme),
        ("README 引擎名单含 Signal Fabric", "Signal Fabric" in readme),
        ("README 横向系统为 H1-H5", "H1 Safety" in readme and "H5 Subtitle" in readme),
    ]
    for name, ok in expectations:
        if not ok:
            errs.append(f"[NUMBER] README.md 口径缺失或被改回: {name}")

    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    for engine in ("Signal Fabric", "Normalize", "Redundancy", "QC"):
        if engine not in changelog:
            errs.append(f"[NUMBER] CHANGELOG.md 引擎名单缺: {engine}")

    spec = ROOT / "docs" / "phase-0.5" / "SURFACE_SPEC.md"
    if spec.exists():
        text = spec.read_text(encoding="utf-8")
        if total and f"**{total}**" not in text:
            errs.append(f"[NUMBER] SURFACE_SPEC §1 当前锁定总计 {total} 口径缺失（SoT: SURFACE_REGISTRY.yaml）")
        if "# 30. 附录：Phase 0.5B 语义收口项总清单" not in text:
            errs.append("[NUMBER] SURFACE_SPEC §30 收口项附录缺失")
    else:
        errs.append("[NUMBER] docs/phase-0.5/SURFACE_SPEC.md 不存在（目录又动了？同步本脚本）")
    return errs


def check_nav_domain_counts():
    """NAVIGATION 的 ENGINEERING 域表面数必须与 SURFACE_REGISTRY.yaml SoT 一致
    （数字由 Registry 派生, 不得手写, 防止 26/29 这类漂移）。"""
    errs = []
    reg = (ROOT / "docs" / "phase-0.5" / "SURFACE_REGISTRY.yaml")
    if not reg.exists():
        return errs
    t = reg.read_text(encoding="utf-8")
    m = re.search(r"ENGINEERING\s+(\d+)\s*\(", t)
    if not m:
        return errs
    eng_sot = m.group(1)
    nav = (ROOT / "docs" / "phase-0.5" / "NAVIGATION.md").read_text(encoding="utf-8")
    m2 = re.search(r"ENGINEERING 域\s*\((\d+) 表面", nav)
    if m2 and m2.group(1) != eng_sot:
        errs.append(
            f"[NUMBER] NAVIGATION ENGINEERING 域表面数 {m2.group(1)} "
            f"与 SURFACE_REGISTRY SoT {eng_sot} 不一致（数字必须由 Registry 派生, 不得手写）"
        )
    return errs


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "all"
    errors = []
    if mode in ("all", "links"):
        errors += check_md_links()
        errors += check_html_links()
    if mode in ("all", "numbers"):
        errors += check_numbers()
        errors += check_nav_domain_counts()

    if errors:
        print(f"FAIL — {len(errors)} 个问题:")
        for e in errors:
            print("  " + e)
        sys.exit(1)
    print("PASS — 链接可达 + 数字口径一致")


if __name__ == "__main__":
    main()
