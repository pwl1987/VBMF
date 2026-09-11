# VBMF V0.3——API 总体设计

> **状态：顶层 API 冻结，进入领域契约详细设计**  
> **版本：V0.3**  
> **定位：VBMF Control / Query / Command / Event API 的顶层规范、语义边界、版本策略、扩展机制与 SDK 演进规则**

本文件是 VBMF API 顶层规范。具体领域的字段级 Schema、状态机、Command Payload、Query Filter、Event Payload 在各领域详细设计中继续细化，但不得违反本文件。

---

## 1. API 冻结裁决

V0.3 API 冻结以下内容：

- API 分层；
- `/api/v1` 版本根；
- Domain namespace；
- Control / Query / Event 边界；
- Resource 与 Command 语义；
- ID / Context / Error / Idempotency / Version 规则；
- Media Plane 与 Control Plane 分离；
- Extension / Compatibility 规则。

本阶段不冻结所有 endpoint 的最终字段。字段级 Contract 必须按领域确定并经过 Contract Review。

## 2. API 的架构位置

API 是 Runtime 的控制与观察边界，不是 Runtime 本身。

```text
Web / Desktop / External System
              ↓
          API Boundary
              ↓
       Command / Query
              ↓
        VBMF Runtime
              ↓
          Media Graph
              ↓
          Media Plane
```

API 不直接实现媒体处理，也不得暴露 GStreamer、FFmpeg 或厂商 SDK 对象。

## 3. 四类 API 语义平面

严格区分：

```text
Query
Command
Event
Preview / Media Delivery
```

### Query

回答“现在是什么”，不得产生隐式 Runtime 副作用。

### Command

表达“请求 Runtime 做什么”。Command 必须经过 authority、policy、validation、idempotency 和 execution path。

### Event

表达“已经发生了什么”。Event 是事实，不是隐藏的 Command 通道。

### Preview / Media Delivery

预览使用独立媒体交付传输，例如 Snapshot、MJPEG、WebRTC、HLS 等，不得通过普通 JSON Control API 承载实时视频主链路。

## 4. API 版本

统一根路径：

```text
/api/v1
```

默认采用：

```text
增量演进优先
向后兼容
显式弃用
破坏性变化使用新主版本
```

必须区分 API Version、Contract Version、Object Version、Configuration Version、Policy Version、Capability Version 和 Runtime Version。客户端不得依赖内部 Runtime version。

## 5. 冻结的顶层 Namespace

### Runtime / Resource

```text
/api/v1/runtime
/api/v1/nodes
/api/v1/devices
/api/v1/resources
/api/v1/capabilities
/api/v1/workloads
/api/v1/placement
/api/v1/allocations
/api/v1/scheduler
```

### Media / Graph

```text
/api/v1/adapters
/api/v1/inputs
/api/v1/sources
/api/v1/signals
/api/v1/media
/api/v1/streams
/api/v1/pipelines
/api/v1/graphs
```

### Codec / Processing

```text
/api/v1/codecs
/api/v1/encoders
/api/v1/decoders
/api/v1/transcoders
/api/v1/containers
/api/v1/muxers
/api/v1/demuxers
/api/v1/processors
/api/v1/filters
```

### Audio / Video

```text
/api/v1/audio
/api/v1/video
/api/v1/audio/routes
/api/v1/audio/mixers
/api/v1/audio/meters
/api/v1/video/processors
```

### Production

```text
/api/v1/production
/api/v1/production/switches
/api/v1/production/program
/api/v1/production/preview
/api/v1/production/scenes
/api/v1/tally
/api/v1/control-surfaces
/api/v1/gpio
/api/v1/gpi
```

### Playout

```text
/api/v1/playout/channels
/api/v1/playout/schedules
/api/v1/playout/events
/api/v1/playout/items
/api/v1/playout/now-playing
/api/v1/playout/next
/api/v1/playout/queues
/api/v1/playout/rules
/api/v1/playout/sessions
```

