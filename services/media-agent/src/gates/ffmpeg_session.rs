//! RF-FF-01E BMD acceptance: real SessionManager -> FFmpeg single-input lifecycle.
//!
//! Gate discipline: dependency construction comes from
//! `bootstrap::build_ffmpeg_session_composition`, the same factory consumed by
//! the production binary. This file only issues an explicit acceptance intent.

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use crate::lease::LeaseManager as _;
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use crate::port::PortDirection;
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use crate::resource::ResourceState;
#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
use crate::session::SessionPhase;

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("RF-FF-01E FAIL: {message}");
    std::process::exit(1);
}

#[cfg(all(feature = "bmd-provider", feature = "ffmpeg-backend"))]
pub fn run(world: &crate::bootstrap::BootstrapContext) {
    if std::env::var("VBMF_FFMPEG_SESSION").is_err() {
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
            "explicit DeviceHandle must resolve exactly once; handle={} matches={}",
            target_handle,
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
            "target must have exactly one authorized stable input port; device={} ports={}",
            target_id,
            ports.len()
        ));
    }
    let port_id = ports[0].identity.port_id.expect("checked Some");
    // Bootstrap's initialization lease yields only for the explicitly accepted device.
    let _ = world.lease_manager.release(&crate::lease::DeviceLease {
        device_id: target_id,
        owner: "bootstrap".into(),
        acquired_at: chrono::Utc::now(),
        ttl: std::time::Duration::from_secs(60),
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

    println!(
        "RF-FF-01E target handle={} device_id={} port_id={}",
        target_handle, target_id, port_id
    );
    let sid = composition
        .manager
        .create(intent)
        .unwrap_or_else(|e| fail(format!("Session create: {e}")));
    let created = composition.manager.status(&sid).expect("created session");
    if created.phase != SessionPhase::Leased {
        fail(format!(
            "create actual phase {:?}, expected Leased",
            created.phase
        ));
    }
    println!("RF-FF-01E create PASS session={sid} phase=Leased");
    composition
        .manager
        .start(&sid)
        .unwrap_or_else(|e| fail(format!("Session start: {e}")));
    std::thread::sleep(std::time::Duration::from_millis(1500));

    let running = composition.manager.status(&sid).expect("running session");
    if running.phase != SessionPhase::Running || running.inputs.len() != 1 {
        fail(format!(
            "running actual phase={:?} inputs={}, expected Running/1",
            running.phase,
            running.inputs.len()
        ));
    }
    let handle = running.pipeline.expect("Running must expose pipeline");
    let events = composition.backend.observe(&handle);
    if !events.is_empty() {
        fail(format!("FFmpeg backend not alive/clean: {events:?}"));
    }
    let running_state = composition.manager.runtime_state();
    if !running_state
        .resources
        .iter()
        .any(|r| r.device_id == target_id && r.state == ResourceState::Allocated)
    {
        fail("target Resource was not Allocated while Session Running");
    }
    println!(
        "RF-FF-01E start PASS phase=Running pipeline={} backend_alive=true resource=Allocated",
        handle.0
    );

    composition
        .manager
        .stop(&sid)
        .unwrap_or_else(|e| fail(format!("Session stop: {e}")));
    let released = composition.manager.status(&sid).expect("released session");
    if released.phase != SessionPhase::Released
        || released.pipeline.is_some()
        || !released.inputs.is_empty()
    {
        fail(format!(
            "stop actual phase={:?} pipeline={:?} inputs={}",
            released.phase,
            released.pipeline,
            released.inputs.len()
        ));
    }
    let released_state = composition.manager.runtime_state();
    if released_state
        .resources
        .iter()
        .any(|r| r.device_id == target_id && r.state != ResourceState::Available)
    {
        fail("target Resource not fully Available after stop");
    }
    if world
        .lease_manager
        .health()
        .iter()
        .any(|l| l.device_id == target_id)
    {
        fail("target Device lease remains after stop");
    }
    composition
        .manager
        .close(&sid)
        .unwrap_or_else(|e| fail(format!("Session close: {e}")));
    if composition.manager.status(&sid).is_some() {
        fail("Session still present after close");
    }

    println!("RF-FF-01E stop PASS phase=Released resources=Available lease=NONE");
    println!("RF_FF_01E_BMD_SESSION_PASS");
    std::process::exit(0);
}
