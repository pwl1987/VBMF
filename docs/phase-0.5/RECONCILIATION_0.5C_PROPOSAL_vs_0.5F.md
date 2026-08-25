# 0.5C 提案 vs 本地 0.5F 逐条对账报告

> 目的：用户基于 GitHub master 旧结构（phase-0.5b 并存、视角停在 0.5A/0.5B）提出长篇
> "Phase 0.5C — Channel / Source / Encode / Network UX Closure" 提案（共 33 节）。
> 本报告将 33 节逐条与本地工作区已落地的 **Phase 0.5F LOCK FINAL** 对照，确认是否有遗漏。
> 结论：**提案绝大部分已被 0.5F 覆盖，仅少数点需做增量，无需从零重写 PIA。**

图例：✅ 已锁 / ⚠ 部分覆盖或待核对 / ❌ 明确缺口

---

## 0. 背景对齐（重要）

| 维度 | GitHub master（用户提案依据） | 本地工作区实际 |
|---|---|---|
| 目录 | `docs/phase-0.5/` 与 `docs/phase-0.5b/` 并存 | `phase-0.5b/` 已 git mv 归并入 `phase-0.5/` |
| 阶段 | 0.5A / 0.5B 视角 | 已推进到 **0.5E / 0.5F LOCK FINAL** |
| 关键文档 | 无 PIA | `PRODUCT_INFORMATION_ARCHITECTURE.md`（PIA V0.1.1，12 锁）、`E-41-NETWORK_PATH_INSPECTOR.md`（Spec V0.1） |
| wireframe | 0.5B 仅 M-11/M-12/M-14/P-21/P-22 | 已含 CD-01 / CH-01 / E-40 / M-14 / M-17 等 |

---

## 1. 逐节对账（33 节）

| # | 提案核心 | 本地 0.5F 状态 | 对应物 |
|---|---|---|---|
| 1 | Source 统一（物理+网络+File+Internal 一页） | ✅ | PIA §2/§4（Source 6 字段分解 + 二级 Taxonomy：Local Device / External Network）；`02-sources.html` 待核对是否为统一列表 |
| 2 | Add Source 统一向导（Type→Adapter→Endpoint→Contract→Clock→QC→Preview→Save） | ✅ | PIA §4.2 两级向导（Mode→Category→Config） |
| 3 | UDP 正式升级为 Network Source | ✅ | `E-40-network-source.html` 5 Mode Tab：UDP Unicast / Multicast / RTP-UDP / SRT Caller / SRT Listener |
| 4 | UDP Unicast 表单（Interface/Bind/Remote/Port/Payload/Buffer/DSCP/Test Receive） | ✅ | E-40 Endpoint 面板（含 Unicast 字段 + Test 操作） |
| 5 | Multicast 独立表单状态（Group/Port/ASM-SSM/IGMP v3/TTL/Join/Leave/Probe） | ✅ | E-40 Endpoint 面板 + 7 项 Multicast Diagnostics + Test Operations（Join/Leave/Probe/IGMP Querier/RTP Inspect） |
| 6 | Network Path Inspector（网卡→VLAN→路由→IGMP→…→Decode） | ✅ | `E-41-NETWORK_PATH_INSPECTOR.md`（Spec V0.1，8 失败模式 + 5 段布局）；E-40 内嵌 8-Stage Network Path 面板 |
| 7 | Output 对称支持 UDP（Input/Output 都含 Unicast/Multicast） | ✅ | PIA §12 / P-22 Output Profile（含 UDP）；P-22 wireframe 待核对是否含 UDP Output + Test Send |
| 8 | Output UDP Network Delivery 层（Mode/Dest/Payload/TTL/DSCP/Bitrate/Test Send） | ✅ | PIA §12 Output Profile 目标含 Unicast/Multicast、TTL、DSCP、Bitrate、Continuity |
| 9 | 转码配置可保存为模板 | ✅ | P-21 Encoding Profile + P-28 Profile Bundle + M-14 |
| 10 | 第一类 File Transcode（质量/效率优先、可排队/暂停/重试/多 Worker） | ✅ | `M-14-file-transcode.html` + PIA §12 M-14 |
| 11 | 第二类 Realtime/Live Encode（持续运行、低延迟、热备、失败自动恢复） | ✅ | `M-17-realtime-transcode.html` + PIA §12 M-17 |
| 12 | Encoding Profile 拆 `FILE_PROFILE` / `REALTIME_PROFILE` 两类 | ❌ | M-14/M-17 已分拆为 workflow，但 **Profile 对象 Schema 未以这两类型名锁定** → 缺口 G1 |
| 13 | Realtime Profile 专属属性（Latency Class / Failover Compatibility / Encoder Warm-up / Hot Standby / Resource Reservation） | ❌ | 未在 0.5F 文档以 Schema 锁定 → 缺口 G1（同 §12） |
| 14 | File Profile 另一套 UI（Purpose: Archive/Proxy/Web/Social） | ⚠ | M-14 已含 File Profile 表单；"Purpose 语义"待核对 |
| 15 | M-14 变 Transcode Center 顶层（File/Live/Jobs/Workers） | ⚠ | M-14/M-17/M-18(Jobs) 已分；Transcode Center 顶层容器待核对 |
| 16 | Profile Bundle（News HD Live 模板聚合 Video+Audio+Output+QC+Edge） | ✅ | P-28 Profile Bundle（PIA §12） |
| 17 | 模板一键自动带出 6 个 Profile | ✅ | P-28 |
| 18 | Template Preview Impact（资源预算 + Preflight） | ⚠ | PIA §6 4-Layer 已含 IMPACT；P-28 是否展示资源预算待核对 |
| 19 | DESIRED/COMPILED/EFFECTIVE 推广到 Video/Audio/Output | ✅ | PIA §6 升级为 **4-Layer**（+ IMPACT）；E-40 已实装 4-Layer 面板 |
| 20 | 音频+切换不合并但需 Channel Control Workspace 联动 | ✅ | PIA §5 双层 UI + CD-01 |
| 21 | Channel Control 页面（CH01 综合操作台 7 块：PVW/PGM/Switch/Audio/Master/Output/Health/Clock） | ✅ | `CD-01-channel-workspace.html` |
| 22 | 音频与切换关联（Take 前 readiness：Video/Audio/PTP/AVSync/QC） | ✅ | CD-01（Source B readiness、FRAME_SWITCH、Clock LOCKED） |
| 23 | Take Preflight 联合检查（TAKE 前 9 项校验） | ❌ | CD-01 含 TAKE 按钮但**无独立 Preflight 联合检查面** → 缺口 G2 |
| 24 | Output "知道自己为什么坏"（Failure Domain Matrix：坏了不误切源） | ✅ | E-40 6 状态样例（含 CRITICAL 强切备份 Source）+ PIA Failure Domain |
| 25 | Source Test Bench（添加源时逐层验证） | ❌ | 本地 **无独立 E-42 面** → 缺口 G3 |
| 26 | Capability Preview（物理/网络源可用切换模式 FRAME/MASTER/PACKET） | ✅ | PIA §2/§4 Source Capability（Available For）；wireframe 待核对 |
| 27 | Network Endpoint Model 统一对象（非新增 Engine） | ✅ | PIA §3 Network Endpoint 统一对象（protocol/direction/address/port/.../multicast_group/...） |
| 28 | 目录重排为 product/ui/...（版本=Git tag，目录=产品结构） | ⚠ | 理念已锁（README："目录不应承担版本职责，Git commit/tag 才是版本管理"），但本地仍以 `phase-0.5` 命名未重排 → 决策点 D1 |
| 29 | 新增 Surface 表（B-11/E-38/E-39/E-40/M-17/P-28/B-13/E-41/M-18/E-42） | ✅ 多数 | B-11=CD-01 ✅ / E-38=Source Manager ✅ / E-39=E-40 ✅ / E-40=Network Path ✅ / M-17 ✅ / P-28 ✅ / E-41 ✅；**缺 B-13(G2)、E-42(G3)、M-18 Template(P1)** |
| 30 | 合并而非新增（Source 合并 / Channel Control 合并 Switcher+Audio+Health+Output / Transcode Center 合并） | ✅ | PIA §5 双层 UI 已锁 |
| 31 | Phase 0.5 状态判断（0.5A 成熟 / 0.5B 成熟 / 0.5C 闭环） | ✅ | 本地已推进到 0.5F（比 0.5C 更进一层），判断方向一致 |
| 32 | 先锁 5 条真实流程 A-E（新建网络源/物理源/实时频道/网络输出/故障切换） | ⚠ | PIA §13 已有 Operator Workflow + 0.5E Cross-Domain Capabilities；5 条 A-E 逐条覆盖待核对 |
| 33 | 最终判断：进 0.5C、不增 Engine、先写 4 文档 | ✅ | 本地 0.5F 已完成；**不增 Engine 已锁（PIA §9）**；4 文档中 PIA 已存在，CHANNEL_CONTROL_SPEC/NETWORK_SOURCE_SPEC/ENCODE_MODEL_SPEC 以 wireframe + PIA 章节承载，独立 SPEC 文件仅 ENCODE_MODEL_SPEC 缺失 |

