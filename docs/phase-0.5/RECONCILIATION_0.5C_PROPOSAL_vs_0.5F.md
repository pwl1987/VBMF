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
