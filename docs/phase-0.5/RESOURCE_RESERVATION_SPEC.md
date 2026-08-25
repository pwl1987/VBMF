# VBMF Resource Reservation Spec (V0.2 · 0.5D.3 Semantic Locked)

> **目的:** 把 Resource Reservation 从"Preflight 预览数值"升级为**正式语义对象**, 焊死 Reservation / Quota / Acquire / Release / HOT 生命周期, 使实时系统(Realtime Encode / HOT Standby)的资源预算不再是 UI 数字。
>
> **关联:** `ENCODE_MODEL_SPEC.md` (REALTIME_PROFILE.resource_reservation=REQUIRED) · `B-13-take-preflight.md` (Resource 三档预检 + TAKE 门禁) · `ARCHITECTURE_V0.2.md` (H2 Resource Scheduler) · `OBJECT_VOCABULARY.md` (§1.15 Reservation)
>
> **状态:** 🟢 **SEMANTIC LOCKED V0.2** (0.5D.3) · `implementation_authority: 本 Spec` · `wireframe_status: TODO (0.5E)` — 语义已锁, 不再用 "DRAFT"; UI 落地前只允许语义级修改。

---

## 1. 问题

- 现状: Resource 只是 Preflight 数值 (≤80% PASS / 80-100% 仅 reservation 满足可放行 / >100% BLOCK), **没有 Reservation 对象**, 无法回答"HOT 备机到底锁没锁 CPU/编码器/端口"。
- 后果: HOT Standby 的资源预算仍是 UI 数字; 跨 Channel 仲裁无对象载体; 释放时机无模型。

## 2. Reservation 对象 (正式定义)

| 字段 | 类型 | 说明 |
|---|---|---|
| `reservation_id` | UUID | 主键 |
| `target_channel` | ref | 预留给哪个 Channel |
| `target_session` | ref | 预留给哪个 Session (MEDIA_SESSION / OUTPUT_SESSION) |
| `resource_vector` | map | **= V0.2 §3.11 ResourceVector (9-dim, 不得另建简化模型)**: `cpu_threads / gpu_sessions / vram_mb / ram_mb / ingress_mbps / egress_mbps / disk_write_mbps / pcie_rx_mb_s / pcie_tx_mb_s` |
| `device_tokens` | list | `{BMD_IN/OUT token, NIC port, encoder_slot}` — 独占性约束 (Exclusivity Constraint) |
| `constraints` | list | 独占约束 / 亲和约束 (同 HOST / NUMA) |
| `scope` | enum | `HOT / WARM / COLD / TRANSIENT` (对应 Hot-Standby 3 级 + 一次性) |
| `priority` | enum | `CRITICAL / HIGH / NORMAL / LOW` |
| `state` | enum | `PROVISIONED / RESERVED / IN_USE / RELEASED` |
| `acquired_at` | datetime | RESERVED 达成时间 |
| `released_at` | datetime | RELEASED 时间 |
| `owner` | ref | 创建者 (Operator/Engineer/System) |
| `quota_id` | ref | 关联的 Quota 对象 |

## 3. 生命周期

```text
PROVISIONED  →  RESERVED  →  IN_USE  →  RELEASED
   (预算已算)     (资源已锁)   (正在使用)   (释放回池)

抢占/释放路径 (0.5D.3 安全语义 — 禁止直接写 FAILED):
RESERVED/IN_USE → PREEMPT_PENDING → DRAINING → RELEASED
                                              │ 仅无法安全释放时
                                              ↓
                                RESOURCE_CONFLICT → Safety Decision
                                (Degrade / Stop / Reject — 由 Failure Domain §8.9 决策)
```

- **PROVISIONED**: Preflight 算账完成, 尚未锁资源。→ 对应 B-13 Resource 三档 PASS/reservation 段。
- **RESERVED**: 调度器**实际锁定** resource_vector (编码器插槽 / BMD 设备 token / NIC 端口 / CPU 配额), 其他 Channel 无法抢占。
- **IN_USE**: Session 启动后资源正式占用。
- **RELEASED**: Session 停止 / Channel 停播 / 手动释放, 资源回池, 触发其他 PENDING 仲裁。

