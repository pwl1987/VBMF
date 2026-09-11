# V0.3 Document Map and Migration Plan

## 1. Canonical V0.3 Package

| New path | Responsibility | Source during migration |
|---|---|---|
| `00_MASTER_DESIGN.md` | Product positioning, boundaries, domain model, frozen invariants | Consolidated V0.3 design baseline |
| `API_MASTER_DESIGN.md` | API namespace, semantic planes, command/query/event, compatibility, SDK rules | `../V0.3_API_MASTER_DESIGN.md` |
| `BROADCAST_MEDIA_RUNTIME_AND_API_MASTER_DESIGN.md` | Detailed Broadcast Runtime/Fabric architecture | `../V0.3_BROADCAST_MEDIA_RUNTIME_AND_API_MASTER_DESIGN.md` |
| `STANDALONE_PRODUCT_BASELINE.md` | Standalone product scope, P1-P6 implementation baseline and acceptance | `../V0.3_STANDALONE_PRODUCT_BASELINE.md` |

## 2. Authority Rules

- `00_MASTER_DESIGN.md` is the V0.3 package entry point.
- `API_MASTER_DESIGN.md` is authoritative for API top-level semantics.
- `BROADCAST_MEDIA_RUNTIME_AND_API_MASTER_DESIGN.md` is authoritative for detailed broadcast Runtime/Fabric architecture unless superseded by a more specific approved contract.
- `STANDALONE_PRODUCT_BASELINE.md` is authoritative for V0.3 standalone implementation scope and phase gates.
- The mother architecture repository remains authoritative for cross-repository ownership, shared context, common event envelope, governance and integration boundaries.
- No document may silently override a frozen V0.2 semantic contract.

## 3. Migration Status

**Phase A — completed:** create the dedicated V0.3 architecture package and master entry point.

**Phase B — next:** copy/reorganize the three existing V0.3 design documents into this directory with reviewed content identity, then update internal references.

**Phase C — cleanup:** after link/reference verification, remove the legacy root-level `V0.3_*.md` duplicates.

The legacy files are intentionally retained during Phase B/C so documentation reorganization cannot accidentally destroy design content.

## 4. Required Review Before Cleanup

1. Compare old and new document contents.
2. Resolve overlap between API Master Design and Broadcast Runtime/API Master Design.
3. Resolve the Standalone Baseline page-scope contradiction (P1 shell vs later professional console capabilities).
4. Add R-number mapping where V0.2 and V0.3 requirement numbering collides.
5. Add HTTP numeric-status mapping without changing the V0.2 error taxonomy.
6. Add D1-D15 design-stage → P0-P6 implementation traceability.
7. Search repository references to legacy paths.
8. Confirm PR #31 remains a clean, reviewable V0.3 documentation baseline.

## 5. Implementation Gate

Documentation migration MUST finish before P1 code changes begin. P1 then follows the approved V0.3 package and its explicit scope: Standalone Runtime lifecycle + Control Plane activation + existing Query/Command/Event projection wiring, not wholesale implementation of all V0.3 namespaces.
