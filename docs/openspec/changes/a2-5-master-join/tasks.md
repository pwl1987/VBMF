# Tasks — a2-5-master-join

> 四栏纪律。七刀链（用户裁定）：00 Probe → 01 Shape → 02 输入/输出模型 →
> 03 实现 → 04 ProgramMaster+AVSync → 05 Semantic Review → 06 收口。

- [x] 1. A2-5-00 SoT Probe: V0.2 Join 侧全景（§1.20/§8.9/§8.10/§8.11/
  §3.8+§3.13/11 出现点）+ 代码现状复核（Join/ProgramMaster/AVSync 零代码
  未变）+ 十危险点双锚表 + OQ-A..E/PD-1..4 + 报告落
  docs/superpowers/reports/2026-09-02-a2-5-master-join-sot-probe.md
  `Contract: V0.2 §1.20 L155+§8.9-8.11+Errata-9+A2-4 Boundary Contract` | 
  `Implementation: 已` | `Verification: 十危险点全双锚·零 .rs diff` | `Gate: 无`
- [x] 2. 用户对 OQ-A..E 逐项裁决（2026-09-02 终裁落 probe 报告 §8: A Join 出
  判定声明/Runtime 消费 / B ProgramMaster=组合根禁展平 / C AVSync 声明面+
  DB≠SoT / D classification input 归 Join·action 归 Runtime / E 禁 
  all==MASTER_JOINED+禁 Participating→Ready; R-A..R-J 硬约束升级）
  `Contract: 用户裁定权` | `Implementation: 已` | `Verification: probe §8` | `Gate: 无`
- [x] 3. A2-5-01 Domain Shape Probe（16 项必查 D1-D16 全证据）: 三 Master 
  API 非对称无公共 trait / 零隐性 Join 类型·零 AVSync·零组合根·零投影·零
  wire 占名 / **三大新发现: offset/drift 已被 ClockObservationState 占用须
  消歧·SupervisorAction 已有家 Join 零 action 词·AgentState::Ready 占用故
  Join 端口禁 Ready 词** / D13 零合法时间类型 / D14 PTS 前体在 gate 观测域 /
  D16 五端点冻结 Join 零消费面; **零真契约冲突**; 报告=
  docs/superpowers/reports/2026-09-02-a2-5-master-join-shape-probe.md
  `Contract: 终裁 §九 16 项清单+R-A..R-J` | `Implementation: 已（零 .rs diff）` | 
  `Verification: 16 项全有代码实锚` | `Gate: 无`
- [ ] 4. A2-5-02 输入/输出模型裁定（Design Doc §1 四件提案已交付, 待用户
  终裁）: ①MasterJoinInput/Output 最小闭合（组合参数零 trait; avsync 非
  Option; failed 事实参数注入; Result 三值 Acceptable/Degraded/Failed=Program
  Join semantic failure）②三层矩阵（Eligibility 三域判定·复用
  is_program_scope_master 不重定义/Readiness 合取/Result 四行: C′ 矛盾→
  Failed·双路 failed→Failed·单路 failed→Degraded §1.20 逐字·否则 Acceptable;
  AVSync 不改 Result 伴随输出——待裁）③AVSyncClassification 四值+
  消歧三不（不复用 Clock/不复制 DB/不带 offset 字段名）+ 阈值归属 Join 零阈值
  ④投影边界表（Degraded→§8.9 Master 域信号; 禁 Channel 直推/禁 
  SupervisorAction 直映射）
  `Contract: 01 终裁四必裁+R-A..R-J` | `Implementation: 提案已交（零代码）` | 
  `Verification: Design §1.5 待终裁清单四项` | `Gate: 无`
- [ ] 5. A2-5-03..06（实现/ProgramMaster+AVSync 边界/Semantic Review/收口）
  `Contract: 七刀链` | `Implementation: 待` | `Verification: 后续核` | `Gate: 后续定`
