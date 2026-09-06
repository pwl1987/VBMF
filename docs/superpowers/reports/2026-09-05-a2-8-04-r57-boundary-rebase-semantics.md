# A2-8-04 R57: 边界回退语义裁决支持（Boundary-Rebase Semantics — 只读重建）

- 状态: **READ-ONLY ADJUDICATION SUPPORT（R57, 2026-09-05）**
- 基线: 2972fb8（远端 == 本地, R56 推送后同步）
- 裁决授权（用户 R56 终裁原文要点）: 选 **(b) 严格版**——A2-8-04 =
  **FAIL / HOLD FOR BOUNDARY-REBASE SEMANTICS ADJUDICATION**; 三项
  blocking（P1-pr_v / P2b-pr_v / P2c-1）不撤销、不豁免、不修改;
  FAIL 定性 = "Semantic adjudication required"（非 "R53 实现已证伪"）;
  **P6b 升级 FieldPending → FieldProven（仅生命周期证据域, 不改变
  blocking verdict）**; 本轮零代码、零判据、零 Gate、零阈值。
- 本文档性质: 执行层只读重建 + 分析输入。**合法/非法的裁定权在验收层**;
  本文把问题锐化到可裁形状, 不替裁。

---

## §1 证据基础（本轮全部只读）

- 现场证据: 证据盒 `~/a2-8-02i-evidence/2026-09-05-r56-a204-final-gate/`
  （run1-obs-n30-dwell1000.log + nm-event-switch8-extract.txt; R56 已
  P8 身份链验证——bin md5 7a0ed95c、72/72 源 sha 盒==HEAD 2972fb8）。
- 代码锚点（本轮逐行核验, 全部只读）:
  - `program_timeline.rs`: AnchorPair :57-67 / declare offset=锚差 :98-121 /
    map_pts :124-129 / on_mapped_buffer :584-635（映射校验 :601-607、
    边界 DD :618、连续性 :620-624、last_program_pts :627）/
    close_transition :639-729（Preserved ⇔ 双 Continuous :660-661）/
    on_program_pts :747-772（回退→NM+Violated+fail_closed :753-763）/
    snapshot :790-803。
  - `program_execution.rs`: switch_program :585-723（①a 预喂 :596-608、
    锚采样 :610-613、declare :615-623、**install :625-627**、flip
    :629-636、⑤⑥⑦ 轮询 :641-668、⑨ settle 轮询 :669-705）; 轮询节流
    常量 :738-742（**POLL=50ms**、SETTLE=3 轮）; feed_authority :756+;
    SixPathEvidence/path_row :204-247（pr 行 :339-340、av_delta
    :341-344）。
  - `switch_graph.rs`: attach_plane_probes :159-209（⑤ EVENT 探针
    :170-185、⑥⑦ BUFFER 探针=映射施加+PTS 原位改写 :190-207）/
    apply_declared_mapping :216-247（passthrough 门 :221-227、**段内
    逐缓冲回退谓词 :238-243**）/ note_declared_boundary :259-282（clean
    判定 :265-268、**干净边界 DD 释放 :273-280**）/ plane_row_state
    :289-304 / attach_program_video_sink :422-446（**plain 逐缓冲喂入
    :439**）/ switch :767-850（video→audio 翻转 :822-824、executed=true
    :843-845）/ install :852-910（**状态整体置换, executed=false**）/
    sample_switch_anchors :912-973（**program_anchor=健康弧末值
    :946-954、source_anchor=目标分支末值 :955-962**）/ observe
    :997-1105（**pr 行 st=健康弧 :1058-1067**）。
  - `pipeline.rs`: plain 四态 :323-333 / declared 四态 :354-365
    （**"即便有声明, PTS 回退仍 → NonMonotonic sticky——映射的职责就是
    连续; 禁以声明洗回退" :353-356**）。
  - `a204_obs.rs`: 窗口节拍常量 :60-63（SAMPLE_GAP=3s、POST_SWITCH=2s）/
    snapshot_row **克隆共享 ProgramObservation** :436-445 / 行打印
    :446-465 / 切换行打印=Authority snapshot 字段 :537-554。

---

## §2 事件重建表（R57-B: #7 → #8 → #9 全字段）

**两套 epoch 词汇**（同日志并存, 勿混）: `av_epoch` = 执行事件计数
（SwitchExecuted, 7/8/9）; `tl_epoch` = ProgramEpoch(0) = 媒体时间线世代
（**Preserve 全程不变**——同一世代延续）。

| 事件 | program_anchor（=健康弧末值@采样） | source_anchor（=A 分支末值@采样） | offset | 边界帧 mapped | 结果 |
|---|---|---|---|---|---|
| #7 A→B seg7 | —（PRE7 未在摘录） | — | 100131607 | — | Preserved/DD |
| **#8 B→A seg8** | **74137405051**（=PRE8 末样本精确） | **74037405049** | **100000002**（=差值精确） | **=74137405051**（连续零间隙声明） | Preserved/DD（Authority 面）; **健康弧 NM 闩锁**（观察面） |
| #9 A→B seg9 | 83304071718（=PRE9 末样本精确） | 83203940110 | 100131608（=差值精确） | =83304071718 | Preserved/DD——**干净边界, NM 解除回 DD** |

**算术自证**: 74137405051−74037405049 = 100000002 == 打印 offset（到 ns）;
83304071718−83203940110 = 100131608 == #9 offset（到 ns）。
**锚构造对双源时钟差（skew）免疫**: 两锚同瞬采样, offset=差值——skew
在构造中相消, 连续性声明与源间偏移无关（见 §5）。

**切换行字段解码**（此前 R56 登记的修正）: `src_pts` = Authority
`video.last_source_pts` = **边界帧源值**（仅 on_mapped_buffer 更新, 冻结）;
`mapped` = `last_program_pts`@打印时 = 边界值 + settle 期推进
（#8: 74304071718−74137405051 = 166666667 ≈ 3 轮×50ms settle 采样推进
+ε）; `offset` = 当前段声明 offset。**打印的 mapped 不是锚**——R56
"锚点前跳 +26.67ms" 的读法就地修正: 真锚 = PRE8 末样本, 零间隙。

**段 8 期间行级事实**（三窗×双设备, pr 字段双行恒同——§4）:
pr_v pts 单调步进 76304071718→79304071718→83304071718（+3s/+4s 窗距,
媒体时钟正常）; 帧计数 2299→2389→2509 推进; adv=Some(true) 全程;
pr_a 全程 DD; av_delta 15150956（B 段）→**1515711（A 段, 随活跃源切换）**
→6817622（B 段）; 输入面原始 skew: PRE8 in_v B−A=25827996（B 前 25.8ms）
→PRE9 A−B=14066414（**方向反转**）; 源 offset 差 131605.5ns（A=100000002
/B=100131607-8, 逐源稳定, 18s 漂移 1ns）。

---

## §3 R57-A: 六行 NM 的精确产生链（哪个回调、谁先看到回退）

**行 `st=` 的唯一来源 = PipelineHealth 程序面健康弧**（不是 adapter
MappedContinuation, 不是 TimelineAuthority）:

```
selector src-pad BUFFER 探针（映射施加+PTS 原位改写, switch_graph.rs:190-207）
  ↓ 改写后的 mapped PTS
program appsink new_sample 回调（每缓冲）
  ├─ plain: h.observe_video_pts(pts)          :439 → pipeline.rs:323-333
  └─ declared: note_declared_boundary（每段首枚映射帧, :199-201）
       → h.observe_video_pts_declared(mapped)  :259-282 → pipeline.rs:354-365
  ↓ 四态闩锁（NM sticky, 段作用域）
switcher.observe() 读 program_video_pts_state    :1058-1067
  ↓
OBS snapshot_row 克隆共享 ProgramObservation      a204_obs.rs:436-445
  ↓
assemble_six_path_evidence pr_v 行                program_execution.rs:339-340
  ↓
evidence_row 打印 st=NonMonotonic                  a204_obs.rs:446-465
```

**TimelineAuthority 从未收到违例值**——喂入粒度是采样不是逐缓冲:
①a 切换前预喂一次（:596-608）; ⑤⑥⑦/⑨ 证据与 settle 皆 **50ms 轮询**
（:641-668/:669-705, 常量 :739）。单缓冲级瞬态回退在 50ms 采样下不可见
（下一轮采样读到的是已恢复的末值）。Authority 的基线 = ①a 预喂的
74137405051 = 锚 ⇒ 边界帧比较 `mapped < last` 为假 ⇒ Continuous ⇒
Preserved ⇒ ProgramEpoch(0) 不变 ⇒ 后续切换正常（若 on_program_pts 曾
收到回退值, :753-763 会 fail_closed 进 TransitionFailed 终态, 后续
declare_transition 将 InvalidPhase 拒收——与 #9~#30 全部正常执行矛盾,
反证 Authority 面无违例）。**这就是"同一 PTS 数学关系、两个状态机、
两个裁定"的接缝实锚: 观察面逐缓冲（健康弧）, 控制面 50ms 采样
（Authority, IMP-4 证据由 Runtime 喂入的设计结果）。**

---

## §4 R57-C: 六行 = 一次事实的六次投影, 不是六次回退

- 窗口行 = 采样时刻读闩锁**状态**: SPAN8/POST8/PRE9 三窗 × 双设备两行
  = 6 次读数, 全部读同一个程序健康弧（snapshot_row 克隆同一
  ProgramObservation——日志中双设备行 pr 字段逐 ns 恒同即证）。
- 闩锁性质: NM sticky 段作用域, 唯一解除 = 下一干净声明边界
  （note_declared_boundary :273-280; #9 SPAN 回 DD 即解除实证）。
- 行 pts 单调 + 帧计数推进 + adv=true ⇒ 段内不存在持续回退流。
- **结论: 底层回退事件 ≥1 次, 最简假设恰 1 次**（单缓冲级, 窗内自愈）。
  行数 ≠ 事件数——P1 现行 "NM 行==0" 谓词度量的是**闩锁投影数**。
  ⚠️ 若验收层未来要改判 "rollback event count", 证据面当前**没有事件
  计数字段**（健康弧只存状态）——那是 Domain/Observation 扩展, 非谓词
  改写可达。本轮不自行改（红线）。

---

## §5 机制分析: 候选形状排查（M1 唯一存活）

### 5.1 锚构造 skew 免疫（"跨源 skew 边界 rebase" 框架的消解）

offset = program_anchor(T) − source_anchor(T), 两锚同瞬读健康弧/分支
末值。设 B_src = A_src + s(t)（任意 skew）: program_anchor = B 映射末值
= B_src(T)+off_B; source_anchor = A_src(T)。边界帧 mapped = A_src(T′)+off
其中 off = 上式差。skew s(T) 作为常量在差中**相消**——连续性声明不含
源间偏移项。日志佐证: #8/#9 锚差精确等于 offset 到 ns, 边界帧 mapped
精确落在旧段末值上（零间隙）。**skew 不进入程序面回退; 它只表现为
逐源 offset 差（0.13ms）、输入面原始差（25.8ms 振荡）、av_delta 随
活跃源（15.15→1.52→6.82ms）。**

### 5.2 M2（未映射串行缓冲）被 R53 语义洗除——排除

install（:625-627, 状态置换 executed=false）到 ⑤ 生效之间穿越 selector
出口的缓冲以**原始 PTS** 透传（apply_declared_mapping :221-227 门全关）
≈ 程序基线 −100ms（offset 量级）。plain 路径会在该缓冲处置 NM——但
随后首枚映射缓冲（边界帧）触发 note_declared_boundary: 边界值(≈锚)
> 原始值(≈锚−100ms) ⇒ **clean ⇒ DD 释放把该 NM 洗掉**（:273-280）,
在任何窗口采样（SPAN 距切换 ≥2s）之前。⇒ M2 在 R53 语义下**不可能
产生观测行 NM**。（pre-R53 无解除机制, M2/M1 都会闩锁到运行尾——
R52 run2 "无复位闩锁" 16/60 行与此相容。）

### 5.3 M1（违例声明边界）唯一存活

```
T0  锚采样: program_anchor=74137405051(健康弧末值), source_anchor=74037405049(A分支末值)
T0→T1 [微秒级竞态窗] declare(:615-623)+install(:625-627)——旧段7映射仍生效至置换完成
    ★ 一枚 B(出发源)映射帧在此窗穿越 selector 出口(旧段7 offset 改写)
      → appsink plain: 基线推进至 74137405051+40ms(一枚视频帧@25fps)
T1  install 完成(段8状态, executed=false) → begin → flip(video :822 → audio :824) → executed=true(:843-845)
T2  ⑤ SEGMENT 事件(探针 :170-185, segment_observed=true)
T3  A 边界帧: src=74037405049(=source_anchor, A 未推进——采样时已在途的帧被翻转后转发)
    mapped = src+offset = 74137405051 = program_anchor(零间隙声明按设计成立)
    → note_declared_boundary: clean = !(74137405051 < 基线 74137405051+40ms) = false
    → observe_video_pts_declared: pts < last → NonMonotonic（"声明不豁免回退" pipeline.rs:353-356）
    → 段作用域 sticky; appsink plain 路径同帧同样置 NM（两写点同闩锁）
T4  段8续流全部单调(A 源内部单调, 输入面 VM 佐证), NM 保持(帧计数/PTS 正常推进)
T5  #9 干净边界: mapped=83304071718 ≥ 基线 → clean → DD 释放（rt_05 生命周期全链）
```

- **幅度 = 一枚出发源视频帧（≈40ms @25fps, br_v 1654→1729/3s 实测）**;
  Authority 面全程不可见（§3）。
- **概率量级**: 窗（declare+install+锁往返, 数十~数百 µs）/帧周期 40ms
  ≈ 0.1-0.6%/切换。与 R53 语义后 ~170 切换 1 例（R56 run1 #8）同量级;
  R52 run2（pre-R53, 同窗存在、plain 写路径、无解除）20 切换 1 例相容。
- **同索引 #8 复现**（R52 run2 dwell1s + R56 run1 dwell1000, 均 B→A）:
  同节拍运行 ⇒ #8 落在同一 elapsed 相位; 候选 = 节拍相关周期性系统
  瞬态拉长竞态窗。**n=2, 未证, 如实登记。**

---

## §6 R57-D: 跨源/边界概念在 Domain 的实况

