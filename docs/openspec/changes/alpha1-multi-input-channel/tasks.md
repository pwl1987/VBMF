# Tasks — alpha1-multi-input-channel

> 四栏纪律：每项标注 `Contract` / `Implementation` / `Verification` / `Gate`。

## 1. 多管线编排（session.rs, D10 激活）

- [x] 1.1 `MediaSession.inputs: Vec<InputSummary>` 加法（device_id+handle; `pipeline` 首输入保留兼容）+ create 初始化空 + start 全量回填 `Contract: design D1/D2 / 债务 D10` | `Implementation: 待` | `Verification: Unit——双 plan 会话句柄表` | `Gate: 无`
- [x] 1.2 `start()` 实例化**全部** plans（逐个; 任一失败逆序回滚已建零孤儿）; stop/close 逆序停全部 `Contract: design D1` | `Implementation: 待` | `Verification: Unit——多设备回滚/停止零孤儿` | `Gate: 无`

## 2. Channel 投影 + 输出策略

- [x] 2.1 `SessionRuntimeState` 加法 `channel: String` + `inputs: Vec<InputSummary>` 投影; `ApiSession` 投影; 顶层 8 键不动 `Contract: design D2` | `Implementation: 待` | `Verification: Unit——投影/8 键测试原样` | `Gate: 无`
- [x] 2.2 materialize 输出策略: **仅首 plan 物化输出段**, 其余纯分析（单输出承诺, P1a/P1b gate 不变） `Contract: design D3` | `Implementation: 待` | `Verification: Unit——多设备 intent 仅首 plan 有 outputs` | `Gate: 无`

## 3. 诊断接线 + 控制台

- [x] 3.1 `VBMF_DIAG_INPUTS`（默认 1）诊断多输入 `Contract: design D4` | `Implementation: 待` | `Verification: 无 env 行为不变` | `Gate: 无`
- [x] 3.2 控制台输入行 + Channel 聚合状态显示 `Contract: design D4` | `Implementation: 待` | `Verification: Hardware gate A1-06` | `Gate: A1-06`

## 4. Gate 与交付

- [x] 4.1 Hardware Gate A1-01..07（盒上双卡真机） `Contract: proposal 验收场景` | `Implementation: 待` | `Verification: 盒上实跑全 PASS` | `Gate: A1`
- [x] 4.2 既有全回归（P1a+P1b gate+矩阵+lifecycle+loopback+transport）零退化 `Contract: 验收口径` | `Implementation: 待` | `Verification: 盒上全 PASS` | `Gate: BOX`
- [x] 4.3 债务账本 D10 行 CLOSED + review + CI + verify 报告 + archive + PR + merge `Contract: 交付纪律` | `Implementation: 待` | `Verification: PR merged` | `Gate: CI/RELEASE`
