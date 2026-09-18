# RH-RES-01A — per-claim Reservation TTL 收口报告

日期：2026-09-18
Runtime commit：`0718757e42146cb41bd313e227f5fc7f19f3a171`
任务：关闭 `PHASE_0_7A_POST_MERGE_DEBT.md` D3。

## 1. Authority / Scope

Authority 为 `.project/STATE.md`、`RUNTIME_RESOURCE_MODEL.md §4.2` 与 D3 债务条目。冻结语义保持：

`Reserve → Renew → Expire / Release / Abort / Recover`

Resource Registry 继续是 Resource state 的唯一 owner；本任务不新增 scheduler、registry、Backend 行为或 wire vocabulary。

## 2. Before

旧实现把 Reservation 过期近似为 Session `created_at_ms` + `reservation_window_ms`，由 `SessionManager::tick()` 判断整个 Reserved/Leased Session 是否“停留过久”。Resource claim 本身没有 TTL，因此 Session 时间与 Resource state 分离。

## 3. Implementation

- `Reservation` 增加 Runtime 内部 `expires_at_ms`，`#[serde(skip)]`，不改变既有 JSON/wire。
- `acquire(req, now, ttl)` 在 Registry 锁域内给每个 claim 建立独立 deadline。
- `renew_claims()` 对 expected claims 做同锁域全量校验；缺失、错 holder、已到 TTL 任一成立即零修改失败。
- `expire_due(now)` 只过期各 claim 自己的 TTL；`abort_reservations_of(holder)` 显式处理中止。
- Session `created_at_ms` TTL 判断删除；Binding 完成作为真实进度点统一 Renew。
- `tick()` 将 Registry 返回的 expired claim 归并到 holder；同 Session 其余 Reserved siblings Abort，leases 回收，Session 收敛到 Terminated。
- `reservation_window_ms` 字段保留兼容，但语义改为每 claim Reservation TTL。

## 4. Failure-first coverage

新增/收紧测试验证：

- 不同 claim 独立 Expire；
- wrong-holder Renew fail-closed；
- TTL 已到但尚未 scan 的 claim 不得 Renew 复活；
- 多 claim Renew all-or-none，失败不得部分延长 sibling deadline；
- 任一 claim Expire 后，同 Session sibling claims Abort；
- Session lease/resource 零孤儿并进入 Terminated；
- serialized Resource 不出现 `reservation.expires_at_ms`；
- 原 resource conflict、partial allocation、stop failure、lease renew、event projection 回归继续成立。

## 5. Verification

Development VM：

- D3 Resource focused：**7/7 PASS**
- Session `resource_rt_01_*`：**6/6 PASS**
- mock lib：**432/432 PASS**
- mock integration：**9/9 + 12/12 PASS**
- default lib：**248/248 PASS**
- `cargo fmt --check`：PASS
- default/mock `cargo clippy --all-targets -- -D warnings`：PASS

GitHub Actions：run `35293309933` @ `0718757…`，7/7 required jobs **PASS**，含 `gstreamer-build` 与 `hardware-test-compile`。

## 6. Hardware / Contract verdict
本 packet 未修改 DeckLink/GStreamer adapter 或媒体数据面；BMD runtime/hardware smoke **NOT REQUIRED**，不得写成 hardware verified。

无 frozen Contract 变化。D3 的已冻结 Reservation lifecycle 由近似实现收敛为真实 per-claim TTL ownership。

**Verdict：RH-RES-01A COMPLETE / SOFTWARE + CI VERIFIED。**
