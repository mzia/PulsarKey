# 🌌 PulsarKey

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Pop!__OS%20COSMIC%20%7C%20macOS-teal.svg?style=flat-square)](https://system76.com/cosmic)
[![Packaging](https://img.shields.io/badge/Packaging-Debian%20.deb%20%7C%20macOS%20.dmg%20%7C%20Homebrew-purple.svg?style=flat-square)](packaging/)
[![Release](https://img.shields.io/github/v/release/mzia/PulsarKey?style=flat-square)](https://github.com/mzia/PulsarKey/releases)

> **Hardware-backed FIDO2 & Biometric Security Suite for Pop!_OS COSMIC & macOS**  
> *Instant YubiKey Bio unlocking, passwordless sudo, Presence Sentinel auto-lock, and native desktop settings.*

---

## ⚡ Quick Start (Get Running in 2 Minutes)

### 1. Install PulsarKey

#### 🐧 Pop!_OS / Linux
Download the `.deb` package from the [Latest Release](https://github.com/mzia/PulsarKey/releases):
```bash
sudo apt install ./pulsarkey_1.4.1_amd64.deb
```
*(Or build from source: `git clone https://github.com/mzia/PulsarKey.git && cd PulsarKey && cargo build --release && sudo make install`)*

#### 🍎 macOS (Monterey 12+)
Download your preferred installer from the [Latest Release](https://github.com/mzia/PulsarKey/releases):
- **Guided Installer**: Download and run **`PulsarKey-1.4.1.pkg`**
- **Drag-and-Drop**: Open **`PulsarKey-1.4.1.dmg`** and drag to `/Applications`
- **Homebrew Cask**:
  ```bash
  brew install pam-u2f ykman
  brew install --cask packaging/macos/homebrew/pulsarkey.rb
  ```
*(Or compile locally: `./packaging/macos/build_mac.sh`)*

---

### 2. Run Guided Setup

Insert your YubiKey and run:
```bash
sudo pulsarkey setup
```

The wizard will:
1. Detect your hardware and install required PAM libraries automatically.
2. Prompt you to touch or scan your fingerprint on your YubiKey.
3. Optionally pair a secondary backup key.
4. Safely configure `sudo`, lockscreen, and administrative elevation.

---

### 3. Launch PulsarKey or Panel Applet

- **Interactive Security Dashboard**:
  ```bash
  pulsarkey           # Launches interactive terminal control suite
  pulsarkey status    # Direct summary of security profile, PAM health, & keys
  ```
- **Top Bar Panel Applet** (COSMIC Linux):
  ```bash
  pulsarkey applet --install-autostart
  ```
- **Background Sentinel Daemon** (macOS):
  ```bash
  pulsarkey applet &
  ```

---

## 💡 Everyday Usage

### 🔒 Lockscreen Unlocking
- **Pop!_OS COSMIC Greeter**: Press <kbd>Space</kbd> + <kbd>Enter</kbd>, then scan your fingerprint or touch your key when the sensor pulses green.
- **macOS Screensaver**: Wake your Mac display and scan/touch your YubiKey to unlock.

### ⚡ Passwordless `sudo` & Elevation
Run any administrative command or trigger a system elevation dialog (e.g. Pop!_Shop, Eddy):
```bash
sudo whoami
```
Scan your fingerprint or touch your key when prompted — no password typing required!

### 🛡️ Presence Sentinel (Auto-Lock on Key Removal)
PulsarKey continuously monitors token presence. The second you unplug your YubiKey from your computer, your desktop locks automatically.
- **Toggle Sentinel**:
  ```bash
  pulsarkey autolock enable     # or disable / status
  ```
  *(Or toggle directly from the CLI or panel applet).*

---

## ⚙️ Interactive Security Control Center (`pulsarkey`)

PulsarKey provides an interactive terminal control suite and native COSMIC panel applet with sub-second response times:

Launch via `pulsarkey`:

| Menu Option | What You Can Do |
| :--- | :--- |
| **[1] 📊 Overview** | Live hardware telemetry, active security profile, PAM health, and Sentinel auto-lock status |
| **[2] 🛡️ Profiles** | One-click switching between Convenience, Fortress (2FA), and Lockdown profiles |
| **[3] 🧬 Biometrics & PIN** | View on-key fingerprints, enroll new fingers, rename templates, manage FIDO2 PIN |
| **[4] 👯 Backup Keys** | Redundancy health check and guided pairing for secondary security keys |
| **[5] 🛟 Rescue Kit** | Generate emergency recovery paper tokens and access offline rescue runbooks |
| **[6] 🔑 SSH & Git Signing** | Generate hardware-backed `ed25519-sk` keys and configure commit verification |
| **[7] 🛡️ Presence Sentinel** | Toggle automatic workstation lock upon USB key removal |

---

## 🛡️ Security Profiles

Choose how strict you want your desktop security to be:

| Profile | Mode | Authentication Requirement | Fallback |
| :--- | :--- | :--- | :--- |
| **Convenience** (Default) | 1FA | Fingerprint or token touch alone | Password fallback enabled if key is absent |
| **Fortress** | True 2FA | **Both** password **and** security key touch | Password alone is denied |
| **Lockdown** | Strict | Physical key strictly mandatory | **Zero password fallback** |

Switch anytime:
```bash
sudo pulsarkey profile [convenience | fortress | lockdown]
```
*(Or switch directly from the interactive menu or panel applet).*

---

## 🛟 Emergency Recovery & Paper Key Kit

PulsarKey guarantees you **never get locked out** if your hardware keys are lost, stolen, or damaged:

1. **Generate Printable Emergency Paper Keys**:
   ```bash
   pulsarkey rescue generate
   ```
   *Creates 8 single-use, cryptographically hashed recovery tokens (`XXXX-XXXX-XXXX-XXXX`) and saves a printable certificate to your Desktop (`PulsarKey-Emergency-Recovery-Kit.txt`).*

2. **Verify a Token**:
   ```bash
   pulsarkey rescue verify <CODE>
   ```

3. **Offline Rescue Runbook (Single-User & Live USB)**:
   ```bash
   pulsarkey rescue runbook
   ```
   *Displays step-by-step procedures to recover root access via systemd-boot maintenance mode or Live USB chroot.*

4. **Create Offline Rescue Flash Drive**:
   ```bash
   sudo pulsarkey rescue usb /media/$USER/<USB_NAME>
   ```
   *Creates an automated emergency script (`pulsar-rescue.sh`) on a USB drive that auto-detects LUKS partitions and restores password access.*

---

## 🧬 Biometrics, SSH, & Redundancy Tools

- **Manage Fingerprints & PIN Directly**:
  ```bash
  pulsarkey bio                           # Interactive fingerprint management dashboard
  pulsarkey bio add "Right Index"         # Enroll new fingerprint with custom label
  pulsarkey pin change                    # Change FIDO2 hardware PIN
  ```
- **Hardware SSH & Git Commit Signing**:
  ```bash
  pulsarkey ssh-setup                     # Generates resident ed25519-sk key & configures Git signing
  ```
- **Pair a Secondary Backup Key**:
  ```bash
  sudo pulsarkey backup pair              # Enrolls secondary token for fail-safe redundancy
  pulsarkey backup test                   # Verifies currently connected key
  ```
- **View Security Audit Log**:
  ```bash
  pulsarkey audit                         # Formatted log viewer (or pulsarkey audit --clear)
  ```

---

## 📋 Command Quick Reference

| Command | Privileges | Description |
| :--- | :--- | :--- |
| `pulsarkey` | User | Launches interactive Security Control Center dashboard |
| `pulsarkey status` | User | Displays comprehensive security status and hardware health |
| `pulsarkey applet [--install-autostart]` | User | Runs COSMIC panel applet (Linux) or Sentinel daemon (macOS) |
| `sudo pulsarkey setup` | Root | Guided enrollment for primary & backup keys and PAM setup |
| `sudo pulsarkey profile <mode>` | Root | Switches profile: `convenience`, `fortress`, or `lockdown` |
| `pulsarkey bio` | User | Interactive on-key fingerprint & PIN manager |
| `pulsarkey autolock [on\|off\|status]` | User | Toggles Presence Sentinel auto-lock on key removal |
| `pulsarkey ssh-setup` | User | Hardware-backed FIDO2 SSH key & Git commit signing wizard |
| `sudo pulsarkey backup pair` | Root | Interactive wizard to pair a secondary backup YubiKey |
| `pulsarkey rescue generate` | User | Generates 8 emergency paper keys with desktop certificate |
| `pulsarkey rescue runbook` | User | Displays offline disaster recovery runbook |
| `sudo pulsarkey rescue usb <PATH>` | Root | Creates automated offline `pulsar-rescue.sh` on USB drive |
| `pulsarkey audit` | User | Formatted viewer for authentication pulses & hardware events |
| `sudo pulsarkey uninstall` | Root | Reverts all PAM configurations back to password authentication |

---

## 🔄 Safe Rollback & Uninstallation

To cleanly revert all PAM modifications, remove hardware rules, and return to standard password authentication:
```bash
sudo pulsarkey uninstall
```
To also purge underlying PAM packages:
```bash
sudo pulsarkey uninstall --purge-packages
```

---

## 📁 Repository Structure

```
PulsarKey/
├── Cargo.toml                                 # Rust build manifest (pulsarkey, cosmic-fido2, pulsarkey-settings)
├── src/
│   ├── lib.rs                                 # Core library & shared module declarations
│   ├── main.rs                                # CLI entry point (setup, uninstall, status, rescue, bio)
│   ├── gui_main.rs                            # Dedicated binary entry point for pulsarkey-settings (deprecation stub)
│   ├── gui.rs                                 # Deprecation notice stubs (replaces legacy GUI)
│   ├── platform/                              # Cross-platform abstractions (Linux / macOS PAM, locking, notifications)
│   │   └── mod.rs
│   ├── rescue.rs                              # Emergency paper recovery tokens & offline rescue runbook
│   ├── applet.rs                              # StatusNotifierItem panel applet & macOS Sentinel daemon
│   ├── audit.rs                               # Authentication Audit Journal ('Recent Pulses')
│   ├── backup.rs                              # Backup Key pairing & redundancy assistant
│   ├── bio.rs                                 # On-key biometric & FIDO2 PIN manager
│   ├── config.rs                              # Config persistence (~/.config/pulsarkey/config.json)
│   ├── profiles.rs                            # Security strictness profiles (Convenience, Fortress, Lockdown)
│   └── ssh_setup.rs                           # Hardware SSH & Git commit signing wizard
├── packaging/
│   ├── build_deb.sh                           # Automated Debian .deb builder
│   ├── macos/                                 # macOS packaging (PulsarKey.app, DMG, PKG, Homebrew Cask)
│   │   ├── Info.plist                         # macOS App bundle property list
│   │   ├── build_mac.sh                       # macOS App bundle, DMG, and PKG builder
│   │   ├── homebrew/pulsarkey.rb              # Homebrew Cask formula definition
│   │   └── README.md                          # macOS deployment guide
│   ├── README.md                              # Packaging guide
│   ├── cosmic-fido2.svg                       # 512x512 vector icon
│   └── flatpak/                               # Flathub / COSMIC Flatpak manifest & AppStream XML
│       ├── io.github.mzia.PulsarKey.yaml      # Flatpak build definition
│       ├── io.github.mzia.PulsarKey.metainfo.xml # AppStream 1.0 metadata
│       └── io.github.mzia.PulsarKey.desktop   # Desktop launcher
├── dist/
│   └── pulsarkey_1.4.1_amd64.deb              # Pre-compiled native Debian package
├── Makefile                                   # 'make build', 'make install', 'make deb'
├── LICENSE                                    # MIT License
└── README.md                                  # Documentation
```

---

## 📄 License

Distributed under the **MIT License**. See [LICENSE](LICENSE) for details.  
Developed with ❤️ by **M. Zia**.