---

## 2. 真实缺口（需动手，对应后续任务）

- **G1 — Encoding Profile 拆两类 + Realtime 专属属性**（提案 §12-13）：写 `ENCODE_MODEL_SPEC.md`，定义 `FILE_PROFILE` / `REALTIME_PROFILE` 两种类型，并列出 Realtime 专属属性（Latency Class、Rate Control、GOP Strategy、Frame/Audio Sync Strict、Encoder Warm-up、Fallback Encoder、Max Startup Latency、Target CPU/GPU、Failover Compatibility PACKET/FRAME/MASTER、Resource Reservation）。这是提案最核心、0.5F 最薄弱的一点。
- **G2 — B-13 Take Preflight 独立面**（提案 §23）：在 CD-01 TAKE 流程上补独立联合检查面（Source/Video/Audio/Clock/Switch/Backup/Output/Latency 9 项 PASS 后才放 TAKE）。
- **G3 — E-42 Source Test Bench 独立面**（提案 §25）：添加源时的逐层验证（Network→Transport→Container→Video→Audio→Clock→QC）。

## 3. phase-0.5b/ 残留引用（清理项，非功能性死链）

`search "phase-0.5b/"` 命中 4 个文件、10 处，但**全部为历史叙述 / 迁移记录**（描述"已 git mv / 已删除"），无 `](phase-0.5b/...)` 功能性死链：
- `SURFACE_SPEC.md`（目录树示例 + "从 phase-0.5b/ 移过来"）
- `README.md`（"phase-0.5b/README.md 已删除"）
- `MILESTONES.md`（目录树示例 + 删除记录）
- `milestones/0.5C-INFO_ARCH_CLOSURE.md`（迁移说明）

清理建议：将目录树示例中的 `phase-0.5b/` 引用改为 `phase-0.5/`，并标注"迁移已完成"，保留历史叙述。

## 4. 待核实项（不影响结论，但影响对账精度）

