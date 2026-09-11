# VBMF V0.3 — Master Design

> **Status: LOCKED BASELINE**  
> Version: V0.3  
> Product: **Professional Real-Time Media Playout System / 专业实时媒体播出系统 / Real-Time Media Runtime & Fabric**

## 1. Product Positioning

VBMF V0.3 is a professional real-time media runtime/fabric whose first-principle responsibility is the reliable runtime lifecycle of professional media from input, synchronization and processing through production, playout, protection, transport and output.

Encoder, decoder, transcoder, gateway, switcher, scheduler, receiver, recorder and capture hardware are capabilities, workloads, resources or adapters within this product boundary—not independent product identities.

## 2. Runtime Boundary

```text
Input / Contribution
        ↓
Clock / Timestamp / Synchronization
        ↓
Media Graph / Processing
        ↓
Production / Switching
        ↓
Playout / Scheduling
        ↓
Broadcast Protection
        ↓
Encode / Mux / Packetize
        ↓
Transport / Gateway
        ↓
Output / Delivery
        ↓
Monitoring / Audit / Recovery
```

This is a logical lifecycle. It does not require one machine, one process or one vendor.

## 3. Primary Domains

1. Media I/O
2. Media Processing
3. Media Gateway
4. Media Graph
5. Synchronization
6. Production
7. Playout
8. Broadcast Protection
9. Transport / Delivery
10. Resource / Workload / Placement
11. High Availability / Continuity
12. Recording / Replay / Timeshift
13. Observability / Diagnostics / Audit
14. Runtime Control / Query / Event
15. Asset Runtime Integration Boundary

These are domain boundaries, not a requirement to create fifteen services.

## 4. Core Architectural Model

```text
Runtime
 ├── Node / Device / Adapter
 ├── Resource / Capability
 ├── Workload / Placement / Allocation
 ├── Source / Input / Signal / Stream
 ├── Pipeline / MediaGraph / MediaLink
 ├── Production / Playout
 ├── Protection / HA / Continuity
 ├── Synchronization / MediaTimeline
 └── Observation / Control / Query / Event
```

### Frozen invariants

- Pipeline MUST NOT be bound to a physical Node.
- Workload MUST NOT be equated with an OS process.
- Node MUST NOT be permanently assigned to one function.
- Scheduler MUST NOT become a media processor.
- Control Plane MUST NOT carry the real-time media main path.
- Cross-node media MUST use explicit transport/link semantics.
- Single-node and distributed deployments MUST use the same domain contracts.
- HA and scheduling MUST NOT introduce a second pipeline model.
- Capability / Resource / Placement MUST drive deployment variation.
- All deployment modes MUST preserve signal continuity requirements.

## 5. Synchronization

Synchronization is first-class Runtime infrastructure covering clock synchronization, timestamps and frame alignment.

Supported concepts include PTP, NTP, GPS/GNSS, Genlock, Timecode, ClockDomain, Timebase, FrameSyncGroup, JitterBuffer and MediaTimeline.

The Runtime MUST be able to represent states such as `SYNCED`, `DEGRADED`, `WAITING`, `LATE`, `DROPPED`, `UNSYNCED` and `RECOVERING`.

## 6. Production and Playout

Production switching is a professional Runtime capability with a model that can accommodate M/E, PGM, PVW, Keyer, DSK, AUX, AFV, Tally, Multiview, Scene, Macro, Transition, Cut and Take.

Playout is a first-class product domain with VirtualChannel, Playlist, Schedule, ProgramItem, PlayoutEvent, PlayoutSession, Queue, NowPlaying, Override and Rule.

## 7. Broadcast Continuity

V0.3 supports a common continuity model across receiver/source/card/port/pipeline/node/output/cluster protection levels, including single, active/standby, active/active and N+1 patterns.

Preferred transition sequence:

```text
Prepare New
 → Start New
 → Verify Ready
 → Fence Old
 → Atomic Switch
 → Verify
 → Stop Old
```

Continuity semantics remain part of Runtime execution and MUST NOT be delegated to UI behavior.

## 8. Adapter Boundary

Hardware and protocol vendors are implementations behind Adapter boundaries. BMD/DeckLink is one implementation, not the architectural category. The abstraction also permits AJA, Magewell, Deltacast, Network, Mobile Contribution and Virtual adapters.

Vendor-specific objects and SDK types MUST NOT leak into canonical API/domain contracts.

## 9. Control / Query / Event

```text
HTTP / External Consumer
        ↓
API Boundary
   ┌────┼────┐
 Query Command Event
   ↓      ↓     ↓
Runtime Snapshot / Control Plane / Event Projection
```

API is not Runtime. Query is observation, Command expresses intent, and Event records facts. Preview/media delivery remains a separate transport plane.

## 10. Standalone / Integrated

Standalone mode is first-class, not a debug mode. Integrated mode may replace identity, registry, policy or provider dependencies, but MUST preserve the same Runtime, domain, command, query and event semantics.

VBMF MUST remain independently buildable, bootable, testable, operable and deployable without the mother architecture repository.

## 11. Asset Boundary

Upstream CMS/DAM remains the business Asset Authority. VBMF may reference, cache, preload, validate readiness, materialize a runtime view, consume, execute and audit runtime use, but MUST NOT create a competing business asset truth.

## 12. V0.3 Implementation Principle

V0.3 implementation is incremental. The frozen top-level architecture does not imply that every domain or API namespace is implemented in P1.

P1 activates the standalone Runtime/Control Plane and existing tested API contract first; later phases add professional capabilities and conformance evidence without silently changing frozen semantics.
