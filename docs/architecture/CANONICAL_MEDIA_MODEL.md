# CANONICAL_MEDIA_MODEL — Canonical 媒体模型冻结契约

> Phase 0.6 门禁依据（P0 Canonical Media Format 冻结契约）。综合论述见 [`IMPLEMENTATION_ADDENDUM.md §3,§7`](./IMPLEMENTATION_ADDENDUM.md)。

## 1. 冻结契约原则（澄清 #20 Format「过度设计」误解）
- **冻结契约** = 定义 Canonical 类型、只填当前用到字段（如 VideoFormat 仅 1080i50/1080p50 等），不实现全部色彩学/编解码。
- **完整实现** = 全部色彩学/编解码能力落地。
- 二者不同：**冻结契约是必要的**，不应被误判为过度设计。当前阶段只冻结，不实现全部。

## 2. 核心实体与关系（冻结，不得再"边写边发现抽象"）
```
Device ── Port ── Capability
Media  ── Signal / MediaFormat (Observed)
Resource (Logical consumable, P0.5)
Session (Runtime instance owning Port/Pipeline/Lease/Clock/Health)
Binding (Physical → Provider → Runtime Resource)
Clock / Timecode (P1 Contract)
Error / Event (P0 统一模型)
```

> **身份层级（见 [`CANONICAL_IDENTITY.md`](./CANONICAL_IDENTITY.md)）**：`Device` 含 Canonical `DeviceId`（Domain 只见此）+ Provider Identity（`persistent_id`/`DeviceHandle`/`SDK GUID` 等，vendor-specific）。`DeviceHandle`/`PersistentId` 是 **Provider Identity**，**不是** VBMF Canonical Identity；须经 Provider Identity Adapter 映射为 `DeviceId`。更换硬件厂商（BMD→AJA）只改 Provider 层身份机制，Canonical Domain 零变化。

## 3. Session Ownership（写死边界）
```
Session ≠ Device / Port / Pipeline / Lease
Session ├── references Port
         ├── owns Pipeline lifecycle
         ├── acquires Lease
         ├── references Hardware/Backend Resource
         ├── belongs to Clock Domain
         └── aggregates Health
```
生命周期：`Requested → Provisioning → Binding → Leased → Starting → Running → Stopping → Released`（P0 只定义，不实现 Scheduler）。

## 4. Audio Backend / Routing（第 9 替换轴，显式化）
- Audio 不当 VideoPort 附属字段；独立 `AudioSource / AudioPort / AudioRoute / AudioChannel / AudioFormat / AudioClock`。
- 语义：`Embedded / De-embedded / Independent / Mixed / External`（SDI Embedded / AES / MADI / Dante / IP）。
- 当前 audio 隐含内嵌 SDI，须显式建模，避免 Normalize 阶段返工。

## 5. 门禁判据
- Canonical Media Format 类型被 Domain / Graph / Session / Supervisor / Health 共享，不含 vendor 字段。
- 换 Embedded SDI→MADI：仅修改 Audio Provider/Backend，**不重新定义** Video Graph / MASTER_SWITCH / Session。
