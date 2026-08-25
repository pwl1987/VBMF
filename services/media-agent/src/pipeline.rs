//! GStreamer Pipeline Controller — capture lifecycle.
//! Frozen interface per SoT §15.2. No logic yet.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    pub device_id: Uuid,
    pub video_mode: String,   // e.g. "1080p59.94"
    pub audio: bool,
}

/// Create/start/pause/stop/recover a GStreamer pipeline (decklinkvideosrc → …).
pub trait PipelineController {
    fn start(&self, spec: &PipelineSpec) -> Result<PipelineHandle, PipelineError>;
    fn stop(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    /// MEDIA-03: recover a hung/crashed pipeline.
    fn recover(&self, handle: &PipelineHandle) -> Result<PipelineHandle, PipelineError>;
}

#[derive(Debug, Clone)]
pub struct PipelineHandle(pub Uuid);

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("gstreamer init failed: {0}")]
    Init(String),
    #[error("device enumerate failed (0 devices)")]
    NoDevice,
    #[error("pipeline crashed")]
    Crashed,
}
