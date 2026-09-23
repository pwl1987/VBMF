/**
 * agent 四出口裁决 → C8 Operation 终态 + Product 响应映射。
 *
 * agent wire（ApiCommandResponse）：status.status ∈
 * executed/replayed/conflict/rejected；失败语义经 classification
 * （permanent/retryable/unknown/rejected…）传达，不暴露 Failed（0.7C-7）。
 *
 * 映射（冻结）：
 * - executed/replayed 且无 classification        → completed
 * - executed 且 classification=rejected           → rejected（诚实拒绝裁决）
 * - executed 且 classification∈{permanent,retryable,unknown} → failed（记录 classification）
 * - conflict                                      → conflict
 * 响应 HTTP：completed/failed/rejected → 200（命令旅程事实）；conflict → 409。
 */

export interface AgentVerdict {
  command_id: string;
  status: { status: string };
  kind: string;
  classification?: string;
  detail?: string;
}

export type TerminalState = "completed" | "failed" | "timeout" | "conflict" | "rejected";

export interface VerdictMapping {
  state: TerminalState;
  responseStatus: number;
}

export function mapVerdict(verdict: AgentVerdict): VerdictMapping {
  const status = verdict.status?.status;
  switch (status) {
    case "executed":
    case "replayed": {
      const cls = verdict.classification;
      if (cls === undefined || cls === null || cls === "") {
        return { state: "completed", responseStatus: 200 };
      }
      if (cls === "rejected") {
        return { state: "rejected", responseStatus: 200 };
      }
      // permanent / retryable / unknown —— 失败语义经 classification 传达。
      return { state: "failed", responseStatus: 200 };
    }
    case "conflict":
      return { state: "conflict", responseStatus: 409 };
    case "rejected":
      return { state: "rejected", responseStatus: 200 };
    default:
      // 未知裁决词 = Fastify↔agent 内部契约违例（F1 家族 5xx，非客户端错）。
      throw new Error(`unknown agent verdict status: ${String(status)}`);
  }
}

/** 依赖失败（F1/F2）的 timeout 终态响应（不猜成功、不假数据）。 */
export function timeoutResponse(): { state: TerminalState; responseStatus: number; body: unknown } {
  return {
    state: "timeout",
    responseStatus: 503,
    body: {
      error: {
        code: "DEPENDENCY_UNAVAILABLE",
        message: "command dispatch did not complete against the media agent",
        retryable: true,
      },
    },
  };
}
