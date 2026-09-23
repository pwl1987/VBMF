import test from "node:test";
import assert from "node:assert/strict";
import {
  AgentWireSessionIdError,
  assertCanonicalUuid,
  wireSessionIdToUuid,
} from "../src/lib/sessionIds.ts";

test("wireSessionIdToUuid: valid session-<hex32> becomes canonical UUID", () => {
  assert.equal(
    wireSessionIdToUuid("session-11111111222233334444555566667777"),
    "11111111-2222-3333-4444-555566667777",
  );
});

test("wireSessionIdToUuid: non-string, wrong prefix, wrong length, uppercase all rejected", () => {
  for (const bad of [
    42,
    null,
    "11111111-2222-3333-4444-555566667777", // canonical UUID is NOT the wire form
    "session-1111111122223333444455556666777", // 31 hex
    "session-111111112222333344445555666677777", // 33 hex
    "session-1111111122223333444455556666777g", // 非 hex 字符
    "session-11111111222233334444555566667777 ",
    "SESSION-11111111222233334444555566667777",
    "",
  ]) {
    assert.throws(() => wireSessionIdToUuid(bad), AgentWireSessionIdError);
  }
});

test("assertCanonicalUuid: accepts canonical form only", () => {
  assert.equal(
    assertCanonicalUuid("11111111-2222-3333-4444-555566667777"),
    "11111111-2222-3333-4444-555566667777",
  );
  for (const bad of [
    "session-11111111222233334444555566667777", // display form is label-only on Product wire
    "11111111222233334444555566667777",
    "11111111-2222-3333-4444-55556666777", // 35 chars
    "1111111g-2222-3333-4444-555566667777", // 非 hex 字符
    "",
    "not-a-uuid",
  ]) {
    assert.throws(() => assertCanonicalUuid(bad), (err: unknown) => err instanceof Error);
  }
});
