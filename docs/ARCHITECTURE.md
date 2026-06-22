# WireArch - Architecture

This document records the design decisions and their rationale. Target platform: Arch Linux with the in-kernel WireGuard module and KDE Plasma 6.

## Principles

1. **Privilege separation.** Bringing tunnels up/down, editing routes, and installing firewall rules need `CAP_NET_ADMIN`. The GUI never runs as root - a small, hardened daemon does all privileged work, and every action is authorized by polkit.
2. **Native, not wrapped.** Configure the kernel WireGuard device directly via generic netlink instead of shelling out to `wg-quick`. Faster, more reliable, and lets us honor config fields NetworkManager drops.
3. **Fail-closed.** The kill switch defaults to dropping traffic that isn't the tunnel or the handshake to the pinned endpoint.
4. **Offline & private.** No per-server calls to third-party geolocation APIs.

## Components

### 1. `wirearchd` - privileged daemon (Rust)

- Runs as a **systemd system service** activated on the **D-Bus system bus**, well-known name `tr.cebi.wirearch`.
- Holds **only `CAP_NET_ADMIN`** (plus `CAP_NET_RAW` if needed); all other capabilities dropped. Hardened via `systemd.exec`: `ProtectSystem=strict`, `RestrictAddressFamilies=AF_NETLINK AF_INET AF_INET6 AF_UNIX` (must include `AF_NETLINK`), `SystemCallFilter=@system-service`, `NoNewPrivileges=yes`, `MemoryDenyWriteExecute=yes`, … Note: `ProtectKernelModules` is avoided so the `wireguard` module can load; we pre-load it via `modules-load.d`.
- **Device config + live stats:** [`defguard_wireguard_rs`](https://github.com/DefGuard/wireguard-rs) over kernel netlink (`WGApi`). Reads `rx_bytes` / `tx_bytes` / `last_handshake_time` straight from netlink - no `wg show` parsing.
- **Links / addresses / routes:** [`rtnetlink`](https://docs.rs/rtnetlink). Full-tunnel policy routing uses `fwmark` + a dedicated routing table + `suppress_prefixlength 0` (the same technique as `wg-quick`).
- **Kill switch:** nftables via [`rustables`](https://docs.rs/rustables) - an `inet` table with a fail-closed `output` chain permitting only loopback, established/related, the tunnel interface, and the pinned `endpoint:port` handshake (plus optional LAN).
- **DNS:** systemd-resolved over D-Bus (`org.freedesktop.resolve1`), setting per-link DNS and the `~.` routing domain for leak-safe full-tunnel.
- **Metrics:** polls netlink every 15-30 s, stores in SQLite via [`rusqlite`](https://docs.rs/rusqlite).
- **GeoIP:** offline `.mmdb` lookups via [`maxminddb`](https://crates.io/crates/maxminddb) (≥ 0.27 - earlier versions carry RUSTSEC-2025-0132).

### 2. `wirearch` - GUI (C++ / Qt6 / QML / Kirigami)

- Kirigami + Kirigami Addons UI; a C++ backend (required for generic QtDBus to the daemon and for `KStatusNotifierItem` - neither is available from pure QML).
- System tray via **KStatusNotifierItem**, exposed from C++ to QML.
- Talks to `wirearchd` over the D-Bus system bus. Holds **no privilege** itself.
- Secrets (private/preshared keys) stored via **KWallet**.

### 3. Plasma panel widget (plasmoid, QML)

- A Plasma 6 applet (root `PlasmoidItem`, `X-Plasma-API-Minimum-Version: 6.0`) talking to the same daemon over D-Bus.
- Quick connect / switch, active tunnel, live throughput, handshake age, server country + flag.

## The D-Bus contract

`tr.cebi.wirearch` on the **system bus**:

- `Manager` interface - list / import / add / edit / remove tunnels; connect / disconnect; kill-switch on/off; query state; signals for state changes and live stats.
- Each privileged method is gated by a polkit action under `tr.cebi.wirearch.*` (e.g. `auth_admin_keep` for connect/disconnect).

## Metrics schema (sketch)

- `sample(ts, iface, pubkey, rx_bytes, tx_bytes, last_handshake)` - raw u64 snapshots; timestamp-first composite PK; WAL mode.
- `rollup_hour` / `rollup_day` - reset-aware deltas: `delta = (cur >= prev) ? cur - prev : cur` (counters reset to 0 when an interface is recreated).
- Session duration and lifetime-connected time derived from connect/disconnect events.

## GeoIP {#geoip}

- Resolve the **actual** endpoint IP from the running interface (kernel-reported numeric endpoint) once connected - no extra DNS lookups, and correct after roaming.
- Look it up against bundled **DB-IP Lite** Country + ASN `.mmdb` (CC BY 4.0 - redistributable with attribution): country name + ISO code (→ flag) and AS number + organization (→ ISP / hosting provider).
- Flags rendered from bundled **flag-icons** (MIT) SVGs via Qt's SVG image plugin. Lookup cache keyed by IP, invalidated on database update (`build_epoch`).
