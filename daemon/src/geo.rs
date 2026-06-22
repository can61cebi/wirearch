//! Offline GeoIP / ASN lookups using bundled DB-IP Lite .mmdb databases.
//!
//! No network calls are ever made for geolocation: looking a server's country
//! and ISP up offline avoids leaking which VPN endpoints a user connects to.

use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};

use maxminddb::{geoip2, MaxMindDbError, Reader};

/// Resolved geo information for an IP address.
#[derive(Debug, Default, Clone)]
pub struct GeoInfo {
    /// ISO 3166-1 alpha-2 code, e.g. "DE". Empty if unknown.
    pub country_code: String,
    /// English country name, e.g. "Germany". Empty if unknown.
    pub country_name: String,
    /// Autonomous system number, 0 if unknown.
    pub asn: u32,
    /// AS organization (ISP / hosting provider). Empty if unknown.
    pub as_org: String,
}

/// Bundled country + ASN databases, opened once and queried in-process.
pub struct GeoDb {
    country: Reader<Vec<u8>>,
    asn: Reader<Vec<u8>>,
}

impl GeoDb {
    pub fn open(dir: &Path) -> Result<Self, MaxMindDbError> {
        let country = Reader::open_readfile(dir.join("dbip-country-lite.mmdb"))?;
        let asn = Reader::open_readfile(dir.join("dbip-asn-lite.mmdb"))?;
        Ok(Self { country, asn })
    }

    /// Open from the first directory that has the databases:
    /// `$WIREARCH_GEOIP_DIR`, /usr/share/wirearch/geoip, /var/lib/wirearch/geoip, ./geoip.
    pub fn open_default() -> Option<Self> {
        for dir in geoip_dirs() {
            if let Ok(db) = Self::open(&dir) {
                eprintln!("wirearchd: loaded GeoIP databases from {}", dir.display());
                return Some(db);
            }
        }
        eprintln!("wirearchd: no GeoIP databases found; country/ISP will be empty");
        None
    }

    pub fn lookup(&self, ip: IpAddr) -> GeoInfo {
        let mut info = GeoInfo::default();

        if let Ok(result) = self.country.lookup(ip) {
            if let Ok(Some(country)) = result.decode::<geoip2::Country>() {
                if let Some(code) = country.country.iso_code {
                    info.country_code = code.to_string();
                }
                if let Some(name) = country.country.names.english {
                    info.country_name = name.to_string();
                }
            }
        }

        if let Ok(result) = self.asn.lookup(ip) {
            if let Ok(Some(asn)) = result.decode::<geoip2::Asn>() {
                info.asn = asn.autonomous_system_number.unwrap_or(0);
                if let Some(org) = asn.autonomous_system_organization {
                    info.as_org = org.to_string();
                }
            }
        }

        info
    }
}

/// Extract the host part of an endpoint (`host:port`, `[v6]:port`, or bare host).
pub fn host_of(endpoint: &str) -> &str {
    if let Some(rest) = endpoint.strip_prefix('[') {
        rest.split(']').next().unwrap_or(endpoint)
    } else {
        endpoint.rsplit_once(':').map(|(h, _)| h).unwrap_or(endpoint)
    }
}

/// Resolve an endpoint to an IP (DNS if it is a hostname) and look it up.
/// Blocking; call from a blocking context.
pub fn resolve_and_lookup(db: &GeoDb, endpoint: &str) -> Option<(IpAddr, GeoInfo)> {
    let host = host_of(endpoint);
    let ip = match host.parse::<IpAddr>() {
        Ok(ip) => ip,
        Err(_) => (host, 0u16).to_socket_addrs().ok()?.next()?.ip(),
    };
    Some((ip, db.lookup(ip)))
}

fn geoip_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = std::env::var_os("WIREARCH_GEOIP_DIR") {
        dirs.push(PathBuf::from(d));
    }
    dirs.push(PathBuf::from("/usr/share/wirearch/geoip"));
    dirs.push(PathBuf::from("/var/lib/wirearch/geoip"));
    dirs.push(PathBuf::from("geoip"));
    dirs
}
