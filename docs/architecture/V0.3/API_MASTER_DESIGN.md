# VBMF V0.3 API 总体设计

> **状态：顶层接口冻结**  
> **版本：V0.3**  
> **定位：VBMF 控制、查询、命令、事件及预览/媒体传输接口的总体规范**

## 1. 目的与权威关系

本文件是 V0.3 接口层面的总体事实源。它冻结接口分层、`/api/v1` 版本根、顶层领域命名空间、控制/查询/事件边界、身份上下文、错误、幂等、版本兼容及扩展规则。

字段级契约、状态机、命令载荷、查询条件和事件载荷必须在对应领域详细设计中继续确定，并不得违反本文件和 V0.3 总体设计中的冻结红线。

## 2. 接口在系统中的位置

```text
浏览器 / 桌面端 / 外部系统
          ↓
       接口边界
          ↓
   ┌──────┼──────┐
  查询    命令    事件
   ↓       ↓       ↓
运行时事实  控制面  事件投影
          ↓
       VBMF 运行时
          ↓
       媒体平面
```

接口是运行时的控制与观察边界，不是运行时本身。接口层不得直接实现媒体处理，不得暴露 GStreamer、FFmpeg 或厂商 SDK 对象。

## 3. 四类语义平面

### 3.1 查询

查询回答“现在是什么”，必须是只读观察，不得产生隐藏的运行时副作用。

典型形式：

```text
GET /api/v1/runtime
GET /api/v1/nodes
GET /api/v1/sources
GET /api/v1/sessions/{id}
GET /api/v1/playout/channels/{id}/now-playing
```

### 3.2 命令

命令表达“请求运行时做什么”。危险动作不得通过隐式状态修改表达，而应使用明确命令，例如：

```text
POST /api/v1/production/switches/{id}/cut
POST /api/v1/production/switches/{id}/take
POST /api/v1/playout/sessions/{id}/start
POST /api/v1/ha/groups/{id}/failover
```

命令必须经过权限、策略、校验、幂等及实际执行路径。

### 3.3 事件

事件表达“已经发生了什么”，不是隐藏命令通道。例如：

```text
runtime.session.started
media.signal.lost
production.switch.completed
playout.item.started
ha.failover.completed
```

事件必须保留事实语义，并遵守母框架规定的事件信封。

### 3.4 预览与媒体传输

实时视频预览必须使用独立媒体传输，不得把实时媒体主链路塞进普通 JSON 控制接口。

```text
快照
MJPEG
WebRTC
HLS
未来媒体传输
```

接口只负责建立、查询、关闭预览会话；实际媒体由独立媒体传输承担。

## 4. 接口版本

统一版本根：

```text
/api/v1
```

演进原则：

```text
优先增加
向后兼容
显式弃用
破坏性变化使用新大版本
```

必须区分接口版本、契约版本、对象版本、配置版本、策略版本、能力版本和运行时版本。客户端不得依赖内部运行时版本实现业务判断。

## 5. 顶层领域命名空间

V0.3 冻结以下顶层命名空间：

```text
运行时 / 资源：
/runtime /nodes /devices /resources /capabilities /workloads /placement /allocations /scheduler

媒体 / 图：
/adapters /inputs /sources /signals /media /streams /pipelines /graphs

编解码 / 处理：
/codecs /encoders /decoders /transcoders /containers /muxers /demuxers /processors /filters

音视频：
/audio /video /audio/routes /audio/mixers /audio/meters /video/processors

制作：
/production /production/switches /production/program /production/preview /production/scenes /tally /control-surfaces /gpio /gpi

播出：
/playout/channels /playout/schedules /playout/events /playout/items /playout/now-playing /playout/next /playout/queues /playout/rules /playout/sessions

播出保护 / 高可用：
/broadcast/policies /broadcast/overlays /broadcast/actions /broadcast/guards /broadcast/rules
/ha/nodes /ha/clusters /ha/groups /ha/roles /ha/failover /ha/policies /ha/assignments /ha/leader /ha/lease /ha/epoch /ha/quorum /ha/replication

同步：
/synchronization /clocks /clock-domains /timebases /timecodes /ptp /genlock /frame-sync

网关 / 网络 / 输出：
/gateway /gateway/sessions /transports /network/nodes /network/ports /network/links /network/routes /network/graphs /outputs /deliveries

预览 / 录制：
/preview /preview/sessions /snapshot /recordings /replays /clips /timeshift

可观测性：
/health /metrics /events /alerts /incidents /diagnostics /audit /evidence
```

