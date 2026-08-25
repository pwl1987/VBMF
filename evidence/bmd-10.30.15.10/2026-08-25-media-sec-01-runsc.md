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

## Step 2：runsc + /dev/blackmagic 设备透传（⚠️ 受阻：镜像源不可用）

**步骤**：原计划用最小容器（`alpine` 或 `nginx:1.27-alpine`）以 `--runtime=runsc --device=/dev/blackmagic:/dev/blackmagic` 启动，验证容器内可见 `dv0/dv1/io0`。

**受阻原因（新阻塞项）**：
- BMD 服务器**出向 HTTPS 全面受限**（docker.com / google storage TLS 被 reset，docker hub mirror 如 163/dockerpull.org DNS 或 TCP 不可达）。
- 阿里云 `registry.cn-hangzhou.aliyuncs.com` 仅作为 `registry-mirrors` 配置，但 `nginx` / `library/nginx` 拉取均 `pull access denied`（该 mirror 未开放 library 命名空间或需登录）。
- 结果：`docker pull` 任何公共镜像均失败。**本地开发机也未安装 docker**，无法用 `docker save` 中转。

**结论**：Step 2/3 的 smoke 因**镜像不可得**而暂停，非 runsc 自身问题。runsc 注册已验证，待镜像源就绪后补做。

## 待解决（阻塞项）

- [ ] **镜像源**：BMD 需可用镜像获取渠道。候选：
  - 离线镜像 tar（如你提供 `alpine`/`busybox` 的 `docker save` 包，scp 后 `docker load`）；
  - 或内网/私有 registry 地址（配置 `registry-mirrors` 或 `insecure-registries`）；
  - 或放开 BMD 出向对某一镜像 CDN 的 HTTPS。
- [ ] 镜像就绪后补做：runsc 容器内 `ls /dev/blackmagic` 确认 `dv0/dv1/io0` 可见（对比 runc baseline）。
- [ ] 再补 GStreamer DeckLink open/capture 最小 probe（需含 gstreamer + decklink 插件的镜像，更重，建议离线准备）。

## 决策影响

- 在 Step 2 PASS 前，**MEDIA-SEC-01 仍判定为"未验证"**。
- 当前 compose `compose.acceptance.yml` 的 `media-agent.runtime: runsc` 与 `compose.prod.yml` 的 `runtime: runc` + `seccomp:unconfined` 均为**待 smoke 结果最终锁定**的占位（DEPLOY-04）。
- 不因为 runsc 已注册就宣布 Option A 成立；也不因镜像受阻就跳到 Option B。需先拿到镜像完成透传验证。
