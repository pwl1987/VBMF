# Comet Design Handoff

- Change: alpha1-multi-input-channel
- Phase: design
- Mode: compact
- Context hash: f3f148281049b1108f2fae71d12497405aa02b1ad437c4fcb5e61b2150ba4471

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/alpha1-multi-input-channel/proposal.md

- Source: docs/openspec/changes/alpha1-multi-input-channel/proposal.md
- Lines: 1-37
- SHA256: 348b3c9cd4f4c78963f0a6286c89c642cc226aaf3d4083aaeeddcb149d37aa20

```md
# Proposal — alpha1-multi-input-channel

## Why

Prototype-1 收口后（master=`d2a24fb`）, VBMF 有了单路完整链路（SDI→编码→HLS/RTMP→浏览器）, 但整条链假设**单输入**。Alpha 产品化纵切第一段（用户 2026-09-02 裁定路线 Alpha-1..6）: 让 VBMF 同时吃多路真实 SDI 并以 **Channel** 聚合呈现——这是 Switch/Program Master（Alpha-2）的直接前置。

代码级缺口（probe 实证 @d2a24fb）:
- `GraphRuntimeIntent.devices` 已是 `Vec`（多输入意图面已存在）, `materialize` 已按设备产出多 `PipelinePlan`, 租约/资源也已全量持有——**但 `session.rs start()` 取 `plans.first()` 只实例化第一个管线**（= 登记债务 **D10: Session 内多 Pipeline**）。
- 运行时可见性单管线: `MediaSession.pipeline: Option<PipelineHandle>`（单数）; 控制台 CH01 行无输入维度。
- Channel 概念: 冻结文档 V0.2 已定义 Channel 健康聚合语义（HEALTHY/DEGRADED/FAILED + failover/standby 节点, §929-966）——代码零实现。
- 硬件: 盒上 2 张采集卡（SDI-IN-1 gst0 + DeckLink SDI gst1）, probe 时双卡均有信号（真多输入可验证）。

## What Changes

- **D10 激活——Session 内多 Pipeline**: `start()` 实例化**全部** plans; 会话持 per-input 管线句柄表（`MediaSession.pipeline: Option<Handle>` → 加法 per-input 表, 单输入行为向后兼容）; stop/close/release 逆序停全部; Supervisor/health 维度按管线句柄（既有 device_uuid 维度兼容）。
- **Channel 模型（Alpha-1 保守子集）**: Channel = 命名的多输入聚合容器（1 Channel = 1 Session = N inputs）。健康聚合**只实现保守投影**: 全输入健康=HEALTHY / 任一输入信号缺失=DEGRADED（V0.2 failover/standby 节点语义**不进本 change**, 留 Alpha-5/V0.3——不重开冻结语义, 只做其无争议子集）。
- **运行时可见性**: `SessionRuntimeState` 增 per-input 摘要（device/句柄/帧活性）; `ApiSession` 投影; 控制台 CH01 展开输入行（每输入 SDI 锁定状态）。
- **诊断接线**: 诊断 auto-start 可选多设备（env `VBMF_DIAG_INPUTS=2` 指定输入数, 默认 1 = 现行为）。

## Non-Goals

- Switch/Program Master/混合（Alpha-2）; 多输出/录制（Alpha-3）; failover/standby 节点与 Channel FAILED 聚合全语义（Alpha-5/V0.3）
- Channel 独立生命周期/ChannelConfig 版本化（V0.2 X2 Playout/Channel 静态检查面属后续）
- transport 五端点/commands 语义任何改动（Channel 经既有 session/runtime 投影呈现）
- Federation / Control Plane

## 验收场景（Gate A1-01..07 草案）

1. **A1-01** 双卡真 SDI: 诊断 2 输入, 两路管线同时 PLAYING
2. **A1-02** 每输入分析链独立存活（双 appsink 帧计数各自持续）
3. **A1-03** 双输入编码输出（HLS 分片持续; 单输出聚合自两路之一=CH01 program 源, Alpha-1 不混流——Program 输出来自输入 0, 输入 1 独立编码产出第二 HLS 流或仅分析, 由设计定）
4. **A1-04** 单输入故障诚实性: 拔一路信号（或选当前无信号卡）⇒ Channel=DEGRADED 如实、另一路不受影响
5. **A1-05** Stop/Close 逆序零孤儿（双管线全停, 资源/租约全还）
6. **A1-06** 控制台多输入行显示真实状态
7. **A1-07** 既有全回归零退化（P1a+P1b gate + 矩阵 + 单输入行为逐字节兼容）

完成定义: **两路真实 SDI 同时进入 VBMF 并各自可观察、可停止, Channel 聚合状态诚实**。

```

## docs/openspec/changes/alpha1-multi-input-channel/design.md

- Source: docs/openspec/changes/alpha1-multi-input-channel/design.md
- Lines: 1-37
- SHA256: 50eabfa267e53ba15b71adbbacc778d28ba4754237a4c62ae18f14a027afbc54

```md
# Design — alpha1-multi-input-channel（高层框架）

## D1 D10 激活: 多管线实例编排

```
materialize(intent N devices) → plans[N]
start():
  for plan in plans: backend.instantiate(plan) → handle_i   (逐个; 失败逆序回滚已建)
  会话管线表: inputs: Vec<InputRuntime{ device_id, handle, … }>
  全部 start → RUNNING
