# RF-SRC-RTMP-02 — Production RTMP input boundary

- Date: 2026-09-19
- Status: **READY / PLAN FROZEN; implementation not started**
- Implementation packet: `RF-SRC-RTMP-02-IMPLEMENTATION`
- Boundary: production RTMP listener admission and lifecycle for one network source
- This document freezes design only. It does not authorize runtime implementation in
  this commit.

## 1. Authority and relationship to RTMP-01

This packet is the bounded follow-up to `RF-SRC-RTMP-01`. RTMP-01 proved the
typed Network Source ownership path, FFmpeg `-rtmp_listen 1` loopback lifecycle,
recovery and teardown. It did not prove production LAN admission, endpoint
allowlisting, path enforcement, or the absence of DeckLink side effects.

The authority chain is:

1. `docs/architecture/MEDIA_BACKEND_CONTRACT.md` §§1, 1.1/P0-8, 3, 4:
   Runtime Orchestrator owns Resource Reservation → Binding → Backend
   instantiation; Backend cannot acquire an undeclared resource; all vendor
   failures close through canonical events; backend replacement cannot change
   `CanonicalPipelinePlan`.
2. `docs/architecture/RUNTIME_SESSION_MODEL.md` §§4, 4.1: SessionManager is the
   only Session creator/destroyer; Session owns lifecycle, Backend owns the
   implementation object, and Supervisor observes/decides but does not create a
   Session.
3. `docs/architecture/RUNTIME_RESOURCE_MODEL.md` §§3, 4, 4.2: Resource,
   Reservation, Lease and Allocation remain distinct; ResourceRegistry owns
   Resource state; the Reservation lifecycle is `Preflight → Reserve → Create
   Session → Acquire Lease → Start → Allocated`, with explicit Release/Abort.
4. `docs/architecture/RUNTIME_BINDING_MODEL.md`: Binding is the authorized
   Physical/Provider/Runtime Resource connection; it is not a second source of
   canonical identity and cannot silently replace a stale binding.
5. `docs/superpowers/plans/2026-09-18-rf-src-rtmp-01-source-boundary.md` and
   `docs/superpowers/plans/2026-09-18-rf-src-rtmp-01-boundary-design.md`: RTMP-01
   remains single protocol/single input/FFmpeg only, preserves the DeckLink wire
   shape, uses typed `NetworkSourceId`, and does not add credentials, SRS,
   failover, UI/API, or a second owner.
6. `.project/STATE.md` §3.50 and
   `docs/superpowers/plans/2026-09-19-runtime-features-next-candidate-review.md`:
   RTMP external/non-loopback expansion was not READY until a new bounded
   security, endpoint and fixture design existed. This document is that design;
   it does not reopen SRT, Network umbrella, Program/Switch, Output, Recording,
   Flow, Idempotency or stability work.

The existing `SourceIntent::Rtmp` wire shape remains unchanged under D11. The
new production authorization is a server-side startup manifest and is not added
to the wire intent.

## 2. Frozen decisions D1–D11

### D1 — VBMF is the passive RTMP listener

VBMF is the listener for a third-party publisher. The implementation uses
FFmpeg's `-rtmp_listen 1` mode. It does not pull from a remote RTMP server and
does not introduce SRS or another external media server.

There is one listener and one FFmpeg input per Session. The third party pushes
to an address owned by the VBMF host.

### D2 — Strict endpoint form and address policy

Only the following endpoint forms are accepted:

```text
rtmp://<canonical-ipv4>:<explicit-port>/<path>
rtmp://[<canonical-ipv6>]:<explicit-port>/<path>
```

The parser must validate the input and require that its textual host form is
already canonical. It must not accept a non-canonical input and then silently
normalize it for authorization. Authorization compares the complete canonical
endpoint `(protocol, canonical IP, explicit port, path)`.

The frozen validation rules are:

- IPv4 is strict dotted decimal: four decimal octets, each in `0..255`, with
  canonical spelling and no leading-zero alternate spelling.
