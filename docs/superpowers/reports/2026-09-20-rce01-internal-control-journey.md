# RCE-01 收口报告 — internal Runtime Control 真机命令旅程（RUNTIME-CONTROL-ENTRY-01·2026-09-20）

**Packet**: `RUNTIME-CONTROL-ENTRY-01`（planning §3.68 → RCE-01A §3.69 → RCE-01B 本报告）
**Commit 链**: `dcc5589`（planning）→ `06bc01a`（实现·CI `35543068464` **7/7**）→ `b71e1c6`（RCE-01A STATE 收口）→ 本报告 + §3.70 收口提交
**Evidence**: `evidence/bmd-10.30.15.10/2026-09-20-rce01-internal-control-journey/`（7 文件）+ EVIDENCE-INDEX 行

## 1. RCE-01B 验收矩阵（BMD `lytv@10.30.15.10`·exact `06bc01a`·修复版 installer 安装 `0.1.0-06bc01a`）

- **Provenance**：archive `0cdfa946…`（VM/BMD 双算一致）；embedded SHA = manifest `git_commit_sha` = GitHub main SHA `06bc01afaf…`；binary `172f46ac…`；`current -> 0.1.0-06bc01a`；`var_lib root:root 755`。
- **Service**：`systemctl start` → `/health {"state":"Ready","devices":0}`；journald `internal runtime control listening (network-only; /internal/v1/agent) bind=127.0.0.1:50051`（listener 归属 media-agent 进程）。

| # | 旅程步 | 结果 | 证据 |
|---|---|---|---|
| 1 | `runtime.query` 基线 | sessions 空、Network Resource `available`、`program_switch:null` 诚实缺席 | journey.log §1 |
| 2 | `command.dispatch` `start_session`（授权五元组 rtmp 127.0.0.1:19351/live/probe） | 裁决 `executed`（command_id v5 确定性派生 `fc8449d9…`） | journey.log §2 |
| 3 | **actual state 经查询面** | session `state:running, phase:running`、resource `allocated` | journey.log §3 |
| 4 | **真实媒体物化** | **ffmpeg listener `LISTEN 127.0.0.1:19351`**（service 进程命令旅程产生，非 gates 演示） | journey.log §4 |
| 5 | 两平面红线 | `agent.health`=`Ready` 与命令/会话面独立观察 | journey.log §5 |
| 6 | 幂等 replay | 同 envelope 再发 → `replayed`，command_id/outcome 重放 | journey.log §6 |
| 7 | 独占资源语义 | 会话运行中二次 start → preflight FAIL（ResourceCapacity `Allocated` + LeaseConflict）+ `classification:retryable` 诚实 | journey2.log §9 |
| 8 | 显式 `stop_session` | `executed` → 查询面 session `released`、resource `available`、**listener 19351 释放** | journey4.log |
| 9 | 释放后重启 | 新 session `running`、resource `allocated`（release 路径可复用） | journey4.log |
| 10 | lifecycle 守卫 | 对 Running 会话 `release_session` → 诚实拒绝（`close 仅接受 Released/…，请先 stop 防 pipeline 孤儿`·`classification:permanent`） | journey4.log |
| 11 | SIGTERM 带活会话 | `session stopped` → `network sessions drained count=1` → `graceful shutdown complete (exit 0)`；`Result=success ExecMainStatus=0 NRestarts=0` | journey2.log §10 |
| 12 | 边界 | 19351/50051 零残留、零 ffmpeg 残留、**device-2 PID 992634 全程存活**、`/opt/vbmf-dev` 未触碰、零 `.staging-*`、终态 unit inactive | journey2/4.log |

## 2. RCE-01A 软件面（摘要，详见 STATE §3.69）

`internal_control.rs`（JSON-RPC `/internal/v1/agent` 四方法 + 错误码分层）+ 三生产根接线（`MEDIA_AGENT_RPC_BIND` 默认 `127.0.0.1:50051` 回环·UNWIRED 债关闭）+ prototype `/api/v1/*` 生产 503 契约与 `/health` wire 零变化 + `rpc.rs` 历史 reconciliation + 语义页 §6。Dev VM：focused 11/11、default 332、simulation 332、mock 537、ffmpeg-backend 367+1、clippy×4、fmt、architecture lint、remove-adapters 全 PASS；VM production smoke（真实 ffmpeg-backend binary）四方法 + 503 契约 + SIGTERM exit 0 实证。

## 3. 如实披露

- **session id wire 缝隙**（§3.69 已登记）：查询面 `session-<hex>` 显示形态 vs 命令面 canonical UUID，消费方自行映射；Product API 资源形状归 Fastify 阶段。
- journey2/3 两轮中显式 stop 腿先因编排脚本引号/JSON 紧凑格式空 SID 而未触发（意外留下 `-32602 invalid params` 参数验证实证）；该编排缺陷在 journey4 修正后全腿通过——编排问题非产品代码。
- A/V 内容验证不在本旅程宣称（无 publisher 推流；信号验证属 gates/信号面）——本包证明的是**控制旅程**：命令→Runtime 接受→执行→actual state→停止→恢复。
- BMD journalctl 时区为本地（Sep 21 06:xx/07:xx = UTC Sep 20 22/23 点）。
- VM 侧 8080/18080 被本机孤儿进程占用（check_docs.py 家族），health bind 按契约 error 继续；未杀未知归属进程（§3.66 口径维持）。

## 4. 结论

**RUNTIME-CONTROL-ENTRY-01 全链 COMPLETE**（planning §3.68 + RCE-01A §3.69 + RCE-01B 本报告）："standalone service 能运行但只能靠 gate 演示媒体" → **"真实 standalone `media-agent` 经 canonical internal Runtime Control 创建 Session、启动、观察 actual state、停止、恢复，Runtime 唯一 truth"** 已落地并实机验证。frozen Contract 零修改（EXTERNAL_API_CONTRACT `/internal/v1/*` 命名空间 × TECHNOLOGY_STACK JSON-RPC 同时满足）；RH-IDEM-01 维持 DEFER-UNTIL-CONTROL-PLANE；下一步按依赖序评估 CONTROL-PLANE（Fastify Product `/api/v1/*` + durable idempotency 外部入口）→ VBMF-SDK → WEB-CONSOLE。
