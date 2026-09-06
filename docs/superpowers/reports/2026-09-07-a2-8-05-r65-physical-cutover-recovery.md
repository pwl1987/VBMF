# R65：物理 Cutover / Recovery 时序修复（Physical Cutover Recovery）——修复轮第三刀

- 轮次: R65（用户 R65 裁决：R64 三发现分级 P1×1+P2×2 全 BLOCKING·A0→A1→A2→B→R64-6' 梯子）
- 基线: R64 交付 7e16fcc（综合验收 PASS + 三发现登记 + 30min FAIL 5/9 原样交付）
- 性质: 代码轮（失败恢复**观测时机**修复）——生产触碰收敛到 program_execution.rs
  单文件（比用户授权面 program_execution/program_timeline/switch_execution 更窄，
  理由见 §3.4）；switch_graph·switch_mock·contracts/switch·program_timeline·
  switch_execution·command·idempotency·api_boundary·transport·bin 全零触碰。
- 载体: commit1 = 本 A0 契约（docs-only，镜像 R63-A0 a6ecbb0 格式）；commit2 =
  A1 实现+mock 矩阵+A2 真机；commit3 = R65-B；commit4 = R64-6' 30min 重跑。

## 1. 逐条确认表（用户 R65 裁决 → 落实归属）

| # | 指令项 | 状态 | 归属 |
|---|---|---|---|
| 1 | 三发现正式分级（P1 必修/P2 恢复观测必同修/P2 C3 后置） | 已按裁决冻结（§2/§3/§5） | 本轮 |
| 2 | R65-A0 Physical Cutover Recovery Contract（契约先行不改行为） | 本轮落实（本报告 §2/§3·契约 commit 先于代码 commit） | commit1 |
| 3 | R65-A1 实现（fence 终结→物理 settle→observe→reconcile） | 本轮落实（§4） | commit2 |
| 4 | F1-F4 全场景再覆盖 + F2 真机回归 N≥10 确定性 | 本轮落实（§4/§6） | commit2 |
| 5 | complete_switch 冻结（reconcile=失败恢复职责分离） | 已遵守（两函数语义零改动） | 结构性 |
| 6 | R65-B C3 可观测性收口（不降 PTS FailClosed） | 本轮落实（§5·落点=入口早卫兵，见 §5.1 裁决记录） | commit3 |
| 7 | 30min 不刷绿——R65 后重跑 + 谓词修订单独登记 | 本轮落实（§7·谓词 v2 冻结于 §7.2） | commit4 |
| 8 | Step 15（2h/8h/24h）/Step 17 | 不启动（R64-6' 交付后停，等用户裁决） | 后续 |
| 9 | PR #30 收口 | 全程不 merge（CI 信号通道） | 链末 |

## 2. 机制更正（A0 第一个产出——更正 R64 登记建议的措辞）

R64 三发现登记的修复方向为"把恢复落定后移至守卫 Drop 强释之后"。三路只读
勘探 + 本人核验实证（2026-09-07）：

1. **守卫 Drop（force_open 调用）在源码层已经先于恢复执行**。`fence` 是
   `switch_program_locked` 的局部（program_execution.rs:651）：release-Err 时
   守卫在 `confirm_and_release`（按值消费，:976-983）帧内 Drop 强释；更早的
   `?` 失败在该函数返回时 Drop。恢复（:631）总是在 Drop 之后。:627-630 注释
   "恢复先于 Drop"失准——R64 报告同措辞随本节更正登记。
2. **真正的缺口在物理层**：`force_open()`（switch_graph.rs:526-532）仅置双面
   FenceState::Open 即返回——不回滚 selector、不等待缓冲流。物理翻转在屏障
   打开后的缓冲流时刻迟到（R64 实测整链 ~400ms）。恢复的单发 observe
   （:829）落在该窗口内 → 三跑三态（L1 Some(from) 落 Active(from)+迟翻分歧
   死锁 / L2 None 终态 / L3 Some(to) 自洽）。
3. **修复 = 在 force_open 之后、reconcile 之前插入"期望感知稳定再观测协议"**
   （用户 §四 settle 要求的落名），非源码顺序重排——顺序已就位。

## 3. A0 契约冻结文本（Failure Recovery / Physical Cutover Ordering Contract）

> **SoT 续冻结：Observed 优先，absence ≠ false。本契约不改变 R63-A0 的任何
> 落定语义——只改变"何时信任一次观测"。**

### 3.1 时序契约

失败 → ① 捕获原始 Err → ② fence 终结（既有 guard Drop 强释，语义冻结）→
③ **物理稳定再观测**（§3.2 协议）→ ④ 组平面 reconcile（既有
`reconcile_switch` 语义零改动）→ ⑤ 时间线平面 reconcile（既有
`reconcile_executed_failure`/`abort_transition` 语义零改动）→ ⑥ published
出口发布 → ⑦ 原始 CommandOutcome 保持 Failed → ⑧ replay 保持原 outcome
逐字节。

### 3.2 期望感知稳定协议（复用既有常量，零新时间语义）

