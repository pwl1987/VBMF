---
comet_change: alpha1-multi-input-channel
role: technical-design
canonical_spec: openspec
---

# Design Doc — alpha1-multi-input-channel（Alpha-1: 多输入 + Channel 模型 / D10 激活）

基线: master `d2a24fb`（Prototype-1 收口）。用户 Alpha 路线第一段; 不重开 V0.2/P1 冻结语义。

## 1. 现状锚点（probe 实证 @d2a24fb）

- `session.rs:573 start()` 取 `plans.first()` 只实例化首 plan（= 债务 D10 原文）; 租约/资源**已**按全部 intent 设备持有（D10 注记）——多输入只缺口在实例化编排。
- `MediaSession.pipeline: Option<PipelineHandle>`（单数, 首管线）。
- `GraphRuntimeIntent.devices: Vec` / `materialize` 产 `Vec<PipelinePlan>`（每设备一 plan）——意图面与物化面**已多输入**。
- 债务账本 L31: `D10 Session 内多 Pipeline: start() 取 plans.first(), 多设备会话仅物化首计划 | 单管线 | 多管线实例编排（含每管线 Health/句柄表）| 0.7B+`。
- V0.2 Channel 语义（§929-966）: 健康聚合 HEALTHY/DEGRADED/FAILED + failover/standby/offline 节点——**全语义属 Alpha-5/V0.3**; Alpha-1 取无争议子集。
- 盒上硬件: 2 张采集卡（SDI-IN-1 gst0 + DeckLink SDI gst1, probe 时双卡有信号）+ MINI-MON-4K（输出卡）。
- P1a/P1b 产物: 输出物化（sink.kind 契约词 + PrototypeOutputConfig）、静态控制台、`ApiSession.outputs`。

## 2. 类型设计

### 2.1 session.rs（domain）

```rust
/// Alpha-1: 会话输入摘要（每实例化管线一行; D10 句柄表）。
pub struct SessionInput {
    pub device_id: Uuid,
    pub handle: PipelineHandle,
}
// MediaSession 加法:
pub inputs: Vec<SessionInput>,   // 空 = 未 start; start() 全量回填（序 = plans 序）
// pipeline: Option<PipelineHandle> 保留 = inputs.first()（首输入/主输入, 既有消费者零破坏）
```

### 2.2 runtime_state.rs（投影）

```rust
pub struct InputRuntimeSummary { pub device_id: String, pub handle: u64 }
// SessionRuntimeState 加法: pub inputs: Vec<InputRuntimeSummary>
```
顶层 8 键不动（sessions[] 内部投影, D14 契约延续）。**Channel 命名 = 控制台侧规约**
（CH+显示序, 页面对 running 会话编号; 运行时状态**不携带** channel 字段——
sessions 来自 HashMap 迭代, 序不稳定, 状态内编号会漂移; 设计裁决记档）。

### 2.3 api_boundary.rs

`ApiSession.inputs: Vec<ApiInputSummary{id, handle}>` 加法投影。

## 3. 多管线编排（D10 激活）

```
start():
  plans = materialize(intent)                        // 已多 plan（每设备一个）
  handles = []
  for plan in &plans:
      match backend.instantiate(plan):
          Ok(h) => handles.push((plan.source.device_id_uuid, h)),
          Err(e) => { for h in handles: backend.stop(h)   // 逆序回滚已建
                      既有 lease/reservation 回滚 + StartFailed + SessionFailed }
  for h in handles: backend.start(h)                  // 全部启动
  MediaSession.inputs = handles; pipeline = handles.first()
stop()/close(): 既有单句柄路径 → 迭代 inputs 逆序 stop（creator=destroyer 延续）
recover(): per-handle 既有语义零改动（plan 持久于 GstInstance）
```

- 失败原子性: 任一输入实例化失败 ⇒ 已建全部拆除 + 既有回滚链（零孤儿不变量延续, 复用 SessionFailed 既有事件）。
- 借注: `plans` 生命周期——先 collect handles 再构造 MediaSession 更新, 避免 borrow 冲突（实现期编译器裁定）。

## 4. 输出策略（单输出承诺延续, design D3）

`materialize_outputs`: **仅 intent.devices[0]** 的 sink.kind 消费输出物化; 其余设备强制纯分析
（输出段空, 无论 sink.kind 声明——多输出/混流=Alpha-2/3）。单输入行为逐字节不变。
log warn: 非首设备声明输出时提示"Alpha-1 单输出承诺: 仅首输入物化输出"。

## 5. 诊断接线 + 控制台

- `VBMF_DIAG_INPUTS`（usize, 默认 1）: 诊断 auto-start intent 取**已绑定**设备前 N 个
  （manifest 绑定序; N>可用绑定数 ⇒ 取全部并 warn）。无 env ⇒ 现行为（首设备）。
- 控制台: CH01 行（会话聚合）+ 每输入行（设备 id 尾 4 位 + handle + activity 由
  /api/v1/runtime sessions[].inputs 投影; SDI 锁定 = 输入级信号状态经 device 维度心跳）。
  Channel 聚合显示: 全输入 present=HEALTHY 色, 任一缺失=DEGRADED（页面由 inputs 长度
  对比 VBMF_DIAG_INPUTS 期望数推导——诚实来源为服务端 inputs 投影本身）。

## 6. 测试策略

- Unit: 多 plan 句柄表回填; 双设备实例化失败中途回滚零孤儿; stop 全句柄;
  投影（inputs/channel 无字段确认）; materialize 仅首设备输出段; 单输入零回退。
  mock 基线 245 全绿 + 新增。
- Hardware（盒 `~/a1_gate.sh` 不入库）: A1-01 双卡双管线 PLAYING; A1-02 双 appsink
  独立帧计数; A1-03 首输入 HLS 输出持续 + 次输入纯分析; A1-04 诚实性（无信号输入 ⇒
  该路停滞可见 + 另一路不受扰; 双卡当前信号实况自适应断言）; A1-05 stop 零孤儿;
  A1-06 控制台输入行; A1-07 全回归（P1a+P1b gate+矩阵+lifecycle+loopback+transport）。

## 7. 风险与缓解

| 风险 | 缓解 |
|------|------|
| 双 DeckLink 并发打开资源争抢 | 盒上双卡独立设备号; gate 实证; 失败即 A1-01 暴露 |
| 停止路径漏停某句柄（孤儿管线占卡） | 逆序迭代 + 零孤儿 Unit + A1-05 后续 start 可再占卡验证 |
| 多输入下事件/健康归属混淆 | 事件已带 device/session 维度; 句柄表投影显式 per-input |
| HashMap 序致页面编号漂移 | Channel 命名=页面侧规约（§2.2 裁决）, 状态不携带序 |
| 回归面大（session 核心） | 单输入路径逐字节兼容断言 + 全回归 gate |

## 8. 契约冻结点

- 单输入行为逐字节不变（无 VBMF_DIAG_INPUTS ⇒ 现诊断路径原样）。
- 输出仍单路绑首输入（P1a/P1b gate 不变）。
- V0.2 Channel 全语义（failover/standby/FAILED）不进本 change（Alpha-5/V0.3）。
- 顶层 8 键 / 五端点 / commands 平面零触碰。
