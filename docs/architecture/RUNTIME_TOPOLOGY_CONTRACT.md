# RUNTIME_TOPOLOGY_CONTRACT — 媒体拓扑契约

> 状态：🔧 待建 → ✅ 已建（Phase 0.6, P0.5）
> 来源：用户 Final Hardening 审查（2026-08-28）第 ⑥ 优先项（#二十四/#二十五：PhysicalConnection / LogicalRoute / Topology 缺失）。
> 关联：`CANONICAL_MEDIA_MODEL.md`、`RUNTIME_RESOURCE_MODEL.md`、`IMPLEMENTATION_ADDENDUM.md`、`CANONICAL_IDENTITY.md`

## 1. 定位

Topology 描述**设备/资源之间如何连接**。它是独立于 `Resource`（可消耗实体）与 `Binding`（Physical→Provider→Runtime 关联）的第三类关系模型。

- **不实现** Router Scheduler / 自动路由计算（留 P1/P2）。
- 仅冻结模型，使 External Routing / HW Loopback / Multi-device Graph 不再重新造拓扑模型。

## 2. 核心实体

```
TopologyNode       — 拓扑节点（Device / Port / Router / Matrix / Capture / Output / Normalizer）
TopologyEdge       — 节点间有向连接（含 Physical / Logical 两种）
PhysicalConnection — 物理接线（线真的接在哪里）
LogicalRoute       — 系统当前要求怎么走（业务意图）
```

## 3. PhysicalConnection vs LogicalRoute（必须区分）

| 概念 | 含义 | 示例 |
|---|---|---|
| `PhysicalConnection` | 线真的接在哪里 | SDI cable：Output-Port-1 → Input-Port-2 |
| `LogicalRoute` | 系统现在要求怎么走 | Program → Router Input 8 |

- 二者**不得**混成一个 Route。
- `PhysicalConnection` 是拓扑事实（观测/配置）；`LogicalRoute` 是运行意图（由 Control Plane / Routing 决策产出）。
- 设备更换 → `PhysicalConnection` 变化；业务切播 → `LogicalRoute` 变化；两类变更独立建模。

## 4. 拓扑模型（只定义，不实现 Scheduler）

```
Topology
├── nodes:    Vec<TopologyNode>
├── physical: Vec<PhysicalConnection>   // 线接事实
└── routes:   Vec<LogicalRoute>          // 业务意图

TopologyNode
├── id:     NodeId
├── kind:   Device | Port | Router | Matrix | Capture | Output | Normalizer
└── refs:   Vec<CanonicalId>            // 指向 DeviceId / PortId / ResourceId

PhysicalConnection
├── from:   NodeId
├── to:     NodeId
└── medium: SDI | HDMI | OpticalSDI | Component | Composite | SVideo | IP | AES | MADI | Dante

LogicalRoute
├── from:      NodeId
├── to:        NodeId
├── intent:    Program | Preview | Clean | Backup
└── constraint: 互斥 / 优先级 / 回滚
```

示例：

```
Camera ──SDI──▶ SDI Router ──SDI──▶ Capture Card (Duo ChA)
Capture ──▶ Normalize ──▶ Router ──▶ Output Card (Duo ChB)
```

## 5. 与 Resource / Binding 关系

- `Resource` = 可消耗实体（Capacity / Availability / Allocation）。
- `Binding` = Physical→Provider→Runtime 关联。
- `Topology` = 连接关系（谁接到谁）。
- 三者正交：申请一个 Port Resource 占用 Capacity；Topology 描述该 Port 的物理来向；Binding 描述它如何映射到 Runtime。

## 6. Canonical 约束

- 拓扑表达**不得**泄露 vendor：用 `SDI`/`HDMI`/`OpticalSDI`/`IP`，**不**用 `/bmd`/`/gstreamer` 表示节点类型。
- 替换轴（Hardware Vendor / Backend）变化不改变 Topology 模型，仅改变 Node 的 `refs`（指向新的 `DeviceId`）。

## 7. 验收

- `ARCH-TOPOLOGY-01`（规划）：Topology 模型独立于 Resource/Binding，PhysicalConnection 与 LogicalRoute 可分可合，不混为一物。
- External Routing（`EXT-ROUTING-01`）消费 `LogicalRoute`，不重建拓扑模型。
