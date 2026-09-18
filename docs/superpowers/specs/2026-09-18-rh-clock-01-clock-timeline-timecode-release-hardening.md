# RH-CLOCK-01 — Clock observation timeline + timecode release hardening

日期：2026-09-18  
基线：live `main` `b0a273ff436a503b9ea2c89081b1f36168752fd9`

## 1. Decision

下一唯一 READY bounded packet = `RH-CLOCK-01`。

候选比较结论：

| Candidate | 依赖/触碰 | 裁决 |
|---|---|---|
| Network Source | source intent、protocol-neutral binding、new adapter | BACKLOG，不越级 |
| PACKET/MASTER Switch | compressed/normalize、多输入 program truth | BACKLOG；会触发更大 gate |
| Hot-Standby | source candidates + switch/recovery | BACKLOG |
| Recording/Replay / SRS | new I/O ownership、外部服务语义 | BACKLOG |
| FFmpeg follow-up | RF-FF-03 已完成当前 single-input output/recovery slice | 暂无不扩面的自然缺口 |
| RH-CLOCK-01 | 仅 canonical Clock/Timecode observation semantics | **本包** |
## 2. Authority and dependency

- Authority: `.project/STATE.md` §3.36/§4/§5；`CLOCK_TIMECODE_CONTRACT.md`；D11/D13 debt record。
- Dependency: current live `main`；不依赖 Network Source、Normalize execution、Program Switch、FFmpeg、GStreamer 或 BMD output。
- Gate meaning: 这是 Clock/Timecode 下一触碰点的 hardening packet；完成后才允许进入需要 Clock/Timecode 语义的后续 packet。
- Hardware boundary: 本包不改 provider/backend/DeckLink path，不要求新的 BMD hardware claim；硬件 clock/timecode probe 仍保持未实现/未验证。
## 3. Allowed scope

1. `services/media-agent/src/clock.rs`
   - 增加 bounded, append-only observation timeline；
   - timeline 保留观测顺序、观测时间、Clock state 与 evidence；
   - 满容量必须显式失败，禁止静默丢旧事件；
   - 覆盖 `Locked → ClockLost → ClockRecovered` 序列。
2. `services/media-agent/src/timecode.rs`
   - 将 `observe_transitional` 的非法 presence 从 debug-only assertion 改为 release-safe fail-closed Result；
   - 保持合法 `Discontinuous/Recovered` observation 语义与零 action。
3. 对应 canonical 单元测试、release-build 测试与必要的契约注释/债务状态更新。
## 4. Forbidden scope

- 不改 `GraphRuntimeIntent`、wire vocabulary、API、Session/Resource/Lease owner。
- 不新增 Clock master selection、drift correction、resampling、timestamp correction 或 recovery action。
- 不新增 BMD/GStreamer/FFmpeg probe、DeckLink handle/device-number、Network Source、second input、Program/Packet/Master Switch。
- 不把 `ClockLost` 映射成 pipeline restart；既有 degraded/no-auto-restart 策略不变。
- 不处理 D15 multi-flow 或 durable idempotency；24h `rss_bounded` debt 独立保留。
## 5. Acceptance and verification

- Clock timeline：可构造并序列化 observation；严格验证 `Locked → ClockLost → ClockRecovered`；capacity overflow 返回错误且不改变既有 timeline。
- Timecode：合法 transitional states 成功；非法 state 在 debug 与 release 都返回错误；不产生 value/action。
- Software matrix：focused clock/timecode tests；default test；release test；mock/simulation regression；format；clippy `-D warnings`；architecture portability / remove-adapters / diff-check。
- Contract checks：Clock/Timecode JSON 不出现 vendor/runtime binding；Clock 仍是 observation-only；Graph/wire diff 为零。
- CI：push 后 7 required contexts 全部完成并记录 run。
- Evidence：实现 commit、verification output、STATE 更新；无硬件证据则明确写 `NOT REQUIRED BY SCOPE`，不得伪造 hardware acceptance。
## 6. Exit criteria

只有同时满足实现、软件矩阵、CI、diff 复核和 STATE/GitHub sync，`RH-CLOCK-01` 才能从 READY 转 COMPLETE。若实现发现 timeline 所需契约超出上述 canonical observation 边界，立即停回 PLAN REQUIRED，不得临时扩 scope。
## 7. Verification result

Status: **COMPLETE / SOFTWARE VERIFIED（2026-09-18）**。

- Implementation commit: `6433afb63f73cc28a9fe14390d0152210e961466`。
- Focused: Clock timeline 2/2；Timecode release-hardening 1/1。
- Debug full default: 264/264；simulation: 264/264；mock: 449/449 + integration 9/9 + 12/12。
- Release: focused Clock 2/2 + Timecode 1/1；full default 264/264。
- `cargo fmt -- --check`、clippy `-D warnings`、architecture portability、remove-adapters、`git diff --check` 全部 PASS。
- GitHub Actions run `35387811774`：7/7 required contexts PASS。
- Scope audit：仅 `clock.rs` / `timecode.rs` 改动；Graph/wire/Session/Resource/Lease/Backend/provider/BMD path 零改动。
- Hardware boundary：本包未触碰 provider/backend/DeckLink path，BMD hardware acceptance **NOT REQUIRED BY SCOPE**；clock/timecode hardware probe 仍 NOT IMPLEMENTED/NOT VERIFIED。
