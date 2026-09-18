//! Canonical source identities and endpoint values.
//!
//! RF-SRC-RTMP-01: network sources must not borrow DeviceId/PortId fields.
//! These types are the neutral handoff between Graph intent, Resource/Lease
//! ownership and backend-specific runtime binding.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Stable identity for a configured network source. It is not a hardware DeviceId.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkSourceId(pub Uuid);

/// Canonical input identity used by SessionInput and runtime events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceRef {
    Device(Uuid),
    Network(NetworkSourceId),
    SelfTest,
}

/// Resource ownership domain. ResourceRegistry remains the sole state owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceOwner {
    Device(Uuid),
    Network(NetworkSourceId),
}

impl Default for ResourceOwner {
    fn default() -> Self {
        Self::Device(Uuid::nil())
    }
}

/// Exclusive claim key. LeaseManager must never conflate these domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeaseKey {
    Device(Uuid),
    Network(NetworkSourceId),
}

impl From<ResourceOwner> for LeaseKey {
    fn from(owner: ResourceOwner) -> Self {
        match owner {
            ResourceOwner::Device(id) => Self::Device(id),
            ResourceOwner::Network(id) => Self::Network(id),
        }
    }
}

/// Network protocol admitted by the first bounded source packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkProtocol {
    Rtmp,
}

/// Credential-free canonical endpoint. Credentials and query options are not
/// representable, so they cannot leak into canonical state or backend logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkEndpoint {
    pub protocol: NetworkProtocol,
    pub host: String,
    pub port: u16,
    pub path: String,
}

impl NetworkEndpoint {
    pub fn validate(&self) -> Result<(), String> {
        if self.host.is_empty()
            || self.host.chars().any(|c| {
                c.is_control() || c.is_whitespace() || matches!(c, '@' | '?' | '#' | '"' | '\\')
            })
        {
            return Err("network endpoint host must be non-empty and whitespace-free".into());
        }
        if self.port == 0 {
            return Err("network endpoint port must be non-zero".into());
        }
        if !self.path.starts_with('/')
            || self.path.len() < 2
            || self
                .path
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
            || self.path.contains(['?', '#', '@', '"', '\''])
        {
            return Err("network endpoint path must be a credential-free RTMP path".into());
        }
        Ok(())
    }

    pub fn validate_loopback(&self) -> Result<(), String> {
        self.validate()?;
        if !matches!(self.host.as_str(), "127.0.0.1" | "localhost" | "::1") {
            return Err("acceptance endpoint must be loopback".into());
        }
        Ok(())
    }

    pub fn as_url(&self) -> Result<String, String> {
        self.validate()?;
        let scheme = match self.protocol {
            NetworkProtocol::Rtmp => "rtmp",
        };
        Ok(format!(
            "{scheme}://{}:{}{}",
            self.host, self.port, self.path
        ))
    }
}

impl fmt::Display for NetworkSourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(host: &str, path: &str) -> NetworkEndpoint {
        NetworkEndpoint {
            protocol: NetworkProtocol::Rtmp,
            host: host.into(),
            port: 1935,
            path: path.into(),
        }
    }

    #[test]
    fn source_domains_are_distinct_and_serde_roundtrip() {
        let id = NetworkSourceId(Uuid::new_v4());
        let network = SourceRef::Network(id);
        let json = serde_json::to_string(&network).unwrap();
        assert_eq!(serde_json::from_str::<SourceRef>(&json).unwrap(), network);
        assert_ne!(LeaseKey::Network(id), LeaseKey::Device(id.0));
        let key: LeaseKey = ResourceOwner::Network(id).into();
        assert_eq!(key, LeaseKey::Network(id));
    }

    #[test]
    fn endpoint_is_credential_free_and_builds_controlled_url() {
        let e = endpoint("127.0.0.1", "/live/source");
        assert_eq!(e.as_url().unwrap(), "rtmp://127.0.0.1:1935/live/source");
        assert!(e.validate_loopback().is_ok());
        assert!(endpoint("127.0.0.1", "/live/source?token=secret")
            .validate()
            .is_err());
        assert!(endpoint("user@host", "/live/source").validate().is_err());
        assert!(endpoint("10.0.0.1", "/live/source")
            .validate_loopback()
            .is_err());
    }
}
