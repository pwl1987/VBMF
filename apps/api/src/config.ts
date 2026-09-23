/**
 * CP-01A/CP-01B 环境配置（显式默认值；未知键不猜测）。
 *
 * - `MEDIA_AGENT_RPC_URL`：media-agent internal Runtime Control 基址
 *   （canonical env 形态见 D3-R；standalone 默认 `http://127.0.0.1:50051`，
 *   compose 私网形态 `http://media-agent:50051`）。
 * - RPC path 固定 `/internal/v1/agent`（EXTERNAL_API_CONTRACT §1 internal
 *   命名空间；不提供覆盖——命名空间是契约不是配置）。
 * - CP-01B：`DATABASE_URL`（或 DATABASE_HOST 族拼接）启用 durable command
 *   plane；缺省 = 读 API 形态（command 面 503 RESOURCE_UNAVAILABLE）。
 */
export const AGENT_RPC_PATH = "/internal/v1/agent";

export interface AppConfig {
  port: number;
  host: string;
  mediaAgentRpcUrl: string;
  trustProxy: boolean;
  logLevel: string;
  /** null = 未配置持久层（command plane 诚实禁用）。 */
  databaseUrl: string | null;
  migrateOnBoot: boolean;
  /** pending 租约（毫秒）——超龄回收 timeout(retryable)（C6/F6）。 */
  commandLeaseMs: number;
  /** in-flight 重复提交短窗口等待（毫秒·C6）。 */
  commandReplayWaitMs: number;
}

function intEnv(value: string | undefined, fallback: number): number {
  if (value === undefined || value === "") return fallback;
  const n = Number.parseInt(value, 10);
  return Number.isInteger(n) && n > 0 && n <= 3_600_000 ? n : fallback;
}

function databaseUrlFrom(env: NodeJS.ProcessEnv): string | null {
  if (env.DATABASE_URL !== undefined && env.DATABASE_URL !== "") return env.DATABASE_URL;
  const host = env.DATABASE_HOST;
  if (host === undefined || host === "") return null;
  const user = env.DATABASE_USER ?? "vbmf";
  const password = env.DATABASE_PASSWORD ?? "";
  const port = env.DATABASE_PORT ?? "5432";
  const name = env.DATABASE_NAME ?? "vbmf";
  return `postgres://${encodeURIComponent(user)}:${encodeURIComponent(password)}@${host}:${port}/${name}`;
}

export function loadConfig(env: NodeJS.ProcessEnv = process.env): AppConfig {
  return {
    port: intEnv(env.PORT, 3000),
    host: env.HOST ?? "0.0.0.0",
    mediaAgentRpcUrl: (
      env.MEDIA_AGENT_RPC_URL ?? "http://127.0.0.1:50051"
    ).replace(/\/+$/, ""),
    trustProxy: env.TRUST_PROXY === "true",
    logLevel: env.LOG_LEVEL ?? "info",
    databaseUrl: databaseUrlFrom(env),
    migrateOnBoot: env.MIGRATE_ON_BOOT === "true",
    commandLeaseMs: intEnv(env.COMMAND_LEASE_MS, 30_000),
    commandReplayWaitMs: intEnv(env.COMMAND_REPLAY_WAIT_MS, 1_500),
  };
}
