# 0.5C 提案 vs 本地 0.5F 逐条对账报告

> 目的：用户基于 GitHub master 旧结构（phase-0.5b 并存、视角停在 0.5A/0.5B）提出长篇
> "Phase 0.5C — Channel / Source / Encode / Network UX Closure" 提案（共 33 节）。
> 本报告将 33 节逐条与本地工作区已落地的 **Phase 0.5F LOCK FINAL** 对照，确认是否有遗漏。
> 结论：**提案绝大部分已被 0.5F 覆盖，仅少数点需做增量，无需从零重写 PIA。**

图例：✅ 已锁 / ⚠ 部分覆盖或待核对 / ❌ 明确缺口

> **⚠ CURRENT BASELINE (SoT) — 0.5D.1 起生效:** 本文件以下所有章节为 **HISTORY**（逐轮对账与 D1-D7 落地记录），当前状态以本节为准。任何自动化 / 人工审查**禁止**把历史章节的 "待补 / 未完成 / 仍待" 文字当作当前缺口。
>
> | 缺口 | 当前状态 | 关闭方式 (closed_by) |
> |---|---|---|
> | G-A UDP Output 对称面 | ✅ CLOSED | G-A 轮 (06-output UDP OUTPUT) |
> | G-B Output Failure Guided Recovery | ✅ CLOSED | G-A 轮 (06-output FAILURE RECOVERY) |
> | G-C Source→Channel 主动 Assign | ✅ CLOSED | G-C 轮 (02-sources ASSIGN TO CHANNEL) |
> | G-D 3 Spec 登记 | ✅ CLOSED | G-D 轮 (SURFACE_SPEC §29.9.3b Batch 4) |
> | G1 Encoding Profile 双语义 | ✅ CLOSED | ENCODE_MODEL_SPEC.md |
> | G2 Source 内联创建 | ✅ CLOSED | D5 (E-40 v2 + E-42) |
> | G3 基带 SDI 输出变体 | ✅ CLOSED | 0.5D.1 (06-output RESERVED, commit `6367cd8`) |
> | G4 Output Resilience 对象 | ✅ CLOSED | D6 (B-13 v2) + 06-output, 0.5D.1 收口 |
> | G5 源预览端点 | ✅ CLOSED | D5 (E-42 Source Runtime Preview Stream) |
> | G6 ChangeSet 审批界面 | ✅ CLOSED | D7 (ChangeSet Review 独立 surface) |
> | G7 时钟域联动校验 | ✅ CLOSED | 0.5D.1 (E-37, commit `6367cd8`) |
> | G8 Reservation / Quota | ✅ CLOSED | D6 + 0.5D.1 (RESOURCE_RESERVATION_SPEC.md) |
> | G9 Take/Create 口径 | ✅ CLOSED | D1 文案统一 |
> | G10 音频映射 / 权限 / 告警 | 🟡 PARTIAL | D3/D4 部分落, 余项延续 (非冻结阻断) |
> | G11 命名约束 | ✅ CLOSED | D1 命名约束 |
> | P0-1..P0-9 第三轮一致性 | ✅ CLOSED | 第三轮采纳 + 0.5D 实施 |
> | SDI Master Output ACTIVE 回退 | ✅ CLOSED | 0.5D.1 (commit `6367cd8`) |
> | Profile 6/7 残留 | ✅ CLOSED | 0.5D.1 全仓焊死 (commit `6367cd8`) |
> | 页面计数多源打架 | ✅ CLOSED | 0.5D.1 SURFACE_REGISTRY.yaml (commit `6367cd8`) |
> | ChangeSet 状态混用 | ✅ CLOSED | 0.5D.1 三层状态 (commit `6367cd8`) |
>
> **SoT 链:** `OBJECT_VOCABULARY.md` (15 对象) ← `SURFACE_REGISTRY.yaml` (53 表面 = 52 wireframe + 1 Spec E-41) ← `NAVIGATION.md` §2.5 ← `PIA` ← 本表。

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
| 17 | 模板一键自动带出 7 个 Profile | ✅ | P-28 |
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
| 13 | Profile Bundle / Channel Template（一键带出 7 Profile） | ✅ | `P-28-profile-bundle.html` + PIA §12 |
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
- 文件：`operator/CH-02-create-channel.html`（7 步向导：① 模板&基础 ② 信号源 ③ 节目单(Virtual) ④ 编码&音频 ⑤ 输出 ⑥ 资源预览 ⑦ 预检&提交）。
- 覆盖：Channel Template 工厂（不进运行态，见 OBJECT_VOCAB §1.5）、SDI Primary + UDP-Multicast Backup（E-40 双路径）、Source→Channel Assign、E-42 7 层入网验证、Realtime Encode 7 Profile、Audio Quick Adjust、Output delivery_criticality 分级、Resource 三档预览、B-13 9 项联合预检、ChangeSet（E-33）生成。
- 注册：NAVIGATION BROADCAST 列表 + §2.5（BROADCAST 12→13 / 域合计 51→52 / TOTAL 52→53 / 总计 53→54）；SURFACE_SPEC 新增 §29.9.5 Batch 5 + BROADCAST 行 12→13 + TOTAL 行标历史/权威。
- 状态：🟡 DRAFT（0.5D 原型），待与 D2-D6 联调后 LOCK。

