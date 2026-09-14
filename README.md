# 🌌 PulsarKey

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Pop!__OS%20COSMIC-teal.svg?style=flat-square)](https://system76.com/cosmic)
[![Packaging](https://img.shields.io/badge/Packaging-Debian%20.deb%20%7C%20Flathub-purple.svg?style=flat-square)](packaging/)

> **Hardware-backed FIDO2 & Biometric Authentication Manager for Pop!_OS COSMIC**  
> *Seamless, zero-lag YubiKey Bio unlocking, passwordless sudo, and real-time top bar panel monitoring.*

---

## 🌟 The Story Behind the Name

In astronomy, a **Pulsar** is a dense, rapidly rotating celestial neutron star that emits precise, rhythmic pulses of electromagnetic radiation across the cosmos. 

When your FIDO2 security key (such as the **YubiKey C Bio**) awaits your biometric fingerprint or touch presence, its LED sensor pulses with a steady, rhythmic beacon of light. **PulsarKey** bridges the celestial theme of System76's **COSMIC** desktop with the hardware heartbeat of your physical security token.

---

## ✨ Features

- 🔒 **Zero-Lag Lockscreen Integration**: Solves the COSMIC Greeter empty-submit filter (`locker.rs`) via an interactive prompt (`Space` + `Enter`), preventing premature key blinking immediately upon locking and prompting only when you log back in.
- ⚡ **Touch & Biometric Sudo**: Authenticate `sudo` commands instantly with a single touch or fingerprint scan.
- 🖥️ **Native COSMIC Top Bar Applet**: StatusNotifierItem tray applet featuring real-time USB hardware detection, PAM configuration health checks, session locking, and one-click biometric diagnostics.
- 🧬 **Hardware User Verification (UV)**: Enforces FIDO2 `+presence+verification` assertions, perfectly supporting biometric keys like the **YubiKey C Bio** (Firmware 5.7.4+).
- 🛡️ **Anti-Lockout & Safe Rollback**: Never locks you out of your desktop. Standard password authentication remains available as a fallback (`nouserok`), and `pulsarkey uninstall` cleanly restores original PAM configs.
- 📦 **Dual Packaging**: Ships both as a native Debian package (`.deb`) for the Pop!_OS COSMIC Store / Eddy and a containerized Flathub Flatpak bundle.

---

## 🚀 Quick Start

### 1. Installation

#### Option A: Native Debian Package (COSMIC Store / APT)
Download the latest `.deb` release from [Releases](https://github.com/mzia/PulsarKey/releases) or build locally:
```bash
sudo apt install ./packaging/pulsarkey_1.0.0_amd64.deb
```
*(Or right-click the `.deb` file in COSMIC Files and select **Open With -> COSMIC Store / Eddy**).*

#### Option B: Build from Source
```bash
git clone https://github.com/mzia/PulsarKey.git
cd PulsarKey
cargo build --release
sudo make install
```

---

### 2. Configure Your Security Key

Run the interactive guided setup with root privileges:
```bash
sudo pulsarkey setup
```

The setup assistant will:
1. Verify system packages (`libpam-u2f`, `pamu2fcfg`, `yubikey-manager`).
2. Configure udev rules for the `cosmic-greeter` user group.
3. Guide you through enrolling your primary YubiKey with biometric User Verification.
4. Optionally enroll a backup key.
5. Safely inject PAM configurations into `/etc/pam.d/cosmic-greeter` and `/etc/pam.d/sudo`.

---

### 3. Launch the COSMIC Panel Applet

Start the applet and enable session autostart:
```bash
pulsarkey applet --install-autostart
```
The applet will appear on your COSMIC panel with a real-time status icon (`security-high-symbolic` when connected, `security-low-symbolic` when disconnected).

You can also run it as a systemd user service:
```bash
systemctl --user enable --now pulsarkey-applet.service
```

---

## 🖥️ Top Bar Panel Applet

The PulsarKey applet provides an interactive menu directly from the COSMIC panel:

```text
🟢 YubiKey C Bio - FIDO Edition
─────────────────────────────────
🔒 Lockscreen: FIDO2 (Space+Enter)
⚡ Sudo Auth:   FIDO2 (Direct Touch)
👥 Enrolled:    1 Key (Biometrics: Yes)
─────────────────────────────────
🔍 Test Fingerprint Sensor...
🚀 Setup / Add Key (Terminal)...
📊 View Security Status (Terminal)...
🔒 Lock Screen Now
─────────────────────────────────
🚪 Quit Applet
```

---

## 🔍 Status Dashboard

Inspect your hardware and PAM configuration at any time:
```bash
pulsarkey status
```

**Example Output:**
```text
==================================================
 🔍 PulsarKey Security Status (Pop!_OS COSMIC)
==================================================
pamu2fcfg tool:          Installed
COSMIC udev rules:       Configured
System Credential Map:   Present (1 keys registered, Biometrics/UV: Enabled)
PAM sudo:                FIDO2 Enabled (Interactive: No (Direct touch))
PAM cosmic-greeter:      FIDO2 Enabled (Interactive: Yes)

Hardware Detection:
  Device type: YubiKey C Bio - FIDO Edition
  Serial number: 31086111
  Firmware version: 5.7.4
  Formfactor: Keychain (USB-C)
==================================================
```

---

## 🧪 Verification & Testing

1. **Test Sudo Authentication**:
   ```bash
   sudo -k && sudo whoami
   ```
   *Touch your YubiKey sensor when prompted — command will execute without typing a password.*

2. **Test COSMIC Greeter Lockscreen**:
   - Press <kbd>Super</kbd> + <kbd>L</kbd> to lock your session.
   - Notice the key remains dormant until you are ready.
   - Press <kbd>Space</kbd> followed by <kbd>Enter</kbd>.
   - The green sensor ring pulses — scan your registered fingerprint to unlock instantly!

---

## 🔄 Uninstallation & Rollback

To cleanly revert all PAM modifications, remove udev rules, and return to standard password authentication:
```bash
sudo pulsarkey uninstall
```
To also purge the underlying PAM packages:
```bash
sudo pulsarkey uninstall --purge-packages
```

---

## 📁 Repository Structure

```
PulsarKey/
├── Cargo.toml                                 # Rust build manifest (pulsarkey & cosmic-fido2)
├── src/
│   ├── main.rs                                # CLI entry point (setup, uninstall, status)
│   └── applet.rs                              # COSMIC StatusNotifierItem panel applet
├── packaging/
│   ├── build_deb.sh                           # Automated Debian .deb builder
│   ├── README.md                              # Packaging guide
│   ├── cosmic-fido2.svg                       # 512x512 vector icon
│   ├── deb/                                   # Debian package control & postinst scripts
│   └── flatpak/                               # Flathub / COSMIC Flatpak manifest & AppStream XML
│       ├── io.github.mzia.PulsarKey.yaml      # Flatpak build definition
│       ├── io.github.mzia.PulsarKey.metainfo.xml # AppStream 1.0 metadata
│       └── io.github.mzia.PulsarKey.desktop   # Desktop launcher
├── .github/
│   └── workflows/
│       ├── ci.yml                             # Continuous integration (build, test, appstream)
│       └── release.yml                        # Automated GitHub Releases & .deb distribution
├── Makefile                                   # 'make build', 'make install', 'make deb'
├── LICENSE                                    # MIT License
└── README.md                                  # Documentation
```

---

## 📄 License

Distributed under the **MIT License**. See [LICENSE](LICENSE) for details.

Developed with ❤️ for Pop!_OS COSMIC by **M. Zia**.
