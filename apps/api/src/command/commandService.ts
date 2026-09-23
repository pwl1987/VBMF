/**
 * Durable command service —— C4/C6 全语义的外部 boundary-of-record：
 * claim（唯一键恰一次）→ internal dispatch → 四出口裁决 → 单向终态迁移
 * （防 ABA）+ 同事务审计 → replay/conflict/pending → 租约回收。
 *
 * 红线：dispatch 失败绝不假成功、绝不自动重发 start_session（F2）；
 * 超龄 pending 只能回收为 timeout(retryable)（F6）；终态不可变（F3 重放
 * 首次响应）；同键异 fingerprint 绝不执行第二 payload（F4）。
 */
import { and, eq, lt, sql } from "drizzle-orm";
import type { Db } from "../db/index.ts";
import {
  auditEntries,
  commands,
  type commands as commandsTable,
} from "../db/schema.ts";
import { canonicalCommandId, newCommandId } from "./commandIds.ts";
import { commandFingerprint } from "./fingerprint.ts";
import { mapVerdict, timeoutResponse, type AgentVerdict, type TerminalState } from "./verdictMapper.ts";
import { AgentTransportFailure, type AgentControlClient } from "../agent/agentControlClient.ts";
import { errorEnvelope, internalError } from "../lib/errors.ts";

export type CommandKind = "start_session" | "stop_session" | "release_session";

type CommandRow = typeof commandsTable.$inferSelect;

export interface SubmitInput {
  rawCommandId: string | undefined;
  principal: string;
  kind: CommandKind;
  target: Record<string, unknown>;
  requestedBy: string;
}

export interface ServiceResponse {
  status: number;
  body: unknown;
}

export interface CommandServiceDeps {
  db: Db;
  agent: AgentControlClient;
  /** pending 超龄租约（毫秒）；超龄回收为 timeout(retryable)（F6）。 */
  leaseMs: number;
  /** in-flight 重复提交的短窗口等待（毫秒；C6）。 */
  replayWaitMs: number;
  /** 测试注入时钟（默认 Date.now）。 */
  now?: () => number;
  /** 测试注入等待（默认 setTimeout）。 */
  sleep?: (ms: number) => Promise<void>;
}

function operationBody(row: CommandRow): Record<string, unknown> {
  const body: Record<string, unknown> = {
    command_id: row.commandId,
    state: row.state,
    kind: row.kind,
    created_at: row.createdAt.toISOString(),
  };
  if (row.classification !== null) body.classification = row.classification;
  if (row.detail !== null) body.detail = row.detail;
  if (row.verdict !== null) body.verdict = row.verdict;
  if (row.terminalAt !== null) body.terminal_at = row.terminalAt.toISOString();
  return body;
}

const conflictEnvelope = {
  error: {
    code: "RESOURCE_CONFLICT",
    message: "command_id already used with a different payload or principal",
    retryable: false,
  },
};

export class CommandService {
  private readonly db: Db;
  private readonly agent: AgentControlClient;
  private readonly leaseMs: number;
  private readonly replayWaitMs: number;
  private readonly now: () => number;
  private readonly sleep: (ms: number) => Promise<void>;

  constructor(deps: CommandServiceDeps) {
    this.db = deps.db;
    this.agent = deps.agent;
    this.leaseMs = deps.leaseMs;
    this.replayWaitMs = deps.replayWaitMs;
    this.now = deps.now ?? Date.now;
    this.sleep = deps.sleep ?? ((ms) => new Promise((r) => setTimeout(r, ms)));
  }

