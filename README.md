# 🌌 PulsarKey

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Pop!__OS%20COSMIC%20%7C%20macOS-teal.svg?style=flat-square)](https://system76.com/cosmic)
[![Packaging](https://img.shields.io/badge/Packaging-Debian%20.deb%20%7C%20macOS%20.dmg%20%7C%20Homebrew-purple.svg?style=flat-square)](packaging/)
[![Release](https://img.shields.io/github/v/release/mzia/PulsarKey?style=flat-square)](https://github.com/mzia/PulsarKey/releases)

> **Hardware-backed FIDO2 & Biometric Authentication Manager for Pop!_OS COSMIC & macOS**  
> *Seamless, zero-lag YubiKey Bio unlocking, passwordless sudo, Sentinel presence auto-lock, and native GUI settings.*

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

- ⚙️ **COSMIC Native Settings App (`pulsarkey-settings` / `pulsarkey gui`)**: Complete graphical desktop control panel built with Iced / COSMIC styling, featuring tabs for Overview, Security Profiles, Biometrics & PIN, Audit Journal, Backup Key Assistant, and Emergency Recovery Runbook.
- 🛟 **Emergency Paper Key & Rescue Suite (`pulsarkey rescue`)**: Generates 8 high-entropy, cryptographically hashed one-time recovery paper tokens (`XXXX-XXXX-XXXX-XXXX`), printable emergency rescue certificates, automated offline rescue USB scripts, and full single-user / Live USB PAM bypass runbooks.
- 🔒 **Zero-Lag Lockscreen Integration**: Solves the COSMIC Greeter empty-submit filter via an interactive prompt (<kbd>Space</kbd> + <kbd>Enter</kbd>), preventing premature key blinking immediately upon locking and prompting only when you log back in.
- ⚡ **Touch & Biometric Sudo**: Authenticate `sudo` commands instantly with a single touch or fingerprint scan.
- 🪟 **Polkit GUI Elevation**: Authorize graphical administrative dialogs (Pop!_Shop, Eddy, COSMIC Settings, and `pkexec`) with a simple fingerprint scan on your physical token.
- 🛡️ **Security Strictness Profiles**: Switch authentication modes on the fly between **Convenience (1FA)**, **Fortress (True 2FA: Password + Touch)**, and **Lockdown (Hardware Mandatory)** from the panel applet, GUI, or CLI (`pulsarkey profile`).
- 📜 **Authentication Audit Journal**: Real-time logging of authentication, elevation, and USB hardware events with an instant panel applet submenu (**Recent Pulses**) and formatted CLI viewer (`pulsarkey audit`).
- 👯 **Backup Key Pairing & Recovery**: Guided redundant key enrollment and verification (`pulsarkey backup`) ensuring fail-safe multi-key desktop security.
- 🧬 **Native Biometric & Fingerprint Manager**: Direct on-device biometric enrollment, template renaming, fingerprint deletion, and FIDO2 hardware PIN management (`pulsarkey bio` & `pulsarkey pin`) without requiring external GUI tools.
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
sudo apt install ./dist/pulsarkey_1.4.0_amd64.deb
```
*(Or right-click `pulsarkey_1.4.0_amd64.deb` in COSMIC Files and select **Open With -> COSMIC Store / Eddy**).*

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
1. Detect your hardware and install required PAM libraries (`libpam-u2f`, `pamu2fcfg`, `yubikey-manager`).
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
⚙️ Open PulsarKey Settings...
─────────────────────────────────
🔒 Lockscreen: FIDO2 (Space+Enter)
⚡ Sudo Auth:   FIDO2 (Direct Touch)
🛡️ Polkit GUI:  FIDO2 (Direct Touch)
👥 Enrolled:    2 Key(s) (Biometrics: Yes)
─────────────────────────────────
[✓] 🛡️ Auto-Lock on Key Removal
▶ 🛡️ Security Profile: Convenience (1FA)
▶ 📜 Recent Pulses (Audit Log)
─────────────────────────────────
🔍 Test Fingerprint Sensor...
🚀 Setup / Add Key (Terminal)...
🧬 Biometric Fingerprint Manager (Terminal)...
👯 Backup Key Assistant (Terminal)...
🛟 Emergency Rescue Runbook (Terminal)...
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
System Credential Map:   Present (2 keys registered, Biometrics/UV: Enabled)
PAM sudo:                FIDO2 Enabled (Interactive: No (Direct touch))
PAM cosmic-greeter:      FIDO2 Enabled (Interactive: Yes)
PAM polkit-1 (GUI):      Standard (Password only)
Security Profile:        Convenience (1FA Biometric/Touch)
Sentinel Auto-Lock:      Enabled (locks desktop on removal)
Hardware SSH Key:        FIDO2 Active (~/.ssh/id_ed25519_sk)
Git Commit Signing:      Enabled (SSH FIDO2 touch required)
Biometric Engine:        Biometric Sensor Active [Prints: Enrolled (3 attempt(s) remaining), PIN: Set (8 attempt(s) remaining)]
Key Redundancy:          Protected (2 keys enrolled)

Hardware Detection:
  Device type: YubiKey C Bio - FIDO Edition
  Serial number: 33648425
  Firmware version: 5.7.4
  Form factor: Bio (USB-C)
  Enabled USB interfaces: FIDO
==================================================
```

---

## 🧬 Native Biometric & Fingerprint Manager

PulsarKey provides direct, on-device biometric lifecycle management for YubiKey Bio and FIDO2 keys:

```bash
pulsarkey bio
```

Or trigger it directly from the COSMIC panel applet: **🧬 Biometric Fingerprint Manager (Terminal)...**

### Capabilities:
- **Interactive Biometric Dashboard**: Authenticate with your FIDO2 PIN once and manage all on-key fingerprints in a guided terminal session.
- **Biometric Enrollment**: Register new fingerprints directly from PulsarKey with real-time sensor scan prompts and progress feedback.
- **Template Labels**: Name and rename your fingerprints (e.g. *"Right Index"*, *"Left Thumb"*).
- **Template Deletion**: Delete individual fingerprint templates without resetting your security key.
- **FIDO2 PIN Management**: Check remaining PIN attempts, set a new PIN, verify PIN, or change existing PIN.

### CLI Commands:
```bash
pulsarkey bio                            # Interactive management dashboard
pulsarkey bio list                       # List all enrolled fingerprints on token
pulsarkey bio add "Right Index"          # Enroll new fingerprint with custom label
pulsarkey bio rename <ID> "Work Thumb"   # Rename existing fingerprint
pulsarkey bio delete <ID>                # Delete fingerprint template
pulsarkey pin status                     # Check FIDO2 PIN set state & remaining retries
pulsarkey pin change                     # Change FIDO2 hardware PIN
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

## 🛡️ Security Strictness Profiles

PulsarKey lets you toggle authentication modes dynamically depending on your threat model:

| Profile | Mode | Authentication Requirement | Fallback |
| :--- | :--- | :--- | :--- |
| **Convenience** | 1FA (Default) | Fingerprint or token touch alone | Password fallback enabled |
| **Fortress** | True 2FA | **Both** account password **and** physical token touch | Password alone is insufficient |
| **Lockdown** | Hardware Strict | Physical token strictly mandatory | **Zero password fallback** |

### Switching Profiles:
```bash
# Check current profile & available modes
pulsarkey profile

# Switch to Fortress (True 2FA)
sudo pulsarkey profile fortress

# Switch to Lockdown (Hardware Mandatory)
sudo pulsarkey profile lockdown

# Return to Convenience Mode
sudo pulsarkey profile convenience
```
*(Or switch directly from the **🛡️ Security Profile** dropdown in the COSMIC top panel applet).*

---

## 📜 Authentication Audit Journal (Recent Pulses)

PulsarKey records security-sensitive events, elevation attempts, and physical token connections into a local audit log:

```bash
pulsarkey audit
```

### What it tracks:
- **AUTH**: Sudo privilege escalation, Polkit GUI prompts, and COSMIC Greeter unlocks.
- **HARDWARE**: Real-time USB token insertions and removals.
- **SENTINEL**: Automatic desktop session lock events when the key is unplugged.
- **SECURITY**: Profile switches, backup key enrollments, and PIN modifications.

To clear the audit log at any time:
```bash
pulsarkey audit --clear
```
*(The 5 most recent pulses are also visible directly in the COSMIC panel applet under **📜 Recent Pulses**).*

---

## 👯 Backup Key & Recovery Assistant

Ensure you are never locked out of your desktop by pairing a secondary/backup key:

```bash
# Check registered keys & SSH redundancy
pulsarkey backup

# Pair a secondary backup key
sudo pulsarkey backup pair

# Test currently connected key
pulsarkey backup test
```

---

## ⚙️ COSMIC Native Settings Control Panel (GUI)

For users who prefer a graphical desktop interface over the terminal CLI or panel tray dropdown, PulsarKey includes a dedicated control panel built with **Iced** and styled for the Pop!_OS COSMIC desktop:

```bash
pulsarkey gui
# Or launch the standalone binary:
pulsarkey-settings
```

You can also launch it with one click:
- **From the COSMIC Panel Applet**: Click the fingerprint tray icon -> **⚙️ Open PulsarKey Settings...**
- **From COSMIC App Library**: Search for **PulsarKey Settings** in your desktop application launcher.

```text
┌───────────────────────────────────────────────────────────────────────────────┐
│ 🌌 PulsarKey Control Panel                                       [🔄 Refresh] │
├───────────────────┬───────────────────────────────────────────────────────────┤
│ [📊 Overview    ] │ System Security Overview                                  │
│ [🛡️ Profiles    ] │                                                           │
│ [🧬 Biometrics  ] │  Hardware Detection                                       │
│ [📜 Audit Log   ] │  Device: YubiKey C Bio - FIDO Edition (Serial: 33648425)   │
│ [👯 Backup Keys ] │                                                           │
│ [🛟 Rescue Kit   ] │  Security Configuration                                   │
│                   │  • Active Profile:   Convenience (1FA Biometric/Touch)     │
│                   │  • Key Redundancy:   Protected (2 keys enrolled)           │
│                   │  • Sentinel Auto-Lock: Enabled                             │
│                   │    [Disable Sentinel Auto-Lock]                           │
│ v1.4.0 Pop!_OS    │                                                           │
└───────────────────┴───────────────────────────────────────────────────────────┘
```

#### Control Panel Features:
- **📊 Overview**: Live telemetry of connected tokens, firmware, security profile, and Sentinel auto-lock toggle.
- **🛡️ Security Profiles**: Compare and switch between **Convenience**, **Fortress**, and **Lockdown** profiles with administrative `pkexec` elevation.
- **🧬 Biometrics & PIN**: Monitor on-key fingerprint health, enroll new fingers, rename templates, and manage FIDO2 PIN retry counters.
- **📜 Audit Journal**: Live graphical viewer displaying recent authentication pulses, privilege escalations, and hardware events.
- **👯 Backup Keys**: Redundancy diagnostics and one-click launcher for the secondary key pairing wizard.
- **🛟 Emergency Rescue**: Monitor available recovery tokens, generate new recovery certificates, and view the emergency runbook.

---

## 🛟 Emergency Paper Recovery Key & Offline Rescue Suite

PulsarKey includes an enterprise-grade offline recovery mechanism to guarantee that you can **always recover root access and regain control of your desktop**, even in the catastrophic event that all registered physical YubiKeys are simultaneously lost, stolen, or destroyed.

### 1. Generate Emergency Paper Key (Recovery Tokens)
```bash
pulsarkey rescue generate
```
This command:
1. Generates 8 high-entropy, Crockford Base32 single-use recovery codes (`XXXX-XXXX-XXXX-XXXX`).
2. Cryptographically hashes each code with SHA-256 and saves the authorization map to `~/.config/pulsarkey/recovery_codes.auth` (or `/etc/pulsarkey/` with mode `0600`).
3. Formats and saves a printable certificate directly to your desktop:
   `~/Desktop/PulsarKey-Emergency-Recovery-Kit.txt`

```text
┌────────────────────────────────────────────────────────────────────────┐
│ 🛡️ PULSARKEY EMERGENCY RECOVERY KIT & PAPER KEY (Pop!_OS COSMIC)       │
├────────────────────────────────────────────────────────────────────────┤
│  [1]  42SF-8VSR-KU6K-BWRL           [5]  85SQ-TVXZ-ZNZC-J5DV           │
│  [2]  JSQX-7R3V-B7L4-5HQG           [6]  MQ45-3AL6-TPXE-HBCV           │
│  [3]  HES6-WGYE-65CM-TKR7           [7]  PCSU-MXDW-XH5E-TMX7           │
│  [4]  X4QJ-CBKW-5RFQ-D958           [8]  N85L-ATX8-6PME-THXM           │
└────────────────────────────────────────────────────────────────────────┘
```
> [!IMPORTANT]
> Print this document and store it in a physically secure location (e.g. fireproof safe). Each code can be used exactly once.

### 2. Verify and Consume a Recovery Token
```bash
pulsarkey rescue verify 42SF-8VSR-KU6K-BWRL
```
*Validates the SHA-256 hash, marks the token as USED with a cryptographic timestamp, and prevents replay attacks.*

### 3. Check Recovery Token Health
```bash
pulsarkey rescue status
```

### 4. Interactive Emergency Runbook
Display offline disaster recovery procedures directly in your terminal:
```bash
pulsarkey rescue runbook
```
*(Or launch it directly from the panel applet: **🛟 Emergency Rescue Runbook (Terminal)...**).*

The runbook provides step-by-step instructions for:
- **Method 1: Single-User Mode (No Live USB needed)**:
  1. Hold <kbd>Space</kbd> on system boot to open the `systemd-boot` menu.
  2. Press <kbd>e</kbd> on the Pop!_OS entry and append `init=/bin/bash` to the kernel parameters.
  3. Boot directly into root maintenance mode, remount root read-write (`mount -o remount,rw /`), and run `pulsarkey profile convenience` (or `pulsarkey rollback`).
- **Method 2: Live USB LUKS Chroot PAM Bypass**:
  One-liner to unlock LUKS and disable PAM hardware enforcement from any Ubuntu/Pop!_OS installer USB:
  ```bash
  sudo cryptsetup luksOpen /dev/nvme0n1p3 cryptdata && sudo mount /dev/mapper/data-root /mnt
  sudo sed -i 's/^auth.*pam_u2f.so.*/# &/' /mnt/etc/pam.d/sudo /mnt/etc/pam.d/cosmic-greeter
  ```

### 5. Automated Offline Rescue Flash Drive Creator
Prepare a self-contained offline emergency rescue flash drive before an emergency occurs:
```bash
sudo pulsarkey rescue usb /media/$USER/<USB_NAME>
```
Creates `pulsar-rescue.sh` on the USB drive. In a crisis, boot any Live USB, plug in the flash drive, and run:
```bash
sudo bash pulsar-rescue.sh
```
The script auto-detects encrypted LUKS partitions (`cryptdata`), mounts the filesystem, creates safety backups (`.rescue.bak`), and reverts PAM to standard password authentication automatically.

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

6. **Test Biometric Fingerprint Manager**:
   ```bash
   pulsarkey bio
   ```

8. **Launch Native Settings Control Panel (GUI)**:
   ```bash
   pulsarkey gui        # Or pulsarkey-settings
   ```
   *Opens the modern, dark-themed Iced desktop window with full tabbed controls for hardware telemetry, profile switching, biometrics, audit logs, and emergency rescue.*

9. **Generate Emergency Paper Recovery Key**:
   ```bash
   pulsarkey rescue generate
   ```
   *Generates 8 high-entropy, SHA-256 hashed recovery tokens and saves a printable emergency kit to your Desktop (`PulsarKey-Emergency-Recovery-Kit.txt`).*

10. **View Offline Emergency Runbook**:
    ```bash
    pulsarkey rescue runbook
    ```
    *Provides step-by-step offline procedures to recover root access via systemd-boot single-user mode or Live USB chroot if all hardware tokens are destroyed.*

11. **Create Emergency Rescue USB Script**:
    ```bash
    sudo pulsarkey rescue usb /media/$USER/<USB_NAME>
    ```
    *Writes an automated offline rescue shell script (`pulsar-rescue.sh`) with auto-LUKS detection to restore password PAM access from any Live USB environment.*

---

## 📋 CLI Command Quick Reference

| Command | Privileges | Description |
| :--- | :--- | :--- |
| `pulsarkey gui` *(or `pulsarkey-settings`)* | User | Launches the COSMIC native settings desktop control panel window |
| `pulsarkey status` | User | Displays comprehensive security status, PAM states, profiles & key health |
| `pulsarkey applet [--install-autostart]` | User | Runs the COSMIC panel StatusNotifierItem applet |
| `sudo pulsarkey setup` | Root | Guided enrollment of primary & backup tokens, PAM & udev configuration |
| `sudo pulsarkey profile [convenience\|fortress\|lockdown]` | Root | Switches live security strictness profile |
| `pulsarkey profile status` | User | Displays active security profile and mode descriptions |
| `pulsarkey bio` | User | Interactive on-key fingerprint & PIN management dashboard |
| `pulsarkey bio [list\|add\|rename\|delete]` | User | Granular biometric template commands |
| `pulsarkey pin [status\|change\|verify\|set]` | User | FIDO2 hardware PIN management & retry status |
| `sudo pulsarkey backup pair` | Root | Interactive wizard to pair secondary backup YubiKey |
| `pulsarkey backup [status\|test]` | User | Inspects hardware key redundancy and tests secondary key |
| `pulsarkey rescue generate` | User / Root | Generates 8 one-time emergency paper keys with desktop certificate |
| `pulsarkey rescue verify <CODE>` | User / Root | Validates and consumes single-use emergency recovery paper token |
| `pulsarkey rescue status` | User | Displays remaining/consumed emergency recovery paper tokens |
| `pulsarkey rescue runbook` | User | Displays offline disaster recovery runbook (single-user & chroot) |
| `sudo pulsarkey rescue usb <PATH>` | Root | Creates automated offline `pulsar-rescue.sh` script on USB flash drive |
| `pulsarkey audit [--clear]` | User | Formatted table viewer for authentication pulses & hardware events |
| `pulsarkey autolock [enable\|disable\|status]` | User | Toggles Presence Sentinel desktop lock on key removal |
| `pulsarkey ssh-setup` | User | Hardware-backed FIDO2 SSH key & Git commit signing wizard |
| `sudo pulsarkey uninstall [--purge-packages]` | Root | Reverts all PAM configurations and restores password authentication |

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

## 🍎 macOS Installation & Packaging

PulsarKey supports **macOS (Monterey 12+)** as a native cross-platform build sharing a single Rust core with Pop!_OS:

### 1. Prerequisites (Homebrew)
```bash
brew install pam-u2f ykman
```

### 2. Building `PulsarKey.app`, `.dmg`, and `.pkg`
```bash
./packaging/macos/build_mac.sh
```

### 3. Installing via Homebrew Cask
```bash
brew install --cask packaging/macos/homebrew/pulsarkey.rb
```

See [packaging/macos/README.md](packaging/macos/README.md) for full macOS PAM targets (`/etc/pam.d/screensaver`, `/etc/pam.d/authorization`), Apple Metal GPU acceleration details, and Sentinel background daemon instructions.

---

## 📁 Repository Structure

```
PulsarKey/
├── Cargo.toml                                 # Rust build manifest (pulsarkey, cosmic-fido2, pulsarkey-settings)
├── src/
│   ├── lib.rs                                 # Core library & shared module declarations
│   ├── main.rs                                # CLI entry point (setup, uninstall, status, rescue, bio)
│   ├── gui_main.rs                            # Dedicated binary entry point for pulsarkey-settings GUI
│   ├── gui.rs                                 # COSMIC native Iced GUI desktop control panel
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
├── .github/
│   └── workflows/
│       ├── ci.yml                             # Continuous integration (build, test, appstream)
│       └── release.yml                        # Automated GitHub Releases & .deb distribution
├── dist/
│   └── pulsarkey_1.4.0_amd64.deb              # Pre-compiled native Debian package
├── Makefile                                   # 'make build', 'make install', 'make deb'
├── LICENSE                                    # MIT License
└── README.md                                  # Documentation
```

---

## 📄 License

Distributed under the **MIT License**. See [LICENSE](LICENSE) for details.

Developed with ❤️ for Pop!_OS COSMIC by **M. Zia**.
