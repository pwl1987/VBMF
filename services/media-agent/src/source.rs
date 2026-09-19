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

// ===== RF-SRC-RTMP-02 TG-1 (plan D2): canonical strong endpoint types =====
//
// `NetworkEndpoint` above stays the wire-compatible value type. The types below
// are the only currency for production admission: endpoint parsing, manifest
// matching and (in later packets) registry deduplication and FFmpeg argv
// construction must consume them, so no raw string-comparison path can bypass
// D2 validation. `Debug` is deliberately redacted (plan D9): canonical values
// never print host, port or path content. Explicit accessors (`to_url`,
// `as_str`) are the adapter-local escape hatches (manifest input / argv).

/// Lowest listener port; privileged ports (<1024) are rejected (plan D2).
pub const MIN_LISTENER_PORT: u16 = 1024;

/// D2: host text byte limit, enforced before any address parsing operation.
pub const MAX_HOST_TEXT_BYTES: usize = 255;

/// Canonical IP literal — only the strict canonical spelling parses; nothing
/// is normalized on accept. IPv4 is strict dotted decimal without leading
/// zeros; IPv6 is the lowercase compressed RFC 5952 form.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CanonicalIp {
    repr: IpRepr,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum IpRepr {
    V4(std::net::Ipv4Addr),
    V6(std::net::Ipv6Addr),
}

impl CanonicalIp {
    /// Strict parse. Hostnames/DNS names, zone ids, mapped/non-canonical
    /// spellings and malformed literals are rejected. Host text longer than
    /// [`MAX_HOST_TEXT_BYTES`] is rejected before any address parse.
    pub fn parse_strict(text: &str) -> Result<Self, String> {
        if text.len() > MAX_HOST_TEXT_BYTES {
            return Err("endpoint host text exceeds 255 bytes".into());
        }
        if let Ok(v4) = text.parse::<std::net::Ipv4Addr>() {
            if v4.to_string() != text {
                return Err("ipv4 literal must use canonical spelling".into());
            }
            return Ok(Self {
                repr: IpRepr::V4(v4),
            });
        }
        if let Ok(v6) = text.parse::<std::net::Ipv6Addr>() {
            if v6.to_string() != text {
                return Err("ipv6 literal must use canonical lowercase compressed spelling".into());
            }
            return Ok(Self {
                repr: IpRepr::V6(v6),
            });
        }
        Err("endpoint host must be a canonical ipv4/ipv6 literal (hostnames rejected)".into())
    }

    pub fn is_loopback(&self) -> bool {
        match &self.repr {
            IpRepr::V4(v4) => v4.is_loopback(),
            IpRepr::V6(v6) => v6.is_loopback(),
        }
    }

    /// D2 eligible listener classes: loopback (local fixtures), RFC1918
    /// private IPv4, IPv6 ULA. Everything else — public unicast, link-local,
    /// multicast, broadcast, wildcard/unspecified, IPv4-mapped IPv6, CGNAT,
    /// documentation and reserved ranges — is rejected by this allowlist.
    pub fn is_eligible_bind_class(&self) -> bool {
        match &self.repr {
            IpRepr::V4(v4) => {
                let o = v4.octets();
                v4.is_loopback()
                    || o[0] == 10
                    || (o[0] == 172 && (16..=31).contains(&o[1]))
                    || (o[0] == 192 && o[1] == 168)
            }
            IpRepr::V6(v6) => {
                let segments = v6.segments();
                v6.is_loopback() || (segments[0] & 0xfe00) == 0xfc00
            }
        }
    }

    /// URL host form: IPv6 is bracketed (`[fd00::10]`), IPv4 stays bare.
    pub fn to_url_host(&self) -> String {
        match &self.repr {
            IpRepr::V4(v4) => v4.to_string(),
            IpRepr::V6(v6) => format!("[{v6}]"),
        }
    }

    /// Standard-library view of the same address (D5 local-interface
    /// ownership comparison). Same value, different currency; no re-parse.
    pub fn as_ip_addr(&self) -> std::net::IpAddr {
        match &self.repr {
            IpRepr::V4(v4) => std::net::IpAddr::V4(*v4),
            IpRepr::V6(v6) => std::net::IpAddr::V6(*v6),
        }
    }
}

