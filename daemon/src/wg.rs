//! Bringing WireGuard tunnels up and down via the in-kernel module
//! (generic netlink, through defguard_wireguard_rs). Requires CAP_NET_ADMIN.

use std::net::{IpAddr, UdpSocket};
use std::time::{Duration, Instant, UNIX_EPOCH};

use defguard_wireguard_rs::error::WireguardInterfaceError;
use defguard_wireguard_rs::key::Key;
use defguard_wireguard_rs::net::IpAddrMask;
use defguard_wireguard_rs::peer::Peer;
use defguard_wireguard_rs::{InterfaceConfiguration, Kernel, WGApi, WireguardInterfaceApi};
use thiserror::Error;

use crate::config::{self, WgConfig};

#[derive(Debug, Error)]
pub enum WgError {
    #[error("a peer is missing its PublicKey")]
    MissingPublicKey,
    #[error("the [Interface] has no PrivateKey")]
    MissingPrivateKey,
    #[error("invalid key: {0}")]
    Key(String),
    #[error("invalid address or allowed-ip: {0}")]
    Addr(String),
    #[error("{0} (the WireArch service needs CAP_NET_ADMIN; run it privileged or via the system service)")]
    Wg(#[from] WireguardInterfaceError),
}

/// Derive a valid WireGuard interface name (max 15 chars) from a tunnel id.
pub fn ifname_for(id: &str) -> String {
    let mut name: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    name.truncate(15);
    if name.is_empty() {
        "wg0".to_string()
    } else {
        name
    }
}

fn parse_masks(values: &[String]) -> Result<Vec<IpAddrMask>, WgError> {
    values
        .iter()
        .map(|s| s.parse::<IpAddrMask>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| WgError::Addr(e.to_string()))
}

fn build_peer(p: &config::Peer) -> Result<Peer, WgError> {
    if p.public_key.is_empty() {
        return Err(WgError::MissingPublicKey);
    }
    let key = Key::try_from(p.public_key.as_str()).map_err(|e| WgError::Key(e.to_string()))?;
    let mut peer = Peer::new(key);
    if let Some(psk) = &p.preshared_key {
        peer.preshared_key =
            Some(Key::try_from(psk.as_str()).map_err(|e| WgError::Key(e.to_string()))?);
    }
    if let Some(endpoint) = &p.endpoint {
        // Resolves a hostname endpoint to a SocketAddr (blocking).
        peer.set_endpoint(endpoint)?;
    }
    peer.persistent_keepalive_interval = p.persistent_keepalive;
    peer.set_allowed_ips(parse_masks(&p.allowed_ips)?);
    Ok(peer)
}

fn build_config(ifname: &str, cfg: &WgConfig) -> Result<InterfaceConfiguration, WgError> {
    let prvkey = cfg
        .interface
        .private_key
        .clone()
        .ok_or(WgError::MissingPrivateKey)?;
    let peers = cfg
        .peers
        .iter()
        .map(build_peer)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(InterfaceConfiguration {
        name: ifname.to_string(),
        prvkey,
        addresses: parse_masks(&cfg.interface.addresses)?,
        port: cfg.interface.listen_port.unwrap_or(0),
        peers,
        mtu: cfg.interface.mtu,
        fwmark: cfg.interface.fwmark,
    })
}

/// Bring `cfg` up on interface `ifname`: create the link, configure the
/// device (keys/peers/addresses/MTU), set up wg-quick-style routing, and DNS.
pub fn up(ifname: &str, cfg: &WgConfig) -> Result<(), WgError> {
    let config = build_config(ifname, cfg)?;
    let mut wgapi = WGApi::<Kernel>::new(ifname.to_string())?;
    wgapi.create_interface()?;
    wgapi.configure_interface(&config)?;
    wgapi.configure_peer_routing(&config.peers)?;

    let dns: Vec<IpAddr> = cfg
        .interface
        .dns
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    if !dns.is_empty() {
        // Best-effort: requires a resolvconf-compatible tool on PATH.
        if let Err(e) = wgapi.configure_dns(&dns, &[]) {
            eprintln!("wirearchd: DNS configuration failed for {ifname}: {e}");
        }
    }
    Ok(())
}

/// Tear down `ifname`: removes routes, fwmark rules, DNS, and the link itself.
pub fn down(ifname: &str) -> Result<(), WgError> {
    let wgapi = WGApi::<Kernel>::new(ifname.to_string())?;
    wgapi.remove_interface()?;
    Ok(())
}

/// Live stats read from a running interface (summed across peers).
#[derive(Debug, Default, Clone)]
pub struct Stats {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    /// Unix seconds of the most recent handshake; 0 means never.
    pub last_handshake: i64,
    pub endpoint: String,
}

/// Read live transfer counters and last-handshake time for `ifname`.
pub fn stats(ifname: &str) -> Result<Stats, WgError> {
    let wgapi = WGApi::<Kernel>::new(ifname.to_string())?;
    let host = wgapi.read_interface_data()?;
    let mut s = Stats::default();
    for peer in host.peers.values() {
        s.rx_bytes += peer.rx_bytes;
        s.tx_bytes += peer.tx_bytes;
        if let Some(t) = peer.last_handshake {
            let secs = t
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if secs > s.last_handshake {
                s.last_handshake = secs;
            }
        }
        if s.endpoint.is_empty() {
            if let Some(ep) = peer.endpoint {
                s.endpoint = ep.to_string();
            }
        }
    }
    Ok(s)
}

/// Build the set of probe destinations for a config: the configured DNS
/// servers, an address from each peer's allowed-ips (so the probe routes into
/// the tunnel for split-tunnel configs too; the destination need not be real,
/// any packet to an allowed-ip makes WireGuard attempt a handshake), and a
/// public resolver to cover full-tunnel (0.0.0.0/0) configs.
pub fn probe_targets(cfg: &WgConfig) -> Vec<String> {
    let mut targets: Vec<String> = Vec::new();
    for d in &cfg.interface.dns {
        if d.parse::<IpAddr>().is_ok() {
            targets.push(format!("{d}:53"));
        }
    }
    for peer in &cfg.peers {
        for allowed in &peer.allowed_ips {
            if let Some((ip, _)) = allowed.split_once('/') {
                if let Ok(addr) = ip.parse::<IpAddr>() {
                    if !addr.is_unspecified() {
                        targets.push(socket_target(addr));
                    }
                }
            }
        }
    }
    targets.push("1.1.1.1:53".to_string());
    targets.sort();
    targets.dedup();
    targets
}

fn socket_target(addr: IpAddr) -> String {
    match addr {
        IpAddr::V6(a) => format!("[{a}]:53"),
        IpAddr::V4(a) => format!("{a}:53"),
    }
}

/// Best-effort: send small datagrams to `targets` to coax WireGuard into
/// starting a handshake (it only handshakes when there is traffic or keepalive).
pub fn send_probe(targets: &[String]) {
    let v4 = UdpSocket::bind("0.0.0.0:0").ok();
    let v6 = UdpSocket::bind("[::]:0").ok();
    for sock in [&v4, &v6].into_iter().flatten() {
        let _ = sock.set_write_timeout(Some(Duration::from_millis(300)));
    }
    for target in targets {
        let sock = if target.starts_with('[') { v6.as_ref() } else { v4.as_ref() };
        if let Some(sock) = sock {
            let _ = sock.send_to(&[0u8], target.as_str());
        }
    }
}

/// Convenience: probe straight from a config (used during connect).
pub fn probe(cfg: &WgConfig) {
    send_probe(&probe_targets(cfg));
}

/// Poll the interface until a handshake completes or `timeout` elapses.
pub fn wait_for_handshake(ifname: &str, timeout: Duration) -> bool {
    let start = Instant::now();
    loop {
        if let Ok(s) = stats(ifname) {
            if s.last_handshake > 0 {
                return true;
            }
        }
        if start.elapsed() >= timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
