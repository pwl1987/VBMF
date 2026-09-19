# RF-SRC-RTMP-02 TG-5 — Redaction Helper + Unified Negative Suite

- Date: 2026-09-19
- Packet: `RF-SRC-RTMP-02-IMPLEMENTATION` Step 5 / TG-5（前置：TG-4 `11af233`）
- Design authority: `docs/superpowers/plans/2026-09-19-rf-src-rtmp-02-production-rtmp-input-boundary.md`（D9 统一负向测试 + audit appendix）
- Scope（本轮改动）：`pipeline.rs`（`PipelinePlan::redacted_debug` redaction helper + 端点无关性单测）、`session.rs`（D9 点名的 Debug-hash 路径改用 redacted 渲染 + 统一负向套件）。D10 负向套件维持 TG-2 版本并在矩阵中全绿（复核无缺口）。

## 1. 交付内容

### Redaction helper（`PipelinePlan::redacted_debug`）

- 确定性、无 endpoint 文本的计划渲染：`SourcePlan::Network` 的 endpoint 与 `OutputPlan.target`（HLS 路径 / `rtmp://` URL）结构性替换为固定 marker；保留 canonical 语义（source 类别 + canonical ids + 策略 + 输出 kind/码率）。
- **session.rs SourceMaterialized 身份哈希修复**（D9 点名路径）：`Uuid::new_v5(nil, format!("{:?}", plan))` 的输入是派生 Debug——Network 计划会把 host/port/path 文本放进哈希输入（端点依赖身份泄漏，可对候选 endpoint 暴力比对）。改用 `plan.redacted_debug()`：仅 endpoint 不同的两计划哈希输入相同（身份只随 canonical 语义变化）。

### 统一负向套件（`rf_src_rtmp_02_tg5_unified_redaction_negative_suite`，mock 域全生命周期）

以特征字面量（host `10.30.15.10`、port `19350`、path `/live/source`、`rtmp://`）断言以下 canonical 面全部缺席：

1. `runtime_state()` 投影 JSON（Production LAN session create→start→stop 全周期后）；
2. canonical 强类型 Debug（`CanonicalRtmpEndpoint`/`CanonicalIp`/`CanonicalPath`/`ListenerKey`——含 "redacted" 标记断言）；
3. `redacted_debug` 渲染 + **端点无关性**（同 source 不同 endpoint/输出 target → 相同渲染）；
4. 失败路径：未授权五元组 → `PreflightReport` JSON、`SessionError` Display；
5. `NetworkBindingError` 全部 23 变体 Display 扫描；
6. 事件日志全量 drain 的逐事件 `RuntimeEvent` JSON 序列化（含修复后的 SourceMaterialized 身份哈希路径）。

另附 default 域 `redacted_debug` 单测（endpoint/输出 target 无泄漏 + 稳定性）。wire 入站例外（`SourceIntent::Rtmp` 携带已授权 endpoint 形状、拒绝凭据）由既有 wire 兼容测试锚定，不在套件中重复。

### D10 负向复核

TG-2 的 D10 套件（Device lease 集合空/仅 Network Resource/manifest digest 不变/失配 fail-closed）在本轮全矩阵保持全绿；无缺口，无需新增。

## 2. 软件验证（Development VM, Rust 1.98.1）

| 项 | 结果 |
|---|---|
| focused `rf_src_rtmp_02_tg5` | default 1 + mock 2（含统一套件） |
| default lib | **318/318** |
| simulation lib | **318/318** |
| ffmpeg-backend lib | **350 passed / 1 ignored** |
| mock lib + integration | **509/509** + **9/9 + 12/12** |
| clippy `-D warnings` | default / mock / ffmpeg-backend 三档 PASS |
| `cargo fmt --check` / `check --all-targets` | PASS |
| architecture lint + remove-adapters proof | PASS / PROOF OK |
| `git diff --check` | PASS（仅 `pipeline.rs` + `session.rs`） |

## 3. 边界与遗留

- 证据命令行 redact（evidence 侧策略）随 TG-6 BMD 验收报告执行；`to_url`/`as_str` 显式例外（argv/manifest 输入）维持不变。
- 下一步 = TG-6：BMD exact-commit Tier 1（Tier 2 如可用）验收——LAN listener bind/断连恢复/手动路径/deferred risks 逐项标注 + 证据归档 + STATE/CI 收口。
