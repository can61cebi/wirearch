//! Parsing and modelling of wg-quick style WireGuard configuration files.
//!
//! The parser is lenient about whitespace, comments (`#` or `;`) and key
//! casing (WireGuard treats keys case-insensitively), while preserving the
//! advanced fields NetworkManager silently drops: `MTU`, `Table`, `FwMark`
//! and the `PreUp`/`PostUp`/`PreDown`/`PostDown` hooks.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("line {line}: content outside of any [Interface]/[Peer] section")]
    OutsideSection { line: usize },
    #[error("line {line}: malformed entry, expected `key = value`")]
    MalformedEntry { line: usize },
    #[error("line {line}: unknown section [{name}]")]
    UnknownSection { line: usize, name: String },
    #[error("line {line}: invalid value for {key}: {value}")]
    InvalidValue {
        line: usize,
        key: String,
        value: String,
    },
    #[error("the [Interface] section is missing")]
    MissingInterface,
    #[error("the [Peer] starting at line {line} has no PublicKey")]
    PeerMissingPublicKey { line: usize },
}

/// The `[Interface]` section of a tunnel.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Interface {
    pub private_key: Option<String>,
    pub addresses: Vec<String>,
    pub dns: Vec<String>,
    pub mtu: Option<u32>,
    pub table: Option<String>,
    pub fwmark: Option<u32>,
    pub listen_port: Option<u16>,
    pub pre_up: Vec<String>,
    pub post_up: Vec<String>,
    pub pre_down: Vec<String>,
    pub post_down: Vec<String>,
}

/// A single `[Peer]` section.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Peer {
    pub public_key: String,
    pub preshared_key: Option<String>,
    pub endpoint: Option<String>,
    pub allowed_ips: Vec<String>,
    pub persistent_keepalive: Option<u16>,
}

impl Peer {
    /// The endpoint host without its port. Handles bracketed IPv6 literals
    /// (`[2001:db8::1]:51820`), `ipv4:port`, and a bare host with no port.
    pub fn endpoint_host(&self) -> Option<&str> {
        let ep = self.endpoint.as_deref()?;
        if let Some(rest) = ep.strip_prefix('[') {
            rest.split(']').next()
        } else {
            Some(ep.rsplit_once(':').map(|(host, _)| host).unwrap_or(ep))
        }
    }
}

/// A fully parsed WireGuard configuration.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WgConfig {
    pub interface: Interface,
    pub peers: Vec<Peer>,
}

impl WgConfig {
    /// Render the configuration back to wg-quick `.conf` text.
    pub fn to_conf_string(&self) -> String {
        let mut s = String::from("[Interface]\n");
        let i = &self.interface;
        if let Some(pk) = &i.private_key {
            s += &format!("PrivateKey = {pk}\n");
        }
        if !i.addresses.is_empty() {
            s += &format!("Address = {}\n", i.addresses.join(", "));
        }
        if !i.dns.is_empty() {
            s += &format!("DNS = {}\n", i.dns.join(", "));
        }
        if let Some(mtu) = i.mtu {
            s += &format!("MTU = {mtu}\n");
        }
        if let Some(port) = i.listen_port {
            s += &format!("ListenPort = {port}\n");
        }
        if let Some(table) = &i.table {
            s += &format!("Table = {table}\n");
        }
        if let Some(fwmark) = i.fwmark {
            s += &format!("FwMark = 0x{fwmark:x}\n");
        }
        for v in &i.pre_up {
            s += &format!("PreUp = {v}\n");
        }
        for v in &i.post_up {
            s += &format!("PostUp = {v}\n");
        }
        for v in &i.pre_down {
            s += &format!("PreDown = {v}\n");
        }
        for v in &i.post_down {
            s += &format!("PostDown = {v}\n");
        }
        for peer in &self.peers {
            s += "\n[Peer]\n";
            s += &format!("PublicKey = {}\n", peer.public_key);
            if let Some(psk) = &peer.preshared_key {
                s += &format!("PresharedKey = {psk}\n");
            }
            if let Some(ep) = &peer.endpoint {
                s += &format!("Endpoint = {ep}\n");
            }
            if !peer.allowed_ips.is_empty() {
                s += &format!("AllowedIPs = {}\n", peer.allowed_ips.join(", "));
            }
            if let Some(ka) = peer.persistent_keepalive {
                s += &format!("PersistentKeepalive = {ka}\n");
            }
        }
        s
    }
}

