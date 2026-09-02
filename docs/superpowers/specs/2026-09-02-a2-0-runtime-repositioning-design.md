---
comet_change: a2-0-runtime-repositioning
role: technical-design
canonical_spec: openspec
archived-with: 2026-09-02-a2-0-runtime-repositioning
status: final
---

# Design Doc — a2-0-runtime-repositioning（A2-0 四层归位）

基线 master `f3f86ef`。用户 2026-09-02 裁定四刀; **行为零变红线**; 不改 V0.2 不增媒体能力。

## 1. 现状区块地图（probe 实测, main.rs 1804 行 / 105,837 字节）

| 行区间 | 内容 | 归宿 |
|--------|------|------|
| L1-65 | 陈旧头注释（"Gate 2 skeleton"）+ mod 声明 + imports | → lib.rs + 头注释重写 |
| L66-112 | main 入口: tracing/config/mode | 留 main |
| L113-268 | C1 诊断探针 + Resolver Evidence + HW-PORT-01 报告（cfg bmd-provider） | → `diagnostics.rs`（原位置调用） |
| L269-505 | VBMF_LOOPBACK gate（HW-PORT-01D, cfg gst, env 门控） | → `gates.rs`（gates bin） |
| L506-517 | hardware-test registry + VBMF_REGISTRY_ONLY | → `gates.rs` |
| L518-551 | Supervisor 构建 + /health 最简监听遗迹 | 留 main（组合根） |
| L552-617 | MEDIA-RT-01 自测 + CAP-01 SDK 探针（诊断 boot 常跑） | → `diagnostics.rs` |
| L618-726 | mode 解析 + gst_probes + manifest/bindings 构建 | → `bootstrap.rs` |
| L727-1170 | VBMF_SESSION_LIFECYCLE gate（E1-E8 全系, env 门控） | → `gates.rs` |
| L1171-1450 | 诊断 auto-start + api_mgr + EXTERNAL-API 证据 + transport 接线 | 留 main（证据打印 → diagnostics.rs; 接线留） |
| L1451-1804 | `spawn_ingest_watchdog` 全实现 + 辅助 | → `watchdog.rs` |

## 2. 目标形态

```
src/lib.rs            # 全模块根（含新 diagnostics/bootstrap/gates/watchdog; A2-1 腾位注释锚）
src/bin/media-agent.rs    # 组合根: bootstrap→diagnostic wiring→runtime wiring→transport→lifetime（A20-04 归位终态; 行数为 review target 非硬指标）
src/bin/gates.rs      # 薄壳: env(VBMF_SESSION_LIFECYCLE/VBMF_LOOPBACK/VBMF_REGISTRY_ONLY)→lib::gates
src/diagnostics.rs    # C1/CAP-01/MEDIA-RT-01 自测/EXTERNAL-API 证据（原条件原位置调用）
src/bootstrap.rs      # DiagnosticWorld{cfg,mode,devices,bindings,registry,lm,sup,logs,mgr,ctrl,agent_state}
src/gates.rs          # 两 gate 逐字节搬运（ctx 参数化; cfg(feature gstreamer-backend) 门控保持）
src/watchdog.rs       # spawn_ingest_watchdog 逐字节（签名不变）
Cargo.toml            # [lib] + [[bin]] media-agent + [[bin]] media-agent-gates; description 修正
```

## 3. 关键决策

### 3.1 gates 放 lib 模块而非 bin 内联（路径零漂移）
gate 代码 ~900 行大量 `crate::` 引用——lib 模块内天然成立; bin 薄壳 `media_agent::gates::...`。生产 bin 不引用 gates 模块 ⇒ 运行时零 gate 行为（A20-02 证明 = 生产路径无 gate env 分支 + 日志对照）。

### 3.2 diagnostics 不进 gates bin（行为零变约束）
C1 探针/MEDIA-RT-01 自测/EXTERNAL-API 证据在**每次诊断 boot 都跑**（非 env 门控）——迁 bin 会改变生产行为 ⇒ 只搬代码位置（模块化）不搬执行位置。

### 3.3 bootstrap 单一构建器
main 与 gates 的诊断世界构建完全同源（复制=漂移）; `build_diagnostic() -> DiagnosticWorld` 唯一。生产/诊断差异分支留在 main。

### 3.4 watchdog 签名不变
`spawn_ingest_watchdog(ctrl, handle, device_uuid, sup, lm, agent_state, event_sink, internal_log)` 原样; Supervisor 决策边界零触碰。

## 4. 验证策略

- **A20-01 行为零变**: 盒上 f3f86ef 二进制 vs 新二进制, 同 env 诊断 boot 日志**逐段 diff**（时间戳/uuid 行除外）等价。
- **A20-02 gates bin**: `media-agent-gates` + SESSION_LIFECYCLE → E1-E8 ALL PASS; + LOOPBACK → PASS; grep 生产 main.rs 无 VBMF_SESSION_LIFECYCLE/VBMF_LOOPBACK 字样（符号级: strings 生产二进制可含 gates 符号——lib 整体链接; 语义证明 = 生产路径零 gate env 分支, 以代码审查+日志对照为准, 如实记录此限度）。
- **A20-03..06**: P1a/P1b/A1 回归电池（p1a/p1b/a1 gate 用生产 bin 诊断 boot, 不受影响; 生命周期/loopback 回归换 gates bin 调用）+ 矩阵 + mock 251 + CI。

## 5. 风险

| 风险 | 缓解 |
|------|------|
| 大块搬运漏改/语义漂移 | 逐字节搬运原则（只动缩进/路径）; A20-01 日志对照; 全回归电池 |
| gates ctx 参数化引借用冲突 | DiagnosticWorld 拥有权转移/克隆策略在实现期编译器裁定 |
| 双 bin 链接体积 | 无害（原型）; 记档 |
| 盒脚本旧名调用失效 | 3.2 步统一换名 + 回归验证 |

## 6. 冻结点

- V0.2 / 五端点 / commands / Runtime 语义零触碰; 七条红线不动。
- Program Domain 只腾位（lib.rs 注释锚）零类型。
- 生产 bin 行为 = f3f86ef 逐段等价（唯一允许差异: 模块化后 tracing target 前缀变化——若出现, A20-01 对照规则记为可接受差异并逐项列出）。