impl fmt::Debug for CanonicalIp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Redacted by design (plan D9): never print address content.
        f.write_str("CanonicalIp(redacted)")
    }
}

/// Canonical RTMP path — an exact admission label (plan D2/D4). Case is
/// significant, a trailing `/` is a distinct value, and no equivalence
/// normalization happens. Segment alphabet is `[A-Za-z0-9._-]`; empty
/// segments, dot segments (`.`/`..`), percent-encoding, query, fragment,
/// backslash, control and whitespace characters are all rejected.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CanonicalPath(String);

impl CanonicalPath {
    fn is_segment_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
    }

    pub fn parse_strict(text: &str) -> Result<Self, String> {
        if !text.starts_with('/') || text.len() < 2 {
            return Err("path must be a non-empty slash-prefixed route label".into());
        }
        let body = match text.strip_suffix('/') {
            // A trailing slash stays verbatim (distinct canonical value); the
            // remaining body must still hold at least one real segment.
            Some(stripped) => {
                if stripped.len() < 2 {
                    return Err("path must contain at least one segment".into());
                }
                stripped
            }
            None => text,
        };
        for segment in body[1..].split('/') {
            if segment.is_empty() {
                return Err("path must not contain empty segments".into());
            }
            if segment == "." || segment == ".." {
                return Err("path must not contain dot segments".into());
            }
            if !segment.chars().all(Self::is_segment_char) {
                return Err("path segments may only contain [A-Za-z0-9._-]".into());
            }
        }
        Ok(Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CanonicalPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CanonicalPath(redacted)")
    }
}

/// D6 registry uniqueness key: `(protocol, canonical_ip, port)`. The path is
/// deliberately NOT part of the listener identity — two paths on one
/// protocol/ip/port are never two independent listeners.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerKey {
    ip: CanonicalIp,
    port: u16,
}

impl ListenerKey {
    pub fn ip(&self) -> &CanonicalIp {
        &self.ip
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl fmt::Debug for ListenerKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ListenerKey(redacted)")
    }
}

/// Canonical RTMP listener endpoint: `rtmp://<canonical-ip>:<explicit-port>/<path>`.
/// Non-canonical spellings are rejected, never normalized; the address class
/// must be an eligible bind class and the port must be in 1024..=65535.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CanonicalRtmpEndpoint {
    ip: CanonicalIp,
    port: u16,
    path: CanonicalPath,
}

impl CanonicalRtmpEndpoint {
    /// D2 canonical port text: plain ASCII decimal digits only, spelling
    /// exactly the canonical decimal (`01935`/`+1935`-style forms reject).
    fn parse_port_strict(text: &str) -> Result<u16, String> {
        if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
            return Err("endpoint port must be plain decimal digits".into());
        }
        let port: u16 = text
            .parse()
            .map_err(|_| "endpoint port must be a decimal u16".to_string())?;
        if port.to_string() != text {
            return Err("endpoint port must use canonical decimal spelling".into());
        }
        Ok(port)
    }

    /// Strict D2 parse of the full URL form (manifest entries). IPv6 hosts
    /// must be bracketed; the port is explicit (implicit 1935 rejected);
    /// userinfo/query/fragment cannot be represented.
    pub fn parse_strict(text: &str) -> Result<Self, String> {
        let Some(rest) = text.strip_prefix("rtmp://") else {
            return Err("endpoint must use the rtmp:// scheme".into());
        };
        let Some((authority, path_text)) = rest.split_once('/') else {
            return Err("endpoint must include a non-empty path".into());
        };
        let (host_text, port_text) = if let Some(bracketed) = authority.strip_prefix('[') {
            let Some((v6_text, tail)) = bracketed.split_once(']') else {
                return Err("unterminated ipv6 host brackets".into());
            };
            let Some(port) = tail.strip_prefix(':') else {
                return Err("explicit port is required".into());
            };
            (v6_text, port)
        } else {
            match authority.split_once(':') {
                Some((host, port)) => {
                    if host.contains(':') || port.contains(':') {
                        return Err("ipv6 endpoint hosts must be bracketed".into());
                    }
                    (host, port)
                }
                None => return Err("explicit port is required (implicit port rejected)".into()),
            }
        };
        let port: u16 = Self::parse_port_strict(port_text)?;
        if port < MIN_LISTENER_PORT {
            return Err("privileged ports below 1024 are rejected".into());
        }
        let ip = CanonicalIp::parse_strict(host_text)?;
        if !ip.is_eligible_bind_class() {
            return Err(
                "address class is not an eligible listener (loopback/private/ULA only)".into(),
            );
        }
        let path = CanonicalPath::parse_strict(&format!("/{path_text}"))?;
        Ok(Self { ip, port, path })
    }

    pub fn ip(&self) -> &CanonicalIp {
        &self.ip
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn path(&self) -> &CanonicalPath {
        &self.path
    }

    /// Adapter-local URL materialization (argv construction). This is one of
    /// the explicit D9 escape hatches; never log the result.
    pub fn to_url(&self) -> String {
        format!(
            "rtmp://{}:{}{}",
            self.ip.to_url_host(),
            self.port,
            self.path.as_str()
        )
    }

    pub fn listener_key(&self) -> ListenerKey {
        ListenerKey {
            ip: self.ip,
            port: self.port,
        }
    }
}

