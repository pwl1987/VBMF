#!/usr/bin/env python3
"""Control-plane boundary gate (CONTROL-PLANE-ENTRY-01 / CP-01A).

Machine-checked red lines that back the frozen planning matrix:

  F12 (Gate A lexical, TECHNOLOGY_STACK F9 + Deployment SoT):
      apps/api (Fastify Control Plane) must never import/require
      child_process, call spawn/exec-style process launchers, or reference
      ffmpeg / gst-launch / DeckLink / /dev/blackmagic. Media process
      lifecycle belongs to the Rust media-agent alone; the control plane
      only commands + observes.

  F11 (deployment wiring, RCE-D3 R-a..R-f):
      - no compose file (BASE or any overlay) publishes host port 50051;
      - BASE compose wires media-agent with MEDIA_AGENT_RPC_BIND on the
        container-private network and exposes 50051 alongside the 8080
        health listener;
      - the fastify service points at the internal JSON-RPC control plane
        (media-agent:50051), never at the 8080 health listener;
      - Nginx never routes /internal and never references 50051.

Exit codes: 0 = pass, 1 = violations found. Output is plain text with
file:line anchors, safe for CI annotations.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

COMPOSE_FILES = [
    "ops/docker-compose.yml",
    "ops/compose.dev.yml",
    "ops/compose.acceptance.yml",
    "ops/compose.prod.yml",
]
BASE_COMPOSE = "ops/docker-compose.yml"
NGINX_DIR = "ops/nginx"
CONTROL_PLANE_DIR = "apps/api"

# F12: lexical red lines for the Fastify control plane.
FORBIDDEN_SUBSTRINGS = [
    "child_process",
    "ffmpeg",
    "gst-launch",
    "/dev/blackmagic",
    "DeckLink",
]
# F12: lexical red lines for the Fastify control plane. `(?<![.\w])` keeps
# legitimate method calls (e.g. RegExp.prototype.exec) out of scope.
FORBIDDEN_CALLS = re.compile(r"(?<![.\w])(spawn|spawnSync|execSync|execFile|exec)\s*\(")

SOURCE_SUFFIXES = {".ts", ".js", ".mjs", ".cjs"}


def strip_js_comments(text: str) -> str:
    """Remove // line comments and /* */ blocks (heuristic, good enough
    for a lexical gate that must not flag documentation-only mentions)."""
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)


def gate_f12(violations: list[str]) -> None:
    root = REPO / CONTROL_PLANE_DIR
    if not root.is_dir():
        violations.append(f"F12: {CONTROL_PLANE_DIR}/ does not exist")
        return
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.suffix not in SOURCE_SUFFIXES:
            continue
        rel = path.relative_to(REPO)
        if "node_modules" in path.parts or "dist" in path.parts:
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for needle in FORBIDDEN_SUBSTRINGS:
            if needle in text:
                violations.append(f"F12: {rel}: forbidden token {needle!r}")
        code = strip_js_comments(text)
        m = FORBIDDEN_CALLS.search(code)
        if m:
            violations.append(f"F12: {rel}: forbidden process-launch call {m.group(1)}(")


def iter_ports_entries(text: str, rel: str, violations: list[str]):
    """Yield (lineno, port_entry) for every compose short/long ports item."""
    in_ports = False
    for i, line in enumerate(text.splitlines(), start=1):
        stripped = line.strip()
        if re.match(r"^ports:\s*$", stripped):
            in_ports = True
            continue
        if in_ports:
            if stripped.startswith("-"):
                entry = stripped.lstrip("- ").strip().strip('"').strip("'")
                yield i, entry
            elif stripped and not stripped.startswith("#"):
                in_ports = False


def gate_f11_compose(violations: list[str]) -> None:
    for name in COMPOSE_FILES:
        path = REPO / name
        if not path.is_file():
            violations.append(f"F11: {name}: expected compose file missing")
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for lineno, entry in iter_ports_entries(text, name, violations):
            if "50051" in entry:
                violations.append(
                    f"F11: {name}:{lineno}: host publish of 50051 is forbidden ({entry})"
                )

    base = REPO / BASE_COMPOSE
    text = base.read_text(encoding="utf-8", errors="replace")
    if "MEDIA_AGENT_RPC_BIND: 0.0.0.0:50051" not in text:
        violations.append(
            "F11: ops/docker-compose.yml: media-agent must set "
            "MEDIA_AGENT_RPC_BIND: 0.0.0.0:50051 (container-private network)"
        )
    if "MEDIA_AGENT_RPC_URL: http://media-agent:50051" not in text:
        violations.append(
            "F11: ops/docker-compose.yml: fastify must set "
            "MEDIA_AGENT_RPC_URL: http://media-agent:50051"
        )
    if "MEDIA_AGENT_RPC:" in text:
        violations.append(
            "F11: ops/docker-compose.yml: stale MEDIA_AGENT_RPC placeholder "
            "(8080 health listener was never the control plane)"
        )
    m = re.search(r"MEDIA_AGENT_RPC_URL:\s*(\S+)", text)
    if m and ":8080" in m.group(1):
        violations.append(
            "F11: ops/docker-compose.yml: MEDIA_AGENT_RPC_URL must not target "
            "the 8080 health listener"
        )


def gate_f11_nginx(violations: list[str]) -> None:
    root = REPO / NGINX_DIR
    if not root.is_dir():
        violations.append(f"F11: {NGINX_DIR}/ does not exist")
        return
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(REPO)
        text = path.read_text(encoding="utf-8", errors="replace")
        for i, line in enumerate(text.splitlines(), start=1):
            if re.search(r"location[^{]*\b/internal", line):
                violations.append(f"F11: {rel}:{i}: Nginx must never route /internal")
            if "50051" in line:
                violations.append(f"F11: {rel}:{i}: Nginx must never reference 50051")


def main() -> int:
    violations: list[str] = []
    gate_f12(violations)
    gate_f11_compose(violations)
    gate_f11_nginx(violations)
    if violations:
        print("control-plane gate: FAIL")
        for v in violations:
            print(f"  - {v}")
        return 1
    print("control-plane gate: PASS (F11 deployment wiring + F12 Gate A lexical)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
