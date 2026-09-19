//! RF-SRC-RTMP-02 TG-1 (plan D3 + INV-3): `NetworkSourceBinding` startup manifest.
//!
//! The manifest is the server-side production authorization source for RTMP
//! network sources. It is loaded exactly once at process startup:
//!
//! * race-free on a single file descriptor: `open(O_NOFOLLOW)` → `fstat` →
//!   verify regular file / owner = service user / exact mode `0600` / size
//!   ≤ 1 MiB → `read` → strict parse → `fstat` again to prove the file state
//!   did not change during the load;
//! * strict JSON: unknown fields, duplicate fields, trailing data, unknown
//!   schema version, empty file and empty `entries` all reject;
//! * `source_id` values must be canonical UUID spellings; endpoints must be
//!   canonical `CanonicalRtmpEndpoint` URLs; `source_id ↔ endpoint` is
//!   bidirectionally 1:1 and one `(protocol, ip, port)` may appear once
//!   regardless of path;
//! * plan D5: after strict parsing, every entry IP must be currently owned
//!   by a local network interface (enumerated once via `getifaddrs` at
//!   startup); an eligible-but-not-local address fails the ENTIRE load —
//!   private/ULA classification alone never substitutes for ownership;
//! * `machine_id` is pinned to this host via the existing DeviceBindingManifest
//!   identity mechanism (`resolver::current_machine_id`); a mismatch or an
//!   unresolvable runtime identity rejects fail-closed.
//!
//! There is deliberately NO reload, watch or file-follow API: a manifest,
//! address or NIC change takes effect only after a service restart. The
//! manifest never generates `SourceIntent`s (plan D11) and never authenticates
//! publishers: the RTMP path is an exact admission label only — TG-0 proved
//! the BMD FFmpeg listener accepts any app/play path, so nothing here may ever
//! be described as publisher authentication (plan D4).
//!
//! INV-3 owner note: the owner/permission checks are evaluated against the
//! current effective uid. This runtime runs directly as the service user; if a
//! privileged-loader start sequence is ever introduced, manifest loading must
//! be ordered after the privilege drop (or re-verified against the final
//! service uid) before these checks may pass.
#![allow(dead_code)]

use crate::resolver::current_machine_id;
use crate::source::{CanonicalRtmpEndpoint, NetworkEndpoint, NetworkSourceId};
use serde::Deserialize;
use std::fmt;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use uuid::Uuid;

/// INV-3: manifest files larger than 1 MiB are rejected outright.
pub const NETWORK_BINDING_MANIFEST_MAX_BYTES: u64 = 1024 * 1024;

/// Exact required file mode (plan D3).
const REQUIRED_MODE: u32 = 0o600;

/// Current `NetworkSourceBinding` schema version.
const MANIFEST_VERSION: u32 = 1;

/// Redaction-safe typed errors: no variant ever carries endpoint, host, port,
/// path or file-path content (plan D9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkBindingError {
    OpenRejected,
    SymlinkRejected,
    NotRegularFile,
    ModeNotExact0600,
    OwnerNotServiceUser,
    EmptyFile,
    TooLarge,
    ReadFailed,
    ModifiedDuringLoad,
    ParseFailed,
    UnknownVersion,
    MachineIdUnresolved,
    MachineIdMismatch,
    EmptyEntries,
    DuplicateSourceId,
    DuplicateEndpoint,
    ListenerConflict,
    InvalidSourceId,
    InvalidEndpoint,
    AddressEnumerationFailed,
    AddressNotLocal,
    UnauthorizedSource,
    EndpointMismatch,
}

impl fmt::Display for NetworkBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::OpenRejected => "network binding manifest open rejected",
            Self::SymlinkRejected => "network binding manifest must not be a symlink",
            Self::NotRegularFile => "network binding manifest must be a regular file",
            Self::ModeNotExact0600 => "network binding manifest mode must be exactly 0600",
            Self::OwnerNotServiceUser => "network binding manifest owner must be the service user",
            Self::EmptyFile => "network binding manifest is empty",
            Self::TooLarge => "network binding manifest exceeds 1 MiB",
            Self::ReadFailed => "network binding manifest read failed",
            Self::ModifiedDuringLoad => {
                "network binding manifest changed during load; restart-level reload required"
            }
            Self::ParseFailed => {
                "network binding manifest JSON rejected (schema/unknown/duplicate/trailing data)"
            }
            Self::UnknownVersion => "network binding manifest version is unknown",
            Self::MachineIdUnresolved => {
                "machine identity unresolved (VBMF_MACHINE_ID/HOSTNAME); cannot pin manifest"
            }
            Self::MachineIdMismatch => "network binding manifest machine_id mismatch",
            Self::EmptyEntries => "network binding manifest has no entries",
            Self::DuplicateSourceId => "network binding manifest repeats a source_id",
            Self::DuplicateEndpoint => "network binding manifest repeats a complete endpoint",
            Self::ListenerConflict => {
                "network binding manifest maps one ip:port to more than one path"
            }
            Self::InvalidSourceId => "network binding manifest entry source_id is invalid",
            Self::InvalidEndpoint => "network binding manifest entry endpoint is invalid",
            Self::AddressEnumerationFailed => {
                "local interface address enumeration failed; cannot verify ownership"
            }
            Self::AddressNotLocal => {
                "network binding manifest endpoint address is not owned by a local interface"
            }
            Self::UnauthorizedSource => "source_id is not authorized by the network binding",
            Self::EndpointMismatch => "endpoint does not exactly match the authorized binding",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for NetworkBindingError {}