SixPathEvidence（program_execution.rs:204-247）= 六路行（pts/state/
frames/advanced）+ `program_av_delta_ns`（:341-344, 活跃源程序面 |v−a|）。
跨源相关表达全部清单: ① 逐源 `SourceSegment.offset`/`AnchorPair`
（声明面, skew 相消构造, §5.1）; ② 输入面原始行 in_*; ③ av_delta。
**无 source_skew / boundary_rebase / declared_rebase / 排水（drain）
概念; 无回退事件计数面。** ⇒ D2/av_delta 不能裁决边界合法性（用户
判断确认）; "边界排水合法余量" 若要成立必须是**新 Domain contract**。

---

## §7 锐化后的裁决问题（交验收层）

原问题（"跨源 skew 下的边界 rebase 属测量事实还是 correctness 缺口"）
在证据下**变形**: skew 相消, #8 不是 skew 驱动。真正待裁的是:

> **声明切换边界处, 出发段（departing source）在 [锚采样 → 新段映射
> 生效] 微秒窗内继续推送的映射帧, 使新段边界帧实际低于程序末值一枚
> 帧——这属于:**
> **(i) 连续性违例**（现行代码立场: pipeline.rs:353 "映射的职责就是
> 连续; 禁以声明洗回退"——NM 是正确检出, Gate FAIL 定性为 correctness
> gap）, 还是
> **(ii) 声明切换的合法排水事实**（boundary drain semantics: 允许 ≤N
> 枚出发段残留帧, 属 declared-discontinuity 家族, P1/P2/P2c 语义需
> Domain contract 变更后重定义）?

- **(i) 路径**: 维持 FAIL = correctness gap; 修复归属 = 后续 adapter/
  runtime 边界原子性 change（候选方向, 均不在本轮实施: 锚采样与
  install 同临界区; install 时冻结旧段基准使窗内穿越帧不推高基线;
  或由 Domain 声明出发段余量）→ 回归 → Gate 复跑。不动谓词、不动
  Domain 语义。
- **(ii) 路径**: Domain contract 变更（drain 语义 + 事件计数证据面）,
  且与冻结纪律 "禁以声明洗回退" 正面冲突——比 (i) 重得多。
- **执行层分析（明确非裁决）**: 证据支持 (i)——有界（≤1 帧）的真实
  输出面回跳、四态健康弧正确检出、R53 生命周期按设计自愈（P6b 真机
  全链首证正是这台机器在工作）、Authority 采样粒度下设计性不可见。
  三个状态机各自按设计工作; 缺口在**切换协议的原子性**（锚快照与
  映射安装对旧段在途流不原子）, 不在任一状态机的语义本身。

---

## §8 本轮未确立的事实（诚实边界）

1. **违例帧本身未被直接观测**: 盒证据无逐缓冲轨迹; M1 为**唯一存活
   机制的推理结论**（M2 洗除论证 + 排除法 + 算术自洽）, 非直接测量。
   幅度 ≈40ms 是机制推断值。
2. "A 边界帧未推进（src==source_anchor）" 为最简推断（k=0 ∧ B 推进
   1 帧的 m>k 组合中, 与全部行证据相容的最简解）; 不排除 A 推进 <B
   推进的不对称变体（同族）。
3. 同索引 #8 复现原因未证（n=2, §5.3）。
4. R52 run2 事件为 pre-R53 语义历史, 其机制归属（同窗不同写路径）为
   推断——provenance 保留, 不作为 R57 结论输入。
5. 音频面 pr_a 未触发 = 独立竞态未命中（双平面独立窗）, 无更多信息。

## §9 红线（本轮全部维持）

零代码零判据零 Gate 零阈值; 三 blocking 不撤销不豁免; 首败留证;
absence≠false; 四态禁洗; DD 非异常禁 DD>0→FAIL; L4/Supervisor/
PipelineHealth/SwitchGraph/SixPathEvidence/dual_input 运行时零触碰
（只读核验）; A2-8-05 不进入（Gate 仍 FAIL/HOLD）; 不为跑绿调整任何
判据或删除 NM——R56 的 FAIL 与六行 NM 是本轮全部推理的证据地基。

---

## §10 R57 终裁复核（验收层 M1′ 修正裁决 × 实现层源码逐条复核, 2026-09-05）

授权: 验收层 R57 终裁全文（要求 "逐条复核并逐项确认; 不接受 R57 文档
推理结论作前提, 用 2972fb8 实际代码逐层反证"）。基线核对: 2972fb8 =
R56 Final Gate FAIL 提交（提交信息含 #8 B→A/pr_v/NM=6）✓; 本地
a8ac09a = 其上纯 docs 差异（5 文件 +398/−2, `git diff --stat` 核对）——
代码基线与裁决所引一致。

### 10.1 裁决实体部分——逐项复核全部成立, 已落实

| 裁决条目 | 复核（行号=当前源码实证） |
|---|---|
| A2-8-04 维持 FAIL/HOLD; 三 blocking 不撤销不豁免不修改 | ✓ 登记维持（本轮零 Gate 零判据） |
| §二 调用链 ①a→①b①c→②→③(pre-flip install)→④→⑤-⑧→⑨→⑩ | ✓ 逐行证实（program_execution.rs:596-608/:610-613/:615-623/:625-627/:629-640/:642-668/:669-705/:707-716; "③ pre-flip install" 注释 :624 与 :578 原文在） |
| §三 `apply_declared_mapping` 首条件 `!t.executed→None`; executed=true 仅 switch() 成功后置位; Segment 先于映射 | ✓（switch_graph.rs:221-223; :843-845 在双翻转 :822/:824 后; EVENT 探针 executed 门 :177-179 + :225-227） |
| §四 锚=程序健康弧末值+目标分支末值; +last_delta 外推已删 | ✓（:946-962 read_health+branch_obs; last_delta 已 allow(dead_code) :126-129） |
| §六 clean=!(mapped<last) 相对序决定 | ✓（:265-268） |
| §七 Authority 轮询 vs 健康弧逐缓冲两时间尺度 | ✓（on_program_pts 仅 ①a :596-608 + ⑨ settle 轮询 :673-686; ⑤-⑧ 环只喂 facts :645-647——"可能完全错过"有源码支持） |
| §八 ProgramObservation←HEALTH_ARCS; Authority 另链 | ✓ |
| §九 六行=一事实六窗口投影 | ✓ 维持 |
| §十 R53 段内闩锁/干净边界解除/plain sticky | ✓（:239-243; :273-280; pipeline.rs:323-333） |
| §十一 offset 唯一生产=②declare; adapter 直存 plan 段不重算 | ✓（program_execution.rs:614 注释; :895 segment=plan.video; :229 map_pts） |
| §十二 真正 gap=边界非原子（Authority 用 T0 位置, 健康弧可至 P+Δ） | ✓ 成立 |
| §十三 修复否决三项（语义洗回退/40ms 阈值/Gate 忽略行） | ✓ 登记维持 |
| §十四 修复=边界原子性临界区 | ✓ 接受为 R58 目标 |
| §十五 不能只加 Mutex | ✓ 且加强: 状态本就是 Arc<Mutex<Option<TimelineExecutionState>>>（:328/:712/:891）——缺口是协议级跨线程定序, 非数据访问互斥 |
| §十七 Mock observe()→tick_once() 掩盖并发 | ✓（switch_mock.rs:396）且加强: mock 运行时 program_*_pts_state **硬编码 ValidMonotonic**（:406-415）——结构上不可能产生 NM, 更不可能复现竞态 |
| §十八 R57-A/B/C/D 分级 | ✓ 全维持 |
| §二十 R58 边界原子性实现轮 11 步序 | ✓ 接受（步骤 4 注入缝按 10.4 修正） |

### 10.2 机制叙述部分——M1′ 替换案被双重反驳, M1 维持

裁决核心修正主张（"[锚采样→install] 窗内不存在声明映射帧, 因
executed==false; 故 M1 不可能成立, 应改 M1′=raw/legacy 帧推进基准"）
经源码逐层复核**不成立**——未通过裁决自身为 R57-C 设定的标准
（"必须先从真实调用链证明"）:

1. **窗口归属读反**: `!t.executed` 门控的是**当前已安装**的
   TimelineExecutionState。#8 的 [①c 锚采样→③ install] 窗内, 槽中是
   **#7 的状态**（#7 的 install 写入, #7 的 switch 置 executed=true）。
   全仓唯一槽写点 = install 的替换式 `*slot=Some(...)`（:891）——
   **运行图生命周期内无任何清槽**（:169/:189 探针克隆、:806/:843
   switch、:980 facts、:1080 observe 全为读/改既有 Some, grep 核对）。
   故该窗内出发源帧**仍被旧段 7 映射**（executed ✓ / segment_observed ✓
   / pts ✓ → map_pts 改写）——这正是段间稳态映射持续生效的同一机制
   （若旧状态被清, 段间所有帧都将是 raw——与全部字段据矛盾）。
   executed=false 的无映射窗是 **[③ install→④ 翻转]**, 不是 [①c→③]。
2. **符号封闭（现场数据）**: raw 路线即便经正确的 [③→④] 窗发生:
   offset#7 = +100131607（§2 事件重建表, 正值）⇒ B raw 帧 = 映射值
   −100.13ms ≈ 程序基线−100ms → plain 路径瞬时闩 NM → **#8 自身
   边界 mapped=P 远高于该基线 → clean → DD 覆写解除**（:265-268/
   :273-280/pipeline.rs:354-365）——与观测签名（NM 跨 SPAN8/POST8/
   PRE9、至 #9 干净边界才解除、#8 边界本身非 clean）**直接矛盾**。
   raw 路线对 #8 既产生不了持久 NM, 也解释不了非 clean 边界。

佐证: 裁决自身的算术示例（anchor=P → 旧源帧 P+40ms → health.last=
P+40ms → A 映射=P<P+40ms）是**映射帧签名**（raw 帧位移=−offset7≈
−100ms, 非 +40ms）; §十二 的缺口陈述（"健康弧可能已推进到 P+Δ"）
亦与 M1/W1 同构。

**结论**: M1（[①c→③] 旧段映射帧穿越, 基线+1 帧 40ms, A 边界=锚成
违例边界）维持为与源码结构和 #8 现场数据**双相容的唯一机制**; M1′
作为替换案登记为 **PROPOSED-REFUTED**（窗口误置 + 符号封闭 + 签名
矛盾）。

### 10.3 接受的锐化（裁决 §六/§十七 的有效成分）

- **§六 相对序修正成立为一般化**: "声明边界必洗此前 raw"非代码语义,
  代码只按 mapped vs last 相对序判定。R57 §5.2 的 M2 洗除论证由此
  精确化为**条件排除**（条件: raw 落边界下方 ⇔ offset_prev>0）。该
  条件在 #8 现场成立（100ms 余量）, 故 §5.2 对 #8 的排除结论不变;
  作为一般性陈述需附加符号条件——就地锐化登记。
- **§十七 Mock 缺口确认并加强**: mock 的 program pts_state 为硬编码
  常量, 连 NM 都无法表达——mock 证明 Domain/Adapter 状态机正确性,
  不证明切换边界并发原子性; R58 确定性测试须补流线程交错注入能力。

### 10.4 R58 输入（修复不变量与测试缝, 按复核修正）

- **修复不变量**: 自锚快照时刻起, 至新段首枚映射缓冲被程序健康弧
  **观测**止, 出发段不得再向程序健康弧贡献高于锚的映射观测。（以
  "健康弧观测序"而非"探针穿越序"表述——selector 出口→appsink 有
  管线 transit, 健康弧观测晚于探针改写。）
- **候选实现族**（R58 步骤 3 设计裁决, 本轮不实施）: install 同临界
  区重采样程序锚（漂移→fail-closed 重声明）; install 时冻结旧段映射
  贡献上限; 程序锚读取移入 slot 锁并与 BUFFER 探针互斥。
- **确定性测试注入缝（步骤 4 修正）**: 主缝 = [①c→③] 旧段映射帧
  穿越（M1 复现: 锚 P → 注入映射帧 P+40ms → 翻转 → A 边界 P<基线
  → NM）; 次缝 = [③→④] raw 透传（仅 offset_prev<0 时可见——#8 不
  适用, 作 10.3 符号条件的一般化回归面）。mock 侧需增强为可注入
  交错模型。

### 10.5 红线（本轮维持）

零代码零判据零 Gate 零阈值; 三 blocking 维持; 首败留证; 不为跑绿
调整或删除 NM; **违例帧未被直接观测的诚实边界维持**（M1 仍为排除法
+算术自洽推理, M1′ 被反驳加强其排他性但不构成直接测量, 不升
FieldProven）; A2-8-05 不进入。

## 11 R58-Design 终裁复核 + R58 步骤 4 执行（R58 unit 1, 测试先行零生产代码）

验收层接受 §10 复核（**M1′ 正式撤销, M1 恢复唯一双相容机制**）并下达
R58-Design 终裁 12 条。本轮逐条复核 + 执行步骤 4: 生产代码零字节改动,
仅 switch_graph.rs 测试模块追加。

### 11.1 逐条复核（对照源码基线）

- **§1 根因（M1 保留）— 确认, 源码全相容**: 槽唯一写点=install 整槽
  替换（switch_graph.rs:891 `*slot.lock().unwrap() = Some(...)`）, 运行期
  无清槽点（`take()`/`timeline = None` 全文零命中）; executed=true 唯一
  落点 :844; [①c→③] 窗内槽=#7（executed=true ∧ segment_observed=true）
  → 旧段映射帧继续施加（与切换间续流同一机制）; offset#7=+100131607>0
  ⇒ 窜帧上推基线一帧（40ms@25fps）; 锚=程序弧 last PTS
  （sample_switch_anchors :946-949, read_health 同源）。
- **§2 三案取舍 — 确认**: 现有锁 `Arc<Mutex<Option<TimelineExecutionState>>>`
  （:390/:163）仅护槽自身, 不能阻止 selector→appsink 流面推进;
  HEALTH_ARCS=观测事实源（pipeline_events.rs:16-29）——install 冻结/
  覆盖基线=污染 observation, 否决正确; 重采样兜底仍有 sample→再竞态→
  install 窗——归第二层 fail-closed, 非主修复。
- **§3 Cutover Fence 主方案 — 方向确认 + 三条实现条件登记**:
  (a) **fence-confirmed 语义必须覆盖在途缓冲**: 真实拓扑=selector→
  queue→appsink（:630-634 链接; program-video-queue :508 /
  program-audio-queue :552）——selector src 挂 fence 后 queue 在途帧仍会
  到达 appsink plain 写弧（:439）, M1 窗未闭; confirmed 须含 queue 排放
  /观测面静止证据（appsink 侧）, 非 fence 挂点静止;
  (b) **pre-flip 拦截的旧段缓冲必须 DROP 不得 flush**——放行即把旧段帧
  投入新段映射域（时间轴错位, 依原始 PTS 可低于推进后基线→NM, 亦挤占
  新段帧位）;
  (c) **同 pad 探针序**: mapping 探针建图时挂（:630-631）, fence 运行时
  挂——同 pad 按添加序触发, fence 晚于 mapping; mapping 对续流帧仅
  状态面（不写弧）, 弧写点在 appsink——§11 不变量不受此序影响, 实现
  时须显式声明该前提。
