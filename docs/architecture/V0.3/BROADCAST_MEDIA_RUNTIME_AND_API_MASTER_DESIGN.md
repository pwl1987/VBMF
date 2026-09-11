# VBMF V0.3 广播媒体运行时与总体架构详细设计

> **状态：详细设计基线**  
> **版本：V0.3**  
> **定位：专业实时媒体播出系统的领域、运行时、部署、同步、制作、播出、保护、传输、观察及扩展详细设计**

## 0. 顶层设计裁决

VBMF V0.3 正式定位为：

> **专业实时媒体播出系统 / 实时媒体运行时与媒体织构**

第一性任务不是单独提供编码、转码、网关、切换或采集，而是可靠完成专业实时媒体从输入、同步、处理、制作、编排、播出、保护、传输到输出，并持续提供观察、审计和恢复能力。

编码器、解码器、转码器、媒体网关、制作切换、调度器、接收器、录制器、采集卡均属于运行时能力、资源、工作负载或适配器，而不是平行产品。

## 1. 与母框架关系

母框架负责跨系统身份、上下文、契约、生命周期基础语义、事件信封、版本、来源追踪、权威、所有权、安全治理、兼容性和集成边界。

VBMF 负责广播运行时专业语义，包括会话、源、输入、信号、媒体流、管线、媒体图、工作负载、放置、适配器、网关、编解码、处理、同步、制作、播出、虚拟频道、播出保护、传输、输出、录制、回放、高可用、运行时健康和运行时控制。

VBMF 不得夺取用户、组织、新闻内容、业务资产、AI/知识、全局任务、全局搜索通知等事实所有权。

## 2. 完整实时播出生命周期

```text
源 / 资产引用
 ↓
输入 / 接入
 ↓
时间戳 / 时钟 / 同步
 ↓
抖动缓冲 / 帧对齐
 ↓
媒体图
 ↓
处理 / 合成 / 混音
 ↓
制作切换
 ↓
播出 / 编排
 ↓
播出保护
 ↓
编码 / 复用 / 打包
 ↓
传输 / 网关
 ↓
输出 / 交付
 ↓
监控 / 观察 / 审计
 ↓
连续性 / 恢复
```

这是逻辑生命周期，不要求所有能力位于同一机器或同一进程。

## 3. 一级领域

V0.3 冻结 15 个专业领域：

1. 媒体输入输出
2. 媒体处理
3. 媒体网关
4. 媒体图
5. 同步
6. 制作
7. 播出
8. 播出保护
9. 传输与交付
10. 资源、工作负载与放置
11. 高可用与连续性
12. 录制、回放与时移
13. 可观测性、诊断与审计
14. 运行时控制、查询与事件
15. 资产运行时集成边界

这些是领域分组，不要求拆成十五个服务。

## 4. 核心对象模型

### 基础设施与运行时

```text
Runtime / Node / Device / Adapter / Resource / Port / Capability / Workload / Placement / Allocation
```

### 媒体

```text
Source / Input / Signal / Media / Stream / VideoStream / AudioStream / DataStream
ElementaryStream / Packet / PES
```

### 图与执行

```text
Pipeline / MediaGraph / MediaLink / Processor / Filter / Route / ExecutionPlan
```

### 广播

```text
Program / ProgramFeed / ProgramOutput / Switch / ProtectionGroup / FailoverPolicy
BackupMedia / Output / Transport / Delivery
```

### 播出

```text
VirtualChannel / Playlist / Schedule / ProgramItem / PlayoutEvent / PlayoutSession
Queue / NowPlaying / Override
```

### 同步

```text
Clock / ClockDomain / Timebase / Timecode / PTPDomain / Genlock
FrameSyncGroup / MediaTimeline / JitterBuffer
```

### 制作

```text
ProductionSession / MESwitcher / Bus / Keyer / DSK / AUX / Scene / Macro
Tally / Multiview / ControlSurface
```

### 可靠性

```text
FaultDomain / ProtectionGroup / Failover / Recovery / Lease / Epoch / Generation
Fence / Quorum / ReplicationState / ContinuityProfile
```

### 观察

```text
Health / Metric / Event / Alert / Incident / Diagnostic / AuditRecord / Evidence
```