### D2 ✅ CH-02B Channel Template Center（已建 + 已注册）
- 文件：`operator/CH-02b-channel-template-center.html`（模板注册表 + 模板详情（默认 7 Profile 引用 / 默认源 / 默认输出）+ 从零创建表单 + 6 状态样例）。
- 覆盖：Channel Template = 创建工厂（**不进运行态**），实例化出 Profile Bundle（7 Profile 引用，P-21/22/23/24/25/26/27）+ Channel(DRAFT)；Template≠Bundle≠Profile≠Variant 层级明示；覆盖 TV_LIVE / RADIO_LIVE / VIRTUAL_PLAYOUT 三类；模板卡可 Clone / Deprecate / Use→CH-02；创建表单动态生成 7 Profile 引用 + 默认输出 Variants。
- 缺口标注（沿用 D1 口径）：G2 默认源内联创建待定 / G3 基带 SDI 输出待补 / G4 Output Resilience 待补。`state: historical: OPEN@D2 · current: CLOSED · closed_by: D5(G2) + 0.5D.1 6367cd8(G3/G4)`
- 注册：NAVIGATION BROADCAST 列表 + §2.5（BROADCAST 13→14 / 域合计 52→53 / TOTAL 53→54 / 总计 54→55）；SURFACE_SPEC §29.9.3 条目已同步；CH-02 页脚验收链标注 D2。
- 状态：🟡 DRAFT（0.5D 原型），待与 D3-D6 联调后 LOCK。

### D3 ✅ CD-01 Channel Control Workspace（已建 + 已注册 · 0.5D.3 并入正页）
- 文件：`operator/CD-01-channel-workspace.html`（运行态反射：Provenance 条 + 7 Profile 引用快照 + Output criticality 升级 + 源冗余(srcP/srcB 来自模板) + 反向追溯 D2）。
- 覆盖：本页把 D1/D2 产出的 Template→Bundle→Channel 在运行态反射：① 顶部 Provenance 条显示源自 Template Rev + Profile Bundle 快照(immutable, 不回灌); ② Profile Bundle 7 Profile 引用(P-21~P-27)与 D1 第④步 / D2 模板默认引用一致; ③ Output Variants 带 delivery_criticality (REQUIRED/OPTIONAL/AUX) 与 D1 第⑤步口径一致, 可无限添加; ④ 源冗余 PRIMARY=srcP / BACKUP=srcB 来自模板默认; ⑤ 反向追溯链接到 D2 (Used By)。
- 缺口标注（沿用 D1/D2 口径）：G3 基带 SDI 输出变体缺失 / G4 Output Resilience 未建模 / G5 源预览端点缺失 / G9 Take/Create 口径 / G10 音频映射/权限/告警。`state: historical: OPEN@D3 · current: CLOSED · closed_by: D5(G5) + 0.5D.1 6367cd8(G3/G4)`
- 注册：升级既有 CD-01 (0.5F LOCK), **不新增 surface、不计入 NAVIGATION/SURFACE_SPEC 计数**; CD-01 行注 v2 原型。NAVIGATION §2.5 计数维持 D2 末值 (BROADCAST 14 / 域合计 53 / TOTAL 54 / 总计 55)。
- 状态：🟡 DRAFT（0.5D 原型），待与 D4-D6 联调后随 CD-01 一同评估 LOCK。

### D4 ✅ M-17 Realtime Encode（已建 + 已注册 · 0.5D.3 并入正页 + 修正 P-21/REALTIME_PROFILE）
- 文件：`operator/M-17-realtime-transcode.html`（运行态反射：Provenance 条 + 7 Profile 引用(P-21 REALTIME_PROFILE 高亮) + 3-Layer 配置态绑定 P-21 + Pipeline/指标/健康检查(沿用 M-17 0.5D LOCK) + Backup Output retry 标 G4 触点）。
- 覆盖：① 顶部 Provenance 条显示本 RT Encoder Session 属于 Channel CH01 (源自模板 CH01-News-Live Rev T-v3 → Bundle bundle-news-01)，Encoding Profile (P-21) · profile_type=REALTIME_PROFILE 为当前激活 Profile（0.5D.3 修正：原误标 P-22）；② Profile Bundle 7 Profile 引用(P-21~P-27) 与 D1④ / D2 / D3 一致，P-21 高亮；③ 3-Layer 配置态(DESIRED=P-21 ENC-v3 → COMPILED → EFFECTIVE) 绑定 Encoding Profile REALTIME_PROFILE，修改须经 ChangeSet 升 rev，不污染模板默认；④ Pipeline Source→Normalize→Encode→Output + 实时指标 + H1-H7 健康检查（沿用 M-17）；⑤ Backup Output retry 3x backoff 1s 标为 G4 触点；⑥ 反向追溯链接 D3(CD-01) / D2(模板)。
- 缺口标注（沿用 D1/D2/D3 口径）：G4 Output Resilience 在 M-17 的 Backup Output retry 硬编码，但无独立 OutputResilience 配置对象（决策留 06-output 而非 M-17）→ 落 D6；G9 Take/Create 口径；G10 Rights 地域/音频映射。`state: historical: OPEN@D4 · current: CLOSED · closed_by: D6(G4) + 0.5D.1 6367cd8`
- 注册：升级既有 M-17 (0.5D LOCK)，**不新增 surface、不计入 NAVIGATION/SURFACE_SPEC 计数**；M-17 行注 v2 原型。§2.5 计数维持 D2 末值 (BROADCAST 14 / 域合计 53 / TOTAL 54 / 总计 55)。
- 状态：🟡 DRAFT（0.5D 原型），待与 D5-D6 联调后随 M-17 一同评估。

