//! Fail-closed kill switch using nftables (via the `nft` CLI).
//!
//! When enabled, an `inet` table with a drop-by-default output chain permits
//! only loopback, established/related, the active tunnel interface, and the
//! WireGuard handshake to the pinned endpoint. With no active tunnel only
//! loopback and established traffic are allowed, so nothing leaks while
//! disconnected. Requires CAP_NET_ADMIN.

use std::io::Write;
use std::process::{Command, Stdio};

const TABLE: &str = "wirearch_ks";

fn run_nft(script: &str) -> std::io::Result<()> {
    let mut child = Command::new("nft")
        .arg("-f")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("nft: no stdin"))?;
        stdin.write_all(script.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "nft failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// Remove the kill-switch table (no-op if it does not exist).
pub fn disable() -> std::io::Result<()> {
    // `add` then `delete` so this succeeds whether or not the table exists.
    run_nft(&format!("add table inet {TABLE}\ndelete table inet {TABLE}\n"))
}

/// Install fail-closed egress rules. If `wg_ifname` / `endpoint` are given,
/// the active tunnel interface and its handshake are also permitted.
pub fn enable(wg_ifname: Option<&str>, endpoint: Option<&str>) -> std::io::Result<()> {
    let mut chain = String::new();
    chain.push_str("    type filter hook output priority 0; policy drop;\n");
    chain.push_str("    oifname \"lo\" accept\n");
    chain.push_str("    ct state established,related accept\n");
    if let Some(ifname) = wg_ifname {
        chain.push_str(&format!("    oifname \"{ifname}\" accept\n"));
    }
    if let Some((ip, port)) = endpoint.and_then(split_endpoint) {
        if ip.contains(':') {
            chain.push_str(&format!("    ip6 daddr {ip} udp dport {port} accept\n"));
        } else {
            chain.push_str(&format!("    ip daddr {ip} udp dport {port} accept\n"));
        }
    }

    // Atomic replace: ensure the table exists, drop it, recreate with rules.
    let script = format!(
        "add table inet {TABLE}\n\
         delete table inet {TABLE}\n\
         add table inet {TABLE} {{\n  chain output {{\n{chain}  }}\n}}\n"
    );
    run_nft(&script)
}

/// Split "IP:port" or "[v6]:port" into (ip, port).
fn split_endpoint(ep: &str) -> Option<(String, String)> {
    if let Some(rest) = ep.strip_prefix('[') {
        let mut parts = rest.splitn(2, ']');
        let ip = parts.next()?.to_string();
        let port = parts.next()?.trim_start_matches(':').to_string();
        if ip.is_empty() || port.is_empty() {
            return None;
        }
        Some((ip, port))
    } else {
        let (ip, port) = ep.rsplit_once(':')?;
        if ip.is_empty() || port.is_empty() {
            return None;
        }
        Some((ip.to_string(), port.to_string()))
    }
}
