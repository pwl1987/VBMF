/**
 * CP-01C 应用层限流（planning C11 + EXTERNAL_API_CONTRACT §6 默认 rate-limited）。
 *
 * 语义键 = (bucket, principalId)——区分 endpoint/action class（read/write）与
 * 认证主体（API key identity context）；绝不做全局单计数器。单实例内存滑动
 * 窗口（bounded packet；planning 未要求 distributed/PG-backed；Nginx 粗限流
 * 不能替代本层语义限流，二者互补）。
 *
 * 内存有界：每个 key 只保留窗口内时间戳；key 总量有上限（超限先淘汰最旧
 * 活跃 key），防伪造 principal 撑爆内存。时钟可注入（测试确定性）。
 */
export interface RateLimitOptions {
  /** 滑动窗口长度（毫秒）。 */
  windowMs: number;
  /** bucket → 窗口内最大请求数（如 read/write 两桶不同限额）。 */
  maxByBucket: Record<string, number>;
  /** 追踪 key 上限（(bucket, principal) 对数）；默认 50_000。 */
  maxTrackedKeys?: number;
  /** 测试注入时钟。 */
  now?: () => number;
}

export interface RateLimitDecision {
  allowed: boolean;
  /** 拒绝时：窗口最早一条记录滑出后的剩余等待（毫秒，>0）。 */
  retryAfterMs: number;
  /** 允许时：本窗口剩余额度（含本次）。 */
  remaining: number;
}

export class SlidingWindowRateLimiter {
  private readonly windowMs: number;
  private readonly maxByBucket: Record<string, number>;
  private readonly maxTrackedKeys: number;
  private readonly now: () => number;
  /** key → 滑动窗口内命中时间戳（升序）。 */
  private readonly hits = new Map<string, number[]>();

  constructor(opts: RateLimitOptions) {
    if (!Number.isInteger(opts.windowMs) || opts.windowMs <= 0) {
      throw new Error("rateLimit windowMs must be a positive integer");
    }
    if (Object.keys(opts.maxByBucket).length === 0) {
      throw new Error("rateLimit maxByBucket must define at least one bucket");
    }
    for (const max of Object.values(opts.maxByBucket)) {
      if (!Number.isInteger(max) || max <= 0) {
        throw new Error("rateLimit bucket max must be a positive integer");
      }
    }
    this.windowMs = opts.windowMs;
    this.maxByBucket = { ...opts.maxByBucket };
    this.maxTrackedKeys = opts.maxTrackedKeys ?? 50_000;
    this.now = opts.now ?? Date.now;
  }

  check(bucket: string, principalId: string): RateLimitDecision {
    const max = this.maxByBucket[bucket];
    if (max === undefined) {
      // 未定义限额的 bucket = 配置缺陷，fail-closed 拒绝（绝不隐式放行）。
      return { allowed: false, retryAfterMs: this.windowMs, remaining: 0 };
    }
    const key = `${bucket}\u0000${principalId}`;
    const cutoff = this.now() - this.windowMs;

    let hits = this.hits.get(key);
    if (hits === undefined) {
      // 内存上界：只在新 key 需要登记且已满时淘汰最旧 key（其窗口记录最陈旧）。
      if (this.hits.size >= this.maxTrackedKeys) {
        const oldest = this.hits.keys().next();
        if (!oldest.done) this.hits.delete(oldest.value);
      }
      hits = [];
      this.hits.set(key, hits);
    }
    while (hits.length > 0 && hits[0]! <= cutoff) {
      hits.shift();
    }
    if (hits.length >= max) {
      const retryAfterMs = hits[0]! + this.windowMs - this.now();
      return { allowed: false, retryAfterMs: Math.max(1, retryAfterMs), remaining: 0 };
    }
    hits.push(this.now());
    return { allowed: true, retryAfterMs: 0, remaining: max - hits.length };
  }

  /** 测试/运维钩子：清空全部计数（不影响窗口配置）。 */
  reset(): void {
    this.hits.clear();
  }
}
