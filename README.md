# WireArch

A native **KDE Plasma 6** WireGuard® VPN client for **Arch Linux** - built for speed, reliability, and deep desktop integration.

> **Status:** v0.1, functional. The Rust daemon (connect, kill switch, metrics, GeoIP), the Kirigami GUI, the system tray and the Plasma panel widget are implemented and build on Arch Linux + Plasma 6.

WireArch is not a wrapper around `wg-quick`. The privileged core talks to the kernel WireGuard module directly over netlink, runs as a hardened, polkit-gated system service, and the UI is a real Kirigami app with a Plasma panel widget - so tunnels, a kill switch, usage analytics, and per-server geo/ISP information all live in one fast, native package.

## Why WireArch?

KDE already exposes WireGuard through plasma-nm/NetworkManager, but with real gaps. WireArch targets exactly those:

- **In-app `.conf` import and a built-in editor** - create or edit tunnels and generate keypairs without `nmcli`.
- **Built-in kill switch** - fail-closed nftables rules; no traffic leaks if the tunnel drops.
- **Multi-tunnel quick-switch** from a **Plasma panel widget** and the system tray.
- **Hourly / daily usage metrics** with charts, backed by a local SQLite database.
- **Per-server insight** - endpoint **country + flag**, **ISP / ASN (hosting provider)**, live throughput, current session duration and total connected time - all resolved **offline** for privacy.
- **Config fidelity** - preserves `MTU`, `Table`, `FwMark`, pre/post hooks and other fields NetworkManager silently drops.

See [docs/FEATURES.md](docs/FEATURES.md) for the full feature map.

## Architecture (short version)

```
GUI  (C++/Qt6 · QML/Kirigami · KStatusNotifierItem · Plasma plasmoid)
  │  D-Bus (system bus) - every privileged action authorized by polkit
  ▼
wirearchd  (Rust · runs with only CAP_NET_ADMIN · systemd-hardened)
  ├─ kernel WireGuard via generic netlink (no wg-quick shell-outs)
  ├─ addresses, routes and full-tunnel policy routing (wg-quick style)
  ├─ kill switch via nftables (fail-closed)
  ├─ DNS for the tunnel (resolvconf)
  ├─ metrics → SQLite (reset-aware hourly/daily rollups)
  └─ offline GeoIP / ASN lookups (DB-IP Lite .mmdb)
```

Full details and rationale: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Tech stack

| Layer | Choice |
|------|--------|
| GUI | C++20, Qt 6.8+, KDE Frameworks 6, Kirigami + Kirigami Addons |
| Tray / widget | KStatusNotifierItem + Plasma 6 plasmoid (QML) |
| Privileged daemon | Rust - `defguard_wireguard_rs` (kernel netlink), `nftables`, `zbus`/`zbus_polkit`, `rusqlite`, `maxminddb` |
| IPC | D-Bus system bus, polkit per-action authorization |
| GeoIP | DB-IP Lite (`.mmdb`) via `maxminddb`; flags from flag-icons (SVG) |
| Build | CMake + extra-cmake-modules (GUI) · Cargo (daemon) |

## Installing (Arch Linux)

```sh
cd packaging
makepkg -si
```

This builds the daemon and GUI and installs the binary, the systemd service, the
polkit policy, the D-Bus files, the desktop entry and the Plasma widget, and
fetches the offline GeoIP databases. Then enable the service:

```sh
sudo systemctl enable --now wirearchd
```

Launch **WireArch** from your application menu, or add the **WireArch** widget to
a panel.

## Building from source (development)

```sh
sudo pacman -S --needed rust extra-cmake-modules wireguard-tools

# daemon
cargo build --release --manifest-path daemon/Cargo.toml
# GUI + Plasma widget
cmake -B build -S . -G Ninja -DCMAKE_INSTALL_PREFIX=/usr
cmake --build build
```

Run without installing (dev mode, on the session bus):

```sh
./scripts/fetch-geoip.sh                                   # one-off, into ./geoip
WIREARCH_SESSION_BUS=1 WIREARCH_GEOIP_DIR="$PWD/geoip" ./daemon/target/release/wirearchd &
WIREARCH_SESSION_BUS=1 ./build/bin/wirearch
```

Activating a tunnel needs `CAP_NET_ADMIN`: run the daemon with `sudo` for live
testing, or use the installed system service. See [docs/BUILDING.md](docs/BUILDING.md).

## Privacy

GeoIP and ISP/ASN lookups use a **bundled offline database** - WireArch never sends your server IPs to a third-party geolocation API.

## License

[GPL-3.0-or-later](LICENSE).

WireGuard is a registered trademark of Jason A. Donenfeld. WireArch is an independent project, not affiliated with or endorsed by the WireGuard project.
