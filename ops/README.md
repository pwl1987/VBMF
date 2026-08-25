# ops/ — VBMF V0.2 Deployment Artefacts

> **Scope discipline (Deployment SoT §0)**: This dir holds only static, no-code,
> self-hosted deployment contracts. **No auto-deploy / no cloud / no CI push to prod**.
> Runtime Ownership lives in `docs/TECHNOLOGY_STACK_AND_RUNTIME_OWNERSHIP.md`;
> this dir only expresses it as concrete compose/Dockerfile shape.

## Files
- `docker-compose.yml` — 8-service V0.2 stack: `db / cache / rustfs / srs / fastify / worker / web / media-agent`.
- `Dockerfile.*` — placeholder build contexts (Phase 1 wires real `src/`).
- `nginx/` — see `docs/architecture/DEPLOYMENT_AND_DEV_RUNTIME.md` §8 (Nginx Reverse Proxy Contract).

## Key contracts encoded in compose
1. **Readiness wait (INFRA-01)**: Fastify/Worker use `depends_on: condition: service_healthy`
   on `db/cache/rustfs/srs/media-agent` — NOT mere container start. PostgreSQL "started" ≠ "ready".
2. **Health Tree (Self-Healing §5)**: every service has a `healthcheck`; expose Recovery State
   (`NONE/RESTARTING/RECOVERED/RETRYING/BACKOFF/ESCALATED/MANUAL_REQUIRED`) via `/health/*`.
3. **Media Agent ownership (F2/F4/F11)**: sole DeckLink/GStreamer/Live FFmpeg owner.
   - `runtime: runsc` (gVisor), `cap_add: SYS_ADMIN`, `devices: /dev/blackmagic`.
   - Host-side ffmpeg MUST NOT hold DeckLink.
4. **Nginx = sole ingress** (`§8`): Fastify is NOT a media proxy; SRS owns RTMP/SRT/HLS/WHEP.

## G-RUNTIME gate
- `docker-compose.yml` is the **structure draft**. Real `docker compose up` requires:
  Phase 1 source trees (fastify/worker/web/media-agent), DeckLink SDK, and a BMD dev server.
- **Remote BMD Acceptance (§9) cannot run in GitHub CI** — must verify on real hardware.
- Env preflight (§6) must pass before `up`: Docker/compose/network/volume/healthcheck,
  containers UP, infra READY, SSH+driver+devices, media runtime, hot-reload, self-healing config.

## Usage (dev/on-prem, after Phase 1)
```bash
# from repo root
docker compose -f ops/docker-compose.yml up --build
```
