/**
 * CP-01B durable command boundary（RH-IDEM-01 清偿）。
 *
 * 表族对齐 CONTROL-PLANE-ENTRY-01 planning §5：`commands` = 外部
 * boundary-of-record（durable idempotency + 命令旅程）；`audit_entries` =
 * 每外部命令的 who/what/when/command_id/verdict（C10：CP 持久记录，
 * 与 V0.2 §5 通用 `audit_logs` 家族的 CP 专属 reconciliation——收口报告
 * 记录该命名裁定）。
 *
 * 红线（C6/C8/C10）：state 单向迁移（claimed→终态，条件 UPDATE）；
 * 终态不可变、无覆写路径；Fastify 从不读 agent 幂等表；这些表是命令旅程
 * 事实，**绝不**成为 session/resource/health 的 Runtime truth。
 */
import {
  bigserial,
  index,
  integer,
  jsonb,
  pgEnum,
  pgTable,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";

/** C8 冻结词表：pending（claimed 未终态）/ completed / failed / timeout / conflict / rejected。 */
export const commandState = pgEnum("command_state", [
  "pending",
  "completed",
  "failed",
  "timeout",
  "conflict",
  "rejected",
]);

export const commands = pgTable(
  "commands",
  {
    commandId: uuid("command_id").primaryKey(),
    /** 开发态鉴权桩主体（CP-01C 换真实 authn；fingerprint 组成部分·C6）。 */
    principal: text("principal").notNull(),
    kind: text("kind").notNull(),
    /** canonical target JSON（fingerprint 输入，排除 issued_at/requested_by·D9-A）。 */
    target: jsonb("target").notNull(),
    /** sha256(kind + canonical target + principal)——同键异 fingerprint = Conflict。 */
    fingerprint: text("fingerprint").notNull(),
    state: commandState("state").notNull().default("pending"),
    /** agent 裁决原样（ApiCommandResponse）——终态时写入，不可变。 */
    verdict: jsonb("verdict"),
    classification: text("classification"),
    detail: text("detail"),
    /** 首次终态响应的 HTTP 状态与响应体（F3 逐字节重放语义）。 */
    responseStatus: integer("response_status"),
    responseBody: jsonb("response_body"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
    terminalAt: timestamp("terminal_at", { withTimezone: true }),
  },
  (t) => [
    index("commands_state_created_idx").on(t.state, t.createdAt),
  ],
);

export const auditEntries = pgTable(
  "audit_entries",
  {
    id: bigserial("id", { mode: "number" }).primaryKey(),
    commandId: uuid("command_id").notNull(),
    principal: text("principal").notNull(),
    kind: text("kind").notNull(),
    /** 终态（与 commands.state 同事务写入）。 */
    state: commandState("state").notNull(),
    at: timestamp("at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    uniqueIndex("audit_entries_command_id_key").on(t.commandId),
    index("audit_entries_at_idx").on(t.at),
  ],
);
