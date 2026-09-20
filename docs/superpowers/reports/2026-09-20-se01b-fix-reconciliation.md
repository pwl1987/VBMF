# SE-01B-FIX 收口报告 — standalone deployment provenance + failure atomicity + systemd correctness（2026-09-20）

**Packet**: `SE-01B-FIX`（STATE §3.66；用户 2026-09-20 bounded packet，不回退 Runtime/RTMP/SE-01A/C/D 任何已完成工作）
**Commit 链**: `f14cede`（STATE 降级 + §3.66 登记 + PortId collision 入 Active Risk/BACKLOG）→ `a238558`（实现）→ 本报告 + STATE 收口提交
**CI**: `a238558` → GitHub Actions `35541709917` **7/7 required PASS**
**Evidence**: `evidence/bmd-10.30.15.10/2026-09-20-se01b-fix-reconciliation/`（12 文件）+ EVIDENCE-INDEX 行

## 1. 缺陷 → 修复映射

### 缺陷1：Git working tree 口径（`?? .zcodeignore`）

- 处置：`/.zcodeignore` 追加进 checkout-local `.git/info/exclude`（harness 本地文件保留；repo `.gitignore` 未动——该文件非全体开发者共享 policy）。
- 结果：`git status --porcelain --untracked-files=all` = **空**（本地与收口提交时双向复核）。

### 缺陷2：systemd unit StartLimit 落错 section（rate limit 实际未生效）

- 修复：`StartLimitIntervalSec=30` + `StartLimitBurst=3` 移入 `[Unit]`（systemd ≥230 语义；BMD 259 实证 `[Service]` 位置为 unknown key 被忽略）；`Restart=on-failure` 留 `[Service]`；注释更正为 "30s 窗口内最多 3 次的 rate limit（非指数 backoff）"。
- 验证：`systemd-analyze verify` 新 unit（archive 树 + 已装 `/etc/systemd/system/` 双位置）**零 warning / 零 unknown key**；旧 unit 同机复现 `line 21: Unknown key 'StartLimitIntervalSec'`（与 BMD 历史 `se01b-unit.log` 一致，历史证据保留未覆盖）。
- **failure-first smoke**：错误 machine-pin env（确定性 fail-closed）→ `systemctl start` → 恰 **3 次** `Main process exited status=2`（`network binding manifest machine_id mismatch`）→ restart counter 1/2/3 → `Start request repeated too quickly` → `ActiveState=failed`。恢复正确 env + `systemctl reset-failed` → start → `/health {"state":"Ready","devices":0}`。**SIGTERM exit 0 无 restart**：`Result=success ExecMainStatus=0 NRestarts=0`，3s 后仍 inactive（MainPID→0）。重启再 Ready。

### 缺陷3：install provenance 不满足 frozen S6 "exact SHA"

- 修复（fail-closed）：`gzip -dc archive | git get-tar-commit-id` 读取 archive embedded **full 40-hex SHA**；非 git-archive 产物 / 无 embedded id → 拒装；version-tag 的 short SHA 非 full SHA 前缀 → 拒装；manifest 新增 `git_commit_sha`（保留 archive/binary/gates sha256、features、ci_run_id）；installer 内 manifest 落盘后 digest/provenance self-check（含 archive embedded SHA 二次读取复核）。
- `switch-current.sh`：manifest `version_tag` == 请求 tag；`git_commit_sha` 必须合法 40-hex（**legacy manifest 拒绝**）；tag short SHA 前缀一致；binary digest 一致——手工伪造 tag 不能成为 deployment Authority。
- **BMD 实证链（六环）**：GitHub main SHA `a238558ac0e2…` → BMD embedded SHA 同值 → archive sha256 `341f0822…`（VM/BMD 双算一致）→ manifest `git_commit_sha` 同值 → binary `fd2412d2…`（独立复核）→ `current` symlink。
- 拒绝矩阵实证：bad archive rc=2 / 伪造 shortsha `0000bee` rc=2 / 重装同 tag rc=2（manifest 字节不变）/ legacy manifest rc=2 / 改名隔离目录（version_tag 错位）rc=2——全部 `/opt/vbmf` 零变化、零 `.staging-*` 残留。

### 缺陷4：installer failure atomicity（RCA-1 实证半安装目录）

