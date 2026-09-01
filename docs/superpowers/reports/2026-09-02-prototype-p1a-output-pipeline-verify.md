# Verify 报告 — Prototype-1 P1a 输出管线（prototype-p1a-output-pipeline）

- **Change**: `prototype-p1a-output-pipeline`（full workflow, skip_specs:true）
- **分支**: `comet/prototype-p1a-output-pipeline`（base `16a8136` = master）
- **代码提交**: `0a71e27`（6 文件, +687/−9; 含 review gate 修复折入）
- **日期**: 2026-09-02
- **Design Doc**: `docs/superpowers/specs/2026-09-02-prototype-p1a-output-pipeline-design.md`（§3.4 probe 终裁）

## 0. 结论

**P1a 完成。** "输出意图 → 输出物化" 第一次真正打通：`sink.kind` 从被忽略的描述词变成被消费的契约词
（appsink/hls/rtmp 词表, 未知 fail-closed）。真 SDI → H.264/AAC 编码 → HLS 分片 + RTMP 推流全链真机实证
（Gate P1a-01..06 全 PASS）, 分析链零退化, 既有硬件回归全绿。**完成定义达成: 真实 SDI 已变成（编码后）
可消费的媒体。**浏览器可见的页面/服务属 P1b（本 change 范围外, 见 §8 后续）。

## 1. 四栏纪律表

