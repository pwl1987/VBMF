# A2-5-02 四件事终裁

- Change: `a2-5-master-join`
- Phase: A2-5-02
- Status: **CLOSED / ADMITTED TO A2-5-03**
- Production code: **0**
- Decision basis: A2-5-00 CLOSED + A2-5-01 CLOSED + V0.2 §1.20/§3.7/§3.8/§5/§8.9/§8.10 + existing Program/Runtime domain evidence

## 1. 四项终裁

### 1.1 MasterJoinResult 三值 + Result 矩阵

**裁定：APPROVED，采用三值：`ACCEPTABLE | DEGRADED | FAILED`。**

语义锁定：`FAILED` = **Program Join semantic failure**；不等于 Runtime HealthState、CommandStatus::Failed、SupervisorAction。

但原提案有一个必须修正的逻辑点：**Result 不能以 `readiness == Ready` 作为所有判定的前置门槛。**

原因：`video_failed/audio_failed` 是独立注入的 Runtime failure fact，而 failed 路径可能同时导致对应 Master 尚未处于 `MASTER_JOINED`，若先做 readiness gate，则 §1.20 的“任一路 failed → DEGRADED/FAILOVER”反而永远无法命中。

因此 A2-5-03 必须采用以下优先序：

1. **C′ semantic inconsistency**：`NotPresent ∧ ∃ Present fact` → `FAILED`，不受 readiness 限制；
2. **双媒体 failure**：`video_failed ∧ audio_failed` → `FAILED`；
3. **单媒体 failure**：`video_failed XOR audio_failed` → `DEGRADED`；
4. **无 failure，但三域未 Ready** → `result = None`，表示“当前联合结果尚不可判定”，绝不伪造 `DEGRADED` 或 `FAILED`；
5. **Ready 且无上述 failure/inconsistency** → `ACCEPTABLE`。

因此 Output 中 `result` **必须为 `Option<MasterJoinResult>`**，而不是无条件三值；`None` 是 Readiness 层语义，不是第四个 Result 枚举值。

### 1.2 AVSync 不直接改变 MasterJoinResult

**裁定：APPROVED。**

AVSync Classification 是 Join 的伴随输入/分类面，不新增“AVSync red → Degraded”快捷行。

V0.2 §8.10 的正确语义是：red 之后先 `classify_failure_domain`，再决定动作；PLAYER 不得机械 source-switch。因此把 `AVSyncClassification::Failed` 直接写成 `MasterJoinResult::Degraded` 会越过 failure-domain classification 边界，并可能把非节目源问题误判成节目 Master 故障。

Join 可以暴露 AVSync classification 供 Runtime/Safety 消费，但不承担 Recovery/FAILOVER。

### 1.3 AVSyncClassification 四值 + Join 零阈值

**裁定：APPROVED。**

闭集：`ACCEPTABLE | DEGRADED | FAILED | UNKNOWN`。

严格消歧：

- 不复用 `ClockObservationState` 的 offset/drift 语义；
- 不复制 `avsync_measurements` DB schema 为 Domain Object；
- AVSyncClassification 本身不携带 `offset_ms` / `drift_ms_per_min` 等 measurement 字段；
- Join 不执行 40/100/250ms、5ms/min 阈值计算。

§5 的阈值表归属 **AVSync Measurement / Correction / Classification 执行侧**。Join 接收已分类的结果，不成为 threshold engine。

### 1.4 MasterJoinOutput 三件分离 + failed fact 注入

**裁定：APPROVED，但按 1.1 的修正落地。**

`MasterJoinInput`：

- `video: VideoMaster`
- `audio: AudioMaster`
- `metadata: MetadataMaster`
- `avsync: AVSyncClassification`（非 Option；`UNKNOWN` 表达未测）
- `video_failed: bool`
- `audio_failed: bool`

failed flags 是 **Runtime failure facts 的显式参数注入**；Join 不读取 Runtime Snapshot、Event Projection、Health Tree，也不自行探测故障。

`MasterJoinOutput`：

- `eligibility: JoinEligibility`：每域 eligibility + 联合 readiness；
- `result: Option<MasterJoinResult>`：仅在 failure/inconsistency 或 Ready 条件满足时产生 Result；
- `classification_input: JoinClassificationInput`：伴随分类输入，零 action、零 recovery。

## 2. Eligibility / Readiness / Result 最终真值模型

### Eligibility

- Video：复用 `VideoMaster::is_program_scope_master()`；
- Audio：复用 `AudioMaster::is_program_scope_master()`；
- Metadata：`Participating | NotPresent` 均为 eligible；`Unknown` = 声明未完成，不是 failure。

### Readiness

`Ready = video_eligible ∧ audio_eligible ∧ metadata_eligible`。

Readiness **不进入 MasterJoinResult 枚举**。

### Result

| 优先级 | 条件 | Result |
|---|---|---|
| 1 | `NotPresent ∧ ∃ fact Present` | `FAILED` |
| 2 | `video_failed ∧ audio_failed` | `FAILED` |
| 3 | `video_failed XOR audio_failed` | `DEGRADED` |
| 4 | 无 failure/inconsistency 且 `!Ready` | `None` |
| 5 | 无 failure/inconsistency 且 `Ready` | `ACCEPTABLE` |

`NotPresent` 本身绝不触发降级；`Participating` 本身绝不提升为 Ready。

## 3. Projection Boundary

- `ACCEPTABLE`：未来 A2-6 ProgramMaster projection 消费；A2-5 不接 transport。
- `DEGRADED`：作为 §8.9 Master failure-domain input signal；Runtime/Safety/Watchdog 再决定 SAFE_DEGRADE/FAILOVER 等动作。
- `FAILED`：作为 §8.9 Master failure-domain input signal；Runtime/Safety 决定 Filler/Emergency/recovery。
- 任一 Result **不得直接映射 Channel Health**；Health Tree 独立聚合。
- 任一 Result **不得直接映射 SupervisorAction**。
- Join **不得执行 Recovery**。

## 4. A2-5-03 实现红线

1. 禁 `Master` common trait。
2. 禁 `Ready` 进入 `MasterJoinResult`。
3. 禁 `AVSync Failed → Join Degraded` 快捷规则。
4. 禁 Join 自取 Runtime/Health/Event 状态。
5. 禁复制 Clock offset/drift 观测结构。
6. 禁把 DB `avsync_measurements` 变成 Domain SoT。
7. 禁 Result → ChannelHealth 直推。
8. 禁 Result → SupervisorAction 直映射。
9. 禁 D14 `observation_revision` / timestamp 进入 Join 语义。
10. 禁 Timecode 从 `CanonicalMediaDescriptor`/Input Observation 搬入 MetadataMaster 或 MasterJoin。
11. C′ inconsistency 必须 fail-closed，且不得被 readiness gate 吞掉。
12. failed fact 必须能在 Master 未 Ready 时仍产生 `DEGRADED/FAILED`，否则违反 §1.20。

## 5. A2-5-03 准入结论

**APPROVED TO IMPLEMENT.**

下一刀仅允许建立 `master_join.rs` 的最小生产模型与纯函数矩阵测试；ProgramMaster composition root、transport/API projection、Runtime/Safety wiring 仍按既定边界留到后续阶段。
