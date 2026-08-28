# Verify Report — p06-bc-provider-backend-spi（Phase 0.6 C1: Provider/Backend SPI 抽取）

- 日期：2026-08-29
- Change：`p06-bc-provider-backend-spi`
- Workflow：classic full（open → design → build → verify → archive）
- 语言：zh-CN
- Verify 模式：**full**（scale：Tasks 28 / Delta specs 0 / Changed files 43）
- 评审模式：standard（轻量代码审查：正确性 / 安全 / 边界）
- 分支：`comet/p06-bc-provider-backend-spi`（base_ref `68c3a1c`）
- 提交范围：`68c3a1c..HEAD`，43 文件，+2610 / −645，15 提交
- 结论：**PASS（2 项 NOTE，0 CRITICAL / 0 IMPORTANT）**

## 1. 入口检查

- `comet state check p06-bc-provider-backend-spi verify` → ALL PASS（phase=verify，verify_result=pending，bound_branch 匹配）。
- Handoff 哈希不一致（RECORDED `636657…` ≠ CURRENT `2ffdfc…`，因收口时编辑 tasks.md）→ 按协议**完整重读全部交付物**：proposal.md / design.md（C1–C5 全节 + 审计注记）/ plan（T1–T10 全勾）/ tasks.md。

## 2. 七项完整核查

| # | 检查项 | 结果 | 证据 |
|---|--------|------|------|
| 1 | tasks 全部勾选 | ✅ PASS | `tasks.md` 28/28；收口时逐项对照代码核验（trait 签名、impl 位置、PipelinePlan 共享、mock 类型、CI gate）后补勾 C2×3 / C3 / C4×2 / C8，证据写入 tasks.md「收口确认（2026-08-29）」块 |
| 2 | design.md 一致性 | ✅ PASS | 实际落地与 design §3–§5 一致：`contracts/` 真 trait 替代 C1 别名；feature `bmd`→`bmd-provider`、`gstreamer`→`gstreamer-backend` 且保留兼容别名；`DeckLinkDeviceManager` 下沉 `adapters/blackmagic/device_manager.rs`（C4）；`registry.rs::AdapterRegistry` 收口选择逻辑（C5，与 C2c 内联块逐字一致）；偏差（`discover() -> Vec` 非 `Result`）在 design §3 审计注记中书面记录、归因 0.6D |
| 3 | 设计/计划产物存在 | ⚠️ NOTE | design_doc 指向 OpenSpec `design.md`（存在且完整）；`docs/superpowers/specs/` 为空目录（设计文档落在 OpenSpec 根而非 superpowers 根）——见 §5 NOTE-1 |
| 4 | delta spec 一致 | N/A | 本 change 0 个 delta capability（纯代码重构，不改规范） |
| 5 | proposal 目标达成 | ✅ PASS | Why/What/Impact 全达成：SPI 冻结、BMD/GStreamer 降级为 Reference Adapter、main 经 trait 对象接线、feature 重构 + 兼容别名、default/simulation 基线不回归 |
| 6 | delta spec 归档前置 | N/A | 同 #4 |
| 7 | 构建/测试证据 | ✅ PASS | 盒上验证 2026-08-28（记录于 p06-g `tasks.md` §4）：`cargo test` default 84 / `--features simulation` 84 / `--features mock` 87 全通过；clippy 7 套 feature 组合 0 error；build 4 套组合 OK。**自验证后 Rust 源码未变**（`48e23d8` / `46c9a11` 为 C8 文档/CI 修复，`8dac4bf` 为收口文档）→ 证据有效 |

## 3. 轻量代码审查（standard：正确性 / 安全 / 边界）

审查对象：核心 SPI 面 `registry.rs`（61L）/ `adapters/mock.rs`（142L）/ `contracts/provider.rs`（47L）/ `contracts/backend.rs`（28L）+ 变更文件全量 grep 扫描。

