# WireArch

A native **KDE Plasma 6** WireGuard® VPN client for **Arch Linux** — built for speed, reliability, and deep desktop integration.

> **Status:** early development. The architecture is locked; implementation is in progress.

WireArch is not a wrapper around `wg-quick`. The privileged core talks to the kernel WireGuard module directly over netlink, runs as a hardened, polkit-gated system service, and the UI is a real Kirigami app with a Plasma panel widget — so tunnels, a kill switch, usage analytics, and per-server geo/ISP information all live in one fast, native package.

## Why WireArch?

KDE already exposes WireGuard through plasma-nm/NetworkManager, but with real gaps. WireArch targets exactly those:

- **In-app `.conf` and QR import/export** — no `nmcli` required.
- **Built-in kill switch** — fail-closed nftables rules; no traffic leaks if the tunnel drops.
- **Split tunneling** — a friendly `AllowedIPs` editor.
- **Multi-tunnel quick-switch** from a **Plasma panel widget** and system tray, with live handshake + throughput.
- **Hourly / daily usage metrics** with charts, backed by a local time-series database.
- **Per-server insight** — endpoint **country + flag**, **ISP / ASN (hosting provider)**, current session duration, and total connected time — all resolved **offline** for privacy.
- **Config fidelity** — honors `MTU`, `Table`, `FwMark`, pre/post hooks and other fields NetworkManager silently drops.

See [docs/FEATURES.md](docs/FEATURES.md) for the full feature map.

## Architecture (short version)

```
GUI  (C++/Qt6 · QML/Kirigami · KStatusNotifierItem · Plasma plasmoid)
  │  D-Bus (system bus) — every privileged action authorized by polkit
  ▼
wirearchd  (Rust · runs with only CAP_NET_ADMIN · systemd-hardened)
  ├─ kernel WireGuard via generic netlink (no wg-quick shell-outs)
  ├─ links / addresses / routes via rtnetlink
  ├─ kill switch via nftables
  ├─ DNS via systemd-resolved (~. routing domain, leak-safe)
  ├─ metrics → SQLite (raw snapshots + hourly/daily rollups)
  └─ offline GeoIP / ASN lookups (DB-IP Lite .mmdb)
```

Full details and rationale: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Tech stack

| Layer | Choice |
|------|--------|
| GUI | C++20, Qt 6.8+, KDE Frameworks 6, Kirigami + Kirigami Addons |
| Tray / widget | KStatusNotifierItem + Plasma 6 plasmoid (QML) |
| Privileged daemon | Rust — `defguard_wireguard_rs`, `rtnetlink`, `rustables`, `zbus`/`zbus_polkit`, `rusqlite` |
| IPC | D-Bus system bus, polkit per-action authorization |
| GeoIP | DB-IP Lite (`.mmdb`) via `maxminddb`; flags from flag-icons (SVG) |
| Build | CMake + extra-cmake-modules (GUI) · Cargo (daemon) |

## Building

> Build instructions land with the first compilable scaffold. Targets Arch Linux + Plasma 6.

## Privacy

GeoIP and ISP/ASN lookups use a **bundled offline database** — WireArch never sends your server IPs to a third-party geolocation API.

## License

[GPL-3.0-or-later](LICENSE).

WireGuard is a registered trademark of Jason A. Donenfeld. WireArch is an independent project, not affiliated with or endorsed by the WireGuard project.
