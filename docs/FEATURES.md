# WireArch - Feature Map

Legend: ✅ planned for v1 · 🔜 near-term · 💡 later / ideas

## Tunnel management
- ✅ Import tunnels from `.conf` files (drag & drop, file picker)
- ✅ Import / export via **QR code** (scan with camera, display for transfer)
- ✅ Create / edit tunnels in a Kirigami form (Interface + Peers)
- ✅ Generate keypairs in-app (`wg genkey` equivalent)
- ✅ Honor advanced fields NM drops: `MTU`, `Table`, `FwMark`, `PreUp/PostUp/PreDown/PostDown`, `PresharedKey`
- 🔜 Multiple peers per tunnel; per-peer `AllowedIPs`
- 💡 Config validation + linting with helpful errors

## Connecting & status
- ✅ One-click connect / disconnect per tunnel
- ✅ Multi-tunnel management with fast switching
- ✅ Live status: handshake age, endpoint, throughput (↑/↓ rate), transferred totals
- ✅ Reconnect on failure; auto-connect on login (opt-in)
- 💡 Per-tunnel on-demand / trusted-network rules (Wi-Fi SSID based)

## Security
- ✅ **Kill switch** (fail-closed nftables) - global and per-tunnel
- ✅ DNS-leak protection (systemd-resolved `~.` routing domain)
- 🔜 **Split tunneling** - friendly `AllowedIPs` editor; include / exclude subnets (apps later)
- ✅ Secrets in **KWallet**; daemon runs least-privilege (only `CAP_NET_ADMIN`)
- 💡 IPv6 leak guard toggle

## Metrics & analytics
- ✅ **Hourly / daily usage** charts (per tunnel and total), upload vs download
- ✅ Current session duration
- ✅ **Total connected time** since the app/daemon started, and all-time
- 🔜 Per-tunnel history; data caps with alerts
- 💡 Export metrics (CSV / JSON)

## Server insight (per endpoint)
- ✅ **Country name + flag** of the connected server (offline GeoIP)
- ✅ **ISP / hosting provider** (AS organization) and **AS number**
- ✅ Endpoint IP, port, and resolved hostname
- 🔜 Latency / ping to endpoint; "best server" hints across your tunnels
- 💡 Map view of the active server

## KDE / Plasma integration
- ✅ **Plasma panel widget** (plasmoid): connect / switch, live stats, country + flag at a glance
- ✅ **System tray** (KStatusNotifierItem): status icon, quick menu, notifications
- ✅ Native Kirigami UI; light/dark + accent-color aware
- ✅ Desktop notifications on connect / disconnect / failure (KNotifications)
- 💡 KCM (System Settings module); global shortcut to toggle a tunnel

## Search & UX
- ✅ Search / filter tunnels by name, country, provider, or tag
- 🔜 Tags / favorites; sort by last-used or throughput
- 💡 Command-palette-style quick switch
