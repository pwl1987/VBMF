# Tasks — a2-4-metadata-master

> 四栏纪律。本 change = A2-4-00 SoT Probe（零代码）。执行纪律（用户裁定冻结）:
> comet-open → probe 报告 → design guard → handoff → STOP 等裁决。

- [x] 1. A2-4-00 SoT Probe: 七项必答+Q8 强制检查全证据落袋（V0.2 §3.7 拓扑原文/
  §3.1 METADATA+metadata_type 五值/Timecode 三重证据交叉/边界锚点/词表现状/
  阶段语义独立推导/VideoMaster+AudioMaster 零占位确认）+ 报告落
  `docs/superpowers/reports/2026-09-02-a2-4-metadata-master-sot-probe.md`
  （Evidence/Open Questions/Proposed Decisions/No-Build Gate 全节）
  `Contract: V0.2 §3.7+§3.1+§1.20+决策#29/#43+CLOCK_TIMECODE #148` | 
  `Implementation: 已` | `Verification: 报告七问全有节号级证据` | `Gate: 无`
- [x] 2. 用户对 OQ-1..OQ-6 逐项裁决（2026-09-02 终裁落 probe 报告 §7:
  OQ-1 Timecode=observation+AVSync=Join property / OQ-2 CAPTION / OQ-3 deferred
  X5 / OQ-4 五值 taxonomy+三源 topology / OQ-5 三层边界 / OQ-6 NO STAGE;
  附加红线四条 + 实施链 01→05 冻结; 批准进入 A2-4-01）
  `Contract: 用户裁定权` | `Implementation: 已` | `Verification: probe 报告 §7` | `Gate: 无`
- [x] 3. A2-4-01 词表冻结: `MetadataType` 五值（wire TIMECODE/CAPTION/SCTE35/
  KLV/SYSTEM 逐字 V0.2 §3.1+§1.13）+ `MetadataDataPlane` 单值 METADATA +
  Subtitle↔CAPTION 层级 doc + 词表快照 METADATA_TYPES + fail-closed（拒 
  SUBTITLE/SCTE_35/未知串, 测试锁定）+ 红线注释（三域差异/Timecode ownership
  四行/taxonomy≠topology）; **未写 MetadataMaster**（属 02）
  `Contract: V0.2 §3.1 L394-399+§1.13 L69+决策#43+终裁 §7` | 
  `Implementation: 已` | `Verification: 盒上 program 域 25/25（21+4 恰）+
  mock 277（基线 273+4 零回退）+ clippy 4-combo 零警告 + fmt 对齐` | `Gate: 无`
- [x] 4. A2-4-02-00 Domain Shape Probe（用户终裁: APPROVED TO PROBE, NOT TO CODE;
  12 条 NO-CODE 清单冻结）: 10 项必查全证据落袋（P01 零 payload 类型/P02 
  CanonicalSourceRef 已成熟·不新建 MetadataSourceId/P03 零 program identity/
  P04 scope=结构归属/P05 events 时间字段计数 0/P06 组合非复制/P07 零容器/
  P08 data_plane 两案/P09 enum+Option 复合惯例/P10 A2-5 零预留）+ Candidate 
  A/B/C 对比（B 基座⊃C source 维度, A 被 P01 否决）+ SQ-1..SQ-5 字段级待裁;
  报告=docs/superpowers/reports/2026-09-02-a2-4-metadata-master-shape-probe.md
  `Contract: 用户 §一-§二十终裁` | `Implementation: 已` | 
  `Verification: 零 .rs diff·清单完好·十项全有代码实锚` | `Gate: 无`
- [ ] 5. A2-4-02 编码（SQ-1..SQ-5 字段级裁决后）: MetadataFact/MetadataMaster
  (facts+join declaration, Candidate B); A2-4-03 字段语义+serde; A2-4-04 Join
  boundary review; A2-4-05 全回归+architecture guard; 交付链
  `Contract: 终裁 §二十 NO-CODE 清单+SQ 裁决` | `Implementation: 待` | 
  `Verification: 后续核` | `Gate: 后续定`