## 5. 节点、资源、能力、工作负载与放置

Node 是运行能力和资源的逻辑承载者，不等于物理服务器。它可以映射到物理机、虚拟机、容器运行时、边缘节点或专用设备运行时。

资源至少覆盖计算、CPU、内存、GPU、GPU 显存、设备、采集/输出卡、端口、带宽、编码槽、解码槽、传输容量和存储容量。

能力描述“节点能做什么”，例如视频采集、H.264/H.265 编解码、转码、TS/PS 复用解复用、SRT/RIST/RTP 传输、PTP/Genlock、制作切换和虚拟频道播出。

能力必须具备标识、版本、提供者、实现、约束、输入类型、输出类型、资源要求和可用性。

部署关系：

```text
Pipeline
 ↓
Workload
 ↓
Resource Requirement
 ↓
Placement
 ↓
Node / Resource
```

支持手工放置、约束放置和自动放置。

## 6. 媒体输入输出与适配器

顶层类别使用“媒体输入输出适配器”，而不是把某个厂商采集卡作为架构类别：

```text
媒体输入输出适配器
├── 硬件适配器
│   ├── BMD / DeckLink
│   ├── AJA
│   ├── Magewell
│   └── 其他厂商
├── 网络适配器
├── 移动贡献适配器
└── 虚拟适配器
```

设备关系：

```text
Node → Device → Adapter → Card / Interface → Port → Input / Output → Media Stream
```

单机多卡支持主用、备用、空闲卡池。卡级故障应优先进行最小范围资源迁移，而不是无条件重建整个管线。

## 7. 媒体图、管线、工作负载

Pipeline 是逻辑媒体处理链，不绑定节点、进程或具体适配器。

Workload 表示可运行的媒体工作阶段，例如采集、解码、编码、转码、处理、复用、解复用、传输、录制和回放工作负载。

跨节点媒体必须经过明确的媒体链路或传输：

```text
节点 A → 端口 → 媒体流 → 媒体传输 → 媒体流 → 端口 → 节点 B
```

禁止控制接口承载实时媒体主链路。

## 8. 分布式运行时

顶层支持：

```text
一体机
功能拆分
多节点
集群
混合部署
```

例如采集、解码、处理、编码、网关和播出编排可以位于不同节点。重新放置工作负载不得迫使逻辑管线变成另一套模型。

不可变规则：

1. Pipeline 不绑定物理 Node；
2. Workload 不等于操作系统进程；
3. Node 不固定承担单一功能；
4. Scheduler 不执行媒体处理；
5. Control Plane 不承载实时媒体流；
6. 跨节点媒体必须通过明确 Transport；
7. 单机与分布式使用同一领域契约；
8. 高可用和调度不得产生第二套 Pipeline Model；
9. Capability、Resource、Placement 驱动部署变化；
10. 所有部署模式必须服从信号连续性要求。

## 9. 同步架构

同步是运行时基础设施，不是普通视频滤镜。

### 时钟同步

```text
PTP / NTP / GPS / GNSS / Genlock
```

### 时间戳同步

```text
PTS / DTS / Capture Timestamp / Source Timestamp / Timecode / Clock Domain / Sequence
```

### 帧同步

```text
输入
 ↓
时间戳
 ↓
抖动缓冲
 ↓
帧对齐
 ↓
帧同步
 ↓
统一媒体时间线
```

必须支持：

```text
SYNCED
DEGRADED
WAITING
LATE
DROPPED
UNSYNCED
RECOVERING
```

### 4G/5G 移动贡献

支持多背包到一个接收器、一个背包到多个接收器，以及接收器串联和并联拓扑。接收后的时间戳对齐、抖动缓冲和帧同步必须进入统一媒体时间线，再交给制作或播出。

## 10. 制作切换

制作切换是可选但完整的专业运行时能力：

```text
制作体验
 ↓
制作控制接口
 ↓
制作切换核心
 ↓
媒体图
 ↓
节目输出
```

支持 M/E、PGM、PVW、Keyer、DSK、AUX、AFV、Tally、Multiview、Scene、Macro、Transition、Cut、Take。

