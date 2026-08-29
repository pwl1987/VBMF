---
comet_change: p07b-clock-domain
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-30-p07b-clock-domain
status: final
---

# Design Doc — p07b-clock-domain（Phase 0.7B-2A: Clock Domain 建模）

> open design.md D1-D6 实现级细化。契约锚点：`CLOCK_TIMECODE_CONTRACT.md` §1（#147 冻结观测态词表；Observation≠Configuration/R3）；终审裁定形状（kind/reference/confidence/evidence）+ 三红线（无 choose_master_clock / 无 select_clock / 无 auto_route）。

## 1. `src/clock.rs` — 类型族（canonical 层，零 vendor 依赖，零方法）

```rust
/// Clock 观测态 — **冻结词表 #147** (Locked/Unlocked/Offset/Drift/ClockLost/ClockRecovered)
/// + Unknown (0.7B-2A 无 clock 探针的观测前置态; 真机 Unknown 合法)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockObservationState {
    Locked, Unlocked, Offset, Drift, ClockLost, ClockRecovered, Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockKind { Internal, External, Unknown }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockReference { FreeRunning, Locked, Unknown }

/// 置信语义: Observed(直接探针证据) / Inferred(transport/拓扑推断) / Unknown。
/// 0.7B-2A 只会产出 Unknown (无探针); 枚举为 0.7B 探针阶段预留。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockConfidence { Observed, Inferred, Unknown }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockEvidence { pub code: String, pub detail: String }

/// Canonical Clock Domain — **只描述观测, 绝不决策** (Observation≠Configuration)。
/// 类型族零决策方法: choose_master_clock / select_clock / auto_route 在本模块
/// 类型层面不存在 (终审红线); Clock 策略属 Runtime/Backend/Control Plane。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalClockDomain {
    pub id: Uuid,
    pub kind: ClockKind,
    pub reference: ClockReference,
    pub state: ClockObservationState,
    pub confidence: ClockConfidence,
    pub evidence: Vec<ClockEvidence>,
}
```

**白盒红线测试**（编译期+存在性）：`clock_public_surface_has_no_decision_apis` —— 断言模块公开 API 集不含 `choose|select|auto_route|master` 字样（遍历不便时退化为对 `grep` 等价的白盒断言：公开函数清单硬编码比对）。实现取**硬编码清单比对**（新公开项需显式更新清单——防新增决策方法静默通过）。

## 2. normalize.rs 联动（D4 最小增补）

```rust
pub struct CanonicalClockRef {
    pub domain: Option<Uuid>,
    /// 可携带 Domain 观测描述 (P0.7B-2A); normalize 恒 None + 既有 INFO 诊断不变。
    pub domain_description: Option<std::sync::Arc<CanonicalClockDomain>>,
}
```
- `Option<Box<_>>` vs `Arc<_>`：选 `Option<Arc<_>>`（跨会话共享更自然，且 Clone 廉价；Box 与 Arc 在此场景语义等价）。**不接探针**：`normalize_input` 恒 None + 既有 INFO 诊断不变。
- 0.7B-1 测试同步：`normalize_rt_01_bmd_loopback_shape...` 的 `assert_eq!(clock, CanonicalClockRef{domain:None})` → 需补 `domain_description: None`。

## 3. MEDIA-SEMANTICS-RT-01（Clock 部分，三层）

| 层 | 测试 |
|----|------|
| Unit | `clock_semantics_01_frozen_state_vocabulary_complete`（#147 六态+Unknown 全部可构造且 serde 往返）；`clock_semantics_01_public_surface_has_no_decision_apis`（硬编码公开清单比对）；`clock_semantics_01_unknown_kind_reference_state_legal`（Unknown 三处合法表达）；serde roundtrip |
| Simulation | MockProvider 世界：无 clock 探针 → 装配 `CanonicalClockDomain { kind: Unknown, reference: Unknown, state: Unknown, confidence: Unknown, evidence: [无 clock 探针] }`（Unknown 合法） |
| Hardware | 盒上证据输出（`VBMF_LOOPBACK` 路径 NORMALIZE-RT-01 段落追加 clock 段 JSON）：Unknown kind/state + evidence —— Unknown 合法（终审明确） |

## 4. 实施顺序

clock.rs 类型+白盒测试 → normalize.rs 增补+0.7B-1 测试同步 → Simulation/Hardware 证据 → 盒上全矩阵（首提交仅类型+serde+单测，不接 runtime）→ 真机 gate → CI。
