# E-41 · Network Path Inspector (Spec 锁)

> **Network Path Inspector · 网络路径检查器** · VBMF Operator Console
>
> - 域: **ENGINEERING** (Network 子域)
> - 表面编号: **E-41** (PIA V0.1 §10 锁 · 0.5F 实施 Spec, 0.5G 实施 wireframe)
> - 版本: **Spec V0.1** · 2026-08-25
> - 状态: **DRAFT 0.1** (待审, 通过后升 LOCK FINAL)
> - 上游: [`PRODUCT_INFORMATION_ARCHITECTURE.md` §10](PRODUCT_INFORMATION_ARCHITECTURE.md#10-network-path-model-新架构对象-v02-24-扩展)
> - 配套: [E-40 Network Source](operator/E-40-network-source.html) · [E-37 Clock](operator/E-37-clock.html) · [E-38 Hardware Inventory](operator/E-38-hardware-inventory.html)

---

## 1. 目的与背景 / Purpose

### 1.1 真实故障场景

广播网络故障的典型症状:

```
"UDP Source 断了"
```

但根因可能是:

```text
[1] 远端发射机没开机
[2] 远端发射机到机房间的光纤断
[3] 机房交换机端口 down
[4] 交换机 VLAN 配置错误
[5] VBMF 网卡没接 / down
[6] VBMF 网卡 IP 配置错
[7] IGMP 没 join (Multicast)
[8] IGMP join 成功但 ASM 拒绝 (SSM 验证)
[9] VBMF 路由表缺 (gateway 不通)
[10] VBMF 进程没监听 (防火墙)
[11] VBMF 进程在听但 Source 状态机卡住
[12] 流量到了 Worker, 但容器 / cgroup / 进程隔离丢包
[13] Output 那边反向的链路问题
[14] CDN / 远端 Output 接收端问题
```

**当前 UI 只能告诉你"断了", 不能告诉你"在哪断"。**

这就是 E-41 要解决的核心问题。

### 1.2 E-41 在 4 域中的位置

```text
ENGINEERING (深页为主)
├── ...
├── Network Source (E-40)        ← 配置: "Source 是什么, 怎么连"
├── Network Path Inspector (E-41) ← 诊断: "现在端到端真的通吗, 在哪里出问题"  ★ 本文
├── Network Interfaces           ← 状态: "网卡本身怎么样"
└── Multicast Diagnostics        ← 单边: "Multicast join 状态"
```

E-41 与 E-40 互为**配置 / 诊断**两端:

| 表面 | 角色 | 用户问的问题 |
|---|---|---|
| **E-40 Network Source** | 配置 | "我想加一个 UDP Multicast 源, 怎么填?" |
| **E-41 Network Path** | 诊断 | "这个源端到端能通吗? 在哪里不通?" |

### 1.3 与 PIA V0.1 §10 锁定项的对账

| 锁 | PIA 原文 | E-41 Spec 落实章节 |
|---|---|---|
| Network Path Inspector 作为 Engineering UI | §10.3 ✅ | §2 (对象模型) · §7 (显示布局) |
| 不引入新 Engine (Network Path 是 Network Endpoint 内部诊断) | §10.3 ✅ | §3 (依赖现有 12 Engine) · §10 (无新 Engine) |
| 输入: Source ID + Output ID | §10.2 | §4.1 (双端) · §4.2 (单端) |
| 自动探测: 中间路由节点 (traceroute / SNMP) | §10.2 | §6.1 / §6.2 |
| 显示: 完整路径 + 关键 hop 状态 + 延迟 + 丢包率 | §10.2 | §7 (5 段可视化) |
| 用途: 故障定位 | §10.2 | §1.1 + §8 (失败模式分类) |

---

## 2. Network Path 对象模型

### 2.1 Path = Source → Hops → Output 的有向图

```yaml
network_path:
  id: PATH-CH01-UDP-MULTICAST-001-TO-CDN-A
  direction: BIDIRECTIONAL       # 单向 / 双向

  source:
    type: SOURCE                 # NODE_KIND (见 §2.2)
    ref: UDP-MULTICAST-001       # 引用 Source Adapter (E-40)

  destination:
    type: OUTPUT
    ref: CDN-A                   # 引用 Output Destination (CD-01 Tab 6)

  hops:
    - index: 0
      kind: SOURCE
      node: "远端发射机 10.30.20.100"
      role: ORIGIN

    - index: 1
      kind: NETWORK_DEVICE
      node: "10G 交换机 (gateway 10.30.20.1)"
      role: TRANSIT
      discovery: SNMP            # 怎么发现的
      ifIndex: 4                 # 交换机端口

    - index: 2
      kind: NIC
      node: "VBMF eno1 (10.30.20.10, VLAN 120)"
      role: INGRESS
      ifIndex: 2

    - index: 3
      kind: WORKER
      node: "vbmf-source-worker@pid=8421"
      role: PROCESS

    - index: 4
      kind: WORKER
      node: "vbmf-master-join@pid=8430"
      role: PROCESS

    - index: 5
      kind: WORKER
      node: "vbmf-output-worker@pid=8442"
      role: PROCESS

    - index: 6
      kind: NIC
      node: "VBMF eth0 (10.30.30.10)"
      role: EGRESS

    - index: 7
      kind: NETWORK_DEVICE
      node: "10G 交换机 (gateway 10.30.30.1)"
      role: TRANSIT

    - index: 8
      kind: DESTINATION
      node: "CDN-A edge 1.2.3.4:1935"
      role: TERMINUS

  status: HEALTHY                # HEALTHY / DEGRADED / FAILED / UNKNOWN
  last_probed_at: 2026-08-25T08:50:14Z
  measured_latency_ms: 4.2
  measured_loss_pct: 0.003
```

### 2.2 NODE_KIND (5 类, 不可扩展)

| Kind | 含义 | 代表对象 | 数据来源 |
|---|---|---|---|
| **SOURCE** | 远端信号源 | 卫星接收机 / 远端编码器 / 演播室 | E-40 + 静态拓扑 |
| **NETWORK_DEVICE** | 中间路由 / 交换机 / 防火墙 | 10G 交换机 / VLAN gateway | SNMP / traceroute / LLDP |
| **NIC** | VBMF 自身网卡 | eno1 / eth0 / bond0 | E-38 Hardware Inventory |
| **WORKER** | VBMF 内部进程 | Source worker / Master Join / Output worker | E-36 Resource + 进程列表 |
| **DESTINATION** | 远端输出目标 | CDN / SRS / UDP Multicast group | CD-01 Tab 6 + E-40 |

⛔ **不引入第 6 种 Kind。** 如果未来出现新对象 (如 Load Balancer, WAN Optimizer), 归入 NETWORK_DEVICE 子分类。

### 2.3 Edge = Hop 之间的连接

每条边携带测量值:

```yaml
edge:
  from: HOP[1]              # 交换机
  to: HOP[2]                # eno1

  link_state: UP
  rtt_ms: 0.3
  loss_pct: 0.001
  jitter_ms: 0.05
  bandwidth_used_pct: 42    # 估算占用率

  measured_at: 2026-08-25T08:50:14Z
  probe_method: ICMP_RATE_LIMITED
```

---

## 3. 与 V0.2 12 Engine 的边界

E-41 **不引入新 Engine**, 完全在 Network Endpoint 内部实现:

| Engine (V0.2) | E-41 用到的能力 | 不做的 |
|---|---|---|
| **Source Adapter** | 读取 Source Endpoint 配置 | 不改 Source 协议 |
| **Signal Fabric** | - | - |
| **Normalize** | - | - |
| **Redundancy** | 读 Primary/Backup Path 选择 | 不做 failover (CD-01 负责) |
| **QC** | - | - |
| **Playout** | - | - |
| **Switcher** | - | - |
| **Composition** | - | - |
| **Audio** | - | - |
| **Output** | 读 Output Endpoint 配置 | 不改 Output 协议 |
| **Recording** | - | - |
| **Replay** | - | - |

**E-41 = 只读诊断工具**, 不修改任何 Engine 状态。

---

## 4. 输入与输出

### 4.1 双端查询 (默认)

```text
GET /api/v1/network-path?source_id={src_id}&output_id={dst_id}
```

返回完整 Path 对象 (§2.1)。

### 4.2 单端查询 (向后兼容)

只查 Source 一端:

```text
GET /api/v1/network-path/source/{src_id}?direction=egress
```

只查 Output 一端:

```text
GET /api/v1/network-path/output/{dst_id}?direction=ingress
```

适用: 用户只关心单向链路 (Source 接收 / Output 发送), 不需要完整 8-hop 视图。

### 4.3 UI 入口

| 入口 | 上下文 | 默认查询 |
|---|---|---|
| **CD-01 Channel Detail · Tab 3 (Source)** | 已选 Channel + Source | 双端 (Source → Channel → Output) |
| **CD-01 Channel Detail · Tab 6 (Output)** | 已选 Channel + Output | 双端 (Source → Channel → Output) |
| **E-40 Network Source · 顶部按钮 [PATH]** | 已选 Source | 单端 (Source → 第一个 Worker) |
| **CD-01 Channel Workspace · 顶部 INCIDENT 条幅** | Channel Health 异常 | 双端 (Channel 所有 Source → Channel 所有 Output) |
| **ENGINEERING · Network · Network Path Inspector (直接访问)** | 无 | 用户手填 Source / Output |

⛔ **不在 BROADCAST 工作台 (CH-01 / CD-01 Workspace) 直接展开 E-41 详情。** 一旦发现路径异常, 跳转到 E-41, 不在 Channel 工作台做深度诊断。

---

## 5. Hop 拓扑获取

### 5.1 三层数据源

```text
┌──────────────────────────────────────────────┐
│ Layer 1: VBMF 自身 (E-38 Hardware Inventory) │  ← 必填, 永远有
│   · 所有 NIC 列表 (eno1, eth0, bond0)
│   · 每个 NIC 的 IP / VLAN / Link state / Speed
│   · 进程列表 (Worker)
├──────────────────────────────────────────────┤
│ Layer 2: 静态配置 (来自 E-40 / CD-01)         │  ← 必填, 永远有
│   · Source Endpoint (IP / Port / Mode)
│   · Output Endpoint (IP / Port / Mode)
│   · Gateway / Route Table
├──────────────────────────────────────────────┤
│ Layer 3: 动态探测 (traceroute / SNMP / LLDP)  │  ← 尽力而为, 失败不阻塞
│   · traceroute 中间跳
│   · SNMP ifTable / ifXTable
│   · LLDP 邻居
│   · ICMP echo 测量 RTT / Loss
└──────────────────────────────────────────────┘
```

**Layer 3 失败时**: E-41 仍然可以显示 VBMF 自身 + Source/Output 端点, 但中间跳标记为 UNKNOWN。

### 5.2 探测技术映射

| 协议 | Layer 3 探测技术 | 是否需要凭据 |
|---|---|---|
| **UDP Unicast** | ICMP echo (用 source IP) + UDP 包探测 (3 次) | 不需要 |
| **UDP Multicast** | IGMP join 验证 + `tcpdump` 5 秒采样 + (可选) `mtrace` 反向追踪 | 不需要 (但要 root 或 CAP_NET_RAW) |
| **RTP/UDP** | RTP header 解析 + RTCP RR (Sender Report) | 不需要 |
| **SRT** | SRT handshake 探测 (加密前可以 ping) | passphrase 影响探测深度 |
| **RTMP** | RTMP `connect` + 1 packet 探测 | 不需要 |
| **HLS** | HTTPS HEAD 到 m3u8 URL | URL 可访问 |
| **RTSP** | RTSP DESCRIBE | 不需要 |
| **WebRTC** | STUN binding request | 不需要 |
| **RIST** | RTCP 控制包探测 | passphrase 影响探测深度 |

⛔ **不向 Output 端点发送业务流量。** 所有探测都是 read-only / control packet, 不影响正常播出。

### 5.3 探测频率

| 场景 | 频率 | 触发 |
|---|---|---|
| **主动探测** (健康时) | 每 30 秒 1 次 | 后台定时 |
| **被动探测** (Channel ON AIR 时) | 每 5 秒 1 次 | 当 Channel 状态为 ON_AIR |
| **异常触发** (任一 hop 状态变化) | 立即 | 状态机事件 |
| **手动探测** (用户按 [RE-PROBE]) | 立即 | UI 按钮 |

---

## 6. 显示布局 (Spec 文本 wireframe)

> **0.5F Spec 锁本节; wireframe 0.5G 实施。**

### 6.1 整体布局: 5 段垂直堆叠

```text
┌──────────────────────────────────────────────────────────────────────┐
│ [1] PATH HEADER (Source → Output 摘要, 状态, 测量值)                  │
├──────────────────────────────────────────────────────────────────────┤
│ [2] PATH DIAGRAM (水平 8-hop 节点图, 带状态色)                       │
├──────────────────────────────────────────────────────────────────────┤
│ [3] HOP TABLE (8 行表格, 每 hop 1 行, 详细字段)                      │
├──────────────────────────────────────────────────────────────────────┤
│ [4] EDGE METRICS (相邻 hop 之间的边指标, 6 段表)                      │
├──────────────────────────────────────────────────────────────────────┤
│ [5] DIAGNOSTICS (失败时显示根因提示 + 操作按钮)                      │
└──────────────────────────────────────────────────────────────────────┘
```

### 6.2 段 1: PATH HEADER

```text
PATH-CH01-UDP-MULTICAST-001-TO-CDN-A
═══════════════════════════════════════════════════════════
UDP-MULTICAST-001 (SSM 239.20.10.10:5000)  →  CDN-A edge (1.2.3.4:1935)
8 hops · HEALTHY · RTT 4.2ms · Loss 0.003% · Last probe 12s ago
[ RE-PROBE ]   [ EXPORT JSON ]   [ HISTORY (24h) ]
```

字段:

| 字段 | 含义 | 数据来源 |
|---|---|---|
| `path_id` | Path 唯一标识 | 拼接 |
| `source_label` | Source 名称 + Endpoint | E-40 |
| `destination_label` | Output 名称 + Endpoint | CD-01 Tab 6 |
| `hop_count` | 当前 Path 总 hop 数 | 计算 |
| `status` | HEALTHY / DEGRADED / FAILED / UNKNOWN | 状态机 |
| `rtt_ms` | 端到端 RTT (中位) | 测量 |
| `loss_pct` | 端到端 Loss | 测量 |
| `last_probed_at` | 上次探测时间 | 状态机 |

### 6.3 段 2: PATH DIAGRAM (核心)

水平方向, 每个 hop 一个圆角矩形节点, 节点之间用箭头连边, 边上有状态色:

```text
┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐
│ SOURCE │━━━▶│  SW 1  │━━━▶│ eno1   │━━━▶│ WORKER│━━━▶│ eno1   │
│        │    │        │    │        │    │ SRC    │    │ OUT    │
│ ●  UP  │    │ ●  UP  │    │ ●  UP  │    │ ●  UP  │    │ ●  UP  │
│ 0.3ms  │    │ 0.5ms  │    │ 0.1ms  │    │ 0.2ms  │    │ 0.1ms  │
└────────┘    └────────┘    └────────┘    └────────┘    └────────┘

(继续到 CDN-A)
```

**节点状态色** (与 DESIGN_SYSTEM.md 一致):

| 状态 | 色 | 含义 |
|---|---|---|
| HEALTHY | 绿 ● | 通, 延迟 / 丢包在阈值内 |
| DEGRADED | 黄 ● | 通, 但有告警 (RTT > 100ms 或 loss > 0.1%) |
| FAILED | 红 ● | 不通 (Link down / 探测超时) |
| UNKNOWN | 灰 ● | 探测失败, 状态未知 |

**鼠标悬停** (0.5G wireframe 细化):

- 节点: 显示节点详细字段 (Hostname / IP / VLAN / Model)
- 边: 显示边的指标 (RTT / Loss / Jitter / Bandwidth)

### 6.4 段 3: HOP TABLE

```text
┌────┬──────────┬────────────┬──────┬──────────┬───────┬────────┐
│ #  │ KIND     │ NODE       │ ROLE │ RTT      │ LOSS  │ STATUS │
├────┼──────────┼────────────┼──────┼──────────┼───────┼────────┤
│ 0  │ SOURCE   │ 10.30.20.100 │ ORIGIN  │ 0.3ms │ 0.000% │ ● UP  │
│ 1  │ SWITCH   │ sw-core-01   │ TRANSIT │ 0.8ms │ 0.001% │ ● UP  │
│ 2  │ NIC      │ eno1 (10.30.20.10 VLAN 120) │ INGRESS │ 0.1ms │ 0.000% │ ● UP  │
│ 3  │ WORKER   │ src-worker@8421 │ PROCESS │ 0.2ms │ 0.000% │ ● UP  │
│ 4  │ WORKER   │ master-join@8430 │ PROCESS │ 0.2ms │ 0.000% │ ● UP  │
│ 5  │ WORKER   │ out-worker@8442 │ PROCESS │ 0.2ms │ 0.000% │ ● UP  │
│ 6  │ NIC      │ eth0 (10.30.30.10) │ EGRESS  │ 0.1ms │ 0.000% │ ● UP  │
│ 7  │ SWITCH   │ sw-edge-02   │ TRANSIT │ 0.5ms │ 0.002% │ ● UP  │
│ 8  │ DEST     │ CDN-A (1.2.3.4:1935) │ TERMINUS │ - │ - │ ● UP  │
└────┴──────────┴────────────┴──────┴──────────┴───────┴────────┘
```

每行可点击 → 展开**侧栏详情** (0.5G 实施):

- SOURCE: 跳 E-40 Network Source 配置
- SWITCH: SNMP ifTable (Ports / Speed / Errors)
- NIC: 跳 E-38 Hardware Inventory NIC 详情
- WORKER: 跳 E-36 Resource + Worker 进程详情 (CPU/MEM/Fds/Threads)
- DEST: 跳 CD-01 Tab 6 Output 详情

### 6.5 段 4: EDGE METRICS

```text
┌───────────────┬───────────┬───────────┬─────────┬───────────────┐
│ EDGE          │ RTT       │ LOSS      │ JITTER  │ BANDWIDTH     │
├───────────────┼───────────┼───────────┼─────────┼───────────────┤
│ SOURCE→SWITCH │ 0.3ms     │ 0.000%    │ 0.02ms  │ 8.2 Mbps / 10G│
│ SWITCH→eno1   │ 0.5ms     │ 0.001%    │ 0.05ms  │ 8.2 Mbps / 10G│
│ eno1→Worker   │ 0.1ms     │ 0.000%    │ 0.01ms  │ 8.2 Mbps / 10G│
│ Worker→Worker │ 0.2ms     │ 0.000%    │ 0.02ms  │ -             │
│ Worker→eth0   │ 0.2ms     │ 0.000%    │ 0.02ms  │ 8.2 Mbps / 10G│
│ eth0→CDN-A    │ 0.5ms     │ 0.002%    │ 0.10ms  │ 8.2 Mbps / 10G│
└───────────────┴───────────┴───────────┴─────────┴───────────────┘
```

### 6.6 段 5: DIAGNOSTICS (失败时)

**HEALTHY 时**: 显示绿色 ✓ "All hops healthy, no action needed."

**DEGRADED 时**: 显示黄色 ⚠ 提示:

```text
⚠ DEGRADED
─────────────────────────────────────────────────────────────
Hop 1 (sw-core-01) RTT 142ms > threshold 100ms
→ Likely cause: switch port congestion
→ Suggested action:
  [1] 检查交换机端口流量 [SSH to sw-core-01]
  [2] 联系网络运维
  [3] 切换到 Backup Path
─────────────────────────────────────────────────────────────
```

**FAILED 时**: 显示红色 ⛔ 根因分析:

```text
⛔ FAILED
─────────────────────────────────────────────────────────────
Hop 3 (eno1) link state = DOWN
→ Root cause: NIC eno1 link down
→ Suggested action:
  [1] 检查物理网线
  [2] 检查交换机端口
  [3] [DIAG] 跑 NIC diagnostics
  [4] [FALLBACK] 切到 Backup Source
─────────────────────────────────────────────────────────────
```

每个 root cause 类别对应 §8 失败模式表中的一个条目。

---

## 7. 失败模式分类 (8 类根因)

| # | 根因分类 | 症状 | 诊断方法 | 修复建议 |
|---|---|---|---|---|
| **F1** | Source 端未发送 | 上游 RTT 0 但无数据 | UDP 包探测 (3 次, 间隔 100ms) | 联系上游 / 切 Backup |
| **F2** | 网卡物理层 down | eno1 link state DOWN | E-38 NIC status | 检查网线 / 交换机端口 |
| **F3** | 网卡 IP 配置错 | 网卡 UP 但 ICMP 不到 gateway | route + ping gateway | 重配 IP |
| **F4** | VLAN 不通 | ICMP gateway 通但远端 Source 不通 | traceroute 到 Source IP | 查 VLAN 配置 |
| **F5** | IGMP 未 join (Multicast) | 网卡 UP, Source IP 通, 但无组播流 | `tcpdump` / Multicast Diagnostics | 检查 IGMP / SSM 配置 |
| **F6** | ASM 被拒 (SSM 验证) | IGMP joined 但收不到 Source-Specific 流 | 检查 `mtrace` / 流量采样 | 改 SSM 模式 / 修 Source Address |
| **F7** | Worker 进程死 | 链路通, NIC 有流量, 但 Source state = UNLOCKED | ps + E-36 Resource | 重启 Worker (受限操作) |
| **F8** | 远端 Destination 不收 | Worker 在发, NIC 有出向流量, 但 CDN 反馈 0 | 5xx/4xx 探测 / HTTP probe | 联系 CDN / 切 Backup Output |

E-41 显示时按下列算法定位根因:

```text
1. 从 DESTINATION 反向回溯
2. 第一个 status=FAILED 的 hop 即为根因节点
3. 根据 hop kind 查 §7 失败模式表, 推荐修复
4. 如果根因节点是 NETWORK_DEVICE (中间), 提示"请联系网络运维" (E-41 不接管中间设备)
```

⛔ **E-41 不接管中间网络设备** (交换机 / 路由器 / 防火墙), 仅提示问题位置, 不自动重配。

---

## 8. 4-Layer (Desired / Compiled / Effective / Impact) 应用

PIA V0.1 §6 锁的 4-Layer 模型在 E-41 上的体现:

| Layer | E-41 上的含义 | 状态字段 |
|---|---|---|
| **Desired** | 用户配置的 Source / Output 端点 | `desired_source_id`, `desired_output_id` |
| **Compiled** | Graph Compiler 解析后的实际端点 + 路由 | `compiled_route`, `compiled_interfaces` |
| **Effective** | 当前正在使用的实际路径 | `effective_hops[]`, `effective_status` |
| **Impact** | 此 Path 变化对 Channel / Bundle 的影响 | `affected_channels[]`, `affected_sessions[]` |

**Path 变化触发的 Impact 提示** (示例):

```text
⚠ IMPACT
─────────────────────────────────────────────────────────────
Path PATH-CH01-UDP-MULTICAST-001-TO-CDN-A 状态变化:
HEALTHY → FAILED
影响:
  · 1 Channel: CH01 (新闻综合) — 当前 ON AIR
  · 2 Output: CDN-A (HLS) + CDN-A (RTMP)
  · 0 Session (Realtime Transcode 暂未切到此 Path)
建议: 立即切 CH01 到 Backup Source (SRT-001)
[ APPLY BACKUP ]   [ DISMISS ]
─────────────────────────────────────────────────────────────
```

---

## 9. 与其他表面集成

### 9.1 E-40 Network Source (配套)

| 触发 | 行为 |
|---|---|
| E-40 配置保存成功 | 自动跑一次 Path probe, 刷新 E-41 缓存 |
| E-40 删除 Source | E-41 中所有引用此 Source 的 Path 标 STALE |
| E-40 检测到 Source 状态变化 | E-41 自动重 probe |

### 9.2 CD-01 Channel Detail (入口)

- **CD-01 · Tab 3 (Source)** 每行右侧加 [PATH] 按钮 → 跳 E-41 双端查询
- **CD-01 · Tab 6 (Output)** 每行右侧加 [PATH] 按钮 → 跳 E-41 双端查询
- **CD-01 · Tab 4 (Health)** 任一 Source 状态为 RED 时, 顶部加红色条幅 → 跳 E-41

### 9.3 CH-01 Channel List (入口)

- Channel 卡片底部"Health"区, 任一 Path 状态为 FAILED 时显示 ⚠ 图标 → 跳 E-41

### 9.4 E-37 Clock (Clock Source 关联)

- Path 中 Worker hop 的 RTT 测量需要 Clock Reference (PTP / TIMECODE / SYSTEM)
- 如果 E-37 选定的 Reference 不可用, E-41 的 RTT 测量标 ⚠ "Clock not synced, RTT may be inaccurate"

### 9.5 E-38 Hardware Inventory (NIC 详情)

- E-41 的 NIC hop 点击 → 跳 E-38 该 NIC 详情页 (Capabilities / Ports / Health / Firmware)

### 9.6 09 Health (汇总)

- Health Tree 中"Network"分支聚合所有 E-41 监控的 Path 状态

---

## 10. V0.2 / V0.3 实施边界

| 功能 | V0.2 (0.5G 实施) | V0.3 后续 |
|---|---|---|
| Source 端到 Output 端双端 Path 视图 | ✅ | - |
| 5 类 Hop 模型 (SOURCE / NETWORK / NIC / WORKER / DEST) | ✅ | - |
| 主动 + 被动探测 (30s / 5s / 事件触发) | ✅ | - |
| ICMP / UDP / RTP 探测 | ✅ | - |
| 8 类失败模式根因提示 | ✅ | - |
| SNMP 交换机发现 | ❌ (Layer 3 探测, 尽力) | ✅ |
| LLDP 邻居发现 | ❌ | ✅ |
| WAN 链路 SLA 测量 (Jitter / Out-of-order) | ❌ | ✅ |
| Path 历史 (24h 趋势图) | ✅ (基础表格) | 完整图表 |
| Path 异常告警 (推送到 INCIDENT) | ✅ (基础事件) | 智能降噪 |
| 历史 Path 回放 (查"昨天 14:32 断了") | ❌ | ✅ |
| Path 模拟 (改动 Source, 预测 Path 变化) | ❌ | ✅ |

---

## 11. 6 状态 (Spec 级)

> 口径分层: Spec 级必须有 6 状态定义; Wireframe 级 0.5G 实施时逐页呈现。

| 状态 | 触发条件 | UI 表现 |
|---|---|---|
| **EMPTY** | 尚未配置任何 Source / Output, 或用户没选 | 提示"请先选择 Source 和 Output" + 推荐 5 个最近用过的 |
| **LOADING** | Path 正在探测中 | 骨架屏 + "Probing 8 hops..." 进度条 |
| **HEALTHY** | 全部 8 hop 状态 UP, RTT/Loss 在阈值内 | 绿色 ✓ 完整 path diagram, "All healthy" |
| **DEGRADED** | 至少 1 hop DEGRADED (RTT > 阈值 或 loss > 阈值) | 黄色 ⚠ 标黄节点 + 段 5 root cause 提示 |
| **FAILED** | 至少 1 hop FAILED (Link down / 探测超时) | 红色 ⛔ 标红节点 + 段 5 root cause + [切 Backup] 按钮 |
| **STALE** | 配置变更 (E-40 / CD-01 修改) 尚未重 probe | 灰色 "Configuration changed, re-probing..." 自动 5s 后刷新 |
| **UNKNOWN** | Layer 3 探测失败 (权限 / 网络隔离) | 灰色节点, 提示"探测受限, 请联系 SRE" |

⛔ **不允许出现"半透明混合"** (如部分节点 HEALTHY 部分 UNKNOWN 且整体显示绿色)。要么 HEALTHY, 要么 DEGRADED, 要么 FAILED, 要么 UNKNOWN。

---

## 12. Schema 草稿 (YAML · V0.2 字段集)

```yaml
# /api/v1/network-path/{path_id} response
network_path:
  schema_version: "0.1"
  id: PATH-CH01-UDP-MULTICAST-001-TO-CDN-A
  direction: BIDIRECTIONAL        # BIDIRECTIONAL | EGRESS_ONLY | INGRESS_ONLY

  source:
    id: UDP-MULTICAST-001
    label: "CH01 远端演播室 Multicast"
    endpoint:
      mode: MULTICAST             # UNICAST | MULTICAST
      group: 239.20.10.10
      port: 5000
      source_specific: true        # SSM
      source_ip: 10.30.20.100

  destination:
    id: CDN-A-HLS-001
    label: "CDN-A HLS 主"
    endpoint:
      scheme: HLS
      url: https://cdn-a.example.com/live/CH01.m3u8

  hops:
    - index: 0
      kind: SOURCE
      label: "10.30.20.100"
      role: ORIGIN
      status: HEALTHY

    - index: 1
      kind: NETWORK_DEVICE
      label: "sw-core-01 (10.30.20.1)"
      role: TRANSIT
      discovery: SNMP             # SNMP | TRACEROUTE | LLDP | STATIC
      status: HEALTHY
      if_index: 4

    # ... (index 2-8)

  edges:
    - from: 0
      to: 1
      rtt_ms: 0.3
      loss_pct: 0.0
      jitter_ms: 0.02
      bandwidth_used_mbps: 8.2
      bandwidth_max_mbps: 10000
      measured_at: 2026-08-25T08:50:14Z
      probe_method: ICMP_RATE_LIMITED

    # ... (5 edges)

  overall:
    status: HEALTHY
    rtt_ms: 4.2
    loss_pct: 0.003
    last_probed_at: 2026-08-25T08:50:14Z
    next_probe_at: 2026-08-25T08:50:44Z

  impact:
    affected_channels: [CH01]
    affected_outputs: [CDN-A-HLS-001, CDN-A-RTMP-001]
    affected_sessions: []        # 0 (Realtime Transcode 暂未切到此 Path)

  audit:
    created_at: 2026-08-25T08:30:00Z
    created_by: system
    last_modified_at: 2026-08-25T08:50:14Z
    last_modified_by: system
```

---

## 13. 验收标准 (0.5G 实施前检查)

### 13.1 必达

- [ ] E-41 入口从 CD-01 (Tab 3 / Tab 6)、CH-01 (Channel 卡片)、E-40 (顶部按钮) 都能跳通
- [ ] 5 类 Hop 模型在 path diagram 中颜色与符号一致 (DESIGN_SYSTEM.md)
- [ ] 8 类失败模式 (F1-F8) 都有对应的 root cause 提示
- [ ] 主动探测 (30s) + 被动探测 (5s) + 事件触发都生效
- [ ] ICMP / UDP / RTP 三种探测方法覆盖 UDP Unicast / Multicast / SRT / RTMP / HLS / RTSP
- [ ] 4-Layer (Desired/Compiled/Effective/Impact) 在 E-41 都有对应显示
- [ ] 6 状态 (EMPTY / LOADING / HEALTHY / DEGRADED / FAILED / STALE / UNKNOWN) 都有 Spec
- [ ] 不引入新 Engine, 完全在 Network Endpoint 内部实现
- [ ] 所有探测是 read-only, 不影响正常播出

### 13.2 不在 0.5G 范围 (V0.3)

- ❌ SNMP 完整 ifTable 发现
- ❌ LLDP 邻居自动发现
- ❌ WAN 链路 SLA 详细测量
- ❌ Path 历史趋势图 (仅 24h 表格)
- ❌ Path 模拟

---

## 14. 已知留白 (Open Questions)

| # | 问题 | 决策时机 | 默认行为 |
|---|---|---|---|
| **OQ-1** | Worker hop 的 RTT 如何测量? 跨进程 IPC? | V0.2 实施时 | 用 clock_gettime(CLOCK_MONOTONIC) 差值, 测量 worker 内处理耗时 |
| **OQ-2** | IGMP join 状态如何探测? (SSM 验证) | V0.2 实施时 | 用 `ip maddr show` + `tcpdump` 5 秒采样 |
| **OQ-3** | E-41 跳过去后能否"锁定"探测频率 (临时高频)? | 0.5G 实施时 | 默认不支持, V0.3 加 |
| **OQ-4** | Path 历史数据保留多久? | V0.2 数据层决策 | 7 天, 30 天, 90 天 3 档 (默认 30 天) |
| **OQ-5** | Path 异常是否自动触发 INCIDENT? | V0.2 决策 | 默认开, 阈值可配 (failed 1 次即触发, degraded 持续 30s 触发) |

---

## 15. 锁清单 (本 Spec 锁定的 8 项)

| # | 锁 | 章节 |
|---|---|---|
| 1 | Network Path = Source → Hops → Output 有向图, 5 类 Node Kind (SOURCE / NETWORK_DEVICE / NIC / WORKER / DEST) | §2 |
| 2 | 输入: 双端 (Source + Output) 或单端 (Source 单独 / Output 单独) | §4 |
| 3 | 数据来源 3 层: VBMF 自身 (E-38) + 静态配置 (E-40/CD-01) + 动态探测 (traceroute / SNMP) | §5 |
| 4 | 探测频率 3 档: 30s 健康 / 5s ON_AIR / 事件触发 | §5.3 |
| 5 | 显示布局 5 段: Header / Diagram / Hop Table / Edge Metrics / Diagnostics | §6 |
| 6 | 失败模式 8 类 (F1-F8) + 根因回溯算法 | §7 |
| 7 | 4-Layer (Desired/Compiled/Effective/Impact) 在 E-41 完整应用 | §8 |
| 8 | V0.2 仅实施端到端 + 5 类 Hop + 3 档探测 + 8 类失败模式; SNMP / LLDP / WAN SLA 推到 V0.3 | §10 |

---

## 16. 相关文档

- [`PRODUCT_INFORMATION_ARCHITECTURE.md` §10](PRODUCT_INFORMATION_ARCHITECTURE.md#10-network-path-model-新架构对象-v02-24-扩展) — PIA V0.1 Network Path Model 来源
- [`OBJECT_VOCABULARY.md`](OBJECT_VOCABULARY.md) — 15 核心对象权威定义 (Route, Output Adapter, Output Destination)
- [`PRODUCT_OBJECT_MODEL.md`](PRODUCT_OBJECT_MODEL.md) — 3 层组合关系 (Profile → Bundle → Variant)
- [`NAVIGATION.md` §3](NAVIGATION.md) — ENGINEERING 域 Network 子域位置
- [`SURFACE_SPEC.md` §29](SURFACE_SPEC.md) — 0.5C/0.5D/0.5F 收口
- [`DESIGN_SYSTEM.md`](DESIGN_SYSTEM.md) — 颜色 / 状态 / 组件规范
- [E-40 Network Source wireframe](operator/E-40-network-source.html) — 配套配置表面
- [E-37 Clock wireframe](operator/E-37-clock.html) — Clock Reference 来源
- [E-38 Hardware Inventory wireframe](operator/E-38-hardware-inventory.html) — NIC 数据来源
- [CH-01 Channel List](operator/CH-01-channel-list.html) — 入口
- [CD-01 Channel Detail](operator/CD-01-channel-detail.html) — 入口

---

**VBMF Contributors** · E-41 Network Path Inspector Spec V0.1 · Phase 0.5F Channel/Network UX Closure (DRAFT 0.1)
