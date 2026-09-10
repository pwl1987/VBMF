# VBMF Media-Agent — A2-8 Normal-Use Preview v0.1（零代码形态）

**VBMF Media-Agent A2-8 Normal-Use / Engineering Preview · v0.1** —— 第一个
「正常使用形态」可运行测试包（R59 · 2026-09-06）。形态 = **诊断模式真实
服务二进制自启双输入**（非 gates 测试二进制）; 全链（bootstrap→设备发现→
manifest 绑定→SessionManager→双输入 program graph→watchdog→transport→
teardown）与 A2-8-04 Final Gate 验证过的 gates 是**同一份 lib 代码**。

身份链见 `IDENTITY`（REV 4b473b3 · bin md5 6a0fa224 · manifest md5
7521d17e · 构建主机 10.30.15.10）。审计与缺口全表 =
`docs/superpowers/reports/2026-09-06-a2-8-05-normal-use-startup-entry-audit.md`。

## 已知限制（如实·不冒充）

| # | 限制 | 归属 |
|---|---|---|
| 1 | **无 A↔B 切换触发入口**（命令词表封闭三命令） | v0.2 控制面扩面轮（审计 §7 提案·待裁决） |
| 2 | **无 switch 状态回读端点**（epoch/Desired/Observed 仅 watchdog 活体行间接可见） | 同上 |
| 3 | 正常形态当前仅诊断模式（Production 命令面 503 待 Control Plane） | 后续 Control Plane 接线 |
| 4 | 无优雅停止/信号处理 → 进程停止 = kill（会话级停止走 API 完整链） | 已知债务 |
| 5 | 输出 = 观测面推进 + 输入管线物化（program 面物化未合流） | 口径如实呈现 |
| 6 | 无 `--version`（以 IDENTITY 文件替代）·日志仅 stdout（脚本重定向） | 打包面已兜 |

## 部署（盒 10.30.15.10）

```bash
mkdir -p ~/a2-8-05-preview-v0.1 && cd ~/a2-8-05-preview-v0.1
# 1) 按本目录内容放置: README/IDENTITY/env.sample/start.sh/stop.sh
# 2) 复制构建产物（构建: cd ~/media-agent-build/services/media-agent &&
#    bash scripts/build-bmd.sh --bin media-agent）
cp ~/media-agent-build/services/media-agent/target/debug/media-agent .
chmod +x media-agent start.sh stop.sh
md5sum media-agent   # 须 == IDENTITY.binary_md5
```

## 启动

```bash
export MEDIA_AGENT_DEVICE_BINDING=$HOME/a2-8-02i-v5.manifest.json
./start.sh
```

（环境样例见 `env.sample`; v0.1 固定 diagnostic+2 输入, 勿改。）

## 验证清单（冒烟验收线）

```bash
BIND=127.0.0.1:8080
curl -s http://$BIND/health                   # 200; state=Ready/Capturing; devices>=2
curl -s http://$BIND/api/v1/runtime           # sessions[].inputs == 2
curl -s http://$BIND/api/v1/events/projection # 非空 (IdentityResolved/SourceMaterialized/...)
grep -c "组 watchdog 活体观测行" media-agent-v0.1.log   # >=1; 行内两路 advancing=true
```

## 停止 / 回滚

```bash
./stop.sh    # 先 POST stop_session（teardown 完整链）再 kill 进程
```

回滚 = `rm -rf ~/a2-8-05-preview-v0.1`（零系统面副作用; 会话资源经
stop_session 归还; manifest/构建产物不在本包内, 不受影响）。

## 会话停止链（stop_session 之后日志应出现）

hook: Program Stop → Tap Detach（**先于** Input Stop）→ 逆序
Backend.stop → allocation/lease/reservation 归还 → Released（零孤儿;
`session.rs:758-817`）。

## 证据

盒上原件 = `~/a2-8-02i-evidence/2026-09-06-a2-8-05-v0.1-smoke/`
（origin）; 入库审计副本 = `evidence/bmd-10.30.15.10/a2-8-05-v0.1-smoke/`
（本地验证口径·非 GitHub CI）。