- **§4 位置=selector 出口 — 链路确认**: attach_program_video_sink
  :422-446（new_sample→pull_sample→buf.pts→`h.observe_video_pts(pts)`
  :439）; BUFFER/EVENT 探针挂 selector `static_pad("src")`（:165-167/
  :170/:190）。"控制进入观测的帧, 不改观测"立场成立, 前提=§3a 确认
  语义。
- **§5 双面 fence — 确认**: video/audio selector 独立结构字段
  （:1332-1335/:2432-2433）, 每平面独立探针/弧（:630-631）; 成对契约
  （:879-888 V/A 一致性 fail-closed 校验）。单面冻结制造 V 冻/A 推进
  新态。
- **§6 fence 只拦 BUFFER — 确认**: EVENT 探针=EVENT_DOWNSTREAM ∧
  Segment ∧ executed 门（:170-185）置 segment_observed; 映射门控
  executed ∧ segment_observed（:221-227）。Segment EVENT→首枚映射
  BUFFER 微观序必须保留——fence 语义与现结构一致。
- **§7 确定性测试 — 本轮落实 + 一处顺序校正**: 终裁草图 T1/T2（窜帧）
  先于 T3（锚采样）不可复现——锚=弧 last（:946-949）, 窜帧先行则锚=
  P+40ms。按 §1/§11 机制序实现（锚→窜帧→install→首枚）, 其余照草图。
- **§8 fence 修复后同序测试 — 未实现（属步骤 5+）**, 以 §3a/§3b 为其
  实现输入。
- **§9 Mock 增强 — 未实现（属步骤 6 前置）**: 缺口复核确认
  （switch_mock.rs:396 observe→tick_once 绑定; :406-415 running ⇒ 硬编码
  ValidMonotonic——不能表达 NM）; 步骤 4 用真实组件缝, 不被阻塞。
- **§10 方案族排序 — 登记为冻结设计裁定**: Fence ✅主 / 重采样兜底
  ✅二线 / 冻结基线❌ / 放宽 declared❌ / 阈值❌ / 单纯 Mutex❌ / 改
  Authority❌ / 判 #8 合法排水❌——与既有红线（禁以声明洗回退等）
  完全一致。
- **§11 实现层不变量 — 原文登记（不触碰 Domain predicate）**:
  "Once the program anchor is sampled for a transition, no buffer
  belonging to the pre-transition execution state may newly advance the
  Program observation baseline before the transition's selector cutover
  becomes effective."（一旦切换锚被采样, 切换前执行态不得再有新 BUFFER
  推进 Program observation baseline, 直到 selector cutover 生效。）
- **§12 远端基线 — 确认**: 远端=2972fb8（`git ls-remote` 实测）, 本地
  a8ac09a→f9dc935 未推送; 本轮不推送; 盒同步走既有 tar 通道
  （BMD-LOCAL-BUILD.md:148-155）, 无需 GitHub 推送。

### 11.2 R58 步骤 4 执行证据

- **改动面**: switch_graph.rs 测试模块追加两测 + M1 装置辅助
  （m1_segment/m1_rig/m1_install_8）——生产代码零改动。
- **测试一（M1 确定性复现, 红测）**
  `switch_graph_m1_straggler_race_reproduces_program_nonmonotonic`:
  T0 #7 续流 S7→mapped=P（首枚映射, plain 写弧）→ T1 锚采样=P
  （read_health 实路径）→ T2 竞态窗窜帧 S7+40ms→mapped=P+40ms
  （Continuing——#7 行不 Violated, 违例只在程序弧=R56 pr_v 行签名）→
  plain 写弧基线=P+40ms → T3 install #8（真实整槽替换 :891; 断言
  offset#8=+100000002>0）→ ④⑤落点置位（executed=:844 落点/
  segment_observed=EVENT 探针 :177-179 落点）→ T6 首枚映射 S8→P +
  note_declared_boundary（违例边界→observe_declared NM——声明不豁免）
  + plain 写弧 → **断言 NonMonotonic** → T7 #9 干净边界 P+40ms →
  **断言 DiscontinuityDeclared**（闩锁解除=R56 #8→#9 现场签名闭环;
  干净边界重开, 非声明洗违例）。数值全取 R56 #8 实测锚
  （74137405051×74037405049; offset#7=+100131607）。
- **测试二（差分对照）**
  `switch_graph_m1_control_no_straggler_boundary_stays_clean`: 同序列
  去窜帧 → 干净边界 → 断言 DiscontinuityDeclared——证明 NM 由窜帧
  因果致, 边界/生命周期机制自身不产违例。
- **盒证据（SSH lytv@10.30.15.10, tar 通道, cargo 全经盒）**: 编译
  `cargo check --features bmd,gstreamer --tests` 通过; default 227/227 ✓;
  simulation 227/227 ✓; gst m1_ 过滤 2/2 ✓; gst 全量 261=259+2——
  run-A=260 过 1 失败（失败名未捕获——result 行过滤之误）, 零改动
  立即重跑 run-B=**261/261 全绿** ⇒ 瞬态盒 flake（与 R51 rt_01 flaky
  既有记录同型; 未改未删未跳过任何测试）; clippy default +
  bmd,gstreamer `--all-targets -D warnings` 双绿。
- **纪律**: 谓词/Gate/阈值零改动; 三 blocking 维持 Failed; 首败留证
  （#8 证据盒不动）; 步骤 5-11 待执行; A2-8-05 不进入。

## 12 R58 独立远端裁决复核（验收层基于 GitHub 真实状态, docs-only 零代码）

验收层独立复核 GitHub 远端后下达 13 节裁决。本轮逐条复核+登记。
生产修复未进远端 ⇒ **A2-8 = HOLD 维持**——定性（裁决 §十三原话）:
设计方向无错, R56 已证实现有实现存在真实 Program-output cutover
race, R58 目前完成至「根因确定+测试先行」层级。

### 12.1 远端事实（本轮独立实测, 与裁决一致）

- `git ls-remote origin comet/a2-8-dual-input-switch` = **2972fb80b8a3…**
  （R56 FAIL 提交=远端最新——FAIL 结论+三 blocking 在远端证据完整）。
- `git branch -r --contains d1f1e58` = **空**——d1f1e58/f9dc935/a8ac09a
  均不在任何远端分支; 远端 tip=2972fb8 为 d1f1e58 祖先 ⇒ 数学上不可
  能含 R58 测试; GitHub 无 `switch_graph_m1_*`。
- **口径纪律（本轮冻结为常设）**: 本地事实与远端事实分开陈述; 本地
  提交永不表述为「远端已落地」（R58 unit 1 自身记录已守此界——提交
  信息尾 "unpushed"、报告远端行「未推送」; 现升格为常设纪律）; 推送
  仍仅按明确指示执行。

### 12.2 M1=源码闭环证实（裁决 §二-§五与本仓复核逐点相合）

- appsink→HEALTH_ARCS 链 :422-446/:439（§二①）; install 整槽替换
  :889-891+无中间清空步骤（§二②）; `!t.executed` 门 :221-223（§二②）;
  锚=弧 last :946-949（§三）; switch_program ①-⑩ 序 :596-716——anchor
  快照与 switch 之间窗口客观存在（§三）; 回退 fail-closed 链
  program_timeline.rs:746/:755-756/:763/:776——无声明豁免（§四）;
  TIMELINE_POLL_INTERVAL=50ms :739（§五——两时间尺度 ⇒
  ProgramObservation=NonMonotonic 与 Authority=Preserved 可并存:
  观察粒度不同, 非模块谁错）。**接受裁决定性: M1 非「理论 race」,
  是控制平面 T0 快照 × 数据平面 T0→T1 在途帧的结构性缺口。**

### 12.3 fence 不变量升格冻结（裁决 §七/§八/§九）

R58 unit 1 已作为「实现条件」登记（§11.1 §3a/§3b）的两条, 本轮按
裁决**升格为实现不变量并冻结**（从设计注记变为协议强制）:
- **INV-F1（queue-in-flight confirmed）**: 「selector src 已 fence」≠
  「Program output 静止」——confirmed 条件必须纳入 selector→queue→
  appsink 在途排放证据（拓扑事实 :508/:552/:630-634）; 否则 M1 窗
  未闭。
- **INV-F2（旧世代拦截帧不可复放）**: cutover 前被阻塞且属旧世代的
  buffer 永不允许重新进入 Program output——fence=「旧世代排空+cutover
  barrier+新世代重新开放」三段模型, 非 pause/resume; 否则 race 只从
  fence 前移到 fence 后。
- **INV-F3（双面握手形, 本轮新冻结）**: Video Fence+Audio Fence →
  **Both Confirmed** → anchor/declare/install/switch → **Both
  Release**; 代码依据: switch() video 先 :822 / audio 后 :824 /
  audio 败→video 回滚 :828 / 回滚败→degraded :830-837——单面 fence
  必引入 AV skew race。
- 维持（§十/§十一）: BUFFER-only（EVENT 承担 segment_observed 前置
  :170-185——阻塞 EVENT 将把 flip→Segment→首枚映射 执行协议改写为
  flip→fence→segment, Authority 永等不到闭合）; Mock 增强必须表达
  「控制事件×数据事件交错」（步骤 6 前置, 非测试数量）。

### 12.4 步骤 5 执行边界（裁决 §十三, 登记=下轮规格）

selector-output Cutover Fence 树: V/A 双 BUFFER fence → confirmation
（含 queue in-flight, INV-F1）→ anchor snapshot → TimelineAuthority
declare → install 新 TimelineExecutionState → dual selector switch →
旧世代拦截帧不可复放（INV-F2）→ mark executed → release 新世代流;
二线=stale-baseline detection → fail-closed / secondary resample
（不作主一致性机制——race 未消灭只是降概率）。

### 12.5 红线（维持）

零代码零判据零 Gate; 三 blocking 维持; 首败留证; 本地/远端口径分述;
推送仅按明确指示; A2-8-04 不得宣布恢复 PASS; A2-8-05 不进入。

## 13 R58 步骤 5 执行：Cutover Fence 生产实现（代码轮, 本地未推送）

验收层终裁「下一工作单元就是 Step 5·INV-F1/F2/F3 必须直接编码进
Adapter execution contract」——按 §12.4 冻结边界树落地。

### 13.1 实现面（四文件, Domain 零触碰）

- **switch graph 适配器（主修）**: `FenceState{Open,Armed}` +
  `PlaneFence{state,discarded}` + `FencePair{video,audio}`（单字段
  承载=V+A 成对的结构性表达）+ `fence_should_discard` 纯门函数
  （单语义、双应用点）; SwitchGraph 增 `fences` 字段（build 期创建,
  探针/消费门/契约共享）; **两层门**——selector src BUFFER 探针门
  （drop-first: 先于 `apply_declared_mapping`, 丢弃帧不消耗
  first_mapped/不改写 PTS）+ appsink 消费门（V/A 对称, 不计帧数不写
  弧）; `arm_cutover_fence`（V+A 同装, INV-F3 无单面窗口; 未启动
  graph fail-closed）/`release_cutover_fence`（双面 Open+丢弃计数
  交回）; **Open=legacy 逐字节保持**（门仅在 Armed 时改变行为）。
- **switch 执行契约（端口扩展, 无默认实现）**: 新增
  `arm_cutover_fence`/`release_cutover_fence` 两个 trait 方法——
  编译期强制全部 4 个实现方表态: 真实 GStreamer（双面 fence）/
  Mock（staging 旗标+编排序 debug_assert）/两个测试包装器
  （unreachable+转发）。barrier=执行屏障非权威（INV-F3: 永不自宣布
  切换完成）。
- **program execution 编排**: `CutoverFenceGuard`（arm 于 ⓪——
  ①a 基准观测之前; **Drop 兜底**: ①-④ 任意错误路径必解除 barrier
  恢复流面, 不吞切换错误）; 成功路径在 `switch()` executed 落点
  **之后**显式 defuse=Release（INV-F3 落点序; 丢弃计数为证据面）。
  Runtime 只发起 barrier, 不实现 fence——方向与裁决一致。
- **Mock**: staging 最小实现（状态记录面; 真实交错模型=步骤 6——
  OldStraggler/FenceConfirmed/OldBufferDropped 语义接入后驱动
  无 Fence→FAIL/有 Fence→PASS 双模式）。
- **INV-F1 语义落定（登记）**: confirmed=**构造性覆盖**——appsink
  消费门保证 Armed 期间到达的在途帧（含 queue 已有帧）一律
  consumed-as-cutover-discard, 无时间等待、确定性非启发式;
  INV-F2: Drop 无 flush/复放路径, 丢弃计数如实交回。

### 13.2 测试与盒证据

- 契约测 `switch_graph_fence_contract_arm_release_both_planes`:
  初始 Open（legacy）→ arm=V+A 双 Armed（无单面窗口）→ release=
  双 Open+计数如实 → 未知 graph 双向 fail-closed。
- fence 闭合测 `switch_graph_fence_armed_closes_m1_race_boundary_
  stays_clean`（R58-Design §8 同序）: T0 #7 续流 P（Open=放行）→
  T1 arm → T2 锚=P → T3 竞态窗窜帧被探针门处置（**基线未被推进
  ——对照 m1 红测在案: 无 fence 时已被推进 P+40ms**）→ T4 install#8
  → T6 executed 落点 Release（计数=1 如实）→ T7 新世代首枚映射 P
  → **干净声明边界 DD**（对照无 fence→NonMonotonic）。
- 盒（tar 通道）: 首跑 2 处编译错误（测试闭包 E0597 借期+
  clippy too_many_arguments 8/7）→ FencePair 重构+闭包拷贝修复 →
  复跑**全绿**: default 227/227·sim 227/227·**gst 263=259+2(m1)+
  2(fence)**·clippy×2 `--all-targets -D warnings` 过。

### 13.3 边界与红线（维持）

Domain（program timeline/Authority）/谓词/Gate/阈值零字节; 三
blocking 维持 Failed; 首败留证; 真机 #8 场景复现（步骤 7·NM 消失
+ #9 生命周期仍立）与 Mock 交错模型（步骤 6）待执行; Gate 复跑仅
按冻结谓词于步骤 11 执行; A2-8-04 仍 FAIL/HOLD·A2-8-05 不进入;
本地提交未推送（远端基线=dd263be）。