impl fmt::Debug for CanonicalRtmpEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CanonicalRtmpEndpoint(redacted)")
    }
}

impl NetworkEndpoint {
    /// Bridge the wire value into the canonical strong type (D2/D11). The
    /// wire host may be bare or bracketed IPv6, but the spelling must already
    /// be canonical; hostnames, ineligible address classes, privileged ports
    /// and non-canonical paths are rejected.
    pub fn to_canonical(&self) -> Result<CanonicalRtmpEndpoint, String> {
        match self.protocol {
            NetworkProtocol::Rtmp => {}
        }
        let host_text = self
            .host
            .strip_prefix('[')
            .and_then(|h| h.strip_suffix(']'))
            .unwrap_or(&self.host);
        let ip = CanonicalIp::parse_strict(host_text)?;
        if !ip.is_eligible_bind_class() {
            return Err(
                "address class is not an eligible listener (loopback/private/ULA only)".into(),
            );
        }
        if self.port < MIN_LISTENER_PORT {
            return Err("privileged ports below 1024 are rejected".into());
        }
        let path = CanonicalPath::parse_strict(&self.path)?;
        Ok(CanonicalRtmpEndpoint {
            ip,
            port: self.port,
            path,
        })
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

    // ===== RF-SRC-RTMP-02 TG-1 (plan D2): canonical endpoint tests =====

    fn canonical(url: &str) -> Result<CanonicalRtmpEndpoint, String> {
        CanonicalRtmpEndpoint::parse_strict(url)
    }

    #[test]
    fn rf_src_rtmp_02_canonical_ipv4_eligible_classes_only() {
        for ok in [
            "10.0.0.5",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.10.20",
            "127.0.0.1",
        ] {
            assert!(
                canonical(&format!("rtmp://{ok}:19350/live/source")).is_ok(),
                "{ok}"
            );
        }
        // public, link-local, multicast, broadcast, wildcard, CGNAT,
        // documentation ranges, reserved — all rejected (allowlist policy).
        for bad in [
            "8.8.8.8",
            "169.254.169.254",
            "224.0.0.1",
            "255.255.255.255",
            "0.0.0.0",
            "100.64.0.1",
            "100.127.255.254",
            "192.0.2.1",
            "198.51.100.7",
            "203.0.113.9",
            "240.0.0.1",
        ] {
            assert!(
                canonical(&format!("rtmp://{bad}:19350/live/source")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn rf_src_rtmp_02_canonical_ipv6_requires_canonical_spelling() {
        for ok in ["fd00::10", "fd00::", "::1", "fd12::a:1"] {
            let url = format!("rtmp://[{ok}]:19350/live/source");
            assert!(canonical(&url).is_ok(), "{url}");
        }
        // uppercase, expanded (non-compressed), leading-zero group spellings
        // are rejected: only the RFC 5952 canonical form parses.
        for bad in ["FD00::10", "fd00:0:0:0:0:0:0:10", "fd00:0000::10"] {
            let url = format!("rtmp://[{bad}]:19350/live/source");
            assert!(canonical(&url).is_err(), "{url}");
        }
        // unbracketed ipv6 in URL form is rejected outright.
        assert!(canonical("rtmp://fd00::10:19350/live/source").is_err());
    }

    #[test]
    fn rf_src_rtmp_02_canonical_ipv6_eligible_classes_only() {
        assert!(canonical("rtmp://[::1]:19350/live/source").is_ok());
        assert!(canonical("rtmp://[fd12:3456:789a:bcde:f:1:2:3]:19350/live/source").is_ok());
        // link-local, multicast, documentation, public, unspecified and
        // IPv4-mapped ipv6 are all rejected.
        for bad in [
            "fe80::1",
            "ff02::1",
            "2001:db8::1",
            "2001:4860:4860::8888",
            "::",
            "::ffff:10.0.0.1",
            "::ffff:127.0.0.1",
        ] {
            let url = format!("rtmp://[{bad}]:19350/live/source");
            assert!(canonical(&url).is_err(), "{url}");
        }
    }

    #[test]
    fn rf_src_rtmp_02_endpoint_port_policy() {
        // missing / implicit port rejected; privileged rejected; 1024..=65535 ok.
        assert!(canonical("rtmp://10.0.0.5/live/source").is_err());
        assert!(canonical("rtmp://[fd00::10]/live/source").is_err());
        assert!(canonical("rtmp://10.0.0.5:1023/live/source").is_err());
        assert!(canonical("rtmp://10.0.0.5:0/live/source").is_err());
        assert!(canonical("rtmp://10.0.0.5:65536/live/source").is_err());
        assert!(canonical("rtmp://10.0.0.5:1024/live/source").is_ok());
        assert!(canonical("rtmp://10.0.0.5:65535/live/source").is_ok());
    }

    #[test]
    fn rf_src_rtmp_02_endpoint_port_text_must_be_canonical_decimal() {
        // only the plain canonical decimal spelling of the port parses;
        // leading zeros, signs, whitespace and junk are rejected even though
        // u16::from_str would accept some of them.
        for bad in [
            "rtmp://10.0.0.5:019350/live/source",
            "rtmp://10.0.0.5:01024/live/source",
            "rtmp://10.0.0.5:+1935/live/source",
            "rtmp://10.0.0.5:-1935/live/source",
            "rtmp://10.0.0.5: 1935/live/source",
            "rtmp://10.0.0.5:1935x/live/source",
            "rtmp://10.0.0.5:x1935/live/source",
            "rtmp://10.0.0.5:１９３５/live/source", // fullwidth digits
            "rtmp://10.0.0.5:/live/source",
            "rtmp://[fd00::10]:019350/live/source",
        ] {
            let err = canonical(bad).unwrap_err();
            assert!(
                err.contains("port"),
                "{bad}: unexpected rejection reason: {err}"
            );
        }
        // the canonical spelling still parses
        assert!(canonical("rtmp://10.0.0.5:19350/live/source").is_ok());
        assert!(canonical("rtmp://10.0.0.5:1024/live/source").is_ok());
    }

    #[test]
    fn rf_src_rtmp_02_host_text_over_255_bytes_rejected_before_parse() {
        // D2: overlong host text is rejected by the byte-limit check itself,
        // before any address parsing operation, on every entry path.
        let overlong = "h".repeat(MAX_HOST_TEXT_BYTES + 1);
        assert_eq!(
            CanonicalIp::parse_strict(&overlong).unwrap_err(),
            "endpoint host text exceeds 255 bytes"
        );
        let url = format!("rtmp://{overlong}:19350/live/source");
        assert_eq!(
            canonical(&url).unwrap_err(),
            "endpoint host text exceeds 255 bytes"
        );
        let wire = NetworkEndpoint {
            protocol: NetworkProtocol::Rtmp,
            host: overlong,
            port: 19350,
            path: "/live/source".into(),
        };
        assert_eq!(
            wire.to_canonical().unwrap_err(),
            "endpoint host text exceeds 255 bytes"
        );
    }

    #[test]
    fn rf_src_rtmp_02_endpoint_path_grammar() {
        assert!(canonical("rtmp://10.0.0.5:19350/live/source").is_ok());
        // trailing slash is a distinct canonical value
        let a = canonical("rtmp://10.0.0.5:19350/live/source").unwrap();
        let b = canonical("rtmp://10.0.0.5:19350/live/source/").unwrap();
        assert_ne!(a, b);
        assert_eq!(b.path().as_str(), "/live/source/");
        for bad_path in [
            "",
            "/",
            "//",
            "/a//b",
            "/./a",
            "/../a",
            "/a/..",
            "/a/.",
            "/live%2Fsrc",
            "/live/source?x=1",
            "/live/source#frag",
            "/live\\source",
            "/liv e",
            "/live/\u{0}source",
            "/live/sour\u{7f}ce",
            "/live/日本",
            "/live/@user",
            "/live/source\"",
            "/live/'q'",
        ] {
            let url = format!("rtmp://10.0.0.5:19350{bad_path}");
            assert!(canonical(&url).is_err(), "{url}");
        }
    }

    #[test]
    fn rf_src_rtmp_02_wire_endpoint_to_canonical_bridge() {
        // legacy loopback fixture shape still canonicalizes
        let legacy = endpoint("127.0.0.1", "/live/source");
        let c = legacy.to_canonical().unwrap();
        assert_eq!(c.to_url(), "rtmp://127.0.0.1:1935/live/source");
        // bare ipv6 wire host bridges to the bracketed URL form
        let v6 = NetworkEndpoint {
            protocol: NetworkProtocol::Rtmp,
            host: "::1".into(),
            port: 1935,
            path: "/live/source".into(),
        };
        assert_eq!(
            v6.to_canonical().unwrap().to_url(),
            "rtmp://[::1]:1935/live/source"
        );
        let v6b = NetworkEndpoint {
            host: "[fd00::10]".into(),
            port: 19350,
            ..v6.clone()
        };
        assert!(v6b.to_canonical().is_ok());
        // hostname and public address are rejected
        assert!(endpoint("localhost", "/live/source")
            .to_canonical()
            .is_err());
        assert!(endpoint("8.8.8.8", "/live/source").to_canonical().is_err());
        // non-canonical path on the wire value is rejected
        let bad_path = NetworkEndpoint {
            path: "/live/source?x=1".into(),
            ..legacy.clone()
        };
        assert!(bad_path.to_canonical().is_err());
        let privileged = NetworkEndpoint {
            port: 1023,
            ..legacy.clone()
        };
        assert!(privileged.to_canonical().is_err());
    }

    #[test]
    fn rf_src_rtmp_02_listener_key_ignores_path() {
        let a = canonical("rtmp://10.0.0.5:19350/live/source").unwrap();
        let b = canonical("rtmp://10.0.0.5:19350/other/path").unwrap();
        assert_ne!(a, b);
        assert_eq!(a.listener_key(), b.listener_key());
    }

    #[test]
    fn rf_src_rtmp_02_canonical_debug_is_redacted() {
        let c = canonical("rtmp://10.30.15.10:19350/live/probe").unwrap();
        let dbg = format!("{c:?}");
        assert!(!dbg.contains("10.30.15.10"));
        assert!(!dbg.contains("19350"));
        assert!(!dbg.contains("/live/probe"));
        let ip_dbg = format!("{:?}", c.ip());
        let path_dbg = format!("{:?}", c.path());
        let key_dbg = format!("{:?}", c.listener_key());
        for text in [ip_dbg, path_dbg, key_dbg] {
            assert!(text.contains("redacted"), "{text}");
        }
    }
}
