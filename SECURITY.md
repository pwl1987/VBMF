# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| V0.2.x  | ✅ Active          |
| V0.1.x  | ❌ End of life     |

> VBMF is currently in pre-1.0 development (Phase 0.5 complete; Phase 0.6 next).
> Security updates will be applied to the latest V0.2.x branch.

## Reporting a Vulnerability

**Please DO NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them privately via:

- **Email**: [security@your-org.example.com](mailto:security@your-org.example.com) (replace with your org's address)
- **GitHub Security Advisories**: [github.com/\<org\>/VBMF/security/advisories/new](../../security/advisories/new)

### What to include

- **Vulnerability type** (e.g. SQL injection, XSS, RCE, auth bypass)
- **Affected component** (e.g. Media Agent, Graph Designer, REST API)
- **Affected versions** (V0.2.x)
- **Attack vector** (local / network / physical)
- **Reproduction steps** (minimal)
- **Impact assessment** (what an attacker could achieve)
- **Suggested fix** (if any)
- **Your name / handle** (for credit in the advisory, optional)

### Response timeline

| Stage | SLA |
|---|---|
| Acknowledgement | within 72 hours |
| Initial assessment | within 7 days |
| Fix / patch | within 30 days (critical), 90 days (high) |
| Public disclosure | coordinated with reporter |

## Scope

### In scope

- Media Agent (Rust) — once Phase 1 lands
- Backend (Fastify / Drizzle) — once Phase 2 lands
- Web Console (TypeScript) — once Phase 4 lands
- GraphSpec / GraphRuntime / Configuration Versioning data
- Health Tree data exposure / API access control
- BMDP/SDI / NDI / RIST / Zixi input handling
- FFmpeg command construction (avoid command injection)
- PostgreSQL / Valkey / SRS / RustFS deployment scripts

### Out of scope

- BMD Desktop Video SDK (third-party; report to Blackmagic Design)
- FFmpeg (third-party; report to FFmpeg project)
- SRS (third-party; report to ossrs/srs)
- Known limitations in V0.2 architecture doc (V0.2 LOCK FINAL; report as issues)

## Security Best Practices for Deployment

> 适用于 self-hosted 部署。

### Network

- UFW：仅开放 22 (SSH) + 必要管理端口
- SSH：仅 key 认证，禁用密码
- 媒体端口（SRS 1935/1985/8080）：限制来源 IP
- fail2ban：SSH jail 启用（参考 `docs/SYSTEM_AND_PROJECT_PLAN.md`）

### Host

- 操作系统硬化：`/etc/sysctl.d/99-hardening.conf`
- sudo 限定：passwordless 仅 `lytv` 用户
- 文件权限：`/opt` owner 限定

### Application

- 所有配置走 `change_set`（X3 Configuration Versioning），不要直接改库
- `secrets/` 目录权限 700
- API key 轮换周期：90 天
- 数据库密码使用强随机（≥ 32 字符）

### Secrets

- 禁止将 secrets 提交到 git
- 使用 `.env` + `.gitignore`（已配置）
- CI / CD 使用 GitHub Secrets / 部署环境变量

## Reporting Non-Security Bugs

请用 [GitHub Issues](../../issues) 报告普通 bug。安全相关务必用上述私有渠道。

## Acknowledgments

感谢所有负责任地披露安全问题的安全研究者。

---

**VBMF Contributors** · V0.2 LOCK FINAL · Apache 2.0
