---
comet_change: p06-final-merge-hardening
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-29-p06-final-merge-hardening
status: final
---

# Design Doc — p06-final-merge-hardening（Phase 0.6 Final Merge Hardening：合并前清债）

> 本文是 open 阶段 `design.md`（D1–D10）的深度技术细化：逐项实现设计、触面图、技术风险、测试策略与边界条件。canonical 需求以 OpenSpec change 为准，架构契约 SoT 为 `docs/architecture/*.md`。

## 0. 目标与基线（已核实）

基线 = `comet/p06-final-merge-hardening`（自 hi HEAD `0f2a2bf` 拉出）。终审结论：SPI 已是真 trait（`contracts/provider.rs::HardwareProvider` / `contracts/backend.rs::MediaBackend`），但四处结构性偏差 + 四处 P1 债务须在合入 master 前清偿。本 change 交付后形成单一 PR → master = **Phase 0.6 Runtime Abstraction Baseline**。

冻结契约锚点（已核实原文）：
- `MEDIA_BACKEND_CONTRACT.md` §1：`instantiate / start / stop / recover / observe`（observe 返回统一事件，非 Bus Message）；§1.1 Backend 不得自行获取未注册资源。
- `CANONICAL_IDENTITY.md` §4：Provider Identity ≠ Canonical Identity；`DeviceHandle/PersistentId/TopologicalId` 不得进 Canonical Domain；各 Provider 自行定义**局部**优先级，经 Provider Identity Adapter 收敛为 canonical `DeviceId`。
- `HARDWARE_PROVIDER_CONTRACT.md`：`discover -> Vec<CanonicalDevice>`（本 change 修正为 `Result`，见 D8）。

## 1. D1 (P0-1) DeviceInfo 去 BMD identity 泄漏

**触面图（已逐一核实，8 文件）**

| 文件 | 引用数 | 性质 | 处置 |
|------|--------|------|------|
| `device.rs` | 11 | `DeviceInfo` 三字段定义 + FS/Simulation provider 填充 | 字段删除；FS provider 不再伪造（hash 值禁止进 Domain，直接不填）；Simulation 走显式测试世界 |
| `resolver.rs` | 26 | `best_kind_for`/`find_match`/`resolve`/manifest 交叉核验（Domain 身份匹配核心） | 匹配签名改为 `(dev: &DeviceInfo, pid: &ProviderIdentityRef, probes)`；`ProviderIdentityRef` 为 blackmagic 模块导出的 trait/struct 视图，Domain 不携带 |
| `pipeline.rs` | 17 | **`SourcePlan.bmd_persistent_id` 字段本身** + `src_props` 持久 ID 属性 + materialize 身份闸门（L421/L445/L471） | 字段更名 `provider_persistent_id`（语义 = Provider Identity Adapter 已解析的持久标识，机制中立，非 vendor 专名）；闸门逻辑改读 `IdentityStrength`+绑定 |
| `port.rs` | 9 | manifest↔device handle 交叉核验（L390/452/673/748）+ `PortIdentity` 派生 | 交叉核验入参改 `(device_id, &ProviderIdentityRef, manifest)`；`PortIdentity` 内 vendor 句柄字段移至 provider 侧 |
| `hw_port_01.rs` | 4 | 端口绑定闭环引用 handle | 同上：消费 provider 证据参数 |
| `main.rs` | 2 | 诊断输出/解析 | 打印 provider 证据由 discovery 侧附带输出 |
| `adapters/mock.rs` | 4 | `mock_device` 置 `None` | 随字段删除自然消失；Mock 身份 = 确定性 UUIDv5 + `IdentitySource::Simulation`（不变） |
| `adapters/blackmagic/device_manager.rs` | 4 | 真实 BMD discovery 填充 | **新家**：`BmdIdentity { persistent_id: Option<i64>, device_handle: Option<String>, topological_id: Option<i64> }` 常驻本模块；discovery 输出 `(DeviceInfo, BmdIdentity)` 对（evidence 通道），Adapter 函数完成 `→ DeviceId(UUIDv5 over provider 证据) + IdentityStrength` |

