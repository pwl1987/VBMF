# Comet Design Handoff

- Change: p07b-media-semantics
- Phase: design
- Mode: compact
- Context hash: 72145ed5f86a9d9062c5be622e66f7e1413d010a6424bf0e19b5f8bd94c1ea86

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## docs/openspec/changes/p07b-media-semantics/proposal.md

- Source: docs/openspec/changes/p07b-media-semantics/proposal.md
- Lines: 1-30
- SHA256: abb4177aa47ece94fd47d94fadfc739207b2084c622dca2b326cc10d35832533

```md
# Change: Phase 0.7B-1 — p07b-media-semantics（Normalize Foundation：Raw Device Description → Canonical Media Description）

## Why

0.7A 让 Runtime Ownership 闭环，但媒体语义仍散落在 vendor 观测结构里（`SignalStatus.video_format`、loopback 探测的 ad-hoc 字段）。按冻结契约 `CANONICAL_MEDIA_MODEL.md`，必须先建立 **Canonical 媒体描述层**，后续 Normalize/Switch/Audio 才有稳定语义地基。终审裁定 0.7B-1 只做最小地基：`Raw Device Description → CanonicalMediaDescriptor`，配门禁 **NORMALIZE-RT-01**（不同 Provider → 同一 canonical 表征）。

**三条设计纪律（终审冻结）**：① Normalize 不吞 Runtime Intent——只描述"是什么"，链路 `Input → Normalize → Canonical Media Model → (未来) Execution Plan → MediaBackend`；② Audio 独立 Flow 建模（契约 §4：Embedded/De-embedded/Independent/Mixed/External），绝不做 Video 的 Option 附属；③ Clock 只挂 `ClockDomain` 引用占位——绝不允许 Backend 偷偷决定 clock。

## What Changes

- **`src/normalize.rs`（新，canonical 层）**：
  - `CanonicalMediaDescriptor`：`{ source: CanonicalSourceRef{device_id, port_id}, video: CanonicalVideoDescription, audio: CanonicalAudioDescription, clock: CanonicalClockRef }`——只填当前用到字段（契约 §1：冻结类型≠全量实现）；零 vendor 字段（§5）。
  - `CanonicalVideoDescription`：dims / frame_rate / interlaced / pixel_format（对齐现有 `VideoFormat` 观测形状）。
  - `CanonicalAudioDescription`：`presence` + `embedding: AudioEmbedding{Embedded, DeEmbedded, Independent, Mixed, External}`（契约 §4 语义，显式建模）。
  - `CanonicalClockRef`：`Unspecified(ClockDomainId)` 占位——clock 策略属 0.7B Clock 阶段，此处仅引用不决策。
  - `RawInputDescription`（输入侧 raw 描述，provider 中立装配体：来自 `DeviceInfo`/`PortInfo`/`SignalStatus` 的观测值）。
  - `normalize_input(raw) -> NormalizeOutcome{descriptor, diagnostics}`：**纯函数、无副作用、绝不构造 pipeline/intent**（纪律①）。
- **门禁 NORMALIZE-RT-01（三层）**：不同 Provider 装配的 Raw 输入（BMD 真机观测形状 vs Mock 形状）在逻辑媒体相同时 → **同一 descriptor**（provider 无关性证明）；三层：Unit / Simulation-Mock / Hardware（盒上 BMD loopback 观测 → descriptor 与期望 canonical 形状一致）。
- **CI**：normalize 门禁测试并入现有 `rust-test-matrix`（不新增 job——无独立运行时依赖）。
- **债务联动**：D4（PortAvailability 精确化）与 D8（EventSink）不变；本 change 不碰。

## Capabilities

（`skip_specs: true`——canonical 语义 SoT 为 `CANONICAL_MEDIA_MODEL.md` 冻结契约 + 本 change 记录。）

## Impact

- **编译**：五套 feature 矩阵不回退；fmt/clippy -D 全绿；normalize.rs 零 vendor 依赖（ARCH-PORTABILITY-01 词法+PROOF 保持）。
- **受影响**：新 `normalize.rs`；`main.rs` mod 声明；无既有行为变更（纯新增模块，Session 路径零触碰）。
- **明确不做**：不生成 Execution Plan / pipeline；不做 Audio routing；不做 Clock 策略实现（`CanonicalClockRef` 仅占位）；不碰 UI/API/Scheduler；不改 Session/Resource/Lease 任何语义；不做色度学/编解码全量（§1 冻结≠全量）。

```

## docs/openspec/changes/p07b-media-semantics/design.md

