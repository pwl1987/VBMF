# Verify Report — p07c-external-api（Phase 0.7C-7: External API Foundation — API Boundary Model + Idempotency 契约）

- 日期：2026-08-31 · 验证人：开发 AI（ZCode）· 模式：full workflow
- 分支：`comet/p07c-external-api`（base `9b475c1`）· 提交：`8dfb6fc`（实现）+ `ce06679`（真机 gate + 迭代收敛）
- 前置：Contract Probe（只读四问）`docs/superpowers/reports/2026-08-31-p07c7-external-api-contract-probe.md`
- 结论：**PASS**（0 CRIT / 0 IMP / 2 NOTE）

## 1. 范围对表（终审 0.7C-6 裁定逐项）

| 终审裁定 | 落点（8dfb6fc + ce06679） | 证据 |
|---|---|---|
| **API Resource Model 独立**（NOTE-3，非 Runtime 投影） | 五大独立 API 类型 `ApiDevice/Port/Resource/Session/Capability`，字段仅由 API 消费语义驱动；`to_api_*` 纯函数显式映射 state/phase/direction/can_input/can_output 为 API 字符串，**不绑回** `ResourceState`/`SessionPhase`/`CapabilityFlag` 内部 enum | `api_rt_01_api_models_decoupled_from_runtime_types` + `api_rt_01_to_api_query_models` |
| **不暴露 Rust serde tag 习惯**（NOTE-2） | `ApiCommandStatus` 4 态**不暴露 Failed**（失败归因经 classification 传达）；`ApiErrorClass` 去 `_failure` 后缀（`retryable`/`permanent`）；command_id 接受客户端字符串**不绑回** 内部 `CommandId` 类型 | `api_rt_01_api_command_status_no_failed_state` |
| **三平面不合成万能 ApiResult**（❿/⓫） | Query/Command/Event 各自独立类型；`ApiQuerySnapshot`/`ApiCommandResponse`/`ApiProjectionResponse` 三个**独立**响应模型，无 `ApiResponse<T>` 万能包装 | `api_rt_01_no_transport_no_persistence`（禁 ApiResponse 关键字） |
| **EventProjection ≠ CanonicalRuntimeState**（NOTE-1） | `ApiProjectionResponse.snapshot_kind: ApiProjectionKind::EventProjectionSnapshot` 序列化守门（必含 `"event_projection_snapshot"` 字面量，防伪装权威态） | `api_rt_01_api_projection_kind_enforced` |
| **Idempotency 持久化边界裁决**（三选项公开化） | `ApiIdempotencyBoundary` 契约层：current_backend=ProcessLocal / durable_persistence=DurableLogDeferred / cross_restart_semantics=RestartBreaksReplay；三选项对勘以**全部枚举变体稳定序列化名**锁定（暴露非隐藏，防未来悄悄切换被消费者无感） | `api_rt_01_idempotency_boundary_contract` |
| **transport 保持 std-only 纪律** | 本 change 零 transport 依赖；api_boundary.rs 禁 `axum`/`hyper`/`TcpListener`/`OpenAPI` 等关键字（白盒扫描） | `api_rt_01_no_transport_no_persistence` |
| **API-BOUNDARY-01 白盒门禁** | api_boundary.rs 不得 `use` `backend`/`gstreamer`/`decklink`/`ffmpeg`/`pipeline::`/`provider::` 实现路径（API 是消费者不反向修改） | `api_rt_01_boundary_no_vendor_imports` |

## 2. 三层证据

### Unit/Simulation（盒上，~/p07_results.txt 第五轮 + ~/p07_run_console.log）
- 命令：`bash ~/p07_verify.sh`（cd ~/media-agent-build）
- 结果：14 项全 0（fmt×2 / test×4 / clippy×4 / build×3 / PROOF）
- 测试计数：**138 / 138 / 196 / 138**（mock 188→196，+8：`api_rt_01_{boundary_no_vendor_imports, no_transport_no_persistence, api_models_decoupled_from_runtime_types, api_command_status_no_failed_state, api_projection_kind_enforced, to_api_query_models, api_command_request_field_shape, idempotency_boundary_contract}`）

