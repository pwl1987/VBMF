# 媒体运行时安全模型(Option B)

**状态:** 已冻结(2026-08-26)
**替代方案:** Option A(runsc + gVisor 设备透传)
**决策依据:** MEDIA-SEC-01 Step 3 → `SoT §15` / `DEPLOYMENT_AND_DEV_RUNTIME.md §15.1`
**证据:** `evidence/bmd-10.30.15.10/2026-08-26-media-sec-01-step3.md`

---

## 1. 决策结论

媒体平面(`media-agent`,GStreamer + Blackmagic DeckLink SDK)运行在
**`runc` + 安全加固** 之下 —— 而非 `runsc`(gVisor)。

| Runtime | DeckLink 枚举 | Pipeline 打开 | 判定 |
|---|---|---|---|
| `runc` + helper IPC + shm | ✅ 检测到 3 个设备 | ✅ live / PREROLLED | **PASS** |
| `runsc`(×3 变体:bind、`--cap-add=ALL`、`seccomp=unconfined`) | ❌ 检测到 0 个设备 | ❌ | **FAIL** |

**理由:** "稳定采集 > 容器隔离"。gVisor 未实现 Blackmagic SDK 设备枚举所依赖的
底层接口(`/dev/blackmagic` 上的 ioctl、到 `DesktopVideoHelper` 的
`AF_UNIX` + `SCM_RIGHTS` 共享内存 IPC)。这是 gVisor 的已知局限,不是某个
可加的 docker flag 能解决 —— 三种 runsc 变体失败表现完全相同。Option A 已
**永久关闭**。

---

## 2. 运行时分层(已冻结)

| Profile | Runtime | 加固 |
|---|---|---|
| `dev` | `runc` | 无(便于调试) |
| `acceptance` | `runc` | Option B 基线(见下) |
| `prod` | `runc` | Option B + `read_only: true` + `tmpfs` |

**不要**为媒体平面重新引入 `runsc`。若未来出现更强隔离需求,评估
Kata / 带 VM 的容器,而非 gVisor。

---

## 3. Option B 加固(权威定义)

权威来源:`ops/compose.acceptance.yml` / `ops/compose.prod.yml`
(`media-agent` 服务)。摘要:

```yaml
media-agent:
  runtime: runc
  devices:
    - /dev/blackmagic:/dev/blackmagic          # 设备白名单(非 privileged)
  device_cgroup_rules:
    - 'c 10:* rmw'                              # Blackmagic 主设备号 10 (dv0/dv1/io0)
  cap_drop:
    - ALL
  cap_add:
    - SYS_NICE                                  # 实时采集线程调度
    # SYS_ADMIN 被禁止。仅在 SDK 实测证明确需时最小增补。
  security_opt:
    - no-new-privileges:true
  ipc: host                                     # /dev/shm/com_blackmagicdesign_* SDK IPC
  shm_size: 1g
  # 仅 PROD:
  read_only: true
  tmpfs:
    - /tmp:size=512m,mode=1777                  # GStreamer/FFmpeg 可写临时区
```

### 硬性规则(每次变更都必须保持)
1. **绝不** `privileged: true`。设备访问仅通过显式白名单。
2. **始终** `cap_drop: ALL`,再最小回加(`SYS_NICE`)。`SYS_ADMIN` 被评审策略禁止。
3. **始终** `no-new-privileges: true`。
4. **DeckLink SDK IPC 需要 `ipc: host`** —— plugin 通过
   `/dev/shm/com_blackmagicdesign_*` 与宿主 `DesktopVideoHelper` 通信。这正是
   仅 `--device=/dev/blackmagic` 不够的原因(Step 3 根因)。
5. 默认 Docker seccomp profile 已足够;未来可引用更紧的
   `ops/nginx/seccomp-media.json`,但非必需。

---

## 4. 网络引导依赖(NET-01)

构建/拉取路径依赖可达的镜像源(`docker.1ms.run` 主用,`dockerproxy.net` /
网易 备用)。这是**环境约束 / 引导依赖** —— 见 SoT §9。它**不是**运行时安全
控制,只影响配置阶段的镜像获取。

---

## 5. 明确不在本文件范围
- GStreamer pipeline 内部 → `MEDIA_AGENT_STATE_MACHINE.md`
- Lease / 恢复逻辑 → `MEDIA_AGENT_STATE_MACHINE.md`
- 控制平面(Node/Fastify)安全 → 独立的 INFRA-SEC 文档
