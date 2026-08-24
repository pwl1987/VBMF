# Contributing to VBMF

Thank you for your interest in contributing to VBMF (IP Broadcast Media Fabric)!

> **🔴 V0.2 Architecture Baseline is LOCK FINAL.** This is the most important rule to understand before contributing.

## ⚠️ Architecture freeze notice

V0.2 has completed 22 review rounds and is **LOCK FINAL**:

- ✅ 12 Engines + 5 横向系统 + 6 横切能力 + 22 原则 + 57 决策
- ✅ V0.2 Runtime Semantics = CLOSED
- ✅ implementation_ambiguity: NONE
- ❌ **No more V0.2.x architecture reviews** (V0.2.5 is FORBIDDEN)
- ❌ **No architecture-level changes** without V0.3 process

### What this means for contributions

| Type | Status | Process |
|---|---|---|
| **Bug fix** in existing Engines | ✅ Welcome | Open PR → review → merge |
| **Doc typo / wireframe fix** | ✅ Welcome | Open PR |
| **Phase 0.6 Reference Implementation** | 📋 Future | Will be coordinated in separate issues |
| **Phase 1 Media Agent (Rust)** | 📋 Future | After Phase 0.6 acceptance |
| **New Engine (e.g. NDI/RIST/Zixi)** | ❌ Architecture change | Must go through V0.3 process |
| **Adding a 13th Engine** | ❌ FORBIDDEN | V0.2 already LOCK FINAL |
| **Modifying Switch Mode 3 types** | ❌ FORBIDDEN | Core definition |
| **Modifying Data Plane 4 layers** | ❌ FORBIDDEN | Core definition |

## 🚀 How to contribute

### 1. Reporting bugs

Before reporting, please check:
- [architecture/ARCHITECTURE_V0.2.md](docs/architecture/ARCHITECTURE_V0.2.md) — the bug might be a known limitation
- [existing issues](../../issues) — might already be tracked

If new, open an issue with:
- **What** — clear description
- **Where** — which Engine / Chapter / Wireframe
- **Expected vs Actual** — what you expected vs what you saw
- **Reproduction** — minimal steps
- **Environment** — V0.2 chapter version, OS, hardware

### 2. Submitting PRs

#### PR title convention

```
[scope] short description
```

Examples:
- `[docs] fix typo in §3.9`
- `[wireframe] 09-health-tree: add 7 Health Invariants legend`
- `[chain] chain-2-failure: add FI-04 Clock Drift scenario`
- `[bug] channel_health_aggregation: fix Rule 4 UNKNOWN absorption` (V0.2.4 Errata-14 issue)

#### PR body must include

- **What** — what changed
- **Why** — what problem it solves
- **How** — approach
- **Tests** — how you verified
- **Architecture impact** — None / Within V0.2 / Requires V0.3 (must be None for current phase)

#### Review process

1. Open PR with clear description
2. Maintainer reviews for V0.2 alignment
3. CI passes (linting, doc build)
4. Approve → merge

### 3. Doc conventions

- **Style**: V0.2 architecture is mostly in Chinese. Phase 0.5 wireframe labels in English (broadcaster international).
- **Canonical Vocabulary**: TS / Rust / JSON Schema / PG enum 共享的术语 **必须** 严格使用（见 [docs/architecture/ARCHITECTURE_V0.2.md Appendix D](docs/architecture/ARCHITECTURE_V0.2.md)）
- **YAML in code blocks**: use ```yaml fenced code blocks
- **References**: V0.2 §X.Y / §A.B / Decision #NN 引用规范

### 4. Code style (when code lands)

| Language | Style | Linter |
|---|---|---|
| TypeScript | ESLint + Prettier | `pnpm lint` |
| Rust | rustfmt + clippy | `cargo fmt && cargo clippy` |
| SQL | sqlfluff | `sqlfluff lint` |
| YAML | yamllint | `yamllint .` |
| Markdown | markdownlint | `markdownlint '**/*.md'` |

## 📋 Current contribution priorities

| Priority | Area | Note |
|---|---|---|
| 🔥 High | Phase 0.5 wireframe polish | Open `docs/phase-0.5/wireframes/*.html` and improve |
| 🔥 High | Phase 0.6 Reference A1 | PACKET_SWITCH 基础能力，需要实际部署验证 |
| 🔥 High | Phase 0.6 Reference A2 | SDI 主备走 FRAME/MASTER |
| 🟡 Med | Phase 0.6 5 Fault Injection | SDI 冻结 / 音频静音 / FFmpeg 崩溃 / Clock Drift / HLS 切片失败 |
| 🟡 Med | Phase 0.6 7 Health Invariants tests | 转为 executable test cases |
| 🟢 Low | Translation (中文 ↔ English) | Doc + wireframe labels |
| 🟢 Low | Diagram in docs/assets/ | Architecture diagrams |

## 🧪 Test policy

- **Architecture docs**: any change to the 12 Engines / 5 横向系统 / 6 横切能力 / 22 原则 / 57 决策 requires V0.3 process
- **Phase 0.5 wireframes**: visual review + click-through (9 pages interlinked)
- **Phase 0.6 References**: 24h stability + all 5 Fault Injection pass
- **Phase 1 Media Agent**: 24h SDI → HLS pipeline stability

## 📞 Contact

- **GitHub Issues**: [github.com/\<org\>/VBMF/issues](../../issues)
- **Discussions**: [github.com/\<org\>/VBMF/discussions](../../discussions)
- **Security**: see [SECURITY.md](SECURITY.md) for private disclosure

## 📜 Code of Conduct

Be respectful. We are all here to build a great broadcast platform. See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) (if present).

## 🙏 Thank you

Every PR, every issue, every review comment helps VBMF become a reliable open-source platform for broadcast professionals.

---

**VBMF Contributors** · V0.2 LOCK FINAL · Apache 2.0
