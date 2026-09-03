//! A2-8-02-E: Program Execution Runtime——program 执行资源的**生命周期
//! 唯一 owner**（creator=destroyer, 第七轮终裁 §12.4）。
//!
//! A2-8-01 时组合根散持 group/switcher/graph/watchdog 四件（"创建的人
//! 不是销毁的人"风险）; 本对象统一治理并经 `SessionStopHook` 抽象缝接入
//! Session 停止链——**SessionManager 不理解 GStreamer/Program**（缝在
//! session.rs, 语义在.program_execution 层）。
//!
//! 生命周期序（终裁冻结）:
//! - 创建: attach taps（input 侧）→ build graph → start program →
//!   （组合根 spawn watchdog 后 `set_watchdog_stop` 注旗）;
//!   任一步失败 → **部分资源清理**（已挂 tap detach + 已建 graph stop）
//!   后返回 Err（组合根据此回滚整个会话——input/lease/resource 归
//!   SessionManager 既有机制）。
//! - 停止（`teardown`, 经 hook 于 Input 停止前触发）: watchdog 旗置位 →
//!   Program Stop → Tap Detach; **幂等**; 各步失败只记录不阻断其余步。
//!
//! 不做: 不切换（显式 Intent 链不变）·不恢复输入（Supervisor 链不变）·
//! 不持有 Session 语义（SessionInput 原样）。

use std::sync::{Arc, Mutex};

use crate::contracts::media_tap::{MediaTapPort, MediaTapRequest, TapPlanes};
use crate::contracts::switch::SwitchExecutionAdapter;
use crate::pipeline::PipelineHandle;
use crate::session::{SessionId, SessionStopHook};
use crate::switch_execution::{ExecutionGroup, SwitchError};

/// input 侧 tap 接线请求（channel 由组合根从设备标识派生——本模块不理解
/// 其构成; 02-F 真机桥接消费）。
pub struct TapWiring {
    pub input: PipelineHandle,
    pub channel: String,
}

struct Inner {
    group: Arc<Mutex<ExecutionGroup>>,
    switcher: Arc<dyn SwitchExecutionAdapter>,
    graph: PipelineHandle,
    /// 已挂 tap 簿记（input handle + channel）——teardown 时 detach。
    taps: Vec<(PipelineHandle, String)>,
    tap_port: Option<Arc<dyn MediaTapPort>>,
    watchdog_stop: Option<Arc<std::sync::atomic::AtomicBool>>,
}

/// Program Execution Runtime（组合根装配后为 program 资源唯一 owner）。
pub struct ProgramExecutionRuntime {
    session_id: SessionId,
    inner: Mutex<Option<Inner>>,
}

impl std::fmt::Debug for ProgramExecutionRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgramExecutionRuntime")
            .field("session_id", &self.session_id)
            .field("active", &self.is_active())
            .finish()
    }
}

impl ProgramExecutionRuntime {
    /// 组合根装配（creator=destroyer）。任一步失败 → 部分资源清理后 Err。
    pub fn create(
        session_id: SessionId,
        group: ExecutionGroup,
        switcher: Arc<dyn SwitchExecutionAdapter>,
        tap_port: Option<Arc<dyn MediaTapPort>>,
        tap_wirings: Vec<TapWiring>,
    ) -> Result<Self, SwitchError> {
        // 步 1: input 侧 tap attach（失败 → 已挂部分全部 detach）。
        let mut attached: Vec<(PipelineHandle, String)> = Vec::new();
        if let Some(port) = tap_port.as_ref() {
            for w in &tap_wirings {
                let req = MediaTapRequest {
                    channel: w.channel.clone(),
                    planes: TapPlanes::Both,
                };
                match port.attach_media_tap(&w.input, &req) {
                    Ok(()) => attached.push((w.input, w.channel.clone())),
                    Err(e) => {
                        for (h, ch) in &attached {
                            let _ = port.detach_media_tap(h, ch);
                        }
                        return Err(SwitchError::Backend(format!(
                            "tap attach 失败（部分已清理）: {e}"
                        )));
                    }
                }
            }
        }
        // 步 2+3: graph 物化 + 启动（失败 → tap 清理 + 已建 graph 停止）。
        let group = Arc::new(Mutex::new(group));
        let graph = {
            let g = group.lock().unwrap();
            switcher.build_program_graph(&g)
        };
        match graph {
            Ok(graph) => match switcher.start_program(&graph) {
                Ok(()) => Ok(Self {
                    session_id,
                    inner: Mutex::new(Some(Inner {
                        group,
                        switcher,
                        graph,
                        taps: attached,
                        tap_port,
                        watchdog_stop: None,
                    })),
                }),
                Err(e) => {
                    Self::cleanup_partial(&switcher, Some(&graph), &attached, tap_port.as_ref());
                    Err(e)
                }
            },
            Err(e) => {
                Self::cleanup_partial(&switcher, None, &attached, tap_port.as_ref());
                Err(e)
            }
        }
    }

