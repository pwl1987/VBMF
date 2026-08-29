//! Device Lease — exclusive ownership of a DeckLink device.
//! Frozen interface per SoT §15.2 (MEDIA-02). Gate 2.3 adds in-memory impl.
#![allow(dead_code)] // Gate 2.x: 部分接口尚未被上层调用, 编译期静音。

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLease {
    pub device_id: Uuid,
    pub owner: String, // agent session / pipeline id
    pub acquired_at: DateTime<Utc>,
    pub ttl: std::time::Duration,
}

/// Lease lifecycle (acquire/release/health). Shape frozen per SoT §15.2.
pub trait LeaseManager {
    /// Acquire exclusive lease; fails if already leased (prevents host ffmpeg / double-capture).
    fn acquire(
        &self,
        device_id: &Uuid,
        owner: &str,
        ttl: std::time::Duration,
    ) -> Result<DeviceLease, LeaseError>;
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

/// Gate 2.3 实现: 进程内内存租约表。单 agent 进程, Mutex 足矣。
///
/// 核心不变量(见 MEDIA_AGENT_STATE_MACHINE.md): DeckLink 掉线重启 pipeline 前,
/// MUST 用 `is_valid` 重新校验租约仍在有效期内 —— 绝不在无有效租约时采集。
pub struct InMemoryLeaseManager {
    leases: Mutex<HashMap<Uuid, DeviceLease>>,
}

impl InMemoryLeaseManager {
    pub fn new() -> Self {
        Self {
            leases: Mutex::new(HashMap::new()),
        }
    }

    /// Re-validate a lease. Called by Supervisor before re-entering CAPTURING
    /// after a DeckLink drop / restart. Returns false if absent or expired.
    pub fn is_valid(&self, device_id: &Uuid) -> bool {
        let guard = self.leases.lock().unwrap();
        match guard.get(device_id) {
            Some(l) => !is_expired(l),
            None => false,
        }
    }
}

impl Default for InMemoryLeaseManager {
    fn default() -> Self {
        Self::new()
    }
}

fn is_expired(l: &DeviceLease) -> bool {
    let expiry = l
        .acquired_at
        .checked_add_signed(Duration::from_std(l.ttl).unwrap_or(Duration::MAX))
        .unwrap_or(Utc::now());
    Utc::now() > expiry
}

impl LeaseManager for InMemoryLeaseManager {
    fn acquire(
        &self,
        device_id: &Uuid,
        owner: &str,
        ttl: std::time::Duration,
    ) -> Result<DeviceLease, LeaseError> {
        let mut guard = self.leases.lock().unwrap();
        if guard.contains_key(device_id) {
            return Err(LeaseError::AlreadyLeased(*device_id));
        }
        let lease = DeviceLease {
            device_id: *device_id,
            owner: owner.to_string(),
            acquired_at: Utc::now(),
            ttl,
        };
        guard.insert(*device_id, lease.clone());
        Ok(lease)
    }

    fn release(&self, lease: &DeviceLease) -> Result<(), LeaseError> {
        let mut guard = self.leases.lock().unwrap();
        if guard.remove(&lease.device_id).is_some() {
            Ok(())
        } else {
            // NotFound 也视作成功释放(幂等), 但按接口约定返回错误以暴露误调用。
            Err(LeaseError::NotFound(lease.device_id))
        }
    }

    fn health(&self) -> Vec<DeviceLease> {
        let mut guard = self.leases.lock().unwrap();
        let now = Utc::now();
        // 自动清理过期租约(对应状态机 "租约过期 → RECOVERING/READY")。
        guard.retain(|_, l| {
            let expiry = l
                .acquired_at
                .checked_add_signed(Duration::from_std(l.ttl).unwrap_or(Duration::MAX))
                .unwrap_or(now);
            now <= expiry
        });
        guard.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_then_duplicate_fails() {
        let lm = InMemoryLeaseManager::new();
        let id = Uuid::nil();
        let l1 = lm
            .acquire(&id, "a", std::time::Duration::from_secs(30))
            .unwrap();
        assert_eq!(l1.device_id, id);
        let err = lm.acquire(&id, "b", std::time::Duration::from_secs(30));
        assert!(matches!(err, Err(LeaseError::AlreadyLeased(_))));
    }

    #[test]
    fn release_removes() {
        let lm = InMemoryLeaseManager::new();
        let id = Uuid::nil();
        let l = lm
            .acquire(&id, "a", std::time::Duration::from_secs(30))
            .unwrap();
        assert!(lm.is_valid(&id));
        lm.release(&l).unwrap();
        assert!(!lm.is_valid(&id));
        assert!(matches!(lm.release(&l), Err(LeaseError::NotFound(_))));
    }

    #[test]
    fn health_returns_active_only() {
        let lm = InMemoryLeaseManager::new();
        lm.acquire(&Uuid::nil(), "a", std::time::Duration::from_secs(30))
            .unwrap();
        assert_eq!(lm.health().len(), 1);
    }
}
