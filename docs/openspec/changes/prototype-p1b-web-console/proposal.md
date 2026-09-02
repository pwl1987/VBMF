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
