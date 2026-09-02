---
comet_change: prototype-p1b-web-console
role: technical-design
canonical_spec: openspec
---

# Design Doc — prototype-p1b-web-console（P1b 最小 Web Console）

基线: master `6fc863b`（P1a 已收口）。A 方案已由用户裁定（同一 std-only 端口增加静态文件面）。

## 1. 现状锚点（probe 实证 @6fc863b）

- `transport.rs route()`: `(method, path)` 精确匹配 match; `_ => (404)`; 响应层硬编码 `Content-Type: application/json` + String body。
- `TransportContext{events, agent_state, device_count, query, idem}` — main.rs 诊断路径构造。
- Command 平面: `CommandKind{StartSession, StopSession, ReleaseSession}`; `POST /api/v1/commands` 全链可用（C8 真机实证 Executed/Rejected）; StartSession 需 `Session{intent}` 目标, Stop/Release 需 `SessionById`。
- `ApiSession{id, state, phase}` — 无 outputs（canonical `SessionRuntimeState.outputs` 物化事实 P1a 已建）。
- P1a 产物: `VBMF_OUTPUT_HLS_DIR` 分片目录（`index.m3u8` + `seg%05d.ts`）; 盒上无浏览器（headless 不可用）。

## 2. 静态文件面前置层（transport.rs）

### 2.1 结构（零触碰五端点）

```rust
/// P1b: 静态文件面前置层 —— 在既有 route() 之前拦截（A 方案裁定）。
/// 仅处理 GET /（内嵌页）与 GET /hls/*（分片服务）; 其余返回 None 落入 route()
/// （五端点冻结语义逐字节不变）。静态文件面不是 API 资源（无幂等/无命令）。
fn static_response(method: &str, path: &str, hls_dir: Option<&str>)
    -> Option<(u16, &'static str, Vec<u8>)>
```

- `GET /` → `(200, "text/html; charset=utf-8", INDEX_HTML.as_bytes().to_vec())`; 非 GET → `(405, ..., b"method_not_allowed")`。
- `GET /hls/{rest}`:
  - `hls_dir` 缺失 → `(503, "application/json", b"{\"error\":\"hls_dir not configured\"}")`（契约诚实风格同 not_available）。
  - **路径穿越防护**: `rest` 拒绝空段/`..`/包含 `/` 之外的非法构造（只允许 `[A-Za-z0-9._-]+` 白名单字符 + 单文件名, 拒绝子目录/绝对路径/编码穿越）。
  - 文件读取失败 → 404; 成功 → MIME 按扩展名（`.m3u8` → `application/vnd.apple.mpegurl`; `.ts` → `video/mp2t`; 其他 → 404 拒绝, 不发明通用文件服务）。
- serve_connection: 先 static_response, Some → 按 `(status, content_type, bytes)` 写; None → 既有路径（`Content-Type: application/json` 逐字节不变, String body → bytes 写出——wire 输出不变）。

### 2.2 响应头

```
HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {n}\r\nConnection: close\r\n\r\n{bytes}
```

（现有五端点的字节输出与 P1b 前完全一致——content-type 同为 application/json, body 同内容。）

## 3. TransportContext + 接线

`pub hls_dir: Option<String>`（additive; `TransportContext{..}` 构造点: main.rs 诊断路径——从 `PrototypeOutputConfig::from_env()` 读一次传入; 测试构造点补 `None`）。production 路径 `hls_dir: None`（生产无诊断输出, `/hls/*` 503 契约诚实）。

## 4. ApiSession 加法

```rust
pub struct ApiSession { pub id: String, pub state: String, pub phase: String,
    /// P1b: 物化输出事实投影（空 = 纯分析/降级, 绝不虚报）。
    pub outputs: Vec<String> }
```
`to_api_session` 补 `outputs: s.outputs.clone()`。additive 非破坏（wire 新键, 旧客户端忽略）。

## 5. Web Console 页（内嵌 const HTML, 无构建链）

