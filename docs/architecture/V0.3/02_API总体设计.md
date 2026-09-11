# VBMF V0.3 — API 总体设计

> **状态：已锁定，用于详细 Domain Contract 设计**  
> 日期：2026-09-11  
> 范围：VBMF V0.3 Control / Query / Command / Event API 的顶层 namespace、语义边界、版本策略、扩展机制与 SDK 演进规则。
>
> 本文件是 VBMF API 的顶层 SoT。具体 Domain 的字段级 Schema、State Machine、Command Payload、Query Filter、Event Payload 在各 Domain Detailed Design 中继续细化，但不得违反本文件。

---

## 1. API 冻结决定

VBMF V0.3 API 现在正式进入：

> **顶层 API 已冻结**

冻结的是：

- API 分层；
- `/api/v1` 版本根；
- Domain namespace；
- Control / Query / Event 边界；
- Resource 与 Command 的语义；
- ID / Context / Error / Idempotency / Version 规则；
- Media Plane 与 Control Plane 分离；
- Extension / Compatibility 规则。

**不冻结今天所有 endpoint 的最终字段。**

字段级 Contract 必须在 Domain Detailed Design 阶段逐域确定，并通过 Contract Review 后进入实现。

---

# 2. API 架构定位

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

API 不直接实现媒体处理，也不暴露 GStreamer / FFmpeg / Vendor SDK 对象。

---

# 3. API 的四个语义平面

VBMF API 至少严格区分四类语义：
```text
Query
Command
Event
Preview / Media Delivery
```

## Query

回答“现在是什么”。

例如：
```text
GET /api/v1/runtime
GET /api/v1/nodes
GET /api/v1/sources
GET /api/v1/sessions/{id}
GET /api/v1/playout/channels/{id}/now-playing
```

Query 不得偷偷产生 Runtime side effect。

## Command

表达“请求 Runtime 做什么”。

例如：
```text
POST /api/v1/production/switches/{id}/cut
POST /api/v1/production/switches/{id}/take
POST /api/v1/playout/sessions/{id}/start
POST /api/v1/ha/groups/{id}/failover
```

Command 必须经过 Runtime authority、policy、validation、idempotency 与 execution path。

## Event

表达“已经发生了什么”。

例如：
```text
runtime.session.started
media.signal.lost
production.switch.completed
playout.item.started
ha.failover.completed
```

Event 不是隐藏 Command 通道。

## Preview / Media Delivery

实时媒体预览必须使用独立 Media Delivery Transport：
```text
Snapshot
MJPEG
WebRTC
HLS
Future Media Transport
```

不得通过普通 JSON Control API 搬运实时视频主链路。

---

# 4. API 版本

统一根路径：
```text
/api/v1
```

版本原则：
```text
Additive Evolution First
Backward Compatibility
Explicit Deprecation
Major Version for Breaking Change
```

必须区分：
```text
API Version
Contract Version
Object Version
Configuration Version
Policy Version
Capability Version
Runtime Version
```

API v1 可以内部包含多个 Domain Contract Version，但不得让客户端依赖内部 Runtime version。

---

# 5. 冻结的顶层 Namespace

## Runtime / Resource
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

## Media / Graph
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

## Codec / Processing
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

## Audio / Video
```text
/api/v1/audio
/api/v1/video
/api/v1/audio/routes
/api/v1/audio/mixers
/api/v1/audio/meters
/api/v1/video/processors
```

## Production
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

## Playout
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

## Protection / HA
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

## Synchronization
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

## Gateway / Network / Output
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

## Preview / Recording
```text
/api/v1/preview
/api/v1/preview/sessions
/api/v1/snapshot
/api/v1/recordings
/api/v1/replays
/api/v1/clips
/api/v1/timeshift
```

## Observability
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

---

# 6. Resource API 规则

Resource endpoint 默认遵循：
```text
GET    collection
POST   create
GET    /{id}
PATCH  /{id}       # only when partial mutation is semantically valid
DELETE /{id}       # only when domain lifecycle permits deletion
```

但实时播出领域的危险动作不得用隐式 PATCH 表达。

例如：
```text
PATCH switch.state = ACTIVE
```

不推荐。

应使用明确 Command：
```text
POST /switches/{id}/take
POST /switches/{id}/cut
```

因为它们具有明确的 authority、idempotency、audit、execution 与 continuity semantics。

---

# 7. Command 契约

每一个生产 Command 至少需要能够关联：
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

Command response 必须区分：
```text
accepted
rejected
executing
completed
failed
already_applied
conflict
```

异步 Command 应返回：
```text
operation_id
```

不得把“HTTP 200”直接解释为“媒体动作已经完成”。

---

