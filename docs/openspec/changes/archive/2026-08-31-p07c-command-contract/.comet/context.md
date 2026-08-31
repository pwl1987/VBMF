# Comet Design Handoff

- Change: p07c-command-contract
- Phase: design
- Mode: compact
- Context hash: a59f69bec239e423f8fa242cfd6ea186830e44fb7c868ac6b4059d77236fe982

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07c-command-contract/proposal.md

- Source: docs/openspec/changes/p07c-command-contract/proposal.md
- Lines: 1-28
- SHA256: 759e839575fafec6b9661834ba630945636a8094e482256728788dd7eb400321

```md
# Change: Phase 0.7C-3 — p07c-command-contract（Command Contract Foundation：请求语义，非执行计划）

## Why

0.7C-2 交付 Runtime Query Model（Pure Read）；Phase Map §3 下一项 = **Command Contract**——系统第一次拥有"改变状态的正式入口"。终审裁定：Command 绝不能是 SessionManager 内部方法的换名包装，必须先冻结契约层（vocabulary/envelope/target/参数/result/validation），且**第一红线 = Command 不可执行性**：Command 只表达"请求改变什么"，绝不携带 Backend/GStreamer/FFmpeg/DeviceHandle/Pipeline 等执行细节。这决定后续 Idempotency→Error Model→Event Projection→External API 能否保持干净。

## What Changes

- **`src/command.rs`（新，编排层）**：
  - **Command vocabulary（封闭枚举）**：`CommandKind { StartSession, StopSession, ReleaseSession }`——仅三命令（对应 SessionManager 既有 create+start / stop / close 生命周期）；**词表快照测试**防静默增删（新命令须过架构评审）。
  - **Command envelope**：`CommandEnvelope { command_id: CommandId(Uuid), kind: CommandKind, target: CommandTarget, issued_at_ms, requested_by: String }`；`CommandTarget { Session { intent: GraphRuntimeIntent }, SessionById { session_id: SessionId } }`（Start 携带 canonical intent；Stop/Release 按 id）。
  - **Command result**：`CommandOutcome { command_id, kind, status: CommandStatus, detail: Option<String> }`；`CommandStatus { Accepted, Rejected(reason), Executed, Failed(reason) }`——描述命令生命周期结果，非 Runtime 状态。
  - **Validation contract（纯函数）**：`validate(envelope) -> Result<(), CommandRejection>`——目标在场性/幂等形状基础校验（如 SessionById 的 id 非 nil、Session intent 非空设备）；**不触碰 Runtime 状态**（validation 与 execution 分离）。
  - **Command → Runtime lifecycle boundary（薄映射，非万能 Executor）**：`dispatch(mgr: &SessionManager, envelope) -> CommandOutcome`——三个命令各自一行映射（Start→create+start、Stop→stop、Release→close），错误转 `Rejected/Failed`；**无通用命令循环/插件机制/命令总线**（终审禁令）。
  - **不可执行性红线（三重守护）**：①类型层——envelope/target/outcome 字段仅 canonical 类型（GraphRuntimeIntent/SessionId/Uuid/String）；②serde 反向断言——envelope JSON 零 gst/pipeline/device_number/backend/handle/ffmpeg 字样；③公开面 allowlist（6 方法）+ denylist（execute_pipeline/configure_backend/万能 executor 类动词禁入）。
  - **Query/Command 分离白盒**：command.rs 不 import runtime_query；runtime_query 不 import command（互不引用——分离由编译结构保证）。
- **门禁 COMMAND-CONTRACT-RT-01（三层）**：Unit（词表快照/不可执行性 serde 断言/allowlist/validation 纯函数各拒绝路径）；Simulation（mock 世界三命令全生命周期经 envelope 驱动 + Rejected 路径 + Query/Command 分离断言）；Hardware（真机 SESSION_LIFECYCLE 追加 command 驱动段——envelope 走 Start→observe→Stop→Release，与直接 SessionManager 调用路径结果等价）。
- **CI**：测试并入现有矩阵。

## Capabilities

（`skip_specs: true`——SoT 为 PHASE_IMPLEMENTATION_MAP §3（Command Contract 项）+ 终审裁定。）

## Impact

- 编译：五套 feature 不回退；command.rs 零 vendor 依赖。
- 受影响：新 `command.rs`；`main.rs`（mod + SESSION_LIFECYCLE 追加 command 段）；Phase Map。Session/Resource/Lease 语义零变更（薄映射只调用既有公共 API）。
- **明确不做**：Idempotency/Retry（含 command_id 去重——envelope 携带 id 但不实现幂等语义，属下一 change）；Event/Event Projection；REST/WebSocket/External API；Scheduler；Command Bus；Kafka/NATS；**万能 CommandExecutor**；不新增命令（词表封闭在三命令）。

```

