# Operator Workflow — VBMF Phase 0.5

> V0.2 Architecture Baseline（22 轮 review LOCK FINAL）配套的操作员工作流规范。
> 本文档是 UI / 操作员行为的 Single Source of Truth；Wireframe 与代码以本文档为准。

## 1. 角色矩阵

| 角色 | 核心职责 | 关键页面 | 权限 |
|---|---|---|---|
| **Operator**（导播/值班员） | 日常播出、应急切源、监控 | Dashboard / Switcher / Output / Recording / Health Tree | View + Take + Emergency Stop |
| **Director**（节目总监） | 节目编排、Composition | Playout / Composition | View + Apply Change Set |
| **Engineer**（系统工程师） | 图设计、调参、故障诊断 | Sources / Graph Designer / Health Tree / Configuration | View + Compile + Apply + Rollback |
| **Admin**（系统管理员） | 账号、RBAC、审计 | 全页面 | Full（包括 User / System） |

## 2. 三轴状态机（V0.2 §8.11 锁定）

每个 Channel 在 UI 上同时显示 **Lifecycle / Readiness / Health** 三轴：

```
       Lifecycle        Readiness        Health
       STOPPED          NOT_READY        UNKNOWN
       STARTING         ↕                ↕
       RUNNING          READY_TO_TAKE    HEALTHY
       STOPPING                          DEGRADED
                                         FAILED
```

**Channel 对外 status 唯一入口**：`channel_health_view.effective_channel_status`
（= EffectiveChannelStatus：HEALTHY / DEGRADED / FAILED / STARTING / STOPPED / UNKNOWN）
**禁止** UI 直接读 `media_session_runtime.health` 当作 Channel Status（V0.2.4 Errata-13 锁定）。

## 3. 9 页面与操作流程

### 3.1 Dashboard（主控台）

**位置**：wireframes/01-dashboard.html

布局（Dark Mode 24/7）：

```
┌────────────────────────────────────────────────────────┐
│  [Channel: CH01 ▾]  [Time: 14:32:15]      [Profile: op]│
├──────────────────────┬──────────────────────┬──────────┤
│  PVW (Preview)       │  PGM (Program)        │ Status   │
│  ┌──────────────┐    │  ┌──────────────┐    │ HEALTHY  │
│  │  Source.B    │    │  │  Source.A    │    │          │
│  │  1080p25     │    │  │  1080p25     │    │ Lifecycle│
│  │  H.264       │    │  │  H.264       │    │ RUNNING  │
│  └──────────────┘    │  └──────────────┘    │ Ready    │
│                      │                      │ ✓        │
├──────────────────────┴──────────────────────┴──────────┤
│  [ TAKE ]    Emergency  Cut to: Source.A▾              │
│  NOW: Source.A    NEXT: Source.B (queued 14:35)        │
└────────────────────────────────────────────────────────┘
```

**操作**：
- 选 Channel（下拉）
- 看 PVW/PGM 双窗
- 选 NEXT 节目
- 按 **TAKE**（危险操作 — 见 §4）

### 3.2 Sources（源管理）

**位置**：wireframes/02-sources.html

11 个 Source Adapter 状态列表（V0.2 §2.4 锁定）：
- SDI（BMD DeckLink，3 张卡，dev0/dev1/io0）
- SRT / RTMP / HLS / WebRTC / RTP / UDP / RTSP / FILE
- INTERNAL（自环）
- COMPOSITE

每行：Source 名 / 类型 / 健康 / 时钟 / 带宽 / 操作（Lock / Unlock / Delete）

### 3.3 Switcher（切播器）

**位置**：wireframes/03-switcher.html

- 显示当前 COMPILED_MODE（channel_routes.switch_mode）
- 显示当前 EFFECTIVE_RUNTIME_MODE（media_session_runtime.effective_switch_mode）
- 显示最近 5 次切换事件
- **TAKE** 按钮（写入 change_set → apply）

### 3.4 Composition（图文包装）

**位置**：wireframes/04-composition.html

- **Program Composition 层**：节目级 Logo/字幕/版权（所有 Variant 共享）
- **Variant Composition 层**：平台 Logo/水印（按 Output Variant 叠加）
- RAW 域渲染（V0.2 §3.7.1 锁定）；FFmpeg burn-in（On-Air）
- Browser Canvas 仅用于设计 / 预览

### 3.5 Audio（音频）

**位置**：wireframes/05-audio.html

- 混音器（多路 PCM 输入）
- Loudness（EBU R128）
- 延迟补偿
- Audio Master Join 状态

### 3.6 Output（输出）

**位置**：wireframes/06-output.html

