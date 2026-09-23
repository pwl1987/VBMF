/**
 * C6 fingerprint：什么算"同一个命令"。
 * D9-A 字段（kind + canonical target JSON，排除 issued_at/requested_by）
 * **+ 认证主体 id**——同键不同主体 = Conflict（防键跨主体劫持重放）。
 */
import { createHash } from "node:crypto";

/** target 的 canonical JSON 序列化（键排序、稳定分隔符）。 */
export function canonicalTargetJson(target: unknown): string {
  return JSON.stringify(sortDeep(target));
}

function sortDeep(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortDeep);
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>)
        .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
        .map(([k, v]) => [k, sortDeep(v)]),
    );
  }
  return value;
}

export function commandFingerprint(
  kind: string,
  target: unknown,
  principal: string,
): string {
  return createHash("sha256")
    .update("vbmf.command.v1\n")
    .update(`kind:${kind}\n`)
    .update(`target:${canonicalTargetJson(target)}\n`)
    .update(`principal:${principal}`)
    .digest("hex");
}