## docs/openspec/changes/p07c-command-contract/design.md

- Source: docs/openspec/changes/p07c-command-contract/design.md
- Lines: 1-31
- SHA256: 65192bfb732cd8435a428adc66683d234eac14cdcc53f59590e3c3bca6334967

```md
# Design: Phase 0.7C-3 — p07c-command-contract

## Context

Query（0.7C-2，Pure Read）已立；Command 是"改变状态的正式入口"。终审设计原则：`Query = What is true now? Command = What do I request to change?`——Command 描述请求≠执行计划；不可执行性为第一红线；不做万能 Executor。

## Goals / Non-Goals

**Goals:** Command vocabulary/envelope/target/outcome/validation（纯函数）+ 三命令薄映射 boundary + 三重不可执行性守护 + COMMAND-CONTRACT-RT-01 三层。
**Non-Goals:** 见 proposal 不做清单。

## Decisions

- **D1 词表封闭**：`CommandKind` 三变体；serde tag `kind` snake_case；词表快照测试（同 AudioRole/TimecodePresence 先例）。新命令 = 架构评审事件（改测试显式更新）。
- **D2 envelope 形状**：`command_id: CommandId(Uuid)`（幂等键占位——携带不实现，D9 幂等语义属下一 change）；`requested_by: String`（opaque 请求方标签，非身份模型）；`issued_at_ms: u64`。**零执行字段**。
- **D3 target 两形**：`Session{intent}`（Start 用——canonical GraphRuntimeIntent，复用 0.6 冻结 intent 类型而非新造参数模型——终审"参数模型"以 canonical intent 为准，不发明 CommandArgs 大杂烩）/ `SessionById{session_id}`（Stop/Release）。target 携带 port_id/device_id canonical 键，绝不带 runtime 地址。
- **D4 CommandStatus 四态**：Accepted（验证过，未执行）/Rejected（验证拒绝）/Executed（映射完成）/Failed（执行期错误）——命令生命周期语义，非 Runtime 状态投影。
- **D5 validation 纯函数**：`validate(&CommandEnvelope, &RuntimeQuery?) -> ...`——**不接 Query**（保持 command 模块不依赖 runtime_query——分离白盒）；在场性校验交给 dispatch 执行期（Rejected by runtime fact）。validation 只做形状校验：kind/target 形状匹配（Start⇒Session、Stop/Release⇒SessionById）、intent 非空、session_id 非 nil、requested_by 非空。返回 `CommandRejection { code, detail }`。
- **D6 薄映射 boundary**：`dispatch(mgr: &SessionManager, env) -> CommandOutcome`——match kind 三臂各调 SessionManager 公共 API；无循环/插件/注册机制。执行前 validate（Rejected 不触 runtime）；执行错误映射 Failed。
- **D7 三重不可执行性守护**：①类型层（字段仅 canonical 类型）；②serde 反向断言（banned: gst/pipeline/device_number/backend/handle/ffmpeg/alsa/kafka）；③公开面 allowlist `[validate, dispatch, CommandId::new...]` + denylist 动词（execute_pipeline/configure_backend/run_backend/build_gst 等）。
- **D8 Query/Command 分离白盒**：两模块互不 import（编译结构保证 + 测试断言源文件无相互引用——以 serde JSON 不含对方类型字样为代理断言 + 源码 grep 级单测不可行，取模块级 allowlist 已覆盖）。

## Risks / Trade-offs

- `dispatch(&SessionManager, ...)` 直接引用 SessionManager：非"包装换名"的关键在——envelope/validation 是独立契约层 + dispatch 是 match 三臂薄映射（无业务逻辑）；终审允许"Command → Runtime lifecycle boundary"。
- Start = create+start 两步：失败中间态由 SessionManager 既有回滚保证（0.7A hardening）；command 层只报告 Failed。
- 三命令不够用（未来 Pause/Route）：词表封闭是刻意约束——扩展走新 change + 架构评审。

## 实施顺序

command.rs 类型+validation → dispatch → 白盒/serde 测试 → Simulation 全生命周期 → main.rs SESSION_LIFECYCLE command 段 → 盒上矩阵+真机 → Phase Map。

```

