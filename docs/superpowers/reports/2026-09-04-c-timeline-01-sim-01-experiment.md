# C-TIMELINE-01-SIM-01 行为实验报告（IMP-3 + IMP-5 证据，不定案）

- 日期：2026-09-04（盒 2026-09-04T12:16Z 收官）；分支 `comet/a2-8-dual-input-switch`；
  基线 7ea5982（Impact Map=入口账，用户裁决 OQ-IMP-1..7：5 ADOPT+IMP-3/5 授权本实验）。
- 性质：**sim-only 实验刀**——只产生证据，不产生架构漂移。**未触碰**：
  normalize 字段 / PipelineHealth / L4 / SwitchGraph 正式逻辑 / Production graph
  / MediaTap / watchdog / supervisor（用户禁改清单全守）。
- 裁决权声明：本报告只回答 IMP-3/IMP-5 的事实面并给出**候选结论**；
  最终选型裁决权在用户。

## 1. 实验设置

### 1.1 工程与证据位置（不入库——"验证脚本不入库"惯例）

- 盒上 scratch 工程：`~/ct-sim-01/`（独立 cargo bin，gstreamer 0.23.7 +
  gstreamer-app 0.23.7；系统 GStreamer 1.28.2）。
  - `src/main.rs` sha256=`256590cba234be173d57cb0f770b118634beb7bf6b540783695e8f7af25373db`
  - `Cargo.toml` sha256=`d51e9d28c4fe311df6a3ace9aee59b8af3da103979a991d90cb9b3d75d475337`
- 日志：`~/ct-sim-01/out/*.log` 共 9 变体 2583 行（逐变体 sha256 见 §7；
  摘要证据在本报告内嵌）。
- **工程事实（顺带发现）**：gstreamer-rs 0.23 **无公开 `parse_launch`**
  （auto/functions 为 crate-private，handwritten functions.rs 仅导出三项无关
  函数）——实验 graph 与生产同构用 ElementFactory 程序化构链（生产代码
  从未用 parse_launch 故未暴露此差异）。Impact Map §2.4 补充事实。

### 1.2 拓扑（镜像生产 Bridged，产生真独立时基）

- 输入 A/B：两条**独立管线**各自 `videotestsrc is-live（25fps 320x180）/
  audiotestsrc is-live（48k stereo）→ capsfilter → inter[v|audio]sink`；
  **B 延后 200ms 启动**（base_time 独立=内容时间基准独立）。
- Program 管线（按变体）：`inter src ×2 → [变体元素] → input-selector →
  [变体元素] → queue → appsink(sync=false)`；EVENT_DOWNSTREAM 探针×2
  （selector src pad + appsink sink pad）+ appsink new_sample PTS 序列 +
  切换时刻/active-pad 读数全程记录。
- 切换：t=4.2s 翻 active-pad（sink_0→sink_1），后窗 4s。

### 1.3 变体矩阵

| 变体 | 结构/动作 | 目标 |
| --- | --- | --- |
| v0 | 生产 As-Is 基线 | 复现签名+自然边界事件序 |
| va | 每源分支 `identity single-segment=true`（selector **前**） | 分支归一 |
| vb | selector **后** `identity single-segment=true` | 出口单段 |
| vc-pre/post | v0 + 控制线程 `Pad::send_event(Segment)`（翻 pad 前/后） | 外部段注入+微观序 |
| vd-pre/post | v0 + selector src **BUFFER probe 声明映射重打 PTS**（映射安装于翻 pad 前/后） | probe 机制+微观序 |
| aud / aud-map | 音频基线 / 音频+vd-pre 式映射 | 双平面独立 |

映射公式（=Design Freeze OQ-2 语义）：`offset = program_continuity_anchor
− source_B_anchor`（anchor=切换前 program 末帧 PTS+1 帧时长；B anchor=B
分支 probe 实测末帧 PTS）。**非** R2 禁止的 max 假闭合（一次性声明式
offset，非逐帧钳制）。

## 2. 结果总表