**正确性**
- `registry.rs::build_provider()`：四路 `#[cfg]` 分支（mock / simulation / bmd-provider / default）互斥且穷尽，优先级与 C2c 内联块逐字一致，无 fallthrough 风险。✅
- `build_media_backend()`：`all(bmd-provider, gstreamer-backend)` 门控 + `mock > gstreamer-backend` 两分支穷尽，与真机盒 feature 组合一致。✅
- `adapters/mock.rs`：`MockProvider`/`MockProviderB` 确定性 UUID v5（固定命名空间 `VBMF_MOCK_NS`），`bmd_persistent_id: None`、`identity_source: Simulation` —— 不以合成值伪造真实 BMD 身份；3 个单测覆盖确定性/双设备拓扑/后端生命周期。✅
- `contracts/backend.rs`：`Send + Sync` 满足跨线程（Supervisor/watchdog）持有要求；门控 `any(gstreamer-backend, mock)` 使 Mock 侧可独立适用契约（ARCH-PORTABILITY-01 Test B 前提）。✅
- ARCH-BACKEND-01：`MockBackend::prepare(&PipelinePlan)` 与 `GStreamerPipelineController`（`adapters/gstreamer/controller.rs:191`）从同一 `PipelinePlan` 物化，契约共享成立。✅

**安全**
- 变更 Rust 文件 secret/key/token 扫描：0 命中。
- `unsafe` 仅存在于 `adapters/blackmagic/`（`decklink.rs` 135 处、`sdk.rs` 4 处，均为 BMD FFI 既有代码、git mv 平移未改）——FFI 不安全边界被正确隔离在 Concrete Adapter 层内，Domain/contracts/runtime 0 处 `unsafe`。✅

**边界**
- vendor 引用边界：`decklink`/`sdk` 仅 `adapters/blackmagic/` 内引用；`gstreamer` 仅 `adapters/gstreamer/` 与 cfg 门控的 domain 探活函数（design §2 锁定的最小移动面）。`default`/`simulation` 构建不含 vendor 顶层引用（盒上验证覆盖）。✅

**发现**：0 CRITICAL / 0 IMPORTANT / 2 NOTE（见 §5）。

## 4. 构建门禁记录

- 本机（Windows）无 Rust 工具链，`cargo` 不可用 → 按协议以**盒上验证**（2026-08-28，canonical 构建/测试环境）为 build 证据，`comet state record-check … build` 如实记录命令与结果。
- build guard → ALL PASS，`[TRANSITION] build-complete` → phase=verify。

## 5. 发现与建议

- **NOTE-1（SUGGESTION）**：`docs/superpowers/specs/` 为空——本 change 的设计文档以 OpenSpec `design.md` 为唯一权威载体（design_doc 字段指向正确、内容完整，C1–C5 含审计注记）。建议后续 change 在 open 阶段将 Superpowers 设计落点与 Comet design_doc 字段显式对齐，避免两目录漂移。不阻塞归档。
- **NOTE-2（SUGGESTION）**：`HardwareProvider::discover()` 返回 `Vec<DeviceInfo>` 而非 design §3 契约的 `Result<_, ProviderError>`——偏差已书面记录（design §3 审计注记），vendor 错误统一到 `RuntimeEvent` 明确留给 0.6D。跟踪项已存在于 tasks.md 1b，不阻塞归档。
- **观察（无需处理）**：`build_media_backend()` 仅在 `all(bmd-provider, gstreamer-backend)` 下编译——`mock` 单独 feature 组合无法经 Registry 构造 backend；与 C2c 接线语义一致且有文档注释，属有意设计。

## 6. 结论

p06-bc 全部 28 项任务完成并经代码级核验；实现与 design 一致（偏差均书面记录且有归属）；盒上验证基线有效且其后 Rust 源码未变；代码审查 0 CRITICAL / 0 IMPORTANT。
**verify → PASS，可进入 archive 阶段**（归档前最终确认为阻断式决策点，需用户确认；分支收尾在归档提交后由用户选择）。