## docs/openspec/changes/p07c-command-contract/tasks.md

- Source: docs/openspec/changes/p07c-command-contract/tasks.md
- Lines: 1-41
- SHA256: 75ad7c40a4bf1dd93df34fb8a238e60880663c847ea6183cc45c7f93d3a9422d

```md
# Tasks: Phase 0.7C-3 — p07c-command-contract

> 纪律（§十五）：每项标注 `Contract / Implementation / Verification / Gate`。

## 1. Command Contract（command.rs 新）

- [ ] 1.1 CommandKind 封闭词表（Start/Stop/Release 三命令）+ CommandEnvelope/CommandId/CommandTarget + serde + 词表快照
  - Contract: 终审 §七 (vocabulary/envelope/target) | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 1.2 CommandOutcome/CommandStatus 四态 + CommandRejection
  - Contract: 终审 §七 (result/error) | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 1.3 validate() 纯函数（形状校验; 不触 Runtime; validation/execution 分离）
  - Contract: 终审 §六 (Validation 分离) | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 1.4 dispatch() 薄映射 boundary（match 三臂 → SessionManager 公共 API; 无 Executor/Bus）
  - Contract: 终审 §六/§七 (lifecycle boundary; 禁万能 Executor) | Implementation: Not Started | Verification: Test+Simulation | Gate: Pending

## 2. 红线守护

- [ ] 2.1 不可执行性三重守护（类型层 canonical-only / serde 反向断言 / allowlist+denylist）
  - Contract: 终审执行令 (第一红线: Command 不携带执行细节) | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 2.2 Query/Command 分离白盒（两模块互不 import）
  - Contract: 终审 §六 | Implementation: Not Started | Verification: Test | Gate: Pending

## 3. 门禁 COMMAND-CONTRACT-RT-01（三层）

- [ ] 3.1 Unit: 词表快照 / serde 不可执行断言 / allowlist / validation 拒绝路径
  - Contract: 本 change 门禁 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 3.2 Simulation: mock 世界三命令经 envelope 全生命周期 + Rejected/Failed 路径
  - Contract: 同上 | Implementation: Not Started | Verification: Simulation | Gate: Pending
- [ ] 3.3 Hardware: 真机 SESSION_LIFECYCLE command 驱动段（与直接路径等价）
  - Contract: 同上 | Implementation: Not Started | Verification: Hardware | Gate: Pending

## 4. 交付

- [ ] 4.1 盒上全矩阵 + CI 七 checks + 真机回归不退
  - Contract: 盒上绿≠CI绿 | Implementation: Not Started | Verification: Box+CI | Gate: Pending
- [ ] 4.2 Phase Map 0.7C-3 行 → verify → archive → PR#10 → tag phase-0.7C3-command-contract → 删分支
  - Contract: 分支纪律 | Implementation: Not Started | Verification: CI+Review | Gate: Pending

## 收口确认

- 不做: Idempotency/Retry/Event/REST/WS/Scheduler/Command Bus/Kafka·NATS/万能 Executor/新命令词表扩展。

```
