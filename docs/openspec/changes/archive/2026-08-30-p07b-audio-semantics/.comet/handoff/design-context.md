# Comet Design Handoff

- Change: p07b-audio-semantics
- Phase: design
- Mode: compact
- Context hash: e68c1f94b35713d97275e96b5a739cde6ac1bc8abe55cbf7b1a7915c2d4f9462

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07b-audio-semantics/proposal.md

- Source: docs/openspec/changes/p07b-audio-semantics/proposal.md
- Lines: 1-36
- SHA256: 9a48fc17d6099f37d4058098a15c0256ec49e9c2309d251b56483e21010d70eb

```md
# Change: Phase 0.7B-2B — p07b-audio-semantics（Audio Semantic Model：Canonical Audio Semantics，非 Audio Engine）

## Why

0.7B-1 的 `CanonicalAudioDescription` 只回答"有没有音频"；音频会快速滑向 mixer/routing/bus/channel mapping/DSP——必须先冻结 **Canonical Audio Semantics**（"这个音频是什么"），把"应该怎么处理"留给后续 Runtime/Control Plane。契约 §4 已显式建模 Audio 独立性（第 9 替换轴）；本 change 落成类型。**不实现**：GStreamer clock / ALSA/JACK/PipeWire / SDI embedded extraction / Audio mixer / Frame sync / 实时同步算法。

## What Changes

- **`src/audio.rs`（新，canonical 层，零 vendor 依赖）**：
  - `AudioStreamId(Uuid)`。
  - `AudioRole { Program, Commentary, Ambient, Auxiliary, Unknown }` —— **语义角色**；Main/Backup/Emergency/TX/RX 是业务词汇，冻结禁止。
  - `AudioLayout { Mono, Stereo, FiveOne, SevenOne, Unknown }` —— 只描述；Downmix/Upmix/Normalize 是处理，禁止。
  - `AudioSampleFormat { PcmS16, PcmS24, PcmS32, PcmFloat, Unknown }`（最小集；0.7B-2B 无观测源，恒 None/Unknown）。
  - `ObservationEvidence { code, detail }`（观测证据条目；与 clock 的 ClockEvidence 形状统一化留后续统一登记）。
  - `CanonicalAudioStream { id: AudioStreamId, presence: AudioPresence(复用 normalize 既有三态 Present/NotPresent/Unknown), channels: Option<u32>, sample_rate: Option<u32>, sample_format: Option<AudioSampleFormat>, layout: AudioLayout, role: AudioRole, evidence: Vec<ObservationEvidence> }`。
  - `CanonicalAudioStream::unknown(id)`：presence=Unknown / role=Unknown —— **Unknown 贯穿，禁止默认 Program**（终审 Unknown 测试要求）。
  - `AudioSemanticTarget { Role(AudioRole), Named(String) }` + `RoutePolicy { required: bool }`（policy 词汇 mix/duck/switch 属后续）+ `AudioRouteIntent { source: AudioStreamId, destination: AudioSemanticTarget, policy: RoutePolicy }` —— **Semantic Intent**：从 `CanonicalAudioStream` 语义映射（如 Embedded 相机音 → Program），类型层面不可能产出 pipeline/backend/gst 引用（纪律①同构）。
  - `CanonicalAudioStream::from_description(id, &CanonicalAudioDescription)`：Normalize → Stream 桥（presence 直映；channels/sample_rate 0.7B-2B 无观测源 → None；role=Unknown——**绝不默认 Program**）。
- **`normalize.rs` 联动（最小）**：桥接函数消费既有 `CanonicalAudioDescription`（不动 `normalize_input` 返回类型与既有诊断）。
- **`main.rs`（最小）**：`mod audio;` + loopback 证据挂点扩展（audio stream 证据输出——Hardware 层载体；同 0.7A/0.7B-2A 挂点先例，仅诊断路径）。
- **门禁 AUDIO-SEMANTICS-RT-01（三层）**：
  - Unit：provider 无关性（BMD 形状 vs Mock 形状 audio description → 同一 CanonicalAudioStream 媒体语义）；**Unknown 测试**（无观测 → presence=Unknown + role=Unknown，绝不默认 Program）；Route 测试（streams A/B → AudioRouteIntent，**零 pipeline/backend/gst 引用**——serde 反向断言）。
  - Simulation：MockProvider 世界装配。
  - Hardware：真机 loopback audio stream 证据输出（如 presence=Present / role=Unknown；channels/sample_rate Unknown 合法）。
- **CI**：测试并入现有矩阵。
- **债务登记（随本 PR）**：D11 Clock Observation Timeline（时钟是事件流非静态属性）；D12 Clock Confidence 的 ObservationSource 细分。

## Capabilities

（`skip_specs: true`——SoT 为 CANONICAL_MEDIA_MODEL §4 + 终审裁定形状。）

## Impact

- 编译：五套 feature 不回退；零 vendor 依赖。
- 受影响：新 `audio.rs`；`normalize.rs`（桥接函数）；`main.rs`（mod + loopback 挂点）；`PHASE_0_7A_POST_MERGE_DEBT.md`（D11/D12 登记）。
- 明确不做：AudioMixer / AudioBusManager / ChannelAllocator / Gain / Delay Compensation / sample clock / SDI embedded extraction / 任何 pipeline 生成；**不触碰 session.rs / resource.rs / lease.rs / pipeline.rs / backend.rs**（触碰即范围越界）。

```

