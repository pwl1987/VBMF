# Verify 报告 — p07-session-runtime (Phase 0.7A: Session Runtime)

- 日期: 2026-08-29
- 分支: `comet/p07-session-runtime`（自 master `d1cfaa9` = Phase 0.6 Runtime Abstraction Baseline 拉出）
- 验证模式: **full**；盒 10.30.15.10 为准
- 产物: Design Doc `docs/superpowers/specs/2026-08-29-p07-session-runtime-design.md`；Delta spec 无（skip_specs）
- 契约对齐: RUNTIME_SESSION_MODEL / RUNTIME_RESOURCE_MODEL / RUNTIME_LIFECYCLE_SEQUENCE / MEDIA_BACKEND_CONTRACT §1.1 / V0.2 §1.2·§3.11（全部 FROZEN，实现无新发明语义）

## Summary

| Dimension | Status |
|-----------|--------|
| Completeness | 6 任务组 13 项全落地（四栏纪律 Contract/Implementation/Verification/Gate 全 Pass） |
| Correctness | 盒上 final 矩阵：fmt 0 · test **115/115/127/115** · clippy -D ×4 零警告 · build ×3（含 hardware-test）· PROOF PASS · **真机 SESSION-RT-01/RESOURCE-RT-01 ALL PASS** |
| Coherence | 实现逐条对齐 D1-D11 与冻结契约；0 处契约违背；bootstrap 租约让位为接线修正 |

**结论: PASS — 0 CRITICAL / 0 IMPORTANT / 2 NOTE。**

## 1. 完整验证 7 项

| # | 检查项 | 结果 | 证据 |
|---|--------|------|------|
| 1 | tasks 全部完成 | ✅ | 全 `[x]`，四栏纪律表完整（用户 §十五 首次执行） |
| 2 | 符合 open design.md | ✅ | D1-D11 逐项落地；非目标未越界 |
| 3 | 符合 Design Doc | ✅ | 见 §4 Divergence（2 处细化） |
| 4 | 能力规格场景 | ✅ (N/A) | skip_specs；门禁测试即场景 |
| 5 | proposal 目标满足 | ✅ | Session/Orchestration/Lease/Preflight/两门禁/CI 全交付 |
| 6 | delta spec 无矛盾 | ✅ (N/A) | 无 delta spec |
| 7 | Design Doc 可定位 | ✅ | frontmatter 关联本 change |

## 2. 门禁逐项验收

| 门禁 | Unit/Simulation（盒） | Hardware（盒 10.30.15.10） |
|------|----------------------|---------------------------|
| SESSION-RT-01 | 全链 create→start→running→stop→release；double-start/double-stop 拒绝；instantiate/start 失败注入 → **零孤儿断言**（资源回 Available + 无残留租约） | `VBMF_SESSION_LIFECYCLE=1`（真机 loopback manifest, bmd+gstreamer）：create OK → start OK (pipeline Running) → observe 10s OK → stop OK → **ALL PASS exit 0**。`MediaBackend::stop` 首次真实调用并验证 |
| RESOURCE-RT-01 | 双会话争同资源：第二会话 Preflight/资源占用 fail-closed 拒绝且第一会话 Running 不受影响；release→重占；tick 过期回收（Terminated 零孤儿）；renew 续期回写 | 同入口第二会话冲突被拒 → `RESOURCE-RT-01 verdict=OK` |

## 3. 盒上最终矩阵（final code, 全绿）

fmt apply/check **0** · test **115 (default) / 115 (simulation) / 127 (mock) / 115 (bmd,gstreamer)**（基线 110/110/111/110 → 净增 5+16 项：session 8 + preflight 2 + resource 3 + lease 1 + events 1 + mock 既有）· clippy -D ×4 零警告 · build gstreamer-only/bmd,gstreamer/hardware-test ×3 · remove-adapter PROOF PASS。

调试轨迹：7 轮盒上迭代（R1 E0277 LeaseManager Send+Sync + E0282 闭包标注 / R2 preflight 测试遮蔽 + intent 传参 / R3 clippy collapsible+lazy-eval / R4 `as u64 <` 泛型歧义 + 非 gst unused / R5 ok_or_else 收尾 / R6 毫秒竞态 / R7 tick 漏回写租约）。另：真机首轮暴露 **bootstrap 占位租约与会话租约的接管关系**——Preflight LeaseConflict 正确 fail-closed 拒绝（门禁反向实证），入口补"bootstrap 让位"接线后全链通过。

## 4. 代码审查（review_mode=standard）+ Divergence

- **改动面**：新 `session.rs`（SessionId/SessionState/SessionPhase/MediaSession/SessionManager + 8 测试）、新 `preflight.rs`（8 stage + 2 测试）；`resource.rs`（allocate_for/release_allocation/expire_reservations_of + 2 测试）、`lease.rs`（renew + Send+Sync + 1 测试）、`events.rs`（3 additive kinds + 1 测试）、`main.rs`（auto_start 重接线 + VBMF_SESSION_LIFECYCLE 入口）、CI（session-lifecycle job）。
- **正确性**：生命周期顺序=冻结序（LIFECYCLE §1）；creator=destroyer（close 兜底零孤儿）；fail-closed 全链（Preflight FAIL 零预留；start 失败逆序回滚；bootstrap 冲突真机实证拒绝）。115/115/127/115 为最终证据。
- **安全**：无 secrets/unsafe；renew 拒绝跨 owner 抢占；allocate_for 校验 holder；P0-8（Backend 只消费已授权资源）由 Manager 单点注入保持。
- **Divergence（记录于 Design Doc §14 精神下，实际 2 处细化）**：① `LeaseManager` 增 `Send+Sync` supertrait（SessionManager 跨线程持有的必要条件，Mutex 实现天然满足）；② tick 的预留 TTL 以"Reserved 相位停留窗口"近似（per-claim TTL 记 0.7 债务，Design Doc §10 已列）。
- **结论**：0 CRITICAL / 0 IMPORTANT。

## 5. NOTE

- **NOTE-1**：真机门禁首轮 FAIL 为**门禁反向实证**：bootstrap 占位租约与会话租约冲突被 Preflight 正确拒绝——入口补"bootstrap 让位"接线后全链通过。这正是 Preflight 分级判定第一次在真机工作时抓住的真实接管冲突。
- **NOTE-2**：CI 将新增第 7 个 required context `session-lifecycle`（workflow job 已落，随本 PR 生效）。

## 6. 交付路径

archive → 单一 PR `comet/p07-session-runtime` → `master`（gh）→ protection 增 `session-lifecycle` → merge 后删分支（新分支纪律）。后续 0.7A 余项与 0.7B/0.7C 依 roadmap 分段推进。
