# VBMF Object Navigation Matrix (对象跳转矩阵)

> **目的:** Phase 0.5C Information Architecture Closure 的收口交付之一（与 `SURFACE_REGISTRY.yaml` + `NAVIGATION.md` + `PRODUCT_OBJECT_MODEL.md` 互补）。
> `SURFACE_REGISTRY.yaml` = 表面计数唯一事实源（SoT）；`NAVIGATION.md` = 表面到域/路由的映射；**本文 = 核心对象之间的跳转闭环与上下文约束**。
>
> **状态:** 🟢 LOCK FINAL（0.5C IA Closure）· 与 `NAVIGATION.md` / `SURFACE_REGISTRY.yaml` 同状态语言。
> **权威源:** [`SURFACE_REGISTRY.yaml`](SURFACE_REGISTRY.yaml)（surface ID / 计数 SoT）· [`NAVIGATION.md`](NAVIGATION.md)（表面映射）。

---

## 0. 硬规则（Phase 4 实现约束）

> **每一个核心对象都必须存在「查看 → 修改 → 查看运行态 → 查看影响 → 返回」的闭环。**

- 跳转必须**携带对象上下文**（对象 ID / Revision / 路径），**禁止**打开泛化首页（如裸 `Output` 首页、`Source` 首页）。
- 跨域跳转到达后，目标页面顶部必须显示「来自 / Go Back」与携带的 Context。
- 任何 surface ID 必须与 `SURFACE_REGISTRY.yaml` 完全一致，**不能把两个 surface 压缩成同一 ID**（如 `CD-01` 必须拆为 `CD-01-WS` / `CD-01-Detail`）。
- **覆盖完整性:** 下表已覆盖 `OBJECT_VOCABULARY.md` 锁定的全部 **15 个 Canonical Object**（Source / Profile / Profile Bundle / Output Variant / Media Session / ChangeSet / Job / Asset / Asset Version / Route / Destination / Adapter / Revision / Channel / Channel Template）。任何新增对象都必须补一行, 不得留白。

---

## 1. 对象跳转矩阵

| 当前对象 | 操作 | 去哪里（surface） | 必须保持的上下文 |
|---|---|---|---|
| Source | Verify 验证 | E-42 Source Test Bench | `source_id` |
| Source | Open Channel 打开频道 | CD-01-WS / CD-01-Detail | `channel_id` |
| Profile | Used By 影响面 | E-50 Impact Preview | `profile_rev` |
| Bundle | Open Channel 打开频道 | CD-01-WS / CD-01-Detail | `bundle_rev` |
| Variant | Open Output 打开输出 | 06 Output | `variant_id` + `output_profile_ref` + `packaging_profile_ref` |
| Realtime Session | Open Output 打开输出 | 06 Output | `session_id` + `variant_id` |
| Output | Open Health 打开健康 | 09 Health Tree | `node_path` |
| Incident | Replay 回放 | 07 Recording | `incident_id` |
| ChangeSet | Runtime 运行态 | M-17 Realtime Session | `runtime_revision` |
| Channel | Audio Profile 音频配置 | P-23 Audio Profile | `profile_rev` |
| Asset | Create Version 创建版本 | M-12 Asset Detail (Tab ②) | `asset_id` |
| Asset Version | File Transcode 文件转码 | M-14 File Transcode | `asset_version_id` + `asset_id` |
| Asset Version | QC 质检 | M-18 Job Detail (QC) / QC Profile | `asset_version_id` |
| Job | Open Output Asset Version 查看产出 | M-12 Asset Detail | `job_id` + `asset_version_id` |
| Job | Runtime 运行态 | M-18 Job Detail | `job_id` |
| Route | Open Source / Channel 反向跳转 | 02 Sources / CD-01 | `route_id` |
| Destination | Open Adapter 查看执行资源 | 06 Output (Adapter 详情) | `dest_id` + `adapter_ref` |
| Adapter | Open Health 健康 | 09 Health Tree / E-38 | `adapter_id` |
| Revision | Open ChangeSet 查看变更 | D7 ChangeSet | `revision_id` |
| Channel Template | Instantiate 实例化频道 | CD-01 (Create Channel) | `template_id` |

### 1.1 媒体域核心闭环 (Asset → Asset Version → Job → 新 Asset Version → QC → Used By)

> 用户主链路必须全程保留上下文, 转码产出去哪了一目了然:

```
Asset (M-11)
  → Create Asset Version (M-12 Tab ②)
  → File Transcode (M-14, Transcode Center / FILE 模式)
  → 选 Output Version (asset_version_id)
  → 选 Encoding Profile (P-21)
  → 选 Packaging Profile (P-20 Packaging Tab · 默认继承 Bundle Default, 可 Variant Override, 0.5F.13)
  → 选 Output Profile (P-22 · 唯一 SoT = Variant, 0.5F.13)
  → Job Policy (M-14 Step ⑤)
  → Preview / Test Encode (M-14 Step ⑥)
  → Submit → Job (M-18, FILE_TRANSCODE, job_id)
  → COMPLETED → 新 Asset Version (asset_version_id)
  → QC (M-18 Job Detail / QC Profile)
  → Used By → Channel / Playout (CD-01)
```

- 任何跳转都携带 `asset_id` / `asset_version_id` / `job_id`, **禁止**从转码页跳到泛化首页后丢失上下文。
- `Job` 与 `Asset Version` 双向可达: Job 详情能看到产出 Asset Version, Asset Version 能看到触发它的 Job。

### 1.2 对象动作矩阵（OBJECT_ACTION_MATRIX · 与 1 节导航矩阵互补）

