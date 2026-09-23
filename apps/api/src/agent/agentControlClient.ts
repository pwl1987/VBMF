/**
 * AgentControlClient —— Fastify → media-agent 唯一 internal transport adapter
 * （CONTROL-PLANE-ENTRY-01 C2；RCE-01A `/internal/v1/agent` JSON-RPC 2.0 面）。
 *
 * 职责（封闭）：JSON-RPC envelope 构造、per-method 超时预算（query 类 ≤3s、
 * dispatch ≤10s）、HTTP/网络失败、JSON-RPC error、malformed upstream response、
 * result 形状校验。四方法之外不发明方法；Fastify 各 route 禁止自行 fetch。
 *
 * 失败分类 → EXTERNAL_API_CONTRACT §5 映射（`mapAgentFailure`）：
 * - `network`（连接拒绝/DNS/超时）→ DEPENDENCY_UNAVAILABLE 503 retryable
 * - `http-status`（含 agent 诚实 503 unconfigured）→ DEPENDENCY_UNAVAILABLE 503 retryable
 * - `malformed-body` / `rpc-error` / `invalid-result` → INTERNAL_ERROR 500
 *   （Fastify↔agent 内部契约违例；诊断只进 server log，不泄漏给客户端）
 */
import { randomUUID } from "node:crypto";
import type {
  AgentCommandDispatchParams,
  AgentHealthWire,
  AgentProjectionWire,
  AgentQuerySnapshotWire,
} from "./types.ts";
import { AGENT_RPC_PATH } from "../config.ts";

export type AgentRpcMethod =
  | "runtime.query"
  | "command.dispatch"
  | "events.projection"
  | "agent.health";

export type AgentFailureCause =
  | "network"
  | "http-status"
  | "malformed-body"
  | "rpc-error"
  | "invalid-result";

/** adapter 内部失败（未映射）；route 层 `mapAgentFailure` 转 taxonomy。 */
export class AgentTransportFailure extends Error {
  readonly causeKind: AgentFailureCause;
  /** server-log-only 诊断（绝不进 Product 响应体）。 */
  readonly detail: string;

  constructor(cause: AgentFailureCause, detail: string) {
    super(`${cause}: ${detail}`);
    this.name = "AgentTransportFailure";
    this.causeKind = cause;
    this.detail = detail;
  }
}

/** C2 超时预算（毫秒）：query/projection/health = 3s；dispatch = 10s。 */
export const QUERY_TIMEOUT_MS = 3_000;
export const DISPATCH_TIMEOUT_MS = 10_000;

export interface AgentControlClientOptions {
  baseUrl: string;
  path?: string;
  fetchImpl?: typeof fetch;
  queryTimeoutMs?: number;
  dispatchTimeoutMs?: number;
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function requireString(obj: Record<string, unknown>, key: string): string {
  const v = obj[key];
  if (typeof v !== "string") {
    throw new AgentTransportFailure("invalid-result", `result.${key} is not a string`);
  }
  return v;
}

function requireNumber(obj: Record<string, unknown>, key: string): number {
  const v = obj[key];
  if (typeof v !== "number" || !Number.isFinite(v)) {
    throw new AgentTransportFailure("invalid-result", `result.${key} is not a number`);
  }
  return v;
}

function requireArray(obj: Record<string, unknown>, key: string): unknown[] {
  const v = obj[key];
  if (!Array.isArray(v)) {
    throw new AgentTransportFailure("invalid-result", `result.${key} is not an array`);
  }
  return v;
}

function validateQuerySnapshot(result: unknown): AgentQuerySnapshotWire {
  if (!isObject(result)) {
    throw new AgentTransportFailure("invalid-result", "result is not an object");
  }
  // 新鲜度信封是 Product 读的硬前提（C2：无缓存策略下唯一 stale 标注来源）。
  requireNumber(result, "generated_at_ms");
  requireNumber(result, "observation_revision");
  requireString(result, "observation_lineage");
  for (const key of ["devices", "ports", "resources", "sessions", "capabilities"] as const) {
    requireArray(result, key);
  }
  for (const s of requireArray(result, "sessions")) {
    if (!isObject(s)) {
      throw new AgentTransportFailure("invalid-result", "sessions[] entry is not an object");
    }
    requireString(s, "id");
    requireString(s, "state");
    requireString(s, "phase");
    requireArray(s, "outputs");
  }
  // CP-01A 只消费上述字段；inputs/program_switch 透传不深度校验。
  return result as unknown as AgentQuerySnapshotWire;
}

function validateHealth(result: unknown): AgentHealthWire {
  if (!isObject(result)) {
    throw new AgentTransportFailure("invalid-result", "result is not an object");
  }
  requireString(result, "state");
  requireNumber(result, "devices");
  requireNumber(result, "active_pipelines");
  requireNumber(result, "dropped_bus_events");
  requireNumber(result, "clock_lost_events");
  return result as unknown as AgentHealthWire;
}

function validateProjection(result: unknown): AgentProjectionWire {
  if (!isObject(result)) {
    throw new AgentTransportFailure("invalid-result", "result is not an object");
  }
  if (result.snapshot_kind !== "event_projection_snapshot") {
    throw new AgentTransportFailure(
      "invalid-result",
      "result.snapshot_kind is not the projection guard literal",
    );
  }
  requireNumber(result, "total");
  if (typeof result.has_critical !== "boolean") {
    throw new AgentTransportFailure("invalid-result", "result.has_critical is not a boolean");
  }
  return result as unknown as AgentProjectionWire;
}

export class AgentControlClient {
  private readonly url: string;
  private readonly fetchImpl: typeof fetch;
  private readonly queryTimeoutMs: number;
  private readonly dispatchTimeoutMs: number;
  private nextId = 0;

