/**
 * CP-01A Fastify 应用组合根。Product `/api/v1/*` 与 Rust prototype
 * `/api/v1/*` 完全隔离（C1）——本应用不反代/不转发任何请求到 agent 的
 * prototype 诊断面；对 agent 的唯一消费是 AgentControlClient。
 */
import { fastify, type FastifyInstance } from "fastify";
import { pathToFileURL } from "node:url";
import { loadConfig, type AppConfig } from "./config.ts";
import { AgentControlClient } from "./agent/agentControlClient.ts";
import { runtimeRoutes, type RouteDeps } from "./routes/runtime.ts";
import { healthRoutes } from "./routes/health.ts";
import { ApiError, errorEnvelope, internalError, notFound } from "./lib/errors.ts";
import { AgentWireSessionIdError } from "./lib/sessionIds.ts";

export interface BuildOptions {
  agent?: RouteDeps["agent"];
}

export async function buildApp(config: AppConfig, opts: BuildOptions = {}): Promise<FastifyInstance> {
  const agent = opts.agent ?? new AgentControlClient({ baseUrl: config.mediaAgentRpcUrl });
  const app = fastify({
    logger: { level: config.logLevel },
    ...(config.trustProxy ? { trustProxy: true as const } : {}),
  });

  // 错误处理器必须在路由插件 register 之前挂载（await register 会固化子
  // context 的 handler 链；后挂的根级 handler 不作用于已 boot 的插件路由）。
  app.setErrorHandler((err, req, reply) => {
    if (err instanceof ApiError) {
      req.log.warn({ code: err.code, status: err.status }, err.message);
      return reply.code(err.status).send(errorEnvelope(err));
    }
    if (err instanceof AgentWireSessionIdError) {
      // server-log 保留原始 wire 值供诊断；响应不泄漏内部细节。
      req.log.error({ rawId: err.rawId }, "agent wire session id rejected");
      const mapped = internalError("runtime snapshot could not be normalized");
      return reply.code(mapped.status).send(errorEnvelope(mapped));
    }
    if (err instanceof SyntaxError && "body" in err) {
      const mapped = new ApiError("VALIDATION_ERROR", 400, "request body is not valid JSON", false);
      return reply.code(mapped.status).send(errorEnvelope(mapped));
    }
    req.log.error({ err }, "unhandled error");
    const mapped = internalError("internal server error");
    return reply.code(mapped.status).send(errorEnvelope(mapped));
  });

  app.setNotFoundHandler((_req, reply) => {
    const mapped = notFound("route not found");
    return reply.code(mapped.status).send(errorEnvelope(mapped));
  });

  await app.register(runtimeRoutes, { agent });
  await app.register(healthRoutes, { agent });

  return app;
}

async function start(): Promise<void> {
  const config = loadConfig();
  const app = await buildApp(config);
  try {
    await app.listen({ port: config.port, host: config.host });
  } catch (err) {
    app.log.error(err);
    process.exitCode = 1;
  }
  const shutdown = async (): Promise<void> => {
    await app.close();
    process.exit(0);
  };
  process.once("SIGTERM", shutdown);
  process.once("SIGINT", shutdown);
}

const isMain =
  process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  await start();
}