| 变体 | buffers | backward | 首次回退 | appsink 收到 segment | 映射计数 |
| --- | --- | --- | --- | --- | --- |
| v0 | 241 | **1** | buf#121 Δ=−0.108ms | **2**（A 段+B 段） | — |
| va | 241 | **1** | buf#121 Δ=−0.220ms | 2 | — |
| vb | 241 | **1** | buf#121 Δ=−0.155ms | **1**（B 段被吃） | — |
| vc-pre | 241 | 1 | Δ=−0.133ms | 2 | **send_event 被拒** |
| vc-post | 241 | 1 | Δ=−0.267ms | 2 | **send_event 被拒** |
| **vd-pre** | 241 | **0** | **NONE** | 2 | **121/121 mapped·0 unmapped** |
| **vd-post** | 241 | **0** | NONE | 2 | 121/121·0 unmapped（赢得 1ms 竞态） |
| aud | 322 | 1 | Δ=−23.23ms | 2 | — |
| **aud-map** | 322 | **0** | NONE | 2 | 162/162·0 unmapped |

（PTS 打印注：harness 时间戳格式分数位不定长（6-9 位），本报告所有
数值已按节拍交叉解码复核；切换前后节拍全部规整 33.33ms（video）。）

## 3. 七个观察点的回答（F1-F7）

### F1 桥隐式重定时——"独立时钟域"到 program graph 只剩相位差

- 生产者 B 晚启 200ms（内容时基真独立），跨桥后 A/B buffer PTS 差仅
  **0.108-0.267ms（相位级）**——intervideosink/interaudiosrc 传递按
  **接收墙钟**重定基（do-timestamp=false 下 src 侧仍按接收时间基准
  交付）。v0 切换序列实测：`#120 3.969318407 → #121 3.969210317（B，
  −0.108ms）→ #122 4.002543651 → #123 4.035876984 → …` 规整 33.33ms。
- **对位真机**：生产 A/B in 列互差 8-10ms 正是此相位差（真机两发生器
  相位差大于 sim）；L4 NonMonotonic=切换点**相位回退**，非大基差跳变。
- 含义：Program Timeline Continuity 问题的实际规模=帧内相位级，映射
  修正是小量 offset——与 Freeze 的 Source Segment Mapping 模型吻合。

### F2 切换边界自然事件序（免费边界标记）

v0/va/vc 中翻 active-pad 后 selector src pad **自动转发**：
`stream-start(B 新 stream-id/group-id) → caps(B) → segment(B)`，同一序
到达 appsink sink pad（两处 EVENT 探针各见 2 个 segment）。**切换边界
在事件流上天然可见**——TimelineEvidence 可将"B 段 segment 到达 appsink"
用作 declared segment transition 的观测锚（零额外注入）。

### F3 identity single-segment 真实语义——只吃段不修 PTS（假阳性实证）

- vb：appsink **只见 1 个 segment**（B 段被 identity 吃掉=文档"eat
  segments"行为属实），**但 PTS 仍回退 −0.155ms**——single-segment
  **不重打 buffer PTS**（至少对 videotestsrc 无 offset 流）。
- va（分支位）：同样不统一（−0.220ms）。
- **观察点 7 直接实证**：下游若以"单一 segment"为连续性证据=**吞段
  假阳性**——事件面看似连续、PTS 面实际回退。identity single-segment
  **不得**作为 Timeline Mapping 的实现机制或证明面。

### F4 外部 Segment 注入被拒

vc-pre/vc-post：控制线程 `Pad::send_event(Segment)`（selector src pad）
**两序均 sent=false**（pad 拒收，GStreamer 1.28.2）。从控制线程外部
注入下行 segment event **不可行**——段声明只能经元素内部机制或
probe 路径承载（与 F2 的自然转发互补）。

### F5 probe 声明映射=完整可行机制（video+audio 双平面）

- vd-pre：映射安装→翻 pad→**backward=0**；B 首帧 PTS **精确落
  anchor**（`#120 3.969228130 → #121 4.009228130 =A 末帧+40ms`），
  后续全程 `+33.33ms` 规整节拍至窗口尾（121/121 mapped，0 unmapped）。
- aud-map：同样 0 backward、首帧精确落 anchor（162/162）。
- 机制要素：`PadProbeType::BUFFER` + `PadProbeInfo::buffer_mut()` +
  `Buffer::make_mut()` + `set_pts()`（crate 0.23.7 全可用）；映射值由
  Domain 声明、probe 闭包执行——与 R6（Domain 语义/Adapter 执行）吻合。
- **观察点 2/3/4/5 全部回答**：PTS 按预期映射 ✓；切换后单一 Program
  Timeline 维持 ✓；V/A 独立完成 ✓（各自 probe 各自映射）；未映射
  backward 仅出现在无映射基线 ✓。

### F6 微观序（IMP-5 证据）

- **pre-flip 安装**（先装映射→再翻 pad）：结构性无竞态——A 尾帧经
  probe 时 active 仍=A 不动，B 首帧到达前映射已在位。
