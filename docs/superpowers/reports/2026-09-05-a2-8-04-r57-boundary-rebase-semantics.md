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
