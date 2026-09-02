# VBMF Current Reality Audit / Original Goal Reconciliation

- **基线**: master `f3f86ef`（Alpha-1 收口点）· 2026-09-02 · 只读审计（零代码改动）
- **方法**: README/V0.2 权威能力清单（12 Engines + 5 横向系统 + 6 横切能力, README L57-68 原文核验）
  → 逐项映射当前代码锚点 → 完成度/分层/重构需求/Alpha 归属/V0.2 合规
- **审计触发**: 用户 2026-09-02 架构级复核裁定（"实现重心偏移"判断 + 纠偏窗口结论, 事实核验 6/6 属实）

## 0. 总判断（与用户结论对照，逐条落实）

| 用户判断 | 审计结论 |
|----------|----------|
| 无"路线跑偏"级偏离, 有明显"重心偏移" | **同意**——V0.2 边界纪律被完整守住（9/10）; 但 12 Engines 中 **8 个未实现/纯地基**, 产品覆盖 ~20% |
| Runtime 完整度 ≠ VBMF 完整度 | **同意**——Runtime Kernel（session/pipeline/health/supervisor/command/event/idempotency）完成度 85-95%; 播控平台属性能力（Switcher/Composition/Playout/Redundancy/QC/Recording）**合计 <10%** |
| main.rs 是最大技术风险 | **同意并加码**——105,837 字节中除组合根外还内嵌**三段真机 gate 代码**（E1-E8/0.7D/transport, 测试设施编译进生产二进制）+ watchdog 循环——三合一堆积 |
| 文档漂移 | **确认**——ROADMAP 相位停在"0.7C NEXT"（实际已到 Alpha-1）; SYSTEM_AND_PROJECT_PLAN 两处"尚无正式业务代码"（L12/L236）; 四层时间线不一致 |
| Vendor-Neutral 未被证明 | **同意**——SPI 白盒在（adapters 隔离/Provider/Backend trait）, 但**零第二实现**（无 AJA/FFmpeg backend）, "替换后上层不变"无实证 |
| 底座不需推倒重来 | **同意**——七条红线（Observation≠Config / Intent≠Plan / Vendor≠Domain / Supervisor 不碰 GStreamer / Transport 不污染 / Snapshot 一致性 / 多输入全回滚）全部在位且有测试锁 |

## 1. 12 Engines 全量对账（V0.2 §2.1 权威清单）

| # | Engine | 当前代码锚点 | 完成度 | 分层 | 需重构 | Alpha 归属 | 违反 V0.2 |
|---|--------|--------------|--------|------|--------|-----------|-----------|
| 1 | **Source** (11 types) | adapters/blackmagic (decklink/device_manager/sdk) + resolver.rs + registry.rs + port.rs | BMD 一型 ~90%; **11 型覆盖率 ~15%**（mock=测试, 其余 9 型零） | ✓ 正确（adapters+contracts SPI） | 否 | 按需（AJA/NDI/SRT…） | 否 |
| 2 | **Signal Fabric** | 无专属概念（port.rs 是端口身份, 非 fabric） | **0%** | —（不存在） | — | 远期（依赖 Control Plane/多 agent） | 否（未实现≠违反） |
| 3 | **Normalize** | normalize.rs (CanonicalMediaDescriptor+纯函数) + clock.rs/timecode.rs/audio.rs 词表 | **Foundation**（语义 descriptor 归一; 非像素/采样级归一管线） | ✓ 正确（domain） | 否 | **A2 主战场** | 否 |
| 4 | **Redundancy** | supervisor.rs（恢复决策纯引擎）+ lease.rs | **Foundation**（单机恢复; 主备/standby/offline 语义零） | ✓ 正确 | 否 | A5/V0.3 | 否 |
| 5 | **QC** | signal.rs（first-frame/PTS/单调）+ preflight 能力判定 | **Foundation**（信号观测 ≠ QC：无内容/合规检查） | ✓ | 否 | 远期 | 否 |
| 6 | **Playout** | 零（无时间线/播放列表/调度） | **0%** | — | — | 远期（"节目时间线可靠"核心载体, 当前全仓零覆盖） | 否 |
| 7 | **Switcher** (PACKET/FRAME/MASTER) | `PipelinePlan.switch_mode="FRAME_SWITCH"` **占位字符串从未被消费**（probe 实证） | **~5%**（占位） | 字段在 domain ✓ 但零实现 | 否（从零建, 非重构） | **A2 核心入口** | ⚠️ 软性（词表占位无牙齿——与 sink.kind P1a 前同病） |
| 8 | **Composition** (含 Master Join) | 零（无 compositor/overlay/图文） | **0%** | — | — | A2（Master Join）/A3 | 否 |
| 9 | **Audio** | audio.rs（Role/RouteIntent 语义词表）+ AAC 编码支 | **Foundation**（词表 ≠ Audio Engine：无混音/声道映射/响度） | ✓ | 否 | A2（Audio Master 起步） | 否 |
| 10 | **Output** | P1a/P1b: openh264/AAC→HLS/RTMP + OutputPlan + 静态面 | **Partial**（Demo Output 真机实证; 非 Output Engine：1 编码路径, 无 OutputProfile/多输出并行/ABR） | ✓（OutputPlan domain/编码 adapter）; ⚠️ 配置经 env demo 缝 | 小（env→X3 版本化配置时收口） | A3 | ⚠️ 软性（配置未版本化, X3 未动） |
| 11 | **Recording** | 零 | **0%** | — | — | A3 | 否 |
| 12 | **Replay** | 零 | **0%** | — | — | 远期 | 否 |