- SRS Adapter 输出列表（HLS / RTMP / WebRTC / SRT）
- 每路：URL / 健康 / 带宽 / 客户端数（HLS .m3u8 请求数）
- **Restart Adapter** 按钮（危险操作）

### 3.7 Recording（录制）

**位置**：wireframes/07-recording.html

- 录制通道列表（每路 5 min chunk）
- 录制状态 / 磁盘占用
- **事件回溯**链接到 Incident Timeline
- 检索：按时间 / Channel / 标签

### 3.8 Graph Designer（NEW，V0.2 §3.10）

**位置**：wireframes/08-graph-designer.html

- 拖拽：Source / Process / Output 节点
- 边声明：Data Plane / Clock Domain / Edge Policy
- 实时编译预览（X1 Graph Compiler）
- Preflight 报告（X2）
- Dry Run
- Apply with Change Set

### 3.9 Health Tree（NEW，V0.2 §3.9 + §5 7 Health Invariants）

**位置**：wireframes/09-health-tree.html

```
CH01 (Channel: HEALTHY)
├── Source.Primary  ACTIVE    HEALTHY
├── Source.Backup   STANDBY   HEALTHY       # red_group_id=RG-001
├── Switcher        —         HEALTHY
├── Composition     —         HEALTHY
├── Audio Mixer     —         HEALTHY
├── Video Master    —         HEALTHY
├── Audio Master    —         HEALTHY
├── Program Master  —         HEALTHY
├── Output.SRS      —         HEALTHY
│   ├── HLS         —         HEALTHY
│   ├── RTMP        —         HEALTHY
│   └── WebRTC      —         HEALTHY
├── Output.SDI      —         (DISABLED)
└── Recording       —         HEALTHY
```

每个节点：颜色 = state，下钻 = details_json，关联 = Incident。

## 4. 危险操作分层（V0.2 §10.1-10.10 锁定）

操作按风险分 3 层：

| 层 | 风险 | UI 反馈 | 示例 |
|---|---|---|---|
| **L1 普通** | 可逆 | 无二次确认 | 切 PVW 预览、查看统计 |
| **L2 重要** | 半可逆 | 二次确认（弹窗） | TAKE 切 PGM、Apply Change Set |
| **L3 危险** | 不可逆 | 二次确认 + 倒计时 + 输入 "YES" | DELETE Channel、Stop Master、Reset Configuration |

实现：
- L2：`window.confirm()` + 3 秒倒计时
- L3：必须输入大写 "YES" 才能继续
- L3 全部记录到 `incidents` 表（V0.2 §5）

## 5. 4 关键操作链（详细在 chains/）

| 链 | 文件 | 核心步骤数 |
|---|---|---|
| 1. On-Air（Operator） | chains/chain-1-on-air.md | 6 步 |
| 2. Failure（Operator + System） | chains/chain-2-failure.md | 9 步 |
| 3. Playout（Director） | chains/chain-3-playout.md | 7 步 |
| 4. Engineering（Engineer） | chains/chain-4-engineering.md | 9 步 |

## 6. 与 V0.2 架构的对应

| 操作员概念 | V0.2 架构位 |
|---|---|
| Channel | §3.1 Data Plane + §5 channel_* |
| PVW/PGM | §3.7 Program Master（Video Join） |
| TAKE | §3.4 Switch Mode Decision Tree |
| Health 颜色 | §5 channel_health_view（C.26 Errata-14 7 规则） |
| 9 页面 | §10 UX 架构（7+2） |
| Change Set | §1.21 + §5 config_revisions / change_sets |
| Incident | §5 incidents（X4 Incident Timeline） |
| Recording | §3 Recording Engine + §5 chunked recording |

## 7. Acceptance Checklist

- [x] 角色矩阵：Operator / Director / Engineer / Admin
- [x] 三轴状态机：Lifecycle / Readiness / Health
- [x] 9 页面：dashboard / sources / switcher / composition / audio / output / recording / graph-designer / health-tree
- [x] 危险操作 3 层：L1 / L2 / L3
- [x] 4 操作链：On-Air / Failure / Playout / Engineering
- [x] 与 V0.2 架构一一对应

## 8. 不在本阶段范围

- UI 实际像素设计（Phase 4）
- 国际化（V0.2 不锁 i18n）
- 移动端 / 平板适配（24/7 机房 = 桌面浏览器为主）
- 权限系统实现细节（Phase 3 Auth & RBAC）

---

**维护人**：Mavis（Phase 0.5 起草）
**关联文档**：`ARCHITECTURE_V0.2.md`（22 轮 review LOCK FINAL）
