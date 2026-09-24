/**
 * CP-01C RBAC 单元测试（CASL evaluator；hermetic）。
 * 契约：权限由 Product API action/resource 语义驱动；默认 fail-closed；
 * 未知角色/未知 action/resource 不隐式放行。
 */
import test from "node:test";
import assert from "node:assert/strict";
import {
  ROUTE_PERMISSIONS,
  authorizationDecision,
} from "../src/security/rbac.ts";

test("viewer：runtime/command/events read 允许；session 写动作拒绝", () => {
  assert.equal(authorizationDecision("viewer", ROUTE_PERMISSIONS.runtimeRead).allowed, true);
  assert.equal(authorizationDecision("viewer", ROUTE_PERMISSIONS.commandRead).allowed, true);
  assert.equal(authorizationDecision("viewer", ROUTE_PERMISSIONS.eventsRead).allowed, true);
  assert.equal(authorizationDecision("viewer", ROUTE_PERMISSIONS.sessionStart).allowed, false);
  assert.equal(authorizationDecision("viewer", ROUTE_PERMISSIONS.sessionStop).allowed, false);
  assert.equal(authorizationDecision("viewer", ROUTE_PERMISSIONS.sessionRelease).allowed, false);
});

test("operator：全部当前 Product API 动作允许", () => {
  for (const spec of Object.values(ROUTE_PERMISSIONS)) {
    assert.equal(authorizationDecision("operator", spec).allowed, true, `${spec.action} ${spec.resource}`);
  }
});

test("fail-closed：未知角色全拒绝", () => {
  for (const role of ["", "admin_x", "SUPERUSER", "operator ", "root"]) {
    for (const spec of Object.values(ROUTE_PERMISSIONS)) {
      assert.equal(authorizationDecision(role, spec).allowed, false, `role=${role}`);
    }
  }
});

test("fail-closed：缺失角色（NULL/undefined 形态）全拒绝", () => {
  for (const role of [null, undefined]) {
    for (const spec of Object.values(ROUTE_PERMISSIONS)) {
      // 角色缺失在 Principal 层已归一为空串——模拟该归一。
      assert.equal(authorizationDecision(role === null || role === undefined ? "" : role, spec).allowed, false);
    }
  }
});

test("ROUTE_PERMISSIONS 覆盖当前 Product/Event API 全部六条路由语义", () => {
  assert.deepEqual(ROUTE_PERMISSIONS.runtimeRead, { action: "read", resource: "runtime" });
  assert.deepEqual(ROUTE_PERMISSIONS.commandRead, { action: "read", resource: "command" });
  assert.deepEqual(ROUTE_PERMISSIONS.sessionStart, { action: "start", resource: "session" });
  assert.deepEqual(ROUTE_PERMISSIONS.sessionStop, { action: "stop", resource: "session" });
  assert.deepEqual(ROUTE_PERMISSIONS.sessionRelease, { action: "release", resource: "session" });
  assert.deepEqual(ROUTE_PERMISSIONS.eventsRead, { action: "read", resource: "events" });
});
