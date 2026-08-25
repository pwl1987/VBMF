# BMD 验收服务器 — MEDIA-SEC-01 runsc 安装与设备透传 Smoke

> **G-RUNTIME 第二级（Deployment SoT §9 / MEDIA-SEC-01）**：gVisor runsc 安装 + DeckLink 设备透传最小验证。
> **EVID-01/02 合规**：本证据独立于"环境准备证据"，`test_subject_sha` 等于实际被测提交，单独成文件。

## 0. 证据标识

| 字段 | 值 |
|---|---|
| 证据类型 | MEDIA-SEC-01 Hardware Smoke（环境级，非 Runtime Acceptance） |
| `environment_base_sha` | `7cc33dde2ab3070c28087df7d0aae570c6c8df5f`（`7cc33dd`） |
| `test_subject_sha` | `7cc33dde2ab3070c28087df7d0aae570c6c8df5f`（`7cc33dd`） |
| 编写提交 | 随 `b427dd8` 之后的本地未提交改动一并记录 |
| 主机 | `10.30.15.10`（lytv，密钥认证） |

## Step 1：安装 runsc（✅ 完成）

**步骤**：从本地离线包 `gvisor-x86_64.tar.bz2`（164MB，含 runsc + containerd-shim-runsc-v1 + gvisor-bin/）传至 BMD，解压安装：
- `runsc` / `containerd-shim-runsc-v1` → `/usr/local/bin/`
- `gvisor-bin/*` → `/opt/gvisor/bin/` 并软链/复制到 `/usr/local/bin/gvisor-bin/`
- `sudo runsc install` 改写 `/etc/docker/daemon.json` 注册 runtime
- `sudo systemctl restart docker`

**结果**：
```text
runsc version release-20260817.0  spec: 1.2.1
/etc/docker/daemon.json:
  "runtimes": { "runsc": { "path": "/usr/local/bin/runsc" } }
docker info Runtimes = ['io.containerd.runc.v2', 'runc', 'runsc']
Default Runtime = runc
```

✅ **runsc 已成功注册进 Docker daemon。** Default 仍为 `runc`，符合 DEPLOY-04（DEV=runc 默认不阻塞开发）。

## Step 2：runsc + /dev/blackmagic 设备透传（✅ PASS）

**镜像源突破**：用户指定国内镜像站 `docker.1ms.run`（毫秒镜像，CDN 智能分发）。已将其配入 BMD `/etc/docker/daemon.json` 的 `registry-mirrors`，重启 docker 后 `nginx:1.27-alpine` 与 `alpine:3.20` 均成功拉取。原"出向 HTTPS 受限"阻塞解除（仅 docker hub 直连不通，镜像站可达）。

**步骤**：使用 `alpine:3.20`，三组对照：
1. `--runtime=runsc --device=/dev/blackmagic:/dev/blackmagic`
2. `--runtime=runc --device=/dev/blackmagic:/dev/blackmagic`（baseline）
3. `--runtime=runsc`（不带 --device，验证无泄漏）

**结果**：
```text
--- inside runsc container (+device) ---
crw-rw-rw- 1 root root 10, 263 Aug 25 15:59 dv0
crw-rw-rw- 1 root root 10, 264 Aug 25 15:59 dv1
crw-rw-rw- 1 root root 10, 265 Aug 25 15:59 io0   (exit=0)

--- inside runc container (+device) ---   (主从次设备号与 runsc 完全一致)
crw-rw-rw- 1 root root 10, 263 dv0
crw-rw-rw- 1 root root 10, 264 dv1
crw-rw-rw- 1 root root 10, 265 io0

--- runsc no-device ---
ls: /dev/blackmagic: No such file or directory   (exit=1)
```

✅ **结论**：
- runsc 下 DeckLink 设备 `dv0/dv1/io0` 完整透传，主/从/次设备号（10,263/264/265）与宿主机一致。
- 不挂 `--device` 时 runsc 容器**不可见** blackmagic → gVisor 无默认暴露宿主设备，符合最小权限。
- runsc 与 runc 设备透传行为一致 → **MEDIA-SEC-01 Option A（runsc）在设备可达性层面成立**。

## 待解决（剩余项）

- [x] **镜像源**：已用 `docker.1ms.run`（毫秒镜像）作为 BMD `registry-mirrors` 永久配置，拉取正常。**记入 BMD 环境约束**（Deployment SoT §9 / ops/README）。
- [x] Step 2：runsc 容器内 `ls /dev/blackmagic` 确认 `dv0/dv1/io0` 可见（对比 runc baseline）—— **PASS**。
- [ ] **Step 3（GStreamer DeckLink open probe）**：需含 gstreamer + decklink 插件的镜像。DeckLink 插件依赖 Blackmagic `Desktop Video` SDK（闭源），通常需自建镜像（在 `blackmagic:decklink` 基础镜像上叠 gstreamer + 项目媒体 agent）。建议：
  - 先基于 `docker.1ms.run` 拉 `ubuntu:22.04` + apt 装 gstreamer + Blackmagic SDK 构建媒体镜像，再 `gst-launch-1.0 decklinkvideosrc device-number=0 ! fakesink` 做 open 验证；
  - 该步较重，是否现在做取决于验收节奏（见下）。

## 决策影响

- Step 2 PASS → **MEDIA-SEC-01 Option A（runsc）在设备可达性层面成立**，可继续沿 runsc 路线。但**最终 Gate 2 锁定**仍需 Step 3（真实 decklink 驱动 open/capture）与 GStreamer 运行时验证。
- 当前 compose 分层（dev=runc / acceptance=runsc / prod=runc+seccomp）维持为待 Step 3 结果的占位（DEPLOY-04）。Step 3 PASS 后 acceptance 的 `runsc` 可正式敲定；prod 是否切 runsc 视捕获稳定性再定。
- **BMD 出向约束正式明确**：docker hub 直连不通，仅经 `docker.1ms.run` 镜像可拉公共镜像。该约束需写进 Deployment SoT §9 与 ops/README，避免后续有人直连 pull 报"环境坏"。
