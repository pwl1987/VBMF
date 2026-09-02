# Tasks — prototype-p1b-web-console

> 四栏纪律：每项标注 `Contract` / `Implementation` / `Verification` / `Gate`。

## 1. 静态文件面（transport.rs 前置层）

- [x] 1.1 `static_response()` 前置层: `GET /` 内嵌页 + `GET /hls/*` 磁盘服务（路径穿越防护 + MIME + hls_dir 缺失 503）; 响应层 content-type/bytes 扩展; **五端点 route 表逐字符不变（测试锁定）** `Contract: A 方案裁定 / design D1` | `Implementation: 待` | `Verification: Unit——路由/穿越拒绝/MIME/503/route 表回归` | `Gate: 无`
- [x] 1.2 `TransportContext.hls_dir` additive + main.rs 接线（PrototypeOutputConfig） `Contract: design D1` | `Implementation: 待` | `Verification: 编译 + 既有 TransportContext 构造点零破坏` | `Gate: 无`

## 2. wire 投影 + 页面

- [x] 2.1 `ApiSession.outputs` + `to_api_session` 投影 `Contract: P1a verify §8 / design D3` | `Implementation: 待` | `Verification: Unit——投影透传` | `Gate: 无`
- [x] 2.2 最小 Web Console 页（内嵌 const HTML: 轮询渲染 + outputs 物化事实 + hls.js 播放 + Start/Stop 经 commands 平台 + 诚实状态红线） `Contract: 用户十条 / design D2` | `Implementation: 待` | `Verification: Hardware gate P1b-01..07` | `Gate: P1b-01..07`

## 3. Gate 与验收

- [x] 3.1 Hardware Gate P1b-01..08（盒上: 页面可开/状态真实/playlist+分片服务/ffmpeg-HLS-客户端实播/Start-Stop 联动/停后不虚报/P1a 回归） `Contract: proposal 验收场景` | `Implementation: 待` | `Verification: 盒上实跑全 PASS 入报告` | `Gate: P1b`
- [x] 3.2 follow-up① IDR 对齐验证 + follow-up② 故障模式 gate `Contract: P1a verify §8 / design D4` | `Implementation: 待` | `Verification: 盒上实证记档` | `Gate: 无`
- [x] 3.3 **Prototype-1 真机验收报告**（P1a+P1b 全证据 + 人眼浏览器指引） + CI 七 checks + verify + archive + PR + merge `Contract: 项目交付纪律` | `Implementation: 待` | `Verification: PR merged` | `Gate: CI/RELEASE`