# 8. Idempotency

高风险 Command 必须支持幂等：
```text
switch
failover
start
stop
promote
rollback
apply
```

Idempotency 不得与 ErrorClassification 或 CommandStatus 合并。

必须能够区分：
```text
同一请求重复发送
同一资源版本冲突
同一动作已经完成
同一 key 不同 payload
```

---

# 9. 错误契约

跨 Domain 至少统一：
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

Vendor-specific detail 可以进入 `details`，但客户端不得解析厂商错误文本判断核心业务语义。

---

# 10. Context / Identity

跨系统 API 请求至少支持：
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

Runtime Domain object 至少保持：
```text
object_type
object_id
object_version / revision
source_system
```

具体 Identity Authority 仍由母框架 / 上层平台提供；Standalone 可以使用本地 provider，但不得改变 Canonical semantics。

---

# 11. Query 规则

Query 必须支持按 Domain 需要逐步增加：
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

但 Query 不得承担隐式 command。

例如：
```text
GET /sessions/{id}?auto_recover=true
```

禁止。

应拆成：
```text
GET  /sessions/{id}
POST /sessions/{id}/recover
```

---

# 12. Event API

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

Event 表达事实，不表达未来动作。

事件可通过：
```text
GET /api/v1/events
SSE /api/v1/events/stream
WebSocket /api/v1/events/ws
```

等 transport 暴露，但 transport 不是 Event semantics 本身。

---

# 13. Preview API 边界

Preview API 只负责建立或查询预览会话：
```text
POST /api/v1/preview/sessions
GET  /api/v1/preview/sessions/{id}
DELETE /api/v1/preview/sessions/{id}
```

实际媒体通过独立 Media Delivery URL / session transport 提供。

这样可以保证：
```text
Control API ≠ Media Transport
```

---

# 14. API 扩展规则

未来新增 API 必须优先：
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

禁止因为新增一个功能就创建平行 API：
```text
/encoder-v2
/new-switch
/advanced-playout
/ai-broadcast
/vendor-bmd
```

优先扩展既有 Domain Contract。

如果确实需要新 Domain，必须说明：
```text
Why existing domain cannot own it
Canonical owner
Object model
State machine
Commands
Queries
Events
Compatibility
```

---

# 15. API 扩展机制

允许对象保留：
```text
extensions
metadata
labels
annotations
vendor_extensions
```

但：

- Core semantic fields 不得藏入 extension；
- Vendor fields 不得污染 Canonical Contract；
- extension 不得成为第二套 Domain Model；
- extension 必须有 version；
- 长期稳定且跨系统需要的 extension 才可晋升为 canonical field。

---

# 16. API 兼容性

默认策略：
```text
Additive Field
Additive Enum Value
Additive Endpoint
Optional Capability
```

优先兼容扩展。

破坏性变化必须：
```text
New Contract Version
Migration Plan
Compatibility Window
Consumer Evidence
Deprecation
Removal
```

历史 API 的语义不能被静默改变。

---

# 17. Standalone / Integrated API

Standalone：
```text
Browser
  ↓
VBMF API
  ↓
VBMF Runtime
```

Integrated：
```text
Mother Platform
  ↓
Integration Boundary
  ↓
VBMF API / Runtime Contract
```

两种模式可以不同：
```text
Identity Provider
Registry Provider
Authentication Provider
Policy Provider
```

但不能不同：
```text
Domain semantics
Runtime execution
Command semantics
Query semantics
Event semantics
```

---

# 18. API 安全边界

API 必须预留：
```text
Authentication
Authorization
Authority Context
Audit
Rate Limit
Replay Protection
Request Validation
Sensitive-field Redaction
```

但具体 IAM / enterprise identity provider 不在 VBMF API Core 内重新实现。

高风险操作必须具备：
```text
Actor
Authority
Reason
Correlation
Idempotency
Audit
```

---

# 19. API 与 Media Plane 分离

禁止：
```text
POST /api/v1/video/raw-frame-stream
POST /api/v1/audio/raw-samples
```

作为正常实时主链路。

允许：
```text
POST /api/v1/preview/sessions
```

然后由 Media Delivery Transport 承担实际媒体。

API 可以传输小型控制数据、Snapshot metadata、health、metrics、descriptors，但不得成为专业实时媒体主总线。

---

# 20. API 治理

新增 API 前必须经过：
```text
Requirement
 ↓
Ownership
 ↓
Domain
 ↓
Existing Resource Check
 ↓
Contract Impact
 ↓
Compatibility
 ↓
Security / Authority
 ↓
Idempotency
 ↓
Event Impact
 ↓
Observability
 ↓
Test Gate
```

