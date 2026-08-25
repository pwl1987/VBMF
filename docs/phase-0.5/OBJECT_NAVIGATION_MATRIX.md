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

---

## 1. 对象跳转矩阵

| 当前对象 | 操作 | 去哪里（surface） | 必须保持的上下文 |
|---|---|---|---|
| Source | Verify 验证 | E-42 Source Test Bench | `source_id` |
| Source | Open Channel 打开频道 | CD-01-WS / CD-01-Detail | `channel_id` |
| Profile | Used By 影响面 | E-50 Impact Preview | `profile_rev` |
| Bundle | Open Channel 打开频道 | CD-01-WS / CD-01-Detail | `bundle_rev` |
| Variant | Open Output 打开输出 | 06 Output | `variant_id` |
| Realtime Session | Open Output 打开输出 | 06 Output | `session_id` + `variant_id` |
| Output | Open Health 打开健康 | 09 Health Tree | `node_path` |
| Incident | Replay 回放 | 07 Recording | `incident_id` |
| ChangeSet | Runtime 运行态 | M-17 Realtime Session | `runtime_revision` |
| Channel | Audio Profile 音频配置 | P-23 Audio Profile | `profile_rev` |

---

## 2. 闭环自检清单（Acceptance）

- [ ] Source：`source_id` 全程贯穿 E-40 → E-41 → E-42 → CD-01
- [ ] Profile：`profile_rev` 从 P-21 经 P-28 → CD-01 → E-50 可追溯
- [ ] Bundle：`bundle_rev` 在 CD-01 与 ChangeSet 双向可达
- [ ] Variant / Session：`variant_id` / `session_id` 在 M-17 与 06 Output 间闭环
- [ ] Output：`node_path` 在 06 Output 与 09 Health Tree 间闭环
- [ ] ChangeSet：`runtime_revision` 在 D7 → M-17 间单调可回滚
- [ ] Channel Audio：`profile_rev` 在 CD-01-WS 与 P-23 间闭环
