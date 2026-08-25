//! Device Lease — exclusive ownership of a DeckLink device.
//! Frozen interface per SoT §15.2 (MEDIA-02). No logic yet.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLease {
    pub device_id: Uuid,
    pub owner: String,        // agent session / pipeline id
    pub acquired_at: DateTime<Utc>,
    pub ttl: std::time::Duration,
}

/// Lease lifecycle (acquire/release/health). No implementation.
pub trait LeaseManager {
    /// Acquire exclusive lease; fails if already leased (prevents host ffmpeg / double-capture).
    fn acquire(&self, device_id: &Uuid, owner: &str, ttl: std::time::Duration) -> Result<DeviceLease, LeaseError>;
    /// Release lease (explicit or on crash via MEDIA-03).
    fn release(&self, lease: &DeviceLease) -> Result<(), LeaseError>;
    /// Heartbeat / TTL check; expired leases auto-released.
    fn health(&self) -> Vec<DeviceLease>;
}

#[derive(Debug, thiserror::Error)]
pub enum LeaseError {
    #[error("device {0} already leased")]
    AlreadyLeased(Uuid),
    #[error("device {0} not found")]
    NotFound(Uuid),
    #[error("lease expired")]
    Expired,
}