- IPv6 must be bracketed in the URL, use canonical lowercase literal spelling,
  and contain no zone identifier. An unbracketed IPv6 URL is rejected.
- Hostnames, DNS names, implicit port `1935`, missing port, userinfo, query,
  fragment, control characters, whitespace, wildcard or unspecified addresses
  (`0.0.0.0`/`::`) are rejected.
- Host text is limited to 255 bytes. Overlong host text is rejected before any
  address operation.
- Ports must be explicit, in `1024..65535`; privileged ports `<1024` are
  rejected, including an otherwise valid explicit port.
- Public unicast, link-local (`169.254.0.0/16`, IPv6 link-local), multicast and
  broadcast addresses are rejected. So are IPv4-mapped IPv6 (`::ffff:a.b.c.d`),
  CGNAT (`100.64.0.0/10`) and documentation addresses (`192.0.2.0/24`,
  `198.51.100.0/24`, `203.0.113.0/24`, `2001:db8::/32`): any address outside the
  explicitly allowed classes is rejected. Private IPv4 and IPv6 ULA addresses
  are eligible only when D5 proves that the exact address belongs to this host.
- Loopback is reserved for the existing local fixture and regression tests. A
  production LAN claim must use an explicit non-loopback address and Tier 1 or
  Tier 2 evidence; a loopback listener must not be reported as cross-host
  acceptance.
- The path is non-empty, begins with `/`, and contains no credentials, query,
  fragment, or control/whitespace characters. The path is part of the exact
  allowlist key, not an identity proof.

Examples of accepted syntax are illustrative only; the runtime must never print
their literal host, port or path in canonical observations:

```text
rtmp://192.168.10.20:19350/live/source
rtmp://[fd00::10]:19350/live/source
```

The parser emits one canonical endpoint representation. A supplied spelling
that is not already that representation is rejected rather than compared after
normalization. No runtime DNS resolution or later re-resolution is allowed.

### D3 — `NetworkSourceBinding` manifest contract

The production startup manifest is the server-side authorization source. Its
schema is exactly:

```json
{
  "version": 1,
  "machine_id": "<machine-id>",
  "entries": [
    {
      "source_id": "<UUID>",
      "endpoint": "rtmp://<canonical-ip>:<explicit-port>/<path>"
    }
  ]
}
```

Contract rules:

- `version`, `machine_id` and `entries` are required. `entries` may contain
  multiple entries.
- Each `source_id` is a UUID. Each endpoint is the strict canonical string from
  D2.
- The mapping is bidirectionally one-to-one: duplicate `source_id` or duplicate
  complete endpoint makes the entire manifest invalid. Partial loading is not
  allowed.
- A `machine_id` mismatch is a startup failure and rejects the process
  fail-closed.
- The manifest must be a regular file with mode `0600`; any group/other bit or
  unreadable state rejects loading. No permissive fallback is allowed.
- Loading is race-free on a single file descriptor: `open` (with `O_NOFOLLOW`,
  rejecting symlinks) → `fstat` → verify regular file, owner is exactly the
  service-running user, and mode `0600` → `read` → `parse`, all on that one
  already-opened descriptor. Checking the path first and then reopening the file
  is forbidden (file-swap race); every metadata check must guard the same bytes
  that are actually parsed.
- The loader reads the manifest exactly once during process startup. There is no
  hot reload, and the loader does not follow file replacement. A manifest
  change, machine address change or NIC change takes effect only after service
  restart.
- Loading also runs D5 for every entry before any Network Resource is registered.

`SourceIntent::Rtmp` keeps its existing `source_id` and endpoint fields. The
manifest binds that source ID to the exact listener endpoint; it does not add a
credential, stream key, account or publisher record.

### D4 — credential-free; path is not authentication

This packet remains credential-free. It does not create a credential management
system, stream-key system, publisher account system, or secret store.

The endpoint allowlist constrains the local listener address and port. It does
not prove the identity of the publisher. The RTMP path is a routing label and
may be used for admission only if the actual FFmpeg listen mode enforces it.
Whether the BMD FFmpeg build enforces `app`/path in `-rtmp_listen 1` mode is a
capability-probe prerequisite. If it accepts any publisher on the bound port,
the path must not be described as a security boundary; the unverified publisher
identity is a known, explicitly accepted risk.

