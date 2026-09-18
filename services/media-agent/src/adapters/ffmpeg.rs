//! RF-FF-01B: Live FFmpeg concrete MediaBackend — process lifecycle + SelfTest only.
//!
//! This adapter owns FFmpeg child processes inside Media Agent. It does not accept shell
//! command strings, does not open DeckLink in this packet, and does not implement network sources.

use crate::contracts::backend::MediaBackend;
use crate::pipeline::{
    PipelineError, PipelineHandle, PipelinePlan, SourceBindingClass, NEXT_PIPELINE_ID,
};
use crate::pipeline_events::{BusSeverity, PipelineBusEvent, PipelineBusEventKind};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

struct FfmpegInstance {
    plan: PipelinePlan,
    child: Option<Child>,
    started: bool,
    observe_error_reported: bool,
}

/// Concrete live FFmpeg backend. One instance owns one child-process table.
pub struct FFmpegBackend {
    binary: PathBuf,
    #[cfg(test)]
    test_args: Option<Vec<String>>,
    instances: Mutex<HashMap<PipelineHandle, FfmpegInstance>>,
}

impl Default for FFmpegBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl FFmpegBackend {
    pub fn new() -> Self {
        Self::with_binary("ffmpeg")
    }

    /// Binary path is injectable for deterministic tests; arguments remain backend-owned.
    pub fn with_binary(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            #[cfg(test)]
            test_args: None,
            instances: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    fn with_test_command(binary: impl Into<PathBuf>, args: &[&str]) -> Self {
        Self {
            binary: binary.into(),
            test_args: Some(args.iter().map(|s| (*s).to_string()).collect()),
            instances: Mutex::new(HashMap::new()),
        }
    }

    fn validate_self_test(plan: &PipelinePlan) -> Result<(), PipelineError> {
        if plan.source.binding_class != SourceBindingClass::SelfTest
            || plan.source.device_id != "self-test"
            || plan.source.connector.is_some()
            || !plan.outputs.is_empty()
        {
            return Err(PipelineError::PrepareFailed(
                "RF-FF-01B FFmpegBackend only accepts canonical SelfTest plan".into(),
            ));
        }
        Ok(())
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.binary);
        #[cfg(test)]
        if let Some(args) = &self.test_args {
            cmd.args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            return cmd;
        }
        cmd.args([
            "-hide_banner",
            "-nostdin",
            "-nostats",
            "-loglevel",
            "error",
            "-re",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x180:rate=25",
            "-re",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:sample_rate=48000",
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-f",
            "null",
            "-",
        ]);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        cmd
    }

    fn spawn_self_test(&self, plan: &PipelinePlan) -> Result<Child, PipelineError> {
        Self::validate_self_test(plan)?;
        self.command().spawn().map_err(|e| {
            PipelineError::StartFailed(format!(
                "ffmpeg spawn failed (binary={}): {e}",
                self.binary.display()
            ))
        })
    }

    fn terminate(child: &mut Child) -> std::io::Result<()> {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        child.kill()?;
        let _ = child.wait()?;
        Ok(())
    }

    fn event(
        handle: PipelineHandle,
        kind: PipelineBusEventKind,
        severity: BusSeverity,
        detail: String,
    ) -> PipelineBusEvent {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or(0);
        PipelineBusEvent {
            handle,
            kind,
            source: "ffmpeg".into(),
            timestamp,
            detail,
            severity,
        }
    }

    fn exit_event(handle: PipelineHandle, status: ExitStatus) -> PipelineBusEvent {
        if status.success() {
            Self::event(
                handle,
                PipelineBusEventKind::Eos,
                BusSeverity::Info,
                format!("ffmpeg exited successfully: {status}"),
            )
        } else {
            Self::event(
                handle,
                PipelineBusEventKind::Error,
                BusSeverity::Error,
                format!("ffmpeg exited abnormally: {status}"),
            )
        }
    }

    #[cfg(test)]
    fn child_pid(&self, handle: &PipelineHandle) -> Option<u32> {
        self.instances
            .lock()
            .unwrap()
            .get(handle)
            .and_then(|i| i.child.as_ref())
            .map(Child::id)
    }
}

impl Drop for FFmpegBackend {
    fn drop(&mut self) {
        if let Ok(instances) = self.instances.get_mut() {
            for instance in instances.values_mut() {
                if let Some(child) = instance.child.as_mut() {
                    let _ = Self::terminate(child);
                }
                instance.child = None;
            }
            instances.clear();
        }
    }
}