- `02-sources.html` 是否已重写为统一 Source Manager 列表（提案 §1-2）
- `P-22` Output Profile wireframe 是否已含 UDP Output + Test Send + Multicast 表单（提案 §7-8）
- `P-28` 是否展示 Template Preview Impact 资源预算（提案 §18）
- 5 条真实流程 A-E 是否逐条被 0.5F workflow 覆盖（提案 §32）
- 物理/网络源 Capability Preview 是否有独立 wireframe（提案 §26）

## 5. 决策点 D1（目录重排）

提案 §28 建议重排为 `product/ui/...`。本地 `README.md` 已明确锁："目录不应承担版本职责，Git commit/tag 才是版本管理"。理念一致，但本地仍以 `phase-0.5` 命名且已 LOCK FINAL。
**建议：保持 `phase-0.5` 命名，不二次移动目录**（避免破坏已 LOCK 的引用与历史）。如坚持重排，需同步更新所有内部链接。

## 6. 建议的增量执行顺序（已与用户确认）

1. 清理 phase-0.5b/ 残留引用（§3）
2. 写 `ENCODE_MODEL_SPEC.md`（G1，提案 §12-13）
3. 补 `E-42 Source Test Bench`（G3，提案 §25）
4. 补 `B-13 Take Preflight`（G2，提案 §23）

前序条件：本对账报告需用户确认"无遗漏"后再动手。

---

# 第二轮回对账：23 节「真实广播工作流」提案 vs 0.5F

> 用户基于「真实广播机房怎么干活」而非「页面覆盖架构对象」重审，提交 23 节提案，
> 核心判断：**现在还不建议冻结 Phase 0.5**（产品工作流未真正闭环）。
> 本报告将 23 节与本地已落地的 **0.5F LOCK FINAL** 逐条对账（含上一轮已补的 `ENCODE_MODEL_SPEC` / `E-42` / `B-13` 三份 Spec）。
>
> **结论：23 节中约 19 节已被 0.5F 覆盖；真正未落地（缺独立 wireframe/面板 或 未登记）的仅 4 项（见 §B）。**
> 用户「不冻结」的判断在 GitHub master 旧视角成立；本地已具备冻结底子，仅余少量增量面未画出。

图例：✅ 已锁（含 wireframe） / ⚠ 原则/文档已锁，但独立 wireframe/面板缺失 / ❌ 未覆盖

## A. 逐节对账（23 节）

| # | 提案核心 | 本地 0.5F 状态 | 证据 |
|---|---|---|---|
| 1 | UI 按 Engine 组织 vs 按任务工作 | ✅ | PIA §5 双层 UI（Operation 工作台 + Engineering 深页）；§0.5F 范式转移明确「不再按 Engine 一一对应拆页」|
| 2 | Channel Workspace 一级概念（7 块） | ✅ | PIA §7（7 块布局）+ `operator/CD-01-channel-workspace.html` + `CD-01-channel-detail.html`（8 Tab）|
| 3 | 音频与监控/切换关联但不合巨页（二级 Audio Profile） | ✅ | PIA §5 双层；CD-01 Detail Tab 3 Audio（Mixer/Gain/Mapping/Loudness/Delay/Ducking…）|
| 4 | Source 统一入口（Physical/Network/File/Internal 分类） | ✅ | PIA §2 Source 6 字段 + §4 二级 Taxonomy（Local Device / External Network）；`02-sources.html` 已重写 Local+External |
| 5 | UDP Unicast 正式 UX（P0） | ✅ | `E-40-network-source.html`（UDP Unicast 字段 + Test Receive）；PIA §3 Network Endpoint |
| 6 | UDP Multicast（IGMP/SSM/ASM/Join/Probe/Leave） | ✅ | `E-40-network-source.html`（22KB，含 SSM/ASM/IGMP v3/TTL/Join/Leave/Probe + 7 项 Multicast Diagnostics）|
| 7 | 「为什么没信号」分层诊断 | ✅ | `E-41-NETWORK_PATH_INSPECTOR.md`（8 失败模式 F1-F8，逐层根因）+ E-40 内嵌 8-Stage Network Path |
| 8 | Network Path / Signal Path Inspector | ✅ | `E-41-NETWORK_PATH_INSPECTOR.md`（Spec V0.1，双向 Path，ICMP/UDP/RTP 探测）|
| 9 | UDP Output 与 Input 对称（Unicast/Multicast） | ⚠ | E-41 双向 Path 已覆盖；PIA §12 Output Profile 含 UDP；**但 `06-output.html` 仅 HLS/RTMP/Local，无 UDP Unicast/Multicast Output + Test Send wireframe**（缺口 G-A）|
| 10 | File Transcode ≠ Realtime Encode（两类 Job/UX） | ✅ | `M-14-file-transcode.html` + `M-17-realtime-transcode.html` 已分拆；PIA §12 |
| 11 | Profile Type（FILE / REALTIME） | ✅ | `ENCODE_MODEL_SPEC.md`（上一轮补，焊实 `FILE_PROFILE`/`REALTIME_PROFILE` 双语义）|
| 12 | Realtime Profile 专属属性（Latency/Failover/Warm-up/Hot Standby/Resource Reservation） | ✅ | `ENCODE_MODEL_SPEC.md`（Realtime 专属属性全列）|
| 13 | Profile Bundle / Channel Template（一键带出 6 Profile） | ✅ | `P-28-profile-bundle.html` + PIA §12 |
| 14 | Bundle 影响预览（资源预算 + Preflight） | ✅ | PIA §6 4-Layer 含 IMPACT；0.5E LOCK（Impact Preview 跨域 Spec）|
| 15 | TAKE 前联合预检（TAKE PREFLIGHT） | ✅ | `B-13-take-preflight.md`（上一轮补，9 项联合检查）；CD-01 含 TAKE 按钮 |
| 16 | Output 故障从 UI 阻止错误操作员行为（Guided Recovery） | ⚠ | SURFACE_SPEC §8.9 Failure Domain Matrix（Recovery Policy SoT）+ DESIGN_SYSTEM `recovery` 字段 + OPERATOR_WORKFLOW chain-1/2 + `06-output.html` Restart Adapter 按钮；**但无独立 "Output Failure Guided Recovery" 指导面板 wireframe**（缺口 G-B）|
| 17 | Source→Channel 映射管理（Assign to Channel） | ⚠ | `02-sources.html` 有反向「Used By」跨域面板；**但无 "Assign to Channel" 主动分配交互面**（缺口 G-C）|
| 18 | 反向关系（Used By：Source/Channel/Endpoint） | ✅ | `02-sources.html` USED BY 面板 + SURFACE_SPEC §24（Used By 全域）+ P-21/P-22 已锁 |
| 19 | Channel 当前运行 vs 目标配置（Desired/Compiled/Effective） | ✅ | PIA §6 升级 **4-Layer**（+ IMPACT）；E-40 已实装 4-Layer 面板；CD-01 内嵌 |
| 20 | ChangeSet Preview（Before/After/Impact） | ✅ | 0.5E LOCK（Impact Preview + Configuration Diff 跨域 Spec）|
| 21 | 保留深度页 + 新增工作台页（避免页面爆炸） | ✅ | PIA §5 双层已锁（Operation 工作台：CH-01/CD-01；Engineering 深页：02/03/05/06/08/09/E-37/E-40）|
| 22 | 目录重排 product/ui（版本=Git tag） | ⚠ | 决策点 D1（README 已锁「版本=Git tag，目录不担版本」）；本地仍 `phase-0.5` 命名，不二次移动 |
| 23 | 最终 P0 缺口清单（17 P0 + 4 P1） | ✅（方向一致） | 本地已以 0.5F + 上一轮增量覆盖其绝大多数；残余即下方 §B 4 项 |