No endpoint secret, URL, path, host or port may enter canonical events, errors,
health, evidence, panic messages or operator-facing debug output.

### D5 — current local address ownership is mandatory

At startup, after strict parsing and before authorization is accepted, every
manifest endpoint IP is compared with the exact addresses currently assigned to
the host's network interfaces. Private/ULA classification alone is
insufficient. An address that is not currently owned by a local interface
rejects the entire production composition fail-closed.

The listener binds only the addresses explicitly present in the manifest. It
does not bind an unspecified address, all interfaces, a guessed address, or an
address obtained by runtime DNS resolution. Interface changes are not watched;
they require restart and a fresh manifest check.

### D6 — bind and port conflict authority

The Network Resource Registry uniqueness key is:

```text
(protocol, canonical_ip, port)
```

The path is deliberately not in this Registry key: two paths on one protocol/IP/
port cannot be treated as two independent listeners. IPv4 and IPv6 have
separate literal keys and are never collapsed into one wildcard or host key.

Registry duplicate registration is rejected before Session start. The operating
system `bind()` call is the final authority. The implementation must not probe
or reserve a port before `bind()`; a probe-then-bind sequence is a TOCTOU bug.

An OS `Address already in use` or equivalent bind error is classified as
`BindFailure`, transitions the Session to `ManualRequired`, stops the attempt,
and does not consume recovery budget or blindly retry.

### D7 — Session and Recovery decision table

Ownership remains unchanged: SessionManager owns Session lifecycle,
RecoveryMonitor observes, Supervisor decides, and LeaseManager owns lease
validity.

| Condition | Session state | Recovery behavior |
|---|---|---|
| Listener bind succeeds, no publisher yet | `Running / Waiting` | Not a fault; do not restart. Missing first frame is not an automatic restart trigger. |
| Publisher connects and A/V is produced | `Running / SignalVerified` | Reset the recovery budget. |
| Publisher disconnects and the cause is attributed | Existing canonical failure state | Perform bounded restart under D8 after lease revalidation. |
| Listener bind fails | `ManualRequired` | Stop immediately; do not retry and do not consume budget. |
| FFmpeg exits abnormally with unknown cause | `ManualRequired` | Do not blindly retry. |
| Recovery budget is exhausted | `ManualRequired` | Stop recovery. |
| `stop` / `close` | `Released / Available` | Child, listener socket, monitor and lease must all exit/release. |

`Waiting` is an expected state while the listener is idle. It is not evidence of
publisher failure, and it must not create a recovery event.

### D8 — Per-Session/per-endpoint bounded recovery budget

The budget is independent for each `(Session, endpoint)` pair; it is never a
global process budget. The maximum is 5 automatic recovery attempts. Backoff is

```text
min(1 second * 2^attempt, 60 seconds)
```

Only one completed publisher connection that produces both required A/V streams
and reaches `SignalVerified` resets the budget. Listener creation alone does not
reset it.

The following classes do not enter the budget and immediately produce
`ManualRequired`: `BindFailure`, `SpawnFailure` and `UnknownExit`. `stop` and
`close` clear the budget. Once `ManualRequired` is reached, only an explicit
operator `stop` followed by `start` of a new Session may try again; the old
Session never self-revives.

Before every automatic restart, RecoveryMonitor must revalidate the exact lease
and the exact `(protocol, canonical_ip, port)` Registry claim. If either is no
longer valid, recovery stops and becomes manual.

The existing Supervisor remains the decision owner. Any implementation change
needed to express the per-Session/per-endpoint counter must not create a second
recovery policy or a second Session/Lease truth store.

### D9 — bounded stderr capture and complete redaction surface

FFmpeg stderr is diagnostic input only and has a bounded, non-blocking capture
contract:

- Use an independent reader thread that continuously drains the child stderr
  pipe, preventing pipe deadlock.
