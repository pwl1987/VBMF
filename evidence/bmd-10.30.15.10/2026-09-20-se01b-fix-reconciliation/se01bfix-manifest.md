# SE-01B-FIX — standalone deployment provenance / atomicity / systemd reconciliation @ `a238558`（2026-09-20）

**Packet**: SE-01B-FIX（STATE §3.66 三 closure defects 清偿；用户 2026-09-20 bounded packet）

## 环境

- 工件 commit `a238558`（CI `35541709917` **7/7 required PASS**：architecture-portability / rust-clippy / session-lifecycle / gstreamer-build / rust-format / hardware-test-compile / rust-test-matrix 全 success）；STATE 降级 commit `f14cede`
- archive：`vbmf-a238558.tar.gz` sha256 `341f0822d0e54fc111d33395b786bfa3a7bd17fa564ded4123a4db1b5796401d`；embedded commit（BMD 上 `gzip -dc | git get-tar-commit-id`）= `a238558ac0e27fc8017a2255e0d5ce5afdea8db2` = GitHub live main SHA
- 回滚伙伴：`vbmf-0228849.tar.gz` sha256 `6008f804cbec28c75996c11e0616ba7da566f8713d2833bad23badc5b3280587`（embedded `0228849b82965008a7176fe0c317fa25c6c87e71`）
- BMD：`lytv@10.30.15.10`（systemd 259.5-0ubuntu3.4·git 2.53.0·cargo 1.98.0）
- installer/switcher/unit 一律取自 a238558 archive 展开树（`/tmp/se01bfix/ops/standalone/`）——archive 自带修复工件

## Provenance 证据链（六环机械核验）

```
1. GitHub live main SHA      a238558ac0e27fc8017a2255e0d5ce5afdea8db2   (VM: git rev-parse origin/main)
2. git archive embedded SHA  a238558ac0e27fc8017a2255e0d5ce5afdea8db2   (BMD: gzip -dc a.tar.gz | git get-tar-commit-id)
3. archive sha256            341f0822d0e5…966401d                        (VM 与 BMD 各自 sha256sum 一致)
4. manifest.git_commit_sha   a238558ac0e27fc8017a2255e0d5ce5afdea8db2   (/opt/vbmf/0.1.0-a238558/install-manifest.json)
5. binary sha256             fd2412d270bd4488334df0b9533aac928cd312ab4f1b9c5d39198c6aedf1ad81
                             (manifest 记录 == BMD 独立 sha256sum bin/media-agent 复核)
6. current symlink           /opt/vbmf/current -> /opt/vbmf/0.1.0-a238558
```

install-manifest.json（`0.1.0-a238558`，完整内联；`0.1.0-0228849` 同构）：

```json
{
  "lane": "standalone",
  "version_tag": "0.1.0-a238558",
  "git_commit_sha": "a238558ac0e27fc8017a2255e0d5ce5afdea8db2",
  "archive_sha256": "341f0822d0e54fc111d33395b786bfa3a7bd17fa564ded4123a4db1b5796401d",
  "media_agent_sha256": "fd2412d270bd4488334df0b9533aac928cd312ab4f1b9c5d39198c6aedf1ad81",
  "media_agent_gates_sha256": "d7ccdd774001d1f4ec5ee8c74ee07516c1b4ff09591c4fb0bd445c7665104acf",
  "features": "bmd,ffmpeg-backend",
  "ci_run_id": "35541709917",
  "var_lib_path": "/var/lib/vbmf",
  "var_lib_owner_mode": "root:root 755",
  "installed_at_utc": "2026-09-20T22:30:33Z"
}
```

- `0.1.0-0228849`：`git_commit_sha=0228849b8296…`、`archive_sha256=6008f804…`、`ci_run_id=35493444527`、binary 同 `fd2412d2…`（两 commit 间 Rust 源零变化，BMD 确定性构建佐证：隔离前旧目录 binary 与新装 binary 逐字节同 digest）
- **S7**：`/var/lib/vbmf` owner/mode = `root:root 755` 记入两 manifest；未赋任何多余写权限（未来 Runtime 写入须另过 Authority）；`/etc/vbmf/network-binding.json` 恢复后 `-rw------- lytv lytv`（0600 + service-user）复核

## 验收矩阵逐项