/// fstat identity snapshot used to prove the file did not change across the
/// single-descriptor load (INV-3 race guard). Beyond `dev/ino/size` this
/// snapshots nanosecond-resolution `mtime`/`ctime` and the security-relevant
/// `uid`/`mode`, so an in-load rewrite, truncate, chmod or chown — even within
/// the same whole second, even if size is restored — is detected fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    dev: u64,
    ino: u64,
    size: u64,
    mtime: i64,
    mtime_nsec: i64,
    ctime: i64,
    ctime_nsec: i64,
    uid: u32,
    mode: u32,
}

impl FileIdentity {
    fn of(meta: &std::fs::Metadata) -> Self {
        Self {
            dev: meta.dev(),
            ino: meta.ino(),
            size: meta.size(),
            mtime: meta.mtime(),
            mtime_nsec: meta.mtime_nsec(),
            ctime: meta.ctime(),
            ctime_nsec: meta.ctime_nsec(),
            uid: meta.uid(),
            // Full st_mode (type bits included); compared verbatim.
            mode: std::os::unix::fs::PermissionsExt::mode(&meta.permissions()),
        }
    }
}

/// INV-3 load-state guard: the before/after snapshots on the SAME descriptor
/// must be identical and the byte count actually read must equal the file
/// size. Pure function so the transition logic is deterministically testable.
fn load_state_unchanged(before: &FileIdentity, after: &FileIdentity, bytes_read: usize) -> bool {
    before == after && after.size == bytes_read as u64
}

