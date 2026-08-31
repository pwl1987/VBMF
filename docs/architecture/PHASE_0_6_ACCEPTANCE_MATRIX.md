# PHASE_0_6_ACCEPTANCE_MATRIX — 验收门禁矩阵

> 把分散在 Master PRD / 各 Contract / Evidence / 旧 Gate 文档的验收集中到一张表，避免再次漂移。门禁状态词见 [`DOCUMENT_STATUS_MODEL.md`](./DOCUMENT_STATUS_MODEL.md)。
> 关联：[`PHASE_0_6_IMPLEMENTATION_GAP_MATRIX.md`](./PHASE_0_6_IMPLEMENTATION_GAP_MATRIX.md)、`evidence/`。

## 矩阵（Gate → Contract → Code → Test → Evidence → Pass criteria）

| Gate | Contract | Code 要求 | Test | Evidence | Pass criteria |
|---|---|---|---|---|---|
| `ARCH-PORTABILITY-01` Test A | `IMPLEMENTATION_BOUNDARIES` / `HARDWARE_PROVIDER_CONTRACT` | 删除/禁用 BMD Provider 后 Domain/Graph/Session/Supervisor/Health/Acceptance 仍能编译 | `cargo build --no-default-features --features simulation` + `mock-only` + remove-adapter proof | CI（`architecture-portability` required job） | **PASS**（编译门禁 + 真删 adapters 副本 cargo check） |
| `ARCH-PORTABILITY-01` Test B | `MEDIA_BACKEND_CONTRACT` | Mock Provider 下 same `GraphRuntimeIntent`/Session/Supervisor/Health | 单测 | CI | 行为一致 |
| `ARCH-PORTABILITY-01` Test C | `IMPLEMENTATION_BOUNDARIES` | 换 Mock Provider B 不改 Domain/Graph/UI schema | 单测 | CI | 零改动 |
| `ARCH-BACKEND-01` | `MEDIA_BACKEND_CONTRACT` | `MockBackend` vs `GStreamerBackend` 共享 `CanonicalPipelinePlan`/`CanonicalMediaFormat`/`CanonicalRuntimeEvent` | 单测 | CI | 三契约一致 |
| `ARCH-TOPOLOGY-01` | `RUNTIME_TOPOLOGY_CONTRACT` | Topology 模型独立于 Resource/Binding；`PhysicalConnection` 与 `LogicalRoute` 可分可合 | 单测 | — | Contract=`FROZEN`，实现=`NOT_STARTED` |
| `ARCH-API-BOUNDARY-01` | `EXTERNAL_API_CONTRACT` | External→Control→Runtime Contract→Rust；禁 External→Vendor SDK | 集成测试 | — | 边界不被越过 |
| `HW-IDENT-02` | `CANONICAL_IDENTITY` | 稳定身份（非 `device-number`）以 `DeviceId` 表达 | 真机 | `evidence/.../HW-IDENT-02` | PASS（已验证） |
| `MEDIA-RT-01` | `MEDIA_BACKEND_CONTRACT` / `IMPLEMENTATION_ADDENDUM §8` | 统一 `RuntimeEvent`/`RuntimeError` 链 | 真机 | `evidence/.../MEDIA-RT-01` | PASS（已验证） |
| `EXT-API-01` | `EXTERNAL_API_CONTRACT` | Query/Command/Event/Auth/Authz/Audit/Idempotency/Versioning | 集成测试 | — | P1（未实现） |
| `EXT-DEVICE-01` | `DEVICE_INTEGRATION_CONTRACT` | discovery/identity/capability/state/command/error/reconnect | 集成测试 | — | P1（未实现） |
| `EXT-ROUTING-01` | `DEVICE_INTEGRATION_CONTRACT` | reserve/route/conflict/rollback/release | 集成测试 | — | P1（未实现） |
| Vendor Neutrality Gate | `VENDOR_NEUTRALITY_RULES` | Domain 不 `import` vendor/backend；禁自动 Fallback | CI lint | — | 全绿 |

## 说明
- **[对账 2026-08-30, p07b-consolidation]** `ARCH-PORTABILITY-01` / `ARCH-BACKEND-01` 已 **PASS**（0.6 系列：CI 词法 Lint + remove-adapter 编译证明 + Mock/GStreamer 共享 PipelinePlan 三契约；p06-hi 门禁组全绿）。
- `HW-IDENT-02` / `MEDIA-RT-01` 已真机验证（Implementation `PARTIAL`，契约 `FROZEN`）。

## 0.7 系列门禁（0.7A/0.7B, 2026-08-30 对账补录）

| 门禁 | 内容 | 层级 | 结果 | 证据 |
|---|---|---|---|---|
| `SESSION-RT-01` | 会话全生命周期（create→start→running→stop→release + 失败矩阵 + double-start/stop） | Unit/Simulation/**Hardware** | **PASS** | verify 报告 2026-08-29-p07-session-runtime（真机 VBMF_SESSION_LIFECYCLE ALL PASS） |
| `RESOURCE-RT-01` | 并发争抢/容量/冲突拒绝/release/expiry/crash cleanup | Unit/Simulation/**Hardware** | **PASS** | 同上 |
| `NORMALIZE-RT-01` | Provider 无关归一（BMD/Mock → 同一 canonical 表征；缺失观测不臆造） | Unit/Simulation/**Hardware** | **PASS** | verify 报告 2026-08-30-p07b-media-semantics |
| `MEDIA-SEMANTICS-RT-01`（Clock） | Clock 观测词表（#147）/零决策/Unknown 合法 | Unit/Simulation/**Hardware** | **PASS** | verify 报告 2026-08-30-p07b-clock-domain |
| `AUDIO-SEMANTICS-RT-01` | Audio 语义（Role 词表/Unknown 贯穿/Route 零 pipeline 引用） | Unit/Simulation/**Hardware** | **PASS** | verify 报告 2026-08-30-p07b-audio-semantics |
| `TIMECODE-SEMANTICS-RT-01` | Timecode 词表（#148）/Clock 隔离/防臆造 | Unit/Simulation/**Hardware** | **PASS** | verify 报告 2026-08-30-p07b-timecode-foundation |
- `EXT-*` 属 Phase 0.7（P1），契约 `FROZEN` 但实现 `NOT_STARTED`。
- 每次新增验收项，必须同时更新本表与对应 Contract 的「验收」小节，否则视为漂移。
