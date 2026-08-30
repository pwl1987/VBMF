---
comet_change: p07b-timecode-foundation
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-30-p07b-timecode-foundation
status: final
---

# Design Doc — p07b-timecode-foundation（Phase 0.7B-2C: Timecode Foundation）

> open design.md D1-D8 实现级细化。契约锚点：`CLOCK_TIMECODE_CONTRACT.md` §2（#148 冻结词表）/§3（替换不变量）；终审红线（时间标签非时间本体；不实现 parser；Clock/Timecode 概念隔离）。

## 1. `src/timecode.rs` — 类型族（canonical 层，零 vendor 依赖）

```rust
/// Timecode 状态 — **#148 冻结词表** + Unknown (无观测源前置态; 真机合法)。
/// 注意: Discontinuous/Recovered 是**观察事实**, 不是修正/恢复动作 (D5)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimecodePresence {
    Present, Absent, Invalid, Discontinuous, Recovered, Unknown,
}

/// 格式标签 — **只声明, 不解析** (终审最小集; ATC/SMPTE 等扩充留后续)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimecodeFormat { Ltc, Vitc, Embedded, Unknown }

/// 时间标签值 — 仅 presence=Present 且有真实观测时携带;
/// **无观测绝不臆造 00:00:00:00** (D3; 越界校验属 parser 阶段, 本阶段无解析器)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimecodeValue { pub hours: u32, pub minutes: u32, pub seconds: u32, pub frames: u32 }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimecodeEvidence { pub code: String, pub detail: String }

/// Canonical Timecode — **时间标签, 非时间本体**。
/// `frame_rate` = 标签所属媒体帧率 (语义上 ≠ Clock 的 rate — D6 概念隔离)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalTimecode {
    pub presence: TimecodePresence,
    pub format: TimecodeFormat,
    pub value: Option<TimecodeValue>,
    pub frame_rate: Option<(u32, u32)>,
    pub evidence: Vec<TimecodeEvidence>,
}
```

构造器（类型族仅有的 inherent 方法；公开面 allowlist 白盒锁定）：
- `unknown()`：presence/format Unknown + value None + evidence `no_timecode_observation`。
- `absent()`：presence Absent + evidence（观测到"无 timecode"是合法观察事实）。
- `observe_invalid(code, detail)`：presence Invalid + evidence——**异常绝不悄悄转合法**（D4）。
- `observe(value, format, frame_rate)`：presence Present（唯一携带 value 的路径）。
- `observe_transitional(presence: Discontinuous|Recovered, code, detail)`：观察事实路径（无修正动作）。

## 2. Clock/Timecode 隔离（D6+D7）

- 类型层面：`timecode.rs` 不 import `clock.rs`（反之亦然）——serde JSON 互不含对方字段（隔离测试反向断言 `clock`/`timecode` 字样不串）。
- 公开面 allowlist：`["unknown","absent","observe_invalid","observe","observe_transitional"]`——防 `select_clock`/`sync`/`resample`/`correct` 类 API 静默进入（同 0.7B-2A 先例）。
- **无任何路径**使 Timecode 变更影响 `CanonicalClockDomain`（无引用 = 无影响；测试以 serde 隔离断言固化）。

## 3. normalize.rs 联动（D8）

`CanonicalMediaDescriptor` 增 `timecode: CanonicalTimecode`（平级，四基础齐备）；`normalize_input` 恒 `CanonicalTimecode::unknown()`（无观测源，与 clock domain_description 恒 None 同界）；0.7B-1/2A/2B 既有测试的 descriptor 构造/断言点机械同步。

## 4. TIMECODE-SEMANTICS-RT-01（三层）

| 层 | 测试 |
|----|------|
| Unit | 词表快照（#148 五态+Unknown serde 往返+字符串形态）；隔离（公开面 allowlist + serde 零 clock/master/sync 字样 + timecode/clock JSON 互不含对方）；Unknown/Absent 不臆造（value=None，无 00:00:00:00）；Invalid 保证据（observe_invalid 不产合法值）；Discontinuous/Recovered 观察事实（无修正路径——allowlist 锁定）；Vendor independence（相同 canonical observation → 相同 CanonicalTimecode） |
| Simulation | Mock observation → canonical timecode（unknown 装配） |
| Hardware | 真机 loopback timecode 段证据输出（Unknown 合法——只证明"能观察/描述"） |

## 5. 实施顺序

timecode.rs 类型+测试 → normalize.rs 联动+既有测试同步 → main.rs mod+loopback 挂点 → 盒上全矩阵（首提交仅类型+serde+测试）→ 真机 gate → CI → merge 后**Consolidation Review**（不直接进 0.7C——终审裁定）。