- Keep a ring buffer no larger than 64 KiB and no larger than 256 lines.
- Truncate every individual line to 512 bytes before it enters the ring.
- Before child reaping completes, wait for the reader thread to exit. Stop,
  recover, and close must not leave a reader or child behind.

The raw stderr text is never copied into a canonical event, error, `Debug`
representation, health projection, evidence record or panic message. It is
reduced to only one of these finite categories:

```text
BindFailure | PublisherDisconnected | SpawnFailure | UnknownExit
```

Those categories map through the existing canonical event vocabulary. They do
not become a new decision truth path and are not used to expose vendor text.
Canonical events, logs, health and evidence carry `NetworkSourceId` only. The
redaction implementation must cover `PipelinePlan` serialization and the
`session.rs:700` `Debug`-hash path; the latter must not make endpoint text
observable or create endpoint-dependent identity leakage.

The unified negative test is strict: in every relevant canonical state, event,
error, `Debug`, JSON, health and evidence text, the endpoint host/IP, port, URL
or path literal must be absent. The test covers `Display`, `Debug`, `serde`,
panic/error construction and evidence rendering. The inbound
`SourceIntent::Rtmp` wire compatibility test is the explicit ingress exception:
it may carry the already-authorized endpoint shape, but it must still reject
credentials and must never be reused as an observability projection.

### D10 — network-only composition has zero DeckLink side effects

For a Network-only composition, the following are hard invariants:

- Do not acquire `DeviceLease` and do not create `LeaseKey::Device`.
- Do not open a DeckLink input or output.
- Keep every Device Resource `Available` and unchanged.
- Do not touch input `0`, input `1` or output device-number `2`.
- Register only the Network Resource and its exact Network lease/binding.
- Network and Device Resources may coexist in one Registry, but their owner and
  lease types must remain distinct.

The implementation must audit and, if necessary, split the current bootstrap
path so that a network-only composition does not acquire startup device
placeholder leases or call a hardware composition helper as a side effect.
The existing `bootstrap::build()` discovery/placeholder behavior is not allowed
to become an implicit Network source dependency. A negative test must assert
before/after Device Resource state, Device lease set, Network registration,
input 0/1 and output device-number 2 observation, and binding-manifest digest.

### D11 — wire contract remains unchanged

`SourceIntent::Rtmp` keeps its existing shape and fields. No new wire field is
added for credentials, manifest entries, listener mode, port ownership, path
authentication or recovery budget. Production authorization remains a server-side
`NetworkSourceBinding` manifest loaded at startup.

### Implementation Invariants (pre-implementation amendment, 2026-09-19)

These invariants refine D3/D7/D8 for implementers. They do not change D1–D11
direction, scope or stop conditions.

**INV-1 — recovery triggers only on an exited child with a positive
`PublisherDisconnected` classification.**

- The FFmpeg child is still alive and back in the listen/waiting state: no
  restart and no recovery event (D7 `Running / Waiting`).
- The child has exited **and** the classification — derived from exit status
  plus existing canonical events plus the bounded stderr evidence, never from
  an stderr keyword alone — is `PublisherDisconnected`: enter the D8 recovery
  cycle after lease/claim revalidation.
- `UnknownExit`, a stderr reader failure, or any state that cannot be
  positively determined: `ManualRequired` immediately; never an exploratory
  restart.

**INV-2 — recovery actions are generation-isolated.**

Every recovery task captures the session/lease generation (or the existing
equivalent epoch/cancellation token — this adds no new truth store) at the
moment it is scheduled. When `stop`, `close` or `ManualRequired` invalidates
that generation, all late events, timers and restart actions from the old
generation are discarded. A stopped or manually-held Session can never be
re-spawned by a stale background recovery worker.

**INV-3 — manifest parsing is strict and race-free end to end.**

Building on D3:

- The manifest file is rejected when larger than 1 MiB.
- The JSON parser must reject duplicate object fields, unknown fields and any
  trailing data after the top-level value; no permissive fallback.
- `source_id` must be the canonical UUID textual representation; any
  non-canonical spelling is invalid.