  constructor(opts: AgentControlClientOptions) {
    this.url = `${opts.baseUrl.replace(/\/+$/, "")}${opts.path ?? AGENT_RPC_PATH}`;
    this.fetchImpl = opts.fetchImpl ?? fetch;
    this.queryTimeoutMs = opts.queryTimeoutMs ?? QUERY_TIMEOUT_MS;
    this.dispatchTimeoutMs = opts.dispatchTimeoutMs ?? DISPATCH_TIMEOUT_MS;
  }

  /** dispatch 类方法 10s，其余（query/projection/health）3s。 */
  private timeoutFor(method: AgentRpcMethod): number {
    return method === "command.dispatch" ? this.dispatchTimeoutMs : this.queryTimeoutMs;
  }

  private async call(
    method: AgentRpcMethod,
    params: unknown,
    validate: (result: unknown) => unknown,
  ): Promise<unknown> {
    const id = randomUUID();
    const body = JSON.stringify(
      params === undefined ? { jsonrpc: "2.0", method, id } : { jsonrpc: "2.0", method, params, id },
    );
    let res: Response;
    try {
      res = await this.fetchImpl(this.url, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body,
        signal: AbortSignal.timeout(this.timeoutFor(method)),
      });
    } catch (err) {
      const reason = err instanceof Error ? `${err.name}: ${err.message}` : String(err);
      throw new AgentTransportFailure("network", `${method} transport failure (${reason})`);
    }
    if (!res.ok) {
      throw new AgentTransportFailure("http-status", `${method} HTTP ${res.status}`);
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(await res.text());
    } catch (err) {
      throw new AgentTransportFailure(
        "malformed-body",
        `${method} response body is not JSON (${(err as Error).message})`,
      );
    }
    if (!isObject(parsed)) {
      throw new AgentTransportFailure("malformed-body", `${method} response is not a JSON object`);
    }
    if ("error" in parsed) {
      const rpcErr = parsed.error;
      const code = isObject(rpcErr) ? String(rpcErr.code) : "unknown";
      const message = isObject(rpcErr) ? String(rpcErr.message) : String(rpcErr);
      throw new AgentTransportFailure("rpc-error", `${method} JSON-RPC error ${code}: ${message}`);
    }
    if (!("result" in parsed)) {
      throw new AgentTransportFailure("invalid-result", `${method} response has no result field`);
    }
    return validate(parsed.result);
  }

  /** 查询面：Runtime 现状唯一来源（live 读；Fastify 不缓存 mutable 状态）。 */
  runtimeQuery(): Promise<AgentQuerySnapshotWire> {
    return this.call("runtime.query", undefined, validateQuerySnapshot) as Promise<AgentQuerySnapshotWire>;
  }

  agentHealth(): Promise<AgentHealthWire> {
    return this.call("agent.health", undefined, validateHealth) as Promise<AgentHealthWire>;
  }

  /** CP-01B 起消费；envelope/超时/错误映射在本包已冻结。 */
  dispatchCommand(params: AgentCommandDispatchParams): Promise<unknown> {
    return this.call("command.dispatch", params, (r) => r);
  }

  /** CP-01D 起消费（单消费者 drain 约束届时在 route 层强制）。 */
  projectEvents(): Promise<AgentProjectionWire> {
    return this.call("events.projection", undefined, validateProjection) as Promise<AgentProjectionWire>;
  }
}
