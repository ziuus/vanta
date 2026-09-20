//! Live TCP/TCP6 connection table with per-socket process attribution.
//!
//! Source:  `/proc/net/tcp`, `/proc/net/tcp6`
//!          `/proc/<pid>/fd/*` → `socket:[inode]` symlinks
//!
//! No root is required: processes owned by the calling user are attributed;
//! root-owned sockets surface with `pid = null`.  This mirrors what `ss -tp`
//! and `netstat -tp` do without elevated privileges.
//!
//! # Approach
//!
//! 1. Build an `inode → PID` map by scanning every `/proc/<pid>/fd/*` symlink
//!    that resolves to `socket:[<inode>]`.
//! 2. Parse each row of `/proc/net/tcp` and `/proc/net/tcp6`.
//! 3. Join on inode to attach a PID; look up the process name from
//!    `/proc/<pid>/comm` (or fall back to `/proc/<pid>/stat` field 1).
//!
//! The scan is intentionally best-effort: PIDs that exit mid-scan are silently
//! skipped, matching the semantics of `ss`.

use std::collections::HashMap;
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr};

// ── Public types ──────────────────────────────────────────────────────────────

/// Connection state as reported by the kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnState {
    Established,
    SynSent,
    SynRecv,
    FinWait1,
    FinWait2,
    TimeWait,
    Close,
    CloseWait,
    LastAck,
    Listen,
    Closing,
    Unknown(u8),
}

impl ConnState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Established => "ESTABLISHED",
            Self::SynSent => "SYN_SENT",
            Self::SynRecv => "SYN_RECV",
            Self::FinWait1 => "FIN_WAIT1",
            Self::FinWait2 => "FIN_WAIT2",
            Self::TimeWait => "TIME_WAIT",
            Self::Close => "CLOSE",
            Self::CloseWait => "CLOSE_WAIT",
            Self::LastAck => "LAST_ACK",
            Self::Listen => "LISTEN",
            Self::Closing => "CLOSING",
            Self::Unknown(_) => "UNKNOWN",
        }
    }
}

impl From<u8> for ConnState {
    fn from(v: u8) -> Self {
        match v {
            0x01 => Self::Established,
            0x02 => Self::SynSent,
            0x03 => Self::SynRecv,
            0x04 => Self::FinWait1,
            0x05 => Self::FinWait2,
            0x06 => Self::TimeWait,
            0x07 => Self::Close,
            0x08 => Self::CloseWait,
            0x09 => Self::LastAck,
            0x0A => Self::Listen,
            0x0B => Self::Closing,
            other => Self::Unknown(other),
        }
    }
}

/// A single TCP or TCP6 socket entry.
#[derive(Debug, Clone)]
pub struct Connection {
    /// `"tcp"` or `"tcp6"`.
    pub protocol: &'static str,
    /// `"host:port"` (IPv4) or `"[addr]:port"` (IPv6).
    pub local_addr: String,
    pub remote_addr: String,
    pub state: ConnState,
    /// PID of the owning process, if attributable.
    pub pid: Option<u32>,
    /// Process name (`/proc/<pid>/comm`), if attributable.
    pub process_name: Option<String>,
}

use std::sync::{LazyLock, Mutex};

// ── Background cache ──────────────────────────────────────────────────────────

/// Snapshot populated by the background sampler thread. Reading this from
/// `host_api.rs` is O(clone) rather than O(fd-walk), so it fits inside the
/// 10 ms render-widget timeout.
static CACHE: LazyLock<Mutex<Vec<Connection>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Refresh the connection cache. Called by `monitors::sample_all` on every
/// sampler tick (typically every 400–1000 ms).
pub fn sample() {
    let inode_to_pid = build_inode_map();
    let mut out = Vec::new();
    parse_into("/proc/net/tcp",  "tcp",  false, &inode_to_pid, &mut out);
    parse_into("/proc/net/tcp6", "tcp6", true,  &inode_to_pid, &mut out);
    if let Ok(mut g) = CACHE.lock() {
        *g = out;
    }
}

