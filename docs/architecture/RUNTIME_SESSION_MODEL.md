# RUNTIME_SESSION_MODEL（Canonical Session 模型契约）

> 状态：🔧 待建 → ✅ 已建（Phase 0.6, P0）
> 来源：Portability PRD #15, #21, #56；澄清问题点 (b)(d)
> 关联：`CANONICAL_MEDIA_MODEL.md`、`RUNTIME_BINDING_MODEL.md`、`IMPLEMENTATION_ADDENDUM.md`

## 1. 定位
Session 是 Canonical 运行时组合单位，持有 Video/Audio/Metadata Graph 的**语义描述**（Canonical），但 **struct 内不含 real Pipeline / GStreamer 对象**。

## 2. 与 Graph/Pipeline 的关系（澄清 #21 vs #56）
- #56 "Session 可包含 Video/Audio/Metadata Graph" = **逻辑包含**（语义描述层）。
- #21 "Session 不可包含 real Pipeline" = **struct 内不含** GStreamer Pipeline 对象；Session 通过 handle/ref 引用由 Backend 实现的 real Pipeline。
- 两处表述互补，本契约统一：逻辑持有、物理引用。

## 3. 字段（Canonical，无 vendor 类型）
- `session_id: PersistentId`（稳定，禁 device-number 作主身份，#8/#11）
- `graphs: [CanonicalGraph]`（Video/Audio/Metadata 各自独立，不合并业务 Graph，#55）
- `bindings: [RuntimeBindingRef]`
- `lease: LeaseRef`
- `state: RELEASED → RESERVED → RUNNING → PAUSED → RELEASING`（防非法：RELEASED→RUNNING 必须拒绝，#114）
- `ownership`：仅明确边界（V0.2 已定义语义不变；Phase 0.6 仅 additive 明确，**不引入新业务语义**，澄清 #159 d）

## 4. 替换不变量
- BMD→AJA / GStreamer→FFmpeg / Embedded→MADI：Session 与 GraphIntent 不变（#61/#62/#63）。
- Session DEGRADED 当 Binding STALE（硬件移除），禁自动找新卡顶替（#58）。

## 5. 验收
- `ARCH-PORTABILITY-01`（MockProvider A/B 共享 same Session）
- #145 Session Acceptance（create/start/stop/crash/recover/release/double-start/double-stop/lease conflict/resource conflict）