/// INV-3 owner rule: the file must belong to the uid the service actually runs
/// under. Pure helper so the rule stays independently testable.
fn owner_is_service_user(file_uid: u32, effective_uid: u32) -> bool {
    file_uid == effective_uid
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestFile {
    version: u32,
    machine_id: String,
    entries: Vec<ManifestEntryFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestEntryFile {
    source_id: String,
    endpoint: String,
}

/// INV-3 canonical UUID rule: the string must be exactly the canonical
/// hyphenated lowercase spelling of the parsed UUID.
fn parse_canonical_uuid(text: &str) -> Result<Uuid, NetworkBindingError> {
    let parsed = Uuid::parse_str(text).map_err(|_| NetworkBindingError::InvalidSourceId)?;
    if parsed.to_string() != text {
        return Err(NetworkBindingError::InvalidSourceId);
    }
    Ok(parsed)
}

struct BindingEntry {
    source_id: NetworkSourceId,
    endpoint: CanonicalRtmpEndpoint,
}

/// Loaded, validated network source authorization. Constructed only via
/// [`NetworkSourceBinding::load`] at process startup; there is no reload API.
pub struct NetworkSourceBinding {
    entries: Vec<BindingEntry>,
}

impl NetworkSourceBinding {
    /// Production startup-only load: machine identity from the existing
    /// resolver plus a one-shot `getifaddrs` snapshot of the addresses
    /// currently owned by local interfaces (plan D5). Both are captured once
    /// here — there is no watch, no re-enumeration, no reload; an interface
    /// or manifest change requires a service restart.
    pub fn load(path: &Path) -> Result<Self, NetworkBindingError> {
        let runtime_machine_id = current_machine_id();
        let local_ips = local_interface_addresses()?;
        Self::load_verified(path, &runtime_machine_id, &local_ips)
    }

    /// Fully determined load with caller-supplied machine id and local
    /// address set. Internal seam so tests inject the interface snapshot
    /// instead of depending on the Development VM's live NICs; production
    /// code must use [`NetworkSourceBinding::load`].
    pub fn load_verified(
        path: &Path,
        runtime_machine_id: &str,
        local_ips: &[std::net::IpAddr],
    ) -> Result<Self, NetworkBindingError> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|err| {
                if err.raw_os_error() == Some(libc::ELOOP) {
                    NetworkBindingError::SymlinkRejected
                } else {
                    NetworkBindingError::OpenRejected
                }
            })?;
        let meta = file
            .metadata()
            .map_err(|_| NetworkBindingError::OpenRejected)?;
        if !meta.is_file() {
            return Err(NetworkBindingError::NotRegularFile);
        }
        if meta.permissions().mode() & 0o7777 != REQUIRED_MODE {
            return Err(NetworkBindingError::ModeNotExact0600);
        }
        if !owner_is_service_user(meta.uid(), effective_uid()) {
            return Err(NetworkBindingError::OwnerNotServiceUser);
        }
        let before = FileIdentity::of(&meta);
        if before.size == 0 {
            return Err(NetworkBindingError::EmptyFile);
        }
        if before.size > NETWORK_BINDING_MANIFEST_MAX_BYTES {
            return Err(NetworkBindingError::TooLarge);
        }

        let mut buf = Vec::with_capacity(before.size as usize);
        (&file)
            .take(NETWORK_BINDING_MANIFEST_MAX_BYTES + 1)
            .read_to_end(&mut buf)
            .map_err(|_| NetworkBindingError::ReadFailed)?;
        if buf.len() as u64 > NETWORK_BINDING_MANIFEST_MAX_BYTES {
            return Err(NetworkBindingError::TooLarge);
        }

        // Second fstat on the SAME descriptor: every security-relevant field
        // (owner, mode, timestamps at nanosecond resolution, size, identity)
        // must be unchanged and must match exactly the bytes just parsed.
        let after = file
            .metadata()
            .map(|meta| FileIdentity::of(&meta))
            .map_err(|_| NetworkBindingError::ReadFailed)?;
        if !load_state_unchanged(&before, &after, buf.len()) {
            return Err(NetworkBindingError::ModifiedDuringLoad);
        }

        Self::from_bytes(&buf, runtime_machine_id, local_ips)
    }

    fn from_bytes(
        buf: &[u8],
        runtime_machine_id: &str,
        local_ips: &[std::net::IpAddr],
    ) -> Result<Self, NetworkBindingError> {
        // serde_json rejects trailing non-whitespace data; the derived
        // structs reject duplicate fields and (deny_unknown_fields) unknown
        // fields.
        let parsed: ManifestFile =
            serde_json::from_slice(buf).map_err(|_| NetworkBindingError::ParseFailed)?;
        if parsed.version != MANIFEST_VERSION {
            return Err(NetworkBindingError::UnknownVersion);
        }
        if runtime_machine_id.is_empty() {
            return Err(NetworkBindingError::MachineIdUnresolved);
        }
        if parsed.machine_id.is_empty() || parsed.machine_id != runtime_machine_id {
            return Err(NetworkBindingError::MachineIdMismatch);
        }
        if parsed.entries.is_empty() {
            return Err(NetworkBindingError::EmptyEntries);
        }

        let mut entries: Vec<BindingEntry> = Vec::with_capacity(parsed.entries.len());
        let mut seen_source_ids = std::collections::HashSet::new();
        let mut seen_endpoints = std::collections::HashSet::new();
        let mut seen_listener_keys = std::collections::HashMap::new();
        for raw in &parsed.entries {
            let uuid = parse_canonical_uuid(&raw.source_id)?;
            let endpoint = CanonicalRtmpEndpoint::parse_strict(&raw.endpoint)
                .map_err(|_| NetworkBindingError::InvalidEndpoint)?;
            if !seen_source_ids.insert(uuid) {
                return Err(NetworkBindingError::DuplicateSourceId);
            }
            if !seen_endpoints.insert(endpoint.clone()) {
                return Err(NetworkBindingError::DuplicateEndpoint);
            }
            let listener = endpoint.listener_key();
            match seen_listener_keys.get(&listener) {
                Some(existing_path) if *existing_path != endpoint.path().as_str() => {
                    return Err(NetworkBindingError::ListenerConflict);
                }
                Some(_) => return Err(NetworkBindingError::DuplicateEndpoint),
                None => {
                    seen_listener_keys.insert(listener, endpoint.path().as_str().to_string());
                }
            }
            // Plan D5 (last per-entry check, after all structural rejections):
            // the exact endpoint IP must be owned by a local interface in the
            // startup snapshot. One not-local address fails the ENTIRE load —
            // private/ULA class eligibility never substitutes for ownership.
            if !local_ips.contains(&endpoint.ip().as_ip_addr()) {
                return Err(NetworkBindingError::AddressNotLocal);
            }
            entries.push(BindingEntry {
                source_id: NetworkSourceId(uuid),
                endpoint,
            });
        }
        Ok(Self { entries })
    }

    /// Exact five-tuple admission (plan D11):
    /// `(source_id, protocol, canonical_ip, port, exact_path)`. The wire
    /// endpoint is canonicalized first; any mismatch rejects fail-closed.
    pub fn authorize(
        &self,
        source_id: &NetworkSourceId,
        endpoint: &NetworkEndpoint,
    ) -> Result<CanonicalRtmpEndpoint, NetworkBindingError> {
        let canonical = endpoint
            .to_canonical()
            .map_err(|_| NetworkBindingError::InvalidEndpoint)?;
        let entry = self
            .entries
            .iter()
            .find(|entry| &entry.source_id == source_id)
            .ok_or(NetworkBindingError::UnauthorizedSource)?;
        if entry.endpoint == canonical {
            Ok(canonical)
        } else {
            Err(NetworkBindingError::EndpointMismatch)
        }
    }

    /// Redaction-safe projection: authorized source ids only, never
    /// endpoints (plan D9).
    pub fn authorized_sources(&self) -> Vec<NetworkSourceId> {
        self.entries.iter().map(|entry| entry.source_id).collect()
    }

    /// Number of authorized entries (diagnostics only).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl fmt::Debug for NetworkSourceBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Redacted by design (plan D9): machine id and endpoints stay out.
        f.debug_struct("NetworkSourceBinding")
            .field("entries", &self.entries.len())
            .finish()
    }
}

#[cfg(target_os = "linux")]
fn effective_uid() -> u32 {
    // SAFETY: geteuid is async-signal-safe and cannot fail.
    unsafe { libc::geteuid() }
}