- Source: docs/openspec/changes/p07b-media-semantics/design.md
- Lines: 1-29
- SHA256: 63b1f3b30531c64dae9606d5a3a51d6e5ffb759f8a9e3d244c151532e64afcd8

```md
# Design: Phase 0.7B-1 — p07b-media-semantics（Normalize Foundation）

## Context

冻结契约 `CANONICAL_MEDIA_MODEL.md`：§1 冻结类型≠全量实现（只填当前用到字段）；§4 Audio 独立建模（Embedded/De-embedded/Independent/Mixed/External）；§5 canonical 类型被 Domain/Graph/Session/Health 共享且零 vendor 字段。现有观测形状：`port.rs::VideoFormat {width,height,frame_rate,interlaced,pixel_format}`、`SignalStatus {video_locked,audio_locked,...}`、真机 loopback 实证（720x486 / 30000÷1001 / interlaced / v210 + embedded audio）。约束：纪律①（Normalize 不构造 pipeline/intent）、②（Audio 独立 Flow）、③（Clock 仅引用）。

## Goals / Non-Goals

**Goals:** `CanonicalMediaDescriptor`（canonical 媒体描述类型）+ `normalize_input` 纯函数（Raw → Canonical）；NORMALIZE-RT-01 provider 无关性证明（三层）。
**Non-Goals:** Execution Plan/pipeline 生成、Audio routing、Clock 策略实现、色度学/编解码全量、UI/API、Session/Resource 语义变更。

## Decisions

- **D1 描述符形状（契约 §1 最小字段）**：`CanonicalMediaDescriptor { source: CanonicalSourceRef{device_id, port_id: Option<Uuid>}, video: CanonicalVideoDescription{ width: u32, height: u32, frame_rate: Option<Rational>（用 (num, u64) 元组或字符串, 与 VideoFormat 的 "30000/1001" 观测对齐—设计取 (u32,u32) 结构化）, interlaced: bool, pixel_format: String }, audio: CanonicalAudioDescription{ presence: AudioPresence{Present{channels_hint: Option<u32>} | NotPresent | Unknown}, embedding: AudioEmbedding{Embedded|DeEmbedded|Independent|Mixed|External} }, clock: CanonicalClockRef{ domain: Option<Uuid> } }`。零 vendor 字段；serde 可序列化（证据输出用）。
- **D2 RawInputDescription（provider 中立装配体）**：`{ device_id, port_id: Option<Uuid>, observed: Option<ObservedMedia{video: Option<ObservedVideo{width,height,frame_rate:Option<(u32,u32)>,interlaced:Option<bool>,pixel_format:Option<String>}>, audio_present: Option<bool>}> , transport: String }`——由 PortInfo/SignalStatus 装配（port.rs 观测类型 → raw），装配函数放 normalize.rs 的 `From` 实现或独立 fn（canonical 层内，不依赖 adapters）。
- **D3 normalize_input 纯函数**：`normalize_input(raw: &RawInputDescription) -> NormalizeOutcome`；`NormalizeOutcome { descriptor: CanonicalMediaDescriptor, diagnostics: Vec<NormalizeDiagnostic> }`。规则：观测缺失字段 → descriptor 字段 `Unknown` 占位 + diagnostic WARN（不臆造）；provider 字段（persistent_id/handle）**绝不**进入 descriptor（契约 §2 身份层级）；transport 字符串原样保留为 canonical 标签。**绝不**返回 pipeline/intent（纪律①，类型层面即不可能——返回类型不含此类）。
- **D4 NORMALIZE-RT-01 provider 无关性**：`normalize_input(BMD 形状 raw) == normalize_input(Mock 形状 raw)` 当且仅当观测媒体相同——Unit 层用两组装配体证明结构相等；Simulation 层经 MockProvider 世界装配；Hardware 层盒上真机 loopback 观测装配 → descriptor 断言（1080i50 家族形状）。
- **D5 Clock 占位（纪律③）**：`CanonicalClockRef { domain: Option<Uuid> }`，0.7B-1 恒 `None` + diagnostic INFO（clock 策略后续阶段实现；类型在场防将来 Video/Audio 偷塞 clock 字段）。
- **D6 Audio 独立（纪律②/契约 §4）**：`CanonicalAudioDescription` 与 video 平级字段，非 `Option<Audio>` 嵌套；0.7B-1 由 `audio_present` 观测推导 `presence`，`embedding` 缺省 `Embedded`（SDI 内嵌现状显式化）+ diagnostic 注记（MADI/AES 等后续由 Audio Provider 声明）。

## Risks / Trade-offs

- 观测字段缺失时的 Unknown 占位 vs FAIL：0.7B-1 取"Unknown+WARN diagnostics"（描述层不决策；决策属未来 Preflight/Policy——与 D2 derive_claims FAIL 化债务 D2 衔接）。
- frame_rate 结构化 (num,den) vs 字符串：取 (u32,u32)，`VideoFormat.frame_rate: Option<String>` 装配时解析，解析失败→Unknown+WARN（不丢观测）。
- `port.rs::VideoFormat` 与 canonical 结构形状重叠：**不替换**既有观测类型（runtime 探测层不动），normalize 层转换——避免触碰 Session 路径。

## 实施顺序

normalize.rs 类型 → 装配 From → normalize_input → Unit 测试 → NORMALIZE-RT-01 Simulation → main.rs mod + 真机观测装配点（loopback/diagnostic 证据输出可选挂接）→ 盒上矩阵 + 真机门禁 → CI 重跑。

```