以上是顶层契约声明，不代表 P1 必须一次实现全部接口。

## 6. 资源接口规则

默认资源接口可以采用：

```text
GET 集合
POST 创建
GET /{id}
PATCH /{id}    仅用于语义上允许的局部修改
DELETE /{id}   仅用于领域生命周期允许的删除
```

实时播出领域的切换、故障切换、启动、停止、提升、回滚等危险动作不得通过隐式 PATCH 表达。

## 7. 命令契约

生产命令必须能够关联：

```text
command_id
command_type
request_id
correlation_id
actor
authority_context
idempotency_key
expected_version / expected_epoch
reason
created_at
```

命令结果必须能够区分：

```text
accepted
rejected
executing
completed
failed
already_applied
conflict
```

异步命令使用 `operation_id` 追踪。HTTP 成功不等于媒体动作已经完成。

**P1 约束：**V0.2 已验证的四类命令及既有幂等语义暂时保持不变；V0.3 新的完整命令字段在后续领域契约阶段逐步引入，不在 P1 中整体替换既有契约。

## 8. 幂等

高风险命令必须支持幂等，至少覆盖切换、故障切换、启动、停止、提升、回滚、应用等动作。

必须区分：

```text
同一请求重复发送
同一资源版本冲突
同一动作已经完成
同一幂等键对应不同载荷
```

幂等语义不得与命令状态或错误分类混为一体。

## 9. 错误契约

V0.3 跨领域语义错误至少包括：

```text
VALIDATION_ERROR
AUTHENTICATION_ERROR
AUTHORIZATION_ERROR
NOT_FOUND
CONFLICT
IDEMPOTENCY_CONFLICT
POLICY_DENIED
DEPENDENCY_ERROR
TIMEOUT
RATE_LIMITED
CONTRACT_ERROR
INTERNAL_ERROR
RUNTIME_UNAVAILABLE
RESOURCE_UNAVAILABLE
SIGNAL_NOT_READY
SYNC_NOT_READY
CONTINUITY_RISK
VENDOR_ADAPTER_ERROR
```

厂商详细错误可以进入 `details`，客户端不得通过解析厂商错误文本判断核心业务语义。

P1 不重写 V0.2 已冻结的错误分类；增加的是接口到 HTTP 状态码的映射层。

## 10. 身份与上下文

跨系统请求至少预留：

```text
request_id
correlation_id
trace_id
idempotency_key
actor
authority_context
organization / tenant
site
```

运行时对象至少保持：

```text
object_type
object_id
object_version / revision
source_system
```

身份权威由母框架或上层平台提供；Standalone 可以使用本地提供者，但不得改变标准语义。

## 11. 查询规则

查询可以按领域逐步增加：

```text
state
status
capability
node
resource
source
pipeline
channel
time range
version
health
```

禁止通过查询参数隐藏命令，例如：

```text
GET /sessions/{id}?auto_recover=true
```

必须拆为明确的查询和恢复命令。

## 12. 事件规则

事件遵守母框架事件信封：

```json
{
  "event_id": "...",
  "event_type": "production.switch.completed",
  "event_version": "1",
  "occurred_at": "...",
  "producer": "vbmf-runtime",
  "subject": "switch/...",
  "correlation_id": "...",
  "payload": {}
}
```

