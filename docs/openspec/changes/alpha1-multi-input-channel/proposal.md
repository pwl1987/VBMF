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