## docs/openspec/changes/p07b-media-semantics/tasks.md

- Source: docs/openspec/changes/p07b-media-semantics/tasks.md
- Lines: 1-37
- SHA256: 71074d5894d9cbce636b5fc48415f0fb767af2ae6c599fb6dcde39a0c93d74a6

```md
# Tasks: Phase 0.7B-1 — p07b-media-semantics

> 纪律（§十五）：每项标注 `Contract / Implementation / Verification / Gate`。

## 1. Canonical 类型（normalize.rs）

- [ ] 1.1 `CanonicalMediaDescriptor` / `CanonicalVideoDescription` / `CanonicalAudioDescription`(含 `AudioEmbedding` 五语义) / `CanonicalClockRef` / `CanonicalSourceRef`
  - Contract: CANONICAL_MEDIA_MODEL §1(冻结类型≠全量)/§4(Audio 独立)/§5(零 vendor 字段)；纪律①②③ | Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 1.2 `RawInputDescription` 装配体 + 从 PortInfo/SignalStatus 的观测装配
  - Contract: 契约 §2(Media — Signal/MediaFormat Observed) | Implementation: Not Started | Verification: Test | Gate: Pending

## 2. normalize_input 纯函数

- [ ] 2.1 `normalize_input(raw) -> NormalizeOutcome{descriptor, diagnostics}`：观测缺失→Unknown+WARN 不臆造；provider 字段绝不进入 descriptor；绝不构造 pipeline/intent（类型层面不可能）
  - Contract: 纪律①；VENDOR_NEUTRALITY_RULES | Implementation: Not Started | Verification: Test | Gate: Pending

## 3. 门禁 NORMALIZE-RT-01（三层）

- [ ] 3.1 Unit: provider 无关性（BMD 形状 vs Mock 形状 raw → 同一 descriptor）+ 缺失观测 Unknown/WARN + frame_rate 解析失败不丢观测
  - Contract: NORMALIZE-RT-01 定义（不同 Provider → 同一 canonical model）| Implementation: Not Started | Verification: Test | Gate: Pending
- [ ] 3.2 Simulation: MockProvider 世界装配 → descriptor 断言（嵌入 SDI 语义显式化）
  - Contract: 契约 §4 | Implementation: Not Started | Verification: Simulation | Gate: Pending
- [ ] 3.3 Hardware: 盒上真机 loopback 观测装配 → descriptor 与 1080i50 家族 canonical 形状一致
  - Contract: NORMALIZE-RT-01 | Implementation: Not Started | Verification: Hardware | Gate: Pending

## 4. 接线与交付

- [ ] 4.1 main.rs mod 声明 + 诊断证据挂接点（loopback 观测 → descriptor 输出，可选）；Session/Resource 路径零触碰
  - Contract: 纪律① | Implementation: Not Started | Verification: Simulation+Hardware | Gate: Pending
- [ ] 4.2 盒上全矩阵（fmt/test×4/clippy×4/build×3/PROOF）+ CI 七 checks 不回退
  - Contract: 工程门禁不退化 | Implementation: Not Started | Verification: Box+CI | Gate: Pending
- [ ] 4.3 verify（full）→ archive → PR#3 → merge → 删分支
  - Contract: 分支纪律 | Implementation: Not Started | Verification: CI+Review | Gate: Pending

## 收口确认

- 0.7B-1 仅 Normalize Foundation：不碰 UI/API/pipeline/Audio routing/Clock 策略实现；CanonicalClockRef 仅占位引用。

```
