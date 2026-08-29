# Tasks: Phase 0.6 C3 (0.6F)

## 1. Mock Provider

- [ ] 新增 `providers/mock/` 实现 `HardwareProvider`（注入设备/身份/故障）
- [ ] Mock `identity()` 返回注入 CanonicalDeviceId；支持注入 AmbiguousIdentity 以验证拒识

## 2. Mock Backend

- [ ] 新增 `backends/mock/` 实现 `MediaBackend`（注入 SourcePlan→事件流）
- [ ] Mock `src_props()` 返回与真实一致的 `connection=` 片段（audio 不设 connection）
- [ ] 按 plan 发出 C2 `RuntimeEvent` 序列

## 3. AdapterRegistry

- [ ] 新增 `registry.rs::AdapterRegistry`，按配置选择 Provider/Backend 实现
- [ ] 与 0.6B+C trait 配合：只暴露 trait 对象

## 4. feature 接入

- [ ] `simulation` feature 编译 Mock 并接入 Registry；`default` 仍最小可编译
- [ ] 与现有 simulation mock 设备语义合并

## 5. 验证（CI 门禁）

- [ ] `cargo clippy --all-targets -- -D warnings`（default + simulation + `bmd-provider,gstreamer-backend`）
- [ ] `cargo test` default + simulation 通过
- [ ] `cargo build --features simulation` 通过且 Mock 链路可启动