### D5 ✅ E-40 Network Source Wizard + E-42 Test Bench（已建 + 已注册 · 闭合 G2/G5）
- 文件：`operator/E-40-network-source.html`（创建向导：Adapter/Endpoint/Security 8 字段 + 生命周期 DRAFT→E-42→VERIFIED→D1 ASSIGN + 链接 E-42）；`operator/E-42-source-test-bench.html`（7 层验证台 wireframe：Network/Transport/Container/Video/Audio/Clock/QC + 实时预览 + VERIFIED/FAILED 判定）。
- 覆盖（**本链首次真正补模型而非仅反射**）：① 闭合 **G2 (P0)**：源不在频道向导(D1 第②步)内联创建；在 E-40 独立创建后经 E-42 7 层验证为 VERIFIED 才进 VERIFIED 池，供 D1 第②步 ASSIGN 为 PRIMARY/BACKUP；模板(D2)默认源同理须指向 VERIFIED 源；② 闭合 **G5 (P1)**：视频缩略流 + 音频 LUFS/RMS 预览端点定义在 E-42（Source Runtime Preview Stream），D1 第③步/D3 PVW 复用同一端点，不重复定义；③ 验证台单层 FAIL → FAILED，不可存 VERIFIED / 不可用于 ON AIR（呼应 E-40 CRITICAL 不可存 VERIFIED），仅可存 UNVERIFIED/修复/重测；④ 反向追溯链接 D1 / E-40 / E-42 互相印证。
- 缺口标注（沿用 D1/D2/D3/D4 口径）：G5 的子项「音频 16ch→输出布局映射」并入 G10（D3/D4）；G3 基带 SDI 输出变体 / G4 Output Resilience / G6 端点拓扑 / G8 变更门禁 不在 D5，落 D6。`state: historical: OPEN@D5 · current: CLOSED · closed_by: D6(G4/G6/G8) + 0.5D.1 6367cd8(G3/G7)`
- 注册：E-40 (0.5F LOCK)、E-42 (Spec-only 表面, 0.5G 实施) 均为既有 surface；本次 E-42 补 wireframe、E-40 补 v2 创建闭环，**不新增 surface、不计入 NAVIGATION/SURFACE_SPEC 计数**。E-40/E-42 行注 D5 原型。§2.5 计数维持 D2 末值 (BROADCAST 14 / 域合计 53 / TOTAL 54 / 总计 55)。
- 状态：🟡 DRAFT（0.5D 原型），待与 D6 联调后随 E-40/E-42 一同评估。

