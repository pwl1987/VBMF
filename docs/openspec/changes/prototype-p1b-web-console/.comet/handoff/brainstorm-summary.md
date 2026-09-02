# Brainstorm Summary

- Change: prototype-p1b-web-console
- Date: 2026-09-02

## 确认的技术方案

（用户裁定 A 方案 + P1a probe 事实; brainstorming 技能盘上不存在, 按 v03-d14/p1a 前例 handoff+Design Doc 同态, 用户已预授权自主推进）

- 静态面前置层 static_response()（GET / 内嵌页 + GET /hls/{单文件} 磁盘服务）; 五端点 route 表逐字节不变
- TransportContext.hls_dir additive（main 诊断接线; 生产 None → /hls/* 503 契约诚实）
- ApiSession.outputs 物化事实投影（P1a verify §8 登记项）
- 页面: 原生 HTML+JS 单文件; 1s 轮询 /health + /api/v1/runtime; hls.js CDN + Safari 回退; Start/Stop 全走既有 POST /api/v1/commands（零新 API）; sink kind 由 /hls/index.m3u8 可达性诚实推导
- 路径穿越防护: 文件名白名单字符 + 无子目录 + 拒 ..
- follow-up: FU-1 IDR 逐分片验证; FU-2 RTMP 无接收端故障模式（复用 Supervisor 恢复语义）

## 关键取舍与风险

- hls.js CDN 依赖（离线浏览器如实 UNAVAILABLE; 人眼验收在有网环境）
- 大分片单线程写（局域网可接受, 正式化记档）
- 无 headless 浏览器 → ffmpeg 真 HLS 客户端作程序化实播证据 + 人眼指引入报告

## 测试策略

Unit（路由/穿越/MIME/503/route 表回归/投影）+ Hardware Gate P1b-01..08 + FU-1/2 + P1a 回归 + Prototype-1 真机验收报告（含 LAN bind 人眼指引）

## Spec Patch

无（skip_specs:true 同前例）