impl MediaBackend for FFmpegBackend {
    fn instantiate(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError> {
        Self::validate_self_test(plan)?;
        let handle = PipelineHandle(NEXT_PIPELINE_ID.fetch_add(1, Ordering::SeqCst));
        self.instances.lock().unwrap().insert(
            handle,
            FfmpegInstance {
                plan: plan.clone(),
                child: None,
                started: false,
                observe_error_reported: false,
            },
        );
        Ok(handle)
    }

    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        let mut instances = self.instances.lock().unwrap();
        let instance = instances.get_mut(handle).ok_or_else(|| {
            PipelineError::StartFailed(format!("unknown FFmpeg handle (start): {handle:?}"))
        })?;
        if instance.started {
            return Err(PipelineError::StartFailed(format!(
                "FFmpeg handle already started: {handle:?}"
            )));
        }
        let child = self.spawn_self_test(&instance.plan)?;
        instance.child = Some(child);
        instance.started = true;
        instance.observe_error_reported = false;
        Ok(())
    }

    fn stop(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        let mut instances = self.instances.lock().unwrap();
        let instance = instances.get_mut(handle).ok_or_else(|| {
            PipelineError::StopFailed(format!("unknown FFmpeg handle (stop): {handle:?}"))
        })?;
        if let Some(child) = instance.child.as_mut() {
            Self::terminate(child)
                .map_err(|e| PipelineError::StopFailed(format!("ffmpeg stop/reap failed: {e}")))?;
        }
        instance.child = None;
        instances.remove(handle);
        Ok(())
    }

    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError> {
        let mut instances = self.instances.lock().unwrap();
        let instance = instances.get_mut(handle).ok_or_else(|| {
            PipelineError::StartFailed(format!("unknown FFmpeg handle (recover): {handle:?}"))
        })?;
        if !instance.started {
            return Err(PipelineError::StartFailed(format!(
                "FFmpeg handle was never started (recover): {handle:?}"
            )));
        }
        if let Some(child) = instance.child.as_mut() {
            Self::terminate(child).map_err(|e| {
                PipelineError::StartFailed(format!("ffmpeg recover stop/reap failed: {e}"))
            })?;
        }
        instance.child = None;
        let child = self.spawn_self_test(&instance.plan)?;
        instance.child = Some(child);
        instance.observe_error_reported = false;
        Ok(())
    }

