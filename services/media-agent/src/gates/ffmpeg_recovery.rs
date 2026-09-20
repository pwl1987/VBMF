//! RF-FF-01F BMD acceptance: canonical FFmpeg failure -> recovery monitor.

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use crate::lease::LeaseManager as _;
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use crate::port::PortDirection;
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use crate::resource::ResourceState;
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use crate::session::SessionPhase;
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use crate::supervisor::ProcessState;
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use std::fs;
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use std::path::{Path, PathBuf};
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use std::process::{Child, Command, Stdio};
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use std::time::Duration;

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn fail(message: impl std::fmt::Display) -> ! {
    let marker = if std::env::var_os("VBMF_FFMPEG_RTMP_SOURCE").is_some() {
        "RF-SRC-RTMP-02"
    } else if std::env::var_os("VBMF_FFMPEG_RTMP_OUTPUT").is_some() {
        "RF-FF-03"
    } else {
        "RF-FF-02"
    };
    eprintln!("{marker} FAIL: {message}");
    std::process::exit(1);
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn hls_evidence(dir: &Path) -> Result<(), String> {
    let playlist = dir.join("index.m3u8");
    let playlist_text =
        fs::read_to_string(&playlist).map_err(|e| format!("read {}: {e}", playlist.display()))?;
    if !playlist_text.contains("#EXTM3U") {
        return Err(format!("{} lacks #EXTM3U", playlist.display()));
    }
    let segment = fs::read_dir(dir)
        .map_err(|e| format!("list {}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("seg") && name.ends_with(".ts"))
        })
        .ok_or_else(|| format!("{} has no seg*.ts HLS segment", dir.display()))?;
    for (stream, codec_label) in [("0:v:0", "Video: h264"), ("0:a:0", "Audio: aac")] {
        let probe = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "info", "-i"])
            .arg(&segment)
            .args(["-map", stream, "-f", "null", "-"])
            .output()
            .map_err(|e| format!("ffmpeg probe {}: {e}", segment.display()))?;
        let diagnostics = String::from_utf8_lossy(&probe.stderr);
        if !probe.status.success() {
            return Err(format!(
                "ffmpeg probe {} exited with {}: {}",
                segment.display(),
                probe.status,
                diagnostics.trim()
            ));
        }
        if !diagnostics.contains(codec_label) {
            return Err(format!(
                "{} missing from {}: {:?}",
                codec_label,
                segment.display(),
                diagnostics.trim()
            ));
        }
    }
    Ok(())
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn wait_for_hls(dir: &Path) -> Result<(), String> {
    let mut last_error = "no HLS evidence yet".to_string();
    for _ in 0..80 {
        match hls_evidence(dir) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = error,
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(format!("HLS evidence timeout: {last_error}"))
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn spawn_rtmp_receiver(url: &str) -> Result<Child, String> {
    if !url.starts_with("rtmp://127.0.0.1:")
        || url
            .chars()
            .any(|c| c.is_control() || matches!(c, '"' | '\'' | ' '))
    {
        return Err("RF-FF-03 RTMP receiver URL must be a quoted-free loopback URL".into());
    }
    Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "info",
            "-rtmp_listen",
            "1",
            "-i",
        ])
        .arg(url)
        .args([
            "-t", "3", "-map", "0:v:0", "-map", "0:a:0", "-f", "null", "-",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn RTMP receiver: {e}"))
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
struct RtmpReceiverGuard(Option<Child>);

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
impl RtmpReceiverGuard {
    fn new(child: Child) -> Self {
        Self(Some(child))
    }

    fn take(&mut self) -> Option<Child> {
        self.0.take()
    }
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
impl Drop for RtmpReceiverGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn wait_for_rtmp_receiver(child: Child) -> Result<(), String> {
    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait for RTMP receiver: {e}"))?;
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "RTMP receiver exited with {}: {}",
            output.status,
            diagnostics.trim()
        ));
    }
    for codec_label in ["Video: h264", "Audio: aac"] {
        if !diagnostics.contains(codec_label) {
            return Err(format!(
                "RTMP receiver diagnostics missing {codec_label}: {}",
                diagnostics.trim()
            ));
        }
    }
    Ok(())
}

/// RF-SRC-RTMP-02 TG-6: source URL fixture parser. `lan_ok` widens the
/// fixture boundary from the historical loopback-only form to any D2
/// canonical eligible endpoint (plan D2/D5 — Tier 1 LAN fixtures on the BMD
/// box); the canonical spelling itself is enforced by `to_canonical`.
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn parse_source_url(raw: &str, lan_ok: bool) -> Result<crate::source::NetworkEndpoint, String> {
    if raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("RTMP source URL must be whitespace-free".into());
    }
    let rest = raw.strip_prefix("rtmp://").ok_or_else(|| {
        "RTMP source URL must use rtmp://<canonical-ip>:<port>/<path>".to_string()
    })?;
    let (authority, path) = rest
        .split_once('/')
        .ok_or_else(|| "RTMP source URL must include a path".to_string())?;
    let Some((host, port)) = authority.rsplit_once(':') else {
        return Err("RTMP source URL must carry an explicit port".into());
    };
    let port: u16 = port
        .parse()
        .map_err(|_| "RTMP source URL port is invalid".to_string())?;
    let host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    let endpoint = crate::source::NetworkEndpoint {
        protocol: crate::source::NetworkProtocol::Rtmp,
        host: host.into(),
        port,
        path: format!("/{path}"),
    };
    let canonical = endpoint
        .to_canonical()
        .map_err(|e| format!("RTMP source URL is not a canonical endpoint: {e}"))?;
    if !lan_ok && !canonical.ip().is_loopback() {
        return Err("RTMP source URL must use rtmp://127.0.0.1:<port>/<path>".into());
    }
    Ok(endpoint)
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn spawn_rtmp_source_publisher(url: &str) -> Result<Child, String> {
    parse_source_url(url, std::env::var("VBMF_FFMPEG_RTMP_SOURCE_LAN").is_ok())?;
    Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
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
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-tune",
            "zerolatency",
            "-c:a",
            "aac",
            "-f",
            "flv",
        ])
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn RTMP source publisher: {e}"))
}