任何 API 如果不能回答：
```text
Who owns this?
What object does it operate on?
What state transition does it cause?
Is it idempotent?
What fact/event follows?
How is failure represented?
How is it observed?
```

不得进入生产实现。

---

# 21. SDK 策略 — 先 Contract 后 SDK

SDK **不是现在先设计出来再倒逼 API**。

正确顺序：
```text
Master API Semantics
        ↓
Domain Contract
        ↓
OpenAPI / JSON Schema / Event Schema
        ↓
Contract Tests
        ↓
Real Consumer Evidence
        ↓
SDK
```

因此当前阶段：

### 现在冻结

- API namespace；
- API semantic rules；
- versioning；
- command/query/event boundaries；
- error/idempotency/context；
- extension mechanism。

### 后续逐域冻结

- Request / Response schema；
- State enum；
- Command payload；
- Query filters；
- Event payload；
- Capability discovery schema；
- Error detail schema。

### 再之后

根据真实消费者提炼 SDK。

---

# 22. SDK 分层

未来建议：
```text
L0 — Contract
    JSON Schema / OpenAPI / AsyncAPI / fixtures

L1 — vbmf-sdk
    Stable Broadcast Runtime client/types/helpers

L2 — Optional Integration Adapter
    Mother Platform / specific consumer integration
```

`vbmf-sdk` 只表达稳定的 Broadcast Runtime Contract，例如：
```text
BroadcastSession
SignalSource
SignalGraph
Pipeline
Workload
Placement
Switch
Failover
ClockDomain
FrameSyncGroup
VirtualChannel
PlayoutSession
Output
Health
RuntimeCapability
```

不得把 VBMF Rust internal structs 自动生成成 SDK API。

不得把 vendor SDK、GStreamer object、FFmpeg command builder、内部 DB model 暴露给 SDK。

---

# 23. SDK 准入门槛

只有同时满足：
```text
Stable Contract
+ Stable Semantics
+ Canonical Owner = VBMF
+ Real Consumer
+ Version Compatibility
+ Contract Tests
+ Migration / Exit Path
```

才允许进入 `vbmf-sdk`。

因此：

> **API 先定档；Domain Contract 再细化；SDK 最后从稳定 Contract 中提炼。**

SDK 是 API/Contract 的消费层，不是 API 的设计源头。

---

# 24. 与母框架的关系

母框架负责：
```text
Identity
Context
Version
Lifecycle primitives
Event Envelope
Error semantics
Provenance
Authority
Cross-system ownership
Security / Governance
```

VBMF API 负责：
```text
BroadcastSession
MediaSource
Signal
Pipeline
Workload
Placement
Production
Playout
Protection
Synchronization
Transport
Output
HA
Runtime Health
```

两者通过 Integration Contract 对接。

禁止：
```text
VBMF API → Mother DB
VBMF SDK → Mother internal ORM
Mother Platform → Vendor SDK
```

普通新增 endpoint、字段、Command、Query、Event、Adapter、Protocol 不得重新定义本文件；按照兼容策略进行 Domain Contract evolution。

---

# 25. 完成定义

本 API Master Design 定档意味着：

1. `/api/v1` 根版本冻结；
2. 顶层 namespace 冻结；
3. Query / Command / Event / Preview 边界冻结；
4. Control Plane / Media Plane 边界冻结；
5. Command / Idempotency / Error 三平面冻结；
6. Context / Identity 基础语义冻结；
7. Error taxonomy 冻结；
8. Extension Model 冻结；
9. Compatibility policy 冻结；
10. Standalone / Integrated API semantics 冻结；
11. SDK 采用 Contract-first、Consumer-evidence-first；
12. 后续进入 Domain-by-Domain Contract Design。

---

# 26. 变更控制

只有以下情况才允许重新打开 API Master Design：
```text
API root/version model changes
Control/Media boundary changes
Command/Query/Event semantic changes
Canonical ownership changes
Cross-system Contract foundation changes
Security/authority foundation changes
Extension mechanism fundamentally insufficient
```

---

# 27. 最终声明

> **VBMF V0.3 API 现在定档。**
>
> 定档的是 API 的骨架、语义边界、版本机制和扩展能力；不是把所有未来字段一次性猜完。
>
> 后续新增 API 必须首先进入既有 Domain/Resource/Capability/Command/Query/Event 体系，只有现有语义确实无法表达时才允许增加新的 Domain。
>
> **API 先于 SDK；Contract 先于 SDK；真实 Consumer 证据先于 Shared SDK。**
>
> 下一阶段不再讨论“API 要不要存在”，而是按 D1→D15 对每个 Domain 做字段级 Contract、状态机、Command、Query、Event、Error 与 Test Gate 设计。