fn invalid(line: usize, key: &str, value: &str) -> ParseError {
    ParseError::InvalidValue {
        line,
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn strip_comment(line: &str) -> &str {
    let cut = line.find(|c| c == '#' || c == ';').unwrap_or(line.len());
    &line[..cut]
}

fn csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

fn parse_fwmark(value: &str) -> Result<Option<u32>, ()> {
    let v = value.trim();
    if v.eq_ignore_ascii_case("off") {
        return Ok(None);
    }
    let parsed = match v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => v.parse::<u32>(),
    };
    parsed.map(Some).map_err(|_| ())
}

fn apply_interface(
    iface: &mut Interface,
    key_lc: &str,
    key: &str,
    value: &str,
    line: usize,
) -> Result<(), ParseError> {
    match key_lc {
        "privatekey" => iface.private_key = Some(value.to_string()),
        "address" => iface.addresses.extend(csv(value)),
        "dns" => iface.dns.extend(csv(value)),
        "mtu" => iface.mtu = Some(value.parse().map_err(|_| invalid(line, key, value))?),
        "table" => iface.table = Some(value.to_string()),
        "fwmark" => iface.fwmark = parse_fwmark(value).map_err(|_| invalid(line, key, value))?,
        "listenport" => {
            iface.listen_port = Some(value.parse().map_err(|_| invalid(line, key, value))?)
        }
        "preup" => iface.pre_up.push(value.to_string()),
        "postup" => iface.post_up.push(value.to_string()),
        "predown" => iface.pre_down.push(value.to_string()),
        "postdown" => iface.post_down.push(value.to_string()),
        _ => {} // ignore unknown keys for forward compatibility
    }
    Ok(())
}

fn apply_peer(
    peer: &mut Peer,
    key_lc: &str,
    key: &str,
    value: &str,
    line: usize,
) -> Result<(), ParseError> {
    match key_lc {
        "publickey" => peer.public_key = value.to_string(),
        "presharedkey" => peer.preshared_key = Some(value.to_string()),
        "endpoint" => peer.endpoint = Some(value.to_string()),
        "allowedips" => peer.allowed_ips.extend(csv(value)),
        "persistentkeepalive" => {
            peer.persistent_keepalive = if value.eq_ignore_ascii_case("off") {
                None
            } else {
                Some(value.parse().map_err(|_| invalid(line, key, value))?)
            };
        }
        _ => {}
    }
    Ok(())
}

impl FromStr for WgConfig {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        enum Section {
            None,
            Interface,
            Peer,
        }

        let mut section = Section::None;
        let mut iface = Interface::default();
        let mut peers: Vec<Peer> = Vec::new();
        let mut seen_interface = false;
        let mut current_peer: Option<(usize, Peer)> = None;

        for (idx, raw) in s.lines().enumerate() {
            let line_no = idx + 1;
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }

            // Section header, e.g. `[Interface]` or `[Peer]`.
            if let Some(inner) = line.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                if let Some((header_line, peer)) = current_peer.take() {
                    if peer.public_key.is_empty() {
                        return Err(ParseError::PeerMissingPublicKey { line: header_line });
                    }
                    peers.push(peer);
                }
                match inner.trim().to_ascii_lowercase().as_str() {
                    "interface" => {
                        section = Section::Interface;
                        seen_interface = true;
                    }
                    "peer" => {
                        section = Section::Peer;
                        current_peer = Some((line_no, Peer::default()));
                    }
                    other => {
                        return Err(ParseError::UnknownSection {
                            line: line_no,
                            name: other.to_string(),
                        })
                    }
                }
                continue;
            }

            let (key, value) = match line.split_once('=') {
                Some((k, v)) => (k.trim(), v.trim()),
                None => return Err(ParseError::MalformedEntry { line: line_no }),
            };
            let key_lc = key.to_ascii_lowercase();

            match section {
                Section::None => return Err(ParseError::OutsideSection { line: line_no }),
                Section::Interface => apply_interface(&mut iface, &key_lc, key, value, line_no)?,
                Section::Peer => {
                    let peer = &mut current_peer.as_mut().unwrap().1;
                    apply_peer(peer, &key_lc, key, value, line_no)?;
                }
            }
        }

        if let Some((header_line, peer)) = current_peer.take() {
            if peer.public_key.is_empty() {
                return Err(ParseError::PeerMissingPublicKey { line: header_line });
            }
            peers.push(peer);
        }

        if !seen_interface {
            return Err(ParseError::MissingInterface);
        }

        Ok(WgConfig {
            interface: iface,
            peers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
[Interface]
PrivateKey = aaa=
Address = 10.0.0.2/32, fd00::2/128
DNS = 10.0.0.1
# a comment line
MTU = 1420
FwMark = 0xca6c

[Peer]
PublicKey = bbb=
PresharedKey = ccc=
Endpoint = vpn.example.com:51820
AllowedIPs = 0.0.0.0/0, ::/0
PersistentKeepalive = 25
";

    #[test]
    fn parses_sample() {
        let cfg: WgConfig = SAMPLE.parse().unwrap();
        assert_eq!(cfg.interface.private_key.as_deref(), Some("aaa="));
        assert_eq!(
            cfg.interface.addresses,
            vec!["10.0.0.2/32".to_string(), "fd00::2/128".to_string()]
        );
        assert_eq!(cfg.interface.dns, vec!["10.0.0.1".to_string()]);
        assert_eq!(cfg.interface.mtu, Some(1420));
        assert_eq!(cfg.interface.fwmark, Some(0xca6c));
        assert_eq!(cfg.peers.len(), 1);

        let peer = &cfg.peers[0];
        assert_eq!(peer.public_key, "bbb=");
        assert_eq!(peer.preshared_key.as_deref(), Some("ccc="));
        assert_eq!(peer.endpoint.as_deref(), Some("vpn.example.com:51820"));
        assert_eq!(
            peer.allowed_ips,
            vec!["0.0.0.0/0".to_string(), "::/0".to_string()]
        );
        assert_eq!(peer.persistent_keepalive, Some(25));
    }

    #[test]
    fn endpoint_host_handles_ipv4_ipv6_and_bare_host() {
        let mut peer = Peer::default();
        peer.endpoint = Some("1.2.3.4:51820".to_string());
        assert_eq!(peer.endpoint_host(), Some("1.2.3.4"));
        peer.endpoint = Some("[2001:db8::1]:51820".to_string());
        assert_eq!(peer.endpoint_host(), Some("2001:db8::1"));
        peer.endpoint = Some("host.example".to_string());
        assert_eq!(peer.endpoint_host(), Some("host.example"));
    }

    #[test]
    fn missing_interface_is_an_error() {
        let result: Result<WgConfig, _> = "[Peer]\nPublicKey = x=\n".parse();
        assert_eq!(result, Err(ParseError::MissingInterface));
    }

    #[test]
    fn trailing_and_full_line_comments_are_stripped() {
        let cfg: WgConfig =
            "[Interface]\n; full-line comment\nAddress = 10.0.0.1/24 ; trailing comment\n"
                .parse()
                .unwrap();
        assert_eq!(cfg.interface.addresses, vec!["10.0.0.1/24".to_string()]);
    }

    #[test]
    fn fwmark_off_is_none() {
        let cfg: WgConfig = "[Interface]\nFwMark = off\n".parse().unwrap();
        assert_eq!(cfg.interface.fwmark, None);
    }

    #[test]
    fn round_trips_through_conf_string() {
        let cfg: WgConfig = SAMPLE.parse().unwrap();
        let reparsed: WgConfig = cfg.to_conf_string().parse().unwrap();
        assert_eq!(cfg, reparsed);
    }
}
