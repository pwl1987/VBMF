/**
 * CP-01C 认证主体。来源 = Better Auth API key 验证（AuthN identity owner）
 * ——`x-dev-principal` 开发态桩已在 CP-01C 从生产路径删除（planning C11）。
 *
 * principal.userId 是 command fingerprint 的主体输入（C6：同键不同主体 =
 * Conflict）；role 是 CASL authz 输入，未知/缺失角色在 authz 层 fail-closed。
 */
export interface Principal {
  /** Better Auth user id（= api_keys.reference_id；fingerprint 组成部分）。 */
  readonly userId: string;
  /** 原始角色串；未知角色不由本层拦截，由 authz 层统一 fail-closed 拒绝。 */
  readonly role: string;
  /** api_keys 行 id（审计/追溯用；非 credential）。 */
  readonly keyId: string;
  /** key 显示名（可空）。 */
  readonly keyName: string | null;
  /** 官方 `start` 前缀标识（非 secret，官方设计供运营识别）。 */
  readonly keyStart: string | null;
}

export const ANONYMOUS_PRINCIPAL = "anonymous";