单文件原生 HTML+JS, 结构对应用户面板草图:
- **头部状态行**（1s 轮询 `/health` + `/api/v1/runtime`）: AGENT state / devices 数 / CH01 = 首个 session 的 `state/phase` / **OUTPUT = sessions[0].outputs.join()（空 → "ANALYSIS-ONLY" 如实）** / SDI 信号（/health.state 推导 READY/DEGRADED）。
- **视频区**: `<video>` + hls.js（jsdelivr CDN; `video.canPlayType('application/vnd.apple.mpegurl')` 原生回退 Safari）; source=`/hls/index.m3u8`; hls 错误 → 视频区如实显示 "HLS UNAVAILABLE"（不假画面）。
- **控制区**: `[Start]` → POST commands `{kind:"start_session", target:{target_type:"session", intent:{version:"1.0", devices:[{device_id:<首设备id>, role:"CAPTURE", pipeline:{source:{kind:"decklink", device_id:<id>}, sink:{kind:<probe>}}}]}}}`——`<probe>` = 页面探测 `/hls/index.m3u8` 可达 ⇒ "hls" 否则 "rtmp"（页面无 env 可见, 以服务可达性诚实推导）; `[Stop]` → `{kind:"stop_session", target:{target_type:"session_by_id", session_id:<runtime 首会话 id>}}`。按钮反馈 = 命令响应（Executed/Rejected/Failed 如实显示）。
- **诚实红线**: 页面无任何本地缓存的状态推断; 一切显示源于最近一次轮询; 会话停止 → phase 停止 + outputs 不变(物化历史)或会话释放后消失, 视频冻结——状态如实。

## 6. 测试策略

- Unit（transport.rs tests）: `/` 200+html; `/hls/` 缺 dir 503; 穿越样本（`..`、`../x`、空、子目录 `a/b`、非法字符）拒绝; m3u8/ts MIME; 未知扩展 404; 非 GET 405; **五端点 route 表既有测试逐字符不动原样绿**; ApiSession 投影。
- Hardware（盒 `~/p1b_gate.sh` 不入库）: P1b-01 `/` 200 text/html; P1b-02 页面 JS 引用的两个 API 端点活; P1b-03 `/hls/index.m3u8` 200 正确 MIME + 分片 200 与磁盘字节一致; P1b-04 **ffmpeg 真 HLS 客户端**（`ffmpeg -i http://127.0.0.1:8080/hls/index.m3u8 -t N -f null -` 解码出帧计数 > 0 = 实播）; P1b-05 Start（commands Executed + session running + outputs=["hls"]）; P1b-06 Stop（Executed + phase 停止 + 页面数据源如实）; P1b-07 停后轮询 `/api/v1/runtime` 无 running 会话/无虚报; P1b-08 P1a gate 全回归; FU-1 逐分片首帧 IDR 验证; FU-2 RTMP 无接收端故障模式（事件/终态诚实, 无悬挂）。
- **人眼浏览器验收**（报告指引, 非自动 gate）: 盒上 `MEDIA_AGENT_HEALTH_BIND=10.30.15.10:8080` 启动, 用户浏览器开 `http://10.30.15.10:8080/`（内网; 生产经反代+认证的约束不变）。

## 7. 风险与缓解

| 风险 | 缓解 |
|------|------|
| 静态面破坏五端点冻结 | 前置层 + 既有 route 测试逐字符锁定 + 回归 gate |
| 路径穿越读任意文件 | 文件名白名单字符集 + 无子目录 + 拒绝 `..`（Unit 样本锁定） |
| hls.js CDN 不可达（离线浏览器） | 页面如实显示 UNAVAILABLE + 原生 Safari 回退 + 人眼验收在有网环境 |
| Stop 后媒体缓存让画面"看似还在播" | HLS 滚动窗口停更 → 播放器缓冲耗尽自然冻结（如实）; 状态行显示 stopped |
| 大分片读取阻塞单线程 listener | 分片 ~1.4MB std 读 + 单写, 局域网可接受; Prototype 记档（正式化时加超时/分块） |

## 8. 契约冻结点

- 五端点 route 表 + 响应字节逐字节不变（测试锁定）。
- 静态面仅 `GET /` 与 `GET /hls/{单文件名}`; 不发明目录列表/上传/通用文件服务。
- `/hls/*` 在 hls_dir 未配置时 503（生产路径契约诚实）。
- commands 平台零改动（页面复用既有词汇与幂等白盒）。
