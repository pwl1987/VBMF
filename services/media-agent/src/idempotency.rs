//! Phase 0.7C-4: Idempotency Foundation — **先冻结"什么叫同一个命令"**。
//!
//! 设计原则（终审 2026-08-31 裁定, design.md §0）:
//! - 重点不是"实现一个幂等存储", 而是冻结:
//!   **D9-A** command identity — 同一命令 = 同 command_id + 同 canonical fingerprint;
//!   **D9-B** payload conflict — 同 id 异 payload = ID 复用/语义碰撞 → Conflict
//!           (绝不 replay, 绝不执行第二个 payload);
//!   **D9-C** atomic claim — 锁内原子 check-and-insert (禁 check-then-act 竞态),
//!           first claimant 锁外独占执行 → 终态落表 → 唤醒等待者;
//!   **D9-D** result replay — 重复请求重放 claimant 的原 outcome (Failed 同样 replay);
//!   **D9-E** concurrent duplicate — N 线程同 envelope 恰一次执行。
//! - 两平面分层: 0.7C-3 冻结的 `CommandStatus` (执行状态平面) 零改动;
//!   本模块的 `IdempotentDispatch` (幂等裁决平面) 四出口封闭——
//!   InProgress/AlreadyApplied/Retryable 等细分属 0.7C-5 Error Model, 不在此吞并。
//! - 红线延续: 零 vendor 依赖 / 零 runtime_query 引用 (Query/Command 分离) /
//!   包住 command::dispatch 薄映射, 无命令循环/插件/总线 (禁万能 Executor)。
//!
//! 边界: 进程内内存表 (与 InMemoryLeaseManager 同决策级别); 不做持久化/TTL/容量驱逐。

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};

use crate::command::{
    CommandEnvelope, CommandId, CommandOutcome, CommandRejection, CommandStatus,
};
use crate::command as command_contract;
use crate::session::SessionManager;

/// canonical 命令指纹 — "什么叫同一个命令"的冻结语义 (**D9-A**)。
///
/// 组成 = `kind` 判别式 (snake_case 冻结词表) + `CommandTarget` 的 canonical serde JSON
/// (serde 对 struct/enum 按声明序序列化, Uuid/String 无哈希随机性 → 确定性)。
///
/// **不参与字段 (显式冻结)**:
/// - `command_id` — 查表键本身 (fingerprint 是值的同一性, command_id 是请求实例的同一性);
/// - `issued_at_ms` — 投递时刻元数据 (网络重试会重算, 参与则全部重试被判成冲突);
/// - `requested_by` — 审计标签 (opaque, 非负载语义)。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommandFingerprint(pub String);

/// 纯函数 (确定性): envelope → canonical 指纹。
pub fn fingerprint(env: &CommandEnvelope) -> CommandFingerprint {
    let kind = serde_json::to_string(&env.kind).expect("kind serde (枚举判别式)");
    let target = serde_json::to_string(&env.target).expect("target serde (canonical)");
    CommandFingerprint(format!("{kind}|{target}"))
}

/// 幂等裁决平面 — 本请求 dispatch 的结果 (**与 CommandStatus 执行状态平面分层**)。
///
/// 四出口封闭 (词表快照测试锁定; 终审 §10: Duplicate/Conflict/AlreadyApplied/
/// InProgress/RetryableFailure 的统一错误分类属 0.7C-5 Error Model)。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum IdempotentDispatch {
    /// 本请求是 claimant, 已执行一次 (outcome.status 可为 Executed 或 Failed)。
    Executed(CommandOutcome),
    /// 重复命令 — 重放 claimant 的原 outcome (逐字节相等; Failed 同样 replay)。
    Replayed(CommandOutcome),
    /// 同 command_id 异 payload — ID 复用/语义碰撞: 绝不 replay, 绝不执行。
    Conflict {
        command_id: CommandId,
        /// 表内已占用指纹 (先到的 payload)。
        expected: CommandFingerprint,
        /// 本次请求指纹 (后到的异值 payload)。
        actual: CommandFingerprint,
    },
    /// 形状校验拒绝 (未触 Runtime, 未占 id — 不可执行的请求未进入系统)。
    Rejected(CommandRejection),
}

