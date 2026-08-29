# Verify 报告 — p06-final-merge-hardening (Phase 0.6 Final Merge Hardening: 合并前清债)

- 日期: 2026-08-29
- 分支: `comet/p06-final-merge-hardening` (自 hi HEAD 0f2a2bf 拉出)
- 验证模式: **full** (10 任务组; diff 22 文件 +912/−379 + 2 新文件)
- 产物: Design Doc `docs/superpowers/specs/2026-08-29-p06-final-merge-hardening-design.md`; Delta spec: 无 (skip_specs)
- 盒: 10.30.15.10 (lytv), 全矩阵 + 真机回归均以盒为准

## Summary

| Dimension | Status |
|-----------|--------|
| Completeness | 10/10 决策项落地 (D1-D10); tasks 全勾; 三处 BREAKING 一次付清 |
| Correctness | 盒上 test **110/110/114/110** (default/sim/mock/bmd,gstreamer) 全过; clippy -D ×4 零警告; build ×2 过; **remove-adapter PROOF EXIT=0**; 真机三闭环回归全过 |
| Coherence | D1-D10 与 design.md 一致 (3 处实现细化已记 Design Doc §14 Divergence); 冻结契约三文档已加对齐注记; p06-hi 归档旧口径已勘误 |

**结论: PASS — 0 CRITICAL / 0 IMPORTANT / 3 NOTE。Ready for archive + 单一 PR。**

## 1. 完整验证 7 项

| # | 检查项 | 结果 | 证据 |
|---|--------|------|------|
| 1 | tasks.md 全部完成 | ✅ | 全部 `[x]` 且附盒上/真机证据 (7.3-7.5 为 workflow 阶段动作, 已改为注记不作为可勾项) |
| 2 | 符合 open design.md | ✅ | D1-D10 逐项对应; "非目标" (Normalize/Session/完整编排/新厂商) 未越界 |
| 3 | 符合 Design Doc | ✅ | 实施顺序 §12 执行; 3 处实现细化 (D1 落位/D9 方法化/P0-5 细节) 记入 §14 Divergence |
| 4 | 能力规格场景 | ✅ (N/A) | skip_specs; 门禁断言即场景 |
| 5 | proposal 目标满足 | ✅ | P0×6+P1×4 全落地 + 全矩阵/真机证据 + PR/保护/Baseline 交付路径就绪 |
| 6 | delta spec 无矛盾 | ✅ (N/A) | 无 delta spec |
| 7 | Design Doc 可定位 | ✅ | frontmatter 关联本 change (design guard 14 项全 PASS) |

## 2. 盒上最终矩阵 (final code, 全绿)

| 步骤 | 结果 |
|------|------|
| `cargo test` ×4 (default / simulation / mock / bmd,gstreamer) | ✅ **110 / 110 / 114 / 110 passed** (基线 107/107/111/107 + 3 新门禁测试) |
| `cargo clippy --all-targets -- -D warnings` ×4 | ✅ 全 EXIT=0 (0 warning) |
| `cargo build` gstreamer-only / bmd,gstreamer | ✅ EXIT=0 ×2 |
| **remove-adapter proof** (`check_remove_adapters.py`) | ✅ EXIT=0 (真删 adapters/blackmagic+gstreamer 后 simulation/mock cargo check 通过) |

调试轨迹: 5 轮盒上迭代收敛 — R1 七处盲改编译错 (PortInfo 字段/serde derive/DiscoveredDevice 字段/persistent_id) → R2 ProviderIdentity derive 因全角逗号替换未中 → R3 serde `&'static str` 反序列化 ('de/'static) 改 `#[serde(skip)]` + 第二处 build_media_backend Result → R4 clippy (dead_code SPI 注记/doc 列表/嵌套 if) → R5 **全绿**。

## 3. 真机回归 (三闭环, 全过)

| 闭环 | 结果 |
|------|------|
| MEDIA-RT-01 SELFTEST (`MEDIA_AGENT_SELFTEST=1`, bmd+gstreamer) | ✅ 45s **71× "A+B+C 全过"** — D1/D2 触及媒体路径后完好 |
| HW-PORT-01 loopback (MiniMon sink2→Duo capture0) | ✅ **LOOPBACK ALL PASS = true** (fixture BMD-SDI-LOOPBACK-01: locked/test_pattern/format_match) |
| HW-IDENT-02 (v1 manifest 诊断) | ✅ 2× `ManifestVerified`/High (gst_device_number=1/0, probe open OK) + 未入清单第三设备 `Unresolved` (fail-closed) — **D1 身份重构后语义逐字不变** |

## 4. 代码审查 (review_mode=standard: 正确性/安全/边界)

- **改动面**: 16 个 .rs + CI yaml + 3 契约文档 + 2 归档勘误 + 1 新脚本; SPI trait 两轴 BREAKING (方法形状/ discover 签名) + Domain 去 vendor 字段。
- **正确性**: 全部 impl/调用点同步 (6 discover impl、2 backend impl、main.rs 12 处调用); 盒上 4 feature 编译+运行 + 真机回归为最终证据; p06-hi 全部门禁断言 (hw_ident_02/media_rt_01/arch_backend_01/resource_01) 在新形状下继续全过 (114 mock 集内含)。
- **安全**: 无 secrets/unsafe; P0-4 fail-closed 消除静默 Mock 接管; P1-2 消除 SDK 失败静默空表; P1-3 故障事件不可被挤出; remove-adapter 脚本仅操作临时副本。
- **边界**: `#[serde(skip)]` 于运行期配对证据; HRTB `with_inner`; `NEXT_PIPELINE_ID` 从 1 起杜绝 0 哨兵; 归档勘误保留原文可追溯。
- **结论**: 0 CRITICAL / 0 IMPORTANT。

## 5. NOTE

- **NOTE-1 (D1 落位细化)**: 证据信封 `ProviderIdentity` 落 SPI contracts 层 (设计原稿写 adapter 内 `BmdIdentity`) — 依赖方向更严格, 详见 Design Doc §14。
- **NOTE-2 (D9 方法化)**: `severity()` 以方法实现 (非 variant 字段), 保护 serde 形状与既有构造点。
- **NOTE-3 (基线更新)**: 新测试基线 **default+sim 110 / mock 114 / bmd+gstreamer 110**; remove-adapter 证明进入 CI 成为 required 步骤。

## 6. 交付路径 (archive 后执行)

单一 PR `comet/p06-final-merge-hardening` → `master` (gh) → branch protection + required checks (gh api, pwl1987/VBMF owner 权限) → merge 后 master 打 `phase-0.6-runtime-abstraction-baseline` tag → **Phase 0.6 Runtime Abstraction Baseline** 成立, 后续进 0.7 Normalize/Audio/Clock/External API。
