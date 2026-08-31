# Verify 报告 — p07c-command-contract (Phase 0.7C-3: Command Contract Foundation)

- 日期: 2026-08-31
- 分支: `comet/p07c-command-contract`（自 master `4cbaff7` = 0.7C-2 拉出）
- 验证模式: **full**；盒 10.30.15.10 为准
- 契约对齐: PHASE_IMPLEMENTATION_MAP §3（Command Contract 项）；终审执行令（**第一红线：Command 不可执行性**；Query/Command 分离；禁万能 Executor）

## Summary

| Dimension | Status |
|-----------|--------|
| Completeness | 4 任务组 11 项全落地（四栏纪律全 Pass） |
| Correctness | 盒上 final 矩阵：fmt 0 · test **138/138/170/138** · clippy -D ×4 零警告 · build ×3 · PROOF PASS · 真机回归 ALL PASS + envelope 驱动全 Executed |
| Coherence | 不可执行性三重守护 + Query/Command 分离白盒；零新 DTO（envelope 复用 canonical GraphRuntimeIntent）；无 Executor/Bus/Idempotency |

**结论: PASS — 0 CRITICAL / 0 IMPORTANT / 2 NOTE。**

## 1. 完整验证 7 项

| # | 检查项 | 结果 | 证据 |
|---|--------|------|------|
| 1 | tasks 全部完成 | ✅ | 四栏纪律表全 Pass |
| 2 | 符合 open design.md | ✅ | D1-D8 逐项落地；不做清单零越界 |
| 3 | 符合 Design Doc | ✅ | §1-§5 实现级一致（banned 列表精化为 NOTE-1） |
| 4 | 能力规格场景 | ✅ (N/A) | skip_specs |
| 5 | proposal 目标满足 | ✅ | vocabulary/envelope/target/outcome/validation/dispatch + 三重守护 + 三层门禁 |
| 6 | delta spec 无矛盾 | ✅ (N/A) | 无 delta spec |
| 7 | Design Doc 可定位 | ✅ | frontmatter 关联本 change（design guard PASS） |

## 2. 盒上最终矩阵（final code, 全绿）

fmt apply/check **0** · test **138 (default) / 138 (simulation) / 170 (mock) / 138 (bmd,gstreamer)**（0.7C-2 基线 138/138/165/138 → +5 command 门禁测试）· clippy -D ×4 零警告 · build ×3 · PROOF PASS。迭代：CC1（banned 列表误含 canonical 冻结键名 + intent move）→ CC2/3（clone 两处）→ CC4（enum_variant_names allow——命令域词汇表刻意同后缀）→ 全绿。

## 3. 门禁 COMMAND-CONTRACT-RT-01 逐层验收

| 层 | 测试/证据 |
|----|-----------|
| Unit | `vocabulary_snapshot`（三 kind serde 字符串，封闭词表防静默增删）· `non_executability_serde_and_surface`（envelope serde 反向断言零 gst/device_number/backend/handle/ffmpeg/alsa/kafka/nats 字样 + roundtrip + allowlist `[validate, dispatch]` 恒等 + 执行动词禁入）· `validation_rejection_paths`（empty_requester / kind_target_mismatch×2 / nil_session_id / empty_intent 四拒绝 + 通过路径） |
| Simulation | `simulation_full_lifecycle_and_failure_paths`：Rejected 不触 Runtime（会话表空）→ Start Executed（会话 Running）→ Stop Executed（Released）→ Failed（幽灵会话）→ Release Executed（会话移除）；`query_command_separation`（命令面无 get_/list_ 动词；反向由 0.7C-2 runtime_query allowlist 锁定） |
| Hardware | 真机 SESSION_LIFECYCLE command 驱动段：`start status=Executed` → `observe 10s running=true` → `stop status=Executed` → `release status=Executed`——**与直接 SessionManager 路径等价**（同轮 SESSION/RESOURCE-RT-01 ALL PASS） |

## 4. 代码审查（review_mode=standard）+ NOTE

- **改动面**：新 `command.rs`（类型 + validate 纯函数 + dispatch 薄映射 + 5 测试）；`main.rs`（mod + gate 命令段 + 两处 intent.clone()）；Phase Map。Session/Resource/Lease 语义零变更（dispatch 仅调公共 API）。
- **红线核验**：①不可执行性——类型层仅 canonical 类型 + serde 反向断言 + allowlist；②validation/execution 分离（validate 纯函数不触 Runtime，测试锁定 Rejected 后会话表空）；③无万能 Executor（dispatch = match 三臂，无循环/插件/总线）；④Query/Command 分离（两模块互不引用）。
- **NOTE-1（banned 列表精化）**：`pipeline` 从禁列移除——它是 canonical `GraphRuntimeIntent.devices[].pipeline: PipelineIntent` 的**冻结 schema 键名**（0.6 冻结），非执行细节；禁的是执行器/vendor 地址类字段（device_number/handle/backend 等）。测试注释已显式记录该区分。
- **NOTE-2**：`command_id` 为幂等键占位（携带不实现——D9 幂等语义属 0.7C 下一 change，与 Phase Map §3 Idempotency 项对齐）。
- **结论**：0 CRITICAL / 0 IMPORTANT。

## 5. 交付路径

archive → 单一 PR `comet/p07c-command-contract` → `master`（七 checks）→ merge → tag `phase-0.7C3-command-contract` → 删分支。0.7C §3 进度：Canonical Runtime State ✅ → Runtime Query Model ✅ → **Command Contract ✅** → 下一项 Idempotency（D9 并入）→ Error Model → Event Projection → External API。
