# 🌌 PulsarKey

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Pop!__OS%20COSMIC-teal.svg?style=flat-square)](https://system76.com/cosmic)
[![Packaging](https://img.shields.io/badge/Packaging-Debian%20.deb%20%7C%20Flathub-purple.svg?style=flat-square)](packaging/)
[![Release](https://img.shields.io/github/v/release/mzia/PulsarKey?style=flat-square)](https://github.com/mzia/PulsarKey/releases)

> **Hardware-backed FIDO2 & Biometric Authentication Manager for Pop!_OS COSMIC**  
> *Seamless, zero-lag YubiKey Bio unlocking, passwordless sudo, and real-time top bar panel monitoring.*

---

## 📖 What is PulsarKey?

**PulsarKey** is a native Rust security utility, system configuration manager, and desktop status applet designed specifically for **Pop!_OS** and the **COSMIC Desktop Environment**.

It provides seamless integration for hardware security tokens (specifically **YubiKey C Bio**, **YubiKey 5 Series**, and FIDO2/WebAuthn authenticators) to authenticate both the **COSMIC lockscreen greeter** and administrative **sudo** actions.

### The Problems It Solves:
1. **Modern Biometric FIDO2 Support**: System76's official YubiKey documentation relies on legacy OTP challenge-response (`pam_yubico.so`). Modern biometric tokens like the **YubiKey C Bio - FIDO Edition** do not feature an OTP engine and enforce strict hardware User Verification (`alwaysUv: True`). PulsarKey utilizes native FIDO2 assertions with `+presence+verification` to satisfy hardware biometric constraints.
2. **Zero-Lag COSMIC Lockscreen**: In `cosmic-greeter`, the lockscreen interface discards empty password submissions (`if value.is_empty() { return Task::none(); }`). PulsarKey injects an interactive PAM conversation requiring <kbd>Space</kbd> + <kbd>Enter</kbd> to initiate the FIDO2 exchange. This stops the key from flashing prematurely the moment your screen locks, ensuring touch verification is only requested when you are actively logging back in.
3. **Live Desktop Status & Diagnostics**: Users often have no visibility into whether their physical security token is recognized or working. PulsarKey features a lightweight native top bar panel applet that monitors hardware connections, displays current PAM states, and provides a one-click sensor test with desktop notifications.

---

## 📋 Prerequisites

Before setting up PulsarKey, ensure your environment meets the following requirements:

### 1. Hardware Requirements
* **FIDO2 / U2F Security Token**:
  * Tested and optimized for the **YubiKey C Bio – FIDO Edition** (Firmware 5.7.4+).
  * Fully compatible with **YubiKey 5 Series** (5C, 5 NFC, 5 Nano) and any standard FIDO2/WebAuthn key supporting User Verification (biometrics) or capacitive touch presence.
* **USB Port**: An available USB-A or USB-C port to connect your hardware token.

### 2. On-Key Fingerprint Enrollment (Important)
> [!IMPORTANT]
> **PulsarKey connects your enrolled security key to the Linux PAM subsystem and the COSMIC Desktop — it does *not* enroll or store fingerprints on the hardware token itself.**
>
> If you are using a biometric key (such as the **YubiKey C Bio**), your fingerprints must already be registered directly on the key's onboard secure element before running `pulsarkey setup`.

You can enroll your fingerprints using either of the following standard methods:

#### Method A: Using YubiKey Manager CLI (`ykman`) [Terminal]
```bash
# 1. Install yubikey-manager
sudo apt install -y yubikey-manager

# 2. Set a FIDO2 PIN (required by hardware before adding biometric credentials)
ykman fido access change-pin

# 3. Add your fingerprint (follow terminal prompts to touch/scan sensor repeatedly)
ykman fido fingerprints add "Right Index"

# 4. Verify your enrolled fingerprints
ykman fido fingerprints list
```

#### Method B: Using Yubico Authenticator GUI [Desktop App]
1. Install **Yubico Authenticator** from Pop!_Shop / Flathub or via:
   ```bash
   flatpak install flathub com.yubico.yubioath
   ```
2. Insert your YubiKey and launch **Yubico Authenticator**.
3. Select your device from the left sidebar and navigate to **WebAuthn / FIDO2** (or **Passkeys**).
4. Set a FIDO2 PIN if prompted, then click **Fingerprints** -> **Add Fingerprint**.
5. Follow the visual prompts to touch the sensor until biometric enrollment reaches 100%.

### 3. Operating System & Desktop
* **Operating System**: **Pop!_OS 24.04 LTS** (or compatible Debian/Ubuntu derivatives).
* **Desktop Environment**: **COSMIC Desktop (Epoch)** featuring:
  * `cosmic-greeter` (the lockscreen display manager).
  * `cosmic-panel` (supporting D-Bus `StatusNotifierItem` applets).
  * `cosmic-term` or a standard terminal emulator.

### 4. Software Dependencies
The following packages are required for PAM integration and hardware detection:
* **`libpam-u2f`**: The Linux PAM module (`pam_u2f.so`) for FIDO2 and U2F authentication.
* **`pamu2fcfg`**: Utility used to register security tokens and generate cryptographic credential mappings.
* **`yubikey-manager` (`ykman`)**: Command-line tool used by the applet and status dashboard for hardware telemetry.
* **`libnotify-bin` (`notify-send`)**: Required by the applet to trigger desktop diagnostic notifications.

> [!NOTE]
> When running `sudo pulsarkey setup`, the tool will automatically check for these packages and prompt to install any missing dependencies via `apt`.

### 5. System Privileges
* **Root (`sudo`) Privileges**: Required exclusively for the `setup` and `uninstall` subcommands to write PAM files (`/etc/pam.d/`), udev rules (`/etc/udev/rules.d/`), and credential mappings (`/etc/yubico/`).
* **User Privileges**: Regular user permissions are sufficient to run the panel applet (`pulsarkey applet`) and inspect status (`pulsarkey status`).

### 6. Build Requirements *(Source Compilation Only)*
If you are compiling from source rather than installing the pre-built `.deb`:
* **Rust Toolchain**: `rustc` and `cargo` (1.80+ / 2024 edition).
* **System Libraries**: `libc6-dev`, `libdbus-1-dev`, `pkg-config`.

---

## ✨ Key Features

- 🔒 **Zero-Lag Lockscreen Integration**: Solves the COSMIC Greeter empty-submit filter via an interactive prompt (<kbd>Space</kbd> + <kbd>Enter</kbd>), preventing premature key blinking immediately upon locking and prompting only when you log back in.
- ⚡ **Touch & Biometric Sudo**: Authenticate `sudo` commands instantly with a single touch or fingerprint scan.
- 🪟 **Polkit GUI Elevation**: Authorize graphical administrative dialogs (Pop!_Shop, Eddy, COSMIC Settings, and `pkexec`) with a simple fingerprint scan on your physical token.
- 🛡️ **Presence Sentinel (Auto-Lock on Removal)**: Automatically locks your COSMIC desktop session (`loginctl lock-session`) the second your security key is unplugged. Easily toggled on/off from the panel applet or CLI.
- 🔑 **Hardware-Backed SSH & Git Signing**: Generates resident FIDO2 keys (`ed25519-sk`) on your YubiKey and configures Git to cryptographically sign all commits with your biometric touch (`pulsarkey ssh-setup`).
- 🖥️ **Native COSMIC Top Bar Applet**: StatusNotifierItem tray applet featuring real-time USB hardware detection, PAM configuration health checks, session locking, and one-click biometric diagnostics.
- 🧬 **Hardware User Verification (UV)**: Enforces FIDO2 `+presence+verification` assertions, perfectly supporting biometric keys like the **YubiKey C Bio**.
- 🛡️ **Anti-Lockout & Safe Rollback**: Never locks you out of your desktop. Standard password authentication remains available as a fallback (`nouserok`), and `pulsarkey uninstall` cleanly restores original PAM configs.
- 📦 **Dual Packaging**: Ships both as a native Debian package (`.deb`) for the Pop!_OS COSMIC Store / Eddy and a containerized Flathub Flatpak bundle.

---

## 🚀 Quick Start

### 1. Installation

#### Option A: Native Debian Package (COSMIC Store / APT)
Download the latest `.deb` package from [Releases](https://github.com/mzia/PulsarKey/releases):
```bash
sudo apt install ./dist/pulsarkey_1.1.0_amd64.deb
```
*(Or right-click `pulsarkey_1.1.0_amd64.deb` in COSMIC Files and select **Open With -> COSMIC Store / Eddy**).*

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
5. Safely inject PAM configurations into `/etc/pam.d/cosmic-greeter`, `/etc/pam.d/sudo`, and `/etc/pam.d/polkit-1`.

---

### 3. Launch the COSMIC Panel Applet

Start the applet and enable session autostart:
```bash
pulsarkey applet --install-autostart
```
The applet will appear on your COSMIC panel with a real-time status icon (`auth-fingerprint-symbolic`).

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
🛡️ Polkit GUI:  FIDO2 (Direct Touch)
👥 Enrolled:    1 Key (Biometrics: Yes)
─────────────────────────────────
[✓] 🛡️ Auto-Lock on Key Removal
─────────────────────────────────
🔍 Test Fingerprint Sensor...
🚀 Setup / Add Key (Terminal)...
🔑 Hardware SSH & Git Signing (Terminal)...
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
PAM polkit-1 (GUI):      FIDO2 Enabled (Interactive: No (Direct touch))
Sentinel Auto-Lock:      Enabled (locks desktop on removal)
Hardware SSH Key:        Configured (~/.ssh/id_ed25519_sk, FIDO2 Resident)
Git Commit Signing:      Configured (SSH format, Auto-signing enabled)

Hardware Detection:
  Device type: YubiKey C Bio - FIDO Edition
  Serial number: 31086111
  Firmware version: 5.7.4
  Formfactor: Keychain (USB-C)
==================================================
```

---

## 🔑 Hardware-Backed SSH & Git Signing

PulsarKey includes an automated wizard to generate hardware-backed FIDO2 SSH keys (`ed25519-sk`) and configure cryptographic Git commit signing:

```bash
pulsarkey ssh-setup
```

Or trigger it directly from the COSMIC panel applet menu: **🔑 Hardware SSH & Git Signing (Terminal)...**

### What it does:
1. **Hardware Verification**: Queries your connected security key (YubiKey C Bio or standard FIDO2).
2. **Resident Key Generation**: Creates an OpenSSH `ed25519-sk` key pair with biometric User Verification (`-O verify-required`) stored directly on your physical hardware (`-O resident`).
3. **Cryptographic Git Signing**: Configures Git to sign all commits with your hardware key (`gpg.format = ssh`, `commit.gpgsign = true`, `tag.gpgsign = true`).
4. **Allowed Signers & Verification**: Adds your public key to `~/.ssh/allowed_signers` and configures Git so `git log --show-signature` displays verified signatures immediately.
5. **Instant Clipboard Export**: Automatically copies your public key (`~/.ssh/id_ed25519_sk.pub`) to your clipboard via `wl-copy`/`xclip` for one-click pasting into GitHub, GitLab, or remote servers.

### CLI Options:
- `pulsarkey ssh-setup` — Full interactive guided setup with defaults.
- `pulsarkey ssh-setup --no-resident` — Generate non-resident key file without on-chip credential storage.
- `pulsarkey ssh-setup --no-git-sign` — Generate SSH hardware key only, skipping Git signing configuration.
- `pulsarkey ssh-setup --key-path ~/.ssh/custom_key` — Specify custom output path for the key pair.

---

## 🧪 Verification & Testing

1. **Test Sudo Authentication**:
   ```bash
   sudo -k && sudo whoami
   ```
   *Touch your YubiKey sensor when prompted — the command will execute without requiring a password.*

2. **Test COSMIC Greeter Lockscreen**:
   - Press <kbd>Super</kbd> + <kbd>L</kbd> to lock your session.
   - Notice the key remains dormant until you are ready.
   - Press <kbd>Space</kbd> followed by <kbd>Enter</kbd>.
   - The green sensor ring pulses — scan your registered fingerprint to unlock instantly!

3. **Test Polkit GUI Elevation**:
   ```bash
   pkexec whoami
   ```
   *Touch your YubiKey sensor when prompted — the graphical authorization succeeds instantly without typing your password.*

4. **Test Presence Sentinel (Auto-Lock on Removal)**:
   - Toggle Auto-Lock to `[✓]` in the panel applet (or run `pulsarkey autolock enable`).
   - Unplug your YubiKey from the USB port.
   - PulsarKey instantly triggers `loginctl lock-session`, securing your desktop the moment you step away.

5. **Test Hardware Git Commit Signing**:
   - Make a commit in any repository:
     ```bash
     git commit -m "test commit"
     ```
   - Touch or scan your fingerprint on your YubiKey to sign the commit.
   - Verify the signature:
     ```bash
     git log -1 --show-signature
     ```

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
│   ├── applet.rs                              # COSMIC StatusNotifierItem panel applet
│   ├── config.rs                              # Config persistence (~/.config/pulsarkey/config.json)
│   └── ssh_setup.rs                           # Hardware SSH & Git commit signing wizard
├── packaging/
│   ├── build_deb.sh                           # Automated Debian .deb builder
│   ├── README.md                              # Packaging guide
│   ├── cosmic-fido2.svg                       # 512x512 vector icon
│   └── flatpak/                               # Flathub / COSMIC Flatpak manifest & AppStream XML
│       ├── io.github.mzia.PulsarKey.yaml      # Flatpak build definition
│       ├── io.github.mzia.PulsarKey.metainfo.xml # AppStream 1.0 metadata
│       └── io.github.mzia.PulsarKey.desktop   # Desktop launcher
├── .github/
│   └── workflows/
│       ├── ci.yml                             # Continuous integration (build, test, appstream)
│       └── release.yml                        # Automated GitHub Releases & .deb distribution
├── dist/
│   └── pulsarkey_1.1.0_amd64.deb              # Pre-compiled native Debian package
├── Makefile                                   # 'make build', 'make install', 'make deb'
├── LICENSE                                    # MIT License
└── README.md                                  # Documentation
```

---

## 📄 License

Distributed under the **MIT License**. See [LICENSE](LICENSE) for details.

Developed with ❤️ for Pop!_OS COSMIC by **M. Zia**.