## B. 真实缺口（本轮核实 → 已全部闭合）

> 用户于 2026-08-25 确认「是，确认」后，G-A~G-D 已动手完成（见下方 §D 实施记录）。

- ✅ **G-A — UDP Output 对称面**（提案 §9）：已在 `operator/06-output.html` 新增 `📡 UDP OUTPUT` tab + view（Mode=Multicast/Unicast、Group/Port/Interface/IGMP/SSM/TTL/Payload/Packet Size/Bitrate/DSCP + Test Send），与 `E-40-network-source.html` Input 对称。
- ✅ **G-B — Output Failure Guided Recovery 独立面板**（提案 §16）：已在 `operator/06-output.html` 新增 `🛟 FAILURE RECOVERY` tab + view（分层诊断：Program/Source/Switcher HEALTHY vs HLS Adapter FAILED；推荐恢复序 Restart/Retry/Fallback Variant；⛔ 禁止误切节目源），对应 Failure Domain Matrix §8.9。
- ✅ **G-C — Source→Channel 主动 Assign 交互面**（提案 §17）：已在 `operator/02-sources.html` 追加「ASSIGN TO CHANNEL」面板（Source 选择 + Channel/Role + Clock/Switch Mode/Hot Standby + Assign+Preflight + 当前映射表），与上方 USED BY 反向互补。
- ✅ **G-D — 3 份 Spec 登记进表面清单**：已将 `ENCODE_MODEL_SPEC.md` / `E-42-source-test-bench.md` / `B-13-take-preflight.md` 登记进 `NAVIGATION.md`（P-21 引用 + B-13 / E-42 Spec-only 行 + §2.5 变更登记）、`SURFACE_SPEC.md`（§29.9.3b Batch 4 + 计数说明）、`INDEX.md`（补充 Spec 文档段）。

> 注：G-A/G-B/G-C 为「独立 wireframe/面板」缺口；G-D 为「Spec 已存在但未登记」。
> 前一轮 G1（Encode 双类型）/ G2（B-13）/ G3（E-42）**已在上一轮闭合**；本轮 G-A~G-D 一并闭合。至此用户 23 节提案的全部真实 P0 缺口在本地 `phase-0.5` 已闭合。

## C. 与上一轮（33 节）对账的关系

- 上轮 §12-13（Encode 双类型 + Realtime 专属属性）→ 本轮回账 §11-12 → 已由 `ENCODE_MODEL_SPEC.md` 闭合（G1 完成）。
- 上轮 §23（TAKE Preflight）→ 本轮回账 §15 → 已由 `B-13-take-preflight.md` 闭合（G2 完成）。
- 上轮 §25（Source Test Bench）→ 本轮回账（提案未单列，但 §4/§7 间接覆盖）→ 已由 `E-42-source-test-bench.md` 闭合（G3 完成）。
- 上轮 §7（Output UDP 对称）原标 ✅，本轮据 `06-output.html` 实际内容**下修为 ⚠（G-A）**——这是本轮最关键的精度修正。
- 上轮 §17（Source→Channel Assignment）原标 ✅，本轮据 `02-sources.html` 实际内容**下修为 ⚠（G-C）**。
- 上轮 §24（Output Failure Domain）原标 ✅，本轮据是否含独立面板**下修为 ⚠（G-B）**。

## D. 增量执行记录（已于 2026-08-25 用户确认后完成）

执行顺序按 §D 原计划 `G-D → G-A → G-C → G-B`：