- **post-flip 安装**（先翻 pad→再装映射）：本跑以 ~1ms 优势赢得竞态
  （安装 @4205ms，B 首帧 @4206ms，0 unmapped）——**竞态窗口真实存在
  但窄**（≥1 帧间隔）；控制线程更重即可能输。候选序=**pre-flip 安装
  为规范序**，post-flip 仅作反证。
- **附带发现**：`set_property("active-pad")` 返回后**立即读回=
  旧值 sink_0**（9/9 变体一致），而 buffer 流已切到 B——"切换已执行"
  的证明**不能**来自立即 readback；生效边界=下一缓冲（帧边界）。生产
  observe() 在 settle 后读回=B（真机证据）与"读回滞后但收敛"一致。
  TimelineEvidence 的 Observed 面须按帧/事件序列取证明。

### F7 基线复现生产签名（实验有效性锚）

v0/aud 基线唯一翻转点=切换后 program 首帧相位回退（仅 1 次 backward，
前后节拍规整）——与真机 L4"唯一失败项=prog pts NonMonotonic、A/B in
列各自 ValidMonotonic"同构。sim 实验面成立。

## 4. IMP-3 候选结论（待用户裁）

证据指向的执行点组合：

> **selector src pad BUFFER probe（selector 之后、per-plane 单点）+
> Domain 声明映射（anchor−B_anchor offset）+ F2 自然转发的 B segment
> 作为边界声明观测锚；不采用 identity single-segment（F3 假阳性）；
> 不采用控制线程 send_event 注入（F4 被拒）。**

对照 Impact Map §3-Q4 候选：选 (iii) probe 重写为主、(iv) 仅以 F2
自然形态消费（非注入）；(i)/(ii) identity 路线被证据否决。

## 5. IMP-5 候选结论（待用户裁）

> **规范微观序 = 取最新锚（observe 面）→ TimelineAuthority 计算并
> 声明新 SourceSegment 映射 → adapter 安装 probe 映射（pre-flip）→
> set_property(active-pad) → 生效边界=下一缓冲；settle=等待稳定证据
> （TimelineTransition 已成立）；Observed 证明走帧/事件序列，禁立即
> readback。**

（与 Design Freeze §10 状态机一致：SwitchExecuted 后 TimelineTransition
已成立，settle 不等待时间线重建。）

## 6. 诚实边界

- 每变体单跑（9 变体全 exit=0）；未做多跑方差统计——但关键判据
  （backward 有无、映射精确性、事件计数）为确定性签名不受抖动影响。
- 音频 post-switch 缓冲节拍与 pre 窗不同（21.3ms→25ms，源侧属性）
  ——与 IMP-3/5 无关，未解释仅记录。
- vc 被拒只证"控制线程从此 API 路径注入被拒"；未穷尽其它注入路径
  （如专门 element 内部发送）——但该路径已无必要（F2+F5 组合覆盖）。
- harness 用 videotestsrc/audiotestsrc 模拟（无 decklink 真源），F1 的
  相位重定基结论在生产 inter 桥同机制下成立（同一 intersrc 路径），
  幅值不同（8-10ms vs 0.1-0.3ms）。
- segment event 的**数值**（start/base）未解析（Debug 打印为 boxed
  指针）——事件计数/时序/到达面完整，数值面留实现批次（events 视图
  解析是标准 API，无风险）。

## 7. 证据哈希（盒 ~/ct-sim-01/out）

| 文件 | sha256（前 16） |
| --- | --- |
| v0.log | c2a7d11c9c037810 |
| va.log | bebb3cffac882ba5 |
| vb.log | d36fd7f77a0f25ba |
| vc-pre.log | 142c36c8e24de12f |
| vc-post.log | 504f8f5c611593a7 |
| vd-pre.log | b6236b80aeb4e402 |
| vd-post.log | c4c10a0aeb64cbd9 |
| aud.log | e9279cf451760b9f |
| aud-map.log | 76e72633ada89558 |

（全文哈希见盒；本报告证据行均可在对应 log 中 grep 复核。）

## 8. 下一步

- 待用户裁 IMP-3/IMP-5（候选结论 §4/§5）→ 冻结最小变更面 → 正式
  最小实现批次（Impact Map §4 九行候选清单+本轮实验钉死的执行点）。
- 实验工程留在盒（不入库）；若复跑：`~/ct-sim-01 && cargo run
  --release -- <variant>`。
