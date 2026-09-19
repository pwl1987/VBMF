# RF-SRC-RTMP-02 TG-1 — Canonical Endpoint Types + NetworkSourceBinding Loader

- Date: 2026-09-19
- Packet: `RF-SRC-RTMP-02-IMPLEMENTATION` Step 1 / TG-1
- Design authority: `docs/superpowers/plans/2026-09-19-rf-src-rtmp-02-production-rtmp-input-boundary.md`（D2/D3/D11 + Implementation Invariants INV-3）
- Scope（本轮允许清单）: `source.rs` canonical 类型；`NetworkSourceBinding` manifest 加载器（新模块 `network_binding.rs` + `lib.rs` 模块声明 + `Cargo.toml` linux `libc` 依赖）。未触碰 FFmpeg adapter、preflight/session/bootstrap、recovery/supervisor、Web/API、BMD、DeckLink/SRT、D7/D8。

## 1. 交付内容

### source.rs — D2 canonical 强类型（唯一流通货币）

- `CanonicalIp`：严格 IP 字面量——IPv4 严格点分十进制无前导零；IPv6 仅接受 RFC 5952 规范小写压缩拼写（parse → 与 canonical 渲染逐字节比对，非规范直接拒绝，绝不"先规范化再比较"）；hostname/DNS/zone-id 一律拒绝。
- 地址类别 allowlist（`is_eligible_bind_class`）：仅 loopback（fixture）、RFC1918 私网、IPv6 ULA 可用；公网、链路本地（含 169.254.169.254）、组播、广播、通配/未指明、IPv4-mapped IPv6、CGNAT、文档地址（192.0.2/24、198.51.100/24、203.0.113/24、2001:db8::/32）与其余保留段全部拒绝。
- `CanonicalPath`：`/` + 非空段；段字符集 `[A-Za-z0-9._-]`；拒绝空段、点段（`.`/`..`）、百分号编码（`%2F` 等）、query/fragment、反斜杠、控制/空白、非 ASCII；大小写敏感、尾部 `/` 为不同值、零等价归一化。
- `CanonicalRtmpEndpoint::parse_strict`（URL 形式，manifest 输入）：IPv6 必须方括号；端口显式必填（隐式 1935 拒绝）、`1024..=65535`（特权端口拒绝）；userinfo/query/fragment 结构上不可表达；地址类别与 path 语法全量校验。
- `ListenerKey`（D6）：`(protocol, canonical_ip, port)` 唯一键类型，path 刻意不参与 listener 身份。
- `NetworkEndpoint::to_canonical()`：wire 值 → canonical 强类型的唯一桥（bare/bracketed IPv6 均接受但拼写必须已规范）。
- 全部 canonical 类型的 `Debug` 手工 redacted（plan D9 从第一天生效）；`to_url`/`as_str` 为显式 adapter-local 出口（argv/manifest 输入例外边界）。

### network_binding.rs — D3 + INV-3 NetworkSourceBinding 加载器

- 防竞态单 fd 加载序列：`open(O_NOFOLLOW)`（symlink → `SymlinkRejected`）→ `fstat`（普通文件 / owner==euid / mode 恰为 0600 / size ∈ (0, 1 MiB]）→ `read`（带 MAX+1 截断保护）→ 严格 parse → **再次 fstat**（dev/ino/size/mtime 与读前一致且等于实际读得字节数，否则 `ModifiedDuringLoad`）。
- 严格 JSON：`deny_unknown_fields` + serde 结构体天然拒绝重复字段 + `from_slice` 拒绝尾随数据；未知 version / 空 machine_id / 空 entries 全拒绝。
- `machine_id` pin：复用 `resolver::current_machine_id()`（VBMF_MACHINE_ID→HOSTNAME 机制）；runtime 身份未解析 → `MachineIdUnresolved`（比 DeviceBindingManifest 的 skip 更严，符合 D3 fail-closed）。
- `source_id` 必须为 canonical UUID 拼写（hyphen 小写；hyphenless/braced/urn 拒绝）；endpoint 必须通过 `CanonicalRtmpEndpoint::parse_strict`。
- 唯一性：source_id 双向 1:1；完整 endpoint 不得重复；同 `(protocol, ip, port)` 不同 path → `ListenerConflict`（manifest 阶段直接拒绝，不推迟到 Registry）。
- API = `load`（启动一次）/ `authorize(source_id, NetworkEndpoint)`（D11 五元组精确匹配：先 canonical 化再与授权条目全等比较）/ `authorized_sources`（仅 NetworkSourceId 的 redaction-safe 投影）。**无 reload/watch/follow API**；不构造 `SourceIntent`；path 只作精确准入标签（测试锚定"非 publisher 认证"语义）。
- 错误类型 `NetworkBindingError` 全部 typed 且 redaction-safe（不携带 endpoint/host/port/path/文件路径内容）。

### 依赖

- 新增唯一直接依赖：`libc`（`[target.'cfg(target_os = "linux")'.dependencies]`），用于 `O_NOFOLLOW` 与 `geteuid`（INV-3 owner 校验）。

## 2. 软件验证（Development VM, Rust 1.98.1）

| 项 | 结果 |
|---|---|
| focused `source::` | 25 passed（含 9 个新 rf_src_rtmp_02_* canonical 测试 + 既有回归） |
| focused `network_binding::` | 12 passed（D3/INV-3 全 fail-closed 矩阵） |
| default lib | **296/296** |
| mock lib + integration | **483/483** + **9/9 + 12/12** |
| simulation lib | **296/296** |
| ffmpeg-backend lib | **318 passed / 1 ignored**（既有 ignored 项保持） |
| clippy `-D warnings` | default / mock / ffmpeg-backend 三档 PASS |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --all-targets` | PASS |
| architecture lint（ARCH-PORTABILITY-01） | PASS |
| remove-adapters proof（simulation/mock） | PROOF OK |
| `git diff --check` | PASS |

测试矩阵覆盖用户要求的全部负向族：canonical IPv4/IPv6 表示与类别拒绝（mapped/CGNAT/doc/公网/链路本地/组播/广播/通配）、无端口/隐式端口/错误方括号/hostname、path 空值/点段/query/fragment/反斜杠/百分号编码/空白/控制字符/非法字符、source_id 归属匹配与不匹配、同 ip:port 不同 path 拒绝、文件替换/截断竞态（FileIdentity 守卫 + symlink/O_NOFOLLOW + 实文件模式矩阵）、manifest 不生成 SourceIntent、path 仅路由标识非认证。

## 3. 边界与遗留

- 本轮未把 canonical 授权接线进 preflight/session（属 TG-2+ 生产组合路径）；`NetworkEndpoint` wire 值与既有 loopback 强制/argv 路径原样保留（既有测试全部通过 = 零回归）。
- 唯一登记事项：INV-3 owner 校验以当前 euid 实现（本 runtime 直接以服务用户运行）；若未来引入特权启动序列，加载顺序必须移到降权之后（模块头注释已锚定）。
- 下一步 = TG-2：无设备副作用的网络源生产组合路径（bootstrap/config 接线 + preflight Production 授权匹配）。