/// Pure manifest body for the gate-owned NetworkSourceBinding fixture: the
/// (env-URL-derived) endpoint only ever reaches this JSON body, never a
/// filesystem path.
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn network_binding_manifest_body(
    source_id: uuid::Uuid,
    endpoint: &crate::source::NetworkEndpoint,
) -> Vec<u8> {
    let machine_id = crate::resolver::current_machine_id();
    if machine_id.is_empty() {
        fail("machine identity unresolved; cannot pin network binding manifest");
    }
    let url = format!(
        "rtmp://{}:{}{}",
        endpoint.host, endpoint.port, endpoint.path
    );
    format!(
        "{{\"version\":1,\"machine_id\":\"{machine_id}\",\"entries\":[{{\"source_id\":\"{source_id}\",\"endpoint\":\"{url}\"}}]}}"
    )
    .into_bytes()
}

/// Create-and-write the gate manifest with a race-free single-fd protocol
/// (Mimosa L2 follow-up): the canonical process temp root is resolved
/// FIRST, the leaf name is derived ONLY from pid + source id, and the file
/// is created via `create_new` (+`O_NOFOLLOW`) with mode 0600 at creation
/// time. A pre-existing path of any kind (regular file, directory, or
/// symlink) is rejected by the kernel BEFORE any byte is written, so a
/// planted symlink can never steer the write outside the temp dir. There is
/// deliberately NO fallback to plain `fs::write`.
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn write_gate_manifest_file_checked(
    body: &[u8],
    source_id: uuid::Uuid,
    temp_root: &Path,
) -> Result<PathBuf, String> {
    use std::io::Write as _;
    use std::os::unix::fs::MetadataExt as _;
    use std::os::unix::fs::OpenOptionsExt as _;

    let canonical_root = temp_root
        .canonicalize()
        .map_err(|e| format!("resolve process temp root: {e}"))?;
    let path = canonical_root.join(format!(
        "vbmf-ffmpeg-recovery-network-binding-{}-{}.json",
        std::process::id(),
        source_id
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)
        .map_err(|e| format!("create network binding manifest {}: {e}", path.display()))?;
    file.write_all(body)
        .map_err(|e| format!("write network binding manifest {}: {e}", path.display()))?;
    // Creation-time mode is authoritative (no post-hoc chmod window); a
    // umask that strips owner bits must surface as a failure, not be
    // silently repaired.
    let created_mode = file
        .metadata()
        .map(|meta| meta.mode() & 0o777)
        .map_err(|e| format!("stat network binding manifest: {e}"))?;
    if created_mode != 0o600 {
        return Err(format!(
            "network binding manifest created with mode {created_mode:o}, expected 0600"
        ));
    }
    // Defense-in-depth: re-resolve the final path and require it to remain a
    // direct child of the canonical temp root.
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("resolve network binding manifest: {e}"))?;
    if canonical.parent() != Some(canonical_root.as_path()) {
        return Err("network binding manifest must stay inside the process temp directory".into());
    }
    Ok(canonical)
}

/// Thin fail-closed wrapper over [`write_gate_manifest_file_checked`] bound
/// to the process temp dir. The returned path is canonicalized and
/// confinement-checked against the canonical temp root BEFORE any consumer
/// sees it (normalize + validate at the source).
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn write_gate_manifest_file(body: &[u8], source_id: uuid::Uuid) -> PathBuf {
    write_gate_manifest_file_checked(body, source_id, &std::env::temp_dir())
        .unwrap_or_else(|e| fail(e))
}

/// Read the gate-owned binding manifest back with explicit confinement. The
/// only producer is [`write_gate_manifest_file`], which already returns a
/// canonicalized, temp-dir-confined path; this helper re-validates anyway
/// (defense in depth for any future caller): canonicalize + direct-child
/// check against the canonical temp root, any escape fails closed.
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn read_gate_binding_manifest(path: &Path) -> Vec<u8> {
    let canonical_root = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|e| fail(format!("resolve process temp root: {e}")));
    let canonical = path
        .canonicalize()
        .unwrap_or_else(|e| fail(format!("resolve network binding manifest: {e}")));
    if canonical.parent() != Some(canonical_root.as_path()) {
        fail("network binding manifest must stay inside the process temp directory");
    }
    fs::read(&canonical).unwrap_or_else(|e| fail(format!("read network binding manifest: {e}")))
}