## §14 R58 步骤 5 独立终裁复核 + 步骤 5.1 落地（Downstream Segment Cutover Confirmation + 真正原子 FencePair）

### 14.1 终裁 14 节逐项复核（验收层对 0abde4d 反向审查——逐项确认）

1. **当前真实链路（selector→queue→appsink→HEALTH_ARCS）**——属实。
   链接序 `video_selector.link(&v_queue)`→`v_queue.link(&v_sink_el)`
   （Bridged 直链/Simulation 经 capsfilter）; appsink 回调
   `pull_sample()`→fence 消费门→`observe_video_pts()`（:501-528）——
   消费门确实位于 queue 之后, 有能力处置 queue 既有帧。
2. **Release 无 queue drain 等待**——属实。`fence.defuse()` 紧跟
   `switch()` 成功返回（程序执行编排④落点后立即）; `release_cutover_
   fence` 实际动作=双面 `state=Open`+返回累计丢弃计数, 无任何排空确认。
3. **T0→T3 旧帧穿透路径**——属实成立: T0 旧 #7 buffer 已过 selector 入
   queue → T1 arm → T2 switch+defuse（Fence=Open）→ T3 queue 残留 #7
   → 消费门 Open → 不 discard → `HEALTH_ARCS.observe_video_pts()`——
   旧世代帧重新进入 Program baseline, 正是 INV-F2 冻结要消灭的路径。
4. **INV-F1 未闭合**——属实。0abde4d 把 F1 从"等待 queue 排空"改成了
   "queue 后面的消费门在 Armed 时丢弃"——局部补救有效域仅 Armed 窗口;
   "任何 Armed 期到达 appsink 的帧都 discard" ≠ "任何 queue in-flight
   旧世代帧都 discard"（Release 后浮出者仍 PASS）。**我方注册撤回**:
   0abde4d 提交信息与 §13 的"INV-F1 构造性覆盖（按构造, 无时间等待）"
   声明错误——消费门不构成 barrier, 排空未被证明。
5. **FencePair 结构性打包非原子**——属实。`FencePair{video: Arc<Mutex<
   PlaneFence>>, audio: Arc<Mutex<PlaneFence>>}` 两把独立 Mutex; arm=
   先 lock video 置 Armed 再 lock audio——存在 Video=Armed/Audio=Open
   微观窗口; release 同序非原子。对 INV-F3（Both Confirmed/Both
   Release）严格并发语义不足。
6. **两层门无世代标记**——属实。selector 门与消费门均只读"当前是否
   Armed", 无 buffer 世代身份; 这是 Fence 未升级为 generation-aware
   barrier 的根因。修复路径=Segment 事件序号作为世代边界标记（非
   per-buffer 标签——队列序承担世代划分, 见 14.2）。
7. **executed 顺序正确**——确认。`g.active/g.av_epoch/t.executed` 全部
   在 adapter `switch()` 内部落点(:956-962), fence 从不触碰执行态;
   FenceGuard 只 arm/release——"Fence=execution barrier 非权威"在本轮
   维持并继续成立, 无返工。
8. **program_timeline.rs 不需要动**——维持。0abde4d 全部 fence 代码在
   switch graph/程序执行编排/契约端口/mock 四处, Domain Authority
   零触碰; 步骤 5.1 同律（本轮亦零触碰）。
9. **Release 语义必须升级**——落实（14.2/14.3）: "switch success →
   mark execution state → wait/observe downstream cutover evidence →
   Both planes confirmed drained → release"; 等待=真实媒体事件证据
   （下游 Segment 到达+Condvar 通知）, 非任何形式的时钟猜测。
10. **Queue 后 Generation Barrier**——落实: appsink sink pad 新增
    EVENT_DOWNSTREAM **纯观测确认探针**（不阻塞 EVENT——PadProbeReturn::
    Ok 恒返）; 流程=①Arm V+A（原子）→②③④⑤⑥编排序不变→⑦selector
    推新世代 Segment→⑧Segment 穿过 queue→⑨下游探针确认→此刻证明该
    Segment 入队前排队的全部 Arm 前旧世代缓冲已被消费门处置（GStreamer
    queue 保序）→⑩Both confirmed→⑪同一临界区原子 Release→⑫新世代
    首帧才进入观测。
11. **Segment 为确认点**——落实并已获既有绿测试实证: rt_02 真实
    GStreamer 全链（真实 input-selector/真实流线程）多轮全绿, 而
    `apply_declared_mapping` 门在 `segment_observed` 上——即 pad 翻转后
    Segment 事件必经 selector src 探针（既有 segment_observed 事实链=
    input-selector 翻转必推 Segment 的在案证明）。世代身份=**GstEvent
    seqnum**: Armed 期 selector src EVENT 探针捕获首个 Segment 序号
    （arm→switch 之间无 pad 翻转、无其它 Segment 源——结构性唯一）,
    下游探针按序号匹配确认; 陈旧/异世代 Segment 序号不匹配即忽略
    （不可误确认——T-F3 前置）。
12. **真正原子 FencePairState**——落实: `Arc<Mutex<FencePairState>>`
    单锁承载 `{video, audio, video_ready, audio_ready,
    video_segment_seq, audio_segment_seq, generation}`（终裁 §12 形态
    +世代序号捕获字段）; arm/confirm/Release 全在单临界区内双面同变;
    Both-confirmed 等待经 Condvar（确认探针 notify → 编排线程阻塞等
    待——事件驱动非轮询）; Release 与 Both-confirmed 观测**同一临界
    区**完成（确认与释放零窗口）。
13. **裁决表（Step 5 = IMPLEMENTATION PARTIAL / HOLD）**——接受。A2-8-04
    维持 🔴 FAIL/HOLD（三 blocking Failed 未触碰）; 步骤 6 ⏸️ 暂停
    （Mock 交错模型不进入）; 步骤 7 ⏸️ 先完成 Fence 修正后的确定性验证。
14. **"最后一公里"**——接受。0abde4d=Fence 第一版生产实现（方向正确/
    层级正确/两层门方向正确/executed 权威正确, 但 INV-F1/F2 未形成
    可证明闭环）; 本轮=步骤 5.1 + T-F1/T-F2/T-F3。

### 14.2 步骤 5.1 设计与实现（四文件, Domain 零触碰）

- **契约端口（contracts switch 执行端口）**: `release_cutover_fence`
  改签为**确认式 Release** `(graph, timeout) -> CutoverDrainEvidence`
  （generation+双平面 `{segment_confirmed, discarded}`）; 新增
  `force_release_cutover_fence`（**仅失败路径**兜底强释——无确认前提
  恢复流面防饿死; 返回丢弃计数）。两法均无默认实现——4 实现方编译期
  表态纪律延续（真实/Mock/两测试包装器同步更新）。arm 契约文档改写
  （原子双面; 撤回"构造性覆盖"表述）。
- **switch graph**: `FencePairState`（单锁, §14.1-12 字段）+
  `FencePair{Arc<Mutex<FencePairState>>, Arc<Condvar>}`（Clone 供回调
  捕获）; 方法族=`arm()`（原子双面同装同清, 返回世代号）/
  `discard_if_armed(plane)`（门纯决策核 `fence_should_discard` 保持,
  探针层+消费层共用）/`capture_segment(plane, seq)`（Armed 期首个
  Segment=本世代——结构性唯一）/`confirm_segment(plane, seq)`（序号
  匹配→ready→notify; 不匹配忽略）/`release_after_drain(timeout)`
  （阻塞等 Both-confirmed, **同一临界区**原子 Open 双面, Err=超时且
  保持 Armed）/`force_open()`。探针面: selector src EVENT 探针加世代
  捕获（Armed 期首个 Segment）; BUFFER 门改走 `discard_if_armed`;
  appsink 消费门 V/A 对称同改; **新增 `attach_drain_confirm_probe`**
  （appsink sink pad, EVENT_DOWNSTREAM 纯观测, 按序号确认）。
- **程序执行编排**: `CutoverFenceGuard.defuse` 升级为
  `confirm_and_release(timeout)`——switch Ok→④ Domain executed 标记
  （终裁 §9 顺序: mark execution state→等下游 cutover 证据→Release）
  →阻塞等待 Both-confirmed→原子 Release。超时 → Err 且 fence 保持
  Armed → 守卫 Drop 改走 `force_release`（强释=失败处置, 非确认放行;
  T-F3 语义只在成功路径强制）。`CUTOVER_DRAIN_CONFIRM_TIMEOUT=5s`
  （异常界非时间假设——正常排空毫秒级, 证据=Segment 到达非时钟流逝;
  与 TIMELINE_* 超时同族）。graphs 锁在阻塞等待前释放（并发 observe
  不受阻）。
- **Mock**: release=auto-confirm 建模（Mock 无真实 queue/流面——排空
  确认按"立即可用"; **披露**: 协议强制[Both-confirmed 前置/世代序号
  匹配/超时 fail-closed]由真适配器 switch_graph 确定性测试 T-F1/F2/F3
  证明; Mock 交错协议表达=步骤 6 范围, 本轮终裁暂停）+force_release
  幂等; 编排序 debug_assert 保持。
- **残留登记（超时路径语义）**: drain 确认超时 → Err 上抛且 Domain
  停留 SwitchExecuted 相（后续 declare InvalidPhase fail-closed——
  恢复归会话级故障面, 与 TransitionFailed 同层; 不新造 Transition
  Failure 变体=program timeline 零触碰纪律优先）。编排线程在
  confirm 等待期间持 inner 锁（正常 ms 级; 异常界 5s——与既有
  settle/evidence 轮询同类阻塞面）。

### 14.3 确定性测试（T-F1/T-F2/T-F3 + 既有 fence 测试升级）

- **T-F1** `switch_graph_fence_t_f1_queue_stragglers_dropped_until_
  both_confirmed_release`: queue 在途旧帧→arm→消费门处置（弧不动=
  基线不推进）→install#8（executed 落点）→世代捕获+下游确认（陈旧/
  异世代序号不误确认; 单面确认 Release 拒绝）→audio 确认→Both→
  确认式 Release（世代/confirmed/丢弃计数如实）→新世代首帧 S8→P
  →**干净声明边界 DD**——M1 在协议级闭合（对照无 fence 红测 NM 在案）。
- **T-F2** `switch_graph_fence_t_f2_straggler_across_release_boundary_
  never_reaches_arcs`: 旧帧在 queue 中跨过 executed 落点乃至单面确认
  点——Both-Release 前消费门持续 Armed → 不进入 HEALTH_ARCS; 单面
  确认后浮出仍处置; Release Err 保持 Armed; Both 后门开（按队列序迟到
  帧必为新世代——排空锚在案）。**0abde4d 的"Release 立即放行"缺口在
  此闭合为协议断言**。
- **T-F3** `switch_graph_fence_t_f3_release_requires_both_planes_
  downstream_confirmation`: 零确认→Release Err; 陈旧序号（未捕获）
  忽略; 单面（video）确认→仍 Err（audio 未确认, ready 独立在案）;
  audio 捕获后异序号确认→忽略; 正确序号确认→Both→原子双面 Open。
  **世代身份=Seqnum 全程匹配验证**。
- 既有 fence 契约测升级: arm 原子双面（单锁断言）+ 未确认 Release
  Err 保持 Armed + 确认后 Release Ok（generation=1 如实）+ 兜底强释
  幂等 + 未知 graph 三向 fail-closed; fence 闭合 M1 测升级: 门裁决走
  生产决策点 `discard_if_armed`（探针层+消费层同源）, T6 改为世代
  捕获+双面确认后确认式 Release（计数 v=1/a=1 如实）。
- 测试世代序号= `gstreamer::Seqnum::next()`（进程内原子递增, 取值
  互异即确定性; `Seqnum(pub(crate) NonZeroU32)` 无任意值构造——类型
  本尊入状态字段, 消除 u32 转换面）。

### 14.4 盒证据（tar 通道, 逐轮如实登记）

- **run1**: default 227 ✓/sim 227 ✓/mock 393 ✓/clippy default ✓/clippy
  mock ✓; gst 配置 build.rs DeckLink SDK 门 panic（本轮误用 `bmd`
  特征且未导出 `DECKLINK_SDK_INCLUDE`——历史 gst 计数配置澄清）; fmt
  --check FAIL。
- **fmt 尘修复**: `cargo fmt` 于盒上 apply 后拉回——含本轮新代码格式
  + **R53/R56 时代既有 fmt 尘**（m1 复现测长行, R58 步骤 5 矩阵未跑
  fmt 所遗留）——如实登记: 非 5.1 引入, 顺带清偿。
- **run2**: gst 配置 2×E0308——`ev.seqnum()` 在 gstreamer-rs 0.23.7
  返回 `Seqnum` 新类型（`pub(crate) NonZeroU32`, 仅 `Seqnum::next()`
  构造）非 u32 → 世代序号改用类型本尊 `Option<gstreamer::Seqnum>`
  入状态字段（消除 u32 转换面; 测试以 `Seqnum::next()` 原子递进取
  互异值=确定性）。default/sim/mock 配置不受影响（switch graph 模块
  gstreamer-backend 门控, 该三配置不编译此文件）。
- **run3**: gst test **266/266 全绿**（=259+2(m1)+2(fence)+3(T-F)——
  rt_02 真实 GStreamer 全链含排空确认等待通过=input-selector 翻转必
  推 Segment 且 Segment 穿越 queue 到达 appsink sink pad 探针的实机
  证明）; clippy gst 2×needless_borrow（`&fences` 双重引用）→
  调用点去 `&`。
- **最终矩阵（最终源码树全量复跑）全绿**: fmt ✓·default 227/227·
  sim 227/227·mock 393/393·**gst 266/266**·clippy×3（default/mock/
  bmd,gstreamer）`--all-targets -D warnings` 全过（本轮 clippy 升
  ×3——switch_graph 仅 gst 配置编译、switch_mock 仅 mock 配置编译,
  双配置 clippy 才能覆盖全部新代码面）。

### 14.5 边界与红线（维持+增量）

- Domain（program timeline/Authority）/谓词/Gate/阈值零字节; 三
  blocking 维持 Failed; 首败留证。
- **步骤 6 ⏸️ / 步骤 7 ⏸️**（终裁指令——先完成 Fence 修正后的确定性
  验证即本轮）; 步骤 10（全回归）/11（新鲜 Gate）待执行。
- A2-8-04 仍 🔴 FAIL/HOLD; A2-8-05 不进入。

## §15 R58 步骤 5.1 源码级调用链终审（ffdb9ce）——A/B/C 三边界核验 + CLOSED 登记

