# A2-5-05 — Master Join Semantic Deep Review（inconsistency 深化 + 全模型语义复核）

> Status: `SEMANTIC REVIEW ONLY / NO CODE CHANGE（本文件; 另含 04 措辞微修）`
> Authority: A2-5-04 终裁（APPROVED/CLOSED; compose 措辞修正; 05 聚焦
> inconsistency 从真实消费者反推，不提前加字段）
> Date: 2026-09-02 · Base: `d9e24c7`

---

## 0. 04 措辞修正落实（随本刀唯一 .rs diff）

`compose()` doc："唯一构造入口" → "**唯一的显式语义组合入口**"（终裁 §5
措辞一; 补充终裁 §6 信任模型声明："pub 字段 + serde 重建合法——与现有
Master 同一构造信任模型（A2-2 纪律）; compose 锁语义入口唯一，非语言级
构造唯一; 锁定对象=禁 from_join/build_from_join/join_and_compose/本类型
内重算"）。Design/probe 报告同步更新；盒上复验零行为差异。

## 1. 消费者反推取证（inconsistency 深化的五项检查）

### Q-A: A2-6 Projection 需要什么？（未来消费者①）

`api_boundary.rs` 投影先例形态 = **`to_api_*(&RuntimeState) -> Api*`** 纯
映射函数族——**投影消费的是完整 Domain 对象（字段可见），不是 predicate
标志位**。即 A2-6 若需暴露 inconsistency，其信息源是 `MasterJoinOutput`
整体（可读 `classification_input.inconsistency` + `result` + `eligibility`），
**不依赖** `inconsistency` 自身携带 reason——投影层自行决定暴露什么 wire 字段。

### Q-B: Runtime/Safety classify 需要什么？（未来消费者②）

**现有 classify_failure_domain 零代码**（§8.10 决策链属未来 Runtime/Safety
侧）；现有最近似先例 = `supervisor::fault_trigger_from_events`——**输入是
事件流（含 summary 文本），输出 bool 谓词**。即 §8.9 域分类（SOURCE/
PIPELINE/MASTER/OUTPUT…7 域）的输入形态是**结构化事实集合**，不是单个
reason 枚举。`JoinClassificationInput` 已携带 `avsync` + `inconsistency` +
（via Output）`eligibility`/`result`——§8.9 所需的全部 Join 侧事实**已经
可得**；缺的是故障域归属判定本身，而那是 §8.9 侧职责（红线 8: Join 零
action；OQ-D: classify→action 归 Runtime）。

### Q-C: Watchdog/Recovery 需要什么？（未来消费者③）

`Supervisor` 决策输入 = RuntimeEvent 流 + `ProcessState`——**与 Join 平面
无既有接口**。按 A2-4-04 Boundary Contract（§3 五条消费规则），Join 产物
经 **A2-6 projection 转接**才入 Runtime——转换层届时按需取字段，不预设
reason 形态。

### 检查 1: `bool` 是否丢失 A2-6 所需语义？
**否**。A2-6 消费完整对象（Q-A），`inconsistency: bool` 只是其中一位；
丢失风险为零。

### 检查 2: 是否需要 reason / failure-domain candidate？
**当前证据不足**：无任何现有消费者请求 reason（Q-A/B/C 全部以对象/事实集
为输入形态）；`failure_domain candidate` 与 §8.9 职责重叠（R-E/R-H：分类
归 Runtime/Safety——Join 提供 candidate 即半执行分类，违反边界）。

### 检查 3: 若将来需要，加在哪一层？
**加法路径已通**：`JoinClassificationInput` 增字段 = **加法演进**
（struct 字段 additive，serde 端 MasterJoinOutput 无 wire 契约——Input/
Output 均 PartialEq-only 非 serde 对象，零 wire 破坏）；或 A2-6 投影层
自建 Api 级 reason 投影。**届时按真实消费者需求加，现在加 = 臆测**。

### 检查 4: 是否会把 Join 膨胀成 God Object？
**会**。reason/failure_domain/failure_class/failure_source 任意一项入
`JoinClassificationInput` 即开启分类语义内嵌（终裁原话），与 R-E/R-H
（Health≠Classification / Join≠Safety）直接冲突。

### 检查 5: C′ → Runtime/Safety 投影边界在哪？
**已在 A2-4-04 §3 锁定并维持**：`MasterJoinOutput`（含
`classification_input`）→ **A2-6 projection**（唯一转接层）→ Runtime/
Safety（classify_failure_domain → §8.9 action）。A2-5 内**无直连**——
边界成立，本次复核零改动需求。

## 2. 五项检查结论

`inconsistency: bool` **维持原样**——零字段追加。依据链：无消费者请求
reason（三项取证全否）+ 分类职责归 §8.9 侧（红线）+ 加法演进路径畅通
（Input/Output 无 wire 契约，将来加字段零破坏）+ God Object 风险实锤。
**"不提前加字段"终裁经消费者反推检验成立。**

## 3. 全模型语义复核（顺带扫描，非重点项）

- 六文件职责闭环（终裁 §10 图）与实际代码一一对应：三 Master 各自形态
  （stage 机×2 + declaration）→ `join()` 五步优先序 → `ProgramMaster`
  组合根——**零越界**；
- R-A..R-J 抽查（词表拒收测试存在性）：Result 跨平面拒收 / presence 拒
  Timecode 域 / declaration 拒 READY/JOINED / 键集正反向——**全部在位**；
- Option absence 语义（04 发现）：终裁已正式确认"完全正确"，PM-07 三
  Master fail-closed + join_result→None 双轨测试保持。

## 4. No-Build Gate 复认

本刀 .rs diff 仅 compose doc 措辞（终裁指令项）；零字段追加、零结构变更、
零消费者臆测实现。inconsistency 深化**结论=维持 bool**，交用户终裁确认
后进 A2-5-06 收口。
