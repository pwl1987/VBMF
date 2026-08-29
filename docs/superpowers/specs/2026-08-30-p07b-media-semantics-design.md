---
comet_change: p07b-media-semantics
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-30-p07b-media-semantics
status: final
---

# Design Doc — p07b-media-semantics（Phase 0.7B-1: Normalize Foundation）

> open design.md D1-D6 的实现级细化。契约锚点：CANONICAL_MEDIA_MODEL §1/§2/§4/§5（全部 FROZEN）；终审三纪律（Normalize 不吞 Intent / Audio 独立 Flow / Clock 不被 Backend 偷走）。

## 1. `src/normalize.rs` — 类型（canonical 层，零 vendor 依赖）

```rust
/// canonical 帧率 (num/den; 观测 "30000/1001" 结构化)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalFrameRate { pub num: u32, pub den: u32 }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalVideoDescription {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<CanonicalFrameRate>,
    pub interlaced: Option<bool>,
    pub pixel_format: Option<String>,   // canonical 标签 (如 "v210"); 非 vendor 编解码枚举
}

/// AudioEmbedding — 契约 §4 五语义 (显式建模, 绝非 Video 附属)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioEmbedding { Embedded, DeEmbedded, Independent, Mixed, External }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioPresence { Present { channels_hint: Option<u32> }, NotPresent, Unknown }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalAudioDescription {
    pub presence: AudioPresence,
    pub embedding: AudioEmbedding,      // 0.7B-1: 观测到 audio ⇒ Embedded (SDI 现状显式化) + diagnostic
}

/// Clock 引用占位 (纪律③): 只引用 Domain, 绝不决策; 0.7B Clock 阶段实现策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalClockRef { pub domain: Option<Uuid> }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalSourceRef { pub device_id: Uuid, pub port_id: Option<Uuid> }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalMediaDescriptor {
    pub source: CanonicalSourceRef,
    pub transport: String,              // canonical 标签 (如 "sdi"); 不含 vendor 名
    pub video: CanonicalVideoDescription,
    pub audio: CanonicalAudioDescription,
    pub clock: CanonicalClockRef,
}
```

**字段取值原则（契约 §1）**：只填当前用到字段；观测缺失 → `None`/`Unknown` + diagnostic WARN（**不臆造、不丢观测**——diagnostics 保留证据）。**零 vendor 字段**（§5）：BMD handle/persistent_id 绝不出现（纪律：provider 身份已在 SPI 层 `ProviderIdentity`）。

## 2. RawInputDescription（D2 装配体）

```rust
pub struct ObservedVideo { pub width: Option<u32>, pub height: Option<u32>,
    pub frame_rate: Option<(u32, u32)>, pub interlaced: Option<bool>, pub pixel_format: Option<String> }
pub struct ObservedMedia { pub video: Option<ObservedVideo>, pub audio_present: Option<bool> }
pub struct RawInputDescription {
    pub device_id: Uuid,
    pub port_id: Option<Uuid>,
    pub transport: String,              // "sdi" 等 (来自 connector 标签)
    pub observed: Option<ObservedMedia>, // None = 未观测 (descriptor 全 Unknown + WARN)
}
```
装配入口：`impl RawInputDescription { pub fn from_port(port: &PortInfo) -> Self }`——从 `PortInfo`（device_id/connector/provider_binding_ref 不带出）+ `SignalStatus.video_format/audio_locked` 装配；**port.rs 类型不被替换**（runtime 探测层不动），仅转换。`frame_rate: Option<String> "30000/1001"` → 解析 `(30000, 1001)`，失败 → `None` + diagnostic（不丢观测）。

## 3. normalize_input（D3 纯函数）

```rust
pub struct NormalizeDiagnostic { pub level: DiagnosticLevel /*Warn|Info*/, pub code: String, pub detail: String }
pub struct NormalizeOutcome { pub descriptor: CanonicalMediaDescriptor, pub diagnostics: Vec<NormalizeDiagnostic> }
pub fn normalize_input(raw: &RawInputDescription) -> NormalizeOutcome
```
- 规则表：observed=None → video/audio 全 Unknown + WARN("未观测")；audio_present=Some(true) → presence=Present{channels_hint:None} + embedding=Embedded + INFO("SDI 内嵌现状显式化；MADI/AES 等 0.7B Audio Provider 声明")；audio_present=Some(false) → NotPresent；None → Unknown。
- clock：恒 `CanonicalClockRef { domain: None }` + INFO("clock 策略属 0.7B Clock 阶段")（纪律③）。
- **类型层面不可能**返回 pipeline/intent（返回类型不含此类）——纪律①由类型系统保证。
- 纯函数：无 IO、无锁、无全局；同输入恒同输出（NORMALIZE-RT-01 的前提）。

## 4. NORMALIZE-RT-01（D4 provider 无关性，三层）

| 层 | 测试 |
|----|------|
| Unit | `normalize_rt_01_provider_independent_same_media_same_descriptor`：BMD 形状 raw（v210/1080i50 家族 + embedded audio）与 Mock 形状 raw（同逻辑媒体、不同装配路径）→ descriptor **逐字段相等**；`normalize_rt_01_missing_observed_unknown_not_fabricated`（observed=None → 全 Unknown + WARN，绝不默认 1080i50）；frame_rate 解析失败 → None+WARN |
| Simulation | MockProvider 世界：PortInfo 装配 → from_port → normalize → descriptor 断言（audio=Embedded 显式化） |
| Hardware | 盒上真机：loopback 观测（720x486/30000÷1001/interlaced/v210/audio present）装配 → normalize → 断言与期望 canonical 形状一致（证据输出 JSON） |

## 5. main.rs 接线（最小）

`mod normalize;` 声明。真机挂接点：`VBMF_LOOPBACK` 路径在 fixture 验证后追加 descriptor 输出（观测→canonical 证据 JSON，`NORMALIZE-RT-01` 硬件证据载体）；**Session/Resource/Lease 路径零触碰**（纪律①：descriptor 是未来 Execution Plan 的输入，不反向影响现有 runtime）。

## 6. 测试与门禁矩阵

单测 ~6 个（类型/装配/规则/无关性/缺失观测/解析失败）+ Simulation 1 + Hardware 1（并入真机 gate 脚本输出）。盒上全矩阵（fmt/test×4/clippy×4/build×3/PROOF）不回退；CI 七 checks 不回退（normalize 测试入 rust-test-matrix 与 session-lifecycle 均自动覆盖——session:: 过滤器不含 normalize，故 rust-test-matrix 的 `cargo test`（default 含 normalize 测试）已覆盖）。

## 7. 债务衔接

D4（PortAvailability 精确化）依赖本模块的端口级描述能力，本 change 交付 `CanonicalSourceRef.port_id` 字段为其铺路；D2（derive_claims FAIL 化）与本模块的"Unknown+WARN 不决策"原则互补（决策属 Preflight/Policy 层）。Normalize 阶段（0.7B 主体）将在本 Foundation 上加转换规则与 NORMALIZE-RT-01 扩展场景。