### 15.1 终审令与结论

验收层第一轮独立复核（ffdb9ce=真实远端基线确认·ahead_by=1·4 生产
/契约文件+3 审计文档）裁定: Step 5.1 设计闭环基本成立, 关闭前置=
三项源码级最终审查（A Seqnum 捕获/匹配唯一性; B timeout 路径绝不
伪装确认; C EVENT/BUFFER 微观序与测试-生产对齐）。本轮按令对
ffdb9ce 实际调用链逐行核验——**A ✅ / B ✅ / C ✅（一项 GStreamer
内部行为依赖披露）→ Step 5.1 正式关闭**。

### 15.2 A. Segment Seqnum 捕获/匹配唯一性 — CONFIRMED

- `capture_segment`（:446-460）: 仅 `state==Armed && seq.is_none()`
  （first-wins）捕获; `confirm_segment`（:466-495）: 仅
  `Armed && !ready && captured==arriving` 置 ready+notify。序号身份=
  世代内三元组（generation+plane+Seqnum）——与终裁④"generation 身份
  闭合"一致。
- Segment 源全枚举: ①启动初始 Segment——fence 处 Open（arm 仅在
  switch_program ⓪）, `Armed` 门使捕获不可能; ②本次切换 Segment
  ——input-selector 翻转必推（rt_02 真链多轮绿=实机在案证明,
  apply_declared_mapping 的 segment_observed 前置即其事实链）;
  ③假设性伪 Segment（arm→switch 间无 pad 翻转、无 seek、live 源不
  自发新段——理论残余）: first-wins 捕获它→真实切换 Segment 序号
  不匹配→永不 Both-confirmed→超时 Err→**fail-closed**。失败方向=
  拒绝放行而非陈旧误认（安全方向——伪 Segment 只能把切换变失败,
  不能把未排空伪装成已排空）。
- 类型分离: BUFFER 门=PadProbeType::BUFFER（:192, Drop）; EVENT
  探针=EVENT_DOWNSTREAM（:171, 恒返 Ok）——Segment 在 Armed 期
  照常穿透, 终裁 §10"不阻塞 EVENT"成立。

### 15.3 B. timeout 路径绝不伪装确认 — CONFIRMED

- `confirm_and_release`（:870-877）: `Ok→armed=false`（Drop 空转）;
  `Err→armed=true`→Drop 走 `force_release`。**类型面分离**:
  `force_release_cutover_fence` 返回 u64 丢弃计数, 结构上无法构造
  `CutoverDrainEvidence`; 后者仅在真适配器 `release_after_drain`
  Ok 路径（:1303）与 Mock auto-confirm 构造, 不进 ProgramSwitchReport。
- `force_release` 生产调用点全枚举=**仅守卫 Drop（:883）**;
  switch_program 成功路径零调用（:958/:1360 为 cfg-test 包装器,
  :1324/:283 为 trait 实现本体）。
- `release_after_drain`（:502-520）: Open 翻转仅发生在观测
  `both_ready` 的**同一临界区内**; 超时分支 return Err 不翻状态。
- `switch_program:662` 以 `?` 传播（不吞不转成功）; 错误时点 Domain
  已在 SwitchExecuted 相（残留维持 §14.2 登记: 后续 declare
  InvalidPhase fail-closed, 恢复归会话级故障面）。
- **调用链终序（对终审⑥的正面回答）**: ⓪arm→①a/①b/①c→②declare
  →③install→④begin/switch→④Domain on_switch_executed→
  confirm_and_release（mark→wait→release）——无 defuse 先于
  executed、无 confirm 先于 executed 的隐藏序, 与冻结原则逐字相符。

### 15.4 C. EVENT/BUFFER 微观序 + 测试-生产对齐 — CONFIRMED（一项披露）

- 四接线点共用同一 FencePair 实例（build_program_pipeline 单一
  `fences: &FencePair`）: selector src EVENT 捕获（:181）·selector
  src BUFFER 丢弃门（:196）·appsink 消费门（:806/:851 经
  attach_program_*_sink）·appsink sink pad 确认探针（:806/:851
  attach_drain_confirm_probe）。
- T-F1/F2/F3 与既有两测全部驱动**生产方法**（discard_if_armed/
  capture_segment/confirm_segment/release_cutover_fence/
  force_release——测试行 :2325+ 逐点核验）, 无逻辑复刻; 真实 pad
  接线/真实事件序由 rt_02 全链覆盖（含确认等待, 盒上已绿）。
- "confirm 超越已投递旧 buffer"不可能: 旧 buffer 与新 Segment 同经
  queue→appsink sink pad **单流线程**; sink pad 探针在该线程触发,
  appsink sync=false/async=false 时 render/new_sample 亦在该线程
  内联执行——同线程 FIFO 保证旧 buffer 的消费门裁决先于 Segment
  确认。**披露（GStreamer 内部行为依赖, 非本仓代码可证）**:
  basesink 非同步渲染内联于推送线程属 GStreamer 标准行为——
  Step 7 真机 #8 NM 消失将提供端到端反证。

### 15.5 Step 5.1 CLOSED

- 关闭依据: 终裁 A/B/C 三边界全 CONFIRMED + 最终矩阵全绿（§14.4）
  + Domain/谓词/Gate/阈值零字节。
- 后续序（终裁令）: **Step 6 Mock 交错模型 → Step 7 真机 #8 →
  Step 10 全回归 → Step 11 新鲜 Gate**; A2-8-04 仍 🔴 FAIL/HOLD
  （三 blocking 维持 Failed, 仅 Gate PASS 可改）; A2-8-05 不进入。
- 本轮零代码零测试——纯 docs 登记轮。

### 15.6 C 项措辞终审级修正（验收层独立复核 GStreamer 官方语义后下达）

- 终审复核确认: SEGMENT 属**与 buffer flow 序列化的 downstream
  event**（serialized 事件与 buffer 保持数据流顺序）; AppSink 官方
  文档明确 **new_sample 回调从 streaming thread 发出**——本工程直接
  安装 AppSinkCallbacks::new_sample, 回调内立即执行消费门。
- 正确的生产路径表述: 旧 buffer → appsink sink pad → AppSink
  render/new-sample **streaming-thread callback** → 消费门 → 下一
  序列化 Segment → appsink sink pad confirm。在此路径上"Segment 已
  确认但其前面的旧 buffer 尚未经过消费门"不能作为正常串行执行路径
  成立。
- **措辞修正**: §15.4 将该顺序主要归因于"sync=false/async=false
  导致 inline 渲染"不准确——sync=false 是关闭时钟同步等待,
  async=false 是 BaseSink 状态转换语义; 真正决定序列关系的是
  serialized SEGMENT 数据流顺序 + AppSink 回调在 streaming thread
  执行。后续审计措辞统一为:
  > "C 已 CONFIRMED; 其排空证明依赖 GStreamer 的 serialized
  > SEGMENT 数据流顺序, 以及 AppSink new-sample callback 在
  > streaming thread 执行; sync=false/async=false 不应被表述为该
  > 顺序保证的主要来源。最终端到端正确性仍由 Step 7 真机 #8 验证。"
- 定案: Step 5.1 维持 CLOSED（结论不变, 措辞修正）; 不回滚代码;
  A2-8-04 维持 FAIL/HOLD; 进入 Step 6。

## §16 R58 步骤 6 执行: Mock 交错模型（代码轮）

> （本节交付后的独立复核终裁=IMPLEMENTATION PASS / TEST MODEL HOLD-1;
> 状态语义修复+命名/CI 口径修正见 §17——本节"控制/数据交错表达"等
> 措辞以 §17.2 修正后口径为准。）

### 16.1 终裁输入与 C 项修正登记

- 验收层终审: Step 5.1 = **CLOSED**（A/B 源码级成立; C 结论成立但
  措辞修正——排空顺序保证的正确归因=**serialized SEGMENT 数据流顺序
  + AppSink new-sample 回调在 streaming thread 执行**, sync=false/
  async=false 非该顺序主要来源[前者=时钟同步等待关闭, 后者=BaseSink
  状态转换语义]; 不回滚; 端到端正确性由 Step 7 真机 #8 验证）——
  §15.6 已按终审原文登记。
- Step 6 关键要求（终裁锁死）: 七事件交错词汇 AnchorSampled/
  OldStraggler/FenceConfirmed/InstallNew/SwitchNew/OldBufferDropped/
  FirstNewMapped; 双模式证明 无 Fence→M1 FAIL/NonMonotonic ∧
  有 Fence→M1 PASS/DiscontinuityDeclared; **Mock 必须模拟控制/数据
  线程交错, 非孤立测试 FencePair**。

### 16.2 实现（mock adapter 单文件 + runtime 测试; Domain/契约/真适配器零触碰）

- **程序面真实单调状态机**（R57-terminal 点名缺口闭合）: mock observe
  的 program pts_state 从硬编码 ValidMonotonic 改为真状态机——plain
  写点（legacy/段内续流/窜帧）: Unknown→VM·回退→NM sticky·否则保持;
  已声明边界写点（段首枚映射）: 违例→NM sticky·干净→DD（上一段 NM
  就此解除——R53 段作用域生命周期镜像; note_declared_boundary 净语义）。
- **消费门施加于 tick 交付**: Armed 期 tick 的程序出口交付一律
  cutover-discard（per-plane 计数, 不写 pts/状态/帧数）; 设备 PTS 照常
  推进（输入面不受门影响）; Segment 事件观测照常推进（EVENT 不被门
  拦截——与真实探针类型分离同构）。
- **竞态窗窜帧注入 API**（控制/数据交错表达）: `deliver_window_
  straggler`（协议级显式链直投）+ `stage_window_straggler`（Runtime
  级——下一次锚采样读毕即投递, 模型=[锚采样→install] µs 窗; 编排
  同步调用内不可插针, 由 mock 自身在 ①c 落点后触发）。
- **七事件交错日志**: CutoverInterleaveEvent 枚举（AnchorSampled/
  OldStraggler/OldBufferDropped/FenceConfirmed/InstallNew/SwitchNew/
  FirstNewMapped）; arm 清空（每世代独立证据面）; 生产序=Anchor
  Sampled→OldStraggler→[OldBufferDropped:门处置]→InstallNew→
  SwitchNew→FenceConfirmed→FirstNewMapped。**序映射登记**: 终裁
  列举序（FenceConfirmed 第 3/OldBufferDropped 第 6）按事件词汇
  理解; 生产序由 Step 5.1 终审锁死（drain 确认在 switch 之后, 门
  处置在窜帧到达时）——mock 按生产序记录, 七词汇全覆盖。
- **诚实计数**: mock release 交回真实 per-plane 丢弃计数+generation
  （此前恒 0——"状态记录面"披露升级为真实仿真丢弃面）; force_release
  不产 FenceConfirmed（强释=失败处置非确认——类型面分离同构）。
- **披露**: mock 锚=出口+步长外推（真实适配器=last PTS 无外推,
  第四十轮 α 修正未及 mock）——本轮不改（超出 Step 6 范围, 改动
  波及既有 mock 断言面）; M1 关系以"窜帧恒领先边界帧一帧"同构表达
  （mock tick 模型边界带 2-tick 前导: 边界=P+2 步长, 窜帧=P+3 步长;
  真实 M1: 边界 P 零间隙/窜帧 P+40ms）。queue 保序本身不在 mock
  证明范围（真适配器 T-F1/F2/F3 在案）。

### 16.3 测试（三测全绿）

- **T-M1-FAIL（协议级）** `switch_rt_03_m1_no_fence_straggler_
  reproduces_nonmonotonic`: 段#1 生效→①a 基线 P（无 arm——pre-R58
  编排）→①c 锚采样→窜帧直投（open 门 plain 写弧=基线推进至边界+1
  帧）→声明#2/install/switch→Segment tick→首枚映射=边界 P+2 步长
  <被推进基线→**NonMonotonic**（V+A 双面）→段内续流单调帧 NM
  sticky; 日志尾=五事件序（AnchorSampled→OldStraggler→InstallNew→
  SwitchNew→FirstNewMapped）, 全程无 OldBufferDropped/FenceConfirmed。
- **T-M1-PASS（协议级）** `switch_rt_03_m1_fence_closes_race_boundary_
  declared`: 同场景加 R58 编排序（⓪arm→①a[Armed tick 交付被门处置
  基线冻结]→①c→窜帧[armed 门→OldStraggler+OldBufferDropped 不写弧]
  →install→switch→确认式 Release[auto-confirm·FenceConfirmed{gen:1,
  discarded:4}=①a tick v+a+窜帧 v+a 如实]→Segment tick→首枚映射
  ≥冻结基线→**DiscontinuityDeclared**; 日志=恰七事件生产序。
- **T-RUNTIME（Runtime 级）** `timeline_rt_03_m1_staged_straggler_
  fenced_runtime_preserves`: ProgramExecutionRuntime 全链×2 切换;
  #2 前 stage 窜帧（①c 读毕投递）→runtime.switch_program 全链
  （⓪arm→…→executed→确认 Release→⑤ 循环）→**Preserved + 程序面
  DD（非 NM）+ 日志恰七事件**——生产编排序在 mock 交错的端到端证明。
- 装置 `m1_old_segment_rig`: 声明段经 SourceSegment 直构（Authority
  不参与——聚焦 adapter 交错面; 组态 complete_switch 落定后二切）。

### 16.4 盒证据（逐轮如实）

- run1: E0252 重复导入（PtsMonotonicity 经 pipeline 聚合导入已在,
  追加导入重复）→删; run2: 两新测 NotActiveSource（rig 缺
  complete_switch 落定组态）→补; run3: **mock 396/396 全绿**
  （393 既有零破坏+3 新增——pts_state 语义升级[切换后 DD 边界]
  未破坏任何现有断言）; run4 最终矩阵: fmt 差（两修复编辑晚于盒上
  fmt）→apply+拉回+mock 复验绿。
- **最终矩阵全绿: fmt ✓·default 227/227·sim 227/227·mock
  396/396·gst 266/266（不变——真适配器零触碰）·clippy×3
  --all-targets -D warnings**（default/mock/bmd,gstreamer）。

### 16.5 边界与红线

- Domain（program timeline/Authority）/谓词/Gate/阈值/真适配器
  switch graph/契约端口零字节; 三 blocking 维持 Failed; 首败留证。
- 步骤 6 完成; **下一步=Step 7 真机 #8 场景复现**（NM 消失+ #9
  生命周期仍立）→Step 10 全回归→Step 11 新鲜 Gate。A2-8-04 仍 🔴
  FAIL/HOLD; A2-8-05 不进入。

