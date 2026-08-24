# Phase 0.5B — Product UI Surface Closure

> **Phase 0.5A = Operator Semantics (LOCK FINAL) · 9 Core Pages + 1 Validation Page**
>
> **Phase 0.5B = Product UI Surface Definition (当前) · 把 V0.2 架构所有对象映射到 UI**
>
> **Phase 0.5 全部 Freeze 的条件: 0.5B 把 Media / Transcode / Profiles / Engineering / Admin / Operations 这些"架构已有、UI 未落地"的工作域全部定义清楚。**

## 0. 缘起

V0.2 架构 LOCK FINAL 后已经定义了大量 UI 工作面：
- `media_assets / asset_versions / asset_rights`
- `encoding_profiles / output_profiles / audio_profiles / graphic_profiles / qc_profiles / rights_profiles / edge_policy_profiles`
- `playlists / composition_templates / composition_layers`
- `media_jobs / media_job_attempts`
- `preflight_runs`
- `config_revisions / change_sets / change_set_items`
- `incidents / signal_contracts / node_contracts`
- `users / roles / permissions / user_roles / audit_logs / api_keys`
- `device_registry / hardware_capability / clock_fallback_chain`
- `failover_benchmarks / latency_probes / signal_pool / signal_current_state`

但 Phase 0.5A (Operator Semantics) 的 9 Core 页面只覆盖了"运行时播控"一面。其余 25-30 个 UI 工作面**架构层已存在、UI 层未落地**。这是当前最大的断层。

## 1. 阶段目标

**只定义、不实现**：

| 维度 | 目标 |
|---|---|
| 页面树 / Page Tree | 6 大工作域 × 30 个页面 / 抽屉 / 子页 |
| 每页字段 | 目标 / 信息架构 / 主要操作 / 状态模型 / 6 状态样例 / 危险操作 / 权限 / 关联工作流 / 架构对象映射 |
| 跳转关系 / Navigation Graph | 6 大工作域内 + 跨工作域跳转 |
| 角色权限矩阵 | Operator / Director / Engineer / Admin × 30 页 × 5 类操作 |
| 实施顺序 | 哪些 P0 / 哪些 P1 / 哪些 P2 / 哪些可以延后到 Phase 4 Web Console |

**不**做：
- ❌ 不重做 0.5A 的 9 Core 页面（已经 LOCK FINAL）
- ❌ 不开 V0.2.5 / 不动 V0.2 架构
- ❌ 不实现具体 wireframe（实施是另一个阶段）

## 2. 6 大工作域

```
VBMF Console
│
├── 01 Broadcast     播控工作域     ← Phase 0.5A (LOCK FINAL, 9 Core Pages)
├── 02 Media         媒体资产域     ← Phase 0.5B 新增 (6 页)
├── 03 Profiles      配置 Profile   ← Phase 0.5B 新增 (7 页)
├── 04 Engineering   工程工作域     ← Phase 0.5A 部分 (2 页) + 0.5B 新增 (5 页)
├── 05 Operations    运维工作域     ← Phase 0.5A 部分 (1 页) + 0.5B 新增 (4 页)
└── 06 Administration 平台管理域    ← Phase 0.5B 新增 (5 页)

+ 1 Validation State Reference Page (10-states) ← Phase 0.5A LOCK FINAL
```

## 3. 30 个 UI 表面（待定义）

### 02 Media（6 页）
- **M-11** Media Library 媒体库
- **M-12** Asset Detail 资产详情
- **M-13** Upload / Ingest 上传导入
- **M-14** Transcode Center 转码中心
- **M-15** Transcode Jobs 转码任务
- **M-16** Versions / Renders 版本与渲染产物

### 03 Profiles（7 页）
- **P-21** Encoding Profiles 编码 Profile
- **P-22** Output Profiles 输出 Profile
- **P-23** Audio Profiles 音频 Profile
- **P-24** Graphic Profiles 图文 Profile
- **P-25** QC Profiles 质量检测 Profile
- **P-26** Rights Profiles 版权 Profile
- **P-27** Edge Policy Profiles 边策略 Profile

### 04 Engineering（5 新 + 2 已有 = 7 页）
- **E-31** Graph Designer 图设计 *(= 0.5A #08)*
- **E-32** Preflight Center 预检中心
- **E-33** Change Sets 变更集
- **E-34** Capability Registry 能力注册表
- **E-35** Device Registry 设备注册表
- **E-36** Resource / Capacity 资源容量
- **E-37** Clock 时钟

### 05 Operations（4 新 + 1 已有 = 5 页）
- **O-41** Health Tree 健康树 *(= 0.5A #09)*
- **O-42** Alerts / Incident Center 告警事件中心
- **O-43** Incident Timeline 事件时间线
- **O-44** Replay 回放（事件 → 录像窗口定位）
- **O-45** Benchmarks 基准测试结果

### 06 Administration（5 页）
- **A-51** Users 用户
- **A-52** Roles 角色
- **A-53** Permissions 权限
- **A-54** Audit Logs 审计日志
- **A-55** System Settings 系统设置

### 总计
- 0.5A 已 LOCK：9 Core + 1 Validation
- 0.5B 待定义：~25 个 Product UI 表面
- = 全部 ~35 个 UI 工作面

## 4. 阶段产物

- **[`SURFACE_SPEC.md`](SURFACE_SPEC.md)** — VBMF UI/UX Surface Specification V0.2
  - 6 工作域总览
  - 每页字段定义（30+ 页 × 9 维度）
  - Navigation Graph
  - 角色 × 权限矩阵
  - 实施顺序 P0/P1/P2
  - 架构对象映射表

## 5. 与 V0.2 / Phase 0.5A / Phase 0.6 / Phase 1 / Phase 4 衔接

```
V0.2 Architecture (LOCK FINAL)
   ↓ 定义所有架构对象
Phase 0.5A Operator Semantics (LOCK FINAL)
   ↓ 9 Core + 1 Validation = 运行时播控 UI
Phase 0.5B Product UI Surface (当前)
   ↓ 25+ Product UI 表面 = 完整 VBMF Console 信息架构
Phase 0.5 全部 Freeze
   ↓
Phase 0.6 Executable Acceptance Specification
   ↓ 验收用例
Phase 1 Media Core (Rust)
   ↓ 后端实现
Phase 4 Web Console
   ↓ 前端实现 = 真正产品 UI
```

## 6. i18n 锁定（与 0.5A 一致）

- 所有 Markdown 文档：**中文为主**
- 9 Core + 1 Validation wireframe：**中英双语**（中文为主 + 关键术语保留英文）
- 0.5B 新增 wireframe（如后续实施）：**同 i18n 锁定**
- 保留 Canonical Vocabulary 原文（PACKET / FRAME / MASTER / HLS / RTMP / WebRTC / SRT / H.264 / H.265 / PTP / LUFS / EBU R128 / dBTP / TS / Rust / JSON Schema / PG enum 等）
- 保留行业标准术语原文（同上）

---

**VBMF Contributors** · Phase 0.5B Product UI Surface Closure · 在 V0.2 架构基础上
