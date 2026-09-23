/**
 * F13（2026-09-23 责任归属修正）：session id 两个方向、两种错误语义。
 *
 * - **客户端输入**（Product API 查询参数等）：只接受 canonical UUID 形态
 *   （C9：URL/响应统一 UUID 形态）；非法 → 4xx VALIDATION_ERROR。
 * - **agent wire 返回**（`runtime.query` 快照内 `session-<hex32>` 显示形态，
 *   见 session.rs `Display`）：非法 → Fastify↔agent 内部契约违例，
 *   5xx INTERNAL_ERROR——绝不 4xx、绝不猜测、绝不生成伪 UUID、
 *   绝不把 malformed upstream 当成"没有 session"。
 */
const WIRE_SESSION_ID = /^session-([0-9a-f]{32})$/;
const CANONICAL_UUID =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/** agent wire 非法 session id（内部契约违例；调用方映射为 5xx）。 */
export class AgentWireSessionIdError extends Error {
  readonly rawId: unknown;
  constructor(rawId: unknown) {
    super("media-agent returned a session id that is not the session-<hex32> wire form");
    this.name = "AgentWireSessionIdError";
    this.rawId = rawId;
  }
}

/** 客户端输入非法 UUID（调用方映射为 400 VALIDATION_ERROR）。 */
export class ClientSessionIdError extends Error {
  readonly rawId: string;
  constructor(rawId: string) {
    super("session_id must be a canonical UUID");
    this.name = "ClientSessionIdError";
    this.rawId = rawId;
  }
}

/** `session-<hex32>` → canonical dashed UUID（`8-4-4-4-12` 小写）。 */
export function wireSessionIdToUuid(rawId: unknown): string {
  if (typeof rawId !== "string") throw new AgentWireSessionIdError(rawId);
  const m = WIRE_SESSION_ID.exec(rawId);
  if (m === null) throw new AgentWireSessionIdError(rawId);
  const hex = m[1]!;
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20, 32),
  ].join("-");
}

/** canonical UUID 校验（客户端输入唯一接受形态）。 */
export function assertCanonicalUuid(rawId: string): string {
  if (!CANONICAL_UUID.test(rawId)) throw new ClientSessionIdError(rawId);
  return rawId;
}