> 1 节是 **cross-object 点对点导航**（A 怎么到 B）；本矩阵是 **per-object 生命周期动作契约**：每个对象必须存在 `View → Edit → Runtime → Impact → Revision` 五态，且 Edit / Revision 统一走 `session.apply_revision`（V0.2 §3.x Runtime Contract），禁止散落的直接写操作。

| 对象 | View | Edit | Runtime | Impact | Revision |
| --- | --- | --- | --- | --- | --- |
| Source | E-40 / Source List | ChangeSet → E-40 编辑 | M-17 信号态 | E-50 | session.apply_revision |
| Profile (8 kinds) | P-20 / P-21..P-27 | ChangeSet | 模板级（不进运行态） | E-50 | session.apply_revision |
| Bundle | P-28 | ChangeSet（引用不复制） | CD-01 Bundle 快照 | E-50 / E-51 | session.apply_revision |
| Variant | CD-01 Output / 06-output | ChangeSet | CD-01 Runtime Output | E-50 | session.apply_revision |
| Realtime Session | M-17 | ChangeSet（Reservation） | M-17 | E-50 | session.apply_revision |
| Output | 06-output / CD-01 | ChangeSet | CD-01 Output 区 | E-50 | session.apply_revision |
| Incident | E-30 / CD-01 Health | ChangeSet / Auto | CD-01 Health | E-50 | session.apply_revision |
| ChangeSet | E-33 | —（本身即变更单元） | — | E-50 | E-33 apply |
| Channel | CH-01 / CD-01 | CH-02 / ChangeSet | CD-01 运行态 | E-50 | session.apply_revision |
| Asset | Asset Library | ChangeSet | — | E-50 | session.apply_revision |
| Asset Version | Asset Version 详情 | Create Version（ChangeSet） | — | E-50 | session.apply_revision |
| Job | Job List / M-17 | ChangeSet | Job 执行态 | E-50 | session.apply_revision |
| Route | Route 配置 | ChangeSet | Route 运行态 | E-50 | session.apply_revision |
| Destination | 06-output Destination | ChangeSet | Destination 运行态 | E-50 | session.apply_revision |
| Adapter | Adapter 注册 | ChangeSet | Adapter 运行态 | E-50 | session.apply_revision |
| Revision | Revision 历史（E-33） | — | — | — | — |
| Channel Template | CH-02b | CH-02b 编辑 | —（不进运行态） | E-50 | session.apply_revision |

---

## 2. 闭环自检清单（Acceptance）

- [ ] Source：`source_id` 全程贯穿 E-40 → E-41 → E-42 → CD-01
- [ ] Profile：`profile_rev` 从 P-21 经 P-28 → CD-01 → E-50 可追溯
- [ ] Bundle：`bundle_rev` 在 CD-01 与 ChangeSet 双向可达
- [ ] Variant / Session：`variant_id` / `session_id` 在 M-17 与 06 Output 间闭环
- [ ] Output：`node_path` 在 06 Output 与 09 Health Tree 间闭环
- [ ] ChangeSet：`runtime_revision` 在 D7 → M-17 间单调可回滚
- [ ] Channel Audio：`profile_rev` 在 CD-01-WS 与 P-23 间闭环
- [ ] Asset / Asset Version：`asset_id` / `asset_version_id` 贯穿 M-11 → M-12 → M-14 → M-18 → (新 Asset Version) → QC → Used By
- [ ] Job：`job_id` 从 M-14 经 M-18 → 新 Asset Version 闭环, 与 Asset Version 双向可达
- [ ] Route：`route_id` 在 02 / 03 / 04 与 CD-01 间双向可达
- [ ] Destination / Adapter：`dest_id` / `adapter_ref` 在 06 Output 与 09 Health Tree 间闭环
- [ ] Revision：`revision_id` 在 D7 与 ChangeSet 间可达
- [ ] Channel Template：`template_id` 实例化 → CD-01 闭环
- [ ] **0.5F.13**: Variant 跳转携带 `output_profile_ref` + `packaging_profile_ref`, 后者可 Variant Override 继承 Bundle Default
- [ ] **0.5F.13**: 任意 Profile 派生值 Surface 提供 Inherited/Overridden/Explicit/Compiled/Effective 来源链 (闭环覆盖 §3.5)

---

## 3. 配置继承链闭环 (0.5F.13 新增)

> Phase 4 实现约束：Profile → Bundle → Variant → Runtime 的 5 层派生，必须在 UI 上形成「来源可解释」闭环，禁止运行态正确但来源不可追溯。

### 3.1 层级与 5 态来源

```text
Global Profile (Explicit)
      ↓ Inherited
Profile Bundle (引用)
      ↓ Inherited / Overridden
Channel Configuration
      ↓ Overridden (per-Variant)
Output Variant (output_profile_ref / packaging_profile_ref)
      ↓ Compiled
Compiled Runtime
      ↓ Effective
Effective Runtime
```

每屏显示派生值，必须可展开：
`Inherited`（灰）/ `Overridden`（橙）/ `Explicit` / `Compiled` / `Effective`（主显示加粗）。

### 3.2 闭环要求（Acceptance 0.5F.13）

- [ ] P-28 / CD-01 / P-21 / P-22 / M-14 任一 Profile 派生值，点击可见完整来源链
- [ ] Variant 的 `packaging_profile_ref` 显式标注「继承 Bundle」或「Variant Override」
- [ ] `EFFECTIVE_PACKAGING = Bundle Default ↓ Variant Override` 计算路径可回溯
- [ ] 改 Bundle Default 时，Impact Preview 明确区分「受影响 Variant（未指定）」与「不受影响 Variant（已 Override）」