/// 裁决表记录: fingerprint + 状态 (**D9-C/D** — InFlight → Completed 单向)。
#[derive(Debug, Clone)]
struct Record {
    fingerprint: CommandFingerprint,
    state: RecordState,
}

#[derive(Debug, Clone)]
enum RecordState {
    /// claimant 执行中 (锁外)。
    InFlight,
    /// 终态 — 失败也是结果 (**D9-D**: Failed 同样 replay)。
    Completed(CommandOutcome),
}

/// 命令幂等层 — 包住 `command::dispatch` 薄映射的外包装 (非 Executor)。
///
/// `CommandId → fingerprint → atomic claim → execute once → persist outcome
///  → duplicate replay / conflict` 全链 (终审执行令逐字)。
pub struct CommandIdempotency {
    mgr: Arc<SessionManager>,
    records: Mutex<HashMap<CommandId, Record>>,
    completed: Condvar,
}

impl CommandIdempotency {
    pub fn new(mgr: Arc<SessionManager>) -> Self {
        Self {
            mgr,
            records: Mutex::new(HashMap::new()),
            completed: Condvar::new(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<CommandId, Record>> {
        // 记录表是纯数据, poison 不破坏不变量 — 恢复而非 panic 传染。
        self.records.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 幂等 dispatch — 顺序即契约 (design.md §2 五步):
    /// validate(无锁) → fingerprint → 锁内原子 claim → claimant 锁外执行 →
    /// 终态落表 + notify_all → duplicate replay / payload conflict。
    pub fn dispatch(&self, env: &CommandEnvelope) -> IdempotentDispatch {
        // 1. 形状校验 (纯函数): 拒绝不写表、不占 id。
        if let Err(rej) = command_contract::validate(env) {
            return IdempotentDispatch::Rejected(rej);
        }
        let fp = fingerprint(env);
        // 2. 锁内原子 check-and-insert (单临界区 — 非 check-then-act):
        //    两线程同时到达, 只有一个插入成功成为 claimant。
        enum Claim {
            New,
            DuplicateSame,
            Conflict(CommandFingerprint),
        }
        let claim = {
            let guard = self.lock();
            match guard.get(&env.command_id) {
                None => Claim::New,
                Some(rec) if rec.fingerprint == fp => Claim::DuplicateSame,
                // 同 id 异 payload → 碰撞 (无论表内 InFlight/Completed; 记录零改写)。
                Some(rec) => Claim::Conflict(rec.fingerprint.clone()),
            }
        };
        match claim {
            Claim::Conflict(expected) => IdempotentDispatch::Conflict {
                command_id: env.command_id,
                expected,
                actual: fp,
            },
            Claim::DuplicateSame => {
                // 3. 重复投递: 等待 InFlight 完成 (Condvar) 或直接读取终态 → replay。
                let mut guard = self.lock();
                loop {
                    let completed = match guard.get(&env.command_id) {
                        Some(Record {
                            state: RecordState::Completed(outcome),
                            ..
                        }) => Some(outcome.clone()),
                        _ => None,
                    };
                    if let Some(outcome) = completed {
                        return IdempotentDispatch::Replayed(outcome);
                    }
                    guard = self
                        .completed
                        .wait(guard)
                        .unwrap_or_else(|e| e.into_inner());
                }
            }
            Claim::New => {
                {
                    let mut guard = self.lock();
                    guard.insert(
                        env.command_id,
                        Record {
                            fingerprint: fp,
                            state: RecordState::InFlight,
                        },
                    );
                }
                // 4. claimant 锁外独占执行 (执行期不持 records 锁 — 无关命令不被阻塞);
                //    panic 兜底落终态 Failed, 防等待者死等。
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    command_contract::dispatch(&self.mgr, env)
                }))
                .unwrap_or_else(|_| CommandOutcome {
                    command_id: env.command_id,
                    kind: env.kind,
                    status: CommandStatus::Failed,
                    detail: Some("claimant panicked during execution".into()),
                });
                {
                    let mut guard = self.lock();
                    if let Some(rec) = guard.get_mut(&env.command_id) {
                        rec.state = RecordState::Completed(outcome.clone());
                    }
                }
                self.completed.notify_all();
                IdempotentDispatch::Executed(outcome)
            }
        }
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::command::{CommandKind, CommandTarget};
    use crate::graph_intent::GraphRuntimeIntent;
    use uuid::Uuid;

    /// **红线白盒**: 公开关联函数清单 — 仅 fingerprint/dispatch 二方法 (+类型构造)。
    const PUBLIC_SURFACE_ALLOWLIST: &[&str] = &["fingerprint", "dispatch"];
    const BANNED_VERBS: &[&str] = &[
        "execute_pipeline",
        "configure_backend",
        "run_backend",
        "build_gst",
        "emit",
        "send",
        "publish",
        "spawn",
    ];

    fn world() -> Arc<SessionManager> {
        use crate::adapters::mock::MockProvider;
        use crate::contracts::provider::HardwareProvider as _;
        use crate::port::*;
        let devices: Vec<crate::device::DeviceInfo> = MockProvider
            .discover()
            .expect("mock discover")
            .into_iter()
            .map(|d| d.device)
            .collect();
        let pid = PortIdentity::derive(
            &devices[0].device_id,
            ConnectorType::Sdi,
            PortOrdinal::Known(1),
        );
        let registry = PortRegistry {
            ports: vec![PortInfo {
                device_id: devices[0].device_id,
                provider_binding_ref: None,
                identity: PortIdentity {
                    port_id: pid,
                    connector: ConnectorType::Sdi,
                    ordinal: PortOrdinal::Known(1),
                },
                direction: PortDirection::Input,
                capabilities: PortCapabilities::default(),
                runtime_binding: None,
                signal: SignalStatus::default(),
                content: VideoContentState::Unknown,
            }],
        };
        Arc::new(SessionManager::new(
            crate::resource::SharedResourceRegistry::new(
                crate::resource::ResourceRegistry::derive_from_discovery(&registry),
            ),
            Arc::new(crate::lease::InMemoryLeaseManager::new()),
            Arc::new(Mutex::new(crate::supervisor::Supervisor::new(
                crate::supervisor::RestartPolicy::default(),
            ))),
            Arc::new(crate::adapters::mock::MockBackend),
            Arc::new(devices),
            Arc::new(std::collections::HashMap::new()),
            Some(registry),
            crate::pipeline::MaterializeMode::Diagnostic,
            crate::session::SessionTuning::default(),
        ))
    }

    fn start_env() -> CommandEnvelope {
        let mgr = world();
        let dev = mgr.runtime_state().devices[0].device_id;
        CommandEnvelope {
            command_id: CommandId(Uuid::new_v4()),
            kind: CommandKind::StartSession,
            target: CommandTarget::Session {
                intent: GraphRuntimeIntent {
                    version: "1.0".into(),
                    devices: vec![crate::graph_intent::DeviceIntent {
                        device_id: dev.to_string(),
                        role: "CAPTURE".into(),
                        pipeline: crate::graph_intent::PipelineIntent {
                            source: crate::graph_intent::SourceIntent {
                                kind: "decklink".into(),
                                device_id: dev.to_string(),
                                port_id: None,
                            },
                            sink: crate::graph_intent::SinkIntent {
                                kind: "appsink".into(),
                            },
                        },
                    }],
                },
            },
            issued_at_ms: 0,
            requested_by: "idempotency-test".into(),
        }
    }

    /// **D9-A — fingerprint 语义冻结**。
    #[test]
    fn idem_rt_01_fingerprint_semantics() {
        let base = start_env();
        // 确定性: 同输入多次计算恒等。
        assert_eq!(fingerprint(&base), fingerprint(&base));
        // 不参与字段: issued_at_ms / requested_by 变化 → 指纹不变 (语义冻结)。
        let mut meta = base.clone();
        meta.issued_at_ms = 1_750_000_000_000;
        meta.requested_by = "another-caller".into();
        assert_eq!(
            fingerprint(&base),
            fingerprint(&meta),
            "投递元数据 (issued_at/requested_by) 不得影响指纹"
        );
        // 不参与字段: command_id 变化 → 指纹不变 (值同一性 ≠ 请求实例同一性)。
        let mut rekeyed = base.clone();
        rekeyed.command_id = CommandId(Uuid::new_v4());
        assert_eq!(fingerprint(&base), fingerprint(&rekeyed));
        // 参与字段: kind 变 → 指纹变 (合法 Stop 变体)。
        let mut stop = base.clone();
        stop.kind = CommandKind::StopSession;
        stop.target = CommandTarget::SessionById {
            session_id: crate::session::SessionId(Uuid::new_v4()),
        };
        assert_ne!(fingerprint(&base), fingerprint(&stop));
        // 参与字段: canonical payload (intent) 变 → 指纹变。
        let mut other_payload = base.clone();
        if let CommandTarget::Session { intent } = &mut other_payload.target {
            intent.devices[0].pipeline.sink.kind = "rtmp".into();
        }
        assert_ne!(
            fingerprint(&base),
            fingerprint(&other_payload),
            "同 id 异 payload 必须可区分 (D9-B 的前提)"
        );
    }

    /// **词表快照** — 四出口封闭 (防静默加出口; 新出口须过架构评审)。
    #[test]
    fn idem_rt_01_vocabulary_snapshot() {
        let outcome = CommandOutcome {
            command_id: CommandId(Uuid::nil()),
            kind: CommandKind::StartSession,
            status: CommandStatus::Executed,
            detail: None,
        };
        let cases = [
            (
                IdempotentDispatch::Executed(outcome.clone()),
                "\"verdict\":\"executed\"",
            ),
            (
                IdempotentDispatch::Replayed(outcome),
                "\"verdict\":\"replayed\"",
            ),
            (
                IdempotentDispatch::Conflict {
                    command_id: CommandId(Uuid::nil()),
                    expected: CommandFingerprint("a".into()),
                    actual: CommandFingerprint("b".into()),
                },
                "\"verdict\":\"conflict\"",
            ),
            (
                IdempotentDispatch::Rejected(CommandRejection {
                    code: "empty_requester".into(),
                    detail: "d".into(),
                }),
                "\"verdict\":\"rejected\"",
            ),
        ];
        for (value, tag) in cases {
            let json = serde_json::to_string(&value).expect("serialize");
            assert!(json.contains(tag), "词表快照失配: {json}");
            let back: IdempotentDispatch = serde_json::from_str(&json).expect("roundtrip");
            assert_eq!(value, back);
        }
    }

    /// **红线白盒** — 不可执行性 + Query/Command 分离延续。
    #[test]
    fn idem_rt_01_non_executability_surface() {
        assert_eq!(PUBLIC_SURFACE_ALLOWLIST, &["fingerprint", "dispatch"]);
        for name in PUBLIC_SURFACE_ALLOWLIST {
            for verb in BANNED_VERBS {
                assert!(!name.starts_with(verb), "执行动词禁入幂等面: {name}");
            }
            assert!(
                !name.starts_with("get_") && !name.starts_with("list_"),
                "查询动词禁入命令/幂等面: {name}"
            );
        }
        // 指纹仅由 canonical 字段构成 — vendor/执行细节词禁入
        // (注: "pipeline" 不在禁列 — canonical GraphRuntimeIntent 冻结 schema 键名)。
        let fp = fingerprint(&start_env()).0;
        for banned in [
            "gst",
            "device_number",
            "backend",
            "handle",
            "ffmpeg",
            "alsa",
            "kafka",
            "nats",
            "decklinkvideosrc",
        ] {
            assert!(
                !fp.to_lowercase().contains(banned),
                "执行细节渗入指纹: {banned}"
            );
        }
    }

    /// **D9-C 前置** — validate 拒绝不写表不占 id。
    #[test]
    fn idem_rt_01_validate_rejected_does_not_claim() {
        let mgr = world();
        let idem = CommandIdempotency::new(Arc::clone(&mgr));
        let mut bad = start_env();
        bad.requested_by = String::new();
        let id = bad.command_id;
        match idem.dispatch(&bad) {
            IdempotentDispatch::Rejected(rej) => assert_eq!(rej.code, "empty_requester"),
            other => panic!("期望 Rejected, 实得 {other:?}"),
        }
        assert!(mgr.list().is_empty(), "Rejected 不得触 Runtime");
        // 同 command_id 修正为合法 envelope → 正常 Executed (id 未被占用)。
        let mut good = bad;
        good.requested_by = "fixed".into();
        assert!(matches!(
            idem.dispatch(&good),
            IdempotentDispatch::Executed(_)
        ));
        assert_eq!(mgr.list().len(), 1);
    }

    /// **D9-D** — execute once + replay (Failed 同样 replay)。
    #[test]
    fn idem_rt_01_execute_once_and_replay() {
        let mgr = world();
        let idem = CommandIdempotency::new(Arc::clone(&mgr));
        let env = start_env();
        let first = idem.dispatch(&env);
        let CommandOutcome {
            status: s1,
            detail: d1,
            ..
        } = match &first {
            IdempotentDispatch::Executed(o) => o.clone(),
            other => panic!("期望 Executed, 实得 {other:?}"),
        };
        assert_eq!(s1, CommandStatus::Executed, "detail={d1:?}");
        assert_eq!(mgr.list().len(), 1);
        // 重复投递 → 重放原 outcome (逐字节相等), 会话数不增。
        match idem.dispatch(&env) {
            IdempotentDispatch::Replayed(o) => {
                let expected = CommandOutcome {
                    command_id: env.command_id,
                    kind: env.kind,
                    status: s1,
                    detail: d1,
                };
                assert_eq!(o, expected, "replay 必须逐字节重放原 outcome");
            }
            other => panic!("期望 Replayed, 实得 {other:?}"),
        }
        assert_eq!(mgr.list().len(), 1, "重复命令不得二次执行");
        // Failed 同样 replay: ghost stop 首次 Failed, 重发 Replayed(同 Failed)。
        let ghost = CommandEnvelope {
            command_id: CommandId(Uuid::new_v4()),
            kind: CommandKind::StopSession,
            target: CommandTarget::SessionById {
                session_id: crate::session::SessionId(Uuid::new_v4()),
            },
            issued_at_ms: 0,
            requested_by: "t".into(),
        };
        let failed = match idem.dispatch(&ghost) {
            IdempotentDispatch::Executed(o) => o,
            other => panic!("期望 Executed(Failed), 实得 {other:?}"),
        };
        assert_eq!(failed.status, CommandStatus::Failed);
        match idem.dispatch(&ghost) {
            IdempotentDispatch::Replayed(o) => assert_eq!(o, failed, "Failed 结果同样 replay"),
            other => panic!("期望 Replayed(Failed), 实得 {other:?}"),
        }
    }

    /// **D9-D** — 重复 ≠ 重新执行 (stop 重发放行原 Executed, 非对 Released 再 stop 的 Failed)。
    #[test]
    fn idem_rt_01_stop_replay_not_reexecute() {
        let mgr = world();
        let idem = CommandIdempotency::new(Arc::clone(&mgr));
        let env = start_env();
        assert!(matches!(
            idem.dispatch(&env),
            IdempotentDispatch::Executed(_)
        ));
        let sid = mgr.list()[0].session_id;
        let stop = CommandEnvelope {
            command_id: CommandId(Uuid::new_v4()),
            kind: CommandKind::StopSession,
            target: CommandTarget::SessionById { session_id: sid },
            issued_at_ms: 0,
            requested_by: "t".into(),
        };
        match idem.dispatch(&stop) {
            IdempotentDispatch::Executed(o) => assert_eq!(o.status, CommandStatus::Executed),
            other => panic!("期望 Executed, 实得 {other:?}"),
        }
        // 会话已 Released; 若幂等层错误地重新执行, 这里会是 Failed。
        match idem.dispatch(&stop) {
            IdempotentDispatch::Replayed(o) => {
                assert_eq!(
                    o.status,
                    CommandStatus::Executed,
                    "重复 stop 必须重放原 Executed (detail={:?})",
                    o.detail
                );
            }
            other => panic!("期望 Replayed, 实得 {other:?}"),
        }
    }

    /// **D9-B** — 同 id 异 payload → Conflict (零执行 + 原记录零改写)。
    #[test]
    fn idem_rt_01_payload_conflict() {
        let mgr = world();
        let idem = CommandIdempotency::new(Arc::clone(&mgr));
        let env_a = start_env();
        let fp_a = fingerprint(&env_a);
        assert!(matches!(
            idem.dispatch(&env_a),
            IdempotentDispatch::Executed(_)
        ));
        let sessions_after_a = mgr.list().len();
        // 同 command_id, intent 改变 (sink kind) — ID 复用。
        let mut env_b = env_a.clone();
        env_b.command_id = env_a.command_id;
        if let CommandTarget::Session { intent } = &mut env_b.target {
            intent.devices[0].pipeline.sink.kind = "rtmp".into();
        }
        let fp_b = fingerprint(&env_b);
        assert_ne!(fp_a, fp_b);
        match idem.dispatch(&env_b) {
            IdempotentDispatch::Conflict {
                expected, actual, ..
            } => {
                assert_eq!(expected, fp_a, "expected=表内先到 payload 的指纹");
                assert_eq!(actual, fp_b);
            }
            other => panic!("期望 Conflict, 实得 {other:?}"),
        }
        assert_eq!(mgr.list().len(), sessions_after_a, "Conflict 不得执行");
        // 原记录未被改写: 此后同 A 的重复仍可 replay。
        match idem.dispatch(&env_a) {
            IdempotentDispatch::Replayed(o) => {
                assert_eq!(o.status, CommandStatus::Executed);
            }
            other => panic!("conflict 后原指纹重复仍须 Replayed, 实得 {other:?}"),
        }
        // 再来一次 B → 仍是 Conflict (不因重试变成 replay)。
        assert!(matches!(
            idem.dispatch(&env_b),
            IdempotentDispatch::Conflict { .. }
        ));
    }

    /// **D9-E** — 8 线程 barrier 并发击穿: 恰一次执行, 其余 replay, 会话数 1。
    #[test]
    fn idem_rt_01_concurrent_duplicate_single_execution() {
        use std::sync::Barrier;
        const N: usize = 8;
        let mgr = world();
        let idem = Arc::new(CommandIdempotency::new(Arc::clone(&mgr)));
        let env = Arc::new(start_env());
        let barrier = Arc::new(Barrier::new(N));
        let handles: Vec<_> = (0..N)
            .map(|_| {
                let idem = Arc::clone(&idem);
                let env = Arc::clone(&env);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    idem.dispatch(&env)
                })
            })
            .collect();
        let results: Vec<IdempotentDispatch> =
            handles.into_iter().map(|h| h.join().expect("join")).collect();
        let executed: Vec<&CommandOutcome> = results
            .iter()
            .filter_map(|r| match r {
                IdempotentDispatch::Executed(o) => Some(o),
                _ => None,
            })
            .collect();
        let replayed: Vec<&CommandOutcome> = results
            .iter()
            .filter_map(|r| match r {
                IdempotentDispatch::Replayed(o) => Some(o),
                _ => None,
            })
            .collect();
        assert_eq!(executed.len(), 1, "并发下必须恰一次执行");
        assert_eq!(replayed.len(), N - 1, "其余全部 replay");
        for o in &replayed {
            assert_eq!(*o, executed[0], "replay 与原执行 outcome 全等");
        }
        assert_eq!(
            mgr.list().len(),
            1,
            "并发重复不得创建多个会话 (check-then-act 竞态击穿点)"
        );
        // 无其他出口混入。
        for r in &results {
            assert!(
                matches!(r, IdempotentDispatch::Executed(_) | IdempotentDispatch::Replayed(_)),
                "并发重复不得产生 Conflict/Rejected: {r:?}"
            );
        }
    }
}
