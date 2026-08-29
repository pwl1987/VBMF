# PHASE_0_7A_POST_MERGE_DEBT — Phase 0.7A 合并后债务清单（冻结）

> 状态：DEBT REGISTER（P1/后续阶段债务登记；逐项不得在 0.7A 之外顺手实现）。
> 来源：`comet/p07-session-runtime` 四轮 Merge Gate 复核记录（verify 报告 §7/§8/§9 附录）+ 终审裁定。
> 纪律：下一阶段（0.7B Media Semantics / 0.7C External Integration）开工前，本清单为**强制阅读项**；
> 任何一项的关闭都必须走独立 change + 三层测试，不允许"顺手修"。

## 生命周期编排（P1-1 类）

| # | 债务 | 现状 | 关闭条件 | 目标阶段 |
|---|------|------|----------|----------|
| D1 | **LifecycleJournal / LifecycleTransaction**：`start()` 各失败点（instantiate/allocate/start/materialize）各自手写 rollback；新增步骤需复制回滚逻辑 | 手写分段 rollback（已覆盖已知失败点，单测锁定） | `CompletedStep[]` 记录 + `rollback(reverse)` 统一引擎；0.7B 新增步骤（Normalize/Clock/Audio/Encoder/Output）接入 | 0.7B 开工前 |
| D2 | **derive_claims FAIL 化（RESOURCE-RESOLUTION-01）**：intent 设备找不到 Resource 时 claims 为空 → Preflight 仅 WARN | WARN（legacy/无 manifest 路径兼容） | 资源解析三态 `Resolved/Missing/Ambiguous`：Missing/Ambiguous → FAIL | 0.7B |
| D3 | **per-claim Reservation TTL**：预留过期目前以 Reserved 相位停留窗口近似（tick 驱动） | 近似实现（crash-cleanup 语义成立） | 每 claim 独立 TTL + 显式 Renew/Expire/Abort 生命周期 | 0.7B |

## Preflight 精确化（P1-2 类）

| # | 债务 | 现状 | 关闭条件 | 目标阶段 |
|---|------|------|----------|----------|
| D4 | **PortAvailability 精确到端口**：当前仅"设备有任意端口" | 设备级判定 | `source.port_id → 具体端口 → direction==Input → capability 兼容 → runtime binding 可用`（SDI/HDMI/IP 混合卡前置条件） | 0.7B（Port Availability Contract） |
| D5 | **IdentityBinding 实查强度（IDENTITY-BINDING-01）**：当前仅 key-existence | contains_key | 实查 `Confidence==High / match_kind==ManifestVerified` + identity strength 分别判定 | 0.7B |
| D6 | **BACKEND-CAPABILITY-01**：BackendCapability stage 恒 WARN 占位 | 占位报告 | 真实能力探针接线 + 硬性判定规则 | 0.7B/0.7C |

## 结构清理（P1-3 类）

| # | 债务 | 现状 | 关闭条件 | 目标阶段 |
|---|------|------|----------|----------|
| D7 | **`backend: OnceLock` → 直接字段**：构造注入已是真实语义，OnceLock 制造"可后换"假象 | OnceLock（行为正确） | 改 `Arc<dyn MediaBackend>` 直字段 | 0.7A 余项/0.7B |
| D8 | **RuntimeEventSink 与 Supervisor 解耦**：SessionManager 经 `Supervisor::record` 出口（Supervisor 既是决策者又是事件表持有者） | 能工作；架构方向倒置 | `RuntimeEventSink → RuntimeEventLog → {Health Reducer, Supervisor, External Projection}` | 0.7C（Event Architecture） |
| D9 | **create 幂等键（Idempotency-Key）**：随机 SessionId 不解决业务幂等 | 随机 UUID | External API 引入 Idempotency-Key 语义 | 0.7C |
| D10 | **Session 内多 Pipeline**：start() 取 `plans.first()`，多设备会话仅物化首计划 | 单管线（租约/资源已全量持有） | 多管线实例编排（含每管线 Health/句柄表） | 0.7B+ |

## 词汇卫生（已裁定不阻塞，随 Identity/Provider 清理）

- `DeviceIdentitySource::RealBmd` 等 vendor-named 枚举值仍在 Domain vocabulary（0.6 终审遗留）。
- `ResolverEvidence.bmd_device_handle` / `DeviceBindingManifest.bmd_device_handle` JSON 键名保留（宿主证据文件，非 Domain schema）。

## 失败矩阵覆盖现状（SESSION-RT-01 / RESOURCE-RT-01）

Preflight✓ Reserve✓ Lease(partial)✓ Binding/Materialize✓ Allocate(partial×multi)✓ Start✓ Stop(fail×single/multi)✓ Close✓ Double-start/stop✓ Tick/Expiry✓ Resource-conflict✓ Multi-device✓。
**未覆盖（登记不阻塞）**：Backend unavailable at create、provider disappeared mid-run（依赖真实硬件故障注入，随 0.7B 硬件验收扩展）。