1. ✅ **G-D**（登记）：NAVIGATION / SURFACE_SPEC / INDEX 三处追加 3 份 Spec（ENCODE_MODEL_SPEC / E-42 / B-13）。均为 Spec-only，不计 0.5F 48 wireframe 计数；E-42/B-13 表面将在 0.5G 实施 wireframe（+2）。
2. ✅ **G-A**（UDP Output 对称面）：`06-output.html` 新增 UDP OUTPUT tab/view。
3. ✅ **G-C**（Source→Channel Assign）：`02-sources.html` 新增 ASSIGN TO CHANNEL 面板。
4. ✅ **G-B**（Output Failure Guided Recovery）：`06-output.html` 新增 FAILURE RECOVERY tab/view。

> 至此，用户 23 节提案中标注的「真实 P0 缺口」在本地 `phase-0.5` 已**全部闭合**。本地 0.5F 现已具备冻结底子：Channel Workspace + Unified Source + UDP Input/Output + Network Path + File≠Realtime + Profile Bundle + TAKE Preflight + 4-Layer + ChangeSet + 双向 Used By + Guided Recovery + Source→Channel 主动映射均已落地（含 wireframe 或 Spec）。是否正式宣布 0.5F FINAL 由用户决定。

前序条件：本补充对账报告需用户确认「无遗漏」后再动手。

---

## 第三轮回对账（a54d1a0 评审 · 一致性 + 工作流 + UI/UX + 架构边界）

> 用户基于提交 `a54d1a0` 做「一致性 + 工作流 + UI/UX + 架构边界」审查，结论：**仍不能冻结 0.5，但方向正确**，提出 9 个 P0 一致性/语义问题 + 8 个 P1 UX 缺口 + 6 个 0.5D 原型（D1-D6）。本轮回对账已**逐条评审并采纳合理项**，落盘修改如下。

### 采纳并落盘（P0 必须修 · 全部已改）

| 项 | 修改文件 | 处置 |
|---|---|---|
| P0-1 6/7 Profile 冲突 | PRODUCT_OBJECT_MODEL / README / P-28 / ENCODE_MODEL_SPEC | 统一 **7 种 Profile**（标题/引用 6→7） |
| P0-2 Resource 三档 | B-13 | ≤80% PASS / 80-100% 仅 reservation 满足可放行 / >100% BLOCK，对齐 ENCODE_MODEL |
| P0-3 Failure Domain 误用 | M-17 / E-40 | RESOURCE 退化不再建议 Failover；E-40 配置/测试失败不再"强切备份"，仅运行态 Active Source 故障才进 §8.9 |
| P0-4 Output Criticality | B-13 | 引入 delivery_criticality（REQUIRED/OPTIONAL/AUXILIARY）；REQUIRED 必 PASS 否则阻断，OPTIONAL 仅 WARNING |
| P0-5 Stage Latency | M-17 | "E2E Latency (SDI→SRS)" → "Stage Latency"（E2E=Source→Player） |
| P0-6 Network/Media Path | E-40 | 拆为 Network Path（STAGE 1-4）+ Media Path（STAGE 5-8）双视图 |
| P0-7 状态冲突 | README / NAVIGATION | 统一：0.5C RECONCILED · 0.5D IN PROGRESS · 0.5E SPEC；NAV 顶部 LOCK FINAL→RECONCILED |
| P0-8 计数冲突 | README / NAVIGATION / SURFACE_SPEC | 确立 NAVIGATION §2.5 为唯一权威（52 wireframe + 1 spec）；22/44/48 废止 |
| P0-9 Bundle 持久化 | PRODUCT_OBJECT_MODEL | profile_bundles 明确为 **0.5D 持久化对象**（去 V0.4 规划歧义） |

### 采纳并落盘（P1 bounded · 已改）

- §5 改名：M-17 Realtime Transcode → Realtime Encode（M-17 HTML + NAVIGATION）
- §3 CD-01 Audio Quick Adjust：增加现场微调按钮（AV Offset ±10ms / Gain +1dB），深度配置仍走 Audio Profile
- §14 Source 业务生命周期 7 态：写入 OBJECT_VOCABULARY §1.6（DRAFT→TESTING→VERIFIED→ASSIGNED→ACTIVE→STANDBY→OFFLINE，区别于 Runtime 三轴）
- §23 Endpoint 子对象：写入 OBJECT_VOCABULARY §1.6（Source = Adapter+Endpoint+Contract+Runtime+QC，Endpoint 不独立持久化）
- §15/§20 Channel Template 4 层：写入 OBJECT_VOCABULARY §1.5（Template≠Bundle≠Profile≠Variant）

### 采纳为 0.5D 工作单（D1-D6 · 本轮未建 wireframe，待确认后建）

- D1 Create Channel Wizard ｜ D2 Channel Template Center ｜ D3 CD-01 v2 ｜ D4 M-17 Realtime Encode v2 ｜ D5 E-40 Wizard+Test Bench ｜ D6 B-13 Take Preflight v2
- 验收链：新建 CH02 → 选模板 → SDI Primary + UDP-Multicast Backup → Test → Realtime Encode Profile → Audio → HLS+RTMP+UDP-Multicast → Resource Preview → Preflight → ChangeSet → STARTING → READY_TO_TAKE → TAKE → 运行 → 故障按 Failure Domain 恢复（Output 故障不切节目源）。此链跑通后 0.5 具备冻结条件。

---

## E. 0.5D 原型执行记录（D1-D6）

> 用户 (`a54d1a0` 评审后) 确认进入 0.5D 原型构建，逐张落 D1-D6 串联验收链。计数统一以 NAVIGATION §2.5 为权威（P0-8）。

