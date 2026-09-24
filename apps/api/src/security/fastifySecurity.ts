/**
 * CP-01C Fastify 安全 enforcement（EXTERNAL_API_CONTRACT §6 默认安全模型：
 * 默认 authenticated / authorized / audited / rate-limited；Nginx 只做
 * TLS/路由/IP filter/粗限流，业务 authz 全在本层——planning C11）。
 *
 * 每条 `/api/v1/*` 路由必须声明 `config.security`（permission + bucket），
 * 否则请求时 500 fail-closed（配置缺陷绝不隐式放行）；`/health/*`、
 * `/healthz` 是显式登记的无认证运维探针例外（基础设施 liveness/readiness，
 * 由 Nginx 层做 IP 过滤）。
 *
 * preHandler 顺序（Fastify 同阶段按注册顺序执行）：
 *   authn → authz → rate limit → handler
 * 拒绝路径（401/403/429/503）全部：错误模型 envelope + 审计记录 + 绝不进入
 * handler（ ⇒ 绝不 claim 幂等、绝不 dispatch agent——C4"4xx 不占幂等"）。
 */
import type { FastifyInstance, FastifyRequest } from "fastify";
import { API_KEY_HEADER, AuthBackendUnavailableError, type Authenticator } from "./auth.ts";
import { ANONYMOUS_PRINCIPAL } from "./principal.ts";
import { authorizationDecision, ROUTE_PERMISSIONS, type PermissionSpec } from "./rbac.ts";

export type { PermissionSpec };
import { SlidingWindowRateLimiter } from "./rateLimit.ts";
import { SecurityAudit, SecurityAuditAction } from "./audit.ts";
import {
  authenticationFailed,
  authorizationDenied,
  dependencyUnavailable,
  rateLimited,
} from "../lib/errors.ts";

export interface RouteSecurityConfig {
  /** 该路由的 Product API (action, resource) 语义。 */
  permission: PermissionSpec;
  /** 限流桶（read/write……限额由 limiter 配置定义；未定义桶 fail-closed）。 */
  bucket: string;
}

declare module "fastify" {
  interface FastifyContextConfig {
    security?: RouteSecurityConfig;
  }
  interface FastifyRequest {
    /** authn 通过后写入；否则保持 null。 */
    principal: RequestPrincipal | null;
  }
}

/** route 层消费的最小主体视图（测试 fixture 与生产实现共同形态）。 */
export type RequestPrincipal = {
  userId: string;
  role: string;
  keyId: string;
  keyName: string | null;
  keyStart: string | null;
};

export interface SecurityDeps {
  authenticator: Authenticator | null;
  limiter: SlidingWindowRateLimiter | null;
  audit: SecurityAudit | null;
}

/** 供 routes/server 引用的语义映射再导出（唯一 RBAC 输入形态）。 */
export { ROUTE_PERMISSIONS };

function extractApiKey(headers: FastifyRequest["headers"]): string | null {
  const raw = headers[API_KEY_HEADER];
  if (typeof raw !== "string") return null;
  const trimmed = raw.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function registerSecurity(app: FastifyInstance, deps: SecurityDeps): void {
  app.decorateRequest("principal", null);

  app.addHook("preHandler", async (req, reply) => {
    const config = req.routeOptions.config as { security?: RouteSecurityConfig };
    const url = req.routeOptions.url ?? "";

    if (config.security === undefined) {
      // 未声明 security 的 /api/v1/* 路由 = 配置缺陷 → fail-closed 500。
      if (url.startsWith("/api/v1/")) {
        throw new Error(`security misconfiguration: /api/v1 route without security config: ${url}`);
      }
      // 非产品面（/health/*、/healthz）——显式登记的无认证例外。
      return;
    }
    if (deps.authenticator === null) {
      // auth 层未配置（缺 DB/secret）→ 全部 /api/v1/* fail-closed 503。
      throw dependencyUnavailable("authentication backend is not configured on this control plane");
    }
    const security = config.security;

    // 1) authn
    const key = extractApiKey(req.headers);
    if (key === null) {
      await auditDeny(deps, req, {
        action: SecurityAuditAction.authReject,
        reason: "auth.missing_credentials",
        detail: { path: url, method: req.method },
      });
      throw authenticationFailed();
    }
    let verified: Awaited<ReturnType<typeof deps.authenticator.verifyApiKey>>;
    try {
      verified = await deps.authenticator.verifyApiKey(key);
    } catch (err) {
      if (err instanceof AuthBackendUnavailableError) {
        // auth backend/db 不可用 → 503 fail-closed（retryable），绝不假成功。
        await auditDeny(deps, req, {
          action: SecurityAuditAction.authReject,
          reason: "auth.backend_unavailable",
          detail: { path: url, method: req.method },
        });
        throw dependencyUnavailable("authentication backend is unavailable");
      }
      throw err;
    }
    if (!verified.ok) {
      await auditDeny(deps, req, {
        action: SecurityAuditAction.authReject,
        reason: `auth.${verified.reason}`,
        // keyStart = 官方 start 标识同构（前 6 字符含 "vbmf_" 前缀；非 secret，
        // 供运营归因错误配置的客户端）。绝不记录完整 key。
        detail: { path: url, method: req.method, keyStart: key.slice(0, 6) },
      });
      // 响应保持通用语义：不区分 invalid/disabled/expired，不回显 key 材料。
      throw authenticationFailed("invalid or expired credentials");
    }
    const principal = verified.principal;
    req.principal = principal;

    // 2) authz（CASL；fail-closed——未知角色/未知 action-resource 无规则可放行）
    const decision = authorizationDecision(principal.role, security.permission);
    if (!decision.allowed) {
      await auditDeny(deps, req, {
        principal,
        action: SecurityAuditAction.authzDeny,
        reason: "authz.denied",
        detail: {
          path: url,
          method: req.method,
          requested_action: security.permission.action,
          requested_resource: security.permission.resource,
          role: principal.role,
        },
      });
      throw authorizationDenied();
    }

    // 3) rate limit（principal × bucket；拒绝绝不占幂等、绝不进 handler）
    if (deps.limiter !== null) {
      const verdict = deps.limiter.check(security.bucket, principal.userId);
      if (!verdict.allowed) {
        const retryAfterSec = Math.max(1, Math.ceil(verdict.retryAfterMs / 1000));
        reply.header("retry-after", String(retryAfterSec));
        await auditDeny(deps, req, {
          principal,
          action: SecurityAuditAction.rateLimitReject,
          reason: "ratelimit.exceeded",
          detail: { path: url, method: req.method, bucket: security.bucket, retry_after_s: retryAfterSec },
        });
        throw rateLimited();
      }
    }
  });
}

type DenyInput = {
  principal?: RequestPrincipal;
  action: string;
  reason: string;
  detail: Record<string, unknown>;
};

async function auditDeny(deps: SecurityDeps, req: FastifyRequest, input: DenyInput): Promise<void> {
  if (deps.audit === null) return;
  const principal = input.principal;
  await deps.audit.recordBestEffort(
    {
      principal: principal?.userId ?? ANONYMOUS_PRINCIPAL,
      role: principal?.role ?? null,
      action: input.action,
      decision: "denied",
      reason: input.reason,
      detail: input.detail,
    },
    req.log,
  );
}
