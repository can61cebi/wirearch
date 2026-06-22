//! The `tr.cebi.wirearch.Manager` D-Bus interface: tunnel CRUD, connect/
//! disconnect over kernel netlink, live status, usage metrics, and offline
//! geo lookups. SetKillSwitch is implemented in a later commit (nftables).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use zbus::fdo;
use zbus::interface;
use zbus::zvariant::{OwnedValue, Value};

use crate::config::WgConfig;
use crate::geo::GeoDb;
use crate::metrics::Metrics;
use crate::store::{Store, Tunnel};
use crate::wg;

#[derive(Clone, Debug)]
struct ActiveTunnel {
    id: String,
    ifname: String,
    connected_at: Instant,
    endpoint: String,
}

pub struct Manager {
    store: Store,
    active: Arc<Mutex<Option<ActiveTunnel>>>,
    geo: Option<Arc<GeoDb>>,
    /// Total connected time accumulated from finished sessions since startup.
    lifetime: Arc<Mutex<Duration>>,
    metrics: Option<Arc<Metrics>>,
    killswitch: Arc<Mutex<bool>>,
}

impl Manager {
    pub fn new(store: Store, metrics: Option<Metrics>) -> Self {
        let active = Arc::new(Mutex::new(None));
        let lifetime = Arc::new(Mutex::new(Duration::ZERO));
        let metrics = metrics.map(Arc::new);

        if let Some(m) = &metrics {
            let active_for_sampler = Arc::clone(&active);
            let metrics_for_sampler = Arc::clone(m);
            tokio::spawn(async move {
                run_sampler(active_for_sampler, metrics_for_sampler).await;
            });
        }

        Self {
            store,
            active,
            geo: GeoDb::open_default().map(Arc::new),
            lifetime,
            metrics,
            killswitch: Arc::new(Mutex::new(false)),
        }
    }

    /// Apply or remove the nftables kill switch based on the current
    /// preference and active tunnel. Runs `nft` off the async executor.
    async fn apply_killswitch(&self) -> Result<(), String> {
        let enabled = *self.killswitch.lock().unwrap();
        let active = self.active.lock().unwrap().clone();
        tokio::task::spawn_blocking(move || {
            if !enabled {
                return crate::killswitch::disable().map_err(|e| e.to_string());
            }
            match active {
                Some(a) => crate::killswitch::enable(Some(&a.ifname), Some(&a.endpoint)),
                None => crate::killswitch::enable(None, None),
            }
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("join error: {e}"))?
    }
}

#[interface(name = "tr.cebi.wirearch.Manager")]
impl Manager {
    async fn list_tunnels(&self) -> fdo::Result<Vec<HashMap<String, OwnedValue>>> {
        let tunnels = self.store.list().map_err(to_fdo)?;
        Ok(tunnels.iter().map(tunnel_to_dict).collect())
    }

    async fn get_tunnel(&self, id: String) -> fdo::Result<HashMap<String, OwnedValue>> {
        let tunnel = self.store.get(&id).map_err(to_fdo)?;
        let mut dict = tunnel_to_dict(&tunnel);
        // The raw config (including the private key) is returned only on an
        // explicit GetTunnel, for the editor and export.
        // TODO: move secret material to KWallet instead of over the bus.
        dict.insert("config".to_string(), owned(tunnel.config.to_conf_string()));
        Ok(dict)
    }

    async fn import_config(&self, name: String, config: String) -> fdo::Result<String> {
        let parsed: WgConfig = config
            .parse()
            .map_err(|e| fdo::Error::InvalidArgs(format!("invalid config: {e}")))?;
        let id = self.store.unique_id(&name).map_err(to_fdo)?;
        let tunnel = Tunnel {
            id: id.clone(),
            name,
            config: parsed,
        };
        self.store.save(&tunnel).map_err(to_fdo)?;
        Ok(id)
    }

    async fn remove_tunnel(&self, id: String) -> fdo::Result<()> {
        self.store.remove(&id).map_err(to_fdo)
    }

    /// Create (empty id) or update a tunnel from wg-quick `.conf` text.
    async fn save_tunnel(&self, id: String, name: String, config: String) -> fdo::Result<String> {
        let parsed: WgConfig = config
            .parse()
            .map_err(|e| fdo::Error::InvalidArgs(format!("invalid config: {e}")))?;
        let id = if id.is_empty() {
            self.store.unique_id(&name).map_err(to_fdo)?
        } else {
            id
        };
        let tunnel = Tunnel {
            id: id.clone(),
            name,
            config: parsed,
        };
        self.store.save(&tunnel).map_err(to_fdo)?;
        Ok(id)
    }