### D1 ✅ CH-02 Create Channel Wizard（已建 + 已注册）
- 文件：`operator/CH-02-create-channel.html`（6 步向导：① 模板&基础 ② 信号源 ③ 编码&音频 ④ 输出 ⑤ 资源预览 ⑥ 预检&提交）。
- 覆盖：Channel Template 工厂（不进运行态，见 OBJECT_VOCAB §1.5）、SDI Primary + UDP-Multicast Backup（E-40 双路径）、Source→Channel Assign、E-42 7 层入网验证、Realtime Encode 7 Profile、Audio Quick Adjust、Output delivery_criticality 分级、Resource 三档预览、B-13 9 项联合预检、ChangeSet（E-33）生成。
- 注册：NAVIGATION BROADCAST 列表 + §2.5（BROADCAST 12→13 / 域合计 51→52 / TOTAL 52→53 / 总计 53→54）；SURFACE_SPEC 新增 §29.9.5 Batch 5 + BROADCAST 行 12→13 + TOTAL 行标历史/权威。
- 状态：🟡 DRAFT（0.5D 原型），待与 D2-D6 联调后 LOCK。

### D2 ✅ CH-02B Channel Template Center（已建 + 已注册）
- 文件：`operator/CH-02b-channel-template-center.html`（模板注册表 + 模板详情（默认 7 Profile 引用 / 默认源 / 默认输出）+ 从零创建表单 + 6 状态样例）。
- 覆盖：Channel Template = 创建工厂（**不进运行态**），实例化出 Profile Bundle（7 Profile 引用，P-21/22/23/24/25/26/27）+ Channel(DRAFT)；Template≠Bundle≠Profile≠Variant 层级明示；覆盖 TV_LIVE / RADIO_LIVE / VIRTUAL_PLAYOUT 三类；模板卡可 Clone / Deprecate / Use→CH-02；创建表单动态生成 7 Profile 引用 + 默认输出 Variants。
- 缺口标注（沿用 D1 口径）：G2 默认源内联创建待定 / G3 基带 SDI 输出待补 / G4 Output Resilience 待补。
- 注册：NAVIGATION BROADCAST 列表 + §2.5（BROADCAST 13→14 / 域合计 52→53 / TOTAL 53→54 / 总计 54→55）；SURFACE_SPEC §29.9.3 条目已同步；CH-02 页脚验收链标注 D2。
- 状态：🟡 DRAFT（0.5D 原型），待与 D3-D6 联调后 LOCK。

### D3 ✅ CD-01 Channel Control Workspace v2（已建 + 已注册）
- 文件：`operator/CD-01-channel-workspace-v2.html`（运行态反射：Provenance 条 + 7 Profile 引用快照 + Output criticality 升级 + 源冗余(srcP/srcB 来自模板) + 反向追溯 D2）。
- 覆盖：本页把 D1/D2 产出的 Template→Bundle→Channel 在运行态反射：① 顶部 Provenance 条显示源自 Template Rev + Profile Bundle 快照(immutable, 不回灌); ② Profile Bundle 7 Profile 引用(P-21~P-27)与 D1 第④步 / D2 模板默认引用一致; ③ Output Variants 带 delivery_criticality (REQUIRED/OPTIONAL/AUX) 与 D1 第⑤步口径一致, 可无限添加; ④ 源冗余 PRIMARY=srcP / BACKUP=srcB 来自模板默认; ⑤ 反向追溯链接到 D2 (Used By)。
- 缺口标注（沿用 D1/D2 口径）：G3 基带 SDI 输出变体缺失 / G4 Output Resilience 未建模 / G5 源预览端点缺失 / G9 Take/Create 口径 / G10 音频映射/权限/告警。
- 注册：升级既有 CD-01 (0.5F LOCK), **不新增 surface、不计入 NAVIGATION/SURFACE_SPEC 计数**; CD-01 行注 v2 原型。NAVIGATION §2.5 计数维持 D2 末值 (BROADCAST 14 / 域合计 53 / TOTAL 54 / 总计 55)。
- 状态：🟡 DRAFT（0.5D 原型），待与 D4-D6 联调后随 CD-01 一同评估 LOCK。

### D4 ✅ M-17 Realtime Encode v2（已建 + 已注册）
- 文件：`operator/M-17-realtime-transcode-v2.html`（运行态反射：Provenance 条 + 7 Profile 引用(REALTIME 高亮) + 3-Layer 配置态绑定 P-22 + Pipeline/指标/健康检查(沿用 M-17 0.5D LOCK) + Backup Output retry 标 G4 触点）。
- 覆盖：① 顶部 Provenance 条显示本 RT Encoder Session 属于 Channel CH01 (源自模板 CH01-News-Live Rev v3 → Bundle bundle-news-01)，Realtime Profile (P-22) 为当前激活 Profile；② Profile Bundle 7 Profile 引用(P-21~P-27) 与 D1④ / D2 / D3 一致，REALTIME 高亮；③ 3-Layer 配置态(DESIRED=P-22 rev → COMPILED → EFFECTIVE) 绑定 Realtime Profile，修改须经 ChangeSet 升 rev，不污染模板默认；④ Pipeline Source→Normalize→Encode→Output + 实时指标 + H1-H7 健康检查（沿用 M-17）；⑤ Backup Output retry 3x backoff 1s 标为 G4 触点；⑥ 反向追溯链接 D3(CD-01 v2) / D2(模板)。
- 缺口标注（沿用 D1/D2/D3 口径）：G4 Output Resilience 在 M-17 的 Backup Output retry 硬编码，但无独立 OutputResilience 配置对象（决策留 06-output 而非 M-17）→ 落 D6；G9 Take/Create 口径；G10 Rights 地域/音频映射。
- 注册：升级既有 M-17 (0.5D LOCK)，**不新增 surface、不计入 NAVIGATION/SURFACE_SPEC 计数**；M-17 行注 v2 原型。§2.5 计数维持 D2 末值 (BROADCAST 14 / 域合计 53 / TOTAL 54 / 总计 55)。
- 状态：🟡 DRAFT（0.5D 原型），待与 D5-D6 联调后随 M-17 一同评估。

