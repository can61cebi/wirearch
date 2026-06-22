// During early scaffolding several items are defined ahead of their first use.
#![allow(dead_code)]

mod config;

fn main() {
    println!("wirearchd {}", env!("CARGO_PKG_VERSION"));
    // Subsequent commits wire up, in order:
    //   * the tr.cebi.wirearch D-Bus system service (zbus)
    //   * kernel WireGuard control over generic netlink
    //   * the nftables kill switch and policy routing
    //   * usage metrics (SQLite) and offline GeoIP / ASN lookups
}
