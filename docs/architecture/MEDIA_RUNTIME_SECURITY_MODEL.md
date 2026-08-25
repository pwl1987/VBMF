# Media Runtime Security Model (Option B)

**Status:** Frozen (2026-08-26)
**Supersedes:** Option A (runsc + gVisor device passthrough)
**Decision ref:** MEDIA-SEC-01 Step 3 → `SoT §15` / `DEPLOYMENT_AND_DEV_RUNTIME.md §15.1`
**Evidence:** `evidence/bmd-10.30.15.10/2026-08-26-media-sec-01-step3.md`

---

## 1. Decision

The media plane (`media-agent`, GStreamer + Blackmagic DeckLink SDK) runs under
**`runc` with hardening** — not `runsc` (gVisor).

| Runtime | DeckLink enumeration | Pipeline open | Verdict |
|---|---|---|---|
| `runc` + helper IPC + shm | ✅ Detected 3 devices | ✅ live / PREROLLED | **PASS** |
| `runsc` (×3 variants: bind, `--cap-add=ALL`, `seccomp=unconfined`) | ❌ Detected 0 devices | ❌ | **FAIL** |

**Rationale:** "Stable capture > container isolation." gVisor does not implement the
low-level interfaces the Blackmagic SDK relies on for device enumeration
(`/dev/blackmagic` ioctls, `AF_UNIX` + `SCM_RIGHTS` shared-memory IPC to
`DesktopVideoHelper`). This is a gVisor limitation, not a tunable flag — all three
runsc variants failed identically. Option A is **permanently closed**.

---

## 2. Runtime tiering (frozen)

| Profile | Runtime | Hardening |
|---|---|---|
| `dev` | `runc` | none (debug ergonomics) |
| `acceptance` | `runc` | Option B baseline (below) |
| `prod` | `runc` | Option B + `read_only: true` + `tmpfs` |

Do **not** reintroduce `runsc` for the media plane. If a stronger isolation
requirement appears later, evaluate Kata/containers-with-VM, not gVisor.

---

## 3. Option B hardenning (authoritative)

Authoritative source: `ops/compose.acceptance.yml` / `ops/compose.prod.yml`
(`media-agent` service). Summary:

```yaml
media-agent:
  runtime: runc
  devices:
    - /dev/blackmagic:/dev/blackmagic          # device allowlist (NOT privileged)
  device_cgroup_rules:
    - 'c 10:* rmw'                              # Blackmagic major=10 (dv0/dv1/io0)
  cap_drop:
    - ALL
  cap_add:
    - SYS_NICE                                  # realtime capture thread scheduling
    # SYS_ADMIN is FORBIDDEN. Add minimally, only if SDK probe proves a need.
  security_opt:
    - no-new-privileges:true
  ipc: host                                     # /dev/shm/com_blackmagicdesign_* SDK IPC
  shm_size: 1g
  # PROD only:
  read_only: true
  tmpfs:
    - /tmp:size=512m,mode=1777                  # GStreamer/FFmpeg scratch
```

### Rules (must hold on every change)
1. **Never** `privileged: true`. Device access is by explicit allowlist only.
2. **Always** `cap_drop: ALL`, then add back the minimum (`SYS_NICE` today).
   `SYS_ADMIN` is banned by review policy.
3. **Always** `no-new-privileges: true`.
4. **DeckLink SDK IPC requires `ipc: host`** — the plugin talks to the host
   `DesktopVideoHelper` over `/dev/shm/com_blackmagicdesign_*`. This is why
   `--device=/dev/blackmagic` alone is insufficient (Step 3 root-cause).
5. Default Docker seccomp profile is sufficient; a tighter
   `ops/nginx/seccomp-media.json` may be referenced later but is not required.

---

## 4. Network bootstrap dependency (NET-01)

The build/pull path depends on a reachable mirror (`docker.1ms.run` primary,
`dockerproxy.net` / 网易 as fallback). This is an **environment constraint /
bootstrap dependency** — see SoT §9. It is NOT a runtime security control; it
only affects image acquisition at provisioning time.

---

## 5. What is explicitly out of scope here
- GStreamer pipeline internals → `MEDIA_AGENT_STATE_MACHINE.md`
- Lease / recovery logic → `MEDIA_AGENT_STATE_MACHINE.md`
- Control-plane (Node/Fastify) security → separate INFRA-SEC docs
