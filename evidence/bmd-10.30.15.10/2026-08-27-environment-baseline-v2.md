# BMD 环境基线 v2 (Environment Baseline v2) — 2026-08-27

> 本文件是当前 BMD 真机环境的**权威指纹**, 取代旧 `2026-08-25-environment-prep.md` 作为
> 当前态指针。旧文件保留为**历史记录** (`environment_base_sha = 7cc33dd`), 不删除、不抹改。
> 后续 MEDIA-RT-01 / A2 / FI-08 / FI-09 的 Acceptance Manifest 须引用本基线。

## 用途

- 复现环境指纹: 任何人/CI 据此重建可运行 BMD 构建。
- Acceptance 证据的可比对基准: 每条 evidence 的 `environment_base_sha` / `test_subject_sha`
  指向本文件与对应代码提交。

## 环境指纹

```yaml
host:
  address: 10.30.15.10
  user: lytv
  os: "Ubuntu 26.04 (per operator baseline; 以盒上 `cat /etc/os-release` 为准)"
  kernel: "VERIFY ON BOX — `uname -a`"
  arch: x86_64

docker:
  version: "VERIFY ON BOX — `docker --version`"
  compose: "VERIFY ON BOX — `docker compose version`"
  runtime: runc            # runsc 未安装 (见历史 environment-prep); Runtime 选择已裁决 = runc

gstreamer:
  version: 1.28.2          # 与运行时一致, 未扰动生产 (apt libgstreamer1.0-dev 等)
  decklink_plugin: present # decklinkvideosrc / decklinkaudiosrc (Desktop Video 驱动自带)
  gst_launch: "VERIFY — `gst-inspect-1.0 decklinkvideosrc`"

rust:
  toolchain: stable (edition 2021)
  rustc: "VERIFY ON BOX — `rustc --version`"
  cargo: "VERIFY ON BOX — `cargo --version`"

blackmagic:
  desktop_video_sdk: 16.0
  driver: "Desktop Video 16.0 (VERIFY — `dpkg -l | grep desktopvideo` 或盒上 About)"
  libDeckLinkAPI_so: present   # LD_LIBRARY_PATH=/usr/lib 注入
  sdk_include: /home/lytv/decklink-sdk-include  # DECKLINK_SDK_INCLUDE (软链)

repository:
  remote: git@github.com:pwl1987/VBMF.git
  branch: master
  test_subject_sha: 41e0931     # fix(media-agent): Device Registry 身份闭合 (Identity Closure Patch)
  environment_base_sha: 41e0931  # 本基线对应的代码态

toolchain:
  libclang: "LLVM 21 (LIBCLANG_PATH=/usr/lib/llvm-21/lib, 持久化 ~/.bashrc)"
  protobuf: "protobuf-compiler (CI 安装; 本地盒上按需)"
  bindgen: 0.70   # bmd feature 构建期依赖

registry:
  device_count: 3  # 2×DeckLink SDI + 1×DeckLink Mini Monitor 4K

network:
  docker_registry_primary: "VERIFY ON BOX"
  github_push: "SSH only — github.com:443 被 Tailscale 阻断; 用 git@github.com:pwl1987/VBMF.git"
```

## 本地独立编译能力 (已具备, 2026-08-27)

盒上可脱离 CI 直接构建 + 真机验证:

```bash
# 环境变量 (持久化于 ~/.bashrc)
export DECKLINK_SDK_INCLUDE=/home/lytv/decklink-sdk-include
export LIBCLANG_PATH=/usr/lib/llvm-21/lib
# 二进制运行需注入 SDK 运行时
LD_LIBRARY_PATH=/usr/lib ~/.cargo/bin/cargo build --features bmd,gstreamer
LD_LIBRARY_PATH=/usr/lib ./target/debug/media-agent
```

- `apt` 修复: `ubuntu.sources` 曾因 `Types: deb`/`deb-src` 双写致只下 deb-src 索引 → 所有 `-dev`
  包 "Unable to locate"; 已修正为单行 `Types: deb deb-src` 后 `apt-get update`。
- 装 `libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libclang-dev clang` (1.28.2)。
- SDK include 软链 `DECKLINK_SDK_INCLUDE`; `LIBCLANG_PATH` 指向 `llvm-21`。

## 当前构建模型 (不变, 不建议再改)

```text
CI        : default / simulation / bmd,gstreamer / hardware-test (bindgen)
BMD 真机  : 实际 SDK + libDeckLinkAPI.so + 硬件; canonical 构建 = --features bmd,gstreamer
GStreamer : media-agent 执行层, 非控制面 (Node/Fastify 管控制面)
```

## 与旧基线的关系

| 项目 | 旧 (7cc33dd, 2026-08-25) | 当前 v2 (41e0931, 2026-08-27) |
|------|--------------------------|-------------------------------|
| Runtime | runsc 未安装 / Option A 待裁决 | runc (已裁决) |
| 编译环境 | 仅 CI 矩阵, 本地独立编译未就绪 | 盒上本地独立编译已具备 |
| DeviceHandle | 未实测闭环 | 已实测并进入 DeviceRegistry (Identity Closure) |
| C1 Resolver | 未基于真实 Registry 身份 | 代码已闭合, 但 GStreamer hw-serial 实测为空 → 仍 UNRESOLVED |
