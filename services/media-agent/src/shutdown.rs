//! SE-01A (STANDALONE-ENTRY-01 S3/S4): standalone graceful shutdown.
//!
//! 冻结语义（plan S3）：
//! - SIGTERM / SIGINT → 有序停止：所有活跃 Session 经 SessionManager（唯一
//!   lifecycle owner）逆序 stop（RecoveryMonitor join 与 FFmpeg 子进程回收
//!   由既有 stop 链/reaper/Drop 语义在受控路径上得到执行）→ 进程 exit 0。
//! - 收到第一个终止信号后，SIGTERM/SIGINT 恢复默认处置——**第二次信号 =
//!   立即默认终止的运维逃生门**（不与有序停止竞争）。
//! - SIGHUP → 捕获但无操作并 warn：manifests 是 startup-only（frozen D3），
//!   本 Runtime 不假装支持热加载；SIGHUP 默认动作是终止进程，必须显式
//!   捕获才能实现"无操作"。
//!
//! 信号安全实现：classic self-pipe——handler 只做 `write`（async-signal-safe），
//! 字节值即信号编号；等待线程阻塞在 `read`。信号处置是进程属性，任意线程
//! 收到都会写入同一 pipe；只有安装了本模块的等待线程消费。安装失败路径
//! 必须回滚写端原子（否则 handler 会写向已关闭 fd，等价于关闭整个通道）。

use std::sync::atomic::{AtomicI32, Ordering};

/// 等待线程观察到的关闭原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownCause {
    /// SIGTERM — systemd/编排标准停止信号。
    Terminated,
    /// SIGINT — 交互式 Ctrl+C。
    Interrupted,
}

const SIGTERM_BYTE: u8 = libc::SIGTERM as u8;
const SIGINT_BYTE: u8 = libc::SIGINT as u8;
const SIGHUP_BYTE: u8 = libc::SIGHUP as u8;

/// 信号字节 → 等待语义（纯函数；单测锚定 S3 决策表）。
fn classify_signal_byte(byte: u8) -> Option<ShutdownCause> {
    match byte {
        SIGTERM_BYTE => Some(ShutdownCause::Terminated),
        SIGINT_BYTE => Some(ShutdownCause::Interrupted),
        // SIGHUP 与一切未知字节：不是终止原因（SIGHUP 在 wait 循环内 warn）。
        _ => None,
    }
}

// self-pipe 写端（handler 内原子读；-1 = 未安装）。AtomicI32 的 lock-free
// load 在信号上下文内安全（无锁、无分配）。
static PIPE_WRITE_FD: AtomicI32 = AtomicI32::new(-1);

extern "C" fn self_pipe_handler(sig: libc::c_int) {
    let fd = PIPE_WRITE_FD.load(Ordering::Relaxed);
    if fd >= 0 {
        let byte = sig as u8;
        // write(2) async-signal-safe；EAGAIN（pipe 满）时丢弃——等待线程
        // 只需要一个字节即可醒来。
        unsafe {
            libc::write(fd, &byte as *const u8 as *const libc::c_void, 1);
        }
    }
}

/// 已安装的优雅关闭等待端。进程内至多安装一次（二次 install fail-closed）。
pub struct GracefulShutdown {
    read_fd: i32,
}

