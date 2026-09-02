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
- [ ] 4. A2-5-02 输入/输出模型裁定（MasterJoinResult enum/词表 + 三域
  eligibility 真值矩阵 + AVSync 声明面与 Clock 词消歧——用户裁定后进 03 实现）
  `Contract: OQ-A/B/C/E 终裁+D1-D16 事实` | `Implementation: 待` | 
  `Verification: 后续核` | `Gate: 无`
- [ ] 5. A2-5-03..06（实现/ProgramMaster+AVSync 边界/Semantic Review/收口）
  `Contract: 七刀链` | `Implementation: 待` | `Verification: 后续核` | `Gate: 后续定`