| # | 任务 | Contract | Implementation | Verification | Gate |
|---|------|----------|----------------|--------------|------|
| 1.1 | OutputKind/OutputPlan + materialize 消费 sink.kind | design D1/D2 | 已 @ 0a71e27 | Unit ×7（hls/rtmp 物化、缺目标降级、appsink 保持、未知拒绝、词表快照含大小写、码率透传） | 无 |
| 1.2 | PrototypeOutputConfig | design D4 / 用户裁定 | 已 @ 0a71e27 | Unit ×4（默认/env 全设/非法码率回退/**launch 注入卫生拒绝**） | 无 |
| 2.1 | controller 串接（纯执行） | design D1/D3 | 已 @ 0a71e27 | 空 outputs ⇒ 今日串逐字节（review 逐字符比对确认）; bmd,gstreamer build PASS | 无 |
| 2.2 | 盒上 build 前 probe | design D6 | 已（Design §3.4） | rtmpsink/hlssink2/x264/openh264 属性面 + 接线实证 | 无 |
| 3.1 | 运行时可见性 | design D5 | 已 @ 0a71e27（review 修正后=**物化事实**投影） | Unit（hls 投影 + 降级不虚报）; 顶层 8 键测试原样绿 | 无 |
| 3.2 | main 诊断接线 | proposal | 已 @ 0a71e27 | VBMF_OUTPUT_KIND 经 cfg 单读路径; 无 env 行为逐字节一致 | 无 |
| 4.1 | Unit/Simulation 全绿 | 基线 | 已 | mock **237 passed**（基线 221 + 16 新, 零回退） | 无 |
| 4.2 | **Hardware Gate P1a-01..06** | proposal 验收场景 | 已（~/p1a_gate.sh, 不入库） | 盒上真机全 PASS（§2） | P1a-01..06 |
| 4.3 | 既有硬件回归零退化 | 验收口径 | 已 | 14 步矩阵全 exit 0 + E1-E8 ALL PASS + LOOPBACK PASS + transport gate 19 PASS/0 FAIL（含 D14 断言 rev2=3） | BOX |
| 5.1 | CI + verify + archive + PR + merge | 交付纪律 | 本报告 + 后续流程 | 见 §6 | CI/RELEASE |

## 2. Hardware Gate P1a-01..06（盒上真机, 2026-09-02, bmd,gstreamer 诊断模式）

```
PASS: P1a-01 real SDI session (SessionManager owner)
PASS: P1a-02 analysis chain alive (102 heartbeats, first-frame/PTS/health)
PASS: P1a-03 index.m3u8 present
PASS: P1a-03/05 segments sustained (rolling window 11 files)
PASS: P1a-03 real encoded H.264 (resolution follows live SDI feed)
PASS: P1a-03 real encoded AAC
PASS: P1a-05 appsink not stalled during encode (video=1483 audio=1498)
PASS: P1a-05 no ERROR/EOS over 65s
PASS: P1a-04 RTMP sustained receive (45393843 bytes)
PASS: P1a-04 received stream has h264+aac
PASS: P1a-04 rtmp session full lifecycle
PASS: P1a-05 rtmp run no ERROR
```

读数说明: RTMP 65s 收 45.4MB ≈ 6Mbps 码率吻合; appsink 1483/1498 帧 = 分析分支在编码期间持续流入
（queue 解耦结构性保证兑现）; 分片为滚动窗口（max-files=10 + index）。**盒上 SDI-IN-1 外部信号源
分钟级抖动且格式切换（1080p25 ↔ 720x486）**——P1a-03 断言分辨率无关（帧在流 + h264/aac 即成立）,
必要时自持环回发生器（`decklinkvideosink device-number=2 mode=1080p25`）。

## 3. 真机 probe 终裁（Design Doc §3.4, 自持信号交替复验）

1. **编码器 = openh264enc**（bitrate bps; 输出 Constrained Baseline yuv420p = 4:2:0 浏览器最大兼容）。
   x264enc 否决: 本机栈与任何格式 caps/option-string 约束互斥死锁（触发 decklinkvideosrc 运行中
   重协商 → Internal data stream error, 交替对照 3/3 vs 0/2）; 零 caps 时稳定但输出 High 4:4:4
   （MSE/Safari 不解码）。
2. **HLS = hlssink2 命名 request pad**（out.video/out.audio; 内部自带 mux, 外置 mpegtsmux `could not link`）。
3. **RTMP = rtmpsink**（属性面最简; 盒内 `ffmpeg -listen 1` 接收端 E2E）。

## 4. Review gate（standard, requesting-code-review 一次全 change）

裁决: **Ready to merge — With fixes**; 0 Critical / 2 Important / 6 Minor。全部处置:
- **Important#1（声明态投影失真）**: 已修复折入 0a71e27——`MediaSession.outputs` 由 start() 从
  materialize 产物回填, 投影**物化事实**; 降级会话投影空（新增测试锁定"不虚报"）。`OutputKind::as_str`
  由回填消费（连带 Minor#4）。
- **Important#2（fail-soft warn 默认不可见）**: 已修复折入——main 启动面对"声明输出但目标 env 缺失"
  显式 `println WARN`（默认订阅 ERROR 级下唯一可靠可见面）。
- Minor#3（sink_kind_override 死字段）: 已修复——main 两处诊断点统一经 `PrototypeOutputConfig` 单读路径。
- Minor#6（launch 注入卫生）: 已修复——`from_env_lookup` 拒绝空白/`!`/`"` 目标值（新测试 ×5 用例）。
- Minor#7（无谓 clone ×2）: 已修复。
- Minor#8（plan 文本漂移）: 已修复——plan Task 3 标注 probe 终裁修订。
- Minor#5（错误变体语义: 未知 kind 复用 IdentityUnresolved）: **接受**——与既有回滚机制/测试精确
  啮合, 词表增长时再立专属变体（记录于此）。
- Follow-up（review 建议非阻塞, 转 P1b）: (a) openh264enc 无 keyframe 间隔控制, 2s 分片 IDR 对齐
  待验证/`send-keyframe-requests`; (b) 共享 Bus 下编码分支故障可能拖累分析分支——P1b 前加失败模式 gate。

## 5. 测试账目

mock: 221 → **237**（+16: config ×4 / pipeline 物化 ×7 / launch ×4(+词表含在物化组) / 投影 ×1）。
default/simulation/bmd,gstreamer 计数不变（矩阵 155/155/155 实跑）。fmt apply 后 check clean
（本地树与盒零漂移）; 提交前 CRLF 自检 6 文件全 LF。

## 6. CI（PR 后 gh 实查——5.1 完成时回填）

七 required context（rust-format / rust-test-matrix / rust-clippy / session-lifecycle /
hardware-test-compile / architecture-portability / gstreamer-build）: **见 PR 检查记录（合并前全 green）。**

## 7. 不变量与红线复核

- **向后兼容红线**: 无任何 `VBMF_OUTPUT_*` ⇒ launch 串逐字节 = P1a 前（review 逐字符比对）;
  E1-E8/LOOPBACK/transport 回归全过 = 行为不变实证。
- **分析零退化**: 分析分支 element 串逐字符不动（测试锁定）; P1a-05 appsink 帧持续断言。
- **词表有牙齿**: appsink/hls/rtmp 快照锁定（含大小写敏感）; 未知 fail-closed 经既有回滚链零孤儿。
- **顶层 8 键契约不动**（D14 锁定延续）; transport/五端点零触碰; Federation 继续 BLOCKED。
- **用户边界修正兑现**: controller 零编码/输出 element 名（纯拼接）; 物化+launch 构造全在 domain 层。
- **demo 参数不进 Runtime Contract**: PrototypeOutputConfig env 层, 正式契约化留产品配置模型阶段。

## 8. 范围外登记（P1b 输入）

- 最小 Web 页（GET / + /hls/* 静态面, A 方案同端口静态文件裁定已获用户批准）+ HLS 浏览器播放验证
- `ApiSession` wire 投影补 outputs（canonical 已有, API DTO 侧 P1b 消费时加）
- §4 Follow-up (a)(b)
