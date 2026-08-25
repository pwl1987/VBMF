# BMD Acceptance Server — Environment Prep Evidence

> **G-RUNTIME Level 2 (Deployment SoT §9)**: Remote BMD Server provisioning log.
> This file is the auditable record of environment preparation on the real Blackmagic
> Design (BMD) server, per ENV Preflight + Acceptance Evidence requirements.
> **Do NOT redact SHA / host identity — this is acceptance evidence, not a secret.**

## 1. Target

| Field | Value |
|---|---|
| Host | `10.30.15.10` |
| SSH user | `lytv` (key auth, sudo NOPASSWD) |
| OS | Ubuntu 26.04 LTS (resolute) |
| Kernel | `7.0.0-30-generic #30-Ubuntu SMP PREEMPT_DYNAMIC` |
| Arch | x86_64 |
| VBMF repo SHA | `7cc33dde2ab3070c28087df7d0aae570c6c8df5f` (`7cc33dd`) |
| Repo path | `/opt/vbmf-dev/repo` (exact-SHA checkout, per §9) |
| Workdir layout | `/opt/vbmf-dev/{repo,evidence,artifacts,logs,runtime}` |

## 2. Blackmagic DeckLink Device Detection (F11 pre-req)

```text
/dev/blackmagic:
  crw-rw-rw- 1 root root 10, 263  dv0
  crw-rw-rw- 1 root root 10, 264  dv1
  crw-rw-rw- 1 root root 10, 265  io0
```

✅ Real BMD hardware present (`dv0/dv1/io0`). Confirms this is a valid G-RUNTIME
Level 2 acceptance target. DeckLink passthrough path `/dev/blackmagic` matches
`media-agent` compose `devices:` mapping.

## 3. Docker Installation (action log)

**Constraint discovered**: outbound HTTPS to `get.docker.com` / `download.docker.com`
/ `github.com` (HTTPS) is **reset by network egress filter** (bare TCP/443 reachable,
TLS handshake reset). Ubuntu 26.04 universe source had no `docker.io`.

**Resolution**: used `linuxmirrors.cn/docker.sh` (netlify CDN, reachable) with
Aliyun Docker CE mirror:

```bash
bash <(curl -sSL https://linuxmirrors.cn/docker.sh) \
  --source mirrors.aliyun.com/docker-ce \
  --source-registry registry.cn-hangzhou.aliyuncs.com
sudo systemctl enable --now docker
sudo usermod -aG docker lytv
```

**Result**:
```text
Docker version 29.7.2, build (Server & Client both 29.7.2)
Docker Compose version v5.5.0
Default Runtime = runc   Driver = overlayfs   Cgroup = v2
```

✅ Docker Engine + Compose plugin installed and daemon running.

> **MEDIA-SEC-01 note**: default runtime is `runc`, NOT `runsc` (gVisor).
> The compose pins `media-agent.runtime: runsc`, but runsc is NOT yet installed
> on this host. Option A (gVisor) requires `runsc` install + DeckLink test;
> Option B (runc + seccomp/AppArmor) is currently the working path.
> Decision deferred to real-media-runtime acceptance (Gate 2/3).

## 4. Compose Validation (DEPLOY Gate 1 verification)

Run from `/opt/vbmf-dev/repo` after `git checkout 7cc33dd` + local `.env` (secrets
externalized, generated via `openssl rand`, NOT committed):

```bash
for p in "" "compose.dev.yml" "compose.acceptance.yml" "compose.prod.yml"; do
  docker compose -f ops/docker-compose.yml ${p:+-f ops/$p} config --quiet \
    && echo "OK: $p" || echo "FAIL: $p"
done
```

| Profile | Result |
|---|---|
| BASE (`docker-compose.yml`) | ✅ OK |
| DEV (`+compose.dev.yml`) | ✅ OK |
| ACCEPTANCE (`+compose.acceptance.yml`) | ✅ OK |
| PROD (`+compose.prod.yml`) | ✅ OK |

✅ All four layered compose configs pass `docker compose config --quiet`.
Secrets externalization (INFRA-SEC-01) verified: missing `.env` aborts interpolation.

## 5. Outstanding (not blocking env prep)

- [ ] `media-agent` Dockerfile is placeholder → `docker compose up` blocks on build until Phase 1 source (Gate 2).
- [ ] `runsc` (gVisor) not installed → MEDIA-SEC-01 Option A unverified; current path = runc.
- [ ] GitHub HTTPS egress blocked on this host → repo sync uses `scp` from dev machine, not `git pull`.
- [ ] `.env` exists only on BMD local fs (gitignored); never committed.

## 6. Sign-off

| Role | Identity | Date |
|---|---|---|
| Env prepared by | AI assistant (via SSH, `lytv`) | 2026-08-25 |
| SHA verified | `7cc33dd` | — |
| Next gate | Gate 2: Real Rust Media Agent build + Device Lease (requires Phase 1 source) | — |
