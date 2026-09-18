//! RF-FF-01B/01C: Live FFmpeg concrete MediaBackend.
//!
//! The adapter owns FFmpeg child processes and consumes only canonical SelfTest or explicitly
//! authorized Resolved input plans. DeckLink addressing is an adapter-local DeviceHandle view
//! derived from provisioning manifest + live Provider identity; no shell strings, runtime device
//! enumeration, guessed fallback, network source, or output path is accepted in this packet.

use crate::contracts::backend::MediaBackend;
use crate::contracts::provider::DiscoveredDevice;
use crate::pipeline::{
    PipelineError, PipelineHandle, PipelinePlan, SourceBindingClass, NEXT_PIPELINE_ID,
};
use crate::pipeline_events::{BusSeverity, PipelineBusEvent, PipelineBusEventKind};
use crate::port::PortDirection;
use crate::resolver::{current_machine_id, DeviceBindingManifest};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

struct FfmpegInstance {
    plan: PipelinePlan,
    child: Option<Child>,
    started: bool,
    observe_error_reported: bool,
}

/// Concrete live FFmpeg backend. One instance owns one child-process table.
pub struct FFmpegBackend {
    binary: PathBuf,
    /// RF-FF-01C: adapter-local RuntimeBinding view. Values are Blackmagic
    /// Provider binding refs (DeviceHandle), never canonical plan fields.
    runtime_bindings: Arc<HashMap<Uuid, String>>,
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
        Self::with_binary_and_bindings(binary, Arc::new(HashMap::new()))
    }

    fn with_binary_and_bindings(
        binary: impl Into<PathBuf>,
        runtime_bindings: Arc<HashMap<Uuid, String>>,
    ) -> Self {
        Self {
            binary: binary.into(),
            runtime_bindings,
            #[cfg(test)]
            test_args: None,
            instances: Mutex::new(HashMap::new()),
        }
    }

    /// Build the FFmpeg runtime view only from the already-provisioned manifest plus
    /// live Provider identity. FFmpeg never enumerates or guesses a device itself.
    pub(super) fn with_authorized_manifest(
        discovered: &[DiscoveredDevice],
        manifest: &DeviceBindingManifest,
    ) -> Result<Self, String> {
        let bindings =
            Self::authorized_runtime_bindings(discovered, manifest, &current_machine_id())?;
        Ok(Self::with_binary_and_bindings("ffmpeg", Arc::new(bindings)))
    }

    #[cfg(test)]
    fn with_test_command(binary: impl Into<PathBuf>, args: &[&str]) -> Self {
        Self::with_test_command_and_bindings(binary, args, HashMap::new())
    }

    #[cfg(test)]
    fn with_test_command_and_bindings(
        binary: impl Into<PathBuf>,
        args: &[&str],
        runtime_bindings: HashMap<Uuid, String>,
    ) -> Self {
        Self {
            binary: binary.into(),
            runtime_bindings: Arc::new(runtime_bindings),
            test_args: Some(args.iter().map(|s| (*s).to_string()).collect()),
            instances: Mutex::new(HashMap::new()),
        }
    }

    fn authorized_runtime_bindings(
        discovered: &[DiscoveredDevice],
        manifest: &DeviceBindingManifest,
        runtime_machine_id: &str,
    ) -> Result<HashMap<Uuid, String>, String> {
        manifest.validate_manifest()?;
        manifest.check_machine_identity(runtime_machine_id)?;

        let mut mapped = HashMap::new();
        for entry in &manifest.bindings {
            let port = entry.port.as_ref().ok_or_else(|| {
                format!(
                    "RF-FF-01C requires port-level direction for binding '{}'",
                    entry.bmd_device_handle
                )
            })?;
            if !matches!(
                port.direction,
                PortDirection::Input | PortDirection::Bidirectional
            ) {
                continue;
            }

            let matches: Vec<&DiscoveredDevice> = discovered
                .iter()
                .filter(|dev| {
                    dev.identity.as_ref().is_some_and(|identity| {
                        identity.provider == "blackmagic"
                            && identity.device_handle.as_deref()
                                == Some(entry.bmd_device_handle.as_str())
                    })
                })
                .collect();
            if matches.len() != 1 {
                return Err(format!(
                    "RF-FF-01C binding '{}' matched {} live Blackmagic devices; refusing to guess",
                    entry.bmd_device_handle,
                    matches.len()
                ));
            }

            let dev = matches[0];
            let handle = dev
                .identity
                .as_ref()
                .and_then(|identity| identity.device_handle.as_deref())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    format!(
                        "RF-FF-01C device {} lacks a non-empty Provider binding ref",
                        dev.device.device_id
                    )
                })?
                .to_string();

            if let Some(previous) = mapped.insert(dev.device.device_id, handle.clone()) {
                return Err(format!(
                    "RF-FF-01C canonical device {} mapped twice ('{}' vs '{}')",
                    dev.device.device_id, previous, handle
                ));
            }
        }

        if mapped.is_empty() {
            return Err(
                "RF-FF-01C manifest/provider intersection contains no authorized input binding"
                    .to_string(),
            );
        }
        Ok(mapped)
    }

    fn validate_self_test(plan: &PipelinePlan) -> Result<(), PipelineError> {
        if plan.source.device_id != "self-test"
            || plan.source.connector.is_some()
            || !plan.outputs.is_empty()
        {
            return Err(PipelineError::PrepareFailed(
                "FFmpeg SelfTest plan shape is not canonical".into(),
            ));
        }
        Ok(())
    }

    fn resolved_device_name(&self, plan: &PipelinePlan) -> Result<String, PipelineError> {
        if !plan.outputs.is_empty() {
            return Err(PipelineError::PrepareFailed(
                "RF-FF-01C input parity packet does not accept output plans".into(),
            ));
        }
        if plan.source.connector != Some(crate::port::ConnectorType::Sdi) {
            return Err(PipelineError::PrepareFailed(
                "RF-FF-01C currently accepts only an explicitly resolved SDI input".into(),
            ));
        }
        let device_id = Uuid::parse_str(&plan.source.device_id).map_err(|e| {
            PipelineError::IdentityUnresolved(format!(
                "canonical device_id parse failed {}: {e}",
                plan.source.device_id
            ))
        })?;
        self.runtime_bindings
            .get(&device_id)
            .cloned()
            .ok_or_else(|| {
                PipelineError::IdentityUnresolved(format!(
                    "device_id={} lacks an authorized FFmpeg RuntimeBinding",
                    plan.source.device_id
                ))
            })
    }

    fn validate_plan(&self, plan: &PipelinePlan) -> Result<(), PipelineError> {
        match plan.source.binding_class {
            SourceBindingClass::SelfTest => Self::validate_self_test(plan),
            SourceBindingClass::Resolved => self.resolved_device_name(plan).map(|_| ()),
            SourceBindingClass::Persistent | SourceBindingClass::DiagnosticFallback => {
                Err(PipelineError::PrepareFailed(
                    "RF-FF-01C accepts only canonical SelfTest or authorized Resolved input".into(),
                ))
            }
        }
    }

    fn command_for_plan(&self, plan: &PipelinePlan) -> Result<Command, PipelineError> {
        self.validate_plan(plan)?;
        let mut cmd = Command::new(&self.binary);
        #[cfg(test)]
        if let Some(args) = &self.test_args {
            cmd.args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            return Ok(cmd);
        }

        match plan.source.binding_class {
            SourceBindingClass::SelfTest => {
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
            }
            SourceBindingClass::Resolved => {
                let device = self.resolved_device_name(plan)?;
                cmd.args([
                    "-hide_banner",
                    "-nostdin",
                    "-nostats",
                    "-loglevel",
                    "error",
                    "-f",
                    "decklink",
                    "-video_input",
                    "sdi",
                    "-audio_input",
                    "embedded",
                    "-i",
                ])
                .arg(device)
                .args(["-map", "0:v:0", "-map", "0:a:0?", "-f", "null", "-"]);
            }
            SourceBindingClass::Persistent | SourceBindingClass::DiagnosticFallback => {
                unreachable!("validate_plan rejected unsupported binding class")
            }
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        Ok(cmd)
    }

    fn spawn_plan(&self, plan: &PipelinePlan) -> Result<Child, PipelineError> {
        self.command_for_plan(plan)?.spawn().map_err(|e| {
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

    /// Acceptance-only observation of the concrete child process. The monitor and
    /// SessionManager still own lifecycle; the gate uses this only to inject an
    /// external termination and prove that recovery creates a new child.
    pub(crate) fn running_child_pid(&self, handle: &PipelineHandle) -> Option<u32> {
        self.instances
            .lock()
            .unwrap()
            .get(handle)
            .and_then(|i| i.child.as_ref())
            .map(Child::id)
    }

    #[cfg(test)]
    fn child_pid(&self, handle: &PipelineHandle) -> Option<u32> {
        self.running_child_pid(handle)
    }
}

impl crate::contracts::backend::BackendProcessInspector for FFmpegBackend {
    fn running_child_pid(&self, handle: &PipelineHandle) -> Option<u32> {
        FFmpegBackend::running_child_pid(self, handle)
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
        self.validate_plan(plan)?;
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
        let child = self.spawn_plan(&instance.plan)?;
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
        let child = self.spawn_plan(&instance.plan)?;
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

    fn resolved_plan(device_id: Uuid) -> PipelinePlan {
        let mut plan = PipelinePlan::self_test();
        plan.source.device_id = device_id.to_string();
        plan.source.connector = Some(crate::port::ConnectorType::Sdi);
        plan.source.binding_class = SourceBindingClass::Resolved;
        plan
    }

    fn discovered_blackmagic(device_id: Uuid, handle: &str) -> DiscoveredDevice {
        DiscoveredDevice {
            device: crate::device::DeviceInfo {
                device_id,
                model: "DeckLink test input".into(),
                display_name: "test-input".into(),
                serial_number: None,
                video_input_connections: 1,
                video_output_connections: 0,
                identity_strength: crate::device::IdentityStrength::DeviceHandle,
                identity_source: crate::device::DeviceIdentitySource::RealBmd,
                capabilities: crate::port::DeviceCapabilities::default(),
                ports: Vec::new(),
            },
            identity: Some(crate::contracts::provider::ProviderIdentity {
                provider: "blackmagic",
                persistent_id: None,
                device_handle: Some(handle.to_string()),
                topological_id: None,
            }),
        }
    }

    fn manifest_entry(handle: &str, direction: PortDirection) -> crate::resolver::BindingEntry {
        crate::resolver::BindingEntry {
            label: Some("test-input".into()),
            bmd_device_handle: handle.into(),
            gst_device_number: 7,
            expected_hw_serial_number: None,
            expected_model: None,
            port: Some(crate::resolver::PortBinding {
                connector: crate::port::ConnectorType::Sdi,
                ordinal: 1,
                direction,
                required: true,
                verification: crate::port::VerificationLevel::Declared,
            }),
        }
    }

    fn test_manifest(entries: Vec<crate::resolver::BindingEntry>) -> DeviceBindingManifest {
        DeviceBindingManifest {
            manifest_version: "2.0".into(),
            machine_id: "test-host".into(),
            generated_by: "rf-ff-01c-test".into(),
            generated_at: "2026-09-18T00:00:00Z".into(),
            bmd_sdk_version: None,
            gst_decklink_plugin_version: None,
            gst_runtime_version: None,
            notes: None,
            bindings: entries,
        }
    }

    #[test]
    fn ffmpeg_rt_02_authorized_manifest_maps_exact_provider_handle_into_argv() {
        let device_id = Uuid::new_v4();
        let provider_handle = "46:test:0001";
        let discovered = vec![discovered_blackmagic(device_id, provider_handle)];
        let manifest = test_manifest(vec![manifest_entry(provider_handle, PortDirection::Input)]);
        let bindings =
            FFmpegBackend::authorized_runtime_bindings(&discovered, &manifest, "test-host")
                .expect("authorized mapping");
        assert_eq!(
            bindings.get(&device_id).map(String::as_str),
            Some(provider_handle)
        );

        let backend =
            FFmpegBackend::with_binary_and_bindings("/usr/bin/ffmpeg", Arc::new(bindings));
        let command = backend
            .command_for_plan(&resolved_plan(device_id))
            .expect("resolved command");
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.windows(2).any(|w| w == ["-f", "decklink"]));
        assert!(args.windows(2).any(|w| w == ["-i", provider_handle]));
        assert!(!args.iter().any(|arg| arg == "-c" || arg == "sh"));
    }

    #[test]
    fn ffmpeg_rt_02_missing_authorized_binding_fails_closed() {
        let backend = FFmpegBackend::with_test_command("/bin/true", &[]);
        assert!(matches!(
            backend.instantiate(&resolved_plan(Uuid::new_v4())),
            Err(PipelineError::IdentityUnresolved(_))
        ));
    }

    #[test]
    fn ffmpeg_rt_02_manifest_handle_missing_from_live_provider_fails_closed() {
        let discovered = vec![discovered_blackmagic(Uuid::new_v4(), "46:live:0001")];
        let manifest = test_manifest(vec![manifest_entry(
            "46:manifest:0002",
            PortDirection::Input,
        )]);
        let err = FFmpegBackend::authorized_runtime_bindings(&discovered, &manifest, "test-host")
            .expect_err("mismatched Provider binding must fail");
        assert!(err.contains("matched 0 live Blackmagic devices"));
    }

    #[test]
    fn ffmpeg_rt_02_ambiguous_live_provider_match_fails_closed() {
        let handle = "46:ambiguous:0001";
        let discovered = vec![
            discovered_blackmagic(Uuid::new_v4(), handle),
            discovered_blackmagic(Uuid::new_v4(), handle),
        ];
        let manifest = test_manifest(vec![manifest_entry(handle, PortDirection::Input)]);
        let err = FFmpegBackend::authorized_runtime_bindings(&discovered, &manifest, "test-host")
            .expect_err("ambiguous Provider binding must fail");
        assert!(err.contains("matched 2 live Blackmagic devices"));
    }

    #[test]
    fn ffmpeg_rt_02_output_only_manifest_is_not_a_source_binding() {
        let handle = "83:output:0001";
        let discovered = vec![discovered_blackmagic(Uuid::new_v4(), handle)];
        let manifest = test_manifest(vec![manifest_entry(handle, PortDirection::Output)]);
        let err = FFmpegBackend::authorized_runtime_bindings(&discovered, &manifest, "test-host")
            .expect_err("output-only manifest must not authorize FFmpeg input");
        assert!(err.contains("no authorized input binding"));
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

    #[cfg(feature = "bmd-provider")]
    #[test]
    #[ignore = "requires BMD hardware + real ffmpeg + authorized manifest"]
    fn ffmpeg_rt_02_real_decklink_resolved_binding_lifecycle() {
        let manifest_path = std::env::var("VBMF_FFMPEG_BINDING_MANIFEST")
            .expect("VBMF_FFMPEG_BINDING_MANIFEST must name the authorized BMD manifest");
        let target_handle = std::env::var("VBMF_FFMPEG_TEST_DEVICE_HANDLE")
            .expect("VBMF_FFMPEG_TEST_DEVICE_HANDLE must explicitly select one authorized input");

        let manifest =
            DeviceBindingManifest::load(&manifest_path).expect("load authorized BMD manifest");
        let provider =
            crate::registry::AdapterRegistry::build_provider().expect("build Blackmagic provider");
        let discovered = provider
            .discover()
            .expect("discover live BMD Provider identity");

        let target = discovered
            .iter()
            .filter(|dev| {
                dev.identity.as_ref().is_some_and(|identity| {
                    identity.provider == "blackmagic"
                        && identity.device_handle.as_deref() == Some(target_handle.as_str())
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            target.len(),
            1,
            "explicit target handle must resolve to exactly one live Blackmagic device"
        );

        let entry = manifest
            .lookup(&target_handle)
            .expect("explicit target handle must be authorized by manifest");
        assert!(
            entry.port.as_ref().is_some_and(|port| matches!(
                port.direction,
                PortDirection::Input | PortDirection::Bidirectional
            )),
            "explicit target must be an authorized input port"
        );

        let backend = crate::registry::AdapterRegistry::build_ffmpeg_backend_with_manifest(
            &discovered,
            &manifest,
        )
        .expect("build FFmpeg backend from authorized runtime binding");
        let plan = resolved_plan(target[0].device.device_id);
        let handle = backend
            .instantiate(&plan)
            .expect("instantiate DeckLink plan");

        backend
            .start(&handle)
            .expect("start real FFmpeg DeckLink input");
        thread::sleep(Duration::from_millis(1500));
        assert!(
            backend.observe(&handle).is_empty(),
            "real DeckLink FFmpeg process exited during first capture window"
        );

        backend
            .recover(&handle)
            .expect("recover must replace real FFmpeg DeckLink process");
        thread::sleep(Duration::from_millis(1500));
        assert!(
            backend.observe(&handle).is_empty(),
            "real DeckLink FFmpeg process exited after recover"
        );

        backend
            .stop(&handle)
            .expect("stop/reap real FFmpeg process");
    }
}