- After `read` and before parse-accept, `fstat` the same descriptor again and
  require the size and file state to be unchanged; any change rejects loading.
- Owner and permission checks are evaluated for the actual service-running
  user identity after privilege drop. If the process starts privileged, the
  documented load order must place manifest loading after the drop (or verify
  explicitly against the final service uid); the checks never pass merely
  because a privileged loader can read the file.

## 3. Capability probe and BMD evidence levels

The implementation package begins with **Step 0**, a BMD read-only capability
probe. It is governed by the SRT-blocker discipline: no SRT work is reopened and
RTMP success cannot be used to claim SRT support.

Step 0 must verify both:

1. The BMD FFmpeg binary can use `-rtmp_listen 1` to bind an explicit
   non-loopback LAN address.
2. In listen mode, the BMD FFmpeg build's `app`/path behavior is determined
   with a negative matrix, not only the happy path. The probe must cover:
   correct address + correct path; correct address + wrong path; correct
   address + an extra trailing `/`; and query/fragment/percent-encoded
   variants. The outcome is recorded definitively as either "path enforced"
   or "path is a routing label only".

If either probe fails, the implementation package returns to `PLAN REQUIRED`.
If path/app is not enforced, the design remains technically admissible only with
the D4 known-risk wording; the path must not be presented as publisher
authentication or a security boundary.

Evidence levels are frozen:

- **Tier 1:** same-host binding to a LAN address is verified. This supports only
  the statement **“non-loopback listener binding verified.”** It does not prove
  a third-party publisher crossed a network boundary.
- **Tier 2:** an independent host or independent network namespace performs a
  real third-party push to the BMD LAN address. Only this supports
  **“cross-host third-party push verified.”**

The preferred fixture is Development VM → BMD LAN address. If only Tier 1 is
completed, all reports and STATE entries must remain at Tier 1 wording.

### Deferred production risks (registered by this freeze)

The following network-ingress resource-exhaustion exposures are registered as
explicit deferred risks. If the BMD FFmpeg build cannot provide the matching
control, that fact is not a packet blocker, but it must be annotated in the BMD
acceptance report and carried in `.project/STATE.md` §9:

- RTMP handshake timeout: an idle or malicious half-connection may hold the
  single listener slot indefinitely.
- An unauthorized publisher can occupy the unique `(protocol, ip, port)`
  listener; D4 already states that path is not authentication.
- High-rate FFmpeg stderr output (bounded by the D9 ring, but the drain cost
  is real).
- Process-exit ordering: kill, `wait`, stderr-reader join and socket release
  must happen in an order that never leaks a child, reader or listener socket.
- Host firewalling is a deployment recommendation only; it never substitutes
  for VBMF-internal authorization and must not be reported as one.

## 4. Implementation packet boundary (pre-registered; not executed here)

### Allowed scope

- `source.rs`: strict endpoint parser, canonical form, address-class checks,
  privileged-port and host-length checks, redaction-safe display helpers.
- `NetworkSourceBinding`: manifest type, schema validation, duplicate checks,
  exact `0600` check, machine pin, one-time startup load, local-interface
  ownership validation and exact endpoint lookup.
- `preflight.rs` / `session.rs`: production manifest authorization and exact
  endpoint matching, Session/SignalVerified state and budget transitions while
  preserving SessionManager ownership.
- `bootstrap.rs` / `config.rs`: production Network Source registration and a
  composition path with no device side effect; startup audit and fail-closed
  wiring.
- `adapters/ffmpeg.rs`: listener argv, explicit bind input, bounded stderr reader
  thread, child/reaper ordering and finite failure attribution.
- `recovery_monitor.rs` / `supervisor.rs`: D7/D8 routing, lease revalidation,
  per-Session/per-endpoint budget and ManualRequired transitions.
- Resource Registry uniqueness key `(protocol, canonical_ip, port)` and exact
  Network/Device owner separation.
- A redaction helper plus the unified negative test suite.
- `gates/ffmpeg_recovery.rs` LAN fixtures and evidence reporting for Tier 1,
  with Tier 2 attempted where the independent host/namespace is available.

