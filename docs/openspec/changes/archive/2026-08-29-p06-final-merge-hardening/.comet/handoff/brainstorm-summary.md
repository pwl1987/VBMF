# Brainstorm Summary

- Change: p06-final-merge-hardening
- Date: 2026-08-29

## 确认的技术方案

（基于用户分支终审指令 + 代码级锚定；D1-D10 与 open design.md 一致并细化如下）

- **D1 (P0-1)**：`DeviceInfo` 去 `bmd_*` 三字段 → `adapters/blackmagic::BmdIdentity`；`Provider Identity Adapter`（blackmagic 模块）在 discovery 时映射 `→ DeviceId(UUIDv5) + IdentityStrength/IdentitySource`。触面（已核实）：`device.rs`(11 处：定义+FS/Sim provider)、`resolver.rs`(26：best_kind_for/find_match/resolve/manifest 交叉)、`pipeline.rs`(17：**`SourcePlan.bmd_persistent_id` 本身在泄漏面** + src_props/身份闸门)、`port.rs`(9：manifest↔device 交叉核验 + PortIdentity 派生)、`hw_port_01.rs`(4)、`main.rs`(2)、`mock.rs`(4：置 None)、`blackmagic/device_manager.rs`(4)。处置：Domain 内 vendor 专名字段全部更名/迁移（`SourcePlan.bmd_persistent_id → provider_persistent_id`，语义="Provider Identity Adapter 已解析的持久标识"，机制中立）；resolver 匹配函数签名增加 provider 证据参数。
- **D2 (P0-2)**：`prepare→instantiate`、`poll_bus→observe`、补 `stop`（GStreamer: set_state(Null)+join 线程+HEALTH_ARCS 清理；Mock: Ok）。错误类型复用 `PipelineError`。调用点：main.rs 386/387(selftest)/668/669(canonical)/780(watchdog observe)/907(recover) + registry。
- **D3 (P0-3)**：`contracts/backend.rs` 去 `#[cfg(any(gstreamer-backend, mock))]`（其依赖 pipeline.rs/pipeline_events.rs 已无条件编译）；impl 各自保留门控。
- **D4 (P0-4)**：`AdapterRegistry` 生产模式（无 `MEDIA_AGENT_MODE=simulation|diagnostic` 且无 `VBMF_ALLOW_MOCK=1`）下遇 mock+真实 feature 组合 → 启动报错拒启并列冲突 feature；启动日志打印模式+生效 adapter。
- **D5 (P0-5)**：`scripts/check_remove_adapters.py`：临时副本真删 `adapters/blackmagic/`+`adapters/gstreamer/` 并修 mod/main 引用 → `cargo check --no-default-features --features simulation`（+mock）；CI 独立步骤；词法 lint 定位改 Architecture Lint。
- **D6 (P0-6)**：p06-hi 归档 tasks/design 旧口径就地 `[勘误 2026-08-29]` 注记。
- **D7 (P1-1)**：MockBackend 改用 `pipeline::NEXT_PIPELINE_ID`（pub(crate)，从 1 起）；断言非零递增。
- **D8 (P1-2)**：`discover -> Result<Vec<DeviceInfo>, ProviderError>`（BREAKING）；impl 清单：contracts/provider.rs trait + device.rs DeviceManager trait（legacy，同期对齐或标注）+ Filesystem/Simulation impl ×3 + mock A/B + blackmagic device_manager ×2；registry/main 调用点更新；HARDWARE_PROVIDER_CONTRACT.md 修正注记。
- **D9 (P1-3)**：`RuntimeEvent.severity: Observation|Critical`（fault/identity 类 Critical）；Log 满时只挤 Observation，Critical 强推 + `dropped_observations` 计数。
- **D10 (P1-4)**：`ResourceRegistry` Mutex 化 + 原子 `acquire`（锁内 preflight+reserve）+ materialize 失败 `release_reservation` 回滚；main.rs 闸门改走 acquire。

## 关键取舍与风险

- D1 触面最大（8 文件 70+ 处引用），真机 HW-IDENT-02 回归兜底；换取消"换厂商不进 Domain"。
- D2/D8 双 breaking 一次付清，避免 Baseline 后二次 breaking。
- legacy `DeviceManager` trait 与 canonical `HardwareProvider` 并存为本轮新发现：统一收敛到 contracts/provider.rs（DeviceManager 保留为内部别名或删除，design 阶段定）。
- D4 env 信号可误设 → 启动日志显式模式打印缓解。
- D5 脚本只在临时副本操作 + 异常清理。

## 测试策略

全矩阵（default/simulation/mock/bmd,gstreamer × test/clippy -D/build，盒上）+ 新增断言（fail-closed 拒启、acquire 原子性、Critical 不被挤、Mock 句柄非零、remove-adapter 脚本 CI 绿）+ 真机三闭环回归（SELFTEST/loopback/HW-IDENT-02 ManifestVerified 仍 High、Unresolved 仍 fail-closed）。

## Spec Patch

无（skip_specs: true；契约修正落 docs/architecture 对应文档）。
