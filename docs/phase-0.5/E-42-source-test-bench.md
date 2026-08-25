# E-42 · Source Test Bench — Spec

> **状态**: 🟡 **0.5F 增补 Spec（P1，锚定 0.5E 实施）**
> **来源**: 0.5C 提案 §25
> **定位**: Source Manager / Add Source 向导的"入网验证闸门"
> **与 E-40 关系**: E-40 是**运行期** Network Diagnostics；E-42 是**入网前**一次性验证。二者共享探针，职责不同。
> **关联**: E-40 Network Source · `02-sources.html`（Source Manager）· PIA §2/§4（Source 6 字段）· QC Engine

---

## 0. 触发与位置

- **自动**: Add Source 向导末步（PIA §4.2 第 7 步 Preview 之后、Save 之前）运行
- **手动**: Source 详情页 "Test Bench" 按钮
- **约束**: 未 `VERIFIED` 的 Source 可 Save 但标记 `UNVERIFIED`，**不能**用于 ON AIR Channel（PIA §4 Source Capability 据此声明各 Switch Mode 的 capability 资格; 最终 Available Switch Mode 由 Runtime Alignment + Canonical Decision Tree 推导）

---

## 1. 分层验证（7 层，自下而上）

| # | 层 | 检查项 | PASS 标准 | 失败级别 |
|---|---|---|---|---|
| 1 | **Network** | Interface / Route / IGMP Join（网络源） | 网卡 UP · 路由可达 · 组播已 JOIN | 🔴 阻断 Save |
| 2 | **Transport** | Packet receive | 收到数据包 · 无连续丢包 | 🔴 阻断 |
| 3 | **Container** | MPEG-TS / RTP 解析 | 容器头合法 · PCR 连续 | 🔴 阻断 |
| 4 | **Video** | Codec / Resolution / FPS 探测 | H264/HEVC + 分辨率/帧率匹配预期 | 🟡 Warning（可 Save UNVERIFIED） |
| 5 | **Audio** | Codec / Sample Rate | AAC/Opus + 48k 探测到 | 🟡 Warning |
| 6 | **Clock** | PTS stable / Timecode | PTS 漂移 < 阈值 · 时间码稳定 | 🟡 Warning |
| 7 | **QC** | No freeze / No black / No silence | 连续 N 秒无冻结/黑场/静音 | 🔴 阻断（安全优先） |

---

## 2. 结果状态

| 状态 | 含义 | 可 Save? | 可 ON AIR? |
|---|---|---|---|
| `VERIFIED` | 7 层全 PASS | ✅ ACTIVE 候选 | ✅ |
| `UNVERIFIED` | 部分 Warning 层未 PASS | ✅ 但受限 | ❌ |
| `FAILED` | 阻断层 FAIL | ❌ 禁止 Save | ❌ |

> 失败层显示**原因 + 建议动作**（如 "IGMP Join 失败 → 检查 SSM Source IP / VLAN"），对齐 E-40 Diagnostics 表达。

---

## 3. 与 4-Layer / E-40 关系

- E-42 输出作为 Source 的 `capability` 输入（PIA §4）：声明各 Switch Mode 的 capability 资格（`PACKET_SWITCH` eligible / `FRAME_SWITCH` common RAW / `MASTER_SWITCH` normalize）, **不直接决定** Available Switch Mode; 最终由 Runtime Alignment + Canonical Switch Mode Decision Tree（PACKET→FRAME→MASTER→REJECT, V0.2 §3.4）推导
- E-40 持续诊断复用同一组探针（Network Reachability / Quality Metrics），但 E-42 是入网一次性验证

---

## 4. 0.5E 实施锚点

1. Add Source 向导集成 E-42 为末步（Preview → **Test Bench** → Save）
2. 探针复用 E-40 Diagnostics 实现，避免重复
3. `UNVERIFIED` Source 在 02 Sources 列表明确标注，禁止拖入 ON AIR Channel

⛔ 仅为验证流程增补，不新增 Engine；与 E-40 共享 Source 配置对象（PIA §3）。
