//! The `tr.cebi.wirearch.Manager` D-Bus interface.
//!
//! Tunnel management (import/list/get/remove) is implemented here. Connect,
//! Disconnect and SetKillSwitch are stubs until the netlink and nftables
//! layers land (tasks #2 and #3); they will be polkit-gated.

use std::collections::HashMap;

use zbus::fdo;
use zbus::interface;
use zbus::zvariant::{OwnedValue, Value};

use crate::config::WgConfig;
use crate::store::{Store, Tunnel};

pub struct Manager {
    store: Store,
}

impl Manager {
    pub fn new(store: Store) -> Self {
        Self { store }
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

    async fn connect(&self, _id: String) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported(
            "connect is not implemented yet".to_string(),
        ))
    }

    async fn disconnect(&self, _id: String) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported(
            "disconnect is not implemented yet".to_string(),
        ))
    }

    async fn set_kill_switch(&self, _enabled: bool) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported(
            "kill switch is not implemented yet".to_string(),
        ))
    }

    #[zbus(property)]
    async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[zbus(property)]
    async fn active_tunnel(&self) -> String {
        String::new()
    }

    #[zbus(property)]
    async fn kill_switch_enabled(&self) -> bool {
        false
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
    m.insert(
        "peerCount".to_string(),
        owned(t.config.peers.len() as u32),
    );
    m
}

fn owned<'a, T: Into<Value<'a>>>(v: T) -> OwnedValue {
    OwnedValue::try_from(v.into()).expect("owned value conversion")
}

fn to_fdo<E: std::fmt::Display>(e: E) -> fdo::Error {
    fdo::Error::Failed(e.to_string())
}
