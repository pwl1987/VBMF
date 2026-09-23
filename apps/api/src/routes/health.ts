/**
 * Health 面（EXTERNAL_API_CONTRACT §7：liveness/readiness/dependency 区分；
 * 分层不合并单 status）。
 *
 * - `GET /health/live`：liveness——只证明进程服务能力，不探测依赖
 *   （compose healthcheck 契约；agent 重启不得引发 fastify 容器 flap）。
 * - `GET /healthz`：分层依赖健康快照——api 层恒 up（端点自身可应答即真），
 *   runtime 层 live 观测 `agent.health`，db 层（CP-01B 持久记录载体，
 *   非 Runtime truth）SELECT 1 探测或诚实 not_configured。
 */
import type { FastifyInstance } from "fastify";
import { AgentTransportFailure, type AgentControlClient } from "../agent/agentControlClient.ts";
import type { RouteDeps } from "./runtime.ts";
import type { Db } from "../db/index.ts";

export interface HealthRouteDeps {
  agent: AgentControlClient;
  db?: Db | null;
}

export async function healthRoutes(app: FastifyInstance, deps: HealthRouteDeps): Promise<void> {
  app.get("/health/live", async () => ({ status: "live" }));

  app.get("/healthz", async (req) => {
    const checkedAtMs = Date.now();
    let runtime: Record<string, unknown>;
    try {
      const health = await deps.agent.agentHealth();
      runtime = {
        status: "up",
        agent_state: health.state,
        devices: health.devices,
        active_pipelines: health.active_pipelines,
        observed_at_ms: checkedAtMs,
      };
    } catch (err) {
      if (err instanceof AgentTransportFailure) {
        req.log.warn({ agentFailure: err.causeKind, detail: err.detail }, "agent.health failed");
        runtime = { status: "unreachable", observed_at_ms: checkedAtMs };
      } else {
        throw err;
      }
    }
    let db: Record<string, unknown>;
    if (deps.db === null || deps.db === undefined) {
      db = { status: "not_configured", observed_at_ms: checkedAtMs };
    } else {
      try {
        await deps.db.execute("select 1");
        db = { status: "up", observed_at_ms: checkedAtMs };
      } catch (err) {
        req.log.warn({ err: (err as Error).message }, "db probe failed");
        db = { status: "unreachable", observed_at_ms: checkedAtMs };
      }
    }
    return { checked_at_ms: checkedAtMs, layers: { api: { status: "up" }, runtime, db } };
  });
}
