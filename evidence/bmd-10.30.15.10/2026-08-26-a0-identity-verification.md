# A0 实机身份核实 — 2026-08-26 (10.30.15.10 / SDK 16.0)

## 背景
`0589299` 用 `IDeckLinkProfileAttributes::GetInt(BMDDeckLinkPersistentID)` 读持久身份。
用户担忧：若实际走的是 `IDeckLinkAPIInformation`（仅处理 `BMDDeckLinkAPIInformationID`），则实现本身错误；
并建议改用 `IDeckLinkAttributes::GetInt`。A0 目标：一次分清三套接口，拿到真实 `HRESULT / raw i64 / normalized u32`。

代码改为对每台设备 `QueryInterface` 三套接口，记录各自 `GetInt(PersistentID)` 结果；属性读自
**实际可用的接口**（优先 `IDeckLinkAttributes`，回退 `IDeckLinkProfileAttributes`，二者 vtable 布局一致，
区分仅在 IID）。binary `b6c4be8`，`cargo build --features hardware-test` + `VBMF_REGISTRY_ONLY=1` 运行。

## 三接口 QI 结果（三台设备一致）
| 接口 | IID | QI hr |
|------|-----|-------|
| `IDeckLinkAttributes` | ADB82CE7-861B-4B61-81DA-A20B084A702E | `0x80000004` E_NOINTERFACE |
| `IDeckLinkProfileAttributes` | F47551D7-AD22-47AF-BCFD-6BE88AA879D9 | `0x00000000` S_OK |
| `IDeckLinkAPIInformation` | B981ED01-51AB-48DC-9847-F6E42789C3DB | `0x80000004` E_NOINTERFACE |

→ 本设备/SDK **仅暴露 `IDeckLinkProfileAttributes`**。旧实现用该接口属正确路径；
`IDeckLinkAPIInformation` 返回 E_NOINTERFACE，证实**未误用**（用户担忧排除）。

## 设备身份真相表
| SDK# | 型号 (GetDisplayName) | 可用属性接口 | DeviceHandle (`'devh'`) | PersistentID (`'pers'`) | TopologicalID (`'topl'`) |
|------|----------------------|--------------|------------------------|------------------------|--------------------------|
| 0 | DeckLink SDI (1) | ProfileAttributes | `46:00000000:002e4500` | `0x80000003` 不支持 | `0x80000003` 不支持 |
| 1 | DeckLink SDI (2) | ProfileAttributes | `46:00000000:002e4400` | `0x80000003` 不支持 | `0x80000003` 不支持 |
| 2 | DeckLink Mini Monitor 4K | ProfileAttributes | `83:1a66443b:00000000` | `0x80000003` 不支持 | `0x80000003` 不支持 |

- `DisplayName` 经 `GetString('name')` 为空，但 `IDeckLink::GetDisplayName()` 方法正常（表中"型号"列）。
- `0x80000003` = BMD 属性不支持（与官方 FAQ "PersistentID 非所有 DeckLink 都支持" 一致）。

## 判定（HW-IDENT-01）
- ❌ 不能判 "PersistentID 不存在"；✅ 判 "**当前实现已证明：三台设备均不支持 PersistentID（GetInt=0x80000003）**"。
- 接口实现本身**正确**（用 `IDeckLinkProfileAttributes`，SDK 唯一暴露者）；此前的实现怀疑不成立。
- `DeviceHandle` 三台**唯一且可用** → canonical 硬件身份 = DeviceHandle。
- PersistentID / TopologicalID 均不可用 → 身份优先级在此硬件上收敛为 **DeviceHandle canonical**。

## 关键衍生事实（P0 架构风险）
**SDK 枚举序号 ≠ GStreamer `device-number`**：
- SDK 枚举：#0=SDI(1), #1=SDI(2), #2=MiniMonitor4K
- 用户此前观测 GStreamer：#0=MiniMonitor4K, #1=SDI, #2=SDI

→ 任何 "SDK index = GStreamer device-number" 的映射设计都必须删除。正确路径（C 设计）：
`DeviceHandle → Device Registry → GStreamer Resolver(hw-serial-number 匹配) → device-number`。

> 证据连贯性更正：此前 "device 1 → No input / device 2 → Failed PAUSED" 仅说明 GStreamer device-number
> 与正确输入设备未对应，**不能**证明 SDI 无信号。本次 SDK `start_capture` 已产生真实帧（frame 2569），
> 物理 SDI 信号很可能正常，当前第一嫌疑是**设备映射**而非信号。

## 后续决策
用户既定条件触发：**A0 确认 PersistentID 不支持 → 正式选择 C**。
- C：DeviceHandle canonical 身份 + GStreamer `device-number` materialization（经 `hw-serial-number` 解析）。
- 仍**绝不**降级为 "默认 device-number=0"（MiniMonitor 是输出卡，device 0 即错位）。
- 下一步：GStreamer `decklinkvideosrc` 属性核实（`persistent-id` / `hw-serial-number` / `device-number`），
  实现 Resolver，再重跑 canonical GStreamer ingest → MEDIA-RT-01。

## GStreamer 侧核实 (同机, 紧随 A0)
- `gst-launch-1.0 --version` = **1.28.2**。`gst-inspect-1.0 decklinkvideosrc` 确认三属性均在:
  - `device-number` : Output device instance to use
  - `hw-serial-number` : The serial number (hardware ID) of the Decklink card
  - `persistent-id` : Output device instance to use. Higher priority than "device-number".
- 但 **GStreamer 自身调试日志 (`GST_DEBUG=decklink*:5`) 明确打印 `device 0 does not have persistent id. Value set to 0`** →
  独立互证 PersistentID 在此硬件不存在, 故 GStreamer `persistent-id` 属性**用不了**, Resolver 必须走 `hw-serial-number`。
- **顺序不可假设**: 设 `device-number=0/1/2` 时调试日志均打印 "device 0 ... Input 0 supports 525i59.94 NTSC (720x486)" ——
  GStreamer 内部枚举顺序需**运行时探测**, 既不等于 SDK 序号, 也与"GStreamer#0=MiniMonitor"的旧印象可能不符
  (device 0 此处显示 SDI 输入模式)。故 Resolver 不得硬编码任何顺序。
- `hw-serial-number` 实际字符串未在上述日志出现, 须由 agent 运行时 (建 decklinkvideosrc 设 device-number→读 hw-serial-number)
  动态探得, 再与 BMD DeviceHandle 匹配。Resolver 须输出全部 (device-number, hw-serial-number) 对供核对。