    /// Bring a tunnel up (CAP_NET_ADMIN). Tears down the active tunnel first.
    async fn connect(&self, id: String) -> fdo::Result<()> {
        let tunnel = self.store.get(&id).map_err(to_fdo)?;
        let ifname = wg::ifname_for(&tunnel.id);

        let previous = self.active.lock().unwrap().clone();
        if let Some(prev) = previous {
            let elapsed = prev.connected_at.elapsed();
            let prev_if = prev.ifname.clone();
            let _ = tokio::task::spawn_blocking(move || wg::down(&prev_if)).await;
            *self.lifetime.lock().unwrap() += elapsed;
        }

        let cfg = tunnel.config.clone();
        let ifn = ifname.clone();
        tokio::task::spawn_blocking(move || wg::up(&ifn, &cfg))
            .await
            .map_err(|e| fdo::Error::Failed(format!("join error: {e}")))?
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        let ep_if = ifname.clone();
        let endpoint = tokio::task::spawn_blocking(move || wg::stats(&ep_if))
            .await
            .ok()
            .and_then(|r| r.ok())
            .map(|s| s.endpoint)
            .unwrap_or_default();

        *self.active.lock().unwrap() = Some(ActiveTunnel {
            id,
            ifname,
            connected_at: Instant::now(),
            endpoint,
        });

        if *self.killswitch.lock().unwrap() {
            if let Err(e) = self.apply_killswitch().await {
                eprintln!("wirearchd: kill switch update failed: {e}");
            }
        }
        Ok(())
    }

    /// Bring the active tunnel down (CAP_NET_ADMIN).
    async fn disconnect(&self, _id: String) -> fdo::Result<()> {
        let active = self.active.lock().unwrap().clone();
        let Some(active) = active else {
            return Ok(());
        };
        let ifn = active.ifname;
        tokio::task::spawn_blocking(move || wg::down(&ifn))
            .await
            .map_err(|e| fdo::Error::Failed(format!("join error: {e}")))?
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        *self.lifetime.lock().unwrap() += active.connected_at.elapsed();
        *self.active.lock().unwrap() = None;

        if *self.killswitch.lock().unwrap() {
            // Stay fail-closed with no active tunnel (blocks all but lo/established).
            if let Err(e) = self.apply_killswitch().await {
                eprintln!("wirearchd: kill switch update failed: {e}");
            }
        }
        Ok(())
    }

    /// Live status for a tunnel: throughput counters, last handshake, current
    /// session duration, and total connected time since the service started.
    async fn get_status(&self, id: String) -> fdo::Result<HashMap<String, OwnedValue>> {
        let (active_ifname, since, total) = {
            let active = self.active.lock().unwrap();
            let lifetime = *self.lifetime.lock().unwrap();
            match active.as_ref() {
                Some(a) if a.id == id => {
                    let s = a.connected_at.elapsed();
                    (Some(a.ifname.clone()), s.as_secs(), (lifetime + s).as_secs())
                }
                _ => (None, 0u64, lifetime.as_secs()),
            }
        };

        let mut m = HashMap::new();
        m.insert("totalConnected".to_string(), owned(total));

        let Some(ifn) = active_ifname else {
            m.insert("state".to_string(), owned("inactive".to_string()));
            m.insert("sinceConnected".to_string(), owned(0u64));
            m.insert("rxBytes".to_string(), owned(0u64));
            m.insert("txBytes".to_string(), owned(0u64));
            m.insert("lastHandshake".to_string(), owned(0i64));
            m.insert("endpoint".to_string(), owned(String::new()));
            return Ok(m);
        };

        let stats = tokio::task::spawn_blocking(move || wg::stats(&ifn))
            .await
            .map_err(|e| fdo::Error::Failed(format!("join error: {e}")))?
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        m.insert("state".to_string(), owned("active".to_string()));
        m.insert("sinceConnected".to_string(), owned(since));
        m.insert("rxBytes".to_string(), owned(stats.rx_bytes));
        m.insert("txBytes".to_string(), owned(stats.tx_bytes));
        m.insert("lastHandshake".to_string(), owned(stats.last_handshake));
        m.insert("endpoint".to_string(), owned(stats.endpoint));
        Ok(m)
    }

    /// Usage rollups for charts. `period` is "hour" or "day"; returns up to
    /// `count` most-recent buckets (oldest first) with ts/rx/tx.
    async fn get_metrics(
        &self,
        period: String,
        count: u32,
    ) -> fdo::Result<Vec<HashMap<String, OwnedValue>>> {
        let Some(metrics) = self.metrics.clone() else {
            return Ok(Vec::new());
        };
        let is_day = period == "day";
        let rows = tokio::task::spawn_blocking(move || {
            if is_day {
                metrics.daily(count)
            } else {
                metrics.hourly(count)
            }
        })
        .await
        .map_err(|e| fdo::Error::Failed(format!("join error: {e}")))?;

        let bucket: i64 = if is_day { 86_400 } else { 3_600 };
        Ok(rows
            .into_iter()
            .map(|(b, rx, tx)| {
                let mut m = HashMap::new();
                m.insert("ts".to_string(), owned(b * bucket));
                m.insert("rx".to_string(), owned(rx));
                m.insert("tx".to_string(), owned(tx));
                m
            })
            .collect())
    }

