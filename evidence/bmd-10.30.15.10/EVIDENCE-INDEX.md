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

## A2-8 Dual-Input Switch (A2-8-02i/A2-8-04) — 2026-09 R58 步骤 10/11 真机证据

| 目录 | 状态 | 说明 |
|---|---|---|
| `a2-8-04-r58-step11-final-gate/` | Acceptance | **R58 Step 11 新鲜 Final Gate 证据窗（冻结谓词重算=PASS·验收终裁归验收层）**: 案 b 三件（OBS N=30 dwell1000=R56 失败窗同形·六路 NM=0·dual_input 10/10·hw 266/266）+run3b 能力补证; 冻结 bin md5 440c761b（复用）; REV e09ed97; 源 sha 864/864 盒==HEAD; 逐格明细=谓词文档 §11/R57 §22.3. |
| `a2-8-04-r58-step10-regression/` | Acceptance | **R58 Step 10 全回归证据（验收层终裁 PASS/CLOSED）**: 四跑（dual_input 10/10+obs 三场景）全 EXIT=0·NM=0·adv=0·R53 签名逐字; bin md5 440c761b; REV 6ab24a3（e09ed97 为 docs-only 后继·源全等）. |

> A2-8 系列证据口径: 日志 md5 同步登记于 docs/superpowers/reports/ 相应
> 章节; 入库副本 = 审计便利层, **不称 GitHub CI**（盒上本地验证口径）;
> 盒上原件为 P8 origin. `.json` 类新文件受根 `.gitignore` `evidence/**/*.json`
> 规则约束（入库需 `-f`）; `*.log`/`*.md`/`*.txt` 不受约束.

## A2-8-05 Normal-Use Preview v0.1 — 2026-09 R59 冒烟证据

| 目录 | 状态 | 说明 |
|---|---|---|
| `a2-8-05-v0.1-smoke/` | Current | **R59 v0.1 零代码测试包盒上冒烟（run2 全过）**: 诊断模式真实 bin（服务 bin md5 6a0fa224·REV 4b473b3 生产面==e09ed97·manifest 7521d17e）; run1 留证（stop_session 400=打包脚本 UUID 格式缺陷·已修）+ run2 全绿（health Capturing·双输入起流两路 advancing·events total=15·stop_session executed·teardown 链 Program Stop→Tap Detach+watchdog 停止旗·进程死亡）; README/IDENTITY/env.sample 为部署原件; 打包面=仓库 `preview/a2-8-05-v0.1/`; 审计=2026-09-06-a2-8-05-normal-use-startup-entry-audit.md. |

> v0.1 口径: 冒烟非谓词 Gate; 已知限制（无切换入口/无回读端点/无信号处理/
> 无版本行）如实标注于 IDENTITY; `gst_pad_unlink`×3/@teardown 与
> `gst_video_converter_free`×1/@startup = 既有已知类工件非新类.

## A2-8-05 v0.2 Control Plane + Step 14 — 2026-09 R61 闭环证据

| 目录 | 状态 | 说明 |
|---|---|---|
| `r61-v02-step14/` | Current | **R61 v0.2 控制面扩面 + Step 14 闭环（真实服务进程内·非 gates 替代）**: v0.2 服务 bin md5 90186bb9（R61 实现·核心四零 diff）; API A→B executed（av_epoch=1 preserved）→回读 observed=B/seg=1/continuous/discontinuity_declared（R53 冻结签名）→B→A→回读; 错误路径 permanent（TargetNotInGroup/TargetAlreadyActive）+幂等 replay 逐字节+conflict; stop_session→teardown 链→进程死亡; run.log=盒上原件·响应值=终端捕获口径. 实现+裁决回执=docs/superpowers/reports/2026-09-06-a2-8-05-v02-control-plane-expansion-probe.md §9. |
| `r62-cp-safety-probe/` | Current | **R62 Control Plane Safety Probe（Step 16-0A/0B/0C 只读探针）**: 服务 bin 90186bb9（R61 同一 bin·确定性重建复核）+ R62 fault-probe 集成测试（md5 72a66129·生产源码零改动）; 16-0A mock 全链注入 6/6（F0-F4 失败×平面矩阵·F3 真实 5s 墙钟）+ 真机正常切换 0.152s（R53 签名）; 16-0B 并发（串行基线 sub-ms/在途查询排队 61ms/并发 replay 逐字节/双反向 accept 串行化·epoch 恰一次/停滞读者冻结管理面 10.175s=单 accept 铁证/stop_session 1.23s teardown）; 矩阵 fmt/229/229/405+6/268/clippy×3 全绿. 报告=docs/superpowers/reports/2026-09-06-a2-8-05-r62-control-plane-safety-probe.md. |

> v0.2 口径: 命令/查询双平面类型级隔离; 能力拒绝（平面缺席）过形状层→
> 占幂等 id·outcome=Rejected（与形状拒绝不占 id 分层）; wire
> status.status=dispatch 裁决（executed/replayed/conflict）·
> classification/detail=命令 outcome——两层分层诚实.

## 治理规则 (跨证据通用)

1. 任何新证据落地时, 在文件名带日期, 并**在此索引登记状态**; 被覆盖的旧证据改为 `Superseded`/`Historical`.
2. `MEDIA-RT-01` / `HW-IDENT-02` Acceptance 结论**只能**来自显式 Runtime Acceptance 真机运行, 不得由
   SDK first-frame、Gate 阶段性验收或 CI 编译通过推断.
3. 冻结设计 (V0.2 LOCK FINAL / Phase 0.5 LOCK FINAL) 状态词仅用 `LOCK FINAL` 与 `Historical:RECONCILED`,
   禁止 `DRAFT`, 不新增页面 (用户架构守卫).
