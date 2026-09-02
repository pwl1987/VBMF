# Verify 报告 — Alpha-1 多输入 + Channel 模型（alpha1-multi-input-channel）

- **Change**: `alpha1-multi-input-channel`（full workflow, skip_specs:true）
- **分支**: `comet/alpha1-multi-input-channel`（base `d2a24fb` = master, Prototype-1 收口点）
- **代码提交**: `b4e0f0a`（6 文件 +482/−53; review 修复折入）
- **日期**: 2026-09-02
- **Design Doc**: `docs/superpowers/specs/2026-09-02-alpha1-multi-input-channel-design.md`

## 0. 结论

**D10 债务关闭, Alpha-1 达成。** 多设备会话实例化**全部**管线（不再只取首计划）, 每管线句柄表
+ wire 投影 + 控制台输入行; 单输出承诺延续（仅首设备物化输出, P1a/P1b gate 零回退）。
真机实证: **两张真实 SDI 卡同时进入同一 VBMF 会话**（inputs=2）, 主输入编码输出持续,
次管线在外部信号抖动下诚实呈现, Stop 全句柄零孤儿。

## 1. Gate A1-01..07（盒上真机, 2026-09-02, 复跑含 review 修复）

```
PASS: A1-01 双输入会话 running + inputs=2 (running 2 hls)
PASS: A1-02 主输入帧流在场 (心跳) + 次管线诚实 (inputs handles=1,2; SignalLost 信号实况如实)
PASS: A1-03 首输入 HLS 输出持续（单输出承诺）
PASS: A1-04 单输入信号抖动下会话持续如实 (running 2)
PASS: A1-05a Stop executed（全句柄逆序停）
PASS: A1-05b 停后无 running 会话（零虚报）
PASS: A1-06 控制台输入行在场
回归: P1a gate 12 PASS + P1b gate 11 PASS + 14 步矩阵 14/14（复跑确认）
mock: 251 passed（基线 245 + 6 新）
```

**盒配置变更（物理核实记录）**: 新增 `~/loopback-manifest-v3.json`——SDI-IN-2 =
bmd handle `46:00000000:002e4400`（自证据日志比对: 与 SDI-IN-1 `…4400/…4500` 同型号邻序）,
gst device-number=1（实证 device 1 可采集收帧 + 消去法: 0=SDI-IN-1 已绑定, 2=MINI-MON）。
沿 v2 生成惯例（v2 保留不动）。

## 2. Review gate（standard, 一次全 change）

裁决 **With fixes**: 1 Critical / 1 Important / 5 Minor, 全部处置:
- **Critical#1（allocate 失败回滚只停首句柄——1..N-1 管线+GLib 线程泄漏）**: 已修复折入
  ——三条失败路径（instantiate 中途 / backend.start / allocate）全部逆序全句柄 stop;
  新增注入测试 `session_rt_01_multi_device_allocate_failure_stops_all_handles`
  （foreign takeover 注入, stops==2 + 租约全还 + StartFailed）锁定。
- **Important#2（无绑定设备时 intent 落空, 破坏单输入兼容冻结点）**: 已修复——空 bound
  回退 `devices.first()` + 显式 WARN; `VBMF_DIAG_INPUTS` 非法值 WARN 后按 1。
- Minor#3（回滚循环顺序不一致）: 已修复——三处统一 `.rev()`。
- Minor#5（控制台注释与实现不符）: 已修复——注释如实（uuid 前 8 位; 聚合色属 Alpha-2）。
- Minor#4（nil-UUID 防御回退）: **接受**——create_inner 已保证可解析（review 确认死防御）,
  保留防御不 panic。
- Minor#6（次输入无独立监督 + LAST_FATAL_BUS_EVENT 全局单槽）: **接受并记档**——
  D10 CLOSED 行显式列遗留（转 Alpha-2+）; 与保守子集范围一致。
- Minor#7（stop TOCTOU 预存在）: **接受**——与改动前结构一致, 多句柄未加剧; 记档。

## 3. 关键发现（过程）

- **bootstrap 占位租约只让位首设备**（单输入时代代码）⇒ 双输入 LeaseConflict——gate 实跑
  抓出, 修为全部诊断输入让位。
- Channel 命名 = 控制台侧规约（CH+显示序）, 运行时状态不携带序（HashMap 无序防漂移）——
  设计裁决记档（Design §2.2）。
- V0.2 Channel 全语义（failover/standby/FAILED 聚合）不进本 change（Alpha-5/V0.3）。

## 4. 不变量与冻结点复核

- 单输入行为兼容: 无 `VBMF_DIAG_INPUTS` ⇒ 现诊断路径（含空绑定回退, review Important#2
  修复后真正成立）; P1a+P1b gate 全 PASS = 输出/控制台行为不变实证。
- 顶层 8 键 / 五端点 / commands 平面零触碰; 单输出承诺（次设备词表 fail-closed 不豁免,
  输出段强制空 + warn）。
- 零孤儿: 三失败路径全句柄回滚 + stop 全停（Unit ×4 锁定: 双实例化/中途回滚/allocate 注入/全停）。

## 5. CI（PR 后回填）

七 required context: **见 PR 检查记录（合并前全 green）。**