**小计**: 已实证 1.5/12（Source-BMD 强, Output-Demo 半）; 地基 3.5（Normalize/Redundancy/QC/Audio）; 零 7（Signal Fabric/Playout/Switcher 实质/Composition/Recording/Replay + Switcher 占位）。

## 2. 5 横向系统对账（V0.2 §2.2）

| # | 系统 | 当前锚点 | 完成度 | 分层 | 重构 | Alpha | 违反 |
|---|------|----------|--------|------|------|-------|------|
| H1 | Safety | 安全模型文档 + transport 默认 127.0.0.1 + 反代约束 | **Foundation**（文档强机制少; 无 auth/权限平面） | ✓ | 否 | A4 | 否 |
| H2 | Resource Scheduler | resource.rs 状态机 + lease.rs TTL/renew + DEVICE_EXCLUSIVITY + A1 多输入全回滚 | **Implemented（单机域最强横向系统）** | ✓ 正确 | 否 | —（多机归 V0.3） | 否 |
| H3 | Watchdog & Incident | supervisor + health fold + **watchdog 循环在 main.rs 内嵌** + events | **Partial** | ❌ **watchdog 位置错**（应在 domain/独立模块, 非 main 105KB 内） | **是（A2 拆出）** | A2 拆分 | 否 |
| H4 | Audit | RuntimeEventLog + projection + 幂等日志 | **Foundation**（事件审计 ≠ 完整审计面：无操作/配置变更审计） | ✓ | 否 | A4+ | 否 |
| H5 | Subtitle | 零 | **0%** | — | — | 远期 | 否 |

## 3. 6 横切能力对账

| # | 能力 | 当前锚点 | 完成度 | 重构 | Alpha | 违反 |
|---|------|----------|--------|------|-------|------|
| X1 | Graph Compiler | graph_intent.rs（intent 承接）+ materialize（单跳物化） | **Foundation**——V0.2 §1004-1010 定义 **7 步编译器**（validate/insert-nodes/clock-align/latency/resource-plan/preflight/emit）, 当前仅第 1 步子集 | 否（从零补 6 步） | A2 起步（A4 深化） | 否 |
| X2 | Preflight | preflight.rs 六阶段 judge-only + 真机验证 | **Implemented ✓✓（完成度最高的横切能力）** | 否 | — | 否 |
| X3 | Configuration Versioning | 零（GraphSpec/ChannelConfig/OutputProfile 版本化未动; env demo 缝即此缺口产物） | **0%** | — | A4 | ⚠️ 软性（Output 配置绕过版本化） |
| X4 | Incident Timeline | event_projection（kind_counts/session_failures） | **Foundation** | 否 | A4/A5 | 否 |
| X5 | Health Tree | health.rs fold（扁平态）+ D14 信封 | **Foundation**——V0.2 §929-966 Channel→Subsystem→Node 树形聚合（HEALTHY/DEGRADED/FAILED + standby/offline）未实现 | 否 | A2 起步（Channel 聚合）→A5 全语义 | 否 |
| X6 | Capability Registry | capability 投影 + registry.rs 适配器选择 | **Foundation** | 否 | 按需 | 否 |

## 4. UI 对账（56 surfaces vs 现实）

- **SoT**: `docs/phase-0.5/SURFACE_REGISTRY.yaml`（32 LOCK + 24 SPEC）+ 5 P0 wireframes + 36 项语义收口。
- **现实**: 1 个内嵌最小控制台页（P1b: 状态行/输入行/HLS 播放/Start-Stop）≈ **1/56 surface**; Phase 0.5 UX 原型产物在库（HTML 静态, 结论待 Phase 4 React 落地）。
- **结论**: UI 产品化 2-3/10 判断属实; 但 56 surfaces 的**语义收口**中已有部分对应物（runtime/commands/health API 即多个 surface 的后端面）——UI 缺的是壳与编排, 部分语义已在。
- 归属: **A4（Control Plane 线）**——UI 不应在 media-agent 里长出来（当前 1 页是原型例外, A4 时迁 Fastify/独立前端）。

