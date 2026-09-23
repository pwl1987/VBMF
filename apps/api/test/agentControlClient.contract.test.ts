/**
 * AgentControlClient focused contract tests —— 对 mock JSON-RPC HTTP server
 * （CONTROL-PLANE-ENTRY-01 §8：adapter 契约测试对 mock JSON-RPC server）。
 * 覆盖：healthy runtime.query / connection refused / dependency timeout /
 * HTTP non-2xx / malformed JSON / JSON-RPC error envelope / missing-invalid
 * result shape / dispatch 超时预算分级 / envelope 形状。
 */
import test from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import type { AddressInfo } from "node:net";
import {
  AgentControlClient,
  AgentTransportFailure,
  DISPATCH_TIMEOUT_MS,
  QUERY_TIMEOUT_MS,
} from "../src/agent/agentControlClient.ts";

interface CapturedRequest {
  method: string;
  path: string;
  body: Record<string, unknown>;
}

/** mock agent：handler 收到已解析 JSON body，返回 (status, payload)。 */
async function startMock(
  handler: (req: CapturedRequest) => { status: number; body: unknown },
  delayMs = 0,
): Promise<{ url: string; requests: CapturedRequest[]; close: () => Promise<void> }> {
  const requests: CapturedRequest[] = [];
  const server = http.createServer((req, res) => {
    const chunks: Buffer[] = [];
    req.on("data", (c: Buffer) => chunks.push(c));
    req.on("end", () => {
      const text = Buffer.concat(chunks).toString("utf8");
      const parsed: unknown = JSON.parse(text);
      const captured: CapturedRequest = {
        method: req.method ?? "",
        path: req.url ?? "",
        body: parsed as Record<string, unknown>,
      };
      requests.push(captured);
      const { status, body } = handler(captured);
      setTimeout(() => {
        const payload = typeof body === "string" ? body : JSON.stringify(body);
        res.writeHead(status, { "content-type": "application/json" });
        res.end(payload);
      }, delayMs);
    });
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address() as AddressInfo;
  return {
    url: `http://127.0.0.1:${port}`,
    requests,
    close: () => new Promise<void>((resolve) => server.close(() => resolve())),
  };
}

function snapshotResult(): Record<string, unknown> {
  return {
    devices: [{ id: "dev-1", model: "Mock" }],
    ports: [],
    resources: [],
    sessions: [
      {
        id: "session-11111111222233334444555566667777",
        state: "running",
        phase: "running",
        outputs: ["rtmp://out"],
        inputs: [{ id: "dev-1", handle: 7 }],
      },
    ],
    capabilities: [],
    generated_at_ms: 1_700_000_000_012,
    observation_revision: 42,
    observation_lineage: "00000000-0000-0000-0000-0000000000aa",
    program_switch: null,
  };
}

const rpcOk = (result: unknown) => ({
  status: 200,
  body: { jsonrpc: "2.0", result, id: 1 },
});

test("healthy runtime.query: envelope shape on the wire + parsed snapshot", async () => {
  const mock = await startMock((req) => rpcOk(snapshotResult()));
  try {
    const client = new AgentControlClient({ baseUrl: mock.url });
    const snap = await client.runtimeQuery();
    assert.equal(snap.generated_at_ms, 1_700_000_000_012);
    assert.equal(snap.observation_revision, 42);
    assert.equal(snap.sessions[0]?.id, "session-11111111222233334444555566667777");
    // envelope: POST /internal/v1/agent, jsonrpc 2.0, method, unique id
    assert.equal(mock.requests.length, 1);
    const req = mock.requests[0]!;
    assert.equal(req.method, "POST");
    assert.equal(req.path, "/internal/v1/agent");
    assert.equal(req.body.jsonrpc, "2.0");
    assert.equal(req.body.method, "runtime.query");
    assert.ok(!("params" in req.body), "runtime.query 不携带 params");
    assert.ok(typeof req.body.id === "string" && req.body.id.length > 0);
  } finally {
    await mock.close();
  }
});

test("connection refused → AgentTransportFailure(network)", async () => {
  // 绑定后立即关闭，保留一个几乎必然未监听的端口形态。
  const mock = await startMock(() => rpcOk({}));
  const url = mock.url;
  await mock.close();
  const client = new AgentControlClient({ baseUrl: url });
  await assert.rejects(
    () => client.runtimeQuery(),
    (err: unknown) =>
      err instanceof AgentTransportFailure && err.causeKind === "network",
  );
});

test("dependency timeout → network cause（query 预算内 mock 延迟被 abort）", async () => {
  const mock = await startMock(() => rpcOk(snapshotResult()), 2_000);
  try {
    const client = new AgentControlClient({ baseUrl: mock.url, queryTimeoutMs: 100 });
    await assert.rejects(
      () => client.runtimeQuery(),
      (err: unknown) =>
        err instanceof AgentTransportFailure &&
        err.causeKind === "network" &&
        err.detail.includes("Timeout"),
    );
  } finally {
    await mock.close();
  }
});

test("HTTP non-2xx（agent 诚实 503 unconfigured）→ http-status cause", async () => {
  const mock = await startMock(() => ({
    status: 503,
    body: { error: "service_unavailable: runtime.query (session runtime not active)" },
  }));
  try {
    const client = new AgentControlClient({ baseUrl: mock.url });
    await assert.rejects(
      () => client.runtimeQuery(),
      (err: unknown) => err instanceof AgentTransportFailure && err.causeKind === "http-status",
    );
  } finally {
    await mock.close();
  }
});

test("malformed JSON body（2xx 非 JSON）→ malformed-body cause", async () => {
  const mock = await startMock(() => ({ status: 200, body: "not-json{" }));
  try {
    const client = new AgentControlClient({ baseUrl: mock.url });
    await assert.rejects(
      () => client.runtimeQuery(),
      (err: unknown) => err instanceof AgentTransportFailure && err.causeKind === "malformed-body",
    );
  } finally {
    await mock.close();
  }
});

test("JSON-RPC error envelope → rpc-error cause（内部契约违例诊断保留）", async () => {
  const mock = await startMock(() => ({
    status: 200,
    body: { jsonrpc: "2.0", error: { code: -32601, message: "method not found" }, id: 1 },
  }));
  try {
    const client = new AgentControlClient({ baseUrl: mock.url });
    await assert.rejects(
      () => client.runtimeQuery(),
      (err: unknown) => {
        assert.ok(err instanceof AgentTransportFailure && err.causeKind === "rpc-error");
        assert.ok(err.detail.includes("-32601"));
        return true;
      },
    );
  } finally {
    await mock.close();
  }
});

test("missing result field / invalid result shape → invalid-result cause", async () => {
  const cases: { status: number; body: unknown; hint: string }[] = [
    { status: 200, body: { jsonrpc: "2.0", id: 1 }, hint: "no result" },
    { status: 200, body: { jsonrpc: "2.0", result: null, id: 1 }, hint: "null result" },
    {
      status: 200,
      // 缺 generated_at_ms / observation_revision 新鲜度信封
      body: {
        jsonrpc: "2.0",
        result: { devices: [], ports: [], resources: [], sessions: [], capabilities: [] },
        id: 1,
      },
      hint: "missing freshness envelope",
    },
    {
      status: 200,
      body: { jsonrpc: "2.0", result: { ...snapshotResult(), sessions: "not-an-array" }, id: 1 },
      hint: "sessions not array",
    },
  ];
  for (const { status, body, hint } of cases) {
    const mock = await startMock(() => ({ status, body }));
    try {
      const client = new AgentControlClient({ baseUrl: mock.url });
      await assert.rejects(
        () => client.runtimeQuery(),
        (err: unknown) => {
          assert.ok(
            err instanceof AgentTransportFailure && err.causeKind === "invalid-result",
            `case ${hint}`,
          );
          return true;
        },
        `case ${hint}`,
      );
    } finally {
      await mock.close();
    }
  }
});

test("agent.health: result 形状校验通过并透传", async () => {
  const mock = await startMock(() =>
    rpcOk({
      state: "Ready",
      devices: 2,
      active_pipelines: 1,
      dropped_bus_events: 0,
      clock_lost_events: 0,
    }),
  );
  try {
    const client = new AgentControlClient({ baseUrl: mock.url });
    const health = await client.agentHealth();
    assert.equal(health.state, "Ready");
    assert.equal(health.devices, 2);
  } finally {
    await mock.close();
  }
});

test("dispatch 类方法独立超时预算（10s vs query 3s）", async () => {
  // mock 延迟 300ms：query 预算 100ms 会超时，dispatch 预算 2s 应成功。
  const mock = await startMock(
    () => rpcOk({ command_id: "c1", status: { status: "executed" }, kind: "start_session" }),
    300,
  );
  try {
    const client = new AgentControlClient({
      baseUrl: mock.url,
      queryTimeoutMs: 100,
      dispatchTimeoutMs: 2_000,
    });
    await assert.rejects(() => client.runtimeQuery());
    const verdict = (await client.dispatchCommand({
      command_id: "c1",
      kind: "start_session",
      target: { target_type: "session", intent: { version: "1.0", devices: [] } },
      requested_by: "cp-01a-contract-test",
    })) as Record<string, unknown>;
    assert.equal(verdict.command_id, "c1");
    // dispatch envelope 携带 params（ApiCommandRequest 形状）
    const last = mock.requests.at(-1)!;
    assert.equal(last.body.method, "command.dispatch");
    assert.deepEqual(Object.keys(last.body.params ?? {}), [
      "command_id",
      "kind",
      "target",
      "requested_by",
    ]);
  } finally {
    await mock.close();
  }
});

test("默认超时预算冻结值（C2：query ≤3s、dispatch ≤10s）", () => {
  assert.equal(QUERY_TIMEOUT_MS, 3_000);
  assert.equal(DISPATCH_TIMEOUT_MS, 10_000);
});

test("events.projection: 守门字段校验 + envelope", async () => {
  const mock = await startMock(() =>
    rpcOk({
      snapshot_kind: "event_projection_snapshot",
      total: 3,
      kind_counts: { session_started: 3 },
      session_states: {},
      session_failures: {},
      has_critical: false,
    }),
  );
  try {
    const client = new AgentControlClient({ baseUrl: mock.url });
    const proj = await client.projectEvents();
    assert.equal(proj.total, 3);
    const bad = await startMock(() => rpcOk({ total: 1, has_critical: false }));
    try {
      const badClient = new AgentControlClient({ baseUrl: bad.url });
      await assert.rejects(
        () => badClient.projectEvents(),
        (err: unknown) =>
          err instanceof AgentTransportFailure && err.causeKind === "invalid-result",
      );
    } finally {
      await bad.close();
    }
  } finally {
    await mock.close();
  }
});
