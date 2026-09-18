# RF-FF-03 — FFmpeg single-input RTMP egress / loopback acceptance

日期：2026-09-18
状态：PLAN FROZEN；实现尚未开始

## 1. 选择与结论

RF-FF-02 已完成 HLS egress/recovery，但 RTMP 仍只有 adapter argv
unit acceptance。下一包补齐同一单输入 FFmpeg Session 的 RTMP output
runtime/recovery/teardown，并在 BMD 上用同一 FFmpeg 构建的临时 RTMP
listener 做 loopback receiver。

候选比较：
- Network Source：BACKLOG；会同时触碰 SourcePlan、网络输入、健康/资源
  语义，超出当前包。
- Program/Packet/Master Switch：依赖第二输入或 Normalize，分别触发
  RH-FLOW/RH-CLOCK，不能越级。
- SRS/Recording/Replay：需要外部 ownership 与持久输出契约，当前没有
  authority 或运行时服务，不能假设已存在。
- RTMP egress：OutputPlan、FFmpeg argv、Session/RecoveryMonitor 已存在，
  只需补 runtime receiver evidence，边界最小，故选择。

## 2. Allowed scope

- 单 input，目标 handle 仍为 exact DeckLink input；
- OutputPlan::Rtmp，一个 rtmp:// target；
- FFmpeg adapter 直接构造 argv，不经过 shell；
- gate-owned ephemeral FFmpeg RTMP listener，仅用于 acceptance fixture；
- 现有 Session/Resource/Lease/RecoveryMonitor owner 与 canonical failure
  path 不变；
- producer child failure 后 same-handle recovery/new child；
- stop/close 后 Released、Resource Available、Lease NONE、monitor exited、
  no FFmpeg orphan；
- receiver 证据必须确认 Video: h264 与 Audio: aac，且 sender/receiver
  在同一 loopback URL 完成连接。

## 3. Forbidden scope

- Network Source、第二 input、Program/Packet/Master Switch；
- multi-output、SRS ownership、外部 RTMP server、Recording/Replay；
- output device-number 2；
- GraphRuntimeIntent、wire、Session/Resource/Lease owner、Clock/FLOW/IDEM
  contract 扩展；
- 24h rss_bounded、长稳态或公网网络质量结论。

## 4. Touch-gates

1. Adapter gate：现有 RTMP target validation/argv tests 必须保持通过。
2. Lifecycle gate：只复用 RF-FF-01F/02 的 canonical recovery path，不新增
   第二套 Supervisor truth。
3. Receiver gate：listener 只绑定 loopback、由 gate 启动并回收；target
   URL 必须受控且无 shell metacharacters。
4. Hardware gate：只使用 manifest exact handle
   46:00000000:002e4500 / device-number 0；output device-number 2 不得
   被枚举选择或写入。
5. Evidence gate：软件、CI、BMD 必须绑定 exact source commit；首轮失败若
   发生要保留 RCA，不能把 capability probe 代替正式验收。

## 5. Acceptance matrix

| Gate | Required result |
|---|---|
| Adapter | RTMP argv contains -f flv, exact target; no shell; bad target/cardinality fail closed |
| Software | focused RTMP tests; ffmpeg-backend/full/default/simulation/mock; fmt/clippy/architecture lints |
| CI | exact implementation commit has all 7 required jobs PASS |
| BMD runtime | listener receives h264/aac from exact handle on initial run |
| BMD recovery | producer PID changes after canonical failure; listener receives h264/aac after recovery |
| BMD teardown | Released + Available + Lease NONE + monitor exited + no FFmpeg orphan |
| Boundary | device-number 2 untouched; no claims outside RF-FF-03; RSS debt unchanged |

## 6. Implementation shape

Extend the existing FFmpeg recovery gate in-place with an explicit RTMP mode:
- VBMF_FFMPEG_RTMP_OUTPUT=1
- VBMF_FFMPEG_RTMP_URL=rtmp://127.0.0.1:<port>/live/<stream>
- gate spawns a bounded ffmpeg -rtmp_listen 1 receiver for each producer
  generation, captures its diagnostics, and requires h264/aac before PASS;
- the production adapter remains the only owner of the producer argv;
- recovery and teardown reuse the existing monitor and Session hooks.

The receiver is an acceptance fixture, not a product SRS implementation.
## 7. Verification commands

Development VM:
- cargo fmt --all -- --check
- cargo test --features ffmpeg-backend ffmpeg_rt_03 -- --nocapture
- cargo test --features ffmpeg-backend --quiet
- default/simulation/mock matrices
- cargo clippy --features ffmpeg-backend --all-targets -- -D warnings
- architecture portability, remove-adapters, git diff --check

BMD:
- build exact implementation commit with DeckLink SDK/libclang injection;
- run media-agent-gates with the exact manifest and handle above;
- archive binary, gate log, receiver evidence, URL/handle metadata and SHA256;
- inspect process table after teardown.

Completion is allowed only after STATE, evidence and GitHub main are
synchronized. RTMP argv/unit PASS alone is not hardware/runtime acceptance.