### D6 ✅ B-13 Take Preflight（已建 + 已注册 · 闭合 G4/G6/G8 · 验收链收尾）
- 文件：`operator/B-13-take-preflight.html`（9 项联合预检面板 (Config/Runtime 时态分离 + 结果闭集 READY/CONDITIONAL/BLOCKED) + Output Resilience 对象(G4) + Reservation/Quota 对象(G8) + ChangeSet 审批闭环(G6) + CANCEL/TAKE 决策）。
- 覆盖（**本链收尾, 闭合最后三个缺口**）：① 9 项联合检查 (Spec §1): Source/Video/Audio/Clock/Switch/Backup/Output/Latency/Resource, 全 PASS 才放 TAKE, 对齐 Failure Domain Matrix (Output 坏不误切源); ② **G4 (P0) 闭合**: 建模独立 OutputResilience 子对象 (P-28 Bundle 子对象 / 06-output) — 每 REQUIRED Output 带 retry 3x·指数退避 1s / heartbeat 5s / zombie &gt;30s / Test Send 联动, 决策落 06-output 而非 M-17 (呼应 D3/D4 标注); ③ **G8 (P1) 闭合**: 显式 Reservation/Quota 对象 + HOT 独占扣减/释放时机 + 跨 Channel 仲裁, 与 REALTIME_PROFILE.resource_reservation=REQUIRED 一致; ④ **G6 (P0) 闭合**: 9 PASS → 提交 ChangeSet (E-33) 带 L2 Review/Approve/回滚闭环 (原子提交+审阅), 原 E-33 仅结构缺审批界面; ⑤ 反向追溯链接 D1-D5 (CD-01 TAKE 触发 / E-42 VERIFIED 源 / D1 输出 criticality / D4 编码资源)。
- 缺口标注（链末收口）: G3 基带 SDI 输出变体仍待 06-output 升级 (B-13 #7 已显示 SDI REQUIRED, 但 06-output 缺该变体) / G7 时钟域联动校验 (E-37) / G9 Take/Create 口径已在 D1 文案统一 / G10 音频映射 (D3/D4) / G11 命名约束 (D1) — 均不在 D6 范围, 延续既有标注。G6 若需独立审阅 surface, 可拆 D7 ChangeSet Review (见 F.4 建议), 本链 D6 已内联闭合。`state: historical: OPEN@D6 · current: G3/G7 CLOSED (0.5D.1 6367cd8), G10 PARTIAL`
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

---

## G. 0.5D 后续升级闭环 (2026-08-25 末 · 用户"全做")

> 用户确认执行前序 F.4 建议的全部三项后续: ① 06-output 升级落 G3/G4; ② E-37 落 G7; ③ 新建 D7 ChangeSet Review 独立审批 surface。

### G.1 G3 (P0) 基带 SDI 输出变体 — 06-output.html
- 原 `CH01-SDI-Master` 仅标 `DISABLED (V0.4 Target)`; 现建模为**基带 SDI 输出变体** (BNC 12G-SDI / 3G-SDI 1080p50 / 16ch AES 嵌入音频 / Embedded Timecode / PTP 帧同步), 状态 **BASEBAND·RESERVED (V0.2 Implementation DISABLED · Target V0.4)** — 0.5D.1 修正: 禁止运行态 ACTIVE (架构边界回退修复, 见 §H.1)。
- 新增 🎞 SDI OUTPUT Tab, 强调"无网络依赖、故障不切节目源 (Failure Domain 输出适配器级)"。

### G.2 G4 (P0) Output Resilience 独立对象 — 06-output.html
- 原仅 Failure Recovery 概念, 无独立配置对象; 现每 REQUIRED Output Variant 携带 **OutputResilience 子对象** (P-28 Bundle 子对象 / 06-output): retry 3x·exp-backoff 1s / heartbeat 5s / zombie >30s / test-send 联动。
- 新增 🛟 OUTPUT RESILIENCE Tab (HLS/RTMP/SDI/UDP-MC 四行 criticality + 韧性参数), 决策落在 06-output 而非 M-17 (呼应 D3/D4)。

### G.3 G7 (P1) 时钟域联动校验 — E-37-clock.html
- 新增 🔗 时钟域联动校验 面板: 基带 SDI 源须 PTP 域 (BROADCAST_GRADE/GOOD), 网络源 (UDP/RTMP/RTP/SRT) 时钟域须与源类型匹配; 选 Clock 时联动校验所引用源, 不匹配 (如 Reference 退化为 NTP-only) → 阻断该 Channel TAKE。
- 落点: D1 创建向导选 Clock 校验 + E-37 (F.2 G7 原定落点)。

### G.4 D7 (P0) ChangeSet Review 独立审批 surface — 新建 operator/D7-changeset-review.html
- 从 B-13 (D6) 内联 L2 审批拆出**独立审阅面**: ChangeSet 队列 (Pending/Approved/Rejected/Applied) + Before/After Diff + Risk Assessment + Reviewer 指派 + Approve/Reject/Rollback + Audit Trail。
- 与 E-33 (Business Status) / B-13 (提交) / A-54 (Audit hash chain) 共享对象; G6 闭环 (E-33 原缺审批界面) 由此 surface 收口。
- **新 surface, 计入 NAVIGATION/SURFACE_SPEC 计数**: ENGINEERING 26→27 · 域合计 53→54 · TOTAL 54→55 · 总计 55→56。

### G.5 缺口闭环总账
- 本轮闭合: G3✅ G4✅ G7✅ G6✅ (D7 收口) — 余 G9(D1 已统一) G10(D3/D4 部分) G11(D1) 延续标注。
- 至此 F.2 全部 11 项缺口均有落点, 其中 10 项已闭 (G1 D1 / G2 D5 / G3 06-output / G4 06-output / G5 D5+D3 / G6 D7 / G7 E-37 / G8 D6 / G9 D1 / G11 D1), G10 部分落 D3/D4 延续。

---

## H. 0.5D.1 Semantic Consistency Closure (2026-08-25 末 · 用户第 4 轮检修 e164c826)

> 用户检修结论: 不能宣布 Phase 0.5 LOCK FINAL — 问题是**语义/边界回退**而非缺页面。风险排序: SDI Master Output ACTIVE (越界回退) > 7 Profile 残留 > Template 未正式对象化 > 页面计数打架 > Reservation 未建模 > ChangeSet 状态混用。本轮只做 6 项语义焊死, 不再加页面。

### H.1 SDI Master Output 回 RESERVED (P0-1, 架构边界守卫)
- `06-output.html`: `CH01-SDI-Master` 由 BASEBAND·ACTIVE → **BASEBAND·RESERVED (V0.2 Implementation DISABLED · Target V0.4)**; Enable / Test Send 禁用, 仅"查看预留 Schema"。
- 同步修正: CH-02 (D1) step5 SDI 输出 RESERVED + 预检移除 SDI; CD-01 v2 (D3) 输出区 SDI RESERVED + 预检移除; B-13 v2 (D6) Output Resilience SDI 行 RESERVED; D7 Diff 示例 SDI RESERVED。
- 原则锁定: `Architecture Contract RESERVED → UI 可预览 → Configuration 可预留 → Runtime Implementation DISABLED → Runtime State 禁止 ACTIVE`。

### H.2 Profile 7/7 全仓焊死 (P0-2)
- 全仓清扫 "6 Profile / 6 种子类 / 6 个引用" 残留: OBJECT_VOCABULARY (§1.3, §2), POM (§1.2/§4/§5/§6), 0.5E, SURFACE_SPEC, RECONCILIATION, PIA, MILESTONES, P-20 html, CH-01 html。
- invariant 唯一: **Profile = 7, Bundle = 7 refs** (Encoding/Audio/Output/Graphic/QC/Rights/Edge)。

### H.3 Channel Template 正式对象 (P0-6)
- OBJECT_VOCABULARY 新增 §1.15: `CHANNEL_TEMPLATE` (kind / DB 表 / Revision / Used By / Instantiate), 核心对象 14→15。Template 为创建工厂, 不进运行态; 模板默认 criticality 只影响新实例化, 不回灌已在播 Channel (D3 Bundle 快照不变)。

### H.4 SURFACE_REGISTRY (P0-5, 计数单一事实源)
- 新增 `SURFACE_REGISTRY.yaml`: 53 表面 (52 wireframe + 1 Spec E-41) 逐条登记 (id/domain/kind/status/milestone)。
- 计数重排: M-17 规范归 MEDIA (BROADCAST 14→13); ENGINEERING 按注册表行数校正 (27→26, 含 E-41 SPEC + D7)。
- **最终权威数: BROADCAST 13 / MEDIA 8 / ENGINEERING 26 / ADMIN 5 = 域合计 52 · TOTAL 53 (52 wireframe + 1 Spec)**。README / NAVIGATION / MILESTONES / SURFACE_SPEC 一律引用 Registry, 禁止手写 22/39/44/52/54/55/56。

### H.5 Resource Reservation 语义焊死 (P1-3)
- 新增 `RESOURCE_RESERVATION_SPEC.md`: Reservation 对象 (reservation_id / target / resource_vector / scope HOT|WARM|COLD|TRANSIENT / priority / state / acquired_at / released_at) + 生命周期 PROVISIONED→RESERVED→IN_USE→RELEASED + Quota / 仲裁规则 + HOT 必须 RESERVED 才算真锁 + Preflight 三档联动 (B-13 第 8 项)。

### H.6 ChangeSet 三层状态 (P1-4)
- OBJECT_VOCABULARY §1.14 焊死三层: `ChangeSetStatus` (DRAFT/VALIDATED/APPROVED/SCHEDULED/APPLIED/ROLLED_BACK/ABORTED) · `ReviewState` (NOT_REQUIRED/PENDING/APPROVED/REJECTED) · `TransactionPhase` (PREPARING/APPLYING/COMMITTED/ABORTED)。`phase` 只描述事务执行, `status` 不再含 APPLYING。

### H.7 0.5D.1 状态判断
- 本轮无新增 surface (D1-D7 已在前序); 修复: SDI 边界回退 / Profile 计数 / Template 对象 / Registry / Reservation / ChangeSet。
- 后续 (非本轮): P1-1 Source/Endpoint/Adapter 边界、E-40 Media Contract 屏 (十三)、Network/Media Path 双视图组件 (十四)、Duplicate Channel (十九)、Host/Device Capacity 视图 (二十)、E-40→E-42 数据契约显式化。

---

## I. 0.5D.2 / 0.5E Closure — SoT 回写 + 状态/工作流闭环 (2026-08-25 末 · 用户第 5 轮检修 6367cd8)

> 用户检修结论: 已非常接近 Phase 0.5 FINAL, 但**规范层尾部不一致** ("新模型已修, 旧文档/旧 UI 仍留旧语义") 仍不能冻结。本轮不加页面, 只做 SoT 回写 + 状态/工作流 Closure。

### I.1 P0-1 全仓旧计数 / 对象数残留清零
- 14→15 对象: PIA (3 处) / 0.5E (2 处) / SURFACE_SPEC (5 处) / E-41 / MILESTONES / milestones / POM / phase-0.5 README 全部改 15。
- 48→53 表面: 0.5E Navigate / Command Palette / 验证清单 (4 处) + SURFACE_SPEC ROADMAP 改注册表口径。
- 6→7 Profile: NAVIGATION ENGINEERING 域表 Profile (6)→(7)。
- 说明: Job "6 子类" (FILE_TRANSCODE/REALTIME_ENCODE/PROBE/QC/UPLOAD/ARCHIVE) 为正确, 未动。

### I.2 P0-2 RECONCILIATION 切 Current Baseline
- 文件顶部新增 **CURRENT BASELINE (SoT)** 块: G-A~G-D / G1-G11 / P0-1..P0-9 / SDI 回退 / Profile 残留 / 计数多源 / ChangeSet 混用 全部登记当前状态 + closed_by。下方所有章节标注 HISTORY, 禁止把历史 "待补/未完成" 当当前缺口。
- D2-D6 历史缺口标注行内联 `state: historical / current / closed_by` 标记。

### I.3 P0-3 Preflight 时态分离 (B-13 v2)
- 9 项拆两区: **A · CONFIGURATION PREFLIGHT** (Video/Audio/Latency/Resource — "能部署吗") + **B · RUNTIME TAKE READINESS** (Source/Clock/Switch/Backup/Output — "现在能切吗")。
- 最终判断: CONFIGURATION = PASS · RUNTIME READINESS = PASS → **TAKE AVAILABLE**; 附四动作语义 (Configure ≠ Validate ≠ Apply ≠ Operate)。

### I.4 P1-1 Source Assignment Matrix (02-sources)
- ASSIGN TO CHANNEL 面板升级为矩阵视图: Source × Channel × Role × Priority × Standby × Verified × Preflight + [Change Role][Promote][Detach]。
- 关系语义焊死: Source→Channel 多对多, 均为 **Channel relationship 操作, 不修改 Source 自身配置** (Adapter/Endpoint/Contract 属 E-40/E-42)。

### I.5 P1-2 Reservation 对象化回写 (CH-02 step6)
- 消除矛盾: CH-02 资源预览原 "缺 Reservation/Quota 对象" 缺口卡 → 改为 **Reservation 对象卡** (state PROVISIONED→RESERVED / scope HOT / owner CH02 / expires session stop), 与 RESOURCE_RESERVATION_SPEC.md 一致, 设计缺口已关闭。

### I.6 附带回写
- D3 CD-01 v2 Provenance 条完整化: Template / **Instantiated at** / Bundle Snapshot / **Template Sync OFF · immutable snapshot** — 明确"实例化后与模板的关系"。
- B-13 v2 页脚 G3/G7 状态改为已闭 (0.5D.1)。

### I.7 剩余 (非冻结阻断, 0.5E/0.5G)
- E-40 Media Contract 屏 · Network/Media Path 双视图组件 · Duplicate Channel · Host/Device Capacity 视图 · E-40→E-42 数据契约显式化 · M-18 kind 差异化 UI · Context Command Palette · Dependency/Impact Workspace · 退役工作流统一 UX。

---

## J. 0.5D.3 Object/State/Execution Closure (2026-08-25 末 · 用户第 6 轮检修 eb3b021)

> 用户检修结论: 不能冻结 — 出现 **P-22 / Realtime Profile 对象类型错位** (最危险: Phase 1 可能建出 `realtime_profile_id → output_profiles` 错误 FK)。本轮 4 P0 + 4 P1, 不做新页面, 只焊对象/状态/执行链。

### J.1 P0-1 修正 P-21 / REALTIME_PROFILE / P-22 关系 (M-17)
- 权威链: `P-21 Encoding Profile { FILE_PROFILE / REALTIME_PROFILE } → M-17 REALTIME_PROFILE → MEDIA_SESSION`; `P-22 Output Profile → Output Variant → Destination → Adapter`。
- M-17 全部 "Realtime Profile (P-22)" → **Encoding Profile (P-21) · profile_type=REALTIME_PROFILE**; DESIRED = P-21 ENC-v3; 7 Profile chip 纠正 (P-21 Encoding active / P-22 Output 不激活 / P-23 Audio / P-24 Graphic / P-25 QC / P-26 Rights / P-27 Edge)。
- 附: M-17 边界注 "Encode Session ≠ Output" (Encoder → Program Master → Output Variant → Output Session/Adapter)。

### J.2 P0-2 P-21 增 profile_type UI
- P-21 (product/P-21-encoding-profile.html): 顶部 Profile Type 单选 (FILE_PROFILE / REALTIME_PROFILE, 创建后不可变); 旧 Latency Mode → **Latency Class (NORMAL/LOW/ULTRA_LOW)**; 新增 Realtime Only 专属区 (Startup Budget / Hot Standby / Resource Reservation / Failover Policy / Worker Constraints)。

### J.3 P0-3 Object Vocabulary ER: Adapter ≠ P-22
- ER 图 `ADAPTER (P-22)` → `ADAPTER (Runtime)`; 附注: Adapter 来自 Runtime / Capability Registry (E-34) / Device Registry (E-35/E-38), 四层边界 P-22 → Variant → Destination → Adapter。

### J.4 P0-4 B-13 #9 Resource 漏判 + 结果闭集
- 决策规则修复: 任一 hard blocker (含 #9 Resource >100%) → BLOCKED, 不再只写 #1–#8。
- 焊死闭集 **TakePreflightResult = READY / CONDITIONAL / BLOCKED** (READY=全PASS; CONDITIONAL=仅 WARNING+Reservation 满足+REQUIRED 全 PASS; BLOCKED=任意 hard blocker)。READY/CONDITIONAL → TAKE ENABLED。

### J.5 P1-5 Reservation 引用 V0.2 9-dim ResourceVector
- RESOURCE_RESERVATION_SPEC: `resource_vector` 改为 **= V0.2 §3.11 (9-dim: cpu_threads/gpu_sessions/vram_mb/ram_mb/ingress/egress/disk_write/pcie_rx/pcie_tx)** + `device_tokens` + `constraints`, 禁止另建简化模型; CH-02 卡同步。

### J.6 P1-6 抢占改 DRAIN/RELEASE
- 被抢占方不再直接 FAILED: **PREEMPT_PENDING → DRAINING → RELEASED**; 仅无法安全释放才 `RESOURCE_CONFLICT → Safety Decision (Degrade/Stop/Reject)`, 对齐 V0.2 §8.9 RESOURCE → Degrade background jobs。

### J.7 P1-7 清除 *-v2.html 并存
- v2 提升为正页, 删除 v1: `CD-01-channel-workspace.html` / `M-17-realtime-transcode.html` / `E-40-network-source.html` / `B-13-take-preflight.html`; 全仓 `-v2.html` 引用清零 (NAV/RECON/D7/页内链接/链描述)。原则: Git commit 表版本, 文件名表产品语义。

### J.8 P1-8 Apply → Provision 链显式化
- CH-02 提交后执行链: **ChangeSet Apply → PROVISION (Runtime Provision) → STARTING → RESERVED → READY_TO_TAKE → (TAKE) → RUNNING** (Apply ≠ Start)。Reservation Spec 数据流同步加 Apply/Provision。

### J.9 附带回写
- OBJECT_VOCABULARY §1.13 Revision 前缀约定 (T-v3 / B-v2 / ENC-v7 / OUT-v4 / RS-); POM §1.2 Bundle 权限 (Operator 只能选兼容 Revision + ChangeSet, Engineer 才能编辑 Profile Definition)。
- 剩余 (0.5E/0.5G): 同 §I.7 + E-40 Media Contract / Network-Media Path 双视图 / Duplicate Channel / Host/Device Capacity / Dependency/Impact Workspace。

---

## K. 0.5D.3b Closure — P-21/P-22 边界 + Reservation 时序 + EXECUTION_MODEL (2026-08-25 末 · 用户第 7 轮检修 64bacae)

> 用户检修结论: 已进入收口阶段, 不再发散加页面 — 但 P-22 仍在配 Codec/Bitrate (边界重新污染), Reservation Spec 状态仍是 DRAFT, 缺 EXECUTION_MODEL。本轮 3 P0 + 3 P1。

### K.1 P0-1 Canonical terminology residue 清零
- B-13 "匹配 Realtime Profile P-22" → **Encoding Profile (P-21) · REALTIME_PROFILE · ENC-v3**。全仓 `P-22=Realtime` 语义清零。

### K.2 P0-2 P-22 去 Encoding 参数 (继承 P-21 只读)
- P-22 移除 Codec / Bitrate / GOP 等字段: 4-Tuple 示例 / 3 张 Profile 卡 / 3-Layer DESIRED / HLS 表单 Codec 行 → **Encoding Constraint inherited from P-21 ENC-v3 (RO)**。
- Latency 只配置 **Delivery**; Channel E2E / Failover 改为 **Inherited Context 只读** (归属 Channel Latency Budget / Hot Standby Policy, 避免三处重复字段)。
- 原则: `P-21 = HOW TO ENCODE` · `P-22 = HOW TO DELIVER`。

### K.3 P0-3 P-21 profile_type Edit 态 immutable
- P-21: Create 可选 FILE/REALTIME; **Edit 态 REALTIME_PROFILE 🔒 immutable after creation (radio disabled)**, 禁止切换。

### K.4 P1-4 Reservation Spec 状态毕业
- `DRAFT 0.1` → **SEMANTIC LOCKED V0.2** (implementation_authority: 本 Spec · wireframe_status: TODO 0.5E), 不再使用 DRAFT 词。

### K.5 P1-5 TAKE 不触发资源抢占
- **TAKE 只验证 `reservation.state == RESERVED`**; 抢占仅发生在 PREPROVISION/RESERVE 阶段。未 RESERVED → TAKE BLOCKED (Reason: NOT_READY → Action: Open Resource Impact/Provision)。Reservation Spec §6.1 + B-13 决策面板 + #9 检查均已锁。

### K.6 P1-6 EXECUTION_MODEL.md
- 新建 `EXECUTION_MODEL.md`: REALTIME 链 (Template→Channel→Bundle→Profiles→GraphRuntime→Reservation→Session→READY_TO_TAKE→TAKE→RUNNING→FAILOVER/OUTPUT RECOVERY) + FILE 链 (Asset→Job→Worker→Asset Version) + 时序判定表 (谁执行/前置/后置) + 对象创建/引用总表 + 状态机对照。

### K.7 UX 附带回写
- B-13: **FINAL TAKE GATE** 视觉条 (Config/Runtime/READY 三格) + Switch **Policy Target vs Measured** 分离 (100ms·HOT / p95 87ms)。
- M-17 Pipeline 六段: Source→Normalize→**Program Master**→Encode→**Output Variants**→Adapters。
- M-14: Output Version 绑定 **FILE_PROFILE (ENC-v12..)**, 禁 REALTIME_PROFILE; **Job Policy (Batch/Concurrency/Worker/Retry/Schedule) 与 Profile 分层** (Step 5)。
- E-40: **Source Kind 首选** → Adapter → Protocol → Endpoint Schema 三级联动 (Network→UDP MC SSM 演示)。
- CD-01: **Action Context Bar** (PGM/PVW/MODE/BACKUP/AUDIO/OUTPUT/RESOURCE/CLOCK + TAKE) — 一眼回答"能不能切"。

### K.8 剩余 (0.5E/0.5G)
- 同 §I.7 + E-40 Media Contract 屏 / Network-Media Path 双视图 / Duplicate Channel / Host-Device Capacity / Dependency-Impact Workspace / M-18 kind 差异化 / Context Command Palette / 退役工作流。

---

## L. 0.5D.4 Semantic Closure — P-22/P-21 对象边界 + 网络源建模 + 执行时序 (2026-08-25 末 · 用户第 8 轮反向审计 2d818f8)

> 用户以 `2d818f8` 做反向一致性审计: 上一轮关键修正已落地, 但发现更深一层对象边界渗透。**结论: 仍不能 Freeze, 但本轮只做 `0.5D.4 Semantic Closure` (P0/P1 对象边界 + 执行时序), 不扩展页面。** 本轮 6 P0 + 7 P1 + 1 P2。

### L.1 P0-1 P-22 移除 Destination / CDN Endpoint 表单
- P-22 删除 `CDN Endpoint (Primary) https://cdn-a.internal/live` 表单字段 → 改为只读 `Destination 归 CD-21 Output → Variant → Destination`, 并加 field-hint 警告三处同存风险。Edge Policy 注修正: "Alternate Destination 由 P-27 **引用** (非本页创建)"。

### L.2 P0-2 P-22 Adapter 归并 (SRT 不是独立 Adapter)
- P-22 Adapter 示例 `SRSAdapter · FileAdapter / UDPAdapter · SRTAdapter` → `SRSAdapter (HLS·RTMP·SRT·WebRTC) · UDPAdapter · FileAdapter`。SRT 是 SRS Gateway 的一种 delivery protocol, 不再在 UI 造并列 `SRTAdapter` (与 V0.2 Output (SRS)→HLS/RTMP/SRT/WebRTC 一致)。

### L.3 P0-3 P-21 REALTIME Resource Reservation 锁 REQUIRED
- P-21 `Resource Reservation [REQUIRED/OFF]` → `🔒 REQUIRED` 只读, 禁 OFF (resource_reservation=REQUIRED 强约束); FILE_PROFILE 显示 N/A。

### L.4/L.5 P0-4/5 P-21 Failover Policy → Failover Compatibility (多选)
- P-21 `Failover Policy [PACKET/FRAME/MASTER]` 单选 → `Failover Compatibility` 多选 checkbox (FRAME_SWITCH / MASTER_SWITCH / PACKET_SWITCH); field-hint: Encoding Profile 只声明兼容哪些 Switch Mode, 实际 Effective 由 Graph Compiler Decision Tree 决定。
- ENCODE_MODEL_SPEC §3 `failover_compatibility` 命名同步为 `PACKET_SWITCH/FRAME_SWITCH/MASTER_SWITCH` (可多选), 与 UI 一致。

### L.6 P0-6 P-21 REALTIME Rate Control 限 CBR/Capped VBR
- P-21 `Mode [CBR/VBR/Capped VBR]` → `[CBR / Capped VBR / (disabled) VBR (Unbounded · 🚫 禁用于 REALTIME)]`, 禁 Unbounded VBR (ENCODE_MODEL_SPEC §3 rate_control)。

### L.7 P1-7 ENCODE_MODEL_SPEC 回写 profile_type 已落地
- §1 标题 `profile_type 枚举（P-21 当前缺失）` → `（✅ 0.5D.3 已落地于 P-21）`; §1 说明 / §5 映射表 / §7 锚点三处同步回写, 消除 "Spec 与 HTML 两份事实"。

### L.8/L.9 P1-8/9 E-40 三级联动 + UDP Unicast 分支 + Transport Format
- E-40 表单重构为 **Source Kind → Transport → Delivery Mode → Endpoint Schema**: Transport (UDP/RTP/SRT/...) → Delivery Mode (UNICAST/MULTICAST_ASM/MULTICAST_SSM) → Endpoint Schema。补全 **UDP Unicast** (Remote Address/Port) 与 **Multicast ASM** (Group/IGMPv2) 分支 Schema (此前仅 SSM)。
- `Container [MPEG-TS / RTP raw]` → `Transport Format [MPEG-TS over UDP / RTP (encapsulation)]`, 明确 UDP/RTP 是 Transport (封装层) 而非 Container。

### L.10/L.11/L.12 P1-10/11/12 UX 闭环
- E-40 验证面板回写 **E-42 Verification Result** compact 摘要 (Carrier/Packets/PAT-PMT/Video/Audio/Bitrate/Jitter/Loss/Clock → SOURCE READY), 再允许 SAVE VERIFIED/ASSIGN。
- M-17 指标区加 **异常→恢复动作** 下钻 (Open AVSync Detail / Apply Compensation / Restart Adapter / Open Incident), 从监控页变生产操作页。
- B-13 加 **Compact Confirmation** 说明: 正常 TAKE 显示紧凑确认, 仅异常展开 9 项诊断 (避免 24/7 盲点式确认)。

### L.13 P1-13 EXECUTION_MODEL 加 TAKE vs ChangeSet 分离
- 新增 §5: Configuration Change (ChangeSet→Apply→Runtime Rev) ≠ Operational TAKE (Runtime Event, 引用但不创建 ChangeSet); 模型层 Configuration/Runtime/Operational Surface 三分离。

### L.14 P2-14 M-17 Target vs Measured 视觉分离
- M-17 切换表 `target_failover_time_ms=100 (HOT)` / `measured_failover_ms p95=87` → 拆为 **HOT POLICY (Target 100ms)** 与 **BENCHMARK (Measured 87ms, ≠ Target)** 两段, 明确分割线。

### L.15 关系焊死 (用户原话)
```
P-21 = 如何编码        P-22 = 如何交付 (≠ Destination ≠ Adapter)
P-27 = 如何处理交付故障  CD-01 = Channel 操作组合
M-17 = REALTIME Session  M-14 = FILE Job   E-40 = 外部 Source Endpoint
REALTIME_PROFILE → Reservation → Session → READY_TO_TAKE → TAKE
```

### L.16 剩余 (0.5E/0.5G, 全仓一致性扫描后 Freeze)
- E-40 Media Contract 屏 / Network-Media Path 双视图 / P-22 未来 O-xx Destination 独立页 / Player Capability 由 Capability Registry 动态计算 / 全仓 Canonical Vocabulary + Surface Registry + Workflow 一致性扫描。
- 用户建议下一轮做 **全仓一致性扫描**, 确认无 "两个模型之间来回渗透" 后, 再进入 **Phase 0.5 Freeze**。
