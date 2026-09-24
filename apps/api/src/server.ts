/**
 * CP-01A/CP-01B/CP-01C Fastify 应用组合根。Product `/api/v1/*` 与 Rust prototype
 * `/api/v1/*` 完全隔离（C1）——本应用不反代/不转发任何请求到 agent 的
 * prototype 诊断面；对 agent 的唯一消费是 AgentControlClient。
 *
 * CP-01C：Better Auth（AuthN identity owner）+ CASL（应用层 AuthZ）+
 * 应用层限流 + 安全审计由 registerSecurity 统一 enforcement；`x-dev-principal`
 * 开发态桩已从生产路径删除（C11）。测试只能经显式注入的 Authenticator
 * fixture 模拟身份，生产无 dev bypass。
 */
import { fastify, type FastifyBaseLogger, type FastifyInstance } from "fastify";
import { pino } from "pino";
import { pathToFileURL } from "node:url";
import { loadConfig, type AppConfig } from "./config.ts";
import { AgentControlClient } from "./agent/agentControlClient.ts";
import { runtimeRoutes, type RouteDeps } from "./routes/runtime.ts";
import { healthRoutes } from "./routes/health.ts";
import { commandRoutes } from "./routes/commands.ts";
import { ApiError, errorEnvelope, internalError, notFound } from "./lib/errors.ts";
import { AgentWireSessionIdError } from "./lib/sessionIds.ts";
import { CommandService, type CommandPlane } from "./command/commandService.ts";
import { createDb, runMigrations, type Db } from "./db/index.ts";
import { BetterAuthAuthenticator, createAuth, type Authenticator } from "./security/auth.ts";
import { SlidingWindowRateLimiter } from "./security/rateLimit.ts";
import { SecurityAudit } from "./security/audit.ts";
import { registerSecurity } from "./security/fastifySecurity.ts";
import type pg from "pg";

export interface BuildOptions {
  agent?: RouteDeps["agent"];
  /** CP-01B 注入（测试）；CP-01C hermetic 测试可注入显式 CommandPlane stub。 */
  commandService?: CommandPlane;
  /** CP-01C 注入（测试 fixture；生产路径由 db+secret 构建 Better Auth）。 */
  authenticator?: Authenticator;
  /** 显式禁用持久层（默认按 config.databaseUrl 推导）。 */
  withoutDb?: boolean;
  /** 测试注入日志目的地（redact 配置仍取同一来源——机检不削弱）。 */
  logDestination?: NodeJS.WritableStream;
}

/** 日志选项唯一来源：credential redact 管道（安全红线）在此集中定义。 */
export function buildLoggerOptions(level: string): { level: string; redact: { paths: string[]; censor: string } } {
  return {
    level,
    redact: {
      // 安全红线：credential/token 绝不进日志（CP-01C 机检覆盖）。
      paths: ['req.headers["x-api-key"]', "req.headers.authorization", "req.headers.cookie"],
      censor: "[REDACTED]",
    },
  };
}

export interface AppHandle {
  app: FastifyInstance;
  db: Db | null;
  pool: pg.Pool | null;
  /** CP-01C：auth 层实例；null = not_configured（/api/v1 fail-closed 503）。 */
  authenticator: Authenticator | null;
}

export async function buildApp(config: AppConfig, opts: BuildOptions = {}): Promise<FastifyInstance> {
  const handle = await buildAppHandle(config, opts);
  return handle.app;
}

export async function buildAppHandle(config: AppConfig, opts: BuildOptions = {}): Promise<AppHandle> {
  const agent = opts.agent ?? new AgentControlClient({ baseUrl: config.mediaAgentRpcUrl });

  let db: Db | null = null;
  let pool: pg.Pool | null = null;
  if (!opts.withoutDb && config.databaseUrl !== null) {
    const created = createDb(config.databaseUrl);
    db = created.db;
    pool = created.pool;
    if (config.migrateOnBoot) {
      await runMigrations(db);
    }
  }

  const commandService =
    opts.commandService ??
    (db !== null
      ? new CommandService({
          db,
          agent,
          leaseMs: config.commandLeaseMs,
          replayWaitMs: config.commandReplayWaitMs,
        })
      : null);

  // CP-01C auth 层：需要 db + secret；测试可显式注入 authenticator fixture。
  const authenticator: Authenticator | null =
    opts.authenticator ?? (db !== null && config.authSecret !== null ? new BetterAuthAuthenticator(createAuth({ db, secret: config.authSecret }), db) : null);

  const limiter =
    authenticator !== null
      ? new SlidingWindowRateLimiter({
          windowMs: config.rateLimitWindowMs,
          maxByBucket: { read: config.rateLimitReadMax, write: config.rateLimitWriteMax },
        })
      : null;

  const app = fastify({
    // redact 配置唯一来源 buildLoggerOptions；测试经 logDestination 注入
    // 捕获流（同 redact 配置，机检不削弱）。
    ...(opts.logDestination !== undefined
      ? { loggerInstance: pino(buildLoggerOptions(config.logLevel), opts.logDestination) as FastifyBaseLogger }
      : { logger: buildLoggerOptions(config.logLevel) }),
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
    // 含 security misconfiguration（/api/v1 路由缺 security config）= fail-closed 500。
    req.log.error({ err }, "unhandled error");
    const mapped = internalError("internal server error");
    return reply.code(mapped.status).send(errorEnvelope(mapped));
  });

  app.setNotFoundHandler((_req, reply) => {
    const mapped = notFound("route not found");
    return reply.code(mapped.status).send(errorEnvelope(mapped));
  });

  // CP-01C enforcement 链必须先注册（preHandler 全局钩子；路由随后声明
  // config.security）。
  registerSecurity(app, {
    authenticator,
    limiter,
    audit: db !== null ? new SecurityAudit(db) : null,
  });

  await app.register(runtimeRoutes, { agent });
  await app.register(healthRoutes, { agent, db, authenticator });
  await app.register(commandRoutes, { commandService });

  // F6：启动时回收上次运行遗留的超龄 pending（租约回收，不猜成功）。
  if (commandService !== null) {
    const recovered = await commandService.recoverStalePending().catch((err: unknown) => {
      app.log.error({ err }, "startup lease recovery failed");
      return -1;
    });
    if (recovered > 0) {
      app.log.warn({ recovered }, "startup lease recovery marked stale pending as timeout");
    }
  }

  return { app, db, pool, authenticator };
}

async function start(): Promise<void> {
  const config = loadConfig();
  const { app, pool } = await buildAppHandle(config);
  try {
    await app.listen({ port: config.port, host: config.host });
  } catch (err) {
    app.log.error(err);
    process.exitCode = 1;
  }
  const shutdown = async (): Promise<void> => {
    await app.close();
    if (pool !== null) await pool.end();
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
