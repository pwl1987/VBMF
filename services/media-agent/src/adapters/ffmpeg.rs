//! RF-FF-01B/01C: Live FFmpeg concrete MediaBackend.
//!
//! The adapter owns FFmpeg child processes and consumes only canonical SelfTest or explicitly
//! authorized Resolved input plans. DeckLink addressing is an adapter-local DeviceHandle view
//! derived from provisioning manifest + live Provider identity; no shell strings, runtime device
//! enumeration, guessed fallback, or network source is accepted. RF-FF-02 consumes only the
//! already-materialized single-output Hls/Rtmp plan through backend-owned argv.

use crate::contracts::backend::MediaBackend;
use crate::contracts::provider::DiscoveredDevice;
use crate::pipeline::{
    OutputKind, OutputPlan, PipelineError, PipelineHandle, PipelinePlan, SourceBindingClass,
    SourcePlan, NEXT_PIPELINE_ID,
};
use crate::pipeline_events::{BusSeverity, PipelineBusEvent, PipelineBusEventKind};
use crate::port::PortDirection;
use crate::resolver::{current_machine_id, DeviceBindingManifest};
use std::collections::{HashMap, VecDeque};
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// ── RF-SRC-RTMP-02 TG-3 (plan D9): bounded stderr capture ─────────────────────

/// Ring bounds (plan D9): at most 64 KiB total, at most 256 lines, every
/// line truncated to 512 bytes before it enters the ring.
const STDERR_RING_MAX_BYTES: usize = 64 * 1024;
const STDERR_RING_MAX_LINES: usize = 256;
const STDERR_LINE_MAX_BYTES: usize = 512;

/// Byte-precise truncation on a UTF-8 char boundary.
fn truncate_bytes(text: &mut String, max: usize) {
    if text.len() <= max {
        return;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
}

/// Bounded stderr ring. Adapter-internal: the raw text is NEVER copied into
/// a canonical event, error, `Debug`, health, evidence or panic message; it
/// is only ever reduced to [`NetworkExitClass`].
#[derive(Default)]
struct StderrRing {
    lines: VecDeque<String>,
    total_bytes: usize,
}

impl StderrRing {
    fn push_line(&mut self, mut line: String) {
        truncate_bytes(&mut line, STDERR_LINE_MAX_BYTES);
        while self.total_bytes + line.len() > STDERR_RING_MAX_BYTES && self.lines.len() > 1 {
            if let Some(evicted) = self.lines.pop_front() {
                self.total_bytes -= evicted.len();
            }
        }
        if self.total_bytes + line.len() > STDERR_RING_MAX_BYTES {
            // a single line can never exceed the ring (512 ≤ 64 KiB), but
            // fail-closed on the impossible case anyway
            return;
        }
        if self.lines.len() == STDERR_RING_MAX_LINES {
            if let Some(evicted) = self.lines.pop_front() {
                self.total_bytes -= evicted.len();
            }
        }
        self.total_bytes += line.len();
        self.lines.push_back(line);
    }

    /// Adapter-internal diagnostic view (classification input only).
    fn recent_text(&self) -> String {
        self.lines.iter().cloned().collect::<Vec<_>>().join("\n")
    }
}

/// D9 finite attribution categories. These are the only observable products
/// of the captured stderr; vendor text never leaves the adapter.
/// `PublisherDisconnected`/`SpawnFailure` are constructed by the TG-4
/// recovery wiring (INV-1 positive attribution combines canonical signal
/// history; spawn failures map at the Session layer) — reserved here so the
/// category set is the frozen four from day one.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkExitClass {
    BindFailure,
    PublisherDisconnected,
    SpawnFailure,
    UnknownExit,
}

impl std::fmt::Display for NetworkExitClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::BindFailure => "BindFailure",
            Self::PublisherDisconnected => "PublisherDisconnected",
            Self::SpawnFailure => "SpawnFailure",
            Self::UnknownExit => "UnknownExit",
        };
        f.write_str(name)
    }
}

/// Reader-thread products: the bounded ring plus its join handle (D9).
type StderrCapture = (Arc<Mutex<StderrRing>>, std::thread::JoinHandle<()>);

