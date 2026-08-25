# Media Agent State Machine

**Status:** Frozen contract (2026-08-26)
**Scope:** Media plane (`media-agent`, Rust) lifecycle — Device × Lease × Supervisor.
**Companion:** `MEDIA_RUNTIME_SECURITY_MODEL.md` (runtime/isolation), SoT §10 (Gate A).

> This document defines the **state contract** only. No GStreamer / DeckLink code
> is wired in the skeleton. Gate 2.1 freezes interfaces; GStreamer attaches at
> Gate 2.6+.

---

## 1. States

```
        INIT
         │  (boot, load config)
         ▼
     DISCOVERING
         │  (enumerate DeckLink devices via SDK + DesktopVideoHelper IPC)
         │  ├─ 0 devices ──────────────► DEGRADED
         │  └─ ≥1 device ─────────────► READY
         ▼
       READY
         │  (idle, devices available, no lease)
         │  (HandleAcquire request)
         ▼
      LEASED
         │  (lease granted; pipeline not yet started)
         │  (HandleStart request)
         ▼
    CAPTURING
         │  (pipeline live, frames flowing)
         │
         ├─ device lost / pipeline error ──► DEGRADED
         ├─ lease expired / revoked ──────► RECOVERING
         └─ fatal / unrecoverable ────────► FAILED
         ▼
     DEGRADED
         │  (transient fault; supervisor attempts recovery)
         │  ├─ recovered + lease valid ───► CAPTURING
         │  ├─ lease invalid/expired ─────► RECOVERING
         │  └─ recovery exhausted ────────► FAILED
         ▼
    RECOVERING
         │  (re-acquire device, re-negotiate lease)
         │  ├─ ok + lease valid ──────────► CAPTURING
         │  └─ lease cannot be re-formed ──► READY (await new lease)
         ▼
      FAILED
            (terminal; requires operator / control-plane intervention)
```

---

## 2. State table

| State | Meaning | Lease | Pipeline | Supervisor action |
|---|---|---|---|---|
| `INIT` | Process boot, config loaded | none | none | → DISCOVERING |
| `DISCOVERING` | Enumerating DeckLink | none | none | 0→DEGRADED, ≥1→READY |
| `READY` | Idle, devices present | none | none | await HandleAcquire |
| `LEASED` | Lease granted, pipeline idle | active | none | await HandleStart |
| `CAPTURING` | Frames flowing | active | live | monitor health |
| `DEGRADED` | Transient fault | active/maybe | error | attempt recover |
| `RECOVERING` | Re-forming device+lease | re-forming | none | →CAPTURING or →READY |
| `FAILED` | Terminal | n/a | n/a | await external reset |

---

## 3. The critical invariant (why this doc exists)

> **A pipeline restart after a DeckLink drop MUST re-validate the lease before
> resuming capture.**

The failure mode this contract prevents:

```
Pipeline start
   │
   ▼
DeckLink lost (cable pull / SDK disconnect)
   │
   ▼
restart
   │   ← WRONG: blindly restart and keep capturing
   ▼
lease still valid?   ← this question is the bug if unanswered
```

Correct behavior (enforced by the state machine):

1. DeckLink drop → `CAPTURING → DEGRADED`.
2. Supervisor attempts recovery. **Before** re-entering `CAPTURING`, it MUST check
   lease validity:
   - lease **valid** → re-form pipeline → `CAPTURING`.
   - lease **expired/revoked** → `DEGRADED → RECOVERING → READY` (release device,
     await a fresh `HandleAcquire`). Never capture without a live lease.
3. A lease that expires mid-capture is treated as `RECOVERING`, not silent continue.

This is **not** a GStreamer problem — it is the Device + Lease + Supervisor
interaction. The state machine is the source of truth; GStreamer is a leaf.

---

## 4. Interface freeze (Gate 2.1)

These traits are declared in the skeleton (`src/*.rs`) and MUST NOT change shape
without a versioned decision:

- `DeviceManager` — enumerate / state (`DeviceState::{Unknown,Available,Leased,Error}`)
- `DeviceLease` — `acquire()` / `release()` / `renew()`
- `Pipeline` — `start()` / `stop()` / `restart()`  (restart re-validates lease)
- `Supervisor` — `health()` / `recover()`

**GStreamer is NOT connected in Gate 2.1.** The skeleton compiles with these
interfaces as inert declarations; real device/pipeline logic lands at Gate 2.2+.

---

## 5. Rollout order (frozen)

```
1. Rust skeleton          ✅ done (compiles, CI green)
2. Device discovery        ← next
3. Lease manager
4. Health endpoint
5. Supervisor
6. GStreamer pipeline
7. First frame
```

First frame is intentionally last: it depends on real SDI input + the full
Device/Lease/Supervisor state machine being correct first.