  /** C4 主链：claim → dispatch → verdict → 终态 + 审计（同事务）。 */
  async submit(input: SubmitInput): Promise<ServiceResponse> {
    const commandId = canonicalCommandId(input.rawCommandId ?? newCommandId());
    const fingerprint = commandFingerprint(input.kind, input.target, input.principal);

    // 恰一次 claim：唯一键 + ON CONFLICT DO NOTHING（并发者进 replay/pending 分支·F5）。
    const inserted = await this.db
      .insert(commands)
      .values({
        commandId,
        principal: input.principal,
        kind: input.kind,
        target: input.target,
        fingerprint,
        state: "pending",
      })
      .onConflictDoNothing({ target: commands.commandId })
      .returning();

    if (inserted.length === 0) {
      return this.existingKey(commandId, fingerprint);
    }

    try {
      const raw = await this.agent.dispatchCommand({
        command_id: commandId,
        kind: input.kind,
        target: input.target,
        requested_by: input.requestedBy,
      });
      const verdict = this.asVerdict(raw);
      const mapping = mapVerdict(verdict);
      const body = operationBody({
        ...(inserted[0] as CommandRow),
        state: mapping.state,
        classification: verdict.classification ?? null,
        detail: verdict.detail ?? null,
        verdict: raw as object,
        terminalAt: new Date(this.now()),
      });
      await this.finalize(commandId, {
        state: mapping.state,
        classification: verdict.classification ?? null,
        detail: verdict.detail ?? null,
        verdict: raw as object,
        responseStatus: mapping.responseStatus,
        responseBody: body,
        principal: input.principal,
        kind: input.kind,
      });
      return { status: mapping.responseStatus, body };
    } catch (err) {
      if (err instanceof AgentTransportFailure) {
        // F1/F2：依赖失败 → timeout(retryable) 终态；503 响应被持久化供重放。
        const tr = timeoutResponse();
        await this.finalize(commandId, {
          state: tr.state,
          classification: "retryable",
          detail: "agent dispatch transport failure",
          verdict: null,
          responseStatus: tr.responseStatus,
          responseBody: tr.body,
          principal: input.principal,
          kind: input.kind,
        });
        return { status: tr.responseStatus, body: tr.body };
      }
      if (err instanceof UnrecognizedVerdictError) {
        // agent 返回不可辨识裁决 = 内部契约违例 → failed(unknown) + 500 语义。
        const body = errorEnvelope(internalError("media agent returned an unrecognized verdict"));
        await this.finalize(commandId, {
          state: "failed",
          classification: "unknown",
          detail: err.message.slice(0, 500),
          verdict: null,
          responseStatus: 500,
          responseBody: body,
          principal: input.principal,
          kind: input.kind,
        });
        return { status: 500, body };
      }
      throw err;
    }
  }

  private asVerdict(raw: unknown): AgentVerdict {
    if (typeof raw === "object" && raw !== null) {
      const status = (raw as Record<string, unknown>).status;
      if (typeof status === "object" && status !== null) {
        const inner = (status as Record<string, unknown>).status;
        if (typeof inner === "string") {
          return raw as AgentVerdict;
        }
      }
    }
    throw new UnrecognizedVerdictError(raw);
  }

  /** 已存在键：fingerprint 不匹配 → 409；终态 → 重放首次响应；pending → 短等待。 */
  private async existingKey(commandId: string, fingerprint: string): Promise<ServiceResponse> {
    const [row] = await this.db.select().from(commands).where(eq(commands.commandId, commandId));
    if (row === undefined) {
      // 极端竞态：冲突后行被外部删除（正常路径不存在；防御性诚实）。
      throw internalError("command record disappeared after claim conflict");
    }
    if (row.fingerprint !== fingerprint) {
      // F4：第二 payload 绝不执行、绝不重放。
      return { status: 409, body: conflictEnvelope };
    }
    if (row.state !== "pending") {
      // F3：重放首次终态响应（含 failed/timeout）。
      return { status: row.responseStatus ?? 500, body: row.responseBody ?? operationBody(row) };
    }
    // F5：并发重复——短窗口等待终态，仍未终态 → 200 pending（非阻塞长轮询）。
    const deadline = this.now() + this.replayWaitMs;
    while (this.now() < deadline) {
      await this.sleep(Math.min(100, this.replayWaitMs));
      const [again] = await this.db.select().from(commands).where(eq(commands.commandId, commandId));
      if (again !== undefined && again.state !== "pending") {
        return { status: again.responseStatus ?? 500, body: again.responseBody ?? operationBody(again) };
      }
    }
    const [current] = await this.db.select().from(commands).where(eq(commands.commandId, commandId));
    const rowNow = current ?? row;
    if (rowNow.state === "pending") {
      return { status: 200, body: operationBody(rowNow) };
    }
    return { status: rowNow.responseStatus ?? 500, body: rowNow.responseBody ?? operationBody(rowNow) };
  }

