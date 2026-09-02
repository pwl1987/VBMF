---
archived-with: 2026-09-02-prototype-p1b-web-console
status: final
---
# 执行计划 — prototype-p1b-web-console

---
base-ref: 6fc863b9d05d426d4d55b7dd64b0ea4b25816b8e
comet_change: prototype-p1b-web-console
design_doc: docs/superpowers/specs/2026-09-02-prototype-p1b-web-console-design.md
---

> TDD 硬律; cargo 经盒; 基线 mock 237 / FMT_CLEAN。

## Task 0 基线 ✓（237 / FMT_CLEAN / HEAD=6fc863b）

## Task 1 静态面前置层（transport.rs, TDD）

- [x] 1.1 RED: static_response 单测——`GET /` 200 html 含 VBMF 标识; 非 GET `/` 405; `/hls/x` 缺 dir 503; 穿越样本（`..`/`a/b`/空/非法字符/绝对路径）404-拒绝; `.m3u8`/`.ts` MIME; 未知扩展 404; 其他 path None（落 route()）
- [x] 1.2 GREEN: `static_response()` + `INDEX_HTML` const + serve_connection 前置接线 + content-type/bytes 响应; **既有 route 测试逐字符原样绿**
- [x] 1.3 `TransportContext.hls_dir` + 构造点（main 诊断接线 + 测试补 None）

## Task 2 ApiSession.outputs（TDD）

- [x] 2.1 RED+GREEN: ApiSession.outputs + to_api_session 投影单测

## Task 3 Web Console 页（INDEX_HTML 内容）

- [x] 3.1 页面: 轮询渲染（/health + /api/v1/runtime; CH01/SDI/OUTPUT 物化事实/ANALYSIS-ONLY 如实）+ hls.js(CDN)+Safari 回退 + Start/Stop 经 commands（sink kind 由 /hls/index.m3u8 可达性推导）+ 错误如实显示
- [x] 3.2 盒上无 UI 单测——页面正确性由 Hardware gate 承载

## Task 4 Hardware Gate P1b（盒 ~/p1b_gate.sh 不入库）

- [x] 4.1 P1b-01..03: `/` 200 html; 两 API 活; /hls/index.m3u8 200 正确 MIME + 分片与磁盘字节一致
- [x] 4.2 P1b-04: ffmpeg 真 HLS 客户端经 HTTP 实播（帧计数 > 0）
- [x] 4.3 P1b-05..07: Start（Executed+running+outputs=["hls"]）/ Stop（Executed+phase 停止）/ 停后无虚报
- [x] 4.4 P1b-08: P1a gate 全回归 + 既有矩阵
- [x] 4.5 FU-1 分片 IDR 逐片验证; FU-2 RTMP 无接收端故障模式诚实性

## Task 5 交付

- [x] 5.1 review gate + 单代码 commit + build/verify guard + verify 报告=**Prototype-1 真机验收报告**（含人眼 LAN bind 指引）+ archive + PR + CI + merge + 删分支
