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
| `r63a-recovery/` | Current | **R63-A 切换失败状态恢复（域状态机修复·真机正常路径回归）**: hw bin 重建 md5 f6e6303b（重建先于真机·R62 纪律）+manifest 7521d17e; API A→B executed（av_epoch=1 preserved）→回读 observed=B/seg=1/DD/continuous（R53 签名逐字）→B→A（av_epoch=2）→回读; stop_session executed→svc.log teardown 完成行+watchdog 停止旗退出行（服务常驻=设计语义·取证后显式停止）; mock-recovery-matrix.log=R63-MATRIX 九行矩阵（ctrl/F0/R1+R7/R2+R8/R3+R6/R4/degraded/0a/replay）; md5 盒=origin 全等. 故障→恢复主证=mock 全链（真机注入无旋钮·R64 再议）. 报告=docs/superpowers/reports/2026-09-06-a2-8-05-r63-a-switch-failure-recovery.md. |
| `r63b-concurrency/` | Current | **R63-B Transport 并发与 Query 解耦（真机并发验证 B5/B6）**: hw bin md5 1326be28（重建先行）+manifest 7521d17e; 后台 A→B/B→A 双 preserved+R53 签名回读（seg 1/2/3）·切换窗内 health/runtime/events 并发全 200 ~1ms 级（latencies.txt 18 行）; **慢读者挂 8s 期间三时点 962µs–1.08ms（R62 同形=10.175s 全管理面冻结的修复实证）**; 同 id replay=executed+replayed detail/classification identical（replay-summary）; N=8 并发 /runtime 680µs–1.04ms; stop_session→teardown 完成行+watchdog 停止旗退出行; mock-concurrency-matrix.log=R63B-MATRIX 八行（b3 窗内 19.9µs/t5 串行双消费/t6 718µs）; md5 盒=origin（14 件）. 报告=docs/superpowers/reports/2026-09-06-a2-8-05-r63-b-transport-concurrency.md. |
| `r64-recovery/` | Current | **R64 真服务恢复矩阵（real Control Plane + test-controlled adapter·两阶段 gate）**: gates bin 31a00197+manifest 7521d17e; 规范跑 gate-run.log=**exit0 PASS-AND-FINDINGS（阶段一 failures=0·findings=12）**——阶段一: C0 R53 签名/C1 observed=from 落 Active(from)+replay 原样+再切成功/C2 **observed=to 硬件真翻转+真 5.02s 证据超时（R62 场景+恢复闭环·NewEpoch(1)+DD+identity 段+反向再切 preserved@1）**/STORM 真切换窗 0.87s 四路并发 200<2s+快照真值窗内=旧已提交无未来时间戳/C3 observed=None→RecoveryRequired 终态拒收→teardown 全缺席; 13 静息六平面检查点全 OK; 阶段二 C2b（release Err=真机 5s 确认超时同型）**四跑三态**（attempt1=L1 Active(from)+强释迟到翻转→静息分歧+死锁唯一出口 teardown / attempt2·4=L2 RecoveryRequired 诚实终态 / attempt3·5=L3 Active(to) 自洽）——attempt1-4 全档保留; **发现三条停裁未在线修（①P1 迟到翻转死锁 ②P2 恢复观测非确定 ③P2 PTS 伪影遮蔽拒收形态）**; mock-storm-matrix.log=r64_storm; md5 盒=origin（9 件）. 报告=docs/superpowers/reports/2026-09-06-a2-8-05-r64-control-plane-recovery-acceptance.md. |
| `r64-stability-30m/` | Current | **R64-6 30min 稳定基线（真服务·无注入·bin e08978e1 重建钉扎）**: 脚本裁决 VERDICT FAIL（5/9）**按原样交付**——实质数据全绿: 60/60 切换 executed+preserved（tl 恒 0·R53 签名逐周期）·同 id replay 12/12 原样·854 并发查询全 200 ~1ms（max 1.26ms）·fd 14→14 零漂移·RSS 首/末 1/3 均值 1226.8→1239.0MB（+12MB 无爬升）·dropped/clock_lost 恒 0·events 30 采样 has_critical=0·frames v 24→51033/a 33→68033; 四项 FAIL 定性=threads 29-31 有界振荡（per-connection 模型本性·无增长趋势·语义归裁）+plus60/tracks 谓词实现取样伪影（绝对纪元 0→60 恰+60·readbacks 60/60 observed==target）+watchdog_ticks 测量缺口（warn 级遮蔽 info 级 tick/teardown 行·非 watchdog 失败）——无一项指向系统缺陷·重测与否归裁决; 19 件 md5 盒=origin. 报告 §9. |
| `r65-cutover-recovery/` | Current | **R65-A 物理 Cutover/Recovery 时序修复真机验收（期望感知稳定再观测协议）**: gates bin 1e4f0372+manifest 7521d17e; 规范跑 gate-run.log（attempt2）=**exit0 failures=0**——阶段一回归全绿（C0/C1 replay+retry av 直跳 4/C2=F3 真 5.12s 证据超时闭环→落 A+tl=1+DD+反向 preserved/STORM 窗内快照=旧已提交/C3 RecoveryRequired 终态·本次 c3-next=permanent recovery-required 干净形态·拒收零委托）; 阶段二=**R65 确定性段: F2×10 全部 10/10 确定性落 Active(to)——R64 四跑三态（L1 死锁/L2 终态/L3 自洽）零复现**（每轮失败轮委托 plan epoch=2i−1/反向=2i/tl=i 记账精确·六平面检查点全 OK·replay 原样·失败切换全程 100-152ms）+F4 ⑨ PTS 回注矛盾→确定性落 Active(to)+NewEpoch(11)+av=21→反向 preserved（av=22）→teardown; attempt1 如实全档=F4 首版布尔旋钮在 ①a 提前点火成 0a pre-begin 形态（gate 场景设计错误·产品零改动·阈值制修复）; **R65-B 增量 gate-run-b.log（bin f7f7db7d）=exit0 failures=0——c3-next 确定性 permanent+recovery required（①a 遮蔽真机消失）+c3-after-reject 六平面 OK（tl 诚实停留未治愈）+F2×10 仍 10/10（R65-A 零回归）**; md5 盒=origin（6 件）. 报告=docs/superpowers/reports/2026-09-07-a2-8-05-r65-physical-cutover-recovery.md §4-§5. |
| `r64-stability-30m-v2/` | Current | **R64-6' 30min 稳定基线重跑（真服务·无注入·R65 后 bin 2b5aa760 重建钉扎·谓词 v2=R65 裁决单独登记的修订）**: **VERDICT PASS（10/10）**——threads 有界振荡 spread=2（29-31·v1"恒定"字面废弃）·fd 14→14 零漂移·RSS 首/末 1/3 1236.7→1239.5MB（+2.8MB 无爬升）·**switch_epoch 逐命令恰 +1（1→60 连续）+60/60 executed+preserved**·**observed 逐命令==target 60/60（readbacks 行级核对）**·frames v 31→51035/a 42→68032 严格递增·dropped/clock_lost 恒 0·**watchdog tick 84→173（RUST_LOG=info 可测——v1 warn 级测量缺口消除）**·events 30 采样 has_critical=0·replay 12/12 replayed 原样（数据行）·teardown 完成行=1·R53 签名逐周期（tl 恒 0·DD·continuous）; 如实登记: 首跑 summary 9/10=v2 解析器 bug（detail 误当 status 内嵌→0/60 假 FAIL）——**原始证据零改动**, 修正解析器（顶层 detail 词表）对同一份产物重析=10/10, 首跑保留 summary-parserbug.txt, 脚本+reanalyze-v2.py 修正入库; 22 件 md5 盒=origin. 报告 §7. |
| `r64-stability-2h/` | Current | **Step 15 2h 长稳 rung（真服务·无注入·bin 2b5aa760=R64-6' 同源·参数化探针首跑）**: **VERDICT PASS（10/10）**——240 周期×~30s（07:10:59-09:13 盒钟）: threads spread=2（29-31）·fd 14→14·**RSS 首/末 1/3 1237.6→1238.7MB（2h 仅 +1.1MB 无爬升）**·**switch_epoch 逐命令恰 +1（1→240 连续）+240/240 executed+preserved**·**observed 逐命令==target 240/240**·frames v 28→206653/a 38→275500 严格递增·dropped/clock_lost 恒 0·**watchdog tick 343→691**·events 120 采样 has_critical=0·replay 48/48 replayed 原样（数据行）·**3415 并发查询零非-200**·teardown 完成行=1·服务干净退出; 独立 reanalyze-long.py 对同一产物重析=同 verdict; 21 件 md5 盒=origin. 报告 §96. |
| `r64-stability-2h-gate/` | Current | **Step 15 2h rung 开场 gate（「错误后恢复切换」R62 用户令·真机错误注入唯一冻结载体）**: r64 控制面真机一轮（C0-C3+F2×10+F4·phase1 委托 7 次/phase2 委托 22 次）=**exit0 failures=0 findings=1（=F2×10 确定性 10/10 数据行·R64 四跑三态零复现维持）**; gates bin f7f7db7d（touch 强制重编同哈希——26e6cde 纯空白改动产出同一二进制的证明）·manifest 7521d17e; 3 件 md5 盒=origin. 报告 §96. |
| `r64-stability-8h/` | Current | **Step 15 8h 长稳 rung（真服务·无注入·bin 2b5aa760）: VERDICT FAIL（9/10）按原样交付**: 960 周期中 cycle 908（运行 7h34m 处）**唯一一例自发视频面 PTS 证据超时**——响应 executed+classification=unknown+detail「timeline 证据超时 FailClosed: timeline evidence insufficient (pending planes: [Video])」（不伪装成功）→ svc.log 全窗唯一关键行 WARN 08:29:01.557Z=**R63-A 恢复路径真实自发执行（Observed 优先落定 to·NewEpoch{ProgramEpoch(1)·SourceSegment(to)}）**→ tl_ep 0→1+seg 909+DD+v_cont=declared_discontinuity 诚实再基准 → 后续 52 周期全 executed+preserved·零死锁零发散·teardown 完成行=1; 其余九项全过: epoch 1→960 逐命令恰 +1·observed==target 960/960·threads spread=2·fd 14→14·RSS 首/末 1/3 1249.4→1277.2MB（+27.8MB 无爬升）·frames v 25→829254/a 34→1105503·drops 0·watchdog 1379→2764·events 480 采样 critical=0; 数据行: replay 192/192 原样·13659 查询零非-200; 定性=全程序首次自发（非注入）证据超时（1320 次稳定基线切换唯一一例≈0.1%）·R64 gate C2 注入形态的自发对应·系统行为符合 R65/R63-A 设计·归用户裁决; **用户裁决定性=独立事件 S15-E01; RCA 完成（只读）: ⑤ 旗门控读-竞争窗错过已到达事件（非 PTS 迟到）——报告=docs/superpowers/reports/2026-09-07-s15-e01-video-evidence-timeout-rca.md**; 独立 reanalyze-long.py 同 verdict; 21 件 md5 盒=origin. 报告 §96. |
| `r64-stability-8h-gate/` | Current | **Step 15 8h rung 开场 gate（「错误后恢复切换」·真机）**: r64 控制面一轮=exit0 failures=0 findings=1（F2×10 确定性 10/10 数据行）; gates bin f7f7db7d·manifest 7521d17e; 3 件 md5 盒=origin. 报告 §96. |
| `s15e01-race/` | Current | **S15-E01 修复③（fix3）真机强制竞争窗场景（新 gate env VBMF_A2_8_S15E01_RACE·用户新增回归判据真机层）**: wrapper switch 委托**前**一次性注入 pre-executed 新世代 Segment 事实（fence Armed+timeline installed+executed=false=竞争窗入口·确定性强制非概率轰击; seam=同一生产函数 capture+暂存+同 seqnum 配对下游确认·零媒体流扰动）——对照轮+注入 ×10 交替=**11/11 outcome=preserved·单轮 ~152-158ms（远低 5s 证据窗·无超时无恢复）·av_epoch 1→11 恰 +1·program_epoch 保持 0（无 FailClosed→NewEpoch rebase=竞争窗被修复吸收直接证明）·segment_id 1→11·每轮 discontinuity_declared·六平面检查点 init/after-races/teardown 全 OK·注入计数对账 10/10·run2 EXIT=0**; run1 EXIT=2 如实在案=场景断言初版误抄 F2 失败轮账本（核心判据 run1 即全绿）; gates bin 51ce1477·manifest 7521d17e; 2 件 md5 盒=origin. 报告 §96 + RCA §8. |
| `s15e01-fix-r64cp-gate/` | Current | **S15-E01 fix3 回归 gate（修复不得扰动 R65 闭合语义）**: r64 控制面真机一轮（C0-C3+F2×10+F4）=exit0 failures=0·phase1 委托 7/phase2 委托 22·findings=1（F2×10 确定性 landed_to=10/10 保持）; gates bin 51ce1477·manifest 7521d17e; 2 件 md5 盒=origin. 报告 §96. |
| `s15e01-fix-dual-input-gate/` | Current | **S15-E01 fix3 回归 gate（dual_input 五层）**: ALL PASS 10/10 verdicts（L0→L5+Teardown 全链完成）; gates bin 51ce1477·manifest 7521d17e; 2 件 md5 盒=origin. 报告 §96. |
| `r64-stability-8h-rerun/` | Current | **Step 15 8h 重跑 rung（fix3 后·真服务·无注入·bin f52b0161）: VERDICT PASS（10/10）**: 960 周期×~30s（11:57-19:37 盒钟）——**switches_all_executed_preserved 960/960（首轮 959/960 唯一败项=cycle 908 S15-E01 竞争窗, 修复后全量通过·零复发·tl_ep=0 全程保持无 rebase）**; 其余九项: threads spread=2（29-31）·fd 14→14·RSS 首/末 1/3 1248.9→1276.3MB（+27.4MB 无爬升）·epoch 1→960 逐命令恰 +1·observed==target 960/960·frames v 28→829125/a 37→1105347 严格递增·drops 0·watchdog 1379→2764·events 480 采样 critical=0; replay 192/192 原样·teardown 完成行=1; 独立 reanalyze-long.py（expected_cycles=960）同 verdict; 21 件 md5 盒=origin; **旧 r64-stability-8h/ FAIL 证据原样保留（新 rung 不覆盖）**. 报告 §96. |
| `r64-stability-8h-rerun-gate/` | Current | **Step 15 8h 重跑 rung 开场 gate（「错误后恢复切换」·真机）**: r64 控制面一轮=exit0 failures=0 findings=1（F2×10 确定性 10/10 数据行）; gates bin 51ce1477·manifest 7521d17e; 2 件 md5 盒=origin. 报告 §96. |
| `r64-stability-24h/` | Current | **Step 15 24h rung（fix3 后·真服务·无注入·bin f52b0161）: VERDICT FAIL（9/10）按原样交付——唯一败项=rss_bounded**: 首/末 1/3 均值 1262.4→1349.0MB（**+86.6MB·阈值 +50MB**·monotonic=False）; RSS 形态（只读报告用·归因未做归裁决）: 十分位近似线性缓增 ~5MB/h **无平台化**（末 5h 仍 +25.3MB·峰值 1368.9MB@cycle 2868·首样本含启动谷 1118.8MB）·与 8h rerun +27.4MB/跨 5.3h≈5.2MB/h 同斜率（8h 未越界·24h 越界）·2h +1.1MB 显著平; **切换面全绿: switches 2880/2880 executed+preserved（S15-E01 修复 24h 零复发·修复后累计 4080 次切换零事件）·epoch 1→2880 逐命令恰+1·observed==target 2880/2880·tl_ep=0 全程保持无 rebase·frames v 26→2489044/a 35→3318327 严格递增·drops 0·watchdog 4143→8291·events 1440 采样 critical=0·threads spread=2·fd 14→14·replay 576/576·teardown 完成行=1**; 独立 reanalyze-long.py（expected_cycles=2880）同 verdict; 21 件 md5 盒=origin. 报告 §96. |
| `r64-stability-24h-gate/` | Current | **Step 15 24h rung 开场 gate（「错误后恢复切换」·真机）**: r64 控制面一轮=exit0 failures=0 findings=1（F2×10 确定性 10/10 数据行）; gates bin 51ce1477·manifest 7521d17e; 2 件 md5 盒=origin. 报告 §96. |
| `2026-09-10-b1-diag-4h/` | Current | **B1-DIAG 4h 诊断 workload（24h FAIL 后 RSS 构成归因·非 Step15 rung·无验收语义）**: 真服务 bin 重建 **f52b0161==24h 同一二进制**·manifest 7521d17e·CYCLES=480/DWELL=28/replay 5（Step15 同构）; runner 机械 verdict PASS 10/10=**informational only**（4h 窗内 RSS 未越 50MB 界·与事件周期模型一致）; 21 件 md5 盒=origin. 报告 §96. |
| `2026-09-10-b1-diag-4h-observer/` | Current | **B1 外部只读内存构成 observer（与 runner 完全分离·procfs-only 红线·零网络零业务端点零应用内部）**: **observer_status=COMPLETE/exit 0**（覆盖 95%·gap_max 0s·零内部错误·零 rollup 失败·rollup 13 字段全可用）·**domain T0 冻结=[1811469] 单进程 children=0 零漂移**·462 主采样(30s)+1381 轻采样(10s)+3×topology 全量 smaps 快照（160KB/223KB/220KB·不截断·md5 在 topology-index）; 脚本=r64-probe/b1-observer.sh（commit 667e9ff·部署 md5 盒=源=repo HEAD）; 11 件 md5 盒=origin. 报告 §96. |
| `b1-diag-4h-analysis/` | Current | **B1 离线分析派生产物（本地生成·非盒采集）**: composition-table.txt（构成分解+业务对齐+步进检测+topology 三点 diff）+四图（chart1 RSS 构成/chart2 PSS/chart3 vs cycle/chart4 vs watchdog）; **classification=ANON_GROWTH**——+11.2MB 100% 落 RssAnon（RssFile 恒 16.1MB）·离散周期事件模式（4h 仅一次 +11.0MB@03:18 盒钟·两个 5.5MB 子步相隔 10s·其余精确平台）·R²(sw_epoch)=0.04 与切换无线性·[heap] 3.4→4.0MB 不动·增量全在 anon rw-p 私有映射; **解释性证据·不改判 24h rss_bounded FAIL**. 报告 §96. |
| `2026-09-10-c2o1-2p5h/` | Current | **C2-O1 2.5h 定点诊断 workload（RSS RCA C 线·非 Step15 rung·无验收语义）**: 真服务 bin 重建 **f52b0161==24h/B1 同一二进制**（header 与 observer /proc/exe 双源一致）·manifest 7521d17e·CYCLES=315/DWELL=28/replay 5（Step15 同构）; runner 机械 verdict PASS 10/10=**informational only·不入梯**; 21 件 md5 盒=origin. 报告 §96. |
| `2026-09-10-c2o1-2p5h-observer/` | Current | **C2-O1 外部只读 mapping-closure observer（procfs-only 红线·v2.1 契约：双时间轴 RUN_REL_S 主轴/累计制触发器/PRE→TRIG→POST 捕获链/capture_reason 七类/DEGRADED 超限态/identity 四元组可重算）**: **COMPLETE/exit 0**（覆盖 98%·gap 0s·main 304+lite 907+fast 1454·trig 5+/0-·cap 20 未触）·domain=[1923127] 单进程零漂移·101 件（~90×全量 smaps：startup/300s base/60s 窗内密集/触发/POST/shutdown + fastlite@2s + threads 清单×2 + identity/meta/md5s）; 脚本=r64-probe 下 C2-O1 observer（本轮入库·部署 md5 盒=源=1caa0f45…·本地 fixtures 阶梯三场景 A=DIRECT/B=REFUTED/C=PARTIAL 预验证+两真缺陷修复）; md5 盒=origin 逐件. 报告 §96. |
| `c2o1-analysis/` | Current | **C2-O1 离线分析派生产物（本地生成）**: **M1/M2 定位完成——MORPHOLOGY-REFINED**: E01(run_rel 7150,+5796kB)/E02(run_rel 7204,+5632kB) 双连跳=**两个不同 64MiB 对齐预留（基址 0x713004000000/0x713010000000）各自 rw-p 头部经 mprotect(PROT_NONE→rw) 扩展一段（+5584/+5588kB）且新段立即全触（Rss==Size==Anon==PD）**; 预留基址/末端/总量恒定·VmSize 平直(窗内 Δ=12kB 瞬态)·[heap] 零增长·**ΣΔAnonymous==ΔRssAnon 逐 kB 零缺口**（5584/5588/簇 11216）·VmData 簇增 11256kB==Σrw 头扩展(5584+40+5588+44) 逐 kB 精确; 两预留热身填至 ~22.6MB 后**精确休眠 7023s** 再同步扩展·事件后再休眠至 run 末; 跨 run 首事件 7132(B1)/7142(24h)/7150(本run) 对齐 ±18s; 判定=**非 CONFIRMED-DIRECT**（rw-p VMA 末端移动·字面 same-VMA 不满足）·**非 REFUTED**（无新地址映射创建/VmSize/heap 增长·§九归因主导制不触发）·=裁决 §五 MORPHOLOGY-REOPEN 分支的精化形态成立（page-commit 方向成立·精化为 mprotect 头部扩展+立即全触）; **O1 只证 mapping 不证 arena——M1/M2 是否 arena/哪个线程归 C2-O2**; 不改判 rss_bounded FAIL. 报告 §96. **v2 模型修订注册（同日·用户 v3.1 终裁·零新采集对同批证据机判重析）**: c2o1-envelope-tables.txt（机器表·fixtures A/B/C/D 全绿·D=重划反回归）+c2o1-reservation-closure.txt（v1 superseded-by）——**O1 FINAL 升格=RESERVATION-EXTENSION·BOUNDARY-MIGRATION**（两级身份 address_extent≠reservation_envelope 四证·R2 处 832MiB 连续链 over-merge 由窗口四证化解·簇三方逐kB全等 11216·前奏 +40/+44 精确定位各自早于大扩展 ≤60s/36s）; **c2rules 四句总原则+状态机冻结+O2 裁决矩阵控制表注册**; **C2-O2 设计注册=O2-A（ALLOCATOR-STRUCTURALLY-COMPATIBLE 语义）/O2-B（activity-correlated TID set）+STOP 条件——O2 执行 NOT AUTHORIZED·entry satisfied no execution implied**. 报告 §96. |
| `2026-09-10-c2o2-2p5h/` | Current | **C2-O2-B workload（RSS RCA C 线·用户「授权」含红线扩围批复·非 Step15 rung）**: 真服务 bin 重建 **f52b0161==24h/B1/O1 同一冻结二进制**·manifest 7521d17e·CYCLES=315/DWELL=28/replay 5; runner 机械 verdict PASS 10/10=**informational only 不入梯**（315/315 切换全绿·RSS 首/末 1/3 1238.3→1246.4MB 窗内 +8.1MB）; 21 件 md5 盒=origin. 报告 §96. |
| `2026-09-10-c2o2-2p5h-observer/` | Current | **C2-O2-B 外部只读 activity-correlation observer（c2o1 v2.1 全契约继承+per-tid {comm,stat} 2s 采样不加快+触发全线程快照+tid 生命周期+一次性 libc 钉扎）**: **COMPLETE/exit 0**（terminal=gone·覆盖 95%·gap 0·tid_ticks 1443·tid_rows 41847·**tid_stat_err 0·born/gone 0**·trig 2+/0-·域冻结零漂移）; **libc 钉扎=glibc 2.43（Ubuntu 2.43-2ubuntu2.4）**; 部署 md5 盒=源=repo（2e195d41）·fixtures 四轮全链绿（两真缺陷修复于部署前: task 路径段+尾空格 flap）; 99 件 md5 盒=origin（含 17 组休眠期相邻快照字节相同=静止形态互证）. 报告 §96. |
| `c2o2-analysis/` | Current | **C2-O2 执行记录+双 verdict**: **O2-A=ALLOCATOR-STRUCTURALLY-COMPATIBLE**（零新增采集·40 保留形态窗口四普查点恒定+832MiB 链 13/13 分解+[heap] 事件零相关+glibc 上下文·表述纪律禁 "this is glibc arena X"）; **O2-B=事件准点复现 E1@7149/E2@7200（win==VmData==+5588 逐kB·冻结阶梯机判均 BOUNDARY-MIGRATION·本 run 全新 ASLR 基址 0x732284000000/0x732280000000 相邻 64MiB·census=40 复现）·thread_activity_visibility E1/E2=AMBIGUOUS**（decklinkvideosr×2 恒忙面 ev==base_max 无事件独特激活·5.5MB 触页 CPU 低于 10ms tick 粒度地板·线程集 29/29/29 零漂移·STOP 纪律执行未加速采样）; active TID≠causal TID·arena ownership/因果线程未答=O3 职责; **停等 O3 裁决**. 报告 §96. |
| `c2o3-analysis/` | Current | **C2-O3-0 静态 candidate inventory+O3 设计注册（本地生成·零盒操作·零生产代码改动）**: **判定 COMPLETE（=inventory 完成·非 root cause complete）**——三层候选分层 A Rust 显式 retention 已量级证伪（V3 定量 ≤1.59MB/24h vs ≥86.6MB·差两数量级）/ B Rust→native allocation pressure 有 morphology-compatible 路径未证 / C GStreamer/native 内部状态仅待验证假设; **V1 recover 候选 EXCLUDED**（四 run svc.log 11,703 行 fault/recover 零命中+O2-B born=0 双轴反证）; **V2 首事件四 run 全锁 cycle 249**（计数坐标稳定·已注册计数候选成员级全证伪: fresh-command≈199≠幂次 128/256·簇距 246±9 非几何; 判读按冻结句执行不跳机制层）; make_mut（switch_graph 探针）命名=allocator-behavior candidate / ring3-compatible candidate·永不 root cause; **E2 判读四行表+单一问题句冻结**（分叉证据非因果·单输入事件消失≠"双输入路径导致泄漏"）; O3-1（含 E2=VBMF_DIAG_INPUTS=1 config-only 差分）**需独立授权·未授权**. 报告 §96. |

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