    /// 部分资源清理（创建失败路径）: 已建 graph 停止 + 已挂 tap detach。
    fn cleanup_partial(
        switcher: &Arc<dyn SwitchExecutionAdapter>,
        graph: Option<&PipelineHandle>,
        attached: &[(PipelineHandle, String)],
        tap_port: Option<&Arc<dyn MediaTapPort>>,
    ) {
        if let Some(g) = graph {
            if let Err(e) = switcher.stop_program(g) {
                tracing::warn!(error = ?e, "A2-8-02-E 创建失败清理: graph 停止失败（残留风险已记录）");
            }
        }
        if let Some(port) = tap_port {
            for (h, ch) in attached {
                if let Err(e) = port.detach_media_tap(h, ch) {
                    tracing::warn!(error = ?e, channel = %ch, "A2-8-02-E 创建失败清理: tap detach 失败");
                }
            }
        }
    }

    /// 停止序: watchdog 旗 → Program Stop → Tap Detach。幂等（已 teardown
    /// = no-op）。各步失败只记录——**不因 Program 停止失败截断 Session
    /// 停止链**（hook 调用方保证; 本函数不向上传播错误）。
    pub fn teardown(&self) {
        let Some(inner) = self.inner.lock().unwrap().take() else {
            return;
        };
        if let Some(flag) = &inner.watchdog_stop {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if let Err(e) = inner.switcher.stop_program(&inner.graph) {
            tracing::warn!(error = ?e, "A2-8-02-E teardown: Program Stop 失败（记录不阻断 Tap Detach）");
        }
        if let Some(port) = inner.tap_port.as_ref() {
            for (h, ch) in &inner.taps {
                if let Err(e) = port.detach_media_tap(h, ch) {
                    tracing::warn!(error = ?e, channel = %ch, "A2-8-02-E teardown: Tap Detach 失败");
                }
            }
        }
        tracing::info!(
            session = %self.session_id.0,
            graph = inner.graph.0,
            "A2-8-02-E Program Execution Runtime teardown 完成（Program Stop→Tap Detach）"
        );
    }

    /// program 执行资源是否仍存活。
    pub fn is_active(&self) -> bool {
        self.inner.lock().unwrap().is_some()
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// 组合根接线访问器（watchdog spawn 需要 graph/group/switcher）。
    pub fn graph_handle(&self) -> Option<PipelineHandle> {
        self.inner.lock().unwrap().as_ref().map(|i| i.graph)
    }

    pub fn group_arc(&self) -> Option<Arc<Mutex<ExecutionGroup>>> {
        self.inner.lock().unwrap().as_ref().map(|i| i.group.clone())
    }

    pub fn switcher_arc(&self) -> Option<Arc<dyn SwitchExecutionAdapter>> {
        self.inner
            .lock()
            .unwrap()
            .as_ref()
            .map(|i| i.switcher.clone())
    }

    /// watchdog spawn 后注入停止旗（teardown 置位）。
    pub fn set_watchdog_stop(&self, flag: Arc<std::sync::atomic::AtomicBool>) {
        if let Some(inner) = self.inner.lock().unwrap().as_mut() {
            inner.watchdog_stop = Some(flag);
        }
    }
}

impl SessionStopHook for ProgramExecutionRuntime {
    fn on_session_stopping(&self, id: &SessionId) -> Result<(), String> {
        if *id == self.session_id {
            self.teardown();
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::adapters::mock::{MockBackend, MockMediaTapPort};
    use crate::adapters::switch_mock::MockSwitchExecutionAdapter;
    use crate::contracts::backend::MediaBackend;
    use crate::contracts::media_tap::MediaTapPort;
    use crate::pipeline::PipelinePlan;
    use crate::session::{SessionId, SessionInput};
    use crate::switch_execution::SwitchDesired;
    use uuid::Uuid;

    fn dual_group(a: Uuid, b: Uuid) -> ExecutionGroup {
        let backend = MockBackend;
        let h1 = backend.instantiate(&PipelinePlan::self_test()).unwrap();
        let h2 = backend.instantiate(&PipelinePlan::self_test()).unwrap();
        ExecutionGroup::new(
            SessionId(Uuid::new_v4()),
            vec![
                SessionInput {
                    device_id: a,
                    handle: h1,
                },
                SessionInput {
                    device_id: b,
                    handle: h2,
                },
            ],
            a,
        )
        .unwrap()
    }

    struct FailingSwitcher;
    impl SwitchExecutionAdapter for FailingSwitcher {
        fn build_program_graph(
            &self,
            _group: &ExecutionGroup,
        ) -> Result<PipelineHandle, SwitchError> {
            Err(SwitchError::Backend("注入: graph 物化失败".into()))
        }
        fn start_program(&self, _g: &PipelineHandle) -> Result<(), SwitchError> {
            Ok(())
        }
        fn switch(
            &self,
            _g: &PipelineHandle,
            _p: &crate::switch_execution::SwitchExecutionPlan,
        ) -> Result<crate::contracts::switch::SwitchExecuted, SwitchError> {
            unreachable!("失败注入不用于切换")
        }
        fn observe(&self, _g: &PipelineHandle) -> crate::contracts::switch::ProgramObservation {
            unreachable!("失败注入不用于观测")
        }
        fn stop_program(&self, _g: &PipelineHandle) -> Result<(), SwitchError> {
            Ok(())
        }
    }

    #[test]
    fn program_exec_rt_01_create_teardown_idempotent() {
        // 正常全序: create[taps attach→graph→start] → teardown[Program
        // Stop→Tap Detach] 幂等; 观测面可证（tap 簿记清空+graph 停止后
        // observe 归零——非仅内部标志）。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let group = dual_group(a, b);
        let h1 = group.inputs[0].handle;
        let h2 = group.inputs[1].handle;
        let sid = SessionId(Uuid::new_v4());
        let switcher = Arc::new(MockSwitchExecutionAdapter::new());
        let taps = Arc::new(MockMediaTapPort::new());
        let runtime = ProgramExecutionRuntime::create(
            sid,
            group,
            switcher.clone(),
            Some(taps.clone()),
            vec![
                TapWiring {
                    input: h1,
                    channel: format!("dev-{}-raw", a),
                },
                TapWiring {
                    input: h2,
                    channel: format!("dev-{}-raw", b),
                },
            ],
        )
        .expect("创建成功");
        assert!(runtime.is_active());
        assert_eq!(
            taps.tap_attachments(&h1).len() + taps.tap_attachments(&h2).len(),
            2
        );
        let graph = runtime.graph_handle().expect("graph 在");
        assert!(
            switcher.observe(&graph).observed_active.is_some(),
            "program 运行中（观测面）"
        );

        runtime.teardown();
        assert!(!runtime.is_active(), "teardown 后失活");
        assert!(
            taps.tap_attachments(&h1).is_empty() && taps.tap_attachments(&h2).is_empty(),
            "Tap Detach 完成（簿记清空）"
        );
        assert!(
            switcher.observe(&graph).observed_active.is_none(),
            "Program Stop 完成（observe 归零）"
        );
        runtime.teardown(); // 幂等
        assert!(!runtime.is_active());
    }

    #[test]
    fn program_exec_rt_01_create_failure_cleans_partial_taps() {
        // 创建失败（graph 物化注入失败）: 已 attach 的 tap 必须全部清理
        // ——零部分资源残留。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let group = dual_group(a, b);
        let h1 = group.inputs[0].handle;
        let h2 = group.inputs[1].handle;
        let taps = Arc::new(MockMediaTapPort::new());
        let err = ProgramExecutionRuntime::create(
            SessionId(Uuid::new_v4()),
            group,
            Arc::new(FailingSwitcher),
            Some(taps.clone()),
            vec![
                TapWiring {
                    input: h1,
                    channel: "dev-f1".into(),
                },
                TapWiring {
                    input: h2,
                    channel: "dev-f2".into(),
                },
            ],
        )
        .expect_err("注入失败应传播");
        assert!(matches!(err, SwitchError::Backend(_)));
        assert!(
            taps.tap_attachments(&h1).is_empty() && taps.tap_attachments(&h2).is_empty(),
            "部分资源已清理（tap 零残留）"
        );
    }

    #[test]
    fn program_exec_rt_01_stop_hook_scoped_to_own_session() {
        // hook 仅对本 session 触发 teardown（他 session 停止零副作用）。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let runtime = ProgramExecutionRuntime::create(
            SessionId(Uuid::new_v4()),
            dual_group(a, b),
            Arc::new(MockSwitchExecutionAdapter::new()),
            None,
            Vec::new(),
        )
        .expect("创建");
        let own = *runtime.session_id();
        let other = SessionId(Uuid::new_v4());
        SessionStopHook::on_session_stopping(&runtime, &other).unwrap();
        assert!(runtime.is_active(), "他 session 停止不触发");
        SessionStopHook::on_session_stopping(&runtime, &own).unwrap();
        assert!(!runtime.is_active(), "本 session 停止触发 teardown");
        let _ = SwitchDesired::ActiveInput(a); // 引用锚（模块语义完整性）
    }
}
