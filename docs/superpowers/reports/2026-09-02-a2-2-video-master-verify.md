# Verify 报告 — A2-2 Video Master（a2-2-video-master）

- **Change**: `a2-2-video-master`（full workflow, skip_specs:true）
- **分支**: `comet/a2-2-video-master`（base `78a8319` = master, A2-1 收口点）
- **代码提交**: `35c98c7`（+review 修复 `a1ca2ef`）
- **日期**: 2026-09-02
- **Design Doc**: `docs/superpowers/specs/2026-09-02-a2-2-video-master-design.md`

## 0. 结论

**Program Domain 第二块落地。** VideoMasterStage 阶段机（§3.7 逐节点对应）+
`advance_to` 显式目标白名单（5×5 全组合矩阵）+ **RAW 域类型层锁死**（Errata-3:
压缩域 Master 不可构造——`VideoDataPlane` 唯一变体）+ ProgramComposition 事实位。
声明性 only, 零行为变化。

## 1. 交付内容

- `VideoMasterStage`（SOURCE_RAW/NORMALIZED/SWITCHED/PROGRAM_COMPOSED/MASTER_JOINED;
  §3.7 Video Graph 节点逐一对应对应, serde 名 LOCK FINAL）
- `advance_to(target)`: 白名单 = 相邻下一阶段唯一合法; **5×5 全组合矩阵测试**（4 OK +
  跳级×3 + 倒退×4 + 同阶段×5 + 终态后继×5 全拒绝; `{from,to}` 载荷真实 wire 词表名）;
  no-arg `advance()` = sugar
- `VideoDataPlane` 唯一 `RawElementary`（**类型层**杜绝压缩域 Master——比运行时校验更强;
  serde `"COMPRESSED"`/`"H264"` fail-closed 测试）
- `ProgramComposition { applied }`（默认 bypassed 直通; "Clean Master" 术语不存在——
  Errata-3/Cleanup-3）
- **信任边界记档**（review Important#2）: pub 字段 + serde 可重建 = 声明性数据对象的
  有意设计（持久化/传输往返需要）; advance_to 是语义守卫非唯一构造路径;
  A2-5 消费 `is_program_scope_master()` 前须在消费点重审
- `ProgramDomainError::InvalidStageTransition`（A2-3+ 复用）
- **serde(default) 移除**（review Important#1）: 新生儿类型无旧序列化实例——
  additive 先例（D14/runtime_state）适用于"已有实例的类型加字段", 不适用于新生儿;
  缺字段 fail-closed 与模块自身纪律一致; `{}`/缺 composition 拒绝测试锁定

## 2. 测试证据

- **+6 测试**（mock 259 → **265**）: 词表+serde 锁 / **5×5 advance_to 全组合矩阵** /
  RAW 域唯一（含压缩域 serde 拒绝+全链携带不变）/ composition 事实位 / 终态判定 /
  **结构级 serde 往返 + Default==new() + 缺字段 fail-closed**
- **全回归零退化**: 矩阵 14/14; clippy 四组合零警; 硬件电池（lifecycle ALL PASS via
  gates bin / P1a 12 / P1b 11 / transport 19/0）

## 3. Review Gate（standard, subagent 全 change @35c98c7）

裁决 **With fixes**: 0 Critical / 4 Important / 4 Minor——全部处置:
- **Important#1（serde(default) 新生儿误用先例——LOCK FINAL 契约弱化）**: 已修——
  四字段 default 全移除; 缺字段 fail-closed 测试锁定（"unknown 值 fail 而 missing 字段
  静默过"的纪律不一致被抓住并修正）。
- **Important#2（白名单可被直构/serde 绕过, "唯一迁移入口"过度声明）**: 已修——信任边界
  在类型文档显式记档（有意的声明性设计 + A2-5 消费前重审义务）; 测试注释不再过度声明。
- **Important#3（无目标参数的 advance 使承诺的全组合矩阵不可测 + `{to}` 载荷死值）**:
  已修——`advance_to(target)` 落地, 5×5 矩阵真实可测, 载荷携带真实词表名。
- **Important#4（design doc 未提交——A2-1 Important#1 复发）**: 已修——本次产物随代码
  同 commit; **流程教训记档: 产物提交必须在代码 commit 时同步, 不留到 review 后**。
- Minor#5（proposal `CompressedMasterForbidden` 未实现）: 对账修正（类型层执行使运行时
  变体不可达——proposal 删除线记档）; #6（tasks 未勾）: 已勾; #7（结构级 serde/
  Default==new 测试缺）: 已补; #8（Debug 名 vs wire 名不一致）: 已修（`as_wire()` 统一）。

## 4. 冻结点

- 阶段词表 LOCK（§3.7 逐节点; serde 名 = wire 契约锚）
- RAW 域类型层唯一（Errata-3）; "Clean Master" 术语不存在
- advance_to 白名单无通配; 跳级/倒退/同阶段/终态后继 fail-closed
- 声明性 only: 无合成执行（A2-7+）/无 Audio/Metadata Master（A2-3/4）/无 Master Join（A2-5）

## 5. CI（PR 后回填）

七 required context: **见 PR 检查记录（合并前全 green 为 Merge Gate 前置）。**
