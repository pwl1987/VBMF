import test from "node:test";
import assert from "node:assert/strict";
import { canonicalCommandId, uuidV5 } from "../src/command/commandIds.ts";
import { commandFingerprint } from "../src/command/fingerprint.ts";

test("v5 派生与 Rust uuid crate 逐字节一致（实测锚定·transport COMMAND_ID_NAMESPACE）", () => {
  // 锚值来源：Rust `Uuid::new_v5(&COMMAND_ID_NAMESPACE, s.as_bytes())` 实机输出。
  assert.equal(uuidV5("not-a-uuid"), "447c1880-7192-525f-9d3a-10d0e5957ab6");
  assert.equal(uuidV5("cp01b-cross-check"), "27586aa1-b1d6-55d8-95d1-14e623ce264e");
  assert.equal(uuidV5("同一命令键-中文"), "97a5ecfb-9ca2-5551-9cb4-c74a27e4a649");
});

test("canonicalCommandId：合法 UUID 直用（Rust parse 分支等价）", () => {
  assert.equal(
    canonicalCommandId("11111111-1111-1111-1111-111111111111"),
    "11111111-1111-1111-1111-111111111111",
  );
  assert.equal(canonicalCommandId("not-a-uuid"), "447c1880-7192-525f-9d3a-10d0e5957ab6");
});

test("fingerprint：canonical 键序稳定性 + principal 参与（同键异主体必不同）", () => {
  const a = { target_type: "session", intent: { devices: [1], version: "1.0" } };
  const b = { intent: { version: "1.0", devices: [1] }, target_type: "session" };
  assert.equal(
    commandFingerprint("start_session", a, "p1"),
    commandFingerprint("start_session", b, "p1"),
    "键序不同语义相同 → 同 fingerprint",
  );
  assert.notEqual(
    commandFingerprint("start_session", a, "p1"),
    commandFingerprint("start_session", a, "p2"),
    "同键不同主体 = 不同 fingerprint（C6）",
  );
  assert.notEqual(
    commandFingerprint("start_session", a, "p1"),
    commandFingerprint("stop_session", a, "p1"),
  );
});
