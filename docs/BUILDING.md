# Building WireArch

WireArch targets **Arch Linux + KDE Plasma 6**. It has two build systems: **Cargo** (the Rust daemon `wirearchd`) and **CMake/ECM** (the C++/QML GUI and the Plasma widget).

## Dependencies

Most of the KDE/Qt stack ships with a Plasma desktop. On a typical Arch + Plasma 6 system you usually only need to add the Rust toolchain and `extra-cmake-modules`:

```sh
sudo pacman -S --needed rust extra-cmake-modules wireguard-tools
```

Full dependency list:

- **Toolchain:** `rust` (or `rustup`), `extra-cmake-modules`, `cmake`, `ninja`, `gcc`, `pkgconf`
- **Qt6:** `qt6-base`, `qt6-declarative`, `qt6-svg`
- **KDE Frameworks 6:** `kcoreaddons`, `ki18n`, `kconfig`, `kcrash`, `kdbusaddons`, `kiconthemes`, `kirigami`, `kirigami-addons`, `knotifications`, `kstatusnotifieritem`, `kwallet`, `prison` (QR), `kquickcharts` (charts), `kpackage`, `libplasma`
- **Networking:** in-kernel WireGuard module, `nftables`, `wireguard-tools` (optional, for testing)

The kernel WireGuard module must be loadable:

```sh
sudo modprobe wireguard            # one-off
# packaging/modules-load.d/wirearch.conf loads it automatically at boot
```

## Build (work in progress)

```sh
# Daemon (Rust)
cargo build --release --manifest-path daemon/Cargo.toml

# GUI + plasmoid (CMake/ECM)
cmake -B build -S . -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build
```

## Install (system integration)

Deployment files live under `packaging/`:

| File | Installs to |
|------|-------------|
| `packaging/systemd/wirearchd.service` | `/usr/lib/systemd/system/` |
| `packaging/systemd/wirearch.sysusers` | `/usr/lib/sysusers.d/wirearch.conf` |
| `packaging/dbus/tr.cebi.wirearch.conf` | `/usr/share/dbus-1/system.d/` |
| `packaging/dbus/tr.cebi.wirearch.service` | `/usr/share/dbus-1/system-services/` |
| `packaging/polkit/tr.cebi.wirearch.policy` | `/usr/share/polkit-1/actions/` |
| `packaging/modules-load.d/wirearch.conf` | `/usr/lib/modules-load.d/` |
| `wirearchd` binary | `/usr/lib/wirearch/` |

A `PKGBUILD` will wrap all of this once the binaries build.