- 修复（staging 协议）：`/opt/vbmf/.staging-<tag>-<pid>` 同文件系统完成 全部 步骤（provenance 验证 → extract → build → bin/ → var-lib 准备 → manifest → self-check）；`trap cleanup EXIT` + HUP/INT/TERM 落 EXIT 清理，失败自动 `rm -rf` staging；promote 前显式 `[ ! -e DEST ]` fail-closed（防 rename(2) 对已存空目录静默成功）→ `mv -T` 原子落成 → 再原子切 `current`。失败路径上正式 `<tag>` 目录**完全不出现**、`current` 不变。
- **VM failure matrix 37/37**（`ops/standalone/tests/install-failure-matrix.sh`，stub cargo + 测试观测缝 `VBMF_INSTALL_ROOT`/`VBMF_VAR_LIB_DIR`，生产默认不变）：bad archive / 前缀不符 / 构建失败 → final dir 不出现 + staging 清理；成功 → manifest 逐字段复核；重装拒绝 + 字节不变；四类 switch 篡改拒绝；current 全程不变；双向回滚。

### S7 权限再核（不扩权）

- `/var/lib/vbmf` = `root:root 755`（owner/mode 写入 install-manifest：`var_lib_path` + `var_lib_owner_mode` 字段）；未赋任何多余写权限；README 明示未来 Runtime 写入须另过 Authority。`/etc/vbmf/network-binding.json` 恢复后 `-rw------- lytv lytv`（0600 + service-user）复核。

## 2. 部署现场处置（如实披露）

- **legacy `0.1.0-0228849` 隔离改名**（`legacy-se01b-0.1.0-0228849`）：原 manifest 无 `git_commit_sha`（frozen S6 不合规实物）。内容字节保留（binary `fd2412d2…` 隔离前后一致）；随后以修复版 installer + 0228849 archive 正规重装同 tag 作为 provenance-verified 回滚伙伴（binary 同 digest——两 commit 间 Rust 源零变化的确定性构建佐证）。改名目录被 switcher 以 version_tag 错位永久拒绝（失效化）。`0.1.0-6932d5e` 原样保留（legacy 拒绝测试靶）。
- **build-failure 注入仅 VM stub 覆盖**（BMD 未烧一次真实编译窗口；语义由 VM matrix + trap 机制证明）。
- **installer 内 self-check 的"安装中触发"分支**无独立注入手段（需文件系统中途损坏）；防御纵深每成功安装必执行，失败分支由 archive-provenance mismatch + switch 面篡改矩阵覆盖。
- BMD 终态：`current -> 0.1.0-a238558`；unit inactive + disabled（演练不常开）；`/etc/vbmf` 恢复 0600；device-2 PID 992634 全程存活；`/opt/vbmf-dev` 未触碰；零 ffmpeg/listener/staging 残留。
- verify 输出中两行 `xfs_scrub*` CPUAccounting 提示为 BMD 宿主既有系统单元噪声（verify 加载依赖带出），与本 unit 无关。

## 3. 边界

- Runtime/RTMP 源码零改动（`a238558` diff 仅 `ops/standalone/` 5 文件）；frozen Authority 未触碰；`ops/` 全栈占位未动；无 Web Console/Fastify/SDK 建设。
- PortId collision（Device smoke 两 warning）按指令**未修**，已登记 STATE §8 Active risks + §5.1 `PORT-COLLISION-01` BACKLOG。
- `scripts/check_docs.py` 本 VM hang + 孤儿进程维持 Verification Debt 登记（§9），未作为门禁、未杀进程。

## 4. 结论

SE-01B-FIX 验收清单全项 PASS（§3.66 DoD）：shell/static + `systemd-analyze verify` 零 warning + installer focused failure matrix + Dev VM regression + CI 7/7 + BMD（exact-commit 安装 / exact-SHA provenance / failed install 不污染 / valid install / StartLimit 注入 / reset-failed 恢复 / SIGTERM exit 0 / rollback / device-2 / `/opt/vbmf-dev`）。**SE-01B 恢复无遗留 COMPLETE；STANDALONE-ENTRY-01 恢复全链 COMPLETE**（SE-01A/C/D/B + 本 reconciliation）。下一 Work Packet：`RUNTIME-CONTROL-ENTRY-01`（PLAN / RECONCILIATION ONLY，用户已裁定）。
