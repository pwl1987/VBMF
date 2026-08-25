# ops/ — VBMF V0.2 Deployment Artefacts

> **Scope discipline (Deployment SoT §0)**: This dir holds only static, no-code,
> self-hosted deployment contracts. **No auto-deploy / no cloud / no CI push to prod**.
> Runtime Ownership lives in `docs/TECHNOLOGY_STACK_AND_RUNTIME_OWNERSHIP.md`;
> this dir only expresses it as concrete compose/Dockerfile shape.

## Files
- `docker-compose.yml` — **BASE**: 9 services (`db/cache/rustfs/srs/fastify/worker/web/media-agent/nginx`),
  internal `expose` only, secrets via `${}` from `.env`, Nginx as sole ingress.
- `compose.dev.yml` / `compose.acceptance.yml` / `compose.prod.yml` — **profile overlays**
  (FLOW-01 / §11): HMR+bind-mount / pinned+BMD media ports / immutable+Nginx-443-only.
- `Dockerfile.*` — placeholder build contexts (Phase 1 wires real `src/`).
- `nginx/default.conf` — Nginx ingress routes (`/api /ws /events /ops /admin /health` → fastify; `/` → web).
- `.env.example` — secrets template; real `.env` is gitignored (INFRA-SEC-01).

## Key contracts encoded (Deployment SoT)
1. **Readiness wait (INFRA-01)**: `depends_on: condition: service_healthy` on deps — not mere start.
2. **Health Tree (§5)**: every service has `healthcheck`; Nginx adds `/health/nginx`.
3. **Media Agent ownership (F2/F4/F11)**: `runtime: runc` (MEDIA-SEC-01 裁决 Option B) + `cap_add: SYS_ADMIN` + `/dev/blackmagic`.
   DeckLink 设备发现依赖宿主 `DesktopVideoHelper` IPC：部署须 bind `/dev/shm/com_blackmagicdesign_DeckLinkDiscoveryNotifier` + `--ipc=host`
   （compose 不支持 inline shm bind，由部署脚本/daemon 保证）。Host ffmpeg MUST NOT hold DeckLink。
   gVisor `runsc` 因 Step 3 实测枚举失败已否决（证据 2026-08-26-media-sec-01-step3.md）。
4. **Nginx = sole ingress (§8)**: internal services use `expose`, NOT host `ports` (DEPLOY-02).
   SRS owns media protocol plane (RTMP/SRT/HLS/WHEP), published only via profile overlays.
5. **Web same-origin (DEPLOY-UI-01)**: `VITE_API_BASE=/api` — never `localhost` (breaks remote browser).
6. **Secrets externalized (INFRA-SEC-01)**: no hardcoded passwords; `.env` gitignored.
7. **Image pins (DEPLOY-BASELINE-01)**: specific patch versions; SRS `6.0.42`; no `latest`.

## G-RUNTIME gate
- Base compose is the **structure draft**. Real `up` requires Phase 1 source, DeckLink SDK, BMD server.
- **Remote BMD Acceptance (§9) cannot run in GitHub CI** — verify on real hardware.
- Env preflight (§6) must pass before `up`.

## Usage (after Phase 1)
```bash
# DEV (HMR, bind mount, publish 5173/3000)
docker compose -f ops/docker-compose.yml -f ops/compose.dev.yml up --build

# ACCEPTANCE (pinned SHA, real BMD, Nginx 443 + SRS media ports)
docker compose -f ops/docker-compose.yml -f ops/compose.acceptance.yml up --build

# PROD (immutable, Nginx 443 only, no HMR)
docker compose -f ops/docker-compose.yml -f ops/compose.prod.yml up --build -d
```
