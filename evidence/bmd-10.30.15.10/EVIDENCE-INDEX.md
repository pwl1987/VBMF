# BMD 10.30.15.10 真机证据索引 (EVIDENCE-INDEX)

> 生成日期: 2026-08-27 — 配合 `media-agent` commit `457837a` 及本轮 **Production Hardening 收口** (P1-1/P1-2/P1-4).
>
> 目的: 防止历史证据被误读为当前验收结论 (用户 §十九). 例如 `cap01-first-frame` 的 SDK first-frame
> PASS **不是** `MEDIA-RT-01` PASS; `gate2.2/2.4/2.5` 是阶段性 Gate 验收, 不等同 Phase 0.6 Acceptance.
>
> 状态词 (用户 §十九):
> - **Current**    : 当前仍有效、代表现状的证据.
> - **Acceptance** : 已达成某验收门禁 (Gate / Phase) 的正式结论.
> - **Historical**  : 已完成其历史使命、仅供追溯, 不代表现状.
> - **Superseded** : 已被后续证据覆盖/推翻.

## 环境基线 (Current / 基线)

| 文件 | 状态 | 说明 |
|---|---|---|
| `2026-08-27-environment-baseline-v2.md` | Current | Rust 1.98.0 / GStreamer 1.28.2 / BMD SDK 本地独立编译环境; 当前盒实际版本基线. |
| `2026-08-25-environment-prep.md` | Historical | 初始环境准备; 已被 baseline-v2 取代. |
| `2026-08-25-media-sec-01-runsc.md` | Historical | MEDIA-SEC-01 runsc 运行方案探索; 当前未采用. |
| `2026-08-26-media-sec-01-step3.md` | Historical | MEDIA-SEC-01 step3 探索; 当前未采用. |

## 设备身份与绑定 (Current / 权威)

| 文件 | 状态 | 说明 |
|---|---|---|
| `2026-08-26-a0-identity-verification.md` | Acceptance | A0 设备身份 (DeviceHandle 权威) 验证. |
| `2026-08-26-canonical-ingest-boundary.md` | Current | canonical 采集边界 (GStreamer decklinkvideosrc/audiosrc 唯一路径). |
| `2026-08-26-real-sdi-probe.md` | Current | 真实 SDI probe 证据. |
| `2026-08-27-device-registry-current.json` | Current | 当前 Device Registry 快照. |
| `2026-08-27-device-binding-manifest-abda19f.json` | Acceptance | DeviceBindingManifest 权威路径验证 (commit `abda19f`). |
| `2026-08-27-binding-fail-closed-5ce34e1.json` | Acceptance | Production 绑定失败闭合 (commit `5ce34e1`). |
| `2026-08-27-c1-resolver-41e0931.json` | Superseded | C1 (VBMF_RESOLVER) 探测器输出快照 (commit `41e0931`); 已被 `abda19f`(权威绑定) / `457837a`(硬化) 覆盖. |
| `2026-08-27-c1-element-probe-correction.md` | Historical | C1 element 探测修正说明 (ProbeError 之前). |
| `2026-08-27-hw-ident-02-devicehandle-stability.md` | Acceptance | HW-IDENT-02 多轮冷启动 DeviceHandle 稳定性 + Manifest 绑定闭环 (commit `d182cb5`); 判定 PASS, 释放 MEDIA-RT-01 A/B/C 占机窗口. |

## Runtime Hardening (Current / 生产硬化)

| 提交 / 证据 | 状态 | 说明 |
|---|---|---|
| commit `9f3a1df` | Acceptance | RESOLVER-ERR-01 probe 失败分类 (ProbeError). |
| commit `5ce34e1` | Acceptance | Production Binding 失败闭合硬化. |
| commit `457837a` | Current | 6 项 Production Hardening (P1-2 真实版本接入 / P1-3 生产不自动启动 / P1-4 /health 回环 / P1 bus 溢出 / P1-1 config 文档 / evidence 索引). |
| 本轮收口 (同 commit) | Current | P1-1 SDK 版本 declared↔detected 拆分 (真实 libDeckLinkAPI.so 身份探测); P1-2 `rpc_bind` 默认 `127.0.0.1` + 安全校验; P1-4 Supervisor ClockLost=degraded 最低策略 + Warning/StateChanged 日志. |

## Gate 阶段性验收 (Historical / 不代表 Phase 0.6 Acceptance)

| 文件 | 状态 | 说明 |
|---|---|---|
| `2026-08-26-gate2.2-device-discovery.md` | Historical | Gate 2.2 设备发现验收. |
| `2026-08-26-gate2.2-in-container.md` | Historical | Gate 2.2 容器内验收. |
| `2026-08-26-gate2.4-health-lease.md` | Historical | Gate 2.4 /health + lease 验收. |
| `2026-08-26-gate2.5-bmd-verify.md` | Historical | Gate 2.5 BMD 校验验收. |
| `2026-08-26-gate2.5-sdk-probe.md` | Historical | Gate 2.5 SDK probe 验收. |
| `2026-08-26-gate6.7-bmd-enumeration.md` | Historical | Gate 6/7 BMD 枚举验收. |

## 媒体运行时 (MEDIA-RT-01) — ⚠️ 当前仍 BLOCKED

| 文件 | 状态 | 说明 |
|---|---|---|
| `2026-08-26-cap01-first-frame.md` | Historical | CAP-01 SDK first-frame PASS. **注意: 这是 SDK 层 first-frame, 不是 MEDIA-RT-01 GStreamer canonical 采集验收.** 不得据此判定 MEDIA-RT-01 通过. |
| `2026-08-26-cap01-first-frame.log` | Historical | 同上, 原始日志. |
| `real-canonical-run-2026-08-26.log` | Historical | 早期 canonical run 探索日志; 非最终验收. |
| `2026-08-27-media-rt-01-2ec54a2.json` | Current | MEDIA-RT-01 当前快照 (commit `2ec54a2`): **A/B/C 仍未达成**; HW-IDENT-02 已 PASS (`d182cb5`), 现可占设备做 MEDIA-RT-01 A/B/C 真机验收. |

## 治理规则 (跨证据通用)

1. 任何新证据落地时, 在文件名带日期, 并**在此索引登记状态**; 被覆盖的旧证据改为 `Superseded`/`Historical`.
2. `MEDIA-RT-01` / `HW-IDENT-02` Acceptance 结论**只能**来自显式 Runtime Acceptance 真机运行, 不得由
   SDK first-frame、Gate 阶段性验收或 CI 编译通过推断.
3. 冻结设计 (V0.2 LOCK FINAL / Phase 0.5 LOCK FINAL) 状态词仅用 `LOCK FINAL` 与 `Historical:RECONCILED`,
   禁止 `DRAFT`, 不新增页面 (用户架构守卫).