/// Return a snapshot of the last connection table read by the background
/// sampler.  This is cheap (a Vec clone) and safe to call from the host
/// function context.
///
/// Falls back to a live scan only if the cache has never been populated
/// (first call before the first sampler tick).
pub fn snapshot() -> Vec<Connection> {
    // Prefer the background cache — hot path.
    if let Ok(g) = CACHE.lock() {
        if !g.is_empty() {
            return g.clone();
        }
    }
    // Cold start: sampler hasn't ticked yet. Do a live scan once.
    let inode_to_pid = build_inode_map();
    let mut out = Vec::new();
    parse_into("/proc/net/tcp",  "tcp",  false, &inode_to_pid, &mut out);
    parse_into("/proc/net/tcp6", "tcp6", true,  &inode_to_pid, &mut out);
    out
}

// ── Inode → PID map ──────────────────────────────────────────────────────────

/// Walk `/proc/<pid>/fd/*` for every PID accessible to us and build a map
/// `socket_inode → pid`.  PIDs that exit during the walk are silently skipped.
fn build_inode_map() -> HashMap<u64, u32> {
    let mut map = HashMap::new();
    let Ok(proc_dir) = fs::read_dir("/proc") else {
        return map;
    };
    for entry in proc_dir.flatten() {
        let fname = entry.file_name();
        let name = fname.to_string_lossy();
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        let fd_dir = format!("/proc/{pid}/fd");
        let Ok(fds) = fs::read_dir(&fd_dir) else {
            continue; // no permission or process exited
        };
        for fd in fds.flatten() {
            let Ok(target) = fs::read_link(fd.path()) else {
                continue;
            };
            let t = target.to_string_lossy();
            // Symlink target looks like `socket:[12345678]`
            if let Some(inode_str) = t.strip_prefix("socket:[").and_then(|s| s.strip_suffix(']'))
            {
                if let Ok(inode) = inode_str.parse::<u64>() {
                    map.insert(inode, pid);
                }
            }
        }
    }
    map
}

// ── /proc/net/tcp parser ──────────────────────────────────────────────────────

/// Parse one of `/proc/net/tcp` or `/proc/net/tcp6` into `out`.
fn parse_into(
    path: &str,
    proto: &'static str,
    v6: bool,
    inode_map: &HashMap<u64, u32>,
    out: &mut Vec<Connection>,
) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    for line in text.lines().skip(1) {
        if let Some(conn) = parse_line(line, proto, v6, inode_map) {
            out.push(conn);
        }
    }
}

/// Parse a single data row from `/proc/net/tcp{,6}`.
///
/// Column layout (whitespace-delimited):
/// ```text
/// sl  local_addr rem_addr state tx:rx  tr:tm uid timeout inode ...
/// 0   1          2        3     4      5  6   7   8      9
/// ```
fn parse_line(
    line: &str,
    proto: &'static str,
    v6: bool,
    inode_map: &HashMap<u64, u32>,
) -> Option<Connection> {
    let cols: Vec<&str> = line.split_whitespace().collect();
    if cols.len() < 10 {
        return None;
    }
    let local_addr = parse_addr(cols[1], v6)?;
    let remote_addr = parse_addr(cols[2], v6)?;
    let state_hex = u8::from_str_radix(cols[3], 16).ok()?;
    let state = ConnState::from(state_hex);
    let inode: u64 = cols[9].parse().ok()?;

    let pid = inode_map.get(&inode).copied();
    let process_name = pid.and_then(proc_name);

    Some(Connection {
        protocol: proto,
        local_addr,
        remote_addr,
        state,
        pid,
        process_name,
    })
}

// ── Address helpers ───────────────────────────────────────────────────────────