## §17 R58 步骤 6 独立复核终裁: IMPLEMENTATION PASS / TEST MODEL HOLD-1 → 修复落地

### 17.1 终裁（验收层独立复核 04c3dd1 实际源码后下达）

- 裁决: **Step 6 = IMPLEMENTATION PASS / TEST MODEL HOLD-1**（非回滚、
  非推倒重来——先修一个 Mock 交错模型自身的状态语义问题, 再进 Step 7）。
- 终裁确认成立项: 04c3dd1 为真实远端提交（SHA
  04c3dd1178a10d90c88f074ec273be96358a5873）; Mock 状态机/straggler
  注入/七事件日志/三测在案; 明确未修改 Domain/真 GStreamer adapter/
  契约端口。三项核心证明方向均对: 无 Fence→M1 FAIL（窜帧 plain 写弧
  推进基线→边界落回→NM+sticky）/ 有 Fence→M1 PASS（Armed 期 tick+
  straggler 走 discard→Release→新段首帧 DD; 丢弃计数实际累加非伪造
  常数）/ Runtime 级非孤立测 FencePair（stage_window_straggler 由
  runtime 的锚采样消费→真 switch_program 全链→Preserved+DD+事件序）
  ——"不是只把单元测试换个名字"。
- **HOLD-1 根因（终裁源码级发现）**: tick_once 的 timeline 分支顺序=
  先置 `first_mapped = true` 再检查 `cutover_fence_armed`——Armed 丢弃
  的缓冲先占用"首枚已映射"槽位。与字段定义（"首枚 B 缓冲已按声明映射
  施加"=真正被接受/映射的首帧, 非尝试过的首帧）不一致; 与步骤 6 自称
  "Armed 期 cutover-discard 不写 pts/状态/帧数"不完全一致（未写
  Program PTS 但改变了 timeline evidence state）。三测未抓到=Runtime/
  协议测试调用序恰好避开 [install+switch→仍 Armed→timeline 缓冲到达
  →Drop] 窗口——恰是 Step 5.1 要防的边界（post-switch、pre-release
  在途新 timeline 缓冲未走过 tick_once）。
- **修复处方（终裁原文）**: "Fence Drop 必须先于任何 first_mapped /
  timeline evidence 状态推进"——timeline buffer 到达→Fence Armed?
  Yes→Drop / No→更新 first_mapped/facts/Program PTS; 并补专门测试:
  install→switch→Armed→timeline buffer 到达→Drop→first_mapped 仍
  false→release→下一枚真正放行的 buffer 才成为 FirstNewMapped。

### 17.2 命名口径修正（登记层, 终裁 §3）

- 终裁原文: stage_window_straggler 的实现"实际上不是操作系统意义上的
  真实并发……它证明的是 Runtime call-chain cut-point injection 而
  不是真实控制线程/数据线程并发 race。这没有问题, 甚至是更好的确定性
  测试方式, 但登记时不能把它描述成已经证明了真实线程竞态。真实并发
  仍然要靠 Step 7 的 GStreamer/BMD 真机链验证。"
- 登记修正: §16.2"竞态窗窜帧注入 API"中 Runtime 级注入的正确口径=
  **Runtime 调用链 cut-point 注入**（确定性——①c 落点同步触发, 非
  OS 线程并发竞态）; §16.3 T-RUNTIME 的证明力=生产编排序在 mock 的
  端到端证明, **不含真实多线程竞态证明**; 真实并发=Step 7 真机。
  代码 doc comment 已同步该口径（stage_window_straggler + runtime
  测试头注）。

### 17.3 CI 口径修正（登记层）

- 终裁: GitHub 对 04c3dd1 无 combined status——"396/396 全绿"属盒上
  本地验证, 不得写成 GitHub CI 结论。
- 登记修正: §16.4 及后续所有矩阵结论一律以**盒上本地验证**口径表述
  （tar 通道上盒+盒上 cargo; 本仓无 GitHub CI 覆盖该提交）。

### 17.4 修复实现（单文件 mock adapter + runtime 测试注释; Domain/契约/真适配器零触碰）

- **tick_once timeline 分支重排**: segment_seen 推进（Segment=EVENT
  不受门拦截——与真实 EVENT/BUFFER 探针类型分离同构, drain 确认锚
  即挂在该事件上）→ **消费门（Armed→丢弃+计数+OldBufferDropped 入
  日志）** → first_mapped/facts/程序 PTS/帧数推进。被丢弃缓冲不再
  占用首映射槽位。门不分辨世代只认 Armed——post-switch Armed 窗口/
  confirm→release 控制隙内到达的新世代缓冲同受此处置（真适配器消费
  门同语义: appsink 门 discard_if_armed 不检查缓冲世代）。
- **同根证据面缺口顺带闭合**: 修复前 tick 路径门处置不入交错日志
  （仅窜帧路径入 OldBufferDropped）——tick 丢弃在证据面不可见; 现
  如实入账（词汇仍七, 事件数如实: T-M1-PASS/T-RUNTIME 日志 7→8——
  ①a Armed tick 门处置如实成为首个事件）。
- legacy 分支/deliver_straggler 审计: 门已在一切状态写之前（顺序
  本正确——零改动）。

### 17.5 新增回归测试（终裁处方逐条落地）

`switch_rt_03_m1_armed_gate_precedes_first_mapped_evidence_state`
（协议级）: 段#1 装置→①a 基线 P（未 arm）→锚采样→声明#2/install→
arm（install 后, 专测 post-switch Armed 窗口; 生产编排 arm 在 ①a 前,
Armed 横跨 ⓪→release 窗口语义相同）→switch（SwitchNew）→Segment
tick（EVENT 不拦, 无缓冲交付）→ **post-switch Armed 窗口首枚缓冲候选
到达→Drop** ——断言三件: timeline 行仍 no_evidence（first_mapped 未
推进——外部可见）/程序出口 PTS 冻结于 P/帧数不进→确认式 Release
（gen=1, v/a 各 1=窗口恰一次门丢弃）→下一 tick 首放行帧=**First
NewMapped**+DiscontinuityDeclared（映射=P+3 步长——比常规边界多一个
dropped tick 前导, 模型如实）→窗口日志恰四事件 SwitchNew→
OldBufferDropped→FenceConfirmed{gen:1, discarded:2}→FirstNewMapped。
**修复判别性**（修复前该测试必红）: 旧序下窗口内丢弃 tick 预占
first_mapped→timeline 行提前出事实（mapped_program_pts=Some(P)）+
FirstNewMapped 永不出现+首放行帧走 plain 弧。

### 17.6 盒证据（盒上本地验证口径——无 GitHub CI）

- run1: mock **397/397**（396 既有零破坏+1 新增 HOLD-1 回归一次通过）。
- run2 矩阵: default 227/sim 227/gst 266（真适配器零触碰, 不变）;
  fmt 差（新测断言换行）→盒上 cargo fmt+格式化文件拉回本地（语义零
  变化）+复验。
- run3 最终矩阵全绿: **fmt ✓·default 227/227·sim 227/227·mock
  397/397·gst 266/266·clippy×3 --all-targets -D warnings**
  （default/mock/bmd,gstreamer）。
- T-M1-PASS/T-RUNTIME 事件数断言 7→8 如实更新（①a 门处置入日志后
  的诚实计数）。

### 17.7 边界与红线

- Domain（program timeline/Authority）/谓词/Gate/阈值/真适配器
  switch graph/契约端口零字节; 三 blocking 维持 Failed; 首败留证。
- **Step 6 状态**: 终裁处方修复+回归测试已落地并全绿——终裁原文
  "这个测试一旦通过, Step 6 就可以正式 CLOSED"——测试已通过, 最终
  CLOSED 裁决权在验收层, 本轮不自行宣布 CLOSED。**Step 7 不先于该
  裁决启动**（终裁: "现在不要直接进入 Step 7"）。A2-8-04 仍 🔴
  FAIL/HOLD; A2-8-05 不进入。

## §18 R58 步骤 7 执行: 真机 #8 场景复现（fence 生效轮）

### 18.1 终裁输入

- 验收层终裁: **Step 6 = 实质 CLOSED（HOLD-1 解除）**——cfb0943 修复
  按处方准确落地（重排方向正确·回归测试击中 [post-switch+pre-release+
  timeline buffer 在途] 窗口·deliver_straggler 正确路径未被无谓改动·
  04c3dd1→cfb0943 严格单提交前进仅 5 文件）; 三层证明成立（协议 FAIL/
  协议 PASS/Runtime cut-point 注入口径正确）; 盒上验证=盒上本地验证
  PASS 不冒充 GitHub CI。**Step 7 正式放行**。
- Step 7 硬验收（终裁列举, 非测试命令跑绿）: #8 切换 M1 NM 消失·首枚
  新段边界保持 DD·后续普通帧恢复 VM/Continuous·#9 clean boundary 仍
  正确·V/A 双平面一致·无新跨边界 NM; 并把 C 项最后的外部行为假设落到
  真实 BMD/GStreamer 链——serialized SEGMENT 顺序+AppSink streaming-
  thread 回调是否兑现为"旧 buffer 消费门先于 Segment confirmation"。

### 18.2 部署与构建指纹（证据可审计）

- 盒部署=cfb0943 干净树 tar 通道; 源 SHA 抽验四文件**盒==HEAD 全等**
  （switch_graph d2167b82…/program_execution 64f8890f…/switch_mock
  1c5c17a0…/a204_obs 08fc1d63…）。
- gates bin 重建 `--features bmd,gstreamer`（DECKLINK_SDK_INCLUDE）
  md5=**d05be28fda9e5e5138f99c6828124afa**; manifest v5 md5=7521d17e…
  （与 R53/R54 记录一致）; 证据盒 `~/a2-8-02i-evidence/2026-09-06-
  r58-step7-fence-replay/`（header 五件套+REV+bin/manifest md5+四跑
  log 及其 md5: run1 07365dbe…/run2 b7e69a75…/run3 c84befd1…/run4
  509c7e3a…）。

### 18.3 真机四跑（2026-09-06 12:29-12:42 CST, fence 在链·switch_program 全链）

- **run1（#8 精确形态复刻 N=10 dwell=1s——R52 run2/R56 #8 闩锁同形）**:
  EXIT=0·10/10 采集完整·全 Preserved·ProgramEpoch(0)×10·**pr_v/pr_a
  对称 DD=58+VM=2**（首切前 2 行 PRE=VM·此后边界事实四态纪律保持
  ——R53 基线签名逐字复现）·**NonMonotonic=0**·adv=Some(false)=0。
  **switch #8 B→A 执行行: outcome=Preserved·seg=8·disc=
  DiscontinuityDeclared·v/a=Continuous/Continuous·epoch(0)**——R52/
  R56 的 #8 闩锁位点干净; #9 A→B: PRE=DD→新边界 SPAN=DD（干净边界
  生命周期仍立·无 NM 传播/无跨边界 NM）; SPAN av_delta 17.9/34.6/
  1.28ms 量级（drain 确认等待未引入秒级停顿——无 5s 逼近）。
- **run2（扩展窗 N=30 dwell=1000ms）**: EXIT=0·30/30·全 Preserved·
  epoch(0)×30·in/br 四路 VM=180 各·pr_v/pr_a DD=178+VM=2 对称·NM=0·
  adv=0。
- **run3（burst N=30 dwell=0——在途窗最大化压力形态）**: EXIT=0·同
  签名全净·NM=0。
- **run4（dual_input 五层 Gate 回归·fence 在 L4 切换链）**: **ALL PASS
  10/10（L0→L5+Teardown）**——L3 切前 state v=ValidMonotonic（legacy
  路径不变）; L4: switch_ok=true·timeline_ok=true·outcome=Preserved·
  程序面 state=DiscontinuityDeclared·v/a=Continuous/Continuous·
  ProgramEpoch(0)·segment/mapped/offset 证据行齐（两 face 分层签名
  同帧共存——R53 第三轮实证复现）; L5 三段+故障域归因+Teardown 全绿。
- **工件**: CRITICAL/错误类=既有隔离债零新增（gst_pad_unlink×4/跑+
  video_converter interlace 家族——与 R53 登记同类同量级）; 四跑
  **"cutover" 错误串=0**——71 个 fence 周期（70 obs 切换+1 L4）全部
  Both-confirmed 确认式 Release 零超时。

### 18.4 验收点逐条对照（终裁 Step 7 清单）

| 终裁验收点 | 真机证据 | 结论 |
|---|---|---|
| #8 切换 M1 NM 消失 | run1 #8 位 SPAN/POST=DD·NM=0; run2/3 扩展共 70 切换 NM=0 | ✅（见 18.5 口径） |
| 首枚新段边界保持 DD | 每切换执行行 disc=DiscontinuityDeclared·pr 行 DD | ✅ |
| 后续普通帧恢复 VM/Continuous | in/br 四路全程 VM=180 各·timeline face v/a=Continuous·pr 面 DD=R53 四态纪律的段声明事实（VM 恢复面=输入/桥/连续性 face, 与 R53 基线一致） | ✅（按 R53 语义分 face 读数） |
| #9 clean boundary 仍正确 | run1 #9: PRE=DD→SPAN 新边界 DD·epoch(0)·无 NM 传播 | ✅ |
| V/A 双平面一致 | pr_v/pr_a·in_v/in_a·br_v/br_a 计数全对称 | ✅ |
| 无新跨边界 NM | 四跑 NM=0·adv=0 | ✅ |
| **C 项真链兑现** | 71 fence 周期 Both-confirmed 零超时+零 cutover 错误+NM=0——serialized SEGMENT 顺序与 streaming-thread 回调在真实 BMD/GStreamer 链兑现为"旧 buffer 消费门先于 Segment confirmation"（若序被破坏: 要么确认超时要么 NM 重现——两者皆未发生） | ✅ 端到端反证闭合 |

### 18.5 诚实口径（样本量与证明分工）

- **NM 消失=必要非充分证据**: 历史 NM 事件本身概率性（0.1-0.6%/切换;
  R53 无 fence 30 切换亦 NM=0 未复现）——本轮 70 个 fence 切换 NM=0
  =无反例+基线签名保持, **M1 机制的确定性击杀证明在 mock 双模式+
  真适配器红/绿测**（T-M1-FAIL/PASS+R58 步骤 5 红测在案）; 真机贡献=
  fence 周期端到端可运行性（71/71 确认式 Release）+无新破坏。
- 裁决权: 本节为执行记录; Step 7 验收判定归验收层。后续=**Step 10
  全回归**（六路/Authority/Desired/epoch——dual_input L4 本轮已绿）→
  **Step 11 新鲜 Final Gate（冻结谓词）**——仅届时 A2-8-04 verdict
  可变。A2-8-04 仍 🔴 FAIL/HOLD; A2-8-05 不进入。

