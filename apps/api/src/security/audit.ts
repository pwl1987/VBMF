/**
 * CP-01C 安全审计写入（planning C10/C11）。
 *
 * - 命令旅程审计：CommandService.finalize 在命令终态**同事务**写入（CP-01B
 *   不变量保持——审计写失败 ⇒ 终态迁移回滚，绝不出现"已终态但无审计"）。
 * - 安全事件审计（本模块）：认证拒绝 / 授权拒绝 / 限流拒绝 / key 生命周期。
 *   拒绝路径的审计写入是 best-effort（失败只记 server log，不影响已生效的
 *   fail-closed 拒绝本身——审计可用性不得把安全拒绝变成放行或 500）。
 * - detail 永不含 credential/token/secret（安全红线；测试机检）。
 */
import type { Db } from "../db/index.ts";
import { auditEntries } from "../db/schema.ts";

export interface SecurityAuditEvent {
  /** who：认证主体 id；未认证尝试 = "anonymous"。 */
  principal: string;
  /** 决策时角色（未知为 null）。 */
  role: string | null;
  /** what：语义动作（session.start…）或安全事件（auth.reject…）。 */
  action: string;
  /** authorization decision。 */
  decision: "allowed" | "denied";
  /** 拒绝原因机读码。 */
  reason?: string | null;
  /** secret-free 结构化上下文（如 keyStart、bucket、路径）。 */
  detail?: Record<string, unknown> | null;
}

/** 安全事件 action 词表（命令面 action 由各 route 语义决定并复用同一表）。 */
export const SecurityAuditAction = {
  authReject: "auth.reject",
  authzDeny: "authz.deny",
  rateLimitReject: "ratelimit.reject",
  apiKeyCreate: "apikey.create",
  apiKeyRevoke: "apikey.revoke",
} as const;

export class SecurityAudit {
  private readonly db: Db;

  constructor(db: Db) {
    this.db = db;
  }

  /** 显式等待的安全审计（ provisioning 等可靠路径）。失败向上抛。 */
  async record(ev: SecurityAuditEvent): Promise<void> {
    await this.db.insert(auditEntries).values({
      principal: ev.principal,
      role: ev.role,
      action: ev.action,
      decision: ev.decision,
      reason: ev.reason ?? null,
      detail: ev.detail ?? null,
      commandId: null,
      state: null,
      classification: null,
    });
  }

  /** 请求路径上的 best-effort 审计：失败降级为 server log，绝不影响拒绝语义。 */
  async recordBestEffort(ev: SecurityAuditEvent, log: { warn: (o: object, m: string) => void }): Promise<void> {
    try {
      await this.record(ev);
    } catch (err) {
      log.warn({ err: (err as Error).message, action: ev.action, reason: ev.reason }, "security audit write failed");
    }
  }
}