## docs/openspec/changes/p07b-audio-semantics/design.md

- Source: docs/openspec/changes/p07b-audio-semantics/design.md
- Lines: 1-29
- SHA256: 1f741d8329b3ac442772a4c0ea36494faa8f414779635077486bc74f78dcb0ff

```md
# Design: Phase 0.7B-2B — p07b-audio-semantics（Audio Semantic Model）

## Context

冻结契约 `CANONICAL_MEDIA_MODEL.md` §4：Audio 独立建模（第 9 替换轴；AudioSource/AudioPort/AudioRoute/AudioChannel/AudioFormat/AudioClock；语义 Embedded/De-embedded/Independent/Mixed/External）。终审裁定：只回答"这个音频是什么"，不回答"应该怎么处理"；Unknown 贯穿；禁止 AudioMixer/AudioBusManager/ChannelAllocator/Gain/DelayCompensation。

## Goals / Non-Goals

**Goals:** `CanonicalAudioStream` + `AudioRole`（Program/Commentary/Ambient/Auxiliary/Unknown）+ `AudioLayout`（Mono/Stereo/FiveOne/SevenOne/Unknown）+ `AudioSampleFormat`（最小集）+ `AudioRouteIntent`（Semantic Intent）+ `ObservationEvidence`；AUDIO-SEMANTICS-RT-01 三层。
**Non-Goals:** mixer/bus/channel allocator/gain/delay/sample clock；GStreamer/ALSA 接线；Session 语义变更；Timecode（0.7B-2C）。

## Decisions

- **D1 Role 冻结词表**：`Program | Commentary | Ambient | Auxiliary | Unknown`。Main/Backup/Emergency/TX/RX 是业务词汇——冻结禁止。**默认 Unknown，绝不默认 Program**（与 0.7A "absence≠evidence" 同构）。
- **D2 presence 复用**：`AudioPresence` 复用 `normalize.rs` 既有三态（Present{channels_hint}/NotPresent/Unknown）——终审"保持 Present/Absent/Unknown 不要改"；Absent≡NotPresent（既有命名，测试已锁定，不重命名制造 churn）。
- **D3 `CanonicalAudioStream`**：`{ id: AudioStreamId, presence, channels: Option<u32>, sample_rate: Option<u32>, sample_format: Option<AudioSampleFormat>, layout: AudioLayout, role: AudioRole, evidence: Vec<ObservationEvidence> }`。`unknown(id)` 构造器：presence/role/layout 全 Unknown + evidence 记录"无 audio 观测"。
- **D4 `AudioRouteIntent` = Semantic Intent**：`{ source: AudioStreamId, destination: AudioSemanticTarget, policy: RoutePolicy }`；`AudioSemanticTarget { Role(AudioRole), Named(String) }`；`RoutePolicy { required: bool }`（mix/duck/switch 等 policy 词汇属后续 Runtime/Control Plane）。**类型层面不可能产出 pipeline/backend/gst 引用**（纪律①同构：Intent → (未来) Execution Plan → Backend）。
- **D5 Normalize → Stream 桥**：`CanonicalAudioStream::from_description(id, &CanonicalAudioDescription)`——presence 直映；channels/sample_rate/sample_format → None（0.7B-2B 无观测源）；layout → Unknown；role → Unknown。桥是纯映射，不改 `normalize_input` 返回类型与既有诊断。
- **D6 `ObservationEvidence { code, detail }`**：与 clock 的 `ClockEvidence` 形状统一——定义于 audio.rs；两处合并为共享类型登记为后续统一债务（避免本轮 churn clock.rs）。
- **D7 门禁 AUDIO-SEMANTICS-RT-01 三层**：
  - Unit：provider 无关性（BMD 形状 vs Mock 形状 description → 同一 stream 媒体语义）；**Unknown 贯穿**（无观测 → presence=Unknown + role=Unknown，绝不默认 Program）；Route 测试（A/B streams → intent，serde 反向断言零 pipeline/backend/gst 引用）；Role 冻结词表快照。
  - Simulation：MockProvider 世界装配 audio stream。
  - Hardware：真机 loopback audio stream 证据输出（presence/channels/role；channels/sample_rate Unknown 合法）。

## Risks / Trade-offs

- `AudioSemanticTarget::Named(String)` 可能被滥用为业务名：文档锁定"opaque 语义标签，非路由配置"。
- `ObservationEvidence` 与 `ClockEvidence` 形状重复：本轮不合并（避免 churn clock.rs，0.7C 统一）。
- main.rs 仅 mod + loopback 诊断挂点（0.7A/0.7B-2A 先例）：不触碰五模块禁改清单。

```