## 19. R58 步骤 7 终裁登记: ✅ PASS / CLOSED——C 项升级 CONFIRMED / REAL-HARDWARE-VALIDATED

### 19.1 终裁（验收层, 2026-09-06, 独立复核 6ab24a3）

- **Step 7 = ✅ PASS / CLOSED**。独立核对: 6ab24a3 为真实远端提交;
  零生产源码变更的验证/登记轮（Git 重点=部署指纹+真机四跑+登记,
  未把盒上结果冒充 GitHub CI）; tasks 已解除 Step 6 HOLD-1 并登记
  Step 7 真机执行结果。
- 五关键点逐条裁定通过:
  1. **#8 M1 位点**: B→A 第 8 切换实际通过——Preserved +
     DiscontinuityDeclared + V/A Continuous + ProgramEpoch(0) +
     NM=0, 直接击中 R52/R56 历史闩锁位点;
  2. **#9 生命周期**: 后续干净边界仍 DD、无 NM 传播——修复未破坏
     下一段生命周期;
  3. **V/A 一致**: run1-3 pr_v/pr_a 对称; run4 dual_input 五层回归
     通过（L4 明确带 fence、timeline_ok、Preserved、DD、V/A
     Continuous）;
  4. **Fence 真链**: 71 fence 周期全部 Both-confirmed, 零超时零
     cutover 错误串——真实 BMD/GStreamer 执行结果, 非 Mock 推演;
  5. **性能/等待面**: SPAN 1.28-34.6ms, 无 5s 级异常等待。

### 19.2 C 项升级

- **C = CONFIRMED / REAL-HARDWARE-VALIDATED**（自 §15.4 披露的
  GStreamer 内部行为依赖升级）: 真实 BMD→GStreamer→selector→queue→
  appsink→Fence→Program observation 链上 71/71 fence 周期正常完成,
  confirmation timeout=0 · cross-boundary NM=0 · #8 historical
  latch absent · #9 DD lifecycle preserved。
- **措辞红线保留**: "NM=0"不能单独证明 M1 永不存在——必要非充分
  （§18.5 口径不变）; 确定性因果证明仍来自 Mock 双模式+真适配器
  红/绿测试; 真机承担=Fence 协议落实到真实媒体链并观察无反例。

### 19.3 状态表（终裁）与放行

| 层 | 状态 |
|---|---|
| Step 5.1 Fence | ✅ CLOSED |
| Step 6 Mock 交错 | ✅ CLOSED |
| Step 7 真机 #8 | ✅ CLOSED |
| #8 M1 NM / #9 DD 生命周期 / V-A 一致 | ✅ |
| Fence Both-confirmed 71/71 / timeout 0 / 新增隔离债 0 | ✅ |
| A2-8-04 | 🔴 仍 FAIL/HOLD |

- 纪律: A2-8-04 不因 5.1/6/7 连续通过而自动转绿——三 blocking
  cells 由冻结门禁控制, 仅 Step 10 全回归+Step 11 新鲜 Final Gate
  允许改判。**🚦放行 Step 10**（重点非再证 Fence, 而是证明六路
  PTS→TimelineAuthority→Desired→observed active→epoch→V/A
  continuity→teardown 加入 Fence/Mock/真机修复后全局无回归）;
  之后才进入 Step 11 新鲜 Final Gate（冻结谓词重算, 此前不提前
  宣布 PASS）。

## 20. R58 步骤 10: 全回归执行记录（零代码轮, 2026-09-06）

### 20.1 部署指纹

- 双侧核验: ls-remote/rev-parse 均=**6ab24a3**（远端=本地, 工作树
  clean）; tar 通道上盒（排除 target/.git）+ BUILD_REV 落盘。
- 源 SHA 四文件**盒==HEAD 全等**: switch_mock 1c5c17a0…/switch_graph
  d2167b82…/program_execution 64f8890f…/a204_obs 08fc1d63…（与
  §18.2 同值——6ab24a3 为 docs-only, 源未变）。
- gates bin 重建 `--features bmd,gstreamer` md5=**440c761b…**（新
  构建指纹; 源 SHA 已证与 d05be28f 轮逐字节同源, debug 构建非逐
  字节复现属预期, 如实登记）; manifest v5 md5=7521d17e… 不变。
- 证据盒 `~/a2-8-02i-evidence/2026-09-06-r58-step10-regression/`
  （header 五件套+REV+bin/manifest md5+四跑 log 及 md5: run1
  8361e98a…/run2 2d2b724e…/run3 f9c60fbe…/run4 355330b3…）。

### 20.2 盒矩阵（本地验证口径, 非 CI）

fmt ✓ · default 227/0 · simulation 227/0 · mock 397/0 ·
bmd,gstreamer 266/0 · clippy×3（default/mock/bmd,gstreamer）
--all-targets -D warnings 全绿——**计数与基线齐平, 单元面零回归**
（Desired 状态机/fence/timeline/四态生命周期测试族全在其中）。

### 20.3 真机四跑（12:57-13:13 CST）

- **run1 dual_input 五层 Gate: ALL PASS 10/10（EXIT=0）**——L1a-d
  身份/绑定/能力/信号/Port↔Resource 闭包全 PASS; L2a/b 双输入会话+
  Tap/Bridge 接线; L3 program 推进（切前 VM=legacy 不变）; **L4:
  switch epoch=1·observed=B·completed·switch_ok=true·timeline_ok=
  true·outcome=Preserved{ProgramEpoch(0)}·程序面 DD·v/a Continuous/
  Continuous·seg=1·offset=123912**（TimelineAuthority+Desired+
  observed active+epoch+V/A continuity 的 Gate 级证据一行齐, fence
  在链）; L5 四 verdict（A-fail→B-alive/recover-A 桥复流/B-fail→
  A-alive/故障域归因完整）; Teardown session_stop/rt_inactive/
  Released 全 PASS。
- **run2 obs #8 精确形态（N=10 dwell=1000）**: EXIT=0·10/10·全
  Preserved·tl_epoch=ProgramEpoch(0)×10·NM=0·**adv=Some(false)
  出现=0**·tally: in/br 四路 VM=60 各·pr_v/pr_a DD=58+VM=2 对称
  （R53 基线签名逐字）; **#8 B→A 执行行 Preserved+PE(0)+seg=8+v/a
  Continuous+DD**（闩锁位点干净）·**#9 A→B 同签名, SPAN pr=DD
  （PRE=DD→新边界 DD 生命周期仍立）**; SPAN av_delta 毫秒量级
  （样值 1.56-31.8ms）无 5s 逼近。
- **run3 obs N=30 dwell=1000**: EXIT=0·30/30·全 Preserved·PE(0)×30·
  NM=0·adv=0·tally VM=180×4·pr DD=178+VM=2 对称。
- **run4 obs burst N=30 dwell=0**: EXIT=0·同签名全净。
- **错误类**: 四跑 ERROR=0·"cutover" 字串=0·CRITICAL 全=gst_pad_
  unlink 族 ×4/跑·interlace 家族 3-6/跑——**既有隔离债同类同量级,
  零新类**。
- **fence 周期 carried watch（非本轮目标）**: 70 obs 切换+1 L4=71
  周期全部确认式 Release（超时必产生切换失败截断/exit≠0, 均未
  出现）。
- 分析澄清如实登记: 初判 "adv=Some(false)=1" 系 grep 命中定位行
  自身字面量（该行含 "adv=Some(false) 出现: N" 字样）——locator
  明示 0, 证据行零非推进格。

### 20.4 回归结论

**全局无回归成立**: 六路 PTS（obs 三跑签名逐字）→ TimelineAuthority/
Desired/observed active/epoch/V-A continuity（dual_input L4 一行齐+
obs tl_epoch 全保持）→ teardown（L5+Teardown+obs 卫生打印全绿）,
盒矩阵基线齐平。A2-8-04 仍 🔴 FAIL/HOLD——Step 11 新鲜 Final Gate
（冻结谓词）前不宣布 PASS; A2-8-05 不进入。

## 21. R58 步骤 10 终裁登记: ✅ PASS / CLOSED（验收层全盘裁决, 2026-09-06）

### 21.1 终裁要点

- 验收层对 e09ed97 全盘裁决送达: **Step 10 = PASS / CLOSED**。裁决
  标准 = Step 10 为 regression 而非 implementation step——四条件同时
  成立: ① 生产源码未被修改（e09ed97 仅 docs 登记, 基线 6ab24a3, 盒上
  四关键源文件 SHA 与基线一致——不存在执行中改代码绕过回归）;
  ② 编译/API/测试零回归（盒矩阵 fmt/default 227/sim 227/mock 397/
  gst 266/clippy×3 与基线齐平; contracts Fence 接口无 default
  implementation 掩盖——实现缺失在编译/静态语义层暴露, 强于单个
  integration test 绿）; ③ 控制面→Fence→数据面状态链零回归
  （FencePair 单锁事务 V+A·Seqnum 身份匹配防假释放·Armed Drop 先于
  first_mapped/PTS 证据推进——Step 6 cut-point 修复+Step 7 真机
  replay+Step 10 全回归把该链闭合）; ④ 真机无反例（dual_input L4
  状态机收敛一行齐·#8 精确 replay·#9 lifecycle·V/A 对称·NM=0·adv=0）。
- 边界口径（终裁原话）: 验收层本轮 GitHub 连接器未成功返回 raw 文件
  内容/commit API——代码裁决基于此前对当前分支实际源码的审计+e09ed97
  登记+盒/真机执行记录, 不把"本轮工具重新抓取"冒充已读取事实; 真机
  数字按"执行记录已登记"口径, 不称"此刻独立复跑所得"。
- adv=Some(false) 误报定性获认可: 初判 1 例系 grep 命中 locator 行
  自身字面量, 证据行实际=0; 澄清入登记（否则后续审计产生错误历史结论）。
- 既有 artifact（gst_pad_unlink×4/跑·interlace 家族）不重新定性为
  A2-8 新缺陷——继续作为已知债务保留, 不重开 implementation debt。
- Mimosa advisory 口径（终裁原话入册）: "功能编译、测试、真机回归通过;
  Mimosa AST 扫描能力不完整, 因此不构成全项目静态安全保证。"——不阻塞
  Step 10（任务=regression 非 security certification）, 亦不得反向宣称
  项目安全。

### 21.2 状态表（终裁）

| 项 | 裁决 |
|---|---|
| e09ed97 | ✅ docs-only registration |
| Step 5.1 / 6 / 7 / 10 | ✅ CLOSED |
| C 项 | ✅ CONFIRMED / REAL-HARDWARE-VALIDATED |
| 生产源码回归 | ❌ 未发现 |
| 新错误类 | 0 |
| 真机 counterexample | 0 |
| Mimosa | ⚠️ advisory·AST 不完整（口径如上, 不阻塞不宣称安全） |
| A2-8-04 | 🔴 FAIL / HOLD（三 blocking cells 冻结门禁） |
| A2-8-05 | ⛔ BLOCKED |
| 下一执行单元 | Step 11 Fresh Final Gate |

### 21.3 纪律重申与放行

- **当前没有足够证据要求继续修改生产代码**（终裁第十三节）——特别不得
  为使 Final Gate 变绿而提前修改: Fence predicate / NM 定义 /
  ProgramEpoch 判据 / Desired-Observed 判据 / P1-P2b-P2c-1 gate
  threshold / known-artifact 分类——否则"验证失败"与"代码改到通过"
  混在一起, 违反 A2-8 冻结纪律。
- Step 10 ≠ Final Gate: 5.1/6/7/10 连续 PASS **不推导** A2-8-04 PASS。
- **🚦放行 Step 11——新鲜 Final Gate**: 冻结 predicates + 新鲜
  evidence window 重算 P1-pr_v/P2b-pr_v/P2c-1; 不修改 predicate·
  不降低 threshold·不新增 exemption; 仅两种合法结果——三项全 PASS→
  A2-8-04 PASS·A2-8-05 解锁 / 任一 FAILED→FAIL·HOLD+定位 blocking
  cell + 按 A 代码/B 环境硬件/C 已登记债分类。

## 22. R58 步骤 11: 新鲜 Final Gate 执行记录（零代码轮, 2026-09-06）——冻结谓词重算

### 22.1 身份链（P8）

- 部署: git archive e09ed97 tar 通道上盒（byte-exact commit 内容·
  排除 target/.git 等价口径的确定性加强）; 源 sha **864/864 文件
  盒==archive==HEAD 全等**; BUILD_REV=e09ed975…。
- **冻结 bin 复用**: gates bin md5=440c761b79457dd51f6dd49ca6aa5bb2 ==
  Step 10 登记值逐字节（未重建——案 b "冻结 bin md5 逐字节可比" 口径,
  R56 复用 7a0ed95c 同法）; manifest v5 7521d17e… 不变。
- 证据盒 `~/a2-8-02i-evidence/2026-09-06-r58-step11-final-gate/`（五件套
  +REV+bin/manifest md5+四跑日志及 md5+NM 抽取件[0 行·空文件亦证据]
  +av_delta 全序列 n=180+stats）; **入库审计副本** `evidence/bmd-10.30.
  15.10/a2-8-04-r58-step11-final-gate/`（md5sum -c 全过·盒上原件=
  origin·本地验证口径非 CI——同轮入库 Step 10 四跑副本）。

### 22.2 案 b 新鲜确认集（13:48-13:57 CST）

- run1 **OBS N=30 dwell=1000ms（=R56 失败窗同形）**: EXIT=0·30/30 采集
  完整·全 Preserved·tl_epoch=ProgramEpoch(0)×30·**六路 NM 行独立计数
  全 0**（in_v/in_a/br_v/br_a/pr_v/pr_a 各 0——nm-extract 0 行·tally
  无 NM 键）·adv=Some(false)=0（locator 明示·证据行 1080 格全 Some(true)）
  ·adv=None=0·tally in/br VM=180×4·pr_v/pr_a DD=178+VM=2 对称·VM 恰为
  首切前 PRE 对（switch#1 PRE A/B·行 35/36）其后全 DD=**P2b 签名逐字**
  ·**#8 B→A 执行行 Preserved+PE(0)+v/a Continuous+DD·SPAN pr_v=DD
  （R52/R56 闩锁位点干净）**·#9 PRE=DD→新边界 DD 生命周期仍立·
  av_delta n=180 min 2.037/max 118.704/mean 51.124ms（P4 登记性·
  R54/R56 振荡家族同形态·案 a 无阈值·无 5s 级等待）。
