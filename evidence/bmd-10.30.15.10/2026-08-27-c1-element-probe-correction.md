# C1 Resolver 探测入口纠正：DeviceMonitor → decklinkvideosrc 直接 Element Probe

- 日期：2026-08-27
- 触发：用户复核 `master`（`b52e2b6` C1 真实 GStreamer 探测），判定 **`GstDeviceMonitor` 作 DeckLink 枚举入口不可靠**，要求改为直接 probe `decklinkvideosrc` 实例。
- 性质：Resolver 架构（数据模型 / `find_match` / `Ambiguous` / `Confidence` / `ResolvedDeviceBinding`）**保留不变**；仅替换"运行时探测 GStreamer 实例"的实现方式。**非**新开架构，无需 V0.3。

## 为什么 DeviceMonitor 不对（用户复核，撤回 b52e2b6）

- GStreamer 官方 `GstDeviceMonitor` 是基于 `GstDeviceProvider` 的设备监控框架。
- 当前 DeckLink 官方插件只暴露 `decklinkvideosrc` / `decklinkaudiosrc` **element**，**不提供 `GstDeviceProvider`**；实机 `gst-device-monitor` 不列出 DeckLink。
- 故 `monitor.devices()` 在本机恒为空 → `collect_bindings` 为空 → `materialize` 拒绝 → 现场看到 `HW-IDENT-02 FAIL`，但实际真因是"**探测方法无效**"而非"设备未解析"。这正是 §九 要区分的假失败。

## 正确实现（已落地 `resolver.rs`）

```text
SDK DeviceRegistry
      │ canonical DeviceHandle / serial
      ▼
GStreamer Resolver
      │  for N = 0..MAX_PROBE_DEVICES:
      │      create decklinkvideosrc
      │      set device-number = N
      │      set_state(READY)          # 打开设备, 不 PLAYING, 不拉真实帧
      │      read hw-serial-number / persistent-id / signal / model
      │      set_state(NULL)
      ▼
resolved device-number
```

- `probe_gstreamer_devices(max)` 返回 **`GstProbeOutcome`**：
  - `Available(Vec<GStreamerDeviceProbe>)` — 探测成功。
  - `Unavailable(String)` — 探测方法不可用（GStreamer 未初始化 / `decklinkvideosrc` 工厂缺失 / decklink 插件未装）。**≠ 设备未解析**，绝不压成 `Unresolved`。
  - `Empty` — 探测正常但枚举到 0 个实例（本机确无采集卡）。**≠ Unavailable**。
- 每序号 `device-number` 绑定目标卡，`set_state(READY)` 打开后读只读属性；打不开 / 无身份线索（ghost）的序号 `None`，不计入（防 `Ambiguous` 误判）。
- `connection` / `mode` 用 element 默认（connection=auto 自动识别 SDI，mode=auto 自动探测），满足 binding probe；显式设置 enum 需插件专属 enum 类型，核心 `gstreamer` crate 不暴露，故不显式设。
- **`device-number` 绝不默认 0**（GStreamer#0 在实机是 Mini Monitor 输出卡）。由 Resolver 经 `hw-serial-number` 命中后确定。

## 属性读取的盒上事实（关键，避免 panic）

- 本机 `gst-inspect-1.0 decklinkvideosrc`（GStreamer **1.28.2**）**只有 `device-number` 与 `hw-serial-number` 两个选卡属性**，**没有 `persistent-id`**；`signal` / `model` 因版本而异。
- 因此每个属性读取前用 `find_property(name)` 守卫（glib 0.20 稳定 API），缺属性则跳过；直接 `el.property::<T>()` 缺属性会 **panic**。
- `hw-serial-number`：官方文档定义为**只读**硬件 ID（代码注释已据此订正）；本机 `gst-inspect` 显示 "可读写" — 仅现象差异，代码只读取、不写入，无影响。
- `persistent-id`：本机不存在 → 恒 `None`；这与 A0 实测（BMD `PersistentID` 不支持）一致，故当前硬件 Resolver 真实路径是 `DeviceHandle → 经 hw-serial-number → device-number`。

## 错误模型（§九, P1）

新增 `ProbeUnavailable` / `ProbeEmpty` / `DeviceUnresolved`(= 现有 `Unresolved`) / `Ambiguous` 区分：调用方（C1 / materialize）据 `GstProbeOutcome` 分支，不再把"方法不可用/空"压成 `HW-IDENT-02 FAIL` 的 `Unresolved`。

## 身份与信号是两个维度（§六/§十三）

`GStreamerDeviceProbe.signal` 已补入并真实读取（`find_property("signal")` 守卫）。`signal=false` **不**判身份失败：身份（hw-serial-number 命中）与信号（是否锁定 SDI）独立。C1 证 `DeviceHandle ↔ hw-serial-number ↔ device-number`；MEDIA-RT-01 才是证信号+首帧+PTS+稳定。

## 编译验证状态

- 本机 Windows 无系统 GStreamer，`--features gstreamer` 分支无法本地编译；**必须在 BMD 盒 `cargo build --features bmd,gstreamer` 实编译**。
- 已规避的 API 风险：
  - 不用 `has_property(name, None)` 双参（glib 0.20 可能仅单参）→ 改用 `find_property(name)` 单参守卫。
  - 不直接读缺属性（`persistent-id` 盒上不存在）→ `find_property` 守卫。
  - enum 属性（connection/mode）不显式设，避核心 crate 未暴露的插件 enum 类型。

## 下一步

1. BMD 盒 rebuild `--features bmd,gstreamer` → `gh run download` → `scp` → `VBMF_RESOLVER=1 ./media-agent-gstreamer` 拿 C1 真实证据（raw probes + evidence + bindings）。
2. 现场比对 SDK `DeviceHandle`/`serial` 与 GStreamer `hw-serial-number`，确认 `SerialExact` / `DeviceHandleExact` 唯一命中 → **HW-IDENT-02 PASS**。
3. 再走 MEDIA-RT-01 A/B/C（接稳定 SDI 信号或 `MEDIA_AGENT_SELFTEST=1`）。