## docs/openspec/changes/p07b-audio-semantics/tasks.md

- Source: docs/openspec/changes/p07b-audio-semantics/tasks.md
- Lines: 1-33
- SHA256: 417e290c27fdbcbbbd861c4a346a20167b8871a28f882e883bd1b9872b6dbc72

```md
# Tasks: Phase 0.7B-2B — p07b-audio-semantics

> 纪律（§十五）：每项标注 `Contract / Implementation / Verification / Gate`。

## 1. audio.rs 类型族（canonical 层，零 vendor 依赖）

- [ ] 1.1 `AudioRole`(Program/Commentary/Ambient/Auxiliary/Unknown — 业务词冻结禁止) / `AudioLayout`(Mono/Stereo/FiveOne/SevenOne/Unknown — 只描述) / `AudioSampleFormat`(最小集) / `ObservationEvidence` + serde
  - Contract: CANONICAL_MEDIA_MODEL §4 + 终审裁定词表 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 1.2 `CanonicalAudioStream`（presence 复用 normalize 三态; Unknown(id) 构造器 presence/role 全 Unknown + evidence）+ `CanonicalAudioStream::from_description` 桥
  - Contract: 契约 §4 + 终审 Unknown 贯穿 | Implementation: Not Started | Verification: Test | Gate: Pending

## 2. AudioRouteIntent（Semantic Intent）

- [ ] 2.1 `AudioRouteIntent { source, destination: AudioSemanticTarget, policy: RoutePolicy }` — 类型层面不可能产出 pipeline/backend/gst 引用
  - Contract: 纪律① 同构; 终审"不能 Intent→gst_pipeline" | Implementation: Not Started | Verification: Test(serde 反向断言) | Gate: Pending

## 3. 门禁 AUDIO-SEMANTICS-RT-01（三层）

- [ ] 3.1 Unit: provider 无关性 / Unknown 贯穿（绝不默认 Program）/ Route 零 pipeline 引用 / Role 词表快照
  - Contract: 终审测试要求 | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 3.2 Simulation: MockProvider 世界装配 audio stream
  - Contract: 契约 §4 | Implementation: Not Started | Verification: Simulation | Gate: Pending
- [ ] 3.3 Hardware: 真机 loopback audio stream 证据输出（role=Unknown 合法; channels/sample_rate Unknown 合法）
  - Contract: 终审 Hardware 要求 | Implementation: Not Started | Verification: Hardware | Gate: Pending

## 4. 交付

- [ ] 4.1 禁改清单核验（session/resource/lease/pipeline/backend 五文件零触碰）+ 盒上全矩阵 + CI 七 checks 不回退
  - Contract: 终审"不修改"清单 | Implementation: Not Started | Verification: Box+CI | Gate: Pending
- [ ] 4.2 债务登记 D11 (Clock Observation Timeline) / D12 (ObservationSource) 入 PHASE_0_7A_POST_MERGE_DEBT.md
  - Contract: 终审登记要求 | Implementation: Not Started | Verification: Docs | Gate: Pending
- [ ] 4.3 verify（full）→ archive → PR#5 → merge → 删分支
  - Contract: 分支纪律 | Implementation: Not Started | Verification: CI+Review | Gate: Pending

```
