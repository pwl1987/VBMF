# Media Agent 状态机

> **STATUS: HISTORICAL REFERENCE — NOT CURRENT RUNTIME CONTRACT.**
> **SUPERSEDED BY:** [`PHASE_0_6_MASTER_PRD.md`](./PHASE_0_6_MASTER_PRD.md) / [`IMPLEMENTATION_ADDENDUM.md`](./IMPLEMENTATION_ADDENDUM.md) / [`MEDIA_BACKEND_CONTRACT.md`](./MEDIA_BACKEND_CONTRACT.md) / [`HARDWARE_PROVIDER_CONTRACT.md`](./HARDWARE_PROVIDER_CONTRACT.md)
> 本文件描述早期 skeleton 阶段（Gate 2.1 冻结接口、GStreamer 2.6 才挂载）状态机，与当前 Phase 0.6 四层架构不一致，仅作历史参考，AI 开发工具不得据此实现新代码。

**状态:** 已冻结契约(2026-08-26，历史)
**范围:** 媒体平面(`media-agent`,Rust)生命周期 —— Device × Lease × Supervisor。
**配套文档:** `MEDIA_RUNTIME_SECURITY_MODEL.md`(运行时/隔离),SoT §10(Gate A)。

> 本文档仅定义**状态契约**。skeleton 中不接入 GStreamer / DeckLink 代码。
> Gate 2.1 冻结接口;GStreamer 在 Gate 2.6+ 才挂载。

---

## 1. 状态

```
        INIT
         │  (启动, 加载配置)
         ▼
     DISCOVERING
         │  (通过 SDK + DesktopVideoHelper IPC 枚举 DeckLink 设备)
         │  ├─ 0 设备 ──────────────► DEGRADED
         │  └─ ≥1 设备 ─────────────► READY
         ▼
       READY
         │  (空闲, 设备可用, 无租约)
         │  (收到 HandleAcquire 请求)
         ▼
      LEASED
         │  (租约已授予; pipeline 尚未启动)
         │  (收到 HandleStart 请求)
         ▼
    CAPTURING
         │  (pipeline 在线, 帧在流动)
         │
         ├─ 设备丢失 / pipeline 错误 ──► DEGRADED
         ├─ 租约过期 / 被撤销 ──────► RECOVERING
         └─ 致命 / 不可恢复 ────────► FAILED
         ▼
     DEGRADED
         │  (瞬时故障; supervisor 尝试恢复)
         │  ├─ 已恢复 + 租约有效 ───► CAPTURING
         │  ├─ 租约无效/过期 ──────► RECOVERING
         │  └─ 恢复耗尽 ───────────► FAILED
         ▼
    RECOVERING
         │  (重新获取设备, 重新协商租约)
         │  ├─ 成功 + 租约有效 ──────► CAPTURING
         │  └─ 无法重建租约 ────────► READY (等待新租约)
         ▼
      FAILED
            (终态; 需运维 / 控制平面介入)
```

---

## 2. 状态表

| 状态 | 含义 | 租约 | Pipeline | Supervisor 动作 |
|---|---|---|---|---|
| `INIT` | 进程启动, 配置已加载 | 无 | 无 | → DISCOVERING |
| `DISCOVERING` | 枚举 DeckLink | 无 | 无 | 0→DEGRADED, ≥1→READY |
| `READY` | 空闲, 设备存在 | 无 | 无 | 等待 HandleAcquire |
| `LEASED` | 租约已授予, pipeline 空闲 | 有效 | 无 | 等待 HandleStart |
| `CAPTURING` | 帧在流动 | 有效 | 在线 | 监控健康 |
| `DEGRADED` | 瞬时故障 | 有效/可能 | 错误 | 尝试恢复 |
| `RECOVERING` | 重建设备+租约 | 重建中 | 无 | →CAPTURING 或 →READY |
| `FAILED` | 终态 | 不适用 | 不适用 | 等待外部重置 |

---

## 3. 关键不变量(本文档存在的原因)

> **DeckLink 掉线后重启 pipeline,在恢复采集前 MUST 重新校验租约有效性。**

本契约要防止的故障模式:

```
Pipeline start
   │
   ▼
DeckLink 丢失 (线缆拔出 / SDK 断开)
   │
   ▼
restart
   │   ← 错误: 盲目重启并继续采集
   ▼
租约还有效吗?   ← 若不问这个问题就是 bug
```

正确行为(由状态机强制):

1. DeckLink 掉线 → `CAPTURING → DEGRADED`。
2. Supervisor 尝试恢复。**在**重新进入 `CAPTURING` 之前,MUST 检查租约有效性:
   - 租约**有效** → 重建 pipeline → `CAPTURING`。
   - 租约**过期/被撤销** → `DEGRADED → RECOVERING → READY`(释放设备,
     等待新的 `HandleAcquire`)。绝不在无有效租约时采集。
3. 采集中过期的租约被视为 `RECOVERING`,而非静默继续。

这**不是** GStreamer 问题 —— 而是 Device + Lease + Supervisor 的交互问题。
状态机是唯一事实来源;GStreamer 只是叶子节点。

---

## 4. 接口冻结(Gate 2.1)

以下 trait 已在 skeleton(`src/*.rs`)中声明,MUST NOT 在未版本化决策的情况下
改变形状:

- `DeviceManager` — 枚举 / 状态(`DeviceState::{Unknown,Available,Leased,Error}`)
- `DeviceLease` — `acquire()` / `release()` / `renew()`
- `Pipeline` — `start()` / `stop()` / `restart()`  (restart 会重新校验租约)
- `Supervisor` — `health()` / `recover()`

**Gate 2.1 不接入 GStreamer。** skeleton 以惰性声明编译通过;真实设备/pipeline
逻辑在 Gate 2.2+ 落地。

---

## 5. 推进顺序(已冻结)

```
1. Rust skeleton          ✅ 完成 (编译通过, CI 绿)
2. Device discovery        ← 下一步
3. Lease manager
4. Health endpoint
5. Supervisor
6. GStreamer pipeline
7. First frame
```

First frame 故意排在最后:它依赖真实 SDI 输入 + 完整的 Device/Lease/Supervisor
状态机先就位。
