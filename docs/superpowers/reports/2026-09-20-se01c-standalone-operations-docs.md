# SE-01C 收口报告 — standalone 运维语义文档（readiness/liveness · 配置权威清单 · 退出码契约）

**Packet**: STANDALONE-ENTRY-01 / SE-01C（冻结计划 S4/S8/S10 文档面）· **日期**: 2026-09-20 · **性质**: docs-only（零 Runtime 代码改动，`/health` wire 零变化）

## 1. 交付物

1. `docs/architecture/STANDALONE_MEDIA_AGENT_OPERATIONS.md`（新，语义权威页）：
   - §1 `/health` as-is 契约（固定 HTTP 200 + 五字段；bind 默认/覆盖；wire 冻结声明；health bind 失败仅日志、进程继续的如实披露）。
   - §2 Liveness/Readiness 语义 + **八态全矩阵**（每态：process live? / operationally ready? / 现实现生产可达性 / 操作员含义 / consumer 动作）+ 按启动路径的可达性披露。
   - §3 退出码契约：production `media-agent`（0=优雅停止；2=九类 fail-closed 启动拒绝，逐一对应真实 callsites；panic=101 非契约；第二信号=逃生门非 exit 0；SIGHUP 不 reload）与 `media-agent-gates`（0/1/2 acceptance tooling 语义）**分开成文**。
   - §4 配置权威清单四组（production `MEDIA_AGENT_*` 11 项 / `VBMF_OUTPUT_*` 5 项 / 身份与运行环境 4 项 / gates 专属 acceptance env 全表），逐项：变量｜消费位置｜语义｜默认｜production 是否允许｜缺失/非法行为｜startup-only；含启动模式互斥规则汇总与"gate env 不得进 production unit"红线。
2. `docs/architecture/DEPLOYMENT_AND_DEV_RUNTIME.md` §17 Standalone Lane 增补（只增补 lane，不改 full-stack lane 与 `ops/` 占位；install/unit 留 SE-01B）。
3. `docs/architecture/README.md` 索引登记。

## 2. 关键裁定与如实披露

- **S4 reconciliation（§2.2）**：冻结 S4"非 Starting/ManualRequired 即 ready"会把 Degraded/Restarting/Backoff/Escalated 含糊计入 ready；按现行权威（`health.rs` 优先级格 + Supervisor Escalate→manual_required 唯一出口）收敛为 **ready ⟺ Ready|Capturing**。判据收紧、非 wire/状态机变更；已在 STATE §3.63 立档。
- **词表诚实**：`Restarting`/`Backoff` 无事件生产者、`Escalated` 无 /health 构造点（Supervisor Escalated 以 `HealthChanged{to:"manual_required"}` 呈现为 ManualRequired）——沿用 `health.rs` 模块头既有登记；矩阵为预留态预先钉死判据。
- **按路径可达性**：Network-only 生产路径 `state` 现仅 Starting→Ready（`projection_log` 尚未接线 fold 消费者，网络源故障 Degraded 派生属 watchdog 演进项）；Device 命令服务路径经 `spawn_ingest_watchdog` 折叠，四态真实可达。
- **machine identity 按现实现写**：`VBMF_MACHINE_ID` > `HOSTNAME` > 空串=跳过 pin 校验；未提前实现 SE-01D 的 `/etc/machine-id`。
- `MEDIA_AGENT_RPC_BIND` 标注 UNWIRED；`VBMF_OUTPUT_*` 标注 P1a demo 层（显式不进 Runtime Contract）——均按代码现状。

## 3. 验证

- code-to-doc cross-check：Group A 11 个 `MEDIA_AGENT_*`、Group B 5 个 `VBMF_OUTPUT_*` 与 `grep` 全仓枚举**逐一致**（无遗漏无多余）；gates env 表与 `bin/gates.rs` usage 提示及各 gate 模块 `env::var` 一致。
- `/health` wire 零变化：transport 既有测试全 PASS（default 324/324、mock 536/536、ffmpeg-backend 362 测试含 1 ignored，本轮回归运行）。
- architecture lint / remove-adapters proof PASS；fmt/clippy（本地可编译档位）不受影响（docs-only）。
- CI 见 STATE §7 对应行；三方 HEAD 对齐见 STATE 收口段。
- 如实披露：`scripts/check_docs.py` 在本 VM 存在病态挂起（100% CPU，另有 Sep14/17 孤儿进程），未作为本轮门禁；该脚本不在 7 个 CI required context 内。

## 4. 边界与后续

- 零代码改动、零 wire 变化、零 Control Plane/API 扩面；未触碰 frozen Architecture 语义（S4 判据收敛为用户授权的 reconciliation 记录）。
- 后续按冻结顺序：**SE-01D**（machine-id 收敛，Runtime 行为变化，独立 commit + 独立验收）→ **SE-01B**（install/systemd/BMD 部署对账执行）。