轮询节奏 `TIMELINE_POLL_INTERVAL`(50ms)、稳定窗 `TIMELINE_SETTLE_ROUNDS`(3
连续同值)、界 `TIMELINE_SETTLE_TIMEOUT`(5s)——全部复用 ⑨ settle 同族常量；
协议只读 `observed_active`，**不喂时间线**（纯观测稳定器，非第二套时间语义）。

- **executed=true**（本轮 `adapter.switch()` 返回 Ok——翻转已执行、必落）：
  只接受**连续 3 次 ==Some(to)** 为落定。
  - 稳定 from **不可信**（迟翻窗口伪稳定——期望规则的存在理由）；
  - None **不可信**（V/A 分歧瞬态或真未知）；
  - 界尽(5s) 未获稳定 to → **视为未知 → RecoveryRequired 诚实终态**
    （硬件 5s 内必落 [R64 实测 ~400ms]，界尽=真异常，诚实不猜）。
- **executed=false**（switch() Err 或未到达——翻转未执行/已同步回滚）：
  普通稳定协议（任意连续 3 次相同值即落）——Some(from)→abort 语义回旧源；
  稳定 None→RecoveryRequired（degraded 语义保持）。
- executed 标志经 `Inner` 新私有 bool 字段传递（chain 顶复位 false；:690
  Ok 后置 true）——不经过错误字符串、不经过相位推断（on_switch_executed
  失败路径的硬件事实只有该位点持有）。

### 3.3 三态落定表（对应用户 §五——分叉态从此不可构造）

| 落定 | 触发 | Desired | Timeline | 下一次切换 |
|---|---|---|---|---|
| 旧源 | executed=false + 稳定 Some(from) | Active(from) | abort 语义（epoch 不变） | ✓ 合法 |
| 新源 | executed=true + 稳定 Some(to) | Active(to) | Stable{to}+ProgramEpoch+1+DD+基线清空 | ✓ 合法（反向再切） |
| 无法确认 | 界尽 / 稳定 None | RecoveryRequired{from,to} 终态 | 诚实停留（相位不变） | Permanent 拒收；唯一出口=teardown |

"Desired=A / Observed=A / Physical=B"不可构造：落 Active(from) 仅在
executed=false（此时物理确未翻转——真实 adapter 翻转失败在同锁内同步回滚）；
executed=true 时 from 稳定不被接受。

### 3.4 冻结面与触碰面

- **生产触碰 = program_execution.rs 唯一**：Inner 私有字段 + 协议函数 +
  recover 单发观测替换 + 失准注释修正 + 文内单测。落定逻辑两函数
  （reconcile_switch / reconcile_executed_failure）与 complete_switch /
  force_release / force_open / watchdog 语义**全部零改动**——病灶在观测
  时机，不在落定逻辑（R63-A 已把落定逻辑修正确）。
- **测试脚手架**：tests/switch_fault_probe.rs（+2 测试）、
  src/gates/r64_control_plane.rs（C2b 确定性重写 + F4 行）、
  r64-probe/（30min 脚本谓词 v2）。env/bin 不变。
- 恢复窗（≤5s，仅失败路径）持有 inner——与既有 ⑤-⑨ 同纪律；查询回退
  published 快照语义不变（R63-B3/R64-4 已验收）。

### 3.5 R65-B 落点裁决记录（入口早卫兵）

用户 §九 示意机制=时间线重基（复用 epoch+1/新段/基线清空）。经分析：
observed=None 时重基必须为 `Stable{source}` 选源（from 或 to）——在"观测
未知"上声称观测到了某源，与冻结原则 absence≠false 冲突；否则须发明新相位
（TimelinePhase 契约手术，program_timeline.rs 大开）。问答未获答复，按
推荐冻结（计划批准即裁决）：

- **落点 = 入口早卫兵**：`switch_program_locked` ⓪ 之前 group.desired ==
  RecoveryRequired → 直接 `Err(SwitchError::RecoveryRequired)`（classify=
  PermanentFailure，与 R63-A 词表一致）。下一次切换在 fence 装甲 / ①a PTS
  喂入之前被诚实拒绝——C3 遮蔽（①a PTS FailClosed unknown 抢先）消除，
  PTS FailClosed 正确性原样，时间线零触碰。
- **重基登记为残留**：若真机仍现任何 ①a 遮蔽形态再议。

### 3.6 验收谓词 v2（R64-6' 重跑用——单独登记的谓词修订，非静默改脚本）

| 谓词 | v1（R64-6） | v2（R65 裁决） |
|---|---|---|
| threads | 恒定 | **有界振荡 max−min ≤ 4**（per-connection 线程模型本性） |
| switch_epoch | 首尾差=+60 | **逐命令恰 +1**（60 成功命令 → 绝对 epoch 恰 +60） |
| observed_tracks | 序列首尾 | **逐命令 observed == target** |
| watchdog_ticks | 递增（warn 级测不到） | **RUST_LOG=info 运行**，tick/teardown 行可测且递增 |
| RSS/fd/drops/critical | — | 原样冻结（上轮全 PASS） |

## 4. A1 实现 + A2 真机（commit 2）

