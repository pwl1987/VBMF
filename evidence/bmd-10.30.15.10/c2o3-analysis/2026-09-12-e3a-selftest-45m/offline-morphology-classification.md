# C2-O3-2 / E3A Source-Family Differential 离线目标形态学分类 —— 2026-09-12

## 0. 权威层级（用户裁决 2026-09-12）

- 本目录原始证据 = harness 收集（45min 定长·无 trigger·无 per-TID observer）。
- **本文件 = authoritative E3A interpretation**（离线形态学分类，判据逐字复用 O1/O2 reservation-envelope classifier）。
- 本轮非 Step15 rung：无 PASS/FAIL，24h FAIL 原判不变，不授权 FIX。

## 1. 执行记录（零代码改动）

- 条件：既有 `MEDIA_AGENT_SELFTEST=1` 路径（bin/media-agent.rs:98 `skip_decklink`→`PipelinePlan::self_test()`，代码事实已核：该分支**完整跳过** DeckLink canonical 路径与 SessionManager/diag-inputs 块）；同一 GStreamer controller + `video/x-raw` + tee + queue + appsink；`MEDIA_AGENT_MODE=diagnostic` + 同 manifest env（config-only）。
- 环境同一性：同盒 10.30.15.10 · kernel 7.0.0-30-generic · **glibc 2.43** · 同 `build-bmd.sh` · **exe_md5=`ad89c6d3`（与 E2 二进制逐字节相同 → 同一进程 allocator 确证）**。
- 运行健康：`MEDIA-RT-01: A+B+C 全过` 累计 5,397 行（videotestsrc/audiotestsrc 全程持续出帧，video/audio frames 计数在 svc.log）；/health 2s 背景查询（E2 管理面负载对齐）；svc.log 零 ERROR；零 abort；45min 定长完成（elapsed 2700s）。
- 观察配置：warm-up 0–300s；steady 300–2700s（实测 274→2707s）；full smaps 300s 周期（start + base-300…base-2700 + end = 11 快照）；status+smaps_rollup 30s（status.csv）；进程 identity 一次锚定（identity.txt：pid/cmdline/exe_md5/btime/starttime/clk_tck/libc 钉扎）。

## 2. 分类结果（机器表见 offline-morphology-tables.txt）

### 2.1 STEADY 段（274s→2707s，9 个相邻对）：绝对静止

- 每一对：`ΔVmSize=+0 · ΔVmData=+0 · ΔRssAnon=+0 · Δheap=+0 · ΔAnonTotal=+0`；**≥16kB 级零结构变化**（无 stable-envelope rw/pn 任何方向迁移·无 intro/gone）。
- 快照恒等：VmSize=681,636 / VmData=62,368 / RssAnon=6,512 kB，base-300 至 end 逐字节不变。

### 2.2 WARM-UP 段（4→274s）：重构型扩容，非目标形态

- `ΔVmSize=+67,588 kB`：extent `0x792964000000` 65,536→589,824 kB（**dpn=+523,232**·drw=+1,056，PROT_NONE 增长型扩容）+ rw-only INTRO 41,356kB + 两处 GONE 重构；ΔRssAnon 仅 +32。
- 无守恒边界迁移（dpn 与 drw 不构成 RW+Δ/PN−Δ 对偶）→ 按 warm-up 处理，不判事件。
- 披露：净 `ΔVmSize=+67,588` 与 E2 4806→5106 ENVELOPE-INTRODUCTION 净量**数值相同**——登记为 allocator-structure supporting clue 级观察（巧合未判机制·不开启第二 RCA 支线·裁决 10）。

### 2.3 检测力论证（机械·非因果）

E2 steady 速率 +51.7 kB/min；若同族形态在本轮 40min steady 窗内存在，应产生 ≈+2.1MB 的 ΔVmData/Δrw。本轮分类下限 16kB，全窗零观测——**NOT_OBSERVED 不是窗口长度伪影**。

## 3. 判定（四值之一）

```
E3A_CLASSIFICATION = SELFTEST_MORPHOLOGY_NOT_OBSERVED
```

（不附加 WORKLOAD_SENSITIVE：不是"形态在场但速率/量级改变"，而是完全缺席。）

## 4. 允许结论（fenced·逐字）

> 只能说 **"selftest 所移除的条件集合"**——DeckLink source/plugin/hardware + 诊断 SessionManager materialization 路径 + 会话图自动启动——**成为底层 reservation-boundary morphology 的必要条件候选**。
> 由于 selftest 同时绕过部分 diagnostic SessionManager/materialization 路径，**不得直接写成 "DeckLink 导致问题"**。
> 下一刀需设计更严格的 source-only substitution（例如保留会话/物化路径、仅将 decklinkvideosrc/audiosrc 替换为 test source element）以分离两个被合并移除的条件。

## 5. 候选降权登记（用户裁决 9）

以下候选已被 E2 证明为底层形态学**非必要条件**，本轮维持降权：**Program Graph / MediaTap-inter bridge / switch_graph::make_mut / dual-input switching** —— 保留 workload-shaping candidate 身份（可解释双输入下量子化/平台期呈现差异），**退出底层 RCA 主线**。

## 6. 红线与披露

- 六环硬门不变：`candidate → allocation path → allocator behavior → same reservation/arena → magnitude → repeatability`；E3A 只推进 candidate/allocation-family 必要条件分叉，不授权 FIX。
- 本轮无 per-TID observer（裁决 5）；无 trigger 判事件（裁决 7）；45min 未因结果延长（裁决 4）。
- 量级对照披露（WARM-UP/STEADY 口径·E2 勘误规则）：E2 steady +7,376kB/142.6min vs E3A steady 0kB/40.5min；两进程工作集本身差异巨大（RssAnon 769MB vs 6.5MB）——该差异属于"selftest 所移除的条件集合"的解释域，本轮不拆分。
- Mimosa `python_ast_unavailable`：tooling/audit issue 登记延续，与 RCA 判读隔离，不构成 blocker，不宣称安全结论。
- **STOP：E3A 完成，停等下一刀裁决。**