### Hardware（真机 lytv@10.30.15.10，bmd,gstreamer 构建）
- 命令：`VBMF_SESSION_LIFECYCLE=1 MEDIA_AGENT_DEVICE_BINDING=/home/lytv/loopback-manifest-v2.json timeout 240 ./target/debug/media-agent`
- 结果：**GATE_EXIT=0**（工件 `~/p07_gate_hw.log`）：
  - `EXTERNAL-API-RT-01 verdict=OK devices=3 sessions=1 resources=2 boundary=process_local/durable_log_deferred/restart_breaks_replay`
  - `EXTERNAL-API-RT-01 projection_total=46 snapshot_kind=event_projection_snapshot`（NOTE-1 守门实证）
  - 回归：`EVENT-PROJECTION-RT-01 total=46 has_critical=true dropped 0/0` + `SESSION-RT-01/RESOURCE-RT-01/EXTERNAL-API-RT-01 ALL PASS`

### CI
- PR required checks 以 GitHub 实跑为准（§6）。

## 3. 红线核验

- **API Resource 独立性**：`to_api_*` 显式纯转换，无 type alias 偷渡。
- **Status/Error 独立性**：`ApiCommandStatus`/`ApiErrorClass` 不复用内部 serde 命名。
- **Projection 独立性**：`snapshot_kind` 守门，EventProjection 不伪装 RuntimeState。
- **API-BOUNDARY-01**：零 vendor import（白盒扫描）。
- **终审禁清单 11 项零偷渡**：无 transport/listener/REST/OpenAPI/持久化/跨重启幂等/第二套 RuntimeState/万能包装。
- **零触碰**：runtime_state/runtime_query/command/idempotency/error_model/event_projection/supervisor 零 diff（仅 main.rs mod 声明 + gate 段）。

## 4. 迭代披露（三轮，全部如实）

1. R1：`main.rs` 缺 `mod api_boundary` 声明（第一轮 mock 188 未含 api 测试）——补 mod。
2. R2：`to_api_query_snapshot` 引用 `CanonicalRuntimeState` 不存在的 `capabilities` 字段（capabilities 实为 **per-device** 字段）——改为从 `devices[*].capabilities` 收集；测试构造同步去掉该字段。
3. R3：clippy `clone_on_copy`（`DeviceCapabilitiesSummary` 是 Copy）——`c.clone()`→`*c`；禁清单静态扫描误伤（文档注释/测试自身禁词字面量）——扫描截断到测试模块前并剔除注释行；idempotency boundary 测试改直测全部枚举变体稳定序列化名（boundary 只序列化当前选定值）。

## 5. 文档对账

- Phase Map：0.7C-7 行 ✅ COMPLETE（tag `phase-0.7C7-external-api`）；0.7C §3 下一项 = **Transport 实现**（std-only，单独开 change）。
- 债表：Idempotency 持久化条目登记（进程内 Foundation 已 CLOSED @ 0.7C-4；durable/跨重启语义 deferred to External API stage——本 change 已冻结**契约层**三选项，实现层分步）。

## 6. 分级

- **CRIT：0** · **IMP：0**
- NOTE 1：`to_api_query_snapshot` 的 `capabilities` 从 per-device 收集——当前真机 devices=3 均无 capabilities 投影（`ApiQuerySnapshot.capabilities` 为空）。这是既有 `DeviceCapabilitiesSummary` 投影行为（0.7C-2 RuntimeQuery 已锁定），非本 change 引入；API 模型如实反映，不伪造。
- NOTE 2：`ApiCommandTarget::Session { intent: serde_json::Value }` 用 JSON Value 透传 intent（**不绑回** `GraphRuntimeIntent` 结构）——这是 API 边界对 Semantic Intent 的松耦合表达；若后续 transport 需要强类型 intent，属 transport change 的契约演进，本 change 不提前发明。