### Forbidden scope

- SRS, an external media server, pull mode, or any non-FFmpeg listener owner.
- Credentials, stream keys, accounts, publisher identity management or secret
  storage.
- Web UI/API, multiple inputs, Program/Packet/Master Switch work, or a second
  Session/Resource/Lease truth.
- SRT or any other protocol, Hot-Standby, source switching, Recording/Replay,
  output expansion or a new output device path.
- Clock/FLOW/IDEM, 24-hour stability, or unrelated warning cleanup.
- Changing `SourceIntent::Rtmp` shape or adding endpoint truth to a canonical
  plan solely to make the backend convenient.
- `output device-number 2`, input 0/1 manipulation, or weakening loopback
  regression coverage.
- Hot reload of the manifest or runtime network re-resolution.
- Port pre-probing before `bind()`.

## 5. Touch-gates and sequencing

| Gate | Required action | Advance condition | Return to PLAN REQUIRED |
|---|---|---|---|
| TG-0 | Run the two BMD capability probes, read-only. | Both probes have reproducible evidence. | Either `-rtmp_listen` LAN bind or path/app behavior is unknown/failed. |
| TG-1 | Implement and test endpoint parser + manifest loader. | Strict canonical forms, duplicate rejection, `0600`, machine pin and D5 all fail closed. | Authorization needs credentials, DNS, wildcard or a second binding truth. |
| TG-2 | Split/configure production network composition. | Network-only construction registers Network Resource only and has zero Device lease/DeckLink side effects. | No side-effect-free path can be constructed. |
| TG-3 | Wire FFmpeg listener, stderr reader and finite attribution. | `bind()` is authoritative; reader drains; child/reaper has no leak; raw stderr is not observable. | Backend requires URL/secret in canonical state or raw stderr in decision truth. |
| TG-4 | Wire D7/D8 through existing SessionManager/RecoveryMonitor/Supervisor. | Idle wait, SignalVerified reset, attributed disconnect recovery, bind/unknown/spawn manual paths all pass. | SessionManager must be bypassed or a second owner is required. |
| TG-5 | Run redaction and D10 negative tests. | All output surfaces omit endpoint literals; Device state/lease and manifest digest are unchanged. | Any canonical surface leaks endpoint or touches DeckLink. |
| TG-6 | Run exact-commit BMD fixture and CI. | Software matrix, 7/7 CI, and Tier 1/Tier 2 evidence satisfy the declared level. | Capability or LAN acceptance fails without relaxing D10 or the evidence wording. |

## 6. Acceptance matrix

### Software acceptance

The focused suite must reject, fail-closed and cover all of the following:

- canonical and non-canonical IPv4/IPv6 forms;
- hostname/DNS, unbracketed IPv6, missing/implicit port, zone-id, empty path,
  query/fragment/userinfo, control/whitespace and credential-bearing values;
- privileged ports, overlong hosts, unspecified, public, link-local, multicast
  and broadcast addresses;
- private/ULA addresses not owned by a current local interface;
- manifest file mode other than exact `0600`, malformed schema, duplicate source
  ID, duplicate endpoint and machine ID mismatch;
- manifest one-time loading and no hot reload behavior;
- unauthorized source ID, unauthorized complete endpoint and endpoint path
  mismatch;
- a `SourceIntent::Rtmp` wire round trip without changing its shape;
- Registry duplicate `(protocol, canonical_ip, port)` registration, with IPv4
  and IPv6 remaining distinct;
- D7 row-by-row behavior: idle listener does not restart; A/V SignalVerified
  resets; attributed disconnect recovers within budget; bind failure is immediate
  ManualRequired; unknown exit does not blindly retry; stop/close leaves no
  child, listener or monitor;
- D8: five-attempt cap, `1s → 60s` capped backoff, per-session/per-endpoint
  independence, reset only after SignalVerified, and lease revalidation before
  each restart;
- D9: bounded ring, line truncation, independent reader, reader join before
  reap completion, finite attribution and the unified negative redaction matrix;