    fn observe(&self, handle: &PipelineHandle) -> Vec<PipelineBusEvent> {
        let mut instances = self.instances.lock().unwrap();
        let Some(instance) = instances.get_mut(handle) else {
            return Vec::new();
        };
        let Some(child) = instance.child.as_mut() else {
            return Vec::new();
        };
        match child.try_wait() {
            Ok(None) => Vec::new(),
            Ok(Some(status)) => {
                instance.child = None;
                vec![Self::exit_event(*handle, status)]
            }
            Err(e) if !instance.observe_error_reported => {
                instance.observe_error_reported = true;
                vec![Self::event(
                    *handle,
                    PipelineBusEventKind::Error,
                    BusSeverity::Error,
                    format!("ffmpeg process observation failed: {e}"),
                )]
            }
            Err(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::thread;
    use std::time::Duration;

    fn sleeping_backend() -> FFmpegBackend {
        FFmpegBackend::with_test_command("/bin/sleep", &["1000"])
    }

    #[test]
    fn ffmpeg_rt_01_rejects_non_selftest_plan() {
        let backend = FFmpegBackend::with_test_command("/bin/true", &[]);
        let mut plan = PipelinePlan::self_test();
        plan.source.binding_class = SourceBindingClass::Resolved;
        assert!(matches!(
            backend.instantiate(&plan),
            Err(PipelineError::PrepareFailed(_))
        ));
    }

    #[test]
    fn ffmpeg_rt_01_spawn_failure_leaves_prepared_instance_recoverable_by_stop() {
        let backend = FFmpegBackend::with_test_command("/definitely/missing/vbmf-ffmpeg", &[]);
        let handle = backend.instantiate(&PipelinePlan::self_test()).unwrap();
        assert!(matches!(
            backend.start(&handle),
            Err(PipelineError::StartFailed(_))
        ));
        backend
            .stop(&handle)
            .expect("prepared instance can still be cleaned");
    }

    #[test]
    fn ffmpeg_rt_01_nonzero_exit_becomes_canonical_error_once() {
        let backend = FFmpegBackend::with_test_command("/bin/false", &[]);
        let handle = backend.instantiate(&PipelinePlan::self_test()).unwrap();
        backend.start(&handle).unwrap();
        thread::sleep(Duration::from_millis(30));
        let events = backend.observe(&handle);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, PipelineBusEventKind::Error);
        assert_eq!(events[0].severity, BusSeverity::Error);
        assert!(events[0].detail.contains("abnormally"));
        assert!(
            backend.observe(&handle).is_empty(),
            "terminal event must be one-shot"
        );
        backend.stop(&handle).unwrap();
    }

    #[test]
    fn ffmpeg_rt_01_duplicate_start_rejected_and_stop_reaps_child() {
        let backend = sleeping_backend();
        let handle = backend.instantiate(&PipelinePlan::self_test()).unwrap();
        backend.start(&handle).unwrap();
        let pid = backend.child_pid(&handle).expect("child pid");
        assert!(matches!(
            backend.start(&handle),
            Err(PipelineError::StartFailed(_))
        ));
        backend.stop(&handle).unwrap();
        #[cfg(target_os = "linux")]
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }

    #[test]
    fn ffmpeg_rt_01_unknown_and_never_started_recover_fail_closed() {
        let backend = FFmpegBackend::with_test_command("/bin/true", &[]);
        assert!(matches!(
            backend.recover(&PipelineHandle(u64::MAX)),
            Err(PipelineError::StartFailed(_))
        ));
        let handle = backend.instantiate(&PipelinePlan::self_test()).unwrap();
        assert!(matches!(
            backend.recover(&handle),
            Err(PipelineError::StartFailed(_))
        ));
        backend.stop(&handle).unwrap();
    }

    #[test]
    fn ffmpeg_rt_01_recover_replaces_process_without_orphan() {
        let backend = sleeping_backend();
        let handle = backend.instantiate(&PipelinePlan::self_test()).unwrap();
        backend.start(&handle).unwrap();
        let first = backend.child_pid(&handle).unwrap();
        backend.recover(&handle).unwrap();
        let second = backend.child_pid(&handle).unwrap();
        assert_ne!(first, second);
        #[cfg(target_os = "linux")]
        assert!(!std::path::Path::new(&format!("/proc/{first}")).exists());
        backend.stop(&handle).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn ffmpeg_rt_01_recover_spawn_failure_does_not_leave_old_process() {
        let link = std::env::temp_dir().join(format!(
            "vbmf-ffmpeg-link-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        symlink("/bin/sleep", &link).expect("create test binary symlink");
        let backend = FFmpegBackend::with_test_command(&link, &["1000"]);
        let handle = backend.instantiate(&PipelinePlan::self_test()).unwrap();
        backend.start(&handle).unwrap();
        let old_pid = backend.child_pid(&handle).unwrap();
        fs::remove_file(&link).expect("remove binary path before recover");
        assert!(matches!(
            backend.recover(&handle),
            Err(PipelineError::StartFailed(_))
        ));
        #[cfg(target_os = "linux")]
        assert!(
            !std::path::Path::new(&format!("/proc/{old_pid}")).exists(),
            "failed recover must still reap old process"
        );
        backend.stop(&handle).expect("instance remains cleanable");
    }

    #[test]
    fn ffmpeg_rt_01_drop_reaps_owned_child() {
        let pid = {
            let backend = sleeping_backend();
            let handle = backend.instantiate(&PipelinePlan::self_test()).unwrap();
            backend.start(&handle).unwrap();
            backend.child_pid(&handle).unwrap()
        };
        #[cfg(target_os = "linux")]
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }

    #[test]
    #[ignore = "requires a real ffmpeg binary; run on BMD exact commit"]
    fn ffmpeg_rt_01_real_binary_selftest_smoke() {
        let backend = FFmpegBackend::new();
        let handle = backend.instantiate(&PipelinePlan::self_test()).unwrap();
        backend.start(&handle).expect("real ffmpeg start");
        thread::sleep(Duration::from_millis(750));
        assert!(
            backend.observe(&handle).is_empty(),
            "selftest process must stay alive"
        );
        backend.stop(&handle).expect("real ffmpeg stop");
    }
}
