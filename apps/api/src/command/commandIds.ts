/**
 * 外部 command_id 规范化 —— 与 agent `transport::command_id_from_string`
 * 逐字节一致的确定性派生（合法 UUID 直用；否则 RFC 4122 v5，命名空间 =
 * COMMAND_ID_NAMESPACE `62b79f8c-1a2e-4c3d-9f0b-5d6e7a8b9c0d`）。
 *
 * 一致性是跨层幂等链的前提：PG Operation 记录、agent 进程内幂等表、
 * 审计行三处的 command_id 必须同值（C4/C6）。
 */
import { createHash, randomUUID } from "node:crypto";

const COMMAND_ID_NAMESPACE = "62b79f8c-1a2e-4c3d-9f0b-5d6e7a8b9c0d";
const CANONICAL_UUID =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

function uuidBytesToHex(bytes: Uint8Array): string {
  const hex = Buffer.from(bytes).toString("hex");
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20, 32),
  ].join("-");
}

function namespaceBytes(): Uint8Array {
  return Uint8Array.from(Buffer.from(COMMAND_ID_NAMESPACE.replace(/-/g, ""), "hex"));
}

/** RFC 4122 v5（SHA-1；uuid crate `Uuid::new_v5` 同算法同布局）。 */
export function uuidV5(name: string): string {
  const hash = createHash("sha1");
  hash.update(namespaceBytes());
  hash.update(Buffer.from(name, "utf8"));
  const digest = hash.digest();
  const bytes = Uint8Array.from(digest.subarray(0, 16));
  bytes[6] = (bytes[6]! & 0x0f) | 0x50; // version 5
  bytes[8] = (bytes[8]! & 0x3f) | 0x80; // RFC variant
  return uuidBytesToHex(bytes);
}

/** agent `command_id_from_string` 等价实现：UUID 直用，否则 v5 派生。 */
export function canonicalCommandId(raw: string): string {
  return CANONICAL_UUID.test(raw) ? raw : uuidV5(raw);
}

/** 服务器生成的新 command_id（canonical UUID）。 */
export function newCommandId(): string {
  return randomUUID();
}