- D10: no `DeviceLease`, no `LeaseKey::Device`, Device Resources remain
  `Available`, Network-only registration only, input 0/1 and output 2 untouched,
  and manifest digest unchanged;
- strong canonical types are the only endpoint currency: parsing, manifest
  matching, Registry deduplication and FFmpeg argv generation all accept only
  the canonical newtype (e.g. `CanonicalRtmpEndpoint` / `CanonicalIp` /
  `CanonicalPath` / a network binding-key type); no raw string-comparison path
  may bypass D2 validation;
- wording discipline: no acceptance test, log line, evidence text or
  operator-facing string may equate "authorized endpoint" with "authenticated
  publisher"; when probe ② shows path is not enforced, no output may call it
  path authentication.

Run the existing mock/default/FFmpeg feature matrix, `fmt`, `clippy`,
architecture-portability, remove-adapters and diff checks. A passing unit test
or argv test alone is not acceptance.

### BMD exact-commit acceptance

On the exact implementation commit, perform:

1. Step 0 probes (explicit non-loopback LAN bind and path/app behavior).
2. Existing loopback RTMP regression from RTMP-01.
3. Tier 1 LAN acceptance at minimum: authorized address binding, H.264/AAC
   publisher, A/V `SignalVerified`, attributed disconnect, bounded recovery and
   clean teardown.
4. Tier 2 cross-host push if the Development VM → BMD LAN fixture is available.
5. Unauthorized endpoint rejection in the box before child spawn.
6. No residual socket, FFmpeg child, stderr reader or RecoveryMonitor.
7. D10 before/after evidence: zero new DeviceLease, Device Resource
   `Available`, input 0/1 unchanged, output device-number 2 unchanged, and the
   binding-manifest MD5 unchanged.
8. Deferred-risk annotation: observed FFmpeg-side limits on handshake timeout,
   connection control or path enforcement are recorded in the report per §3
   deferred risks and carried into `.project/STATE.md` §9; the absence of such
   limits is recorded explicitly too.

The report must use the correct evidence claim. Tier 1 must never be promoted to
“cross-host third-party push verified.”

### CI and governance acceptance

- The implementation commit must pass all seven required contexts:
  `rust-format`, `rust-test-matrix`, `rust-clippy`, `hardware-test-compile`,
  `architecture-portability`, `gstreamer-build`, `session-lifecycle`.
- The exact implementation commit, `main`, `origin/main` and GitHub must be
  reconciled before any COMPLETE transition.
- BMD evidence remains a separate exact-commit manual acceptance; ordinary PR CI
  does not become a substitute for hardware evidence.

## 7. Stop conditions

Return to `PLAN REQUIRED` and do not improvise if any of these occurs:

- authorization requires a canonical plan to carry credentials, raw address
  truth, or a new wire field;
- the Network Source needs a second Session, Resource, Lease or Recovery owner;
- the implementation must bypass SessionManager;
- a side-effect-free Network composition cannot be constructed because bootstrap
  acquires hardware leases or opens hardware unavoidably;
- either Step 0 capability probe fails or cannot distinguish path enforcement
  from port-only acceptance;
- LAN acceptance cannot complete without touching output device-number 2 or
  changing the manifest;
- endpoint literals appear in any canonical state/event/error/Debug/JSON/health/
  evidence/panic surface;
- a bind conflict is handled by pre-probing, blind retry or budget consumption;
- BMD evidence is only Tier 1 but the proposed report claims Tier 2;
- the fixture cannot carry both H.264 video and AAC audio, or a failure cannot be
  attributed without exposing raw vendor stderr.

## 8. Audit findings from the live main tree

These are design inputs, not claims that implementation has started.