/// Convert a kernel hex address `HEXIP:HEXPORT` into a human-readable string.
///
/// IPv4: kernel stores the 32-bit address in little-endian hex on x86/arm.
/// IPv6: kernel stores four 32-bit little-endian words (128 bits total).
fn parse_addr(raw: &str, v6: bool) -> Option<String> {
    let (hex_ip, hex_port) = raw.split_once(':')?;
    let port = u16::from_str_radix(hex_port, 16).ok()?;

    if v6 {
        // 32 hex chars = 16 bytes
        if hex_ip.len() != 32 {
            return None;
        }
        let mut words = [0u32; 4];
        for (i, w) in words.iter_mut().enumerate() {
            *w = u32::from_str_radix(&hex_ip[i * 8..i * 8 + 8], 16).ok()?;
        }
        // Each word is little-endian; swap bytes to get network order
        let mut bytes = [0u8; 16];
        for (i, w) in words.iter().enumerate() {
            let be = w.swap_bytes().to_be_bytes();
            bytes[i * 4..i * 4 + 4].copy_from_slice(&be);
        }
        let addr = Ipv6Addr::from(bytes);
        // Simplify loopback / mapped IPv4
        if let Some(v4) = addr.to_ipv4_mapped() {
            Some(format!("{v4}:{port}"))
        } else if addr.is_loopback() {
            Some(format!("[::1]:{port}"))
        } else {
            Some(format!("[{addr}]:{port}"))
        }
    } else {
        // 8 hex chars = 4 bytes, little-endian
        if hex_ip.len() != 8 {
            return None;
        }
        let raw_u32 = u32::from_str_radix(hex_ip, 16).ok()?;
        let addr = Ipv4Addr::from(raw_u32.swap_bytes());
        Some(format!("{addr}:{port}"))
    }
}

// ── Process name lookup ───────────────────────────────────────────────────────

/// Read `/proc/<pid>/comm` (15-char process name).  Falls back silently.
fn proc_name(pid: u32) -> Option<String> {
    let raw = fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    let name = raw.trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // The IPv4 address `0100007F` is 127.0.0.1 in kernel little-endian hex.
    #[test]
    fn ipv4_loopback_decodes() {
        let addr = parse_addr("0100007F:1F90", false).expect("parses");
        assert_eq!(addr, "127.0.0.1:8080");
    }

    // `00000000` → 0.0.0.0 (wildcard LISTEN)
    #[test]
    fn ipv4_wildcard_decodes() {
        let addr = parse_addr("00000000:0035", false).expect("parses");
        assert_eq!(addr, "0.0.0.0:53");
    }

    #[test]
    fn ipv6_loopback_decodes() {
        // Kernel stores ::1 as `00000000000000000000000001000000`
        let addr = parse_addr("00000000000000000000000001000000:0050", true).expect("parses");
        // Should simplify to 127.0.0.1:80 (IPv4-mapped) or [::1]:80
        assert!(addr.ends_with(":80"), "got {addr}");
    }

    #[test]
    fn invalid_addr_returns_none() {
        assert!(parse_addr("ZZZZZZZZ:0050", false).is_none());
        assert!(parse_addr("bad", false).is_none());
    }

    #[test]
    fn snapshot_does_not_panic_on_live_system() {
        // Just verify it runs without panicking and returns parseable data.
        let conns = snapshot();
        // There should always be at least one socket open (loopback, etc.)
        // But don't assert a minimum — CI containers may have none visible.
        for c in &conns {
            assert!(
                c.protocol == "tcp" || c.protocol == "tcp6",
                "unexpected protocol"
            );
            assert!(
                !c.local_addr.is_empty(),
                "local_addr should not be empty for {:?}",
                c
            );
        }
    }

    #[test]
    fn state_codes_round_trip() {
        assert_eq!(ConnState::from(0x01).as_str(), "ESTABLISHED");
        assert_eq!(ConnState::from(0x0A).as_str(), "LISTEN");
        assert_eq!(ConnState::from(0x06).as_str(), "TIME_WAIT");
        assert_eq!(ConnState::from(0xFF).as_str(), "UNKNOWN");
    }

    #[test]
    fn short_line_returns_none() {
        assert!(parse_line("  0: 00000000:0035", "tcp", false, &HashMap::new()).is_none());
    }
}
