# 目标

P2-E3：`rust-test-matrix` 迁条件 `runs-on`（general pool 最后一个 required job，
P2-E 收口项）。job 特殊点（相对前四个）：

- **6 个 cargo 步骤**（default build + simulation/mock build 门禁 + 三组 test）：
  若沿用逐步骤内联 env 分流，将重复 6 份相同 if 块——改用**单个 prep 步骤写
  `$GITHUB_ENV`**（官方机制，后续步骤自动继承），prep 内同时 fail-closed 校验
  caller 未注入 `RUSTUP_TOOLCHAIN`；
- **upload-artifact 步骤**：self-hosted 侧 `CARGO_TARGET_DIR` 重定向后二进制落在
  checkout 外 `target-testmatrix`，artifact `path:` 需条件表达式分叉（artifact
  名称/语义不变；上传走 runner agent 通道，不受 git 出网窗影响）。

同时补记：2026-09-16 首个故障窗（07:04–07:05 UTC，ci-01，run `35066419711`，
1 rerun 收口）随本 packet STATE 更新入 risk 8。

# 范围

- `.github/workflows/media-agent.yml` 仅 `rust-test-matrix` job；
- `.project/STATE.md`（窗补记 + P2-E3 闭环 + P2-E 收口 + P2-M0 解锁）；
- 本 change 的 Comet 正式产物。

# 非目标

- 其余 6 个 required job 任何改动；
- 测试集/feature 组合增删（6 条 cargo 命令逐字不变）；
- artifact 消费方/名称/保留策略变更（`media-agent-linux-default` 名称不变）；
- 出网缓解方案实施（未裁决；观察窗继续计数）；
- runner 标签 / 安全模型（§15.6 冻结）变更；
- 给 `vbmf-ci` 任何新 sudo/root。

# 验收示例

- Scenario: `rust-test-matrix` 迁条件 `runs-on` 后 push `main`：CI 7/7 required
  全绿；job `runner_name` ∈ {`vbmf-ci-01`,`vbmf-ci-02`}；job log 显示 prep 步骤
  写入 `GITHUB_ENV` 的 Rust env 契约、6 条 cargo 命令全 PASS、artifact 上传
  成功且来源路径为 self-hosted 分支路径。
- Scenario: workflow diff 审计——仅 `rust-test-matrix` job 变化，其余 6 job
  byte-identical；job id/name 仍 `rust-test-matrix`；无 `pull_request_target`；
  fork 路径 dtolnay 保持无 pin；`timeout-minutes: 30`、concurrency、
  `permissions` 不变；6 条 cargo 命令逐字不变。
- Scenario: STATE 更新——2026-09-16 窗（07:04–07:05）补记 risk 8；P2-E3
  COMPLETE（run id、runner identity、观察数据）；P2-E 收口、P2-M0 变 READY。
- Scenario: 稳定性观察继续——本 packet 验证期故障窗四元组记录（或 "0 故障窗"）。

# 约束与不变量

- §15.6 冻结边界全部不变：fork 双通道、concurrency、`timeout-minutes: 30`、
  `permissions: contents: read`、7 required context 名称。
- §3.6 RCA 调用契约：self-hosted 路径显式 `RUSTUP_HOME=/usr/local/rustup`，
  caller `RUSTUP_TOOLCHAIN` 不得存在（fail-closed 拒绝）；`CARGO_HOME` 不用
  root-only `/usr/local/cargo`。
- self-hosted 路径零 install。
- 本 repo public：内网地址/fingerprint/凭据不入 repo/logs/evidence。
- 观察窗只记录不缓解；rerun 收口可接受。

# 决策

- D1（coordinator，沿用已冻结模式）：条件 `runs-on` 表达式与 dtolnay/cache 的
  `runner.environment == 'github-hosted'` 分流逐字沿用。
- D2（coordinator，同 D2 谱系）：fork 路径 dtolnay 保持无 pin 参数——build/test
  断言不依赖 exact rustc 补丁版本。
- D3（coordinator）：持久目录——`CARGO_HOME` 共享 `.cargo-home` + **新
  `CARGO_TARGET_DIR="target-testmatrix"`**（本 job 6 步共享一个目录，与
  GitHub-hosted 单 target 语义对齐；与他 job 目录隔离防 feature 组合互踩）。
- D4（coordinator）：**env 经单个 prep 步骤写 `$GITHUB_ENV`**（替代 6 份内联
  if 块）；prep 步骤 fail-closed：caller 已设 `RUSTUP_TOOLCHAIN` 则直接失败。
- D5（coordinator）：upload-artifact 单步骤条件 `path:` 表达式
  （self-hosted → checkout 外 `target-testmatrix/debug/media-agent`；github-hosted
  → 原 `services/media-agent/target/debug/media-agent`）；artifact
  `name: media-agent-linux-default` 不变。
- D6（coordinator）：2026-09-16 窗补记随本 packet STATE 更新落盘。

# 待解决问题

（无——等待用户确认 Shape。）

# 验证预期

- 本地结构断言：仅 `rust-test-matrix` job diff、其余 6 job byte-identical、
  6 条 cargo 命令逐字不变、`on:` 键 {push, pull_request}。
- push `main` 后 CI 7/7 required 全绿（故障窗 rerun 收口可接受，计数）；job
  runner identity + log 内 prep env 契约 + 6 命令 PASS + artifact 上传证据。
- 独立 Verifier 全项核对；STATE 闭环（P2-E 收口 + P2-M0 READY）。
