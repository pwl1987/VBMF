# HW-IDENT-02 多轮冷启动复核 — DeviceHandle 身份稳定性 + Manifest 绑定闭环

> 生成日期: 2026-08-27
> 盒: BMD 10.30.15.10 (lytv) | 介质机
> `media-agent` commit under test: **`d182cb5`** (含 resolver/main 修复 + HW-IDENT-02 harness)
> 验收性质: **Runtime Acceptance 真机运行** (治理规则 §2 要求)

## 1. 目的

`device.rs` 注释明示 *"DeviceHandle 非跨重启永久稳定"*；而当前 canonical 设备身份
= `device_id = UUIDv5(VBMF_BMD_NS, "vbmf:bmd:"+handle)`，且 `DeviceBindingManifest`
绑定契约按 `bmd_device_handle` 索引（缺失/不匹配 → 整条目 `Unresolved`，失败闭合）。

HW-IDENT-02 要判决：**多次冷启动后，本机全部 DeckLink 的 `bmd_device_handle` 集合与
`device_id` 是否逐轮一致**；若漂移，则 canonical=DeviceHandle 决策失效，须 V0.3 返工。
同时复核 Manifest 绑定闭环（声明条目是否逐轮 `ManifestVerified`）。

## 2. 方法

- 每轮：物理 `sudo reboot` 盒 → 等回连（盒启动较慢，约 2–3 min，PAM `nologin` 阶段不可登入）
  → 跑 `scripts/hw-ident-02.sh <N> ~/hw-ident-02.manifest.json`。
- 捕获：C1 Resolver Evidence 全量抽取为规范化 TSV（handle / device_id / gst_device_number /
  gst_hw_serial_number / match_kind），逐轮落盘于 `~/hw-ident-02/`。
- 比对：`scripts/hw-ident-02-collate.sh ~/hw-ident-02` 跨轮判定 handle 集合稳定性与
  match_kind 一致性。
- 所用清单 `scripts/hw-ident-02.manifest.json`：采用 abda19f 已物理核实的 2 条目绑定
  （`#0` Mini Monitor 4K / `#1` SDI Input 1）；SDI2 未纳入采集契约（其 GStreamer
  device-number 待物理核实），但其 SDK `DeviceHandle` 仍由 Evidence 捕获以复核冷启动稳定性。

## 3. 复核中发现并修复的回归（重要上下文）

首次运行即全 `Unresolved`，但 `gst-launch decklinkvideosrc device-number=0` **正常**
（进 PLAYING、甚至检测到输入源），定位为两处 probe 侧回归（非盒上进程占用——已清理
上一轮 selftest 残留 media-agent 进程，仍不解决）：

1. `probe_one_device_number` 硬性要求 `hw-serial-number`/`persistent-id`/`model` 非空，
   注释声称"当前硬件 hw-serial-number 恒暴露(=DeviceHandle)"——实测三者恒空串，
   致本该打开的卡被判 `PropertyMissing`。
2. `resolve_with_manifest` 交叉校验 `Some(exp) => actual == Some(exp)`：清单声明
   `expected_model` 但 GStreamer 实测 `model=None` 时 `None==Some` 为 false → 失败闭合。

均违反 abda19f 设计本意（*信任显式清单、停止 runtime 猜测*）。修复（`d182cb5`）：
清单模式（`require_identity=false`）下卡能打开即计入 probe；交叉校验实测 `None` 跳过、
仅实测值矛盾才失败闭合。修复后 2 条目绑定恢复 `ManifestVerified`（与 abda19f 一致）。

## 4. 结果（3 轮冷启动）

| 轮次 | `bmd_device_handle` 集合 | SDI1 `#1` | MiniMon4K `#0` | SDI2 |
|---|---|---|---|---|
| 1 | `46:..:002e4400, 46:..:002e4500, 83:1a66443b:00000000` | ManifestVerified | ManifestVerified | Unresolved |
| 2 | 同上（完全一致） | ManifestVerified | ManifestVerified | Unresolved |
| 3 | 同上（完全一致） | ManifestVerified | ManifestVerified | Unresolved |

`device_id`（UUIDv5 of handle）逐轮一致，命中基线：
`6ede00d0-baf4-573f-a0dd-4a503bf7f766` (SDI2) / `4fa33dcb-5f76-5f76-aea8-330df7ada03e` (SDI1)
/ `1afe2dcc-6d85-5b46-b7d6-e14102501c77` (MiniMon4K)。

跨轮比对脚本判定：**`RESULT=PASS : DeviceHandle 跨冷启动稳定, 绑定闭环(match_kind)逐轮一致`**。

原始产物见 `evidence/bmd-10.30.15.10/hw-ident-02/`：`round-{1,2,3}.tsv`、
`round-{1,2,3}-evidence.json`、`round-{1,2,3}-*.txt`（完整 stdout）、`COLLATION.txt`。

## 5. 结论

- **DeviceHandle 跨冷启动稳定** → canonical=DeviceHandle 决策在本盒成立，**无需 V0.3 返工**。
- **Manifest 绑定闭环逐轮成立** → 显式绑定契约 + 失败闭合机制在真机多轮冷启动下稳定可用。
- 释放 `MEDIA-RT-01` A/B/C 真机采集验收占机窗口（canonical 路径身份已锁定）。

## 6. 局限

- 冷启动为 `sudo reboot`（软重启，重枚举 PCIe/重载驱动），非物理断电上电；若需更严判据
  可后续做物理断电多轮复核（SDI2 含入）。
- SDI2（`46:..:002e4400`）每轮均被 Evidence 捕获且 handle 稳定，但**未纳入采集清单**
  （其 GStreamer `device-number` 映射待单独物理核实后再补入绑定契约）。
- 盒启动耗时较长（PAM `nologin` 阶段约 2–3 min 不可登入），编排时已加回连重试。
