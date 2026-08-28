# IMPLEMENTATION_BOUNDARIES — 实现层边界契约

> Phase 0.6 门禁依据之一。综合论述见 [`IMPLEMENTATION_ADDENDUM.md`](./IMPLEMENTATION_ADDENDUM.md)。本文件定义**实现层边界**，是架构改动的红线。

## 1. 定位边界
- 当前 `services/media-agent` 正式定义为 **Canonical Media Runtime**。
- `BMD Provider` + `GStreamer Backend` 仅是**当前 Reference Implementation**，不是 VBMF 系统本身。
- 所有 vendor/backend 都是"实现资源"，而非业务身份。

## 2. 与 V0.2 的关系
- `ARCHITECTURE_V0.2.md` **LOCK FINAL**，任何语义改动须 V0.3 流程拍板。
- 本契约文件**不修改 V0.2 任何语义**，仅定义实现层（Implementation Architecture）边界。
- 禁止"边写边发现抽象"；实体关系以 Addendum 为准。

## 3. 四层架构边界
```
1. Canonical Domain        (Device/Port/Capability/Media/Signal/Resource/Clock/Timecode/Event)
2. Runtime Contracts/SPI   (HardwareProvider/MediaBackend/Encoder/Gateway/Acceleration/Clock/Audio)
3. Runtime Orchestration  (Session/Pipeline/Lease/Supervisor/Health/Binding/Scheduler/Preflight)
4. Concrete Adapters      (BMD/GStreamer/FFmpeg/SRS/PostgreSQL/Valkey/RustFS/...)
```
- 第 2 层 `Binding Resolver` 负责 Physical ↔ Provider ↔ Runtime Resource 解耦。
- 上层（1/3）不得依赖第 4 层具体类型。

## 4. 三态分离边界
- `Configuration`（用户希望）= Manifest / Intent
- `Runtime State`（现在是什么）= Lease / Session / Allocation
- `Observed State`（刚观测到）= Signal / Format / Content
- 三者不得混用；任何模块不得"顺便改状态"而无明确 owner。

## 5. Ownership 边界（最终明确）
```
Control Plane     owns Intent
Session           owns Runtime Lifecycle
Lease             owns Exclusive Runtime Claim
Resource Registry owns Resource State
Hardware Provider owns Vendor Resource Translation
Media Backend     owns Backend Runtime
Supervisor        owns Recovery Decision
Health            owns Health State
Scheduler         owns Placement Decision   (P1/P2)
```

## 6. 门禁判据（换品牌/后端时只发生的事）
| 变更 | 只发生 | 不发生 |
|---|---|---|
| BMD → AJA | Remove BMD / Add AJA / Rediscover / Rebind | 修改 Session / Graph / Health / Supervisor / UI |
| GStreamer → FFmpeg | Backend replaced / RuntimeBinding changed | 改变 CanonicalPipelinePlan |
| Embedded SDI → MADI | 修改 Audio Provider / Backend | 重新定义 Video Graph / MASTER_SWITCH / Session |
