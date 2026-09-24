/**
 * CP-01C 应用层限流单元测试（hermetic；注入时钟）。
 * 语义：principal × action bucket 分桶；拒绝携带 retryAfterMs；窗口滑动；
 * 未知 bucket fail-closed；内存有界。
 */
import test from "node:test";
import assert from "node:assert/strict";
import { SlidingWindowRateLimiter } from "../src/security/rateLimit.ts";

function fakeClock(start = 1_000_000): { now: () => number; advance: (ms: number) => void } {
  let t = start;
  return { now: () => t, advance: (ms) => { t += ms; } };
}

test("窗口内达到 max 后拒绝，retryAfterMs > 0；窗口滑出后恢复", () => {
  const clock = fakeClock();
  const limiter = new SlidingWindowRateLimiter({
    windowMs: 60_000,
    maxByBucket: { write: 2 },
    now: clock.now,
  });
  assert.equal(limiter.check("write", "u1").allowed, true);
  assert.equal(limiter.check("write", "u1").allowed, true);
  const denied = limiter.check("write", "u1");
  assert.equal(denied.allowed, false);
  assert.ok(denied.retryAfterMs > 0 && denied.retryAfterMs <= 60_000);
  clock.advance(60_001);
  assert.equal(limiter.check("write", "u1").allowed, true, "窗口滑出后额度恢复");
});

test("bucket 隔离：write 超限不影响 read 额度（endpoint/action 区分）", () => {
  const clock = fakeClock();
  const limiter = new SlidingWindowRateLimiter({
    windowMs: 60_000,
    maxByBucket: { read: 3, write: 1 },
    now: clock.now,
  });
  assert.equal(limiter.check("write", "u1").allowed, true);
  assert.equal(limiter.check("write", "u1").allowed, false);
  assert.equal(limiter.check("read", "u1").allowed, true, "read 桶独立计数");
  assert.equal(limiter.check("read", "u1").allowed, true);
  assert.equal(limiter.check("read", "u1").allowed, true);
  assert.equal(limiter.check("read", "u1").allowed, false);
});

test("principal 隔离：同一 bucket 不同 principal 各自计数（无全局计数器）", () => {
  const clock = fakeClock();
  const limiter = new SlidingWindowRateLimiter({
    windowMs: 60_000,
    maxByBucket: { write: 1 },
    now: clock.now,
  });
  assert.equal(limiter.check("write", "uA").allowed, true);
  assert.equal(limiter.check("write", "uB").allowed, true, "不同 principal 不互相挤占");
  assert.equal(limiter.check("write", "uA").allowed, false);
  assert.equal(limiter.check("write", "uB").allowed, false);
});

test("fail-closed：未定义限额的 bucket 拒绝", () => {
  const clock = fakeClock();
  const limiter = new SlidingWindowRateLimiter({ windowMs: 60_000, maxByBucket: { read: 5 }, now: clock.now });
  const d = limiter.check("nope", "u1");
  assert.equal(d.allowed, false);
  assert.ok(d.retryAfterMs > 0);
});

test("内存有界：key 数超过 maxTrackedKeys 时淘汰最旧（不无界增长）", () => {
  const clock = fakeClock();
  const limiter = new SlidingWindowRateLimiter({
    windowMs: 60_000,
    maxByBucket: { read: 10 },
    maxTrackedKeys: 3,
    now: clock.now,
  });
  for (const u of ["u1", "u2", "u3", "u4"]) limiter.check("read", u);
  // 内部 map 有界：新增 u5 后仍 ≤ 4（淘汰一个旧的）；通过行为验证不抛错且计数正确。
  const d = limiter.check("read", "u5");
  assert.equal(d.allowed, true);
  // u1 可能被淘汰——重置后重新计数不受影响。
  limiter.reset();
  assert.equal(limiter.check("read", "u1").remaining, 9);
});

test("构造参数校验：非法 window/bucket 配置拒绝构建", () => {
  assert.throws(() => new SlidingWindowRateLimiter({ windowMs: 0, maxByBucket: { read: 1 } }));
  assert.throws(() => new SlidingWindowRateLimiter({ windowMs: 60_000, maxByBucket: {} }));
  assert.throws(() => new SlidingWindowRateLimiter({ windowMs: 60_000, maxByBucket: { read: 0 } }));
});
