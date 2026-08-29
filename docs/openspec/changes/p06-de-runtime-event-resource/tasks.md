# Tasks: Phase 0.6 C2 (0.6D+E)

## 1. RuntimeEvent 模型

- [ ] 新增 `events.rs` 定义 `RuntimeEvent` 枚举（全生命周期成员）
- [ ] 定义 vendor 错误 → `RuntimeEvent` 的映射 trait / 辅助

## 2. Supervisor 归一化

- [ ] `supervisor.rs` 改为唯一 `RuntimeEvent` 出口，消费 Provider/Backend 上抛事件
- [ ] Health / RPC / 日志改为只消费 `RuntimeEvent`，移除直接 vendor 错误依赖

## 3. Resource 模型

- [ ] 新增 `resource.rs`：`Resource` + 状态机（Available→Reserved→Allocated→Releasing→Faulted），对齐 V0.2 §3.11
- [ ] Resource 由 Discovery（`DeviceCapabilities` / `PortRegistry`）派生

## 4. Preflight 闸门

- [ ] `materialize` 入口前置 `preflight(plan, resources)`：可用性 + 冲突预留 + 身份 Resolve 校验
- [ ] 失败返回 `AmbiguousIdentity` / `ResourceUnavailable`，由 Policy 决策；禁止静默回退

## 5. 验证（CI 门禁）

- [ ] `cargo clippy --all-targets -- -D warnings`（default + `bmd-provider,gstreamer-backend` 两套 feature）
- [ ] `cargo test` default + simulation 通过
- [ ] `cargo build --features bmd-provider,gstreamer-backend` 通过
