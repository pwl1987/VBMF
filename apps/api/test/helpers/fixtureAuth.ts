/**
 * CP-01C 测试 fixture：显式注入的 Authenticator 测试替身（planning C11——
 * synthetic identity 只允许走显式测试注入；生产代码路径不存在任何 dev bypass）。
 */
import { AuthBackendUnavailableError, type Authenticator, type AuthFailureReason, type VerifyResult } from "../../src/security/auth.ts";
import type { Principal } from "../../src/security/principal.ts";

export function fixturePrincipal(userId: string, role: string): Principal {
  return {
    userId,
    role,
    keyId: `fixture-key-${userId}`,
    keyName: `fixture key for ${userId}`,
    keyStart: "vbmf_",
  };
}

export class FixtureAuthenticator implements Authenticator {
  private readonly entries = new Map<string, VerifyResult>();
  /** 模拟 auth backend（DB/Better Auth 层）不可用。 */
  failMode: "none" | "unavailable" = "none";

  register(key: string, principal: Principal): void {
    this.entries.set(key, { ok: true, principal });
  }

  registerFailure(key: string, reason: AuthFailureReason): void {
    this.entries.set(key, { ok: false, reason });
  }

  async verifyApiKey(key: string): Promise<VerifyResult> {
    if (this.failMode === "unavailable") {
      throw new AuthBackendUnavailableError("fixture: auth backend unavailable");
    }
    const entry = this.entries.get(key);
    if (entry === undefined) return { ok: false, reason: "invalid" };
    return entry;
  }
}
