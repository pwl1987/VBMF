# A2-5-04 — ProgramMaster Composition Root + AVSync Boundary（Shape/Consumer Probe + 提案）

> Status: `PROBE + PROPOSAL / NO CODE CHANGE（本文件）`
> Authority: A2-5-03 终裁（首刀 APPROVED 不返工; 04 先 Probe 再实现）
> Date: 2026-09-02 · Base: `871a646`

## 1. 组合先例实查（仓库唯一纪律源）

- **`CanonicalRuntimeState`**（runtime_state.rs L142-155）：顶层组合根 = 整值
  字段家族（devices/ports/resources/sessions/media_semantics: Vec<*RuntimeState>）
  ——**组合不展平**；D14 字段带 serde(default) 属 **additive 先例**（旧实例
  加字段），**不适用**于 ProgramMaster（新生儿零旧实例——A2-2 立规禁）。
- **`PortMediaSemantics`**（L103）："**整值组合, 绝不平铺——终审加严红线**"。
- 消费者面：`ProgramMaster` 域外引用 = **零**（transport 五端点冻结; A2-6
  ProgramMaster projection 是首个未来消费者）。

## 2. ProgramMaster 形态提案（唯一候选——组合先例唯一）

```rust
pub struct ProgramMaster {
    pub video: VideoMaster,
    pub audio: AudioMaster,
    pub metadata: MetadataMaster,
    pub join_result: Option<MasterJoinResult>,
}
```

| 设计点 | 提案 | 依据 |
|---|---|---|
| 组合成员 | 三 Master 按值 + `join_result: Option<MasterJoinResult>` | OQ-B 终裁字面（"三 Master + MasterJoinResult"）; None 语义同 02 终裁 |
| **不含** eligibility / classification_input | Join 判定**过程产物**非 Program 终态成员; A2-6 需要时投影另行组合 | 防 ProgramMaster 吞 Output 全件成 God Object |
| **不含** AVSyncClassification | 见 §3 双 SoT 禁 | 消歧红线 |
| **不含** stage/advance/时间/健康/action | 同 MetadataMaster 纪律（组合根非 pipeline） | OQ-B 终裁"非第四 Stage Pipeline" |
| 构造 | `compose(video, audio, metadata, join_result)` 纯函数**唯一入口**; 不做 from_join sugar（等真实消费者=A2-6 反推——用户"从消费者反推"原则） | 03 终裁 inconsistency 深化同精神 |
| Default | derive（三 Master Default + None = 冷启动未判定组合） | MetadataMaster Default 先例 |
| serde | derive + **零 serde(default)** + 键集恰四锁 + 缺字段 fail-closed | A2-2 立规; 家族一致性（三 Master 全 serde） |
| PartialEq | only（AudioMaster f32 同律） | A2-3/A2-5-03 先例 |
| 测试 | 键集恰四（**"绝不平铺"的 wire 级锁**——video_stage 等展平键即红）/ 缺字段 fail-closed / compose 恒等 / Default 语义 / Result None 三态携带 | A2-4 家族纪律 |

## 3. AVSyncClassification 层级钉死（提案）

- **唯一家 = `master_join.rs`**（Join 伴随输入分类; 03 已落）;
- **ProgramMaster 零持有**（双 SoT 禁——Join 输入分类若再入组合根即两处
  真相）;
- 数据流单向：A2-7 执行面 measurement → 上游分类 → `AVSyncClassification`
  → `MasterJoinInput.avsync` → `JoinClassificationInput.avsync` 透传 →
  Runtime/Safety（A2-6+）;
- **不挪不移不加字段**（03 终裁: inconsistency 深化留 05, 同律适用 AVSync）。

## 4. 04 实现范围（提案——待裁后执行）

仅 `src/program/program_master.rs`（新文件, ProgramMaster + compose + 测试
约 4-5 个）+ `mod.rs` 挂载。零 transport/Runtime/三 Master 改动。

## 5. No-Build Gate

本文件零 .rs diff; 提案未裁不编码; 不实现 from_join/投影/AVSync 挪动。
