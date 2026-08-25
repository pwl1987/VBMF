# VBMF Design System V0.1

> **文档定位:** VBMF Console 全局 UI 设计语言 — 锁 Phase 4 Web Console 实施
>
> **适用版本:** VBMF V0.2 LOCK FINAL + Phase 0.5 UX BASELINE LOCK FINAL
>
> **关联文档:**
> - [`SURFACE_SPEC.md`](SURFACE_SPEC.md) — V0.2 架构对象 → UI 表面
> - [`I18N_SPEC.md`](I18N_SPEC.md) — i18n Contract
> - [`../phase-0.5/ERRATA.md`](../phase-0.5/ERRATA.md) — Phase 0.5A 20 项修复归档

---

## 0. 目的

Phase 0.5A / 0.5B.1 wireframe 当前每页自己定义 CSS / State, Phase 4 实施会各做各的。本 Design System V0.1 锁定:
- **4 State Models Taxonomy** (不混用)
- **Color Tokens** (6 状态 + 4 角色)
- **Typography / Spacing / Border / Radius**
- **核心组件清单** (15 个)
- **Keyboard Shortcuts**

---

## 1. 4 State Models Taxonomy (锁定 — 不能互换)

V0.1 锁定 4 套独立的 State Model:

| 套 | 用途 | 枚举 (canonical) | UI 颜色 |
|---|---|---|---|
| **UI Surface State** | 页面渲染状态 | NORMAL / LOADING / EMPTY / WARNING / ERROR / CRITICAL | neutral / amber / red / red+pulse |
| **Lifecycle** | Session 生命周期 | STOPPED / STARTING / RUNNING / STOPPING | outline / blue(anim) / solid / gray(anim) |
| **Readiness** | 是否可接管 | NOT_READY / READY_TO_TAKE | yellow / green |
| **Health** | 健康度 | HEALTHY / DEGRADED / FAILED / UNKNOWN | green / amber / red / gray |
| **Node Role** | 节点角色 | ACTIVE / STANDBY / OFFLINE | solid blue / outline blue / outline gray |
| **Effective Channel Status** | Channel 对外唯一 status | HEALTHY / DEGRADED / FAILED / STARTING / STOPPED / UNKNOWN | (同 Health + blue for STARTING) |

**反模式:**
- ❌ 把 HealthState (HEALTHY/DEGRADED/FAILED) 和 NodeRole (ACTIVE/STANDBY/OFFLINE) 用同一组颜色
- ❌ 把 Lifecycle (STARTING) 和 Health (STARTING presentation policy) 混
- ❌ 把 UI Surface State (LOADING) 和 Lifecycle (STARTING) 混

**V0.2 锁定 (Errata-14):** ECHS = `channel_health_view.effective_channel_status`, 不允许 UI 自己算。

---

## 2. Color Tokens (V0.1 锁定)

```yaml
# Base
--bg:       #0d1117    # 主背景
--bg2:      #161b22    # 卡片背景
--bg3:      #1f242c    # 内容背景
--border:   #30363d    # 边框
--fg:       #c9d1d9    # 主文本
--fgdim:    #8b949e    # 次要文本
--accent:   #58a6ff    # 强调 / 链接

# Runtime Health (V0.2 锁定)
--health-healthy:   #3fb950  # green
--health-degraded:  #d29922  # amber
--health-failed:    #f85149  # red
--health-unknown:   #6e7681  # gray
--health-starting:  #58a6ff  # blue (Policy: lifecycle=STARTING → ECHS=STARTING)
--health-stopped:   #6e7681  # neutral gray outline

# Operational Role (V0.1 锁定 — 必须独立)
--role-active:    #58a6ff  # solid blue (NOT green! 不与 HEALTHY 混淆)
--role-standby:   #58a6ff  # outline blue / dashed
--role-offline:   #6e7681  # outline gray (NOT red! 不与 FAILED 混淆)

# UI Surface State
--ui-warning:   #d29922  # amber
--ui-error:     #f85149  # red
--ui-critical:  #f85149  # red + pulse animation
--ui-loading:   #6e7681  # neutral
--ui-empty:     #6e7681  # neutral + 引导

# 状态色块 (Badge 背景)
--badge-green-bg:   #1a3a1a
--badge-yellow-bg:  #3a3a1a
--badge-red-bg:     #3a1a1a
--badge-blue-bg:    #1a2a3a
--badge-gray-bg:    #1f242c
```

---

## 3. Typography

```yaml
font-family:
  primary:   -apple-system, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif
  monospace: Menlo, Consolas, "Courier New", monospace

font-size:
  xs:    9px    # 仅 micro label
  sm:    10px   # 状态徽章 / 表头
  base:  12px   # 正文
  md:    13px   # 主要内容
  lg:    14px   # 页面标题
  xl:    16px   # 区块标题
  xxl:   18px   # Hero 标题
  xxxl:  24px   # 大数字 / KPI

font-weight:
  normal:   400
  medium:   600
  bold:     700

line-height:
  tight:  1.3
  base:   1.5
  loose:  1.7
```

