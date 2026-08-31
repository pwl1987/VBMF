# Canonical → Runtime Integration Audit — p07b-consolidation（2026-08-30）

- 审计对象：`services/media-agent/src/` @ master `c574238`（0.7B-2C 合并后）
- 审计性质：代码级（file:line 证据；仅陈述代码所示，无推测）
- 目标（终审 §七）：①Provider→Canonical 统一性 ②Canonical→Runtime Intent 旁路 ③Canonical→Backend 直连

## 总判定：三区全部 CLEAN

| # | 审计区 | 判定 | 关键证据 |
|---|---|---|---|
| 1 | Provider → Canonical 统一 | **CLEAN**（+P2 观察） | `RawInputDescription::from_port`（normalize.rs:137-158）是唯一观测装配路径，唯一生产调用点 main.rs:355（loopback 证据段）；normalize.rs 零 provider 分支（只读中性 PortInfo 字段）；接缝是 `PortRegistry::build`（port.rs:380-496，输入=manifest 绑定+GStreamer probe+Resolver 绑定，非 BMD SDK 观测直通）；provider 无关性由 normalize.rs:342-360 测试钉死（含 provider_binding_ref 不泄漏断言） |
| 2 | Canonical → Runtime Intent 旁路 | **CLEAN** | Canonical 类型（CanonicalMediaDescriptor/NormalizeOutcome/clock/audio/timecode 类型族）被 runtime 代码消费为**零**（grep session/pipeline/preflight/graph_intent/rpc/resource/lease/supervisor/health/events/registry = 0 命中）。`SourcePlan.device_number` 仅来自 Resolver 绑定（pipeline.rs:447←resolver.rs:527-558/998-1022——冻结 "Binding = Physical→Provider→Runtime Resource" 合法路径，生产缺绑定 fail-closed pipeline.rs:470-482）；`provider_persistent_id`←绑定←ProviderIdentity（SPI 证据层，字段名已中立化）；`connector`←Manifest 声明经 PortRegistry（port.rs:413 HARD RULE 绝不从信号推断）。preflight.rs/session.rs 零读取 signal/video_format/audio_locked |
| 3 | Canonical → Backend 直连 | **CLEAN** | 四 canonical 模块（normalize/clock/audio/timecode）import 仅 serde/uuid/`crate::port`/互相——零 gstreamer/adapter/vendor 符号（"bmd/gstreamer" 字样仅存在于测试的禁词断言）；`GStreamerPipelineController` 只消费 `PipelinePlan`（controller.rs:10-30,194）；vendor 字符串拼装点 `src_props` 位于 pipeline.rs:371-420（编排层），依赖方向正确 |

## 结构性事实（最重要结论）

**Canonical 层与 Runtime 层目前是两个不相交子图**，仅在 `PortInfo`（normalize 输入）与 `device_id/port_id`（intent 键）处相邻：

- 四个 canonical 输出（normalize/clock/audio/timecode）全部位于 main.rs:355-416 的 `VBMF_LOOPBACK` 证据段，打印 JSON 后 `exit(0)`（main.rs:422）；
- `SessionManager` API（session.rs:251-276）不含任何 canonical 输入；create/start/stop/tick（session.rs:330-846）零引用 canonical 类型；
- `AudioRouteIntent` 无任何非测试构造点（audio.rs 测试外零调用）；`CanonicalClockDomain` 仅 main.rs:378 证据构造；`CanonicalTimecode` 仅 normalize.rs:283（恒 unknown）+ main.rs:407。

**推论**：最高红线当前"平凡满足"（不存在违规路径，因为消费路径本身还不存在）。0.7B→0.7C 之间的真正工作**不是移除旁路，而是补上 "Canonical → Runtime/Policy" 这条边**——届时本次审计验证的边界（`SourcePlan` 仅由 Resolver 绑定 + manifest 声明连接器喂养、绝不来自观测字段）将成为**承载性不变量**，必须在接线 change 中以测试固化。

## P2 观察（不阻塞，如实记录）

Mock Provider 无生产 canonical 路径：`adapters/mock.rs` 只实现 discover/MediaBackend，其媒体观测从不流经 normalize——"Provider unification" 目前由 normalize.rs/audio.rs 的测试形状证明（BMD 形状 vs Mock 形状 → 同一 canonical 表征），而非活的 Mock 管线。这符合 normalize judge-only 的冻结设计；当 0.7C 之前补 "Canonical→Runtime" 边时应把 Mock 观测装配接入测试世界，使 unification 从"测试证明"升级为"管线证明"。

## 与债务清单的衔接

- 本审计结论直接支撑 `PHASE_0_7A_POST_MERGE_DEBT.md` 的优先级重分类（D2/D4/D5/D6 为 0.7C 前必须）。
- "补 Canonical→Runtime/Policy 边" 本身登记为 0.7C 前置工作（见 PHASE_IMPLEMENTATION_MAP §3 顺序首项 "Canonical Runtime State"）。
