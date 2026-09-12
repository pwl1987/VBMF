# C2-O3 E2 离线目标形态学分类（authoritative E2 interpretation）—— 2026-09-12

## 0. 权威层级声明（用户指令 2026-09-12 冻结）

- `e2-summary.txt` = **harness mechanical output**（`trig_positive > 0` 累计 ±1MiB 门的机械输出，保留原样，未覆盖）。
- **本文件 = authoritative E2 interpretation**（existing-evidence-only 离线目标形态学分类）。
- 两者冲突时以本文件为准。harness 三值输出中的 `SINGLE_INPUT_EVENT_OBSERVED` 以本分类为准修正登记（见 §5）。

## 1. 输入与方法（零新采集）

- 数据：盒侧现存 90 个 `topology-*.smaps`（existing evidence only，md5 对 `topology-index.txt` 90/90 全对）+ `captures-status.tsv`（每捕获点 status 九字段）+ `trigger-log.txt`。
- 方法：O1/O2 冻结的 reservation-envelope 判定（`c2o1-reservation-closure.txt`）——两级身份（address_extent ≠ reservation_envelope·四证：stable base/end / cross-snapshot persistence / RW↔PROT_NONE conservation / permission-transition continuity；64MiB 对齐=supporting only 非判据）；目标子态语义逐字：`RESERVATION-EXTENSION·BOUNDARY-MIGRATION = 恒定 envelope 内 RW 头扩展 + PN 尾等量收缩`。
- 扫描：全部 89 个时间相邻对（全窗，防确认偏差）+ 必查子集（冻结目标窗 6900–7300 含 7083/7143/7203/7263；4806→5106；7 个 trigger 的 PRE→TRIG 对）。守恒容差 8kB（procfs 非原子快照披露继承）。
- 禁项遵守：未重运行 media-agent；未改生产代码/runner/observer 参数/旧证据；分析器按 B1/O1 先例会话内联执行（Mimosa 拦文件化），机器表入库=审计载体。

## 2. 发现（全部为机械事实）

### 2.1 稳态形态学：目标形态**在场**，但呈连续小步而非量子

- 持久 envelope `[0x78e334000000, 0x78e368000000)`（832MiB 匿名链·64MiB 对齐·全窗 base/end 恒定·89 对全持久）。
- **每个** 300s 相邻对（306s→8710s 全程）与每个 60s 窗内对均满足：
  `rw +Δ | pn −Δ | conservation=+0 | ΔVmSize=0 | Δheap=0 | ΔVmData=ΔRssAnon=Δrw（逐 kB 零缺口）| new_out_rss=0（增长全部落在 reservation 内）`。
- 单粒度定位：迁移**独占地**落在链内单一 64MiB unit `0x78e338000000`，warm-up 后速率严格均匀 **+52 kB / 60 s（≈3.1 MB/h）**，全窗累计 +24,616 kB；链内其余 unit 89 对全部零变化。
- **无平台期、无 ≥1MiB 量子步**（冻结节拍=7043s 精确平台后 ~5,584kB 离散量子；本 run 最大单对步长 260kB@300s），冻结目标窗 6900–7300 亦为纯爬升（win 对逐对 +4..+52kB，无 7083/7143/7203/7263 定位）。

### 2.2 4806→5106：ENVELOPE-INTRODUCTION（非目标事件）

- `ΔVmSize=+67,588 kB = 65,536（新 64MiB envelope @0x78e32c000000，rw 头 132kB / PN 尾 65,404kB）+ 2,052（0x78e381dfb000 区域重构：intro 100,372kB − gone 98,320kB）`，逐 kB 对账精确。
- `ΔRssAnon=+308 kB`（纯地址空间事件，驻留近零）；Δheap=0；VmPeak 瞬时 1,469,156→1,602,268 kB（新旧共存窗披露）。
- 按冻结子态语义登记为 `ENVELOPE-INTRODUCTION`（allocator-structure clue·不等价 arena creation·**非** RESERVATION-EXTENSION·BOUNDARY-MIGRATION 目标事件）。census（≥32MiB extent 数）4→5。

### 2.3 7 个 observer trigger：机械证实为累计里程碑（用户预判成立）