- run2 **dual_input: ALL PASS 10/10（EXIT=0 首跑·无需重试）**——L1a-d/
  L2a-b/L3 全 PASS; **L4: switch epoch=1·observed=B·completed·
  switch_ok=true·timeline_ok=true·outcome=Preserved{ProgramEpoch(0)}·
  程序面 DD·v/a Continuous/Continuous**（Authority/Desired/observed
  active/epoch/V-A Gate 级一行齐·fence 在链）; L5 四 verdict; Teardown
  全 PASS。
- run3 **hw 矩阵 bmd,gstreamer: 266/266 EXIT=0**（冻结字面 259+R58
  步骤 5/5.1 验收层批准测试增量 7——M1 复现×2+fence×2+确认式 Release
  ×3; 计数差量如实登记·非谓词改动）; rt_04×4/rt_05（switch_graph_
  rt_05_program_arc_lifecycle_declared_boundary_release）/fence 五测
  全 ok。
- run3b（窗口外补充件·P3 能力定向补证）: group_fold_rt_01_av_
  divergence_detected **ok**——**口径修正**: 该测试 cfg(test,mock)
  门控∈mock 397 套件非 bmd,gstreamer 集, R56 §15 表 "∈259" 引用不准
  （本轮定向补证+如实登记）。
- 错误面: run1 ERROR=0（区分大小写）·cutover 字串=0·pad_unlink×4·
  collision×2·interlace×3·MainContext×1——零新类; vs R56 OBS 4/2/3/0
  仅 MainContext 0→1 **已知类内计数漂移**（与 Step 10 run3 同形
  4/2/3/1·OQ-P5 如实登记）; 分析过程一步 "ERROR:2" 系 grep -ci 命中
  WARN 行 error= 字段, 区分大小写复核=0（与 §20.3 口径一致·如实登记）。

### 22.3 逐格 verdict 重算（冻结谓词·词表 v2.1·窗口=22.2 三件+run3b 补证）

| 格 | verdict（R56 → Step 11） |
|---|---|
| P1 in_v/in_a/br_v/br_a | Satisfied → Satisfied（各 VM=180·NM=0·充分性前置过: 每路 PRE 60/SPAN 60/POST 60 非 Unknown） |
| P1 pr_a | Satisfied → Satisfied（NM=0·VM=2+DD=178 对称） |
| **P1 pr_v** | **Failed → Satisfied**（**NM=0**·VM=2+DD=178·R53 语义后全新窗 30 切换·#8 位点干净） |
| P2 核心 outcome↔continuity | Satisfied → Satisfied（30/30 Preserved·v/a Continuous×30·NewEpoch/FailClosed/Violated/TransitionFailed=0） |
| P2a | Satisfied 记录性（in/br 180 行全 VM·无 DD 制造） |
| **P2b pr_v face** | **Failed → Satisfied**（{首切前 PRE=VM 恰 2 行·行 35/36}∧{其后=DD 178 行}·pr_a face 同构） |
| **P2c-1 可观测通道** | **Failed → Satisfied**（①程序面 NM==0 与 P1 pr 路交叉引用·不合并且同数字 ②Violated 观测==0·OBS 30/30 Preserved + dual_input L4 Authority outcome=Preserved 同检） |
| P2c-2 | Gap 披露（Authority sink 缺失·Unproven·不阻塞·维持） |
| P3 D1 | Satisfied → Satisfied（窗内 av_paired 分离观测=0[A/B 设备逐相位 av_delta 全等]·能力在证=run3b ok[口径修正见 22.2]+R46 活体背景） |
| P4 D2 | Satisfied 登记性（min/max/mean=2.037/118.704/51.124ms·全序列入盒·案 a 无阈值） |
| P5 starvation 六路 | Satisfied → Satisfied（Some(false)=0·None=0·SPAN 含被切离路推进） |
| P6a | Satisfied → Satisfied（rt_05∈266 绿） |
| P6b | FieldProven（R57 升级维持·本窗 #9 生命周期复证·不改写 Satisfied） |
| P7 | Satisfied → Satisfied（dual_input 10/10 首跑 EXIT=0） |
| P8 | Satisfied → Satisfied（22.1 身份链+工件零新类·MainContext 类内漂移如实登记） |
| P9 | Gap 披露（stalled 硬编码/S5 caps=None/switch_mock 分歧/P2c-2·维持） |

### 22.4 Gate 层固定合取与状态

**全部 blocking 格 Satisfied ∧ Gap 格披露齐（owner/reason/scope 在册）
∧ 增量格登记（P6b FieldProven 不改写 Satisfied）∧ P8 证据完整 →
A2-8-04 Final Gate = PASS（冻结谓词机械产出）**。三 blocking cells
（P1-pr_v/P2b-pr_v/P2c-1）由 R56 Failed → 本窗 Satisfied——驱动差异 =
R58 Fence+Mock 交错+HOLD-1 修复后, **同一失败窗形态（N=30 dwell1000）
NM 消失且 R53 基线签名完整**, 与 Step 7 #8 精确形态复现相互印证。

- **A2-8-05 = 解锁待令·不进入**（item 7 仍为唯一入口·待用户明令）;
  **Step 11 验收终裁归验收层**（PASS 为冻结合取的机械产出与登记·非
  提前宣布; 本轮零代码零判据零豁免·首跑留证纪律未触发[无 FAIL 跑]
  ·无重试）。

## 23. R59 步骤 11 验收终裁登记: **A2-8-04 = PASS / CLOSED**（2026-09-06）

### 23.1 终裁结论（验收层裁决要点逐条入册）

- **代码裁决: PASS。架构裁决: PASS。Fence 状态模型: PASS。Control
  Plane → Contract → Data Plane 调用链: 闭合。Mock ↔ GStreamer 两侧
  契约: 闭合。Step 5.1 / 6 / 7 / 10 / 11: 全部 CLOSED。A2-8-04:
  PASS**——原话: "没有发现必须回滚或重新修改生产代码的问题。"
- 证据层终裁表（验收层十八行表）全数通过: Source identity / Frozen
  binary identity / Manifest identity / OBS N=30 / P1-pr_v / P2b-pr_v /
  P2c-1 / dual_input 10/10 / hw matrix 266/266 / P3 capability（Mock
  定向补证）/ Fence 71 cycles / New ERROR class = 0 / New
  counterexample = 0 / Step 11 生产源码改动 = 0 / 冻结谓词改动 = 0 /
  A2-8-04 = PASS / A2-8-05 = NOT STARTED·BLOCKED BY GOVERNANCE
  （终裁时点状态; 同轮末尾明令进入 release 链 → R59 开启, 见 §23.4）。
- 架构层认可要点（验收层代码裁决依据·登记备查）: FencePairState 收
  Arc&lt;Mutex&gt; 单事务状态域（V ready/A ready/captured seqnum/
  generation/discard counters 同域——V+A 确认与释放无双锁漂移）;
  generation 生命周期隔离（晚到确认不污染下一轮 fence 状态）;
  Armed Drop 先于 first_mapped/PTS/帧证据推进（被 Fence 丢弃的旧
  Buffer 不再伪装 FirstNewMapped——证据状态防污染）; install→
  on_switch_executed 两段推进（计划切换 ≠ 实际媒体切换）;
  release_after_drain 超时 = Err 非 Ok（timeout 不等价 success·
  fail-closed）; 契约 trait 无默认空实现（无"编译过而功能不存在"
  后门）; Mock 证明协议顺序/状态机/事件因果 + 真机证明真实
  queue/drain/Segment/appsink/selector/buffer——两证互补非替代;
  A2-8 与 Transport/SessionManager 状态域分离（不为 A2-8 再动
  Transport/Session 生命周期）。

### 23.2 裁决基础与边界声明（逐字登记）

- 基础 = 冻结谓词 Step 11 重算（§22）+ 验收层既往源码审计 + 本轮
  登记的执行记录与盒上证据。
- **边界声明（验收层原话）**: "我本轮 GitHub 原始文件读取接口没有
  成功返回正文，因此上述'当前 HEAD 的远端代码逐文件复核'不能冒充
  已经在本轮重新完成；代码部分是基于此前已经实际检查过的源码 +
  你本轮登记的 Step 11 实际证据裁决。"——真机数字按"执行记录已
  登记"口径采信, 不称本轮独立复跑。

### 23.3 残余风险四条冻结登记（验收层明令·后续轮禁触碰）

1. **force_release = 故障恢复动作, 非成功切换证据**（"force
   release ≠ successful cutover evidence"）; 禁改为
   force_release→return Ok 式"快速恢复"。
2. **P3 capability proof = Mock 定向补证**——文档口径冻结: 必须写
   "P3 capability proof = Mock", 禁写 "P3 physical hardware
   proof"。
3. **sync=false / async=false 非正确性依据**（仅 AppSink 行为配置）;
   正确依据 = SEGMENT event 序列化 + pad probe + new-sample
   streaming callback + 共享 FencePairState。
4. **Mimosa = 工具能力限制**（python_ast_unavailable）——"功能
   编译通过/测试通过/真机回归通过"成立, "全项目静态安全审计通过"
   不成立。

### 23.4 红线与状态表

- **红线（验收层明令）**: 不再动 A2-8 核心四文件（switch graph /
  program execution / switch 契约 / switch mock）做无必要优化式
  修改——"当前最大的错误反而会是继续在已经通过的 A2-8 Fence 核心
  代码上做'优化式修改'"。
- 状态表: Step 5.1 / 6 / 7 / 10 / 11 = CLOSED; **A2-8-04 = PASS /
  CLOSED**; **A2-8-05 = 解锁（R59 开启）**——收口时序用户裁决 =
  **开 PR 跑 CI · 链末收口**（本轮开 PR 仅求 GitHub CI 真实信号·
  不 merge; archive+merge+tag 推至 Step 13-17 阶梯完成后）;
  阶段切换 = 核心切换正确性验证 → **正常使用形态验证**（Step
  12-17 阶梯, §24.4）。

## 24. R59 步骤 12: A2-8-05 前置解锁与基线冻结（2026-09-06）

### 24.1 基线身份链（冻结对象）

- commit **4b473b3**（remote = local 已核·R58 unit 9）; 源基线
  **e09ed97**（4b473b3 对 `services/` 零改动——`git diff --stat
  e09ed97..4b473b3 -- services/` 为空, 27 文件全为 docs/evidence/
  memory/tasks）。
- 核心四文件 SHA256（Step 10 四文件口径·本轮盒==本地复核**全等**）:
  - switch graph: `d2167b82f8087d715c9441d35a705dfd7fd31620e9f2e856ceb39848f344ab90`
  - program execution: `64f8890fe739216181beedd4ccc4ff1160b510b4e474c56a7e8248e512c728ad`
  - switch 契约: `74189c2fa886b881ddcc97d7e61d5c41fa2fa3698f223952538626bc65f7459e`
  - switch mock: `1c5c17a010b7291857e355ed2aa49cc61f94f04b0f55542b2cf433eaac44f5f9`
- manifest v5 md5 `7521d17e` 不变; 冻结 gates bin md5 `440c761b`
  （R58 Step 10/11 案 b 口径）; 矩阵计数 fmt / 227 / 227 / 397 /
  266 + clippy ×3（-D warnings）。
- 证据盒索引: `~/a2-8-02i-evidence/` 三箱（2026-09-06-r58-
  step7-fence-replay / -step10-regression / -step11-final-gate）+
  `evidence/bmd-10.30.15.10/` 入库副本（盒 = origin·本地验证口径
  非 CI）。

### 24.2 冻结清单（禁改面）

- A2-8 核心四文件（上列 SHA 锚定）+ 谓词文档 §2/§3/§4 冻结文本 +
  词汇表 v2.1 + 全部阈值 + 豁免清单 = **禁改**; 验收层 Step 12 令:
  "后续版本只允许新增验证, 不允许为了通过测试修改 A2-8 核心谓词";
  force_release 语义禁改（§23.3-1）。

### 24.3 A2-8-05 前置审查事实表

- **CI**: `.github/workflows/media-agent.yml` 7 job（rust-format /
  architecture-portability / rust-clippy ×2 / session-lifecycle /
  rust-test-matrix / hardware-test-compile / gstreamer-build）;
  触发 = master·main push + 全部 PR——**当前分支 114 提交从未跑过
  GitHub CI**; hardware-test-compile 依赖 secrets
  DECKLINK_SDK_HEADERS_1/2 分片注入（缺失则步骤门控跳过·非代码
  回归——盒上已证编译）。
- **收口时序（用户裁决 R59）**: 本轮开 PR 跑 CI（不 merge）→
  Step 13-17 阶梯完成 → 链末 archive + merge + tag。
  **执行结果: PR #30 已开（github.com/pwl1987/VBMF/pull/30）·
  GitHub CI 7/7 首跑全绿**（run 34018242258·2026-09-06——
  hardware-test-compile secrets 在位 bindgen+FFI 通过·
  gstreamer-build 通过; 本分支 114+ 提交首次 CI 实测）; 不
  merge 维持。
- **归档/合并惯例**: `changes/archive/` 29 例 = `YYYY-MM-DD-
  <change-id>` 整目录迁移（`specs/` 目录为空·真实 spec/设计同步走
  `docs/superpowers/` specs+plans+reports）; merge 惯例 = 每 change
  一 PR squash 进 master + `phase-*` tag（16 例 phase-0.6-* …
  phase-0.7D-*）; `master..HEAD` = 114 提交纯领先·master 顶 =
  7745968（A2-7 #29）。

### 24.4 步骤 13-17 阶梯登记（验收层路线图入册）

Step 12 前置解锁与基线冻结（本节）→ **Step 13** 正常使用形态测试版
（真实运行入口验证·非 gates 二进制）→ **Step 14** 真实业务操作测试
（A→B / B→A / 连续 / 多次 / 长时 / 输入丢失恢复 / 单双路异常 / 重启
恢复; 观察 Desired/Observed 最终一致·epoch 单调·Timeline/Fence
正确·无旧帧穿透/状态卡死/恢复漂移）→ **Step 15** 长稳（30min → 2h
→ 8h → 24h; 内存/FD/socket/pipeline/pad-probe 泄漏·repeated
switch 漂移·BMD 长稳）→ **Step 16** 控制面/Transport 联调
（API→Command→Program State→Switch→GStreamer→Observed/Health
回读·验证调用链完整性非重验 Fence）→ **Step 17** 正常使用测试版
（版本号/源 SHA/binary SHA/manifest SHA/部署目录/启停/配置样例/
health/操作入口/回滚/测试报告/已知限制）= **VBMF Media-Agent
Normal-Use / Engineering Preview**。