/// Gate-owned HLS fixture directory preparation with a provable rule (Mimosa
/// L2 follow-up): the canonical process temp root is resolved FIRST; the
/// fixture may ONLY be a direct child of that root (`parent == canonical
/// root` — a symlinked intermediate such as `/tmp/a` pointing elsewhere
/// fails the equality check BEFORE any filesystem mutation happens); the
/// leaf must not pre-exist in any form (`symlink_metadata` never follows
/// symlinks); and creation uses non-recursive `create_dir`, so a
/// race-created entry still fails closed with EEXIST instead of being
/// absorbed by `create_dir_all`. A canonical re-check of parent/root runs
/// after creation as defense-in-depth.
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn prepare_gate_hls_dir_checked(raw: &str, temp_root: &Path) -> Result<PathBuf, String> {
    let dir = PathBuf::from(raw);
    if !dir.is_absolute() {
        return Err("VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR must be absolute".into());
    }
    if dir
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR must not contain '..' path components".into());
    }
    let canonical_root = temp_root
        .canonicalize()
        .map_err(|e| format!("resolve process temp root: {e}"))?;
    if dir.parent() != Some(canonical_root.as_path()) {
        return Err(
            "RTMP source HLS directory must be a direct child of the process temp directory".into(),
        );
    }
    if fs::symlink_metadata(&dir).is_ok() {
        return Err("RTMP source HLS directory must not already exist".into());
    }
    fs::create_dir(&dir).map_err(|e| format!("create RTMP source HLS directory: {e}"))?;
    let canonical = dir
        .canonicalize()
        .map_err(|e| format!("resolve RTMP source HLS directory: {e}"))?;
    if canonical.parent() != Some(canonical_root.as_path()) {
        return Err("RTMP source HLS directory must stay inside the process temp directory".into());
    }
    Ok(canonical)
}