**关键设计**：
- `ProviderIdentityRef`（`adapters/blackmagic` 内 `pub struct`，`#[cfg(bmd-provider)]` + 非 gated 只读视图）：resolver 的匹配函数只在**持有 provider 证据**时可调用——诊断/绑定路径（`VBMF_RESOLVER=1`、manifest 校验）由 discovery 输出携带证据；生产 materialize 路径只消费 manifest 校验结果（`ManifestVerified`），不读 vendor 字段。
- `SourcePlan.provider_persistent_id: Option<i64>`：canonical plan 只承载"backend 应使用的持久标识"这一**机制中立**概念；`bmd_` 专名从 canonical 类型消失。`src_props` 对该字段的消费保持在 pipeline.rs（GStreamer 属性拼装的中立共享层，p06-g C7 已解耦）。
- **边界**：不改变匹配语义本身（优先级链/多重 HIGH→Ambiguous/MEDIUM 拒生产，p06-hi 门禁断言全部保留且必须继续全绿）。

**风险**：触面最大；以 p06-hi 全部门禁断言 + 真机 HW-IDENT-02 回归（ManifestVerified/High、Unresolved fail-closed 不变）兜底。

## 2. D2 (P0-2) MediaBackend 契约对齐

```rust
pub trait MediaBackend: Send + Sync {
    fn instantiate(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError>;
    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    fn stop(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    fn observe(&self, handle: &PipelineHandle) -> Vec<PipelineBusEvent>;
}
```

- 更名映射：`prepare→instantiate`（语义一致：从 canonical plan 物化实例）、`poll_bus→observe`（载荷 `PipelineBusEvent` 已 vendor-neutral 于 `pipeline_events.rs`，仅命名对齐契约）。
- **新增 `stop`**：GStreamer 侧 = `set_state(Null)` + join bus 线程 + `HEALTH_ARCS.remove(handle)`（防句柄泄漏）；Mock 侧 = `Ok(())`。契约冻结缺 stop 是原实现偏差的根源，本 change 一次补齐。
- 不引入 `BackendError` 新类型（复用 `PipelineError`）——避免一次 change 两套 breaking；契约文档加对齐注记说明错误类型映射。
- 调用点（已核实 6 处）：`main.rs` L386/387（selftest）、L668/669（canonical）、L780（watchdog `observe`）、L907（recover）+ `registry.rs::build_media_backend`。watchdog 停止路径接入 `stop`（现无显式 stop）。

## 3. D3 (P0-3) trait 去 feature 门控

- `contracts/backend.rs` 删除 `#[cfg(any(feature = "gstreamer-backend", feature = "mock"))]` 及 import 门控——依赖项 `pipeline.rs`/`pipeline_events.rs` 均无条件编译（p06-g C7 已中性化），可直接解除。
- `impl MediaBackend for GStreamerPipelineController`（`#[cfg(gstreamer-backend)]`）与 `for MockBackend`（`#[cfg(mock)]`）保留各自门控。
- 验收：`cargo check --no-default-features`（default 无任何 backend）通过，且 trait 存在（编译期断言测试 `fn media_backend_contract_always_compiled`）。

## 4. D4 (P0-4) Registry feature 冲突 fail-closed

```text
生产模式（默认）: mock ∧ (bmd-provider ∨ gstreamer-backend) → eprintln 列冲突 + std::process::exit(2)
显式测试模式:   MEDIA_AGENT_MODE ∈ {simulation, diagnostic} ∨ VBMF_ALLOW_MOCK=1 → 放行（Mock 优先级语义保留）
```

- 实施于 `registry.rs` 两个构造函数入口（provider/backend 各一）+ 启动早期一次性检查（main.rs 启动日志打印 `mode=... adapters={provider, backend}`）。
- `MEDIA_AGENT_MODE=diagnostic` 已存在（config.rs L34），复用该约定；新增 `simulation` 值与 `VBMF_ALLOW_MOCK=1`。
- **边界**：mock 单独 feature（无真实实现共存）行为不变；盒上 bmd+gstreamer 构建（无 mock）不受影响；CI 的 CANONMOCK（三 feature）构建仅编译不运行，运行需显式测试模式。