### D5 ✅ E-40 Network Source Wizard + E-42 Test Bench（已建 + 已注册 · 闭合 G2/G5）
- 文件：`operator/E-40-network-source-v2.html`（创建向导：Adapter/Endpoint/Security 8 字段 + 生命周期 DRAFT→E-42→VERIFIED→D1 ASSIGN + 链接 E-42）；`operator/E-42-source-test-bench.html`（7 层验证台 wireframe：Network/Transport/Container/Video/Audio/Clock/QC + 实时预览 + VERIFIED/FAILED 判定）。
- 覆盖（**本链首次真正补模型而非仅反射**）：① 闭合 **G2 (P0)**：源不在频道向导(D1 第②步)内联创建；在 E-40 独立创建后经 E-42 7 层验证为 VERIFIED 才进 VERIFIED 池，供 D1 第②步 ASSIGN 为 PRIMARY/BACKUP；模板(D2)默认源同理须指向 VERIFIED 源；② 闭合 **G5 (P1)**：视频缩略流 + 音频 LUFS/RMS 预览端点定义在 E-42（Source Runtime Preview Stream），D1 第③步/D3 PVW 复用同一端点，不重复定义；③ 验证台单层 FAIL → FAILED，不可存 VERIFIED / 不可用于 ON AIR（呼应 E-40 CRITICAL 不可存 VERIFIED），仅可存 UNVERIFIED/修复/重测；④ 反向追溯链接 D1 / E-40 / E-42 互相印证。
- 缺口标注（沿用 D1/D2/D3/D4 口径）：G5 的子项「音频 16ch→输出布局映射」并入 G10（D3/D4）；G3 基带 SDI 输出变体 / G4 Output Resilience / G6 端点拓扑 / G8 变更门禁 不在 D5，落 D6。
- 注册：E-40 (0.5F LOCK)、E-42 (Spec-only 表面, 0.5G 实施) 均为既有 surface；本次 E-42 补 wireframe、E-40 补 v2 创建闭环，**不新增 surface、不计入 NAVIGATION/SURFACE_SPEC 计数**。E-40/E-42 行注 D5 原型。§2.5 计数维持 D2 末值 (BROADCAST 14 / 域合计 53 / TOTAL 54 / 总计 55)。
- 状态：🟡 DRAFT（0.5D 原型），待与 D6 联调后随 E-40/E-42 一同评估。