控制面可以来自 Web、桌面端、物理切换台、MIDI/GPIO、网络控制面或远程操作台。外部切换台既可以作为控制面，也可以作为外部设备接入。

## 11. 播出与虚拟频道

实时媒体播出必须把播出作为正式核心领域。

核心对象：

```text
VirtualChannel / Playlist / Schedule / ProgramItem / PlayoutEvent
PlayoutSession / Queue / NowPlaying / Override / Rule
```

统一媒体源：

```text
直播源
文件源
备用源
远程贡献
外部节目源
生成媒体
```

支持：

```text
节目 → 直播 → 插播 → 备用 → 恢复
```

## 12. 播出保护与连续性

保护策略至少支持：

```text
PASS
BLACK
MUTE
REPLACE
BACKUP
DELAY
HOLD
FREEZE
ALTERNATE_SOURCE
```

保护层级覆盖接收器、源、卡、端口、管线、节点、输出和集群。

连续性配置至少表达：

```text
CONTINUITY_REQUIRED
CONTINUITY_PREFERRED
BEST_EFFORT
```

以及：

```text
SEAMLESS
NEAR_SEAMLESS
FAST_FAILOVER
COLD_FAILOVER
```

首选切换顺序：

```text
准备新路径
 → 启动新路径
 → 验证就绪
 → 隔离旧路径
 → 原子切换
 → 验证
 → 停止旧路径
```

连续性属于运行时执行语义，不得由 UI 自己实现。

## 13. 编解码、容器与传输

Codec、Decoder、Encoder、Transcoder 与 Container、Muxer、Demuxer 分离建模。

必须显式支持：

```text
Elementary Stream
PES
Packet
TS
PS
MP4
MOV
```

PS/MPEG Program Stream 是正式支持的媒体语义，不是临时特例。

网关和传输预留：

```text
SDI / HDMI / IP / RTP / RTSP / SRT / RIST / RTMP / NDI
ST 2110 / ASI / WebRTC / 4G / 5G
```

## 14. 音频与视频同等地位

音频必须使用与视频一致的运行时时间线，并作为一等媒体能力：

```text
AudioStream
AudioClock
AudioRoute
AudioMixer
AudioDelay
Loudness
AudioFollowVideo
Mute
ChannelMap
```

音视频同步、延迟、路由和监测不能通过“视频附带音频”的弱语义处理。

## 15. 录制、回放、时移

正式对象包括：

```text
RecordingSession
ReplaySession
TimeshiftBuffer
Clip
RecordingTarget
StorageTarget
```

录制和回放属于 Runtime 能力，不创建独立于 Runtime 的第二套媒体事实模型。

## 16. 高可用

支持：

```text
单机
主备
双活
N+1 集群
```

对象至少包括：

```text
Node / Cluster / HA Group / Role / Assignment / Lease / Epoch
Generation / Leader / Quorum / Fence / Replication / Failover / Recovery
```

高可用必须复用同一运行时、领域和媒体语义。

## 17. 观察、诊断、审计

运行时事实必须能够形成：

```text
Health
Metric
Event
Alert
Incident
Diagnostic
AuditRecord
Evidence
```

观察与配置必须分离；观察结果不得隐式修改运行时配置或媒体图。

## 18. 资产运行时边界

上游 CMS/DAM 是业务资产权威。VBMF 可以引用、缓存、预加载、验证就绪、形成运行时视图、消费、执行并审计运行时使用，但不能建立第二套业务资产事实。

## 19. API 详细设计归属

本文件负责领域和运行时语义；具体接口总体规则由 `API_MASTER_DESIGN.md` 负责。接口实现必须遵守：

```text
控制 / 查询 / 事件 ≠ 实时媒体传输
```

命令进入控制面，查询来自运行时事实，事件来自运行时事实投影，实时媒体走独立媒体传输。

## 20. 部署与演进原则

单机、分布式、硬件变化、厂商变化、资源变化、高可用变化均应通过能力、资源、工作负载、放置、适配器和明确的传输边界表达，而不是复制第二套业务逻辑。

任何新能力进入 V0.3 都必须回答：事实所有者、领域归属、运行时复用点、契约影响、兼容性、故障模式、连续性影响和真实消费者证据。