事件表达事实。事件传输可以使用轮询、SSE、WebSocket 等方式，但传输机制不能改变事件语义。

P1 继续使用现有事件投影与轮询，不引入 SSE/WebSocket。

## 13. 预览接口边界

预览接口负责建立、查询和关闭预览会话：

```text
POST /api/v1/preview/sessions
GET /api/v1/preview/sessions/{id}
DELETE /api/v1/preview/sessions/{id}
```

实际媒体使用独立媒体传输地址或会话承载。

## 14. 接口扩展规则

新增能力优先沿以下路径扩展：

```text
已有领域
  ↓
已有资源
  ↓
已有能力
  ↓
新的子资源 / 命令
  ↓
只有在语义确实全新时才建立新领域
```

禁止因为增加功能就平行制造新的 `encoder-v2`、`new-switch`、`advanced-playout`、`ai-broadcast`、`vendor-bmd` 等接口体系。

确需新增领域时，必须说明归属、对象模型、状态机、命令、查询、事件、兼容性及为什么已有领域无法承担。

## 15. 扩展字段

允许：

```text
extensions
metadata
labels
annotations
vendor_extensions
```

但核心语义不得隐藏在扩展字段中；厂商字段不得污染标准契约；扩展不得形成第二套领域模型；长期稳定且跨系统需要的字段才可晋升为标准字段。

## 16. 兼容性

默认采用增加式演进：

```text
增加字段
增加枚举值
增加接口
增加可选能力
```

破坏性变化必须有新契约版本、迁移方案、兼容窗口、真实消费者证据、弃用过程和最终移除步骤。

历史接口语义禁止静默改变。

## 17. Standalone 与集成模式

Standalone：

```text
浏览器
  ↓
VBMF 接口
  ↓
VBMF 运行时
```

集成模式：

```text
母平台
  ↓
集成边界
  ↓
VBMF 接口 / 运行时契约
```

两种模式可以替换身份、注册、认证、策略提供者，但不能改变领域语义、运行时执行、命令、查询和事件语义。

## 18. 安全边界

接口必须预留：

```text
认证
授权
权威上下文
审计
限流
重放保护
请求校验
敏感字段脱敏
```

具体企业身份系统不在 VBMF 核心接口内重新实现。

高风险动作必须具备操作者、权威上下文、原因、关联标识、幂等和审计信息。

## 19. 接口与媒体平面分离

禁止普通控制接口承担实时媒体主链路，例如原始视频帧或原始音频采样流。

允许接口传输控制数据、快照元数据、健康状态、指标和描述信息；专业实时媒体主链路必须走独立媒体传输。

## 20. 接口治理

新增接口必须依次经过：

```text
需求
 ↓
事实所有权
 ↓
领域归属
 ↓
已有资源检查
 ↓
契约影响
 ↓
兼容性
 ↓
安全 / 权威
 ↓
幂等
 ↓
事件影响
 ↓
消费者证据
 ↓
契约测试
```

没有明确所有者、生命周期、兼容策略和真实消费者证据的新接口，不得进入稳定接口集合。

## 21. 软件开发工具包原则

`vbmf-sdk` 必须遵循“契约优先、真实消费者优先”：

```text
稳定领域契约
 → 接口契约
 → 契约测试
 → 真实消费者证据
 → SDK
```

SDK 不得暴露 Rust 内部结构、GStreamer/FFmpeg 内部对象、厂商 SDK 对象或数据库模型。

V0.3 当前阶段不以 SDK 先行。

## 22. P1 接口范围

P1 的目标不是实现全部 V0.3 顶层接口，而是激活 Standalone 运行时控制面：

```text
启动
 ↓
运行时就绪
 ↓
健康查询
 ↓
运行时查询
 ↓
命令提交
 ↓
运行时执行
 ↓
事件投影
 ↓
幂等重放
 ↓
正常关闭
```

P1 保持现有已验证接口形状和核心语义，后续阶段再按领域逐步实现完整命名空间。