impl GracefulShutdown {
    /// 安装 SIGTERM/SIGINT/SIGHUP handler（self-pipe）。必须在进入长等待
    /// 前调用；失败 fail-closed（调用方应拒启或回退显式提示）。
    pub fn install() -> Result<Self, String> {
        let mut fds = [0i32; 2];
        // 写端保持 O_NONBLOCK：handler 写满时丢弃而非阻塞信号上下文；
        // 读端随后改回阻塞——等待线程 park 在 read 上（无空转）。
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) } != 0 {
            return Err(format!(
                "shutdown self-pipe: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut flags = unsafe { libc::fcntl(fds[0], libc::F_GETFL) };
        if flags < 0 {
            return Err(format!(
                "shutdown self-pipe flags: {}",
                std::io::Error::last_os_error()
            ));
        }
        flags &= !libc::O_NONBLOCK;
        if unsafe { libc::fcntl(fds[0], libc::F_SETFL, flags) } != 0 {
            return Err(format!(
                "shutdown self-pipe blocking read: {}",
                std::io::Error::last_os_error()
            ));
        }
        let prev = PIPE_WRITE_FD.swap(fds[1], Ordering::SeqCst);
        if prev >= 0 {
            // 回滚 swap：把写端所有权还给已生效的实例（否则 handler 会写向
            // 本分支即将 close 的 fd，等价于关闭整个 shutdown 通道）。
            PIPE_WRITE_FD.store(prev, Ordering::SeqCst);
            let _ = unsafe { libc::close(fds[0]) };
            let _ = unsafe { libc::close(fds[1]) };
            return Err("shutdown handlers already installed".to_string());
        }

        install_handler(libc::SIGTERM)?;
        install_handler(libc::SIGINT)?;
        install_handler(libc::SIGHUP)?;
        Ok(GracefulShutdown { read_fd: fds[0] })
    }

    /// 阻塞等待终止信号。SIGHUP 只 warn 并继续等待（S3 无操作语义）；
    /// 第一个终止信号到达后立即把 SIGTERM/SIGINT 恢复默认处置——此后
    /// 再发信号直接杀死进程（逃生门），不会回到本函数。
    pub fn wait(&self) -> ShutdownCause {
        loop {
            let mut byte = 0u8;
            let n =
                unsafe { libc::read(self.read_fd, &mut byte as *mut u8 as *mut libc::c_void, 1) };
            if n != 1 {
                // EINTR（无 SA_RESTART 的其它信号）或 EAGAIN 竞态：重试。
                continue;
            }
            if byte == SIGHUP_BYTE {
                tracing::warn!(
                    "SIGHUP received: manifests/config are startup-only (frozen D3); \
                     no reload performed — use SIGTERM for graceful stop or restart the unit"
                );
                continue;
            }
            if let Some(cause) = classify_signal_byte(byte) {
                restore_default(libc::SIGTERM);
                restore_default(libc::SIGINT);
                return cause;
            }
        }
    }
}

impl Drop for GracefulShutdown {
    fn drop(&mut self) {
        // 卸载顺序说明：只复位写端原子并关闭读端；已注册的 handler 保留（其
        // 写端 load 会得到 -1 → no-op）。进程内重新 install 会重注册全部三个
        // 信号，因此 Drop→install 循环（测试形态）始终自洽。
        PIPE_WRITE_FD.store(-1, Ordering::SeqCst);
        unsafe { libc::close(self.read_fd) };
    }
}

fn install_handler(sig: libc::c_int) -> Result<(), String> {
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = self_pipe_handler as *const () as usize;
        action.sa_flags = libc::SA_RESTART;
        if libc::sigaction(sig, &action, std::ptr::null_mut()) != 0 {
            return Err(format!(
                "install handler for signal {sig}: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
    Ok(())
}

fn restore_default(sig: libc::c_int) {
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = libc::SIG_DFL;
        let _ = libc::sigaction(sig, &action, std::ptr::null_mut());
    }
}

/// SE-01A S3：经 SessionManager（唯一 lifecycle owner）逆序停止全部活跃
/// Session。`runtime_state()` 的 sessions 投影来自 HashMap（无创建序），
/// 因此经 `status()` 读取每会话 `created_at` 按创建时间**降序**停止（S3
/// 逆序语义；同毫秒并列以 session_id 降序决出确定序）。只有持运行态
/// （pipeline 已物化/Running/Starting/Degraded）的会话调用 stop；空闲相位
/// （Leased 等）交由进程退出自然消亡，避免 double-stop 噪音。stop 失败只
/// 记录并继续 drain（best-effort，诚实记录）。
///
/// 返回实际执行了 stop 的 session id（按停止顺序）。
pub fn stop_all_sessions(
    manager: &crate::session::SessionManager,
) -> Vec<crate::session::SessionId> {
    use crate::session::SessionPhase;
    let mut live: Vec<(i64, crate::session::SessionId)> = manager
        .runtime_state()
        .sessions
        .iter()
        .filter_map(|session| {
            let id = session.session_id;
            let full = manager.status(&id)?;
            let live = full.pipeline.is_some()
                || matches!(
                    full.phase,
                    SessionPhase::Running | SessionPhase::Starting | SessionPhase::Degraded
                );
            live.then_some((full.created_at, id))
        })
        .collect();
    live.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(b.1 .0.cmp(&a.1 .0)));
    let mut stopped = Vec::new();
    for (_, id) in live {
        match manager.stop(&id) {
            Ok(()) => {
                tracing::info!(session = %id.0, "graceful shutdown: session stopped");
                stopped.push(id);
            }
            Err(error) => tracing::warn!(
                session = %id.0,
                %error,
                "graceful shutdown: session stop failed (continuing drain)"
            ),
        }
    }
    stopped
}

#[cfg(test)]
mod se_01a_tests {
    use super::*;

    #[test]
    fn se_01a_signal_byte_classification_matches_frozen_semantics() {
        // S3 决策表：TERM/INT = 终止；HUP 与未知字节 = 非终止（HUP 在 wait 内 warn）。
        assert_eq!(
            classify_signal_byte(libc::SIGTERM as u8),
            Some(ShutdownCause::Terminated)
        );
        assert_eq!(
            classify_signal_byte(libc::SIGINT as u8),
            Some(ShutdownCause::Interrupted)
        );
        assert_eq!(classify_signal_byte(libc::SIGHUP as u8), None);
        assert_eq!(classify_signal_byte(0), None);
        assert_eq!(classify_signal_byte(255), None);
    }

    #[test]
    fn se_01a_real_signal_lifecycle_singleton_hup_noop_terminate() {
        // 单一测试串行覆盖真实信号路径（pipe/handler 是进程级 singleton，
        // 并行测试会互相消费字节）：install → 二次 install fail-closed →
        // SIGHUP 仅 warn（wait 不返回）→ SIGTERM 终止 → Drop 后可重装。
        let shutdown = GracefulShutdown::install().expect("first install");
        assert!(
            GracefulShutdown::install().is_err(),
            "second install must fail closed"
        );
        unsafe {
            libc::raise(libc::SIGHUP);
            libc::raise(libc::SIGTERM);
        }
        let cause = shutdown.wait();
        assert_eq!(cause, ShutdownCause::Terminated);
        drop(shutdown);
        // Drop 后允许重新安装（测试隔离；生产单次）。
        let again = GracefulShutdown::install().expect("reinstall after drop");
        drop(again);
    }
}

#[cfg(all(test, feature = "mock"))]
mod se_01a_mock_tests {
    use super::stop_all_sessions;
    use crate::adapters::mock::MockBackend;
    use crate::session::SessionManager;
    use std::sync::{Arc, Mutex};

    fn network_manager(sources: &[crate::source::NetworkSourceId]) -> SessionManager {
        let mut registry = crate::resource::ResourceRegistry::new();
        for source in sources {
            registry.register_network_source(*source);
        }
        let resources = crate::resource::SharedResourceRegistry::new(registry);
        let event_log: Arc<dyn crate::events::RuntimeEventSink> =
            Arc::new(crate::events::FanoutSink::new(
                Arc::new(crate::events::RuntimeEventLog::new()),
                Arc::new(crate::events::RuntimeEventLog::new()),
            ));
        let supervisor = Arc::new(Mutex::new(crate::supervisor::Supervisor::new(
            crate::supervisor::RestartPolicy::default(),
            event_log.clone(),
        )));
        SessionManager::new(
            resources,
            Arc::new(crate::lease::InMemoryLeaseManager::new()),
            supervisor,
            Arc::new(MockBackend),
            Arc::new(Vec::new()),
            Arc::new(Default::default()),
            None,
            None,
            crate::pipeline::MaterializeMode::Diagnostic,
            crate::session::SessionTuning::default(),
            event_log,
        )
    }

    fn rtmp_intent(
        source_id: crate::source::NetworkSourceId,
        port: u16,
    ) -> crate::graph_intent::GraphRuntimeIntent {
        crate::graph_intent::GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![crate::graph_intent::DeviceIntent {
                device_id: "network-source-node".into(),
                role: "CAPTURE".into(),
                pipeline: crate::graph_intent::PipelineIntent {
                    source: crate::graph_intent::SourceIntent::rtmp(
                        source_id,
                        crate::source::NetworkEndpoint {
                            protocol: crate::source::NetworkProtocol::Rtmp,
                            host: "127.0.0.1".into(),
                            port,
                            path: "/live/source".into(),
                        },
                    ),
                    sink: crate::graph_intent::SinkIntent {
                        kind: "appsink".into(),
                    },
                },
            }],
        }
    }

    #[test]
    fn se_01a_stop_all_sessions_with_no_live_sessions_is_empty() {
        let manager = network_manager(&[]);
        assert!(stop_all_sessions(&manager).is_empty());
    }

    #[test]
    fn se_01a_stop_all_sessions_skips_released_and_stops_live() {
        // 两个网络会话：s1 先 start 后手动 stop（Released），s2 保持 Running。
        // stop_all_sessions 只停 s2（且逆序 drain 语义下无额外噪音）。
        let source_a = crate::source::NetworkSourceId(uuid::Uuid::new_v4());
        let source_b = crate::source::NetworkSourceId(uuid::Uuid::new_v4());
        let manager = network_manager(&[source_a, source_b]);

        let s1 = manager
            .create(rtmp_intent(source_a, 19350))
            .expect("create s1");
        manager.start(&s1).expect("start s1");
        manager.stop(&s1).expect("stop s1");
        let s2 = manager
            .create(rtmp_intent(source_b, 19351))
            .expect("create s2");
        manager.start(&s2).expect("start s2");

        let stopped = stop_all_sessions(&manager);
        assert_eq!(stopped, vec![s2], "only the live session is stopped");
        assert_eq!(
            manager.status(&s2).expect("s2").phase,
            crate::session::SessionPhase::Released
        );
        // 已 Released 的 s1 不被 double-stop（相位保持 Released）。
        assert_eq!(
            manager.status(&s1).expect("s1").phase,
            crate::session::SessionPhase::Released
        );
    }

    #[test]
    fn se_01a_stop_all_sessions_drains_multiple_live_sessions_in_reverse_order() {
        let source_a = crate::source::NetworkSourceId(uuid::Uuid::new_v4());
        let source_b = crate::source::NetworkSourceId(uuid::Uuid::new_v4());
        let manager = network_manager(&[source_a, source_b]);
        let s1 = manager
            .create(rtmp_intent(source_a, 19352))
            .expect("create s1");
        manager.start(&s1).expect("start s1");
        // created_at 是 ms 精度：隔开创建时间保证逆序判定确定性
        // （同毫秒并列时按 session_id 降序，本测试不依赖该并列语义）。
        std::thread::sleep(std::time::Duration::from_millis(3));
        let s2 = manager
            .create(rtmp_intent(source_b, 19353))
            .expect("create s2");
        manager.start(&s2).expect("start s2");

        let stopped = stop_all_sessions(&manager);
        // S3 逆序：最新创建的会话最先停止。
        assert_eq!(stopped, vec![s2, s1]);
        for id in [&s1, &s2] {
            assert_eq!(
                manager.status(id).expect("session").phase,
                crate::session::SessionPhase::Released
            );
        }
    }
}
