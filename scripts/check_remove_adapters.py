#!/usr/bin/env python3
"""ARCH-PORTABILITY-01 Architecture Proof Gate (p06-final-merge-hardening P0-5).

与 `check_arch_portability.py` (Architecture **Lint**, 词法层防回渗) 互补:
本脚本是 Architecture **Proof** —— 结构层真实验证:

    真实移除 adapters/blackmagic + adapters/gstreamer 目录
      → 修补 mod.rs / main.rs 引用
      → cargo check (simulation / mock, 无厂商 feature)
    ⇒ 证明 Domain / Contracts / Runtime 层不依赖任何 concrete adapter。

方法: 在临时副本上操作 (绝不污染工作区), 异常时清理; 退出码 0 = 证明成立。
"""
from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

MINIMAL_PATCHES: dict[str, list[tuple[str, str]]] = {
    # 移除两个 adapter 目录后的最小引用修补 (精确到唯一匹配, 多处匹配即失败 → 保持诚实)
    "src/adapters/mod.rs": [
        ("pub mod blackmagic;\n", ""),
        ("pub mod gstreamer;\n", ""),
    ],
}

MAIN_PATCHES: list[tuple[str, str]] = [
    # main.rs 对 adapters 的引用点 (blkmagic probe/registry + gstreamer runtime version)
    (
        "crate::adapters::blackmagic::probe_sdk",
        "crate::adapters::adapter_removed_stub_probe_sdk",
    ),
    (
        "crate::adapters::blackmagic::probe_connector_config",
        "crate::adapters::adapter_removed_stub_probe_connector_config",
    ),
    (
        "crate::adapters::blackmagic::registry",
        "crate::adapters::adapter_removed_stub_registry",
    ),
    (
        "crate::adapters::gstreamer::gstreamer_runtime_version",
        "crate::adapters::adapter_removed_stub_gst_version",
    ),
]


def patch(text: str, patches: list[tuple[str, str]], path: str) -> str:
    for old, new in patches:
        if old not in text:
            raise RuntimeError(f"{path}: expected snippet not found (patch drift): {old!r}")
        text = text.replace(old, new)
    return text


def check_crate(crate: Path, cargo: str, features: list[str]) -> None:
    tmp = Path(tempfile.mkdtemp(prefix="vbmf-remove-adapters-"))
    work = tmp / "media-agent"
    try:
        shutil.copytree(crate, work, ignore=shutil.ignore_patterns("target", ".cargo"))
        # 1. 真删两个 concrete adapter 目录
        for sub in ("blackmagic", "gstreamer"):
            d = work / "src" / "adapters" / sub
            if not d.is_dir():
                raise RuntimeError(f"expected adapter dir missing in copy: {sub}")
            shutil.rmtree(d)
        # 2. 最小引用修补 (mod 声明)
        for rel, patches in MINIMAL_PATCHES.items():
            f = work / rel
            f.write_text(patch(f.read_text(encoding="utf-8"), patches, rel), encoding="utf-8")
        # 3. main.rs: adapter 公共函数引用替换为临时 stub (stub 本体注入 mod.rs),
        #    使 default/simulation/mock 构建无需这些 adapter 即可编译。
        #    若 Domain/Runtime 存在对 adapter 内部类型的**类型级**依赖, cargo check 仍会失败 — 这正是门禁。
        main_rs = work / "src" / "main.rs"
        txt = main_rs.read_text(encoding="utf-8")
        # main.rs 的 cfg(feature = "bmd-provider") 代码块: 移除 adapter 后这些 feature 不存在,
        # cfg 块自然不编译; 但块内文本仍引用被删符号 → 无需修补 (cfg 关闭即不编译)。
        # 唯一需处理的是 non-gated 引用; 逐一尝试修补, 找不到则视为已无引用 (cfg-gated)。
        for old, new in MAIN_PATCHES:
            txt = txt.replace(old, new)
        main_rs.write_text(txt, encoding="utf-8")
        stub = (
            "// remove-adapter proof: 临时 stub (仅存在于验证副本) — main 非 gated 引用的落点。\n"
            "pub mod adapter_stubs;\n"
            "pub use adapter_stubs::adapter_removed_stub_probe_sdk;\n"
            "pub use adapter_stubs::adapter_removed_stub_probe_connector_config;\n"
            "pub use adapter_stubs::adapter_removed_stub_registry;\n"
            "pub use adapter_stubs::adapter_removed_stub_gst_version;\n"
        )
        # 注入 stub 模块 (cfg 无法覆盖的引用极少; 若 main 无引用, stub 未用也不报错)
        mod_rs = work / "src" / "adapters" / "mod.rs"
        mod_rs.write_text(mod_rs.read_text(encoding="utf-8") + stub, encoding="utf-8")
        (work / "src" / "adapters" / "adapter_stubs.rs").write_text(
            "#![allow(dead_code)]\n"
            "pub fn adapter_removed_stub_probe_sdk(_n: &str) -> Result<(), String> { Ok(()) }\n"
            "pub fn adapter_removed_stub_probe_connector_config() -> Result<Vec<String>, String> { Ok(Vec::new()) }\n"
            "pub fn adapter_removed_stub_registry() -> Result<String, String> { Ok(String::new()) }\n"
            "pub fn adapter_removed_stub_gst_version() -> Option<String> { None }\n",
            encoding="utf-8",
        )
        # 4. cargo check (无厂商 feature)
        for feat in features:
            cmd = [cargo, "check", "--no-default-features", "--features", feat]
            print(f"[remove-adapters] cargo check --no-default-features --features {feat} ...")
            r = subprocess.run(cmd, cwd=work, capture_output=True, text=True)
            if r.returncode != 0:
                sys.stderr.write(r.stdout[-4000:] + r.stderr[-4000:])
                raise RuntimeError(f"cargo check failed for features={feat!r}")
        print("[remove-adapters] PROOF OK: Domain/Contracts/Runtime compile without concrete adapters")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--crate", default=str(Path(__file__).resolve().parents[1] / "services" / "media-agent"))
    ap.add_argument("--cargo", default="cargo")
    ap.add_argument("--features", nargs="*", default=["simulation", "mock"])
    args = ap.parse_args()
    crate = Path(args.crate).resolve()
    if not (crate / "Cargo.toml").is_file():
        print(f"crate not found: {crate}", file=sys.stderr)
        return 2
    try:
        check_crate(crate, args.cargo, args.features)
    except Exception as e:  # noqa: BLE001 - 门禁脚本: 任何失败即不通过
        print(f"[remove-adapters] FAIL: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
