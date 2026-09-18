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
use std::process::Command;
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use std::sync::Arc;
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use std::time::Duration;

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("RF-FF-01F FAIL: {message}");
    std::process::exit(1);
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
pub fn run(world: &crate::bootstrap::BootstrapContext) {
    if std::env::var("VBMF_FFMPEG_RECOVERY").is_err() {
        return;
    }
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
                source: crate::graph_intent::SourceIntent {
                    kind: "decklink".into(),
                    device_id: target_id.to_string(),
                    port_id: Some(port_id.to_string()),
                },
                sink: crate::graph_intent::SinkIntent {
                    kind: "appsink".into(),
                },
            },
        }],
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
        .backend
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
    println!(
        "RF-FF-01F running PASS session={sid} pipeline={} child_pid={old_pid}",
        handle.0
    );

    let kill = Command::new("/bin/kill")
        .args(["-TERM", &old_pid.to_string()])
        .status()
        .unwrap_or_else(|e| fail(format!("external child termination: {e}")));
    if !kill.success() {
        fail(format!("external child termination exited with {kill}"));
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
        fail("monitor did not observe failure and recover within 6s");
    }
    let new_pid = composition
        .backend
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
    println!("RF-FF-01F recovery PASS old_pid={old_pid} new_pid={new_pid} canonical_failure=true supervisor=Recovered");
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
    println!("RF-FF-01F teardown PASS phase=Released resources=Available lease=NONE monitor=exited ffmpeg_orphan=NONE");
    println!("RF_FF_01F_BMD_RECOVERY_PASS");
    std::process::exit(0);
}