### 4.1 diff 清单（生产 1 文件 + 测试脚手架 2 文件）

| 文件 | 改动 | 性质 |
|---|---|---|
| src/program_execution.rs | Inner 私有字段 `switch_executed_in_attempt`（chain 顶复位/switch() Ok 后置真）+ `observe_active_settled`（⑨ 同型 streak+deadline+sleep 循环, 期望规则 §3.2, 只读 observed_active 不喂时间线）+ `SettleStreak`/`settle_accepts` 纯决策核 + `recover_after_failed_switch` 单发观测替换 + :627-630 失准注释更正 + `r65_settle_tests` 纯单测 ×3 | **生产唯一触碰** |
| tests/switch_fault_probe.rs | FaultProbeAdapter 增迟翻/振荡两旋钮（`late_flip`=force_release 后前 6 次 observe 滞报旧源·`oscillate_observed`=真值/None 逐次交替）+ 新测试 ×2 | 测试脚手架 |
| src/gates/r64_control_plane.rs | 阶段二重写: 三变体容忍 C2b → `phase2_c2b_f2_deterministic_loop`（F2×10 确定性 + 反向再切 + 精确记账断言）+ F4 行（`regress_pts_from` **阈值制**旋钮——锚定本轮 switch 委托后, 避免 ①a 提前点火）+ `checkpoint_finding` 删除（无调用点） | 测试脚手架（gate） |

### 4.2 盒矩阵（最终代码态）

fmt 0（产物 scp 回·md5 双侧一致）/ default **232**（229+3 纯单测）/ simulation **232** /
mock **434**（414+9+11=旧 9+新 2+纯 3）/ hw **276**（273+3）/ gates bin hw 构建成功
（md5 1e4f0372）/ clippy ×3（default/mock/hw）全 0。工程坑两笔如实登记:
① clippy `manual_is_multiple_of`（`n % 2 == 0`→`is_multiple_of(2)`）; ② **R62 坑复发**——
`cargo test --features mock` 会以 mock 特性重建 debug bin, 在显式 hw 构建**之后**跑了
mock 腿复跑导致 gates bin mock 化（provider=mock + r64 派发块被编译掉→"未命中任何
gate env"）; 重建后恢复（顺序纪律: hw bin 构建必须是最后一次构建）。

### 4.3 A2 真机（gates bin `VBMF_A2_8_R64_CP=1`, attempt2 规范 run exit 0）

- **F2×10 确定性: 10/10 落 Active(to)**——R64 四跑三态（L1 死锁/L2 终态/L3 自洽）
  **零复现**; 每轮记账精确: 失败轮委托 plan epoch=2i−1 / 反向=2i / tl=i, 10 轮后
  av=20·tl=10; 六平面检查点全 OK; 同 id replay 原样; 失败切换全程 ~100-152ms
  （force_open 后物理翻转快速落定, 期望协议 ~3×50ms 收敛）。
- **F4 行**: ⑨ 回注矛盾（observed 1 vs 真实基线 ~10.3e9）→ FailClosed → 确定性落
  Active(to) + NewEpoch(11) + av=21 → 反向再切 preserved（av=22）→ teardown OK。
- 阶段一回归全绿: C0/C1（replay+retry av 直跳 4）/C2=F3（真 5.12s 证据超时→落 A+
  tl=1+DD→反向 preserved）/STORM（窗内快照=旧已提交·无未来时间戳）/C3（RecoveryRequired
  终态→c3-next 本次=permanent recovery-required 干净形态·拒收零委托·teardown 诚实缺席）。
- **attempt1（如实归档）**: F4 首版旋钮为布尔门（switches_done≥1）——gate 世界携带
  20 次前置切换史, 回注在 ①a 提前点火=0a pre-begin 形态（落 Active(from)+abort 语义,
  链完全自洽）→ 7 项断言 fail; 修复=阈值制锚定本轮 switch 委托后。**产品代码零改动**
  ——纯 gate 场景设计错误, attempt1.log 全量存档。

### 4.4 证据

`evidence/bmd-10.30.15.10/r65-cutover-recovery/`: header.txt（时间/bin+manifest md5/env）+
gate-run.log（attempt2 规范 exit0）+ gate-run-attempt1.log（F4 场景错误全档）+ md5s.txt
（盒=origin 逐字节一致）。gates bin md5 1e4f0372·manifest 7521d17e（钉扎吻合）。

## 5. R65-B（commit3 填充）

（占位——§3.5 早卫兵落地 + mock 测试 + gate C3 断言更新 + 真机 C3 复跑。）

## 6. 披露（A0 时点）

- watchdog 不接线于 gates 矩阵（a204 先例沿袭）；恢复窗内 watchdog 经
  adapter observe 独立通路不受影响（R63-A 已单测）。
- mock 无法自然产生迟翻（翻转在 switch() 内同步落）——P1 回归以测试包装器
  模拟迟翻序列表达（force_release 后先报 from 连续 K=6 次再报 to），首次
  使该缺陷在 mock 侧可回归。
- 本契约不改变 0.7C 冻结契约的任何 wire 词表/classification 映射。