## 5. D5 (P0-5) remove-adapter 编译证明

- `scripts/check_remove_adapters.py`：
  1. `cp -r services/media-agent` → 临时目录（target 排除）；
  2. 删除 `src/adapters/blackmagic/` 与 `src/adapters/gstreamer/`；修 `adapters/mod.rs`/`main.rs` 最小引用（脚本内置最小 patch 规则，精确到行）；
  3. `cargo check --no-default-features --features simulation` + `--features mock`；
  4. 退出码即门禁；`finally` 清理临时目录。
- CI：`.github/workflows/media-agent.yml` 新增独立步骤 `Architecture proof gate (remove-adapter compile)`。
- 文档定位：`check_arch_portability.py` 注释改为 **Architecture Lint（词法层，防回渗）**；本脚本为 **Architecture Proof（结构层，真移除编译）**。

## 6. D6 (P0-6) 归档勘误

- `archive/2026-08-29-p06-hi-backend-resource-gates/tasks.md` §5 与 `design.md` MEDIA-RT-01 节：旧口径 "`PipelineHealth` Default=true" / "pts_monotonic 只置 false" 处就地保留原文 + 追加 `> [勘误 2026-08-29, p06-final-merge-hardening] 实际实现为 P1-2 三态 (Unknown/ValidMonotonic/NonMonotonic) + acceptance 默认全 false (absence-of-evidence ≠ pass, P1-2)；verify 报告 INFO 已记录。` 不改写其他历史内容。

## 7. D7 (P1-1) Mock 句柄统一

- `MockBackend::{instantiate}` 改 `PipelineHandle(NEXT_PIPELINE_ID.fetch_add(1, SeqCst))`（`pipeline.rs` pub(crate) static，从 1 起）。
- 测试更新：`arch_backend_01_mock_*` 断言 `handle != PipelineHandle(0)`；mock 生命周期测试同改。

## 8. D8 (P1-2) discover Result 化（BREAKING）

```rust
pub struct ProviderError { pub kind: ProviderErrorKind, pub detail: String }
pub enum ProviderErrorKind { SdkUnavailable, DriverFailure, PermissionDenied, InitFailed }
fn discover(&self) -> Result<Vec<DeviceInfo>, ProviderError>;
```

- impl 更新清单：`contracts/provider.rs`（trait）+ `device.rs`（legacy `DeviceManager` trait 及 FS/Simulation 等 3 impl——**同期收敛**：`DeviceManager` 与 canonical `HardwareProvider` 并存属过渡残留，本 change 将 `DeviceManager` 标记 `#[deprecated]` 别名或直接删除（design 终审：直接删除，调用点走 HardwareProvider）+ `mock.rs` A/B + `blackmagic/device_manager.rs`（SDK 不可用 → `SdkUnavailable`，不再空 Vec）。
- 调用点：`registry.rs`（unwrap/传播）、`main.rs` discovery 路径（错误 → RuntimeEvent::HealthChanged degraded + 日志，不 panic）。
- `HARDWARE_PROVIDER_CONTRACT.md` 修正注记：discover 语义 = fail-closed，"无硬件" = `Ok(vec![])`，SDK/驱动失败 = `Err`。

## 9. D9 (P1-3) RuntimeEvent 两级语义

- `events.rs`：`RuntimeEvent` 增字段 `severity: EventSeverity`（`Observation | Critical`）；`kind()` 映射：`PipelineFault`/`HardwareFault`/`AmbiguousIdentity`/`LeaseDenied`/`ResourceRejected` 等 = Critical；`HealthChanged`/`StateChanged`/计数类 = Observation。
- `RuntimeEventLog::record`：满时 pop_front 仅当队首为 Observation；队首为 Critical 时丢弃**新入队 Observation**（反侧让位）并 `dropped_observations += 1`；全 Critical 满 → 强推（挤最旧 Observation，无则挤最旧 Critical 但保证 `dropped_criticals` 计数，正常容量下不触发）。
- Health/supervisor 消费侧暴露 `dropped_observations` 计数（运维可见）。

