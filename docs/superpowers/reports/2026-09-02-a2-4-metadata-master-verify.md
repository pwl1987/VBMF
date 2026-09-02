# A2-4 Verify Report — Metadata Master（Program Domain 第四块）

> Change: a2-4-metadata-master · Date: 2026-09-02 · Base: master `b378a0d`
> 最终提交链：`d0b16fa`（probe）→ `b8dbf88`（终裁+01 词表）→ `7342cdd`（02 模型）
> → `2a632ab`（03 语义深审）→ `90d34dd`（03 终裁 C′+04 边界契约）→ 05 收口
> 定位（终裁）：**A2-4 = Verification & Delivery Closure**——非 "Join
> implementation readiness"；Join/ProgramMaster/AVSync 零生产代码为 A2-5 未
> 开始的真实状态。

## 1. 交付物总账

| 阶段 | 产物 | 状态 |
|---|---|---|
| A2-4-00 SoT Probe | sot-probe 报告（7+1 问证据 / OQ-1..6 / PD-1..4 / No-Build Gate） | ✅ 用户六项终裁（报告 §7） |
| A2-4-01 词表冻结 | `MetadataType` 五值 + `MetadataDataPlane` 单值 + `METADATA_TYPES` 快照 + fail-closed（拒 SUBTITLE/SCTE_35） | ✅ CLOSED/APPROVED |
| A2-4-02 最小闭合模型 | `MetadataPresence` 三态 / `MetadataJoinDeclaration` 三态 / `MetadataFact{kind,source,presence}` / `MetadataMaster{data_plane,facts,join_declaration}` + 测试 05-09 | ✅ CLOSED（16 行字段表冻结） |
| A2-4-03 Semantic Review | 六组合矩阵（修正版）/ 四问审查 / TQ-1=C′（NotPresent 收紧 + Join fail-closed + 不加 is_consistent + producer-bug 定性撤销 + 5a 降级 + 三层规则） | ✅ CLOSED |
| A2-4-04 Boundary Contract | 九项探针 J1-J9 / Join 判定输入矩阵 / 五态红线 R-1..R-5 / C′ Join 侧消费规则五条 / Gap G-1..G-4 / §1.5b 六不变量+两条 A2-5 红线（Event Projection 不成 Join / D14 不偷渡） | ✅ CLOSED（Boundary Contract，非 Join 设计完成） |

## 2. 六架构不变量实证（终裁 G-A..G-F，逐项 grep/结构实查）

| 不变量 | 实证 | 判定 |
|---|---|---|
| G-A 结构不扩张 | `metadata_master.rs` 中 `pub (stage\|payload\|timestamp\|scope\|health\|status\|ready\|revision)` 计数 = **0** | ✅ |
| G-B Timecode SoT 不迁移 | `CanonicalTimecode` 仍在 `normalize.rs L104`（CanonicalMediaDescriptor）；program 域 2 处命中均为纪律注释，零字段零搬移 | ✅ |
| G-C Unknown 不坍缩 | `Unknown →(NotPresent\|Failed\|Degraded)` 映射零存在（全 program 域 grep） | ✅ |
| G-D 空 facts 无推导 | `is_empty` 唯一命中 = 测试断言 `facts` 空这一事实本身（SQ-4 正交测试），零 declaration 推导逻辑 | ✅ |
| G-E health 不入 | 词边界 grep `health\|fault\|degraded\|failed` 唯一命中 = L141 禁令注释（"禁止以 Vec 空推导 readiness/health/join"）；其余 20 命中系 `default` 含子串 `fault` 误匹配（词边界排除） | ✅ |
| G-F A2-5 placeholder 不落地 | `src/program/` 恰五文件（switch_policy/video/audio/metadata/mod），无 master_join.rs/program_master.rs；`MetadataMaster` 域外零消费 | ✅ |

## 3. 盒上全矩阵（p07_verify.sh，14 步 ALL_DONE，总 EXIT=0）

- fmt apply + check：PASS（FMT_CHECK_PASS）
- test×4：**default 185 / simulation 185 / mock 282 / bmd,gstreamer 185**——全 0 failed（mock 基线 277→**282**，+5 恰：presence 词表 / declaration 词表 / fact serde+键集 / 正交维度 / master serde+fail-closed）
- clippy -D ×4（default/mock/simulation/bmd,gstreamer）：EXIT=0 ×4
- build×3（gs-only / bmd,gstreamer / hardware-test）：EXIT=0 ×3
- remove-adapter PROOF：OK（Domain/Contracts/Runtime 无具体适配器可编译）

## 4. 硬件电池

A2-4 为声明性域对象零执行面——硬件行为零变化；盒上矩阵含 hardware-test build
+ bmd,gstreamer 全量 test（185/185）即硬件路径回归。真机 gate（E1-E8 等）无
涉及面，不重复跑（变更面 program/ 声明层，无 runtime 接线）。

## 5. 架构成果（终裁复述）

- 三域差异成立：Video/Audio=processing progression，Metadata=fact
  aggregation + join declaration——未复制 Stage 模式（OQ-6 全链贯彻）；
- `absence≠evidence` 与 `CapabilityFlag::Unknown`（runtime_state.rs L249）
  全仓库同构——Unknown≠NotPresent 有既有纪律背书；
- 三平面分离代码层成立：Program 声明 / Runtime 健康（AgentState+reduce）/
  Join 未来联合判定——A2-5 红线（Event Projection 不成 Join / D14 不偷渡 /
  五态不混淆）已入 Design §1.5b。

## 6. 债务与遗留

零新增债务。A2-5 前瞻约束清单（五态红线 / C′ Join 消费规则 / Event
Projection 禁令 / D14 禁偷渡 / AVSync 归 Join）全部记档于 Design §1.5a-1.5b
+ join-boundary-review 报告 §3-§4，属 A2-5 设计输入非本 change 债务。