| File:line | Finding | Required consequence |
|---|---|---|
| `services/media-agent/src/bootstrap.rs:50-83` | `bootstrap::build()` loads config, selects the provider and discovers devices during the common construction path. | Network-only composition must not inherit hardware acquisition as an implicit dependency; discovery may remain diagnostic only if it has no Device lease/open side effect. |
| `services/media-agent/src/bootstrap.rs:99-120` | Common bootstrap acquires placeholder Device leases for every discovered device and performs a Device lease collision self-check. | D10 requires an explicit audit/split so Network-only startup cannot acquire or retain these Device leases. |
| `services/media-agent/src/bootstrap.rs:170-220` | `build_ffmpeg_session_composition()` loads the DeviceBinding manifest, builds an authorized PortRegistry/Device Resource set and creates the FFmpeg SessionManager. | The production Network path must not call this hardware composition helper when it is not using Device resources. |
| `services/media-agent/src/bootstrap.rs:223-253` | `build_ffmpeg_network_source_composition()` first calls `build_ffmpeg_session_composition()` and only then creates a Network Resource; it also registers the source in the shared Supervisor. | This is the known D10 audit finding. The implementation must provide a genuinely network-only composition path while retaining the existing Session/Supervisor owners. |
| `services/media-agent/src/config.rs:32-35,57-103` | Config currently exposes `MEDIA_AGENT_DEVICE_BINDING` for the Device manifest but has no NetworkSourceBinding manifest contract. | Add only the production startup binding configuration required by this packet; no hot reload or second config truth. |
| `services/media-agent/src/source.rs:60-112` | Current `NetworkEndpoint` accepts a free-form host string, allows any non-zero port, has a loopback-only acceptance helper, and formats a URL. | Replace/extend validation with D2 canonical IP/address/port policy and keep URL construction adapter-local and redaction-safe. |
| `services/media-agent/src/graph_intent.rs:28-45` | `SourceIntent::Rtmp` is tagged and contains `source_id` plus `endpoint`. | D11 preserves this wire shape; production authorization stays in the startup manifest. |
| `services/media-agent/src/pipeline.rs:61-77,179-193` | `SourcePlan::Network` currently carries the endpoint and `PipelinePlan` derives `Debug`/`Serialize`/`Deserialize`. | The implementation must keep backend input available without leaking endpoint literals through canonical plan observation/serialization; the ingress wire exception remains explicit. |
| `services/media-agent/src/pipeline.rs:653-705` | Network materialization currently calls `validate_loopback()` and places the endpoint into `SourcePlan::Network`; it has no production manifest exact-match or local-interface ownership check. | Add production admission before Session start; preserve loopback regression while widening only through D2–D5. |
| `services/media-agent/src/adapters/ffmpeg.rs:332-413` | The RTMP listener argv is adapter-owned, but production stderr is currently `Stdio::null()` and the input URL is materialized in the adapter. | Add bounded stderr reader/finite attribution and retain adapter ownership; no raw diagnostic enters canonical output. |
| `services/media-agent/src/adapters/ffmpeg.rs:416-430,510-604` | Child spawn, terminate/reap, recover and observe are owned by the FFmpeg backend; observe currently emits exit/error details. | Preserve Backend ownership while making bind/spawn/unknown/disconnect attribution finite and ensuring reader join/reap ordering. |
| `services/media-agent/src/recovery_monitor.rs:180-279` | The monitor maps backend detail into observation text, asks Supervisor for a restart, revalidates the lease, and recovers the same handle; the current path can carry detail text. | Keep the single RecoveryMonitor/Supervisor owner, but classify without raw stderr and add D7/D8 state/budget semantics. |
| `services/media-agent/src/session.rs:694-701` | `SourceMaterialized` uses `format!("{:?}", plan)` to derive a pipeline UUID. | D9 requires the hash/debug surface to be endpoint-independent and free of endpoint literals; no endpoint may become a hidden identity input. |

The audit appendix is intentionally limited to the live source locations that
drive this packet. It does not authorize changes outside the allowed scope.

## 9. Handoff

This document is the sole authority for `RF-SRC-RTMP-02-IMPLEMENTATION`. The
next implementation window starts at TG-0 and must stop before runtime code if
either capability probe fails. It must update `.project/STATE.md` only after
the exact implementation commit, software verification, declared BMD evidence
tier and 7/7 CI have been reconciled. No implementation is started by this
freeze commit.
