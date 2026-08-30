---
comet_change: p07b-audio-semantics
role: technical-design
canonical_spec: openspec
archived-with: 2026-08-30-p07b-audio-semantics
status: final
---

# Design Doc — p07b-audio-semantics（Phase 0.7B-2B: Audio Semantic Model）

> open design.md D1-D7 实现级细化。契约锚点：`CANONICAL_MEDIA_MODEL.md` §4（Audio 独立建模/第 9 替换轴）；终审裁定词表与三红线（无 Mixer/BusManager/ChannelAllocator/Gain/DelayCompensation；Unknown 贯穿禁止默认 Program；Intent 绝不产 pipeline）。

## 1. `src/audio.rs` — 类型族（canonical 层，零 vendor 依赖）

```rust
pub struct AudioStreamId(pub Uuid);

/// 语义角色 — 冻结词表。Main/Backup/Emergency/TX/RX 是业务词汇, 冻结禁止。
/// 默认 Unknown, 绝不默认 Program (与 0.7A absence≠evidence 同构)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioRole { Program, Commentary, Ambient, Auxiliary, Unknown }

/// 布局 — 只描述 (Mono/Stereo/FiveOne/SevenOne/Unknown); Downmix/Upmix 是处理, 禁止。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioLayout { Mono, Stereo, FiveOne, SevenOne, Unknown }

/// 采样格式最小集 (0.7B-2B 无观测源 → 恒 None/Unknown)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSampleFormat { PcmS16, PcmS24, PcmS32, PcmFloat, Unknown }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationEvidence { pub code: String, pub detail: String }

/// Canonical Audio Stream — 回答"这个音频是什么" (不是"应该怎么处理")。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalAudioStream {
    pub id: AudioStreamId,
    pub presence: crate::normalize::AudioPresence,   // 复用既有三态 (Present/NotPresent/Unknown)
    pub channels: Option<u32>,
    pub sample_rate: Option<u32>,
    pub sample_format: Option<AudioSampleFormat>,
    pub layout: AudioLayout,
    pub role: AudioRole,
    pub evidence: Vec<ObservationEvidence>,
}

impl CanonicalAudioStream {
    /// Unknown 贯穿构造器: presence/role/layout 全 Unknown + evidence "no_audio_observation"。
    pub fn unknown(id: AudioStreamId) -> Self;
    /// Normalize → Stream 桥 (D5 纯映射): presence 直映; channels/sample_rate/format → None; role/layout → Unknown。
    pub fn from_description(id: AudioStreamId, desc: &crate::normalize::CanonicalAudioDescription) -> Self;
}
```

## 2. AudioRouteIntent（D4 Semantic Intent）

```rust
pub enum AudioSemanticTarget { Role(AudioRole), Named(String) }  // Named = opaque 语义标签 (非路由配置)
pub struct RoutePolicy { pub required: bool }                    // mix/duck/switch 属后续 Runtime/Control Plane
pub struct AudioRouteIntent {
    pub source: AudioStreamId,
    pub destination: AudioSemanticTarget,
    pub policy: RoutePolicy,
}
```
**类型层面不可能**产出 pipeline/backend/gst 引用（返回/字段类型不含此类——纪律①同构：Intent → (未来) Execution Plan → Backend）。语义映射示例（仅文档）：Camera A Embedded Audio → Role(Program)。

## 3. AUDIO-SEMANTICS-RT-01（三层）

| 层 | 测试 |
|----|------|
| Unit | `audio_semantics_rt_01_provider_independent`（BMD 形状 vs Mock 形状 `CanonicalAudioDescription` → `from_description` 产出媒体语义相同的 stream）；`audio_semantics_rt_01_unknown贯穿`（无观测 → `unknown()` → presence=Unknown + role=Unknown，**绝不默认 Program**）；`audio_semantics_rt_01_route_no_pipeline_refs`（A/B streams → intent → serde JSON 反向断言零 `gst/pipeline/backend` 字样）；Role 词表快照（防静默增删） |
| Simulation | MockProvider 世界装配 audio stream（presence 由装配推导） |
| Hardware | 真机 loopback：audio stream 证据输出（presence=Present / role=Unknown / channels/sample_rate=None — Unknown 合法） |

## 4. 实施顺序

audio.rs 类型+测试 → normalize.rs 桥 → main.rs mod + loopback 挂点 → 债务登记 D11/D12 → 盒上全矩阵（首提交仅类型+serde+测试，不接 runtime）→ 真机 gate → CI。

## 5. 禁改清单核验

session.rs / resource.rs / lease.rs / pipeline.rs / backend.rs 五文件 `git diff` 零触碰（verify 阶段 `git diff --stat` 断言）。触碰即范围越界——回退。
