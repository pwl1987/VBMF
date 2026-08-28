# V0_2_TO_PHASE_0_6_CROSSWALK — V0.2 与 Phase 0.6 语义对照

> 回答「Phase 0.6 每一条新契约到底映射 V0.2 哪个语义」。V0.2 `LOCK FINAL` 不可改；本文件只做**映射与对齐说明**，不修改 V0.2 任何语义。
> 关联：[`ARCHITECTURE_V0.2.md`](./ARCHITECTURE_V0.2.md)、[`IMPLEMENTATION_ADDENDUM.md`](./IMPLEMENTATION_ADDENDUM.md)。

## 1. 映射表

| Phase 0.6 契约 / 概念 | V0.2 对应语义 | 关系 |
|---|---|---|
| Canonical Domain：Device / Port / Capability / Signal / MediaFormat | V0.2 Source / Signal Fabric / Normalize / 12 Engines 的底层实体 | 0.6 抽象出 Canonical 类型，V0.2 是业务语义 SoT |
| `Resource` / `Resource Vector` / 四概念 | V0.2 §3.11 九维 Resource Vector（`cpu`/`gpu`/`pcie`/`mem`/`net`/`bw`/`storage`/`license`/`port`） | **严格对齐**，0.6 不另起一套；Resource≠Device 与 V0.2 一致 |
| `Session`（Runtime instance） | V0.2 §8.11 三轴状态机 / Media Session | 0.6 把 Session 提升为一等公民（V0.2 已定义语义，Rust 未落地） |
| `Binding`（Physical→Provider→Runtime） | V0.2 无直接等价（新增 SPI 层） | 0.6 新增；不冲突 V0.2 |
| `Topology`（PhysicalConnection/LogicalRoute） | V0.2 无直接等价（跨 Engine 连接关系隐含在各 Engine） | 0.6 新增横切模型；不冲突 V0.2 |
| `Clock` / `Timecode` | V0.2 Clock / Timecode 语义 | 0.6 仅冻结 Contract，不重定义 V0.2 语义 |
| `Audio` Backend/Routing | V0.2 Engine 9 Audio | 0.6 补「Audio 独立 Backend」缺口（V0.2 已含 Audio，但 0.6 前隐含内嵌 SDI） |
| `Capability` | V0.2 X6 Capability Registry | 0.6 Provider Capability 经 Mapping 收敛为 Canonical Capability → X6（见 §3） |
| `Provider` / `Backend` SPI | V0.2 无（V0.2 是 Runtime 语义，不含 vendor SPI 分层） | 0.6 新增实现层；V0.2 不禁止 |
| `RuntimeEvent` / `RuntimeError` | V0.2 Health Tree / Incident（观测结果） | 0.6 补统一事件源，投影到 V0.2 Health Tree |
| `Canonical Identity`（`DeviceId` 等） | V0.2 §5 schema 身份字段 | 0.6 新增 vendor-neutral 映射层；V0.2 历史字段见 §2 ADR |

## 2. ADR-001：V0.2 `DeviceToken: BMD_INPUT_PORT | BMD_OUTPUT_PORT` 的处理

> **状态**：🔴 已裁决（2026-08-28 第四轮审查发现）

- V0.2 `ARCHITECTURE_V0.2.md` §Canonical Vocabulary 含 `DeviceToken: BMD_INPUT_PORT | BMD_OUTPUT_PORT`（见 `README.md` 规范词汇卡）。
- 这是 **V0.2 LOCK FINAL 中真实冻结的旧术语**，属 V0.2 历史 / Implementation-specific 表述。
- **裁决**：**不修改 V0.2**（LOCK FINAL 不可私自重开）。但 Phase 0.6 Canonical Provider Model **不将 `DeviceToken` 扩展为新的 Vendor Identity**；Phase 0.6 用 `PortId`（`uuid5` 生成， vendor-neutral）取代之。任何新代码/契约不得新增 `BMD_*` 作为 Canonical 身份。
- 若未来 Phase 0.7 External API 需暴露端口类型，使用 `ConnectorType`（SDI/HDMI/OpticalSDI/...）而非 `DeviceToken`。

## 3. Provider Capability ↔ V0.2 X6 Capability Registry 映射

```
Vendor Capability (Provider-specific)
        ↓  Provider Capability Mapping
Canonical Capability
        ↓
V0.2 X6 Capability Registry
```

- 禁止两套能力真相：Provider 只报 vendor 能力，经 Mapping 收敛为 Canonical，最终入 X6 Registry（单一 SoT）。
- Mapping 规则（P1 冻结 Contract，不实现）：`ProviderCapability` → `CanonicalCapability` 由 Provider Identity Adapter 同层完成；X6 消费 Canonical，不做 vendor 解析。

## 4. 不冲突声明
所有 Phase 0.6 契约均为 V0.2 的**实现层补充 / 抽象层提升**，未修改 V0.2 任何锁定语义。任何与 V0.2 的表面冲突，以 V0.2 为准，并回到本文件更新映射。
