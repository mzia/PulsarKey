# Packaging Guide: COSMIC YubiKey FIDO2

This directory contains production packaging assets for **COSMIC YubiKey FIDO2** across both supported formats in Pop!_OS and the COSMIC Desktop ecosystem:

1. **Native Debian Package (`.deb`)** — Best for full PAM integration, greeter hooks, and COSMIC Store native installs.
2. **Flathub / COSMIC Applet Flatpak** — Containerized manifest and AppStream metadata for Flathub or the COSMIC Flatpak repository.

---

## 1. Native Debian Package (`.deb`) for COSMIC Store & APT

Pop!_OS COSMIC natively supports 1-click `.deb` package installation through the COSMIC Store and Eddy.

### Ready-Built Package:
`packaging/cosmic-fido2_1.0.0_amd64.deb`

### Installation Methods:
- **COSMIC Store / Eddy**:
  Right-click `cosmic-fido2_1.0.0_amd64.deb` in COSMIC Files -> **Open With -> Eddy / COSMIC Store** -> Click **Install**.
- **CLI / APT**:
  ```bash
  sudo apt install ./packaging/cosmic-fido2_1.0.0_amd64.deb
  ```

### What the `.deb` package installs:
- `/usr/bin/cosmic-fido2`: The compiled Rust manager & applet
- `/usr/share/applications/com.system76.cosmic-fido2.desktop`: Desktop launcher
- `/etc/xdg/autostart/com.system76.cosmic-fido2-applet.desktop`: Automatic panel applet on login
- `/usr/lib/systemd/user/cosmic-fido2-applet.service`: Systemd user service
- `/usr/share/icons/hicolor/scalable/apps/com.system76.cosmic-fido2.svg`: Modern vector icon
- `/usr/share/metainfo/com.system76.cosmic-fido2.metainfo.xml`: AppStream metadata for the COSMIC Store

### How to Rebuild the `.deb` Package:
```bash
dpkg-deb --build packaging/deb/cosmic-fido2_1.0.0_amd64 packaging/cosmic-fido2_1.0.0_amd64.deb
```

---

## 2. Flathub & COSMIC Flatpak Applet (`packaging/flatpak/`)

### Files:
- `io.github.mzia.CosmicFido2.yaml`: Flathub build manifest
- `io.github.mzia.CosmicFido2.metainfo.xml`: Freedesktop AppStream metadata
- `io.github.mzia.CosmicFido2.desktop`: Desktop entry
- `io.github.mzia.CosmicFido2.svg`: Flathub 512x512 icon
- `flathub.json`: Architecture filters

### Flatpak & PAM Sandboxing Context:
- Flatpaks run inside an isolated sandbox (`bwrap`). By Freedesktop and Flathub security policy, Flatpak applications **cannot modify `/etc/pam.d/` directly** on the host filesystem without sandbox escape mechanisms (`flatpak-spawn --host`).
- The **Applet status monitor** (`cosmic-fido2 applet`) runs seamlessly inside Flatpak with `--socket=session-bus` and `--device=all` to monitor YubiKey insertion and test biometric hardware.
- When configuring system authentication (`setup`), the app invokes the host terminal or pkexec.
- Because of this system-level PAM requirement, the **native `.deb` package is the recommended distribution mechanism** in Pop!_OS COSMIC Store.

### Building with Flatpak Builder:
```bash
cd packaging/flatpak
flatpak-builder --user --install --force-clean build-dir io.github.mzia.CosmicFido2.yaml
```

### Submitting to Flathub:
1. Fork `https://github.com/flathub/flathub`.
2. Create a new branch `add-io.github.mzia.CosmicFido2`.
3. Add the files from `packaging/flatpak/`.
4. Open a pull request against Flathub.