### Protection / HA

```text
/api/v1/broadcast/policies
/api/v1/broadcast/overlays
/api/v1/broadcast/actions
/api/v1/broadcast/guards
/api/v1/broadcast/rules
/api/v1/ha/nodes
/api/v1/ha/clusters
/api/v1/ha/groups
/api/v1/ha/roles
/api/v1/ha/failover
/api/v1/ha/policies
/api/v1/ha/assignments
/api/v1/ha/leader
/api/v1/ha/lease
/api/v1/ha/epoch
/api/v1/ha/quorum
/api/v1/ha/replication
```

### Synchronization

```text
/api/v1/synchronization
/api/v1/clocks
/api/v1/clock-domains
/api/v1/timebases
/api/v1/timecodes
/api/v1/ptp
/api/v1/genlock
/api/v1/frame-sync
```

### Gateway / Network / Output

```text
/api/v1/gateway
/api/v1/gateway/sessions
/api/v1/transports
/api/v1/network/nodes
/api/v1/network/ports
/api/v1/network/links
/api/v1/network/routes
/api/v1/network/graphs
/api/v1/outputs
/api/v1/deliveries
```

### Preview / Recording

```text
/api/v1/preview
/api/v1/preview/sessions
/api/v1/snapshot
/api/v1/recordings
/api/v1/replays
/api/v1/clips
/api/v1/timeshift
```

### Observability

```text
/api/v1/health
/api/v1/metrics
/api/v1/events
/api/v1/alerts
/api/v1/incidents
/api/v1/diagnostics
/api/v1/audit
/api/v1/evidence
```

以上是顶层契约空间，不是 P1 全量实现清单。

## 6. Resource API 规则

普通资源原则上使用 GET collection、POST create、GET `/{id}`、必要时 PATCH、在生命周期允许时 DELETE。

实时播出危险动作不得通过隐式状态 PATCH 表达。例如不得用 `PATCH switch.state = ACTIVE` 代替明确的 `POST /switches/{id}/take` 或 `POST /switches/{id}/cut`。

## 7. Command 契约

生产 Command 至少需要能够关联：

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

结果必须区分：

```text
accepted
rejected
executing
completed
failed
already_applied
conflict
```

异步命令可返回 `operation_id`。HTTP 200 不等于媒体动作已经完成。

**P1 保持 V0.2 已冻结并通过测试的四命令契约，不在 P1 擅自升级为新的字段/状态体系。**

## 8. 幂等

高风险 Command 必须具备幂等语义，包括 switch、failover、start、stop、promote、rollback、apply 等。

必须能够区分：

```text
重复请求
资源版本冲突
动作已经完成
相同 key 但 payload 不同
```

幂等语义不得与 ErrorClassification 或 CommandStatus 混为一体。

## 9. 错误契约

跨领域统一预留以下语义错误：

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

厂商细节可进入 `details`，客户端不得通过解析厂商错误文本判断核心业务语义。

V0.3 增加 HTTP 数字状态码映射层，但不得重写 V0.2 的五类 ErrorClassification 冻结语义。

## 10. Context / Identity

跨系统请求至少支持：

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

Identity Authority 仍由母框架或上层平台提供；Standalone 可使用本地 Provider，但不得改变规范语义。

## 11. Query 规则

Query 可以逐步支持 state、status、capability、node、resource、source、pipeline、channel、time range、version、health 等过滤条件，但不得承担隐式 Command。

例如 `GET /sessions/{id}?auto_recover=true` 禁止，应拆分为查询和明确的 recover Command。

## 12. Event API

Event 必须遵守母框架 Event Envelope：

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

Event 表达事实，不表达未来动作。未来可以通过 GET、SSE、WebSocket 等 transport 暴露事件，但 transport 不等于 Event 语义本身。

## 13. Preview API 边界

Preview API 只负责建立、查询和关闭预览会话：

```text
POST /api/v1/preview/sessions
GET  /api/v1/preview/sessions/{id}
DELETE /api/v1/preview/sessions/{id}
```

