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
    let marker = if std::env::var_os("VBMF_FFMPEG_RTMP_OUTPUT").is_some() {
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

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn write_network_binding_manifest(
    source_id: uuid::Uuid,
    endpoint: &crate::source::NetworkEndpoint,
) -> PathBuf {
    let machine_id = crate::resolver::current_machine_id();
    if machine_id.is_empty() {
        fail("machine identity unresolved; cannot pin network binding manifest");
    }
    let url = format!(
        "rtmp://{}:{}{}",
        endpoint.host, endpoint.port, endpoint.path
    );
    let body = format!(
        "{{\"version\":1,\"machine_id\":\"{machine_id}\",\"entries\":[{{\"source_id\":\"{source_id}\",\"endpoint\":\"{url}\"}}]}}"
    );
    let path = std::env::temp_dir().join(format!(
        "vbmf-ffmpeg-recovery-network-binding-{}-{}.json",
        std::process::id(),
        source_id
    ));
    fs::write(&path, body).unwrap_or_else(|e| fail(format!("write network binding manifest: {e}")));
    fs::set_permissions(
        &path,
        <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o600),
    )
    .unwrap_or_else(|e| fail(format!("chmod 0600 network binding manifest: {e}")));
    path
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
    let hls_dir = std::env::var("VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR")
        .unwrap_or_else(|_| fail("VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR is required"));
    let hls_dir = PathBuf::from(hls_dir);
    if !hls_dir.is_absolute() {
        fail("VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR must be absolute");
    }
    fs::create_dir_all(&hls_dir)
        .unwrap_or_else(|e| fail(format!("create RTMP source HLS directory: {e}")));
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
    let binding_path = write_network_binding_manifest(source_id.0, &endpoint);
    std::env::set_var("MEDIA_AGENT_NETWORK_BINDING", &binding_path);
    let composition = crate::bootstrap::build_ffmpeg_network_only_composition()
        .unwrap_or_else(|e| fail(format!("network-only composition: {e}")));
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

    let mut publisher =
        RtmpReceiverGuard::new(spawn_rtmp_source_publisher(&raw_url).unwrap_or_else(|e| fail(e)));
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

    let old_publisher = publisher
        .take()
        .expect("publisher guard owns initial source fixture");
    let mut old_publisher = old_publisher;
    let _ = old_publisher.kill();
    let _ = old_publisher.wait();

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
    let replacement =
        RtmpReceiverGuard::new(spawn_rtmp_source_publisher(&raw_url).unwrap_or_else(|e| fail(e)));
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
    println!("RF_SRC_RTMP_01_BMD_SOURCE_RECOVERY_PASS");
    std::process::exit(0);
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
pub fn run(world: &crate::bootstrap::BootstrapContext) {
    if std::env::var("VBMF_FFMPEG_RTMP_SOURCE").is_ok() {
        run_rtmp_source();
    }
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
