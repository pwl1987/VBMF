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