/// Plan D5 production enumerator: one read-only `getifaddrs` snapshot of the
/// unicast addresses currently assigned to local interfaces. No DNS, no
/// watch, no refresh — callers use it exactly once at startup. Only AF_INET
/// and AF_INET6 families are collected; scope ids are ignored (zone ids are
/// rejected by `CanonicalIp` anyway).
#[cfg(target_os = "linux")]
fn local_interface_addresses() -> Result<Vec<std::net::IpAddr>, NetworkBindingError> {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs allocates the list; it is fully walked and freed
    // below on every path after a zero return code.
    if unsafe { libc::getifaddrs(&mut ifap) } != 0 {
        return Err(NetworkBindingError::AddressEnumerationFailed);
    }

    let mut addresses = Vec::new();
    let mut cursor = ifap;
    while !cursor.is_null() {
        // SAFETY: cursor walks the libc-owned linked list returned above.
        let ifa = unsafe { &*cursor };
        let addr = ifa.ifa_addr;
        if !addr.is_null() {
            // SAFETY: the kernel-provided sockaddr is at least the family
            // field; family gates the wider struct interpretations below.
            let family = unsafe { (*addr).sa_family as libc::c_int };
            if family == libc::AF_INET {
                // SAFETY: AF_INET sockaddr per the family check.
                let sa = unsafe { &*(addr as *const libc::sockaddr_in) };
                addresses.push(IpAddr::V4(Ipv4Addr::from(u32::from_be(sa.sin_addr.s_addr))));
            } else if family == libc::AF_INET6 {
                // SAFETY: AF_INET6 sockaddr per the family check.
                let sa = unsafe { &*(addr as *const libc::sockaddr_in6) };
                addresses.push(IpAddr::V6(Ipv6Addr::from(sa.sin6_addr.s6_addr)));
            }
        }
        cursor = ifa.ifa_next;
    }
    // SAFETY: releases the list allocated by getifaddrs.
    unsafe { libc::freeifaddrs(ifap) };
    Ok(addresses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::NetworkProtocol;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    /// Minimal temp-dir helper (no tempfile dev-dependency in this crate).
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "vbmf-tg1-{}-{}-{}",
                tag,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn wire_endpoint(host: &str, port: u16, path: &str) -> NetworkEndpoint {
        NetworkEndpoint {
            protocol: NetworkProtocol::Rtmp,
            host: host.into(),
            port,
            path: path.into(),
        }
    }

    /// Injected D5 interface snapshot — tests never touch the Development
    /// VM's live NICs.
    fn local(ips: &[&str]) -> Vec<std::net::IpAddr> {
        ips.iter().map(|s| s.parse().unwrap()).collect()
    }

    fn write_manifest(dir: &TempDir, name: &str, body: &str, mode: u32) -> PathBuf {
        let path = dir.path(name);
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        path
    }

    fn valid_body(endpoint: &str) -> String {
        format!(
            r#"{{"version":1,"machine_id":"box-a","entries":[{{"source_id":"{UUID}","endpoint":"{endpoint}"}}]}}"#,
            UUID = "11111111-1111-1111-1111-111111111111",
        )
    }

    const SOURCE_ONE_RAW: u128 = 0x11111111_1111_1111_1111_111111111111;

    fn source_one() -> NetworkSourceId {
        NetworkSourceId(Uuid::from_u128(SOURCE_ONE_RAW))
    }

    #[test]
    fn rf_src_rtmp_02_manifest_roundtrip_and_exact_authorization() {
        let dir = TempDir::new("roundtrip");
        let path = write_manifest(
            &dir,
            "ok.json",
            &valid_body("rtmp://10.30.15.10:19350/live/probe"),
            0o600,
        );
        let binding =
            NetworkSourceBinding::load_verified(&path, "box-a", &local(&["10.30.15.10"])).unwrap();
        assert_eq!(binding.len(), 1);
        assert_eq!(binding.authorized_sources(), vec![source_one()]);
        let ok = binding.authorize(
            &source_one(),
            &wire_endpoint("10.30.15.10", 19350, "/live/probe"),
        );
        assert_eq!(ok.unwrap().to_url(), "rtmp://10.30.15.10:19350/live/probe");
        // manifest Debug never carries endpoints or machine id
        let dbg = format!("{binding:?}");
        assert!(!dbg.contains("10.30.15.10"));
        assert!(!dbg.contains("19350"));
        assert!(!dbg.contains("box-a"));
    }

    #[test]
    fn rf_src_rtmp_02_manifest_rejects_mismatched_source_or_endpoint() {
        let dir = TempDir::new("mismatch");
        let path = write_manifest(
            &dir,
            "m.json",
            &valid_body("rtmp://10.30.15.10:19350/live/probe"),
            0o600,
        );
        let binding =
            NetworkSourceBinding::load_verified(&path, "box-a", &local(&["10.30.15.10"])).unwrap();
        let other = NetworkSourceId(Uuid::new_v4());
        assert_eq!(
            binding.authorize(&other, &wire_endpoint("10.30.15.10", 19350, "/live/probe")),
            Err(NetworkBindingError::UnauthorizedSource)
        );
        for wrong in [
            wire_endpoint("10.30.15.10", 19350, "/live/other"),
            wire_endpoint("10.30.15.10", 19351, "/live/probe"),
            wire_endpoint("10.30.15.11", 19350, "/live/probe"),
            wire_endpoint("10.30.15.10", 19350, "/live/probe/"),
        ] {
            assert_eq!(
                binding.authorize(&source_one(), &wrong),
                Err(NetworkBindingError::EndpointMismatch)
            );
        }
        // non-canonical wire endpoint rejects as invalid before any matching
        assert_eq!(
            binding.authorize(
                &source_one(),
                &wire_endpoint("localhost", 19350, "/live/probe")
            ),
            Err(NetworkBindingError::InvalidEndpoint)
        );
    }

    #[test]
    fn rf_src_rtmp_02_manifest_mode_must_be_exactly_0600() {
        let dir = TempDir::new("mode");
        for mode in [0o644u32, 0o400, 0o600, 0o640] {
            let path = write_manifest(
                &dir,
                &format!("m{mode:o}.json"),
                &valid_body("rtmp://10.30.15.10:19350/live/probe"),
                mode,
            );
            let result =
                NetworkSourceBinding::load_verified(&path, "box-a", &local(&["10.30.15.10"]));
            if mode == 0o600 {
                assert!(result.is_ok());
            } else {
                assert_eq!(result.unwrap_err(), NetworkBindingError::ModeNotExact0600);
            }
        }
    }

    #[test]
    fn rf_src_rtmp_02_manifest_rejects_symlink_and_non_regular() {
        let dir = TempDir::new("filetype");
        let real = write_manifest(
            &dir,
            "real.json",
            &valid_body("rtmp://10.30.15.10:19350/live/probe"),
            0o600,
        );
        let link = dir.path("link.json");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert_eq!(
            NetworkSourceBinding::load_verified(&link, "box-a", &[]).unwrap_err(),
            NetworkBindingError::SymlinkRejected
        );
        // a directory is not a regular file
        assert_eq!(
            NetworkSourceBinding::load_verified(&dir.0, "box-a", &[]).unwrap_err(),
            NetworkBindingError::NotRegularFile
        );
    }

    #[test]
    fn rf_src_rtmp_02_manifest_rejects_empty_and_oversize() {
        let dir = TempDir::new("size");
        let empty = write_manifest(&dir, "empty.json", "", 0o600);
        assert_eq!(
            NetworkSourceBinding::load_verified(&empty, "box-a", &[]).unwrap_err(),
            NetworkBindingError::EmptyFile
        );
        let mut big = String::from("{\"version\":1,\"machine_id\":\"box-a\",\"entries\":[");
        big.push_str(&format!(
            r#"{{"source_id":"{}","endpoint":"rtmp://10.0.0.1:19351/a"}}"#,
            "22222222-2222-2222-2222-222222222222"
        ));
        while big.len() <= NETWORK_BINDING_MANIFEST_MAX_BYTES as usize {
            big.push_str(&format!(
                r#",{{"source_id":"{}","endpoint":"rtmp://10.0.0.2:193{}/b"}}"#,
                Uuid::new_v4(),
                big.len() % 10
            ));
        }
        big.push_str("]}");
        let big_path = write_manifest(&dir, "big.json", &big, 0o600);
        assert_eq!(
            NetworkSourceBinding::load_verified(&big_path, "box-a", &[]).unwrap_err(),
            NetworkBindingError::TooLarge
        );
    }

    #[test]
    fn rf_src_rtmp_02_manifest_rejects_strict_json_violations() {
        let dir = TempDir::new("json");
        let base = valid_body("rtmp://10.30.15.10:19350/live/probe");
        // unknown field
        let unknown = base.replace(
            "\"machine_id\":\"box-a\"",
            "\"machine_id\":\"box-a\",\"extra\":1",
        );
        // duplicate field (machine_id twice; serde rejects duplicates)
        let dup_field = format!(
            "{{\"version\":1,\"machine_id\":\"box-a\",\"machine_id\":\"box-a\",\"entries\":[{{\"source_id\":\"{ID}\",\"endpoint\":\"rtmp://10.30.15.10:19350/live/probe\"}}]}}",
            ID = "11111111-1111-1111-1111-111111111111"
        );
        // trailing data after the top-level value
        let trailing = format!("{base}x");
        for (name, body) in [
            ("unknown.json", unknown.as_str()),
            ("dup.json", dup_field.as_str()),
            ("trailing.json", trailing.as_str()),
            ("garbage.json", "not json at all"),
        ] {
            let path = write_manifest(&dir, name, body, 0o600);
            assert_eq!(
                NetworkSourceBinding::load_verified(&path, "box-a", &[]).unwrap_err(),
                NetworkBindingError::ParseFailed,
                "{name}"
            );
        }
    }

    #[test]
    fn rf_src_rtmp_02_manifest_rejects_version_and_machine_pin_violations() {
        let dir = TempDir::new("pin");
        let v2 = valid_body("rtmp://10.30.15.10:19350/live/probe")
            .replace("\"version\":1", "\"version\":2");
        let path = write_manifest(&dir, "v2.json", &v2, 0o600);
        assert_eq!(
            NetworkSourceBinding::load_verified(&path, "box-a", &[]).unwrap_err(),
            NetworkBindingError::UnknownVersion
        );
        let ok_path = write_manifest(
            &dir,
            "ok.json",
            &valid_body("rtmp://10.30.15.10:19350/live/probe"),
            0o600,
        );
        assert_eq!(
            NetworkSourceBinding::load_verified(&ok_path, "box-b", &[]).unwrap_err(),
            NetworkBindingError::MachineIdMismatch
        );
        assert_eq!(
            NetworkSourceBinding::load_verified(&ok_path, "", &[]).unwrap_err(),
            NetworkBindingError::MachineIdUnresolved
        );
    }

    #[test]
    fn rf_src_rtmp_02_manifest_rejects_empty_entries_and_entry_violations() {
        let dir = TempDir::new("entries");
        let empty_entries = r#"{"version":1,"machine_id":"box-a","entries":[]}"#;
        let path = write_manifest(&dir, "empty.json", empty_entries, 0o600);
        assert_eq!(
            NetworkSourceBinding::load_verified(&path, "box-a", &[]).unwrap_err(),
            NetworkBindingError::EmptyEntries
        );

        let dup_source = format!(
            "{{\"version\":1,\"machine_id\":\"box-a\",\"entries\":[{{\"source_id\":\"{ID}\",\"endpoint\":\"rtmp://10.0.0.1:19351/a\"}},{{\"source_id\":\"{ID}\",\"endpoint\":\"rtmp://10.0.0.2:19352/b\"}}]}}",
            ID = "11111111-1111-1111-1111-111111111111"
        );
        let path = write_manifest(&dir, "dupsrc.json", &dup_source, 0o600);
        assert_eq!(
            NetworkSourceBinding::load_verified(&path, "box-a", &local(&["10.0.0.1", "10.0.0.2"]))
                .unwrap_err(),
            NetworkBindingError::DuplicateSourceId
        );

        let dup_endpoint = format!(
            "{{\"version\":1,\"machine_id\":\"box-a\",\"entries\":[{{\"source_id\":\"{A}\",\"endpoint\":\"rtmp://10.0.0.1:19351/a\"}},{{\"source_id\":\"{B}\",\"endpoint\":\"rtmp://10.0.0.1:19351/a\"}}]}}",
            A = "11111111-1111-1111-1111-111111111111",
            B = "22222222-2222-2222-2222-222222222222"
        );
        let path = write_manifest(&dir, "dupend.json", &dup_endpoint, 0o600);
        assert_eq!(
            NetworkSourceBinding::load_verified(&path, "box-a", &local(&["10.0.0.1", "10.0.0.2"]))
                .unwrap_err(),
            NetworkBindingError::DuplicateEndpoint
        );

        // same ip:port with two different paths is a listener conflict
        let conflict = format!(
            "{{\"version\":1,\"machine_id\":\"box-a\",\"entries\":[{{\"source_id\":\"{A}\",\"endpoint\":\"rtmp://10.0.0.1:19351/a\"}},{{\"source_id\":\"{B}\",\"endpoint\":\"rtmp://10.0.0.1:19351/b\"}}]}}",
            A = "11111111-1111-1111-1111-111111111111",
            B = "22222222-2222-2222-2222-222222222222"
        );
        let path = write_manifest(&dir, "conflict.json", &conflict, 0o600);
        assert_eq!(
            NetworkSourceBinding::load_verified(&path, "box-a", &local(&["10.0.0.1", "10.0.0.2"]))
                .unwrap_err(),
            NetworkBindingError::ListenerConflict
        );
    }

    #[test]
    fn rf_src_rtmp_02_manifest_requires_canonical_uuid_and_endpoint() {
        let dir = TempDir::new("canonical");
        for bad_id in [
            "11111111111111111111111111111111",       // hyphenless
            "11111111-1111-1111-1111-111111111111X",  // garbage suffix
            "{11111111-1111-1111-1111-111111111111}", // braced
        ] {
            let body = format!(
                r#"{{"version":1,"machine_id":"box-a","entries":[{{"source_id":"{bad_id}","endpoint":"rtmp://10.0.0.1:19351/a"}}]}}"#
            );
            let path = write_manifest(&dir, "id.json", &body, 0o600);
            assert_eq!(
                NetworkSourceBinding::load_verified(&path, "box-a", &[]).unwrap_err(),
                NetworkBindingError::InvalidSourceId,
                "{bad_id}"
            );
        }
        for bad_endpoint in [
            "rtmp://box-a.local:19351/a", // hostname
            "rtmp://8.8.8.8:19351/a",     // public
            "rtmp://10.0.0.1:1023/a",     // privileged port
            "rtmp://10.0.0.1:19351",      // missing path
            "rtmp://10.0.0.1/a",          // implicit port
        ] {
            let body = format!(
                r#"{{"version":1,"machine_id":"box-a","entries":[{{"source_id":"{}","endpoint":"{bad_endpoint}"}}]}}"#,
                "11111111-1111-1111-1111-111111111111"
            );
            let path = write_manifest(&dir, "end.json", &body, 0o600);
            assert_eq!(
                NetworkSourceBinding::load_verified(&path, "box-a", &[]).unwrap_err(),
                NetworkBindingError::InvalidEndpoint,
                "{bad_endpoint}"
            );
        }
    }

    #[test]
    fn rf_src_rtmp_02_identity_transition_guard_detects_metadata_drift() {
        // INV-3 single-fd race guard, tested as DETERMINISTIC identity
        // transitions on a real file (no concurrent writer is involved —
        // this is a guard unit test, not a concurrency race injection):
        // any drift in size, nanosecond timestamps, owner bits, mode or
        // inode between the two same-fd snapshots rejects the load.
        let dir = TempDir::new("guard");
        let path = write_manifest(
            &dir,
            "ok.json",
            &valid_body("rtmp://10.30.15.10:19350/live/probe"),
            0o600,
        );
        let snapshot = |p: &Path| {
            let meta = std::fs::metadata(p).unwrap();
            FileIdentity::of(&meta)
        };

        let before = snapshot(&path);
        let bytes = before.size as usize;
        assert!(load_state_unchanged(&before, &before, bytes));
        assert!(!load_state_unchanged(&before, &before, bytes - 1));

        // mode drift (chmod during load)
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let drifted = snapshot(&path);
        assert_ne!(before, drifted);
        assert!(!load_state_unchanged(&before, &drifted, bytes));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        // size drift: growth and truncation
        let grown = snapshot_after(&path, |p| {
            let mut f = std::fs::OpenOptions::new().append(true).open(p).unwrap();
            std::io::Write::write_all(&mut f, b"x").unwrap();
        });
        assert!(!load_state_unchanged(&before, &grown, bytes));
        let truncated = snapshot_after(&path, |p| {
            let f = std::fs::OpenOptions::new().write(true).open(p).unwrap();
            f.set_len(bytes as u64 - 1).unwrap();
        });
        assert!(!load_state_unchanged(&before, &truncated, bytes));

        // same-size rewrite drift via explicit mtime change (deterministic on
        // any filesystem; a real same-second rewrite shows up through
        // mtime_nsec/ctime_nsec, which the identity carries)
        let rewritten = snapshot_after(&path, |p| {
            let f = std::fs::OpenOptions::new().write(true).open(p).unwrap();
            f.set_times(std::fs::FileTimes::new().set_modified(
                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1),
            ))
            .unwrap();
        });
        assert!(!load_state_unchanged(&before, &rewritten, bytes));

        // structural pure-field transitions (replacement / cross-device swap)
        let a = FileIdentity {
            dev: 1,
            ino: 2,
            size: 100,
            mtime: 3,
            mtime_nsec: 0,
            ctime: 3,
            ctime_nsec: 0,
            uid: 1000,
            mode: 0o100600,
        };
        for mutated in [
            FileIdentity { size: 99, ..a },      // truncation
            FileIdentity { size: 101, ..a },     // growth
            FileIdentity { mtime: 4, ..a },      // rewrite (seconds)
            FileIdentity { mtime_nsec: 1, ..a }, // same-second rewrite
            FileIdentity { ctime_nsec: 1, ..a }, // metadata change
            FileIdentity { uid: 0, ..a },        // chown mid-load
            FileIdentity {
                mode: 0o100644,
                ..a
            }, // chmod mid-load
            FileIdentity { ino: 9, ..a },        // replacement (rename-over)
            FileIdentity { dev: 7, ..a },        // cross-device swap
        ] {
            assert_ne!(a, mutated);
            assert!(!load_state_unchanged(&a, &mutated, 100));
        }
        // owner rule
        assert!(owner_is_service_user(1000, 1000));
        assert!(!owner_is_service_user(0, 1000));
        assert!(!owner_is_service_user(1000, 0));
    }

    /// Snapshot a real file after deterministically mutating it.
    fn snapshot_after(p: &Path, mutate: impl FnOnce(&Path)) -> FileIdentity {
        mutate(p);
        let meta = std::fs::metadata(p).unwrap();
        FileIdentity::of(&meta)
    }

    #[test]
    fn rf_src_rtmp_02_d5_requires_every_endpoint_ip_on_a_local_interface() {
        // Plan D5 allow/reject matrix with an INJECTED interface snapshot:
        // eligibility (private class) never substitutes for local ownership.
        let dir = TempDir::new("d5");
        let path = write_manifest(
            &dir,
            "ok.json",
            &valid_body("rtmp://10.30.15.10:19350/live/probe"),
            0o600,
        );
        // allow: the exact manifest IP is in the local snapshot
        let ok = NetworkSourceBinding::load_verified(&path, "box-a", &local(&["10.30.15.10"]));
        assert!(ok.is_ok());
        // reject: same eligible private class, but not this host's address
        for snapshot in [
            local(&["10.30.15.11"]),         // neighbour host
            local(&["127.0.0.1"]),           // loopback is local but not the claim
            local(&["fd00::10", "fd00::1"]), // v6-only snapshot cannot satisfy a v4 claim
        ] {
            assert_eq!(
                NetworkSourceBinding::load_verified(&path, "box-a", &snapshot).unwrap_err(),
                NetworkBindingError::AddressNotLocal
            );
        }
        // empty snapshot (no interfaces): fail-closed, not permissive
        assert_eq!(
            NetworkSourceBinding::load_verified(&path, "box-a", &[]).unwrap_err(),
            NetworkBindingError::AddressNotLocal
        );

        // one not-local entry fails the ENTIRE load — no partial acceptance
        let mixed = format!(
            "{{\"version\":1,\"machine_id\":\"box-a\",\"entries\":[{{\"source_id\":\"{A}\",\"endpoint\":\"rtmp://10.30.15.10:19350/live/probe\"}},{{\"source_id\":\"{B}\",\"endpoint\":\"rtmp://10.30.15.99:19350/live/probe\"}}]}}",
            A = "11111111-1111-1111-1111-111111111111",
            B = "22222222-2222-2222-2222-222222222222"
        );
        let mixed_path = write_manifest(&dir, "mixed.json", &mixed, 0o600);
        assert_eq!(
            NetworkSourceBinding::load_verified(&mixed_path, "box-a", &local(&["10.30.15.10"]))
                .unwrap_err(),
            NetworkBindingError::AddressNotLocal
        );
    }

    #[test]
    fn rf_src_rtmp_02_d5_production_enumerator_snapshots_local_interfaces() {
        // The REAL getifaddrs enumerator (the production D5 source): every
        // Linux host — including CI runners — owns 127.0.0.1 on `lo`, so the
        // snapshot must be non-empty and contain loopback. This is the only
        // assertion that does not depend on host-specific addressing.
        let snapshot = local_interface_addresses().unwrap();
        assert!(snapshot.contains(&std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)));
    }

    #[test]
    fn rf_src_rtmp_02_manifest_rejects_noncanonical_port_spellings() {
        // D2 tightening through the manifest path: the explicit port text
        // must be the canonical decimal spelling.
        let dir = TempDir::new("port");
        for bad_endpoint in [
            "rtmp://10.0.0.1:01935/a", // leading zero
            "rtmp://10.0.0.1:+1935/a", // plus sign
            "rtmp://10.0.0.1:-1935/a", // minus sign
            "rtmp://10.0.0.1: 1935/a", // whitespace
            "rtmp://10.0.0.1:1935x/a", // trailing junk
            "rtmp://10.0.0.1:/a",      // empty port
        ] {
            let body = format!(
                r#"{{"version":1,"machine_id":"box-a","entries":[{{"source_id":"{ID}","endpoint":"{bad_endpoint}"}}]}}"#,
                ID = "11111111-1111-1111-1111-111111111111"
            );
            let path = write_manifest(&dir, "p.json", &body, 0o600);
            assert_eq!(
                NetworkSourceBinding::load_verified(&path, "box-a", &local(&["10.0.0.1"]))
                    .unwrap_err(),
                NetworkBindingError::InvalidEndpoint,
                "{bad_endpoint}"
            );
        }
    }

    #[test]
    fn rf_src_rtmp_02_manifest_api_never_constructs_source_intent() {
        // Plan D11: the manifest only answers admission questions. Its whole
        // API surface is load / authorize (-> canonical endpoint) /
        // authorized_sources (-> ids). It cannot produce a SourceIntent, and
        // this test pins that contract: the returned value is the authorized
        // canonical endpoint, nothing else.
        let dir = TempDir::new("nointent");
        let path = write_manifest(
            &dir,
            "ok.json",
            &valid_body("rtmp://10.30.15.10:19350/live/probe"),
            0o600,
        );
        let binding =
            NetworkSourceBinding::load_verified(&path, "box-a", &local(&["10.30.15.10"])).unwrap();
        let decision: Result<CanonicalRtmpEndpoint, NetworkBindingError> = binding.authorize(
            &source_one(),
            &wire_endpoint("10.30.15.10", 19350, "/live/probe"),
        );
        let _: Option<CanonicalRtmpEndpoint> = decision.ok();
        let _: Vec<NetworkSourceId> = binding.authorized_sources();
    }

    #[test]
    fn rf_src_rtmp_02_path_is_exact_admission_label_not_authentication() {
        // Plan D4 + TG-0: the BMD FFmpeg listener accepts any app/play path,
        // so the manifest path is an exact admission label only. Two
        // manifests differing only in path authorize only their exact path.
        let dir = TempDir::new("pathlabel");
        let path = write_manifest(
            &dir,
            "ok.json",
            &valid_body("rtmp://10.30.15.10:19350/live/probe"),
            0o600,
        );
        let binding =
            NetworkSourceBinding::load_verified(&path, "box-a", &local(&["10.30.15.10"])).unwrap();
        assert!(binding
            .authorize(
                &source_one(),
                &wire_endpoint("10.30.15.10", 19350, "/live/probe")
            )
            .is_ok());
        // any other path — including near-misses — is refused admission
        for path_variant in [
            "/live/other",
            "/live/probe/",
            "/live/probe?x=1",
            "/live/pro%62e",
        ] {
            assert!(
                binding
                    .authorize(
                        &source_one(),
                        &wire_endpoint("10.30.15.10", 19350, path_variant)
                    )
                    .is_err(),
                "{path_variant}"
            );
        }
    }
}
