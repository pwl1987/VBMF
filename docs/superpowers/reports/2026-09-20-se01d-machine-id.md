# SE-01D 收口报告 — machine identity 收敛（`VBMF_MACHINE_ID` > `/etc/machine-id` > 空）

**Packet**: STANDALONE-ENTRY-01 / SE-01D（冻结计划 S5）· **日期**: 2026-09-20 · **性质**: Runtime 行为变化（bounded，独立 commit/验收）

## 1. 变更

`resolver.rs::current_machine_id()` 解析链收敛（frozen S5）：

- `VBMF_MACHINE_ID`（显式覆盖，测试/容器用；trim，空白视为未提供）
- `>` `/etc/machine-id`（生产权威；trim，纯空白视为未解析出）
- `>` 空串——消费点语义**不变**：NetworkSourceBinding `from_bytes` 拒绝（`MachineIdUnresolved`，既有）；DeviceBindingManifest `check_machine_identity` 跳过而非误报（既有；空串永不匹配清单 pin）。
- **`HOSTNAME` fallback 移除**（行为变化：原 env 缺失时回退 `HOSTNAME`）。
- 实现带可测缝隙 `current_machine_id_from(env, etc)`（注入式，并行测试不碰进程 env/真实文件）；`network_binding` 的 `MachineIdUnresolved` 消息与 resolver 注释同步（不再提 HOSTNAME）。

**明确不做**：不改任何 pin 消费点语义（S5 "fail-closed 语义不变"）；不改 wire；不引入新身份源。

## 2. 回归测试

- 解析链 6 项（`se01d_machine_id_*`）：env 优先且 trim；`/etc/machine-id` 回落且 trim；空白 env 穿透到 etc；两级均缺=空串；纯空白 etc 内容=未解析；**HOSTNAME 不再被读取**（注入 HOSTNAME 可返回的 env 查找，结果仍为空——移除的直接回归证明）。
- 两类 manifest pin 既有锚定复验（未改动、全部保持绿）：network 类 `MachineIdMismatch` + 空 runtime id `MachineIdUnresolved`（`rf_src_rtmp_02_manifest_rejects_version_and_machine_pin_violations`）；device 类 mismatch 拒绝 + 空身份跳过（`manifest_check_machine_identity_rejects_mismatch`）。

## 3. 验证

- Dev VM：default **332/332**；simulation **332/332**；mock **547/547**（520+9+12）；ffmpeg-backend **367**（361+6 新增）；clippy×4 档 `-D warnings`、fmt、architecture lint、remove-adapters 全 PASS。
- CI：见 STATE §7 对应行（7/7 required）。
- BMD exact-commit 重 pin 实证：见 STATE §3.64 与 evidence 目录（network 类 loopback gate leg **不设 `VBMF_MACHINE_ID`** → manifest 由 `/etc/machine-id` 值 pin 并经生产 loader 载入；device 类真实 binary 正/负 pin；负 pin = 错误 machine_id manifest 被 fail-closed 拒绝）。

## 4. 运维影响（迁移注记）

- 以旧 HOSTNAME 值 pin 的既有清单在新版本下**不再匹配**（HOSTNAME 不被读取）——必须重 pin 为 `/etc/machine-id` 值或显式设置 `VBMF_MACHINE_ID`。BMD 两类清单已按此重 pin 并留证。
- 语义页 `STANDALONE_MEDIA_AGENT_OPERATIONS.md` §4.3/§4.5 已随实现回改（HOSTNAME 行移除，`/etc/machine-id` 行新增）。
