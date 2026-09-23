/**
 * CP-01B 开发态鉴权桩（CP-01C 换 Better Auth + CASL；本包 Forbidden 真实
 * Auth 集成）。principal 参与 fingerprint（C6：同键不同主体 = Conflict）。
 *
 * 契约：显式 dev-stub——header `x-dev-principal`（≤128 字符），缺省
 * `dev-operator`。绝不作为生产鉴权语义。
 */
import { validationError } from "../lib/errors.ts";

export const DEV_PRINCIPAL_HEADER = "x-dev-principal";
const DEFAULT_PRINCIPAL = "dev-operator";

export function resolveDevPrincipal(headers: Record<string, unknown>): string {
  const raw = headers[DEV_PRINCIPAL_HEADER];
  if (raw === undefined) return DEFAULT_PRINCIPAL;
  if (typeof raw !== "string") {
    throw validationError(`${DEV_PRINCIPAL_HEADER} must be a single string`);
  }
  const value = raw.trim();
  if (value.length === 0 || value.length > 128) {
    throw validationError(`${DEV_PRINCIPAL_HEADER} must be 1..128 characters`);
  }
  return value;
}