/// Classify a network-listener child exit (plan D6/D9 + INV-1).
///
/// * `BindFailure` — the OS bind error surfaces on ffmpeg stderr (plan D6
///   explicitly classifies "Address already in use" and equivalents this way).
/// * `PublisherDisconnected` is deliberately NOT attributed here: INV-1
///   forbids stderr keywords alone; the positive attribution combines exit
///   status with canonical signal history and lands with the TG-4 recovery
///   wiring.
/// * everything else is `UnknownExit` (D7/D8: ManualRequired, never an
///   exploratory restart).
fn classify_network_exit(status: &ExitStatus, ring: &StderrRing) -> NetworkExitClass {
    let _ = status;
    let text = ring.recent_text();
    const BIND_ERROR_MARKERS: [&str; 4] = [
        "Address already in use",
        "address in use",
        "Address in use",
        "Failed to bind",
    ];
    if BIND_ERROR_MARKERS.iter().any(|m| text.contains(m)) {
        return NetworkExitClass::BindFailure;
    }
    NetworkExitClass::UnknownExit
}

struct FfmpegInstance {
    plan: PipelinePlan,
    child: Option<Child>,
    started: bool,
    observe_error_reported: bool,
    /// TG-3: network-listener children get a piped stderr drained by an
    /// independent reader thread into this bounded ring (D9).
    stderr_ring: Option<Arc<Mutex<StderrRing>>>,
    stderr_reader: Option<std::thread::JoinHandle<()>>,
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