**注意:** 中文不要 line-height 太紧 (推荐 1.5-1.7), 英文 1.3 即可。

---

## 4. Spacing

```yaml
space-0:  0
space-1:  4px
space-2:  8px
space-3:  12px
space-4:  16px
space-5:  20px
space-6:  24px
space-8:  32px
space-10: 40px
```

---

## 5. Border / Radius

```yaml
border:
  width:    1px
  color:    var(--border)
  style:    solid

radius:
  sm:   3px
  base: 4px
  md:   6px
  lg:   8px
  full: 9999px
```

---

## 6. Components (V0.1 锁定 — 15 个)

### 6.1 Button

```yaml
Button:
  sizes: [sm, base, lg]
  variants: [primary, default, danger, l2, l3]
  props:
    label:    string
    icon:     string (optional)
    disabled: boolean
    dangerLevel: L1 | L2 | L3
    onClick:  handler
  i18n: button.label
```

**Danger Level 视觉:**
- L1: 默认按钮 (无确认)
- L2: 黄色背景 + 黑色文字 (3s 倒计时 Modal)
- L3: 红色背景 + 白色文字 (输入 "YES"/"DELETE" 确认)

### 6.2 Badge

```yaml
Badge:
  variants: [green, yellow, red, blue, gray, outline]
  props:
    label:    string
    variant:  enum
    i18n:     string
```

### 6.3 StatusBadge

```yaml
StatusBadge (继承 Badge + 加 dot):
  表示 Runtime HealthState
  props:
    state: HEALTHY | DEGRADED | FAILED | UNKNOWN | STARTING | STOPPED
    label: string
    dot:   bool (default true)
```

### 6.4 HealthDot

```yaml
HealthDot:
  颜色: health-healthy / health-degraded / health-failed / health-unknown
  size: 8 / 10 / 12 px
  animation: pulse (CRITICAL only)
```

### 6.5 RuntimeStateChip

```yaml
RuntimeStateChip (Lifecycle + Readiness + Health 合一):
  props:
    lifecycle: STOPPED | STARTING | RUNNING | STOPPING
    readiness: NOT_READY | READY_TO_TAKE
    health:    HEALTHY | DEGRADED | FAILED | UNKNOWN
    uptime:    duration
  表现: 三色 chip 横排
```

### 6.6 NodeRoleChip

```yaml
NodeRoleChip (Node Role 独立色):
  ACTIVE:   solid blue
  STANDBY:  outline blue / dashed
  OFFLINE:  outline gray
```

### 6.7 Tabs

```yaml
Tabs:
  orientation: horizontal
  activeIndicator: 2px var(--accent)
  variant: top / side
  i18n: tab.{name}
```

### 6.8 Table

```yaml
Table:
  rowHeight: 32px (默认) / 40px (高密度)
  headerHeight: 24px
  zebra: false (Dark Mode 不需要)
  rowHover: var(--bg3)
  rowSelected: #1f2a3a
  columns:
    - key
    - label (i18n)
    - type: text | number | status | action | badge
    - width
    - sortable
```

### 6.9 Drawer / Modal / Wizard

```yaml
Drawer:
  width: 280 / 360 / 480 / 720 px
  position: right (默认) / left
  title: string
  body: slot
  actions: 底部按钮 (Save L2 / Cancel)

Modal:
  size: sm (确认) / md (表单) / lg (详细) / xl (审批)
  backdrop: 黑色 60% 透明
  actions: 底部按钮

Wizard (M-14 New Job):
  steps: 5 (按 Phase 0.5B.1 锁定)
  stepIndicator: 顶部 Stepper
  prev/next: 底部按钮
  final: 提交按钮
```

### 6.10 MetricCard

```yaml
MetricCard (大数字 + 标签):
  props:
    label: string (i18n)
    value: number | string
    unit:  string (Mbps / ms / GB 等, 不本地化)
    trend: up / down / flat
    color: green / yellow / red / blue / gray
    size:  sm (32px) / md (48px) / lg (64px)
```

### 6.11 ResourceGauge (E-36 实时面板 + E-32 Preflight)

```yaml
ResourceGauge (9-Dim):
  dims: [CPU, GPU, VRAM, RAM, NIC_IN, NIC_OUT, DISK, PCIE_RX, PCIE_TX, BMD_IN, BMD_OUT, DEVICE_EXCLUSIVITY]
  每个 dim:
    required:   number
    available:  number
    delta:      number
    headroom:   number
    status:     OK | WARN | FAIL | N/A
  UI: horizontal bar + 数字 + status
  颜色:
    OK:   green
    WARN: amber (>80% 利用率)
    FAIL: red (>95% 或 conflict)
    N/A:  gray (无相关资源)
```

### 6.12 HealthNode (O-41 Health Tree 单个节点)

```yaml
HealthNode:
  name:        string
  role:        ACTIVE | STANDBY | OFFLINE
  state:       HEALTHY | DEGRADED | FAILED | UNKNOWN
  reason:      string
  detected:    timestamp
  duration:    duration
  impact:      string
  autoAction:  string
  recovery:    string
  operatorRequired: bool
  i18n: 全部字段
```

