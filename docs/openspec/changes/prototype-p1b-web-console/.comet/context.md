# Comet Design Handoff

- Change: prototype-p1b-web-console
- Phase: design
- Mode: compact
- Context hash: a966d991c9724b9e4b97eb6a022354794e877252494879ccb844e0fb49673aab

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/prototype-p1b-web-console/proposal.md

- Source: docs/openspec/changes/prototype-p1b-web-console/proposal.md
- Lines: 1-36
- SHA256: 552253603e6524231490ef570b8f5c886d09ef2666a33d270b4a81623faab450

```md
# Proposal — prototype-p1b-web-console

## Why

P1a 已让真 SDI 变成编码后可消费的媒体（master=6fc863b, Gate P1a-01..06 全 PASS）, 但只能靠 curl/ffmpeg 看证据。Prototype-1 的收口缺口是**人的可见性**: 一个打开浏览器就能看到 VBMF 在工作的最小 Web Console。用户 2026-09-02 裁定 A 方案（同一 std-only HTTP 端口增加静态文件面）并给出十条验收标准。

## What Changes

- **静态文件面前置层**（transport.rs, 零触碰五端点 route 表）: `GET /` 服务内嵌最小 Web Console 页（const HTML, 无磁盘读）; `GET /hls/*` 服务 HLS 分片/playlist（`VBMF_OUTPUT_HLS_DIR` 文件读取 + 路径穿越防护 + 按 MIME `.m3u8`/`.ts`）; 响应层扩展 content-type（现有 JSON 端点逐字节不变）。
- **`TransportContext.hls_dir`**（additive; main.rs 从 PrototypeOutputConfig 接线）。
- **`ApiSession.outputs`** wire 投影（canonical 物化事实已有, API DTO 侧补齐——P1a verify §8 登记项）。
- **Web Console 页**: 轮询 `/health` + `/api/v1/runtime` 显示真实状态（devices/CH01/SDI 状态/session phase/**outputs 物化状态**）; hls.js 播放 `/hls/index.m3u8`（CDN, Safari 原生回退）; **Start/Stop 按钮经既有 `POST /api/v1/commands`**（start_session/stop_session, 零新 API）。
- **诚实状态红线**: 页面状态全部来自活轮询——输出停止后 phase/outputs 如实变化, 绝不虚报 RUNNING/READY。
- **P1a follow-up 纳入验收**: ① openh264 分片 IDR 对齐验证; ② 编码分支故障模式 gate（输出目标不可达 → 行为诚实不悬挂）。

## Non-Goals

- React/构建链/任何前端依赖管理（单文件原生 HTML+JS）
- 鉴权/HTTPS（内网 Prototype; 生产经反代+认证的约束已在 /health 注释锁定）
- Program Master/Switch/多通道 UI、输出参数配置 UI（Alpha）
- transport 五端点任何语义改动、新 API 端点
- Federation / Control Plane

## 验收场景（用户十条 + follow-up, 即 Gate P1b-01..10）

1. `/` 可打开（HTTP 200 text/html）
2. 显示 /health + /api/v1/runtime 真实状态
3. 当前输出状态可见（ApiSession.outputs 物化事实）
4. HLS playlist 可访问（正确 MIME）
5. **实际 HLS 消费**: ffmpeg 作为真实 HLS 客户端经 HTTP 拉取 playlist+分片并解码出帧（非仅 200）; 浏览器画面 = 人眼验收（LAN bind 指引入报告）
6. Start/Stop 与 Session 生命周期正确联动（经 commands 平台）
7. 输出停止后页面状态不虚报（stop 后 phase/outputs 如实）
8. P1a 全 Gate 回归零退化
9. follow-up① 分片首帧 IDR 对齐验证
10. follow-up② 编码分支故障模式（RTMP 无接收端 → 诚实失败/恢复, 不静默悬挂）
→ 最终形成 **Prototype-1 真机验收报告**

```

## docs/openspec/changes/prototype-p1b-web-console/design.md

- Source: docs/openspec/changes/prototype-p1b-web-console/design.md
- Lines: 1-37
- SHA256: 9156ad6ccf0ab7e84f3bd8ccdde08cba00263d04183ac46f4a966af3717a81d6

```md
# Design — prototype-p1b-web-console（高层框架）

## D1 静态面前置层（零触碰五端点冻结）

```
serve_connection:
  static_response(method, path, ctx.hls_dir)  ← 前置: GET / 与 /hls/* 在此终结
    ├─ Some(resp) → 直接按 (status, content_type, bytes) 写响应
    └─ None       → 落入既有 route()（五端点逐字节不变）
```

- `/` = **内嵌 const HTML**（无磁盘依赖; 页面自带 JS, 无构建链）。
- `/hls/*` = 磁盘读取（仅 `VBMF_OUTPUT_HLS_DIR` 目录内; **路径穿越防护**——拒绝 `..`/绝对路径/空段; MIME: `.m3u8`→`application/vnd.apple.mpegurl`, `.ts`→`video/mp2t`）。
- 响应层: 写响应改为携带 content-type（JSON 路径恒 `application/json` 不变; body 通道扩为 bytes 供二进制分片）。
- hls_dir 缺失 ⇒ `/hls/*` 返回 503（契约诚实, 同 not_available 风格）。

## D2 页面行为（原生 HTML+JS, 轮询诚实）

- 1s 轮询 `/health` + `/api/v1/runtime`; 渲染: devices 数 / CH01 状态（session phase）/ SDI 信号（/health state）/ **outputs**（`ApiSession.outputs` 物化事实——空数组如实显示 ANALYSIS-ONLY, 绝不虚报）。
- 视频: `hls.js`（CDN, Safari 原生 HLS 回退）加载 `/hls/index.m3u8`; 加载失败如实显示（不假画面）。
- **Start**: `POST /api/v1/commands {kind:start_session, target:{target_type:session, intent}}`——intent = 首设备 + sink kind 由 `/hls/index.m3u8` 可达性探测得出（hls↔rtmp, 页面无 env 可见的诚实推导）。
- **Stop**: `{kind:stop_session, target:{target_type:session_by_id, session_id}}`（id 取自 runtime sessions）。
- 全部经既有 commands 平台（幂等/错误分类白盒已锁）, **零新 API**。

## D3 ApiSession 加法

`ApiSession.outputs: Vec<String>` + `to_api_session` 投影（canonical `SessionRuntimeState.outputs` 物化事实, P1a 已建）。additive 非破坏。

## D4 故障模式与 follow-up 裁定

- **follow-up②**: RTMP 目标不可达 = 管线 bus Error → 既有 Supervisor 恢复策略接管（重启预算内重试, 耗尽升级）——**复用既有语义, 不新增故障面**; gate 断言: 无接收端运行 → 事件/终态诚实、无静默悬挂、分析链状态一致。
- **follow-up①**: gate 用 ffprobe/ffmpeg 逐分片验证首视频帧为 keyframe（IDR 对齐）; 若未对齐 → 调 `hlssink2 send-keyframe-requests=true` 复验（openh264enc 是否响应由真机定; 不对齐则如实记档不阻塞）。

## D5 验证

- Unit/Simulation: static_response 路由/穿越拒绝/MIME/hls_dir 缺失 503; ApiSession 投影; 现有测试零回退（含五端点 route 表测试逐字符不变）。
- Hardware（盒, 脚本不入库）: Gate P1b-01..10（proposal 十条）+ P1a gate 全回归 + **Prototype-1 真机验收报告**（verify 报告承载, 含人眼浏览器验收指引: `MEDIA_AGENT_HEALTH_BIND=10.30.15.10:8080`）。

```

## docs/openspec/changes/prototype-p1b-web-console/tasks.md

- Source: docs/openspec/changes/prototype-p1b-web-console/tasks.md
- Lines: 1-19
- SHA256: abc91485b917d4508aa6d288c403f01e8025e81574a4cc2e72e0167554609cf4

```md
# Tasks — prototype-p1b-web-console

> 四栏纪律：每项标注 `Contract` / `Implementation` / `Verification` / `Gate`。

## 1. 静态文件面（transport.rs 前置层）

- [ ] 1.1 `static_response()` 前置层: `GET /` 内嵌页 + `GET /hls/*` 磁盘服务（路径穿越防护 + MIME + hls_dir 缺失 503）; 响应层 content-type/bytes 扩展; **五端点 route 表逐字符不变（测试锁定）** `Contract: A 方案裁定 / design D1` | `Implementation: 待` | `Verification: Unit——路由/穿越拒绝/MIME/503/route 表回归` | `Gate: 无`
- [ ] 1.2 `TransportContext.hls_dir` additive + main.rs 接线（PrototypeOutputConfig） `Contract: design D1` | `Implementation: 待` | `Verification: 编译 + 既有 TransportContext 构造点零破坏` | `Gate: 无`

## 2. wire 投影 + 页面

- [ ] 2.1 `ApiSession.outputs` + `to_api_session` 投影 `Contract: P1a verify §8 / design D3` | `Implementation: 待` | `Verification: Unit——投影透传` | `Gate: 无`
- [ ] 2.2 最小 Web Console 页（内嵌 const HTML: 轮询渲染 + outputs 物化事实 + hls.js 播放 + Start/Stop 经 commands 平台 + 诚实状态红线） `Contract: 用户十条 / design D2` | `Implementation: 待` | `Verification: Hardware gate P1b-01..07` | `Gate: P1b-01..07`

## 3. Gate 与验收

- [ ] 3.1 Hardware Gate P1b-01..08（盒上: 页面可开/状态真实/playlist+分片服务/ffmpeg-HLS-客户端实播/Start-Stop 联动/停后不虚报/P1a 回归） `Contract: proposal 验收场景` | `Implementation: 待` | `Verification: 盒上实跑全 PASS 入报告` | `Gate: P1b`
- [ ] 3.2 follow-up① IDR 对齐验证 + follow-up② 故障模式 gate `Contract: P1a verify §8 / design D4` | `Implementation: 待` | `Verification: 盒上实证记档` | `Gate: 无`
- [ ] 3.3 **Prototype-1 真机验收报告**（P1a+P1b 全证据 + 人眼浏览器指引） + CI 七 checks + verify + archive + PR + merge `Contract: 项目交付纪律` | `Implementation: 待` | `Verification: PR merged` | `Gate: CI/RELEASE`

```
