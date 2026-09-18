//! Device Lease — exclusive ownership of a DeckLink device.
//! Frozen interface per SoT §15.2 (MEDIA-02). Gate 2.3 adds in-memory impl.
#![allow(dead_code)] // Gate 2.x: 部分接口尚未被上层调用, 编译期静音。

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use crate::source::LeaseKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLease {
    pub device_id: Uuid,
    pub owner: String, // agent session / pipeline id
    pub acquired_at: DateTime<Utc>,
    pub ttl: std::time::Duration,
}

/// Runtime lease keyed by a typed Device or Network identity.
///
/// `DeviceLease` remains the wire-compatible hardware projection; this type is
/// the ownership value used by the network-source migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeLease {
    pub key: LeaseKey,
    pub owner: String,
    pub acquired_at: DateTime<Utc>,
    pub ttl: std::time::Duration,
}

/// Lease lifecycle (acquire/release/renew/health). Shape frozen per SoT §15.2
/// (P0-7A 增补 Renew op 与 `Send + Sync` supertrait — 租约跨运行时线程持有,
/// SessionManager (Session 表) 与 watchdog 均跨线程访问)。
pub trait LeaseManager: Send + Sync {
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
    /// **有副作用**: 会清扫过期租约 — 仅供运维/tick 使用; 纯查询用 [`Self::list_active`]。
    fn health(&self) -> Vec<DeviceLease>;
    /// 纯读快照 (P0-7A Preflight judge-only): 返回当前表中全部租约
    /// (**含已过期但未清扫的**)。绝不修改存储 — Preflight 判定专用。
    fn list_active(&self) -> Vec<DeviceLease>;
    /// Renew (extend) a lease held by `owner` (RUNTIME_RESOURCE_MODEL §4.1 Renew op;
    /// P0-7A Session Runtime). Fails `NotFound` if absent; `AlreadyLeased` if held by
    /// a different owner (绝不可借 renew 抢占他人租约).
    fn renew(
        &self,
        device_id: &Uuid,
        owner: &str,
        ttl: std::time::Duration,
    ) -> Result<DeviceLease, LeaseError>;

    /// Acquire a lease for a typed runtime identity. Device keys use the
    /// legacy implementation; Network keys use the typed implementation.
    fn acquire_key(
        &self,
        key: &LeaseKey,
        owner: &str,
        ttl: std::time::Duration,
    ) -> Result<RuntimeLease, LeaseError> {
        match key {
            LeaseKey::Device(device_id) => {
                self.acquire(device_id, owner, ttl)
                    .map(|lease| RuntimeLease {
                        key: *key,
                        owner: lease.owner,
                        acquired_at: lease.acquired_at,
                        ttl: lease.ttl,
                    })
            }
            LeaseKey::Network(_) => Err(LeaseError::UnsupportedKey(*key)),
        }
    }

    /// Release a typed runtime lease.
    fn release_key(&self, lease: &RuntimeLease) -> Result<(), LeaseError> {
        match lease.key {
            LeaseKey::Device(device_id) => self.release(&DeviceLease {
                device_id,
                owner: lease.owner.clone(),
                acquired_at: lease.acquired_at,
                ttl: lease.ttl,
            }),
            LeaseKey::Network(_) => Err(LeaseError::UnsupportedKey(lease.key)),
        }
    }

