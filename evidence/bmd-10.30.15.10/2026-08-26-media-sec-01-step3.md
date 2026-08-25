# BMD 验收服务器 — MEDIA-SEC-01 Step 3（GStreamer + DeckLink open probe）

> **G-RUNTIME 第二级 / MEDIA-SEC-01 Option A vs B 裁决依据**。
> 接续 `2026-08-25-media-sec-01-runsc.md`（Step 1/2 PASS）。
> `test_subject_sha` = 实际被测仓库提交（`a7569e1`），但本验证为**环境级 runtime 探测**，结论与具体 app commit 解耦。

## 0. 证据标识

| 字段 | 值 |
|---|---|
| 证据类型 | MEDIA-SEC-01 Step 3 — 媒体栈设备访问验证 |
| `environment_base_sha` | `a7569e1` |
| `test_subject_sha` | `a7569e1`（runtime 探测，独立于 app 代码） |
| 主机 | `10.30.15.10`（lytv） |
| BMD 既有资产 | `desktopvideo 16.2a1` 已装；`/usr/lib/libDeckLinkAPI.so`；`/usr/lib/x86_64-linux-gnu/gstreamer-1.0/libgstdecklink.so`；`DesktopVideoHelper` 守护进程在跑；`/dev/blackmagic/{dv0,dv1,io0}` 在线 |
| 离线 SDK 包 | `/home/lytv/Blackmagic_Desktop_Video_Linux_16.2.tar`、`/home/lytv/Blackmagic_DeckLink_SDK_16.0.zip`（onboarding 离线备用） |

## 1. 关键机制发现

DeckLink GStreamer plugin（`libgstdecklink.so`）**不靠 `/dev/blackmagic` 字符设备直接枚举设备**，而是通过 **`DesktopVideoHelper` 守护进程的 IPC 通道**发现设备：
- 宿主机 `DesktopVideoHelper` 监听 `/dev/shm/com_blackmagicdesign_DeckLinkDiscoveryNotifier`
- 容器内 plugin 须能访问该 shm socket + `--ipc=host` 才能收到设备列表
- 仅 bind `/dev/blackmagic` 不够（会 `Detected 0 devices`）

## 2. runc 验证（BASELINE / Option B 路径）

命令（最小 probe，无 Rust/Fastify/Lease）：
```bash
docker run --rm --runtime=runc \
  --device=/dev/blackmagic:/dev/blackmagic \
  -v /usr:/usr:ro \
  -v /dev/shm/com_blackmagicdesign_DeckLinkDiscoveryNotifier:/dev/shm/com_blackmagicdesign_DeckLinkDiscoveryNotifier \
  --ipc=host ubuntu:22.04 \
  gst-launch-1.0 -v decklinkvideosrc device-number=0 ! fakesink
```
结果：
```
decklink gstdecklink.cpp:2151:init_devices: Detected 3 devices
Setting pipeline to PAUSED ...
Pipeline is live and does not need PREROLL ...
Pipeline is PREROLLED ...
ERROR: Internal data stream error  (decklinkvideosrc)   ← 预期：fakesink 未消费 + 当前无 SDI 信号
```
✅ **Detected 3 devices + Pipeline is live/PREROLLED** → SDK open 成功、设备可达。符合 Step 3 验收标准（runtime/device/sdk/gstreamer 四项成立；"first frame"因无实时 SDI 信号 + fakesink 协商而未产生 buffer，但 open+live 已证明链路打通）。

## 3. runsc 验证（Option A 候选）— ❌ FAIL

同样 bind（含 shm + host IPC），runtime=runsc：
```
decklink gstdecklink.cpp:2151:init_devices: Detected 0 devices
Failed to set pipeline to PAUSED.
```

**抢救尝试（均仍 Detected 0 devices）**：
- A: `bind /dev/blackmagic`（代替 `--device`）+ shm + ipc
- B: `--device` + `--cap-add=ALL` + `--security-opt seccomp=unconfined`

→ 排除"某个可加的 docker flag 能解决"。根因为 **gVisor (runsc) 对 `libDeckLinkAPI` 枚举所需的底层 syscall / 共享内存 / ioctl 支持不完整**（gVisor 已知局限）。

## 4. 裁决（MEDIA-SEC-01 → Option B）

| Runtime | 设备枚举 | Pipeline open | 判定 |
|---|---|---|---|
| runc + shm/IPC | ✅ 3 devices | ✅ live | **PASS** |
| runsc (×3 变体) | ❌ 0 devices | ❌ | **FAIL** |

**按预定决策树：Step 3 FAIL on runsc → 切 Option B（runc + 其他隔离）。**
理由（用户决策原则）：**稳定采集 > 容器隔离**。广播系统第一优先级是可靠访问 DeckLink，gVisor 的兼容缺口不可接受。

## 5. 后续（Option B 收敛）

- Media Agent runtime 正式定为 **runc**。
- 隔离改为：seccomp profile（`ops/nginx/seccomp-media.json`）+ AppArmor + capability drop + read_only rootfs + `/dev/blackmagic` device allowlist + tmpfs。
- compose 分层更新：dev=runc / acceptance=**runc** / prod=runc（Option B 隔离加固）。
- Step 3 "first frame"（真实 SDI 信号下 buffer 产生）留待媒体 agent 骨架（Gate 2）就绪后在 acceptance 复测。

## 6. 资产/约束记录

- BMD 出向仅 `docker.1ms.run` 可拉公共镜像（NET-01，bootstrap dependency）。
- Blackmagic SDK 离线包在 `/home/lytv/`，作为 onboarding 离线源。
- 宿主机原生（无容器）同 probe 亦 `Detected 3 devices` + `Pipeline is live`，佐证容器化在 runc 下无损。