| # | 项 | 结果 | 证据 |
|---|---|---|---|
| 1 | `systemd-analyze verify` 零 warning/零 unknown key | **PASS** | 新 unit（archive 树 + 已装 `/etc/systemd/system/`）rc=0、vbmf-media-agent.service 零输出；对照旧已装 unit 复现 `line 21: Unknown key 'StartLimitIntervalSec' in section [Service]`（与历史 `se01b-unit.log` 一致，保留不覆盖）。verify 附带两行宿主 `xfs_scrub_all/system-xfs_scrub.slice` CPUAccounting 提示为 BMD 既有系统单元噪声，与本 unit 无关 | `bmd-verify.log` |
| 2 | bad archive（无 embedded commit id）拒装 | **PASS** | rc=2 `archive does not embed a git commit id`；`/opt/vbmf` 内容零变化；零 `.staging-*` 残留 | `bmd-failinject.log` |
| 3 | 伪造 tag（shortsha 非 embedded SHA 前缀）拒装 | **PASS** | rc=2 `short SHA '0000bee' is not a prefix of archive embedded commit 'a238558…'`；目录零变化 | `bmd-failinject.log` |
| 4 | 重装同 tag 拒绝且旧目录不变 | **PASS** | rc=2 `refusing to overwrite existing version dir`；manifest sha256 前后一致 | `bmd-refusals.log` |
| 5 | legacy manifest（无 git_commit_sha）拒绝切换 | **PASS** | `0.1.0-6932d5e`：rc=2 `no valid full git_commit_sha (legacy/tampered manifest: 'missing')`；隔离目录（改名）另由 version_tag mismatch 拒绝；current 不变 | `bmd-refusals.log` |
| 6 | valid install ×2 + provenance 链 | **PASS** | 双 manifest 全字段（见上）；链六环闭合；`var_lib=root:root 755` | `bmd-installs.log`/`bmd-refusals.log` |
| 7 | StartLimit 故障注入 | **PASS** | 错误 machine-pin → 恰 **3 次** `Main process exited status=2`（`network binding manifest machine_id mismatch` fail-closed）→ restart counter 1/2/3 → `Start request repeated too quickly` → `ActiveState=failed`（NRestarts=3·Result=exit-code）——无无限快速重启 | `bmd-startlimit-1/2.log` |
| 8 | 恢复配置 + reset-failed → Ready | **PASS** | 恢复 binding（0600 保持）→ `reset-failed` → start → `/health {"state":"Ready","devices":0}` | `bmd-recovery.log` |
| 9 | SIGTERM exit 0 不 restart | **PASS** | stop → `Result=success ExecMainStatus=0 NRestarts=0` + `Deactivated successfully`；3s 后仍 inactive、MainPID→0（无 respawn） | `bmd-sigterm-restart.log` |
| 10 | 重启再 Ready | **PASS** | 二次 start → Ready → stop → inactive（unit 维持 disabled，演练不常开） | `bmd-sigterm-restart.log` |
| 11 | rollback（provenance-verified 双向） | **PASS** | `switch-current 0.1.0-0228849` → `provenance verified: commit=0228849b… digest=ok` → start → Ready → stop exit 0 → 切回 `0.1.0-a238558` 同样 verified | `bmd-rollback-boundary.log` |
| 12 | 边界完整性 | **PASS** | device-2 gst-launch PID **992634** 存活；`/opt/vbmf-dev` 未触碰；零 ffmpeg/listener 残留（19350/19351 无监听）；零 `.staging-*` 残留 | `bmd-rollback-boundary.log` |

## VM 侧（Development VM）

- failure-injection matrix（`ops/standalone/tests/install-failure-matrix.sh`）：**PASS=37 FAIL=0** @ HEAD=`a238558`（bad archive/前缀不符/构建失败 stub/成功完整性逐字段/重装拒绝/四类 switch 篡改拒绝/双向回滚/current 与 staging 断言）——`vm-failure-matrix.log`
- unit 新旧对照 verify（同 systemd 259.5-0ubuntu3.4）：旧 unit 复现 line 21 Unknown key；新 unit（ExecStart 指向临时 stub，隔离路径存在性因素）rc=0 零 warning
- `bash -n` 三脚本通过；working tree `git status --porcelain --untracked-files=all` = 空（`.zcodeignore` 经 checkout-local `.git/info/exclude` 处理，repo `.gitignore` 未动）

## Failure-first 与如实披露

- **隔离 legacy `0.1.0-0228849`（运维动作）**：原目录 manifest 无 `git_commit_sha`（frozen S6 exact-SHA 不合规 = 本包缺陷3 实物），重命名 `/opt/vbmf/legacy-se01b-0.1.0-0228849` 保内容（binary `fd2412d2…` 隔离前后一致），随后以修复版 installer + 0228849 archive 正规重装同 tag。改名目录因 version_tag≠目录名被 switcher 永久拒绝（失效化）。`0.1.0-6932d5e` 原样保留（legacy 拒绝测试靶）。
- **build-failure 注入仅在 VM matrix 覆盖**（stub cargo exit 1）：BMD 未做真构建失败注入（真实构建失败=浪费一次完整编译窗口且语义已由 VM matrix + trap 机制证明）；如实标注。
- **install-time manifest self-check 的失败分支**：在 VM matrix 以 archive-provenance mismatch + switch 面四类篡改覆盖；installer 内 self-check 本身每次成功安装都执行（防御纵深），其"安装中触发"分支无独立注入（需文件系统中途损坏注入，未实现）——如实标注。
- **journalctl 显示 BMD 本地时区**（Sep 21 06:34 = UTC Sep 20 22:34）；`systemctl start` 返回 rc=0 后 unit 即进入 failed（start job 完成 ≠ 运行成功——语义页 §2 口径）。
- BMD 终态：`current -> 0.1.0-a238558`；unit inactive + disabled；`/etc/vbmf` 恢复（0600）；旧 `se01b-unit.log`（含历史 Unknown key warning）未覆盖。

## 文件清单

`bmd-verify.log`·`bmd-failinject.log`·`bmd-quarantine.log`·`bmd-installs.log`·`bmd-refusals.log`·`bmd-startlimit-1.log`·`bmd-startlimit-2.log`·`bmd-recovery.log`·`bmd-sigterm-restart.log`·`bmd-rollback-boundary.log`·`vm-failure-matrix.log`
