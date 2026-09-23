/**
 * PG 连接与迁移（CP-01B）。DB 是控制面持久记录载体，不是 Runtime truth
 * （C10）。standalone lane 不运行 Fastify，本模块仅在 full-stack/dev 存在。
 */
import pg from "pg";
import { drizzle, type NodePgDatabase } from "drizzle-orm/node-postgres";
import { migrate } from "drizzle-orm/node-postgres/migrator";
import * as schema from "./schema.ts";

export type Db = NodePgDatabase<typeof schema>;

export function createDb(databaseUrl: string): { db: Db; pool: pg.Pool } {
  const pool = new pg.Pool({ connectionString: databaseUrl, max: 10 });
  return { db: drizzle(pool, { schema }), pool };
}

export async function runMigrations(db: Db): Promise<void> {
  await migrate(db, { migrationsFolder: new URL("../../drizzle/", import.meta.url).pathname });
}
