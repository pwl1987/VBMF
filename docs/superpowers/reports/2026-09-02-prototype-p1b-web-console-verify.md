# Verify 报告 — Prototype-1 P1b 最小 Web Console（= **Prototype-1 真机验收报告**）

- **Change**: `prototype-p1b-web-console`（full workflow, skip_specs:true）
- **分支**: `comet/prototype-p1b-web-console`（base `6fc863b` = master, P1a 收口点）
- **代码提交**: `3fca86a`（4 文件 +314/−15; review 修复折入）
- **日期**: 2026-09-02
- **Design Doc**: `docs/superpowers/specs/2026-09-02-prototype-p1b-web-console-design.md`

## 0. 结论 — Prototype-1 达成

**"打开浏览器就能看到 VBMF 在工作"成立。** A 方案（同一 std-only 端口）静态文件面上线：
`GET /` 最小控制台（活轮询真实状态 + HLS 播放 + Start/Stop 经既有 commands 平台）+
`GET /hls/*` 分片服务。Gate P1b-01..07 + FU-1/2 真机全 PASS（含 **ffmpeg 作为真实 HLS
客户端经 HTTP 拉流解码 242 帧**——不是仅 HTTP 200）; P1a 全 Gate + 既有矩阵/门禁回归零退化。
叠加 P1a（真 SDI→编码→HLS/RTMP）, **Prototype-1 完整收口**。

## 1. Prototype-1 全链证据总账（P1a + P1b）

```
真实 SDI → DeckLink → GStreamer tee ──→ appsink 分析（103 心跳存活, P1a-02）
                                      └──→ openh264enc+avenc_aac ──→ hlssink2 分片（IDR 7-8/8 对齐）
                                                                    └→ rtmp2sink 推流（45.7MB 持续收流）
GET /（控制台: CH01/SDI/OUTPUT 物化事实/ANALYSIS-ONLY 如实）→ hls.js → 浏览器画面
GET /hls/*（playlist+分片, 字节与磁盘 cmp 一致）→ ffmpeg 真 HLS 客户端 242 帧实播
POST /api/v1/commands（Start→running+outputs=[hls]; Stop→released 不虚报）
```

## 2. Gate P1b-01..07 + FU-1/2（盒上真机, 2026-09-02, 复跑含 review 修复）

```
PASS: P1b-01 / opens (200 text/html)
PASS: P1b-02 /health + /api/v1/runtime live
PASS: P1b-03a playlist 200 + 正确 MIME (application/vnd.apple.mpegurl)
PASS: P1b-03b 分片字节与磁盘一致 (cmp)
PASS: P1b-04 真 HLS 客户端经 HTTP 解码 242 帧（三跑 199/181/242）
PASS: P1b-05a Start 命令 executed（经 commands 平台）
PASS: P1b-05b 新会话 running + outputs=[hls] 物化事实
PASS: P1b-06 Stop 命令 executed
PASS: P1b-07 停后 phase 如实 (released, 非虚报 running)
PASS: FU-1 分片首帧全部 IDR 对齐 (7/7~8/8; gop-size=50 + send-keyframe-requests=true
      + min-force-key-unit-interval; 注: 初版 0/N 为 ffmpeg showinfo 输出格式 grep 假象,
      实证格式 "n: 0 ... type:I" 后修正)
PASS: FU-2 诚实不虚报（目标不可达: rtmp2sink timeout=5 显式 Connection refused @gst-launch
      级实证; agent 级跑到无信号分支: Signal lost + 零帧流, 绝不假装在输出——
      盒上 SDI 外部源分钟级抖动, 两分支皆为诚实行为, 断言不变量=无不诚实）
回归: P1a gate 01-05 全 PASS（rtmp2sink 下 45.7MB 收流; appsink 1496 帧编码期不停滞）
      + 14 步矩阵 14/14 + SESSION_LIFECYCLE 0 + LOOPBACK 0 + transport 19 PASS/0 FAIL
mock: 245 passed（基线 237 + 8 新: 静态面 ×7 + /health wire 大小写锚 ×1）
```

## 3. Review gate（standard, 一次全 change）

