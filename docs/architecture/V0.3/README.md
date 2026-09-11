# VBMF V0.3 Architecture Design Package

> **Status: BASELINE / IMPLEMENTATION SOURCE OF TRUTH**  
> Version: V0.3  
> Product: **Professional Real-Time Media Playout System / 专业实时媒体播出系统 / Real-Time Media Runtime & Fabric**

## 1. Purpose

`docs/architecture/V0.3/` is the complete, reviewable and independently maintainable architecture design package for VBMF V0.3.

V0.3 is a product/runtime evolution of the V0.2 baseline, not a second unrelated architecture. The directory exists so that one V0.3 design package can be frozen, reviewed, implemented and later compared with V0.2/V0.4 as a coherent version baseline.

## 2. Document Authority

The V0.3 package is organized into four authority levels:

1. **Master Design** — product positioning, boundaries, domain model and non-negotiable architecture rules.
2. **Domain / Runtime Design** — detailed Runtime, media, synchronization, production, playout, HA and integration semantics.
3. **API Master Design** — API namespace, semantic planes, command/query/event boundaries, compatibility and SDK rules.
4. **Standalone Product Baseline** — independent boot, Control Plane activation, Web Console and V0.3 implementation acceptance.

When documents overlap, the more specific document may refine semantics but MUST NOT contradict the Master Design or frozen red lines.

## 3. Current Source Documents

The following documents originated from the V0.3 design work and are being consolidated into this directory:

- `00_MASTER_DESIGN.md` — V0.3 master entry point and architecture index.
- `API_MASTER_DESIGN.md` — API Master Design.
- `BROADCAST_MEDIA_RUNTIME_AND_API_MASTER_DESIGN.md` — Broadcast Runtime and detailed architecture baseline.
- `STANDALONE_PRODUCT_BASELINE.md` — Standalone Product Baseline and implementation gates.

During migration, the original root-level `V0.3_*.md` files remain temporarily as compatibility copies. They MUST NOT be treated as a second source of truth. Final cleanup will remove the duplicate paths after content identity and link migration are verified.

## 4. V0.3 Implementation Sequence

```text
V0.3 Architecture Freeze
        ↓
P1 Standalone Runtime Control-Plane Activation
        ↓
P2 Operator Console
        ↓
P3 Runtime Hardening
        ↓
P4 Broadcast Professionalization
        ↓
P5 Conformance
        ↓
P6 Mother Framework Integration
```

P1 is intentionally an activation of the existing Runtime/Control Plane, not an implementation of every V0.3 API namespace.

## 5. Non-Negotiable Direction

- VBMF remains independently buildable, bootable, testable and deployable.
- Standalone and Integrated modes share the same Runtime semantics.
- API is a control/observation boundary, not the Runtime itself.
- Control Plane and Media Plane remain separate.
- Pipeline is logical and is not bound to a physical Node.
- Workload is not equivalent to an OS process.
- Capability / Resource / Placement determine deployment.
- Synchronization is first-class Runtime infrastructure.
- Production switching and Playout are first-class broadcast capabilities.
- HA/failover preserves the same Runtime and media semantics; it does not create a second pipeline model.
- Vendor SDKs remain behind Adapter boundaries.
- Upstream CMS/DAM remains Asset Authority; VBMF owns runtime execution truth, not business asset truth.
- No mother-framework dependency may be introduced into the standalone boot path.

## 6. Migration Rule

The directory migration is documentation-only and MUST NOT change V0.2 frozen Runtime semantics or implementation behavior.

Before deleting legacy `docs/architecture/V0.3_*.md` paths, verify:

1. byte/content-level equivalence or an explicit reviewed consolidation diff;
2. all internal links are updated;
3. no external documentation references the legacy paths unexpectedly;
4. PR #31 remains reviewable as the V0.3 documentation baseline;
5. the final V0.3 tree has one authoritative copy of every design decision.