## 10. D10 (P1-4) 原子 acquire

- `resource.rs`：`ResourceRegistry` 增 `Mutex` 包装层 `SharedResourceRegistry(Arc<Mutex<ResourceRegistry>>)`；新 API `acquire(req) -> Result<Reservation, PreflightError>`：锁内 `preflight` + `reserve`（+ `state→Reserved`）原子完成；`release_reservation(holder)` 供 materialize 失败回滚（`Releasing→Available`）。
- `main.rs` Preflight 闸门：从"每次 derive 临时 registry + preflight"改为持有 `SharedResourceRegistry`（启动时 derive 一次）+ `acquire`；materialize Err 路径回滚。
- 完整编排（bind→instantiate 全链、TTL、跨 session 协调）记 0.7 债务（proposal 已列非目标）。

## 11. 测试策略

| 层 | 内容 |
|----|------|
| 单测（盒上 4 套 feature） | 现有 107/107/111/107 基线不回退 + 新增：trait 无条件编译断言（D3）、registry fail-closed 拒启/测试模式放行（D4）、acquire 原子性 + 回滚（D10）、Critical 不被挤 + dropped 计数（D9）、Mock 句柄非零递增（D7）、ProviderError 分类（D8） |
| 架构证明 | `check_remove_adapters.py` 本地 + CI 双跑（D5）；词法 lint 保持绿（D1 后 Domain 零 `bmd_` 专名可加 lint 规则强化） |
| 真机回归（盒） | 三闭环：SELFTEST A+B+C、loopback ALL PASS、HW-IDENT-02（ManifestVerified/High 与 Unresolved fail-closed 语义不变——D1 重构的权威兜底） |
| 交付验证 | archive + 单一 PR（gh）+ branch protection/required checks（gh api）+ Baseline tag |

## 12. 实施顺序

D3（trait 解门控，最小先行）→ D2（契约对齐）→ D7（Mock 句柄）→ D8（discover Result 化 + DeviceManager 收敛）→ D1（identity 迁移，最大触面）→ D9（事件分级）→ D10（原子 acquire）→ D4（registry fail-closed）→ D5（remove-adapter 脚本 + CI）→ D6（勘误）→ 全矩阵 + 真机回归 → PR/branch protection/Baseline。

## 13. 不做（边界重申）

Normalize(0.7) 功能；Session ownership 完整层；Audio loopback 独立门禁；完整 Resource Orchestration；新厂商/新传输抽象（IP/NDI/SRT）；`PipelinePlan` canonical 字段语义变更（仅 vendor 专名中立化更名）。

## 14. Implementation Divergence (verify 阶段记录)

- **D1 证据类型落位**: 设计原稿写 "`BmdIdentity` 常驻 `adapters/blackmagic`"; 实现改为 vendor 中立的
  `ProviderIdentity` 落于 `contracts/provider.rs` (SPI 层)。原因: resolver/port (身份适配层) 不能反向
  import adapters (ARCH-PORTABILITY-01 依赖方向), 而 CANONICAL_IDENTITY §4 本就规定接口层以
  `(provider, provider_identity)` 二元组承载 — 落位修正后语义更严格 (任何 Provider 均同形)。
- **D9 severity 表达**: 设计写 "RuntimeEvent 增字段 severity"; 实现为 `severity()` 方法
  (避免 11 个 variant 全部加字段破坏 serde 形状与既有构造点), 语义等价。
- **P0-5 脚本细节**: remove-adapter 副本对 main.rs 非 gated 的 `probe_sdk` 引用以临时 stub 落点修补;
  DeviceDiscovery.identity 字段 `#[serde(skip)]` (证据为运行期配对物, `&'static str` 不反序列化)。