    /// Enable or disable the fail-closed nftables kill switch.
    async fn set_kill_switch(&self, enabled: bool) -> fdo::Result<()> {
        *self.killswitch.lock().unwrap() = enabled;
        self.apply_killswitch().await.map_err(fdo::Error::Failed)
    }

    /// Resolve an endpoint (host:port or IP) to its country and ISP/ASN,
    /// fully offline. Returns an empty dict if no GeoIP database is available.
    async fn geo(&self, endpoint: String) -> fdo::Result<HashMap<String, OwnedValue>> {
        let Some(db) = self.geo.clone() else {
            return Ok(HashMap::new());
        };
        let resolved =
            tokio::task::spawn_blocking(move || crate::geo::resolve_and_lookup(&db, &endpoint))
                .await
                .map_err(|e| fdo::Error::Failed(format!("join error: {e}")))?;
        let Some((ip, info)) = resolved else {
            return Ok(HashMap::new());
        };
        let mut m = HashMap::new();
        m.insert("ip".to_string(), owned(ip.to_string()));
        m.insert("countryCode".to_string(), owned(info.country_code));
        m.insert("country".to_string(), owned(info.country_name));
        m.insert("asn".to_string(), owned(info.asn));
        m.insert("asOrg".to_string(), owned(info.as_org));
        Ok(m)
    }

    /// Generate a fresh WireGuard keypair, returned as (private, public) base64.
    async fn generate_keypair(&self) -> fdo::Result<(String, String)> {
        let private = defguard_wireguard_rs::key::Key::generate();
        let public = private.public_key();
        Ok((private.to_string(), public.to_string()))
    }

    #[zbus(property)]
    async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[zbus(property)]
    async fn active_tunnel(&self) -> String {
        self.active
            .lock()
            .unwrap()
            .as_ref()
            .map(|a| a.id.clone())
            .unwrap_or_default()
    }

    #[zbus(property)]
    async fn kill_switch_enabled(&self) -> bool {
        *self.killswitch.lock().unwrap()
    }
}

/// Reset-aware byte delta: if the counter went backwards (interface recreated),
/// treat the current value as the increment.
fn delta(cur: u64, prev: u64) -> u64 {
    if cur >= prev {
        cur - prev
    } else {
        cur
    }
}

/// Background task: every 30s, while a tunnel is active, record the byte
/// deltas into the metrics rollups.
async fn run_sampler(active: Arc<Mutex<Option<ActiveTunnel>>>, metrics: Arc<Metrics>) {
    let mut prev: Option<(String, u64, u64)> = None;
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let ifname = active.lock().unwrap().as_ref().map(|a| a.ifname.clone());
        let Some(ifname) = ifname else {
            prev = None;
            continue;
        };

        let ifn = ifname.clone();
        let stats = match tokio::task::spawn_blocking(move || wg::stats(&ifn)).await {
            Ok(Ok(s)) => s,
            _ => continue,
        };

        let (drx, dtx) = match &prev {
            Some((pif, prx, ptx)) if *pif == ifname => {
                (delta(stats.rx_bytes, *prx), delta(stats.tx_bytes, *ptx))
            }
            // First sample for a freshly created interface (counters start at 0).
            _ => (stats.rx_bytes, stats.tx_bytes),
        };
        prev = Some((ifname, stats.rx_bytes, stats.tx_bytes));

        if drx > 0 || dtx > 0 {
            let m = Arc::clone(&metrics);
            let _ = tokio::task::spawn_blocking(move || m.add(drx, dtx)).await;
        }
    }
}

/// Convert a stored tunnel into the `a{sv}` dictionary the GUI consumes.
/// List-valued fields are joined with ", " for now.
fn tunnel_to_dict(t: &Tunnel) -> HashMap<String, OwnedValue> {
    let endpoint = t
        .config
        .peers
        .first()
        .and_then(|p| p.endpoint.clone())
        .unwrap_or_default();
    let allowed_ips: Vec<String> = t
        .config
        .peers
        .iter()
        .flat_map(|p| p.allowed_ips.clone())
        .collect();

    let mut m = HashMap::new();
    m.insert("id".to_string(), owned(t.id.clone()));
    m.insert("name".to_string(), owned(t.name.clone()));
    m.insert("endpoint".to_string(), owned(endpoint));
    m.insert(
        "addresses".to_string(),
        owned(t.config.interface.addresses.join(", ")),
    );
    m.insert("dns".to_string(), owned(t.config.interface.dns.join(", ")));
    m.insert("allowedIps".to_string(), owned(allowed_ips.join(", ")));
    m.insert("peerCount".to_string(), owned(t.config.peers.len() as u32));
    m
}

fn owned<'a, T: Into<Value<'a>>>(v: T) -> OwnedValue {
    OwnedValue::try_from(v.into()).expect("owned value conversion")
}

fn to_fdo<E: std::fmt::Display>(e: E) -> fdo::Error {
    fdo::Error::Failed(e.to_string())
}