实际媒体通过独立 Media Delivery URL / Session Transport 提供。

## 14. API 扩展规则

新增 API 优先沿以下路径扩展：

```text
Existing Domain
      ↓
Existing Resource
      ↓
Existing Capability
      ↓
New Sub-resource / Command
      ↓
New Domain only if semantics genuinely new
```

不得因新增一个功能就创建 `/encoder-v2`、`/new-switch`、`/advanced-playout`、`/ai-broadcast`、`/vendor-bmd` 等平行模型。

如果确需新增领域，必须说明既有领域为何不能承载、规范所有者、对象模型、状态机、Commands、Queries、Events、兼容性影响。

## 15. 扩展字段机制

允许保留：

```text
extensions
metadata
labels
annotations
vendor_extensions
```

核心语义不得藏入 extension；厂商字段不得污染 Canonical Contract；extension 不得成为第二套 Domain Model；extension 必须具备版本；长期稳定且跨系统需要的字段才可晋升为 canonical field。

## 16. 兼容性

默认优先支持：

```text
新增字段
新增枚举值
新增 endpoint
可选能力
```

破坏性变化必须具备：

```text
新 Contract Version
迁移方案
兼容窗口
消费者证据
弃用阶段
最终移除
```

历史 API 语义不得静默改变。

## 17. Standalone / Integrated API

Standalone：

```text
Browser → VBMF API → VBMF Runtime
```

Integrated：

```text
母平台 → Integration Boundary → VBMF API / Runtime Contract
```

Identity Provider、Registry Provider、Authentication Provider、Policy Provider 可以不同；Domain、Runtime execution、Command、Query、Event semantics 不得不同。

## 18. 安全边界

API 必须预留 Authentication、Authorization、Authority Context、Audit、Rate Limit、Replay Protection、Request Validation 和 Sensitive-field Redaction。

具体 IAM / Enterprise Identity Provider 不在 VBMF API Core 内重复实现。

高风险操作至少具备 Actor、Authority、Reason、Correlation、Idempotency 和 Audit。

## 19. API 与 Media Plane 分离

禁止把 `/api/v1/video/raw-frame-stream`、`/api/v1/audio/raw-samples` 作为正常实时主链路。

允许通过 `/api/v1/preview/sessions` 建立预览，再由 Media Delivery Transport 承担实际媒体。

API 可以承载小型控制数据、Snapshot metadata、health、metrics 和 descriptors，但不得成为专业实时媒体主总线。

## 20. API 治理

新增 API 必须至少经过：

```text
需求
 ↓
权威归属
 ↓
领域归属
 ↓
既有资源检查
 ↓
契约影响分析
 ↓
兼容性分析
 ↓
安全 / Authority
 ↓
幂等
 ↓
事件影响
 ↓
实现与消费者证据
```

API 的实现不得反向定义领域模型。领域契约稳定后，再生成 OpenAPI / JSON Schema / AsyncAPI、Contract Tests 和 SDK。

## 21. SDK 原则

`vbmf-sdk` 必须遵循 **契约优先、SDK 后置、真实消费者优先**：

```text
稳定 L0 契约
 ↓
领域契约
 ↓
OpenAPI / Schema / Event Contract
 ↓
Contract Tests
 ↓
真实消费者证据
 ↓
vbmf-sdk
```

SDK 不得暴露 Rust 内部结构、GStreamer/FFmpeg 内部对象、厂商 SDK 对象或数据库模型，也不得演变成 Universal Media SDK。

## 22. P1 边界

P1 不实现全部 V0.3 namespace，不引入 Scheduler、Placement、Workload、Allocation、Cluster/HA 全套实现，不引入 SSE/WebSocket，不重构 Error Model，不升级 Command 契约。

P1 的 API 任务是激活 Standalone Runtime Control Plane，并让已有 Query / Command / Idempotency / Event Projection 契约真正连接到 Runtime。
