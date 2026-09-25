# 🍎 PulsarKey for macOS

PulsarKey brings unified hardware-backed FIDO2 and biometric security (YubiKey Bio, YubiKey 5 Series, Nitrokey, SoloKeys, Google Titan) to macOS (macOS 12 Monterey or later), with full support for Apple Touch ID & WebAuthn.

It provides a native, "Mac-First" security experience featuring:
- **Native macOS Menu Bar Companion (`PulsarKeyBar`):** Unobtrusive Apple Menu Bar status item displaying live key presence, Touch ID status, one-click screen locking, and Sentinel auto-lock toggling.
- **Apple Touch ID & WebAuthn Compatibility:** Seamlessly combines built-in Touch ID biometrics (`pam_tid.so`) with physical FIDO2 tokens in `/etc/pam.d/sudo_local`. Authenticate `sudo` with Touch ID when using your MacBook keyboard, or with your external security key when in clamshell mode!
- **Persistent PAM Configuration (`sudo_local`):** Uses macOS Ventura/Sonoma/Sequoia `/etc/pam.d/sudo_local` so macOS point and major updates never wipe your authentication configuration.
- **Background Sentinel via `launchd`:** One-command background service management (`pulsarkey daemon install`) via native macOS LaunchAgents (`~/Library/LaunchAgents/io.github.mzia.pulsarkey.plist`) that starts automatically at login.
- **Interactive Security Dashboard (Ratatui TUI):** Full-screen terminal control suite with live device telemetry, biometric manager, strictness profiles, and audit log.
- **Hardware SSH & Git Signing:** Automate `ed25519-sk` and `ecdsa-sk` key generation and commit signing.

---

## 📦 Prerequisites

Install the required FIDO2 PAM module and Yubico CLI via Homebrew:

```bash
brew install pam-u2f ykman
```

---

## 🔨 Building for macOS

To compile binaries, assemble the native Menu Bar applet, and create the `PulsarKey.app` bundle:

```bash
# Clone the repository
git clone https://github.com/mzia/PulsarKey.git
cd PulsarKey

# Run the macOS packaging script
./packaging/macos/build_mac.sh
```

### Outputs generated:
1. **`dist/macos/PulsarKey.app`**: Complete macOS Application Bundle (includes `pulsarkey` CLI and `PulsarKeyBar` native Menu Bar app).
2. **`dist/PulsarKey-1.4.1.dmg`**: Drag-and-drop disk image installer.
3. **`dist/PulsarKey-1.4.1.pkg`**: Standard macOS package installer (via `pkgbuild`).

---

## 🍺 Installation via Homebrew Cask

Once packaged or downloaded:

```bash
brew install --cask packaging/macos/homebrew/pulsarkey.rb
```

This installs `PulsarKey.app` to `/Applications` and creates symlinks for CLI commands:
```bash
pulsarkey status       # Check PAM, Touch ID, and hardware key health
pulsarkey touchid      # Manage Apple Touch ID WebAuthn integration
pulsarkey daemon       # Manage background launchd agent (install/uninstall/status)
sudo pulsarkey setup   # Run guided onboarding wizard
pulsarkey applet       # Launch menu bar status companion
```

---

## 🍏 Apple Touch ID & WebAuthn Integration

PulsarKey integrates built-in Apple Touch ID into your system PAM workflow:
- **Status check:** `pulsarkey touchid status`
- **Enable Touch ID for sudo:** `sudo pulsarkey touchid enable`
- **Disable Touch ID for sudo:** `sudo pulsarkey touchid disable`

When enabled, `/etc/pam.d/sudo_local` is configured with:
```pam
auth       sufficient     pam_tid.so
auth       sufficient     pam_u2f.so cue origin=pam://pulsarkey appid=pam://pulsarkey authfile=/etc/yubico/u2f_keys
```
This enables dual authentication: touch your MacBook's Touch ID sensor when open, or touch/scan your plugged-in Security Key when docked in clamshell mode!

---

## 🛡️ macOS PAM Configuration Details

macOS PAM files are located in `/etc/pam.d/`:

| Service | PAM Configuration File | Description | Survives OS Updates |
| :--- | :--- | :--- | :--- |
| **Sudo CLI** | `/etc/pam.d/sudo_local` | Terminal privilege elevation | **Yes (Apple native standard)** |
| **Lockscreen** | `/etc/pam.d/screensaver` | macOS screensaver and display lock authentication | Maintained by PulsarKey |
| **GUI Elevation** | `/etc/pam.d/authorization` | SecurityAgent system prompts (system settings, keychain access) | Maintained by PulsarKey |

---

## 🔒 Presence Sentinel & `launchd` Service

On macOS, PulsarKey detects token presence via native IOKit USB registries (`ioreg`). When the token is unplugged:
1. A native macOS notification is sent.
2. The session is immediately locked via native macOS security keystroke or `pmset displaysleepnow`.
3. An audit record is logged to `~/.config/pulsarkey/audit.log`.

### Managing the Background Sentinel Service:
```bash
pulsarkey daemon install      # Installs and starts launchd agent at login
pulsarkey daemon status       # Displays live launchd status and logs
pulsarkey daemon stop         # Temporarily stops the daemon
pulsarkey daemon start        # Resumes the daemon
pulsarkey daemon uninstall    # Removes the launchd agent
```