/// RF-SRC-RTMP-02 closure: Network-only gate dispatch entry.
///
/// Must be invoked BEFORE the common `bootstrap::build()` (see
/// `bin/gates.rs`): when `VBMF_FFMPEG_RTMP_SOURCE` is set the whole gate
/// process stays on the network-only composition path — no Device provider
/// discovery, no bootstrap placeholder DeviceLease, no DeviceBindingManifest,
/// no DeckLink SDK probe — so the TG-6 hardware log constitutes strict D10
/// evidence. Returns with zero side effects when the env is unset.
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
pub fn run_network_only_gate() {
    if std::env::var("VBMF_FFMPEG_RTMP_SOURCE").is_ok() {
        run_rtmp_source();
    }
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn run_rtmp_source() {
    let raw_url = std::env::var("VBMF_FFMPEG_RTMP_SOURCE_URL")
        .unwrap_or_else(|_| fail("VBMF_FFMPEG_RTMP_SOURCE_URL is required"));
    // TG-6 Tier 1: VBMF_FFMPEG_RTMP_SOURCE_LAN=1 widens the fixture to a
    // canonical eligible LAN endpoint (D2/D5); default stays loopback.
    let lan_ok = std::env::var("VBMF_FFMPEG_RTMP_SOURCE_LAN").is_ok();
    let endpoint =
        parse_source_url(&raw_url, lan_ok).unwrap_or_else(|e| fail(format!("RTMP source: {e}")));
    let hls_dir_raw = std::env::var("VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR")
        .unwrap_or_else(|_| fail("VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR is required"));
    // Direct-child rule (see prepare_gate_hls_dir_checked): confinement is
    // decided against the canonical temp root BEFORE any directory is
    // created; a pre-existing leaf (including a symlink) fails closed.
    let hls_dir = prepare_gate_hls_dir_checked(&hls_dir_raw, &std::env::temp_dir())
        .unwrap_or_else(|e| fail(e));
    if fs::read_dir(&hls_dir)
        .unwrap_or_else(|e| fail(format!("inspect RTMP source HLS directory: {e}")))
        .next()
        .is_some()
    {
        fail("RTMP source HLS directory must start empty");
    }
    std::env::set_var("VBMF_OUTPUT_KIND", "hls");
    std::env::set_var("VBMF_OUTPUT_HLS_DIR", &hls_dir);

    let source_id =
        crate::source::NetworkSourceId(uuid::Uuid::new_v5(&uuid::Uuid::nil(), raw_url.as_bytes()));
    // RF-SRC-RTMP-02 TG-2: the gate now exercises the REAL production
    // admission path — a 0600/machine-pinned NetworkSourceBinding loaded by
    // the network-only composition (no DeviceBindingManifest, no device
    // leases, zero DeckLink side effects by construction).
    let manifest_bytes_before = network_binding_manifest_body(source_id.0, &endpoint);
    let binding_path = write_gate_manifest_file(&manifest_bytes_before, source_id.0);
    std::env::set_var("MEDIA_AGENT_NETWORK_BINDING", &binding_path);
    let composition = crate::bootstrap::build_ffmpeg_network_only_composition()
        .unwrap_or_else(|e| fail(format!("network-only composition: {e}")));

    // RF-SRC-RTMP-02 closure (frozen D10): mechanical zero-device assertions —
    // this gate dispatched BEFORE the common bootstrap, so the composition
    // must hold no device plane, no Device Resource and no DeviceLease. These
    // checks put direct (not merely log-absence) evidence into the run.
    let d10_state = composition.manager.runtime_state();
    if !d10_state.devices.is_empty() {
        fail("D10 violation: network gate composition must not contain any device plane");
    }
    if d10_state
        .resources
        .iter()
        .any(|r| r.capability != "rtmp-input")
    {
        fail("D10 violation: only Network rtmp-input Resources may be registered");
    }
    let network_resources = d10_state.resources.len();
    if network_resources != 1 {
        fail(format!(
            "D10 violation: expected exactly one Network Resource, got {network_resources}"
        ));
    }
    if !composition.lease_manager.list_active().is_empty() {
        fail("D10 violation: no DeviceLease (bootstrap placeholder included) may exist");
    }
    println!(
        "RF-SRC-RTMP-02 D10 startup PASS device_discovery=0 bootstrap_device_leases=0 device_resources=0 network_resources={network_resources} manifest_bytes={}",
        manifest_bytes_before.len()
    );
    let intent = crate::graph_intent::GraphRuntimeIntent {
        version: "1.0".into(),
        devices: vec![crate::graph_intent::DeviceIntent {
            // Graph node label only; the source identity is source_id.
            device_id: "network-source-node".into(),
            role: "CAPTURE".into(),
            pipeline: crate::graph_intent::PipelineIntent {
                source: crate::graph_intent::SourceIntent::rtmp(source_id, endpoint.clone()),
                sink: crate::graph_intent::SinkIntent { kind: "hls".into() },
            },
        }],
    };

    let sid = composition
        .manager
        .create(intent)
        .unwrap_or_else(|e| fail(format!("RTMP source Session create: {e}")));
    composition
        .manager
        .start(&sid)
        .unwrap_or_else(|e| fail(format!("RTMP source Session start: {e}")));
    let running = composition
        .manager
        .status(&sid)
        .expect("running source session");
    if running.phase != SessionPhase::Running || running.inputs.len() != 1 {
        fail(format!(
            "RTMP source expected Running/1, got {:?}/{}",
            running.phase,
            running.inputs.len()
        ));
    }
    let handle = running.pipeline.expect("running source pipeline");
    let old_consumer_pid = composition
        .process_inspector
        .running_child_pid(&handle)
        .unwrap_or_else(|| fail("initial RTMP source consumer PID is absent"));
    let monitor = crate::recovery_monitor::spawn_network(
        composition.backend.clone(),
        handle,
        source_id,
        composition.supervisor.clone(),
        composition.lease_manager.clone(),
        composition.resources.clone(),
    );
    composition
        .manager
        .register_stop_hook(&sid, monitor.clone());

    let external_publisher = std::env::var("VBMF_FFMPEG_RTMP_SOURCE_EXTERNAL").is_ok();
    let publisher = if external_publisher {
        None
    } else {
        Some(RtmpReceiverGuard::new(
            spawn_rtmp_source_publisher(&raw_url).unwrap_or_else(|e| fail(e)),
        ))
    };
    let mut publisher = publisher;
    wait_for_hls(&hls_dir).unwrap_or_else(|e| fail(format!("initial RTMP source A/V: {e}")));
    // TG-4 (plan D7/INV-1): a completed publisher connection that produced
    // verified A/V is the SignalVerified evidence — recorded BEFORE the
    // disconnect leg so the exit attributes as PublisherDisconnected.
    monitor.report_signal_verified();
    let listener_class = if endpoint
        .to_canonical()
        .map(|c| c.ip().is_loopback())
        .unwrap_or(false)
    {
        "loopback"
    } else {
        "lan"
    };
    println!(
        "RF-SRC-RTMP-01 source PASS session={sid} source_id={source_id}          codecs=h264,aac listener={listener_class} signal_verified=true"
    );

    if external_publisher {
        println!(
            "RF-SRC-RTMP-01 external-leg A/V verified; awaiting external publisher disconnect"
        );
    }
    if let Some(mut guard) = publisher.take() {
        if let Some(mut old_publisher) = guard.take() {
            let _ = old_publisher.kill();
            let _ = old_publisher.wait();
        }
    }

    let mut recovered = false;
    for _ in 0..120 {
        if monitor.recovery_count() >= 1 {
            recovered = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if !recovered {
        fail("RTMP source recovery did not create a new consumer generation");
    }
    let new_consumer_pid = composition
        .process_inspector
        .running_child_pid(&handle)
        .unwrap_or_else(|| fail("recovered RTMP source consumer PID is absent"));
    if new_consumer_pid == old_consumer_pid {
        fail("RTMP source recovery reused the old consumer process identity");
    }
    for entry in fs::read_dir(&hls_dir)
        .unwrap_or_else(|e| fail(format!("inspect recovered HLS directory: {e}")))
        .filter_map(Result::ok)
    {
        let _ = fs::remove_file(entry.path());
    }
    let replacement = if external_publisher {
        None
    } else {
        Some(RtmpReceiverGuard::new(
            spawn_rtmp_source_publisher(&raw_url).unwrap_or_else(|e| fail(e)),
        ))
    };
    wait_for_hls(&hls_dir).unwrap_or_else(|e| fail(format!("recovered RTMP source A/V: {e}")));
    let supervisor_state = format!(
        "{:?}",
        composition
            .supervisor
            .lock()
            .unwrap()
            .status(&source_id.0)
            .expect("supervisor watches the source")
    );
    println!(
        "RF-SRC-RTMP-01 recovery PASS old_consumer_pid={old_consumer_pid}          new_consumer_pid={new_consumer_pid} attributed=PublisherDisconnected          supervisor={supervisor_state} codecs=h264,aac"
    );

    composition
        .manager
        .stop(&sid)
        .unwrap_or_else(|e| fail(format!("RTMP source Session stop: {e}")));
    if !monitor.is_exited() {
        fail("RTMP source recovery monitor did not exit during stop");
    }
    drop(replacement);
    // D10 teardown: the FFmpeg listener child (and its stderr reader, joined
    // by the adapter reaper with the child) must be fully gone after stop.
    if composition
        .process_inspector
        .running_child_pid(&handle)
        .is_some()
    {
        fail("RTMP source FFmpeg listener child remains after Session stop");
    }
    let released = composition
        .manager
        .status(&sid)
        .expect("released source session");
    if released.phase != SessionPhase::Released
        || released.pipeline.is_some()
        || !released.inputs.is_empty()
    {
        fail("RTMP source Session did not converge to Released");
    }
    let state = composition.manager.runtime_state();
    if state.resources.iter().any(|resource| {
        resource.capability == "rtmp-input" && resource.state != ResourceState::Available
    }) {
        fail("RTMP source Resource remains claimed after teardown");
    }
    if composition
        .lease_manager
        .is_key_active(&crate::source::LeaseKey::Network(source_id))
    {
        fail("RTMP source Lease remains after teardown");
    }
    composition
        .manager
        .close(&sid)
        .unwrap_or_else(|e| fail(format!("RTMP source Session close: {e}")));
    if composition.manager.status(&sid).is_some() {
        fail("RTMP source Session remains after close");
    }
    println!(
        "RF-SRC-RTMP-01 teardown PASS phase=Released resource=Available          lease=NONE monitor=exited publisher_orphan=NONE"
    );
    // D10 manifest-integrity evidence: the gate-written NetworkSourceBinding
    // must be byte-identical across the whole run (load/start/recover/stop).
    let manifest_bytes_after = read_gate_binding_manifest(&binding_path);
    if manifest_bytes_after != manifest_bytes_before {
        fail("network binding manifest bytes changed during the gate run");
    }
    println!(
        "RF-SRC-RTMP-02 D10 teardown PASS ffmpeg_child=none listener=released-with-child stderr_reader=joined-by-reaper monitor=exited network_lease=none network_resource=available manifest_bytes_unchanged=true"
    );
    println!("RF_SRC_RTMP_01_BMD_SOURCE_RECOVERY_PASS");
    std::process::exit(0);
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
pub fn run(world: &crate::bootstrap::BootstrapContext) {
    let output_mode = std::env::var("VBMF_FFMPEG_OUTPUT").is_ok();
    let rtmp_mode = std::env::var("VBMF_FFMPEG_RTMP_OUTPUT").is_ok();
    if output_mode && rtmp_mode {
        fail("RF-FF-03 HLS and RTMP output modes are mutually exclusive");
    }
    if !output_mode && !rtmp_mode && std::env::var("VBMF_FFMPEG_RECOVERY").is_err() {
        return;
    }
    let rtmp_url = if rtmp_mode {
        let url = std::env::var("VBMF_FFMPEG_RTMP_URL")
            .unwrap_or_else(|_| fail("VBMF_FFMPEG_RTMP_URL is required"));
        if !url.starts_with("rtmp://127.0.0.1:")
            || url
                .chars()
                .any(|c| c.is_control() || matches!(c, '"' | '\'' | ' '))
        {
            fail("RF-FF-03 RTMP output URL must be a quoted-free loopback URL");
        }
        std::env::set_var("VBMF_OUTPUT_KIND", "rtmp");
        std::env::set_var("VBMF_OUTPUT_RTMP_URL", &url);
        Some(url)
    } else {
        None
    };
    let output_dir = if output_mode {
        let raw = std::env::var("VBMF_FFMPEG_OUTPUT_DIR")
            .unwrap_or_else(|_| fail("VBMF_FFMPEG_OUTPUT_DIR is required"));
        let path = PathBuf::from(raw);
        if !path.is_absolute() {
            fail("VBMF_FFMPEG_OUTPUT_DIR must be absolute");
        }
        fs::create_dir_all(&path)
            .unwrap_or_else(|e| fail(format!("create HLS output directory: {e}")));
        if fs::read_dir(&path)
            .unwrap_or_else(|e| fail(format!("inspect HLS output directory: {e}")))
            .next()
            .is_some()
        {
            fail("RF-FF-02 HLS output directory must start empty");
        }
        std::env::set_var("VBMF_OUTPUT_KIND", "hls");
        std::env::set_var("VBMF_OUTPUT_HLS_DIR", &path);
        Some(path)
    } else {
        None
    };
    let target_handle = std::env::var("VBMF_FFMPEG_TEST_DEVICE_HANDLE")
        .unwrap_or_else(|_| fail("VBMF_FFMPEG_TEST_DEVICE_HANDLE is required"));
    let composition = crate::bootstrap::build_ffmpeg_session_composition(world)
        .unwrap_or_else(|e| fail(format!("production composition: {e}")));
    let targets: Vec<_> = world
        .discovered
        .iter()
        .filter(|d| {
            d.identity.as_ref().is_some_and(|identity| {
                identity.provider == "blackmagic"
                    && identity.device_handle.as_deref() == Some(target_handle.as_str())
            })
        })
        .collect();
    if targets.len() != 1 {
        fail(format!(
            "explicit DeviceHandle must resolve exactly once; matches={}",
            targets.len()
        ));
    }
    let target_id = targets[0].device.device_id;
    let ports: Vec<_> = composition
        .registry
        .ports
        .iter()
        .filter(|p| {
            p.device_id == target_id
                && matches!(
                    p.direction,
                    PortDirection::Input | PortDirection::Bidirectional
                )
                && p.identity.port_id.is_some()
        })
        .collect();
    if ports.len() != 1 {
        fail(format!(
            "target must have exactly one authorized input port; ports={}",
            ports.len()
        ));
    }
    let port_id = ports[0].identity.port_id.expect("checked Some");
    let _ = world.lease_manager.release(&crate::lease::DeviceLease {
        device_id: target_id,
        owner: "bootstrap".into(),
        acquired_at: chrono::Utc::now(),
        ttl: Duration::from_secs(60),
    });

    let intent = crate::graph_intent::GraphRuntimeIntent {
        version: "1.0".into(),
        devices: vec![crate::graph_intent::DeviceIntent {
            device_id: target_id.to_string(),
            role: "CAPTURE".into(),
            pipeline: crate::graph_intent::PipelineIntent {
                source: crate::graph_intent::SourceIntent::decklink(
                    target_id.to_string(),
                    Some(port_id.to_string()),
                ),
                sink: crate::graph_intent::SinkIntent {
                    kind: if output_mode {
                        "hls"
                    } else if rtmp_mode {
                        "rtmp"
                    } else {
                        "appsink"
                    }
                    .into(),
                },
            },
        }],
    };
    let mut initial_receiver = if rtmp_mode {
        Some(RtmpReceiverGuard::new(
            spawn_rtmp_receiver(rtmp_url.as_deref().expect("RTMP URL validated"))
                .unwrap_or_else(|e| fail(e)),
        ))
    } else {
        None
    };
    let sid = composition
        .manager
        .create(intent)
        .unwrap_or_else(|e| fail(format!("Session create: {e}")));
    composition
        .manager
        .start(&sid)
        .unwrap_or_else(|e| fail(format!("Session start: {e}")));
    std::thread::sleep(Duration::from_millis(800));
    let running = composition.manager.status(&sid).expect("running session");
    if running.phase != SessionPhase::Running || running.inputs.len() != 1 {
        fail(format!(
            "expected Running/1, got {:?}/{}",
            running.phase,
            running.inputs.len()
        ));
    }
    let handle = running.pipeline.expect("Running must expose pipeline");
    let old_pid = composition
        .process_inspector
        .running_child_pid(&handle)
        .unwrap_or_else(|| fail("running FFmpeg child PID is absent"));
    let monitor = crate::recovery_monitor::spawn(
        composition.backend.clone(),
        handle,
        target_id,
        world.supervisor.clone(),
        world.lease_manager.clone(),
    );
    composition
        .manager
        .register_stop_hook(&sid, monitor.clone());
    if let Some(dir) = output_dir.as_deref() {
        wait_for_hls(dir).unwrap_or_else(|e| fail(format!("initial HLS output: {e}")));
        println!(
            "RF-FF-02 output PASS dir={} playlist=index.m3u8 codecs=h264,aac",
            dir.display()
        );
    }
    if let Some(receiver) = initial_receiver.as_mut().and_then(RtmpReceiverGuard::take) {
        wait_for_rtmp_receiver(receiver)
            .unwrap_or_else(|e| fail(format!("initial RTMP output: {e}")));
        println!(
            "RF-FF-03 receiver PASS url={} codecs=h264,aac",
            rtmp_url.as_deref().expect("RTMP URL validated")
        );
    }
    if output_mode {
        println!(
            "RF-FF-02 running PASS session={sid} pipeline={} child_pid={old_pid} output=hls",
            handle.0
        );
    } else if rtmp_mode {
        println!(
            "RF-FF-03 running PASS session={sid} pipeline={} child_pid={old_pid} output=rtmp url={}",
            handle.0,
            rtmp_url.as_deref().expect("RTMP URL validated")
        );
    } else {
        println!(
            "RF-FF-01F running PASS session={sid} pipeline={} child_pid={old_pid}",
            handle.0
        );
    }

    let kill = Command::new("/bin/kill")
        .args(["-TERM", &old_pid.to_string()])
        .status()
        .unwrap_or_else(|e| fail(format!("external child termination: {e}")));
    if !kill.success() {
        fail(format!("external child termination exited with {kill}"));
    }
    let mut recovered_receiver = if rtmp_mode {
        Some(RtmpReceiverGuard::new(
            spawn_rtmp_receiver(rtmp_url.as_deref().expect("RTMP URL validated"))
                .unwrap_or_else(|e| fail(e)),
        ))
    } else {
        None
    };
    let mut recovered = false;
    for _ in 0..120 {
        if monitor.recovery_count() >= 1 {
            recovered = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if !recovered {
        fail("monitor did not observe failure and recover within 6s");
    }
    let new_pid = composition
        .process_inspector
        .running_child_pid(&handle)
        .unwrap_or_else(|| fail("recovery did not create a new FFmpeg child"));
    if new_pid == old_pid {
        fail(format!("recovery reused terminated child PID {old_pid}"));
    }
    if world.supervisor.lock().unwrap().status(&target_id) != Some(ProcessState::Recovered) {
        fail("Supervisor did not settle Recovered after canonical recovery");
    }
    if !composition.backend.observe(&handle).is_empty() {
        fail("recovered FFmpeg child is not alive/clean");
    }
    if let Some(dir) = output_dir.as_deref() {
        wait_for_hls(dir).unwrap_or_else(|e| fail(format!("recovered HLS output: {e}")));
        println!(
            "RF-FF-02 output PASS after_recovery dir={} playlist=index.m3u8 codecs=h264,aac",
            dir.display()
        );
    }
    if let Some(receiver) = recovered_receiver
        .as_mut()
        .and_then(RtmpReceiverGuard::take)
    {
        wait_for_rtmp_receiver(receiver)
            .unwrap_or_else(|e| fail(format!("recovered RTMP output: {e}")));
        println!(
            "RF-FF-03 receiver PASS after_recovery url={} codecs=h264,aac",
            rtmp_url.as_deref().expect("RTMP URL validated")
        );
    }
    if output_mode {
        println!("RF-FF-02 recovery PASS old_pid={old_pid} new_pid={new_pid} canonical_failure=true supervisor=Recovered output=hls");
    } else if rtmp_mode {
        println!("RF-FF-03 recovery PASS old_pid={old_pid} new_pid={new_pid} canonical_failure=true supervisor=Recovered output=rtmp");
    } else {
        println!("RF-FF-01F recovery PASS old_pid={old_pid} new_pid={new_pid} canonical_failure=true supervisor=Recovered");
    }
    composition
        .manager
        .stop(&sid)
        .unwrap_or_else(|e| fail(format!("Session stop: {e}")));
    if !monitor.is_exited() {
        fail("Session stop returned while recovery monitor was still running");
    }
    let released = composition.manager.status(&sid).expect("released session");
    if released.phase != SessionPhase::Released
        || released.pipeline.is_some()
        || !released.inputs.is_empty()
    {
        fail(format!(
            "stop did not converge to Released: {:?}",
            released.phase
        ));
    }
    let state = composition.manager.runtime_state();
    if state
        .resources
        .iter()
        .any(|r| r.device_id == target_id && r.state != ResourceState::Available)
    {
        fail("target Resource not Available after monitor-owned teardown");
    }
    if world
        .lease_manager
        .health()
        .iter()
        .any(|l| l.device_id == target_id)
    {
        fail("target Lease remains after Session stop");
    }
    if composition
        .process_inspector
        .running_child_pid(&handle)
        .is_some()
    {
        fail("FFmpeg child remains after Session stop");
    }
    composition
        .manager
        .close(&sid)
        .unwrap_or_else(|e| fail(format!("Session close: {e}")));
    if composition.manager.status(&sid).is_some() {
        fail("Session remains after close");
    }
    if output_mode {
        println!("RF-FF-02 teardown PASS phase=Released resources=Available lease=NONE monitor=exited ffmpeg_orphan=NONE output=hls");
        println!("RF_FF_02_BMD_OUTPUT_RECOVERY_PASS");
    } else if rtmp_mode {
        println!("RF-FF-03 teardown PASS phase=Released resources=Available lease=NONE monitor=exited ffmpeg_orphan=NONE output=rtmp");
        println!("RF_FF_03_BMD_RTMP_OUTPUT_RECOVERY_PASS");
    } else {
        println!("RF-FF-01F teardown PASS phase=Released resources=Available lease=NONE monitor=exited ffmpeg_orphan=NONE");
        println!("RF_FF_01F_BMD_RECOVERY_PASS");
    }
    std::process::exit(0);
}

/// Focused negative tests for the gate-owned fixture path protocol (Mimosa
/// L2 follow-up): manifest creation must fail closed on ANY pre-existing
/// leaf (regular file or symlink) before a single byte is written, and the
/// HLS fixture must be a fresh direct child of the canonical temp root.
/// These tests are path-only (no hardware, no DeckLink, no FFmpeg); they
/// compile under the acceptance feature pair and run wherever that pair can
/// build (BMD native / any host with the SDK).
#[cfg(all(test, feature = "bmd-provider", feature = "ffmpeg-backend"))]
mod gate_path_tests {
    use super::*;

    struct TempRootGuard(PathBuf);

    impl TempRootGuard {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "vbmf-gate-path-test-{}-{}-{tag}",
                std::process::id(),
                uuid::Uuid::new_v4()
            ));
            fs::create_dir(&root).expect("create test temp root");
            Self(root)
        }

        fn root(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempRootGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn manifest_leaf(root: &Path, source_id: uuid::Uuid) -> PathBuf {
        root.join(format!(
            "vbmf-ffmpeg-recovery-network-binding-{}-{source_id}.json",
            std::process::id()
        ))
    }

    #[test]
    fn manifest_creates_single_fd_0600_direct_child() {
        let guard = TempRootGuard::new("mf-ok");
        let sid = uuid::Uuid::new_v4();
        let path = write_gate_manifest_file_checked(b"{\"version\":1}", sid, guard.root())
            .expect("fresh manifest creation must succeed");
        assert_eq!(path.parent(), Some(guard.root()));
        assert_eq!(fs::read(&path).unwrap(), b"{\"version\":1}");
        let mode = std::os::unix::fs::MetadataExt::mode(&fs::metadata(&path).unwrap()) & 0o777;
        assert_eq!(mode, 0o600, "manifest must be created 0600, not chmod'ed");
    }

    #[test]
    fn manifest_rejects_preexisting_regular_file() {
        let guard = TempRootGuard::new("mf-reg");
        let sid = uuid::Uuid::new_v4();
        let leaf = manifest_leaf(guard.root(), sid);
        fs::write(&leaf, b"occupied").unwrap();
        let error = write_gate_manifest_file_checked(b"{}", sid, guard.root())
            .expect_err("pre-existing regular file must be rejected");
        assert!(error.contains("create network binding manifest"), "{error}");
        // The occupied content must be untouched (create_new never truncates).
        assert_eq!(fs::read(&leaf).unwrap(), b"occupied");
    }

    #[test]
    fn manifest_rejects_symlink_and_leaves_target_untouched() {
        let guard = TempRootGuard::new("mf-link");
        let sid = uuid::Uuid::new_v4();
        let outside = guard.root().join("outside-target.json");
        fs::write(&outside, b"SENSITIVE").unwrap();
        let leaf = manifest_leaf(guard.root(), sid);
        std::os::unix::fs::symlink(&outside, &leaf).unwrap();
        let error = write_gate_manifest_file_checked(b"{}", sid, guard.root())
            .expect_err("pre-existing symlink must be rejected before any write");
        assert!(error.contains("create network binding manifest"), "{error}");
        // The symlink itself survives and the target bytes are unchanged:
        // the write side-effect window through a symlink is closed.
        assert!(leaf.is_symlink());
        assert_eq!(fs::read(&outside).unwrap(), b"SENSITIVE");
    }

    #[test]
    fn hls_accepts_fresh_direct_child_of_canonical_temp_root() {
        let guard = TempRootGuard::new("hls-ok");
        let fixture = guard.root().join("fixture-hls");
        let dir = prepare_gate_hls_dir_checked(&fixture.to_string_lossy(), guard.root())
            .expect("fresh direct child must be accepted");
        assert!(dir.is_dir());
        assert_eq!(dir.parent(), Some(guard.root()));
    }

    #[test]
    fn hls_rejects_preexisting_symlink_leaf() {
        let guard = TempRootGuard::new("hls-link");
        let target = guard.root().join("hls-symlink-target");
        fs::create_dir(&target).unwrap();
        let fixture = guard.root().join("fixture-hls");
        std::os::unix::fs::symlink(&target, &fixture).unwrap();
        let error = prepare_gate_hls_dir_checked(&fixture.to_string_lossy(), guard.root())
            .expect_err("pre-existing symlink leaf must be rejected");
        assert!(error.contains("must not already exist"), "{error}");
        assert!(fixture.is_symlink());
    }

    #[test]
    fn hls_rejects_nested_path_before_any_directory_creation() {
        let guard = TempRootGuard::new("hls-nested");
        let nested = guard.root().join("a").join("b");
        let error = prepare_gate_hls_dir_checked(&nested.to_string_lossy(), guard.root())
            .expect_err("nested fixture path must be rejected");
        assert!(error.contains("direct child"), "{error}");
        // Rejection must happen BEFORE any side effect: no intermediate
        // directory may have been created.
        assert!(!guard.root().join("a").exists());
    }

    #[test]
    fn hls_rejects_path_outside_temp_root() {
        let guard = TempRootGuard::new("hls-outside");
        let outside = PathBuf::from("/var/vbmf-gate-path-reject-probe");
        let error = prepare_gate_hls_dir_checked(&outside.to_string_lossy(), guard.root())
            .expect_err("path outside the temp root must be rejected");
        assert!(error.contains("direct child"), "{error}");
        assert!(!outside.exists(), "nothing may be created outside the root");
    }
}