    fn validate_output_plan(plan: &PipelinePlan) -> Result<(), PipelineError> {
        if plan.outputs.len() > 1 {
            return Err(PipelineError::PrepareFailed(
                "RF-FF-02 accepts at most one output plan".into(),
            ));
        }
        let Some(output) = plan.outputs.first() else {
            return Ok(());
        };
        if output.target.is_empty() || output.target.chars().any(char::is_control) {
            return Err(PipelineError::PrepareFailed(
                "RF-FF-02 output target must be non-empty and free of control characters".into(),
            ));
        }
        match output.kind {
            OutputKind::Hls => {
                if !PathBuf::from(&output.target).is_absolute() {
                    return Err(PipelineError::PrepareFailed(
                        "RF-FF-02 HLS output target must be an absolute directory".into(),
                    ));
                }
            }
            OutputKind::Rtmp => {
                if !output.target.starts_with("rtmp://")
                    || output.target.chars().any(|c| matches!(c, '"' | '\''))
                {
                    return Err(PipelineError::PrepareFailed(
                        "RF-FF-02 RTMP output target must be a quoted-free rtmp:// URL".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_self_test(plan: &PipelinePlan) -> Result<(), PipelineError> {
        if !matches!(plan.source, SourcePlan::SelfTest) {
            return Err(PipelineError::PrepareFailed(
                "FFmpeg SelfTest plan shape is not canonical".into(),
            ));
        }
        Ok(())
    }

    fn resolved_device_name(&self, plan: &PipelinePlan) -> Result<String, PipelineError> {
        let SourcePlan::Device {
            device_id,
            connector,
            binding_class,
        } = &plan.source
        else {
            return Err(PipelineError::PrepareFailed(
                "RF-FF-01C hardware input requires a Device source plan".into(),
            ));
        };
        if *connector != Some(crate::port::ConnectorType::Sdi)
            || *binding_class != SourceBindingClass::Resolved
        {
            return Err(PipelineError::PrepareFailed(
                "RF-FF-01C currently accepts only an explicitly resolved SDI input".into(),
            ));
        }
        let device_id = Uuid::parse_str(device_id).map_err(|e| {
            PipelineError::IdentityUnresolved(format!(
                "canonical device_id parse failed {device_id}: {e}"
            ))
        })?;
        self.runtime_bindings
            .get(&device_id)
            .cloned()
            .ok_or_else(|| {
                PipelineError::IdentityUnresolved(format!(
                    "device_id={device_id} lacks an authorized FFmpeg RuntimeBinding"
                ))
            })
    }

    fn validate_plan(&self, plan: &PipelinePlan) -> Result<(), PipelineError> {
        Self::validate_output_plan(plan)?;
        match &plan.source {
            SourcePlan::SelfTest => Self::validate_self_test(plan),
            SourcePlan::Device {
                binding_class: SourceBindingClass::Resolved,
                ..
            } => self.resolved_device_name(plan).map(|_| ()),
            // TG-3 (plan D2/D5): the listener address comes from the strict
            // canonical endpoint (eligible classes incl. production LAN);
            // the loopback-only gate is replaced by D2 canonical validation —
            // Production admission happened at the Session layer (TG-2).
            SourcePlan::Network { endpoint, .. } => {
                endpoint.to_canonical().map_err(PipelineError::PrepareFailed)?;
                Ok(())
            }
            SourcePlan::Device { .. } => Err(PipelineError::PrepareFailed(
                "RF-FF-01C accepts only canonical SelfTest, RTMP Network, or authorized Resolved input".into(),
            )),
        }
    }

    fn append_output_args(cmd: &mut Command, outputs: &[OutputPlan]) {
        let Some(output) = outputs.first() else {
            cmd.args(["-f", "null", "-"]);
            return;
        };
        let video_bitrate = format!("{}k", output.video_bitrate_kbps);
        let audio_bitrate = output.audio_bitrate_bps.to_string();
        match output.kind {
            OutputKind::Hls => {
                let playlist = PathBuf::from(&output.target).join("index.m3u8");
                cmd.args([
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-tune",
                    "zerolatency",
                    "-b:v",
                    &video_bitrate,
                    "-g",
                    "50",
                    "-c:a",
                    "aac",
                    "-b:a",
                    &audio_bitrate,
                    "-f",
                    "hls",
                    "-hls_time",
                    "2",
                    "-hls_list_size",
                    "5",
                    "-hls_flags",
                    "delete_segments+append_list",
                    "-hls_segment_filename",
                ])
                .arg(PathBuf::from(&output.target).join("seg%05d.ts"))
                .arg(playlist);
            }
            OutputKind::Rtmp => {
                cmd.args([
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-tune",
                    "zerolatency",
                    "-b:v",
                    &video_bitrate,
                    "-g",
                    "50",
                    "-c:a",
                    "aac",
                    "-b:a",
                    &audio_bitrate,
                    "-f",
                    "flv",
                ])
                .arg(&output.target);
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

        match &plan.source {
            SourcePlan::SelfTest => {
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
                ]);
            }
            SourcePlan::Device {
                binding_class: SourceBindingClass::Resolved,
                ..
            } => {
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
                .args(["-map", "0:v:0", "-map", "0:a:0?"]);
            }
            SourcePlan::Network { endpoint, .. } => {
                // TG-3: argv consumes the strict canonical URL (explicit
                // bind input — the OS bind() on this address/port is the
                // final authority, plan D6). D9 escape hatch: never log it.
                let canonical = endpoint
                    .to_canonical()
                    .map_err(PipelineError::PrepareFailed)?;
                let url = canonical.to_url();
                cmd.args([
                    "-hide_banner",
                    "-nostdin",
                    "-nostats",
                    "-loglevel",
                    "error",
                    "-rtmp_listen",
                    "1",
                    "-i",
                ])
                .arg(url)
                .args(["-map", "0:v:0", "-map", "0:a:0?"]);
            }
            SourcePlan::Device { .. } => {
                unreachable!("validate_plan rejected unsupported device binding class")
            }
        }
        Self::append_output_args(&mut cmd, &plan.outputs);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        Ok(cmd)
    }

    /// TG-3 (plan D9): independent reader thread continuously draining the
    /// child stderr pipe into the bounded ring (prevents pipe deadlock).
    /// The thread exits at EOF (child exit closes the write end).
    fn spawn_stderr_reader(
        pipe: std::process::ChildStderr,
    ) -> (Arc<Mutex<StderrRing>>, std::thread::JoinHandle<()>) {
        let ring = Arc::new(Mutex::new(StderrRing::default()));
        let sink = Arc::clone(&ring);
        let reader = std::thread::Builder::new()
            .name("ffmpeg-stderr-reader".into())
            .spawn(move || {
                let mut reader = std::io::BufReader::new(pipe);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            let cleaned = line.trim_end_matches(['\n', '\r']).to_string();
                            if let Ok(mut ring) = sink.lock() {
                                ring.push_line(cleaned);
                            }
                        }
                    }
                }
            })
            .expect("spawn ffmpeg stderr reader");
        (ring, reader)
    }

    /// D9 reaping order: the child is terminated and reaped FIRST, then the
    /// reader thread is joined (EOF guarantees its exit) so stop/recover/
    /// close/Drop never leave a reader or child behind.
    fn reap_reader(instance: &mut FfmpegInstance) {
        if let Some(reader) = instance.stderr_reader.take() {
            let _ = reader.join();
        }
    }

    fn spawn_plan(
        &self,
        plan: &PipelinePlan,
    ) -> Result<(Child, Option<StderrCapture>), PipelineError> {
        let mut cmd = self.command_for_plan(plan)?;
        let network_listener = matches!(plan.source, SourcePlan::Network { .. });
        if network_listener {
            // D9: network-listener children get a piped stderr drained by the
            // independent reader thread; all other plans keep null stderr.
            cmd.stderr(Stdio::piped());
        }
        let mut child = cmd.spawn().map_err(|e| {
            PipelineError::StartFailed(format!(
                "ffmpeg spawn failed (binary={}): {e}",
                self.binary.display()
            ))
        })?;
        let reader_parts = if network_listener {
            child.stderr.take().map(Self::spawn_stderr_reader)
        } else {
            None
        };
        Ok((child, reader_parts))
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

    /// TG-3 test probe: (reader still present, ring still present) — both
    /// false after the D9 reap path proves the reader thread was joined.
    #[cfg(test)]
    fn tg3_reader_state(&self, handle: &PipelineHandle) -> (bool, bool) {
        match self.instances.lock().unwrap().get(handle) {
            Some(i) => (i.stderr_reader.is_some(), i.stderr_ring.is_some()),
            None => (false, false),
        }
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
                // D9: no reader left behind on drop either.
                Self::reap_reader(instance);
                instance.stderr_ring = None;
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
                stderr_ring: None,
                stderr_reader: None,
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
        let (child, reader_parts) = self.spawn_plan(&instance.plan)?;
        instance.child = Some(child);
        if let Some((ring, reader)) = reader_parts {
            instance.stderr_ring = Some(ring);
            instance.stderr_reader = Some(reader);
        }
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
        // D9 order: child reaped, then reader joined — no reader left behind.
        Self::reap_reader(instance);
        instance.child = None;
        instance.stderr_ring = None;
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
        Self::reap_reader(instance);
        instance.child = None;
        instance.stderr_ring = None;
        let (child, reader_parts) = self.spawn_plan(&instance.plan)?;
        instance.child = Some(child);
        if let Some((ring, reader)) = reader_parts {
            instance.stderr_ring = Some(ring);
            instance.stderr_reader = Some(reader);
        }
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
                // D9: reader joined after reaping; the bounded ring reduces to
                // a finite classification — raw stderr never leaves here.
                Self::reap_reader(instance);
                let ring = instance.stderr_ring.take();
                let event = match (&instance.plan.source, ring) {
                    (SourcePlan::Network { .. }, Some(ring)) => {
                        let class = classify_network_exit(&status, &ring.lock().unwrap());
                        Self::event(
                            *handle,
                            PipelineBusEventKind::Error,
                            BusSeverity::Error,
                            format!("network listener exit classified: {class}"),
                        )
                    }
                    (SourcePlan::Network { .. }, None) => Self::event(
                        *handle,
                        PipelineBusEventKind::Error,
                        BusSeverity::Error,
                        "network listener exit classified: UnknownExit".to_string(),
                    ),
                    _ => Self::exit_event(*handle, status),
                };
                vec![event]
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
        plan.source = SourcePlan::Device {
            device_id: device_id.to_string(),
            connector: Some(crate::port::ConnectorType::Sdi),
            binding_class: SourceBindingClass::Resolved,
        };
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

    fn output_plan(kind: OutputKind, target: &str) -> OutputPlan {
        OutputPlan {
            kind,
            video_bitrate_kbps: 6000,
            audio_bitrate_bps: 128_000,
            target: target.into(),
        }
    }

    fn command_args(plan: &PipelinePlan) -> Vec<String> {
        FFmpegBackend::with_binary("ffmpeg")
            .command_for_plan(plan)
            .expect("command")
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    fn network_plan() -> PipelinePlan {
        PipelinePlan {
            source: SourcePlan::Network {
                source_id: crate::source::NetworkSourceId(Uuid::new_v4()),
                endpoint: crate::source::NetworkEndpoint {
                    protocol: crate::source::NetworkProtocol::Rtmp,
                    host: "127.0.0.1".into(),
                    port: 1935,
                    path: "/live/source".into(),
                },
            },
            timeline_policy: crate::pipeline::TimelinePolicy::ProgramTimelineMapped,
            switch_mode: crate::program::SwitchPolicy::FrameSwitch,
            outputs: Vec::new(),
        }
    }

    #[test]
    fn rf_src_rtmp_uses_controlled_url_and_audio_video_maps() {
        let args = command_args(&network_plan());
        assert!(args.windows(2).any(|w| w == ["-rtmp_listen", "1"]));
        assert!(args
            .windows(2)
            .any(|w| w == ["-i", "rtmp://127.0.0.1:1935/live/source"]));
        assert!(args.windows(2).any(|w| w == ["-map", "0:v:0"]));
        assert!(args.windows(2).any(|w| w == ["-map", "0:a:0?"]));
        assert!(!args.iter().any(|arg| arg == "decklink"));
    }

    // ── RF-SRC-RTMP-02 TG-3 (plan D2/D6/D9) ────────────────────────────────────

    fn network_plan_with(host: &str, port: u16, path: &str) -> PipelinePlan {
        PipelinePlan {
            source: SourcePlan::Network {
                source_id: crate::source::NetworkSourceId(Uuid::new_v4()),
                endpoint: crate::source::NetworkEndpoint {
                    protocol: crate::source::NetworkProtocol::Rtmp,
                    host: host.into(),
                    port,
                    path: path.into(),
                },
            },
            ..network_plan()
        }
    }

    #[test]
    fn rf_src_rtmp_02_tg3_listener_argv_consumes_canonical_endpoint() {
        // Production LAN endpoint: argv uses the canonical URL (explicit
        // bind input; the OS bind() on it is the final authority, plan D6).
        let args = command_args(&network_plan_with("10.30.15.10", 19350, "/live/source"));
        assert!(args
            .windows(2)
            .any(|w| w == ["-i", "rtmp://10.30.15.10:19350/live/source"]));
        assert!(args.windows(2).any(|w| w == ["-rtmp_listen", "1"]));
        // bracketed canonical ipv6
        let args = command_args(&network_plan_with("[fd00::10]", 19350, "/live/source"));
        assert!(args
            .windows(2)
            .any(|w| w == ["-i", "rtmp://[fd00::10]:19350/live/source"]));
    }

    #[test]
    fn rf_src_rtmp_02_tg3_non_canonical_endpoint_rejects_before_argv() {
        for (host, port, path) in [
            ("localhost", 19350, "/live/source"),  // hostname
            ("8.8.8.8", 19350, "/live/source"),    // ineligible class
            ("10.30.15.10", 1023, "/live/source"), // privileged port
            ("10.30.15.10", 19350, "/live/a b"),   // non-canonical path
        ] {
            let err = FFmpegBackend::with_binary("ffmpeg")
                .command_for_plan(&network_plan_with(host, port, path))
                .err()
                .unwrap_or_else(|| panic!("non-canonical {host}:{port}{path} must reject"));
            assert!(matches!(err, PipelineError::PrepareFailed(_)), "{err:?}");
        }
    }

    #[test]
    fn rf_src_rtmp_02_tg3_stderr_ring_bounds() {
        // 512-byte per-line truncation on a UTF-8 boundary
        let mut ring = StderrRing::default();
        let long_line = format!("{}é", "x".repeat(STDERR_LINE_MAX_BYTES));
        assert!(long_line.len() > STDERR_LINE_MAX_BYTES);
        ring.push_line(long_line.clone());
        {
            let stored = ring.lines.back().unwrap();
            assert!(stored.len() <= STDERR_LINE_MAX_BYTES);
            assert!(stored.is_char_boundary(stored.len()));
            assert!(stored.starts_with(&"x".repeat(STDERR_LINE_MAX_BYTES - 2)));
        }

        // 256-line cap with FIFO eviction
        let mut ring = StderrRing::default();
        for i in 0..(STDERR_RING_MAX_LINES + 10) {
            ring.push_line(format!("line-{i}"));
        }
        assert_eq!(ring.lines.len(), STDERR_RING_MAX_LINES);
        assert_eq!(ring.lines.front().unwrap(), "line-10");
        assert_eq!(
            ring.lines.back().unwrap(),
            &format!("line-{}", STDERR_RING_MAX_LINES + 9)
        );

        // 64 KiB total cap: pushing a long series evicts oldest lines
        let mut ring = StderrRing::default();
        let chunk = "y".repeat(1024);
        for _ in 0..(STDERR_RING_MAX_BYTES / 1024 + 8) {
            ring.push_line(chunk.clone());
        }
        assert!(ring.total_bytes <= STDERR_RING_MAX_BYTES);
        assert!(ring.lines.len() <= STDERR_RING_MAX_LINES);
    }

    #[test]
    fn rf_src_rtmp_02_tg3_exit_classification_is_finite_and_redacted() {
        // bind errors (plan D6) classify as BindFailure from the bounded ring
        let mut ring = StderrRing::default();
        ring.push_line("rtmp: Address already in use".into());
        let status = Command::new("/bin/true")
            .status()
            .expect("run /bin/true for an ExitStatus");
        assert_eq!(
            classify_network_exit(&status, &ring),
            NetworkExitClass::BindFailure
        );
        // INV-1: no stderr keyword alone may claim PublisherDisconnected —
        // unknown evidence stays UnknownExit (ManualRequired per D7/D8).
        let mut ring = StderrRing::default();
        ring.push_line("some vendor noise about disconnect".into());
        assert_eq!(
            classify_network_exit(&status, &ring),
            NetworkExitClass::UnknownExit
        );
        let empty = StderrRing::default();
        assert_eq!(
            classify_network_exit(&status, &empty),
            NetworkExitClass::UnknownExit
        );
        // Display carries the category name only — never vendor text
        for class in [
            NetworkExitClass::BindFailure,
            NetworkExitClass::PublisherDisconnected,
            NetworkExitClass::SpawnFailure,
            NetworkExitClass::UnknownExit,
        ] {
            let text = class.to_string();
            assert_eq!(
                text,
                format!("{class:?}"),
                "Display must equal the variant name"
            );
        }
    }

    #[test]
    fn rf_src_rtmp_02_tg3_reader_drains_classifies_and_reaps_without_leak() {
        // real child with piped stderr through the network-listener path
        // (test argv list; the echo is redirected to stderr explicitly)
        let backend = FFmpegBackend::with_test_command(
            "/bin/sh",
            &["-c", "echo 'rtmp: Address already in use' >&2; exit 1"],
        );
        let plan = network_plan();
        let handle = backend.instantiate(&plan).expect("instantiate listener");
        backend.start(&handle).expect("start listener child");
        assert_eq!(backend.tg3_reader_state(&handle), (true, true));

        // poll observe until the child exit is reaped
        let mut exit_event = None;
        for _ in 0..100 {
            for event in backend.observe(&handle) {
                if event.kind == PipelineBusEventKind::Error {
                    exit_event = Some(event);
                }
            }
            if exit_event.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let event = exit_event.expect("exit event observed");
        assert!(
            event.detail.contains("BindFailure"),
            "classification surfaced: {}",
            event.detail
        );
        // D9 redaction: raw vendor stderr never appears in the canonical event
        assert!(
            !event.detail.contains("Address already in use"),
            "raw stderr must not be observable: {}",
            event.detail
        );
        // reader joined + ring dropped after the reap path
        assert_eq!(backend.tg3_reader_state(&handle), (false, false));
        let _ = backend.stop(&handle);
    }

    #[test]
    fn rf_src_rtmp_02_tg3_stop_joins_reader_for_live_listener_child() {
        // a live network-listener child stopped by the operator: kill → wait
        // → reader join must complete (no reader/child leak); the test would
        // hang if the join were missing or deadlockable.
        let backend = FFmpegBackend::with_test_command("/bin/sleep", &["100"]);
        let plan = network_plan();
        let handle = backend.instantiate(&plan).expect("instantiate listener");
        backend.start(&handle).expect("start listener child");
        assert_eq!(backend.tg3_reader_state(&handle), (true, true));
        backend
            .stop(&handle)
            .expect("stop reaps child and joins reader");
        assert_eq!(backend.tg3_reader_state(&handle), (false, false));
    }

    #[test]
    fn ffmpeg_rt_03_hls_output_argv_is_backend_owned() {
        let mut plan = PipelinePlan::self_test();
        plan.outputs = vec![output_plan(OutputKind::Hls, "/tmp/vbmf-rf-ff-02-hls")];
        let args = command_args(&plan);
        assert!(args.windows(2).any(|w| w == ["-f", "hls"]));
        assert!(args.windows(2).any(|w| w == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|w| w == ["-b:a", "128000"]));
        assert!(args
            .iter()
            .any(|arg| arg == "/tmp/vbmf-rf-ff-02-hls/index.m3u8"));
        assert!(args
            .iter()
            .any(|arg| arg == "/tmp/vbmf-rf-ff-02-hls/seg%05d.ts"));
        assert!(!args.iter().any(|arg| arg == "sh" || arg == "-c"));
        assert!(!args.windows(2).any(|w| w == ["-f", "null"]));
    }

    #[test]
    fn ffmpeg_rt_03_rtmp_output_argv_is_backend_owned() {
        let mut plan = PipelinePlan::self_test();
        plan.outputs = vec![output_plan(
            OutputKind::Rtmp,
            "rtmp://127.0.0.1:1935/vbmf/rf-ff-02",
        )];
        let args = command_args(&plan);
        assert!(args.windows(2).any(|w| w == ["-f", "flv"]));
        assert!(args
            .iter()
            .any(|arg| arg == "rtmp://127.0.0.1:1935/vbmf/rf-ff-02"));
    }

    #[test]
    fn ffmpeg_rt_03_output_targets_and_cardinality_fail_closed() {
        let mut relative = PipelinePlan::self_test();
        relative.outputs = vec![output_plan(OutputKind::Hls, "relative/out")];
        assert!(matches!(
            FFmpegBackend::with_binary("ffmpeg").instantiate(&relative),
            Err(PipelineError::PrepareFailed(_))
        ));

        let mut quoted = PipelinePlan::self_test();
        quoted.outputs = vec![output_plan(OutputKind::Rtmp, "rtmp://host/live/\"bad\"")];
        assert!(matches!(
            FFmpegBackend::with_binary("ffmpeg").instantiate(&quoted),
            Err(PipelineError::PrepareFailed(_))
        ));

        let mut multiple = PipelinePlan::self_test();
        multiple.outputs = vec![
            output_plan(OutputKind::Hls, "/tmp/one"),
            output_plan(OutputKind::Rtmp, "rtmp://127.0.0.1/live/two"),
        ];
        assert!(matches!(
            FFmpegBackend::with_binary("ffmpeg").instantiate(&multiple),
            Err(PipelineError::PrepareFailed(_))
        ));
    }

    #[test]
    fn ffmpeg_rt_03_no_output_retains_null_sink_command() {
        let args = command_args(&PipelinePlan::self_test());
        assert!(args.windows(2).any(|w| w == ["-f", "null"]));
        assert!(args.last().is_some_and(|arg| arg == "-"));
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
        plan.source = SourcePlan::Device {
            device_id: "not-a-device".into(),
            connector: None,
            binding_class: SourceBindingClass::Resolved,
        };
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