stop()/close(): 逆序 stop 全部句柄（creator=destroyer, 零孤儿不变量延续）
recover(): per-handle 既有语义（plan 已持久于 GstInstance）
```

- `MediaSession.pipeline: Option<PipelineHandle>` **保留**（= 首输入/主输入, 向后兼容既有消费者）, 加法 `inputs: Vec<InputSummary>`（device_id + handle + kind）。
- 租约/资源: 已全量持有（D10 注记）——零改动。

## D2 Channel 模型（保守子集）

- Channel **不是新运行时实体**（Alpha-1 不建独立 Channel struct 生命周期）: Channel = Session 的多输入聚合**命名投影**——"CH01" 即首会话; `SessionRuntimeState` 加法 `channel: String`（"ch01" 命名规约: 会话序号）+ `inputs: Vec<InputSummary>`。
- 健康聚合保守投影: 全输入帧活性健康 = `healthy`; 任一输入无帧 = `degraded`。**不做** V0.2 standby/offline/FAILED 全语义（显式记档于 debt D10 行关闭语）。
- 帧活性来源: 既有 per-handle appsink 计数（MEDIA-RT-01 心跳数据面）。

## D3 输出策略（单输出承诺延续）

Alpha-1 **不混流**: Program 输出（HLS/RTMP）绑定**输入 0**（CH01 主输入）; 其余输入 Alpha-1 仅分析（appsink）+ 运行时可见。materialize: 仅首 plan 物化输出段, 其余纯分析——保证既有单输出契约与 P1a/P1b gate 不变。多输出/混流=Alpha-2/3。

## D4 诊断接线与控制台

- `VBMF_DIAG_INPUTS`（默认 1=现行为）: 诊断 intent 取前 N 个已绑定设备。
- 控制台: CH01 行下增输入行（每输入: 设备名 + SDI 锁 + 帧活性）; Channel 状态 = 聚合投影。

## D5 验证

- Unit/Simulation: 多 plan 实例化/逆序回滚/句柄表投影/聚合状态; 单输入路径零回退（mock 基线 245）。
- Hardware: Gate A1-01..07（proposal; A1-04 用双卡当前信号实况——probe 时双卡有信号, gate 顺序自适应: 若卡 1 无信号则该路天然 DEGRADED 断言语义仍成立）。
- 回归: P1a+P1b gate + 矩阵 + lifecycle/loopback/transport。

```

## docs/openspec/changes/alpha1-multi-input-channel/tasks.md

- Source: docs/openspec/changes/alpha1-multi-input-channel/tasks.md
- Lines: 1-24
- SHA256: 76721bb814c66c884831ddb49ae15bd06d3c5ba5518953df4cc8f21f2fac2bc6

```md
# Tasks — alpha1-multi-input-channel

> 四栏纪律：每项标注 `Contract` / `Implementation` / `Verification` / `Gate`。

## 1. 多管线编排（session.rs, D10 激活）

- [ ] 1.1 `MediaSession.inputs: Vec<InputSummary>` 加法（device_id+handle; `pipeline` 首输入保留兼容）+ create 初始化空 + start 全量回填 `Contract: design D1/D2 / 债务 D10` | `Implementation: 待` | `Verification: Unit——双 plan 会话句柄表` | `Gate: 无`
- [ ] 1.2 `start()` 实例化**全部** plans（逐个; 任一失败逆序回滚已建零孤儿）; stop/close 逆序停全部 `Contract: design D1` | `Implementation: 待` | `Verification: Unit——多设备回滚/停止零孤儿` | `Gate: 无`

## 2. Channel 投影 + 输出策略

- [ ] 2.1 `SessionRuntimeState` 加法 `channel: String` + `inputs: Vec<InputSummary>` 投影; `ApiSession` 投影; 顶层 8 键不动 `Contract: design D2` | `Implementation: 待` | `Verification: Unit——投影/8 键测试原样` | `Gate: 无`
- [ ] 2.2 materialize 输出策略: **仅首 plan 物化输出段**, 其余纯分析（单输出承诺, P1a/P1b gate 不变） `Contract: design D3` | `Implementation: 待` | `Verification: Unit——多设备 intent 仅首 plan 有 outputs` | `Gate: 无`

## 3. 诊断接线 + 控制台

- [ ] 3.1 `VBMF_DIAG_INPUTS`（默认 1）诊断多输入 `Contract: design D4` | `Implementation: 待` | `Verification: 无 env 行为不变` | `Gate: 无`
- [ ] 3.2 控制台输入行 + Channel 聚合状态显示 `Contract: design D4` | `Implementation: 待` | `Verification: Hardware gate A1-06` | `Gate: A1-06`

## 4. Gate 与交付

- [ ] 4.1 Hardware Gate A1-01..07（盒上双卡真机） `Contract: proposal 验收场景` | `Implementation: 待` | `Verification: 盒上实跑全 PASS` | `Gate: A1`
- [ ] 4.2 既有全回归（P1a+P1b gate+矩阵+lifecycle+loopback+transport）零退化 `Contract: 验收口径` | `Implementation: 待` | `Verification: 盒上全 PASS` | `Gate: BOX`
- [ ] 4.3 债务账本 D10 行 CLOSED + review + CI + verify 报告 + archive + PR + merge `Contract: 交付纪律` | `Implementation: 待` | `Verification: PR merged` | `Gate: CI/RELEASE`

```