## 5. 分层归位审计（Domain / Control Plane / Runtime / Adapters / Infra / UI）

| 层 | 现状 | 判定 |
|----|------|------|
| Domain（normalize/clock/audio/timecode/session 语义/command/idempotency/error） | media-agent src 内, vendor 零依赖（白盒测试在位） | ✓ 基本正确 |
| Runtime Kernel（session/pipeline/resource/health/supervisor/events） | 同上, 边界纪律强 | ✓ 正确 |
| Adapters（blackmagic/gstreamer/mock + contracts SPI） | vendor 隔离白盒（API-BOUNDARY-01/ARCH-PORTABILITY CI 门禁） | ✓ 正确; **未证明可替换**（零第二实现） |
| **Control Plane** | **不存在**（Fastify=ROADMAP Phase 2 未建）。其职责（intent 编排/UI 服务/多 agent 协调/ChannelConfig 版本化）当前由 **media-agent main.rs 诊断路径 + transport 静态面代偿** | ❌ **"塞进 Media Agent"主堆积点**——A2/A4 必须开始外移 |
| UI | transport.rs 内嵌 1 页（原型例外） | ⚠️ 临时位, A4 迁出 |
| Infrastructure | cargo/GitHub CI/盒验证链 | ✓ |

**main.rs 105KB 解剖**: 组合根（应保留, 瘦身后 ~20-30KB）+ **三段真机 gate 代码**（E1-E8 SESSION_LIFECYCLE / 0.7D gate / transport gate——测试设施编译进生产二进制, 应拆 cfg-gated 测试 bin 或独立入口）+ **watchdog 循环**（H3 系统, 应拆 domain 模块）+ 诊断 auto-start（Control Plane 代偿, A4 外移）。**Alpha-2 拆分窗口成立**。

## 6. 结论与 Alpha-2 入口建议

**同意用户核心判断**: 架构设计正确、无需推倒; 实现路线需从 **Runtime-first 转 Product-domain-first**（Channel→Graph→Program Master→Output→Runtime Adapter; GStreamer/BMD 退纯 adapter）。

**Alpha-2 具体代码切入点（建议顺序）**:
1. **A2-0 结构拆分（先行, 不加新能力）**: main.rs 三拆（gate 段→`#[cfg]` 测试设施 bin; watchdog→domain 模块; 组合根瘦身）——用户点名的结构窗口, 先还债再上语义。
2. **A2-a Program Master 域对象（TDD domain 先行）**: Channel/SwitchPolicy(PACKET/FRAME/MASTER 三模式词表, 与 sink.kind 同款 fail-closed)/Video-Audio-Metadata Master/Master Join/ProgramMaster——纯 domain 对象 + mock 全测; GStreamer compositor/videomixer 实现进 adapter（像素级 Switch 首证）。
3. **A2-b X5 Channel 健康聚合起步**（V0.2 §929-966 保守子集→A5 全语义）。
4. **并行 A4 线（Control Plane 骨架）**: Channel 模型完整语义落 Fastify 骨架或独立 crate（把 main.rs 诊断代偿职责开始外移）——按用户修正后的双线图。
5. **文档治理小 change（独立, 不混 Alpha-2）**: ROADMAP 相位刷新至现实（0.7C/0.7D/Prototype-1/Alpha-1 已完成 + Alpha 路线）; SYSTEM_AND_PROJECT_PLAN 顶部加"历史 V0.1 规划文档（截至 2026-08-25 冻结时点）"重定性标头; README 与 PHASE_IMPLEMENTATION_MAP 对齐锚点。

**不做的**: 不改 V0.2 冻结语义; 不在 Alpha-2 混入 Recording/多输出（A3）; 不提前宣布 Vendor-Neutral 完成（留第二 backend 实证时点）。

## 附: 逐条复核记录（用户事实判断 vs 实测）

| 用户判断 | 实测 | 结论 |
|----------|------|------|
| main.rs 105KB/session 91KB/pipeline 63KB | 105,837 / 91,634 / 62,909 字节 | ✅ 逐字节吻合 |
| services/ 仅 media-agent | 仅 media-agent | ✅ |
| ROADMAP 停在 0.7C NEXT/0.8/Phase 1-4 全 📋 | L25-27/L35-40 原文一致 | ✅ |
| SYSTEM_AND_PROJECT_PLAN "尚无正式业务代码" | L12/L236 两处 | ✅ |
| Channel=控制台侧规约（A1 交付自述） | 设计 §2.2 裁决原文 | ✅ |
| Vendor-Neutral 架构在/未证明 | contracts SPI+adapters 隔离在; 零第二实现 | ✅ |
| 各完成度评分 | 审计表逐项校准（Switcher 实为占位 ~5%; Program Master 代码存在度 0%, 文档语义在） | ✅ 基本吻合, 个别项用户略高估 |