### D6 ✅ B-13 Take Preflight v2（已建 + 已注册 · 闭合 G4/G6/G8 · 验收链收尾）
- 文件：`operator/B-13-take-preflight-v2.html`（9 项联合预检面板 + Output Resilience 对象(G4) + Reservation/Quota 对象(G8) + ChangeSet 审批闭环(G6) + CANCEL/TAKE 决策）。
- 覆盖（**本链收尾, 闭合最后三个缺口**）：① 9 项联合检查 (Spec §1): Source/Video/Audio/Clock/Switch/Backup/Output/Latency/Resource, 全 PASS 才放 TAKE, 对齐 Failure Domain Matrix (Output 坏不误切源); ② **G4 (P0) 闭合**: 建模独立 OutputResilience 子对象 (P-28 Bundle 子对象 / 06-output) — 每 REQUIRED Output 带 retry 3x·指数退避 1s / heartbeat 5s / zombie &gt;30s / Test Send 联动, 决策落 06-output 而非 M-17 (呼应 D3/D4 标注); ③ **G8 (P1) 闭合**: 显式 Reservation/Quota 对象 + HOT 独占扣减/释放时机 + 跨 Channel 仲裁, 与 REALTIME_PROFILE.resource_reservation=REQUIRED 一致; ④ **G6 (P0) 闭合**: 9 PASS → 提交 ChangeSet (E-33) 带 L2 Review/Approve/回滚闭环 (原子提交+审阅), 原 E-33 仅结构缺审批界面; ⑤ 反向追溯链接 D1-D5 (CD-01 TAKE 触发 / E-42 VERIFIED 源 / D1 输出 criticality / D4 编码资源)。
- 缺口标注（链末收口）: G3 基带 SDI 输出变体仍待 06-output 升级 (B-13 #7 已显示 SDI REQUIRED, 但 06-output 缺该变体) / G7 时钟域联动校验 (E-37) / G9 Take/Create 口径已在 D1 文案统一 / G10 音频映射 (D3/D4) / G11 命名约束 (D1) — 均不在 D6 范围, 延续既有标注。G6 若需独立审阅 surface, 可拆 D7 ChangeSet Review (见 F.4 建议), 本链 D6 已内联闭合。
- 注册：B-13 为既有 Spec-only 表面 (0.5G 实施); 本次补其 wireframe, **不新增 surface、不计入 NAVIGATION/SURFACE_SPEC 计数**。B-13 行注 D6 原型。§2.5 计数维持 D2 末值 (BROADCAST 14 / 域合计 53 / TOTAL 54 / 总计 55)。
- 状态：🟡 DRAFT（0.5D 原型）。**验收链 D1→D2→D3→D4→D5→D6 全链完成**。

### 验收链总览（D1-D6 全完成）
- D1 CH-02 创建向导 (Template≠Bundle 落地, 虚拟编排去"信号源") ✅
- D2 CH-02B 模板工厂 (Template→Bundle→Channel, 反向 Used By) ✅
- D3 CD-01 v2 (运行态反射 Template/Bundle/7 Profile/Output criticality) ✅
- D4 M-17 v2 (Realtime Profile P-22 激活 + 3-Layer 配置态) ✅
- D5 E-40/E-42 (闭合 G2 源不可内联创建 + G5 源预览端点) ✅
- D6 B-13 v2 (9 项预检 + 闭合 G4 Output Resilience + G8 Reservation + G6 ChangeSet 审批) ✅
- 缺口闭环: G2✅ G4✅ G5✅ G6✅ G8✅ (落 D5/D6); 余 G3(06-output) G7(E-37) G9(D1 已统一) G10(D3/D4) G11(D1) 延续标注。建议后续: 06-output 升级落 G3/G4、E-37 落 G7、可选 D7 ChangeSet Review。

---

## F. 设计缺口复盘（用户 4 点迭代 + 深挖 · 2026-08-25）

> 用户就 D1 (CH-02) 提出 4 点迭代（字号切换 / 源可选跳转 / 编码预览 / 输出基带+韧性），并要求"结合当前项目复盘、深挖、找问题"。以下为对 **Create Channel 验收链 + 整个 phase-0.5 模型** 的缺口盘点。严重程度 P0=阻塞验收 / P1=体验或一致性 / P2=增强。
> 迭代落点：G1 已在 D1 修复并级联全 22 个 operator 原型（精致/标准字号切换，默认标准=125%）。

### F.1 已修复（本轮）
- **G1 (P1) 字号可读性**：所有 operator 原型新增"精致/标准"浮层切换（标准=125% zoom，localStorage 记忆）。CH-02 头部内置切换，其余 21 个文件 `</body>` 前注入统一脚本。

### F.2 缺口清单（深挖）
- **G2 (P0) 信号源不能就地创建/选择**：向导第②步只能"选"已有 VERIFIED 源，不能"新建并验证"。内联新建源应在 D5/E-40 提供，经 E-42 7 层验证后回向导 ASSIGN。→ 落 D5。
- **G3 (P0) 基带输出（SDI OUT）缺失**：06-output 当前仅有 UDP OUTPUT + Failure Recovery，**无 SDI 基带输出变体**；Channel 输出模型应含 SDI OUT（无网络依赖、故障不切节目源）。→ 落 06-output 升级 / D6。
- **G4 (P0) 网络输出韧性未建模**：Health Check 间隔 / 重连策略(指数退避·次数) / 心跳 在 B-13 §8.9 仅 Failure Recovery 概念，**无独立 OutputResilience 配置对象**；重连阈值、心跳间隔、zombie 输出判定、Test Send 与运行态监控联动均未定义。→ 落 P-28 Bundle 子对象 / D6 / 06-output。
- **G5 (P1) 信号源预览缺失**：第③步应有视频缩略流 + 音频 LUFS/RMS 实时预览，但 Source Runtime 未定义预览流端点（并入 D5/E-42）；音频 16ch→输出布局映射也未在向导体现。→ 落 D5 + D3。
- **G6 (P0) ChangeSet 无审批界面**：E-33 只定义结构，生成后缺 Review/Approve/回滚 surface，原子提交无审阅闭环。→ 落 D6 或独立 D7 (ChangeSet Review)。
- **G7 (P1) 时钟域未与源类型联动校验**：PTP 要求基带源、NTP 适用网络源；向导选 Clock 不校验源类型。→ 落 D1 校验 + E-37。
- **G8 (P1) Reservation 对象缺失**：资源预览仅数值，无显式 Reservation/Quota 对象与跨 Channel 仲裁；HOT 独占扣减/释放时机未建模。→ 落 D6 / B-13。
- **G9 (P1) Take vs Create 概念混淆**：原第⑥步叫"Take Preflight"，实际向导只到 STARTING；TAKE 在 CD-01 的 READY_TO_TAKE 阶段。已在 D1 文案改为"创建前预检"，需全局口径统一（D3/D6）。
- **G10 (P2) 输出音频映射 / 权限 / 监控告警**：编码 7 Profile 含 Rights，但向导未暴露窗口/地域；无告警端点；多输出音频布局映射缺失。→ 落 D3/D4。
- **G11 (P2) 规模与命名约束**：Channel 名唯一性、Workspace 作用域校验、单 Channel 最大源/输出数未定义。→ 落 D1 校验 / OBJECT_VOCAB。

### F.3 缺口落点映射
| 缺口 | 严重 | 落点 |
|---|---|---|
| G2 源内联创建 | P0 | D5 (E-40 源向导) |
| G3 基带输出 | P0 | 06-output 升级 / D6 |
| G4 输出韧性对象 | P0 | P-28 / D6 / 06-output |
| G6 ChangeSet 审批 | P0 | D6 或 **D7 ChangeSet Review** |
| G5 源预览 | P1 | D5 + D3 |
| G7 时钟域校验 | P1 | D1 校验 + E-37 |
| G8 Reservation 对象 | P1 | D6 / B-13 |
| G9 Take/Create 口径 | P1 | D3 / D6 全局统一 |
| G10 音频映射/权限/告警 | P2 | D3 / D4 |
| G11 规模/命名约束 | P2 | D1 / OBJECT_VOCAB |

### F.4 结论
- 模型骨架（7 Profile / Template≠Bundle / Failure Domain / Output Criticality / Resource 三档）稳健；**缺口集中在"闭环动作对象"缺失**：源内联创建、基带输出、输出韧性、ChangeSet 审批、Reservation 五处是阻断 0.5 冻结的真实 P0。
- 建议在 D2-D6 基础上**追加 D7（ChangeSet Review）**，并把 G3/G4 明确并入 06-output 升级工作单。D1 已用橙色"设计缺口"块在 wireframe 内联标注 G2/G3/G4/G6/G8，便于评审时对照。
