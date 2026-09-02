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
