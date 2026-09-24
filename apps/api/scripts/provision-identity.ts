/**
 * CP-01C identity provisioning 工具（运营面，非 HTTP surface）。
 *
 * Better Auth 是 AuthN identity owner：API key 一律经官方
 * `auth.api.createApiKey`（库内 SHA-256 摘要持久化，明文只在创建瞬时
 * stdout 输出一次）；user 行为纯 identity 记录（无密码 credential），
 * 由本工具直接写入 auth_user。revoke = `enabled=false`（验证路径即时生效）。
 * 所有 key 生命周期操作写审计（who/what/when/decision，无 secret）。
 *
 * 用法（DATABASE_URL + BETTER_AUTH_SECRET 必须已配置）：
 *   node scripts/provision-identity.ts create --email ops@example.com \
 *     --name "Broadcast Ops" --role operator [--expire-days 30]
 *   node scripts/provision-identity.ts revoke --key-id <api_keys.id>
 *
 * 本工具不做任何媒体操作（F12/Gate A 红线适用于 apps/api 全部源码）。
 */
import { randomUUID } from "node:crypto";
import { eq } from "drizzle-orm";
import { loadConfig } from "../src/config.ts";
import { createDb, runMigrations } from "../src/db/index.ts";
import { authUser, apiKeys } from "../src/db/schema.ts";
import { createAuth } from "../src/security/auth.ts";
import { SecurityAudit } from "../src/security/audit.ts";

interface Args {
  command: "create" | "revoke";
  email?: string;
  name?: string;
  role?: string;
  expireDays?: number;
  keyId?: string;
}

function parseArgs(argv: string[]): Args {
  const [command, ...rest] = argv;
  if (command !== "create" && command !== "revoke") {
    fail("usage: provision-identity.ts <create|revoke> [options]");
  }
  const args: Args = { command };
  for (let i = 0; i < rest.length; i += 2) {
    const flag = rest[i];
    const value = rest[i + 1];
    if (flag === undefined || value === undefined) fail(`missing value for ${flag ?? "?"}`);
    switch (flag) {
      case "--email": args.email = value; break;
      case "--name": args.name = value; break;
      case "--role": args.role = value; break;
      case "--expire-days": {
        const n = Number.parseInt(value, 10);
        if (!Number.isInteger(n) || n <= 0) fail("--expire-days must be a positive integer");
        args.expireDays = n;
        break;
      }
      case "--key-id": args.keyId = value; break;
      default: fail(`unknown flag: ${flag}`);
    }
  }
  return args;
}

function fail(message: string): never {
  console.error(message);
  process.exit(2);
}

const VALID_ROLES = new Set(["viewer", "operator"]);

async function main(): Promise<void> {
  const args = parseArgs(process.argv.slice(2));
  const config = loadConfig();
  if (config.databaseUrl === null) fail("DATABASE_URL / DATABASE_HOST must be configured");
  if (config.authSecret === null) fail("BETTER_AUTH_SECRET must be configured");

  const { db, pool } = createDb(config.databaseUrl);
  await runMigrations(db);
  const auth = createAuth({ db, secret: config.authSecret });
  const audit = new SecurityAudit(db);

  try {
    if (args.command === "create") {
      const email = args.email;
      const role = args.role;
      const name = args.name ?? email ?? "";
      if (email === undefined || email.length === 0) fail("--email is required for create");
      if (role === undefined || !VALID_ROLES.has(role)) {
        fail(`--role must be one of: ${[...VALID_ROLES].join(", ")} (fail-closed: unknown roles deny everything)`);
      }
      const existing = await db.select({ id: authUser.id }).from(authUser).where(eq(authUser.email, email));
      let userId: string;
      if (existing.length > 0) {
        userId = existing[0]!.id;
        console.log(`user exists: ${userId} (${email})`);
      } else {
        userId = randomUUID();
        await db.insert(authUser).values({
          id: userId,
          name,
          email,
          emailVerified: true,
          role,
          createdAt: new Date(),
          updatedAt: new Date(),
        });
      }
      const created = (await auth.api.createApiKey({
        body: {
          userId,
          name: name.slice(0, 32) || `${email} key`,
          ...(args.expireDays !== undefined
            ? { expiresIn: args.expireDays * 24 * 60 * 60 }
            : {}),
        },
      })) as { key: string; id: string; start: string | null };
      await audit.record({
        principal: userId,
        role,
        action: "apikey.create",
        decision: "allowed",
        detail: { key_id: created.id, key_start: created.start, expire_days: args.expireDays ?? null },
      });
      // 明文 key 只此一次输出（stdout）；此后任何路径都无法再取回。
      console.log(`key_id: ${created.id}`);
      console.log(`api_key: ${created.key}`);
      return;
    }

    // revoke
    const keyId = args.keyId;
    if (keyId === undefined || keyId.length === 0) fail("--key-id is required for revoke");
    const [row] = await db.select().from(apiKeys).where(eq(apiKeys.id, keyId));
    if (row === undefined) fail(`api key not found: ${keyId}`);
    await db.update(apiKeys).set({ enabled: false, updatedAt: new Date() }).where(eq(apiKeys.id, keyId));
    await audit.record({
      principal: row.referenceId,
      role: null,
      action: "apikey.revoke",
      decision: "allowed",
      detail: { key_id: row.id, key_start: row.start },
    });
    console.log(`revoked: ${row.id}`);
  } finally {
    await pool.end();
  }
}

await main();
