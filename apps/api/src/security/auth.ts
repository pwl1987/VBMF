/**
 * CP-01C AuthN：Better Auth + 官方 api-key plugin（EXTERNAL_API_CONTRACT §6 /
 * planning C11——Better Auth 是 AuthN identity owner）。
 *
 * - 生产路径只消费 `x-api-key` header；`x-dev-principal` 开发态桩已删除，
 *   生产无 dev bypass（测试 identity 只能经显式注入的 Authenticator fixture）。
 * - key 明文只在请求瞬时内存中存在：库内是官方 SHA-256 摘要（defaultKeyHasher），
 *   响应/日志/审计永不回显。
 * - Better Auth 实例只以库形态使用（auth.api.*，不挂载 HTTP handler）——
 *   Fastify 保持 enforcement owner；本模块不新增第二套 crypto。
 * - 插件内建 per-key rate limit 关闭：应用层语义限流统一由 rateLimit.ts
 *   （principal × action bucket）承担，避免双层计数语义漂移。
 */
import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { apiKey } from "@better-auth/api-key";
import { eq } from "drizzle-orm";
import type { Db } from "../db/index.ts";
import { authUser, authSession, authAccount, authVerification, apiKeys } from "../db/schema.ts";
import type { Principal } from "./principal.ts";

export const API_KEY_HEADER = "x-api-key";

/** verifyApiKey 的机读失败原因（进审计 reason；HTTP 响应保持通用 401 语义）。 */
export type AuthFailureReason = "invalid" | "disabled" | "expired" | "unknown_identity";

export type VerifyResult =
  | { ok: true; principal: Principal }
  | { ok: false; reason: AuthFailureReason };

/**
 * 可注入的认证边界：生产实现 BetterAuthAuthenticator；测试用显式 fixture
 * （禁止经 header/env 复活任何生产 dev bypass）。
 */
export interface Authenticator {
  verifyApiKey(key: string): Promise<VerifyResult>;
}

export interface AuthDeps {
  db: Db;
  /** BETTER_AUTH_SECRET（fail-closed：缺失则不构建 auth 实例 → 全部 /api/v1 503）。 */
  secret: string;
}

export function createAuth(deps: AuthDeps) {
  return betterAuth({
    secret: deps.secret,
    database: drizzleAdapter(deps.db, {
      provider: "pg",
      schema: {
        user: authUser,
        session: authSession,
        account: authAccount,
        verification: authVerification,
        apikey: apiKeys,
      },
    }),
    user: {
      additionalFields: {
        role: { type: "string", required: false, input: true },
      },
    },
    plugins: [
      apiKey({
        defaultPrefix: "vbmf_",
        enableMetadata: false,
        rateLimit: { enabled: false },
      }),
    ],
  });
}

interface VerifyApiKeyResponse {
  valid: boolean;
  error: { message?: string; code?: string } | null;
  key: {
    id: string;
    name: string | null;
    start: string | null;
    referenceId: string;
  } | null;
}

export class BetterAuthAuthenticator implements Authenticator {
  private readonly auth: ReturnType<typeof createAuth>;
  private readonly db: Db;

  constructor(auth: ReturnType<typeof createAuth>, db: Db) {
    this.auth = auth;
    this.db = db;
  }

  async verifyApiKey(key: string): Promise<VerifyResult> {
    const res = (await this.auth.api.verifyApiKey({ body: { key } })) as VerifyApiKeyResponse;
    if (!res.valid || res.key === null) {
      return { ok: false, reason: mapFailureReason(res.error?.code) };
    }
    const referenceId = res.key.referenceId;
    let userRole: string | null | undefined;
    try {
      const [row] = await this.db
        .select({ role: authUser.role })
        .from(authUser)
        .where(eq(authUser.id, referenceId));
      userRole = row?.role;
    } catch (err) {
      // 身份查找的 DB 故障 = auth backend unavailable（调用方映射 503 fail-closed）。
      throw new AuthBackendUnavailableError((err as Error).message);
    }
    if (userRole === undefined) {
      // key 有效但属主身份不存在（用户被删除）→ 认证失败（fail-closed）。
      return { ok: false, reason: "unknown_identity" };
    }
    return {
      ok: true,
      principal: {
        userId: referenceId,
        role: userRole ?? "",
        keyId: res.key.id,
        keyName: res.key.name,
        keyStart: res.key.start,
      },
    };
  }
}

function mapFailureReason(code: string | undefined): AuthFailureReason {
  switch (code) {
    case "KEY_EXPIRED":
      return "expired";
    case "KEY_DISABLED":
      return "disabled";
    default:
      return "invalid";
  }
}

/** auth backend（PostgreSQL/Better Auth 层）不可用——fail-closed 语义由调用方映射 503。 */
export class AuthBackendUnavailableError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "AuthBackendUnavailableError";
  }
}
