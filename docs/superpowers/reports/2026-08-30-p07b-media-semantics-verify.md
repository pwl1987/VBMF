# Verify 报告 — p07b-media-semantics (Phase 0.7B-1: Normalize Foundation)

- 日期: 2026-08-30
- 分支: `comet/p07b-media-semantics`（自 master `adc6f19` = 0.7A baseline 拉出）
- 验证模式: **full**；盒 10.30.15.10 为准
- 契约对齐: CANONICAL_MEDIA_MODEL §1/§2/§4/§5（FROZEN）；终审三纪律（Normalize 不吞 Intent / Audio 独立 Flow / Clock 不被 Backend 偷走）

## Summary

| Dimension | Status |
|-----------|--------|
| Completeness | 4 任务组 9 项全落地（四栏纪律全 Pass） |
| Correctness | 盒上 final 矩阵：fmt 0 · test **121/121/141/121** · clippy -D ×4 零警告 · build ×3 · PROOF PASS · 真机 loopback 门禁 GATE_EXIT=0 + normalize 路径真机运行 |
| Coherence | 实现逐条对齐 D1-D6；三纪律由类型系统与测试强制；Session/Resource/Lease 路径零触碰 |

**结论: PASS — 0 CRITICAL / 0 IMPORTANT / 2 NOTE。**

## 1. 完整验证 7 项

| # | 检查项 | 结果 | 证据 |
|---|--------|------|------|
| 1 | tasks 全部完成 | ✅ | 四栏纪律表全 Pass（Implementation Complete / Verification 三层 / Gate Pass） |
| 2 | 符合 open design.md | ✅ | D1-D6 逐项落地；非目标未越界 |
| 3 | 符合 Design Doc | ✅ | §1-§6 实现级一致（帧率结构化 (num,den)、观测缺失 Unknown+WARN、clock 占位） |
| 4 | 能力规格场景 | ✅ (N/A) | skip_specs；NORMALIZE-RT-01 门禁测试即场景 |
| 5 | proposal 目标满足 | ✅ | CanonicalMediaDescriptor + normalize_input + NORMALIZE-RT-01 三层全交付 |
| 6 | delta spec 无矛盾 | ✅ (N/A) | 无 delta spec |
| 7 | Design Doc 可定位 | ✅ | frontmatter 关联本 change（design guard PASS） |

## 2. 盒上最终矩阵（final code, 全绿）

fmt apply/check **0** · test **121 (default) / 121 (simulation) / 141 (mock) / 121 (bmd,gstreamer)**（0.7A 基线 115/115/131/115 → +6 normalize 门禁测试）· clippy -D ×4 零警告 · build ×3（含 hardware-test）· remove-adapter PROOF PASS。

## 3. 门禁 NORMALIZE-RT-01 逐层验收

| 层 | 测试/证据 |
|----|-----------|
| Unit | `normalize_rt_01_provider_independent_same_media_same_descriptor`（BMD 形状 vs Mock 形状 → video/audio/clock/transport 逐字段相等 + provider 绑定引用不渗入）· `missing_observed_unknown_not_fabricated`（全 Unknown + WARN，绝不默认格式）· `bmd_loopback_shape_matches_expected_canonical`（真机实测值锚点 720x486/30000÷1001/interlaced/v210 + audio Embedded）· 纯函数同输入同输出 + 解析失败不丢观测 · serde roundtrip 零 vendor 字样 |
| Simulation | MockProvider 世界 PortInfo 装配 → from_port → normalize → audio 三态映射 + embedding 显式化 |
| Hardware | 真机 loopback 门禁（bmd+gstreamer 二进制）：normalize 路径运行 + `GATE_EXIT=0` + `LOOPBACK ALL PASS=true`；**无信号观测 → Unknown+WARN（纪律①"拒绝臆造"的真机实证）** |

## 4. 代码审查（review_mode=standard）+ NOTE

- **改动面**：新 `normalize.rs`（类型 + 装配 + 纯函数 + 6 测试）；`main.rs` 仅 mod 声明 + loopback 诊断挂点；Session/Resource/Lease 零触碰。
- **正确性/安全**：纯函数（无 IO/锁/全局）；provider 字段绝不进入 descriptor（serde roundtrip 反向断言）；观测缺失 Unknown+WARN 不臆造。
- **NOTE-1（Hardware 证据边界）**：READY 态探测结构上读不到 PLAYING 协商 caps → 无信号时机 descriptor = Unknown+WARN（normalize 拒绝臆造的正确行为）；**有信号实机 descriptor 采集归入 0.7B Normalize 主体**（与 fixture PLAYING 捕获路径集成）。
- **NOTE-2**：`.mimosa/`（安全扫描 hook 本地状态）曾短暂阻塞 workspace prepare，已清理；未入版本库。
- **结论**：0 CRITICAL / 0 IMPORTANT。

## 5. 交付路径

archive → 单一 PR `comet/p07b-media-semantics` → `master`（七 checks）→ merge → 删分支。0.7B 主体（Normalize 转换规则扩展 / Clock / Audio Routing / NORMALIZE-RT-01 场景扩展）依 roadmap 与 PHASE_0_7A_POST_MERGE_DEBT.md 推进。
