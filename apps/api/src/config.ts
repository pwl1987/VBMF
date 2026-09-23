/**
 * CP-01A 环境配置（显式默认值；未知键不猜测）。
 *
 * - `MEDIA_AGENT_RPC_URL`：media-agent internal Runtime Control 基址
 *   （canonical env 形态见 D3-R；standalone 默认 `http://127.0.0.1:50051`，
 *   compose 私网形态 `http://media-agent:50051`）。
 * - RPC path 固定 `/internal/v1/agent`（EXTERNAL_API_CONTRACT §1 internal
 *   命名空间；不提供覆盖——命名空间是契约不是配置）。
 */
export const AGENT_RPC_PATH = "/internal/v1/agent";

export interface AppConfig {
  port: number;
  host: string;
  mediaAgentRpcUrl: string;
  trustProxy: boolean;
  logLevel: string;
}

function intEnv(value: string | undefined, fallback: number): number {
  if (value === undefined || value === "") return fallback;
  const n = Number.parseInt(value, 10);
  return Number.isInteger(n) && n > 0 && n <= 65535 ? n : fallback;
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
  };
}
