# E2 离线分类勘误：WARM-UP / STEADY 拆分口径（纯文档·零证据改动）—— 2026-09-12

用户裁决补记：`offline-morphology-classification.md` §2.1 中 "+24,616 kB" 为目标 unit `0x78e338000000` 自 run start（run_rel 6s）到 end（8865s）的**全窗总迁移**，其中含 warm-up 段大头贡献。拆分如下：

| 段 | 区间 (run_rel) | 迁移量 | 速率 |
|---|---|---|---|
| WARM-UP | 6s → 306s | **+17,240 kB**（唯一 ≥1MiB 对·签名不完整·不判事件） | ≈57 kB/s |
| STEADY | 306s → 8865s（8,559s） | **+7,376 kB（≈+7.4 MB）** | **≈51.7 kB/min（≈+52 kB/min）** |
| 合计 | 6s → 8865s | +24,616 kB | — |

**约束登记**：后续任何 magnitude 比较（跨 run、跨条件、E3A 及以后）必须区分 WARM-UP / STEADY 两段口径，不得拿全窗总量与对端 steady 速率直接对比。§2.1 的 "+52 kB / 60 s" 仅描述 steady 段；机器表 unit 定位列中的各 pair 增量不受影响。

本文件为纯文档勘误：不改既有证据文件、不改机器表、不改分类结论（`TARGET_MORPHOLOGY_OBSERVED + WORKLOAD_SENSITIVE` 维持）。