### 6.13 Timeline (O-43 Incident Timeline)

```yaml
Timeline:
  垂直时间线, 左侧时间戳, 右侧事件
  event:
    timestamp: HH:MM:SS.ms
    actor:    user / system / worker-XX
    action:   ACTION enum
    detail:   string
  i18n: event.{action}.label
```

### 6.14 Diff (P-21 / M-14 Profile Diff)

```yaml
Diff:
  before:   object
  after:    object
  highlights:
    - field:    path
      from:     value
      to:       value
      impact:   "CPU +18% / Time +31% / Size +20%"
  color: red (减少) / green (增加) / gray (无变化)
```

### 6.15 DangerActions (统一危险操作 L1/L2/L3)

```yaml
DangerActions:
  L1: 普通按钮 + 直接执行
  L2: 黄色按钮 + 3s 倒计时 Modal + Confirm/Cancel
  L3: 红色按钮 + 必须输入 "YES" / "DELETE" + 5s 倒计时 + Confirm/Cancel
  always:
    - 显示动作详情 (目标 / 影响 / 不可逆)
    - 强制刷新当前数据 (避免 stale)
    - 记录到 audit_logs
```

---

## 7. 6 状态样例 (UI Surface State — 锁定)

每页 6 状态必须全部实现, 缺一视为不完整:

| 状态 | 触发 | UI 表现 |
|---|---|---|
| **NORMAL** | 业务正常 | 全量数据 + 正常色码 |
| **LOADING** | 首次/刷新/异步 | Skeleton (3-10 行) + spinner 或灰底 |
| **EMPTY** | 0 数据 (新系统) | 中央引导 + 主按钮 + 副按钮 |
| **WARNING** | 部分软指标越界 (漂移/磁盘 80%) | 黄色 Banner + Alert (不阻断) |
| **ERROR** | 单次操作失败 (Encode 失败/Profile 校验错) | 红色 Banner + 错误信息 + Retry 按钮 |
| **CRITICAL** | 业务中断 (Source 全 FAILED / ChangeSet 失败) | 红色 Banner + pulse 动画 + Incident 入口 |

**Phase 4 实施:** 每个 page.tsx 必须包含 6 状态分支, 缺一不可。

---

## 8. Keyboard Shortcuts (V0.1 锁定)

### 8.1 全局

| Shortcut | Action |
|---|---|
| `Ctrl+K` | Command Palette |
| `Esc` | Close Drawer / Modal / Wizard |
| `?` | Show Keyboard Help |

### 8.2 Navigation (G + 首字母)

| Shortcut | Destination |
|---|---|
| `G D` | Dashboard |
| `G S` | Sources |
| `G W` | Switcher |
| `G C` | Composition |
| `G A` | Audio |
| `G O` | Output |
| `G R` | Recording |
| `G M` | Media Library (M-11) |
| `G X` | Transcode Center (M-14) |
| `G P` | Profiles (P-21) |
| `G E` | Engineering (Graph Designer) |
| `G H` | Health Tree |
| `G I` | Incidents |

### 8.3 Action

| Shortcut | Action | 危险级 |
|---|---|---|
| `T` | TAKE 切播 | L2 (3s 倒计时) |
| `F` | Failover 手动触发 | L2 |
| `R` | Retry (选中任务) | L1 |
| `Space` | Toggle Pause/Play (选中任务) | L1 |
| `Enter` | Open Detail | L1 |
| `Ctrl+S` | Save Draft / Save | L1 |
| `Del` | Delete (选中) | L3 (输入确认) |

**危险操作必须显式 L1/L2/L3**, 不允许快捷键绕过确认。

---

## 9. Visual System 总原则

### 9.1 视觉域分离

- **Broadcast 域** (Dashboard / Switcher / Output / Recording): 大状态 / 大数字 / 大预览 / 高优先级告警 / 低阅读负担
- **Engineering 域** (Graph Designer / Health Tree Engineering / Preflight / Profiles): 高信息密度 / 数据表 / Inspector / Graph / Diff / Capability / Resource
- **Admin 域** (Users / Roles / Audit / Settings): 中性 / 表格为主 / 强调安全

两套视觉语言自然形成, 不强行统一。

### 9.2 Dark Mode 优先

V0.1 仅支持 Dark Mode (24/7 广播机房需要)。Light Mode 留作 V0.4+。

---

## 10. 与其他 Phase 文档关系

| 文档 | 关系 |
|---|---|
| [`SURFACE_SPEC.md`](SURFACE_SPEC.md) | 定义**哪些** UI 表面 + **每页做什么** (page-level) |
| [`I18N_SPEC.md`](I18N_SPEC.md) | 定义**怎么翻译** (locale + key + canonical) |
| **`DESIGN_SYSTEM.md` (本文件)** | 定义**怎么画** (color / component / state / shortcut) (component-level) |

三层:
- **Surface Spec** = 数据 / 行为
- **i18n Spec** = 文案 / 多语言
- **Design System** = 视觉 / 组件

---

**VBMF Contributors** · VBMF Design System V0.1 · 锁定 Phase 4 实施
