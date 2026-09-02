# Tasks — a2-6-program-projection

> 四栏纪律。六刀链（用户裁定冻结）：00 Ownership/SoT Probe → 01 
> Consumer+Shape → 02 Projection 实现 → 03 Query 接线 → 04 API Projection
> → 05 Transport → 06 收口。02 前禁止制造第二个 ProgramMaster。

- [x] 1. A2-6-00 Ownership/SoT Probe: 八问逐项证据（Q1 Owner 零现状=
  SessionManager/MediaSession 字段全清点零 Program 引用 / Q2 **join() 与
  三 Master writer 零生产调用者——真前置** / Q3 Snapshot 独立边界=
  assemble 唯一装配点 + D14 绑定禁令 / Q4 API 命名不预设 / Q5 None 投影
  禁坍缩 / Q6 AVSync 透传禁 Health 化 / Q7 failed 不暴露 / Q8 
  inconsistency 默认不暴露）+ 禁止捷径红线落盘 + OQ-1..5 交裁; 报告=
  docs/superpowers/reports/2026-09-03-a2-6-program-projection-ownership-probe.md
  `Contract: A2-5 终裁六刀链+八问+禁止捷径` | `Implementation: 已` | 
  `Verification: 八问全有代码实锚·零 .rs diff` | `Gate: 无`
- [x] 2. 用户对 OQ-1..5 逐项裁决（2026-09-03 终裁落 probe 报告 §7: 
  OQ-1=B 角色批准[Program Runtime Custody] 实现 deferred A2-7 + 双禁令/
  OQ-2=Deferred to A2-7·Watchdog 非 writer / OQ-3=独立 snapshot+API 并列
  projection / OQ-4·OQ-5 deferred to 01 / Q6-Q8 原裁决批准; 事实修正:
  allowlist=7 查询+new=8 项 surface; A2-6-00 CLOSED）
  `Contract: 用户裁定权` | `Implementation: 已` | `Verification: probe §7` | `Gate: 无`
- [x] 3. A2-6-01 Consumer + Projection Shape Probe（Probe Only; 七项全证据）:
  真实消费者盘点（P1b Console 轮询 runtime×2 + HLS 探测, **零 Program 级
  消费**——与 join() 零调用者互为表里）/ API 语义三分类先例（运行状态/
  物化事实/Program semantic 事实新类）/ None wire 三先例（outputs 空 vec/
  observation additive 无 default/倾向 null）/ AVSync 透传零转换/snapshot
  并列位置=API 响应层非 CanonicalRuntimeState/ProgramQuery 零可查物倾向
  02 起纯函数先行/唯一转换点=to_api_* 同址族 + 转换器禁制造缓存组装
  （硬 Gate 执行声明）; OQ-6..9 交裁; 报告=
  docs/superpowers/reports/2026-09-03-a2-6-program-projection-01-shape-probe.md
  `Contract: 00 终裁 §7 七项+硬 Gate` | `Implementation: 已（零 .rs diff）` | 
  `Verification: 七项全有代码实锚` | `Gate: 无`
- [ ] 4. 用户对 OQ-6..9 终裁（A2-6-02 前置: 命名/None wire/挂载点/暴露面）
  `Contract: 用户裁定权` | `Implementation: 待` | `Verification: 裁决记录` | `Gate: 无`
- [ ] 5. A2-6-02..06（Projection 实现→Query 接线→API→Transport→收口）
  `Contract: 六刀链` | `Implementation: 待` | `Verification: 后续核` | `Gate: 后续定`
