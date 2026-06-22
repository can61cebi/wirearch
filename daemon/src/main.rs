// During early scaffolding several items are defined ahead of their first use.
#![allow(dead_code)]

mod config;
mod manager;
mod store;

use std::path::PathBuf;

use manager::Manager;
use store::Store;

const DBUS_NAME: &str = "tr.cebi.wirearch";
const DBUS_PATH: &str = "/tr/cebi/wirearch";

/// Where tunnel definitions live. systemd sets STATE_DIRECTORY for the
/// installed service; otherwise fall back to the user's data dir (dev mode).
fn state_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("STATE_DIRECTORY") {
        return PathBuf::from(dir).join("tunnels");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/wirearch/tunnels");
    }
    PathBuf::from("/var/lib/wirearch/tunnels")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `--session` (or WIREARCH_SESSION_BUS) runs on the session bus for
    // development without root; the installed service uses the system bus.
    let use_session = std::env::args().any(|a| a == "--session")
        || std::env::var_os("WIREARCH_SESSION_BUS").is_some();

    let store = Store::new(state_dir())?;
    let manager = Manager::new(store);

    let builder = if use_session {
        eprintln!("wirearchd: connecting to the SESSION bus (dev mode)");
        zbus::connection::Builder::session()?
    } else {
        zbus::connection::Builder::system()?
    };

    let _conn = builder
        .name(DBUS_NAME)?
        .serve_at(DBUS_PATH, manager)?
        .build()
        .await?;

    eprintln!(
        "wirearchd {} ready, owning {DBUS_NAME}",
        env!("CARGO_PKG_VERSION")
    );

    tokio::signal::ctrl_c().await?;
    eprintln!("wirearchd: shutting down");
    Ok(())
}
