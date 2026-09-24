/**
 * DB 层测试 helper：经 Better Auth 官方路径 provision 测试身份
 * （user 行直插——无密码 credential 的纯 identity 记录；API key 走官方
 * auth.api.createApiKey，库内 SHA-256 摘要持久化）。
 */
import { randomUUID } from "node:crypto";
import type { Db } from "../../src/db/index.ts";
import { authUser } from "../../src/db/schema.ts";

export interface ProvisionedIdentity {
  userId: string;
  apiKey: string;
  keyId: string;
  role: string;
}

export async function provisionIdentity(
  db: Db,
  auth: { api: { createApiKey(args: { body: { userId: string; name: string } }): Promise<{ key: string; id: string }> } },
  opts: { role: string; email?: string; name?: string },
): Promise<ProvisionedIdentity> {
  const userId = randomUUID();
  const email = opts.email ?? `${opts.role}-${userId}@fixture.local`;
  await db.insert(authUser).values({
    id: userId,
    name: opts.name ?? `${opts.role} fixture`,
    email,
    emailVerified: true,
    role: opts.role,
    createdAt: new Date(),
    updatedAt: new Date(),
  });
  const created = await auth.api.createApiKey({ body: { userId, name: `${opts.role} test key` } });
  return { userId, apiKey: created.key, keyId: created.id, role: opts.role };
}
