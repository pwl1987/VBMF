# DeviceBindingManifest — 显式 `SDK-handle → GStreamer device-number` 运行时契约

- 状态：设计决策已落地（`resolver.rs`：`DeviceBindingManifest` / `resolve_with_manifest` / `collect_bindings_from_manifest`）。
- 对应：用户复核 §11 / §12（P1-4 之后下一步）。
- 核心：**停止 runtime 猜测**，绑定权威来自 Provisioning 维护的**显式**清单。

## 为什么需要它（问题复盘）

`VBMF_RESOLVER`（C1 / HW-IDENT-02）在 BMD 盒上的真实结果：

- BMD GStreamer `decklinkvideosrc` 的 `hw-serial-number`：**历史**探测曾为**空串**（`persistent-id=-1`，`model` 因版本而异，见 `c1-element-probe-correction.md`）；**后续 PLAYING+fakesink probe 已确认当前 GStreamer 1.28.2/DeckLink 环境可暴露 `hw-serial-number`，且本机实测值与 SDK `DeviceHandle` 一致**——`ManifestVerified` 已真实通过（见 evidence `2026-08-27-device-binding-manifest-abda19f.json`）；仍待 HW-IDENT-02 多轮重启/采集复核稳定性。
- 因此 auto-resolver 没有任何"SDK 身份 ↔ GStreamer 属性"的交叉线索 → 全部 `Unresolved` → 生产正确拒绝（绝不盲开 `device-number=0`）。
- 但这是**阻塞**，不是"能猜出来"：SDK 枚举序号 ≠ GStreamer `device-number`（A0：SDK#0=SDI 但 GStreamer#0=Mini Monitor 输出卡），且 `device-number`**绝不默认 0**。

结论：在"属性集为空"的前提下，runtime **无法**可靠推断 `handle→device-number`。唯一正确做法是把映射**外置为显式契约**，由运营/Provisioning 在**现场核实**后写入，media-agent 在 runtime 只**消费 + 校验**。

## 设计决策

1. **清单是权威，探测是校验器。**
   - `resolve_with_manifest(devices, probes, manifest)`：对每个 SDK 设备，用其 `bmd_device_handle` 在清单查 `gst_device_number`，再用 `probe_gstreamer_devices` 的结果**验证**该序号确实能被 GStreamer 打开（且可选 `expected_hw_serial_number` / `expected_model` 吻合）。
   - 验证通过 → `ResolverMatch::ManifestVerified`（HIGH 置信）；喂给 `materialize`。
   - 验证失败 / 设备不在清单 → `Unresolved`（**失败闭合**，绝不猜）。

2. **绝不按枚举序号或拓扑反推**——只认 `bmd_device_handle`（canonical 真实身份）。

3. **加载失败即失败闭合**：生产路径 `MEDIA_AGENT_DEVICE_BINDING` 指向的清单若缺失/格式错 → 拒绝 `materialize`（不回退到猜）。

4. **误投防护**：`machine_id` 字段绑定到声明主机；若与运行时主机不符（且运行时可判定主机）→ **失败闭合（生产拒绝，非 warning）**（`check_machine_identity`，用户 §五）。版本不一致（`bmd_sdk_version` / `gst_decklink_plugin_version`）仍仅告警。`bmd_sdk_version` / `gst_decklink_plugin_version` 为可选版本一致性告警。

5. **legacy auto-resolver 仅 diagnostic 模式允许**：生产模式**未提供清单即失败闭合（拒绝 materialize，绝不盲猜）**；仅 `MEDIA_AGENT_MODE=diagnostic` 显式回退 `collect_bindings`（排障用）。`resolve_with_manifest` 是唯一 Production path（用户 §四/§七）。

## Schema（`resolver.rs::DeviceBindingManifest` / `BindingEntry`，JSON 序列化）

```jsonc
{
  "manifest_version": "1.0",
  "machine_id": "10.30.15.10",        // 绑定主机；误投到别机 → 告警/拒绝
  "generated_by": "ops-provisioning",
  "generated_at": "2026-08-27",
  "bmd_sdk_version": "16.0",          // 可选，版本不一致 → 告警
  "gst_decklink_plugin_version": "1.28.2",
  "notes": "...",
  "bindings": [
    {
      "label": "SDI-IN-1",
      "bmd_device_handle": "46:00000000:002e4500", // canonical 真实身份
      "gst_device_number": 1,                       // 现场核实的运行时地址
      "expected_hw_serial_number": null,            // 可选第二道保险：回填真实 hw-serial-number（本机实测==SDK DeviceHandle）后，即使拓扑变化/device-number 漂移，身份校验失败即不启动。建议先冷启动+重启+probe 多轮复核稳定性，再升级为长期强约束（用户 §九）。当前仍允许 null（不校验）。
      "expected_model": "DeckLink SDI"              // 可选交叉校验
    }
  ]
}
```

## 运营流程（生成/轮换）

1. `VBMF_RESOLVER=1 ./media-agent`（无清单）→ 打印 Raw GStreamer Probes：列出本机每个 `device-number` 及其可读属性（`hw-serial-number` / `signal` / `model`）。
2. 现场**物理确认**每个输入的 `device-number`（如"插槽 1 的 SDI 卡 = GStreamer device-number 2"）。
3. 据此填写 `bmd_device_handle`（来自 `device-registry-current.json` 或 discover 日志）与 `gst_device_number`，生成 `DeviceBindingManifest` JSON。
4. 下发到目标主机，media-agent 经 `MEDIA_AGENT_DEVICE_BINDING=/path/to/manifest.json` 加载；`VBMF_RESOLVER=1` 同参数再跑应显示 `manifest-verified`。
5. **变更刚性**：换 PCIe 槽位 / driver / duplex / 机器 → `device-number` 可能漂移 → 必须重新走 1–4 重新核实（这就是"显式契约"存在的意义，runtime 不替你猜）。

## 与 MEDIA-RT-01 / HW-IDENT-02 的关系

- 本清单解决 **HW-IDENT-02 的身份绑定**前提（handle↔device-number 已知）。
- 之后 MEDIA-RT-01 canonical（真机 DeckLink 采集首帧 + PTS 单调）才能以**已验证绑定**跑通，而非 Harness 自测（`MEDIA_AGENT_SELFTEST=1`）。
- 自测 PASS 仍**不是** canonical Gate（见 `media-rt-01-*.json`：`test_mode=SELFTEST / acceptance_scope=HARNESS_ONLY / canonical_gate_status=BLOCKED`）。