- 7/7 个 PRE→TRIG 对：stable drw=+0（≥256kB 级零结构变化）、无 envelope intro/gone、ΔVmData=ΔRssAnon=+4..+144kB（即爬升本身的余量）。
- 结论：`TRIG_KB=1024` 累计门每积满 ~1MiB（≈19 分钟爬升）触发一次；**trigger ≠ 事件**。harness 的 `E2_RESULT=SINGLE_INPUT_EVENT_OBSERVED`（trig_positive>0）在形态学口径下为误判，依 §0 权威层级修正（§5）。

### 2.4 热身段披露（6→306s）

- 唯一 ≥1MiB 边界迁移对：q=+17,240 / cons=+0 / heap=0，但 ΔVmData(11,608)≠q、new_out=−5,672（warm-up 期其他运动）→ **不满足完整冻结签名**，按 O1 同族热身处理（O1 先例：R1 自身热身含量子同族填充步·supporting only），不判为目标事件。

## 3. 每候选对字段

全字段机器表见同目录 `offline-morphology-tables.txt`（T1 清单 90 快照 / T2 全窗结构变化对 / T3 4806→5106 / T4 trigger 对 / T5 冻结窗 / T6 签名命中 / unit 定位表 / 三条 exemplar 全字段行：run_rel PRE/TRIG·envelope base/range·rw before→after·Δrw·PN before→after·Δpn·conservation·ΔVmSize·ΔVmData·ΔRssAnon·ΔAnonymous·Δheap·new_out_rss·形态判定）。
命中对 PRE/POST smaps 8 件已回收入库：`smaps-offline-hits/`（md5 与盒侧 `observer/md5s.txt` 逐件一致；选取=VmSize 事件对 base-4806/base-5106 + 冻结窗坐标对 win-7143/win-7203 + 干净签名 exemplar 对 base-5406/base-5706 + trigger 必查对 win-6962/trig-6）。其余 82 个 smaps 留盒（已登记 md5/行数）。

## 4. 分类（三值 + 标记）

```
E2_OFFLINE_CLASSIFICATION = TARGET_MORPHOLOGY_OBSERVED
WORKLOAD_SENSITIVE = YES
```

依据：冻结目标形态（恒定 envelope 内 RW 头扩展+PN 尾等量收缩+守恒+heap 平直+增长零外溢+ΔVmData==ΔRssAnon==扩展量）在单输入下**复现**且全程在场；但**周期性**（双输入=7043s 平台后 ~7140–7200s 首事件离散量子；单输入=自 306s 起连续 +52kB/min 无平台无量子）与**量级/步长**（~5.5MiB 量子 vs 260kB/300s 小步）改变，且**空间呈现**收敛为单一 unit 连续迁移（双输入为两 unit 离散跳变）。按指令 5 加标 WORKLOAD_SENSITIVE；**不因此指定任何具体 allocator/object**（O3 六环硬门不变）。

## 5. 允许结论（逐字·仅此一条）

> 双输入不是必要条件，候选向 common ingest / native allocation path 收敛。

（依据从"harness 计数器"升级为"冻结形态学复现"；仍为分叉证据，非因果结论、非 root cause。）

## 6. 红线执行与登记

- 未重跑/未延长观察窗；`e2-summary.txt` 未覆盖；生产代码/runner/observer 参数/旧证据零改动；PR #42 保持 Draft 不合并；不进 O4/FIX；Step15 24h FAIL 不翻案。
- **Mimosa 登记（tooling/audit issue·与本次 RCA 判读隔离）**：`python_ast_unavailable` 兼容放行（累计：上轮 commit/push 各一次）+ 本轮分析器文件化被拦（路径污点误报·B1/O1 同类第 4 次）。均为工具链审计事项，不构成本轮 blocker，不据此宣称任何安全结论。
- 披露：smaps 顺序 procfs 非原子；832MiB 链级身份成立+64MiB 网格定位为 supporting；跨 run 地址不可比（ASLR）；`rss_delta_kb=367700`（harness）含预热前基线（cycle1=420MB），观察器口径 RssAnon 全窗 +19,048kB、smaps ΣAnon +24,720kB，与本分类一致。
- **STOP：本步完成即停，停等 O3 下一刀裁决；不得自行进入 O4/FIX。**