  /** 单向终态迁移 + 同事务审计（条件 UPDATE WHERE state='pending'，防 ABA·C6）。 */
  private async finalize(
    commandId: string,
    terminal: {
      state: TerminalState;
      classification: string | null;
      detail: string | null;
      verdict: object | null;
      responseStatus: number;
      responseBody: unknown;
      principal: string;
      kind: string;
    },
  ): Promise<void> {
    await this.db.transaction(async (tx) => {
      const updated = await tx
        .update(commands)
        .set({
          state: terminal.state,
          classification: terminal.classification,
          detail: terminal.detail,
          verdict: terminal.verdict,
          responseStatus: terminal.responseStatus,
          responseBody: terminal.responseBody as object,
          terminalAt: sql`now()`,
          updatedAt: sql`now()`,
        })
        .where(and(eq(commands.commandId, commandId), eq(commands.state, "pending")))
        .returning({ commandId: commands.commandId });
      if (updated.length > 0) {
        await tx
          .insert(auditEntries)
          .values({
            commandId,
            principal: terminal.principal,
            kind: terminal.kind,
            state: terminal.state,
          })
          .onConflictDoNothing({ target: auditEntries.commandId });
      }
    });
  }

  /** F6 租约回收：超龄 pending → timeout(retryable)；绝不猜成功。 */
  async recoverStalePending(): Promise<number> {
    const stale = await this.db
      .select({ commandId: commands.commandId, principal: commands.principal, kind: commands.kind })
      .from(commands)
      .where(
        and(
          eq(commands.state, "pending"),
          lt(commands.createdAt, sql`now() - (${this.leaseMs} * interval '1 millisecond')`),
        ),
      );
    const tr = timeoutResponse();
    for (const row of stale) {
      await this.finalize(row.commandId, {
        state: tr.state,
        classification: "retryable",
        detail: "lease recovery: dispatch verdict was never recorded",
        verdict: null,
        responseStatus: tr.responseStatus,
        responseBody: tr.body,
        principal: row.principal,
        kind: row.kind,
      });
    }
    return stale.length;
  }

  /** GET /commands/{id}：命令旅程事实（≠Runtime 现状·C8 两平面）。
   * GET 恒 200 + Operation 记录视图；首次响应语义重放只属于 POST。 */
  async getOperation(commandId: string): Promise<ServiceResponse | undefined> {
    let [row] = await this.db.select().from(commands).where(eq(commands.commandId, commandId));
    if (row === undefined) return undefined;
    if (row.state === "pending") {
      const ageMs = this.now() - row.createdAt.getTime();
      if (ageMs > this.leaseMs) {
        await this.recoverStalePending();
        const [recovered] = await this.db.select().from(commands).where(eq(commands.commandId, commandId));
        if (recovered !== undefined) row = recovered;
      }
    }
    return { status: 200, body: operationBody(row) };
  }
}

export class UnrecognizedVerdictError extends Error {
  readonly raw: unknown;
  constructor(raw: unknown) {
    super("agent dispatch result is not a recognizable verdict envelope");
    this.name = "UnrecognizedVerdictError";
    this.raw = raw;
  }
}

export { conflictEnvelope };
