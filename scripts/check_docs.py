#!/usr/bin/env python3
"""VBMF 文档一致性校验（Phase 0.5C.1 引入 · 防"只追加、不回写"再发）

检查项:
  1. Markdown 相对链接目标存在（本地运行: python scripts/check_docs.py）
  2. docs/ 下 HTML wireframe 的 href 目标存在（扫描全部, 不再只查第一个）
  3. Markdown / HTML 的锚点 (#section) 目标存在（防死锚点, 如 #final-state）
  4. 关键数字口径（表面计数 / 收口计数 / 引擎与横向系统名单）
  5. Phase 0.6 FI 集合一致性（canonical FI ID 集合 == scope/schedule/outputs 引用数, 防 5→7 类语义漂移）

用法:
  python scripts/check_docs.py          # 全部检查
  python scripts/check_docs.py links    # 只查链接 + 锚点
  python scripts/check_docs.py numbers  # 只查数字口径
  python scripts/check_docs.py phase06  # 只查 Phase 0.6 FI 集合一致性

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


def _slugify(heading):
    """GitHub 风格 heading slug: 小写, 去行内格式, 空格→连字符, 去首尾连字符。"""
    s = heading.strip().lower()
    s = s.replace("`", "")
    s = re.sub(r"[^\w\s-]", "", s, flags=re.UNICODE)  # 保留字母/数字/下划线/空格/连字符(含 CJK)
    s = s.replace(" ", "-")
    s = s.strip("-")
    return s


def _norm(a):
    """宽松归一: 去大小写/标点/空白, 保留字母数字与 CJK。用于锚点近似匹配,
    容忍 `§` / `.` / `-` / 全角等作者约定差异 (如 `#§-3-...signal-graph-v0-2-final` ↔ 标题 `§3 ... V0.2`)。"""
    return re.sub(r"[^a-z0-9\u4e00-\u9fff]", "", a.lower())


def _md_anchors(text):
    """收集一个 md 文件内所有可用锚点: 显式 id/name + 标题 slug。"""
    anchors = set()
    for m in re.finditer(r'\bid="([^"]+)"', text):
        anchors.add(m.group(1))
    for m in re.finditer(r'\bname="([^"]+)"', text):
        anchors.add(m.group(1))
    for m in re.finditer(r"^#{1,6}\s+(.+?)\s*#*\s*$", text, re.MULTILINE):
        anchors.add(_slugify(m.group(1)))
    return anchors


def _html_anchors(text):
    """收集一个 html 文件内所有 id / name 锚点。"""
    anchors = set()
    for m in re.finditer(r'\bid="([^"]+)"', text):
        anchors.add(m.group(1))
    for m in re.finditer(r'\bname="([^"]+)"', text):
        anchors.add(m.group(1))
    return anchors


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
            # 同页锚点 (#section): 校验当前文件自身锚点
            if target.startswith("#"):
                anchor = target[1:]
                if anchor:
                    anc = _md_anchors(text)
                    anc_norm = {_norm(a) for a in anc}
                    if anchor not in anc and _slugify(anchor) not in anc and _norm(anchor) not in anc_norm:
                        errs.append(f"[MD-ANCHOR] {md.relative_to(ROOT)} -> {target} (同页锚点不存在)")
                continue
            path, _, anchor = target.partition("#")
            if not path:
                continue
            resolved = (md.parent / path).resolve()
            if not resolved.exists():
                errs.append(f"[MD-LINK] {md.relative_to(ROOT)} -> {target}")
                continue
            if anchor:
                anc = _md_anchors(resolved.read_text(encoding="utf-8"))
                anc_norm = {_norm(a) for a in anc}
                if anchor not in anc and _slugify(anchor) not in anc and _norm(anchor) not in anc_norm:
                    errs.append(f"[MD-ANCHOR] {md.relative_to(ROOT)} -> {target} (锚点 #{anchor} 不存在)")
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
        # 跨文件锚点校验 (#section 且带路径)
        for m in re.finditer(r'href="([^"]*#[^"]+)"', text):
            target = m.group(1)
            if target.startswith(("#", "http://", "https://", "mailto:", "javascript:")):
                continue  # 同页 / 外部锚点不在此校验
            path, _, anchor = target.partition("#")
            if not path:
                continue
            resolved = (html.parent / path.split("?", 1)[0]).resolve()
            if resolved.exists() and anchor:
                anc = _html_anchors(resolved.read_text(encoding="utf-8"))
                anc_norm = {_norm(a) for a in anc}
                if anchor not in anc and _slugify(anchor) not in anc and _norm(anchor) not in anc_norm:
                    errs.append(f"[HTML-ANCHOR] {html.relative_to(ROOT)} -> {target} (锚点 #{anchor} 不存在)")
    return errs  # BUG FIX: 原实现此处 return 误缩进在 for 循环内, 只查了首个 HTML 即返回


def load_sot():
    """从 SURFACE_REGISTRY.yaml 解析 Surface Count SoT（唯一事实源）"""
    reg = (ROOT / "docs" / "phase-0.5" / "SURFACE_REGISTRY.yaml")
    total = wf = None
    if reg.exists():
        t = reg.read_text(encoding="utf-8")
        m = re.search(r"TOTAL\s*(\d+)", t)
        if m:
            total = m.group(1)
        m2 = re.search(r"TOTAL\s*\d+\s*=\s*(\d+)\s*LOCK", t)
        if m2:
            wf = m2.group(1)
    return total, wf


def check_numbers():
    errs = []
    total, wf = load_sot()
    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    expectations = [
        (f"README 含 SoT 总数 {total}", bool(total and total in readme)),
        (f"README 含 SoT LOCK 数 {wf}", bool(wf and f"{wf} LOCK" in readme)),
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
    """NAVIGATION 每个域的表面数必须与 SURFACE_REGISTRY.yaml SoT 一致
    （数字由 Registry 派生, 不得手写, 防止 BROADCAST / MEDIA / ADMIN / GLOBAL 漂移）。

    - BROADCAST / MEDIA / ENGINEERING / ADMIN: NAVIGATION 有 "DOMAIN 域 (N 表面)" 头,
      与 Registry "DOMAIN (N)" 段头 SoT 比对。
    - GLOBAL: NAVIGATION 无独立"域 (N 表面)"头, 改为校验 Registry 内 GLOBAL 条目数 == SoT。
    """
    errs = []
    reg = (ROOT / "docs" / "phase-0.5" / "SURFACE_REGISTRY.yaml")
    if not reg.exists():
        return errs
    reg_text = reg.read_text(encoding="utf-8")
    nav = (ROOT / "docs" / "phase-0.5" / "NAVIGATION.md").read_text(encoding="utf-8")

    nav_domains = ["BROADCAST", "MEDIA", "ENGINEERING", "ADMIN"]
    for d in nav_domains:
        m = re.search(rf"{d}\s*\((\d+)", reg_text)
        if not m:
            continue
        sot = m.group(1)
        m2 = re.search(rf"{d} 域\s*\((\d+) 表面", nav)
        if m2 and m2.group(1) != sot:
            errs.append(
                f"[NUMBER] NAVIGATION {d} 域表面数 {m2.group(1)} "
                f"与 SURFACE_REGISTRY SoT {sot} 不一致（数字必须由 Registry 派生, 不得手写）"
            )

    # GLOBAL: NAVIGATION 无"域 (N 表面)"头 → 校验 Registry 内 GLOBAL 条目数 == SoT
    mg = re.search(r"GLOBAL\s*\((\d+)", reg_text)
    if mg:
        g_sot = int(mg.group(1))
        g_entries = len(re.findall(r"^\s*domain:\s*GLOBAL\s*$", reg_text, flags=re.MULTILINE))
        if g_entries != g_sot:
            errs.append(
                f"[NUMBER] SURFACE_REGISTRY GLOBAL 条目数 {g_entries} "
                f"与 GLOBAL SoT {g_sot} 不一致"
            )
    return errs


def check_phase06_fi_ids():
    """Phase 0.6 Fault Injection 集合一致性护栏 (0.6 启动前 Doc Patch 引入)。

    目标: 防止 "规范新增 FI-06/FI-07, 但 scope/schedule/outputs 仍写 5 FI" 的语义漂移
    (即用户指出的 '5 Fault Injection' vs 实际 FI-01A/B/02~07 = 7 个)。

    规则:
      1. 从 phase-0.6/README.md 全文中抽取所有 canonical FI ID (形如 FI-01A / FI-02 ...)。
      2. 期望 FI 数 = canonical 集合大小。
      3. 该文件内不得出现与期望数不一致的 "N Fault Injection" / "N FI" 字面量。
      4. scope / schedule / outputs 中每个被引用的 FI ID 必须属于 canonical 集合
         (反向: canonical 中每个 ID 至少应在某处被引用, 防止声明了却没接入)。
    """
    errs = []
    p06 = ROOT / "docs" / "phase-0.6" / "README.md"
    if not p06.exists():
        return errs
    text = p06.read_text(encoding="utf-8")

    fi_ids = re.findall(r"FI-\d{2}[A-Z]?", text)
    canonical = sorted(set(fi_ids))
    if not canonical:
        return errs
    expected = len(canonical)

    # 规则 3: 文件内 "N Fault Injection" / "N FI" 必须与 expected 一致
    for m in re.finditer(r"(\d+)\s*(?:Fault\s*Injection|FI)\b", text, flags=re.IGNORECASE):
        n = int(m.group(1))
        if n != expected:
            errs.append(
                f"[FI-CONSISTENCY] phase-0.6/README.md 出现 '{m.group(0)}' "
                f"但 canonical FI 集合大小为 {expected} ({', '.join(canonical)})"
            )

    # 规则 4a: 每个 canonical FI 至少被引用一次 (已在 fi_ids 中, 必然成立, 这里仅占位)
    # 规则 4b: 反向 — 其它 0.6 摘要文档若写 "N FI" 须等于 expected
    for md in iter_files("*.md"):
        if "phase-0.6" not in md.parts and md.name not in ("README.md", "ROADMAP.md", "CONTRIBUTING.md", "INDEX.md"):
            continue
        t = md.read_text(encoding="utf-8")
        for m in re.finditer(r"(\d+)\s*Fault\s*Injection", t, flags=re.IGNORECASE):
            n = int(m.group(1))
            if n != expected:
                errs.append(
                    f"[FI-CONSISTENCY] {md.relative_to(ROOT)} 出现 '{m.group(0)}' "
                    f"但 phase-0.6 canonical FI 集合大小为 {expected}"
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
    if mode in ("all", "phase06"):
        errors += check_phase06_fi_ids()

    if errors:
        print(f"FAIL — {len(errors)} 个问题:")
        for e in errors:
            print("  " + e)
        sys.exit(1)
    print("PASS — 链接可达 + 锚点有效 + 数字口径一致")


if __name__ == "__main__":
    main()