## 4. Quota 与仲裁

- **Quota**: 每 HOST / 每 Device 的容量上限 (与 E-36 Resource / E-38 Hardware 联动), 例如 `BMD DeckLink Duo 2: 2×IN + 2×OUT`、`CPU 32 核`。
- **仲裁规则** (跨 Channel):
  1. `CRITICAL` > `HIGH` > `NORMAL` > `LOW`。
  2. 同级: 先到先得 (acquired_at)。
  3. 抢占仅允许 `WARM/COLD → HOT` 升级需求且被占方为 `LOW/NORMAL`; 被抢占方走 **PREEMPT_PENDING → DRAINING → RELEASED** 有序释放 (对齐 V0.2 §8.9 `RESOURCE → Degrade background jobs`), 调度器**不直接写 FAILED**; 仅当无法安全释放时才进入 `RESOURCE_CONFLICT` → Safety Decision (Degrade / Stop / Reject)。
  4. 同 priority 时, `HOT` 备机保留量不参与普通复用 (防"备机预算被偷走")。

## 5. HOT Standby 语义 (关键)

- **HOT ≠ reservation=yes 的 UI 勾选。** HOT 必须满足:
  1. `reservation.state = RESERVED` (备机侧资源已实际锁定);
  2. `reservation.scope = HOT`, 锁定完整 resource_vector (编码器 + 输入端口 + 输出 token + CPU 配额);
  3. 备机 Session 处于 `warm/hot` 预启动状态 (视 HOT 级别), 切换无需重新 Acquire。
- **释放时机**: 主备切换完成 / Channel 退役 / 运维显式释放 → `RELEASED`, 释放后立即触发 PENDING 仲裁。

## 6. Preflight 联动 (B-13 第 9 项 Resource 预检)

| 三档 | Preflight 判定 | Reservation 语义 |
|---|---|---|
| ≤80% | PASS | 直接创建 RESERVED (资源充足) |
| 80-100% | 仅 reservation 满足可放行 | 无空闲 → 尝试抢占 (见 §4.3, 仅在 PREPROVISION/RESERVE 阶段); 否则 WARN |
| >100% | BLOCK | 无 Reservation 可达成, TAKE 阻断 |

### 6.1 TAKE 门禁 (0.5D.3 P1-5 — TAKE 不触发资源抢占)

```text
CONFIGURE → PRELIGHT/PREVIEW → PREPROVISION → RESERVE → READY_TO_TAKE → TAKE
```

- **抢占只发生在 PREPROVISION/RESERVE 阶段**, 绝不由 TAKE 操作临时触发。
- **TAKE 只验证 `reservation.state == RESERVED`** (或 HOT 的备机副本 RESERVED)。
- 未 RESERVED → **TAKE BLOCKED**: `Reason: Resource Reservation NOT_READY` / `Action: Open Resource Impact → Provision`。
- 这保证"按下 TAKE"永远不是资源仲裁点 — 播出安全不依赖抢资源的时延。

## 7. 数据流示例 (Realtime Encode + HOT)

```text
1. D1 向导资源预览 (Step 6) → 生成 PROVISIONED 预算 (9-dim ResourceVector + device_tokens)
2. 提交 ChangeSet → Review/Approve → Apply
3. Runtime Provision → H2 Scheduler Acquire: PROVISIONED → RESERVED (锁 9-dim vector + BMD/NIC token)
4. Session 启动: RESERVED → IN_USE
5. HOT 备机: scope=HOT, 独立 RESERVED, 主备共享同一 resource_vector 的两个副本
6. 主 Session 停止 → RELEASED → 触发仲裁
```

---

**VBMF Contributors** · Resource Reservation Spec V0.1 · Phase 0.5D.1 Semantic Closure
