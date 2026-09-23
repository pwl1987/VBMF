/**
 * Command API（CP-01B）：POST 资源动作 + GET /commands/{id}（Operation 面）。
 *
 * - Idempotency-Key header 优先于 body command_id（C6）；均缺省由服务端生成。
 * - 客户端 URL session id 只接受 canonical UUID（C9/F13 客户端方向 → 4xx）。
 * - 两平面纪律：响应是命令裁决/Operation 旅程，不是 Runtime 现状（C8）。
 */
import type { FastifyInstance } from "fastify";
import { ApiError, notFound, validationError } from "../lib/errors.ts";
import { assertCanonicalUuid, ClientSessionIdError } from "../lib/sessionIds.ts";
import { canonicalCommandId } from "../command/commandIds.ts";
import { CommandService, type CommandKind } from "../command/commandService.ts";
import { resolveDevPrincipal } from "../command/principal.ts";

export interface CommandRouteDeps {
  commandService: CommandService | null;
}

function canonicalSessionParam(raw: string): string {
  try {
    return assertCanonicalUuid(raw);
  } catch (err) {
    if (err instanceof ClientSessionIdError) {
      throw validationError("session id path parameter must be a canonical UUID");
    }
    throw err;
  }
}

function commandKeyFrom(
  headers: Record<string, unknown>,
  body: Record<string, unknown>,
): string | undefined {
  const header = headers["idempotency-key"];
  if (header !== undefined) {
    if (typeof header !== "string" || header.length === 0 || header.length > 256) {
      throw validationError("Idempotency-Key must be a string of 1..256 characters");
    }
    return header;
  }
  const bodyId = body.command_id;
  if (bodyId === undefined) return undefined;
  if (typeof bodyId !== "string" || bodyId.length === 0 || bodyId.length > 256) {
    throw validationError("command_id must be a string of 1..256 characters");
  }
  return bodyId;
}

function requireCommandPlane(deps: CommandRouteDeps): CommandService {
  if (deps.commandService === null) {
    // 控制面以读 API 形态部署（未配置持久层）——诚实 503，不是客户端错误。
    throw new ApiError(
      "RESOURCE_UNAVAILABLE",
      503,
      "command persistence is not configured on this control plane",
      false,
    );
  }
  return deps.commandService;
}

export async function commandRoutes(app: FastifyInstance, deps: CommandRouteDeps): Promise<void> {
  app.post("/api/v1/sessions", async (req, reply) => {
    const service = requireCommandPlane(deps);
    const body = (req.body ?? {}) as Record<string, unknown>;
    const intent: unknown = body.intent;
    if (typeof intent !== "object" || intent === null || Array.isArray(intent)) {
      throw validationError("intent must be an object with version and a non-empty devices array");
    }
    const intentRecord = intent as Record<string, unknown>;
    if (
      typeof intentRecord.version !== "string" ||
      !Array.isArray(intentRecord.devices) ||
      intentRecord.devices.length === 0
    ) {
      throw validationError("intent must be an object with version and a non-empty devices array");
    }
    const principal = resolveDevPrincipal(req.headers);
    const res = await service.submit({
      rawCommandId: commandKeyFrom(req.headers, body),
      principal,
      kind: "start_session",
      target: { target_type: "session", intent },
      requestedBy: typeof body.requested_by === "string" ? body.requested_by : principal,
    });
    return reply.code(res.status).send(res.body);
  });

  const sessionAction = (kind: CommandKind, path: string) => {
    app.post(path, async (req, reply) => {
      const service = requireCommandPlane(deps);
      const body = (req.body ?? {}) as Record<string, unknown>;
      const rawId = (req.params as Record<string, unknown>).id;
      if (typeof rawId !== "string") throw validationError("session id must be a canonical UUID");
      const sessionId = canonicalSessionParam(rawId);
      const principal = resolveDevPrincipal(req.headers);
      const res = await service.submit({
        rawCommandId: commandKeyFrom(req.headers, body),
        principal,
        kind,
        target: { target_type: "session_by_id", session_id: sessionId },
        requestedBy: typeof body.requested_by === "string" ? body.requested_by : principal,
      });
      return reply.code(res.status).send(res.body);
    });
  };
  sessionAction("stop_session", "/api/v1/sessions/:id/stop");
  sessionAction("release_session", "/api/v1/sessions/:id/release");

  app.get("/api/v1/commands/:id", async (req, reply) => {
    const service = requireCommandPlane(deps);
    const rawId = (req.params as Record<string, unknown>).id;
    if (typeof rawId !== "string") throw validationError("command id must be canonical");
    const commandId = canonicalCommandId(rawId);
    const res = await service.getOperation(commandId);
    if (res === undefined) {
      throw notFound(`command ${commandId} not found`);
    }
    return reply.code(res.status).send(res.body);
  });
}