    /// Renew a typed runtime lease.
    fn renew_key(
        &self,
        key: &LeaseKey,
        owner: &str,
        ttl: std::time::Duration,
    ) -> Result<RuntimeLease, LeaseError> {
        match key {
            LeaseKey::Device(device_id) => {
                self.renew(device_id, owner, ttl).map(|lease| RuntimeLease {
                    key: *key,
                    owner: lease.owner,
                    acquired_at: lease.acquired_at,
                    ttl: lease.ttl,
                })
            }
            LeaseKey::Network(_) => Err(LeaseError::UnsupportedKey(*key)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LeaseError {
    #[error("device {0} already leased")]
    AlreadyLeased(Uuid),
    #[error("device {0} not found")]
    NotFound(Uuid),
    #[error("runtime key {0:?} already leased")]
    AlreadyLeasedKey(LeaseKey),
    #[error("runtime key {0:?} not found")]
    NotFoundKey(LeaseKey),
    #[error("runtime key {0:?} is not supported by this lease manager")]
    UnsupportedKey(LeaseKey),
    #[error("lease expired")]
    Expired,
}

/// Gate 2.3 实现: 进程内内存租约表。单 agent 进程, Mutex 足矣。
///
/// 核心不变量(见 MEDIA_AGENT_STATE_MACHINE.md): DeckLink 掉线重启 pipeline 前,
/// MUST 用 `is_valid` 重新校验租约仍在有效期内 —— 绝不在无有效租约时采集。
pub struct InMemoryLeaseManager {
    leases: Mutex<HashMap<Uuid, DeviceLease>>,
    runtime_leases: Mutex<HashMap<LeaseKey, RuntimeLease>>,
}

impl InMemoryLeaseManager {
    pub fn new() -> Self {
        Self {
            leases: Mutex::new(HashMap::new()),
            runtime_leases: Mutex::new(HashMap::new()),
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

fn runtime_is_expired(l: &RuntimeLease) -> bool {
    let expiry = l
        .acquired_at
        .checked_add_signed(Duration::from_std(l.ttl).unwrap_or(Duration::MAX))
        .unwrap_or(Utc::now());
    Utc::now() > expiry
}

impl LeaseManager for InMemoryLeaseManager {
    fn acquire_key(
        &self,
        key: &LeaseKey,
        owner: &str,
        ttl: std::time::Duration,
    ) -> Result<RuntimeLease, LeaseError> {
        if let LeaseKey::Device(device_id) = key {
            let lease = self.acquire(device_id, owner, ttl)?;
            return Ok(RuntimeLease {
                key: *key,
                owner: lease.owner,
                acquired_at: lease.acquired_at,
                ttl: lease.ttl,
            });
        }

        let mut guard = self.runtime_leases.lock().unwrap();
        if guard.contains_key(key) {
            return Err(LeaseError::AlreadyLeasedKey(*key));
        }
        let lease = RuntimeLease {
            key: *key,
            owner: owner.to_string(),
            acquired_at: Utc::now(),
            ttl,
        };
        guard.insert(*key, lease.clone());
        Ok(lease)
    }

    fn release_key(&self, lease: &RuntimeLease) -> Result<(), LeaseError> {
        if let LeaseKey::Device(device_id) = lease.key {
            return self.release(&DeviceLease {
                device_id,
                owner: lease.owner.clone(),
                acquired_at: lease.acquired_at,
                ttl: lease.ttl,
            });
        }
        let mut guard = self.runtime_leases.lock().unwrap();
        if guard.remove(&lease.key).is_some() {
            Ok(())
        } else {
            Err(LeaseError::NotFoundKey(lease.key))
        }
    }

    fn renew_key(
        &self,
        key: &LeaseKey,
        owner: &str,
        ttl: std::time::Duration,
    ) -> Result<RuntimeLease, LeaseError> {
        if let LeaseKey::Device(device_id) = key {
            let lease = self.renew(device_id, owner, ttl)?;
            return Ok(RuntimeLease {
                key: *key,
                owner: lease.owner,
                acquired_at: lease.acquired_at,
                ttl: lease.ttl,
            });
        }
        let mut guard = self.runtime_leases.lock().unwrap();
        match guard.get_mut(key) {
            None => Err(LeaseError::NotFoundKey(*key)),
            Some(lease) if lease.owner != owner => Err(LeaseError::AlreadyLeasedKey(*key)),
            Some(lease) if runtime_is_expired(lease) => Err(LeaseError::Expired),
            Some(lease) => {
                lease.acquired_at = Utc::now();
                lease.ttl = ttl;
                Ok(lease.clone())
            }
        }
    }

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

    fn renew(
        &self,
        device_id: &Uuid,
        owner: &str,
        ttl: std::time::Duration,
    ) -> Result<DeviceLease, LeaseError> {
        let mut guard = self.leases.lock().unwrap();
        match guard.get_mut(device_id) {
            None => Err(LeaseError::NotFound(*device_id)),
            Some(l) if l.owner != owner => Err(LeaseError::AlreadyLeased(*device_id)),
            Some(l) => {
                l.acquired_at = Utc::now();
                l.ttl = ttl;
                Ok(l.clone())
            }
        }
    }

    fn list_active(&self) -> Vec<DeviceLease> {
        self.leases.lock().unwrap().values().cloned().collect()
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
    fn renew_extends_holder_lease_and_rejects_other_owner() {
        // P0-7A: Renew 仅限持有者本人; 他人 renew = AlreadyLeased (绝不借 renew 抢占).
        let lm = InMemoryLeaseManager::new();
        let id = Uuid::nil();
        lm.acquire(&id, "session-a", std::time::Duration::from_secs(5))
            .unwrap();
        let renewed = lm
            .renew(&id, "session-a", std::time::Duration::from_secs(60))
            .unwrap();
        assert_eq!(renewed.owner, "session-a");
        assert_eq!(renewed.ttl, std::time::Duration::from_secs(60));
        assert!(lm.is_valid(&id));
        assert!(matches!(
            lm.renew(&id, "session-b", std::time::Duration::from_secs(60)),
            Err(LeaseError::AlreadyLeased(_))
        ));
        assert!(matches!(
            lm.renew(
                &Uuid::new_v4(),
                "session-a",
                std::time::Duration::from_secs(60)
            ),
            Err(LeaseError::NotFound(_))
        ));
    }

    #[test]
    fn network_key_lease_is_exclusive_and_typed() {
        let lm = InMemoryLeaseManager::new();
        let key = LeaseKey::Network(crate::source::NetworkSourceId(Uuid::new_v4()));
        let lease = lm
            .acquire_key(&key, "session-a", std::time::Duration::from_secs(30))
            .unwrap();
        assert_eq!(lease.key, key);
        assert!(matches!(
            lm.acquire_key(&key, "session-b", std::time::Duration::from_secs(30)),
            Err(LeaseError::AlreadyLeasedKey(k)) if k == key
        ));
        assert!(matches!(
            lm.renew_key(&key, "session-b", std::time::Duration::from_secs(30)),
            Err(LeaseError::AlreadyLeasedKey(k)) if k == key
        ));
        let renewed = lm
            .renew_key(&key, "session-a", std::time::Duration::from_secs(60))
            .unwrap();
        assert_eq!(renewed.ttl, std::time::Duration::from_secs(60));
        lm.release_key(&renewed).unwrap();
        assert!(matches!(
            lm.renew_key(&key, "session-a", std::time::Duration::from_secs(30)),
            Err(LeaseError::NotFoundKey(k)) if k == key
        ));
    }

    #[test]
    fn health_returns_active_only() {
        let lm = InMemoryLeaseManager::new();
        lm.acquire(&Uuid::nil(), "a", std::time::Duration::from_secs(30))
            .unwrap();
        assert_eq!(lm.health().len(), 1);
    }
}
