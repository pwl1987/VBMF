# Comet Design Handoff

- Change: p06-final-merge-hardening
- Phase: design
- Mode: compact
- Context hash: 5b9c94e644a20ad1c27c0a03ab3ef7458949cffcdadfa792c8c8e1bf8e5648de

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p06-final-merge-hardening/proposal.md

- Source: docs/openspec/changes/p06-final-merge-hardening/proposal.md
- Lines: 1-33
- SHA256: 7e35dcfbc30f7609dfbc4ef648c9cbb45ad42e25a4f251d3ea8a0da9a5fae18d

```md
# Change: Phase 0.6 Final Merge Hardening — 合并前清债与 Runtime Abstraction Baseline 收口

## Why

0.6 系列五个 change（bc/f/g/de/hi）已在 `comet/p06-hi-backend-resource-gates`（0f2a2bf，相对 master 29 commits ahead / 0 behind）形成串行聚合链，门禁全绿、真机证据齐备。但分支终审发现：**SPI 已是可实现的真 trait，但契约与实现尚未完全对齐**——`DeviceInfo` 仍泄漏 BMD 身份机制（违反 CANONICAL_IDENTITY.md "Provider Identity ≠ Canonical Identity"）、`MediaBackend` 实现契约仍带 GStreamer 遗留（`poll_bus`/缺 `stop`，与冻结契约 `instantiate/start/stop/recover/observe` 不一致）、trait 本身被 adapter feature 门控、Registry 对 feature 冲突静默降级。这些问题现在修成本最低；合入 master 后再修将形成 breaking 变更并污染 Baseline。因此：**合入 master 前做一次集中清债，然后以单一 PR 把 master 定义为 Phase 0.6 Runtime Abstraction Baseline。**

## What Changes

- **P0-1 DeviceInfo 去 BMD identity 泄漏（BREAKING，SPI 内部轴）**：`device.rs` 的 `bmd_persistent_id` / `bmd_device_handle` / `bmd_topological_id` 移出 Canonical Domain，收敛为 `adapters/blackmagic` 内的 Provider Identity（经 Provider Identity Adapter 映射为 canonical `DeviceId` + `IdentityStrength`/`IdentitySource`）；`resolver.rs` 匹配逻辑改为消费 Provider 侧身份证据结构，不再读 Domain 字段。
- **P0-2 MediaBackend 契约对齐冻结契约（BREAKING，SPI 方法轴）**：`prepare→instantiate`、`poll_bus→observe`（返回 canonical 事件集，非 GStreamer Bus Message 语义）、补 `stop`；同步对齐 `MEDIA_BACKEND_CONTRACT.md` 的形状并更新两实现 + 调用点（main.rs watchdog / supervisor 接线）。
- **P0-3 MediaBackend trait 去 adapter feature 门控**：`contracts/backend.rs` 的 trait 无条件编译（default 无任何 backend 实现时契约仍存在）；`impl` 块保留 feature 门控。
- **P0-4 Registry feature 冲突 fail-closed**：生产路径（非显式 simulation/mock 模式）下 `mock` 与 `bmd-provider`/`gstreamer-backend` 组合不再静默按优先级取 Mock，而是显式报错拒启；显式测试模式（env/flag）下允许。
- **P0-5 Architecture Gate 升级为真实编译验证**：新增 remove-adapter 编译验证（移除 `adapters/blackmagic` / `adapters/gstreamer` 后 Domain/Contract/Runtime 仍可编译），接入 CI；现有词法 lint 明确定位为 Architecture Lint（非 Proof）。
- **P0-6 Archived 证据旧口径勘误**：修正 p06-hi 归档 tasks/design 中 "`PipelineHealth` Default=true" 旧表述（实际为 P1-2 三态 + absence-of-evidence 全 false），以显式勘误注记保留历史可读性。
- **P1-1 MockBackend 弃用 `PipelineHandle(0)` 哨兵**：与生产同一 `PipelineId` 生成机制（`NEXT_PIPELINE_ID`，从 1 起），消除与哨兵值撞车的隐患。
- **P1-2 `HardwareProvider::discover` Result 化（BREAKING，对 `HARDWARE_PROVIDER_CONTRACT.md` 的显式修正）**：`discover(&self) -> Result<Vec<DeviceInfo>, ProviderError>`，SDK 不可用/驱动失败不再与"无硬件"（空 Vec）混淆；契约文档同步修正。
- **P1-3 RuntimeEvent 分级（critical/droppable）**：`RuntimeEventLog` 满时 fault/identity 类关键事件不可被静默挤出（至少两级语义：Durable 计数保底 / Observation 可丢弃）。
- **P1-4 Resource 原子占用最小原语**：`preflight + reserve` 在注册表锁内原子完成（`acquire`），materialize 失败有显式释放回滚；完整 Resource Orchestration（bind/instantiate 全链编排）**不在本 change**，记为 0.7 明确债务。
- **收尾交付**：盒上全矩阵复跑（4 套 test / clippy -D / build）+ 真机三闭环回归（SELFTEST / loopback / HW-IDENT）+ 单一 PR `comet/p06-final-merge-hardening` → `master` + GitHub required checks / branch protection 补齐 + master 标记为 Phase 0.6 Runtime Abstraction Baseline（tag + 文档）。

**不拆分说明**：各项为同一清债刀的组成部分，共享同一验证矩阵与同一 PR 交付物，用户已明确以"一次 hardening、单一 PR" framing；拆分会制造中间不可合入态。

## Capabilities

（`skip_specs: true`——本仓库 canonical 语义 SoT 为 `docs/architecture/*.md` 冻结契约，OpenSpec 层零 delta spec 与 bc/f/g/de/hi 五个前序 change 一致；契约修正直接落在 docs/architecture 对应文档并归档于本 change。）

## Impact

- **编译**：`default` / `simulation` / `mock` / `bmd-provider,gstreamer-backend` 全套 feature 矩阵须保持可编译 + 单测通过；SPI 方法更名/新增为 breaking，所有 impl 与调用点同步更新。
- **受影响代码**：`services/media-agent/src/{device.rs, contracts/{backend.rs,provider.rs}, adapters/{blackmagic/*,mock.rs,gstreamer/*}, registry.rs, resolver.rs, events.rs, resource.rs, main.rs, supervisor.rs}`；`scripts/check_arch_portability.py`；`.github/workflows/media-agent.yml`。
- **受影响文档**：`docs/architecture/{MEDIA_BACKEND_CONTRACT.md, HARDWARE_PROVIDER_CONTRACT.md, CANONICAL_IDENTITY.md}`（对齐注记）；p06-hi 归档目录勘误。
- **外部交付**：GitHub branch protection + required checks（`gh`，pwl1987/VBMF）；合并后 master = Phase 0.6 Runtime Abstraction Baseline。
- **风险**：P0-1 触及 resolver 匹配与真机身份链路，须真机 HW-IDENT-02 回归兜底；P0-4 不得破坏盒上现有 bmd+gstreamer 真机构建（该组合无 mock，不受影响）。

```

## docs/openspec/changes/p06-final-merge-hardening/design.md

- Source: docs/openspec/changes/p06-final-merge-hardening/design.md
- Lines: 1-41
- SHA256: 7996c1c34ae7ed7190becfbf855b062f9db666630b1f18f82d39dc7173e64889

```md
# Design: Phase 0.6 Final Merge Hardening — 合并前清债

## Context

聚合候选分支 `comet/p06-final-merge-hardening`（自 hi HEAD 0f2a2bf 拉出）已含 0.6 全部代码：真 trait SPI（`HardwareProvider`/`MediaBackend`）、Mock/BMD/GStreamer 三实现、`AdapterRegistry`、`RuntimeEvent`/`Resource`/Preflight、ARCH-PORTABILITY-01 词法 lint + simulation/mock 编译门禁、真机三闭环证据。冻结契约（`docs/architecture/`）与实现之间存在四处结构性偏差（BMD identity 泄漏、Backend 契约形状、trait feature 门控、Registry 静默优先级），加上四处 P1 债务。约束：master 合入后修这些将成 breaking；盒上 bmd+gstreamer 真机构建不可破坏。

## Goals / Non-Goals

**Goals:**
- Canonical Domain 完全去 vendor 身份字段；Provider 身份只存在于 `adapters/blackmagic`
- `MediaBackend` SPI 形状 = 冻结契约 `instantiate/start/stop/recover/observe`，且 trait 无条件编译
- feature 冲突 fail-closed；架构门禁从"词法 lint"升级为"remove-adapter 真实编译证明"
- Mock/事件/资源四处 P1 债务清偿至"不阻断 Baseline"程度
- 单一 PR 合入 master 并打 Baseline 标记 + branch protection

**Non-Goals:**
- 不做 Normalize(0.7)、Session ownership 完整层、Audio loopback 独立门禁、Clock/External API
- 不做完整 Resource Orchestration（bind→instantiate 全链编排；仅最小原子 `acquire` 原语 + 债务记账）
- 不引入新硬件厂商 / 新采集传输抽象（IP/NDI/SRT 等属 0.7+ 路线）
- 不改 `PipelinePlan` canonical 字段语义

## Decisions

- **D1 (P0-1) ProviderIdentity 收敛方向**：`DeviceInfo` 仅保留 `device_id`/`model`/`display_name`/`serial_number`/`identity_strength`/`identity_source`/`capabilities`/`ports` 等 canonical 字段；`bmd_*` 三字段迁入 `adapters/blackmagic` 的 `BmdIdentity { persistent_id, device_handle, topological_id }`，经 `Provider Identity Adapter`（blackmagic 模块内函数）在 discovery 时完成 `→ DeviceId (UUIDv5) + IdentityStrength` 映射。`resolver.rs` 匹配签名改为 `(canonical_device, &BmdIdentity, probes)`——由调用方（provider 侧/诊断路径）传入 provider 证据，Domain 不携带。备选"保留字段但 lint 禁读"被否：字段在即泄漏，无法防下游误用。
- **D2 (P0-2) 契约对齐为更名+补齐而非重构**：`prepare→instantiate`、`poll_bus→observe`（返回 `Vec<PipelineBusEvent>` 保留现事件载荷，但载荷类型已 vendor-neutral 于 `pipeline_events.rs`，命名对齐契约）、新增 `stop`（GStreamer 侧 `set_state(Null)` + join 线程；Mock 侧 no-op Ok）。不引入 `BackendError` 新类型（复用 `PipelineError`，避免二连 breaking）。`MEDIA_BACKEND_CONTRACT.md` 加对齐注记。
- **D3 (P0-3) trait 门控解除**：`contracts/backend.rs` 去 `#[cfg(any(...))]`——其依赖（`pipeline.rs`/`pipeline_events.rs`）已 vendor-neutral 无条件编译，可直接解除；两个 `impl` 块各自保留 feature 门控。default 构建由此获得"有契约、无实现"的合法状态。
- **D4 (P0-4) fail-closed 判定信号**：`AdapterRegistry` 选择处区分"显式测试模式"（`MEDIA_AGENT_MODE=simulation|diagnostic` 或 `VBMF_ALLOW_MOCK=1`）与默认生产模式；生产模式下 mock+真实 feature 组合 → 启动即报错退出（列出冲突 feature 与解除方法），绝不静默取 Mock。simulation/mock 单独 feature（无真实实现共存）行为不变。
- **D5 (P0-5) remove-adapter 编译证明机制**：新脚本 `scripts/check_remove_adapters.py`：临时副本中删除 `adapters/blackmagic/`、`adapters/gstreamer/` 目录并修 `adapters/mod.rs`/`main.rs` 对应引用后 `cargo check --no-default-features --features simulation`（+mock）；通过后删除副本。接入 CI 独立步骤。词法 lint 保留但文档定位改为 Architecture Lint（Proof = 本脚本）。备选 cfg 空实现被否：cfg 仍保留代码路径，移除目录才是真证明。
- **D6 (P0-6) 勘误方式**：归档文件就地修正旧表述 + 追加 `> [勘误 2026-08-29]` 引用块说明原口径与真实语义（三态 + absence-of-evidence），保留上下文可追溯；不改写其他历史内容。
- **D7 (P1-1) Mock 句柄统一**：`MockBackend` 改用 `crate::pipeline::NEXT_PIPELINE_ID`（pub(crate) AtomicU64，从 1 起）分配句柄，与生产同源；测试断言从 `PipelineHandle(0)` 哨兵改为"非零且递增"。
- **D8 (P1-2) discover Result 化**：`HardwareProvider::discover -> Result<Vec<DeviceInfo>, ProviderError>`（新增 `ProviderError` 于 `contracts/provider.rs`）；BMD SDK 不可用/驱动错显式报错，与"无硬件=Ok(vec![])"区分；frozen 契约文档同步修正并注明 hardening 修正。**BREAKING**：全部 impl（BMD/Mock A/B/simulation/filesystem）+ registry 调用点更新。
- **D9 (P1-3) 事件两级语义**：`RuntimeEvent` 增 `severity: Observation | Critical`（fault/identity 类 = Critical）；`RuntimeEventLog` 满时仍挤出 Observation，Critical 不可被挤出——溢出时强推 Critical 并递增 `dropped_observations` 计数暴露给 Health。不改 VecDeque 结构，仅入队策略。
- **D10 (P1-4) 原子 acquire 最小原语**：`ResourceRegistry` 包 `Mutex` 后提供 `acquire(req) -> Result<Reservation, PreflightError>`（锁内 preflight+reserve 原子完成）；`main.rs` Preflight 闸门改走 `acquire`；materialize 失败路径调 `release_reservation` 回滚。完整编排层记 0.7 债务（proposal 已列）。

## Risks / Trade-offs

- **D1 触面最大**：resolver 匹配、BMD discovery、mock/simulation provider、诊断输出全要改；以真机 HW-IDENT-02 回归（C1 证据仍须 ManifestVerified/High + Unresolved fail-closed）兜底。风险换取消"换厂商不进 Domain"的长期正确性。
- **D2/D8 双 breaking**：本 change 内一次付清，避免 Baseline 后二次 breaking；盒上 bmd+gstreamer 全矩阵 + 三闭环回归为回归兜底。
- **D4 模式判定依赖 env**：env 信号可被误设；以启动日志显式打印当前模式与生效 adapter 为缓解（运维可见）。
- **D5 脚本操作文件树**：只在临时副本操作 + 异常清理，杜绝污染工作区；CI 步骤超时上限防卡死。
- **D9 Critical 强推可能挤掉普通事件**：接受——观测事件可丢、故障不可丢与 VBMF fail-closed 原则一致；dropped 计数保证可观测。

```

## docs/openspec/changes/p06-final-merge-hardening/tasks.md

- Source: docs/openspec/changes/p06-final-merge-hardening/tasks.md
- Lines: 1-38
- SHA256: 569768e7d50a702991fbc628a05ed3c8b92d29b3d24a4793dda8b9a0b038f2bf

```md
# Tasks: Phase 0.6 Final Merge Hardening — 合并前清债

## 1. P0-1 DeviceInfo 去 BMD identity 泄漏

- [ ] 1.1 `BmdIdentity` 收敛进 `adapters/blackmagic`，`device.rs` 删除 `bmd_*` 三字段；discovery 侧经 Provider Identity Adapter 映射为 canonical `DeviceId`/`IdentityStrength`
- [ ] 2.1 `resolver.rs` 匹配签名改消费 provider 侧身份证据（不再读 Domain 字段）；全部调用点/诊断输出更新

## 2. P0-2/P0-3 MediaBackend 契约对齐

- [ ] 2.1 trait 更名 `prepare→instantiate`、`poll_bus→observe`、补 `stop`；GStreamer/Mock 两 impl + main.rs watchdog/supervisor 调用点同步
- [ ] 2.2 `contracts/backend.rs` trait 去 feature 门控（impl 保留门控）；default 构建验证"有契约无实现"可编译

## 3. P0-4 Registry fail-closed

- [ ] 3.1 生产模式 mock+真实 feature 组合启动报错拒启；显式测试模式（env）放行；启动日志打印模式与生效 adapter

## 4. P0-5 remove-adapter 编译验证

- [ ] 4.1 `scripts/check_remove_adapters.py`（临时副本移除 adapters/blackmagic+gstreamer 后 cargo check simulation/mock）+ CI 接入；词法 lint 定位改为 Architecture Lint

## 5. P0-6 证据勘误

- [ ] 5.1 p06-hi 归档 tasks/design 旧口径（Default=true 等）就地勘误注记

## 6. P1 债务清偿

- [ ] 6.1 P1-1 MockBackend 句柄走 `NEXT_PIPELINE_ID`（非零递增），测试断言更新
- [ ] 6.2 P1-2 `discover -> Result<Vec<DeviceInfo>, ProviderError>`（BREAKING）：新增 ProviderError、全 impl + registry 调用点更新、frozen 契约文档修正
- [ ] 6.3 P1-3 RuntimeEvent severity 两级 + RuntimeEventLog Critical 不可挤出 + dropped 计数
- [ ] 6.4 P1-4 `ResourceRegistry` Mutex 化 + 原子 `acquire`（preflight+reserve）+ materialize 失败回滚；编排层债务记 0.7

## 7. 验证与交付

- [ ] 7.1 盒上全矩阵复跑：4 套 test / clippy -D / build 全绿 + 新增断言通过
- [ ] 7.2 真机三闭环回归：SELFTEST A+B+C / loopback / HW-IDENT-02（ManifestVerified 仍 High、Unresolved 仍 fail-closed）
- [ ] 7.3 verify（full）+ 报告
- [ ] 7.4 archive + 单一 PR `comet/p06-final-merge-hardening` → `master`（gh）
- [ ] 7.5 GitHub required checks / branch protection 补齐（gh api）+ master 打 Baseline 标记（tag + 文档）

```
