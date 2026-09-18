# RF-FF-01A — Backend-neutral RuntimeBinding extraction 收口报告

日期：2026-09-18
Runtime commit：`0dd37bc1215677cdd2c823d31e9ca3eb307dde26`

## 1. Authority / Goal

Authority：

- `.project/STATE.md §3.25 / §4`
- `MEDIA_BACKEND_CONTRACT.md §1 / §4`
- `RUNTIME_BINDING_MODEL.md`
- `IMPLEMENTATION_BOUNDARIES.md §6`

冻结要求是：GStreamer → FFmpeg 只替换 Backend + RuntimeBinding，不改变 CanonicalPipelinePlan。live 实现此前把 GStreamer `device-number` / provider persistent runtime value 直接放在 `SourcePlan`，形成实现漂移。

## 2. Implementation

- `SourcePlan` 删除 `device_number`、`provider_persistent_id`、旧 `SourceSelectionMode`。
- 新 `SourceBindingClass::{Persistent, Resolved, DiagnosticFallback, SelfTest}` 只描述 backend-neutral binding 语义，不携带 runtime address。
- `GStreamerPipelineController` 持有组合根注入的 authoritative `ResolvedDeviceBinding` view；`instantiate()` 按 canonical `device_id + binding_class` 解析 GStreamer runtime address。
- `AdapterRegistry` 增加 explicit `*_with_bindings` 构造面，仍保证 Backend / MediaTap / Bridge / Diagnostic views 指向同一 concrete controller。
- production composition root 与真实 hardware gates 全部走 `with_bindings`；空 binding constructor 仅保留 SelfTest / structure-only tests。
- `GraphRuntimeIntent`、command/wire vocabulary、Session / Resource ownership 均未改变。

## 3. Failure-first invariants

- Resolved source 缺 RuntimeBinding → `IdentityUnresolved`，fail-closed。
- Persistent source 缺 `persistent_id` → fail-closed，不生成 `persistent-id=0`。
- DiagnosticFallback：
  - canonical device_id 有 binding → 使用该 binding 的真实 device-number；
  - canonical device_id 合法但 binding 缺席 → 仅 Diagnostic 模式保留历史 device-0 fallback；
  - 非 canonical device_id → fail-closed，不得退化到 device 0。
- SelfTest 不需要任何 hardware RuntimeBinding。
- canonical SourcePlan 序列化机械断言禁止 `device_number/provider_persistent_id/persistent_id/gstreamer` 地址键泄漏。

## 4. Development VM verification

- `src_props_*` focused：**7/7 PASS**
- Persistent materialize focused：**2/2 PASS**
- mock lib：**435/435 PASS**
- integrations：**9/9 + 12/12 PASS**
- default lib：**251/251 PASS**
- `cargo fmt --check`：PASS
- default/mock `cargo clippy --all-targets -- -D warnings`：PASS
- CodeGraph：已 sync
- Development VM `cargo check --features bmd,gstreamer`：**ENVIRONMENT BLOCKED**，缺 `glib-2.0.pc/gstreamer-1.0.pc` 等 system pkg-config 开发文件；不记作代码 FAIL，也不记作本机 PASS。

## 5. GitHub CI

GitHub Actions run `35296461203` @ `0dd37bc…`：

- hardware-test-compile：PASS
- session-lifecycle：PASS
- gstreamer-build：PASS
- rust-test-matrix：PASS
- rust-clippy：PASS
- rust-format：PASS
- architecture-portability：PASS

**Required CI：7/7 PASS。**

## 6. BMD exact-commit hardware verification

环境：`lytv / 10.30.15.10`

- exact archive SHA256：`d9ec96bd91d437024dae2e2b91f931521c7202adebbf90198788374b43514401`
- manifest：`a2-8-02i-v5.manifest.json`
- manifest MD5：`7521d17e7fd02e50eb2b0a84374a43dd`
- gates binary SHA256：`4e80d9aad6521c7f3506dd5168c3156e8893c6e48d663dbd1a2b3fc48bd14560`
- dual-input log SHA256：`d60e9a5abe068eaf7edacc98c8941235c0eadeded3c45d3ad61fe49a5d44ea08`
- BMD native build：`cargo build --features bmd,gstreamer --bin media-agent-gates` → PASS
- Dual Input Gate：**ALL PASS 10/10 / rc=0**
- Runtime binding：Input-1 → gst device 0，Input-2 → gst device 1，production binding 2/2，signal Locked。
- L2 dual session started_inputs=2；L2b actual media frames；L3 Program advancing；L4 switch/timeline PASS；L5 fault isolation/recover PASS；Teardown Released PASS。
- historical output `device-number=2` process PID before/after 均为 `992634`，未触碰。
- stale `/opt/vbmf-dev/repo` 未修改。

Evidence：
`evidence/bmd-10.30.15.10/2026-09-18-rf-ff-01a-runtime-binding/`

## 7. Verdict

**RF-FF-01A COMPLETE / SOFTWARE VERIFIED / CI VERIFIED / BMD HARDWARE VERIFIED。**

无 frozen Contract 变化；本 change 是 live implementation 向既有 `MEDIA_BACKEND_CONTRACT §4` 收敛。24h RSS stability debt 不受本 change 影响，仍不得写成 stability verified。
