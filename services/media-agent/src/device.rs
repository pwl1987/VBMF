//! Device Manager — DeckLink hardware discovery & status.
//! Frozen interface per SoT §15.2 (MEDIA-02/04). No logic yet.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A discovered DeckLink device (maps to /dev/blackmagic/dv0, dv1, io0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: Uuid,
    pub node: String,        // e.g. "/dev/blackmagic/dv0"
    pub model: String,       // e.g. "DeckLink Quad HDMI Recorder"
    pub serial: String,
    pub state: DeviceState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceState {
    Available,
    Leased,
    Capturing,
    Error,
    /// MEDIA-04: device went away (cable lost / driver restart)
    Lost,
}

/// Discovery + status surface (no implementation).
pub trait DeviceManager {
    /// Enumerate DeckLink devices via DesktopVideoHelper / Blackmagic SDK.
    fn discover(&self) -> Vec<DeviceInfo>;
    /// Live status of a device (health, signal presence).
    fn status(&self, device_id: &Uuid) -> DeviceState;
}

/// MEDIA-04: hotplug event channel (udev / DesktopVideoHelper IPC).
#[derive(Debug, Clone)]
pub enum HotplugEvent {
    Attached(DeviceInfo),
    Detached(Uuid),
}