裁决 **With fixes**: 0 Critical / 4 Important / 4 Minor, 全部处置:
- **Important#1（/health 状态大小写失配, SDI 永不亮绿）**: 修复——JS 小写比较 + 新增
  `transport_rt_01_health_state_wire_casing_anchor` 锚测试（防复发; curl 级 gate 结构性
  看不到的浏览器侧 bug, review 独立抓出）。
- **Important#2（crypto.randomUUID 仅安全上下文, LAN HTTP 下 Start/Stop 失效）**: 修复——
  getRandomValues v4 回退。
- **Important#3（sessions[] HashMap 无序, Stop 可能选已释放会话）**: 修复——stop() 与
  poll() 同样优先 running 会话; "插入序"错误注释删除。
- **Important#4（串行 accept 无 socket 超时, 停滞读者可卡死唯一 listener）**: 修复——
  accept 后 set_read/write_timeout(10s/30s)。
- Minor#5 CDN 版本浮动: 修复——钉 `hls.js@1.5.20`（integrity hash 需联网计算, 记为残余项）。
- Minor#6 停止后 OUTPUT 仍绿: 修复——仅 running 会话的输出亮绿。
- Minor#8 Start 双击: 修复——命令在途禁用按钮。
- Minor#7 撕裂 playlist 读/9 测试样本增强: 接受记档（hls.js 下轮询自愈; 残余项）。

## 4. 探针终裁增补（本 change）

- **RTMP sink = `rtmp2sink timeout=5`**（P1a 用 rtmpsink）: rtmpsink 无 timeout 属性且无接收端
  时**静默停滞**; rtmp2sink 5 秒内显式 Connection refused bus Error（盒上交替实证）——
  诚实失败进入既有 Supervisor 恢复语义。E2E 收流 45.7MB 不回退。
- **IDR 对齐三件套**: `openh264enc gop-size=50 + min-force-key-unit-interval=2000000000` +
  `hlssink2 send-keyframe-requests=true`。
- **ffmpeg showinfo 关键帧格式**为 `type:I`/`iskey:1`（非 `pict_type:I`）——测量坑记档。

## 5. 五端点冻结与安全复核

- `route()` 零 diff; JSON 响应字节逐字节不变（review 逐项比对 + 既有测试原样绿 +
  `transport_rt_01_static_other_paths_fall_through` 锁定）。
- 路径穿越: parse_request **不做百分号解码** + 文件名白名单字符集 + 首字符非 `.` +
  仅 .m3u8/.ts 扩展 + `Path::join` 单组件 —— review 逐向量核验（%2e%2e/反斜杠/空字节/
  多字节均拒）。
- 页面 XSS: 全部动态值经 `textContent`, 无 innerHTML（review 确认）。
- 生产路径 `hls_dir=None` ⇒ `/hls/*` 503 契约诚实。

## 6. 人眼浏览器验收指引（程序化 gate 之外的最后一步）

盒上（内网）:
```bash
MEDIA_AGENT_MODE=diagnostic \
MEDIA_AGENT_DEVICE_BINDING=~/loopback-manifest-v2.json \
MEDIA_AGENT_HEALTH_BIND=10.30.15.10:8080 \
VBMF_OUTPUT_KIND=hls VBMF_OUTPUT_HLS_DIR=/tmp/p1a-hls \
./target/debug/media-agent
```
浏览器（有外网, hls.js CDN）打开 `http://10.30.15.10:8080/`: 期望 CH01 状态行亮绿
（AGENT Ready / SDI ● LOCKED / SESSION running / OUTPUT HLS）+ 视频区出画面 + Start/Stop
按预期联动。安全约束不变: 内网原型面; 生产经 Fastify/Nginx 反代+认证（用户 §二十二）。

## 7. 残余项（Prototype-1 后, 不阻塞）

- hls.js CDN integrity hash（需联网计算后补）; 离线浏览器显示 HLS UNAVAILABLE（如实）
- 撕裂 playlist 读（hlssink2 原地重写; hls.js 自愈）; 单 listener 串行模型正式化（超时已加固）
- agent 级 FU-2 "有信号+无接收端"分支依赖外部信号窗口（gst-launch 级已实证 rtmp2sink
  Connection refused; agent 级诚实不变量断言已覆盖）
- ApiCommandStatus wire 扩展（如后续 UI 需要更多命令反馈细节）

## 8. CI（PR 后回填）

七 required context: **见 PR 检查记录（合并前全 green）。**
